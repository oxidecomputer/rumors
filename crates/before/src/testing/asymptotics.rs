//! Liveness pins for the documented asymptotics: the growth behaviors
//! the public rustdoc's `# Complexity` sections claim, held alive
//! against the deterministic meters.
//!
//! A documented non-linear cost is a claim about a mechanism — the
//! balanced fold's log factor, the render merge's superlinear growth,
//! the settle's multiplication-bound worst case — and a cure or rewiring
//! that removes the mechanism must reach the documentation. Each pin
//! here reads a deterministic counter (or an exact value identity) on a
//! committed input family and asserts the documented behavior still
//! exists, so the change that removes it flips a pin red and the rustdoc
//! moves in the same commit. Floors sit midway between the linear
//! reference and the measured reading; both endpoints are exact
//! counters, so a crossing is a class change, never noise.
//!
//! Three pin families:
//!
//! - **Fold doors** (`scan-meter`): one pin per public door whose
//!   rustdoc claims the balanced reduction's `O(D log k)`, each measured
//!   at its own door — the doors share the balanced core, but a door's
//!   wiring (short-circuit arms, per-component walks, the hull's
//!   two-direction carry) can drop the factor without touching the core.
//! - **The render merge** (`limb-meter`): `Display`'s superlinear
//!   summary-merge growth on the wide left-full shape.
//! - **The answer-embedded product** (`meter`): the value structure
//!   behind the `Ω(M(·))` floor the rank, pair (distance/lag), and key
//!   (`Ranked`) rustdoc states — through each family of doors
//!   separately, since each enters the settle by its own path.

use crate::meter::registry::Shape;

/// Big-integer limb work of one full render of the wide left-full shape
/// (the board's mirror-wide event side) at spine scale `s`.
#[cfg(feature = "limb-meter")]
fn render_limb_ops(s: usize) -> u64 {
    let version = Shape::WideTail.packed2(s, s).version();
    crate::meter::reset_limb_ops();
    std::hint::black_box(version.to_string());
    crate::meter::limb_ops()
}

/// The render merge's superlinearity is alive.
///
/// `Display` limb work on the wide left-full shape grows super-linearly
/// across a doubling, which is exactly what the rustdoc's "summary-merge
/// cost that grows faster than the operand" sentence describes. When the
/// render-merge cure lands this pin reads red, and the rustdoc and this
/// floor must move in one change.
///
/// Deterministic counter, dev profile; linear rendering would read ~2.0
/// across the doubling, and the current merge reads x2.93 (8 558 ->
/// 25 114 ops, measured at this shape and scale). The floor
/// sits midway in that gap, so only a class change (never noise — the
/// counter is exact) crosses it.
#[cfg(feature = "limb-meter")]
#[test]
fn render_merge_superlinearity_is_alive() {
    /// Halfway between linear growth (~2.0x) and the measured x2.93.
    const MIN_GROWTH: f64 = 2.45;
    let (lo, hi) = (render_limb_ops(500), render_limb_ops(1000));
    let growth = hi as f64 / lo.max(1) as f64;
    assert!(
        growth >= MIN_GROWTH,
        "the render merge's limb work grew only x{growth:.2} across a doubling \
         ({lo} -> {hi} ops; superlinear read >= x{MIN_GROWTH}, linear ~x2.0): \
         the documented superlinearity is gone, so update the Display \
         `# Complexity` sections and this pin together"
    );
}

// ─── the fold doors' log-factor liveness ───────────────────────────────
//
// One witness per public fold door, each over a committed registry
// population whose balanced merges swell to near the sum of their
// inputs (coalescing intermediates are the enemy of visibility), so
// the reduction's log factor separates measurably from a linear fold.
// Every floor is measured at its own door — the doors share the
// balanced core, but a door's wiring (short-circuit arms,
// per-component walks, the hull's two-direction carry) can drop the
// factor without touching the core, so each pin binds its door.

/// Operand block count of the fold-door populations.
///
/// The board's stagger family at its band scale, held fixed while the
/// arity quadruples so the population's bytes grow (near-)linearly and
/// the scan growth above that is the log factor's own signal.
#[cfg(feature = "scan-meter")]
const FOLD_DOOR_TEETH: usize = 64;

/// The staggered version population `SG(n, 64)` in bit-reversed feed
/// order ([`Shape::StaggerPopulation`](crate::meter::registry::Shape)).
///
/// Every balanced merge joins maximally interleaving region sets, so
/// intermediates stay proportional to the leaves they carry.
#[cfg(feature = "scan-meter")]
fn stagger_versions(n: usize) -> Vec<crate::Version> {
    let (versions, _) =
        crate::meter::registry::Shape::StaggerPopulation.population(n, FOLD_DOOR_TEETH);
    versions.iter().map(|p| p.version()).collect()
}

/// The staggered id population `SI(n, 64)`, same feed order: the party
/// fold's dual of [`stagger_versions`].
#[cfg(feature = "scan-meter")]
fn stagger_parties(n: usize) -> Vec<crate::Party> {
    let (_, ids) = crate::meter::registry::Shape::StaggerPopulation.population(n, FOLD_DOOR_TEETH);
    ids.iter()
        .map(|p| crate::Party::decode(&p.bytes[..]).expect("generated ids are strict normal form"))
        .collect()
}

/// One door run's packed-stream scan bits, with the population's total
/// encoded bytes printed beside it so a re-pin can restate each floor's
/// linear reference without editing the harness.
#[cfg(feature = "scan-meter")]
fn door_scan_bits<R>(name: &str, n: usize, input_bytes: usize, run: impl FnOnce() -> R) -> u64 {
    crate::meter::reset_scan_bits();
    std::hint::black_box(run());
    let bits = crate::meter::scan_bits();
    eprintln!("MEASURED {name}: n={n} input_bytes={input_bytes} scan_bits={bits}");
    bits
}

/// Scan work of one `Version::join_all` over the stagger population
/// (the first element as the receiver, the rest as items).
#[cfg(feature = "scan-meter")]
fn join_all_scan_bits(n: usize) -> u64 {
    let mut population = stagger_versions(n);
    let bytes: usize = population.iter().map(|v| v.encode().len()).sum();
    let receiver = population.remove(0);
    door_scan_bits("version_join_all_scan", n, bytes, || {
        receiver.join_all(population)
    })
}

/// The staggered notch population, the meet dual of
/// [`stagger_versions`].
///
/// Element `i` is the unit plateau (height 1 everywhere) projected
/// onto the complement of the staggered id `SI(n, 64, i)`, so it
/// carries notches (height 0) exactly over that id's interleaved
/// blocks.
///
/// A meet keeps every operand's notches — the pointwise min drops to 0
/// wherever either side does — and the bit-reversed feed makes each
/// balanced merge combine maximally interleaving notch sets, so meet
/// intermediates swell to near the sum of their inputs exactly as the
/// stagger joins' do. (Populations whose meets collapse — dominating
/// shades, disjoint ticks — cannot carry this witness: their balanced
/// meets read sublinear in the input, with nothing for a floor to
/// hold.)
#[cfg(feature = "scan-meter")]
fn stagger_notch_versions(n: usize) -> Vec<crate::Version> {
    let full = crate::Version::try_from(1u64).expect("the unit plateau is a valid version");
    stagger_parties(n)
        .iter()
        .map(|p| {
            let rest = crate::Party::seed()
                .without(p)
                .expect("a staggered id is a proper subregion of the seed");
            (&full / &rest).to_version()
        })
        .collect()
}

/// Scan work of one `Version::meet_all` over the staggered notch
/// population (the first element as the receiver, the rest as items).
#[cfg(feature = "scan-meter")]
fn meet_all_scan_bits(n: usize) -> u64 {
    let mut population = stagger_notch_versions(n);
    let bytes: usize = population.iter().map(|v| v.encode().len()).sum();
    let receiver = population.remove(0);
    door_scan_bits("version_meet_all_scan", n, bytes, || {
        receiver.meet_all(population)
    })
}

/// Scan work of one `Version::span_all` over the stagger population
/// (the first element as the receiver, the rest as items).
#[cfg(feature = "scan-meter")]
fn span_all_scan_bits(n: usize) -> u64 {
    let population = stagger_versions(n);
    let bytes: usize = population.iter().map(|v| v.encode().len()).sum();
    let (receiver, items) = population
        .split_first()
        .expect("the stagger population is nonempty");
    door_scan_bits("version_span_all_scan", n, bytes, || {
        receiver.span_all(items).into_owned()
    })
}

/// Scan work of one `Party::join_all` over the staggered ids (the
/// first id as the receiver, the rest as items).
#[cfg(feature = "scan-meter")]
fn party_join_all_scan_bits(n: usize) -> u64 {
    let mut population = stagger_parties(n);
    let bytes: usize = population.iter().map(|p| p.encode().len()).sum();
    let mut receiver = population.remove(0);
    door_scan_bits("party_join_all_scan", n, bytes, || {
        receiver
            .join_all(population)
            .expect("staggered ids are pairwise disjoint")
    })
}

/// Scan work of one `Clock::join_all` over clocks pairing each
/// staggered id with its staggered version (the first as the
/// receiver), so both components carry per-line history.
#[cfg(feature = "scan-meter")]
fn clock_join_all_scan_bits(n: usize) -> u64 {
    let mut population: Vec<crate::Clock> = stagger_parties(n)
        .into_iter()
        .zip(stagger_versions(n))
        .map(|(p, v)| crate::Clock::from_parts(p, v))
        .collect();
    let bytes: usize = population.iter().map(|c| c.encode().len()).sum();
    let mut receiver = population.remove(0);
    door_scan_bits("clock_join_all_scan", n, bytes, || {
        receiver
            .join_all(population)
            .expect("staggered ids are pairwise disjoint")
            .clone()
    })
}

/// Assert one fold door's scan growth across a x4 population reaches
/// its measured floor, naming the door on failure.
#[cfg(feature = "scan-meter")]
fn assert_log_factor_alive(door: &str, lo: u64, hi: u64, min_growth: f64) {
    let growth = hi as f64 / lo.max(1) as f64;
    assert!(
        growth >= min_growth,
        "{door}'s scan work grew only x{growth:.2} across a x4 population \
         growth ({lo} -> {hi} bits; the log factor reads >= x{min_growth}): \
         the documented `O(D log k)` overstates for this door, so update \
         its `# Complexity` section and this pin together"
    );
}

/// `Version::join_all`'s log factor is alive at its public door.
///
/// Scan work on the scatter population grows faster than its input
/// across a x4 population growth — the balanced reduction's
/// `O(D log k)`, which is what the door's `# Complexity` section
/// documents. If a linear fold lands behind this door, this pin reads
/// red, and the rustdoc and this floor must move in one change.
///
/// Deterministic counter, dev profile. The linear reference is the
/// population's own measured byte growth, x4.77 (63,488 -> 303,104 B
/// across n = 256 -> 1,024 at 64 blocks; leaf paths deepen with the
/// slot count, so bytes grow slightly faster than arity) — a
/// scan-linear fold reads that ratio; the door reads x5.82
/// (4,906,266 -> 28,537,882 bits), the log factor's marginal. The
/// floor sits midway.
#[cfg(feature = "scan-meter")]
#[test]
fn version_join_all_log_factor_is_alive() {
    const MIN_GROWTH: f64 = 5.29;
    assert_log_factor_alive(
        "Version::join_all",
        join_all_scan_bits(256),
        join_all_scan_bits(1024),
        MIN_GROWTH,
    );
}

/// `Version::meet_all`'s log factor is alive at its public door.
///
/// Scan work on the notch population — the meet dual of the scatter
/// ticks, where meets grow instead of shrinking — grows faster than
/// its input across a x4 population growth, per the door's
/// `# Complexity` section. If a linear fold lands behind this door,
/// this pin reads red, and the rustdoc and this floor must move in
/// one change.
///
/// Deterministic counter, dev profile. The linear reference is the
/// population's measured byte growth, x4.77 (63,742 -> 304,126 B
/// across n = 256 -> 1,024); the door reads x5.82
/// (4,907,680 -> 28,543,880 bits). The floor sits midway.
#[cfg(feature = "scan-meter")]
#[test]
fn version_meet_all_log_factor_is_alive() {
    const MIN_GROWTH: f64 = 5.29;
    assert_log_factor_alive(
        "Version::meet_all",
        meet_all_scan_bits(256),
        meet_all_scan_bits(1024),
        MIN_GROWTH,
    );
}

/// `Version::span_all`'s log factor is alive at its public door.
///
/// Scan work on the scatter population grows faster than its input
/// across a x4 population growth: the hull fold carries both lattice
/// directions through one balanced counter, and the join direction
/// grows on scatter exactly as `join_all`'s does. If a linear hull
/// lands behind this door, this pin reads red, and the rustdoc and
/// this floor must move in one change.
///
/// Deterministic counter, dev profile. The linear reference is the
/// population's measured byte growth, x4.77 (63,488 -> 303,104 B
/// across n = 256 -> 1,024); the door reads x5.76
/// (5,241,754 -> 30,212,634 bits). The floor sits midway.
#[cfg(feature = "scan-meter")]
#[test]
fn version_span_all_log_factor_is_alive() {
    const MIN_GROWTH: f64 = 5.26;
    assert_log_factor_alive(
        "Version::span_all",
        span_all_scan_bits(256),
        span_all_scan_bits(1024),
        MIN_GROWTH,
    );
}

/// `Party::join_all`'s log factor is alive at its public door.
///
/// Scan work folding the scattered shares of a balanced fork grows
/// faster than its input across a x4 population growth: scattered
/// sibling unions cannot collapse, so intermediates grow exactly as
/// version scatter joins do. If a linear fold lands behind this door,
/// this pin reads red, and the rustdoc and this floor must move in
/// one change.
///
/// Deterministic counter, dev profile. The linear reference is the
/// population's measured byte growth, x4.80 (40,960 -> 196,608 B
/// across n = 256 -> 1,024); the door reads x4.99
/// (7,260,488 -> 36,191,048 bits) — the factor's expression is
/// weaker on ids than on versions (denser unions spell fewer bits
/// per block, thinning the upper levels), so this floor holds the
/// narrowest gap of the five doors; both endpoints are exact
/// counters, so the gap is stable, not noisy. The floor sits midway.
#[cfg(feature = "scan-meter")]
#[test]
fn party_join_all_log_factor_is_alive() {
    const MIN_GROWTH: f64 = 4.89;
    assert_log_factor_alive(
        "Party::join_all",
        party_join_all_scan_bits(256),
        party_join_all_scan_bits(1024),
        MIN_GROWTH,
    );
}

/// `Clock::join_all`'s log factor is alive at its public door.
///
/// Scan work folding scattered fork-share clocks (each ticked once,
/// so both components carry per-line history) grows faster than its
/// input across a x4 population growth — the party union and the
/// version join both ride the balanced reduction. If a linear fold
/// lands behind this door, this pin reads red, and the rustdoc and
/// this floor must move in one change.
///
/// Deterministic counter, dev profile. The linear reference is the
/// population's measured byte growth, x4.79 (104,448 -> 499,712 B
/// across n = 256 -> 1,024); the door reads x5.32
/// (12,166,758 -> 64,770,022 bits), the two components' factors
/// blended. The floor sits midway.
#[cfg(feature = "scan-meter")]
#[test]
fn clock_join_all_log_factor_is_alive() {
    const MIN_GROWTH: f64 = 5.05;
    assert_log_factor_alive(
        "Clock::join_all",
        clock_join_all_scan_bits(256),
        clock_join_all_scan_bits(1024),
        MIN_GROWTH,
    );
}

/// The multiplication-bound claims' answer-embedded product is alive —
/// in factor *content*, not width alone.
///
/// The plateau-puncture rank equals the closed form `2·x·y + 1` at
/// scale `2^(66d)` over the family's committed factors, computed here
/// through an independent backend multiplication — the value
/// structure behind the `Ω(M(|v|))` floor the rank rustdoc states:
/// the exact answer is a wide × dense integer product whose factors the
/// input funds separately (the arbitrary-factor reduction is the
/// query fold's `arbitrary_factors_embed_their_product_in_exact_rank`
/// proptest; this pin holds the committed measured instance). Width
/// scaling alone does not make the instance hard — a power-of-two
/// plateau's parked factor is an all-ones run the settle's own
/// balanced-digit spelling compacts to two signed digits, and a
/// fixed-stride mass telescopes as a geometric series — so the pin
/// guards the factors' content: the parked plunge `x − 1` must stay
/// `Θ(w)` terms under that same compaction, and the mass's `d` digits
/// must stay isolated by more than a full digit (compaction-immune)
/// at non-uniform jitter. A representation or content change that
/// weakens the embedding reads red here, and the rustdoc and this pin
/// move in one change; the cost legs (flat traffic, schoolbook red)
/// live in the flatness bands (`tests/meter.rs`) and the committed
/// schoolbook kernels beside the query fold's tests.
#[cfg(feature = "meter")]
#[test]
fn mul_bound_embedding_is_alive() {
    use dashu_int::ops::BitTest;
    use dashu_int::UBig;

    /// The count of nonzero balanced signed digits the settle's own
    /// compaction (`mul_into`'s recentering, replicated) spells a
    /// magnitude into.
    fn balanced_terms(value: &UBig) -> usize {
        let bytes = value.to_le_bytes();
        let digits = bytes
            .chunks(4)
            .map(|c| {
                let mut d = [0u8; 4];
                d[..c.len()].copy_from_slice(c);
                u64::from(u32::from_le_bytes(d))
            })
            .collect::<Vec<u64>>();
        let mut terms = 0usize;
        let mut carry = 0u64;
        for digit in digits {
            let t = digit + carry;
            if t > 1 << 31 {
                if (1u64 << 32) != t {
                    terms += 1;
                }
                carry = 1;
            } else {
                if t != 0 {
                    terms += 1;
                }
                carry = 0;
            }
        }
        terms + usize::from(carry == 1)
    }

    let (w, d) = (64usize, 48usize);
    let v = Shape::PlateauPuncture.packed2(w, d).version();
    let (x, y) = crate::meter::plateau_puncture_factors(w, d);
    assert_eq!(
        (x.bit_len(), y.bit_len()),
        (32 * w, 66 * d - 1),
        "both factors must scale with the family parameters: a degenerate \
         factor would make the embedded product one-sided"
    );
    // The parked factor's content: its 64 digits compact to 65
    // balanced terms — the recentering splits high digits into a
    // negative arm and a carry (measured at this content; a plateau
    // of 2^(32w) would read 2).
    assert_eq!(
        balanced_terms(&(&x - 1u8)),
        65,
        "the parked plunge x − 1 must stay incompressible under the \
         settle's own balanced-digit compaction"
    );
    // The mass's content: exactly d isolated bits, pairwise more than
    // a full base-2^32 digit apart (compaction-immune), and not an
    // arithmetic progression (no geometric-series closed form).
    let positions: Vec<usize> = (0..y.bit_len()).filter(|&b| y.bit(b)).collect();
    assert_eq!(positions.len(), d, "the mass spells one bit per turn");
    assert!(
        positions.windows(2).all(|p| p[1] - p[0] > 32),
        "every mass gap must exceed a full digit: the compaction could \
         merge closer terms"
    );
    let strides: std::collections::BTreeSet<usize> =
        positions.windows(2).map(|p| p[1] - p[0]).collect();
    assert!(
        strides.len() > 1,
        "a fixed-stride mass is a geometric series: x·y telescopes to \
         shifts and one short division, and the instance stops witnessing \
         the floor"
    );
    assert_eq!(
        v.rank().to_string(),
        format!("{}/2^{}", ((&x * &y) << 1usize) + 1u8, 66 * d),
        "the plateau-puncture rank must be the plateau times the punctured \
         turn mass: the answer no longer embeds the product, so the \
         documented Ω(M(·)) floor lost its witness"
    );
}

/// The pair door's answer-embedded-product liveness.
///
/// The multiplication-bound pair claims (distance, lag) enter the
/// settle through the pair co-sweep — a distinct entry point from
/// rank's single-stream fold, which the single-stream embedding and
/// schoolbook witnesses exercise — so the `Ω(M(a + b))` floor needs
/// its embedding family constructed through the pair operations' own
/// doors, not inferred from rank alone.
///
/// Against the empty version, the valuation identities collapse to
/// `distance(v, ∅) = lag(∅, v) = rank(v)` and `lag(v, ∅) = 0`, so the
/// plateau-puncture closed form (the factors' scaling and
/// compaction-immunity are [`mul_bound_embedding_is_alive`]'s
/// preconditions, asserted there at the same dimensions) must
/// reproduce exactly through `Version::distance` and `Version::lag`.
/// The co-operand is empty, *not equal*: the pair entries' only fast
/// path is canonical equality, so both directed calls and the
/// symmetric one run the pair integrator whole. An answer that stops
/// embedding the product — or a pair-door rewiring that stops
/// reaching the shared integrator exactly — loses the pair claims
/// their floor witness here.
#[cfg(feature = "meter")]
#[test]
fn mul_bound_pair_embedding_is_alive() {
    let (w, d) = (64usize, 48usize);
    let v = Shape::PlateauPuncture.packed2(w, d).version();
    let empty = crate::Version::new();
    assert!(
        !v.is_empty() && empty.is_empty(),
        "the pair must be unequal: an equal pair would answer through \
         the canonical-equality rung without running the pair co-sweep"
    );
    let (x, y) = crate::meter::plateau_puncture_factors(w, d);
    let closed = format!("{}/2^{}", ((&x * &y) << 1usize) + 1u8, 66 * d);
    assert_eq!(
        v.distance(&empty).to_string(),
        closed,
        "distance against the empty version must be the plateau times the \
         punctured turn mass: the pair door's answer no longer embeds the \
         product, so the pair claims lost their floor witness"
    );
    assert_eq!(
        empty.lag(&v).to_string(),
        closed,
        "the dominated side's lag must be the whole plateau-puncture rank: \
         the pair door's answer no longer embeds the product"
    );
    assert_eq!(
        v.lag(&empty),
        crate::Rank::ZERO,
        "the dominating side lags the empty version by nothing: the \
         directed functional's zero side must stay exact"
    );
}

/// The key doors' answer-embedded-product liveness.
///
/// The multiplication-bound key claims (`Ranked::encode_rank`,
/// `Ranked::encode`, `Ranked::decode`) emit or verify the rank through
/// their own fused entry points — distinct doors from
/// `Version::rank`'s, which [`mul_bound_embedding_is_alive`] pins — so
/// the embedding family must reproduce through them directly, not by
/// composing the committed `encode_rank == rank().encode()` law
/// (whose sampled generators do not reach this family) with rank's
/// pin.
///
/// On the plateau-puncture instance the key's rank component must
/// decode back to the closed-form product rank, and the composite key
/// must survive its own strict decode — whose verifying rank fold is
/// the `Ranked::decode` claim's multiplication-bound term, here
/// demonstrated firing on the embedding family itself. An encode door
/// that stops emitting the product's digits, or a decode door that
/// stops verifying them, loses the key claims their floor witness
/// here.
#[cfg(feature = "meter")]
#[test]
fn mul_bound_key_embedding_is_alive() {
    let (w, d) = (64usize, 48usize);
    let v = Shape::PlateauPuncture.packed2(w, d).version();
    let (x, y) = crate::meter::plateau_puncture_factors(w, d);
    let closed = format!("{}/2^{}", ((&x * &y) << 1usize) + 1u8, 66 * d);
    let rank_key = crate::Ranked::from(&v).encode_rank();
    assert_eq!(
        crate::Rank::decode(&rank_key[..])
            .expect("the fused rank key is canonical")
            .to_string(),
        closed,
        "the fused rank-key emission must carry the plateau times the \
         punctured turn mass: the key door's answer no longer embeds the \
         product, so the key claims lost their floor witness"
    );
    let decoded = crate::Ranked::decode(&crate::Ranked::from(&v).encode()[..])
        .expect("the composite key round-trips through its verifying decode");
    assert_eq!(
        decoded.version(),
        &v,
        "the composite key's strict decode must recover the embedding \
         family's version through its own verifying rank fold"
    );
}
