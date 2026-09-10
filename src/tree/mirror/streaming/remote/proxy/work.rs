//! Background work accumulated by the remote protocol states.
//!
//! Like the materialized implementation's work context, this stores every
//! independently runnable pump as the type-level schedule advances. The final
//! protocol operation concurrently drives the stored pumps, its own terminal
//! work, the session's accept driver, and the incoming-stream error route.

use crate::message::PayloadCodec;
use std::io::Cursor;
use std::pin::{Pin, pin};
use tokio::io::{AsyncRead, AsyncReadExt, Chain};

use futures::{Stream, StreamExt, future::BoxFuture};

use crate::link::Acceptor;
use crate::tree::{
    mirror::streaming::{
        Backend, Leaf,
        channel::{QueueKind, QueueRole, Sender},
        erased,
        materialized::SupplyLedger,
        protocol::BoxResponses,
        remote::{
            codec::{RunBudget, Speaker},
            proxy::{Error, send_or_cancel},
            streams::{AcceptDriver, AcceptError, FirstStreamError},
        },
        tasks::{complete, park_after_published_error},
        window::Window,
    },
    typed::{
        Hash,
        height::{Height, Z},
    },
};

use self::progress::Progress;

mod encode;
pub(super) mod progress;
mod pump;
mod queues;

/// Remaining control input, with bytes read by the departure watch replayed first.
///
/// Tree replies and message payloads travel on separate data streams and never
/// enter this buffer. A conforming peer can send its completion marker and,
/// during bootstrap or retirement, an identity donation while our
/// reconciliation is still running. The buffer has no fixed byte cap: ordinary
/// gossip needs at most the two-byte marker; an identity donation during
/// retirement or bootstrap can also append its encoded party.
pub type ControlRead<R> = Chain<Cursor<Vec<u8>>, R>;

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

/// Deferred reply pumps and the physical session which drives them.
pub struct Work<B, R, W, A>
where
    B: Backend<Node<Z>: Leaf>,
    A: Acceptor,
{
    /// The store used to enumerate and reconstruct nodes.
    backend: B,
    /// Per-edge capacity for the proxy's question and scope queues.
    window: Window,
    /// Byte budget for each outgoing supply run.
    budget: RunBudget,
    /// The remote greeting's `max_version_bytes` declaration, enforced
    /// against every supplied version this session decodes.
    peer_version_bytes: u64,
    /// The remote greeting's `set_len` declaration as a session-total
    /// supply allowance: every leaf record this session decodes charges
    /// it before the payload takes backend custody.
    peer_supplies: SupplyLedger,
    /// The peer's payload codec: the typed ingress every supplied
    /// leaf record decodes through (see [`PayloadCodec`]).
    ///
    /// [`PayloadCodec`]: crate::message::PayloadCodec
    codec: PayloadCodec,
    /// The remote greeting's root-fan listing, consumed by whichever role
    /// the election assigns.
    ///
    /// [`initiator`](Self::initiator) replays it as the remote's opening
    /// question; [`opening_responder`](Self::opening_responder) merges the
    /// local opening's listing against it to decide whether the
    /// early-supply stream opens.
    peer_listing: Vec<(u8, Hash)>,
    /// Transport and error reporting shared by the pumps.
    physical: Physical<R, W, A>,
    /// Pumps driven concurrently when the protocol reaches its terminal step.
    tasks: Vec<BoxFuture<'static, Result<(), Error<B::Error>>>>,
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
    /// Routes arriving data streams to the pumps awaiting them.
    pub accept: AcceptDriver<A>,
    /// Receives pump failures and the acceptor's deferred I/O failure.
    pub errors: FirstStreamError,
}

impl<B, R, W, A> Work<B, R, W, A>
where
    B: Backend<Node<Z>: Leaf>,
    A: Acceptor,
{
    /// Begin accumulating work around an elected physical session.
    #[allow(clippy::too_many_arguments)] // The argument list is the session's
    // greeting-derived configuration, one premise per argument.
    pub fn new(
        backend: B,
        window: Window,
        budget: RunBudget,
        peer_version_bytes: u64,
        peer_set_len: u64,
        peer_listing: Vec<(u8, Hash)>,
        physical: Physical<R, W, A>,
        codec: PayloadCodec,
    ) -> Self {
        Self {
            backend,
            window,
            budget,
            peer_version_bytes,
            peer_supplies: SupplyLedger::new(peer_set_len),
            peer_listing,
            physical,
            tasks: Vec::new(),
            progress: Progress::new(),
            codec,
        }
    }

    /// Clone the backend for one independently-driven task.
    fn backend(&self) -> B {
        self.backend.clone()
    }

    /// Add one independently runnable protocol task.
    fn spawn(&mut self, task: impl Future<Output = Result<(), Error<B::Error>>> + Send + 'static) {
        self.tasks.push(Box::pin(task));
    }

    /// Add a task which actively drives a response stream, and return the
    /// stream's typed exit: the one point where the proxy's decoded erased
    /// replies re-tag at their stage's height.
    fn respond<H>(
        &mut self,
        messages: impl Stream<Item = Result<erased::Reply<B::Erased>, Error<B::Error>>> + Send + 'static,
    ) -> BoxResponses<B, H, Error<B::Error>>
    where
        H: Height,
    {
        // One buffered response is sufficient: whenever the pump blocks,
        // that response is already available to advance the counterparty
        // and release the slot. Buffering a fan would retain whole
        // protocol messages without breaking any additional dependency.
        let (send, responses) = erased::reply_channel::<B, H, Error<B::Error>>(
            QueueRole::new(QueueKind::ProxyResponses, H::HEIGHT),
            1,
        );
        self.spawn(pump(Box::pin(messages), send));
        Box::pin(responses)
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
        // flowing through the pumps under its existing flow control.
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

/// Drive one decoded response stream into its outgoing relay edge.
async fn pump<E: Send, Err: Send + 'static>(
    mut messages: Pin<Box<dyn Stream<Item = Result<erased::Reply<E>, Error<Err>>> + Send>>,
    send: Sender<Result<erased::Reply<E>, Error<Err>>>,
) -> Result<(), Error<Err>> {
    while let Some(message) = messages.next().await {
        let failed = message.is_err();
        send_or_cancel(&send, message).await;
        park_after_published_error(failed).await;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
