//! Differential pins for the fused tick against the recursive oracle.
//!
//! Two committed differentials are the entire pin of the fused walk and its
//! changed flag, both total by canonical uniqueness: [`tick`] must equal the
//! recursive oracle's `event` byte for byte, and the walk's changed flag must
//! read exactly `fill(i, e) ≠ e` decided by the oracle's `fill` — over every
//! committed family crossed with adversarial parties, the exhaustive small
//! scope, arbitrary pairs, and organic histories. The flag is
//! emitted-differs-from-input, plateau-aligned: a value-reproducing raise must
//! not trip it, and the first emitted leaf compares absolute against absolute
//! (the worked corner cases pin both). The oracle walks on native frames, so
//! these grids run at oracle-sized depths; deep-input coverage lives in the
//! closed-form witnesses below, the meter suite's closed-form output asserts at
//! its pinned scales, and the board's determinism tripwire. Those are size-axis
//! instruments: none of them discriminates the width of the flag's value
//! comparison, so that axis is pinned separately — the full-width worked
//! witnesses below (a multiple-of-`2^64` raise offset must read nonzero; a wide
//! value-reproducing raise must read a full-width zero) and
//! `generators::arb_base`'s `2^64`-aligned arm, which keeps generator mass on
//! offsets whose low limb is zero. The unchanged branch's splice is
//! additionally held to the oracle's inflation, the brute-force search, and a
//! reference route probe in `grow/tests.rs`.

use dashu_int::UBig;
use proptest::prelude::*;
use rayon::prelude::*;

use crate::codec::Base;
use crate::idbits::IdReader;
use crate::meter::registry::Shape;
use crate::meter::Packed;
use crate::testing::bridge::{
    from_oracle_party, from_oracle_version, to_oracle_party, to_oracle_version,
};
use crate::testing::exhaustive::{
    all_normal_events, all_normal_ids, EV_SMALL_DEPTH, ID_SMALL_DEPTH,
};
use crate::testing::{generators, optrace};
use crate::version::skyline::{encode, validate};
use crate::{Clock, Party, Version};

use super::super::grow::Cost;
use super::fuse::RouteProbe;
use super::{fused_fill, tick, ticks, FillOutcome};

/// Lift a meter-generated packed event shape into a [`Version`].
fn version_of(p: &Packed) -> Version {
    p.version()
}

/// Decode a meter-generated packed id shape as a [`Party`].
fn party_of(p: &Packed) -> Party {
    Party::decode(&p.bytes[..]).expect("meter shapes are strict normal form")
}

/// Whether the fused walk's changed flag tripped on one pair.
fn flag_of(v: &Version, p: &Party) -> bool {
    matches!(fused_fill(&encode(v), p), FillOutcome::Changed(_))
}

/// The two differentials of record on one pair, plus entry agreement and
/// canonicality.
///
/// The changed flag must read exactly what the recursive oracle's `fill`
/// decides (`fill(i, e) ≠ e`), the changed branch's stream must be the oracle's
/// fill byte for byte, and the whole `tick` must be the oracle's `event` byte
/// for byte — canonical uniqueness makes all three total. The public
/// `Version::tick` routes to the same kernel; asserting it too pins entry
/// agreement and determinism.
fn assert_tick(v: &Version, p: &Party) {
    let enc = encode(v);
    let filled = from_oracle_version(&to_oracle_version(v).fill_for_test(&to_oracle_party(p)));
    let changed = filled != *v;
    match fused_fill(&enc, p) {
        FillOutcome::Changed(bits) => {
            assert!(
                changed,
                "the changed flag tripped but the oracle's fill is the identity: {v} with {p}"
            );
            assert_eq!(
                bits,
                encode(&filled),
                "the changed branch must be the oracle's fill: {v} with {p}"
            );
        }
        FillOutcome::Unchanged(_) => {
            assert!(
                !changed,
                "the changed flag stayed clear but the oracle's fill moved the tree: {v} with {p}"
            );
        }
    }
    let out = tick(&enc, p);
    validate(&out).expect("a ticked stream is canonical");
    let mut oracle = to_oracle_version(v);
    oracle.tick(&to_oracle_party(p));
    assert_eq!(
        out,
        encode(&from_oracle_version(&oracle)),
        "tick must register the recursive oracle's event: {v} with {p}"
    );
    let mut expected = v.clone();
    expected.tick(p);
    assert_eq!(
        out,
        encode(&expected),
        "the module tick and the public tick must agree: {v} with {p}"
    );
}

/// The adversarial event pool: every §2 family at two scales, plus the
/// empty version.
fn event_pool() -> Vec<Version> {
    vec![
        Version::new(),
        version_of(&Shape::Dense.packed1(1)),
        version_of(&Shape::Dense.packed1(2)),
        version_of(&Shape::Dense.packed1(64)),
        version_of(&Shape::Bigroot.packed2(7, 3)),
        version_of(&Shape::Bigroot.packed2(64, 16)),
        version_of(&Shape::Hugeleaf.packed1(1)),
        version_of(&Shape::Hugeleaf.packed1(64)),
        version_of(&Shape::CliffComb.packed2(3, 2)),
        version_of(&Shape::CliffComb.packed2(16, 16)),
        version_of(&Shape::WideToothComb.packed3(16, 8, 8)),
        version_of(&Shape::CliffFan.packed2(16, 8)),
        version_of(&Shape::CancellingChain.packed2(16, 8)),
        version_of(&Shape::AltSpine.packed1(3)),
        version_of(&Shape::AltSpine.packed1(64)),
        version_of(&Shape::Harmonic.packed1(16)),
        version_of(&Shape::WideTail.packed2(7, 3)),
        version_of(&Shape::WideTail.packed2(64, 16)),
        version_of(&Shape::Staircase.packed1(1)),
        version_of(&Shape::Staircase.packed1(16)),
        version_of(&Shape::MemoChain.packed_flagged(1, true)),
        version_of(&Shape::MemoChain.packed_flagged(8, true)),
        version_of(&Shape::MemoChain.packed_flagged(8, false)),
        version_of(&Shape::MemoComb.packed1(1)),
        version_of(&Shape::MemoComb.packed1(4)),
        version_of(&Shape::MemoFanout.packed2(1, 7)),
        version_of(&Shape::MemoFanout.packed2(6, 64)),
        version_of(&Shape::MemoOscillating.packed2(6, 64)),
        version_of(&Shape::MemoChurn.packed1(1)),
        version_of(&Shape::MemoChurn.packed1(5)),
        version_of(&Shape::DescendingRaises.packed1(1)),
        version_of(&Shape::DescendingRaises.packed1(6)),
        version_of(&Shape::RevealComb.packed2(1, 2)),
        version_of(&Shape::RevealComb.packed2(6, 5)),
        version_of(&Shape::RevealCombHifloor.packed2(1, 2)),
        version_of(&Shape::RevealCombHifloor.packed2(6, 5)),
        version_of(&Shape::PureComb.packed2(1, 2)),
        version_of(&Shape::PureComb.packed2(6, 5)),
        version_of(&Shape::AscendCliff.packed2(1, 2)),
        version_of(&Shape::AscendCliff.packed2(6, 5)),
        version_of(&Shape::AscendCliffPlateau.packed2(1, 2)),
        version_of(&Shape::AscendCliffPlateau.packed2(6, 5)),
    ]
}

/// The adversarial party pool: the seed, deep and diverted unary spines,
/// scattered ownership, and every owning exhaustive small-scope id (an empty id
/// never ticks: the public contract requires an owning party).
fn party_pool() -> Vec<Party> {
    let mut pool = vec![
        Party::seed(),
        party_of(&Shape::IdSpine.packed_flagged(1, false)),
        party_of(&Shape::IdSpine.packed_flagged(3, false)),
        party_of(&Shape::IdSpine.packed_flagged(3, true)),
        party_of(&Shape::IdSpine.packed_flagged(64, false)),
        party_of(&Shape::IdSpine.packed_flagged(64, true)),
        party_of(&Shape::ScatteredId.packed1(1)),
        party_of(&Shape::ScatteredId.packed1(16)),
        party_of(&Shape::NestedFullId.packed1(1)),
        party_of(&Shape::NestedFullId.packed1(8)),
        party_of(&Shape::NestedLeftFullId.packed1(1)),
        party_of(&Shape::NestedLeftFullId.packed1(8)),
        party_of(&Shape::MemoChainId.packed1(1)),
        party_of(&Shape::MemoChainId.packed1(8)),
        party_of(&Shape::MemoCombId.packed1(1)),
        party_of(&Shape::MemoCombId.packed1(4)),
        party_of(&Shape::MemoChurnId.packed1(1)),
        party_of(&Shape::MemoChurnId.packed1(5)),
        party_of(&Shape::DescendingRaisesId.packed1(1)),
        party_of(&Shape::DescendingRaisesId.packed1(6)),
        party_of(&Shape::RevealCombId.packed1(1)),
        party_of(&Shape::RevealCombId.packed1(6)),
        party_of(&Shape::PureCombId.packed1(1)),
        party_of(&Shape::PureCombId.packed1(6)),
        party_of(&Shape::AscendCliffId.packed1(1)),
        party_of(&Shape::AscendCliffId.packed1(6)),
    ];
    for oid in all_normal_ids(2) {
        let p = from_oracle_party(&oid);
        if !p.as_bits().is_empty() {
            pool.push(p);
        }
    }
    pool
}

/// Every event-family × party-family pair ticks byte-identically to the
/// recursive oracle's event, with the changed flag agreeing with the oracle's
/// fill, through both entry points.
#[test]
fn family_pairs_tick_and_flag_identically() {
    let events = event_pool();
    let parties = party_pool();
    events.par_iter().for_each(|v| {
        for p in &parties {
            assert_tick(v, p);
        }
    });
}

/// Exhaustive small scope: every normal-form event tree × every owning
/// normal-form id ticks byte-identically to the recursive oracle's event, with
/// the changed flag agreeing with the oracle's fill.
#[test]
fn exhaustive_small_scope_ticks_and_flags_identically() {
    let events: Vec<Version> = all_normal_events(EV_SMALL_DEPTH)
        .iter()
        .map(from_oracle_version)
        .collect();
    let parties: Vec<Party> = all_normal_ids(ID_SMALL_DEPTH)
        .iter()
        .map(from_oracle_party)
        .filter(|p| !p.as_bits().is_empty())
        .collect();
    events.par_iter().for_each(|v| {
        for p in &parties {
            assert_tick(v, p);
        }
    });
}

/// Exhaustive small scope for the fused multi-tick: `ticks(n)` for every `n` in
/// 0..=4 equals the iterated public tick on every normal-form event tree ×
/// every owning normal-form id.
///
/// A total check on both branches (the fill-changed collapses and the grow
/// splices, expansion chains included) at the scope where totality is
/// affordable.
#[test]
fn exhaustive_small_scope_ticks_n_matches_iterated() {
    let events: Vec<Version> = all_normal_events(EV_SMALL_DEPTH)
        .iter()
        .map(from_oracle_version)
        .collect();
    let parties: Vec<Party> = all_normal_ids(ID_SMALL_DEPTH)
        .iter()
        .map(from_oracle_party)
        .filter(|p| !p.as_bits().is_empty())
        .collect();
    events.par_iter().for_each(|v| {
        for p in &parties {
            check_ticks_equivalence(v, p, &[0, 1, 2, 3, 4]);
        }
    });
}

/// The worked fill examples, pinned end to end through the fused tick.
///
/// The cases: the full-id collapse, both shortcut raises (taken and declined),
/// and a nested arm — each a changed-flag trip whose tick is the collapse
/// itself, plus the declined raise whose flag stays clear.
#[test]
fn worked_examples_tick_exactly() {
    // (party, before, fill's result): fill moves the tree, so the tick IS
    // fill's result.
    let changed: [(&str, &str, &str); 5] = [
        // The full id collapses the whole tree to its max (heights 2 and 3; the
        // collapse is the higher plateau).
        ("1", "(2, 0, 1)", "3"),
        // Left-full: the collapsed left rises to the right's minimum (min
        // fill(0, er) = 3 > max(el) = 2), and the pair merges.
        ("(1, 0)", "(2, 0, 1)", "3"),
        // Right-full, mirrored.
        ("(0, 1)", "(2, 1, 0)", "3"),
        // Left-full over an internal left child: the whole el subtree collapses
        // into the raised leaf.
        ("(1, 0)", "(2, (0, 1, 0), 3)", "5"),
        // A node id whose left child is itself a shortcut site: the inner raise
        // lifts the root's minimum, and norm re-lifts it.
        ("((1, 0), 0)", "(1, (0, 0, 1), 2)", "(2, 0, 1)"),
    ];
    for (party, before, after) in changed {
        let p: Party = party.parse().expect("test party literals parse");
        let v: Version = before.parse().expect("test version literals parse");
        let expected: Version = after.parse().expect("test version literals parse");
        match fused_fill(&encode(&v), &p) {
            FillOutcome::Changed(bits) => assert_eq!(
                bits,
                encode(&expected),
                "fill of {before} with {party} must yield {after}"
            ),
            FillOutcome::Unchanged(_) => {
                panic!("fill of {before} with {party} moves the tree: the flag must trip")
            }
        }
        assert_tick(&v, &p);
    }
    // Right-full where the raise is declined (max(er) = 3 already clears
    // min(el') = 2): fill is the identity, the flag stays clear, and the tick
    // is the grow branch.
    let p: Party = "(0, 1)".parse().expect("test party literals parse");
    let v: Version = "(2, 0, 1)".parse().expect("test version literals parse");
    assert!(
        !flag_of(&v, &p),
        "a declined raise reproduces the input: the flag must stay clear"
    );
    assert_tick(&v, &p);
}

/// The changed flag's corner cases, pinned as worked examples: the flag is
/// emitted-differs-from-input, plateau-aligned, never "an arm fired".
///
/// A raise that reproduces the existing leaf value exactly must not trip it
/// (`max(max(el), min(er′)) = el` is the paper's equation producing the input
/// leaf verbatim — here the flag's first comparison is also the stream's first
/// plateau, so the match compares one absolute code against another, never a
/// delta against an absolute); a collapse that shifts which input leaf is first
/// trips on topology (the replaced range is not a single leaf) before any code
/// comparison is reached.
#[test]
fn flag_reads_plateau_divergence_not_arm_firing() {
    // Left-full raise, value-reproducing at the stream's head: max(max(el) = 1,
    // min(er) = 0) = 1 = el. The first emitted leaf is the raise's — absolute
    // against absolute — and the flag stays clear.
    let p: Party = "(1, 0)".parse().expect("test party literals parse");
    let v: Version = "(2, 1, 0)".parse().expect("test version literals parse");
    assert!(
        !flag_of(&v, &p),
        "a value-reproducing raise emits the input plateau: no divergence"
    );
    assert_tick(&v, &p);

    // The same arm over a multi-leaf left child: the collapse shifts which leaf
    // is first, so the flag trips on topology — the range replaced by one leaf
    // was a node — before any code comparison.
    let v: Version = "(2, (0, 1, 0), 5)".parse().expect("test literals parse");
    assert!(
        flag_of(&v, &p),
        "a collapse that moves topology trips the flag on the plateau's depth"
    );
    assert_tick(&v, &p);
}

/// The flag's value comparison is full-width: a raise offset that is a nonzero
/// multiple of `2^64` must read nonzero.
///
/// Such an offset's low 64 bits are all zero, so a comparison truncated to one
/// limb would read it as zero, leave the flag clear, and mis-route the pair to
/// the grow branch — wrong tick output. The dual rides along: a wide raise that
/// reproduces the input leaf exactly computes its zero offset from wide
/// operands, and the comparison must read that zero at full width — the flag
/// must stay clear. Both directions assert the flag and the tick output against
/// the recursive oracle. Ongoing generator mass for the class lives in
/// `generators::arb_base`'s `2^64`-aligned arm.
#[test]
fn flag_compares_offsets_at_full_width() {
    // Left-full raise over a single zero leaf against min(er) = 2^64: the
    // emitted offset is exactly 2^64 — nonzero only above the low limb — so the
    // flag must trip, and the tick is fill's collapse to the single wide leaf.
    let p: Party = "(1, 0)".parse().expect("test party literals parse");
    let v: Version = "(0, 0, 18446744073709551616)"
        .parse()
        .expect("test version literals parse");
    assert!(
        flag_of(&v, &p),
        "a multiple-of-2^64 raise offset reads nonzero: the flag must trip"
    );
    assert_tick(&v, &p);

    // The declined dual: max(max(el) = 2^64, min(er) = 0) = el — the raise
    // reproduces the wide input leaf, the offset is a zero computed as the
    // difference of two wide values, and the flag must stay clear.
    let v: Version = "(0, 18446744073709551616, 0)"
        .parse()
        .expect("test version literals parse");
    assert!(
        !flag_of(&v, &p),
        "a wide value-reproducing raise reads a full-width zero: the flag stays clear"
    );
    assert_tick(&v, &p);
}

/// A dominated undercut's residue carries the emission's offset at the
/// documented polarity: the tracked minimum drops to exactly `v = h + offset`,
/// so a raise reading that minimum later emits the true value.
///
/// The driven path: the left-full raise diverges the walk, the copied sibling
/// region climbs beyond `u64` and returns, and the region's block-minimum
/// emission then arrives with a word-scale negative offset against a
/// wide-negative anchor gap — the watermark web's scale-disparate undercut,
/// whose residue is `m − v = −gap − offset`. Folding the offset into that
/// residue with the opposite polarity would leave a phantom `2·|offset|`
/// boundary on the difference stack, and the enclosing range's minimum — read
/// by the root's right-full raise — would come out low by exactly that:
/// wrong tick output, caught here against the recursive oracle.
#[test]
fn dominated_undercut_residue_carries_its_offset() {
    let p: Party = "((1, 0), 1)".parse().expect("test party literals parse");
    let v: Version = "(0, (0, 0, (3, (0, 237684487543081243156783562749, 0), 1)), 0)"
        .parse()
        .expect("test version literals parse");
    crate::meter::reset_emit_traffic();
    assert_tick(&v, &p);
    // The witness binds to its branch only through the walk's current
    // routing, and reachability is sensitive to the accumulator's digit
    // state — so the binding is asserted, not assumed: the decision
    // counter must show the dominated-undercut arm answered. (`≥`, not
    // `=`: the counter is process-global, and other work in a shared
    // process only adds.)
    assert!(
        crate::meter::emit_traffic().dominated_undercut >= 1,
        "the witness pair no longer routes its block-minimum emission through \
         the dominated-undercut arm: the path it exists to pin is undriven"
    );
}

/// The dominated-undercut family ticks byte-identically to the recursive
/// oracle at every knob.
///
/// Each site's copied sibling region climbs a wide leaf and returns, so
/// its block-minimum emission arrives against a scale-disparate anchor
/// gap, and the raise reading the surviving minimum stays exact.
///
/// The size-generic generalization of the worked witness above, biased
/// toward scale-disparate emissions: per site, a raise value, an exit
/// rise, and a wide climb `m · 2^b` with most of the width range past the
/// domination read's decision bound — so the family keeps generator mass
/// on emissions the post-sign domination arms answer, with the smaller
/// widths exercising the fold-and-restore path beside them. The raise
/// value reaches down to zero, which seats the site's raise exactly at its
/// sibling region's minimum: the tie at the raise decision's own boundary,
/// where the comparison deciding which side arms reads neither strictly
/// below nor strictly above. The `dominated-undercut` meter family pins
/// the arm's decision count and touch cost at committed scales; this
/// differential pins the values over the whole knob space.
#[test]
fn dominated_undercut_family_ticks_identically() {
    let site = |c: u64, m: u64, b: usize, r: u64| -> String {
        let wide = UBig::from(m) << b;
        format!("({c}, (0, {wide}, 0), {r})")
    };
    let strategy = proptest::collection::vec((0u64..=6, 1u64..=7, 64usize..=192, 1u64..=4), 1..=4);
    proptest!(|(sites in strategy)| {
        let mut text = String::new();
        for (c, m, b, r) in &sites {
            text.push_str(&format!("(0, (0, 0, {}), ", site(*c, *m, *b, *r)));
        }
        text.push('0');
        text.push_str(&")".repeat(sites.len()));
        let v: Version = text.parse().expect("the site literal is normal form");
        let id = "((1, 0), ".repeat(sites.len()) + "1" + &")".repeat(sites.len());
        let p: Party = id.parse().expect("the site id literal parses");
        assert_tick(&v, &p);
    });
}

/// Build an undercut-under-a-live-relation pair: a chain of covered left-full
/// sites around an id-absent region whose block-minimum emission undercuts the
/// web while the ledger relation rides its follower slot.
///
/// The walk order, under one outermost site whose fresh pre-scan covers the
/// whole chain: the root site's raise declines and reproduces its single-leaf
/// collapse range, so the walk enters the chain verbatim. Each `pre` site's
/// multi-leaf collapse (peak `z`, over a sibling leaf `y`) diverges the walk,
/// and its close re-anchors the ledger relation onto the web's follower slot
/// (`pop_site`). The id-absent region then arms the web at its `climb` leaf
/// and drops back to a minimum sitting `exit_rise` above the region's exit,
/// so its block-minimum emission undercuts the freshly-armed anchor with the
/// follower live — through the post-sign domination arm when the climb
/// dominates at scale, the fold-and-restore path when it is comparable, and
/// the at-height emission when the exit rise is zero. The `posts` sites then
/// nest — each inside the previous site's sibling — so consecutive raise
/// decisions read the relation with no intervening `pop_site`; the terminal
/// site's decision is directed by `(min_side, margin)`: the minimum side
/// raises a zero collapse leaf to a sibling minimum sitting `margin` above
/// it, the declined side emits a collapse leaf sitting `margin` over a zero
/// sibling minimum. A follower displaced by a wrong-polarity residue fold —
/// an error at the dying anchor gap's own width — reads the other side of
/// those decisions and emits the wrong bytes.
fn live_relation_undercut_pair(
    outer: u64,
    pre: &[(u64, u64)],
    climb: &Base,
    exit_rise: u64,
    posts: &[u64],
    (min_side, margin): (bool, u64),
) -> (Version, Party) {
    use crate::oracle::{Party as P, Version as V};
    let full = P::seed;
    let empty = || P::Leaf(false);
    // (1, 0): an id node over a consumed leaf — the leaf arm.
    let over_leaf = || P::node(full(), empty());

    let (xl, xr) = if min_side { (0, margin) } else { (margin, 0) };
    let mut post = V::node(0u64, V::leaf(xl), V::leaf(xr));
    let mut post_id = P::node(full(), over_leaf());
    for &w in posts.iter().rev() {
        post = V::node(0u64, V::leaf(w), post);
        post_id = P::node(full(), post_id);
    }
    let region = V::node(
        0u64,
        V::node(0u64, V::leaf(climb.clone()), V::leaf(0u64)),
        V::leaf(exit_rise),
    );
    let mut er = V::node(0u64, region, post);
    let mut ir = P::node(empty(), post_id);
    for &(z, y) in pre.iter().rev() {
        let site = V::node(0u64, V::node(0u64, V::leaf(0u64), V::leaf(z)), V::leaf(y));
        er = V::node(0u64, site, er);
        ir = P::node(P::node(full(), over_leaf()), ir);
    }
    let root = V::node(0u64, V::leaf(outer), er);
    let root_id = P::node(full(), ir);
    (from_oracle_version(&root), from_oracle_party(&root_id))
}

/// A dominated undercut under a live ledger relation moves the relation's
/// follower by exactly its residue: a later covered site's raise decision
/// reads the follower, and the tick matches the oracle.
///
/// The worked point of [`live_relation_undercut_pair`]: one diverging site
/// re-anchors the relation, the region's climb sits past the domination
/// read's decision bound so the undercut is answered scale-disparately with
/// the follower live, and the terminal site's raise then reads the minimum
/// side by the thinnest margin — where a follower displaced by a
/// wrong-polarity residue fold reads the other side and emits the declined
/// raise value in place of the oracle's raise. `pop_site` does not intervene
/// between the undercut and that read: only ordinary node closes separate
/// them, and those never touch a follower's value.
///
/// The witness binds to the domination arm only through the walk's current
/// routing, so the binding is asserted via the decision counter, not
/// assumed (`≥`, not `=`: the counter is process-global, and other work in
/// a shared process only adds).
#[test]
fn dominated_undercut_moves_the_live_ledger_relation() {
    let climb = (Base::from(1u8) << 96u32) + (Base::from(1u8) << 98u32);
    let (v, p) = live_relation_undercut_pair(7, &[(5, 3)], &climb, 9, &[], (true, 1));
    crate::meter::reset_emit_traffic();
    assert_tick(&v, &p);
    assert!(
        crate::meter::emit_traffic().dominated_undercut >= 1,
        "the witness pair no longer routes its block-minimum emission through \
         the dominated-undercut arm: the path it exists to pin is undriven"
    );
}

/// A word-scale at-height undercut under a live ledger relation moves the
/// relation's follower by exactly its residue: the terminal site's raise
/// decision reads the follower, and the tick matches the oracle.
///
/// The narrow twin of the domination witness above, at the family's shrink
/// floor: a unit climb and a zero exit rise route the region's block-minimum
/// emission through the at-height undercut instead of the domination arm, so
/// the residue reaches the live follower through the propagation fold that
/// resolves anchor tags. The terminal raise then reads the minimum side by
/// the thinnest margin, where a wrong-polarity fold reads the other side.
/// This exact pair is the family's shrunk counterexample under that
/// polarity error, kept as a worked point so the narrow arm stays pinned
/// independently of the generator.
#[test]
fn narrow_undercut_moves_the_live_ledger_relation() {
    let (v, p) = live_relation_undercut_pair(0, &[(1, 0)], &Base::from(1u8), 0, &[], (true, 1));
    assert_tick(&v, &p);
}

proptest! {
    /// The undercut-under-a-live-relation family ticks byte-identically to
    /// the recursive oracle at every knob.
    ///
    /// The shape- and width-generic generalization of the worked witness
    /// above, over [`live_relation_undercut_pair`]: swept over the outer
    /// raise value, the diverging pre-site chain's length and heights (an
    /// empty chain leaves the relation height-carried through the region,
    /// covering the height-anchored consume beside the follower reads), the
    /// region's climb `m · 2^b` across the full width range (narrow climbs
    /// route the undercut through the fold-and-restore path, wide ones
    /// through the post-sign domination arm), its exit rise (zero routes
    /// through the at-height emission), the nested post-site chain's depth,
    /// and the terminal decision's side at margins down to one. Every arm
    /// lands in a residue propagation that folds the live follower, and
    /// every follower read surfaces in the output bytes the oracle
    /// differential pins.
    #[test]
    fn undercut_under_a_live_relation_family_ticks_identically(
        outer in 0u64..=8,
        // Pre-sites (z, y): peaks are nonzero so each collapse range stays a
        // real plateau split (a zero peak joins to a single leaf and the site
        // no longer diverges the walk).
        pre in proptest::collection::vec((1u64..=6, 0u64..=6), 0..=2),
        climb in (1u64..=7, 0u32..=192),
        exit_rise in 0u64..=9,
        posts in proptest::collection::vec(0u64..=6, 0..=2),
        terminal in (proptest::bool::ANY, 1u64..=6),
    ) {
        let (m, b) = climb;
        let climb = Base::from(m) << b;
        let (v, p) = live_relation_undercut_pair(outer, &pre, &climb, exit_rise, &posts, terminal);
        assert_tick(&v, &p);
    }
}

/// Build a pair whose sibling walk stacks two armed boundaries — a word-scale
/// `w` over a `m · 2^(32c)`-scale one — and drops to `eps` inside the
/// innermost range.
///
/// The arming undercut's residue is then `m·2^(32c) + w − eps` (top digit `m`
/// over a zero digit), meeting the word boundary at exactly two digits of
/// clearance.
///
/// The clearance is the point: the propagation's width guard enters on the
/// digit counts alone, and top-index domination at that clearance is honest —
/// a residue whose top digit is 1 or 2 sits inside the redundant-spelling
/// operand bound, so the read answers undecided and the fold falls through to
/// the total comparable-scale path; a top digit of 3 or more certifies and
/// takes the dominated arm. Every leaf of the sibling rides an id node, so
/// the walk descends and emits per leaf (no block copy), and the trailing
/// zero leaf keeps the root raise declined.
fn undecided_residue_pair(m: u64, c: u32, w: u64, eps: u64) -> (Version, Party) {
    use crate::oracle::{Party as P, Version as V};
    let over_leaf = || P::node(P::seed(), P::Leaf(false));
    let big = Base::from(m) << (32 * c);
    let upper = V::node(
        0u64,
        V::leaf(big.clone() + &Base::from(w)),
        V::leaf(Base::from(eps)),
    );
    let site = V::node(0u64, V::leaf(big), upper);
    let er = V::node(0u64, site, V::leaf(0u64));
    let root = V::node(0u64, V::leaf(0u64), er);
    let i_upper = P::node(over_leaf(), over_leaf());
    let i_site = P::node(over_leaf(), i_upper);
    let ir = P::node(i_site, over_leaf());
    let root_id = P::node(P::seed(), ir);
    (from_oracle_version(&root), from_oracle_party(&root_id))
}

proptest! {
    /// The undercut-residue family at the domination clearance ticks
    /// byte-identically to the recursive oracle across the read's whole
    /// decision boundary.
    ///
    /// Top digits of 1 and 2 leave the width guard's domination read
    /// honestly undecided (the fall-through total fold carries the drop), 3
    /// and above certify and take the dominated arm, and the surviving
    /// boundary's shrunk difference feeds the enclosing minimum the root
    /// raise reads — so a wrong direction on either side of the guard lands
    /// in the output bytes.
    #[test]
    fn undercut_residue_at_the_domination_clearance_ticks_identically(
        m in 1u64..=7,
        c in 2u32..=6,
        w in 2u64..=7,
        eps_below in 1u64..=6,
    ) {
        let eps = eps_below.min(w - 1);
        let (v, p) = undecided_residue_pair(m, c, w, eps);
        assert_tick(&v, &p);
    }
}

/// The left-full raise decision's height seam: the tick matches the oracle on
/// a pair whose sibling range moves the height between the site's collapse
/// scan and the site's close.
///
/// The pre-scan records an interior left-full site's sibling minimum and
/// mirrors no raise for it; the walk alone decides the raise, at the site's
/// own vantage (`prescan.rs`'s site-close arm carries the argument). This pair
/// makes the vantage load-bearing: `max(el) = 5` sits strictly below the
/// sibling minimum `m_s = 9` — the raise takes the min side — while the
/// sibling range's net height movement (+4) carries `max(el) + net` up to
/// exactly `m_s`, so a raise decision denominated at the close's height
/// instead of the site's reads the other side of the boundary. The oracle
/// differential pins the walk's decision byte for byte.
#[test]
fn left_full_raise_decides_at_the_site_not_its_close() {
    let p: Party = "(1, (1, (1, 0)))"
        .parse()
        .expect("test party literals parse");
    let v: Version = "(0, 0, (0, 5, (0, 0, 9)))"
        .parse()
        .expect("test version literals parse");
    assert_tick(&v, &p);
}

/// Build a latent-ladder pair: an arming spine whose closes park a latent
/// boundary, beside a sibling whose raise decision reads it through the
/// domination ladder.
///
/// The driven path, with every enclosing base zero: the leaf left of the spine
/// arms the web at height `0`; each spine level's zero leaf arms one level
/// deeper (the outermost spine base is the surviving innermost minimum
/// `m = bases.last()`), the tip leaf arms last and highest, and the spine's
/// closes then pop those arming boundaries back — one park per level, the
/// first a mint and the rest merges — leaving the latent
/// `Λ = tip + Σ bases[..last]` under the anchor `A = m + Λ`. The sibling
/// consumed by the right-full raise probes the ladder at `v = peak`: the
/// latent `Λ` decides against the drop `δ = A − v`, with the true minimum at
/// `m`. A zero `peak` places a lone zero leaf as the sibling (the two-leaf
/// spelling would collapse); the raise consumes either shape identically.
///
/// A second right-full raise wraps the ladder site and reads the web again
/// over a lone zero leaf: its raise lifts that leaf to the tracked minimum
/// `m`, coding the post-ladder web state into the output stream — a ladder
/// exit that misplaces the web (a wrong fold-restore, a wrong post-collapse
/// re-test) surfaces as wrong bytes rather than dying unread when the last
/// armed range retires.
fn latent_ladder_pair(bases: &[Base], tip: &Base, peak: &Base) -> (Version, Party) {
    use crate::oracle::{Party as P, Version as V};
    // (1, 0): an id node over a consumed leaf — the leaf's emission arms the
    // web without the id owning or moving anything.
    let owned = || P::node(P::seed(), P::Leaf(false));
    let (mut spine, mut spine_id) = (
        V::node(bases[0].clone(), V::leaf(0u64), V::leaf(tip.clone())),
        P::node(owned(), owned()),
    );
    for b in &bases[1..] {
        spine = V::node(b.clone(), V::leaf(0u64), spine);
        spine_id = P::node(owned(), spine_id);
    }
    let sibling = if *peak == Base::ZERO {
        V::leaf(0u64)
    } else {
        V::node(0u64, V::leaf(peak.clone()), V::leaf(0u64))
    };
    let x = V::node(0u64, spine, sibling);
    // The sibling sits under a full id: its consumption is the right-full
    // raise, whose decision read and declined-raise emission are the walk's
    // latent-live reads of the web.
    let ix = P::node(spine_id, P::seed());
    // The wrapping reader: another right-full raise over a zero leaf, whose
    // emission is the tracked minimum read back from the web.
    let w = V::node(0u64, x, V::leaf(0u64));
    let iw = P::node(ix, P::seed());
    let g = V::node(0u64, V::leaf(0u64), w);
    let ig = P::node(owned(), iw);
    let root = V::node(0u64, g, V::leaf(0u64));
    let ip = P::node(ig, P::Leaf(false));
    (from_oracle_version(&root), from_oracle_party(&ip))
}

/// `5 · 2^96`: a ladder operand past the `2^97` certificate line.
fn five_p96() -> Base {
    (Base::from(1u8) << 96u32) + (Base::from(1u8) << 98u32)
}

/// A dominating latent declines a word-scale drop: the emission lands between
/// the true minimum and the anchor (`m < v < A`, `Λ ≥ 5·2^96`, `A − v = 7`),
/// so the tracked minimum must not move and the raise emits `v` itself.
///
/// The worked witness for the ladder's latent-dominates arm and both of its
/// reads: the raise decision (`compare_above`) and the declined drop's exact
/// fold-restore in `emit_offset` (the latent-decide-false gate). Restoring
/// that fold with the wrong polarity displaces the anchor web by `2·|offset|`
/// for every later read; the enclosing minimum then comes out wrong — caught
/// here against the recursive oracle.
#[test]
fn wide_latent_dominates_a_word_scale_drop() {
    let (v, p) = latent_ladder_pair(
        &[Base::from(1u8)],
        &(five_p96() + 7u64),
        &(five_p96() + 1u64),
    );
    assert_tick(&v, &p);
}

/// A wide drop dominates a word-scale latent: the emission reads strictly
/// below the true minimum (`v < m`, `m ≥ 5·2^96`, `Λ = 9`), so the raise must
/// lift the consumed sibling to exactly `m`.
///
/// The worked witness for the ladder's gap-dominates arm: the decision is the
/// O(1) domination read of the drop against the latent, answered without a
/// collapse, and the raise then emits the tracked minimum. A polarity error
/// there declines the raise instead and emits the low sibling value — caught
/// here against the recursive oracle.
#[test]
fn wide_drop_dominates_a_word_scale_latent() {
    let (v, p) = latent_ladder_pair(&[five_p96()], &Base::from(9u8), &Base::ZERO);
    assert_tick(&v, &p);
}

/// Comparable scales collapse the latent, and the re-based re-test reads the
/// drop stopping above the true minimum (`m < v`, `Λ = 2^97`, `A − v = 2^96`):
/// the minimum must not move and the raise emits `v` itself.
///
/// The worked witness for the ladder's comparable-collapse arm on its
/// non-undercut side: neither operand's top digit certifies domination, the
/// near-cancellation funds the latent's retirement, and the post-collapse
/// plain sign read declines the drop. A wrong post-collapse polarity turns
/// the declined drop into an undercut of the re-based anchor — caught here
/// against the recursive oracle.
#[test]
fn comparable_scales_collapse_above_the_true_minimum() {
    let (v, p) = latent_ladder_pair(
        &[Base::from(1u8)],
        &(Base::from(1u8) << 97u32),
        &((Base::from(1u8) << 96u32) + 1u64),
    );
    assert_tick(&v, &p);
}

/// Comparable scales collapse the latent, and the re-based re-test reads the
/// drop passing the true minimum (`v < m`, `Λ = 5·2^96`, `A − v = 5·2^96 +
/// 3`): the raise must lift the consumed sibling to exactly `m`.
///
/// The worked witness for the ladder's comparable-collapse arm on its
/// undercut side: the collapse re-bases the anchor to the true minimum and
/// the plain re-test still reads a drop, so the raise emits the minimum. A
/// wrong post-collapse polarity emits the low sibling value instead — caught
/// here against the recursive oracle.
#[test]
fn comparable_scales_collapse_under_the_true_minimum() {
    let (v, p) = latent_ladder_pair(&[Base::from(5u8)], &five_p96(), &Base::from(2u8));
    assert_tick(&v, &p);
}

/// The dominating side of a scale-disparate ladder case: `[5·2^96, 2^100)`,
/// past the `2^97` certificate line in both accumulator representations.
///
/// The register certifies from `3·2^(32·(floor+1))`; the digit fold from a
/// partial of `3` at digit index `floor + 2` — here index 3 against
/// word-scale floors.
fn arb_ladder_dominant() -> impl Strategy<Value = Base> {
    (5u128 << 64..1u128 << 68).prop_map(|n| Base::from(n) << 32u32)
}

/// One comparable-scale ladder operand: `[2^96, 2^99)` — top digit index 3
/// with a small partial, so no draw dominates another draw (or a sum of two)
/// and every pairing takes the collapse arm.
fn arb_ladder_comparable() -> impl Strategy<Value = Base> {
    (1u128 << 64..1u128 << 67).prop_map(|n| Base::from(n) << 32u32)
}

/// One latent-ladder case for [`latent_ladder_pair`] — `(bases, tip, peak)` —
/// drawn to land in a chosen ladder relation, with the spine depth (one to
/// three parks: a mint plus up to two merges) and every scale free.
fn arb_latent_ladder() -> impl Strategy<Value = (Vec<Base>, Base, Base)> {
    let inner = proptest::collection::vec((1u64..1000).prop_map(Base::from), 0..=2);
    prop_oneof![
        // Latent-dominates: Λ ≥ 5·2^96 against a word-scale drop δ, the
        // emission landing between the true minimum and the anchor.
        (
            inner.clone(),
            1u64..1000,
            arb_ladder_dominant(),
            1u64..1 << 63
        )
            .prop_map(|(mut bases, outer, dominant, delta)| {
                bases.push(Base::from(outer));
                let sum = bases.iter().fold(Base::ZERO, |a, b| a + b);
                (bases, dominant.clone() + delta, sum + dominant)
            },),
        // Gap-dominates: a drop past a huge outermost minimum against a
        // word-scale latent — a true undercut, the raise lifting to m.
        (
            inner.clone(),
            arb_ladder_dominant(),
            1u64..1 << 32,
            0u64..1000
        )
            .prop_map(|(mut bases, outer, tip, peak)| {
                bases.push(outer);
                (bases, Base::from(tip), Base::from(peak))
            },),
        // Comparable scales, the drop stopping above the true minimum: the
        // collapse re-bases the anchor and the raise still emits v itself.
        (
            inner.clone(),
            2u64..1000,
            arb_ladder_comparable(),
            arb_ladder_comparable(),
        )
            .prop_map(|(mut bases, outer, u, t)| {
                bases.push(Base::from(outer));
                (bases, u + &t, Base::from(outer) + t)
            }),
        // Comparable scales, the drop passing the true minimum: the
        // post-collapse re-test reads a plain undercut and the raise lifts
        // to m.
        (
            inner,
            (2u64..1000).prop_flat_map(|outer| (Just(outer), 0..outer)),
            arb_ladder_comparable(),
            arb_ladder_comparable(),
        )
            .prop_map(|(mut bases, (outer, peak), u, t)| {
                bases.push(Base::from(outer));
                (bases, u + t, Base::from(peak))
            }),
    ]
}

proptest! {
    /// Latent-parking armings tick byte-identically to the recursive oracle
    /// across the domination ladder: dominating latents decline, dominated
    /// ones undercut, comparable scales collapse — both re-test sides.
    ///
    /// The generalized family behind the four worked witnesses above: spine
    /// depth sweeps the park chain (a mint plus up to two merges), the
    /// dominating sides draw from `[5·2^96, 2^100)` so the domination
    /// certificate fires in either accumulator representation, and the
    /// comparable arm draws both operands at top digit index 3 so neither
    /// ever certifies. Every case funnels the parked latent into a raise
    /// decision and its emission, where a polarity error in the ladder's
    /// declined-drop restore or its post-collapse re-test displaces the
    /// enclosing minimum — checked total against the recursive oracle.
    #[test]
    fn latent_ladder_ticks_identically(case in arb_latent_ladder()) {
        let (bases, tip, peak) = case;
        let (v, p) = latent_ladder_pair(&bases, &tip, &peak);
        assert_tick(&v, &p);
    }
}

/// The version that is `1` on the leftmost `2^-depth` interval and `0`
/// everywhere else: `depth` nested nodes, all bases zero, the single 1-leaf at
/// the bottom left.
///
/// Built as a text literal — the parser is iterative — so the expected tree
/// shares no walk with the kernel under test.
fn left_spike(depth: usize) -> Version {
    let mut text = "(0, ".repeat(depth - 1);
    text.push_str("(0, 1, 0)");
    text.push_str(&", 0)".repeat(depth - 1));
    text.parse().expect("the spike literal is normal form")
}

/// Deep spines in every regime stay correct at depths that would overflow a
/// native-frame walk.
///
/// The regimes: the collapse scan, the pass-through copy, the two-cursor
/// descent, the memoized pre-scan, and both fused epilogues (the prefix
/// materialization and the route-driven splice).
///
/// The recursive oracle walks on native frames (it is the small-scope
/// reference, not a deep-input one), so the value witnesses here are closed
/// forms, derived per case: the full id collapses the whole spine to one leaf
/// at its maximum height — the alternating spine's only nonzero leaf is the `1`
/// at the bottom pair — and that collapse trips the flag, so it is also the
/// tick; the deep unary id over the empty version leaves the flag clear (fill
/// of a leaf under a node id is the leaf) and ticks to the left spike, the
/// expansion chain to its owned tip; and over the deep spine the same id turns
/// left into the spine's depth-2 zero leaf (the spine's structure continues
/// right there), so the flag again stays clear and the grown tree raises
/// exactly the owned region from 0 to 1 — the pointwise max with the spike,
/// realized through the independently-pinned join kernel and byte-exact by
/// canonical uniqueness. The nested-full- sibling id over its matched spine
/// leaves the flag clear, derived: every level's right-full raise is
/// `max(max(er), min(fill(il, el)))`, where `er` is a single leaf (its own
/// maximum) and the left range's minimum stays at the spine's floor of zero, so
/// no raise ever moves a value — and the id terminus pairs with a leaf (the
/// untouched-leaf arm). The case drives the deferred right-full decision and
/// its per-level raise bookkeeping at full depth with a value witness the small
/// scope pins against the oracle. The mirror id over the wide-tail spine
/// collapses to the single wide leaf, derived bottom-up: the deepest left-full
/// raise is `max(max(el), min(fill(ir, er)))` with `el` a lone zero leaf and
/// `er` the wide tail itself (a leaf's minimum is its value), so the raise
/// lifts the zero leaf to the tail's value, the equal sibling pair collapses,
/// and each enclosing level sees the same wide leaf as its right minimum — the
/// collapse telescopes to the root. The case drives the memoized pre-scan at
/// full depth with wide minima in every memo entry, and the collapse trips the
/// flag. The staircase under the unary id spine leaves the flag clear — its
/// levels pair internal × internal down to the terminus, whose left-full raise
/// `max(max(a), min(b))` with `a` one step above `b` returns `a` itself — and
/// its tick increments the id's owned tip, the bottom-left leaf; every consumed
/// leaf undercuts every open range on the way, the full-penetration
/// minimum-update schedule. A changed pair's output re-ticks through the grow
/// branch (fill is idempotent, restated as the flag reading clear on a filled
/// stream); canonicality and tick entry agreement ride along on every case.
#[test]
fn deep_spines_tick_and_flag_identically() {
    // A changed pair: the flag trips, the fill-branch stream is the derived
    // closed form, and re-running the walk on that stream leaves the flag clear
    // (fill idempotence, flag-denominated).
    let assert_deep_changed = |v: &Version, p: &Party, expected: &Version| {
        let enc = encode(v);
        match fused_fill(&enc, p) {
            FillOutcome::Changed(bits) => {
                validate(&bits).expect("a filled stream is canonical");
                assert_eq!(bits, encode(expected), "the derived closed form");
                let again: Version = Version::from_bits(bits.clone());
                assert!(
                    !flag_of(&again, p),
                    "a filled stream re-ticks through the grow branch"
                );
            }
            FillOutcome::Unchanged(_) => panic!("fill moves this pair: the flag must trip"),
        }
        assert_eq!(
            tick(&enc, p),
            encode(expected),
            "tick takes the fill branch: the collapse"
        );
        let mut ticked = v.clone();
        ticked.tick(p);
        assert_eq!(encode(&ticked), encode(expected), "entry agreement");
    };
    // An unchanged pair: the flag stays clear and the tick is the derived grow
    // closed form.
    let assert_deep_unchanged = |v: &Version, p: &Party, grown: &Version| {
        let enc = encode(v);
        assert!(!flag_of(v, p), "fill is the identity: the flag stays clear");
        let out = tick(&enc, p);
        validate(&out).expect("a ticked stream is canonical");
        assert_eq!(out, encode(grown), "the derived grow closed form");
        let mut ticked = v.clone();
        ticked.tick(p);
        assert_eq!(encode(&ticked), encode(grown), "entry agreement");
    };

    let deep_ev = version_of(&Shape::AltSpine.packed1(4096));
    let deep_id = party_of(&Shape::IdSpine.packed_flagged(4096, false));
    let spike = left_spike(4096);
    let one: Version = "1".parse().expect("test literals parse");

    // The full id: the collapse to the maximum leaf.
    assert_deep_changed(&deep_ev, &Party::seed(), &one);

    // The deep unary id over the empty version: identity fill, so tick grows
    // the expansion chain to the owned tip.
    assert_deep_unchanged(&Version::new(), &deep_id, &spike);

    // Both deep: no fully-owned region meets a subdividable subtree, so fill is
    // the identity; the grown tree is the pointwise max with the spike.
    assert_deep_unchanged(&deep_ev, &deep_id, &(&deep_ev | &spike));

    // The nested-full-sibling id over its matched spine: a right-full shortcut
    // site at every one of the 4096 levels — the deferred right-full decision
    // and its raise bookkeeping at full depth. Fill is the identity (the doc
    // comment's derivation: each raise maxes a lone leaf against a zero
    // minimum), so the tick registers the inflation: the id's cheapest site is
    // the terminus leaf at the bottom right, `(0, 0, 1)` over the matched
    // spine's zero — the pointwise max with the deepest-right unit spike.
    let mut text = "(0, ".repeat(4095);
    text.push_str("(0, 0, 1)");
    text.push_str(&", 0)".repeat(4095));
    let matched: Version = text.parse().expect("the matched spine literal parses");
    let nested = party_of(&Shape::NestedFullId.packed1(4096));
    assert!(
        !flag_of(&matched, &nested),
        "every nested raise maxes a lone leaf against a zero minimum: identity"
    );
    let mut grown = matched.clone();
    grown.tick(&nested);
    validate(&encode(&grown)).expect("a ticked stream is canonical");

    // The mirror id over the wide-tail spine: a left-full shortcut site at
    // every one of the 4096 levels — the memoized pre-scan at full depth, every
    // memo entry a wide minimum. The deepest raise lifts its zero leaf to the
    // tail's value, the equal pair collapses, and the collapse telescopes to
    // the root: fill is the single wide leaf, and the flag trips.
    let mut text = "(0, 0, ".repeat(4095);
    text.push_str(&format!("(0, 0, {})", u64::MAX));
    text.push_str(&")".repeat(4095));
    let tail: Version = text.parse().expect("the wide-tail literal parses");
    let wide_leaf: Version = u64::MAX
        .to_string()
        .parse()
        .expect("the leaf literal parses");
    let mirror = party_of(&Shape::NestedLeftFullId.packed1(4096));
    assert_deep_changed(&tail, &mirror, &wide_leaf);

    // The descending staircase under the unary id spine: every consumed leaf
    // undercuts every open range — the full-penetration minimum-update schedule
    // at 4096 levels, all values word-scale. Fill is the identity (internal ×
    // internal at every level above the terminus, whose raise `max(a, min(b))`
    // returns `a` itself), so tick grows: the id's owned tip is the bottom-left
    // leaf, a zero-expansion increment and the only owned site.
    let mut text = "(0, ".to_string();
    text.push_str(&"(1, ".repeat(4094));
    text.push_str("(1, 1, 0)");
    text.push_str(&", 0)".repeat(4095));
    let stairs: Version = text.parse().expect("the staircase literal parses");
    let spine_id = party_of(&Shape::IdSpine.packed_flagged(4096, false));
    let mut text = "(0, ".to_string();
    text.push_str(&"(1, ".repeat(4094));
    text.push_str("(1, 2, 0)");
    text.push_str(&", 0)".repeat(4095));
    let grown: Version = text.parse().expect("the grown staircase literal parses");
    assert_deep_unchanged(&stairs, &spine_id, &grown);

    // The memo chain at 4096 sites: every interior left-full site collapses its
    // `(0, 0, j)` node to the leaf `j` (the raise meets the site's own
    // single-leaf range), and the covering site's raise stays at the tree
    // minimum 0 — so the filled tree is the spine with each site replaced by
    // its leaf, in closed form. The walk resolves all 4096 memoized minima from
    // one fresh scan.
    let k = 4_096u64;
    let mut text = "(0, 0, ".to_string();
    for j in 1..=k {
        text.push_str(&format!("(0, {j}, "));
    }
    text.push('0');
    text.push_str(&")".repeat(k as usize + 1));
    let expected: Version = text.parse().expect("the chain literal parses");
    let chain = Shape::MemoChain.packed_flagged(k as usize, true).version();
    let chain_id = party_of(&Shape::MemoChainId.packed1(k as usize));
    assert_deep_changed(&chain, &chain_id, &expected);

    // The reveal comb at 4096 sites: every site's left-full raise meets its
    // single-leaf range at the shared minimum `2^b` (the raise is `max(2^b − 1,
    // 2^b)`), so each site collapses to the leaf `2^b`, while the covering
    // site's raise stays at the tree minimum 0 (the floor) — the filled tree is
    // the comb with each site replaced by its wide leaf, in closed form.
    // Between consecutive site consumes the site's node frame closes back into
    // the 0-floor frame: the close-reveal cycle at full depth.
    let (k, b) = (4_096usize, 8usize);
    let w = 1u64 << b;
    let mut text = "(0, 0, ".to_string();
    text.push_str(&"(0, ".repeat(k - 1));
    text.push_str(&format!("(0, 0, {w})"));
    text.push_str(&format!(", {w})").repeat(k - 1));
    text.push(')');
    let expected: Version = text.parse().expect("the reveal-comb literal parses");
    let comb = Shape::RevealComb.packed2(k, b).version();
    let comb_id = party_of(&Shape::RevealCombId.packed1(k));
    assert_deep_changed(&comb, &comb_id, &expected);

    // The ascending cliff at 4096 spine nodes: fill is the identity (no id
    // region covers a subdividable subtree at its minimum), so tick grows — the
    // id's owned site is the cliff leaf, which expands to (0, 1, 0). The
    // cliff's wide undercut propagates through 4095 nonzero unit boundary
    // differences on the way: the fold-direction cascade at full depth.
    let (k, b) = (4_096usize, 13usize);
    let w = 1u64 << b;
    let mut text = String::new();
    for i in 1..=k {
        text.push_str(&format!("(0, {}, ", w + i as u64));
    }
    text.push_str("(0, 1, 0)");
    text.push_str(&")".repeat(k));
    let expected: Version = text
        .parse()
        .expect("the grown ascending-cliff literal parses");
    let cliff = Shape::AscendCliff.packed2(k, b).version();
    let cliff_id = party_of(&Shape::AscendCliffId.packed1(k));
    assert_deep_unchanged(&cliff, &cliff_id, &expected);
}

/// The tick takes both branches: a fill that simplifies is the tick, and a fill
/// that changes nothing falls through to the grow splice.
#[test]
fn tick_splices_fill_and_grow() {
    // fill simplifies: the collapse is the tick.
    let v: Version = "(2, 0, 1)".parse().expect("test literals parse");
    let p: Party = "(1, 0)".parse().expect("test literals parse");
    assert_eq!(tick(&encode(&v), &p), encode(&"3".parse().unwrap()));
    // fill is the identity: grow registers the event.
    let v: Version = "(0, 1, 0)".parse().expect("test literals parse");
    assert_eq!(tick(&encode(&v), &p), encode(&"(0, 2, 0)".parse().unwrap()));
    assert_tick(&v, &p);
}

proptest! {
    /// Arbitrary owning parties over arbitrary normal-form versions tick
    /// byte-identically to the recursive oracle's event, with the changed flag
    /// agreeing with the oracle's fill, magnitudes past `u64::MAX` included.
    #[test]
    fn arbitrary_pairs_tick_and_flag_identically(
        op in generators::arb_oracle_party_nonempty(),
        ov in generators::arb_oracle_version(),
    ) {
        let p = from_oracle_party(&op);
        let v = from_oracle_version(&ov);
        if !p.as_bits().is_empty() {
            assert_tick(&v, &p);
        }
    }

    /// Organic histories tick byte-identically to the recursive oracle's event,
    /// with the changed flag agreeing with the oracle's fill.
    ///
    /// Every clock produced by one fork/tick/send/sync/join history is
    /// exercised from its own party — and from every *other* clock's party, the
    /// concurrent-editor shape.
    #[test]
    fn organic_histories_tick_and_flag_identically(ops in optrace::world_strategy_up_to(40)) {
        let mut clocks = vec![Clock::seed()];
        for op in &ops {
            optrace::step_impl(&mut clocks, op);
        }
        for a in &clocks {
            for b in &clocks {
                if !b.party().as_bits().is_empty() {
                    assert_tick(a.version(), b.party());
                }
            }
        }
    }

    /// `tick`'s output stays within a constant factor of its inputs' coded
    /// sizes: `bits(tick(e, i)) ≤ 2·bits(e) + 4·bits(i) + 32` over arbitrary
    /// pairs.
    ///
    /// The bound the board's input denomination of the tick rows rests on,
    /// computed the second way (the operation itself, against the arithmetic of
    /// its parts). Fill's raises telescope the codes their collapsed ranges
    /// already spent, and grow adds one increment or one expansion chain, a
    /// constant per id bit at the site — but either can re-code up to two
    /// deltas against a wide neighbor (the raise's landing, grow's zero leaf),
    /// each duplicating one input code's width once. Hence the factor of two,
    /// not an additive slack: 255 output bits from a 175-bit event under a
    /// 6-bit id is honest arithmetic (the zero leaf lands next to a wide
    /// value), while a superlinear output would be minting content no operand
    /// paid for.
    #[test]
    fn tick_output_is_input_bounded(
        op in generators::arb_oracle_party_nonempty(),
        ov in generators::arb_oracle_version(),
    ) {
        let p = from_oracle_party(&op);
        let v = from_oracle_version(&ov);
        if !p.as_bits().is_empty() {
            let ev = encode(&v);
            let out = tick(&ev, &p);
            let bound = 2 * ev.len() + 4 * p.as_bits().len() + 32;
            prop_assert!(
                out.len() <= bound,
                "tick output {} bits exceeds input envelope {} (event {}, id {})",
                out.len(), bound, ev.len(), p.as_bits().len(),
            );
        }
    }

    /// The tick ORBIT's coded size is a bounded transient plus logarithmic
    /// growth.
    ///
    /// After the first tick (whose one-step factor the pin above prices),
    /// `bits(tick^k) ≤ bits(tick^1) + 4·bits(id) + 4·⌈log2(k + 1)⌉ + 8` for
    /// every k along the orbit.
    ///
    /// The per-step multiplicative bound cannot compound: a width duplication
    /// needs an unexpanded id-demanded site adjacent to a wide transition, the
    /// orbit mints expansions at most once per id site (the `4·bits(id)`
    /// transient), re-fired raises re-code the same position rather than
    /// stacking, and the steady state is increments whose two re-coded delta
    /// codes grow with the count's own gamma width — the `log k` term
    /// [measured: the envelope holds with zero slack at the log term on the
    /// committed families over 512 ticks, and the fixed-pair orbit freezes at
    /// +24 bits over 4096 ticks].
    #[test]
    fn tick_orbit_growth_is_transient_plus_log(
        op in generators::arb_oracle_party_nonempty(),
        ov in generators::arb_oracle_version(),
    ) {
        let p = from_oracle_party(&op);
        let v = from_oracle_version(&ov);
        if !p.as_bits().is_empty() {
            let mut e = encode(&v);
            e = tick(&e, &p);
            let b1 = e.len();
            for k in 2u32..=48 {
                e = tick(&e, &p);
                let logk = u64::from(32 - (k + 1).leading_zeros());
                let bound = b1 as u64
                    + 4 * p.as_bits().len() as u64
                    + 4 * logk
                    + 8;
                prop_assert!(
                    e.len() as u64 <= bound,
                    "orbit size {} bits at tick {k} exceeds the transient-plus-log \
                     envelope {bound} (first-tick size {b1}, id {} bits)",
                    e.len(), p.as_bits().len(),
                );
            }
        }
    }
}

/// The deterministic deep orbits stay inside their measured bands.
///
/// The fixed wide pair's size freezes after its one-step transient (growing
/// only with the incremented count's gamma width), and the alternating disjoint
/// pair — whose collapse/expand cycle re-fires a width duplication every period
/// — stays inside the same band forever: re-fired raises re-code the same
/// bounded value at the same position, replacing, never stacking.
#[test]
fn tick_deep_orbits_stay_banded() {
    let ev = Shape::Bigroot.packed2(64, 4).version();
    let ida = party_of(&Shape::IdSpine.packed_flagged(4, false));
    let idb = party_of(&Shape::IdSpine.packed_flagged(4, true));

    let mut e = encode(&ev);
    e = tick(&e, &ida);
    let b1 = e.len();
    for k in 2u32..=4096 {
        e = tick(&e, &ida);
        let logk = usize::try_from(32 - (k + 1).leading_zeros()).expect("small");
        assert!(
            e.len() <= b1 + 4 * logk + 8,
            "fixed-id orbit: {} bits at tick {k} (first-tick size {b1})",
            e.len(),
        );
    }

    let mut e = encode(&ev);
    e = tick(&e, &ida);
    e = tick(&e, &idb);
    let b2 = e.len();
    for k in 3u32..=2048 {
        e = tick(&e, if k % 2 == 1 { &ida } else { &idb });
        let logk = usize::try_from(32 - (k + 1).leading_zeros()).expect("small");
        assert!(
            e.len() <= b2 + 4 * logk + 8,
            "alternating orbit: {} bits at tick {k} (two-tick size {b2})",
            e.len(),
        );
    }
}

// ───────────── the route DP's saturation ceiling ─────────────
//
// The expansion DP saturates feasible distances at a ceiling strictly below
// the infeasible sentinel (`Cost::CEILING` < `Cost::INFEASIBLE`), so a
// feasible chain of any length still compares feasible and the recorded route
// always turns into a present child. The ceiling is a parameter of the rise
// loop exactly so these tests can scale it into constructible range: reaching
// the production ceiling honestly would take more id levels than any physical
// encoding can hold.

/// The party owning exactly the region at the end of `path`: one internal id
/// node per direction (the off-path sibling absent), a full terminal below.
///
/// Built as a text literal — the parser is iterative — so the id shares no
/// construction with the DP under test. The packing is depth-first with
/// absent children unencoded, so the chain's level-`i` 2-bit tag sits at bit
/// `2·i` — the route key the expansion DP records level `i`'s direction at.
fn direction_chain(path: &[bool]) -> Party {
    let mut text = "1".to_string();
    for &left in path.iter().rev() {
        text = if left {
            format!("({text}, 0)")
        } else {
            format!("(0, {text})")
        };
    }
    text.parse().expect("the chain literal is normal form")
}

/// Run the expansion DP over `path`'s direction chain at `ceiling` and assert
/// the saturation contract.
///
/// The cost must be the chain's distance saturated at the ceiling — strictly
/// feasible, never [`Cost::MAX`] — and the route must turn into the present
/// child at every level.
///
/// A rise loop that saturated feasible distances *into* the infeasible
/// sentinel would instead compare the chain equal to its absent sibling,
/// record the tie to the right, and send the splice emit into the absent
/// child (a debug panic, an id-cursor desync in release) — the corner the
/// strict sub-sentinel ceiling closes by construction.
fn assert_chain_saturation(path: &[bool], ceiling: u64) {
    let p = direction_chain(path);
    let bits = p.as_bits();
    let mut probe = RouteProbe::new(bits.len());
    let mut id = IdReader::root(bits);
    let cost = probe.expand_subtree(&mut id, ceiling);
    assert_eq!(id.pos(), bits.len(), "the DP consumes exactly the subtree");
    assert!(
        cost < Cost::MAX,
        "a feasible chain must never read infeasible (depth {}, ceiling {ceiling})",
        path.len(),
    );
    let distance = (path.len() as u64).min(ceiling);
    assert_eq!(
        cost,
        Cost {
            expansions: distance,
            depth: distance,
        },
        "the chain's distance saturates at the ceiling, feasibly",
    );
    let route = probe.take_route();
    for (level, &left) in path.iter().enumerate() {
        assert_eq!(
            route.dirs()[2 * level],
            left,
            "the route at level {level} must turn into the present child",
        );
    }
}

/// The scaled-sentinel witness: at a rise-loop ceiling of 7, a feasible
/// left-only chain one past the bound (distance 8) saturates to 7 and stays
/// feasible.
///
/// The route turns into the present child at every level, and chains around
/// that distance tick byte-identically to the oracle at the production
/// ceiling.
#[test]
fn chain_one_past_the_ceiling_stays_feasible() {
    const SCALED_CEILING: u64 = 7;
    assert_chain_saturation(&[true; 8], SCALED_CEILING);
    // End to end at the production ceiling: the same chain family around the
    // scaled bound, grown over a leaf, against the recursive oracle.
    let leaf: Version = "5".parse().expect("test literals parse");
    for depth in [7, 8, 9] {
        assert_tick(&leaf, &direction_chain(&vec![true; depth]));
    }
}

proptest! {
    /// Feasible expansion chains never alias into the infeasible sentinel at
    /// any rise-loop ceiling.
    ///
    /// Direction chains of arbitrary depth and orientation, crossed with
    /// ceilings the chains can reach and pass: the DP's cost is the distance
    /// saturated at the ceiling — feasible by construction — and the recorded
    /// route turns into the present child at every level, the invariant whose
    /// violation would route the splice emit into an absent id child.
    #[test]
    fn expansion_chains_saturate_strictly_feasible(
        path in proptest::collection::vec(any::<bool>(), 1..=64),
        ceiling in 1u64..=32,
    ) {
        assert_chain_saturation(&path, ceiling);
    }
}

// ───────────────────────────── ticks(n) ─────────────────────────────
//
// The fused multi-tick's differentials: `ticks(n)` must equal `n` sequential
// public ticks byte for byte on every branch — the crux the grow module doc's
// compounding argument and the two-walk fill branch both reduce to — with the
// structural facts the argument rests on (fill idempotence, the grow branch
// absorbing) pinned directly, wide-n self-consistency pinned by the
// monoid-action law seamed to a single ground-truth tick, and the k = 1 splice
// pinned as exactly the tick.

/// [`ticks`] lifted to the stored-value level, through the same `from_bits`
/// gate the public entry commits through.
fn ticks_version(v: &Version, id: &Party, n: &Base) -> Version {
    Version::from_bits(ticks(&encode(v), id, n))
}

/// Check `ticks(n)` against the iterated public tick for every `n` in
/// an ascending list, reusing the iterated prefix.
fn check_ticks_equivalence(v: &Version, p: &Party, ns: &[u32]) {
    let mut iterated = v.clone();
    let mut done = 0u32;
    for &n in ns {
        debug_assert!(n >= done, "ascending n list");
        while done < n {
            iterated.tick(p);
            done += 1;
        }
        let fused = ticks_version(v, p, &Base::from(n));
        assert_eq!(
            fused, iterated,
            "ticks({n}) diverged from {n} iterated ticks: {v} with {p}"
        );
    }
}

proptest! {
    /// The crux differential: `ticks(n)` is byte-identical to `n` sequential
    /// public ticks for `n` in {0, 1, 2, 3, 7, 64}, on arbitrary normal-form
    /// (version, party) pairs — including wide (beyond-u64) leaf magnitudes.
    #[test]
    fn ticks_matches_iterated_ticks_arbitrary(
        ov in generators::arb_oracle_version(),
        op in generators::arb_oracle_party_nonempty(),
    ) {
        let v = from_oracle_version(&ov);
        let p = from_oracle_party(&op);
        check_ticks_equivalence(&v, &p, &[0, 1, 2, 3, 7, 64]);
    }

    /// The single-tick byte pin: the `k = 1` splice is exactly the tick.
    ///
    /// `ticks(1)`, the public `tick`, and the recursive oracle's `event` (the
    /// semantic definition of record, untouched by the `+k` splice
    /// generalization) produce one identical stream on a substantial generated
    /// corpus — the committed guard that generalizing the splice's increment
    /// did not move the protocol.
    #[test]
    fn tick_is_ticks_one(
        ov in generators::arb_oracle_version(),
        op in generators::arb_oracle_party_nonempty(),
    ) {
        let v = from_oracle_version(&ov);
        let p = from_oracle_party(&op);
        let one = ticks_version(&v, &p, &Base::from(1u8));
        let mut ticked = v.clone();
        ticked.tick(&p);
        prop_assert_eq!(&one, &ticked, "ticks(1) diverged from tick: {} with {}", v, p);
        let mut oracle = to_oracle_version(&v);
        oracle.tick(&to_oracle_party(&p));
        prop_assert_eq!(
            &one,
            &from_oracle_version(&oracle),
            "ticks(1) diverged from the oracle event: {} with {}",
            v,
            p
        );
    }

    /// Structural fact: fill is idempotent.
    ///
    /// Whenever the fused walk reports a change, a second walk over its output
    /// reports the tree unchanged — so at most the first tick of a run takes
    /// the fill branch, and `ticks` needs at most two walks.
    #[test]
    fn fill_is_idempotent(
        ov in generators::arb_oracle_version(),
        op in generators::arb_oracle_party_nonempty(),
    ) {
        let v = from_oracle_version(&ov);
        let p = from_oracle_party(&op);
        if let FillOutcome::Changed(bits) = fused_fill(&encode(&v), &p) {
            prop_assert!(
                matches!(fused_fill(&bits, &p), FillOutcome::Unchanged(_)),
                "fill moved a tree it had already filled: {} with {}", v, p
            );
        }
    }

    /// Structural fact: the grow branch is absorbing.
    ///
    /// Once a pair sits on the unchanged branch, ticking never flips the next
    /// tick back to the fill branch — so one `+k` splice stands in for ticks
    /// 2..n. (The crux differential covers this too; this pins the mechanism at
    /// every intermediate step of a short run.)
    #[test]
    fn grow_branch_is_absorbing(
        ov in generators::arb_oracle_version(),
        op in generators::arb_oracle_party_nonempty(),
    ) {
        let v = from_oracle_version(&ov);
        let p = from_oracle_party(&op);
        // Land on the fill-fixed state (one tick at most gets there).
        let mut cur = v.clone();
        cur.tick(&p);
        for _ in 0..4 {
            prop_assert!(
                matches!(fused_fill(&encode(&cur), &p), FillOutcome::Unchanged(_)),
                "a grow re-opened the fill branch: {} with {}", v, p
            );
            cur.tick(&p);
        }
    }
}

proptest! {
    /// Deep and wide together: every deep shape at swept depths 8..=128,
    /// carrying one [`generators::arb_base`]-drawn leaf at the tip or an
    /// interior node, ticks byte-identical to the iterated public tick.
    ///
    /// The conjunction no other family reaches: the depth-capped arbitrary
    /// trees carry wide values only at small depth, and the deterministic
    /// deep shapes carry only small distinct counters — so a guard needing
    /// deep suspension state (the fill walk's ancestor bit stacks, the
    /// pre-scan's memoized latents) *and* word-scale arithmetic at once
    /// would sit outside every sampled universe. Here the wide leaf rides a
    /// real spine, at the tip (inside the deepest suspended range) and at an
    /// interior node (crossing every open range on the way up), under
    /// ongoing generator mass.
    #[test]
    fn deep_and_wide_ticks_match_iterated(
        ev_shape in generators::arb_shape(),
        id_shape in generators::arb_shape(),
        depth in 8usize..=128,
        wide in generators::arb_base(),
        at_tip in any::<bool>(),
    ) {
        let v = generators::shape_version_wide(ev_shape, depth, &wide, at_tip);
        let p = generators::shape_party(id_shape, depth);
        check_ticks_equivalence(&v, &p, &[0, 1, 2, 3, 7]);
    }
}

/// The shape corpus at depth: the crux differential over the adversarial deep
/// shapes crossed with the shape parties, `n` to 1000.
///
/// The parties span single deep owned regions and bushy multi-region ids, and
/// the count runs large enough that the iterated side pays a thousand walks
/// while the fused side pays at most two.
#[test]
fn ticks_matches_iterated_ticks_shapes() {
    use generators::{bushy_expand_party, shape_party, shape_version, Shape};
    let shapes = [
        Shape::LeftSpine,
        Shape::RightSpine,
        Shape::Zigzag,
        Shape::Bushy,
    ];
    for ev_shape in shapes {
        for ev_scale in [1usize, 3, 17] {
            let v = shape_version(ev_shape, ev_scale);
            for id_shape in shapes {
                for id_scale in [1usize, 3, 17] {
                    let p = shape_party(id_shape, id_scale);
                    check_ticks_equivalence(&v, &p, &[0, 1, 2, 3, 7, 64, 1000]);
                }
            }
            // The expansion-heavy id: a bushy multi-region id beside an owned
            // terminal (the route weighs two feasible children at every
            // branch).
            let p = bushy_expand_party(ev_scale);
            check_ticks_equivalence(&v, &p, &[0, 1, 2, 3, 7, 64, 1000]);
        }
    }
}

/// The fill-changed branch, witnessed deterministically: a full owner over a
/// bushy tree collapses under fill, and `ticks` then routes the remaining `n −
/// 1` events through the second walk.
///
/// The shape corpus's parties never own the whole tree, so this pins the branch
/// the proptests reach only by generator luck.
#[test]
fn ticks_covers_fill_changed_branch() {
    let v = generators::shape_version(generators::Shape::Bushy, 5);
    let p = Party::seed(); // the full owner of everything
    assert!(
        matches!(fused_fill(&encode(&v), &p), FillOutcome::Changed(_)),
        "witness must take the fill branch"
    );
    check_ticks_equivalence(&v, &p, &[0, 1, 2, 3, 7, 64, 1000]);
}

/// Closed-form witness from the identity: `ticks(n)` on the empty version under
/// the seed party renders as `n` — the whole-line counter, readable without any
/// reference implementation.
#[test]
fn ticks_from_empty_is_the_counter() {
    let v = Version::new();
    let seed = Party::seed();
    let n = Base::from(123_456_789_012_345u64);
    let ticked = ticks_version(&v, &seed, &n);
    assert_eq!(ticked.to_string(), "123456789012345");
    // And the seam back to ground truth at small n.
    check_ticks_equivalence(&v, &seed, &[0, 1, 2, 3, 7, 64, 1000]);
}

/// Wide-`n` self-consistency, beyond any iterative reference: `ticks` is a
/// monoid action — `ticks(a + b) = ticks(b) ∘ ticks(a)` — at `n` around
/// `2^100`, over every shape pair at depths swept 5 to 128.
///
/// A small-tail cross-check `ticks(big + 1) = tick ∘ ticks(big)` seams the wide
/// arm to the ground-truth single tick. The swept depths carry the wide-`n`
/// arithmetic across real suspension depth — the deterministic deep-and-wide
/// leg beside [`deep_and_wide_ticks_match_iterated`]'s sampled family.
#[test]
fn ticks_composes_at_wide_n() {
    use generators::{shape_party, shape_version, Shape};
    let big = Base::from(1u8) << 100u32;
    let shapes = [
        Shape::LeftSpine,
        Shape::RightSpine,
        Shape::Zigzag,
        Shape::Bushy,
    ];
    for scale in [5usize, 8, 32, 128] {
        for ev_shape in shapes {
            let v = shape_version(ev_shape, scale);
            for id_shape in shapes {
                let p = shape_party(id_shape, scale);
                // ticks(2^101) == ticks(2^100) twice.
                let both = ticks_version(&v, &p, &(big.clone() + &big));
                let half = ticks_version(&v, &p, &big);
                let again = ticks_version(&half, &p, &big);
                assert_eq!(both, again, "wide composition diverged");
                // ticks(2^100 + 1) == tick() after ticks(2^100).
                let plus_one = ticks_version(&v, &p, &(big.clone() + 1u64));
                let mut stepped = half.clone();
                stepped.tick(&p);
                assert_eq!(plus_one, stepped, "wide-plus-one seam diverged");
            }
        }
    }
}

/// Directed pre-scan shapes concentrating right-full raises at the earliest
/// reachable moments of a fresh scan.
///
/// The negative space around the invariant that every range resolving ahead
/// of a raise has already emitted (retiring the scan's entry net,
/// `PreScan::max_range`'s entry assertion).
mod prescan_raise_shapes {
    use super::*;
    use crate::codec::{self, BitsMut};

    /// Construction-language pushers (the meter shape builders' vocabulary):
    /// internal node = `1 · gamma(0)`, leaf = `0 · gamma(n)`.
    fn nd(ev: &mut BitsMut) {
        ev.push(true);
        codec::encode_int(ev, &Base::ZERO);
    }
    fn lf(ev: &mut BitsMut, n: u64) {
        ev.push(false);
        codec::encode_int(ev, &Base::from(n));
    }
    /// A deep staircase region (the meter `hole_region`'s lead-2 shape):
    /// wrapper node, staircase `m..0`, wrapper floor leaf — deep enough to
    /// route every consuming scan and copy through its block summary.
    fn hole(ev: &mut BitsMut, m: u64) {
        nd(ev); // wrapper
        nd(ev); // staircase root
        lf(ev, m);
        for v in (1..m).rev() {
            nd(ev);
            lf(ev, v);
        }
        lf(ev, 0);
        lf(ev, 0); // wrapper floor
    }
    /// Seal a built construction-language stream (the meter `Packed` form).
    fn pk(bits: BitsMut) -> Packed {
        let live = bits.len();
        let mut sealed = bits;
        codec::seal_padding(&mut sealed);
        Packed {
            bytes: sealed.into_vec(),
            bits: live,
        }
    }

    const T: bool = true;
    const F: bool = false;

    /// Wrap an entry range and its id in the covering left-full site that
    /// launches the fresh pre-scan: a root site over a fully-owned collapse
    /// leaf, the entry range as its sibling.
    fn covered(er: &BitsMut, ir: &[bool]) -> (Version, Party) {
        let mut ev = BitsMut::new();
        nd(&mut ev); // the covering site's node
        lf(&mut ev, 2); // its fully-owned collapse leaf
        ev.extend_from_bitslice(er);
        let mut id = BitsMut::new();
        for b in [T, T, F, F] {
            id.push(b); // the covering site: internal, full left child
        }
        for &b in ir {
            id.push(b);
        }
        (version_of(&pk(ev)), party_of(&pk(id)))
    }

    /// The chain-raise pair: a `k`-deep chain of suspended left-full sites
    /// under a node whose right child is fully owned — the raise.
    ///
    /// The scan's first (and only) emission is the chain's innermost sibling
    /// leaf. `k = 0` is the minimum-latency raise: the entry range's root
    /// raises immediately after its left leaf's single emission.
    fn chain_raise(k: usize, deep: bool) -> (Version, Party) {
        let mut er = BitsMut::new();
        nd(&mut er); // the raising node
        for _ in 0..k {
            nd(&mut er); // a site along the chain ...
            lf(&mut er, 1); // ... over its skipped, unemitted collapse leaf
        }
        lf(&mut er, 0); // the innermost sibling leaf: the scan's first emission
        if deep {
            hole(&mut er, 3); // the raise range, block-routed
        } else {
            lf(&mut er, 3); // the raise range, a single leaf
        }
        let mut ir = vec![T, T]; // the raising node
        for _ in 0..k {
            ir.extend([T, T, F, F]); // each site: internal, full left child
        }
        ir.extend([T, F, F, F]); // the innermost sibling leaf's id
        ir.extend([F, F]); // the raising node's right child: full
        covered(&er, &ir)
    }

    /// The chain-raise grid runs the full differential in a debug build,
    /// where `PreScan::max_range`'s entry assertion is live.
    ///
    /// The raise fires at the earliest reachable moment of the fresh scan
    /// (one emission, arbitrarily many suspended-and-resolved sites), and
    /// the verdicts match the recursive oracle on every pair.
    #[test]
    fn chain_raise_grid_matches_oracle() {
        for k in 0..=3 {
            for deep in [false, true] {
                let (v, p) = chain_raise(k, deep);
                assert_tick(&v, &p);
            }
        }
    }

    /// A raise fires inside a suspended site's sibling after the scan's one
    /// and only emission, and the verdicts match the recursive oracle.
    ///
    /// The site's collapse skip emits nothing and the sibling's left leaf is
    /// the scan's sole emission, so the raise scans under the deepest
    /// pre-arming pressure a canonical pair can construct, the site still
    /// suspended. The skip-then-forced-copy dual rides along: a no-sibling
    /// site's deep collapse skip (its entry-net fold live) resolves through
    /// the forced copy's emission before its parent's raise.
    #[test]
    fn suspended_and_skipping_site_raises_match_oracle() {
        // The raise under suspension: site over (collapse leaf, raising node).
        let mut er = BitsMut::new();
        nd(&mut er); // the site
        lf(&mut er, 1); // its collapse (skipped, unemitted)
        nd(&mut er); // the raising node
        lf(&mut er, 0); // its left leaf: the scan's first emission
        lf(&mut er, 2); // the raise range
        let (v, p) = covered(
            &er,
            &[
                T, T, F, F, // the site: internal, full left child
                T, T, // the raising node
                T, F, F, F, // its left leaf's id
                F, F, // its right child: full
            ],
        );
        assert_tick(&v, &p);
        // The skip-then-forced-copy dual: raising node over (no-sibling site
        // with a deep collapse, raise range).
        let mut er = BitsMut::new();
        nd(&mut er); // the raising node
        nd(&mut er); // the no-sibling site
        hole(&mut er, 4); // its deep collapse (skipped net-only, unemitted)
        lf(&mut er, 0); // its absent-side sibling: the forced copy's emission
        lf(&mut er, 2); // the raise range
        let (v, p) = covered(
            &er,
            &[
                T, T, // the raising node
                T, F, F, F, // the site: internal, full left, absent right
                F, F, // the raising node's right child: full
            ],
        );
        assert_tick(&v, &p);
    }

    /// The descend-entry full arm is excluded at the type boundary: a full
    /// child's sibling can never itself be full.
    ///
    /// An id handing a full sibling to a full child is `(1, 1)`, which
    /// normal form collapses and decode rejects — every pre-scan entry is a
    /// full child's sibling or a child its caller peeked as not-full.
    #[test]
    fn full_sibling_of_full_child_is_undecodable() {
        let mut id = BitsMut::new();
        for b in [T, T, F, F, F, F] {
            id.push(b);
        }
        codec::seal_padding(&mut id);
        assert!(
            Party::decode(&id.into_vec()[..]).is_err(),
            "id (1, 1) must be rejected as non-normal"
        );
    }
}

proptest! {
    /// A wide climb-and-return region copied under a left-full raise spine
    /// ticks identically to the recursive oracle at every knob.
    ///
    /// The spine is `k` nested left-full sites: each level owns its raise leaf
    /// and copies its right sibling, and the innermost sibling is the
    /// scale-disparate region `(c, (0, m · 2^b, 0), r)` — a climb past the
    /// domination read's decision bound, a return to the region's minimum, and
    /// an exit `r` above it.
    ///
    /// Crossing those two shapes is what this family is for. A raise spine
    /// alone re-anchors the sibling relation across its closes but emits
    /// nothing scale-disparate; a wide region alone arrives at a block-minimum
    /// emission with no relation riding the web. Together the relation is live
    /// *across* a wide-negative block-minimum emission, so the residue the
    /// undercut moves out must also move every relation the walk is still
    /// carrying — a coupling neither shape exercises on its own, and one whose
    /// polarity the oracle differential pins here byte for byte.
    #[test]
    fn wide_region_under_a_left_full_spine_ticks_identically(
        raises in proptest::collection::vec(0u64..=6, 1..=4),
        m in 1u64..=7,
        b in 128usize..=200,
        c in 1u64..=4,
        r in 1u64..=4,
    ) {
        let k = raises.len();
        let wide = UBig::from(m) << b;
        let mut text = String::new();
        for (level, raise) in raises.iter().enumerate() {
            // The innermost site's raise meets the region's own base at zero;
            // the outer ones meet the level below, whose minimum is already
            // zero, so they are free.
            let leaf = if level + 1 == k { 0 } else { *raise };
            text.push_str(&format!("(0, {leaf}, "));
        }
        text.push_str(&format!("({c}, (0, {wide}, 0), {r})"));
        text.push_str(&")".repeat(k));
        let v: Version = text.parse().expect("the spine literal is normal form");
        let id = "(1, ".repeat(k) + "0" + &")".repeat(k);
        let p: Party = id.parse().expect("the spine id literal parses");
        assert_tick(&v, &p);
    }
}
