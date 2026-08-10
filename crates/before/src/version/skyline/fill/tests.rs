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
    assert_tick(&v, &p);
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
