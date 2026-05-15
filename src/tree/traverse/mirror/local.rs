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
        traverse::unknown::Unknown,
        typed::{
            Levels, Node, Prefix,
            height::{Height, Pred, Root, S, Z},
            levels::{Below, Top},
        },
    },
};

use super::message::{self, UnderRoot};

/// An in-progress mirror synchronization on one side of the wire.
///
/// `L` is our zipper, parameterised by the height of its bottom level; as the
/// protocol descends, each [`Self::exchange`] call returns a new `Exchange`
/// whose `L` is two heights below the previous one.
pub struct Exchange<'v, P, F, L>
where
    P: Clone + Ord + AsRef<[u8]>,
{
    /// Our multi-level zipper: agreed heights live near the top, the height
    /// currently under comparison lives at the bottom.
    levels: L,
    /// The counterparty's version vector, used to honor their deletions: any
    /// node of ours at or causally prior to this version that they lack must
    /// have been forgotten on their side.
    their_version: &'v Version<P>,
    /// Invoked whenever the version-vector filter discovers a leaf the
    /// counterparty does not yet know about; lets the embedding code stream out
    /// leaf-level observations as they're discovered.
    on_message: F,
}

/// Begin the protocol as the initiator.
///
/// Returns the opening [`message::Initiate`] (just our root hash) and an
/// `Exchange` whose zipper is at `Top` (height `Root`). The initiator's next
/// call is [`Exchange::open_initiator`], processing the responder's
/// [`message::Opening`].
pub fn initiator<'v, P, F, T>(
    node: Option<Node<P, T, Root>>,
    their_version: &'v Version<P>,
    on_message: F,
) -> (message::Initiate, Exchange<'v, P, F, Top<P, T>>)
where
    F: FnMut(&Version<P>, Key, &Message<T>),
    P: Clone + Ord + AsRef<[u8]>,
    T: Clone,
{
    (
        message::Initiate {
            uncertain: node.iter().map(|n| (Prefix::new(), n.hash())).collect(),
        },
        Exchange {
            levels: Node::levels(node),
            their_version,
            on_message,
        },
    )
}

/// Begin the protocol as the responder, processing the initiator's
/// [`message::Initiate`].
///
/// If our root hash matches the initiator's, we short-circuit: the trees are
/// already equal, so we return `Err(our_root)` and an empty `Opening` to signal
/// completion. Otherwise we explode our root one level down into an
/// [`UnderRoot`]-height zipper and emit its children's hashes as the
/// `Opening`'s `uncertain` set -- unconditionally, since we haven't yet learned
/// what the initiator has.
pub fn responder<P, T, F>(
    node: Option<Node<P, T, Root>>,
    their_version: &Version<P>,
    on_message: F,
    message::Initiate { uncertain }: message::Initiate,
) -> (
    message::Opening,
    Result<Exchange<'_, P, F, Below<P, T, UnderRoot, Top<P, T>>>, Option<Node<P, T, Root>>>,
)
where
    F: FnMut(&Version<P>, Key, &Message<T>),
    P: Clone + Ord + AsRef<[u8]>,
    T: Clone,
{
    // `Initiate.uncertain` is structurally a single entry at the empty root
    // prefix. Treat absence as "empty tree" and let the equality check below
    // handle the symmetric "both empty" case.
    let their_root = uncertain
        .get(&Prefix::new())
        .copied()
        .unwrap_or_else(|| [0; 32].into());

    // If the initiator has the same root hash, our trees must be equal, so
    // there's no need to continue the protocol; however, we need to signal back
    // to the initiator that we're finished.
    if their_root == Node::root_hash(&node) {
        return (message::Opening::default(), Err(node));
    }

    // If the root hashes mismatch, explode our root one level down. The
    // resulting `uncertain` is all the hashes at that level -- we don't yet
    // know which of them the initiator also has.
    let levels = Node::levels(None).down(
        node.map(|n| {
            n.into_children()
                .into_iter()
                .map(|(radix, child)| (Prefix::new().push(radix), child))
                .collect()
        })
        .unwrap_or_default(),
    );
    (
        message::Opening {
            uncertain: levels
                .level()
                .into_iter()
                .map(|(prefix, child)| (prefix.clone(), child.hash()))
                .collect(),
        },
        Ok(Exchange {
            levels,
            their_version,
            on_message,
        }),
    )
}

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

impl<'v, P, F, L> Exchange<'v, P, F, L>
where
    P: Clone + Ord + AsRef<[u8]>,
{
    /// Insert nodes the counterparty has just sent us (because we requested
    /// them last round, or because they unilaterally knew we lacked them) into
    /// our zipper's bottom level.
    fn absorb_providing<H, T>(&mut self, providing: OrdMap<Prefix<H>, Node<P, T, H>>)
    where
        L: Levels<P, T, Height = H>,
        H: Height,
        T: Clone,
    {
        let frontier = self.levels.level_mut();
        for (prefix, node) in providing {
            frontier.insert(prefix, node);
        }
    }

    /// Answer the counterparty's `requested` set by exploding each requested
    /// node into its children, filtered against the counterparty's version so
    /// that any subtrees they have deleted disappear locally too. Returns the
    /// outgoing `providing` map, one height below the frontier.
    fn answer_requested<CH, T>(
        &mut self,
        requested: OrdSet<Prefix<S<CH>>>,
    ) -> OrdMap<Prefix<CH>, Node<P, T, CH>>
    where
        L: Levels<P, T, Height = S<CH>>,
        F: FnMut(&Version<P>, Key, &Message<T>),
        S<CH>: Unknown,
        CH: Height,
        T: Clone,
    {
        let frontier = self.levels.level_mut();
        let mut providing = OrdMap::default();
        for prefix in requested {
            if let Some(node) = frontier.remove(&prefix) {
                // Filter against the counterparty's version: anything causally
                // prior to it that they lack, they have already deleted -- so
                // we should too. The surviving subtree (if any) goes back into
                // our frontier; its children are sent out as `providing`.
                if let Some(node) = Unknown::unknown(
                    Some(node),
                    prefix.clone(),
                    self.their_version,
                    &mut self.on_message,
                ) {
                    frontier.insert(prefix.clone(), node.clone());
                    for (radix, child) in node.into_children() {
                        providing.insert(prefix.clone().push(radix), child);
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
    fn partition_uncertain<H, T>(
        &mut self,
        uncertain: OrdMap<Prefix<S<H>>, blake3::Hash>,
    ) -> Partition<P, T, H>
    where
        L: Levels<P, T, Height = S<S<H>>>,
        F: FnMut(&Version<P>, Key, &Message<T>),
        S<S<H>>: Height,
        S<H>: Height,
        H: Height + Unknown,
        T: Clone,
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
            .chunk_by(|(parent_prefix, _, _)| parent_prefix.clone())
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
                            let child_prefix = parent_prefix.clone().push(child_radix);
                            if let Some(ours) = Unknown::unknown(
                                Some(ours),
                                child_prefix.clone(),
                                self.their_version,
                                &mut self.on_message,
                            ) {
                                providing.insert(child_prefix.clone(), ours.clone());
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
                                    let grandchild_prefix =
                                        child_prefix.clone().push(grandchild_radix);
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
    /// descend the zipper by two heights. Returns the next-level-up outgoing
    /// `providing` / `requested` and a descended [`Exchange`] from which the
    /// caller derives the outgoing `uncertain` (or omits it).
    ///
    /// Shared by [`Self::exchange`] and [`Self::close_initiator`]; they differ
    /// only in how they assemble the outgoing message and detect completion.
    fn step<H, T>(
        mut self,
        providing: OrdMap<Prefix<S<S<H>>>, Node<P, T, S<S<H>>>>,
        requested: OrdSet<Prefix<S<S<H>>>>,
        uncertain: OrdMap<Prefix<S<H>>, blake3::Hash>,
    ) -> (
        OrdMap<Prefix<S<H>>, Node<P, T, S<H>>>,
        OrdSet<Prefix<S<H>>>,
        OrdMap<Prefix<H>, blake3::Hash>,
        Exchange<'v, P, F, Below<P, T, H, Below<P, T, S<H>, L>>>,
    )
    where
        F: FnMut(&Version<P>, Key, &Message<T>),
        L: Levels<P, T, Height = S<S<H>>>,
        S<S<H>>: Height,
        S<H>: Height,
        H: Height + Unknown,
        T: Clone,
    {
        // Phase 1: absorb the counterparty's `providing` into our frontier.
        self.absorb_providing(providing);

        // Phase 2: answer the counterparty's `requested` set, building the
        // outgoing `providing` map (which Phase 3 may extend with Left-case
        // nodes -- subtrees only we have at the current height).
        let mut providing = self.answer_requested(requested);

        // Phase 3: partition the counterparty's `uncertain` set by cell of
        // the asymmetry matrix, then merge its Left-case `providing` with
        // the Phase 2 output.
        let Partition {
            providing: providing_from_left,
            requested,
            matched,
            exploded,
        } = self.partition_uncertain(uncertain);
        providing.extend(providing_from_left);

        // Descend the zipper by two heights: matched children at S<H>, then
        // exploded grandchildren at H.
        let levels = self.levels.down(matched).down(exploded);
        let next = Exchange {
            levels,
            their_version: self.their_version,
            on_message: self.on_message,
        };

        // Compute the hashes of the level returned at the bottom of `next`;
        // these are the children we are uncertain about now.
        let uncertain: OrdMap<_, _> = next
            .levels
            .level()
            .iter()
            .map(|(prefix, node)| (prefix.clone(), node.hash()))
            .collect();

        (providing, requested, uncertain, next)
    }

    /// Process the initiator's first round, applied to the responder's
    /// [`message::Opening`].
    ///
    /// Distinct from [`Self::exchange`] because the opening carries only
    /// `uncertain`, never `providing` or `requested`: the responder enumerates
    /// every child of its root before learning what the initiator has. The
    /// responder may therefore list hashes whose parent (our empty root prefix)
    /// we lack entirely -- a normal case here, but one that would indicate a
    /// protocol bug if it recurred in `Self::exchange`.
    pub fn open_initiator<T>(
        self,
        message::Opening { uncertain }: message::Opening,
    ) -> (
        message::Exchange<P, T, <UnderRoot as Pred>::Pred>,
        Result<
            Exchange<'v, P, F, Below<P, T, <UnderRoot as Pred>::Pred, Below<P, T, UnderRoot, L>>>,
            Option<Node<P, T, Root>>,
        >,
    )
    where
        F: FnMut(&Version<P>, Key, &Message<T>),
        L: Levels<P, T, Height = Root>,
        T: Clone,
    {
        // `Opening` carries only `uncertain`: pass empty `providing` and
        // `requested` to `step`, reducing its absorb/answer phases to no-ops
        // and leaving just `partition_uncertain` + the two-level descent. From
        // there, the assembly mirrors `Self::exchange`: derive the outgoing
        // `uncertain` from the new bottom level and decide whether we're done.
        let (providing, requested, uncertain, next) = self.step::<<UnderRoot as Pred>::Pred, T>(
            OrdMap::default(),
            OrdSet::default(),
            uncertain,
        );

        let finished = uncertain.is_empty() && requested.is_empty();
        let next = if finished {
            Err(next.levels.collapse())
        } else {
            Ok(next)
        };

        let message = message::Exchange {
            providing,
            requested,
            uncertain,
        };

        (message, next)
    }

    /// Process one round of the protocol's steady state, as either party.
    ///
    /// Each call moves our zipper down by two heights and emits the next
    /// outgoing message. The returned `Result` is `Err(final_tree)` once we
    /// have nothing left to ask about and nothing left in dispute -- but the
    /// outgoing message is sent unconditionally, because the counterparty may
    /// still need its contents to converge.
    pub fn exchange<H, T>(
        self,
        message::Exchange {
            providing,
            requested,
            uncertain,
        }: message::Exchange<P, T, S<H>>,
    ) -> (
        message::Exchange<P, T, H>,
        Result<Exchange<'v, P, F, Below<P, T, H, Below<P, T, S<H>, L>>>, Option<Node<P, T, Root>>>,
    )
    where
        F: FnMut(&Version<P>, Key, &Message<T>),
        L: Levels<P, T, Height = S<S<H>>>,
        S<S<H>>: Height,
        S<H>: Height,
        H: Height + Unknown,
        T: Clone,
    {
        let (providing, requested, uncertain, next) = self.step(providing, requested, uncertain);

        // We are finished only when we have nothing left to ask about and
        // nothing left in dispute. (The converse does not hold: an empty new
        // bottom level alone does not mean we're done -- we might still have
        // outstanding `requested` prefixes whose answers we need.)
        let finished = uncertain.is_empty() && requested.is_empty();
        let next = if finished {
            Err(next.levels.collapse())
        } else {
            Ok(next)
        };

        // We send our outgoing message regardless of whether we ourselves are
        // finished: the counterparty may still need its contents to converge.
        let message = message::Exchange {
            providing,
            requested,
            uncertain,
        };

        (message, next)
    }

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
    pub fn close_initiator<T>(
        self,
        message::Exchange {
            providing,
            requested,
            uncertain,
        }: message::Exchange<P, T, S<Z>>,
    ) -> (
        message::Closing<P, T>,
        Result<Exchange<'v, P, F, Below<P, T, Z, Below<P, T, S<Z>, L>>>, Option<Node<P, T, Root>>>,
    )
    where
        F: FnMut(&Version<P>, Key, &Message<T>),
        L: Levels<P, T, Height = S<S<Z>>>,
        T: Clone,
    {
        let (providing, requested, uncertain, next) = self.step(providing, requested, uncertain);

        // We know that the uncertain set should be empty, because we should
        // never be uncertain about a leaf hash; after all, leaf hashes are
        // always 0xff...
        debug_assert!(
            uncertain.is_empty(),
            "uncertain set non-empty when closing initiator"
        );

        // No outgoing `uncertain` (omitted from `Closing`). We are finished
        // only when we have nothing left to ask about.
        let finished = requested.is_empty();
        let next = if finished {
            Err(next.levels.collapse())
        } else {
            Ok(next)
        };

        let message = message::Closing {
            providing,
            requested,
        };

        (message, next)
    }

    /// The responder's final round, processing the initiator's
    /// [`message::Closing`].
    ///
    /// We absorb the initiator's last batch of nodes, answer any final
    /// `requested` set, and collapse our zipper back to a root. The returned
    /// [`message::Complete`] carries our last outgoing `providing` for the
    /// initiator to absorb in [`Self::complete_initiator`].
    pub fn complete_responder<T>(
        mut self,
        message::Closing {
            providing,
            requested,
        }: message::Closing<P, T>,
    ) -> (
        message::Complete<P, T>,
        Result<Infallible, Option<Node<P, T, Root>>>,
    )
    where
        F: FnMut(&Version<P>, Key, &Message<T>),
        L: Levels<P, T, Height = S<Z>>,
        T: Clone,
    {
        self.absorb_providing(providing);
        let providing = self.answer_requested(requested);
        (message::Complete { providing }, Err(self.levels.collapse()))
    }

    /// The initiator's final round.
    ///
    /// Absorbs the responder's last batch of `providing` (from
    /// [`message::Complete`]) and collapses our zipper back to a root. There
    /// is no outgoing message: any `requested` we would have made went out
    /// in our prior [`Self::close_initiator`] call.
    pub fn complete_initiator<T>(
        mut self,
        message::Complete { providing }: message::Complete<P, T>,
    ) -> Result<Infallible, Option<Node<P, T, Root>>>
    where
        L: Levels<P, T, Height = Z>,
        P: Clone + Ord + AsRef<[u8]>,
        T: Clone,
    {
        self.absorb_providing(providing);
        Err(self.levels.collapse())
    }
}
