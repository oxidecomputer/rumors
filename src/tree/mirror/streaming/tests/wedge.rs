//! Bridge 1: wedge realizability — real trees produce the Lean witness's
//! dispute skeleton.
//!
//! The mux impossibility theorem T3 (`wc_impossibility`) quantifies over one
//! fixed skeleton, `wedge` (`formal/lean/StreamingMirror/Mux/Instances.lean`):
//! root fan 7 with the first radix child deep-disputed down to a leaf request
//! and six whole-subtree provisions behind it. The theorem's Rust corollary
//! needs that shape to be *tree-realizable* — a session between two real
//! trees whose dispute skeleton IS the wedge (adjudication repair F4: for
//! impossibilities, realizability flows from Rust to the model).
//!
//! On the committed seeds: `tests/pairwise.proptest-regressions` and
//! `tests/shadow_validity.proptest-regressions` exist on this branch, but
//! they are integration-level seeds (three-peer networks, content-addressed
//! keys, whole-`Rumors` action lists) that realize the wedge's *jam
//! mechanism* on the old transport, not its byte-exact shape; a structural
//! equality pin needs hand-placed paths. This bridge therefore constructs
//! the pair deterministically and pins the decoded skeleton to the literal.
//!
//! The Lean literal lives at `rootH = 6`; the protocol's real root is at
//! height 32, where the same generator yields the same shape with the
//! disputed chain descending every level. Both are pinned: the generator
//! against the transcribed literal, and the session against the generator.

use super::fixtures::{grown, path_at, rooted};
use super::skeleton::{Kind, Party, ROOT_H, Scope, Skel, client_role, decode, wedge};
use super::transcribed_mirror_sides;
use crate::tree::Root;

/// The Lean wedge's `fan` field: the tight fan bound of the witness
/// (`Instances.lean`: `fan := 7`).
const LEAN_WEDGE_FAN: usize = 7;

/// The Lean wedge's `capLevel` field: the margin-0 dispute bound the witness
/// satisfies (`Instances.lean`: `capLevel := 1`; `wedge_margin0`).
const LEAN_WEDGE_CAP_LEVEL: usize = 1;

/// The Lean wedge literal, transcribed scope-for-scope from
/// `formal/lean/StreamingMirror/Mux/Instances.lean` (`Mux.wedge`) — that
/// file is the source of truth; if the literal changes there, change this.
fn lean_wedge_literal() -> Skel {
    let sc = |kind, height, kids: &[usize], leaf_reqs| Scope {
        kind,
        height,
        kids: kids.to_vec(),
        leaf_reqs,
    };
    Skel {
        scopes: vec![
            sc(Kind::D, 6, &[1, 2, 3, 4, 5, 6, 7], 0), // 0: root
            sc(Kind::D, 5, &[8], 0),                   // 1: the deep dispute, radix-first
            sc(Kind::R, 5, &[], 0),                    // 2: the provision wall…
            sc(Kind::R, 5, &[], 0),                    // 3
            sc(Kind::R, 5, &[], 0),                    // 4
            sc(Kind::R, 5, &[], 0),                    // 5
            sc(Kind::R, 5, &[], 0),                    // 6
            sc(Kind::R, 5, &[], 0),                    // 7
            sc(Kind::D, 4, &[9], 0),                   // 8: chain
            sc(Kind::D, 3, &[10], 0),                  // 9: chain
            sc(Kind::D, 2, &[11], 0),                  // 10: chain
            sc(Kind::D, 1, &[], 1),                    // 11: the demanded leaf
        ],
        root_h: 6,
    }
}

/// The wedge generator reproduces the Lean literal at the literal's root
/// height, so pinning a session to `wedge(ROOT_H)` is pinning it to the same
/// shape the kernel-checked theorems quantify over.
#[test]
fn wedge_generator_matches_the_lean_literal() {
    assert_eq!(wedge(6), lean_wedge_literal());
    assert_eq!(wedge(6).max_fan(), LEAN_WEDGE_FAN);
    assert_eq!(wedge(6).max_d_count(), LEAN_WEDGE_CAP_LEVEL);
}

/// A concrete tree pair whose session dispute skeleton is the wedge.
///
/// The wall holder (must play initiator, so the responder lacks the wall and
/// requests it): one shared leaf on the all-zero path, plus six one-sided
/// subtrees at root radices 1..=6. The chain side: the same shared leaf,
/// plus one concurrent extra leaf that shares 31 path bytes with it — that
/// single difference disputes every scope from the root down to their common
/// height-1 parent, where it surfaces as exactly one leaf request.
fn wedge_trees() -> (Root<()>, Root<()>) {
    let shared = grown(None, 0, 1, &(), &[path_at(&[0u8; 32])]);

    let mut wall_paths = Vec::new();
    for radix in 1..=6u8 {
        wall_paths.push(path_at(&[radix]));
    }
    let wall = grown(shared.clone(), 2, 1, &(), &wall_paths);

    let mut extra = [0u8; 32];
    extra[31] = 1;
    let chain = grown(shared, 1, 1, &(), &[path_at(&extra)]);

    (rooted(wall), rooted(chain))
}

/// WEDGE REALIZABILITY: a real session between two concrete trees produces
/// exactly the wedge dispute skeleton.
///
/// Root fan 7, first radix child deep-disputed down to a single leaf
/// request, six whole-subtree provisions behind it — at the protocol's real
/// root height, with the witness's `fan` and margin-0 `capLevel` values
/// intact.
///
/// This is the T3 Rust corollary's bridge: the
/// impossibility's ∃-witness is realizable, so `wc_impossibility` indicts a
/// transport, not a phantom shape. On this branch (link-transport) the
/// session completes — the wedge jams only the retired single-pipe mux — so
/// the pin is shape realizability plus convergence, not a stall.
#[test]
fn session_realizes_the_wedge_shape() {
    let (wall, chain) = wedge_trees();
    assert_eq!(
        client_role(&wall, &chain),
        Party::I,
        "the wall holder must win initiator election (the wedge's provisions \
         are the responder's requests); tune wedge_trees' party indices or \
         tick counts if tree construction changes"
    );

    let (ours, theirs, trace, _) = transcribed_mirror_sides(wall, chain);
    assert_eq!(ours, theirs, "sanity: the session converges");

    let decoded = decode(&trace);
    assert_eq!(
        decoded.skel,
        wedge(ROOT_H),
        "the session's dispute skeleton is the wedge shape at the protocol's \
         root height"
    );
    assert_eq!(decoded.skel.max_fan(), LEAN_WEDGE_FAN);
    assert_eq!(decoded.skel.max_d_count(), LEAN_WEDGE_CAP_LEVEL);
}
