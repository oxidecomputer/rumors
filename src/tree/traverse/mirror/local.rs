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

use std::collections::{BTreeMap, BTreeSet};

use itertools::{EitherOrBoth, Itertools};

use crate::{
    message::Message,
    tree::{
        self,
        key::Key,
        traverse::{unknown, unknown::Unknown},
        typed::{
            Hash, Levels, Node, Prefix,
            height::{Height, Root, S, Z},
            levels::{Below, Top},
        },
    },
    version::Version,
};

use super::message::{self, UnderRoot, UnderUnderRoot};
use super::protocol;
use super::reassemble::{BuildNode, flatten_providing, reassemble_providing};

/// The version state for an [`Exchange`] which has just been initialized but
/// has not yet connected.
pub struct Start {
    our_version: Version,
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
}

impl<OnSend, OnRecv, T> Exchange<OnSend, OnRecv, Start, Top<T>>
where
    T: Send + Sync,
{
    pub fn start(node: tree::Root<T>, on_send: Option<OnSend>, on_recv: Option<OnRecv>) -> Self {
        Self {
            versions: Start {
                our_version: node.ceiling.clone(),
            },
            levels: Node::levels(Option::from(node)),
            on_recv,
            on_send,
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
        Self::start(node, None, None)
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

    async fn connect(self) -> Result<protocol::Step<Version, Self::Next, Infallible>, Self::Error> {
        let our_version = self.versions.our_version;

        let next = Exchange {
            levels: self.levels,
            versions: Connecting {
                our_version: our_version.clone(),
            },
            on_recv: self.on_recv,
            on_send: self.on_send,
        };

        Ok(protocol::Step::Continue {
            msg: our_version,
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
        their_version: Version,
    ) -> Result<protocol::Step<Version, Self::Next, Self::Output>, Self::Error> {
        let our_version = self.versions.our_version;

        // If the two versions are the same, both sides are immediately done
        if our_version == their_version {
            return Ok(protocol::Step::Done {
                msg: our_version.clone(),
                output: tree::Root {
                    ceiling: our_version.clone(),
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
        };

        Ok(protocol::Step::Continue {
            msg: our_version,
            next,
        })
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
    H: Height + Unknown + BuildNode,
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
                providing: flatten_providing(providing),
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
    providing: BTreeMap<Prefix<S<H>>, Node<T, S<H>>>,
    /// Right-case prefixes (the counterparty has them, we do not): the outgoing
    /// `requested`.
    requested: BTreeSet<Prefix<S<H>>>,
    /// `Both`-case children whose hashes agreed, plus Left-case children we
    /// kept locally. Become the new level immediately above the bottom.
    matched: BTreeMap<Prefix<S<H>>, Node<T, S<H>>>,
    /// `Both`-case grandchildren of children whose hashes disagreed. Become the
    /// new bottom of the zipper, and next round's outgoing `uncertain`.
    exploded: BTreeMap<Prefix<H>, Node<T, H>>,
}

impl<OnSend, OnRecv, L> Exchange<OnSend, OnRecv, Connected, L>
where
    L: Levels,
    L::Message: Send + Sync,
    OnSend: Send,
    OnRecv: Send,
{
    /// Insert leaves the counterparty has just sent us (because we requested
    /// them last round, or because they unilaterally knew we lacked them) into
    /// our zipper's bottom level.
    ///
    /// The wire carries only the leaves; we re-materialize the subtrees at this
    /// height by recomputing each leaf's content-addressed path
    /// ([`reassemble_providing`]), so a leaf can only land where its content
    /// hashes to.
    async fn absorb_providing<H, OnRecvFut>(
        &mut self,
        providing: Vec<(Version, Message<L::Message>)>,
    ) where
        L::Message: Send + Sync,
        OnRecv: FnMut(Key, &Version, &Arc<L::Message>) -> OnRecvFut + Send,
        OnRecvFut: Future<Output = ()> + Send,
        L: Levels<Height = H>,
        H: BuildNode,
    {
        let providing = reassemble_providing::<L::Message, H>(providing);

        // Only walk a just-absorbed subtree to fire `on_recv` when there is an
        // `on_recv` to fire: with no callback the walk is pure waste (it does
        // nothing but invoke a no-op per leaf), and skipping it is the dominant
        // saving for callback-less `learn`/`join`. The frontier insert is
        // unconditional: later rounds re-explode from it regardless.
        if let Some(on_recv) = self.on_recv.as_mut() {
            let frontier = self.levels.level_mut();
            for (prefix, node) in providing {
                // Fire `on_recv` for every leaf via a read-only walk, then move
                // the subtree itself into the frontier: no clone, and the
                // subtree's memoized hash/ceiling/floor survive intact.
                for (key, version, message) in node.leaves(prefix) {
                    on_recv(key, version, message.as_arc()).await;
                }
                frontier.insert(prefix, node);
            }
        } else {
            let frontier = self.levels.level_mut();
            for (prefix, node) in providing {
                frontier.insert(prefix, node);
            }
        }
    }

    /// Answer the counterparty's `requested` set by exploding each requested
    /// node into its children, filtered against the counterparty's version so
    /// that any subtrees they have deleted disappear locally too. Returns the
    /// outgoing `providing` map, one height below the frontier.
    async fn answer_requested<H, OnSendFut>(
        &mut self,
        requested: Vec<Prefix<S<H>>>,
    ) -> BTreeMap<Prefix<H>, Node<L::Message, H>>
    where
        L: Levels<Height = S<H>>,
        OnSend: FnMut(Key, &Version, &Arc<L::Message>) -> OnSendFut,
        OnSendFut: Future<Output = ()> + Send,
        S<H>: Unknown,
        H: Height,
    {
        let frontier = self.levels.level_mut();
        let mut providing = BTreeMap::default();
        for prefix in requested {
            if let Some(node) = frontier.remove(&prefix) {
                // Filter against the counterparty's version: anything causally
                // prior to it that they lack, they have already deleted -- so
                // we should too. The surviving subtree (if any) goes back into
                // our frontier; its children are sent out as `providing`.
                let mut on_send = self.on_send.as_mut().map(unknown::from_arc);
                if let Some(node) = Unknown::unknown(
                    Some(node),
                    prefix,
                    &self.versions.their_version,
                    &mut on_send,
                )
                .await
                {
                    frontier.insert(prefix, node.clone());
                    for (radix, child) in node.into_children() {
                        providing.insert(prefix.push(radix), child);
                    }
                }
            } else {
                // The counterparty should only request prefixes we previously
                // listed as `uncertain`; otherwise either the counterparty is
                // misbehaving, or we are.
                #[cfg(debug_assertions)]
                panic!("counterparty requested unknown prefix {:?}", prefix);
            }
        }
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
        let frontier = self.levels.level_mut();
        let mut providing = BTreeMap::default();
        let mut requested = BTreeSet::default();
        let mut matched = BTreeMap::default();
        let mut exploded = BTreeMap::default();

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
        for (parent_prefix, uncertain_children) in by_parent {
            if let Some(parent) = frontier.remove(&parent_prefix) {
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
                                providing.insert(child_prefix, ours.clone());
                                matched.insert(child_prefix, ours);
                            }
                        }
                        // We both have it: drop on hash match, otherwise
                        // recurse one level finer by exploding our copy into
                        // the bottom-most level for the next round.
                        Both((child_radix, ours), (parent_prefix, _, theirs)) => {
                            let child_prefix = parent_prefix.push(child_radix);
                            if ours.hash() == theirs {
                                matched.insert(child_prefix, ours);
                            } else {
                                for (grandchild_radix, grandchild) in ours.into_children() {
                                    let grandchild_prefix = child_prefix.push(grandchild_radix);
                                    exploded.insert(grandchild_prefix, grandchild);
                                }
                            }
                        }
                        // We lack it, they have it: request it.
                        Right((parent_prefix, hash_radix, _)) => {
                            requested.insert(parent_prefix.push(hash_radix));
                        }
                    }
                }
            } else {
                debug_assert_eq!(
                    <L::Height as Height>::HEIGHT,
                    <Root as Height>::HEIGHT,
                    "counterparty indicated uncertainty about unknown parent \
                    prefix {:?} outside of the initiator's first round",
                    parent_prefix,
                );
                for (parent, hash_radix, _) in uncertain_children {
                    requested.insert(parent.push(hash_radix));
                }
            }
        }

        // Symmetric counterpart of the `else` branch above: any parent still
        // sitting in our frontier is one the counterparty never mentioned.
        // From their omission we infer they lack it entirely, so every one of
        // its children is a "Left" case (we have, they lack).
        //
        // Same reachability restriction as the `else` branch: only the
        // initiator's first round can drive this. In steady-state both sides'
        // frontiers were constructed from the previous round's Both-case
        // matches, so the counterparty would always have mentioned every
        // parent we hold. The Root-height guard is *load-bearing*, not just an
        // assertion: at lower heights the same frontier entries are normal,
        // expected leftovers (e.g., responder children we've just re-inserted
        // via `answer_requested`), and turning them into Left cases would
        // re-emit data we already sent.
        if <L::Height as Height>::HEIGHT == <Root as Height>::HEIGHT {
            for (parent_prefix, parent) in mem::take(frontier).into_iter() {
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
                        providing.insert(child_prefix, ours.clone());
                        matched.insert(child_prefix, ours);
                    }
                }
            }
        }

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
        H: Height + Unknown + BuildNode,
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
        let next = Exchange {
            levels,
            versions: self.versions,
            on_send: self.on_send,
            on_recv: self.on_recv,
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

        // Flatten the outgoing `providing` map to its leaves (in ascending path
        // order) and the `requested` set to an ascending `Vec`.
        let response = message::Exchange {
            providing: flatten_providing(providing),
            requested: partition.requested.into_iter().collect(),
            uncertain,
        };

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
