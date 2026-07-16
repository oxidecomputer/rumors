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
//! | [`Connect`]           | --                              | [`message::Handshake`]            | [`CompleteConnect`]                   |
//! | [`CompleteConnect`]   | [`Version`]                     | --                                | [`Initiator`] or [`Responder`]        |
//! | [`Accept`]            | [`message::Handshake`]          | [`message::Handshake`]            | [`Initiator`] or [`Responder`]        |
//! | [`Initiator`]         | --                              | [`message::Initiate`]             | [`OpenInitiator`]                     |
//! | [`Responder`]         | [`message::Initiate`]           | [`message::Opening`]              | [`Exchange`] (first steady round)     |
//! | [`OpenInitiator`]     | [`message::Opening`]            | `message::Exchange<_, U²>`        | [`Exchange`] (first steady round)     |
//! | [`Exchange`]          | [`message::Exchange`]           | [`message::Exchange`]             | [`AfterExchange<H>`] (see below)      |
//! | [`CloseResponder`]    | `message::Exchange<_, Z>`       | [`message::Closing`]              | [`CompleteResponder`]                 |
//! | [`CompleteInitiator`] | [`message::Closing`]            | [`message::Complete`]             | terminal                              |
//! | [`CompleteResponder`] | [`message::Complete`]           | --                                | terminal                              |
//!
//! # The `Exchange<H>::Next` ambiguity
//!
//! After an [`Exchange::exchange`] call at height `H`, the *next* legal call
//! depends on `H`:
//!
//! | `H` after `exchange` | Next legal call                     |
//! |----------------------|-------------------------------------|
//! | `Z`                  | `complete_initiator`                |
//! | `S<Z>`               | `close_responder`                   |
//! | `S<S<Z>>` and below  | `exchange` again, two heights lower |
//!
//! The helper trait [`AfterExchange<H>`] partitions `H` and dispatches to the
//! correct follow-up trait via three non-overlapping blanket impls.

use std::{convert::Infallible, future::Future};

use crate::{
    Version,
    tree::typed::height::{Height, Pred, Root, S, UnderRoot, UnderUnderRoot, Z},
};

use super::message;

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
    /// Terminate the session.
    ///
    /// `msg` is the implementation's final outgoing frame (it may carry
    /// non-vacuous `providing` that the peer still needs); `output` is the
    /// implementation's final value (the reconciled root for `local`, `()` for
    /// `remote`).
    Done { msg: Msg, output: Output },
}

/// Any stage in the protocol is identified by this trait.
///
/// Each stage declares its height, its end-of-protocol output, and the error type
/// its methods may raise. `local::Exchange` sets `Error = Infallible` (a purely
/// in-memory traversal cannot fail); wire-bound implementations (`remote::Exchange`)
/// set `Error` to a concrete type covering I/O and framing failures.
pub trait Stage: Send {
    /// The height in the protocol, starting at the root.
    type Height: Height;

    /// The end result of the protocol.
    type Output: Send;

    /// The error type raised by this stage's protocol methods.
    type Error: Send;
}

/// Open the connect phase on the client side: emit our [`message::Handshake`]
/// greeting. Always continues (the `Done` slot is [`Infallible`]): a side
/// cannot know it has converged before hearing the peer's version.
pub trait Connect<T>: Stage<Height = Root> + Sized
where
    T: Send + Sync,
{
    /// The state that absorbs the peer's version ([`CompleteConnect`]).
    type Next: CompleteConnect<T> + Stage<Output = Self::Output, Height = Root, Error = Self::Error>;

    fn connect(
        self,
    ) -> impl Future<Output = Result<Step<message::Handshake, Self::Next, Infallible>, Self::Error>> + Send;
}

/// Finish the connect phase on the client side: absorb the peer's version.
///
/// `Done` here means the versions were equal — already converged, no
/// descent; `Continue` hands over a state ready to play either role
/// (`Next` is both [`Initiator`] and [`Responder`]; the byte tiebreak picks
/// which, see [`descend`](super::descend)).
pub trait CompleteConnect<T>: Stage<Height = Root> + Sized
where
    T: Send + Sync,
{
    /// The connected state, able to play either descent role.
    type Next: Initiator<T>
        + Responder<T>
        + Stage<Output = Self::Output, Height = Root, Error = Self::Error>;

    fn complete_connect(
        self,
        their_version: Version,
    ) -> impl Future<Output = Result<Step<(), Self::Next, Self::Output>, Self::Error>> + Send;
}

/// The connect phase on the server side: ship the client's greeting to the
/// peer and reply with the peer's own [`message::Handshake`].
///
/// `Done` mirrors [`CompleteConnect`]'s convergence case; the two sides always
/// agree on it (both compare the same pair of versions).
pub trait Accept<T>: Stage<Height = Root> + Sized
where
    T: Send + Sync,
{
    /// The connected state, able to play either descent role.
    type Next: Initiator<T>
        + Responder<T>
        + Stage<Output = Self::Output, Height = Root, Error = Self::Error>;

    fn accept(
        self,
        request: message::Handshake,
    ) -> impl Future<
        Output = Result<Step<message::Handshake, Self::Next, Self::Output>, Self::Error>,
    > + Send;
}

/// Continue the protocol as the initiator.
///
/// The trait is implemented by the state type that the constructor produces;
/// `Self::Next == Self` for any straightforward implementation.
pub trait Initiator<T>: Stage<Height = Root> + Sized
where
    T: Send + Sync,
{
    /// The state that consumes the responder's [`message::Opening`].
    type Next: OpenInitiator<T> + Stage<Output = Self::Output, Height = Root, Error = Self::Error>;

    /// Begin the protocol as the initiator.
    ///
    /// Returns the opening [`message::Initiate`] (just our root hash) and an
    /// `Exchange` whose zipper is at `Top` (height `Root`). The initiator's
    /// next call is [`OpenInitiator::open_initiator`], processing the responder's
    /// [`message::Opening`].
    ///
    /// Always yields [`Step::Continue`]: a side opening the protocol cannot
    /// have converged yet. The [`Step::Done`]'s `Output` slot is
    /// [`Infallible`] to encode that impossibility in the type system.
    fn initiator(
        self,
    ) -> impl Future<Output = Result<Step<message::Initiate, Self::Next, Infallible>, Self::Error>> + Send;
}

/// Continue the protocol as the responder.
pub trait Responder<T>: Stage<Height = Root> + Sized
where
    T: Send + Sync,
{
    /// The first steady-state [`Exchange`] from the responder's side.
    type Next: Exchange<T> + Stage<Output = Self::Output, Height = UnderRoot, Error = Self::Error>;

    /// Begin the protocol as the responder, processing the initiator's
    /// [`message::Initiate`].
    ///
    /// Yields [`Step::Continue`]: the responder explodes its root one level
    /// down into an [`UnderRoot`]-height zipper and emits its children's
    /// hashes as the `Opening`'s `uncertain` set, unconditionally, since it
    /// has not yet learned what the initiator holds. (Equal versions end the
    /// session in the connect phase, before this step; an empty `Opening` is
    /// how an empty responder asks the initiator to provide everything.)
    fn responder(
        self,
        request: message::Initiate,
    ) -> impl Future<Output = Result<Step<message::Opening, Self::Next, Self::Output>, Self::Error>> + Send;
}

/// Process the responder's [`message::Opening`].
///
/// Distinct from [`Exchange`] because the opening carries only `uncertain`,
/// and the responder may list children of the initiator's absent root, a
/// case the steady-state [`Exchange`] is allowed to debug-assert against.
pub trait OpenInitiator<T>: Stage<Height = Root> + Sized
where
    T: Send + Sync,
{
    /// The first steady-state [`Exchange`] from the initiator's side.
    type Next: Exchange<T>
        + Stage<Output = Self::Output, Height = UnderUnderRoot, Error = Self::Error>;

    /// Process the initiator's first round, applied to the responder's
    /// [`message::Opening`].
    ///
    /// Distinct from [`Exchange::exchange`] because the opening carries only
    /// `uncertain`, never `providing` or `requested`: the responder
    /// enumerates every child of its root before learning what the initiator
    /// has. The responder may therefore list hashes whose parent (our empty
    /// root prefix) we lack entirely, a normal case here, but one that would
    /// indicate a protocol bug if it recurred in `Exchange::exchange`.
    #[allow(clippy::type_complexity)]
    fn open_initiator(
        self,
        request: message::Opening,
    ) -> impl Future<
        Output = Result<
            Step<message::Exchange<T, UnderUnderRoot>, Self::Next, Self::Output>,
            Self::Error,
        >,
    > + Send;
}

/// One steady-state round, as either party.
///
/// The outgoing message's height is `Self::Height − 2` (and the incoming
/// message's is `Self::Height − 1`); both are recovered from `Stage::Height`
/// via `Pred` projections, so each implementing type's exchange height is
/// determined by its `Stage::Height` alone.
pub trait Exchange<T>: Stage + Sized
where
    T: Send + Sync,
    Self::Height: Pred,
    <Self::Height as Pred>::Pred: Pred,
    S<<Self::Height as Pred>::Pred>: Height,
    S<<<Self::Height as Pred>::Pred as Pred>::Pred>: Height,
{
    /// Whichever of [`Exchange`], [`CloseResponder`], or [`CompleteInitiator`]
    /// is appropriate at the outgoing message's height. See [`AfterExchange`].
    type Next: AfterExchange<T, <<Self::Height as Pred>::Pred as Pred>::Pred>
        + Stage<
            Output = Self::Output,
            Height = <<Self::Height as Pred>::Pred as Pred>::Pred,
            Error = Self::Error,
        >;

    /// Process one round of the protocol's steady state, as either party.
    ///
    /// Each call moves our zipper down by two heights and emits the next
    /// outgoing message. Yields [`Step::Done`] once we have nothing left to
    /// ask about and nothing left in dispute; the outgoing message is still
    /// emitted unconditionally, because the counterparty may need its
    /// contents to converge.
    #[allow(clippy::type_complexity)]
    fn exchange(
        self,
        request: message::Exchange<T, <Self::Height as Pred>::Pred>,
    ) -> impl Future<
        Output = Result<
            Step<
                message::Exchange<T, <<Self::Height as Pred>::Pred as Pred>::Pred>,
                Self::Next,
                Self::Output,
            >,
            Self::Error,
        >,
    > + Send;
}

/// The responder's closing round; consumes the initiator's final leaf-height
/// [`message::Exchange`] and emits [`message::Closing`].
///
/// The incoming `uncertain` is the initiator's leaf listing under each
/// still-disputed leaf-parent. The response answers it through the ordinary
/// asymmetry matrix minus its dispute cell — vacuous at leaf height, since
/// two leaves at one path are the same leaf — so the reply carries only
/// `providing` and `requested`, which is exactly what [`message::Closing`]
/// encodes.
pub trait CloseResponder<T>: Stage<Height = S<Z>> + Sized
where
    T: Send + Sync,
{
    /// The terminal responder state.
    type Next: CompleteResponder<T> + Stage<Output = Self::Output, Height = Z, Error = Self::Error>;

    /// The responder's closing round, descending the zipper from `S<Z>` to
    /// `Z` and emitting [`message::Closing`].
    ///
    /// Yields [`Step::Done`] when the response requests nothing: the
    /// counterparty's [`message::Complete`] would carry nothing, so neither
    /// side sends again.
    #[allow(clippy::type_complexity)]
    fn close_responder(
        self,
        request: message::Exchange<T, Z>,
    ) -> impl Future<
        Output = Result<Step<message::Closing<T>, Self::Next, Self::Output>, Self::Error>,
    > + Send;
}

/// The initiator's terminal round; absorbs the responder's
/// [`message::Closing`] and answers it with [`message::Complete`].
pub trait CompleteInitiator<T>: Stage<Height = Z> + Sized
where
    T: Send + Sync,
{
    /// The initiator's final round.
    ///
    /// Absorbs the responder's last batch of `providing`, answers its final
    /// leaf-height `requested` — pruning against the responder's version, so
    /// a leaf the responder deleted drops here instead of shipping — and
    /// collapses our zipper back to a root.
    ///
    /// Always yields [`Step::Done`]: the initiator's session ends here. The
    /// `Next` slot is [`Infallible`] to encode the impossibility of
    /// `Continue` in the type system.
    #[allow(clippy::type_complexity)]
    fn complete_initiator(
        self,
        request: message::Closing<T>,
    ) -> impl Future<
        Output = Result<Step<message::Complete<T>, Infallible, Self::Output>, Self::Error>,
    > + Send;
}

/// The responder's terminal round; absorbs the initiator's
/// [`message::Complete`].
pub trait CompleteResponder<T>: Stage<Height = Z> + Sized
where
    T: Send + Sync,
{
    /// The responder's final round.
    ///
    /// Absorbs the initiator's last batch of `providing` (from
    /// [`message::Complete`]) and collapses our zipper back to a root. There
    /// is no outgoing message: any `requested` we would have made went out in
    /// our prior [`CloseResponder::close_responder`] call.
    #[allow(clippy::type_complexity)]
    fn complete_responder(
        self,
        request: message::Complete<T>,
    ) -> impl Future<Output = Result<Step<(), Infallible, Self::Output>, Self::Error>> + Send;
}

/// Blanket marker trait keyed by the height `H` just produced by an
/// [`Exchange::exchange`] call. A state type satisfying `AfterExchange<H>` is
/// "the right kind of state to follow an exchange that ended at height `H`":
///
/// - `H = Z`: must impl [`CompleteInitiator`] — the exchange that produced a
///   leaf-height message was the initiator's last, and the responder's
///   [`message::Closing`] answers it.
/// - `H = S<Z>`: must impl [`CloseResponder`].
/// - `H = S<S<Z>>` or `S<S<S<_>>>`: must impl [`Exchange`] at two heights
///   finer.
///
/// Heights `Z` and `S<Z>` are handled via the blanket impls below, keyed off
/// the appropriate terminal trait. `S<S<Z>>` needs its own blanket only
/// because it does not unify with the `S<S<S<H>>>` pattern.
pub trait AfterExchange<T, H>: Sized
where
    T: Send + Sync,
    H: Height,
{
}

impl<T, X> AfterExchange<T, Z> for X
where
    T: Send + Sync,
    X: CompleteInitiator<T>,
{
}

impl<T, X> AfterExchange<T, S<Z>> for X
where
    T: Send + Sync,
    X: CloseResponder<T>,
{
}

impl<T, X> AfterExchange<T, S<S<Z>>> for X
where
    T: Send + Sync,
    X: Exchange<T> + Stage<Height = S<S<Z>>>,
{
}

impl<T, H, X> AfterExchange<T, S<S<S<H>>>> for X
where
    T: Send + Sync,
    H: Height,
    S<H>: Height,
    S<S<H>>: Height,
    S<S<S<H>>>: Height,
    X: Exchange<T> + Stage<Height = S<S<S<H>>>>,
{
}

/// Declare the [`Peer`] trait and its blanket impl with the long `::Next`
/// chains spelled out as nested associated-type bounds in supertrait
/// position.
///
/// The macro emits the chains tt-munched ahead of time: the
/// initiator side wraps `$init_terminal` in N `Exchange<T, Next: …>`
/// layers (where N is the count of `_` tokens in `init: […]`), and the
/// responder side does the same for `$resp_terminal`.
///
/// Plain `where`-clauses on a trait definition don't propagate to callers,
/// but `Trait<AssocType: Bound>` in supertrait position does, so the entire
/// chain is expressed at the supertrait level rather than as a `where`
/// predicate.
macro_rules! define_peer {
    (
        init: [$($init_count:tt)*],
        resp: [$($resp_count:tt)*],
        $(,)?
    ) => {
        define_peer!(@step
            init: [$($init_count)*],
            resp: [$($resp_count)*],
            init_chain: (CompleteInitiator<T>),
            resp_chain: (CloseResponder<T>),
        );
    };

    // Wrap one `Exchange<T, Next: …>` around the init-chain accumulator
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
            init_chain: (Exchange<T, Next: $($init_chain)*>),
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
            resp_chain: (Exchange<T, Next: $($resp_chain)*>),
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
        /// A type that can play either side of the mirror protocol.
        ///
        /// It implements both [`Initiator`] and [`Responder`] at the root, and
        /// in either role the entire chain of `::Next` projections the
        /// session driver walks implements the right protocol trait at
        /// every height.
        ///
        /// Both `local::Exchange` and `remote::Exchange` pick this up for
        /// free via the blanket impl below; downstream call sites take a
        /// single `Peer<T>` bound on each argument and the chain bounds
        /// propagate.
        pub trait Peer<T>:
            Initiator<T, Next: OpenInitiator<T, Next: $($init_chain)*>> + Responder<T, Next: $($resp_chain)*>
        where
            T: Send + Sync,
        {
        }

        impl<X, T> Peer<T> for X
        where
            T: Send + Sync,
            X: Initiator<T, Next: OpenInitiator<T, Next: $($init_chain)*>> + Responder<T, Next: $($resp_chain)*>
        {
        }

        /// A [`Peer`] entered through the server side of the connect phase:
        /// [`Accept`] first, then either descent role. The whole-session
        /// bound the wire-facing driver takes for the remote party.
        pub trait Server<T>:
            Accept<T, Next: Initiator<T, Next: OpenInitiator<T, Next: $($init_chain)*>> + Responder<T, Next: $($resp_chain)*>>
        where
            T: Send + Sync,
        {
        }

        impl<X, T> Server<T> for X
        where
            T: Send + Sync,
            X: Accept<T, Next: Initiator<T, Next: OpenInitiator<T, Next: $($init_chain)*>> + Responder<T, Next: $($resp_chain)*>>
        {
        }

        /// A [`Peer`] entered through the client side of the connect phase:
        /// [`Connect`] then [`CompleteConnect`], then either descent role.
        /// The whole-session bound the drivers take for the local party.
        pub trait Client<T>:
            Connect<T, Next: CompleteConnect<T, Next: Initiator<T, Next: OpenInitiator<T, Next: $($init_chain)*>> + Responder<T, Next: $($resp_chain)*>>>
        where
            T: Send + Sync,
        {
        }

        impl<X, T> Client<T> for X
        where
            T: Send + Sync,
            X: Connect<T, Next: CompleteConnect<T, Next: Initiator<T, Next: OpenInitiator<T, Next: $($init_chain)*>> + Responder<T, Next: $($resp_chain)*>>>
        {
        }

    };
}

// The initiator chain visits 15 `Exchange` levels (heights `S^30 → S^28 →
// … → S<S<Z>>`) before terminating in `CompleteInitiator` at `Z`.
//
// The responder chain visits 15 `Exchange` levels (heights `S^31 → S^29
// → … → S^3`) before closing in `CloseResponder` at `S<Z>`. The terminal
// `CompleteResponder` after `CloseResponder` is already implied by
// `CloseResponder::Next`'s own trait bound, so it doesn't need re-stating
// here.
define_peer! {
    init: [_ _ _ _ _ _ _ _ _ _ _ _ _ _ _],
    resp: [_ _ _ _ _ _ _ _ _ _ _ _ _ _ _],
}
