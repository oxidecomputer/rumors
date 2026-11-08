//! Background work accumulated by materialized protocol states.
//!
//! [`Work`] collects the pumps created by each protocol phase. [`levels`]
//! drives the phase-specific walks, [`answer`] compares child listings,
//! [`resolver`] pairs replies with outstanding queries, [`assembly`]
//! reconstructs resolved scopes, and [`queues`] owns channel capacities. The
//! terminal state drives every pump and its final result together. A pump
//! failure cancels the remaining work without waiting for a response consumer.
//!
//! Walks use [`erased`] values so their code is compiled once per backend.
//! Their entry methods erase typed requests; [`Work::respond`] assigns the
//! phase's height to outgoing replies for the typed protocol schedule.

use std::pin::{Pin, pin};
use std::sync::Arc;

use futures::{Stream, StreamExt, future::BoxFuture};

mod answer;
mod assembly;
mod charges;
mod levels;
mod queues;
mod quota;
mod resolver;

/// Named for the documentation of `LaterMatches`, which links the covered
/// set's net price.
#[cfg(doc)]
pub(crate) use charges::covered_set_price;
#[cfg(any(test, feature = "test-internals"))]
pub(crate) use charges::grant_bound;
pub(crate) use charges::{
    derived_bytes, floor_allowance, grant_bounds, largest_grant, parked_bytes, scratch,
};
pub(crate) use levels::OpeningAdvance;
pub(crate) use queues::OUTGOING_REPLY_CHUNKS;
#[cfg(test)]
pub(crate) use quota::{Left, Ready, parked_price, parked_room};
pub(crate) use quota::{Overdraft, ReplyQuota};
pub(super) use resolver::Resolver;

/// One leaf request in its queue: the leaf's path, with the charge for the
/// request's queue slot and the parts of the room its reply sets aside that
/// stay with it.
pub(crate) type LeafRequest = (Prefix<Z>, Charge, Reserved);

use super::progress::Progress;
use crate::network::NetworkKeys;
use crate::tree::{
    mirror::streaming::{
        Backend, Leaf,
        channel::Sender,
        erased,
        holding::Holding,
        ledger::{Charge, Ledger},
        materialized::{Error, Reserved},
        protocol::BoxResponses,
        stats::Recorder,
        tasks::{TaskRole, complete},
    },
    typed::{
        Prefix,
        height::{Height, Z},
    },
};

use self::queues::outgoing_responses;

/// Backend and independently runnable tasks retained across protocol phases.
pub struct Work<B>
where
    B: Backend<Node<Z>: Leaf>,
{
    /// The store used to read and reconstruct nodes.
    backend: B,
    /// Shared session statistics updated by the walks.
    stats: Recorder,
    /// Pumps driven together when the protocol reaches its terminal step.
    tasks: Vec<BoxFuture<'static, Result<(), Error<B::Error>>>>,
    /// Records progress-critical publications for tests.
    progress: Progress,
    /// The endpoint's ledger, which every item the walk builds is charged
    /// to.
    ledger: Ledger,
    /// The network's keys, whose hash key the digests the walk lists and
    /// compares are cached under ([`hash_key`](super::hash_key)).
    keys: Option<Arc<NetworkKeys>>,
}

/// Accumulate phase work and drive its pumps to completion.
impl<B> Work<B>
where
    B: Backend<Node<Z>: Leaf>,
{
    /// Construct a new work context with the session's stats recorder, the
    /// endpoint's ledger, and the keys its digests are listed and compared
    /// under.
    pub fn new(
        backend: B,
        stats: Recorder,
        ledger: Ledger,
        keys: Option<Arc<NetworkKeys>>,
    ) -> Self {
        // Every grant this walk asks the ledger for fits a charge's count,
        // checked once per backend as the walk compiles.
        const { charges::assert_grants_fit::<B::Erased, B::Error>() };
        let progress = Progress::new();
        Self {
            backend,
            stats,
            tasks: Vec::new(),
            progress,
            ledger,
            keys,
        }
    }

    /// Clone the endpoint's ledger for one independently driven task.
    fn ledger(&self) -> Ledger {
        self.ledger.clone()
    }

    /// Share the session's keys with one independently driven task.
    fn keys(&self) -> Option<Arc<NetworkKeys>> {
        self.keys.clone()
    }

    /// Clone the backend for one independently driven task.
    fn backend(&self) -> B {
        self.backend.clone()
    }

    /// Clone the stats recorder for one independently driven task.
    pub(super) fn stats(&self) -> Recorder {
        self.stats.clone()
    }

    /// Drive a response stream independently and expose its replies at height `H`.
    ///
    /// The stream is a stage's walk, whose allocations outside the places it
    /// marks itself are its resolutions' for the session-budget meter
    /// ([`holding`](crate::tree::mirror::streaming::holding)).
    fn respond<H: Height>(
        &mut self,
        messages: impl Stream<Item = Result<erased::Reply<B::Erased>, Error<B::Error>>> + Send + 'static,
    ) -> BoxResponses<B, H, Error<B::Error>> {
        let (send, responses) = outgoing_responses::<B, H>();
        self.tasks
            .push(Box::pin(TaskRole::Walk { height: H::HEIGHT }.paced(pump(
                Box::pin(Holding::Resolutions.creating_stream(messages)),
                send,
                self.progress,
                H::HEIGHT,
            ))));
        Box::pin(responses)
    }

    /// Forward a stream of nodes assembled from children at typed height
    /// `height` into an upward return channel.
    fn return_into(
        &mut self,
        height: usize,
        mut returns: Sender<Option<B::Erased>>,
        stream: impl Stream<Item = Result<Option<B::Erased>, Error<B::Error>>> + Send + 'static,
    ) {
        self.tasks
            .push(Box::pin(TaskRole::Assembly { height }.paced(async move {
                let mut stream = pin!(Holding::Assembly.creating_stream(stream));
                while let Some(item) = stream.next().await {
                    if returns.send(item?).await.is_err() {
                        return Ok(());
                    }
                }
                Ok(())
            })));
    }

    /// Drive every registered task and the terminal output to completion,
    /// then credit the session's stats with the units its ledger made wait.
    ///
    /// Every unit the walk builds is admitted by one of the tasks, so once
    /// they complete the ledger's count is the session's.
    pub async fn execute<O>(
        self,
        finish: BoxFuture<'static, Result<O, Error<B::Error>>>,
    ) -> Result<O, Error<B::Error>> {
        let Self {
            stats,
            tasks,
            ledger,
            ..
        } = self;
        let output = complete(tasks, finish).await?;
        stats.held_back(ledger.waited());
        Ok(output)
    }
}

/// Forward one walk's replies to the response queue.
///
/// Queue capacity and its progress argument live at
/// [`queues::outgoing_responses`].
///
/// Errors return directly to the work executor. Sending them through the reply
/// queue would require a consumer that may already have stopped reading.
async fn pump<E: Send, Err: Send + 'static>(
    mut messages: Pin<Box<dyn Stream<Item = Result<erased::Reply<E>, Error<Err>>> + Send>>,
    mut send: Sender<Result<erased::Reply<E>, Error<Err>>>,
    progress: Progress,
    height: usize,
) -> Result<(), Error<Err>> {
    while let Some(reply) = messages.next().await {
        let reply = reply?;
        progress.reply(height, &reply);
        if send.send(Ok(reply)).await.is_err() {
            return Ok(());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
