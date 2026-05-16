//! Two replicas reconcile their trees while honoring deletions: leaves one side
//! has and the other has merely *forgotten* (their version is `<=` the other's
//! version vector) vanish; leaves never seen are transmitted. The protocol
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
//! dispute live at the bottom. After roughly 16 rounds, both bottoms have
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
//! |                | counterparty has it                                  | counterparty lacks it                         |
//! |----------------|------------------------------------------------------|-----------------------------------------------|
//! | **we have it** | hashes match: drop; hashes differ: recurse one finer | we `provide`                                  |
//! | **we lack it** | we `request`                                         | (impossible: neither side would mention it)   |
//!
//! Each cell is realized by one arm of the `merge_join_by` inside
//! [`Exchange::partition_uncertain`].

use std::convert::Infallible;

use imbl::{OrdMap, OrdSet};
use itertools::{EitherOrBoth, Itertools};

use crate::{
    Key, Message, Version,
    tree::{
        traverse::{Paths, get::Get, unknown::Unknown},
        typed::{
            Levels, Node, Prefix,
            height::{Height, Root, S, Z},
            levels::{Below, Top},
        },
    },
};

use super::message::{self, UnderRoot, UnderUnderRoot};
use super::protocol;

/// An in-progress mirror synchronization on one side of the wire.
///
/// `L` is our zipper, parameterised by the height of its bottom level; as the
/// protocol descends, each [`Self::exchange`] call returns a new `Exchange`
/// whose `L` is two heights below the previous one.
pub struct Exchange<'v, OnRecv, OnSend, L>
where
    L: Levels,
    L::Party: Clone + Ord + AsRef<[u8]>,
{
    /// Our multi-level zipper: agreed heights live near the top, the height
    /// currently under comparison lives at the bottom.
    levels: L,
    /// The counterparty's version vector, used to honor their deletions: any
    /// node of ours at or causally prior to this version that they lack must
    /// have been forgotten on their side.
    their_version: &'v Version<L::Party>,
    /// Invoked whenever we discover a leaf that was previously unknown to us;
    /// lets the calling code stream out leaf-level observations as we find
    /// out about them.
    on_recv: OnRecv,
    /// Invoked whenever the version-vector filter discovers a leaf the
    /// counterparty does not yet know about; lets the calling code stream out
    /// leaf-level observations as they're discovered by the counterparty.
    on_send: OnSend,
}

impl<'v, OnRecv, OnSend, P, T> Exchange<'v, OnRecv, OnSend, Top<P, T>>
where
    P: Clone + Ord + AsRef<[u8]>,
    T: Clone,
{
    pub fn new(
        node: Option<Node<P, T, Root>>,
        their_version: &'v Version<P>,
        on_recv: OnRecv,
        on_send: OnSend,
    ) -> Self {
        Self {
            levels: Node::levels(node),
            their_version,
            on_recv,
            on_send,
        }
    }
}

// We define a local `Exchage`'s participation in the protocol as such:

impl<'v, OnRecv, OnSend, L> protocol::Stage for Exchange<'v, OnRecv, OnSend, L>
where
    L: Levels,
    L::Party: Clone + Ord + AsRef<[u8]>,
    L::Message: Clone,
    OnRecv: FnMut(&Version<L::Party>, Key, &Message<L::Message>),
    OnSend: FnMut(&Version<L::Party>, Key, &Message<L::Message>),
{
    type Height = L::Height;
    type Output = Option<Node<L::Party, L::Message, Root>>;
}

impl<'v, P, T, OnRecv, OnSend> protocol::Initiator<P, T> for Exchange<'v, OnRecv, OnSend, Top<P, T>>
where
    P: Clone + Ord + AsRef<[u8]>,
    T: Clone,
    OnRecv: FnMut(&Version<P>, Key, &Message<T>),
    OnSend: FnMut(&Version<P>, Key, &Message<T>),
{
    type Next = Exchange<'v, OnRecv, OnSend, Top<P, T>>;

    fn initiator(self) -> (message::Initiate, Self::Next) {
        let message = message::Initiate {
            uncertain: self
                .levels
                .level()
                .iter()
                .map(|(prefix, node)| (*prefix, node.hash()))
                .collect(),
        };

        (message, self)
    }
}

impl<'v, P, T, OnRecv, OnSend> protocol::Responder<P, T> for Exchange<'v, OnRecv, OnSend, Top<P, T>>
where
    P: Clone + Ord + AsRef<[u8]>,
    T: Clone,
    OnRecv: FnMut(&Version<P>, Key, &Message<T>),
    OnSend: FnMut(&Version<P>, Key, &Message<T>),
{
    type Next = Exchange<'v, OnRecv, OnSend, Below<UnderRoot, Top<P, T>>>;

    fn responder(
        mut self,
        request: message::Initiate,
    ) -> (
        message::Opening,
        Result<Self::Next, Option<Node<P, T, Root>>>,
    ) {
        // `Initiate.uncertain` is structurally a single entry at the empty root
        // prefix. Treat absence as "empty tree" and let the equality check below
        // handle the symmetric "both empty" case.
        let their_root = request
            .uncertain
            .get(&Prefix::new())
            .copied()
            .unwrap_or_else(|| [0; 32].into());

        // We're at the top-most level, so we can use a similar trick to identify
        // our own root hash as the singular inhabitant of the level, if any:
        let our_root = self
            .levels
            .level()
            .get(&Prefix::new())
            .map(Node::hash)
            .unwrap_or_else(|| [0; 32].into());

        // If the initiator has the same root hash, our trees must be equal, so
        // there's no need to continue the protocol; however, we need to signal back
        // to the initiator that we're finished.
        if their_root == our_root {
            return (message::Opening::default(), Err(self.levels.collapse()));
        }

        // If the root hashes mismatch, explode our root one level down. The
        // resulting `uncertain` is all the hashes at that level -- we don't yet
        // know which of them the initiator also has.
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

        let message = message::Opening {
            uncertain: levels
                .level()
                .into_iter()
                .map(|(prefix, child)| (*prefix, child.hash()))
                .collect(),
        };

        let next = Ok(Exchange {
            levels,
            their_version: self.their_version,
            on_recv: self.on_recv,
            on_send: self.on_send,
        });

        (message, next)
    }
}

impl<'v, P, T, OnRecv, OnSend, L> protocol::OpenInitiator<P, T> for Exchange<'v, OnRecv, OnSend, L>
where
    P: Clone + Ord + AsRef<[u8]>,
    T: Clone,
    OnRecv: FnMut(&Version<P>, Key, &Message<T>),
    OnSend: FnMut(&Version<P>, Key, &Message<T>),
    L: Levels<Party = P, Message = T, Height = Root>,
{
    type Next = Exchange<'v, OnRecv, OnSend, Below<UnderUnderRoot, Below<UnderRoot, L>>>;

    fn open_initiator(
        self,
        request: message::Opening,
    ) -> (
        message::Exchange<P, T, UnderUnderRoot>,
        Result<Self::Next, Option<Node<P, T, Root>>>,
    ) {
        self.reply(request)
    }
}

impl<'v, P, T, H, OnRecv, OnSend, L> protocol::Exchange<P, T> for Exchange<'v, OnRecv, OnSend, L>
where
    P: Clone + Ord + AsRef<[u8]>,
    T: Clone,
    OnRecv: FnMut(&Version<P>, Key, &Message<T>),
    OnSend: FnMut(&Version<P>, Key, &Message<T>),
    L: Levels<Party = P, Message = T, Height = S<S<H>>>,
    S<S<H>>: Height,
    S<H>: Height,
    H: Height + Unknown + Get,
    // Assumed at impl-validation time so we don't have to case-analyze `H`
    // here: at use sites `H` is concrete and one of the three blanket impls
    // discharges it.
    Exchange<'v, OnRecv, OnSend, Below<H, Below<S<H>, L>>>: protocol::AfterExchange<P, T, H>,
{
    type Next = Exchange<'v, OnRecv, OnSend, Below<H, Below<S<H>, L>>>;

    fn exchange(
        self,
        request: message::Exchange<P, T, S<H>>,
    ) -> (
        message::Exchange<P, T, H>,
        Result<Self::Next, Option<Node<P, T, Root>>>,
    ) {
        self.reply(request)
    }
}

impl<'v, P, T, OnRecv, OnSend, L> protocol::CloseInitiator<P, T> for Exchange<'v, OnRecv, OnSend, L>
where
    P: Clone + Ord + AsRef<[u8]>,
    T: Clone,
    OnRecv: FnMut(&Version<P>, Key, &Message<T>),
    OnSend: FnMut(&Version<P>, Key, &Message<T>),
    L: Levels<Party = P, Message = T, Height = S<S<Z>>>,
{
    type Next = Exchange<'v, OnRecv, OnSend, Below<Z, Below<S<Z>, L>>>;

    fn close_initiator(
        self,
        request: message::Exchange<P, T, S<Z>>,
    ) -> (
        message::Closing<P, T>,
        Result<Self::Next, Option<Node<P, T, Root>>>,
    ) {
        self.reply(request)
    }
}

impl<'v, P, T, OnRecv, OnSend, L> protocol::CompleteResponder<P, T>
    for Exchange<'v, OnRecv, OnSend, L>
where
    P: Clone + Ord + AsRef<[u8]>,
    T: Clone,
    OnRecv: FnMut(&Version<P>, Key, &Message<T>),
    OnSend: FnMut(&Version<P>, Key, &Message<T>),
    L: Levels<Party = P, Message = T, Height = S<Z>>,
{
    fn complete_responder(
        mut self,
        request: message::Closing<P, T>,
    ) -> (
        message::Complete<P, T>,
        Result<Infallible, Option<Node<P, T, Root>>>,
    ) {
        self.absorb_providing(request.providing);
        let providing = self.answer_requested(request.requested);
        (message::Complete { providing }, Err(self.levels.collapse()))
    }
}

impl<'v, P, T, OnRecv, OnSend, L> protocol::CompleteInitiator<P, T>
    for Exchange<'v, OnRecv, OnSend, L>
where
    P: Clone + Ord + AsRef<[u8]>,
    T: Clone,
    OnRecv: FnMut(&Version<P>, Key, &Message<T>),
    OnSend: FnMut(&Version<P>, Key, &Message<T>),
    L: Levels<Party = P, Message = T, Height = Z>,
{
    fn complete_initiator(
        mut self,
        request: message::Complete<P, T>,
    ) -> Result<Infallible, Option<Node<P, T, Root>>> {
        self.absorb_providing(request.providing);
        Err(self.levels.collapse())
    }
}

// Internal implementation of methods on `Exchange` involved in the protocol:

/// The output of [`Exchange::partition_uncertain`], one field per outgoing
/// channel in the asymmetry matrix.
struct Partition<P, T, H>
where
    P: Clone + Ord + AsRef<[u8]>,
    S<H>: Height,
    H: Height,
{
    /// Left-case subtrees (we have them, the counterparty does not). The caller
    /// will combine these with `answer_requested`'s output to form the final
    /// outgoing `providing`.
    providing: OrdMap<Prefix<S<H>>, Node<P, T, S<H>>>,
    /// Right-case prefixes (the counterparty has them, we do not): the outgoing
    /// `requested`.
    requested: OrdSet<Prefix<S<H>>>,
    /// `Both`-case children whose hashes agreed, plus Left-case children we
    /// kept locally. Become the new level immediately above the bottom.
    matched: OrdMap<Prefix<S<H>>, Node<P, T, S<H>>>,
    /// `Both`-case grandchildren of children whose hashes disagreed. Become the
    /// new bottom of the zipper, and next round's outgoing `uncertain`.
    exploded: OrdMap<Prefix<H>, Node<P, T, H>>,
}

impl<'v, OnSend, OnRecv, L> Exchange<'v, OnRecv, OnSend, L>
where
    L: Levels,
    L::Party: Clone + Ord + AsRef<[u8]>,
    L::Message: Clone,
{
    /// Insert nodes the counterparty has just sent us (because we requested
    /// them last round, or because they unilaterally knew we lacked them) into
    /// our zipper's bottom level.
    fn absorb_providing<H>(&mut self, providing: OrdMap<Prefix<H>, Node<L::Party, L::Message, H>>)
    where
        OnRecv: FnMut(&Version<L::Party>, Key, &Message<L::Message>),
        L: Levels<Height = H>,
        H: Height + Get,
    {
        let frontier = self.levels.level_mut();
        for (prefix, node) in providing {
            Get::get(Some(node.clone()), prefix, Paths::All, &mut self.on_recv);
            frontier.insert(prefix, node);
        }
    }

    /// Answer the counterparty's `requested` set by exploding each requested
    /// node into its children, filtered against the counterparty's version so
    /// that any subtrees they have deleted disappear locally too. Returns the
    /// outgoing `providing` map, one height below the frontier.
    fn answer_requested<H>(
        &mut self,
        requested: OrdSet<Prefix<S<H>>>,
    ) -> OrdMap<Prefix<H>, Node<L::Party, L::Message, H>>
    where
        L: Levels<Height = S<H>>,
        OnSend: FnMut(&Version<L::Party>, Key, &Message<L::Message>),
        S<H>: Unknown,
        H: Height,
    {
        let frontier = self.levels.level_mut();
        let mut providing = OrdMap::default();
        for prefix in requested {
            if let Some(node) = frontier.remove(&prefix) {
                // Filter against the counterparty's version: anything causally
                // prior to it that they lack, they have already deleted -- so
                // we should too. The surviving subtree (if any) goes back into
                // our frontier; its children are sent out as `providing`.
                if let Some(node) =
                    Unknown::unknown(Some(node), prefix, self.their_version, &mut self.on_send)
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
    /// [`Self::close_initiator`]. The "we lack the parent" branch is reachable
    /// only from `open_initiator` (where the responder lists children of our
    /// absent root unconditionally); the debug-assertion guards against a
    /// steady-state caller silently triggering it at any incorrect height.
    fn partition_uncertain<H>(
        &mut self,
        uncertain: OrdMap<Prefix<S<H>>, blake3::Hash>,
    ) -> Partition<L::Party, L::Message, H>
    where
        OnSend: FnMut(&Version<L::Party>, Key, &Message<L::Message>),
        L: Levels<Height = S<S<H>>>,
        S<S<H>>: Height,
        S<H>: Height,
        H: Height + Unknown,
    {
        let frontier = self.levels.level_mut();
        let mut providing = OrdMap::default();
        let mut requested = OrdSet::default();
        let mut matched = OrdMap::default();
        let mut exploded = OrdMap::default();

        // Group the uncertain prefixes by their parent, so we pull each parent
        // out of the frontier at most once.
        for (parent_prefix, uncertain_children) in uncertain
            .into_iter()
            .map(|(prefix, hash)| {
                let (parent_prefix, radix) = prefix.pop();
                (parent_prefix, radix, hash)
            })
            .chunk_by(|(parent_prefix, _, _)| *parent_prefix)
            .into_iter()
        {
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
                            if let Some(ours) = Unknown::unknown(
                                Some(ours),
                                child_prefix,
                                self.their_version,
                                &mut self.on_send,
                            ) {
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
    /// `providing` / `requested` / `uncertain` and a descended [`Exchange`].
    ///
    /// Shared by [`Self::exchange`] and [`Self::close_initiator`]; they differ
    /// only in how they assemble the outgoing message and detect completion.
    #[allow(clippy::type_complexity)]
    fn reply<Request, Response, H>(
        mut self,
        request: Request,
    ) -> (
        Response,
        Result<
            Exchange<'v, OnRecv, OnSend, Below<H, Below<S<H>, L>>>,
            Option<Node<L::Party, L::Message, Root>>,
        >,
    )
    where
        Request: Into<message::Exchange<L::Party, L::Message, S<H>>>,
        Response: From<message::Exchange<L::Party, L::Message, H>>,
        OnRecv: FnMut(&Version<L::Party>, Key, &Message<L::Message>),
        OnSend: FnMut(&Version<L::Party>, Key, &Message<L::Message>),
        L: Levels<Height = S<S<H>>>,
        S<S<H>>: Height,
        S<H>: Height,
        H: Height + Unknown + Get,
    {
        let message::Exchange {
            providing,
            requested,
            uncertain,
        } = request.into();

        // Phase 1: absorb the counterparty's `providing` into our frontier.
        self.absorb_providing(providing);

        // Phase 2: answer the counterparty's `requested` set, building the
        // outgoing `providing` map (which Phase 3 may extend with Left-case
        // nodes -- subtrees only we have at the current height).
        let mut providing = self.answer_requested(requested);

        // Phase 3: partition the counterparty's `uncertain` set by cell of
        // the asymmetry matrix, then merge its Left-case `providing` with
        // the Phase 2 output.
        let partition = self.partition_uncertain(uncertain);
        providing.extend(partition.providing);

        // Descend the zipper by two heights: matched children at S<H>, then
        // exploded grandchildren at H.
        let levels = self.levels.down(partition.matched).down(partition.exploded);
        let next = Exchange {
            levels,
            their_version: self.their_version,
            on_send: self.on_send,
            on_recv: self.on_recv,
        };

        // Compute the hashes of the level returned at the bottom of `next`;
        // these are the children we are uncertain about now.
        let uncertain: OrdMap<_, _> = next
            .levels
            .level()
            .iter()
            .map(|(prefix, node)| (*prefix, node.hash()))
            .collect();

        let response = message::Exchange {
            providing,
            requested: partition.requested,
            uncertain,
        };

        // Determine if we are finished
        let finished = response.requested.is_empty() && response.uncertain.is_empty();
        let next = if finished {
            Err(next.levels.collapse())
        } else {
            Ok(next)
        };

        (response.into(), next)
    }
}
