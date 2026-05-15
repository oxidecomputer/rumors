//! Trait-family abstraction of the mirror protocol.
//!
//! Each public step of the [`super::local`] protocol is named by a trait.
//! Each trait carries a `Next` associated type bounded by the trait that
//! describes the only legal *following* call — so the protocol's allowed
//! transitions are encoded at the type level and the chain of calls is
//! statically forced into the right order.
//!
//! The traits are deliberately silent about how a step is implemented:
//! [`super::local::Exchange`] fulfills them by delegating to its existing
//! inherent methods, but a remote proxy that forwards each call over the wire
//! (and carries only a phantom `Height`) can fulfill them just as easily.
//!
//! # Trait family
//!
//! | Trait               | Wire input               | Wire output                       | `Next`                                |
//! |---------------------|--------------------------|-----------------------------------|---------------------------------------|
//! | [`Initiator`]       | --                       | [`message::Initiate`]             | [`OpenInitiator`]                     |
//! | [`Responder`]       | [`message::Initiate`]    | [`message::Opening`]              | [`Exchange`] (first steady round)     |
//! | [`OpenInitiator`]   | [`message::Opening`]     | [`message::Exchange<_, _, U^2>`]  | [`Exchange`] (first steady round)     |
//! | [`Exchange`]        | [`message::Exchange`]    | [`message::Exchange`]             | [`AfterExchange<H>`] (see below)      |
//! | [`CloseInitiator`]  | [`message::Exchange<_,_,S<Z>>`] | [`message::Closing`]       | [`CompleteInitiator`]                 |
//! | [`CompleteResponder`] | [`message::Closing`]   | [`message::Complete`]             | terminal                              |
//! | [`CompleteInitiator`] | [`message::Complete`]  | --                                | terminal                              |
//!
//! # The `Exchange<H>::Next` ambiguity
//!
//! After an [`Exchange::exchange`] call at height `H`, the *next* legal call
//! depends on `H`:
//!
//! | `H` after `exchange` | Next legal call    |
//! |----------------------|--------------------|
//! | `S<Z>`               | `complete_responder` |
//! | `S<S<Z>>`            | `close_initiator`  |
//! | `S<S<S<_>>>`         | `exchange` again, two heights finer |
//!
//! The helper trait [`AfterExchange<H>`] partitions `H` and dispatches to the
//! correct follow-up trait via three non-overlapping blanket impls; a single
//! bound `Next: AfterExchange<Self::H>` on [`Exchange`] then expresses the
//! tight chain without conditional `where` clauses.

use std::convert::Infallible;

use crate::{
    Key, Message, Version,
    tree::typed::{
        Node,
        height::{Height, Pred, Root, S, Z},
    },
};

use super::message::{self, UnderRoot, UnderUnderRoot};

/// The height of the next [`Exchange::H`] reached after the responder's first
/// [`Responder::responder`] call: two heights below [`UnderRoot`].
pub type AfterResponderH = <<UnderRoot as Pred>::Pred as Pred>::Pred;

/// The height of the next [`Exchange::H`] reached after the initiator's
/// [`OpenInitiator::open_initiator`] call: two heights below [`UnderUnderRoot`].
pub type AfterOpenInitiatorH = <<UnderUnderRoot as Pred>::Pred as Pred>::Pred;

/// Any stage in the protocol is identified by this trait, and must declare its
/// height as an associated type.
pub trait Stage {
    /// The height in the protocol, starting at the root.
    type Height: Height;
}

/// Start the protocol as the initiator.
///
/// The trait is implemented by the state type that the constructor produces;
/// `Self::Next == Self` for any straightforward implementation.
pub trait Initiator<'v, P, T, OnRecv, OnSend>: Stage<Height = Root> + Sized
where
    P: Clone + Ord + AsRef<[u8]>,
    T: Clone,
    OnRecv: FnMut(&Version<P>, Key, &Message<T>),
    OnSend: FnMut(&Version<P>, Key, &Message<T>),
{
    /// The state that consumes the responder's [`message::Opening`].
    type Next: OpenInitiator<'v, P, T, OnRecv, OnSend>;

    /// Begin the protocol as the initiator.
    ///
    /// Returns the opening [`message::Initiate`] (just our root hash) and an
    /// `Exchange` whose zipper is at `Top` (height `Root`). The initiator's
    /// next call is [`Exchange::open_initiator`], processing the responder's
    /// [`message::Opening`].
    fn initiator(
        node: Option<Node<P, T, Root>>,
        their_version: &'v Version<P>,
        on_recv: OnRecv,
        on_send: OnSend,
    ) -> (message::Initiate, Self::Next);
}

/// Start the protocol as the responder.
///
/// `Err(node)` from this call indicates that the initiator's root hash matched
/// ours: the trees are already equal, the protocol short-circuits, and the
/// caller receives the unchanged root.
pub trait Responder<'v, P, T, OnRecv, OnSend>: Stage<Height = UnderRoot> + Sized
where
    P: Clone + Ord + AsRef<[u8]>,
    T: Clone,
    OnRecv: FnMut(&Version<P>, Key, &Message<T>),
    OnSend: FnMut(&Version<P>, Key, &Message<T>),
{
    /// The first steady-state [`Exchange`] from the responder's side.
    type Next: Exchange<'v, P, T, AfterResponderH, OnRecv, OnSend>;

    /// Begin the protocol as the responder, processing the initiator's
    /// [`message::Initiate`].
    ///
    /// If our root hash matches the initiator's, we short-circuit: the trees
    /// are already equal, so we return `Err(our_root)` and an empty `Opening`
    /// to signal completion. Otherwise we explode our root one level down into
    /// an [`UnderRoot`]-height zipper and emit its children's hashes as the
    /// `Opening`'s `uncertain` set -- unconditionally, since we haven't yet
    /// learned what the initiator has.
    fn responder(
        node: Option<Node<P, T, Root>>,
        their_version: &'v Version<P>,
        on_recv: OnRecv,
        on_send: OnSend,
        request: message::Initiate,
    ) -> (
        message::Opening,
        Result<Self::Next, Option<Node<P, T, Root>>>,
    );
}

/// Process the responder's [`message::Opening`].
///
/// Distinct from [`Exchange`] because the opening carries only `uncertain`,
/// and the responder may list children of the initiator's absent root --- a
/// case the steady-state [`Exchange`] is allowed to debug-assert against.
pub trait OpenInitiator<'v, P, T, OnRecv, OnSend>: Stage<Height = Root> + Sized
where
    P: Clone + Ord + AsRef<[u8]>,
    T: Clone,
    OnRecv: FnMut(&Version<P>, Key, &Message<T>),
    OnSend: FnMut(&Version<P>, Key, &Message<T>),
{
    /// The first steady-state [`Exchange`] from the initiator's side.
    type Next: Exchange<'v, P, T, AfterOpenInitiatorH, OnRecv, OnSend>;

    /// Process the initiator's first round, applied to the responder's
    /// [`message::Opening`].
    ///
    /// Distinct from [`Self::exchange`] because the opening carries only
    /// `uncertain`, never `providing` or `requested`: the responder enumerates
    /// every child of its root before learning what the initiator has. The
    /// responder may therefore list hashes whose parent (our empty root prefix)
    /// we lack entirely -- a normal case here, but one that would indicate a
    /// protocol bug if it recurred in `Self::exchange`.
    fn open_initiator(
        self,
        request: message::Opening,
    ) -> (
        message::Exchange<P, T, UnderUnderRoot>,
        Result<Self::Next, Option<Node<P, T, Root>>>,
    );
}

/// One steady-state round, as either party.
///
/// `H` is the height of the outgoing message's `uncertain` map (one less than
/// the incoming message's). It is an associated type rather than a trait
/// parameter because a given state type implements [`Exchange`] at exactly one
/// `H`, and the chain bound [`Self::Next`]`: `[`AfterExchange`]`<Self::H>`
/// requires `H` to be referenceable on the trait body.
pub trait Exchange<'v, P, T, H, OnRecv, OnSend>: Stage<Height = S<S<H>>> + Sized
where
    P: Clone + Ord + AsRef<[u8]>,
    T: Clone,
    H: Height,
    OnRecv: FnMut(&Version<P>, Key, &Message<T>),
    OnSend: FnMut(&Version<P>, Key, &Message<T>),
{
    /// Whichever of [`Exchange`], [`CloseInitiator`], or [`CompleteResponder`]
    /// is appropriate at this height. See [`AfterExchange`].
    type Next: AfterExchange<'v, P, T, OnRecv, OnSend, H>;

    /// Process one round of the protocol's steady state, as either party.
    ///
    /// Each call moves our zipper down by two heights and emits the next
    /// outgoing message. The returned `Result` is `Err(final_tree)` once we
    /// have nothing left to ask about and nothing left in dispute -- but the
    /// outgoing message is sent unconditionally, because the counterparty may
    /// still need its contents to converge.
    fn exchange(
        self,
        request: message::Exchange<P, T, S<H>>,
    ) -> (
        message::Exchange<P, T, H>,
        Result<Self::Next, Option<Node<P, T, Root>>>,
    )
    where
        H: Height,
        S<H>: Height,
        S<S<H>>: Height;
}

/// The initiator's final sending round; emits [`message::Closing`] instead of
/// the vacuous leaf-height [`message::Exchange`].
pub trait CloseInitiator<'v, P, T, OnRecv, OnSend>: Stage<Height = S<S<Z>>> + Sized
where
    P: Clone + Ord + AsRef<[u8]>,
    T: Clone,
    OnRecv: FnMut(&Version<P>, Key, &Message<T>),
    OnSend: FnMut(&Version<P>, Key, &Message<T>),
{
    /// The terminal initiator state.
    type Next: CompleteInitiator<P, T, OnRecv>;

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
    fn close_initiator(
        self,
        request: message::Exchange<P, T, S<Z>>,
    ) -> (
        message::Closing<P, T>,
        Result<Self::Next, Option<Node<P, T, Root>>>,
    );
}

/// The responder's terminal round; absorbs the initiator's [`message::Closing`]
/// and emits [`message::Complete`].
pub trait CompleteResponder<P, T>: Stage<Height = S<Z>> + Sized
where
    P: Clone + Ord + AsRef<[u8]>,
    T: Clone,
{
    /// The responder's final round, processing the initiator's
    /// [`message::Closing`].
    ///
    /// We absorb the initiator's last batch of nodes, answer any final
    /// `requested` set, and collapse our zipper back to a root. The returned
    /// [`message::Complete`] carries our last outgoing `providing` for the
    /// initiator to absorb in [`Self::complete_initiator`].
    fn complete_responder(
        self,
        request: message::Closing<P, T>,
    ) -> (
        message::Complete<P, T>,
        Result<Infallible, Option<Node<P, T, Root>>>,
    );
}

/// The initiator's terminal round; absorbs the responder's [`message::Complete`].
pub trait CompleteInitiator<P, T, OnRecv>: Stage<Height = Z> + Sized
where
    P: Clone + Ord + AsRef<[u8]>,
    T: Clone,
    OnRecv: FnMut(&Version<P>, Key, &Message<T>),
{
    /// The initiator's final round.
    ///
    /// Absorbs the responder's last batch of `providing` (from
    /// [`message::Complete`]) and collapses our zipper back to a root. There is
    /// no outgoing message: any `requested` we would have made went out in our
    /// prior [`Self::close_initiator`] call.
    fn complete_initiator(
        self,
        request: message::Complete<P, T>,
    ) -> Result<Infallible, Option<Node<P, T, Root>>>;
}

/// Marker trait keyed by the height `H` just produced by an
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
pub trait AfterExchange<'v, P, T, OnRecv, OnSend, H>: Sized
where
    P: Clone + Ord + AsRef<[u8]>,
    T: Clone,
    H: Height,
{
}

impl<'v, P, T, OnRecv, OnSend, X> AfterExchange<'v, P, T, OnRecv, OnSend, S<Z>> for X
where
    P: Clone + Ord + AsRef<[u8]>,
    T: Clone,
    X: CompleteResponder<P, T>,
{
}

impl<'v, P, T, OnRecv, OnSend, X> AfterExchange<'v, P, T, OnRecv, OnSend, S<S<Z>>> for X
where
    P: Clone + Ord + AsRef<[u8]>,
    T: Clone,
    OnRecv: FnMut(&Version<P>, Key, &Message<T>),
    OnSend: FnMut(&Version<P>, Key, &Message<T>),
    X: CloseInitiator<'v, P, T, OnRecv, OnSend>,
{
}

impl<'v, P, T, OnRecv, OnSend, H, X> AfterExchange<'v, P, T, OnRecv, OnSend, S<S<S<H>>>> for X
where
    P: Clone + Ord + AsRef<[u8]>,
    T: Clone,
    OnRecv: FnMut(&Version<P>, Key, &Message<T>),
    OnSend: FnMut(&Version<P>, Key, &Message<T>),
    H: Height,
    S<H>: Height,
    S<S<H>>: Height,
    S<S<S<H>>>: Height,
    X: Exchange<'v, P, T, S<H>, OnRecv, OnSend>,
{
}
