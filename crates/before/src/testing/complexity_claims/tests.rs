//! The complexity-claims binding tests.
//!
//! They hold the roster total over the public surface, every site's
//! `# Complexity` section opening with its bound's rendered line, the
//! cited board rows alive, the superlinear claims equal to the bench
//! judge's red set, and the non-linear classes' liveness pins
//! red-on-cure (the render merge's growth, the fold's log factor, and
//! the `MulBound` claims' answer-embedded product with its named
//! witness tests). The roster and the scanner live in the parent
//! module.

use crate::meter::registry::Shape;
use std::collections::BTreeSet;

use super::{doc_index, Bound, Cells, Claim, Class, DocIndex, RedStance, CLAIMS, NON_OPERATIONS};
use crate::meter::board::{self, BenchMode};
use crate::testing::surface_coverage;

/// Every board operation name, from the board's own axis declarations at
/// a tiny build-only scale.
fn board_ops() -> BTreeSet<String> {
    board::bench_cells(0.02, BenchMode::Full)
        .into_iter()
        .map(|cell| cell.op.to_owned())
        .collect()
}

/// The bench judge's committed expected-verdict roster
/// (`tools/benchjudge-expected.json`; its membership is pinned by
/// `tests/bench_judge_roster.rs`).
fn judge_roster() -> serde_json::Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tools/benchjudge-expected.json"
    );
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("reading the judge roster at {path} failed: {err}"));
    serde_json::from_str(&text)
        .unwrap_or_else(|err| panic!("the judge roster at {path} is not JSON: {err}"))
}

/// The claims roster is total over the public surface, exactly.
///
/// Every mechanically extracted `pub fn` and every coverage family row
/// has one claim (or a place in the pinned non-operation list), and
/// nothing else does. A new public operation fails here until its
/// documented class is pinned; a removed one orphans its claim.
#[test]
fn claims_are_total_over_the_public_surface() {
    let mut surface: BTreeSet<String> = surface_coverage::extract_public_fns();
    surface.extend(
        surface_coverage::FAMILY_SURFACE
            .iter()
            .map(|row| row.op.to_owned()),
    );
    let mut claimed = BTreeSet::new();
    for claim in CLAIMS {
        assert!(
            claimed.insert(claim.op.to_owned()),
            "duplicate claim row: {}",
            claim.op
        );
    }
    claimed.extend(NON_OPERATIONS.iter().map(|op| (*op).to_owned()));
    let unclaimed: Vec<_> = surface.difference(&claimed).collect();
    let orphaned: Vec<_> = claimed.difference(&surface).collect();
    assert!(
        unclaimed.is_empty() && orphaned.is_empty(),
        "the claims roster and the public surface disagree:\n  \
         public operations with no complexity claim: {unclaimed:?}\n  \
         claims naming no public operation: {orphaned:?}"
    );
}

/// Every claim's `# Complexity` section exists at its recorded site and
/// opens with the roster bound's rendered claim sentence, byte for byte
/// up to line wrapping.
///
/// A class edit in the rustdoc that skips this roster (or vice versa)
/// is a named failure, and every site's normative claim is the
/// roster's own rendering, never hand-drifted prose.
///
/// Custom bounds must also state a non-empty reason: the escape hatch
/// is a documented decision, never a bare opt-out.
#[test]
fn complexity_sections_open_with_their_rendered_lines() {
    let index = doc_index();
    let mut errors = Vec::new();
    for claim in CLAIMS {
        for check in claim.checks {
            if let Bound::Custom { reason, .. } = check.bound {
                if reason.trim().len() < 20 {
                    errors.push(format!(
                        "{}: a Custom bound must state a substantial reason",
                        claim.op
                    ));
                }
            }
            match index.section(claim.op, check.site) {
                Err(err) => errors.push(err),
                Ok(section) => {
                    let want = check.bound.render();
                    let got = ::complexity_claims::opening_paragraph(section);
                    if !got.starts_with(want) {
                        errors.push(format!(
                            "{}: the `# Complexity` section at {:?} does not open with the \
                             rendered bound\n    want: {want}\n    got:  {got}",
                            claim.op, check.site,
                        ));
                    }
                }
            }
        }
    }
    assert!(
        errors.is_empty(),
        "rustdoc complexity sections drifted from the claims roster:\n  {}",
        errors.join("\n  ")
    );
}

/// Every board row a claim cites exists on the board's operation axis, so
/// a renamed or retired row orphans the claims that leaned on it by name.
#[test]
fn cited_board_rows_exist() {
    let ops = board_ops();
    let mut missing = Vec::new();
    for claim in CLAIMS {
        match &claim.cells {
            Cells::Board(cells) => {
                for (op, _) in *cells {
                    if !ops.contains(*op) {
                        missing.push(format!("{}: cites unknown board row {op}", claim.op));
                    }
                }
            }
            // Uncelled rows carry their reason as data, mirroring the
            // board's coverage table; hold it non-empty.
            Cells::Uncelled(reason) => {
                assert!(
                    !reason.is_empty(),
                    "{}: an uncelled claim must state its reason",
                    claim.op
                );
            }
        }
    }
    assert!(
        missing.is_empty(),
        "claims cite board rows that do not exist:\n  {}",
        missing.join("\n  ")
    );
}

/// Every class satisfies its declared contract ([`super::ClassContract`]):
/// one uniform enforcement over the whole vocabulary, so no class's
/// mechanical binding is bespoke machinery.
///
/// - **exponent reds**: a `Forbidden` class cites no operation with a
///   standing exponent-mechanism red, a `Required` class cites nothing
///   without one (the class-binding seal; the decoration fixture below
///   proves the `Required` leg fires), and `Allowed` classes are
///   indifferent.
/// - **judge reds**: the rows cited under judge-red classes equal the
///   bench judge's committed red set exactly, and no other class cites
///   a rostered row — curing a display red (or rostering a new one)
///   must reach the documentation through this name.
/// - **tokens**: a claim citing a class renders the class's defining
///   token in a rendered claim line, and an exclusive token appears only on
///   claims citing its class, so the prose and the class cannot drift
///   apart.
/// - **witnesses**: every class's named committed witnesses exist in
///   the tree, so deleting or renaming a measurement pin or adequacy
///   kernel fails a reviewed name here, never silently.
#[test]
fn classes_satisfy_their_contracts() {
    let ops = board_ops();
    let exponent_red_ops = exponent_red_ops();
    let rostered: BTreeSet<String> = judge_roster()["red"]
        .as_array()
        .expect("the judge roster's red class is a list")
        .iter()
        .map(|cell| {
            cell.as_str()
                .expect("cell IDs are strings")
                .split('/')
                .next()
                .expect("cell IDs are op/family")
                .to_owned()
        })
        // The judge also rosters non-board tripwire benches (the
        // schoolbook probe); only board rows bind rustdoc claims.
        .filter(|op| ops.contains(op))
        .collect();
    let mut judge_claimed: BTreeSet<String> = BTreeSet::new();
    let mut problems: Vec<String> = Vec::new();
    let index = doc_index();
    for claim in CLAIMS {
        problems.extend(token_problems(claim, &index));
        let Cells::Board(cells) = &claim.cells else {
            continue;
        };
        for (op, class) in *cells {
            problems.extend(stance_contradiction(
                claim.op,
                op,
                *class,
                &exponent_red_ops,
            ));
            if class.contract().judge_red {
                judge_claimed.insert((*op).to_owned());
            } else if rostered.contains(*op) {
                problems.push(format!(
                    "{}: cites {op} as {class:?}, but the bench judge holds it red",
                    claim.op
                ));
            }
        }
    }
    assert_eq!(
        judge_claimed, rostered,
        "the rustdoc's judge-red class claims and the bench judge's red set \
         disagree: update the claims roster and the `# Complexity` sections \
         together"
    );
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for class in Class::ALL.iter().copied() {
        for (file, witness) in class.contract().witnesses {
            let path = root.join(file);
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|err| panic!("reading {}: {err}", path.display()));
            let fns = test_fns(&text);
            assert!(
                !fns.is_empty(),
                "the witness scanner found no #[test] fns in {file}: the scan is broken, \
                 not the witnesses"
            );
            if !fns.contains(*witness) {
                problems.push(format!(
                    "{file} no longer holds the #[test] fn `{witness}`: {class:?} lost a \
                     named witness — re-derive the class with the change that moved it"
                ));
            }
        }
    }
    assert!(
        problems.is_empty(),
        "the claims roster violates its class contracts:\n  {}",
        problems.join("\n  ")
    );
}

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
/// cost that grows faster than the operand" sentence and the claims
/// roster's `SuperlinearTime` class describe. When the render-merge cure
/// lands this pin reads red, and the rustdoc, the claims roster, and this
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
         `# Complexity` sections, the claims roster, and this pin together"
    );
}

// ─── the fold doors' log-factor liveness ───────────────────────────────
//
// One witness per public `FoldLog` door, each over a committed
// registry population whose balanced merges swell to near the sum of
// their inputs (coalescing intermediates are the enemy of visibility),
// so the reduction's log factor separates measurably from a linear
// fold. Every floor is measured at its own door — the doors share the
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

/// Scan work of one `Version::join_all` over the stagger population.
#[cfg(feature = "scan-meter")]
fn join_all_scan_bits(n: usize) -> u64 {
    let population = stagger_versions(n);
    let bytes: usize = population.iter().map(|v| v.encode().len()).sum();
    door_scan_bits("version_join_all_scan", n, bytes, || {
        crate::Version::join_all(population)
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
/// population.
#[cfg(feature = "scan-meter")]
fn meet_all_scan_bits(n: usize) -> u64 {
    let population = stagger_notch_versions(n);
    let bytes: usize = population.iter().map(|v| v.encode().len()).sum();
    door_scan_bits("version_meet_all_scan", n, bytes, || {
        crate::Version::meet_all(population).expect("the notch population is nonempty")
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
         its `# Complexity` section, the claims roster, and this pin \
         together"
    );
}

/// `Version::join_all`'s log factor is alive at its public door.
///
/// Scan work on the scatter population grows faster than its input
/// across a x4 population growth — the balanced reduction's
/// `O(D log k)`, which is what the door's `# Complexity` section and
/// the claims roster's `FoldLog` class document. If a linear fold
/// lands behind this door, this pin reads red, and the rustdoc, the
/// claims roster, and this floor must move in one change.
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
/// `# Complexity` section and the `FoldLog` class. If a linear fold
/// lands behind this door, this pin reads red, and the rustdoc, the
/// claims roster, and this floor must move in one change.
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
/// lands behind this door, this pin reads red, and the rustdoc, the
/// claims roster, and this floor must move in one change.
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
/// this pin reads red, and the rustdoc, the claims roster, and this
/// floor must move in one change.
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
/// lands behind this door, this pin reads red, and the rustdoc, the
/// claims roster, and this floor must move in one change.
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

/// The `MulBound` claims' answer-embedded product is alive — in
/// factor *content*, not width alone.
///
/// The plateau-puncture rank equals the closed form `2·x·y + 1` at
/// scale `2^(66d)` over the family's committed factors, computed here
/// through an independent backend multiplication — the value
/// structure behind the `Ω(M(|v|))` floor the class carries: the
/// exact answer is a wide × dense integer product whose factors the
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
/// weakens the embedding reads red here, and the rustdoc, the roster
/// class, and this pin move in one change; the cost legs (flat
/// traffic, schoolbook red) live in the witness tests the
/// name-binding test above pins.
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
         turn mass: the answer no longer embeds the product, so the MulBound \
         class and the Ω(M(·)) floor lost their witness"
    );
}

/// The pair door's answer-embedded-product liveness.
///
/// The `MulBound` pair claims (distance, lag) enter the settle through
/// the pair co-sweep — a distinct entry point from rank's single-stream
/// fold, which the class contract's other embedding and schoolbook
/// witnesses exercise — so the `Ω(M(a + b))` floor needs its embedding
/// family constructed through the pair operations' own doors, not
/// inferred from rank alone.
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
         product, so the MulBound pair claims lost their floor witness"
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
/// The `MulBound` key claims (`Ranked::encode_rank`, `Ranked::encode`,
/// `Ranked::decode`) emit or verify the rank through their own fused
/// entry points — distinct doors from `Version::rank`'s, which
/// [`mul_bound_embedding_is_alive`] pins — so the embedding family
/// must reproduce through them directly, not by composing the
/// committed `encode_rank == to_rank().encode()` law (whose sampled
/// generators do not reach this family) with rank's pin.
///
/// On the plateau-puncture instance the key's rank component must
/// decode back to the closed-form product rank, and the composite key
/// must survive its own strict decode — whose verifying rank fold is
/// the `Ranked::decode` claim's `MulBound` term, here demonstrated
/// firing on the embedding family itself. An encode door that stops
/// emitting the product's digits, or a decode door that stops
/// verifying them, loses the key claims their floor witness here.
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
         product, so the MulBound key claims lost their floor witness"
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

/// The tripwire the roster's own vocabulary rests on: a doc block whose
/// `# Complexity` section is missing, or whose section does not end
/// where the scanner thinks, is detected — the scanner is not vacuously
/// green.
#[test]
fn scanner_detects_missing_sections_and_tokens() {
    assert_eq!(
        super::section_of("Summary line.\n\nNo sections here.\n"),
        None,
        "a block with no Complexity section must scan as missing"
    );
    let section =
        super::section_of("Summary.\n\n# Complexity\n\n`O(|v|)` time.\n\n# Panics\n\nNever.\n")
            .expect("the section exists");
    assert!(
        section.contains("`O(|v|)`") && !section.contains("Never"),
        "the section slice must carry its own tokens and end at the next heading"
    );
}

/// A `Claim` is inspectable in failure messages (`Site` derives `Debug`);
/// keep the type checked so the roster stays printable.
#[test]
fn claim_rows_are_printable() {
    let row: &Claim = &CLAIMS[0];
    assert!(!format!("{:?}", row.checks[0].site).is_empty());
}

/// The rendered-line vocabulary is fixed: each template renders exactly
/// its documented line, so a template edit is a reviewed diff here
/// before it fans out into every doc site.
#[test]
fn bound_templates_render_their_documented_lines() {
    assert_eq!(Bound::Constant.render(), "`O(1)`.");
    assert_eq!(Bound::Linear.render(), "`O(n)`.");
    assert_eq!(Bound::LinearPair.render(), "`O(a + b)`.");
    assert_eq!(Bound::TextRender.render(), "`O(n + t)`.");
    assert_eq!(Bound::TextParse.render(), "`O(t + n)`.");
    assert_eq!(Bound::Fold.render(), "`O(D log k)` time, `O(D)` space.");
    assert_eq!(
        Bound::FoldSearch.render(),
        "`O(D log k + B log n)` time, `O(D)` space."
    );
    assert_eq!(
        Bound::MulBound.render(),
        "`O(n)` space; time `O(M(n) · log n)` worst case, \
         `O(n log n)` with width-bounded parked drifts."
    );
    assert_eq!(
        Bound::MulBoundPair.render(),
        "`O(a + b)` space; time `O(M(a + b) · log (a + b))` worst \
         case, `O((a + b) log (a + b))` with width-bounded parked drifts."
    );
    assert_eq!(
        Bound::Custom {
            line: "Priced elsewhere.",
            reason: "fixture",
        }
        .render(),
        "Priced elsewhere."
    );
}

/// The operations [`board::BOARD_EXPECTED_REDS`] holds red on an
/// exponent mechanism at either acceptance scale.
fn exponent_red_ops() -> BTreeSet<&'static str> {
    board::BOARD_EXPECTED_REDS
        .iter()
        .filter(|red| red.exponent)
        .map(|red| red.op)
        .collect()
}

// The witness scanner behind the class contracts is the shared machinery's
// `test_fns`: attribute-gated exactly as the board↔band parity pin's, so a
// prose mention of a deleted test never counts as its existence, and cfg
// attributes between `#[test]` and the fn keep the arming.
use ::complexity_claims::test_fns;

/// One cell's exponent-red stance verdict, from the class's contract:
/// the contradiction message, if the cited class and the board's
/// mechanism-tagged red set disagree.
///
/// The class-binding seal's kernel, enforced uniformly inside
/// `classes_satisfy_their_contracts`. The bench judge's red set binds
/// only wall time, and an operation's time legs can sit under the
/// judge's resolution at bench scales — so without this seal a
/// counter-superlinear kernel keeps a `Linear` rustdoc claim with every
/// gate green. The seal's adequacy was verified by mutation: a claim
/// pinned `Class::Linear` while citing board cells whose committed
/// mechanism tags read `exponent` passes every other gate, and this
/// stance names the contradiction. The `Required` stance is the reverse
/// leg: a class whose whole evidence is a standing exponent red is
/// decoration without one, so the cure that flips the board pins must
/// move the class in the same change (the fixture below keeps that leg
/// firing).
fn stance_contradiction(
    claim_op: &str,
    op: &str,
    class: Class,
    exponent_red_ops: &BTreeSet<&str>,
) -> Option<String> {
    match class.contract().exponent_reds {
        RedStance::Forbidden => exponent_red_ops.contains(op).then(|| {
            format!(
                "{claim_op}: cites {op} as {class:?}, but the board holds it red on an \
                 exponent mechanism (BOARD_EXPECTED_REDS)"
            )
        }),
        RedStance::Required => (!exponent_red_ops.contains(op)).then(|| {
            format!(
                "{claim_op}: claims {op} {class:?} with no standing \
                 exponent-mechanism board red: the class is decoration, move it \
                 to the class its evidence supports"
            )
        }),
        RedStance::Allowed => None,
    }
}

/// One claim's token legs against the class contracts: the citing
/// direction over the roster's rendered lines, the exclusive direction
/// over the rendered lines and the live section text.
///
/// The section text is the exclusive leg's ground truth — the rendered
/// lines are the producer's own declaration, so a claim re-cited under
/// a weaker class with its bound swapped would pass a roster-only scan
/// while the rustdoc still carries an exclusive token. (The citing
/// direction may stay on the rendered lines: the opening-line test
/// holds every rendered line verbatim in its section, so a rendered
/// class token is a section-carried one.) The downgrade guard below
/// keeps the exclusive leg firing on exactly the swapped-bound
/// artifact.
fn token_problems(claim: &Claim, index: &DocIndex) -> Vec<String> {
    let rendered: Vec<&str> = claim
        .checks
        .iter()
        .map(|check| check.bound.render())
        .collect();
    let cited: Vec<Class> = match &claim.cells {
        Cells::Board(cells) => cells.iter().map(|(_, class)| *class).collect(),
        Cells::Uncelled(_) => Vec::new(),
    };
    let mut problems = Vec::new();
    for class in Class::ALL.iter().copied() {
        let contract = class.contract();
        let Some(token) = contract.token else {
            continue;
        };
        if cited.contains(&class) {
            if !rendered.iter().any(|line| line.contains(token)) {
                problems.push(format!(
                    "{}: cites a {class:?} cell but renders no line containing `{token}`",
                    claim.op
                ));
            }
        } else if contract.token_exclusive {
            if rendered.iter().any(|line| line.contains(token)) {
                problems.push(format!(
                    "{}: renders a line containing `{token}` without citing a {class:?} \
                     cell: the token is that class's alone",
                    claim.op
                ));
            }
            for check in claim.checks {
                // A missing section is the opening-line test's
                // finding, not this leg's; scan what exists.
                let Ok(section) = index.section(claim.op, check.site) else {
                    continue;
                };
                if section.contains(token) {
                    problems.push(format!(
                        "{}: its `# Complexity` section at {:?} carries `{token}` without \
                         citing a {class:?} cell: the token is that class's alone, so \
                         either the class citation or the prose must move",
                        claim.op, check.site
                    ));
                }
            }
        }
    }
    problems
}

/// The seal's reverse leg fires on a constructed decoration claim: a
/// [`Class::SuperlinearCounter`] cell whose operation has no standing
/// exponent-mechanism red is named as a contradiction.
///
/// This is the committed form of the seal's mutation demonstration (the
/// doc above): the fixture cites `version_min_ticks` — whose exponent
/// reds the anchor-web cure removed — under the class that cure retired,
/// and the seal must flag it. The class stays in the vocabulary as the
/// designed home for the next counter-witnessed superlinearity finding;
/// this fixture is its adequacy tripwire while the roster carries no
/// such claim.
#[test]
fn a_witnessless_superlinear_counter_claim_is_flagged_as_decoration() {
    let exponent_red_ops = exponent_red_ops();
    let fixture = Claim {
        op: "Version::min_ticks",
        checks: &[],
        cells: Cells::Board(&[("version_min_ticks", Class::SuperlinearCounter)]),
    };
    let Cells::Board(cells) = &fixture.cells else {
        unreachable!("the fixture cites a board cell");
    };
    let (op, class) = cells[0];
    let verdict = stance_contradiction(fixture.op, op, class, &exponent_red_ops);
    assert!(
        verdict.is_some_and(|msg| msg.contains("decoration")),
        "the seal's reverse leg did not flag a SuperlinearCounter claim on an \
         operation with no standing exponent red; if {op} regained an exponent \
         red, pick a cured operation for the fixture"
    );
}

/// The token-exclusivity leg catches a downgraded claim whose bound was
/// swapped for a weaker template: the section text is the leg's ground
/// truth.
///
/// The adequacy tripwire for [`token_problems`]'s section scan,
/// holding the cheapest wrong artifact the leg must keep rejecting:
/// `Version::rank` re-cited as `Linear` (its counters *are* flat, so
/// no exponent-red stance objects; `version_rank` is not
/// judge-rostered) with its bound swapped to the plain linear template
/// — every roster-side check blesses it, and only the live
/// `# Complexity` section still carrying the MulBound class's
/// exclusive `M(` token convicts it. A revert of the leg to
/// roster-only scanning reads red here. The preconditions keep the
/// fixture meaningful: the real section must still carry the token
/// (if MulBound legitimately dissolves, re-point the fixture at a
/// live exclusive-token section), and the stance and judge legs must
/// still bless the downgrade (so this leg stays the artifact's only
/// detector).
#[test]
fn a_downgraded_mul_bound_claim_is_convicted_by_its_section_text() {
    let fixture = Claim {
        op: "Version::rank",
        checks: &[super::Check {
            site: super::Site::Fn,
            bound: Bound::Linear,
        }],
        cells: Cells::Board(&[("version_rank", Class::Linear)]),
    };
    // Precondition: the real rustdoc section still carries the
    // exclusive token the fixture's swapped bound dropped.
    let index = doc_index();
    let section = index
        .section("Version::rank", super::Site::Fn)
        .expect("Version::rank has a Complexity section");
    assert!(
        section.contains("M("),
        "Version::rank's Complexity section no longer carries `M(`: \
         re-point this fixture at a live MulBound section"
    );
    // Precondition: the stance and judge legs bless the downgrade
    // (flat counters, no red), so the token leg is the only detector.
    let exponent_red_ops = exponent_red_ops();
    let Cells::Board(cells) = &fixture.cells else {
        unreachable!("the fixture cites a board cell");
    };
    let stance = stance_contradiction(fixture.op, cells[0].0, cells[0].1, &exponent_red_ops);
    assert!(
        stance.is_none(),
        "the stance legs began flagging the downgrade fixture ({stance:?}): \
         pick a fixture only the token leg convicts"
    );
    // The leg of record convicts the artifact, by the section text.
    let problems = token_problems(&fixture, &index);
    assert!(
        problems
            .iter()
            .any(|p| p.contains("section") && p.contains("M(")),
        "the downgraded claim slipped the token-exclusivity leg \
         ({problems:?}): the leg is reading the roster's rendered lines \
         instead of the rustdoc section text"
    );
}

/// The board tiling: every public-surface row either is priced by at
/// least one live board row (its claim's cited cells) or appears in the
/// board's not-applicable table with a mechanism-based reason.
///
/// Both directions, sets disjoint, and every board row witnesses some
/// public claim.
///
/// Combined with `claims_are_total_over_the_public_surface` (one claim
/// per surface row) and `cited_board_rows_exist` (cited rows are live),
/// this closes the tiling: an operation is priced or excused, never
/// both, never neither, and the board carries no orphan row.
#[test]
fn board_coverage_tiles_the_public_surface() {
    let mut surface: BTreeSet<String> = surface_coverage::extract_public_fns();
    surface.extend(
        surface_coverage::FAMILY_SURFACE
            .iter()
            .map(|row| row.op.to_owned()),
    );
    let mut na = std::collections::BTreeMap::new();
    for (op, reason) in board::BOARD_NOT_APPLICABLE {
        assert!(
            surface.contains(*op),
            "BOARD_NOT_APPLICABLE names {op:?}, which is no public-surface row: \
             remove or rename the entry"
        );
        assert!(
            reason.len() >= 20,
            "{op}: the not-applicable reason is too thin to be a mechanism: {reason:?}"
        );
        assert!(
            na.insert(*op, *reason).is_none(),
            "{op} appears twice in BOARD_NOT_APPLICABLE"
        );
    }
    let mut cited_rows: BTreeSet<&str> = BTreeSet::new();
    for claim in CLAIMS {
        match &claim.cells {
            Cells::Board(cells) => {
                assert!(
                    !cells.is_empty(),
                    "{}: a board-celled claim must cite at least one row",
                    claim.op
                );
                assert!(
                    !na.contains_key(claim.op),
                    "{}: priced by board rows AND excused in BOARD_NOT_APPLICABLE — \
                     the tiling sides must stay disjoint; remove one",
                    claim.op
                );
                cited_rows.extend(cells.iter().map(|(row, _)| *row));
            }
            Cells::Uncelled(_) => {
                assert!(
                    na.contains_key(claim.op),
                    "{}: cites no board row and is missing from BOARD_NOT_APPLICABLE — \
                     add the table entry with its mechanism, or cell the claim",
                    claim.op
                );
            }
        }
    }
    for op in NON_OPERATIONS {
        assert!(
            na.contains_key(*op),
            "{op}: a non-operation family row must still appear in \
             BOARD_NOT_APPLICABLE with its disposition"
        );
    }
    let uncelled = CLAIMS
        .iter()
        .filter(|claim| matches!(claim.cells, Cells::Uncelled(_)))
        .count();
    assert_eq!(
        na.len(),
        uncelled + NON_OPERATIONS.len(),
        "BOARD_NOT_APPLICABLE carries entries beyond the uncelled claims and \
         the non-operation rows: the tiling sides must stay disjoint"
    );
    // The reverse leg: every board operation row witnesses some public
    // claim, so the board carries no orphan row a rename could strand.
    let orphans: Vec<String> = board_ops()
        .into_iter()
        .filter(|op| !cited_rows.contains(op.as_str()))
        .collect();
    assert!(
        orphans.is_empty(),
        "board rows cited by no complexity claim (name the public operation \
         each prices, or retire the row): {orphans:?}"
    );
}

/// The red-triage buffer is empty at acceptance, and any in-flight
/// entry names a live board cell, exactly once, with a mechanism tag
/// and a live-task reference.
///
/// Red means untriaged, nothing else: every dashboard contradiction
/// resolves to a cure or an owner-declared model at the cell, so
/// [`board::BOARD_EXPECTED_REDS`] may hold an entry only while its
/// triage is in flight (the `task` field names the work), and this
/// assertion is the acceptance teeth — a red that persists across
/// commits is a process failure, not a status.
#[test]
fn expected_red_buffer_is_an_empty_triage_buffer() {
    let cells: BTreeSet<(String, String)> = board::bench_cells(0.02, BenchMode::Full)
        .into_iter()
        .map(|cell| (cell.op.to_owned(), cell.family.to_owned()))
        .collect();
    let mut seen = BTreeSet::new();
    for red in board::BOARD_EXPECTED_REDS {
        assert!(
            cells.contains(&(red.op.to_owned(), red.family.to_owned())),
            "{}/{} in BOARD_EXPECTED_REDS names no live board cell",
            red.op,
            red.family
        );
        assert!(
            red.exponent || red.constant,
            "{}/{} carries no mechanism tag",
            red.op,
            red.family
        );
        assert!(
            !red.task.trim().is_empty(),
            "{}/{} carries no live-task reference: an untriaged red may sit in \
             the buffer only while someone owns its triage",
            red.op,
            red.family
        );
        assert!(
            seen.insert((red.op, red.family)),
            "{}/{} appears twice in BOARD_EXPECTED_REDS",
            red.op,
            red.family
        );
    }
    assert!(
        board::BOARD_EXPECTED_REDS.is_empty(),
        "the red-triage buffer is not empty: every entry is an untriaged \
         contradiction whose resolution (a cure, or an owner-declared model \
         at the cell) must land before acceptance: {:?}",
        board::BOARD_EXPECTED_REDS
            .iter()
            .map(|red| format!("{}/{} ({})", red.op, red.family, red.task))
            .collect::<Vec<_>>()
    );
}
