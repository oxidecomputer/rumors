# Partition fuzzfit-bands: The fuzz-fit harness: pinned fuel bands, curve fitting, wasm bridge, calibrate/probe/diag binaries, and the enforcement tests

## Partition summary

This partition is the fuel-band instrument for `before`: `bands.rs` holds the committed per-kernel log-log cost laws (49 `Band` constants keyed by kernel and outcome, four constant-classified `SMALL_BANDS` for rumors' bootstrap-scale operands, the `REFIT_COVERAGE` expectation list, `PINNED_RUSTC`, and the four judgment constants `ENFORCE_MARGIN`, `ENFORCE_MARGIN_BELOW`, `REFIT_PREFIX_PROGRAMS`, `REFIT_TOLERANCE`); `fit.rs` fits bucket-median log-log lines with two one-sided residual widths and provides `line_divergence`, the staleness comparator; `curve.rs` is the within-case shape leg (`local_slope_excess` against `SLOPE_ALLOWANCE`, with the two fold kernels in `SHAPE_EXEMPT`); `wasm.rs` drives wasmtime with a pooled instance allocator, a pre-reserved register file, and per-call fuel measurement; `bin/calibrate` sweeps the deterministic corpus, rewrites the generated tail of `bands.rs` below a prose marker, and prints the judgment constants' evidence to stderr; `bin/probe` and `bin/diag` are hand-run tools; `tests/enforce.rs` is the gate leg (the 48-case random sentry, the 256-program deterministic prefix judged total and refitted, the bootstrap replay, two fixed escalation replays, the nop liveness check, the quadratic-burner adequacy check, and the toolchain pin); `tests/sanity.rs` checks generator invariants natively plus the roster parity and the judge's own tripwire. I read every partition file in full (3135 lines) and, for context, `drive.rs`, `build.rs`, the cited regions of `ops.rs`, `strategies.rs`, and the guest, the justfile recipes, the design note, `validation_index.rs`, and the proptest 1.11.0 and wasmtime 47.0.4 sources in the cargo registry. Test files: `src/fit/tests.rs`, `src/curve/tests.rs`, `tests/enforce.rs`, `tests/sanity.rs`, `tests/main.rs`.

The instrument is well designed at the level the doctrine cares most about. Every judgment leg has a committed synthetic tripwire that states what it does and does not prove; the ceiling and floor widths are priced separately with an argument for why (the one-sidedly heavy residual cloud); the liveness margin names both measured edges it sits between; the register-file reservation is tied to both budgets by a `const` assertion, so a budget raise past it is a compile error rather than a reallocation inside a measured window; the staleness cross-check walks a committed coverage roster where a classification flip fails by name; and the deterministic prefix turns a sampled verdict into a total one with the `(1 - q)^48` argument written where the leg lives. The blessed-drift window section in `bands.rs` argues against its own instrument candidly, which is rare and valuable. The vocabulary is mostly anchored (band, arm, leg, reach family, liveness and regression flag are each tied to an identifier or defined by contrast).

The dominant issue is a single mechanism with several faces: the judgment constants are justified by measurements that `bin/calibrate` computes and prints to stderr, and the tree persists those measurements only as hand-transcribed prose in the head of `bands.rs` that the generator preserves verbatim. The 2026-08-04 toolchain re-pin rewrote every constant and no prose, so the head now states small-band numbers the generated region contradicts, a floor gap that recomputes differently, a rejection-envelope range one arm sits outside, and a shape-leg maximum that three documents give three values for. The same stderr-only design means the floors' liveness claim (every pinned floor clears the nop reading) has no committed test, and calibrate's evidence loop evaluates it at one endpoint over the main bands only. Two further mediums are criterion questions for the owner: the shape leg subtracts the pooled slope, which the pin's own prose says is mixture-inflated on twelve non-exempt keys, so a `d^1.4` mechanism on `ff_version_decode` passes both legs by arithmetic over the committed constants; and the staleness leg judges an exactly reproducible per-key quantity against one global tolerance, which is why the design note has to accept a 2x uniform meter undercount. The determinism premise everything replays on has no committed two-execution comparison (the only one is the `probe` binary no recipe runs, written before the pooled allocator landed), and the enforcement sentry's doc says the bands price "every public operation" while the guest exports some sixty measured kernels no band prices and no tiling accounts for. The rest is duplication (bucket medians, the deterministic stream loop, the `Fit`-to-`Band` transcription, a second guest call path), stale or hand-maintained prose, and idiom nits.

## Findings

### fuzzfit-bands-1: "honest" as a moral adjective and "sentry" as an unanchored coinage
- Where: crates/before/fuzzfit/harness/src/bands.rs:66-70 (related: crates/before/fuzzfit/harness/tests/enforce.rs:1, crates/before/fuzzfit/harness/tests/enforce.rs:35-36, crates/before/fuzzfit/harness/src/fit/tests.rs:38, crates/before/fuzzfit/harness/tests/sanity.rs:54; 19 "honest" sites and 13 "sentry" sites across the partition)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (grep -i over harness/src and harness/tests: 19 and 13 hits; no identifier named `sentry` exists); executed: no
- Seen by: structure-prose; refutation: confirmed; history: no rationale found
- Owner-gated: no

"honest" stands in for a mechanism at every site (pre-drift divergence, in-band work, the legitimately cheap tail, canonical bytes), and "sentry" names the random-draw proptest leg thirteen times without being an identifier or being defined once by contrast: the test is `fuel_stays_in_the_pinned_bands` and the file is `enforce.rs`. The vocabulary rule asks that a coined term anchor to an identifier or be defined once; moralized code is a default-dialect tell.

Evidence:

        66	//! key under ~×2.5. Drift inside the window surfaces only at the next
        67	//! deliberate re-pin, as diff the re-pinner must annotate; the
        68	//! mechanism is unchanged by stating this — the window is the honest
        69	//! price of margins that must also absorb allocator variance and honest
        70	//! sampling dispersion.

        35	//! Case count: 48 by default (the calibration corpus is the big sweep; this
        36	//! is the sentry); override with `PROPTEST_CASES`.

Resolution: Replace each "honest" with the mechanism it names ("pre-drift divergence", "in-band work", "the tail of legitimately cheap draws", "canonical bytes"). Either give the sentry a name in code (rename the test `sentry_fuel_stays_in_the_pinned_bands`, or define the term once in the `enforce.rs` module doc's first paragraph) or write "the random-draw leg". Acceptance: `grep -rni honest harness/src harness/tests` is empty; "sentry" appears either as part of an identifier or not at all.

### fuzzfit-bands-2: Pin-time measurements hand-transcribed into prose have rotted; the judgment constants' evidence is printed, never committed or asserted
- Where: crates/before/fuzzfit/harness/src/bands.rs:84-89 (related: bands.rs:74-79, bands.rs:98-100, bands.rs:106-107, bands.rs:116, bands.rs:161-173, bands.rs:175-194, bands.rs:207-218, crates/before/fuzzfit/harness/src/curve.rs:61-71, crates/before/fuzzfit/harness/src/bin/calibrate.rs:218-343, crates/before/fuzzfit/harness/src/bin/calibrate.rs:345-353)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (`git show e7a4b7b0 -- bands.rs`: every hunk sits at line 318 or below and the decode small band moves 3.684848 to 3.705965; `git blame` dates bands.rs:84-89 to d2a9d04e 2026-07-31 and bands.rs:100 to 875c118b 2026-07-27; Python over the parsed constants recomputes the ff_rank_cmp floor gap as 0.1155 at HEAD with nop = 2 and 0.1134 at the d2a9d04e constants); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed; history: deliberate-and-holds for the current-pin head design (d2a9d04e), with the truing discipline lapsed at e7a4b7b0
- Owner-gated: no

`bin/calibrate` rewrites `bands.rs` from the `/// The toolchain that pinned` marker down and preserves everything above it verbatim, while printing the judgment constants' evidence only to stderr (calibrate.rs:218). The head therefore carries numbers the generator cannot refresh, and they are stale: the decode small band's transcription contradicts `SMALL_BANDS`; the rejection-envelope range 1.36–1.42 excludes `ff_clock_sync`'s pinned rejection slope 1.328458 (and did at the prior pin too, 1.328893); `party_without`'s success slope reads 0.72 in prose and 0.713628 in the pin; the shape-leg maximum healthy excess is +0.081 in bands.rs, +0.013 in curve.rs, and +0.006 in the design note; the corpus size is ~2.64M steps here and ~2.62M in curve.rs; the "narrowest gap is 0.113 decades" at bands.rs:181 recomputes to 0.1155 from the committed `ff_rank_cmp` constants. Principle 5 (prose states what is; no hand-maintained restatements of facts the code can change) and Principle 2 (a quantity computable two ways gets a committed comparison): the four judgment constants each justify their value by a number that exists only on stderr and in a hand copy, and nothing in the gate fails when copy and measurement diverge.

Evidence:

        84	//! [`SMALL_BANDS`] carries one constant-classified band per bootstrap-hot
        85	//! kernel, calibrated from the main corpus's own sub-floor samples pooled
        86	//! with the deterministic bootstrap stream (tick level 4.476 +0.782/−0.492
        87	//! over 10..127 bits, join 4.454 +0.395/−0.765 over 20..127, encode
        88	//! 2.729 +0.304/−0.139 over 10..127, decode 3.685 +0.683/−0.308 over
        89	//! 16..120).

    versus the generated decode small band:

       966	        slope: 0.000000,
       967	        intercept: 3.705965,
       968	        width_above: 0.666431,
       969	        width_below: 0.330484,

    the shape-leg maximum, three ways (bands.rs:99-100, curve.rs:67-68):

        99	//! 128-bit fit floor — the within-case shape diagnostic's maximum healthy
       100	//! excess over the whole corpus is +0.081 (see [`crate::curve`]) — while

        67	/// evidence-bearing (band key, case) pair was +0.013 (`ff_clock_join`,
        68	/// a deep `DenseSpine` draw). The allowance sits well above that observed

    the rejection envelope claim versus ff_clock_sync's pinned rejection arm (bands.rs:106, 475):

       106	//! ground-truth view). The rejection envelopes (1.36–1.42:

       475	        slope: 1.328458,

    and calibrate's own statement of the design:

       218	    // ── judgment-constant evidence (stderr; never part of the pin file) ──

Resolution: Have `calibrate` emit the evidence it already computes into the generated region as one constant, e.g. `pub const PIN_EVIDENCE: PinEvidence` with fields `corpus_programs`, `corpus_steps`, `nop_fuel`, `shape_max_excess` (key, case), `refit_max_divergence` (key), `floor_min_gap` (key), `replay_ceiling_excess` (key, depth), and the uncovered keys with their reasons. Make the docs of `ENFORCE_MARGIN`, `ENFORCE_MARGIN_BELOW`, `REFIT_TOLERANCE`, and `SLOPE_ALLOWANCE` cite those fields by name instead of by number, and delete the numeric narrative from the head (the small-band numbers duplicate `SMALL_BANDS` outright; the slope-by-slope commentary at bands.rs:98-125 belongs in the re-pin commit, as bands.rs:127 itself says). Add one enforcement test asserting the orderings the docs claim: `ENFORCE_MARGIN > PIN_EVIDENCE.replay_ceiling_excess`, `PIN_EVIDENCE.floor_min_gap > 0.0`, `SLOPE_ALLOWANCE > PIN_EVIDENCE.shape_max_excess`, `REFIT_TOLERANCE > PIN_EVIDENCE.refit_max_divergence`. Acceptance: after `just fuzzfit-calibrate` at HEAD, no decimal literal above the splice marker duplicates a value the generated region or calibrate's stderr carries; the ordering test exists and passes; grep finds one shape-leg maximum in the tree, generated.

### fuzzfit-bands-3: `Band.constant`'s doc invites the converse reading; three pinned bands have slope 0 and `constant: false`
- Where: crates/before/fuzzfit/harness/src/bands.rs:157-158 (related: bands.rs:364-375, bands.rs:376-387, bands.rs:496-507, crates/before/fuzzfit/harness/src/fit.rs:66-68, crates/before/fuzzfit/harness/tests/enforce.rs:381-387)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (read the three Band literals with `slope: 0.000000` and `constant: false`); executed: no
- Seen by: structure-prose; refutation: reframed (accurate as written, but invites the converse); history: no rationale found
- Owner-gated: no

`constant` means constant-classified by the span and bucket thresholds in `fit`, and the classification-flip check in `enforce.rs` depends on that meaning; the parenthetical "slope pinned at 0" reads as if a zero slope implied the classification, which `ff_clock_from_parts`, `ff_clock_into_parts`, and `ff_clock_version` disprove. Maintainer docs should state what the type cannot show.

Evidence:

       157	    /// Whether the band was constant-classified (slope pinned at 0).
       158	    pub constant: bool,

       367	        slope: 0.000000,
       368	        intercept: 2.424882,
       ...
       374	        constant: false,

Resolution: "Whether the band was constant-classified: too little denominator span or too few buckets for a slope estimate (see `fit::fit`), so the slope is 0 by rule. A fitted slope of exactly 0 on a size-law band is not this." Acceptance: the doc names the rule, agreeing with `Fit.constant`'s doc at fit.rs:66-68.

### fuzzfit-bands-4: The deterministic prefix leg pays the out-of-corpus ceiling margin it does not need
- Where: crates/before/fuzzfit/harness/src/bands.rs:161-173 (related: bands.rs:42-70, crates/before/fuzzfit/harness/tests/enforce.rs:53-56, tests/enforce.rs:355-366, tests/enforce.rs:221-240, crates/before/fuzzfit/harness/src/bin/calibrate.rs:80-131)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: assessed (read); executed: no
- Seen by: adequacy; refutation: reframed (the bootstrap leg's main-band judgments are out-of-corpus and legitimately need the margin; the prefix leg and the small-band portion of the bootstrap leg do not); history: already-known (d17a8018 documents the window as an accepted property; a separate deterministic-leg margin was not considered)
- Owner-gated: yes: it narrows the blessed-drift window and therefore raises re-pin frequency, a cadence trade the owner rules on

`ENFORCE_MARGIN` is justified as absorbing variance between calibration and enforcement contexts, yet `judge` applies it identically to the deterministic prefix, whose programs are the fitted corpus itself (residual at most `width_above` by construction, modulo six-decimal rounding of the printed constants). The documented x1.6 window on most keys is therefore set by a margin whose rationale does not apply on that leg. Principle 6: name the worst passing artifact and close the path.

Evidence:

       161	/// Slack beyond each band's fitted ceiling, in `log₁₀` units.
       162	///
       163	/// Absorbs allocator-history variance between calibration and
       164	/// enforcement contexts (≈ 1.6× in fuel). Measured at pin time
       165	/// (`bin/calibrate` replays the enforcement suite's fixed escalation
       166	/// programs — deterministic enforcement-context executions outside the
       167	/// calibration corpus — and re-derives their worst ceiling excess on
       168	/// every re-pin): the observed maximum is +0.024 decades

Resolution: Thread the margins through `judge` (or add `judge_against_with(band, d, fuel, margin_above, margin_below)`), pass `ENFORCE_MARGIN` for the sentry, the escalation replays, and the bootstrap leg's main-band judgments, and a documented `DETERMINISTIC_MARGIN` (sized for the printed constants' rounding, or a deliberate x1.1) for the prefix leg and the small-band judgments; state the two windows separately in the blessed-drift section. Acceptance: the blessed-drift doc states two windows; a committed synthetic case shows a uniform x1.5 drift on one key reads `Above` on the prefix while still inside the sentry's margin; the gate stays green at the current pin.
Construction: Multiply every `ff_clock_tick` fuel by 1.5 (a uniform +0.176 decade drift). Prefix leg: the argmax sample's residual becomes `width_above + 0.176 < width_above + 0.2`, `InBand`; refit divergence 0.176 < 0.7; the shape leg is deaf to intercept shifts. The suite stays green, exactly as bands.rs:61 documents. With a 0.05 deterministic margin the same drift reads `Above` on that sample every run.

### fuzzfit-bands-5: `REFIT_TOLERANCE` is one global 0.7 decades where the per-key evidence is deterministic and exact
- Where: crates/before/fuzzfit/harness/src/bands.rs:207-218 (related: crates/before/fuzzfit/harness/tests/enforce.rs:367-395, bands.rs:42-70, bands.rs:989-1038, crates/before/fuzzfit/harness/src/bin/calibrate.rs:238-272, crates/before/fuzzfit/harness/src/drive.rs:101-108)
- Class / severity / confidence: verification-gap / medium / medium
- Provenance: assessed (read); executed: no
- Seen by: scaffolding; refutation: confirmed; history: already-known (the design note's 2026-07-26 ruling accepts the sub-3x uniform meter-degradation residual "without new machinery", rejecting a dedicated meter-calibration instrument; the per-key proposal here is not that instrument and is not addressed by the ruling)
- Owner-gated: yes: it reopens a recorded ruling and changes re-pin cadence

The staleness check compares an exactly reproducible per-key quantity (the prefix refit's line against the pin; drive.rs:101-108 documents the stream as byte-identical across consumers) against one tolerance sized by the worst key's pin-time sampling gap. It measures `|d|` against 0.7 rather than `|d - d0|` against a slack, so on most keys it is deaf to roughly 4.5x drift and on every key to a 2x uniform undercount (the "k = 2 hides" the design note records). Per-key pinning is the same comparison with a generated constant per key, and `REFIT_COVERAGE` is already the per-key list those values would ride on; `calibrate` already computes `d` per key at calibrate.rs:244-255. Principle 2: a threshold over a stable quantity belongs in a measured gap.

Evidence:

       210	/// Measured at pin time (`bin/calibrate` re-derives the
       211	/// evidence on every re-pin; the 4096-program corpus): the prefix's own
       212	/// sampling difference from the full corpus peaks at 0.497
       213	/// (`ff_party_join`'s rejection arm, the thinnest-sampled band key
       214	/// genre). The tolerance sits above that, so the
       215	/// check is deaf to sub-half-decade drift on the worst key but fails
       216	/// loud on anything past ~5x in fuel constants — a staleness detector,
       217	/// not the criterion of record.
       218	pub const REFIT_TOLERANCE: f64 = 0.7;

Resolution: Generate `REFIT_COVERAGE` as `(kernel, rejected, divergence_at_min, divergence_at_max)` carrying the signed pin-time endpoint deltas; in the enforcement test compute the fresh deltas and assert `|fresh - pinned| <= REFIT_SLACK` at each endpoint, with `REFIT_SLACK` set deliberately as the cadence knob (0.2 matches `ENFORCE_MARGIN` and catches k = 2). Keep `REFIT_TOLERANCE` only if the coarse absolute bound is also wanted. Re-denominate the blessed-drift paragraph from the slack. Acceptance: a test that halves `Measured.fuel` for one kernel inside `Guest::call` reads red on the deterministic prefix by name; on unchanged code every pinned delta reproduces to floating-point noise.
Construction: In `wasm.rs` `Guest::call`, temporarily return `fuel: (FUEL_TANK - remaining) / 2` when `name == "ff_version_tick"` (log10 2 = 0.301). Point leg: every sample drops 0.301 below its line, inside `width_below + 0.8`, no `Below`. Shape leg: slopes unchanged. Staleness leg: `|d| <= 0.301 + d0 < 0.7` on every covered key. The suite stays green. With per-key pinned deltas and slack 0.2, `ff_version_tick`'s fresh delta moves by 0.301 > 0.2 and the test reads red.

### fuzzfit-bands-6: The wasmtime-bump-is-a-re-pin sentence is a convention with no mechanism, contradicted at both patch bumps
- Where: crates/before/fuzzfit/harness/src/bands.rs:312-321 (related: crates/before/fuzzfit/harness/tests/enforce.rs:319-326, crates/before/fuzzfit/harness/build.rs:1-15, crates/before/fuzzfit/Cargo.lock:1193-1194, crates/before/fuzzfit/harness/src/bin/calibrate.rs:360-369)
- Class / severity / confidence: verification-gap / low / high
- Provenance: verified (`git show --stat 4e64a4fb -- crates/before/fuzzfit` lists only Cargo.lock; `git show --stat 8490af3f` lists only the two detached Cargo.lock files; `git log -- bands.rs` ends at e7a4b7b0 2026-08-04; Cargo.lock at HEAD names wasmtime 47.0.4; enforce.rs asserts only `PINNED_RUSTC`); executed: no
- Seen by: scaffolding, structure-prose; refutation: confirmed; history: deliberate-and-holds for the rustc-constant/lockfile asymmetry (e7a4b7b0's rationale); the "likewise a re-pin event" sentence (c1fe9388) has no mechanism and was skipped at 8490af3f and 4e64a4fb, both with green gates
- Owner-gated: no

`PINNED_RUSTC` is enforced by `building_toolchain_matches_the_pin`; the doc's parallel claim about wasmtime is enforced by nothing: no constant records the pinning wasmtime and no test compares it. Two lockfile bumps (47.0.2 to 47.0.3, 47.0.3 to 47.0.4) landed without a re-pin, and the staleness leg would notice a fuel-schedule change only past `REFIT_TOLERANCE` (about 5x). Principle 6: every hole found becomes a committed check, never a convention held in memory; or the sentence states the softer guarantee that actually holds.

Evidence:

       315	/// Generated by `just fuzzfit-calibrate` alongside [`BANDS`]: guest
       316	/// codegen (and so every fuel constant) is a function of this compiler,
       317	/// and the suite asserts the building toolchain matches, so a toolchain
       318	/// bump reads red until the bands are re-pinned. wasmtime (the fuel
       319	/// schedule's other half) is pinned exactly by the workspace
       320	/// `Cargo.lock`; bumping it there is likewise a re-pin event.
       321	pub const PINNED_RUSTC: &str = "rustc 1.97.1 (8bab26f4f 2026-07-14)";

Resolution: Either enforce it (build.rs parses the workspace Cargo.lock's `wasmtime` version into `FUZZFIT_WASMTIME_VERSION`; calibrate emits `PINNED_WASMTIME`; a sibling of `building_toolchain_matches_the_pin` asserts equality) or soften the sentence to what holds ("a wasmtime bump that moves the fuel schedule reads red through the staleness leg past `REFIT_TOLERANCE`; a bump that leaves the schedule alone stays green and is not a re-pin event"). Acceptance: a lockfile wasmtime bump without re-pin turns `just fuzzfit` red by name, or the sentence names the mechanism (`REFIT_TOLERANCE`) that actually bounds it.
Construction: The two historical bumps are the demonstration: `git show --stat 8490af3f` and `git show --stat 4e64a4fb` touch only lockfiles, `bands.rs` is unchanged since e7a4b7b0, and 4e64a4fb's message records a clean gate; `grep -n wasmtime tests/enforce.rs` returns nothing.

### fuzzfit-bands-7: `fit()`'s floor fallback contradicts `FIT_FLOOR_BITS`'s doc, and "classifies constant" is not guaranteed
- Where: crates/before/fuzzfit/harness/src/fit.rs:71-73 (related: fit.rs:104-107, fit.rs:114-118, fit.rs:149, crates/before/fuzzfit/harness/tests/enforce.rs:367-395)
- Class / severity / confidence: correctness / low / high
- Provenance: verified (Python over the rule: 8..127 bits spans 1.201 decades and four half-decade buckets, so the constant test at fit.rs:149 is false for an all-sub-floor set; with one floored sample the corner is the same); executed: no
- Seen by: instrument-correctness; refutation: confirmed, with the fit.rs:106-107 over-claim added; history: deliberate-and-holds for the two-sample fallback (b1403c59 restated `fit()`'s doc to the actual guard); the stale half is `FIT_FLOOR_BITS`'s doc (b1c7d31f), which b1403c59 did not touch
- Owner-gated: no

`FIT_FLOOR_BITS`'s doc promises `Fit::min_denom` never sits below the floor when floored samples exist, but `fit` uses the floored subset only when it has at least two members; with exactly one floored sample the whole set is fitted and `min_denom` lands below 128. `fit()`'s own doc says a kernel sampled only below the floor "classifies constant", but sub-floor samples spanning 8..127 bits cover 1.2 decades and four buckets, so such a set fits a size-law slope through the constant-overhead regime the floor exists to exclude. Principle 1 (a contract clause breached is a finding regardless of the input's likelihood) and Principle 5 (prose states what is). No committed band exhibits the corner today; the refit leg runs `fit` on 256-program prefixes where a thinly sampled key could.

Evidence:

        71	/// The fit floor: samples below this denominator are excluded from both
        72	/// the fit and the committed judgment range (`Fit::min_denom` never sits
        73	/// below it when floored samples exist).

       104	/// Samples below [`FIT_FLOOR_BITS`] are dropped first when at least two
       105	/// samples remain above the floor (a slope needs two points); otherwise
       106	/// the full sample set is fitted — a kernel sampled only below the floor
       107	/// fits over what it has and classifies constant.

       114	    let samples: &[(u64, u64)] = if floored.len() >= 2 {
       115	        &floored
       116	    } else {
       117	        samples
       118	    };

Resolution: Either state the real rule at both docs ("when at least two floored samples exist"; drop "and classifies constant") or make the fallback classify constant by rule (the sub-floor law of record, as `fit_constant` does), so a size-law slope is never fitted through sub-floor points. Acceptance: a unit test in `fit/tests.rs` with sub-floor samples over 8..127 bits plus one at 128 either yields `constant == true` or the docs name the two-sample condition; the test's doc comment states which.
Construction: `samples = [(8,f),(12,f),(16,f),(24,f),(32,f),(48,f),(64,f),(96,f),(100,f),(110,f),(120,f),(127,f),(128,f)]` with `f` constant: `floored.len() == 1`, so all thirteen are fitted; `decades = log10(128/8) = 1.204 >= 1.0`; populated buckets {1, 2, 3, 4} >= 3; `constant` is false and `min_denom` is 8.

### fuzzfit-bands-8: `fit` and `fit_constant` have no direct tests; the fitter's headline claims are untested families
- Where: crates/before/fuzzfit/harness/src/fit/tests.rs:26-35 (related: crates/before/fuzzfit/harness/src/fit.rs:25-35, fit.rs:102-179, fit.rs:194-210)
- Class / severity / confidence: test-quality / low / high
- Provenance: assessed (read all three test files; only `line_divergence` is exercised); executed: no
- Seen by: structure-prose; refutation: confirmed (a raw-OLS fitter might trip the staleness leg on some wide key, but not by name); history: no rationale found (c1fe9388 scoped fit/tests.rs to the comparator and never widened it)
- Owner-gated: no

`fit/tests.rs` exercises only `line_divergence`; its `linear_fit()` helper fits a clean `fuel = 100 · d` corpus and never asserts the fitter recovered slope 1, intercept 2, zero widths. The module doc's claims (bucket medians keep bounded spikes out of the slope; the floor drop rule; constant classification under a decade or three buckets; `fit_constant`'s `# Panics`) are each a family with no committed demonstration. Property tests over point tests where the claim is a family; every criterion needs a committed demonstration that a known-bad mechanism fails it.

Evidence:

        26	/// A clean linear corpus to fit: fuel = 100 · d over 128..131072 bits.
        27	fn linear_fit() -> Fit {
        28	    let samples: Vec<(u64, u64)> = (7..=17)
        29	        .flat_map(|k| {
        30	            let d = 1u64 << k;
        31	            [(d, d * 100), (d + d / 2, (d + d / 2) * 100)]
        32	        })
        33	        .collect();
        34	    fit(&samples).expect("enough samples to fit")
        35	}

Resolution: Add proptests to `fit/tests.rs`: (a) a noise-free `fuel = 10^b · d^a` corpus over at least two decades recovers (a, b) within 1e-9 with both widths near 0; (b) the same corpus plus k spikes of x10 on random samples leaves the slope within 1e-6 of a while `width_above` is about 1 (raw OLS moves the slope); (c) samples spanning under a decade, or fewer than three buckets, classify constant with slope 0 and intercept the mean; (d) sub-floor samples are dropped when at least two floored remain and kept otherwise; (e) `fit_constant` returns the mean level and panics on a floored sample (`#[should_panic]`). Acceptance: the five properties committed and green; replacing the bucket medians at fit.rs:138-147 with raw OLS fails (b) by name.
Construction: Replace fit.rs:138-147 with a plain OLS over `logs` and run the harness unit tests: nothing in `fit/tests.rs` or `curve/tests.rs` fails.

### fuzzfit-bands-9: `SHAPE_EXEMPT`'s justification (the point leg catches a degenerate fold) is argued inline, never pinned
- Where: crates/before/fuzzfit/harness/src/curve.rs:38-49 (related: crates/before/fuzzfit/harness/src/bands.rs:820-831, bands.rs:856-867, crates/before/fuzzfit/harness/src/strategies.rs:89-94, strategies.rs:1096-1108)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: assessed (read); executed: no
- Seen by: adequacy; refutation: confirmed; history: deliberate-and-holds (86350975 introduced the exemption with a measured left-fold demonstration: 10.6x, +1.03 decades against a +0.35 ceiling at the ladder top, after a first attempt at `max_fold` 64 read green; the shrunk shape is a committed seed)
- Owner-gated: no

The fold kernels are carved out of the shape leg on the claim that the point leg catches a left fold's `n / log n` excess "far past the pinned ceiling inside the reachable width range". Nothing committed binds that claim to the pinned `join_all` and `meet_all` ceilings (0.355 + 0.2 and 0.293 + 0.2 decades) or to `BUDGET.max_fold` (1024), and nothing asserts the deterministic prefix contains folds wide enough for detection. The rationale is stated at the site, which is right; the premise it rests on is not mechanically held, so a re-pin widening the fold ceilings or a budget cut would silently turn the exemption into an accepted failure. Principle 3: a carve-out names what still catches the failure, mechanically.

Evidence:

        45	/// regression. The point leg owns these rows instead: the generators'
        46	/// width ladder puts a degenerate (left-fold) reduction's excess, which
        47	/// grows as n / log n, far past the pinned ceiling inside the reachable
        48	/// width range.
        49	pub const SHAPE_EXEMPT: &[&str] = &["ff_version_join_all", "ff_version_meet_all"];

Resolution: Either (a) an arithmetic tripwire in `curve/tests.rs` binding the argument to the constants: assert `log10(max_fold / (2 · log2 max_fold)) > width_above + ENFORCE_MARGIN` for both fold bands, and assert in the prefix leg that at least one `join_all`/`meet_all` step above the detection width is judged; or (b) a guest control kernel that left-folds the same registers, judged against the pinned `join_all` band and required to read `Above` at the budget width. Acceptance: a committed test fails if the fold ceilings widen or `max_fold` shrinks past the point where a left fold reads `InBand`, or if the prefix stops exercising wide folds.
Construction: With a balanced-versus-left model of `n / (2 · log2 n)`: at n = 32 the excess is about 3.2x = 0.51 decades < 0.555 (`join_all` ceiling plus margin), in band; at n = 64, 5.3x = 0.73 decades, `Above`. Whether the 256-program prefix contains a `join_all` at n >= 64 is not determinable from code (`ScatterFold` draws `clocks` in 8..=1024 and ladders at doubling widths below it).

### fuzzfit-bands-10: The shape leg's allowance is stacked on the mixture-tilted pooled slope, so the within-case threshold reaches exponent 1.4–1.55 on the worst keys
- Where: crates/before/fuzzfit/harness/src/curve.rs:61-71 (related: curve.rs:113, crates/before/fuzzfit/harness/src/bands.rs:98-106, bands.rs:595, bands.rs:607, bands.rs:751, crates/before/fuzzfit/harness/tests/enforce.rs:139-153, crates/before/fuzzfit/harness/src/bin/calibrate.rs:218-237)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (Python over the parsed constants: for the twelve non-exempt keys with pooled slope > 1.1 the escaping exponent `min(slope + 0.3, slope + (width_above + 0.2) / span_decades)` is 1.549 `ff_party_join`, 1.543 `ff_party_is_disjoint`, 1.465 `ff_version_decode`, 1.417 `ff_version_cmp`/`concurrent`, 1.628–1.709 on the rejection arms; a `d^1.4` mechanism anchored at `ff_version_decode`'s 128-bit line reaches log fuel 6.754 at 8768 bits under the ceiling 6.872 with within-case excess 0.225 < 0.3); executed: no
- Seen by: adequacy, instrument-correctness; refutation: confirmed; history: already-known in part (the design note's residual-risk section states the per-row bound and rules per-family bands "architectural" and out of scope; the note's listed worst rows are ~1.28 and understate the current pin, and changing only the shape leg's reference slope is not the architectural change ruled out)
- Owner-gated: yes: a criterion change; reopens the note's residual-risk ruling with new arithmetic

`local_slope_excess` subtracts `band.slope`, the pooled log-log fit, from a within-case local slope. The module doc of `bands.rs` states that pooled slopes above 1.1 are "family-mixture composition" that "every lane's own medians" do not follow, and `curve.rs`'s whole argument for judging within one case is that a family-pure population cannot be mixture-tilted; the leg then compares it to the mixture-tilted number anyway. The allowance's stated premise ("a third of the +1.0 a quadratic mechanism adds over a linear pin") is false for those twelve keys. Principle 6 (the cheapest passing artifact) and the crate's asymptotic contract: a sub-quadratic superlinear mechanism (`O(n sqrt n)`, a bad block size) is a real regression class this instrument exists to catch, and today `d^1.5` on `ff_party_join` passes both legs.

Evidence:

        68	/// a deep `DenseSpine` draw). The allowance sits well above that observed
        69	/// ceiling and a third of the +1.0 a quadratic mechanism adds over a
        70	/// linear pin, so the gap it lives in is wide on both sides.
        71	pub const SLOPE_ALLOWANCE: f64 = 0.3;

       113	    Some((top_y - bot_y) / (top_x - bot_x) - band.slope)

       101	//! the pooled envelope slopes above 1.1 remain the documented
       102	//! family-mixture composition (families' per-bit cost levels differ
       103	//! severalfold and the cheap families' mass sits in the small buckets,
       104	//! tilting a pooled envelope no single family follows; every lane's own
       105	//! medians are flat or falling, and `bin/diag` per family is the

       751	        slope: 1.175460,

Resolution: Judge the within-case slope against the claimed law rather than the pooled fit: for non-fold keys `excess = local - band.slope.min(LINEAR_CLAIM)` with `LINEAR_CLAIM = 1.0`, or a per-key declared exponent where a documented superlinear within-case mechanism exists (the note names `ff_rank_display`'s digits-times-limbs conversion as a candidate). Re-derive `SLOPE_ALLOWANCE` from `bin/calibrate`'s shape-leg evidence under the new reference (the code path at calibrate.rs:221-229 exists; only the reference changes), and rewrite curve.rs:64-70 to name the reference actually used. State the per-row escaping-exponent bound in the blessed-drift section so the instrument's reach is documented where its criterion lives. Acceptance: a committed `curve/tests.rs` case builds `fuel = L(128) · (d / 128)^1.4` over 128..8768 against `band_for("ff_version_decode", false)` and asserts `local_slope_excess > SLOPE_ALLOWANCE` (today it reads +0.225 and passes); calibrate's recomputed maximum healthy excess stays under the allowance on the 4096-program corpus; the suite stays green at the current pin.
Construction: `ff_version_decode`'s line at 128 bits is `1.706616 + 1.175460 · log10(128) = 4.184`. A mechanism costing `10^4.184 · (d / 128)^1.4` reaches log fuel 6.754 at `max_denom` 8768, below the ceiling `1.706616 + 1.175460 · log10(8768) + 0.330785 + 0.2 = 6.872`, so every point reads `InBand`; its within-case excess is `1.4 - 1.175460 = 0.225 < 0.3`, so the shape leg passes. At exponent 1.45 both still pass (6.845 < 6.872; 0.275 < 0.3). For `ff_party_join` the shape threshold is `1.251540 + 0.3 = 1.552`.

### fuzzfit-bands-11: Four shared computations are each spelled twice: bucket medians and their thresholds, the deterministic stream loop, and the `Fit`-to-`Band` transcription
- Where: crates/before/fuzzfit/harness/src/curve.rs:100-106 (related: crates/before/fuzzfit/harness/src/fit.rs:86-100, fit.rs:138-147, curve.rs:51-59, crates/before/fuzzfit/harness/src/bin/diag.rs:33-53, crates/before/fuzzfit/harness/src/drive.rs:115-131, crates/before/fuzzfit/harness/src/fit/tests.rs:10-24, crates/before/fuzzfit/harness/src/bin/calibrate.rs:45-59, calibrate.rs:139-155, calibrate.rs:186-202, crates/before/fuzzfit/harness/src/bands.rs:133-159)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (read both median expressions side by side; `MIN_BUCKETS`/`MIN_DECADES` declared at fit.rs:88,100 private and curve.rs:52,59 public with equal values; diag.rs:33-46 and drive.rs:119-128 differ only in the family filter and the accumulation target; the two `writeln!` templates differ only in the hard-coded `rejected: false`); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed (nuance: diag's own loop skips `build`/`run_program` for filtered-out families); history: no rationale found (3d11ecc3 unified `BUCKETS_PER_DECADE` only; the rest are copies)
- Owner-gated: no

`curve.rs` re-implements `fit.rs`'s half-decade bucketing and per-bucket median statistic and re-declares `MIN_BUCKETS` and `MIN_DECADES`; `diag.rs` re-implements `drive::for_each_deterministic_program`, so its "same corpus as `calibrate`" holds by copy, not by construction; `Fit` and `Band` share eight fields transcribed by two identical `band_of` helpers and two byte-identical source templates in `calibrate`. fit.rs:92-96 already makes the argument for one exported constant ("the diagnostics must bucket exactly the way the fit does, or their medians stop describing the fit's inputs"); the same argument applies to the median itself and to the thresholds. Legibility and one source of truth.

Evidence:

       100	    let median = |pts: &mut Vec<(f64, f64)>| {
       101	        pts.sort_by(|a, b| a.1.total_cmp(&b.1));
       102	        let y = pts[pts.len() / 2].1;
       103	        pts.sort_by(|a, b| a.0.total_cmp(&b.0));
       104	        let x = pts[pts.len() / 2].0;
       105	        (x, y)
       106	    };

    the same statistic in fit.rs:

       140	        .map(|pts| {
       141	            pts.sort_by(|a, b| a.1.total_cmp(&b.1));
       142	            let my = pts[pts.len() / 2].1;
       143	            pts.sort_by(|a, b| a.0.total_cmp(&b.0));
       144	            let mx = pts[pts.len() / 2].0;
       145	            (mx, my)
       146	        })

    the twin `band_of` helpers (calibrate.rs:45-47, fit/tests.rs:10-12):

        45	/// A [`Band`] transcribing a fresh [`Fit`] (what the pin will say).
        46	fn band_of(kernel: &'static str, rejected: bool, f: &Fit) -> Band {

        10	/// A band transcribing `f` exactly (the agreeing pin).
        11	fn band_of(f: &Fit) -> Band {

Resolution: One `pub fn bucket_medians(samples) -> Vec<(f64, f64)>` (and a bucket-key helper) in `fit.rs`, consumed by `curve.rs` after its thin-bucket filter and by `diag.rs` for the key; `MIN_BUCKETS` and `MIN_DECADES` declared once (`fit.rs`, public) and imported by `curve.rs`. Expose the deterministic stream as an iterator or add a family predicate to `for_each_deterministic_program` so `diag` consumes it (keeping its skip of filtered-out families). Compose `Band { kernel, rejected, fit: Fit }` (or a `Band::of(kernel, rejected, Fit)` constructor), delete both `band_of`s, and factor the two `writeln!` bodies into one `fn band_source(&Band) -> String`. Acceptance: one `total_cmp` median expression and one `BUCKETS_PER_DECADE).floor()` expression in the crate; `grep -rn 'TestRunner::deterministic' harness/src` returns only drive.rs; `grep -c 'slope: {:.6}' calibrate.rs` is 1; `just fuzzfit-calibrate` on unchanged code produces byte-identical data modulo any deliberate layout change.

### fuzzfit-bands-12: The noisy-linear tripwire's jitter is bucket-correlated, so its doc overstates what it proves
- Where: crates/before/fuzzfit/harness/src/curve/tests.rs:63-64 (related: curve/tests.rs:27-39, curve/tests.rs:58-70)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (Python transcription of `sampled` and `local_slope_excess` reproducing the test's inputs exactly: per-bucket fraction of samples at 2x is 0.0, 0.5, 0.5, 1.0, 1.0, 0.5 for buckets 4..9 and the excess is +0.0817; the quadratic arm of the same transcription reads 1.000, matching the sibling test); executed: yes: the Python transcription (scratchpad `arith.py`), whose output settles both numbers
- Seen by: adequacy, instrument-correctness; refutation: confirmed (the instrument-correctness lens's "~+0.12" was imprecise; +0.082 is right); history: no rationale found
- Owner-gated: no

The comment claims deterministic 1x..2x jitter "uncorrelated with size", but `sampled()` strides by `d / 16`, which is even at d = 128 (every `dd` even, so every sample in the bottom bucket is at 1x) and odd or mixed in later buckets, so two upper buckets are entirely at 2x. The measured excess is +0.082 (27% of the allowance) from a size-correlated step, not spread. Every test's doc comment must state its invariant accurately; "the leg flags trends, not spread" is the property and the inputs prove a weaker one while consuming allowance headroom.

Evidence:

        63	    // Deterministic 1x..2x jitter, uncorrelated with size.
        64	    let excess = local_slope_excess(&band, &sampled(|d| d * 100 * (1 + d % 2)))

Resolution: Jitter by within-bucket index (expose `i` from `sampled`, or hash `d` so parity is balanced within every bucket), then tighten the assertion to a bound that demonstrates median insulation (e.g. `excess.abs() < 0.05`); or reword the comment to say the jitter is a size-correlated step and the assertion is against the allowance. Acceptance: every bucket holds half its samples at 2x, and the asserted bound is well under `SLOPE_ALLOWANCE`.

### fuzzfit-bands-13: `POOL_SLOTS`'s doc names a fallback the pooling allocator does not have
- Where: crates/before/fuzzfit/harness/src/wasm.rs:49-57 (related: wasm.rs:171-178)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (wasmtime-47.0.4 `src/runtime/vm/instance/allocator/pooling.rs:108-120`: exhaustion returns `PoolConcurrencyLimitError`, "maximum concurrent limit of {limit} for {kind} reached"; `Guest::new` turns that into a panic at `expect("guest instantiates with no imports")`); executed: no
- Seen by: scaffolding; refutation: reframed (the sizing rationale is sound; the failure mode is misstated and the expect message does not name the pool); history: no rationale found
- Owner-gated: no

Under `InstanceAllocationStrategy::Pooling` an exhausted pool is a hard error surfaced as a panic in `Guest::new` whose message says nothing about the pool; there is no fall-back to fresh mappings. Comments state what the code cannot show and must be true: the reader should know what red looks like on a wider host. The "192 on the largest host in use" figure is a machine fact at a declaration site.

Evidence:

        51	/// Instantiation must never fall back to fresh mappings (the pool is
        52	/// the point: per-sample `mmap`/`munmap` serializes every worker on
        53	/// the process's address-space lock in the kernel), so the pool is
        54	/// sized above any driver's concurrency — the samplers hold at most
        55	/// one guest per rayon worker (one per hardware thread; 192 on the
        56	/// largest host in use) plus the harness's own transient guests.
        57	const POOL_SLOTS: u32 = 512;

Resolution: Reword: "The pool has no fallback: exceeding it fails `Instance::new` with wasmtime's concurrency-limit error, so the ceiling sits above any driver's concurrency (one guest per sampler worker plus the harness's transient guests)". Name `POOL_SLOTS` in `Guest::new`'s expect message. Drop the host count or state it as the sizing premise without the machine. Acceptance: the doc names the real failure; the expect message points at `POOL_SLOTS`.

### fuzzfit-bands-14: Em-dashes in `//` comments and assert strings; past-tense incident narration at two declaration sites
- Where: crates/before/fuzzfit/harness/src/wasm.rs:131-135 (related: wasm.rs:59-61, crates/before/fuzzfit/harness/src/bin/calibrate.rs:75, calibrate.rs:117, calibrate.rs:275, calibrate.rs:301-302, crates/before/fuzzfit/harness/tests/enforce.rs:236, tests/enforce.rs:384)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (grep for em-dashes in non-doc `//` lines across harness/src and harness/tests; the two enforce.rs sites read inside assert strings); executed: no
- Seen by: adequacy, structure-prose; refutation: confirmed; history: no rationale found (both narrating comments postdate the d2a9d04e dated-notes excision)
- Owner-gated: no

Non-doc comments and panic strings use true em-dashes at the listed sites (rustdoc may keep them); the register rule asks for spaced double-hyphens or colons in code comments and terminal-bound messages. wasm.rs:131-134 and 60-61 narrate a past measurement ("were decommitted ... measured as double-digit DFL% mid-run") and a superseded configuration ("the on-demand allocator grew unboundedly") where the invariant each setting maintains belongs (Principle 5).

Evidence:

       131	        // Every slot stays warm: the default warm cap (100) is below a
       132	        // wide host's worker count, so half of every cycle's returns
       133	        // were decommitted (an address-space call again) and re-faulted
       134	        // on reuse — measured as double-digit DFL% mid-run.
       135	        pool.max_unused_warm_slots(POOL_SLOTS);

       235	            "{kernel}: the bootstrap corpus never landed a step inside its small \
       236	             band's calibrated span — the leg is decoration for this kernel; \

Resolution: Replace the em-dashes in the eight `//` and string sites with `--` or a colon; reword wasm.rs:131-134 positively ("Every slot stays warm: a warm cap below the worker count decommits half of each cycle's returns and re-faults them on reuse, the address-space traffic the pool exists to remove") and wasm.rs:60-61 as the invariant ("a pooled slot declares its maximum; the reservation is virtual"). Acceptance: `grep -rn '^\s*//[^/!].*—' harness/src harness/tests` and a grep for em-dashes inside string literals return nothing outside `strategies.rs`; no past-tense measurement remains in a `//` comment.

### fuzzfit-bands-15: `Guest::call_i64` and `Op::returns_i64` form a second call path the untyped `call` already covers
- Where: crates/before/fuzzfit/harness/src/wasm.rs:237-252 (related: wasm.rs:206-210, crates/before/fuzzfit/harness/src/drive.rs:47-51, crates/before/fuzzfit/harness/src/ops.rs:251-254, crates/before-fuelscape/src/ops.rs:377, 390, 559, 693, 884, 1179)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (wasmtime-47.0.4 `src/runtime/func.rs:1151-1157`: `call_impl_check_args` bails only when `ty.results().len() != results.len()`; `func.rs:1200-1202` writes each result slot by the function's declared type, `Val::from_raw(&mut *store, *val, ty)`; grep of before-fuelscape/src/ops.rs for the six `call_i64` sites); executed: no
- Seen by: scaffolding, structure-prose; refutation: confirmed; history: no rationale found (both paths born in 8cfd3c92; "Typed convenience" is the only stated reason)
- Owner-gated: no

wasmtime's untyped `Func::call` requires only that the results slice length equal the function's result count and writes each result by the declared type, so `Guest::call` with `results = [Val::I32(0)]` already returns i64 kernels correctly through its `Val::I64(v) => v` arm. `call_i64`, `Op::returns_i64`, and the branch in `drive.rs` are a parallel path for the same kernels; `call_i64` also indexes `args[0]` unconditionally. Legibility: one call path per kernel keeps the driver obviously correct, and the `returns_i64` predicate is a per-variant fact the driver keeps in sync with the guest ABI by hand. Fuel is consumed by guest instructions only, so the host-side call path cannot move a reading.

Evidence:

       237	    /// Typed convenience for i64-returning kernels (`ff_version_min_ticks`).
       238	    pub fn call_i64(&mut self, name: &str, args: &[u32]) -> Measured {
       239	        let func: TypedFunc<u32, i64> = self
       240	            .instance
       241	            .get_typed_func(&mut self.store, name)
       242	            .unwrap_or_else(|e| panic!("guest kernel {name}: {e}"));
       243	        self.store.set_fuel(FUEL_TANK).expect("fuel is enabled");
       244	        let ret = func
       245	            .call(&mut self.store, args[0])

    while `call` already reads:

       206	        let ret = match results[0] {
       207	            Val::I32(v) => v as i64,
       208	            Val::I64(v) => v,
       209	            ref other => panic!("guest kernel {name} returned unexpected type {other:?}"),
       210	        };

Resolution: Delete `call_i64` and `Op::returns_i64`; make drive.rs:47-51 a single `guest.call(op.kernel(), &args)`; switch the six fuelscape call sites to `call`. Acceptance: both detached workspaces build; `just fuzzfit` and `just fuelscape-test` pass with the bands untouched, and `just fuzzfit-calibrate` on the changed code produces no diff (fuel cannot move, and the empty diff is the check).

### fuzzfit-bands-16: `calibrate` accepts any `programs` count without asserting it covers the refit prefix
- Where: crates/before/fuzzfit/harness/src/bin/calibrate.rs:62-65 (related: calibrate.rs:86, crates/before/fuzzfit/harness/src/bands.rs:205, crates/before/fuzzfit/harness/tests/enforce.rs:358)
- Class / severity / confidence: correctness / nit / high
- Provenance: verified (read: `REFIT_COVERAGE` and the refit evidence derive from `case < REFIT_PREFIX_PROGRAMS` over whatever `programs` was given; nothing asserts `programs >= REFIT_PREFIX_PROGRAMS`); executed: no
- Seen by: raised as new by the refutation pass; history: not examined
- Owner-gated: no

A run such as `calibrate 100` pins a coverage list and refit evidence from a 100-program "prefix" while `enforce.rs` refits 256 programs, so the two legs would silently compare different streams. The module doc names 4096 as the corpus of record but nothing enforces the relation to the prefix. Principle 1: handle every input; a one-line assertion closes it.

Evidence:

        62	    let programs: usize = std::env::args()
        63	        .nth(1)
        64	        .map(|s| s.parse().expect("programs must be a number"))
        65	        .unwrap_or(4096);

        86	            if case < REFIT_PREFIX_PROGRAMS {

Resolution: `assert!(programs >= REFIT_PREFIX_PROGRAMS, "corpus must cover the {REFIT_PREFIX_PROGRAMS}-program refit prefix")` after parsing; name the default as `CORPUS_OF_RECORD` (see finding 26). Acceptance: `calibrate 100` fails by name before sampling.
Construction: run `cargo run --bin calibrate -- 100`: `REFIT_COVERAGE` is regenerated from 100 programs and the "refit evidence: prefix (256 programs)" line reports over a prefix that does not exist.

### fuzzfit-bands-17: The floors' liveness claim is calibrate stderr, evaluated at one endpoint over the main bands only; no committed check asserts floor > nop
- Where: crates/before/fuzzfit/harness/src/bin/calibrate.rs:273-297 (related: crates/before/fuzzfit/harness/src/bands.rs:175-194, bands.rs:688-699, bands.rs:700-711, crates/before/fuzzfit/harness/tests/enforce.rs:242-258, crates/before/fuzzfit/harness/tests/sanity.rs:185-217, crates/before/fuzzfit/harness/src/fit.rs:242-250)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read all three test files: the only `ff_nop` use is `fuel_metering_is_live`, and no test compares any pinned floor to the nop reading; Python over the pinned constants with nop = 2 gives floor gaps at both endpoints of every band and small band: narrowest +0.1155 `ff_rank_cmp` at 128 bits, then +0.7015 `ff_clock_sync` [err], +0.8343 `ff_version_meet`; `ff_rank_checked_sub` [err] has slope −0.018256 with floors 1.460 at 128 bits and 1.432 at 4216 bits, the lower one at `max_denom`); executed: no
- Seen by: adequacy, structure-prose, instrument-correctness; refutation: confirmed; history: deliberate-and-holds for the stderr mechanism (b528f2c6; it worked once by human reading when 875c118b moved the margin 1.0 to 0.8), with the committed-test ask exceeding the recorded decision, not contradicting it; the "non-negative slope" caveat was written beside a negative-slope arm; the small-band omission has no rationale
- Owner-gated: no

The only place the instrument evaluates whether every pinned floor sits above the dead-meter reading is this loop, which prints the narrowest gap for a human at re-pin time. The `ENFORCE_MARGIN_BELOW` doc claims "every pinned floor still clears that reading with this slack subtracted" over every band key, and records that at 1.0 the `ff_rank_cmp` floor already dipped under nop, so the margin is tuned to the edge of validity; the last re-pin demonstrably did not re-read the evidence (finding 2). The loop also takes the floor at `min_denom` only, under a premise ("for a non-negative slope") the pin falsifies, and walks `fits` only, never `small_fits`, though `judge_small` applies the same margin to the `SMALL_BANDS` floors. Principle 6 (every hole becomes a committed check, never a convention held in memory) and Principle 2 (a ceiling passes vacuously when the floor does not bite). At this pin the `ff_rank_cmp` floor is about 2.6 fuel at 128 bits against a nop of 2, so it distinguishes only a return-immediately kernel from work; a stubbed pair kernel that still borrows its registers costs more than nop.

Evidence:

       273	    // The liveness margin's evidence: the narrowest gap, over every band
       274	    // key, between the effective floor (line − width_below −
       275	    // ENFORCE_MARGIN_BELOW, at min_denom — the floor's lowest judged
       276	    // point for a non-negative slope) and the nop-level reading a dead
       277	    // meter produces. The liveness claim rests on every gap staying
       278	    // positive.
       279	    let nop = Guest::new().call("ff_nop", &[]).fuel;
       280	    let nop_log = (nop.max(1) as f64).log10();
       281	    let mut floor_min: Option<(f64, Key)> = None;
       282	    for (&key, f) in &fits {
       283	        let floor = f.intercept + f.slope * (f.min_denom as f64).log10()
       284	            - f.width_below
       285	            - ENFORCE_MARGIN_BELOW;

       691	        slope: -0.018256,

Resolution: Add an enforcement test that calls `ff_nop` on a live `Guest` and, for every band in `BANDS` and `SMALL_BANDS`, asserts `intercept + slope · log10(d) - width_below - ENFORCE_MARGIN_BELOW > log10(nop)` at both `d = min_denom` and `d = max_denom` (the endpoint argument `line_divergence` already uses). Make calibrate's loop evaluate both endpoints over `fits` and `small_fits` and fail rather than print when any gap is non-positive; drop the caveat once the code no longer needs it. Consider measuring a dispatch-shaped control kernel (borrow two registers, return) once, so the `ff_rank_cmp` floor's liveness claim is stated against the dead reading a stubbed pair kernel actually produces (see open question 6). This composes with finding 2's `PIN_EVIDENCE.floor_min_gap`. Acceptance: a committed synthetic case (the pinned `ff_rank_cmp` band with `width_below` widened by 0.12) reads `InBand` for `fuel = nop` at 128 bits and the new check rejects such a band by name; calibrate exits non-zero on a non-positive gap; the printed minimum covers 53 floors at both endpoints.
Construction: Copy the pinned `ff_rank_cmp` `Band` (bands.rs:700-711) into a test, add 0.12 to `width_below`, and call `judge_against(&band, 128, 2)`: the result is `InBand`. Nothing in `tests/enforce.rs` or `tests/sanity.rs` would fail if calibrate emitted such a band; the judgment tripwire at sanity.rs:186-217 and the burner test at enforce.rs:273-310 use synthetic bands and never touch the pinned floors.

### fuzzfit-bands-18: Generated data and hand-written prose share one file, which is what the splice marker, the column-zero template, and the duplicated rustdoc exist to manage
- Where: crates/before/fuzzfit/harness/src/bin/calibrate.rs:345-358 (related: calibrate.rs:359-403, crates/before/fuzzfit/harness/src/bands.rs:312-327, bands.rs:918-926, bands.rs:977-989)
- Class / severity / confidence: scaffolding / low / high
- Provenance: verified (the scaffolding lens diffed bands.rs:312-327, 918-926, 977-989 against the template at calibrate.rs:360-375, 378-386, 389-401 with the marker and rustc substitutions applied and found all three byte-identical; I re-read both copies); executed: no
- Seen by: scaffolding, structure-prose; refutation: confirmed; history: deliberate-but-expired (co-location served the in-doc dated movement ledger, which d2a9d04e retired in favour of the re-pin commit message; the marker guard 4b71d569 and the column-zero literal 27ace7a2 are patches on the shared-file design)
- Owner-gated: no

Because calibrate rewrites the tail of `bands.rs` in place, it must locate a prose marker by string, guard its absence, format a template whose continuation lines sit at column zero to satisfy the doc-summary linter, and carry the rustdoc for `PINNED_RUSTC`, `BANDS`, `SMALL_BANDS`, and `REFIT_COVERAGE` as string literals that duplicate the same prose in `bands.rs`; a doc edit in either place is lost or ineffective until the other is edited too. Principle 3: infrastructure that generates its own maintenance cascade is suspect, and the constraint that justified it has expired.

Evidence:

       345	    let bands_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/bands.rs");
       346	    let current = std::fs::read_to_string(&bands_path).expect("bands.rs exists");
       347	    let marker = "/// The toolchain that pinned";
       348	    // The marker line is prose in a generated region: a rewording that
       349	    // loses it must fail here by name, never silently splice the whole
       350	    // file into the head.
       351	    let head = &current[..current
       352	        .find(marker)
       353	        .expect("bands.rs splice marker present")];
       354	    let rustc = env!("FUZZFIT_RUSTC_VERSION");
       355	    // A plain multi-line literal (continuation lines at column zero): the
       356	    // emitted `///` lines keep their paragraph structure both in the
       357	    // written file and here in the source, where the doc-summary linter
       358	    // reads them too.

Resolution: Keep the constant declarations and their rustdoc in `bands.rs` and have calibrate emit only the array bodies and the rustc literal into data files (`src/bands/pinned_bands.rs`, `pinned_small_bands.rs`, `pinned_refit_coverage.rs`, `pinned_rustc.rs`, each `include!`d as an expression, since `include!` cannot carry item docs), plus the `PIN_EVIDENCE` value from finding 2. The marker search, its expect, the column-zero literal, the template prose, and the whole-file rewrite of a hand-edited file all dissolve; calibrate's row rendering becomes one `fn` over `Band`. Acceptance: calibrate writes no file containing `///` lines; `bands.rs` is never rewritten by a tool; `just fuzzfit-calibrate` on unchanged code yields no diff; `just fuzzfit`'s fmt and clippy legs stay clean.

### fuzzfit-bands-19: Fuel determinism, the instrument's stated foundation, has no committed test; the only check is a probe binary no recipe runs
- Where: crates/before/fuzzfit/harness/src/bin/probe.rs:1-8 (related: probe.rs:62-64, crates/before/fuzzfit/harness/src/wasm.rs:4-11, wasm.rs:117-135, crates/before/fuzzfit/harness/src/lib.rs:40-42, crates/before/fuzzfit/harness/src/drive.rs:19-31, justfile:573-607)
- Class / severity / confidence: verification-gap / medium / high
- Provenance: verified (read every test file in the harness: no test runs a program twice or compares two `Measured.fuel` values from separate guests; `grep -n probe justfile` matches only `muxprobe` and "probe cursors"; `git log -- wasm.rs` shows the pooled allocator (78e24230) and the warm-slot cap (9b051e19) landing 2026-08-06, after the last re-pin e7a4b7b0 2026-08-04; `Sample` derives only `Debug, Clone, Copy`); executed: no
- Seen by: scaffolding, adequacy, structure-prose; refutation: confirmed; history: deliberate-but-expired (the probe was the bring-up feasibility check, its purpose discharged when the harness was built; the design note's "re-run at each pin" convention is unenforced and no re-pin commit records a probe run)
- Owner-gated: no for the test; deleting `probe.rs` afterwards is the owner's call (removal of an instrument)

The harness states everywhere that fuel is a pure function of (guest bytes, call sequence, payloads) and that fresh pooled-slot guests start from identical state, and builds replay, shrinking, the committed seeds, and the staleness cross-check on that premise. The only check that two executions of one program agree is `probe.rs:64`, run by hand. Two of the probe's three checks are performed by every enforcement run (instantiation by every `Guest::new`; the host-to-guest-to-host differential by `drive::run_program`), leaving fuel determinism as its one live function, which the pooled allocator's slot-reset semantics (environmental state a harness cannot reason about from code) now carry without any committed exercise. Principles 2 and 6: a premise everything rests on needs a committed instrument; nondeterminism here would surface only as unexplained seed-replay flakiness inside the margins, never as a named failure.

Evidence:

         4	//! Verifies, before anything is built on top: (1) the guest builds and
         5	//! instantiates; (2) the same input yields byte-identical fuel across two
         6	//! fresh instances in-process (and across process invocations — compare two
         7	//! runs' stdout); (3) a random input round-trips host → guest → host with
         8	//! the guest's result byte-equal to the native mirror's.

        64	    assert_eq!(a, b, "fuel is not deterministic across fresh instances");

Resolution: Add `fuel_is_deterministic_across_fresh_guests` to `tests/enforce.rs`: run one fixed program (e.g. `build(&Family::Escalation { depth }, seed)` from `ESCALATION_REPLAYS[0]`, or the first bootstrap program) through `run_program` twice, each with its own fresh `Guest`, and `assert_eq!` the two `Vec<Sample>` including fuel (derive `PartialEq, Eq` on `Sample`). Then retire `probe.rs`, whose remaining unique function the test owns, or reword its doc so it no longer claims to be the determinism proof. Acceptance: the committed test fails if two fresh-guest replays differ in any sample's fuel; demonstrated red under a deliberate break (skip `ff_regs_reserve` on the second instance so a reallocation lands inside a measured call) before the probe is removed; `just fuzzfit` green after.
Construction: Grep the tests for a second `run_program` on the same program or any comparison of two `Measured.fuel` values from separate guests: none exists. A failure the test would catch: a `static mut` counter read inside a guest kernel, or a pooled slot that is not reset to the initial image; the suite today stays green while replays diverge.

### fuzzfit-bands-20: The seed-location sentence is a ghost of the pre-anchor layout
- Where: crates/before/fuzzfit/harness/tests/enforce.rs:12-15 (related: crates/before/fuzzfit/harness/tests/main.rs:1-7, crates/before/fuzzfit/harness/proptest-regressions/enforce.txt, crates/before/fuzzfit/guest/src/lib.rs:276-277)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`ls harness/tests` shows enforce.rs, main.rs, sanity.rs; the only seed file is harness/proptest-regressions/enforce.txt with six entries; `git show --stat ce3664dd` adds tests/main.rs; `git blame` dates enforce.rs:13 to c1fe9388 2026-07-26; grep finds the stale `harness/tests/enforce.proptest-regressions` path at guest/src/lib.rs:277); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed; history: deliberate-but-expired (ce3664dd moved the seed and added the anchor without updating either sentence)
- Owner-gated: no

Since ce3664dd introduced `tests/main.rs` as the persistence anchor, seeds resolve to `proptest-regressions/enforce.txt` at the package root, not beside the test binary; the doc still describes the sibling-file layout, and the guest's `ff_regs_reserve` doc (outside this partition, but pointing at this partition's artifact) names a path that does not exist. Principle 5 and the root AGENTS.md hard rule: no references to things that no longer exist; a maintainer following either sentence to check that a seed is committed lands nowhere.

Evidence:

        12	//! determinism makes a failure replay exactly: proptest shrinks to a
        13	//! minimal out-of-band shape and writes a seed file next to this binary —
        14	//! commit any seed that appears (repo hard rule); it is an
        15	//! out-of-band-shape finding of record.

Resolution: "writes a seed to `proptest-regressions/enforce.txt` at the package root (the `tests/main.rs` anchor)"; fix guest/src/lib.rs:276-277 to `harness/proptest-regressions/enforce.txt`. Acceptance: `grep -rn 'next to this binary\|enforce.proptest-regressions' crates/before/fuzzfit` is empty; both sentences name the path `tests/seed_liveness.rs` resolves.

### fuzzfit-bands-21: The `PROPTEST_CASES` override the suite documents is discarded by `ProptestConfig::with_cases`
- Where: crates/before/fuzzfit/harness/tests/enforce.rs:35-36 (related: tests/enforce.rs:438-439, crates/before/fuzzfit/harness/tests/sanity.rs:13-14, crates/before/fuzzfit/harness/src/strategies.rs:86-88, justfile:582)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (proptest-1.11.0 `src/test_runner/config.rs:456-461`: `pub fn with_cases(cases: u32) -> Self { Self { cases, ..Config::default() } }`; `Config::default()` clones `DEFAULT_CONFIG` (591-594), which applies `contextualize_config` (191-195) reading `PROPTEST_CASES` (20-26); the struct update then overwrites `cases` with 48; the harness Cargo.lock pins proptest 1.11.0); executed: no
- Seen by: scaffolding, structure-prose, instrument-correctness; refutation: confirmed; history: no rationale found (both the sentence and the config date from 7fb3b5ce)
- Owner-gated: no

The environment-derived default is evaluated first and then overwritten, so the sentry always runs exactly 48 cases and the module doc promises a knob that does nothing. A re-pinner following the doc to widen a local run before a re-pin gets 48 cases and no warning. Principle 8: a false operational claim in the file's first paragraph.

Evidence:

        35	//! Case count: 48 by default (the calibration corpus is the big sweep; this
        36	//! is the sentry); override with `PROPTEST_CASES`.

       439	    #![proptest_config(ProptestConfig::with_cases(48))]

Resolution: Either delete the override sentence (the justfile already states 48), or honor it: `ProptestConfig { cases: std::env::var("PROPTEST_CASES").ok().and_then(|s| s.parse().ok()).unwrap_or(SENTRY_CASES), ..ProptestConfig::default() }` with `const SENTRY_CASES: u32 = 48` (which also anchors the other hand-written 48s in this file; see finding 26). Acceptance: the doc describes what the config does; if an override is kept, `PROPTEST_CASES=4 PROPTEST_VERBOSE=1 cargo nextest run ... fuel_stays_in_the_pinned_bands` reports 4 cases.
Construction: Run the sentry with `PROPTEST_CASES=1 PROPTEST_VERBOSE=1` in the fuzzfit workspace: 48 cases execute.

### fuzzfit-bands-22: The band key is a bare `(&str, bool)` tuple with the `" [err]"` rendering repeated eleven times
- Where: crates/before/fuzzfit/harness/tests/enforce.rs:56-66 (related: tests/enforce.rs:79-127, crates/before/fuzzfit/harness/src/bin/calibrate.rs:67, crates/before/fuzzfit/harness/src/bands.rs:989, crates/before/fuzzfit/harness/src/drive.rs:17-31, crates/before/fuzzfit/harness/src/bin/diag.rs:68)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (grep for `" [err]"` across harness/src and harness/tests: diag.rs:68; calibrate.rs:159, 234, 261, 268, 295, 340; enforce.rs:63, 66, 149, 368); executed: no
- Seen by: structure-prose; refutation: confirmed; history: no rationale found
- Owner-gated: no

The band key (kernel times outcome) is central to every module yet has no type: `type Key = (&'static str, bool)` local to calibrate's `main`, `BTreeMap<(&'static str, bool), ...>` in enforce, diag, and calibrate, `REFIT_COVERAGE: &[(&str, bool)]`, and two-parameter lookups everywhere. Its display form `if rejected { " [err]" } else { "" }` recurs at eleven sites, and `judge` carries four structurally identical panic blocks. Types-first (newtypes over synonyms) and legibility.

Evidence:

        57	    let mut by_key: BTreeMap<(&'static str, bool), Vec<(u64, u64)>> = BTreeMap::new();
        58	    for s in samples {
        59	        let band = band_for(s.kernel, s.rejected).unwrap_or_else(|| {
        60	            panic!(
        61	                "{}{} has no pinned band: re-run `just fuzzfit-calibrate`",
        62	                s.kernel,
        63	                if s.rejected { " [err]" } else { "" },
        64	            )
        65	        });
        66	        let arm = if s.rejected { " [err]" } else { "" };

Resolution: `#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)] pub struct BandKey { pub kernel: &'static str, pub rejected: bool }` with `impl Display` (`{kernel}` or `{kernel} [err]`); `Band.key`, `Sample.key`, `REFIT_COVERAGE: &[BandKey]`, `band_for(key)`; one `fn violation(label, sample, band, predicted) -> !` in `judge`. Acceptance: one `" [err]"` literal in the crate; `judge` under 60 lines.

### fuzzfit-bands-23: Two dead guards: `bands_are_pinned` is subsumed by the roster parity test, and the small-band `BelowFloor` arm is unreachable
- Where: crates/before/fuzzfit/harness/tests/enforce.rs:158-164 (related: tests/enforce.rs:77-78, crates/before/fuzzfit/harness/tests/sanity.rs:162-168, crates/before/fuzzfit/harness/src/bands.rs:246-248, bands.rs:299-310)
- Class / severity / confidence: vestigial / nit / high
- Provenance: verified (read: `judge_small` returns `None` when `denom_bits < band.min_denom` (bands.rs:306), so the `judge_against` inside it never reaches its `BelowFloor` branch (bands.rs:247); an empty `BANDS` fails `bands_and_op_roster_name_the_same_kernels` by kernel name at sanity.rs:162-168 and every `judge` call at enforce.rs:59-65); executed: no
- Seen by: scaffolding; refutation: confirmed; history: deliberate-but-expired for `bands_are_pinned` (7fb3b5ce predates the roster test 61398dc9); the `BelowFloor` arm was unreachable from its birth in af2330a6
- Owner-gated: no

A guard earns its place by naming a failure the other instruments miss (Principle 3); neither does.

Evidence:

       158	#[test]
       159	fn bands_are_pinned() {
       160	    assert!(
       161	        !BANDS.is_empty(),
       162	        "no pinned bands: run `just fuzzfit-calibrate` and commit src/bands.rs"
       163	    );
       164	}

        77	                    match verdict {
        78	                        Verdict::InBand | Verdict::BelowFloor => {}

Resolution: Delete `bands_are_pinned`; in `judge`, match `Verdict::InBand => {}` and route `BelowFloor` to `unreachable!("judge_small cuts the span below min_denom")`, or restructure `judge_small` to return a verdict type without the `BelowFloor` variant. Acceptance: the test is gone; the match has no arm the code cannot reach.

### fuzzfit-bands-24: "the demonstrations ledger" resolves to nothing in the tree; the decision to keep the reach demonstrations in git history is recorded only in the design note
- Where: crates/before/fuzzfit/harness/tests/enforce.rs:268-271 (related: tests/enforce.rs:398-436, .agent-notes/2026-07-26-before-fuzzfit-asymptotics/before-fuzzfit-asymptotics.md:348-354 and 441-463, crates/before/src/testing/validation_index.rs:100-107)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep for the phrase across crates/before and .agent-notes finds it only here; the design note §8 records that each of the five genre demonstrations "is a dated record in git history at its pin commit" and lists the standing demonstrations that run every suite); executed: no
- Seen by: scaffolding (as a medium verification-gap); refutation: confirmed; history: already-known (the note records the design: demonstrations in git history, the defenses they forced standing in the tree; the phrase originally pointed at a design document since deleted and resurrected under .agent-notes)
- Owner-gated: no for the doc fix; committing real-kernel sabotage demonstrations would reopen the note's §8 design and is an open question below

The burner test's doc hands responsibility for generator reach to "the demonstrations ledger", a thing a reader cannot find: the phrase names the design note's §8, which is a design-document citation from rustdoc (a hard rule) and, after e13854de's deletion and the .agent-notes resurrection, resolves only through LLM-written notes. The owner's decision (the five known-bad reconstructions live in git history at their pin commits; what stands in the tree is the synthetic tripwires, the escalation replays, `REFIT_COVERAGE`, and the seeds) is a recorded ruling, so the verification-gap the lens raised converts to: state the decision where the doc points at it. Principle 5.

Evidence:

       268	    /// proves the wasm-execution → fuel-metering → judgment path can flag a
       269	    /// quadratic at all; whether the *generators* place real kernels where a
       270	    /// regression must flag is the reach families' and the demonstrations
       271	    /// ledger's business, not this check's.

Resolution: Replace "the demonstrations ledger's business" with the decision in the tree's own terms: "the reach families' and the escalation replays' business; the known-bad reconstructions that accepted each reach genre are recorded at their pin commits, and the defenses they forced (`ESCALATION_REPLAYS`, the outcome-keyed bands, `REFIT_COVERAGE`, the committed seeds) are what stand here". Acceptance: the phrase names tree artifacts or git history explicitly; no rustdoc in the partition points at a design document.

### fuzzfit-bands-25: Idiom nits: qualified paths beside imports, magic numbers, an expect message that asserts rather than names
- Where: crates/before/fuzzfit/harness/tests/enforce.rs:369-371 (related: tests/enforce.rs:193, tests/enforce.rs:254, tests/enforce.rs:286, tests/enforce.rs:388, crates/before/fuzzfit/harness/src/curve/tests.rs:35-36, crates/before/fuzzfit/harness/src/bin/calibrate.rs:111, calibrate.rs:137)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (grep `fuzzfit_harness::` in enforce.rs outside the `use` block: lines 193, 286, 371, 388; read the other sites); executed: no
- Seen by: structure-prose; refutation: confirmed; history: no rationale found (the bare `100` nop ceiling matches the design note's stated liveness pin "(0, 100)", so a named constant would carry that provenance)
- Owner-gated: no

`enforce.rs` writes `fuzzfit_harness::fit::fit`, `fuzzfit_harness::fit::line_divergence`, `fuzzfit_harness::fit::FIT_FLOOR_BITS`, and `fuzzfit_harness::bands::Band` while importing from both modules at lines 42-46; `curve/tests.rs:36` encodes the square root of ten as `3.163` where `10f64.powf(1.0 / BUCKETS_PER_DECADE)` states the derivation; `enforce.rs:254` bounds the nop at a bare `100`; `calibrate.rs:111` reports every `% 32`; `calibrate.rs:137`'s expect asserts the premise a one-sample key would falsify instead of naming the key. Imports over long qualified paths; named constants over magic numbers; every expect message is a one-line proof or, for a tool, names what went wrong.

Evidence:

       369	        let f = by_key
       370	            .get(&(kernel, rejected))
       371	            .and_then(|samples| fuzzfit_harness::fit::fit(samples))

        35	        // Two buckets per decade: step by √10.
        36	        d = (d as f64 * 3.163) as u64;

       137	        let f = fit(samples).expect("every band key has ≥ 2 samples");

Resolution: Add `fit`, `line_divergence`, `FIT_FLOOR_BITS`, `Band` to the existing `use` lines; `let step = 10f64.powf(1.0 / BUCKETS_PER_DECADE)`; `const NOP_CEILING: u64 = 100` with its rationale; `const PROGRESS_EVERY: usize = 32`; `.unwrap_or_else(|| panic!("band key {kernel}{arm} has a single sample; the corpus must reach every key twice"))`. Acceptance: no `fuzzfit_harness::` path in enforce.rs outside `use`; no bare `3.163`, `100`, or `32` at those sites.

### fuzzfit-bands-26: Hand-maintained counts and derived probabilities across the harness prose
- Where: crates/before/fuzzfit/harness/tests/enforce.rs:402-410 (related: tests/enforce.rs:21, tests/enforce.rs:35-36, tests/enforce.rs:338-339, tests/enforce.rs:424-425, crates/before/fuzzfit/harness/src/wasm.rs:34-36, wasm.rs:55-56, crates/before/fuzzfit/harness/src/bin/calibrate.rs:62-65, crates/before/fuzzfit/harness/src/bands.rs:74-79, crates/before/fuzzfit/harness/src/curve.rs:64-65, crates/before/fuzzfit/harness/src/strategies.rs:1975-1978)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (recomputed 137 from `any_family`'s `prop_oneof!` weights at strategies.rs:1980-2015: seventeen `8 =>` arms plus one `1 =>` arm; 48 · 8 / 137 = 2.8; 20,048 = 2 · 9000 + 2048 from `ESCALATION_BUDGET` at strategies.rs:106-111; the const assertion at wasm.rs:41-47 already enforces the register bound); executed: no
- Seen by: scaffolding, adequacy, structure-prose; refutation: confirmed; history: no rationale found
- Owner-gated: no

Prose restates values the code computes or constants determine: the sentry's 48 (lines 21, 35, 338-339 versus the literal at 439), the escalation draw odds ("once in 137", from seventeen families at weight 8 plus one at weight 1), the independent-regime rate ("nearly three times per default run"), "the seven single-operand rows", the register-reserve bound (wasm.rs:35-36 "20,048", beside the const assertion that enforces it), a host core count, and the corpus size 4096 as a bare `unwrap_or(4096)` named in prose at bands.rs:74, 181, 211 and curve.rs:64. Each is accurate today and rots the moment its source changes (Principle 5: state the structure, not the tally; a number that matters lives in a mechanically enforced place prose may cite by name).

Evidence:

       402	/// The sentry's random draws pick the escalation family about once in 137
       403	/// cases, so a 48-case run usually never leaves the small-operand regime —
       404	/// and an instrument whose deep reach is exercised only by rare draws has
       405	/// no standing proof its at-scale bands (the seven single-operand rows,
       406	/// the rejection arms, the deep-overlap scans) still bite. This replay is

        34	/// share count is capped by its fork budget — so no program allocates
        35	/// more than `2 · max_ops + max_forks` slots (20,048 under the larger,
        36	/// escalation budget).

Resolution: Name the constants (`pub const SENTRY_CASES: u32 = 48`, `pub const CORPUS_OF_RECORD: usize = 4096` with `calibrate` defaulting to it) and cite them by name; replace "once in 137" with the structure ("Escalation carries weight 1 against 8 for every other family"); drop the 20,048 parenthetical (the const assertion is the statement) and the host count; replace "the seven single-operand rows" with the structural description. Acceptance: changing `any_family`'s weights, `ESCALATION_BUDGET`, or the sentry case count leaves no numeral in prose to update; `grep -rn '137\|20,048\|seven single' harness/` is empty.

### fuzzfit-bands-27: "Every public operation" overstates the 44-kernel vocabulary the bands price
- Where: crates/before/fuzzfit/harness/tests/enforce.rs:441-443 (related: crates/before/fuzzfit/harness/src/lib.rs:4-5, justfile:569-570, crates/before/fuzzfit/harness/tests/sanity.rs:87-96, crates/before/src/testing/validation_index.rs:139-142 and 158-162, crates/before/src/surface.rs:322)
- Class / severity / confidence: claim / medium / high
- Provenance: verified (extracted the guest's `pub extern "C" fn ff_*` exports (107) and `BANDS`' distinct kernels (44) and diffed the sets: 63 exports outside the bands, of which 7 are control or self-test (`ff_nop`, `ff_reset`, `ff_regs_reserve`, `ff_stage_*`, `ff_selftest_quadratic`) and the rest measured kernels for public operations: `ff_span_*` (16), `ff_query_*` and `ff_floor_contains`/`ff_ceiling_contains` (12), `ff_own_span_*`/`ff_own_version_*` (6), `ff_ranked_*`/`ff_rank_encode`/`ff_rank_decode` (6), `ff_clock_display`/`fromstr`/`forks`/`join_all`/`sync_all`/`recv_all`, `ff_party_join_all`/`hash`/`shape`, `ff_version_eq`/`hash`/`shape`/`span`/`span_all`/`ticks`, `ff_shape_combine`, `ff_clock_shape`; `before::surface::METHOD_SURFACE` exists and no fuzzfit test reads it); executed: no
- Seen by: scaffolding; refutation: confirmed; history: no rationale found (the wording dates from the harness's first crate doc; the design note §5 claims a public-API addition fails the roster test by name, which is unfounded since the roster test ties `Op` variants to `BANDS` and never reads the surface; later guest kernels landed for fuelscape panels with "no re-pin")
- Owner-gated: yes: the doc restatement can land now; a tiling test against `METHOD_SURFACE` extends the gate, and extending the vocabulary to the exported-but-unbanded kernels is strategies-partition work the owner scopes

The sentry's doc comment, the crate doc, and the justfile describe the bands as the committed cost law for every public operation, but the vocabulary prices 44 kernels while the shared guest exports about fifty-six further measured kernels for public operations that no band judges and no tiling test accounts for. The board and the fuelscape both carry a tiling against `crate::surface` ("priced or excused, never neither"); the fuzz-fit vocabulary has none, so a public operation outside the 44 is neither priced nor excused while the prose claims otherwise. Every test's doc comment must state its invariant accurately, and an asymptotic claim needs an instrument that fails when it is false.

Evidence:

       441	    /// Every public operation stays inside its pinned fuel band on
       442	    /// shapes nobody chose, and no band key's within-case cost trend
       443	    /// out-climbs its pinned slope.

         4	//! The crate's asymptotic claims (every public operation amortized linear in
         5	//! its denominated size) are guarded elsewhere by chosen adversarial families

       569	# in harness/src/bands.rs — the committed cost law for every public
       570	# operation, so a change that moves an operation's asymptotics fails here

Resolution: Now: restate the three claims as "every operation in the program vocabulary ([`ops::Op`])". Then add a tiling test (sibling to `bands_and_op_roster_name_the_same_kernels`) that walks `before::surface::METHOD_SURFACE` and requires each row to be either a vocabulary kernel or an entry in a committed exclusion list stating its mechanism, so a new public operation cannot land unpriced and unexcused. Owner-gated: extend the vocabulary to the exported-but-unbanded kernels (the span algebra and causally kernels already exist in the guest because the fuelscape prices them; the generator work is the missing piece). Acceptance: the tiling test fails when a surface row is removed from both the vocabulary and the exclusion list; each exclusion names a mechanism; no doc says "every public operation" unless the vocabulary covers the surface.
Construction: Pick an existing public operation with a guest kernel and no band, such as `Span::join` (`ff_span_join`), and run `just fuzzfit`: every leg passes because nothing samples the kernel and nothing lists it as excused, while enforce.rs:441 asserts the claim over every public operation.

### fuzzfit-bands-28: The pure `judge_against` tripwire lives in an integration file titled "Generator sanity"; `bands` has no sibling tests
- Where: crates/before/fuzzfit/harness/tests/sanity.rs:1-3 (related: tests/sanity.rs:185-217, crates/before/fuzzfit/harness/src/bands.rs:244-259, crates/before/fuzzfit/harness/src/fit.rs:252-253)
- Class / severity / confidence: modularity / nit / high
- Provenance: verified (read; no `src/bands/` directory exists, while `fit.rs` and `curve.rs` each declare `mod tests;`); executed: no
- Seen by: structure-prose; refutation: confirmed; history: no rationale found
- Owner-gated: no

The module doc describes generator invariants, but `judgment_flags_quadratic_and_dead_readings` is a pure unit test of `bands::judge_against` and the roster test is about the pin roster. Tests live in a sibling `tests.rs`; a doc comment's first sentence stands alone in a listing and should describe the file's contents.

Evidence:

         1	//! Generator sanity: the strategies' structural invariants, judged natively
         2	//! (no guest required), so a generator bug fails here before it can confuse
         3	//! a fuel reading.

       185	#[test]
       186	fn judgment_flags_quadratic_and_dead_readings() {

Resolution: Move `judgment_flags_quadratic_and_dead_readings` to `src/bands/tests.rs` (add `#[cfg(test)] mod tests;` to `bands.rs`); keep the roster test here (cross-module) and retitle the module doc to cover generator and roster checks. Acceptance: sanity.rs's first sentence describes every test in it; `bands.rs` has a `tests.rs`.

### fuzzfit-bands-29: Vacuous denominator assertion in `programs_are_well_formed`
- Where: crates/before/fuzzfit/harness/tests/sanity.rs:51-51 (related: crates/before/fuzzfit/harness/src/ops.rs:431-448)
- Class / severity / confidence: test-quality / nit / high
- Provenance: verified (read: `Step` is constructed only by `done` and `done_pair` at ops.rs:432 and 443, both with `denom_bits: denom_bits.max(1)`); executed: no
- Seen by: instrument-correctness; refutation: confirmed; history: no rationale found (the assertion and the floor it tests were written in the same commit)
- Owner-gated: no

`prop_assert!(step.expect("checked").denom_bits >= 1)` cannot fail for any generator output; it tests the presence of the `.max(1)` floor, not a property of the programs. Principle 3: an assertion earns its place by naming a constructible failure it catches.

Evidence:

        51	            prop_assert!(step.expect("checked").denom_bits >= 1);

       433	                denom_bits: denom_bits.max(1),

Resolution: Delete the line, or assert the intended property on the mirror's registers (no live operand encodes to zero bits). Acceptance: the test's doc comment describes only invariants that can fail.

### fuzzfit-bands-30: The kernel roster in `sanity.rs` is a hand-maintained one-per-variant list nothing ties to `Op`
- Where: crates/before/fuzzfit/harness/tests/sanity.rs:94-99 (related: tests/sanity.rs:99-160, crates/before/fuzzfit/harness/src/ops.rs:56-150, ops.rs:152-201)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (counted 44 `Op` variants at ops.rs:56-150 and 44 roster entries at sanity.rs:99-160; `Op::kernel` at ops.rs:154-201 is an exhaustive match the compiler polices; the roster is not); executed: no
- Seen by: scaffolding, adequacy, structure-prose, instrument-correctness; refutation: confirmed; history: deliberate-and-holds (61398dc9 chose a committed expectation list so a kernel added or orphaned fails by name in a reviewable diff; that rationale is stated inline and stands; the totality gap is real and unaddressed by it)
- Owner-gated: no

The parity test derives the kernel set from a 44-entry hand list whose completeness over `Op` rests on the sentence asking the author to extend it. A variant added with a kernel and no band, and not emitted by any strategy, passes this test and is priced and flagged by nothing until a generator happens to emit it. Principle 6 (the cheapest passing artifact, adding a variant and skipping the roster, is not the intended one) and Principle 5 (no hand-maintained enumerations of facts the code can change without touching the prose). The expectation-list design can stay; what closes the gap is making its totality a compile error.

Evidence:

        94	/// sample the hole. The roster is one representative op per `Op`
        95	/// variant: a variant added to the vocabulary belongs in this list, and
        96	/// its kernel in the pinned bands.
        97	#[test]
        98	fn bands_and_op_roster_name_the_same_kernels() {
        99	    let roster: Vec<Op> = vec![

Resolution: Make totality a compile error without a new dependency: an exhaustive `match op { Op::ClockSeed { .. } => (), ... }` over the roster's variants in the test (or a `fn representative(op: &Op) -> Op` the test walks), so a new variant fails to compile until rostered; or colocate the roster beside `Op::kernel` in `ops.rs` as `pub const REPRESENTATIVES` so both edits land in one diff. Acceptance: adding an `Op` variant without a roster entry fails to compile or fails a test by name; the convention sentence is gone.
Construction: Add `Op::VersionNoop { src: Reg }` with `kernel() => "ff_version_noop"`, omit it from the roster and from every strategy: `bands_and_op_roster_name_the_same_kernels` stays green and no test names the unpriced kernel.

## Positives

- Two one-sided residual widths with separately priced margins (fit.rs:13-23; bands.rs:161-194): the ceiling carries the regression claim with a tight margin priced from enforcement-context replays, the floor carries liveness with a wide margin sized between the dead-meter reading and the legitimately cheap tail, each margin names the two measured edges it sits between, and `bin/calibrate` re-derives both on every run. Derivation beside mechanism, exactly as the doctrine asks.
- `REGS_RESERVE` is tied to both budgets at compile time (wasm.rs:39-47): a calibration constant that could have drifted into a mid-measurement reallocation is a compile error when a budget is raised; the seed that found the reallocation artifact is committed.
- Every judgment leg ships a synthetic tripwire that states what it does not prove: the burner test (enforce.rs:260-310) is explicit that it proves the wasm-to-fuel-to-judge path and nothing about generator reach; `fit/tests.rs` proves the comparator reads its perturbation exactly; `curve/tests.rs` proves fire, quiet, and abstain; `judgment_flags_quadratic_and_dead_readings` proves both flags.
- The deterministic prefix as a total verdict (enforce.rs:328-343) with the `(1 - q)^48` argument makes the difference between sampled and total judgment concrete, and the same programs feed the refit, so the leg's cost is the verdict rather than runtime.
- The small-band leg carries its own liveness floor (enforce.rs:232-239): a rostered kernel that never lands a step inside its calibrated span fails by name instead of passing as decoration.
- `REFIT_COVERAGE` is a committed expectation list with classification-flip detection (bands.rs:977-1038; enforce.rs:367-395): coverage decay, a reach regression, and line drift each fail by name, and the splice-marker assertion in calibrate (calibrate.rs:347-353) fails by name rather than silently regenerating an empty file.
- Identity-outcome routing (drive.rs:57-66) names where the fast paths' liveness lives (before's `identity_fast_paths` meter pins, verified present at crates/before/tests/meter.rs:10552) rather than leaving them unpriced without an owner.
- The blessed-drift window (bands.rs:42-70) is disclosed candidly with its mechanism, its measured size, and the reason it is the price of the margins; the doc argues against its own instrument where it must.
- Determinism is stated as a model (wasm.rs:4-11): fuel as a pure function of guest bytes, call sequence, and payloads; fresh guest per program; every downstream choice, including the pooled allocator's per-slot reset, is justified against it.
- The committed seed file corresponds to the live property (six entries, each a `program = [...]` drawn by `any_program()`), anchored where `tests/seed_liveness.rs` resolves by the deliberate `tests/main.rs`.
- `fit_constant`'s panic (fit.rs:194-210) is a genuine programmer-error guard with a one-line proof, and `judge_small`'s two-sided span cut (bands.rs:295-298) is justified in place.
- The detached workspace carries its own fmt and clippy leg (justfile:588-597) because the root sweep cannot reach it, and the 08-04 re-pin's commit message attributes all 188 constants to the toolchain change with "nothing else moved" checked against the board, the envelopes, and the trybuild snapshots: provenance kept in git the way the doctrine asks.

## Open questions for Finch

1. Re-pin cadence. The last re-fit is e7a4b7b0 (2026-08-04); substrate changes to `before/src` and the harness's own allocator model have landed since, all inside the drift window with the gate green. Is red-triggered re-pinning the intended cadence, or should substrate changes re-pin so the pin of record describes the code that runs? This interacts with findings 4 and 5. Recommendation: since the corpus is byte-reproducible and a re-pin costs only a diff review, re-pin at each change that touches the walked paths, and let a per-key slack (finding 5) decide when a re-pin is mandatory rather than the global 0.7.
2. Shape-leg reference (finding 10). If the reference moves to `min(band.slope, 1.0)`, does any lane legitimately trend above 1.0 within a case (the note names `ff_rank_display`'s digits-times-limbs conversion)? Recommendation: one `bin/calibrate` run with the new reference answers it and names the rows needing a declared exponent; declare those per key rather than loosening the global allowance.
3. `crates/before/AGENTS.md` (45 lines) never names the fuzzfit workspace, the `just fuzzfit`/`fuzzfit-calibrate` recipes, or the re-pin discipline, though `rust-toolchain.toml`'s own comment routes toolchain bumps through `just fuzzfit-calibrate`. Recommendation: one guidepost line pointing at the justfile recipes and `bands.rs`'s module doc.
4. `REFIT_COVERAGE` lists 48 of the 49 band keys; the uncovered one is `ff_rank_checked_sub`'s underflow arm, the pin's only flat-slope rejection arm (verified). Is that intended (too few prefix samples at pin time) and worth a note? Recommendation: have calibrate emit the uncovered keys with their reason into the generated region as part of `PIN_EVIDENCE` (finding 2), so the reader does not need the stderr of a run they did not perform.
5. Reach demonstrations (finding 24). The five known-bad reconstructions that accepted the instrument live in git history by the note's §8 design; the tree's standing demonstrations prove the judge on synthetic bands. Should one real-kernel sabotage demonstration per genre be committed (a guest switch adding a `black_box`-pinned burner to a named kernel, judged red on `ESCALATION_REPLAYS[0]`)? Recommendation: the doc fix regardless; the sabotage tests only if the owner wants the reach argument standing in the tree at the cost of one re-pin.
6. Dead-reading reference (finding 17). The floors are compared to `ff_nop` (2 fuel); a stubbed pair kernel that still borrows two registers costs more than that, so the `ff_rank_cmp` floor (about 2.6 fuel at 128 bits) holds liveness only against a return-immediately stub. Recommendation: measure a dispatch-shaped control kernel once before restating the floor doc; if the `ff_rank_cmp` floor sits under it, say so at `ENFORCE_MARGIN_BELOW`.
7. Do the six committed seeds still replay in-band under the 1.97.1 pin (bands.rs:113-115 claims the 139-bit overlap seed does)? Not runnable under this review's rules; a green `just fuzzfit` at HEAD settles it.
8. `SMALL_BAND_KERNELS` prices four kernels below 128 bits on a consumer-relevance argument; the other 40 kernels are unjudged sub-floor. Recommendation: leave the scope as-is (the premise, rumors' bootstrap hot path, is stated at the constant) unless rumors' hot path grows; the argument for the four is the argument against widening.
9. Informational: the design note (a dated record, exempt from the present-tense rule) states `ENFORCE_MARGIN_BELOW` as 1.0 (lines 383, 425) where the code pins 0.8, and names the seed path `enforce.proptest-regressions` (line 462); a reader cross-checking finds the discrepancies without a pointer to 875c118b, the commit that moved the margin. The fuzzfit tier's absence from CI (ci.yml:117-120) is deliberate and documented there; 4e64a4fb's message records a clean local gate for the wasmtime bump, so that question is answered for that bump.

## Dropped

- Guest toolchain provenance test checks the harness's compiler as a stand-in for the guest's [54]: deliberate and documented at the code site (build.rs:4-7 states the stand-in; e7a4b7b0 pinned one toolchain for both halves; wasm.rs:94-95 names `FUZZFIT_GUEST_WASM` as explicit operator provenance). A hardening suggestion, not a defect.
- No-op `as u64` casts on `encoded_bits()` [59]: out of partition (ops.rs belongs to the ops/strategies partition); forwarded there. Verified `encoded_bits` returns `u64` in version.rs:1151, party.rs:597, clock.rs:852; whether `just fuzzfit`'s clippy leg flags the same-type cast was not run here.
- Pin-time evidence prose drift [18], [28], [49]: duplicates of fuzzfit-bands-2.
- Shape leg judges against the mixture-inflated pooled slope [16]: duplicate of fuzzfit-bands-10.
- Noisy-linear test doc overstates its jitter [58]: duplicate of fuzzfit-bands-12 (its "~+0.12" was imprecise; +0.082 is the reproduced value).
- Seed-location ghost references [22], [35], [51]: duplicates of fuzzfit-bands-20.
- PROPTEST_CASES override claim [34], [50]: duplicates of fuzzfit-bands-21.
- Hand-maintained probabilities and counts [27], [36]: duplicates of fuzzfit-bands-26.
- Kernel roster hand-maintained [25], [40], [55]: duplicates of fuzzfit-bands-30.
- call_i64 second call path [33]: duplicate of fuzzfit-bands-15.
- Splice marker and duplicated rustdoc [41]: duplicate of fuzzfit-bands-18.
- probe.rs not in the gate / fuel determinism untested [17], [32]: duplicates of fuzzfit-bands-19.
- wasmtime pin convention unenforced [38]: duplicate of fuzzfit-bands-6.
- Floor evidence at min_denom only, skipping small bands [37], [53]: merged into fuzzfit-bands-17.
- Duplicated bucket medians, MIN_* constants, diag stream loop, Fit/Band transcription [23], [24], [29], [30], [31], [57]: merged into fuzzfit-bands-11.
- Em-dashes and past-tense narration in wasm.rs [45]: merged into fuzzfit-bands-14.
- Reach demonstrations not committed [3] as a medium verification-gap: converted to fuzzfit-bands-24 (documentation, low) because the design note records the decision that the demonstrations live in git history and the defenses they forced stand in the tree; the design question is carried as open question 5.
