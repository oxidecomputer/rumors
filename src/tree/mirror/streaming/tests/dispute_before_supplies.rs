//! Real trees can produce one dispute ahead of six whole-subtree supplies.
//!
//! The shape has a root fan of seven, with the first child disputed down to a
//! leaf request and six whole-subtree supplies behind it. At a budget of zero,
//! the requesting side's stage cannot take the later supplies until the
//! disputed chain resolves, every level down to the leaf, while the supplier
//! writes the supplies as soon as it encodes them, long before the chain's
//! deeper replies, each of which waits on a round trip. So the receiving side
//! must hold supplies its stage has not taken, which its parking, unbounded,
//! does. A reader that waited for each frame's stage to accept it would stop
//! reading at those supplies, ahead of the replies the chain needs; the
//! `remote::pipe` module docs explain that hazard and why this design's reader
//! never waits on the walk. No test runs such a reader, so the stall it would
//! meet is argued from the code, not observed. These tests establish that a
//! session between real trees can produce the shape.
//!
//! The pair is hand-placed because a structural equality check needs exact
//! paths, which content-addressed generators cannot supply.
//!
//! The hand-written literal lives at root height 6; the protocol's real root is
//! at height 32, where the same construction yields the same shape with the
//! disputed chain descending every level. Both are pinned: the generator
//! against the literal, and the session against the generator.

use super::fixtures::{grown, leaf_sibling_path, path_at, rooted};
use super::skeleton::{
    Kind, Party, ROOT_H, Scope, Skel, client_role, decode, dispute_before_supplies,
};
use super::transcribed_mirror_sides;
use crate::tree::Root;

/// The shape's widest fan: the root's seven children.
const SHAPE_FAN: usize = 7;

/// The shape's largest per-scope dispute count: no scope has more than one
/// disputed child.
const SHAPE_MAX_D_COUNT: usize = 1;

/// Writes out the shape at root height 6, scope for scope, independently of the
/// generator.
///
/// The generator and session checks below hold the Rust side to this shape.
fn height_six_literal() -> Skel {
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

/// The generator reproduces the hand-written literal at the literal's root
/// height, with the shape's fan and dispute count, so pinning a session to
/// `dispute_before_supplies(ROOT_H)` pins it to the same shape.
#[test]
fn generator_matches_the_literal_at_height_six() {
    assert_eq!(dispute_before_supplies(6), height_six_literal());
    assert_eq!(dispute_before_supplies(6).max_fan(), SHAPE_FAN);
    assert_eq!(dispute_before_supplies(6).max_d_count(), SHAPE_MAX_D_COUNT);
}

/// A concrete tree pair whose session dispute skeleton is one dispute ahead of
/// six supplies.
///
/// The supplier (must play initiator, so the responder lacks the six subtrees
/// and requests them): one shared leaf on the all-zero path, plus six one-sided
/// subtrees at root radices 1..=6. The chain side: the same shared leaf, plus
/// one concurrent extra leaf that shares 31 path bytes with it — that single
/// difference disputes every scope from the root down to their common height-1
/// parent, where it surfaces as exactly one leaf request — plus seven ballast
/// leaves at root radices 7..=13. The ballast makes the chain side the larger
/// set, so the supplier initiates under the smaller-set-initiates election;
/// because the ballast is the *responder's* exclusive content, it ships as
/// whole root-level supplies and never enters the dispute skeleton.
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

/// A real session between two concrete trees produces one dispute ahead of six
/// supplies.
///
/// The shape has a root fan of seven: its first child is disputed down to one
/// leaf request, followed by six whole-subtree supplies. The construction uses
/// the protocol's real root height while preserving the shape's fan and
/// per-scope dispute count.
///
/// This establishes that the shape can arise from real trees. The session still
/// converges, here in process and in the `pipe` tests over one pipe, where the
/// reader never waits for a frame's stage to accept it.
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
        "the session's dispute skeleton is the generated shape at the protocol's \
         root height"
    );
    assert_eq!(decoded.skel.max_fan(), SHAPE_FAN);
    assert_eq!(decoded.skel.max_d_count(), SHAPE_MAX_D_COUNT);
}
