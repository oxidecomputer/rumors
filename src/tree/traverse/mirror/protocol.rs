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
    Key, Message, Version,
    tree::typed::{
        Node,
        height::{Height, Pred, Root, S, Z},
    },
};

use super::message::{self, UnderRoot, UnderUnderRoot};

/// Any stage in the protocol is identified by this trait, and must declare its
/// height as an associated type.
pub trait Stage {
    /// The height in the protocol, starting at the root.
    type Height: Height;

    /// The end result of the protocol.
    type Output;
}

/// Start the protocol as the initiator.
///
/// The trait is implemented by the state type that the constructor produces;
/// `Self::Next == Self` for any straightforward implementation.
pub trait Initiator<P, T>: Stage<Height = Root> + Sized
where
    P: Clone + Ord + AsRef<[u8]>,
    T: Clone,
{
    /// The state that consumes the responder's [`message::Opening`].
    type Next: OpenInitiator<P, T> + Stage<Output = Self::Output, Height = Root>;

    /// Begin the protocol as the initiator.
    ///
    /// Returns the opening [`message::Initiate`] (just our root hash) and an
    /// `Exchange` whose zipper is at `Top` (height `Root`). The initiator's
    /// next call is [`Exchange::open_initiator`], processing the responder's
    /// [`message::Opening`].
    fn initiator(self) -> (message::Initiate, Self::Next);
}

/// Start the protocol as the responder.
///
/// `Err(node)` from this call indicates that the initiator's root hash matched
/// ours: the trees are already equal, the protocol short-circuits, and the
/// caller receives the unchanged root.
pub trait Responder<P, T>: Stage<Height = Root> + Sized
where
    P: Clone + Ord + AsRef<[u8]>,
    T: Clone,
{
    /// The first steady-state [`Exchange`] from the responder's side.
    type Next: Exchange<P, T> + Stage<Output = Self::Output, Height = UnderRoot>;

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
        self,
        request: message::Initiate,
    ) -> (message::Opening, Result<Self::Next, Self::Output>);
}

/// Process the responder's [`message::Opening`].
///
/// Distinct from [`Exchange`] because the opening carries only `uncertain`,
/// and the responder may list children of the initiator's absent root --- a
/// case the steady-state [`Exchange`] is allowed to debug-assert against.
pub trait OpenInitiator<P, T>: Stage<Height = Root> + Sized
where
    P: Clone + Ord + AsRef<[u8]>,
    T: Clone,
{
    /// The first steady-state [`Exchange`] from the initiator's side.
    type Next: Exchange<P, T> + Stage<Output = Self::Output, Height = UnderUnderRoot>;

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
    fn open_initiator(
        self,
        request: message::Opening,
    ) -> (
        message::Exchange<P, T, UnderUnderRoot>,
        Result<Self::Next, Self::Output>,
    );
}

/// One steady-state round, as either party.
///
/// The outgoing message's height is `Self::Height − 2` (and the incoming
/// message's is `Self::Height − 1`); both are recovered from `Stage::Height`
/// via `Pred` projections, so each implementing type's exchange height is
/// determined by its `Stage::Height` alone.
pub trait Exchange<P, T>: Stage + Sized
where
    P: Clone + Ord + AsRef<[u8]>,
    T: Clone,
    Self::Height: Pred,
    <Self::Height as Pred>::Pred: Pred,
    S<<Self::Height as Pred>::Pred>: Height,
    S<<<Self::Height as Pred>::Pred as Pred>::Pred>: Height,
{
    /// Whichever of [`Exchange`], [`CloseInitiator`], or [`CompleteResponder`]
    /// is appropriate at the outgoing message's height. See [`AfterExchange`].
    type Next: AfterExchange<P, T, <<Self::Height as Pred>::Pred as Pred>::Pred>
        + Stage<Output = Self::Output, Height = <<Self::Height as Pred>::Pred as Pred>::Pred>;

    /// Process one round of the protocol's steady state, as either party.
    ///
    /// Each call moves our zipper down by two heights and emits the next
    /// outgoing message. The returned `Result` is `Err(final_tree)` once we
    /// have nothing left to ask about and nothing left in dispute -- but the
    /// outgoing message is sent unconditionally, because the counterparty may
    /// still need its contents to converge.
    #[allow(clippy::type_complexity)]
    fn exchange(
        self,
        request: message::Exchange<P, T, <Self::Height as Pred>::Pred>,
    ) -> (
        message::Exchange<P, T, <<Self::Height as Pred>::Pred as Pred>::Pred>,
        Result<Self::Next, Self::Output>,
    );
}

/// The initiator's final sending round; emits [`message::Closing`] instead of
/// the vacuous leaf-height [`message::Exchange`].
pub trait CloseInitiator<P, T>: Stage<Height = S<S<Z>>> + Sized
where
    P: Clone + Ord + AsRef<[u8]>,
    T: Clone,
{
    /// The terminal initiator state.
    type Next: CompleteInitiator<P, T> + Stage<Output = Self::Output, Height = Z>;

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
    ) -> (message::Closing<P, T>, Result<Self::Next, Self::Output>);
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
    ) -> (message::Complete<P, T>, Result<Infallible, Self::Output>);
}

/// The initiator's terminal round; absorbs the responder's [`message::Complete`].
pub trait CompleteInitiator<P, T>: Stage<Height = Z> + Sized
where
    P: Clone + Ord + AsRef<[u8]>,
    T: Clone,
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
    ) -> Result<Infallible, Self::Output>;
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
    P: Clone + Ord + AsRef<[u8]>,
    T: Clone,
    H: Height,
{
}

impl<P, T, X> AfterExchange<P, T, S<Z>> for X
where
    P: Clone + Ord + AsRef<[u8]>,
    T: Clone,
    X: CompleteResponder<P, T>,
{
}

impl<P, T, X> AfterExchange<P, T, S<S<Z>>> for X
where
    P: Clone + Ord + AsRef<[u8]>,
    T: Clone,
    X: CloseInitiator<P, T>,
{
}

impl<P, T, H, X> AfterExchange<P, T, S<S<S<H>>>> for X
where
    P: Clone + Ord + AsRef<[u8]>,
    T: Clone,
    H: Height,
    S<H>: Height,
    S<S<H>>: Height,
    S<S<S<H>>>: Height,
    X: Exchange<P, T> + Stage<Height = S<S<S<H>>>>,
{
}
