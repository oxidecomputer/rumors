//! Trait-family abstraction of the mirror protocol.
//!
//! Each public step of the [`super::local`] protocol is named by a trait. Each
//! trait carries a `Next` associated type bounded by the trait that describes
//! the only legal *following* call, so the protocol's allowed transitions are
//! encoded at the type level and the chain of calls is statically forced into
//! the right order.
//!
//! The traits are deliberately silent about how a step is implemented:
//! [`super::local::Exchange`] fulfills them by traversing an inner data
//! structure locally, but a remote proxy that forwards each call over the wire
//! (and carries only a phantom `Height`) can fulfill them just as easily.
//!
//! # Trait family
//!
//! | Trait                 | Wire input                      | Wire output                       | `Next`                                |
//! |-----------------------|---------------------------------|-----------------------------------|---------------------------------------|
//! | [`Connect`]           | --                              | [`Version`]                       | [`CompleteConnect`]                   |
//! | [`CompleteConnect`]   | [`Version`]                     | --                                | [`Initiator`] or [`Responder`]        |
//! | [`Accept`]            | [`Version`]                     | [`Version`]                       | [`Initiator`] or [`Responder`]        |
//! | [`Initiator`]         | --                              | [`message::Initiate`]             | [`OpenInitiator`]                     |
//! | [`Responder`]         | [`message::Initiate`]           | [`message::Opening`]              | [`Exchange`] (first steady round)     |
//! | [`OpenInitiator`]     | [`message::Opening`]            | [`message::Exchange<_, _, U^2>`]  | [`Exchange`] (first steady round)     |
//! | [`Exchange`]          | [`message::Exchange`]           | [`message::Exchange`]             | [`AfterExchange<H>`] (see below)      |
//! | [`CloseInitiator`]    | [`message::Exchange<_,_,S<Z>>`] | [`message::Closing`]              | [`CompleteInitiator`]                 |
//! | [`CompleteResponder`] | [`message::Closing`]            | [`message::Complete`]             | terminal                              |
//! | [`CompleteInitiator`] | [`message::Complete`]           | --                                | terminal                              |
//!
//! # The `Exchange<H>::Next` ambiguity
//!
//! After an [`Exchange::exchange`] call at height `H`, the *next* legal call
//! depends on `H`:
//!
//! | `H` after `exchange` | Next legal call                     |
//! |----------------------|-------------------------------------|
//! | `S<Z>`               | `complete_responder`                |
//! | `S<S<Z>>`            | `close_initiator`                   |
//! | `S<S<S<_>>>`         | `exchange` again, two heights lower |
//!
//! The helper trait [`AfterExchange<H>`] partitions `H` and dispatches to the
//! correct follow-up trait via three non-overlapping blanket impls.

use std::convert::Infallible;

use crate::{
    tree::typed::height::{Height, Pred, Root, S, Z},
    version::Version,
};

use super::message::{self, UnderRoot, UnderUnderRoot};

/// One step in a protocol session.
///
/// Returned by every protocol method that exchanges a wire message. `Continue`
/// advances to `next` after sending `msg`; `Done` terminates the session, but
/// `msg` is still a real message the driver may need to deliver so the
/// counterparty can absorb its `providing` before terminating itself.
#[derive(Debug)]
pub enum Step<Msg, Next, Output> {
    /// Continue the session: send `msg`, then transition to `next`.
    Continue { msg: Msg, next: Next },
    /// Terminate the session. `msg` is the implementation's final outgoing
    /// frame (it may carry non-vacuous `providing` that the peer still needs);
    /// `output` is the implementation's final value (the reconciled root for
    /// `local`, `()` for `remote`).
    Done { msg: Msg, output: Output },
}

/// Any stage in the protocol is identified by this trait. Each stage declares
/// its height, its end-of-protocol output, and the error type its methods may
/// raise. `local::Exchange` sets `Error = Infallible` (a purely in-memory
/// traversal cannot fail); wire-bound implementations (`remote::Exchange`) set
/// `Error` to a concrete type covering I/O and framing failures.
pub trait Stage {
    /// The height in the protocol, starting at the root.
    type Height: Height;

    /// The end result of the protocol.
    type Output;

    /// The error type raised by this stage's protocol methods.
    type Error;
}

pub trait Connect<P, T>: Stage<Height = Root> + Sized
where
    P: Clone + Ord + AsRef<[u8]> + Send + Sync,
    T: Send + Sync,
{
    type Next: CompleteConnect<P, T>
        + Stage<Output = Self::Output, Height = Root, Error = Self::Error>;

    async fn connect(self) -> Result<Step<Version<P>, Self::Next, Infallible>, Self::Error>;
}

pub trait CompleteConnect<P, T>: Stage<Height = Root> + Sized
where
    P: Clone + Ord + AsRef<[u8]> + Send + Sync,
    T: Send + Sync,
{
    type Next: Initiator<P, T>
        + Responder<P, T>
        + Stage<Output = Self::Output, Height = Root, Error = Self::Error>;

    async fn complete_connect(
        self,
        their_version: Version<P>,
    ) -> Result<Step<(), Self::Next, Self::Output>, Self::Error>;
}

pub trait Accept<P, T>: Stage<Height = Root> + Sized
where
    P: Clone + Ord + AsRef<[u8]> + Send + Sync,
    T: Send + Sync,
{
    type Next: Initiator<P, T>
        + Responder<P, T>
        + Stage<Output = Self::Output, Height = Root, Error = Self::Error>;

    async fn accept(
        self,
        their_version: Version<P>,
    ) -> Result<Step<Version<P>, Self::Next, Self::Output>, Self::Error>;
}

/// Continue the protocol as the initiator.
///
/// The trait is implemented by the state type that the constructor produces;
/// `Self::Next == Self` for any straightforward implementation.
pub trait Initiator<P, T>: Stage<Height = Root> + Sized
where
    P: Clone + Ord + AsRef<[u8]> + Send + Sync,
    T: Send + Sync,
{
    /// The state that consumes the responder's [`message::Opening`].
    type Next: OpenInitiator<P, T>
        + Stage<Output = Self::Output, Height = Root, Error = Self::Error>;

    /// Begin the protocol as the initiator.
    ///
    /// Returns the opening [`message::Initiate`] (just our root hash) and an
    /// `Exchange` whose zipper is at `Top` (height `Root`). The initiator's
    /// next call is [`Exchange::open_initiator`], processing the responder's
    /// [`message::Opening`].
    ///
    /// Always yields [`Step::Continue`]: a side opening the protocol cannot
    /// have converged yet. The [`Step::Done`]'s `Output` slot is
    /// [`Infallible`] to encode that impossibility in the type system.
    async fn initiator(
        self,
    ) -> Result<Step<message::Initiate, Self::Next, Infallible>, Self::Error>;
}

/// Continue the protocol as the responder.
///
/// `Err(node)` from this call indicates that the initiator's root hash matched
/// ours: the trees are already equal, the protocol short-circuits, and the
/// caller receives the unchanged root.
pub trait Responder<P, T>: Stage<Height = Root> + Sized
where
    P: Clone + Ord + AsRef<[u8]> + Send + Sync,
    T: Send + Sync,
{
    /// The first steady-state [`Exchange`] from the responder's side.
    type Next: Exchange<P, T>
        + Stage<Output = Self::Output, Height = UnderRoot, Error = Self::Error>;

    /// Begin the protocol as the responder, processing the initiator's
    /// [`message::Initiate`].
    ///
    /// If our root hash matches the initiator's, we short-circuit with
    /// [`Step::Done`] and an empty `Opening`: the trees are already equal.
    /// Otherwise we yield [`Step::Continue`], explode our root one level down
    /// into an [`UnderRoot`]-height zipper, and emit its children's hashes as
    /// the `Opening`'s `uncertain` set -- unconditionally, since we haven't
    /// yet learned what the initiator has.
    async fn responder(
        self,
        request: message::Initiate,
    ) -> Result<Step<message::Opening, Self::Next, Self::Output>, Self::Error>;
}

/// Process the responder's [`message::Opening`].
///
/// Distinct from [`Exchange`] because the opening carries only `uncertain`,
/// and the responder may list children of the initiator's absent root --- a
/// case the steady-state [`Exchange`] is allowed to debug-assert against.
pub trait OpenInitiator<P, T>: Stage<Height = Root> + Sized
where
    P: Clone + Ord + AsRef<[u8]> + Send + Sync,
    T: Send + Sync,
{
    /// The first steady-state [`Exchange`] from the initiator's side.
    type Next: Exchange<P, T>
        + Stage<Output = Self::Output, Height = UnderUnderRoot, Error = Self::Error>;

    /// Process the initiator's first round, applied to the responder's
    /// [`message::Opening`].
    ///
    /// Distinct from [`Self::exchange`] because the opening carries only
    /// `uncertain`, never `providing` or `requested`: the responder enumerates
    /// every child of its root before learning what the initiator has. The
    /// responder may therefore list hashes whose parent (our empty root prefix)
    /// we lack entirely -- a normal case here, but one that would indicate a
    /// protocol bug if it recurred in `Self::exchange`.
    #[allow(clippy::type_complexity)]
    async fn open_initiator(
        self,
        request: message::Opening,
    ) -> Result<Step<message::Exchange<P, T, UnderUnderRoot>, Self::Next, Self::Output>, Self::Error>;
}

/// One steady-state round, as either party.
///
/// The outgoing message's height is `Self::Height − 2` (and the incoming
/// message's is `Self::Height − 1`); both are recovered from `Stage::Height`
/// via `Pred` projections, so each implementing type's exchange height is
/// determined by its `Stage::Height` alone.
pub trait Exchange<P, T>: Stage + Sized
where
    P: Clone + Ord + AsRef<[u8]> + Send + Sync,
    T: Send + Sync,
    Self::Height: Pred,
    <Self::Height as Pred>::Pred: Pred,
    S<<Self::Height as Pred>::Pred>: Height,
    S<<<Self::Height as Pred>::Pred as Pred>::Pred>: Height,
{
    /// Whichever of [`Exchange`], [`CloseInitiator`], or [`CompleteResponder`]
    /// is appropriate at the outgoing message's height. See [`AfterExchange`].
    type Next: AfterExchange<P, T, <<Self::Height as Pred>::Pred as Pred>::Pred>
        + Stage<
            Output = Self::Output,
            Height = <<Self::Height as Pred>::Pred as Pred>::Pred,
            Error = Self::Error,
        >;

    /// Process one round of the protocol's steady state, as either party.
    ///
    /// Each call moves our zipper down by two heights and emits the next
    /// outgoing message. Yields [`Step::Done`] once we have nothing left to
    /// ask about and nothing left in dispute -- but the outgoing message is
    /// emitted unconditionally, because the counterparty may still need its
    /// contents to converge.
    #[allow(clippy::type_complexity)]
    async fn exchange(
        self,
        request: message::Exchange<P, T, <Self::Height as Pred>::Pred>,
    ) -> Result<
        Step<
            message::Exchange<P, T, <<Self::Height as Pred>::Pred as Pred>::Pred>,
            Self::Next,
            Self::Output,
        >,
        Self::Error,
    >;
}

/// The initiator's final sending round; emits [`message::Closing`] instead of
/// the vacuous leaf-height [`message::Exchange`].
pub trait CloseInitiator<P, T>: Stage<Height = S<S<Z>>> + Sized
where
    P: Clone + Ord + AsRef<[u8]> + Send + Sync,
    T: Send + Sync,
{
    /// The terminal initiator state.
    type Next: CompleteInitiator<P, T>
        + Stage<Output = Self::Output, Height = Z, Error = Self::Error>;

    /// The initiator's last sending round, descending the zipper from
    /// `S<S<Z>>` to `Z` and emitting [`message::Closing`].
    ///
    /// Like [`Self::exchange`] internally, but emits `Closing` rather than
    /// `Exchange<_, _, Z>`: the leaf-height `uncertain` that the steady-state
    /// path would produce is structurally vacuous (leaves all hash to the
    /// same all-ones sentinel, so any "Both"-case is necessarily a match),
    /// so we omit it from the wire. This lets [`Self::complete_responder`]
    /// consume `Closing` directly, without a runtime check that a
    /// well-behaved peer would never trip.
    #[allow(clippy::type_complexity)]
    async fn close_initiator(
        self,
        request: message::Exchange<P, T, S<Z>>,
    ) -> Result<Step<message::Closing<P, T>, Self::Next, Self::Output>, Self::Error>;
}

/// The responder's terminal round; absorbs the initiator's [`message::Closing`]
/// and emits [`message::Complete`].
pub trait CompleteResponder<P, T>: Stage<Height = S<Z>> + Sized
where
    P: Clone + Ord + AsRef<[u8]> + Send + Sync,
    T: Send + Sync,
{
    /// The responder's final round, processing the initiator's
    /// [`message::Closing`].
    ///
    /// We absorb the initiator's last batch of nodes, answer any final
    /// `requested` set, and collapse our zipper back to a root. The returned
    /// [`message::Complete`] carries our last outgoing `providing` for the
    /// initiator to absorb in [`Self::complete_initiator`].
    ///
    /// Always yields [`Step::Done`]: the responder's session ends here. The
    /// `Next` slot is [`Infallible`] to encode the impossibility of
    /// `Continue` in the type system.
    #[allow(clippy::type_complexity)]
    async fn complete_responder(
        self,
        request: message::Closing<P, T>,
    ) -> Result<Step<message::Complete<P, T>, Infallible, Self::Output>, Self::Error>;
}

/// The initiator's terminal round; absorbs the responder's [`message::Complete`].
pub trait CompleteInitiator<P, T>: Stage<Height = Z> + Sized
where
    P: Clone + Ord + AsRef<[u8]> + Send + Sync,
    T: Send + Sync,
{
    /// The initiator's final round.
    ///
    /// Absorbs the responder's last batch of `providing` (from
    /// [`message::Complete`]) and collapses our zipper back to a root. There is
    /// no outgoing message: any `requested` we would have made went out in our
    /// prior [`Self::close_initiator`] call.
    #[allow(clippy::type_complexity)]
    async fn complete_initiator(
        self,
        request: message::Complete<P, T>,
    ) -> Result<Step<(), Infallible, Self::Output>, Self::Error>;
}

/// Blanket marker trait keyed by the height `H` just produced by an
/// [`Exchange::exchange`] call. A state type satisfying `AfterExchange<H>` is
/// "the right kind of state to follow an exchange that ended at height `H`":
///
/// - `H = S<Z>`: must impl [`CompleteResponder`].
/// - `H = S<S<Z>>`: must impl [`CloseInitiator`].
/// - `H = S<S<S<_>>>`: must impl [`Exchange`] at two heights finer.
///
/// Heights `S<Z>` and `S<S<Z>>` are handled via the blanket impls below,
/// keyed off the appropriate terminal trait.
///
/// Height `Z` is never reached as the result of an exchange (the leaf-height
/// uncertain map would be vacuous), so there is no `AfterExchange<Z>` impl.
pub trait AfterExchange<P, T, H>: Sized
where
    P: Clone + Ord + AsRef<[u8]> + Send + Sync,
    T: Send + Sync,
    H: Height,
{
}

impl<P, T, X> AfterExchange<P, T, S<Z>> for X
where
    P: Clone + Ord + AsRef<[u8]> + Send + Sync,
    T: Send + Sync,
    X: CompleteResponder<P, T>,
{
}

impl<P, T, X> AfterExchange<P, T, S<S<Z>>> for X
where
    P: Clone + Ord + AsRef<[u8]> + Send + Sync,
    T: Send + Sync,
    X: CloseInitiator<P, T>,
{
}

impl<P, T, H, X> AfterExchange<P, T, S<S<S<H>>>> for X
where
    P: Clone + Ord + AsRef<[u8]> + Send + Sync,
    T: Send + Sync,
    H: Height,
    S<H>: Height,
    S<S<H>>: Height,
    S<S<S<H>>>: Height,
    X: Exchange<P, T> + Stage<Height = S<S<S<H>>>>,
{
}

/// Declare the [`Peer`] trait and its blanket impl with the long `::Next`
/// chains spelled out as nested associated-type bounds in supertrait
/// position. The macro emits the chains tt-munched ahead of time: the
/// initiator side wraps `$init_terminal` in N `Exchange<P, T, Next: …>`
/// layers (where N is the count of `_` tokens in `init: […]`), and the
/// responder side does the same for `$resp_terminal`.
///
/// Plain `where`-clauses on a trait definition don't propagate to callers,
/// but `Trait<AssocType: Bound>` in supertrait position does --- which is
/// why we go through the trouble of expressing the entire chain at the
/// supertrait level rather than as a `where` predicate.
macro_rules! define_peer {
    (
        init: [$($init_count:tt)*],
        resp: [$($resp_count:tt)*],
        $(,)?
    ) => {
        define_peer!(@step
            init: [$($init_count)*],
            resp: [$($resp_count)*],
            init_chain: (CloseInitiator<P, T>),
            resp_chain: (CompleteResponder<P, T>),
        );
    };

    // Wrap one `Exchange<P, T, Next: …>` around the init-chain accumulator
    // until the init-side counter is exhausted.
    (@step
        init: [_ $($init_rest:tt)*],
        resp: [$($resp_count:tt)*],
        init_chain: ($($init_chain:tt)*),
        resp_chain: ($($resp_chain:tt)*) $(,)?
    ) => {
        define_peer!(@step
            init: [$($init_rest)*],
            resp: [$($resp_count)*],
            init_chain: (Exchange<P, T, Next: $($init_chain)*>),
            resp_chain: ($($resp_chain)*),
        );
    };

    // Init side done; munch the resp-side counter the same way.
    (@step
        init: [],
        resp: [_ $($resp_rest:tt)*],
        init_chain: ($($init_chain:tt)*),
        resp_chain: ($($resp_chain:tt)*) $(,)?
    ) => {
        define_peer!(@step
            init: [],
            resp: [$($resp_rest)*],
            init_chain: ($($init_chain)*),
            resp_chain: (Exchange<P, T, Next: $($resp_chain)*>),
        );
    };

    // Both counters exhausted: emit the trait and blanket impl with the
    // fully-built chain expressions inlined.
    (@step
        init: [],
        resp: [],
        init_chain: ($($init_chain:tt)*),
        resp_chain: ($($resp_chain:tt)*) $(,)?
    ) => {
        /// A type that can play either side of the mirror protocol: it
        /// implements both [`Initiator`] and [`Responder`] at the root, and
        /// in either role the entire chain of `::Next` projections that the
        /// drivers [`super::initiator`] / [`super::responder`] walk
        /// implements the right protocol trait at every height.
        ///
        /// Both `local::Exchange` and `remote::Exchange` pick this up for
        /// free via the blanket impl below; downstream call sites take a
        /// single `Peer<P, T>` bound on each argument and the chain bounds
        /// propagate.
        pub trait Peer<P, T>:
            Initiator<P, T, Next: OpenInitiator<P, T, Next: $($init_chain)*>> + Responder<P, T, Next: $($resp_chain)*>
        where
            P: Clone + Ord + AsRef<[u8]> + Send + Sync,
            T: Send + Sync,
        {
        }

        impl<X, P, T> Peer<P, T> for X
        where
            P: Clone + Ord + AsRef<[u8]> + Send + Sync,
            T: Send + Sync,
            X: Initiator<P, T, Next: OpenInitiator<P, T, Next: $($init_chain)*>> + Responder<P, T, Next: $($resp_chain)*>
        {
        }

        pub trait Server<P, T>:
            Accept<P, T, Next: Initiator<P, T, Next: OpenInitiator<P, T, Next: $($init_chain)*>> + Responder<P, T, Next: $($resp_chain)*>>
        where
            P: Clone + Ord + AsRef<[u8]> + Send + Sync,
            T: Send + Sync,
        {
        }

        impl<X, P, T> Server<P, T> for X
        where
            P: Clone + Ord + AsRef<[u8]> + Send + Sync,
            T: Send + Sync,
            X: Accept<P, T, Next: Initiator<P, T, Next: OpenInitiator<P, T, Next: $($init_chain)*>> + Responder<P, T, Next: $($resp_chain)*>>
        {
        }

        pub trait Client<P, T>:
            Connect<P, T, Next: CompleteConnect<P, T, Next: Initiator<P, T, Next: OpenInitiator<P, T, Next: $($init_chain)*>> + Responder<P, T, Next: $($resp_chain)*>>>
        where
            P: Clone + Ord + AsRef<[u8]> + Send + Sync,
            T: Send + Sync,
        {
        }

        impl<X, P, T> Client<P, T> for X
        where
            P: Clone + Ord + AsRef<[u8]> + Send + Sync,
            T: Send + Sync,
            X: Connect<P, T, Next: CompleteConnect<P, T, Next: Initiator<P, T, Next: OpenInitiator<P, T, Next: $($init_chain)*>> + Responder<P, T, Next: $($resp_chain)*>>>
        {
        }

    };
}

// The initiator chain visits 14 `Exchange` levels (heights `S^30 → S^28 →
// … → S^4`) before terminating in `CloseInitiator` at `S<S<Z>>`. The
// terminal `CompleteInitiator` after `CloseInitiator` is already implied
// by `CloseInitiator::Next`'s own trait bound, so it doesn't need
// re-stating here.
//
// The responder chain visits 15 `Exchange` levels (heights `S^31 → S^29
// → … → S^3`) before terminating in `CompleteResponder` at `S<Z>`.
define_peer! {
    init: [_ _ _ _ _ _ _ _ _ _ _ _ _ _],
    resp: [_ _ _ _ _ _ _ _ _ _ _ _ _ _ _],
}
