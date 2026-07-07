//! The height-recursive descent: one [`Descending`] stage per exchange round,
//! down to the two terminals.
//!
//! Each stage's walk lives in [`reconcile`]; this module owns the stage state
//! that travels between rounds and the terminal futures that drive the session's
//! accumulated work to its reconciled [`Root`].

use std::pin::pin;

use futures::future::{self, BoxFuture};
use futures::join;
use futures::stream::StreamExt;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio_stream::wrappers::ReceiverStream;

use crate::tree::mirror::streaming::protocol::BoxResponses;
use crate::{
    Version,
    tree::{
        mirror::streaming::materialized::merge::merge_disjoint,
        typed::{
            Prefix,
            height::{Height, S, Z},
        },
    },
};

use super::super::backend::{Backend, BoxNodeStream, Leaf, Material, Root};
use super::super::convert::{Convert, converted};
use super::super::message;
use super::super::protocol::{self, OutputError, Requests, Responses};
use super::unknown::Unknown;
use super::{FAN, reconcile};

/// A mirror stage inside the descent: its frontier at height `H` flows downward
/// as a stream while the levels above reassemble concurrently.
pub struct Descending<B, O, T, H>
where
    B: Backend<T, Materialized = Material, Node<Z>: Leaf<T>>,
    O: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
{
    from: B,
    into: O,
    their_version: Version,
    frontier: BoxNodeStream<B, T, H>,
    up: Sender<(Prefix<H>, B::Node<H>)>,
    work: Vec<BoxFuture<'static, Result<(), B::Error>>>,
    finish: BoxFuture<'static, Result<Root<B, T>, B::Error>>,
}

impl<B, O, T, H> Descending<B, O, T, H>
where
    B: Backend<T, Materialized = Material, Node<Z>: Leaf<T>>,
    O: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
{
    /// Assemble a stage from its parts, boxing the frontier channel.
    #[allow(clippy::type_complexity)]
    pub(super) fn new(
        backend: B,
        into: O,
        their_version: Version,
        frontier: Receiver<(Prefix<H>, B::Node<H>)>,
        up: Sender<(Prefix<H>, B::Node<H>)>,
        work: Vec<BoxFuture<'static, Result<(), B::Error>>>,
        finish: BoxFuture<'static, Result<Root<B, T>, B::Error>>,
    ) -> Self {
        Self {
            from: backend,
            into,
            their_version,
            frontier: Box::pin(ReceiverStream::new(frontier).map(Ok)),
            up,
            work,
            finish,
        }
    }
}

impl<B, O, T, H> protocol::Protocol for Descending<B, O, T, H>
where
    B: Backend<T, Materialized = Material, Node<Z>: Leaf<T>>,
    O: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
{
    type Height = H;
    type Output = Root<B, T>;
    type Error = B::Error;
}

impl<B, O, T, H> Descending<B, O, T, S<S<H>>>
where
    B: Backend<T, Materialized = Material, Node<Z>: Leaf<T>>,
    O: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height + Unknown,
    S<H>: Height,
    S<S<H>>: Height,
{
    #[allow(clippy::type_complexity)]
    fn descend(
        self,
        requests: impl Requests<message::Exchanged<B, T, S<S<H>>>>,
    ) -> (
        impl Responses<message::Exchanged<B, T, S<H>>, B::Error>,
        Descending<B, O, T, H>,
    ) {
        let Descending {
            from: backend,
            into,
            their_version,
            frontier,
            up,
            mut work,
            finish,
        } = self;
        let (down_tx, down_rx) = mpsc::channel(FAN);
        let (keep_tx, keep_rx) = mpsc::channel(FAN);
        let (level_tx, level_rx) = mpsc::channel(FAN);
        let (below_tx, below_rx) = mpsc::channel(FAN);

        let walk = reconcile::walk(
            backend.clone(),
            their_version.clone(),
            frontier,
            requests,
            down_tx,
            keep_tx,
            level_tx,
        );
        work.push(ascend(backend.clone(), keep_rx, level_rx, below_rx, up));

        let next = Descending::new(
            backend,
            into,
            their_version,
            down_rx,
            below_tx,
            work,
            finish,
        );
        (walk, next)
    }
}

impl<B, O, T, H> protocol::Exchange<B, O, T> for Descending<B, O, T, S<S<S<H>>>>
where
    B: Backend<T, Materialized = Material, Node<Z>: Leaf<T>>,
    O: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    // `Convert` so the outgoing wire (keyed one level below the frontier)
    // can be re-represented in `O`'s node types.
    H: Convert + Unknown,
    S<H>: Height,
    S<S<H>>: Height,
    S<S<S<H>>>: Height,
    // Discharged at each concrete height by one of the three `AfterExchange`
    // blanket impls; cannot be proven generically by the trait solver.
    Descending<B, O, T, S<H>>: protocol::AfterExchange<B, O, T, S<H>>,
{
    type Next = Descending<B, O, T, S<H>>;

    fn exchange(
        self,
        requests: impl Requests<message::Exchanged<B, T, S<S<S<H>>>>>,
    ) -> (
        BoxResponses<message::Exchanged<O, T, S<S<H>>>, OutputError<Self, O, T>>,
        Self::Next,
    ) {
        let backend = self.from.clone();
        let into = self.into.clone();
        let (walk, mut next) = self.descend(requests);
        let sending = reconcile::outgoing(&mut next.work, converted(backend, into, walk));
        (Box::pin(ReceiverStream::new(sending)), next)
    }
}

impl<B, O, T> protocol::CloseInitiator<B, O, T> for Descending<B, O, T, S<S<Z>>>
where
    B: Backend<T, Materialized = Material, Node<Z>: Leaf<T>>,
    O: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    type Next = Descending<B, O, T, Z>;

    fn close_initiator(
        self,
        requests: impl Requests<message::Exchanged<B, T, S<S<Z>>>>,
    ) -> (
        BoxResponses<(Prefix<S<Z>>, message::Closing<O, T>), OutputError<Self, O, T>>,
        Self::Next,
    ) {
        let backend = self.from.clone();
        let into = self.into.clone();
        let (walk, mut next) = self.descend(requests);
        let closing = walk.filter_map(|item| async {
            match item {
                Ok((prefix, message::Exchange::Providing(node))) => {
                    Some(Ok((prefix, message::Closing::Providing(node))))
                }
                Ok((prefix, message::Exchange::Requested)) => {
                    Some(Ok((prefix, message::Closing::Requested)))
                }
                // Vacuous at leaf height (see [`message::Closing`]) because
                // it's not possible to be uncertain about a leaf: you either
                // know you have it, or know you don't.
                Ok((_, message::Exchange::Uncertain(..))) => None,
                Err(error) => Some(Err(error)),
            }
        });
        let sending = reconcile::outgoing(&mut next.work, converted(backend, into, closing));
        (Box::pin(ReceiverStream::new(sending)), next)
    }
}

impl<B, O, T> protocol::CompleteResponder<B, O, T> for Descending<B, O, T, S<Z>>
where
    B: Backend<T, Materialized = Material, Node<Z>: Leaf<T>>,
    O: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    fn complete_responder(
        self,
        requests: impl Requests<(Prefix<S<Z>>, message::Closing<B, T>)>,
    ) -> (
        BoxResponses<(Prefix<Z>, message::Complete<O, T>), OutputError<Self, O, T>>,
        impl Future<Output = Result<Root<B, T>, Self::Error>> + Send,
    ) {
        let Descending {
            from: backend,
            into,
            their_version,
            frontier,
            up,
            mut work,
            finish,
        } = self;

        let complete =
            reconcile::complete_walk(backend.clone(), their_version, frontier, requests, up);
        let sending = reconcile::outgoing(&mut work, converted(backend, into, complete));
        (Box::pin(ReceiverStream::new(sending)), settle(work, finish))
    }
}

impl<B, O, T> protocol::CompleteInitiator<B, T> for Descending<B, O, T, Z>
where
    B: Backend<T, Materialized = Material, Node<Z>: Leaf<T>>,
    O: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    async fn complete_initiator(
        self,
        requests: impl Requests<(Prefix<Z>, message::Complete<B, T>)>,
    ) -> Result<Root<B, T>, Self::Error> {
        let Descending {
            from: _,
            into: _,
            their_version: _,
            frontier,
            up,
            work,
            finish,
        } = self;

        let absorb = reconcile::absorb_leaves(frontier, requests, up);
        let (absorbed, settled) = join!(absorb, settle(work, finish));
        absorbed?;
        settled
    }
}

/// Reassemble one stage's slice of the reconciled tree.
///
/// A stage with its frontier at `S<S<H>>` sees reconciled nodes at three
/// heights: kept frontier nodes (`keep`), walk verdicts one level below
/// (`level`), and (once the stages beneath have resolved them) reconciled
/// disputes two levels below, arriving through `below`. The three sets are
/// prefix-disjoint because they route through mutually exclusive verdicts, so
/// the stage's reconciled frontier level is two folds and two disjoint merges;
/// it flows out through `up`, becoming the previous stage's `below`.
fn ascend<B, T, H>(
    backend: B,
    keep: Receiver<(Prefix<S<S<H>>>, B::Node<S<S<H>>>)>,
    level: Receiver<(Prefix<S<H>>, B::Node<S<H>>)>,
    below: Receiver<(Prefix<H>, B::Node<H>)>,
    up: Sender<(Prefix<S<S<H>>>, B::Node<S<S<H>>>)>,
) -> BoxFuture<'static, Result<(), B::Error>>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
    S<H>: Height,
    S<S<H>>: Height,
{
    let reconciled = merge_disjoint(
        ReceiverStream::new(keep).map(Ok),
        backend.clone().parents(merge_disjoint(
            ReceiverStream::new(level).map(Ok),
            backend.parents(ReceiverStream::new(below).map(Ok)),
        )),
    );
    Box::pin(async move {
        let mut reconciled = pin!(reconciled);
        while let Some(item) = reconciled.next().await {
            // A closed channel means the consumer of this level is gone and the
            // session is being torn down.
            if up.send(item?).await.is_err() {
                break;
            }
        }
        Ok(())
    })
}

/// Drive the session's accumulated `work` and its root reassembly to completion
/// together, resolving to the reconciled [`Root`], only if no errors occurred.
async fn settle<B, T, E>(
    work: Vec<BoxFuture<'static, Result<(), B::Error>>>,
    finish: BoxFuture<'static, Result<Root<B, T>, B::Error>>,
) -> Result<Root<B, T>, E>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    E: From<B::Error>,
{
    let (finished, root) = join!(future::join_all(work), finish);
    for result in finished {
        result?;
    }
    Ok(root?)
}
