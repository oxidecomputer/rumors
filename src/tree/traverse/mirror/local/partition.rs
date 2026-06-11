//! The reconciliation internals of the local [`Exchange`]: the per-round
//! pipeline ([`Exchange::reply`]) and its three phases — absorb the incoming
//! `providing`, answer the incoming `requested`, and partition the incoming
//! `uncertain` by cell of the asymmetry matrix (see the [`super`] module docs
//! for the matrix and the channel vocabulary).

use std::mem;

use itertools::{EitherOrBoth, Itertools};

use crate::tree::{
    self,
    traverse::unknown::Unknown,
    typed::{
        Hash, Level, Levels, Prefix,
        height::{Height, Root, S},
        levels::Below,
    },
};

use super::{Connected, Exchange, message, protocol};

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

impl<L> Exchange<Connected, L>
where
    L: Levels,
    L::Message: Send + Sync,
{
    /// Insert nodes the counterparty has just sent us (because we requested
    /// them last round, or because they unilaterally knew we lacked them) into
    /// our zipper's bottom level.
    ///
    /// Each subtree arrives as a whole `(prefix, node)` pair, in ascending
    /// prefix order, and is inserted directly at the named prefix.
    pub(super) fn absorb_providing<H>(&mut self, providing: message::Providing<L::Message, H>)
    where
        L::Message: Send + Sync,
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
    pub(super) fn answer_requested<H>(
        &mut self,
        requested: Vec<Prefix<S<H>>>,
    ) -> Level<L::Message, H>
    where
        L: Levels<Height = S<H>>,
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
                if let Some(node) = Unknown::unknown(Some(node), &self.versions.their_version) {
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
    /// Shared by [`open_initiator`](protocol::OpenInitiator::open_initiator),
    /// [`exchange`](protocol::Exchange::exchange), and
    /// [`close_initiator`](protocol::CloseInitiator::close_initiator).
    /// The two "asymmetric root" branches — `else`
    /// (we lack the parent) and the post-loop drain (we have a parent the
    /// counterparty never mentioned) — are reachable only from
    /// `open_initiator`: in steady-state both sides' frontiers were
    /// constructed by Both-case matches in the previous round and therefore
    /// agree on every parent. The debug-assertions guard against a
    /// steady-state caller silently triggering either branch.
    fn partition_uncertain<H>(
        &mut self,
        uncertain: Vec<(Prefix<S<H>>, Hash)>,
    ) -> Partition<L::Message, H>
    where
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
        // out of the frontier at most once. The groups are consumed lazily:
        // each loop iteration fully drains its group (`merge_join_by` and the
        // `else` arm both run their group to exhaustion) before the next group
        // is formed.
        let by_parent = uncertain
            .into_iter()
            .map(|(prefix, hash)| {
                let (parent_prefix, radix) = prefix.pop();
                (parent_prefix, radix, hash)
            })
            .chunk_by(|(parent_prefix, _, _)| *parent_prefix);

        // Drain the frontier in one pass, co-iterating it (ascending) against
        // the ascending `by_parent` groups. Both are sorted, so each parent is
        // matched, kept, or requested in a single linear walk — no per-parent
        // binary-search `remove`. Parents the counterparty did mention are
        // consumed here; parents it never mentioned carry over into `kept` (the
        // rebuilt frontier) unless we are at Root, where they drain below.
        let mut frontier = mem::take(self.levels.level_mut()).into_iter().peekable();
        let mut kept = Level::default();
        for (parent_prefix, uncertain_children) in &by_parent {
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
                            if let Some(ours) =
                                Unknown::unknown(Some(ours), &self.versions.their_version)
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
        // drain it here. The Root guard is required for correctness, not just
        // an assertion: below Root these leftovers are normal (e.g. responder
        // children just carried through `answer_requested`), and turning them
        // into Left cases would re-emit data we already sent.
        for (parent_prefix, parent) in frontier {
            if !at_root {
                kept.push(parent_prefix, parent);
                continue;
            }
            for (child_radix, ours) in parent.into_children() {
                let child_prefix = parent_prefix.push(child_radix);
                if let Some(ours) = Unknown::unknown(Some(ours), &self.versions.their_version) {
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
    /// Shared by [`exchange`](protocol::Exchange::exchange) and
    /// [`close_initiator`](protocol::CloseInitiator::close_initiator); they
    /// differ only in how they assemble the outgoing message.
    #[allow(clippy::type_complexity)]
    pub(super) fn reply<Request, Response, H>(
        mut self,
        request: Request,
    ) -> protocol::Step<
        Response,
        Exchange<Connected, Below<H, Below<S<H>, L>>>,
        tree::Root<L::Message>,
    >
    where
        Request: Into<message::Exchange<L::Message, S<H>>>,
        Response: From<message::Exchange<L::Message, H>>,
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
        #[cfg_attr(not(debug_assertions), allow(unused_mut))]
        let mut next = Exchange {
            levels,
            versions: self.versions,
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
                super::expected_providing_parents(&response.requested, &response.uncertain);
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
