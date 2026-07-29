//! The complexity-claims binding tests.
//!
//! They hold the roster total over the public surface, the prose tokens
//! present, the cited board rows alive, the superlinear claims equal to
//! the bench judge's red set, and the non-linear classes' liveness pins
//! red-on-cure (the render merge's growth, the fold's log factor, and
//! the `MulBound` claims' answer-embedded product with its named
//! witness tests). The roster and the scanner live in the parent
//! module.

use std::collections::BTreeSet;

use super::{doc_index, Cells, Claim, Class, RedStance, CLAIMS, NON_OPERATIONS};
use crate::meter::board::{self, BenchMode};
use crate::testing::triangle;

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
/// Every mechanically extracted `pub fn` and every triangle family row
/// has one claim (or a place in the pinned non-operation list), and
/// nothing else does. A new public operation fails here until its
/// documented class is pinned; a removed one orphans its claim.
#[test]
fn claims_are_total_over_the_public_surface() {
    let mut surface: BTreeSet<String> = triangle::extract_public_fns();
    surface.extend(triangle::FAMILY_SURFACE.iter().map(|row| row.op.to_owned()));
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
/// carries its pinned Big-O tokens verbatim, so a class edit in the
/// rustdoc that skips this roster (or vice versa) is a named failure.
#[test]
fn complexity_sections_carry_their_pinned_tokens() {
    let index = doc_index();
    let mut errors = Vec::new();
    for claim in CLAIMS {
        for check in claim.checks {
            match index.section(claim.op, check.site) {
                Err(err) => errors.push(err),
                Ok(section) => {
                    for token in check.tokens {
                        if !section.contains(token) {
                            errors.push(format!(
                                "{}: the `# Complexity` section at {:?} lost its pinned \
                                 token {token}",
                                claim.op, check.site
                            ));
                        }
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
            // board module doc's coverage list; hold it non-empty.
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
/// - **tokens**: a claim citing a class pins the class's defining
///   token, and an exclusive token appears only on claims citing its
///   class, so the prose and the class cannot drift apart.
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
    for claim in CLAIMS {
        let pinned: Vec<&str> = claim
            .checks
            .iter()
            .flat_map(|check| check.tokens.iter().copied())
            .collect();
        let cited: Vec<Class> = match &claim.cells {
            Cells::Board(cells) => cells.iter().map(|(_, class)| *class).collect(),
            Cells::Uncelled(_) => Vec::new(),
        };
        for class in Class::ALL.iter().copied() {
            let contract = class.contract();
            let Some(token) = contract.token else {
                continue;
            };
            if cited.contains(&class) {
                if !pinned.iter().any(|t| t.contains(token)) {
                    problems.push(format!(
                        "{}: cites a {class:?} cell but pins no token containing `{token}`",
                        claim.op
                    ));
                }
            } else if contract.token_exclusive && pinned.iter().any(|t| t.contains(token)) {
                problems.push(format!(
                    "{}: pins a token containing `{token}` without citing a {class:?} cell: \
                     the token is that class's alone",
                    claim.op
                ));
            }
        }
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
    let version = crate::meter::wide_tail(s, s).version();
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
/// 25 114 ops, measured 2026-07-27 at this shape and scale). The floor
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

/// Packed-stream scan work of one `Version::join_all` over the scatter
/// fold population (the board's shape: `n` balanced-forked single-tick
/// versions, evens before odds).
#[cfg(feature = "scan-meter")]
fn fold_scan_bits(n: usize) -> u64 {
    use crate::{Party, Version};
    let mut parties = vec![Party::seed()];
    while parties.len() < n {
        let mut next = Vec::with_capacity(parties.len() * 2);
        for mut p in parties {
            let q = p.fork();
            next.push(p);
            next.push(q);
        }
        parties = next;
    }
    let versions: Vec<Version> = parties
        .iter()
        .map(|p| {
            let mut v = Version::new();
            v.tick(p);
            v
        })
        .collect();
    let (evens, odds): (Vec<_>, Vec<_>) = versions
        .into_iter()
        .enumerate()
        .partition(|(i, _)| i % 2 == 0);
    let scattered: Vec<Version> = evens.into_iter().chain(odds).map(|(_, v)| v).collect();
    crate::meter::reset_scan_bits();
    std::hint::black_box(Version::join_all(scattered));
    crate::meter::scan_bits()
}

/// The n-ary fold's log factor is alive.
///
/// `Version::join_all`'s scan work on the scatter population grows faster
/// than its input across a x4 population growth — the balanced
/// reduction's `O(D log k)`, which is what the fold operations'
/// `# Complexity` sections and the claims roster's `FoldLog` class
/// document. If an n-cursor merge (or any linear fold) lands, this pin
/// reads red, and the rustdoc, the claims roster, and this floor must
/// move in one change.
///
/// Deterministic counter, dev profile; a linear fold would read ~4.0
/// across the x4 growth, `D log k` predicts `4 x log(4n)/log(n)` (5.0 at
/// n = 256), and the current reduction reads x5.16 (51 354 -> 264 730
/// bits, measured 2026-07-27). The floor sits midway between linear and
/// measured.
#[cfg(feature = "scan-meter")]
#[test]
fn fold_log_factor_is_alive() {
    /// Halfway between linear growth (~4.0x) and the measured x5.16.
    const MIN_GROWTH: f64 = 4.6;
    let (lo, hi) = (fold_scan_bits(256), fold_scan_bits(1024));
    let growth = hi as f64 / lo.max(1) as f64;
    assert!(
        growth >= MIN_GROWTH,
        "join_all's scan work grew only x{growth:.2} across a x4 population \
         growth ({lo} -> {hi} bits; the log factor reads >= x{MIN_GROWTH}, a \
         linear fold ~x4.0): the documented `O(D log k)` overstates, so \
         update the fold `# Complexity` sections, the claims roster, and \
         this pin together"
    );
}

/// The `MulBound` claims' answer-embedded product is alive.
///
/// The plateau-puncture rank equals the closed form `H · M + 1` at
/// scale `2^(33d)`, with both factors' widths scaling linearly in the
/// family parameters — the value structure behind the `Ω(M(|v|))`
/// floor the class carries: the exact answer is a wide × dense integer
/// product whose factors the input funds separately, so no fold that
/// answers exactly goes below one multiplication. A representation
/// change that stops the answer embedding the product reads red here,
/// and the rustdoc, the roster class, and this pin move in one change;
/// the cost legs (flat traffic, schoolbook red) live in the witness
/// tests the name-binding test above pins.
#[cfg(feature = "meter")]
#[test]
fn mul_bound_embedding_is_alive() {
    use dashu_int::ops::BitTest;
    use dashu_int::UBig;
    let (w, d) = (64usize, 48usize);
    let v = crate::meter::plateau_puncture(w, d).version();
    let h = UBig::ONE << (32 * w);
    let m: UBig = (1..=d).map(|i| UBig::ONE << (33 * i - 1)).sum();
    assert_eq!(
        (h.bit_len(), m.bit_len()),
        (32 * w + 1, 33 * d),
        "both factors must scale with the family parameters: a degenerate \
         factor would make the embedded product one-sided"
    );
    assert_eq!(
        v.rank().to_string(),
        format!("{}/2^{}", &h * &m + 1u8, 33 * d),
        "the plateau-puncture rank must be the plateau times the punctured \
         turn mass: the answer no longer embeds the product, so the MulBound \
         class and the Ω(M(·)) floor lost their witness"
    );
}

/// The tripwire the roster's own vocabulary rests on: a doc block whose
/// `# Complexity` section is missing, or whose section lost a pinned
/// token, is detected — the scanner is not vacuously green.
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

/// The operations [`board::BOARD_EXPECTED_REDS`] holds red on an
/// exponent mechanism at either acceptance scale.
fn exponent_red_ops() -> BTreeSet<&'static str> {
    board::BOARD_EXPECTED_REDS
        .iter()
        .filter(|red| red.exponent)
        .map(|red| red.op)
        .collect()
}

/// Every `#[test]`-attributed function name in a source file.
///
/// The witness scanner behind the class contracts: attribute-gated
/// exactly as the board↔band parity pin's, so a prose mention of a
/// deleted test never counts as its existence, and cfg attributes
/// between `#[test]` and the fn keep the arming.
fn test_fns(source: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut armed = false;
    for line in source.lines() {
        let t = line.trim();
        if t == "#[test]" {
            armed = true;
            continue;
        }
        if t.starts_with("#[") || t.is_empty() {
            continue;
        }
        if armed {
            if let Some(rest) = t.strip_prefix("fn ") {
                if let Some(name) = rest.split('(').next() {
                    names.insert(name.to_string());
                }
            }
            armed = false;
        }
    }
    names
}

/// One cell's exponent-red stance verdict, from the class's contract:
/// the contradiction message, if the cited class and the board's
/// mechanism-tagged red set disagree.
///
/// The class-binding seal's kernel, enforced uniformly inside
/// `classes_satisfy_their_contracts`. The bench judge's red set binds
/// only wall time, and the `version_min_ticks` time legs sit under the
/// judge's resolution at bench scales — so before this seal, a
/// counter-superlinear kernel could keep a `Linear` rustdoc claim with
/// every gate green: at `395f0e72` the min_ticks claim read
/// `Class::Linear` while its pure-comb, reveal-comb, and ascend-cliff
/// board cells read touch/limb exponents 1.58–1.98 on the release
/// boards of record — verified by mutation before the seal landed. The
/// `Required` stance is the reverse leg: a class whose whole evidence
/// is a standing exponent red is decoration without one, so the cure
/// that flips the board pins must move the class in the same change
/// (the fixture below keeps that leg firing).
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

/// The `Ω(M(·))` floor's committed witness family embeds no hard
/// product: the plateau-puncture numerator is one shift and one
/// increment away from the turn mass, because the plateau is
/// `H = 2^(32w)` — the parked factor `H − 1` is an all-ones run the
/// settle's own balanced-digit spelling compacts to two signed
/// digits.
///
/// A witness for review, not a ratification. This test computes the
/// exact rank of `PP(w, d)` with **no multiplication anywhere in its
/// own arithmetic** — `(M << 32w) + 1` — and matches the shipped fold
/// exactly. A fold that compacts its *factor* side the way
/// `charge_digits` already compacts its digits side answers this
/// family in `O(|v|)` digit work, so the family cannot mandate
/// `Ω(M(|v|))` for "any fold that answers exactly": the input funds
/// the factor's *width* but not its *content* (~zero entropy in
/// `H − 1` beyond the width itself). The neighbouring
/// [`mul_bound_embedding_is_alive`] pin guards both factors'
/// `bit_len` — width, which this degenerate factor satisfies — so the
/// pin blesses a family on which the floor is beatable. The floor
/// clause needs a plateau whose climb carries dense content (varied
/// funded increments summing to an incompressible `H'`), making the
/// numerator a genuine two-sided multiplication instance; when such a
/// family lands, re-point the embedding pin at it and retire this
/// witness with the change.
#[test]
fn plateau_puncture_numerator_is_computable_by_shift_alone() {
    use dashu_int::UBig;
    let (w, d) = (64usize, 48usize);
    let v = crate::meter::plateau_puncture(w, d).version();
    // The turn mass, then the numerator by shift and increment only:
    // no product is formed anywhere in this test.
    let m: UBig = (1..=d).map(|i| UBig::ONE << (33 * i - 1)).sum();
    let numerator = (m << (32 * w)) + 1u8;
    assert_eq!(
        v.rank().to_string(),
        format!("{numerator}/2^{}", 33 * d),
        "the plateau-puncture rank is its turn mass shifted by the plateau's \
         width, plus one: if this stops holding, the family changed and this \
         witness (and the floor claim it disputes) must be re-derived"
    );
}

/// The token-exclusivity leg reads only the roster's *pinned* tokens,
/// never the rustdoc section itself: a claim downgraded to `Linear`
/// that also drops `Ω(M(` from its pinned list passes every contract
/// check while the live `# Complexity` section still carries the
/// MulBound class's exclusive token.
///
/// A witness for review, not a ratification: this fixture is the
/// cheapest artifact the current criteria bless — `Version::rank`
/// re-cited as `Linear` (its counters *are* flat, so no exponent-red
/// stance objects; `version_rank` is not judge-rostered) with the
/// pinned tokens trimmed to the space claim alone. The exclusivity
/// predicate replicated below (verbatim from
/// [`classes_satisfy_their_contracts`]) raises nothing, and the last
/// assertion proves the gap is live against the real rustdoc: the
/// scanned section still contains `Ω(M(`, so roster and prose have
/// drifted apart with every gate green. Closing the hole means
/// scanning the *section text* for exclusive tokens (not just the
/// pinned list); when that lands, this witness flips and retires.
#[test]
fn a_downgraded_mul_bound_claim_slips_the_token_exclusivity_leg() {
    let fixture = Claim {
        op: "Version::rank",
        checks: &[super::Check {
            site: super::Site::Fn,
            tokens: &["`O(|v|)` space"],
        }],
        cells: Cells::Board(&[("version_rank", Class::Linear)]),
    };
    let pinned: Vec<&str> = fixture
        .checks
        .iter()
        .flat_map(|check| check.tokens.iter().copied())
        .collect();
    let cited: Vec<Class> = match &fixture.cells {
        Cells::Board(cells) => cells.iter().map(|(_, class)| *class).collect(),
        Cells::Uncelled(_) => Vec::new(),
    };
    // The exclusivity predicate, replicated verbatim from
    // classes_satisfy_their_contracts.
    let mut problems: Vec<String> = Vec::new();
    for class in Class::ALL.iter().copied() {
        let contract = class.contract();
        let Some(token) = contract.token else {
            continue;
        };
        if cited.contains(&class) {
            if !pinned.iter().any(|t| t.contains(token)) {
                problems.push(format!("cites {class:?} without its token"));
            }
        } else if contract.token_exclusive && pinned.iter().any(|t| t.contains(token)) {
            problems.push(format!("pins {class:?}'s exclusive token uncited"));
        }
    }
    // The stance and judge legs bless it too: flat counters, no red.
    let exponent_red_ops = exponent_red_ops();
    let Cells::Board(cells) = &fixture.cells else {
        unreachable!("the fixture cites a board cell");
    };
    let stance = stance_contradiction(fixture.op, cells[0].0, cells[0].1, &exponent_red_ops);
    assert!(
        problems.is_empty() && stance.is_none(),
        "the downgraded claim was flagged ({problems:?}, {stance:?}): the \
         exclusivity gap has been closed — retire this witness with the fix"
    );
    // The gap is live: the real rustdoc section still carries the
    // exclusive token the fixture's pinned list dropped.
    let index = doc_index();
    let section = index
        .section("Version::rank", super::Site::Fn)
        .expect("Version::rank has a Complexity section");
    assert!(
        section.contains("Ω(M("),
        "Version::rank's Complexity section no longer carries `Ω(M(`: \
         re-point this witness at a live MulBound section"
    );
}

/// The expected-red roster's own hygiene: every entry names a live
/// board cell exactly once and carries at least one mechanism.
///
/// Every bench rider must also be a rostered red — a rider exists to
/// keep a standing red's time leg judged, so an unrostered rider is a
/// stale census.
#[test]
fn expected_red_roster_names_live_cells() {
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
            seen.insert((red.op, red.family)),
            "{}/{} appears twice in BOARD_EXPECTED_REDS",
            red.op,
            red.family
        );
    }
    for (op, family) in board::BOARD_RED_BENCH_RIDERS {
        assert!(
            board::BOARD_EXPECTED_REDS
                .iter()
                .any(|red| red.op == *op && red.family == *family),
            "rider {op}/{family} is not a rostered standing red: re-realize the census"
        );
    }
}
