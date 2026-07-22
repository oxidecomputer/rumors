//! Bridge 2: `LocalEq` soundness — the Lean view projection agrees with
//! actual tree pairs.
//!
//! The Lean locality definition (`Mux/Strategy.lean` `viewEnc`/`LocalEq`)
//! projects a skeleton to what party `p` holds: every D child (recursed, in
//! radix order), R children only where `p` is the scope's asker (a held
//! cut), nothing where `p` is the answerer (a child `p` lacks, invisible at
//! session start), `leafReqs` erased from both views. This bridge ties that
//! projection to actual trees, in both directions:
//!
//! - **soundness** ([`assert_view_sound`], `view_projection_is_sound`):
//!   everything the projection RETAINS is genuinely held by the local tree,
//!   and everything it ERASES as peer-side is genuinely absent from it —
//!   checked against the literal leaf-path set the tree was built from;
//! - **nondegeneracy** (`free_insertions_are_invisible_to_the_local_view`,
//!   `leaf_requests_are_erased_from_the_view`): the full skeletons CAN
//!   differ while the p-views agree — the free-insertion moves of the
//!   corrected fooling alphabet (R children at p-answerer
//!   scopes; leaf requests) realized by concrete trees, occurring in every
//!   constructed case.

use std::collections::BTreeSet;

use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::TestRunner;

use crate::Version;
use crate::tree::arb::nth_party;

use super::fixtures::{arb_divergence, ceiling_of, grown, path_at, rooted, rooted_at};
use super::skeleton::{Decoded, Kind, Party, asks, client_role, decode, local_eq, view_enc};
use super::transcribed_mirror_sides;

/// Does the local tree hold a node at this prefix?
fn holds(local: &BTreeSet<[u8; 32]>, prefix: &[u8]) -> bool {
    local.iter().any(|path| path.starts_with(prefix))
}

/// Assert the Lean view projection is sound for one session against the
/// local tree's literal leaf-path set.
///
/// Retention: every D scope is held by the local tree (a dispute means both
/// sides hold it), and every R child of a scope the local party asks is held
/// (the cut: the subtree exists locally, absent from the skeleton).
/// Erasure: every R child of a scope the local party answers is absent
/// locally (peer-only — exactly what the view refuses to show), and every
/// requested leaf is absent locally when local plays initiator (the h1
/// answerer requests what it lacks) and held when local plays responder
/// (the supplier).
fn assert_view_sound(decoded: &Decoded, p: Party, local: &BTreeSet<[u8; 32]>) {
    for (i, prefix) in decoded.prefixes.iter().enumerate() {
        let scope = &decoded.skel.scopes[i];
        match scope.kind {
            Kind::D => assert!(
                holds(local, prefix),
                "disputed scope {prefix:02x?} must be locally held"
            ),
            Kind::R => {
                let parent_height = scope.height + 1;
                if asks(p, parent_height) {
                    assert!(
                        holds(local, prefix),
                        "requested scope {prefix:02x?} under a locally-asked \
                         parent is a held cut: the view retains it"
                    );
                } else {
                    assert!(
                        !holds(local, prefix),
                        "requested scope {prefix:02x?} under a locally-answered \
                         parent is peer-only: the view erases it"
                    );
                }
            }
        }
    }
    for leaf in &decoded.leaf_requests {
        match p {
            Party::I => assert!(
                !holds(local, leaf),
                "leaf request {leaf:02x?}: the initiator requests what it lacks"
            ),
            Party::R => assert!(
                holds(local, leaf),
                "leaf request {leaf:02x?}: the responder supplies what it holds"
            ),
        }
    }
}

proptest! {
    /// LOCALEQ SOUNDNESS: the p-view retains only locally-held structure
    /// and erases only locally-absent structure.
    ///
    /// For one local tree against two different remotes, each session's
    /// p-view draws exactly `viewEnc`'s split — asker-side R children
    /// retained, answerer-side R children and leaf requests erased — with
    /// retention and erasure checked against the local tree's literal leaf
    /// paths; the local role is identical across the two sessions once the
    /// remotes advertise equal versions; and `local_eq` is reflexive on
    /// decoded skeletons.
    #[test]
    fn view_projection_is_sound(spec in arb_divergence()) {
        let (local, remote_0, remote_1) = spec.trees(&());
        let p = client_role(&local, &remote_0);
        prop_assert_eq!(
            p,
            client_role(&local, &remote_1),
            "equalized remote ceilings fix the local role across sessions"
        );

        let local_paths = spec.local_path_set();
        let (_, _, trace_0, _) = transcribed_mirror_sides(local.clone(), remote_0);
        let (_, _, trace_1, _) = transcribed_mirror_sides(local, remote_1);
        let decoded_0 = decode(&trace_0);
        let decoded_1 = decode(&trace_1);

        assert_view_sound(&decoded_0, p, &local_paths);
        assert_view_sound(&decoded_1, p, &local_paths);
        prop_assert!(local_eq(p, &decoded_0.skel, &decoded_0.skel));
        if decoded_0.skel == decoded_1.skel {
            prop_assert!(local_eq(p, &decoded_0.skel, &decoded_1.skel));
        }
    }
}

/// NONDEGENERACY, counted: an R child inserted at a p-answerer scope is a
/// *free insertion* — the full skeletons differ while the p-views agree —
/// and it is exactly one-party-blind: the counterparty's view sees the cut.
///
/// Construction, per sampled shape: a base divergence with a guaranteed
/// disputed chain through `[0]`, `[0, 0]`, `[0, 0, 0]`; the second remote is
/// the first plus one subtree at radix 7 under whichever chain scope the
/// local party ANSWERS (parity chosen from the computed role — `viewEnc`
/// shows R children to the asker only). Every sampled case must come out
/// nondegenerate: `LocalEq` holds for p while the skeletons differ and
/// `LocalEq` fails for the counterparty. This realizes the adjudicated
/// answerer-side free-insertion move with concrete trees at 100% frequency.
#[test]
fn free_insertions_are_invisible_to_the_local_view() {
    const CASES: u32 = 32;
    let mut runner = TestRunner::deterministic();
    let strategy = arb_divergence();
    let mut nondegenerate = 0u32;

    for case in 0..CASES {
        let spec = strategy
            .new_tree(&mut runner)
            .expect("divergence specs generate")
            .current();

        // The anchored deep-D chain: shared, local, and remote leaves under
        // the cell [0, 0, 0] keep [0], [0, 0], and [0, 0, 0] disputed no
        // matter what the sampled decoration does.
        let mut shared: Vec<_> = spec
            .shared_paths()
            .into_iter()
            .map(|b| path_at(&b))
            .collect();
        shared.push(path_at(&[0, 0, 0, 0x10]));
        let mut local_extras: Vec<_> = spec
            .local_paths()
            .into_iter()
            .map(|b| path_at(&b))
            .collect();
        local_extras.push(path_at(&[0, 0, 0, 0x40]));
        let mut remote_extras: Vec<_> = spec
            .remote_paths(0)
            .into_iter()
            .map(|b| path_at(&b))
            .collect();
        remote_extras.push(path_at(&[0, 0, 0, 0x80]));

        let base = grown(None, 0, 1, &(), &shared);
        let local_node = grown(base.clone(), 2, 1, &(), &local_extras);
        let remote_node = grown(base, 1, 1, &(), &remote_extras);

        // Local membership oracle: the constructed local paths.
        let anchor = |slot: u8| {
            let mut path = [0u8; 32];
            path[3] = slot;
            path
        };
        let local_paths: BTreeSet<[u8; 32]> = spec
            .local_path_set()
            .into_iter()
            .chain([anchor(0x10), anchor(0x40)])
            .collect();

        // Role first (a pure function of advertised versions), then the
        // insertion parent: a scope the local party answers, so the new R
        // child is invisible to its view.
        let insertion = |parent: &[u8]| {
            let mut path = [0u8; 32];
            path[..parent.len()].copy_from_slice(parent);
            path[parent.len()] = 7;
            path_at(&path)
        };
        let remote_plus =
            |parent: &[u8]| grown(remote_node.clone(), 3, 1, &(), &[insertion(parent)]);
        // [0] sits at height 31 (answered by I), [0, 0] at height 30
        // (answered by R): pick by the local role.
        let candidate_i = remote_plus(&[0]);
        let candidate_r = remote_plus(&[0, 0]);

        let join = ceiling_of(&candidate_i) | &ceiling_of(&candidate_r);
        let local = rooted(local_node);
        let remote_0 = rooted_at(remote_node.clone(), join.clone());
        let p = client_role(&local, &remote_0);
        let remote_1 = rooted_at(
            match p {
                Party::I => candidate_i,
                Party::R => candidate_r,
            },
            join,
        );

        let (_, _, trace_0, _) = transcribed_mirror_sides(local.clone(), remote_0);
        let (_, _, trace_1, _) = transcribed_mirror_sides(local, remote_1);
        let decoded_0 = decode(&trace_0);
        let decoded_1 = decode(&trace_1);

        assert_view_sound(&decoded_0, p, &local_paths);
        assert_view_sound(&decoded_1, p, &local_paths);
        assert_ne!(
            decoded_0.skel, decoded_1.skel,
            "case {case}: the insertion changes the full skeleton"
        );
        assert!(
            local_eq(p, &decoded_0.skel, &decoded_1.skel),
            "case {case}: the answerer-side insertion is invisible to the \
             local view"
        );
        assert_ne!(
            view_enc(p.other(), &decoded_0.skel),
            view_enc(p.other(), &decoded_1.skel),
            "case {case}: the counterparty (the asker) sees the cut"
        );
        nondegenerate += 1;
    }

    assert_eq!(
        nondegenerate, CASES,
        "every constructed case realizes a nondegenerate LocalEq pair"
    );
}

/// NONDEGENERACY at the leaves: `leafReqs` is erased from both views, so
/// leaf-request-only skeleton differences are `LocalEq`.
///
/// The erasure is `Mux/Strategy.lean`'s adjudicated one: two sessions from
/// the same local tree whose skeletons differ ONLY in a height-1 scope's
/// leaf request count agree in the local party's view while the full
/// skeletons differ.
///
/// Two constructions, one per local role (leaf requests are always issued
/// by the initiator, so the free move differs by role): local as initiator,
/// the second remote adds one more concurrent leaf under the disputed leaf
/// parent (one more request for what local lacks); local as responder, the
/// second remote additionally holds one of local's shared leaves (one fewer
/// request against local's listing). Each construction forces its role with
/// a tiebreak inflation: extra ceiling ticks on a party that owns no leaf
/// anywhere, semantically inert (nothing's supplies ride it), searched until
/// `descend`'s canonical-byte comparison lands the required way.
#[test]
fn leaf_requests_are_erased_from_the_view() {
    fn leaf(last: u8) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        bytes[31] = last;
        bytes
    }

    /// `ceiling` joined with `ticks` ticks of the leafless tiebreak party.
    fn inflated(ceiling: Version, ticks: usize) -> Version {
        let party = nth_party(9);
        let mut extra = Version::new();
        for _ in 0..ticks {
            extra.tick(&party);
        }
        ceiling | &extra
    }

    /// How many tiebreak ticks to search before declaring the construction
    /// untunable.
    const TIEBREAK_ATTEMPTS: usize = 8;

    /// Run the two sessions and assert the leafReqs-only difference is
    /// invisible to the local view.
    fn check(
        local: crate::tree::Root<()>,
        remote_0: crate::tree::Root<()>,
        remote_1: crate::tree::Root<()>,
        local_paths: &BTreeSet<[u8; 32]>,
        p: Party,
    ) {
        let (_, _, trace_0, _) = transcribed_mirror_sides(local.clone(), remote_0);
        let (_, _, trace_1, _) = transcribed_mirror_sides(local, remote_1);
        let decoded_0 = decode(&trace_0);
        let decoded_1 = decode(&trace_1);
        assert_view_sound(&decoded_0, p, local_paths);
        assert_view_sound(&decoded_1, p, local_paths);

        let reqs = |decoded: &Decoded| -> Vec<usize> {
            decoded
                .skel
                .scopes
                .iter()
                .filter(|scope| scope.height == 1)
                .map(|scope| scope.leaf_reqs)
                .collect()
        };
        assert_ne!(
            reqs(&decoded_0),
            reqs(&decoded_1),
            "the sessions differ in leaf request counts"
        );
        assert_ne!(decoded_0.skel, decoded_1.skel);
        assert!(
            local_eq(p, &decoded_0.skel, &decoded_1.skel),
            "leafReqs is erased: the views agree while the skeletons differ"
        );
    }

    // Branch I: local = {leaf 0}; remotes = {leaf 0, 0x80} vs
    // {leaf 0, 0x80, 0x81} — one vs two leaf requests when local initiates.
    // Local's ceiling is inflated until it wins initiator election.
    {
        let base = grown(None, 0, 1, &(), &[path_at(&leaf(0))]);
        let remote_0_node = grown(base.clone(), 1, 1, &(), &[path_at(&leaf(0x80))]);
        let remote_1_node = grown(
            base.clone(),
            1,
            1,
            &(),
            &[path_at(&leaf(0x80)), path_at(&leaf(0x81))],
        );
        let join = ceiling_of(&remote_0_node) | &ceiling_of(&remote_1_node);
        let remote_0 = rooted_at(remote_0_node, join.clone());
        let remote_1 = rooted_at(remote_1_node, join);
        let local = (0..TIEBREAK_ATTEMPTS)
            .map(|ticks| rooted_at(base.clone(), inflated(ceiling_of(&base), ticks)))
            .find(|local| client_role(local, &remote_0) == Party::I)
            .expect("some tiebreak inflation makes the local side initiate");
        assert_eq!(client_role(&local, &remote_1), Party::I);
        check(
            local,
            remote_0,
            remote_1,
            &BTreeSet::from([leaf(0)]),
            Party::I,
        );
    }

    // Branch R: local = {leaf 0, leaf 1} (one shared chain); remotes =
    // {leaf 0, 0x80} vs {leaf 0, leaf 1, 0x80} — the initiating remote
    // requests leaf 1 in the first session and matches it in the second.
    // The remotes' joint ceiling is inflated until local loses election.
    {
        let base_1 = grown(None, 0, 1, &(), &[path_at(&leaf(0))]);
        let base_2 = grown(base_1.clone(), 4, 1, &(), &[path_at(&leaf(1))]);
        let remote_0_node = grown(base_1, 1, 1, &(), &[path_at(&leaf(0x80))]);
        let remote_1_node = grown(base_2.clone(), 1, 1, &(), &[path_at(&leaf(0x80))]);
        let join = ceiling_of(&remote_0_node) | &ceiling_of(&remote_1_node);
        let local = rooted(base_2);
        let (remote_0, remote_1) = (0..TIEBREAK_ATTEMPTS)
            .map(|ticks| {
                let join = inflated(join.clone(), ticks);
                (
                    rooted_at(remote_0_node.clone(), join.clone()),
                    rooted_at(remote_1_node.clone(), join),
                )
            })
            .find(|(remote_0, _)| client_role(&local, remote_0) == Party::R)
            .expect("some tiebreak inflation makes the local side respond");
        assert_eq!(client_role(&local, &remote_1), Party::R);
        check(
            local,
            remote_0,
            remote_1,
            &BTreeSet::from([leaf(0), leaf(1)]),
            Party::R,
        );
    }
}
