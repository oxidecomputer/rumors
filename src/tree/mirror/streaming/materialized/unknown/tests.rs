//! The streaming [`unknown`] prune must agree with the in-memory
//! [`Unknown`](crate::tree::traverse::unknown::Unknown) oracle, and with a
//! leaf-by-leaf causal sieve that reads no subtree bounds.

use std::collections::BTreeSet;

use proptest::collection::vec;
use proptest::prelude::*;

use crate::tree::mirror::streaming::stats::Recorder;
use crate::{
    Version,
    counter::widen,
    message::Message,
    network::NetworkKeys,
    tree::{
        Tree,
        arb::{arb_divergent_roots, leaf_sibling_path, nth_party, roots_at_depth},
        mirror::contained,
        mirror::streaming::{
            Backend, ErasedNode, Local,
            erased::ops,
            materialized::unknown::{unknown, unknown_providing},
        },
        traverse::{Action, act, unknown::Unknown},
        typed::{self, HashKey, Path, Prefix, height::Root},
    },
};

/// Build a root from `flags_a.len()` party-0 leaves and `flags_b.len()` party-1
/// leaves, inserted with strictly ascending versions, plus a `known` version
/// that is the join of the leaf versions flagged `true`.
///
/// Flagged leaves fall causally at or before `known` (the prune must drop
/// them); the rest are concurrent with or after it (the prune must keep them).
/// Splitting leaves across two parties guarantees cross-party concurrency, so
/// the "floor concurrent, keep whole subtree" fast path is exercised alongside
/// the drop path.
fn tree_and_known(flags_a: &[bool], flags_b: &[bool]) -> (Option<typed::node::Root>, Version) {
    let mut actions: Vec<(Path, Version, Action)> = Vec::new();
    let mut known = Version::new();

    for (party_index, flags) in [(0, flags_a), (1, flags_b)] {
        let party = nth_party(party_index);
        let mut version = Version::new();
        for &flagged in flags {
            version.tick(&party);
            let message = Message::new(());
            let path = Path::for_fixture_leaf(&version);
            actions.push((path, version.clone(), Action::Insert(message)));
            if flagged {
                known |= version.clone();
            }
        }
    }

    (act(None, actions, &mut |_, _| ()), known)
}

/// Build a root of `flags.len()` sibling leaves under one 31-byte prefix,
/// each on its own party, plus a `known` version that is the join of the
/// leaf versions flagged `true`.
///
/// The leaves are mutually concurrent, so with both flag values present
/// the parent's span classifies as mixed and the prune descends to judge
/// each leaf on its own: the one shape in which the leaf-height verdict
/// decides.
fn sibling_tree_and_known(flags: &[bool]) -> (Option<typed::node::Root>, Version) {
    let mut actions: Vec<(Path, Version, Action)> = Vec::new();
    let mut known = Version::new();
    for (index, &flagged) in flags.iter().enumerate() {
        let mut version = Version::new();
        version.tick(&nth_party(index));
        actions.push((
            leaf_sibling_path(index as u8),
            version.clone(),
            Action::Insert(Message::new(())),
        ));
        if flagged {
            known |= version;
        }
    }
    (act(None, actions, &mut |_, _| ()), known)
}

/// Prune an optional root through the single-node streaming filter, driving
/// the future to completion with a trivial executor.
fn stream_prune(root: Option<typed::node::Root>, known: &Version) -> Option<typed::node::Root> {
    root.and_then(|node| {
        pollster::block_on(unknown::<Local>(
            &Local,
            known,
            Prefix::new().erase(),
            <Local as Backend>::erase(node.into()),
            &Recorder::default(),
        ))
        .expect("the in-memory backend lists exactly the children it reports")
        .map(|pruned| <Local as Backend>::assume(&Prefix::<Root>::new(), pruned).into_node())
    })
}

proptest! {
    /// The streamed prune and the in-memory prune produce the same
    /// tree (equal Merkle roots) for every tree and every `known` version.
    #[test]
    fn agrees_with_in_memory_oracle(
        flags_a in vec(any::<bool>(), 0..=8),
        flags_b in vec(any::<bool>(), 0..=8),
    ) {
        let (root, known) = tree_and_known(&flags_a, &flags_b);

        let oracle = Unknown::unknown(root.clone(), &known);
        let streamed = stream_prune(root, &known);

        prop_assert_eq!(
            typed::Node::root_hash(oracle.as_ref(), &typed::HashKey::fixture()),
            typed::Node::root_hash(streamed.as_ref(), &typed::HashKey::fixture()),
        );
    }

    /// Sibling leaves under one leaf-parent prefix are judged one at a
    /// time, and each leaf-height verdict agrees with the in-memory
    /// prune: the flagged leaves drop and the concurrent ones survive.
    #[test]
    fn leaf_height_verdicts_agree_with_in_memory_oracle(
        flags in vec(any::<bool>(), 1..=8),
    ) {
        let (root, known) = sibling_tree_and_known(&flags);

        let oracle = Unknown::unknown(root.clone(), &known);
        let streamed = stream_prune(root, &known);

        prop_assert_eq!(
            typed::Node::root_hash(oracle.as_ref(), &typed::HashKey::fixture()),
            typed::Node::root_hash(streamed.as_ref(), &typed::HashKey::fixture()),
        );
    }
}

/// The versions of the leaves `root` holds, as their canonical bytes.
fn held(ceiling: &Version, node: Option<typed::node::Root>) -> BTreeSet<Vec<u8>> {
    let root = crate::tree::Root {
        ceiling: ceiling.clone(),
        node,
    };
    Tree::<()>::from_root(NetworkKeys::fixture(), root)
        .iter()
        .map(|(version, _)| version.as_bytes().to_vec())
        .collect()
}

/// The leaves of `root` that a counterparty at `known` lacks, each judged
/// by the causal order of its own version alone.
///
/// The reconciliation docs say the subtree-wise prune is semantically
/// identical to this sieve.
fn sieve(root: &crate::tree::Root, known: &Version) -> BTreeSet<Vec<u8>> {
    Tree::<()>::from_root(NetworkKeys::fixture(), root.clone())
        .iter()
        .filter(|(version, _)| !contained(version, known))
        .map(|(version, _)| version.as_bytes().to_vec())
        .collect()
}

/// The counterparty versions that stress the prune's case analysis on
/// `roots`.
///
/// They are every replica's ceiling and every pair's join, which dominate
/// shared leaves and are concurrent with the other replicas' own; every
/// leaf's version, which a subtree's floor or ceiling can equal exactly;
/// the empty version, which dominates nothing; and a tick on a party no
/// replica uses, concurrent with every leaf.
fn counterparty_versions(roots: &[crate::tree::Root]) -> Vec<Version> {
    let mut known = vec![Version::new()];
    let mut beside = Version::new();
    beside.tick(&nth_party(31));
    known.push(beside);
    for (index, root) in roots.iter().enumerate() {
        known.push(root.ceiling.clone());
        for other in &roots[index + 1..] {
            known.push(root.ceiling.clone() | &other.ceiling);
        }
        let tree = Tree::<()>::from_root(NetworkKeys::fixture(), root.clone());
        known.extend(tree.iter().map(|(version, _)| version.clone()));
    }
    known
}

proptest! {
    /// The streaming prune and its providing top keep exactly the leaves a
    /// leaf-by-leaf causal sieve keeps, credit every other leaf as shed,
    /// and the providing top's survivors are exactly the surviving
    /// parent's children.
    ///
    /// The replicas share a history and add concurrent inserts and
    /// redactions on three parties, at every branching depth, and each is
    /// pruned against every version [`counterparty_versions`] names. The
    /// sieve reads only `<=` on each leaf's version, so it judges the
    /// prune's classification by memoized version bounds independently of
    /// the bounds' own dominance test, which the in-memory oracle shares.
    #[test]
    fn prune_equals_the_leaf_by_leaf_sieve(
        roots in arb_divergent_roots::<3>(0..6, 0..5),
        depth in proptest::option::of(0usize..32),
    ) {
        let roots = match depth {
            Some(depth) => roots_at_depth(roots, depth),
            None => roots,
        };
        let top = Prefix::new().erase();
        for known in counterparty_versions(&roots) {
            for root in &roots {
                let expected = sieve(root, &known);
                let shed = widen(held(&root.ceiling, root.node.clone()).len() - expected.len());

                let stats = Recorder::default();
                let pruned = root.node.clone().and_then(|node| {
                    pollster::block_on(unknown::<Local>(
                        &Local,
                        &known,
                        top,
                        <Local as Backend>::erase(node.into()),
                        &stats,
                    ))
                    .expect("the in-memory backend lists exactly the children it reports")
                    .map(|pruned| {
                        <Local as Backend>::assume(&Prefix::<Root>::new(), pruned).into_node()
                    })
                });
                prop_assert_eq!(held(&root.ceiling, pruned), expected.clone());
                prop_assert_eq!(stats.snapshot().messages_shed, shed);

                let Some(node) = root.node.clone() else {
                    continue;
                };
                let stats = Recorder::default();
                let (parent, survivors) = pollster::block_on(unknown_providing::<Local>(
                    &Local,
                    &known,
                    top,
                    <Local as Backend>::erase(node.into()),
                    &stats,
                ))
                .expect("the in-memory backend lists exactly the children it reports");
                prop_assert_eq!(stats.snapshot().messages_shed, shed);
                let listed = |children: Vec<(u8, <Local as Backend>::Erased)>| {
                    children
                        .into_iter()
                        .map(|(radix, child)| (radix, child.hash(&top.push(radix), &HashKey::fixture())))
                        .collect::<Vec<_>>()
                };
                let children = match &parent {
                    Some(parent) => pollster::block_on(ops::children_of(&Local, top, parent.clone()))
                        .expect("the in-memory backend lists exactly the children it reports"),
                    None => Vec::new(),
                };
                prop_assert_eq!(listed(survivors), listed(children));
                let parent = parent
                    .map(|parent| <Local as Backend>::assume(&Prefix::<Root>::new(), parent).into_node());
                prop_assert_eq!(held(&root.ceiling, parent), expected);
            }
        }
    }
}
