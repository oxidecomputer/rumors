//! Background work accumulated by the remote protocol states.
//!
//! [`Work`] owns the translation tasks created as the typestate advances.
//! [`stages`] wires each transition, [`encode`] writes local replies, and
//! [`queues`] owns the internal channel capacities. The terminal operation drives these
//! tasks with the stream acceptor and incoming error route.

use std::io::Cursor;
use std::pin::{Pin, pin};
use tokio::io::{AsyncRead, AsyncReadExt, Chain};

use futures::{Stream, StreamExt, future::BoxFuture};

use crate::{
    link::Acceptor,
    message::PayloadCodec,
    tree::{
        mirror::streaming::{
            Backend, Leaf,
            channel::{Receiver, Sender, TrySendError},
            erased,
            materialized::SupplyLedger,
            protocol::BoxResponses,
            remote::{
                adapter::{ReplyLevel, Scope, derive_questions},
                codec::{RunBudget, Speaker},
                proxy::{Error, send_or_cancel},
                streams::{AcceptDriver, AcceptError, FirstStreamError, StreamReceiver},
            },
            stats::Recorder,
            tasks::{cancelled, complete},
            window::Window,
        },
        typed::{
            Hash,
            height::{Height, Z},
        },
    },
};

use self::progress::Progress;

mod encode;
#[cfg(test)]
pub(crate) mod parking_census;
pub(super) mod progress;
mod queues;
mod stages;

/// Remaining control input, with bytes read by the departure watch replayed first.
///
/// Tree replies and message payloads travel on separate data streams and never
/// enter this buffer. A conforming peer can send its completion marker and,
/// during bootstrap or retirement, an identity donation while our
/// reconciliation is still running. The buffer has no fixed byte cap: ordinary
/// gossip needs at most the two-byte marker; an identity donation during
/// retirement or bootstrap can also append its encoded party.
pub type ControlRead<R> = Chain<Cursor<Vec<u8>>, R>;

/// Backend and validation state shared by every incoming reply decoder.
#[derive(Clone)]
pub(super) struct Ingress<B>
where
    B: Backend<Node<Z>: Leaf>,
{
    /// The store used to reconstruct supplied nodes.
    backend: B,
    /// The peer's declared maximum encoded version size.
    version_bytes: u64,
    /// The peer's remaining declared supply allowance.
    ledger: SupplyLedger,
    /// The codec used to decode supplied messages.
    codec: PayloadCodec,
}

impl<B> Ingress<B>
where
    B: Backend<Node<Z>: Leaf>,
{
    /// Combine the backend with the peer's greeting-derived decode limits.
    pub(super) fn new(backend: B, version_bytes: u64, set_len: u64, codec: PayloadCodec) -> Self {
        Self {
            backend,
            version_bytes,
            ledger: SupplyLedger::new(set_len),
            codec,
        }
    }
}

/// Watch for control EOF while preserving later session items for their
/// readers.
///
/// During reconciliation the peer may send its identity donation or completion
/// marker before our data-stream work finishes. Keep those bytes in order and
/// continue to EOF; stopping at the first byte would miss a departure after a
/// partially delivered donation. The peer must await our completion marker
/// before starting its next session.
///
/// This watch stops when reconciliation returns. The returned [`ControlRead`]
/// then replays the saved bytes to the identity and completion readers before
/// they resume reading the transport.
async fn departure(read: &mut (impl AsyncRead + Unpin), ahead: &mut Vec<u8>) -> std::io::Error {
    let mut buffer = [0; 64];
    loop {
        match read.read(&mut buffer).await {
            Ok(0) => return std::io::ErrorKind::UnexpectedEof.into(),
            Ok(len) => ahead.extend_from_slice(&buffer[..len]),
            Err(error) => return error,
        }
    }
}

/// Deferred translation tasks and the physical session which drives them.
pub struct Work<B, R, W, A>
where
    B: Backend<Node<Z>: Leaf>,
    A: Acceptor,
{
    /// Backend and validation state for incoming replies.
    ingress: Ingress<B>,
    /// The session window, which sizes the proxy's question queues and,
    /// through the in-flight count, its parking queues; the scope queues
    /// hold a fan plus one whatever the window.
    window: Window,
    /// Byte budget for each outgoing supply run.
    budget: RunBudget,
    /// The remote greeting's root-fan listing, which
    /// [`initiator`](Self::initiator) replays as the remote's opening
    /// question when the remote initiates.
    remote_listing: Vec<(u8, Hash)>,
    /// Transport and error reporting shared by the tasks.
    physical: Physical<R, W, A>,
    /// Tasks driven concurrently when the protocol reaches its terminal step.
    tasks: Vec<BoxFuture<'static, Result<(), Error<B::Error>>>>,
    /// The session's stats recorder, which counts the window stalls of the
    /// proxy's window-sized question queues.
    stats: Recorder,
    /// Records reply and scope publication order in tests.
    progress: Progress,
}

/// Transport ownership and incoming error reporting for one session.
pub struct Physical<R, W, A>
where
    A: Acceptor,
{
    /// Control input watched during reconciliation and returned with lookahead.
    pub control_read: R,
    /// Control output returned alongside the input.
    pub control_write: W,
    /// The remote elected speaker: the direction whose failures the
    /// terminal attributes when no single stream can be named.
    pub remote: Speaker,
    /// Routes arriving data streams to the tasks awaiting them.
    pub accept: AcceptDriver<A>,
    /// Receives incoming-stream failures and the acceptor's deferred I/O failure.
    pub errors: FirstStreamError,
}

/// Accumulate protocol tasks and coordinate them with the transport.
impl<B, R, W, A> Work<B, R, W, A>
where
    B: Backend<Node<Z>: Leaf>,
    A: Acceptor,
{
    /// Begin accumulating work around an elected physical session.
    pub fn new(
        ingress: Ingress<B>,
        window: Window,
        budget: RunBudget,
        remote_listing: Vec<(u8, Hash)>,
        physical: Physical<R, W, A>,
        stats: Recorder,
    ) -> Self {
        Self {
            ingress,
            window,
            budget,
            remote_listing,
            physical,
            tasks: Vec::new(),
            stats,
            progress: Progress::new(),
        }
    }

    /// Clone the backend for one independently-driven task.
    fn backend(&self) -> B {
        self.ingress.backend.clone()
    }

    /// Clone the state shared by incoming reply decoders.
    fn ingress(&self) -> Ingress<B> {
        self.ingress.clone()
    }

    /// Add one independently runnable protocol task.
    fn spawn(&mut self, task: impl Future<Output = Result<(), Error<B::Error>>> + Send + 'static) {
        self.tasks.push(Box::pin(task));
    }

    /// Decode the remote replies at height `H` from `incoming`, park them,
    /// and expose them to the stage that takes them.
    ///
    /// Each reply is decoded against the record of the local question it
    /// answers, taken from `questions` in order, and then parked as
    /// [`park_and_take`](Self::park_and_take) describes. The typed height
    /// alone chooses the [`ReplyLevel`], here and in the take, so decoding
    /// and the take's derivation cannot disagree on it.
    fn respond<H>(
        &mut self,
        questions: Receiver<Scope>,
        incoming: StreamReceiver,
        publish: Option<Sender<Scope>>,
        progress: Progress,
    ) -> BoxResponses<B, H, Error<B::Error>>
    where
        H: Height,
    {
        let decoded = self
            .ingress()
            .decode_pump(ReplyLevel::at::<H>(), questions, incoming);
        self.park_and_take::<H>(decoded, publish, progress)
    }

    /// Park a decode stream's replies at height `H` and expose them to the
    /// stage that takes them.
    ///
    /// An independent task parks each decoded reply, so decoding never waits
    /// on the walk. When the stage takes a parked reply, the returned stream
    /// derives the scopes the reply's questions create, at the level of
    /// height `H`, and publishes them through `publish`, then hands the
    /// reply over. The responder's terminal replies ask nothing and publish
    /// nothing.
    fn park_and_take<H>(
        &mut self,
        decoded: impl Stream<Item = Result<Parked<B::Erased>, Error<B::Error>>> + Send + 'static,
        mut publish: Option<Sender<Scope>>,
        progress: Progress,
    ) -> BoxResponses<B, H, Error<B::Error>>
    where
        H: Height,
    {
        let level = ReplyLevel::at::<H>();
        let (parking, mut parked, capacity) =
            queues::parked_replies(H::HEIGHT, self.window.capacity(H::HEIGHT));
        let lane = Lane::new(H::HEIGHT, capacity, self.physical.remote.other());
        self.spawn(park(Box::pin(decoded), parking, lane));
        Box::pin(async_stream::stream! {
            while let Some(Parked { reply, answered }) = parked.recv().await {
                let Some(scopes) = &mut publish else {
                    progress.taken_reply(H::HEIGHT, 0);
                    yield Ok(erased::assume_reply::<B, H>(reply));
                    continue;
                };
                let questions = match derive_questions(level, &answered, &reply) {
                    Ok(questions) => questions,
                    Err(error) => {
                        yield Err(Error::TakeDerivation(error));
                        return;
                    }
                };
                progress.taken_reply(H::HEIGHT, questions.len());
                for question in questions {
                    progress.next_scope(H::HEIGHT);
                    send_or_cancel(scopes, question).await;
                }
                yield Ok(erased::assume_reply::<B, H>(reply));
            }
        })
    }

    /// Drive the protocol, incoming streams, and their error reports together.
    ///
    /// Reconciliation uses data streams, leaving control input available to
    /// watch for departure. The accept driver uses that signal to close missing
    /// stream claims, then waits for the protocol to decide its outcome.
    ///
    /// The protocol is polled first: a completed reconciliation succeeds even
    /// if the peer has since departed or closed its stream supply. A reported
    /// violation also stands on its own. For a transport failure, poll the
    /// accept driver once more to collect any ready cause before attributing
    /// the error. This final poll never waits and cannot change a successful
    /// outcome.
    async fn execute<O>(
        self,
        finish: impl Future<Output = Result<O, Error<B::Error>>> + Send,
    ) -> Result<(O, ControlRead<R>, W), Error<B::Error>>
    where
        R: AsyncRead + Unpin,
    {
        let Self {
            physical, tasks, ..
        } = self;
        let Physical {
            mut control_read,
            control_write,
            remote,
            accept,
            mut errors,
        } = physical;
        // Only early control items accumulate here; data-stream traffic keeps
        // flowing through the tasks under its existing flow control.
        let mut ahead = Vec::new();
        let outcome = {
            let mut protocol = Box::pin(complete(tasks, finish));
            let mut accept = pin!(accept.run(departure(&mut control_read, &mut ahead)));
            let mut stream_errors = pin!(errors.first());
            let (mut outcome, protocol_finished) = tokio::select! {
                biased;
                output = &mut protocol => (output, true),
                error = &mut stream_errors => (Err(Error::Stream(error)), false),
                error = &mut accept => (Err(Error::Accept(error)), false),
            };
            if matches!(&outcome, Err(error) if error.is_transport_failure()) {
                match futures::poll!(accept.as_mut()) {
                    // A failed protocol drops its claim receivers. Delivery
                    // to one of those slots is a consequence of teardown,
                    // not evidence that the peer sent an unasked stream.
                    std::task::Poll::Ready(AcceptError::Unexpected { .. }) if protocol_finished => {
                    }
                    std::task::Poll::Ready(error) => outcome = Err(Error::Accept(error)),
                    std::task::Poll::Pending => {}
                }
            }
            // A selected accept error is a violation, so the completed
            // accept future never reaches the extra poll above.
            outcome
        };
        outcome
            .map(|output| {
                (
                    output,
                    Cursor::new(ahead).chain(control_read),
                    control_write,
                )
            })
            .map_err(|error| error.attribute(&mut errors, remote))
    }
}

/// A decoded reply parked until its stage takes it.
pub(super) struct Parked<E> {
    /// The decoded reply.
    reply: erased::Reply<E>,
    /// The record of the local question the reply answers, from which the
    /// take derives the scopes the reply's own questions create.
    answered: Scope,
}

/// One parking queue: its height and derived capacity, and in test builds
/// the local speaker of the side that parks there, which the parking census
/// keys by (both sides park leaf replies).
///
/// Every parking event passes through a `Lane` method, which is where the
/// test-only census hooks live; the parking path itself carries none.
#[derive(Clone, Copy)]
pub(super) struct Lane {
    /// The typed height of the replies parked here.
    height: usize,
    /// The derived parking capacity ([`queues::parking_capacity`]), which an
    /// overflow reports also when a test caps the channel below it.
    capacity: usize,
    /// The local speaker of the side that parks here.
    #[cfg(test)]
    local: Speaker,
}

/// Name a parking queue and record its traffic in test builds.
impl Lane {
    /// Name the parking queue of derived `capacity` at `height`, on the side
    /// whose local speaker is `local`.
    fn new(
        height: usize,
        capacity: usize,
        #[cfg_attr(not(test), expect(unused_variables))] local: Speaker,
    ) -> Self {
        Self {
            height,
            capacity,
            #[cfg(test)]
            local,
        }
    }

    /// Send `parked` into `parking` without waiting, and in test builds
    /// record the queue in the parking census once a reply is parked.
    fn try_park<E>(
        self,
        parking: &mut Sender<Parked<E>>,
        parked: Parked<E>,
    ) -> Result<(), TrySendError<Parked<E>>> {
        parking.try_send(parked)?;
        #[cfg(test)]
        parking_census::parked(self);
        Ok(())
    }
}

/// Park decoded replies until their stage takes them.
///
/// Parking holds every reply this side can have invited at its height
/// ([`queues::parking_capacity`]), and each reply is decoded against the
/// record of the local question it answers, so no reply finds it full while
/// the count's premises hold. Parking therefore never waits: a full channel
/// means a premise has failed, and the reply fails with
/// [`Error::ParkingOverflow`] instead, reporting the lane's derived
/// capacity. A closed channel means the stage is gone, and the task awaits
/// cancellation as [`send_or_cancel`] does. Decode errors return to the
/// executor without waiting for the stage.
async fn park<E: Send, Err: Send + 'static>(
    mut decoded: Pin<Box<dyn Stream<Item = Result<Parked<E>, Error<Err>>> + Send>>,
    mut parking: Sender<Parked<E>>,
    lane: Lane,
) -> Result<(), Error<Err>> {
    while let Some(parked) = decoded.next().await {
        match lane.try_park(&mut parking, parked?) {
            Ok(()) => {}
            Err(TrySendError::Full(_)) => {
                return Err(Error::ParkingOverflow {
                    height: lane.height,
                    capacity: lane.capacity,
                });
            }
            Err(TrySendError::Closed(_)) => cancelled().await,
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
