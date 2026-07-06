//! The height-recursive descent: one [`Descending`] stage per exchange round,
//! down to the two terminals.
//!
//! Each stage's walk lives in [`reconcile`]; this module owns the stage state
//! that travels between rounds and the terminal futures that drive the session's
//! accumulated work to its reconciled [`Root`].

use futures::channel::mpsc;
use futures::future::BoxFuture;
use futures::join;
use futures::stream::StreamExt;

use crate::{
    Version,
    tree::{
        mirror::streaming::BoxMessages,
        typed::{
            Prefix,
            height::{Height, S, Z},
        },
    },
};

use super::super::backend::{Backend, BoxNodeStream, Leaf, Material, Root};
use super::super::convert::{Convert, converted};
use super::super::message;
use super::super::protocol::{self, Messages, OutputError, Requests};
use super::super::unknown::Unknown;
use super::{FAN, Level, ascend, outgoing, reconcile, settle};

/// A mirror stage inside the descent: its frontier at height `H` flows downward
/// as a stream while the levels above reassemble concurrently.
///
/// `frontier` holds this side's disputed subtrees at `H`, fed by the previous
/// stage's walk through a [`FAN`]-bounded channel; `up` is where this stage's
/// [`ascend`] future sends the reconciled level at `H`, feeding the previous
/// stage's ascent in turn; `work` accumulates every concurrent future the
/// session has created so far, and `finish` is the fold their reassembly
/// converges to: the reconciled [`Root`] the terminal resolves to. `into` is
/// the counterparty's backend, whose node types the stage's node-carrying
/// output [converts](super::super::convert) into. Each
/// [`exchange`](protocol::Exchange::exchange) consumes the stage and produces
/// its successor two heights finer, until the terminal stages drive the
/// accumulated work to completion.
pub struct Descending<B, O, T, H>
where
    B: Backend<T, Materialized = Material, Node<Z>: Leaf<T>>,
    O: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
{
    backend: B,
    into: O,
    their_version: Version,
    frontier: BoxNodeStream<B, T, H>,
    up: mpsc::Sender<Level<B, T, H>>,
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
        frontier: mpsc::Receiver<Level<B, T, H>>,
        up: mpsc::Sender<Level<B, T, H>>,
        work: Vec<BoxFuture<'static, Result<(), B::Error>>>,
        finish: BoxFuture<'static, Result<Root<B, T>, B::Error>>,
    ) -> Self {
        Self {
            backend,
            into,
            their_version,
            frontier: Box::pin(frontier.map(Ok)),
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
    /// One step of the descent: start this stage's [`walk`](reconcile::walk)
    /// and [`ascend`] futures and construct the successor two heights finer.
    ///
    /// The returned walk output is still in this side's own node vocabulary
    /// and error type; the callers convert it into the counterparty's.
    /// Shared by [`exchange`](protocol::Exchange::exchange) and
    /// [`close_initiator`](protocol::CloseInitiator::close_initiator).
    #[allow(clippy::type_complexity)]
    fn descend(
        self,
        requests: impl Requests<message::Exchanged<B, T, S<S<H>>>>,
    ) -> (
        impl Messages<message::Exchanged<B, T, S<H>>, B::Error>,
        Descending<B, O, T, H>,
    ) {
        let Descending {
            backend,
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
        BoxMessages<message::Exchanged<O, T, S<S<H>>>, OutputError<Self, O, T>>,
        Self::Next,
    ) {
        let backend = self.backend.clone();
        let into = self.into.clone();
        let (walk, mut next) = self.descend(requests);
        let sending = outgoing(&mut next.work, converted(backend, into, walk));
        (Box::pin(sending), next)
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
        BoxMessages<(Prefix<S<Z>>, message::Closing<O, T>), OutputError<Self, O, T>>,
        Self::Next,
    ) {
        let backend = self.backend.clone();
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
        let sending = outgoing(&mut next.work, converted(backend, into, closing));
        (Box::pin(sending), next)
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
        BoxMessages<(Prefix<Z>, message::Complete<O, T>), OutputError<Self, O, T>>,
        impl Future<Output = Result<Root<B, T>, Self::Error>> + Send,
    ) {
        let Descending {
            backend,
            into,
            their_version,
            frontier,
            up,
            mut work,
            finish,
        } = self;

        let complete =
            reconcile::complete_walk(backend.clone(), their_version, frontier, requests, up);
        let sending = outgoing(&mut work, converted(backend, into, complete));
        (Box::pin(sending), settle(work, finish))
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
            backend: _,
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
