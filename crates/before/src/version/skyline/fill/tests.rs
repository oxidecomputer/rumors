//! Differential pins for the skyline fill and the tick splice.
//!
//! `fill` is held to the recursive oracle (`oracle::Version::fill`
//! through the bridge) over every pool, with canonical uniqueness
//! making the differential total: the filled stream must equal the
//! oracle's encoded result byte for byte. The `tick` asserts run the
//! module function against the public `Version::tick`, which routes to
//! the same kernel — an entry-agreement and determinism pin, not an
//! independent value; the splice's value correctness rests on the fill
//! oracle here and the grow suite's oracle and brute-force pins
//! (`grow/tests.rs`), so each branch is pinned, not just their
//! composition. The deep-spine case derives its expected values in
//! closed form (its test doc states each derivation).

use proptest::prelude::*;
use rayon::prelude::*;

use crate::meter::{
    alt_spine, bigroot, cancelling_chain, cliff_comb, cliff_fan, dense, harmonic, hugeleaf,
    id_spine, memo_chain, memo_chain_id, memo_comb, memo_comb_id, nested_full_id,
    nested_left_full_id, scattered_id, staircase, wide_tail, wide_tooth_comb, Packed,
};
use crate::testing::bridge::{
    from_oracle_party, from_oracle_version, to_oracle_party, to_oracle_version,
};
use crate::testing::exhaustive::{
    all_normal_events, all_normal_ids, EV_SMALL_DEPTH, ID_SMALL_DEPTH,
};
use crate::testing::{generators, optrace};
use crate::version::skyline::{encode, validate};
use crate::{Clock, Party, Version};

use super::{fill, tick};

/// Lift a meter-generated packed event shape into a [`Version`].
fn version_of(p: &Packed) -> Version {
    p.version()
}

/// Decode a meter-generated packed id shape as a [`Party`].
fn party_of(p: &Packed) -> Party {
    Party::decode(&p.bytes[..]).expect("meter shapes are strict normal form")
}

/// Assert the fill kernel against the recursive oracle on one pair, and
/// that its output validates as canonical.
fn assert_fill(v: &Version, p: &Party) {
    let enc = encode(v);
    let out = fill(&enc, p);
    validate(&out.bytes, out.bits).expect("a filled stream is canonical");
    let oracle = to_oracle_version(v).fill_for_test(&to_oracle_party(p));
    assert_eq!(
        out,
        encode(&from_oracle_version(&oracle)),
        "fill must match the recursive oracle: {v} with {p}"
    );
}

/// Assert the tick splice through both entry points on one pair, and
/// that the output validates as canonical.
///
/// The module function runs against the public `Version::tick`, which
/// routes to the same kernel: this pins entry agreement and
/// determinism, not an independent value.
fn assert_tick(v: &Version, p: &Party) {
    let out = tick(&encode(v), p);
    let mut expected = v.clone();
    expected.tick(p);
    assert_eq!(
        out,
        encode(&expected),
        "the tick splice and the public tick must agree: {v} with {p}"
    );
    validate(&out.bytes, out.bits).expect("a ticked stream is canonical");
}

/// The adversarial event pool: every §2 family at two scales, plus the
/// empty version.
fn event_pool() -> Vec<Version> {
    vec![
        Version::new(),
        version_of(&dense(1)),
        version_of(&dense(2)),
        version_of(&dense(64)),
        version_of(&bigroot(7, 3)),
        version_of(&bigroot(64, 16)),
        version_of(&hugeleaf(1)),
        version_of(&hugeleaf(64)),
        version_of(&cliff_comb(3, 2)),
        version_of(&cliff_comb(16, 16)),
        version_of(&wide_tooth_comb(16, 8, 8)),
        version_of(&cliff_fan(16, 8)),
        version_of(&cancelling_chain(16, 8)),
        version_of(&alt_spine(3)),
        version_of(&alt_spine(64)),
        version_of(&harmonic(16)),
        version_of(&wide_tail(7, 3)),
        version_of(&wide_tail(64, 16)),
        version_of(&staircase(1)),
        version_of(&staircase(16)),
        version_of(&memo_chain(1, true)),
        version_of(&memo_chain(8, true)),
        version_of(&memo_chain(8, false)),
        version_of(&memo_comb(1)),
        version_of(&memo_comb(4)),
    ]
}

/// The adversarial party pool: the seed, deep and diverted unary
/// spines, scattered ownership, and every exhaustive small-scope id —
/// including the empty id, which `fill` (unlike `grow`) accepts as the
/// identity arm.
fn party_pool() -> Vec<Party> {
    let mut pool = vec![
        Party::seed(),
        party_of(&id_spine(1, false)),
        party_of(&id_spine(3, false)),
        party_of(&id_spine(3, true)),
        party_of(&id_spine(64, false)),
        party_of(&id_spine(64, true)),
        party_of(&scattered_id(1)),
        party_of(&scattered_id(16)),
        party_of(&nested_full_id(1)),
        party_of(&nested_full_id(8)),
        party_of(&nested_left_full_id(1)),
        party_of(&nested_left_full_id(8)),
        party_of(&memo_chain_id(1)),
        party_of(&memo_chain_id(8)),
        party_of(&memo_comb_id(1)),
        party_of(&memo_comb_id(4)),
    ];
    pool.extend(all_normal_ids(2).iter().map(from_oracle_party));
    pool
}

/// Every event-family × party-family pair fills byte-identically to
/// the recursive oracle and ticks in agreement through both entry
/// points (owning parties; the empty id has no tick).
#[test]
fn family_pairs_fill_and_tick_identically() {
    let events = event_pool();
    let parties = party_pool();
    events.par_iter().for_each(|v| {
        for p in &parties {
            assert_fill(v, p);
            if !p.as_bits().is_empty() {
                assert_tick(v, p);
            }
        }
    });
}

/// Exhaustive small scope: every normal-form event tree × every
/// normal-form id fills byte-identically to the recursive oracle, and
/// every owning id ticks in agreement through both entry points.
#[test]
fn exhaustive_small_scope_fills_and_ticks_identically() {
    let events: Vec<Version> = all_normal_events(EV_SMALL_DEPTH)
        .iter()
        .map(from_oracle_version)
        .collect();
    let parties: Vec<Party> = all_normal_ids(ID_SMALL_DEPTH)
        .iter()
        .map(from_oracle_party)
        .collect();
    events.par_iter().for_each(|v| {
        for p in &parties {
            assert_fill(v, p);
            if !p.as_bits().is_empty() {
                assert_tick(v, p);
            }
        }
    });
}

/// The worked examples, pinned end to end: the full-id collapse, both
/// shortcut raises (taken and declined), and a nested arm.
#[test]
fn worked_examples_fill_exactly() {
    let cases: [(&str, &str, &str); 6] = [
        // The full id collapses the whole tree to its max (heights 2
        // and 3; the collapse is the higher plateau).
        ("1", "(2, 0, 1)", "3"),
        // Left-full: the collapsed left rises to the right's minimum
        // (min fill(0, er) = 3 > max(el) = 2), and the pair merges.
        ("(1, 0)", "(2, 0, 1)", "3"),
        // Right-full, mirrored.
        ("(0, 1)", "(2, 1, 0)", "3"),
        // Right-full where the raise is declined (max(er) = 3 already
        // clears min(el') = 2): nothing changes.
        ("(0, 1)", "(2, 0, 1)", "(2, 0, 1)"),
        // Left-full over an internal left child: the whole el subtree
        // collapses into the raised leaf.
        ("(1, 0)", "(2, (0, 1, 0), 3)", "5"),
        // A node id whose left child is itself a shortcut site: the
        // inner raise lifts the root's minimum, and norm re-lifts it.
        ("((1, 0), 0)", "(1, (0, 0, 1), 2)", "(2, 0, 1)"),
    ];
    for (party, before, after) in cases {
        let p: Party = party.parse().expect("test party literals parse");
        let v: Version = before.parse().expect("test version literals parse");
        let expected: Version = after.parse().expect("test version literals parse");
        assert_eq!(
            fill(&encode(&v), &p),
            encode(&expected),
            "fill of {before} with {party} must yield {after}"
        );
        assert_fill(&v, &p);
    }
}

/// The splice takes both branches: a fill that simplifies is the tick,
/// and a fill that changes nothing falls through to grow.
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

/// The version that is `1` on the leftmost `2^-depth` interval and `0`
/// everywhere else: `depth` nested nodes, all bases zero, the single
/// 1-leaf at the bottom left.
///
/// Built as a text literal — the parser is iterative — so the expected
/// tree shares no walk with fill or grow.
fn left_spike(depth: usize) -> Version {
    let mut text = "(0, ".repeat(depth - 1);
    text.push_str("(0, 1, 0)");
    text.push_str(&", 0)".repeat(depth - 1));
    text.parse().expect("the spike literal is normal form")
}

/// Deep spines in every regime stay correct at depths that would
/// overflow a native-frame walk: the collapse scan, the pass-through
/// copy, and the two-cursor descent.
///
/// The recursive `oracle` enums walk on native frames (they are the
/// small-scope reference, not a deep-input one), so the value witnesses
/// here are closed forms, derived per case: the full id collapses the
/// whole spine to one leaf at its maximum height — the alternating
/// spine's only nonzero leaf is the `1` at the bottom pair — and that
/// collapse is the tick; the deep unary id over the empty version fills
/// to the identity (fill of a leaf under a node id is the leaf) and
/// ticks to the left spike, the expansion chain to its owned tip; and
/// over the deep spine the same id turns left into the spine's depth-2
/// zero leaf (the spine's structure continues right there), so fill is
/// again the identity and the grown tree raises exactly the owned
/// region from 0 to 1 — the pointwise max with the spike, realized
/// through the independently-pinned join kernel and byte-exact by
/// canonical uniqueness. The nested-full-sibling id over its matched
/// spine fills to the identity, derived: every level's right-full
/// raise is `max(max(er), min(fill(il, el)))`, where `er` is a single
/// leaf (its own maximum) and the left range's minimum stays at the
/// spine's floor of zero, so no raise ever moves a value — and the id
/// terminus pairs with a leaf (the untouched-leaf arm). The case
/// drives the deferred right-full decision and its per-level raise
/// bookkeeping at full depth with a value witness the small scope
/// pins against the oracle. The mirror id over the wide-tail spine
/// collapses to the single wide leaf, derived bottom-up: the deepest
/// left-full raise is `max(max(el), min(fill(ir, er)))` with `el` a
/// lone zero leaf and `er` the wide tail itself (a leaf's minimum is
/// its value), so the raise lifts the zero leaf to the tail's value,
/// the equal sibling pair collapses, and each enclosing level sees
/// the same wide leaf as its right minimum — the collapse telescopes
/// to the root. The case drives the memoized pre-scan at full depth
/// with wide minima in every memo entry, and the collapse is the
/// tick (fill changed the tree). The staircase under the unary id
/// spine fills to the identity — its levels pair internal × internal
/// down to the terminus, whose left-full raise `max(max(a), min(b))`
/// with `a` one step above `b` returns `a` itself — and its tick
/// increments the id's owned tip, the bottom-left leaf; every
/// consumed leaf undercuts every open range on the way, the
/// full-penetration minimum-update schedule. Canonicality, fill
/// idempotence, and tick entry agreement ride along on every case.
#[test]
fn deep_spines_fill_and_tick_identically() {
    let assert_deep = |v: &Version, p: &Party| {
        let out = fill(&encode(v), p);
        validate(&out.bytes, out.bits).expect("a filled stream is canonical");
        let again = fill(&out, p);
        assert_eq!(again, out, "deep fill must be idempotent");
        assert_tick(v, p);
        out
    };
    let deep_ev = version_of(&alt_spine(4096));
    let deep_id = party_of(&id_spine(4096, false));
    let spike = left_spike(4096);
    let one: Version = "1".parse().expect("test literals parse");

    // The full id: the collapse to the maximum leaf is the fill, and
    // fill changed the tree, so it is also the tick.
    let filled = assert_deep(&deep_ev, &Party::seed());
    assert_eq!(
        filled,
        encode(&one),
        "the full id collapses the spine to its maximum leaf"
    );
    assert_eq!(
        tick(&encode(&deep_ev), &Party::seed()),
        encode(&one),
        "tick takes the fill branch: the collapse"
    );

    // The deep unary id over the empty version: identity fill, so tick
    // falls through to grow's expansion chain.
    let filled = assert_deep(&Version::new(), &deep_id);
    assert_eq!(
        filled,
        encode(&Version::new()),
        "fill of a leaf under a node id is the identity"
    );
    assert_eq!(
        tick(&encode(&Version::new()), &deep_id),
        encode(&spike),
        "tick grows the expansion chain to the owned tip"
    );

    // Both deep: no fully-owned region meets a subdividable subtree, so
    // fill is the identity; the grown tree is the pointwise max with
    // the spike.
    let filled = assert_deep(&deep_ev, &deep_id);
    assert_eq!(
        filled,
        encode(&deep_ev),
        "no full-id region: fill is the identity"
    );
    assert_eq!(
        tick(&encode(&deep_ev), &deep_id),
        encode(&(&deep_ev | &spike)),
        "the grown stream is the pointwise max with the spike"
    );

    // The nested-full-sibling id over its matched spine: a right-full
    // shortcut site at every one of the 4096 levels — the deferred
    // right-full decision and its raise bookkeeping at full depth.
    // Fill is the identity (the doc comment's derivation: each raise
    // maxes a lone leaf against a zero minimum), and the small scope
    // pins the same cross against the oracle at every depth it
    // enumerates.
    let mut text = "(0, ".repeat(4095);
    text.push_str("(0, 0, 1)");
    text.push_str(&", 0)".repeat(4095));
    let matched: Version = text.parse().expect("the matched spine literal parses");
    let nested = party_of(&nested_full_id(4096));
    let filled = assert_deep(&matched, &nested);
    assert_eq!(
        filled,
        encode(&matched),
        "every nested raise maxes a lone leaf against a zero minimum: identity"
    );

    // The mirror id over the wide-tail spine: a left-full shortcut
    // site at every one of the 4096 levels — the memoized pre-scan at
    // full depth, every memo entry a wide minimum. The deepest raise
    // lifts its zero leaf to the tail's value (a leaf's minimum is
    // its value), the equal pair collapses, and the collapse
    // telescopes to the root: fill is the single wide leaf, and the
    // collapse is the tick.
    let mut text = "(0, 0, ".repeat(4095);
    text.push_str(&format!("(0, 0, {})", u64::MAX));
    text.push_str(&")".repeat(4095));
    let tail: Version = text.parse().expect("the wide-tail literal parses");
    let wide_leaf: Version = u64::MAX
        .to_string()
        .parse()
        .expect("the leaf literal parses");
    let mirror = party_of(&nested_left_full_id(4096));
    let filled = assert_deep(&tail, &mirror);
    assert_eq!(
        filled,
        encode(&wide_leaf),
        "the deepest raise meets the tail and the collapse telescopes to the root"
    );
    assert_eq!(
        tick(&encode(&tail), &mirror),
        encode(&wide_leaf),
        "tick takes the fill branch: the telescoped collapse"
    );

    // The descending staircase under the unary id spine: every
    // consumed leaf undercuts every open range — the full-penetration
    // minimum-update schedule at 4096 levels, all values word-scale.
    // Fill is the identity (internal × internal at every level above
    // the terminus, whose raise `max(a, min(b))` returns `a` itself),
    // so tick grows: the id's owned tip is the bottom-left leaf, a
    // zero-expansion increment and the only owned site.
    let mut text = "(0, ".to_string();
    text.push_str(&"(1, ".repeat(4094));
    text.push_str("(1, 1, 0)");
    text.push_str(&", 0)".repeat(4095));
    let stairs: Version = text.parse().expect("the staircase literal parses");
    let spine_id = party_of(&id_spine(4096, false));
    let filled = assert_deep(&stairs, &spine_id);
    assert_eq!(
        filled,
        encode(&stairs),
        "no full region meets a subdividable subtree: identity"
    );
    let mut text = "(0, ".to_string();
    text.push_str(&"(1, ".repeat(4094));
    text.push_str("(1, 2, 0)");
    text.push_str(&", 0)".repeat(4095));
    let grown: Version = text.parse().expect("the grown staircase literal parses");
    assert_eq!(
        tick(&encode(&stairs), &spine_id),
        encode(&grown),
        "tick increments the owned bottom-left leaf"
    );

    // The memo chain at 4096 sites: every interior left-full site
    // collapses its `(0, 0, j)` node to the leaf `j` (the raise meets
    // the site's own single-leaf range), and the covering site's raise
    // stays at the tree minimum 0 — so the filled tree is the spine
    // with each site replaced by its leaf, in closed form. The walk
    // resolves all 4096 memoized minima from one fresh scan.
    let k = 4_096u64;
    let mut text = "(0, 0, ".to_string();
    for j in 1..=k {
        text.push_str(&format!("(0, {j}, "));
    }
    text.push('0');
    text.push_str(&")".repeat(k as usize + 1));
    let expected: Version = text.parse().expect("the chain literal parses");
    let chain = memo_chain(k as usize, true).version();
    let chain_id = party_of(&memo_chain_id(k as usize));
    assert_eq!(
        fill(&encode(&chain), &chain_id),
        encode(&expected),
        "every site's raise meets its single-leaf range"
    );
    assert_eq!(
        tick(&encode(&chain), &chain_id),
        encode(&expected),
        "tick takes the fill branch: the sites collapse"
    );
}

proptest! {
    /// Arbitrary parties over arbitrary normal-form versions fill
    /// byte-identically to the recursive oracle (and tick through both
    /// entry points, when the party owns anything), magnitudes past
    /// `u64::MAX` included.
    #[test]
    fn arbitrary_pairs_fill_and_tick_identically(
        op in generators::arb_oracle_party_nonempty(),
        ov in generators::arb_oracle_version(),
    ) {
        let p = from_oracle_party(&op);
        let v = from_oracle_version(&ov);
        assert_fill(&v, &p);
        if !p.as_bits().is_empty() {
            assert_tick(&v, &p);
        }
    }

    /// Organic histories fill byte-identically to the recursive oracle
    /// and tick in agreement through both entry points.
    ///
    /// Every clock produced by one fork/tick/send/sync/join history is
    /// exercised from its own party — and from every *other* clock's
    /// party, the concurrent-editor shape.
    #[test]
    fn organic_histories_fill_and_tick_identically(ops in optrace::world_strategy_up_to(40)) {
        let mut clocks = vec![Clock::seed()];
        for op in &ops {
            optrace::step_impl(&mut clocks, op);
        }
        for a in &clocks {
            for b in &clocks {
                assert_fill(a.version(), b.party());
                if !b.party().as_bits().is_empty() {
                    assert_tick(a.version(), b.party());
                }
            }
        }
    }

    /// `tick`'s output stays within a constant factor of its inputs'
    /// coded sizes: `bits(tick(e, i)) ≤ 2·bits(e) + 4·bits(i) + 32`
    /// over arbitrary pairs.
    ///
    /// The bound the board's input denomination of the tick rows rests
    /// on, computed the second way (the operation itself, against the
    /// arithmetic of its parts). Fill's raises telescope the codes
    /// their collapsed ranges already spent, and grow adds one
    /// increment or one expansion chain, a constant per id bit at the
    /// site — but either can re-code up to two deltas against a wide
    /// neighbor (the raise's landing, grow's zero leaf), each
    /// duplicating one input code's width once. Hence the factor of
    /// two, not an additive slack: 255 output bits from a 175-bit
    /// event under a 6-bit id is honest arithmetic (the zero leaf
    /// lands next to a wide value), while a superlinear output would
    /// be minting content no operand paid for.
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
            let bound = 2 * ev.bits + 4 * p.as_bits().len() + 32;
            prop_assert!(
                out.bits <= bound,
                "tick output {} bits exceeds input envelope {} (event {}, id {})",
                out.bits, bound, ev.bits, p.as_bits().len(),
            );
        }
    }

    /// The tick ORBIT's coded size is a bounded transient plus
    /// logarithmic growth.
    ///
    /// After the first tick (whose one-step factor the pin above
    /// prices), `bits(tick^k) ≤ bits(tick^1) + 4·bits(id) +
    /// 4·⌈log2(k + 1)⌉ + 8` for every k along the orbit.
    ///
    /// The per-step multiplicative bound cannot compound: a width
    /// duplication needs an unexpanded id-demanded site adjacent to a
    /// wide transition, the orbit mints expansions at most once per id
    /// site (the `4·bits(id)` transient), re-fired raises re-code the
    /// same position rather than stacking, and the steady state is
    /// increments whose two re-coded delta codes grow with the count's
    /// own gamma width — the `log k` term [measured: the envelope holds
    /// with zero slack at the log term on the committed families over
    /// 512 ticks, and the fixed-pair orbit freezes at +24 bits over
    /// 4096 ticks].
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
            let b1 = e.bits;
            for k in 2u32..=48 {
                e = tick(&e, &p);
                let logk = u64::from(32 - (k + 1).leading_zeros());
                let bound = b1 as u64
                    + 4 * p.as_bits().len() as u64
                    + 4 * logk
                    + 8;
                prop_assert!(
                    e.bits as u64 <= bound,
                    "orbit size {} bits at tick {k} exceeds the transient-plus-log \
                     envelope {bound} (first-tick size {b1}, id {} bits)",
                    e.bits, p.as_bits().len(),
                );
            }
        }
    }
}

/// The deterministic deep orbits stay inside their measured bands.
///
/// The fixed wide pair's size freezes after its one-step transient
/// (growing only with the incremented count's gamma width), and the
/// alternating disjoint pair — whose collapse/expand cycle re-fires a
/// width duplication every period — stays inside the same band
/// forever: re-fired raises re-code the same bounded value at the
/// same position, replacing, never stacking.
#[test]
fn tick_deep_orbits_stay_banded() {
    let ev = crate::meter::bigroot(64, 4).version();
    let ida = party_of(&crate::meter::id_spine(4, false));
    let idb = party_of(&crate::meter::id_spine(4, true));

    let mut e = encode(&ev);
    e = tick(&e, &ida);
    let b1 = e.bits;
    for k in 2u32..=4096 {
        e = tick(&e, &ida);
        let logk = usize::try_from(32 - (k + 1).leading_zeros()).expect("small");
        assert!(
            e.bits <= b1 + 4 * logk + 8,
            "fixed-id orbit: {} bits at tick {k} (first-tick size {b1})",
            e.bits,
        );
    }

    let mut e = encode(&ev);
    e = tick(&e, &ida);
    e = tick(&e, &idb);
    let b2 = e.bits;
    for k in 3u32..=2048 {
        e = tick(&e, if k % 2 == 1 { &ida } else { &idb });
        let logk = usize::try_from(32 - (k + 1).leading_zeros()).expect("small");
        assert!(
            e.bits <= b2 + 4 * logk + 8,
            "alternating orbit: {} bits at tick {k} (two-tick size {b2})",
            e.bits,
        );
    }
}
