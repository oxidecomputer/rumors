//! Two replicas reconcile their trees while honoring deletions: leaves one side
//! has and the other has since *forgotten* (their version is `<=` the other's
//! version vector) vanish; leaves not yet seen are transmitted. The protocol
//! recurses down the *disjoint frontier* of the two trees, alternating sender
//! each message, so it costs `O(log n)` round-trips and never re-sends a hash
//! the other side can already infer.
//!
//! # State machine
//!
//! Each side keeps a [`Levels`](crate::tree::typed::Levels) zipper: a stack of
//! level maps from `Root` down to the height currently under comparison. In one
//! round, the sender examines its zipper's bottom level, sends a message, and
//! pushes two new (mostly empty) levels onto the bottom; the receiver's next
//! round operates on its own zipper, offset by one height. Heights on which the
//! parties have agreed end up nearer the top of the zipper; heights still in
//! dispute live at the bottom. After max 16 rounds (likely fewer), both have
//! reached `Z` (leaf height) and the zippers collapse back to roots.
//!
//! The wire conversation:
//!
//!   1. Initiator sends [`message::Initiate`] (a single hash at the empty
//!      prefix: our root hash).
//!   2. Responder either declares the trees equal, or replies with
//!      [`message::Opening`] enumerating the hashes of *every* child of its
//!      root.
//!   3. Both sides alternately send [`message::Exchange`]s, each round
//!      descending the sender's zipper by two heights.
//!   4. The initiator's last outgoing message is [`message::Closing`] in
//!      lieu of an `Exchange` at leaf height (whose `uncertain` would be
//!      vacuous); the responder replies with [`message::Complete`] carrying
//!      only the final `providing`; the initiator absorbs that and is done.
//!
//! # Three channels
//!
//! The wire format has three independent flows of information. Each message
//! type carries the subset of fields that are non-vacuous for its role:
//! `Initiate` and `Opening` carry only `uncertain`; `Complete` only
//! `providing`; `Closing` carries `providing` and `requested`; the
//! steady-state `Exchange` carries all three.
//!
//! | Field       | Sender's claim                                 | Receiver's action            |
//! |-------------|------------------------------------------------|------------------------------|
//! | `uncertain` | "I have these hashes at this height"           | compare against my own       |
//! | `requested` | "your last `uncertain` listed hashes I lack"   | answer via `providing` next  |
//! | `providing` | "you asked for these, or I know you lack them" | insert into my zipper        |
//!
//! # Asymmetry matrix
//!
//! For every prefix at the current comparison height there are four cases,
//! depending on whether each side has the node. The protocol is correct iff
//! every case routes its information to some channel:
//!
//! |                | counterparty has it                                 | counterparty lacks it                         |
//! |----------------|-----------------------------------------------------|-----------------------------------------------|
//! | **we have it** | hashes match: no action; hashes differ: recur below | we `provide`                                  |
//! | **we lack it** | we `request`                                        | (impossible: neither side would mention it)   |
//!
//! Each cell is realized by one arm of the `merge_join_by` inside
//! [`Exchange::partition_uncertain`].

use std::{convert::Infallible, future::Future, mem, sync::Arc};

use itertools::{EitherOrBoth, Itertools};

use before::Party;

use crate::{
    network::Network,
    tree::{
        self,
        key::Key,
        traverse::{unknown, unknown::Unknown},
        typed::{
            Hash, Level, Levels, Node, Prefix,
            height::{Height, Root, S, Z},
            levels::{Below, Top},
        },
    },
    version::Version,
};

use super::message::{self, UnderRoot, UnderUnderRoot};
use super::protocol;

/// The version state for an [`Exchange`] which has just been initialized but
/// has not yet connected. Carries the fields the [`Connect`](protocol::Connect)
/// / [`Accept`](protocol::Accept) step needs to build its outgoing
/// [`message::Handshake`]: our universe [`Network`], our latest [`Version`], and
/// — iff we are *retiring* — the [`Party`] we offer the peer to absorb.
pub struct Start {
    network: Network,
    our_version: Version,
    party: Option<Party>,
}

/// The version state for an [`Exchange`] which has sent its version to its peer
/// but has not yet received its peer's version.
pub struct Connecting {
    our_version: Version,
}

/// The version state for an [`Exchange`] which has sent and received versions
/// with its peer, and so can proceed to the rest of the protocol.
pub struct Connected {
    our_version: Version,
    their_version: Version,
}

/// The "no callback" callback type: a bare function pointer with the mirror
/// callback signature. Used as the `OnSend`/`OnRecv` type parameter when a
/// side passes [`None`] (see [`Exchange::silent`]); it is never actually
/// invoked, only named to pin the otherwise-unconstrained type parameter.
pub type Silent<T> = fn(Key, &Version, &Arc<T>) -> std::future::Ready<()>;

/// The parent prefixes a counterparty may legitimately `provide` against, given
/// the `requested` and `uncertain` we are about to send: each `requested`
/// prefix is the parent of the subtree-children it will answer with, and each
/// `uncertain` prefix's parent is the parent of the Left-case siblings it may
/// unilaterally provide. Returned as raw bytes so the membership test in
/// [`Exchange::absorb_providing`] is height-agnostic. Debug-only: it backs a
/// `debug_assert!`.
#[cfg(debug_assertions)]
fn expected_providing_parents<A, B>(
    requested: &[Prefix<A>],
    uncertain: &[(Prefix<B>, Hash)],
) -> std::collections::BTreeSet<Box<[u8]>>
where
    A: Height,
    B: Height,
{
    let mut parents = std::collections::BTreeSet::default();
    // The root is always implicitly compared, so the counterparty may always
    // provide root's children (the first round's asymmetric-root drain, where
    // the initiator hands over root children an empty/divergent responder never
    // listed). Its parent is the empty prefix.
    parents.insert(Box::from(&[][..]));
    for prefix in requested {
        parents.insert(Box::from(prefix.as_bytes()));
    }
    for (prefix, _) in uncertain {
        let bytes = prefix.as_bytes();
        parents.insert(Box::from(&bytes[..bytes.len().saturating_sub(1)]));
    }
    parents
}

/// An in-progress mirror synchronization on one side of the wire.
///
/// `L` is our zipper, parameterised by the height of its bottom level; as the
/// protocol descends, each [`Self::exchange`] call returns a new `Exchange`
/// whose `L` is two heights below the previous one.
///
/// Both callbacks are [`Option`]al. A [`None`] `on_recv` lets
/// [`Self::absorb_providing`] skip the per-leaf discovery walk entirely (the
/// dominant cost when no observation is wanted, as in [`join`] and
/// callback-less [`learn`]); a [`None`] `on_send` skips firing the
/// counterparty-side notification while still running the load-bearing
/// version filter.
///
/// [`join`]: crate::Known::join
/// [`learn`]: crate::Known::learn
pub struct Exchange<OnSend, OnRecv, V, L> {
    /// Our multi-level zipper: agreed heights live near the top, the height
    /// currently under comparison lives at the bottom.
    levels: L,
    /// The counterparty's version vector, used to honor their deletions: any
    /// node of ours at or causally prior to this version that they lack must
    /// have been forgotten on their side.
    versions: V,
    /// Invoked whenever we discover a leaf that was previously unknown to us;
    /// lets the calling code stream out leaf-level observations as we find
    /// out about them. [`None`] skips the discovery walk altogether.
    on_recv: Option<OnRecv>,
    /// Invoked whenever the version-vector filter discovers a leaf the
    /// counterparty does not yet know about; lets the calling code stream out
    /// leaf-level observations as they're discovered by the counterparty.
    /// [`None`] skips the notification (the filter itself still runs).
    on_send: Option<OnSend>,
    /// The parent prefixes (raw bytes, height-agnostic) the counterparty is
    /// allowed to `provide` next: those we `requested` last round, plus the
    /// parents of those we listed as `uncertain` (whose siblings the
    /// counterparty may unilaterally provide as the Left case). Used by
    /// [`absorb_providing`](Self::absorb_providing) to reject a peer that
    /// provides subtrees we had no basis to receive.
    ///
    /// Tracked only in debug builds, since it backs a `debug_assert!`: release
    /// builds carry no field and pay nothing to maintain it.
    #[cfg(debug_assertions)]
    expected_parents: std::collections::BTreeSet<Box<[u8]>>,
}

impl<OnSend, OnRecv, T> Exchange<OnSend, OnRecv, Start, Top<T>>
where
    T: Send + Sync,
{
    pub fn start(
        node: tree::Root<T>,
        network: Network,
        party: Option<Party>,
        on_send: Option<OnSend>,
        on_recv: Option<OnRecv>,
    ) -> Self {
        Self {
            versions: Start {
                network,
                our_version: node.ceiling.clone(),
                party,
            },
            levels: Node::levels(Option::from(node)),
            on_recv,
            on_send,
            #[cfg(debug_assertions)]
            expected_parents: Default::default(),
        }
    }
}

// Only the tests construct a both-silent local exchange now: production
// `join`/`join_then` merge in-process via [`Tree::join`](crate::tree::Tree),
// and `gossip` pairs a callback-carrying local side with a remote one.
#[cfg(test)]
impl<T> Exchange<Silent<T>, Silent<T>, Start, Top<T>>
where
    T: Send + Sync,
{
    /// Start a side with no callbacks: neither leaf discovery nor
    /// counterparty-side notification is reported. Equivalent to
    /// [`Self::start`] with both callbacks [`None`], but pins the
    /// otherwise-unconstrained callback type parameters to [`Silent`] so the
    /// caller need not spell them out. This is the path that elides the
    /// [`absorb_providing`](Self::absorb_providing) discovery walk.
    pub fn silent(node: tree::Root<T>) -> Self {
        // Local-local test sessions never run the lib-level network/party
        // dispatch, so the placeholder network and absent party are inert: the
        // handshake they produce is consumed only for its version.
        Self::start(node, Network::ZERO, None, None, None)
    }
}

// We define a local `Exchage`'s participation in the protocol as such:

impl<OnSend, OnRecv, V, L> protocol::Stage for Exchange<OnSend, OnRecv, V, L>
where
    L: Levels,
{
    type Height = L::Height;
    type Output = tree::Root<L::Message>;
    type Error = Infallible;
}

impl<T, OnSend, OnSendFut, OnRecv, OnRecvFut> protocol::Connect<T>
    for Exchange<OnSend, OnRecv, Start, Top<T>>
where
    T: Send + Sync,
    OnRecv: FnMut(Key, &Version, &Arc<T>) -> OnRecvFut + Send,
    OnRecvFut: Future<Output = ()> + Send,
    OnSend: FnMut(Key, &Version, &Arc<T>) -> OnSendFut + Send,
    OnSendFut: Future<Output = ()> + Send,
{
    type Next = Exchange<OnSend, OnRecv, Connecting, Top<T>>;

    async fn connect(
        self,
    ) -> Result<protocol::Step<message::Handshake, Self::Next, Infallible>, Self::Error> {
        let Start {
            network,
            our_version,
            party,
        } = self.versions;

        let next = Exchange {
            levels: self.levels,
            versions: Connecting {
                our_version: our_version.clone(),
            },
            on_recv: self.on_recv,
            on_send: self.on_send,
            #[cfg(debug_assertions)]
            expected_parents: self.expected_parents,
        };

        Ok(protocol::Step::Continue {
            msg: message::Handshake {
                network,
                version: our_version,
                party,
            },
            next,
        })
    }
}

impl<T, OnSend, OnSendFut, OnRecv, OnRecvFut> protocol::CompleteConnect<T>
    for Exchange<OnSend, OnRecv, Connecting, Top<T>>
where
    T: Send + Sync,
    OnRecv: FnMut(Key, &Version, &Arc<T>) -> OnRecvFut + Send,
    OnRecvFut: Future<Output = ()> + Send,
    OnSend: FnMut(Key, &Version, &Arc<T>) -> OnSendFut + Send,
    OnSendFut: Future<Output = ()> + Send,
{
    type Next = Exchange<OnSend, OnRecv, Connected, Top<T>>;

    async fn complete_connect(
        self,
        their_version: Version,
    ) -> Result<protocol::Step<(), Self::Next, Self::Output>, Self::Error> {
        let our_version = self.versions.our_version;

        // If the two versions are the same, both sides are immediately done
        if our_version == their_version {
            return Ok(protocol::Step::Done {
                msg: (),
                output: tree::Root {
                    ceiling: our_version,
                    root: self.levels.collapse(),
                },
            });
        }

        let next = Exchange {
            levels: self.levels,
            versions: Connected {
                our_version,
                their_version,
            },
            on_recv: self.on_recv,
            on_send: self.on_send,
            #[cfg(debug_assertions)]
            expected_parents: self.expected_parents,
        };

        Ok(protocol::Step::Continue { msg: (), next })
    }
}

impl<T, OnSend, OnSendFut, OnRecv, OnRecvFut> protocol::Accept<T>
    for Exchange<OnSend, OnRecv, Start, Top<T>>
where
    T: Send + Sync,
    OnRecv: FnMut(Key, &Version, &Arc<T>) -> OnRecvFut + Send,
    OnRecvFut: Future<Output = ()> + Send,
    OnSend: FnMut(Key, &Version, &Arc<T>) -> OnSendFut + Send,
    OnSendFut: Future<Output = ()> + Send,
{
    type Next = Exchange<OnSend, OnRecv, Connected, Top<T>>;

    async fn accept(
        self,
        request: message::Handshake,
    ) -> Result<protocol::Step<message::Handshake, Self::Next, Self::Output>, Self::Error> {
        let Start {
            network,
            our_version,
            party,
        } = self.versions;
        let their_version = request.version;

        // If the two versions are the same, both sides are immediately done
        if our_version == their_version {
            return Ok(protocol::Step::Done {
                msg: message::Handshake {
                    network,
                    version: our_version.clone(),
                    party,
                },
                output: tree::Root {
                    ceiling: our_version,
                    root: self.levels.collapse(),
                },
            });
        }

        let next = Exchange {
            levels: self.levels,
            versions: Connected {
                our_version: our_version.clone(),
                their_version,
            },
            on_recv: self.on_recv,
            on_send: self.on_send,
            #[cfg(debug_assertions)]
            expected_parents: self.expected_parents,
        };

        Ok(protocol::Step::Continue {
            msg: message::Handshake {
                network,
                version: our_version,
                party,
            },
            next,
        })
    }
}

impl<OnSend, OnRecv, T> Exchange<OnSend, OnRecv, Connected, Top<T>>
where
    T: Send + Sync,
{
    /// Collapse a connected exchange back to its tree root *without* running the
    /// descent. The tree is unchanged since [`start`](Self::start); used when
    /// the session ends right after the handshake — an absorbed retiree, a
    /// declined retirement, or already-converged peers — instead of descending.
    pub fn into_root(self) -> tree::Root<T> {
        tree::Root {
            ceiling: self.versions.our_version,
            root: self.levels.collapse(),
        }
    }
}

impl<T, OnSend, OnSendFut, OnRecv, OnRecvFut> protocol::Initiator<T>
    for Exchange<OnSend, OnRecv, Connected, Top<T>>
where
    T: Send + Sync,
    OnRecv: FnMut(Key, &Version, &Arc<T>) -> OnRecvFut + Send,
    OnRecvFut: Future<Output = ()> + Send,
    OnSend: FnMut(Key, &Version, &Arc<T>) -> OnSendFut + Send,
    OnSendFut: Future<Output = ()> + Send,
{
    type Next = Exchange<OnSend, OnRecv, Connected, Top<T>>;

    async fn initiator(
        self,
    ) -> Result<protocol::Step<message::Initiate, Self::Next, Infallible>, Infallible> {
        let msg = message::Initiate {
            uncertain: self
                .levels
                .level()
                .iter()
                .map(|(prefix, node)| (*prefix, node.hash()))
                .collect(),
        };

        Ok(protocol::Step::Continue { msg, next: self })
    }
}

impl<T, OnSend, OnSendFut, OnRecv, OnRecvFut> protocol::Responder<T>
    for Exchange<OnSend, OnRecv, Connected, Top<T>>
where
    T: Send + Sync,
    OnRecv: FnMut(Key, &Version, &Arc<T>) -> OnRecvFut + Send,
    OnRecvFut: Future<Output = ()> + Send,
    OnSend: FnMut(Key, &Version, &Arc<T>) -> OnSendFut + Send,
    OnSendFut: Future<Output = ()> + Send,
{
    type Next = Exchange<OnSend, OnRecv, Connected, Below<UnderRoot, Top<T>>>;

    async fn responder(
        mut self,
        _request: message::Initiate,
    ) -> Result<protocol::Step<message::Opening, Self::Next, Self::Output>, Infallible> {
        // Always explode our root one level down and enumerate the resulting
        // children, regardless of the initiator's root hash. We deliberately do
        // *not* short-circuit on matched roots: an empty `Opening` is the
        // unambiguous "responder has no children" signal that drives the
        // initiator's [`Self::open_initiator`] "we have, they lack" Left case
        // when the responder is empty. Pushing the matched case through the
        // steady-state pipeline costs one round's worth of child hashes
        // (~16 entries) but keeps a single termination path on the wire.
        let levels = Node::levels(None).down(
            self.levels
                .level_mut()
                .remove(&Prefix::new())
                .map(|n| {
                    n.into_children()
                        .into_iter()
                        .map(|(radix, child)| (Prefix::new().push(radix), child))
                        .collect()
                })
                .unwrap_or_default(),
        );

        let msg = message::Opening {
            uncertain: levels
                .level()
                .iter()
                .map(|(prefix, child)| (*prefix, child.hash()))
                .collect(),
        };

        let next = Exchange {
            levels,
            versions: self.versions,
            on_recv: self.on_recv,
            on_send: self.on_send,
            // The `Opening` carries only `uncertain`; the initiator may answer
            // it with Left-case `providing` whose parents are the parents of
            // these prefixes (it carries no `requested` to honor).
            #[cfg(debug_assertions)]
            expected_parents: expected_providing_parents::<UnderRoot, _>(&[], &msg.uncertain),
        };

        Ok(protocol::Step::Continue { msg, next })
    }
}

impl<T, OnSend, OnSendFut, OnRecv, OnRecvFut, L> protocol::OpenInitiator<T>
    for Exchange<OnSend, OnRecv, Connected, L>
where
    T: Send + Sync,
    OnRecv: FnMut(Key, &Version, &Arc<T>) -> OnRecvFut + Send,
    OnRecvFut: Future<Output = ()> + Send,
    OnSend: FnMut(Key, &Version, &Arc<T>) -> OnSendFut + Send,
    OnSendFut: Future<Output = ()> + Send,
    L: Levels<Message = T, Height = Root>,
{
    type Next = Exchange<OnSend, OnRecv, Connected, Below<UnderUnderRoot, Below<UnderRoot, L>>>;

    async fn open_initiator(
        self,
        request: message::Opening,
    ) -> Result<
        protocol::Step<message::Exchange<T, UnderUnderRoot>, Self::Next, Self::Output>,
        Infallible,
    > {
        Ok(self.reply(request).await)
    }
}

impl<T, H, OnSend, OnSendFut, OnRecv, OnRecvFut, L> protocol::Exchange<T>
    for Exchange<OnSend, OnRecv, Connected, L>
where
    T: Send + Sync,
    OnRecv: FnMut(Key, &Version, &Arc<T>) -> OnRecvFut + Send,
    OnRecvFut: Future<Output = ()> + Send,
    OnSend: FnMut(Key, &Version, &Arc<T>) -> OnSendFut + Send,
    OnSendFut: Future<Output = ()> + Send,
    L: Levels<Message = T, Height = S<S<H>>>,
    S<S<H>>: Height,
    S<H>: Height,
    H: Height + Unknown,
    // Assumed at impl-validation time so we don't have to case-analyze `H`
    // here: at use sites `H` is concrete and one of the three blanket impls
    // discharges it.
    Exchange<OnSend, OnRecv, Connected, Below<H, Below<S<H>, L>>>: protocol::AfterExchange<T, H>,
{
    type Next = Exchange<OnSend, OnRecv, Connected, Below<H, Below<S<H>, L>>>;

    async fn exchange(
        self,
        request: message::Exchange<T, S<H>>,
    ) -> Result<protocol::Step<message::Exchange<T, H>, Self::Next, Self::Output>, Infallible> {
        Ok(self.reply(request).await)
    }
}

impl<T, OnSend, OnSendFut, OnRecv, OnRecvFut, L> protocol::CloseInitiator<T>
    for Exchange<OnSend, OnRecv, Connected, L>
where
    T: Send + Sync,
    OnRecv: FnMut(Key, &Version, &Arc<T>) -> OnRecvFut + Send,
    OnRecvFut: Future<Output = ()> + Send,
    OnSend: FnMut(Key, &Version, &Arc<T>) -> OnSendFut + Send,
    OnSendFut: Future<Output = ()> + Send,
    L: Levels<Message = T, Height = S<S<Z>>>,
{
    type Next = Exchange<OnSend, OnRecv, Connected, Below<Z, Below<S<Z>, L>>>;

    async fn close_initiator(
        self,
        request: message::Exchange<T, S<Z>>,
    ) -> Result<protocol::Step<message::Closing<T>, Self::Next, Self::Output>, Infallible> {
        Ok(self.reply(request).await)
    }
}

impl<T, OnSend, OnSendFut, OnRecv, OnRecvFut, L> protocol::CompleteResponder<T>
    for Exchange<OnSend, OnRecv, Connected, L>
where
    T: Send + Sync,
    OnRecv: FnMut(Key, &Version, &Arc<T>) -> OnRecvFut + Send,
    OnRecvFut: Future<Output = ()> + Send,
    OnSend: FnMut(Key, &Version, &Arc<T>) -> OnSendFut + Send,
    OnSendFut: Future<Output = ()> + Send,
    L: Levels<Message = T, Height = S<Z>>,
{
    async fn complete_responder(
        mut self,
        request: message::Closing<T>,
    ) -> Result<protocol::Step<message::Complete<T>, Infallible, Self::Output>, Infallible> {
        self.absorb_providing(request.providing).await;
        let providing = self.answer_requested(request.requested).await;
        Ok(protocol::Step::Done {
            msg: message::Complete {
                providing: providing.into_iter().collect(),
            },
            output: tree::Root {
                ceiling: self.versions.our_version | self.versions.their_version,
                root: self.levels.collapse(),
            },
        })
    }
}

impl<T, OnSend, OnSendFut, OnRecv, OnRecvFut, L> protocol::CompleteInitiator<T>
    for Exchange<OnSend, OnRecv, Connected, L>
where
    T: Send + Sync,
    OnRecv: FnMut(Key, &Version, &Arc<T>) -> OnRecvFut + Send,
    OnRecvFut: Future<Output = ()> + Send,
    OnSend: FnMut(Key, &Version, &Arc<T>) -> OnSendFut + Send,
    OnSendFut: Future<Output = ()> + Send,
    L: Levels<Message = T, Height = Z>,
{
    async fn complete_initiator(
        mut self,
        request: message::Complete<T>,
    ) -> Result<protocol::Step<(), Infallible, Self::Output>, Infallible> {
        self.absorb_providing(request.providing).await;
        Ok(protocol::Step::Done {
            msg: (),
            output: tree::Root {
                ceiling: self.versions.our_version | self.versions.their_version,
                root: self.levels.collapse(),
            },
        })
    }
}

// Internal implementation of methods on `Exchange` involved in the protocol:

/// The output of [`Exchange::partition_uncertain`], one field per outgoing
/// channel in the asymmetry matrix.
struct Partition<T, H>
where
    S<H>: Height,
    H: Height,
{
    /// Left-case subtrees (we have them, the counterparty does not). The caller
    /// will combine these with `answer_requested`'s output to form the final
    /// outgoing `providing`.
    providing: Level<T, S<H>>,
    /// Right-case prefixes (the counterparty has them, we do not): the outgoing
    /// `requested`. Built in strictly ascending order (see
    /// [`Exchange::partition_uncertain`]).
    requested: Vec<Prefix<S<H>>>,
    /// `Both`-case children whose hashes agreed, plus Left-case children we
    /// kept locally. Become the new level immediately above the bottom.
    matched: Level<T, S<H>>,
    /// `Both`-case grandchildren of children whose hashes disagreed. Become the
    /// new bottom of the zipper, and next round's outgoing `uncertain`.
    exploded: Level<T, H>,
}

impl<OnSend, OnRecv, L> Exchange<OnSend, OnRecv, Connected, L>
where
    L: Levels,
    L::Message: Send + Sync,
    OnSend: Send,
    OnRecv: Send,
{
    /// Insert nodes the counterparty has just sent us (because we requested
    /// them last round, or because they unilaterally knew we lacked them) into
    /// our zipper's bottom level.
    ///
    /// Each subtree arrives as a whole `(prefix, node)` pair, in ascending
    /// prefix order, and is inserted directly at the named prefix.
    async fn absorb_providing<H, OnRecvFut>(&mut self, providing: message::Providing<L::Message, H>)
    where
        L::Message: Send + Sync,
        OnRecv: FnMut(Key, &Version, &Arc<L::Message>) -> OnRecvFut + Send,
        OnRecvFut: Future<Output = ()> + Send,
        L: Levels<Height = H>,
        H: Height,
    {
        // The counterparty may only provide subtrees we requested last round,
        // or whose parent we listed as `uncertain` (the Left case it may infer
        // we lack). Each provided prefix's parent must therefore be one we
        // recorded as expected; anything else means the peer is misbehaving, or
        // we are.
        #[cfg(debug_assertions)]
        for (prefix, _) in &providing {
            let bytes = prefix.as_bytes();
            let parent = &bytes[..bytes.len().saturating_sub(1)];
            debug_assert!(
                self.expected_parents.contains(parent),
                "counterparty provided prefix {prefix:?} we neither requested nor left to infer",
            );
        }

        // Only walk a just-absorbed subtree to fire `on_recv` when there is an
        // `on_recv` to fire: with no callback the walk is pure waste (it does
        // nothing but invoke a no-op per leaf), and skipping it is the dominant
        // saving for callback-less `learn`/`join`.
        if let Some(on_recv) = self.on_recv.as_mut() {
            for (prefix, node) in &providing {
                // Read-only walk: borrow each subtree to fire `on_recv` per
                // leaf, leaving the node (and its memoized hash/ceiling/floor)
                // intact to move into the frontier below.
                for (key, version, message) in node.leaves(*prefix) {
                    on_recv(key, version, message.as_arc()).await;
                }
            }
        }

        // Merge the provided subtrees into the frontier in a single pass. Both
        // sides are sorted ascending by prefix — the wire frame is canonical,
        // and the frontier maintains the `Level` invariant — so this is an
        // O(n+m) merge rather than m separate O(n) binary-search inserts.
        self.levels
            .level_mut()
            .extend(Level::from_sorted(providing));
    }

    /// Answer the counterparty's `requested` set by exploding each requested
    /// node into its children, filtered against the counterparty's version so
    /// that any subtrees they have deleted disappear locally too. Returns the
    /// outgoing `providing` map, one height below the frontier.
    async fn answer_requested<H, OnSendFut>(
        &mut self,
        requested: Vec<Prefix<S<H>>>,
    ) -> Level<L::Message, H>
    where
        L: Levels<Height = S<H>>,
        OnSend: FnMut(Key, &Version, &Arc<L::Message>) -> OnSendFut,
        OnSendFut: Future<Output = ()> + Send,
        S<H>: Unknown,
        H: Height,
    {
        // Drain the frontier in one pass, co-iterating it against the ascending
        // `requested` set (a subset of the frontier's prefixes). Requested
        // nodes are exploded into the outgoing `providing`; their surviving
        // selves and every un-requested node carry over into the rebuilt
        // frontier. Both the frontier and `requested` are sorted, so this is
        // O(n) rather than a binary-search `remove`/`insert` per requested
        // prefix.
        let mut providing = Level::default();
        // Grow `kept` by `push` rather than pre-sizing to `frontier.len()`: these
        // levels are allocated and freed every round, and `Vec`'s power-of-two
        // growth recycles through the allocator's size classes across rounds far
        // better than an exact, round-varying `with_capacity` (measured: the
        // pre-sized variant regressed).
        let mut kept = Level::default();
        let mut requested = requested.into_iter().peekable();
        for (prefix, node) in mem::take(self.levels.level_mut()) {
            if requested.peek() == Some(&prefix) {
                requested.next();
                // Filter against the counterparty's version: anything causally
                // prior to it that they lack, they have already deleted -- so
                // we should too. The surviving subtree (if any) carries over
                // into the rebuilt frontier; its children are sent out as
                // `providing`.
                let mut on_send = self.on_send.as_mut().map(unknown::from_arc);
                if let Some(node) = Unknown::unknown(
                    Some(node),
                    prefix,
                    &self.versions.their_version,
                    &mut on_send,
                )
                .await
                {
                    for (radix, child) in node.clone().into_children() {
                        providing.push(prefix.push(radix), child);
                    }
                    kept.push(prefix, node);
                }
            } else {
                kept.push(prefix, node);
            }
        }
        // The counterparty should only request prefixes we previously listed as
        // `uncertain`; a leftover means the request named a prefix we lack, so
        // either the counterparty is misbehaving, or we are.
        #[cfg(debug_assertions)]
        if let Some(prefix) = requested.peek() {
            panic!("counterparty requested unknown prefix {:?}", prefix);
        }
        *self.levels.level_mut() = kept;
        providing
    }

    /// Partition the counterparty's `uncertain` hashes against our own tree by
    /// cell of the asymmetry matrix (see module docs). The returned
    /// [`Partition`] names one output per cell; the caller folds them into the
    /// outgoing message and the zipper's next two levels.
    ///
    /// Shared by [`Self::open_initiator`], [`Self::exchange`], and
    /// [`Self::close_initiator`]. The two "asymmetric root" branches — `else`
    /// (we lack the parent) and the post-loop drain (we have a parent the
    /// counterparty never mentioned) — are reachable only from
    /// `open_initiator`: in steady-state both sides' frontiers were
    /// constructed by Both-case matches in the previous round and therefore
    /// agree on every parent. The debug-assertions guard against a
    /// steady-state caller silently triggering either branch.
    async fn partition_uncertain<H, OnSendFut>(
        &mut self,
        uncertain: Vec<(Prefix<S<H>>, Hash)>,
    ) -> Partition<L::Message, H>
    where
        OnSend: FnMut(Key, &Version, &Arc<L::Message>) -> OnSendFut,
        OnSendFut: Future<Output = ()> + Send,
        L: Levels<Height = S<S<H>>>,
        S<S<H>>: Height,
        S<H>: Height + Unknown,
        H: Height + Unknown,
    {
        let mut providing = Level::default();
        // `requested` is appended in strictly ascending order: parents are
        // visited in ascending order (`by_parent` chunks the sorted `uncertain`)
        // and, within each, `merge_join_by` yields ascending radixes — so a
        // `push` keeps it sorted with no per-entry search.
        let mut requested = Vec::new();
        let mut matched = Level::default();
        let mut exploded = Level::default();

        // The Root level holds a single (empty-prefix) entry, so the two
        // "asymmetric root" cases below — a counterparty parent we lack, and a
        // parent we hold that the counterparty never mentioned — are reachable
        // only when the frontier sits at Root, i.e. the initiator's first round.
        let at_root = <L::Height as Height>::HEIGHT == <Root as Height>::HEIGHT;

        // Group the uncertain prefixes by their parent, so we pull each parent
        // out of the frontier at most once. We collect into an owned `Vec`
        // before iterating: `itertools::ChunkBy` uses interior `RefCell`/
        // `Cell` state, which is `!Sync` and would otherwise make the
        // surrounding `async fn`'s state machine `!Send`.
        let by_parent: Vec<(_, Vec<_>)> = uncertain
            .into_iter()
            .map(|(prefix, hash)| {
                let (parent_prefix, radix) = prefix.pop();
                (parent_prefix, radix, hash)
            })
            .chunk_by(|(parent_prefix, _, _)| *parent_prefix)
            .into_iter()
            .map(|(parent_prefix, group)| (parent_prefix, group.collect()))
            .collect();

        // Drain the frontier in one pass, co-iterating it (ascending) against
        // the ascending `by_parent` groups. Both are sorted, so each parent is
        // matched, kept, or requested in a single linear walk — no per-parent
        // binary-search `remove`. Parents the counterparty did mention are
        // consumed here; parents it never mentioned carry over into `kept` (the
        // rebuilt frontier) unless we are at Root, where they drain below.
        let mut frontier = mem::take(self.levels.level_mut()).into_iter().peekable();
        let mut kept = Level::default();
        for (parent_prefix, uncertain_children) in by_parent {
            // Frontier parents preceding this group are ones the counterparty
            // never mentioned; below Root they are expected leftovers that stay
            // put (turning them into Left cases would re-send data). At Root
            // prefixes are all empty, so this never fires.
            while frontier.peek().is_some_and(|(fp, _)| *fp < parent_prefix) {
                let (fp, fnode) = frontier.next().unwrap();
                kept.push(fp, fnode);
            }

            if frontier.peek().is_some_and(|(fp, _)| *fp == parent_prefix) {
                let (_, parent) = frontier.next().unwrap();
                // Merge-join our children against theirs by radix. Each cell of
                // the asymmetry matrix from the module docs corresponds to
                // exactly one arm below: `Left` is (we have, they lack), `Both`
                // is (we have, they have), `Right` is (we lack, they have). The
                // fourth cell (we lack, they lack) is unreachable: neither side
                // would have mentioned it.
                for cell in parent.into_children().into_iter().merge_join_by(
                    uncertain_children,
                    |(child_radix, _), (_, hash_radix, _)| child_radix.cmp(hash_radix),
                ) {
                    use EitherOrBoth::*;
                    match cell {
                        // We have it, they lack it: provide the surviving
                        // subtree (filtered against their version to honor
                        // their deletions) and keep a local copy.
                        Left((child_radix, ours)) => {
                            let child_prefix = parent_prefix.push(child_radix);
                            let mut on_send = self.on_send.as_mut().map(unknown::from_arc);
                            if let Some(ours) = Unknown::unknown(
                                Some(ours),
                                child_prefix,
                                &self.versions.their_version,
                                &mut on_send,
                            )
                            .await
                            {
                                providing.push(child_prefix, ours.clone());
                                matched.push(child_prefix, ours);
                            }
                        }
                        // We both have it: drop on hash match, otherwise
                        // recurse one level finer by exploding our copy into
                        // the bottom-most level for the next round.
                        Both((child_radix, ours), (parent_prefix, _, theirs)) => {
                            let child_prefix = parent_prefix.push(child_radix);
                            if ours.hash() == theirs {
                                matched.push(child_prefix, ours);
                            } else {
                                for (grandchild_radix, grandchild) in ours.into_children() {
                                    let grandchild_prefix = child_prefix.push(grandchild_radix);
                                    exploded.push(grandchild_prefix, grandchild);
                                }
                            }
                        }
                        // We lack it, they have it: request it.
                        Right((parent_prefix, hash_radix, _)) => {
                            requested.push(parent_prefix.push(hash_radix));
                        }
                    }
                }
            } else {
                debug_assert!(
                    at_root,
                    "counterparty indicated uncertainty about unknown parent \
                    prefix {:?} outside of the initiator's first round",
                    parent_prefix,
                );
                for (parent, hash_radix, _) in uncertain_children {
                    requested.push(parent.push(hash_radix));
                }
            }
        }

        // Frontier parents past the last mentioned group. Below Root they are
        // expected leftovers and carry over unchanged. At Root, any parent we
        // hold that the counterparty never mentioned is one we infer it lacks
        // entirely, so every child is a "Left" case (we have, they lack); we
        // drain it here. The Root guard is *load-bearing*, not just an
        // assertion: below Root these leftovers are normal (e.g. responder
        // children just carried through `answer_requested`), and turning them
        // into Left cases would re-emit data we already sent.
        for (parent_prefix, parent) in frontier {
            if !at_root {
                kept.push(parent_prefix, parent);
                continue;
            }
            for (child_radix, ours) in parent.into_children() {
                let child_prefix = parent_prefix.push(child_radix);
                let mut on_send = self.on_send.as_mut().map(unknown::from_arc);
                if let Some(ours) = Unknown::unknown(
                    Some(ours),
                    child_prefix,
                    &self.versions.their_version,
                    &mut on_send,
                )
                .await
                {
                    providing.push(child_prefix, ours.clone());
                    matched.push(child_prefix, ours);
                }
            }
        }

        *self.levels.level_mut() = kept;

        Partition {
            providing,
            requested,
            matched,
            exploded,
        }
    }

    /// Run a steady-state round end-to-end: absorb the incoming `providing`,
    /// answer the incoming `requested`, partition the incoming `uncertain`, and
    /// descend the zipper by two heights. Returns the next-level outgoing
    /// `providing` / `requested` / `uncertain` and a descended [`Exchange`],
    /// wrapped in [`protocol::Step::Continue`] or [`protocol::Step::Done`]
    /// according to whether the outgoing message has anything left to
    /// negotiate.
    ///
    /// Shared by [`Self::exchange`] and [`Self::close_initiator`]; they differ
    /// only in how they assemble the outgoing message.
    #[allow(clippy::type_complexity)]
    async fn reply<Request, Response, H, OnRecvFut, OnSendFut>(
        mut self,
        request: Request,
    ) -> protocol::Step<
        Response,
        Exchange<OnSend, OnRecv, Connected, Below<H, Below<S<H>, L>>>,
        tree::Root<L::Message>,
    >
    where
        Request: Into<message::Exchange<L::Message, S<H>>>,
        Response: From<message::Exchange<L::Message, H>>,
        OnRecv: FnMut(Key, &Version, &Arc<L::Message>) -> OnRecvFut,
        OnRecvFut: Future<Output = ()> + Send,
        OnSend: FnMut(Key, &Version, &Arc<L::Message>) -> OnSendFut,
        OnSendFut: Future<Output = ()> + Send,
        L: Levels<Height = S<S<H>>>,
        S<S<H>>: Height,
        S<H>: Height,
        H: Height + Unknown,
    {
        let message::Exchange {
            providing,
            requested,
            uncertain,
        } = request.into();

        // Phase 1: absorb the counterparty's `providing` into our frontier.
        self.absorb_providing(providing).await;

        // Phase 2: answer the counterparty's `requested` set, building the
        // outgoing `providing` map (which Phase 3 may extend with Left-case
        // nodes -- subtrees only we have at the current height).
        let mut providing = self.answer_requested(requested).await;

        // Phase 3: partition the counterparty's `uncertain` set by cell of
        // the asymmetry matrix, then merge its Left-case `providing` with
        // the Phase 2 output.
        let partition = self.partition_uncertain(uncertain).await;
        providing.extend(partition.providing);

        // Descend the zipper by two heights: matched children at S<H>, then
        // exploded grandchildren at H.
        let levels = self.levels.down(partition.matched).down(partition.exploded);
        #[cfg_attr(not(debug_assertions), allow(unused_mut))]
        let mut next = Exchange {
            levels,
            versions: self.versions,
            on_send: self.on_send,
            on_recv: self.on_recv,
            #[cfg(debug_assertions)]
            expected_parents: Default::default(),
        };

        // Compute the hashes of the level returned at the bottom of `next`;
        // these are the children we are uncertain about now. Iterating the
        // sorted level yields ascending prefixes, so the `Vec` is canonical.
        let uncertain: Vec<_> = next
            .levels
            .level()
            .iter()
            .map(|(prefix, node)| (*prefix, node.hash()))
            .collect();

        // Collect the outgoing `providing` level into an ascending `(prefix,
        // node)` `Vec`; the `Level` already holds its entries in prefix order.
        // `partition.requested` is likewise already ascending.
        let response = message::Exchange {
            providing: providing.into_iter().collect(),
            requested: partition.requested,
            uncertain,
        };

        // Record which parents the counterparty may `provide` against in its
        // reply to this message: the prefixes we just `requested`, plus the
        // parents of those we listed as `uncertain` (Left-case siblings).
        #[cfg(debug_assertions)]
        {
            next.expected_parents =
                expected_providing_parents(&response.requested, &response.uncertain);
        }

        // Convergence: nothing left to ask, nothing left in dispute. The
        // outgoing message is still meaningful (it may carry `providing`), so
        // the caller still needs to deliver it.
        let finished = response.requested.is_empty() && response.uncertain.is_empty();
        if finished {
            protocol::Step::Done {
                msg: response.into(),
                output: tree::Root {
                    ceiling: next.versions.our_version | next.versions.their_version,
                    root: next.levels.collapse(),
                },
            }
        } else {
            protocol::Step::Continue {
                msg: response.into(),
                next,
            }
        }
    }
}
