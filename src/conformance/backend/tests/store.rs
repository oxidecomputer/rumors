//! The Store conformance suite: the generic default towers against the
//! synchronous in-memory engines, differentially.
//!
//! [`Local`] overrides every [`Store`] seam with the synchronous engines,
//! so driving one seam on `Local` runs the engine and driving it on
//! [`Materializing`](super::Materializing) — whose `Store` impl overrides
//! nothing — runs the generic tower. The differential proptests here hold
//! the two observationally identical over generated action sequences,
//! which is what licenses every future backend to trust the defaults.
//!
//! Deliberate internal-entry check: the `Store` boundary is crate-internal
//! (see the visibility note in [`conformance::backend`](super::super)), so
//! this suite pins the trait seam itself rather than a public operation —
//! the seam *is* the surface a backend implements.

use std::ops::Bound;

use futures::StreamExt as _;
use proptest::prelude::*;

use super::Materializing;
use crate::{
    Version,
    message::Message,
    tree::{
        backend::{Action, Leaf, Local, Node, Store},
        typed::{
            self, Hash, Path,
            height::{self, Z},
        },
    },
};

/// One scripted step of a differential run, resolved to a concrete
/// `(path, version, action)` triple at apply time.
#[derive(Clone, Debug)]
enum Step {
    /// Insert a fresh message with this payload.
    Insert(u64),
    /// Forget a previously inserted leaf, by index into the inserts so
    /// far (falls back to an unknown path when nothing was inserted).
    ForgetInserted(proptest::sample::Index),
    /// Forget a path that (almost surely) holds nothing.
    ForgetUnknown([u8; 32]),
    /// Re-apply a previously resolved insert verbatim — same path, same
    /// version, same message. Exercises the leaf fold's replace
    /// transitions: stored-occupant replacement across batches and
    /// fresh-occupant replacement within one.
    ReapplyInserted(proptest::sample::Index),
    /// Aim a Forget at a previously inserted leaf, carrying the version of
    /// an insert *no later than* the target's.
    ///
    /// A strictly earlier version exercises the causally-prior skip arms
    /// (the forget is skipped and the leaf survives); the target's own
    /// version exercises the equal-version boundary (the forget lands).
    ForgetStale {
        target: proptest::sample::Index,
        stale: proptest::sample::Index,
    },
}

fn steps(max: usize) -> impl Strategy<Value = Vec<Step>> {
    proptest::collection::vec(
        prop_oneof![
            6 => any::<u64>().prop_map(Step::Insert),
            2 => any::<proptest::sample::Index>().prop_map(Step::ForgetInserted),
            2 => any::<[u8; 32]>().prop_map(Step::ForgetUnknown),
            2 => any::<proptest::sample::Index>().prop_map(Step::ReapplyInserted),
            2 => (any::<proptest::sample::Index>(), any::<proptest::sample::Index>())
                .prop_map(|(target, stale)| Step::ForgetStale { target, stale }),
        ],
        0..max,
    )
}

/// Resolve scripted steps against a running clock, ticking once per fresh
/// action as [`Tree::act`](crate::tree::Tree::act) does.
///
/// `inserted` accumulates each insert's full `(path, version, message)` so
/// a later batch can forget, replay, or stale-forget an earlier batch's
/// leaves. Replayed coordinates deliberately do not tick: the seam under
/// test is the versioned batch-apply, which admits any caller-supplied
/// triples.
fn resolve(
    steps: &[Step],
    clock: &mut before::Clock,
    inserted: &mut Vec<(Path, Version, Message<u64>)>,
) -> Vec<(Path, Version, Action<u64>)> {
    steps
        .iter()
        .map(|step| match step {
            Step::Insert(payload) => {
                let version = clock.tick().clone();
                let message = Message::new(*payload);
                let path = Path::for_leaf(&version, message.bytes());
                inserted.push((path, version.clone(), message.clone()));
                (path, version, Action::Insert(message))
            }
            Step::ForgetInserted(index) if !inserted.is_empty() => (
                inserted[index.index(inserted.len())].0,
                clock.tick().clone(),
                Action::Forget,
            ),
            Step::ReapplyInserted(index) if !inserted.is_empty() => {
                let (path, version, message) = inserted[index.index(inserted.len())].clone();
                (path, version, Action::Insert(message))
            }
            Step::ForgetStale { target, stale } if !inserted.is_empty() => {
                let target = target.index(inserted.len());
                let stale = stale.index(target + 1);
                (
                    inserted[target].0,
                    inserted[stale].1.clone(),
                    Action::Forget,
                )
            }
            Step::ForgetInserted(_) | Step::ReapplyInserted(_) | Step::ForgetStale { .. } => {
                (Path::from([0u8; 32]), clock.tick().clone(), Action::Forget)
            }
            Step::ForgetUnknown(bytes) => {
                (Path::from(*bytes), clock.tick().clone(), Action::Forget)
            }
        })
        .collect()
}

/// One backend's evolving replica in a differential run: the root, the
/// accumulated ceiling, and every version the act observer fired with.
struct Replica<B: Store<u64, Node<Z>: Leaf<u64>>> {
    backend: B,
    root: Option<B::Node<height::Root>>,
    ceiling: Version,
    observed: Vec<Version>,
}

impl<B: Store<u64, Node<Z>: Leaf<u64>>> Replica<B> {
    fn new(backend: B) -> Self {
        Self {
            backend,
            root: None,
            ceiling: Version::new(),
            observed: Vec::new(),
        }
    }

    /// Apply one resolved batch through the `Store::act` seam, joining
    /// effectual versions into the ceiling as the replica's commit path
    /// does.
    fn act(&mut self, actions: Vec<(Path, Version, Action<u64>)>) {
        let ceiling = &mut self.ceiling;
        let observed = &mut self.observed;
        self.root = pollster::block_on(self.backend.clone().act(
            self.root.take(),
            actions,
            |version: &Version| {
                *ceiling |= version;
                observed.push(version.clone());
            },
        ))
        .expect("both differential backends are infallible");
    }

    fn hash(&self) -> Hash {
        self.root
            .as_ref()
            .map(|node| node.hash())
            .unwrap_or_else(Hash::empty_root)
    }
}

/// Clone one resolved batch (messages are cheap `Arc`/`Bytes` handles).
fn duplicate(actions: &[(Path, Version, Action<u64>)]) -> Vec<(Path, Version, Action<u64>)> {
    actions.to_vec()
}

proptest! {
    /// The generic `act`, `get`, and `range` defaults are observationally
    /// identical to the synchronous in-memory engines.
    ///
    /// Over generated action sequences (inserts, redactions of live
    /// leaves, redactions of unknown paths, verbatim replays, stale
    /// forgets, applied in three batches — the third minted on a forked
    /// clock so the replica holds mutually concurrent versions), the
    /// tower-built and engine-built replicas agree on the root hash, the
    /// leaf count, the accumulated ceiling, every observer firing, every
    /// point lookup, and every causal range walk, including walks bounded
    /// by `Excluded` endpoints and endpoints concurrent with the leaves.
    #[test]
    fn towers_agree_with_local_engines(
        first in steps(32),
        second in steps(16),
        forked in steps(12),
        probe in any::<[u8; 32]>(),
        start in any::<Option<proptest::sample::Index>>(),
        end in any::<Option<proptest::sample::Index>>(),
        start_excluded in any::<bool>(),
        end_excluded in any::<bool>(),
    ) {
        let mut clock = before::Clock::seed();
        let mut inserted = Vec::new();

        let mut local = Replica::new(Local);
        let mut towered = Replica::new(Materializing);

        let actions = resolve(&first, &mut clock, &mut inserted);
        local.act(duplicate(&actions));
        towered.act(actions);

        // Fork before the remaining batches: the second batch advances the
        // main clock and the third the forked one, so the two batches'
        // versions are mutually concurrent and the version set genuinely
        // exercises the partial-order arms.
        let mut fork = clock.fork();
        for (steps, clock) in [(&second, &mut clock), (&forked, &mut fork)] {
            let actions = resolve(steps, clock, &mut inserted);
            local.act(duplicate(&actions));
            towered.act(actions);
        }

        prop_assert_eq!(local.hash(), towered.hash());
        prop_assert_eq!(
            local.root.as_ref().map(Node::len),
            towered.root.as_ref().map(|node| node.len()),
        );
        prop_assert_eq!(&local.ceiling, &towered.ceiling);
        prop_assert_eq!(&local.observed, &towered.observed);

        // Point lookups: every path ever inserted (live or since
        // redacted), plus one arbitrary probe.
        for path in inserted
            .iter()
            .map(|(path, _, _)| *path)
            .chain([Path::from(probe)])
        {
            let ours = pollster::block_on(Local.get(local.root.clone(), path))
                .expect("the in-memory engine is infallible");
            let theirs = pollster::block_on(Materializing.get(towered.root.clone(), path))
                .expect("the generic tower is infallible");
            prop_assert_eq!(ours.is_some(), theirs.is_some());
            if let (Some(ours), Some(theirs)) = (ours, theirs) {
                prop_assert_eq!(ours.ceiling(), theirs.ceiling());
                prop_assert_eq!(ours.hash(), theirs.hash());
                prop_assert_eq!(ours.message().bytes(), theirs.message().bytes());
            }
        }

        // Range walks: bounds drawn from the versions the run minted
        // (`None` index = unbounded, inclusive or exclusive per the flags),
        // compared leaf for leaf in order. Bound versions coincide with
        // actual leaf versions, so the inclusive/exclusive boundary and the
        // concurrent (incomparable) comparisons are both genuinely reached.
        let pick = |index: &Option<proptest::sample::Index>, exclude: bool| {
            match (index, local.observed.len()) {
                (Some(index), len) if len > 0 => {
                    let version = local.observed[index.index(len)].clone();
                    if exclude {
                        Bound::Excluded(version)
                    } else {
                        Bound::Included(version)
                    }
                }
                _ => Bound::Unbounded,
            }
        };
        let bounds = crate::tree::backend::VersionBounds {
            start: pick(&start, start_excluded),
            end: pick(&end, end_excluded),
        };
        let ours: Vec<_> = pollster::block_on(
            Local
                .range(local.root.clone(), bounds.clone())
                .collect::<Vec<_>>(),
        );
        let theirs: Vec<_> = pollster::block_on(
            Materializing
                .range(towered.root.clone(), bounds)
                .collect::<Vec<_>>(),
        );
        prop_assert_eq!(ours.len(), theirs.len());
        for (ours, theirs) in ours.into_iter().zip(theirs) {
            let (our_key, our_leaf) = ours.expect("the in-memory walk is infallible");
            let (their_key, their_leaf) = theirs.expect("the generic walk is infallible");
            prop_assert_eq!(our_key, their_key);
            prop_assert_eq!(our_leaf.ceiling(), their_leaf.ceiling());
            prop_assert_eq!(our_leaf.hash(), their_leaf.hash());
        }
    }

    /// The generic `join` default is observationally identical to the
    /// synchronous merge.
    ///
    /// Two replicas forked from a shared corpus and diverged — including
    /// redactions of shared leaves, the deletion-honoring case — merge to
    /// the same root hash whether the tower or the engine performs the
    /// join, and the merge is symmetric.
    #[test]
    fn join_tower_agrees_with_local_engine(
        common in steps(24),
        ours in steps(12),
        theirs in steps(12),
    ) {
        let mut left_clock = before::Clock::seed();
        let mut inserted = Vec::new();
        let shared = resolve(&common, &mut left_clock, &mut inserted);
        let mut right_clock = left_clock.fork();

        // Both sides of each backend's pair replay identical resolved
        // actions: the fork point and every version match across the
        // differential exactly.
        let mut right_inserted = inserted.clone();
        let ours_resolved = resolve(&ours, &mut left_clock, &mut inserted);
        let theirs_resolved = resolve(&theirs, &mut right_clock, &mut right_inserted);

        let merged: Vec<Hash> = [false, true]
            .into_iter()
            .map(|swap| {
                let mut a = Replica::new(Materializing);
                let mut b = Replica::new(Materializing);
                a.act(duplicate(&shared));
                a.act(duplicate(&ours_resolved));
                b.act(duplicate(&shared));
                b.act(duplicate(&theirs_resolved));
                let (a, b) = if swap { (b, a) } else { (a, b) };
                let joined = pollster::block_on(Materializing.join(
                    a.root.clone(),
                    b.root.clone(),
                    &a.ceiling,
                    &b.ceiling,
                ))
                .expect("the generic tower is infallible");
                joined
                    .map(|node| node.hash())
                    .unwrap_or_else(Hash::empty_root)
            })
            .collect();

        let mut a = Replica::new(Local);
        let mut b = Replica::new(Local);
        a.act(duplicate(&shared));
        a.act(ours_resolved);
        b.act(duplicate(&shared));
        b.act(theirs_resolved);
        let joined = pollster::block_on(Local.join(
            a.root.clone(),
            b.root.clone(),
            &a.ceiling,
            &b.ceiling,
        ))
        .expect("the in-memory engine is infallible");
        let oracle = joined
            .map(|node| node.hash())
            .unwrap_or_else(Hash::empty_root);

        prop_assert_eq!(merged[0], oracle);
        prop_assert_eq!(merged[1], oracle);
    }

    /// `same` is sound: any two handles it unifies carry equal subtree
    /// hashes.
    ///
    /// This is the license every join short-circuit rests on — a `same`
    /// that answered `true` across differing content would silently
    /// corrupt every merge, so the law is pinned for the reference
    /// backend over the handle relationships a replica actually creates:
    /// clones (unified), and independently built content twins (not
    /// unified, reconciled by the hash fallback instead). Reflexivity
    /// rides along: every handle is `same` as itself.
    #[test]
    fn same_unifies_only_equal_subtrees(
        payloads in proptest::collection::vec(any::<u64>(), 1..8),
    ) {
        let mut clock = before::Clock::seed();
        let mut pool: Vec<typed::Node<u64, Z>> = Vec::new();
        for payload in &payloads {
            let version = clock.tick().clone();
            let message = Message::new(*payload);
            let leaf = typed::Node::<u64, Z>::leaf(version.clone(), message.clone());
            pool.push(leaf.clone());
            pool.push(leaf);
            pool.push(typed::Node::<u64, Z>::leaf(version, message));
        }
        for a in &pool {
            prop_assert!(<Local as Store<u64>>::same(a, a));
            for b in &pool {
                if <Local as Store<u64>>::same(a, b) {
                    prop_assert_eq!(a.hash(), b.hash());
                }
            }
        }
    }
}

/// `Store::same` reports allocation identity, not content equality.
///
/// An in-memory handle and its clone share one allocation and one hash,
/// while two independently built content-identical leaves are *not*
/// `same` — the hash fallback, not identity, unifies them.
#[test]
fn same_is_identity_not_content() {
    let version = Version::new();
    let message = Message::new(7u64);
    let a = typed::Node::<u64, Z>::leaf(version.clone(), message.clone());
    let clone = a.clone();
    assert!(<Local as Store<u64>>::same(&a, &clone));
    assert_eq!(a.hash(), clone.hash());

    let rebuilt = typed::Node::<u64, Z>::leaf(version, message);
    assert!(!<Local as Store<u64>>::same(&a, &rebuilt));
    assert_eq!(a.hash(), rebuilt.hash());
}

/// Copy-on-write freshness and structural sharing at the assembly seams.
///
/// Assembling a parent mints a new allocation (two assemblies of one
/// group are never `same`), while the exploded children keep their input
/// identities: sharing survives a round trip through `parent`/`children`.
#[test]
fn parent_mints_fresh_and_children_round_trip() {
    use crate::tree::backend::{Backend as _, children_of};
    use crate::tree::typed::{Prefix, height::S};

    let mut clock = before::Clock::seed();
    let leaves: Vec<typed::Node<u64, Z>> = (0..3u64)
        .map(|payload| typed::Node::leaf(clock.tick().clone(), Message::new(payload)))
        .collect();
    let group = || -> Vec<(u8, Option<typed::Node<u64, Z>>)> {
        leaves
            .iter()
            .enumerate()
            .map(|(radix, leaf)| (radix as u8, Some(leaf.clone())))
            .collect()
    };

    let prefix = Prefix::<S<Z>>::containing(&Path::from([0u8; 32]));
    let once = pollster::block_on(Local.parent(prefix, group()))
        .expect("the in-memory backend is infallible")
        .expect("a non-empty group assembles a parent");
    let twice = pollster::block_on(Local.parent(prefix, group()))
        .expect("the in-memory backend is infallible")
        .expect("a non-empty group assembles a parent");
    assert!(!<Local as Store<u64>>::same(&once, &twice));
    assert_eq!(once.hash(), twice.hash());

    let children = pollster::block_on(children_of(&Local, prefix, once))
        .expect("the in-memory backend is infallible");
    assert_eq!(children.len(), leaves.len());
    for ((radix, child), (expected_radix, leaf)) in children.into_iter().zip(
        leaves
            .iter()
            .enumerate()
            .map(|(radix, leaf)| (radix as u8, leaf)),
    ) {
        assert_eq!(radix, expected_radix);
        assert!(<Local as Store<u64>>::same(&child, leaf));
    }
}
