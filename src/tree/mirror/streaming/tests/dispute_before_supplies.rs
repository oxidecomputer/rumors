//! Real trees can produce one dispute ahead of six whole-subtree supplies.
//!
//! The Lean witness `Mux.wedge`, which the theorem `wc_impossibility` uses,
//! has this shape: a root fan of seven, with the first child disputed down to
//! a leaf request and six whole-subtree supplies behind it. It is the smallest
//! schedule in which one shared socket deadlocks. These tests establish that
//! a session between real trees can produce that shape.
//!
//! The pair is hand-placed because a structural equality check needs exact
//! paths, which content-addressed generators cannot supply.
//!
//! The Lean literal lives at `rootH = 6`; the protocol's real root is at
//! height 32, where the same construction yields the same shape with the
//! disputed chain descending every level. Both are pinned: the generator
//! against the transcribed literal, and the session against the generator.

use super::fixtures::{grown, leaf_sibling_path, path_at, rooted};
use super::skeleton::{
    Kind, Party, ROOT_H, Scope, Skel, client_role, decode, dispute_before_supplies,
};
use super::transcribed_mirror_sides;
use crate::tree::Root;

/// The Lean witness's `fan` field: the tight fan bound of the witness
/// (`Mux.wedge`: `fan := 7`).
const LEAN_WITNESS_FAN: usize = 7;

/// The Lean witness's `capLevel` field: the margin-0 dispute bound the witness
/// satisfies (`Mux.wedge`: `capLevel := 1`; the theorem `Mux.wedge_margin0`).
const LEAN_WITNESS_CAP_LEVEL: usize = 1;

/// Transcribes the Lean definition `Mux.wedge`, the source of truth, scope
/// for scope.
///
/// Review this transcription against `Mux.wedge` whenever either changes.
/// The generator and session checks below hold the Rust side to this shape.
fn lean_witness_literal() -> Skel {
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
            sc(Kind::R, 5, &[], 0),                    // 2: the six supplies…
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

/// The generator reproduces the Lean literal at the literal's root height,
/// so pinning a session to `dispute_before_supplies(ROOT_H)` is pinning it to
/// the same
/// shape the kernel-checked theorems quantify over.
#[test]
fn generator_matches_the_lean_literal() {
    assert_eq!(dispute_before_supplies(6), lean_witness_literal());
    assert_eq!(dispute_before_supplies(6).max_fan(), LEAN_WITNESS_FAN);
    assert_eq!(
        dispute_before_supplies(6).max_d_count(),
        LEAN_WITNESS_CAP_LEVEL
    );
}

/// A concrete tree pair whose session dispute skeleton is one dispute ahead
/// of six supplies.
///
/// The supplier (must play initiator, so the responder lacks the six
/// subtrees and requests them): one shared leaf on the all-zero path, plus six
/// one-sided subtrees at root radices 1..=6. The chain side: the same shared leaf,
/// plus one concurrent extra leaf that shares 31 path bytes with it — that
/// single difference disputes every scope from the root down to their common
/// height-1 parent, where it surfaces as exactly one leaf request — plus
/// seven ballast leaves at root radices 7..=13. The ballast makes the chain
/// side the larger set, so the supplier initiates under the
/// smaller-set-initiates election; because the ballast is the *responder's*
/// exclusive content, it ships as whole root-level supplies and never enters
/// the dispute skeleton.
pub(super) fn dispute_before_supplies_trees() -> (Root, Root) {
    let shared = grown(None, 0, 1, &(), &[leaf_sibling_path(0)]);

    let mut supplier_paths = Vec::new();
    for radix in 1..=6u8 {
        supplier_paths.push(path_at(&[radix]));
    }
    let supplier = grown(shared.clone(), 2, 1, &(), &supplier_paths);

    let mut chain_paths = vec![leaf_sibling_path(1)];
    for radix in 7..=13u8 {
        chain_paths.push(path_at(&[radix]));
    }
    let chain = grown(shared, 1, 1, &(), &chain_paths);

    (rooted(supplier), rooted(chain))
}

/// A real session between two concrete trees produces one dispute ahead of
/// six supplies.
///
/// The shape has a root fan of seven: its first child is disputed down to one
/// leaf request, followed by six whole-subtree supplies. The construction uses
/// the protocol's real root height while preserving the model's fan and
/// capacity values.
///
/// This establishes that the witness used by `wc_impossibility` can arise from
/// real trees. The session still converges, here in process and in the
/// `pipe` tests over one pipe: the theorem concerns a shared pipe whose
/// reader waits for each frame's stage, which this design's reader never
/// does.
#[test]
fn session_realizes_one_dispute_before_six_supplies() {
    let (supplier, chain) = dispute_before_supplies_trees();
    assert_eq!(
        client_role(&supplier, &chain),
        Party::I,
        "the supplier must win initiator election (its provisions \
         are the responder's requests); tune dispute_before_supplies_trees' chain-side ballast \
         if tree construction changes"
    );

    let (ours, theirs, trace, _) = transcribed_mirror_sides(supplier, chain);
    assert_eq!(ours, theirs, "sanity: the session converges");

    let decoded = decode(&trace);
    assert_eq!(
        decoded.skel,
        dispute_before_supplies(ROOT_H),
        "the session's dispute skeleton is the modeled shape at the protocol's \
         root height"
    );
    assert_eq!(decoded.skel.max_fan(), LEAN_WITNESS_FAN);
    assert_eq!(decoded.skel.max_d_count(), LEAN_WITNESS_CAP_LEVEL);
}
