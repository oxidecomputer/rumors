<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P1 lane: the fuzz-fit vocabulary, the fuzz gate legs, and the wasm32 terminal pins

## Goal

Three detached workspaces hold the instruments that price `before` on
shapes nobody chose, and each has a gap between what its prose claims and
what it judges: the fuzz-fit bands are described as the cost law for every
public operation while pricing 44 kernels of the 107 the guest exports,
and their shape leg compares a within-case slope against a mixture-tilted
pooled fit; the fuzz targets' own oracles execute only at `just all`
cadence and the fuzz workspace is never linted; the wasm32 memory-terminal
pins are labeled as red baselines awaiting a cure while their docs call the
terminal intended, and they assert a trap genre every guest abort produces.
The invariant restored: the fuzz-fit vocabulary is bound to the method
surface with a reviewed exemption list, and every leg judges against the
reference its own prose names; the fuzz seeds replay per commit and the
workspace is linted like its siblings; the terminal pins state the
address-space bound as the model and discriminate allocation failure from
any other abort.

## Ground rules

These apply to every P1 lane; the lane sections below add to them and
never relax them.

- **Base.** Your worktree's HEAD must equal `bba0e31a` before you start.
  Run `git -C <worktree> rev-parse HEAD`. If HEAD is an ancestor of
  `bba0e31a`, fast-forward; if it has diverged, stop and report. Never call
  EnterWorktree; operate on the worktree through `git -C <path>` and
  absolute paths, one shell invocation at a time.
- **The review documents are the specification.** Each member entry below
  quotes its Resolution and Acceptance verbatim from the class document in
  `.agent-notes/2026-09-01-holistic-review-before/`; the entry's full
  record (evidence, construction, witness outcome) is under `### <id>:`
  there (`#### <id>:` in `simplification.md`; nits in `documentation.md`,
  `simplification.md`, and `verification.md` are one-line table rows) and
  in `evidence/`. Line anchors are at the reviewed commit `9e5784fb`, and
  the tree outside `.agent-notes/` is byte-identical at your base, so they
  hold; re-anchor from the quoted evidence, never from a line number
  alone, once your own commits move the file.
- **The rulings govern.** `triage/rulings.md` records Finch's decisions.
  Where a quoted Resolution offers alternatives, the ruling named beside
  it picks one; where the ruling amends the Resolution, the amendment is
  stated under the quote and wins. Where a quoted Resolution and the lane
  goal come apart, the goal wins, and the discrepancy is reported.
- **Typed references, never strings (ruling 43).** Wherever this lane
  touches `meter/registry.rs`, a family roster, `TRIPWIRE_ROSTER`, the
  surface rosters, or any test that names another test, file, or line:
  reasons, pins, and enforcement homes are expressed as references the
  compiler resolves (function items, registered law names, `Shape` and
  `Op` values), never as strings naming a test function, a file, or a
  line number. A lane that sees a cleaner idiomatic shape for a roster is
  authorized to adopt it and reports the reshaping in its diff. Finch's
  words: "please make these instruments impossible to drift in the
  future. I *really don't like* the pattern of hard-coded strings and
  Rust source locations embedded in tests; the way these family rosters
  ended up is not really to my taste, but I haven't had time to make it
  more idiomatic and obviously correct. If you see a good way to clean it
  up, please do."
- **Prose (ruling 105).** Every paragraph you touch passes the three tests in
  `PROSE.md` (altitude, concision, legibility); the reviewer applies its
  checks; the diff is net shorter in prose unless your report says what the
  additions buy.
- **Stops.** Report and leave the entry open; do not work around: anything
  that moves an `insta` snapshot or a committed pin the brief does not
  name as moving; any change to a public signature or public rustdoc
  contract the resolution does not name (the meter surface under
  `any(test, feature = "meter")` is instrument surface by ruling 8 and
  not public API, but a change there lands with the `rumors` test update
  in the same commit); anything that contradicts a ruling; any deviation
  from the stated resolution; anything this brief marks as a stop. A stop
  on one entry does not block the others.
- **Negative controls.** Every repaired or added instrument lands with a
  committed demonstration that a known-bad artifact fails it. The
  constructions in `evidence/witness.md` and each entry's Construction
  line are those artifacts; convert each into a committed test
  (`should_panic`, an asserted `Err`, a judge test over synthetic samples,
  or a reversible mutation whose observed failure the commit message
  records verbatim). Ruling 20 is the one place this brief set says
  otherwise, and it says so at the entry.
- **Resource discipline.** Build with `cargo nextest run --no-run` before
  running; during iteration run only the binaries the entry names; never
  iterate on a timing measurement; launch no daemons or services. One full
  `just gate` per agent, before the final commit, run in the background
  redirected to a log under `<scratchpad>/<lane>/` and polled with short
  foreground checks (the foreground command cap is ten minutes; a
  foreground gate run cannot finish). Keep every working file and evidence
  log under that directory, never loose at the scratchpad top level.
- **Commits.** One commit per logical unit, its message describing the
  change and naming the entry ids and the ruling. Every re-pin is measured
  at the parent and its movement named in the commit. Commit every
  proptest seed file that appears. Prose speaks in the present tense: no
  reference to code that no longer exists, no dated rationale at a
  declaration site. Comments use spaced double-hyphens, never em-dashes;
  every test has a doc comment stating its invariant. Commit with a
  descriptive message before finishing.
- **Never delete anything outside your worktree.** If the disk fills
  (ENOSPC), stop and report; it is the coordinator's problem, not a
  reason to trim caches you do not own.
- **Self-retirement.** After the final commit, from inside the worktree:
  `cargo metadata --no-deps --format-version 1 | jq -r .build_directory`,
  then delete that directory and the worktree's `target/`. Leave the
  worktree in place.
- **Report.** For each entry: landed, stopped, or open; the commit sha(s);
  the acceptance evidence (the command and its decisive output, verbatim).
  Then anything left open and why, and every deviation from a stated
  resolution, explicitly. Your report is data: the coordinator verifies
  each entry's Acceptance against the tree at the reported sha before the
  ledger records it. Report what you could not do rather than working
  around it.

## Ordering inside the lane

1. The fuzz workspace legs (ruling 15): the seed replay leg and the
   clippy leg, red-first (the flipped predicate, the planted lint), then
   the flat-cap doc restatement.
2. The wasm32 pins (ruling 17): the discriminator first (red on the
   construction's planted `assert!`), then the renames and restatements.
3. The fuzz-fit work (ruling 14): the parity test against
   `METHOD_SURFACE` red-first (naming the 49 uncovered kernels), the
   riders that change criteria (shape-leg reference, `REFIT_SLACK`,
   `DETERMINISTIC_MARGIN`) each with its synthetic red case, the
   floor-over-nop enforcement (fuzzfit-bands-17, ruling 50) with its
   synthetic red case, the multi-scale rank band past 65 KiB
   (skyline-query-9, ruling 10) added to the vocabulary, then the
   vocabulary extension and one `just fuzzfit-calibrate` re-pin as the
   last commit of the lane, attributed in full.
4. The overflow-checks self-test (fuzz-guests-pins-26, ruling 50) rides
   step 2 (it uses the discriminator); the seed derivation fix
   (tests-other-26, ruling 50) rides step 1, before the seed-replay leg
   lands, so the replay leg's first green run is over the regenerated
   seeds; the fuelscape overlay twins (fuelscape-pipeline-23, ruling 50)
   are independent and may land at any point.

Re-pin discipline for step 3: one calibration run, at the end, after
every criterion change has landed with its own red case; never iterate
on the calibration. The bands file carries a provenance note binding the
pin to the wasmtime version and the guest toolchain; state both in the
re-pin commit. `p1-gate` may bump wasmtime in the detached lockfiles
(deps-9); if it has landed, rebase before calibrating so the provenance
note names the version you ran.

Files shared: `fuzzfit/harness/src/ops.rs` (the gate lane removes two
casts at :764 and :858; rebase over that small commit); `crates/before-fuelscape/src/families.rs` (fuelscape-pipeline-23;
`p1-gate`'s deps-9 sweep runs `just fuelscape-test` but edits no
fuelscape source). The rank fuel fit past 65 KiB (skyline-query-9,
ruling 10) is this lane's, so `bands.rs` has one owner.

## Members

### fuzzfit-strategies-7 (medium, verification-gap): ruling 14

Resolution: bind the `Op` vocabulary to `before::surface::METHOD_SURFACE` the way fuelscape does (enable the `surface` feature on the harness's `before` dependency; add a parity test requiring every method-surface row to have either an `Op` whose kernel is pinned in `BANDS` or a one-line reviewed exemption naming why it is outside the register-machine vocabulary). Add `Op`s and strategy emissions for the operations that fit the vocabulary today (`ticks`, `forks`, the `*_all` doors, `span`/`span_all`, `eq`, the `Rank` codec, `Ranked`, clock text I/O) and re-pin with `just fuzzfit-calibrate`; exempt the rest by name. Acceptance: a committed parity test fails by name for any `METHOD_SURFACE` row with neither an `Op` nor an exemption; the 49-kernel gap shrinks to an explicit exemption list; `just fuzzfit` green after the re-pin.

Ruled (14): as stated. The exemption list is "reviewed": each entry
names the mechanism that puts the operation outside the register-machine
vocabulary, and the list is reported in full so Finch can review it at
the diff. Operations you judge to fit the vocabulary but do not add are
a deviation to report, not an exemption to write.

### meter-adequacy-2 (medium, verification-gap): ruling 14

Resolution: (a) re-state justfile:569-570 to what the bands cover (the vocabulary in `bands.rs`, not every public operation); (b) add a surface-tiling test to the fuzz-fit harness in the fuelscape idiom (every `before::surface` row either reached by some `Op` kernel or carrying a reasoned exemption), so the 56 uncovered kernels fail by name until each gains an `Op` or an exemption; (c) owner's call on which operations join the vocabulary. Acceptance: the tiling test committed and green with an explicit exemption table; the justfile comment no longer says "every public operation".

Ruled (14): one change with fuzzfit-strategies-7; (c) is answered by
that entry's list of operations that fit today.

### fuzzfit-bands-27 (medium, claim): ruling 14

Resolution: Now: restate the three claims as "every operation in the program vocabulary ([`ops::Op`])". Then add a tiling test (sibling to `bands_and_op_roster_name_the_same_kernels`) that walks `before::surface::METHOD_SURFACE` and requires each row to be either a vocabulary kernel or an entry in a committed exclusion list stating its mechanism, so a new public operation cannot land unpriced and unexcused. Owner-gated: extend the vocabulary to the exported-but-unbanded kernels (the span algebra and causally kernels already exist in the guest because the fuzzfit prices them; the generator work is the missing piece). Acceptance: the tiling test fails when a surface row is removed from both the vocabulary and the exclusion list; each exclusion names a mechanism; no doc says "every public operation" unless the vocabulary covers the surface.

Ruled (14): one change with the two entries above. Once the vocabulary
is bound to the surface with exemptions, the three claims (enforce.rs:441,
the crate doc, the justfile) may say the bands price every public
operation not named in the exemption list, and must say no more.

### fuzzfit-bands-10 (medium, verification-gap): ruling 14

Resolution: Judge the within-case slope against the claimed law rather than the pooled fit: for non-fold keys `excess = local - band.slope.min(LINEAR_CLAIM)` with `LINEAR_CLAIM = 1.0`, or a per-key declared exponent where a documented superlinear within-case mechanism exists (the note names `ff_rank_display`'s digits-times-limbs conversion as a candidate). Re-derive `SLOPE_ALLOWANCE` from `bin/calibrate`'s shape-leg evidence under the new reference (the code path at calibrate.rs:221-229 exists; only the reference changes), and rewrite curve.rs:64-70 to name the reference actually used. State the per-row escaping-exponent bound in the blessed-drift section so the instrument's reach is documented where its criterion lives. Acceptance: a committed `curve/tests.rs` case builds `fuel = L(128) · (d / 128)^1.4` over 128..8768 against `band_for("ff_version_decode", false)` and asserts `local_slope_excess > SLOPE_ALLOWANCE` (today it reads +0.225 and passes); calibrate's recomputed maximum healthy excess stays under the allowance on the 4096-program corpus; the suite stays green at the current pin.

Ruled (14): the shape leg judges against `min(band.slope, 1.0)` with
per-key declared exponents where a documented superlinear within-case
mechanism exists. Every declared per-key exponent names its mechanism at
the declaration. The `SLOPE_ALLOWANCE` re-derivation is part of the one
calibration run.

### fuzzfit-bands-5 (medium, verification-gap): ruling 14

Resolution: Generate `REFIT_COVERAGE` as `(kernel, rejected, divergence_at_min, divergence_at_max)` carrying the signed pin-time endpoint deltas; in the enforcement test compute the fresh deltas and assert `|fresh - pinned| <= REFIT_SLACK` at each endpoint, with `REFIT_SLACK` set deliberately as the cadence parameter (0.2 matches `ENFORCE_MARGIN` and catches k = 2). Keep `REFIT_TOLERANCE` only if the coarse absolute bound is also wanted. Re-denominate the blessed-drift paragraph from the slack. Acceptance: a test that halves `Measured.fuel` for one kernel inside `Guest::call` reads red on the deterministic prefix by name; on unchanged code every pinned delta reproduces to floating-point noise.

Ruled (14): as stated, with one global `REFIT_SLACK` as the cadence
parameter (the ruling's "one global `REFIT_TOLERANCE`" means one global
slack constant, not one per key; the per-key values are the generated
pinned deltas the slack is applied to). Drop `REFIT_TOLERANCE` unless you
find a case the per-key slack does not cover; if you keep it, say which
case.

### fuzzfit-bands-4 (low, verification-gap): ruling 14

Resolution: Thread the margins through `judge` (or add `judge_against_with(band, d, fuel, margin_above, margin_below)`), pass `ENFORCE_MARGIN` for the sentry, the escalation replays, and the bootstrap leg's main-band judgments, and a documented `DETERMINISTIC_MARGIN` (sized for the printed constants' rounding, or a deliberate x1.1) for the prefix leg and the small-band judgments; state the two windows separately in the blessed-drift section. Acceptance: the blessed-drift doc states two windows; a committed synthetic case shows a uniform x1.5 drift on one key reads `Above` on the prefix while still inside the sentry's margin; the gate stays green at the current pin.

Ruled (14): as stated. Size `DETERMINISTIC_MARGIN` from the printed
constants' six-decimal rounding and state the derivation at the constant.

### fuzzfit-strategies-1 (nit, performance): ruling 14

Resolution: introduce a guest-only profile (`[profile.guest] inherits = "release"` carrying both settings, built with `--profile guest`) or a per-package override for the heavy tool dependencies; move `fuzzfit_guest_wasm` in the justfile and the provenance note in bands.rs with it; measure `just fuzzfit-build` wall time before and after on a quiet machine. Acceptance: the guest wasm is byte-identical under the new profile and the harness build no longer compiles wasmtime single-unit.

Ruled (14): the guest-only profile. The guest wasm's byte identity
(`sha256` before and after) is the acceptance you provide; the wall-time
measurement is not run in this lane (no timing iteration, no quiet
machine); state that in the commit and leave the number to the
coordinator.

### fuzz-guests-pins-38 (medium, verification-gap): ruling 15

Resolution: Add a gate leg beside `fuzz-build` that replays the committed seeds through each built target once: `cargo +nightly fuzz run --target <host> <target> seeds/<target> -- -runs=0` executes every corpus input and exits; deterministic, seconds, and it turns the seed corpus from a smoke-time asset into a per-commit oracle liveness check. Acceptance: inverting `(_, c, f) if c == f => true` in `span_differential` reddens the new leg on the committed `span_ordered` and `span_crossed` seeds; the committed targets pass it.

Ruled (15): as stated; spell the toolchain `+{{ nightly_toolchain }}`
like the sibling recipes, never a floating `+nightly`. Record the
inverted-predicate red run in the commit message.

### fuzz-guests-pins-39 (low, verification-gap): ruling 15

Resolution: Add `cargo clippy --all-targets -- -D warnings` to `fuzz-build`, mirroring wasm32-pins:642-643; if stable clippy cannot build the `#![no_main]` bins, pin a clippy component on the nightly toolchain instead. Acceptance: a `clippy::needless_borrow` planted in a fuzz target reddens `just gate`.

Ruled (15): as stated. If the first clippy run over the untouched fuzz
workspace reports findings, fix them in the same commit and list them.

### fuzz-guests-pins-14 (medium, verification-gap): ruling 15, amended

Resolution: Owner rules the shape (the note's two branches). If proportional: `cap = max(FLOOR_BYTES, PER_INPUT_BYTE * data.len())`, threading `data.len()` into `under_heap_cap`, with `PER_INPUT_BYTE` measured over the seeds and a smoke run first and committed with slack, so the doc's "constants in the hundreds" becomes an enforced number. If ratified flat: restate the doc positively without "not yet" and name the class the flat cap is for. Either way, add a `#[test]` to the fuzz lib (plain `cargo test --lib` in the workspace, runnable from `fuzz-build`) that `under_heap_cap` panics on a synthetic over-cap allocation and passes on a linear one. Acceptance: the lib test is green in the gate; under the proportional rule, `let _sink = vec![0u8; data.len() * data.len()];` planted in a target's `run` crashes on fuzzer-generated inputs near `-max_len` (not on the tiny seeds); "not yet" is gone.

Ruled (15): the flat cap is ratified. Take the "ratified flat" branch
only: restate `fuzz/src/lib.rs:10-17` positively, without "not yet", and
name the class the flat cap is for (survivable amplification in the
[1 GiB, 2 GiB) window between the cap and libFuzzer's RSS limit). The
"either way" `#[test]` that the cap fires is not landed under this
ruling; the acceptance is the doc restatement and `grep -n 'not yet'
crates/before/fuzz/src/lib.rs` empty. The proportional cap and its
planted-quadratic acceptance are not taken.

### fuzz-guests-pins-33 (medium, claim): ruling 17

Resolution: Owner rules the genre. If the address-space bound is the model (the evidence reads that way): drop "PINNED AS FOUND" from the three, rename them to state the behavior positively (for example `version_decode_past_address_space_aborts_loudly`), state the working-set multiple they pin, narrow the header and harness/src/lib.rs:23-26 to distinguish red-first seam pins (none today) from declared terminals, and pair the restatement with the origin discriminator from finding 35 so the instrument, not the prose, establishes "allocation failure". If a leaner working set is a planned cure: say so at each pin and track it as open work with its target multiple, and consider whether a heap envelope in `tests/meter.rs` on decode's working set is the right home for the constant factor, leaving the wasm32 leg to seams. Acceptance: every trap pin in the file is either a seam with a tracked cure or a positively stated declared model, and the header's contract is true of every pin below it.

Ruled (17): the address-space bound is the model. The first branch in
full: the three pins renamed to state the behavior positively, the
working-set multiple they pin stated at each (measure it: the input size
at the terminal against the 4 GiB address space, read from the pin's own
arguments), the header and `harness/src/lib.rs:23-26` narrowed to
distinguish red-first seam pins from declared terminals, paired with
fuzz-guests-pins-35's discriminator. The pins' doc is the home of the
model.

### fuzz-guests-pins-35 (medium, verification-gap): ruling 17

Resolution: Record the abort genre before the trap: install a `std::panic::set_hook` at each export's entry that sets a `static PANICKED: AtomicBool` (std's alloc-error path calls `abort` directly and never runs the panic hook, so the flag separates the two), expose it through `pin_panicked() -> i64` or a fixed linear-memory address the harness reads from the store after the trap, and widen `Outcome::Trapped` into panic-versus-abort so the terminal pins assert the allocation-abort genre and every other pin asserts no panic flag. A cheaper second discriminator: read `Memory::size(&store)` after the trap and assert linear memory grew to within a page budget of 4 GiB. Confirm wasmtime's behavior for calling exports in the same store after a trap before choosing. Acceptance: a guest whose `pin_version_decode` panics for `n_bytes >= 1 << 30`, or spells `Bits::len` as a `usize` product, turns all three terminal pins red; the committed guest keeps them green.

Ruled (17): the discriminator lands. Confirm wasmtime's post-trap
behavior in the same store first, as the resolution says, and choose the
discriminator (the panic flag, the memory-size read, or both) by what
that confirmation allows; state the finding in the commit. The planted
`assert!(n < 1_000_000_000)` construction is the negative control,
recorded in the commit message with all three pins' observed red.

### fuzz-guests-pins-26 (medium, verification-gap): ruling 50

Resolution: Add `pin_selftest_overflow() -> i64` to the guest that computes `black_box(u32::MAX) + black_box(1u32)` (and a `usize` variant) and a harness test asserting `Outcome::Trapped(Trap::UnreachableCodeReached)` (with the panic-genre discriminator from finding 35, asserting the panic genre). Name it beside `version_small_roundtrips_and_rejects_typed` as the leg's second liveness pin. Acceptance: `CARGO_PROFILE_RELEASE_OVERFLOW_CHECKS=false just wasm32-pins` turns the self-test red; the default build keeps it green.
Construction: Run `CARGO_PROFILE_RELEASE_OVERFLOW_CHECKS=false just wasm32-pins`: every existing pin passes, because no pin's observation is a wrapped quantity and no export exercises a wrap on purpose.

Ruled (50): as stated, with the panic-genre discriminator from
fuzz-guests-pins-35 (ruling 17) asserting the overflow panic genre. The
negative control is the acceptance's own: one build with
`CARGO_PROFILE_RELEASE_OVERFLOW_CHECKS=false` turning the self-test red,
recorded in the commit message; the default build keeps it green.

### fuzzfit-bands-17 (medium, verification-gap): ruling 50

Resolution: Add an enforcement test that calls `ff_nop` on a live `Guest` and, for every band in `BANDS` and `SMALL_BANDS`, asserts `intercept + slope · log10(d) - width_below - ENFORCE_MARGIN_BELOW > log10(nop)` at both `d = min_denom` and `d = max_denom` (the endpoint argument `line_divergence` already uses). Make calibrate's loop evaluate both endpoints over `fits` and `small_fits` and fail rather than print when any gap is non-positive; drop the caveat once the code no longer needs it. Consider measuring a dispatch-only control kernel (borrow two registers, return) once, so the `ff_rank_cmp` floor's liveness claim is stated against the dead reading a stubbed pair kernel actually produces (see open question 6). This composes with finding 2's `PIN_EVIDENCE.floor_min_gap`. Acceptance: a committed synthetic case (the pinned `ff_rank_cmp` band with `width_below` widened by 0.12) reads `InBand` for `fuel = nop` at 128 bits and the new check rejects such a band by name; calibrate exits non-zero on a non-positive gap; the printed minimum covers 53 floors at both endpoints.
Construction: Copy the pinned `ff_rank_cmp` `Band` (bands.rs:700-711) into a test, add 0.12 to `width_below`, and call `judge_against(&band, 128, 2)`: the result is `InBand`. Nothing in `tests/enforce.rs` or `tests/sanity.rs` would fail if calibrate emitted such a band; the judgment tripwire at sanity.rs:186-217 and the burner test at enforce.rs:273-310 use synthetic bands and never touch the pinned floors.

Ruled (50): the enforcement test over both rosters at both endpoints,
`calibrate` failing rather than printing on a non-positive gap, the
caveat dropped, and the widened-band synthetic case committed as the
known-bad the check rejects by name. The dispatch-only control kernel the
entry suggests considering is not taken in this lane; report if the
`ff_rank_cmp` floor's gap over nop is under the margin at the current
pin, since the re-pin at the end of the lane must then clear it. This is
a criterion change: it lands before the one calibration run.

### tests-other-26 (medium, correctness): ruling 50

Resolution: Capture `let concurrent = sibling.version().clone();` before the sync (or build the message from a sibling that has not synced) and use it for the flavour-1 payload and the laws family's second version; assert the relation at the derivation (`assert!(clock.version().concurrent(&concurrent))`) and in the ops contract test of tests-other-18; regenerate with `cargo run -p before --example fuzz_seeds` and commit the changed seed files. Acceptance: the relation assertion fails against the current derivation and passes after the reorder; `committed_seeds_match_the_live_derivation` is red until regeneration and green after; the two comments read true of the bytes.
Construction: After fuzz_seed_set.rs:275 add `assert!(clock.version().concurrent(sibling.version()));`. It panics: `sync` joins both histories into both clocks, as the `Clock::sync` doc example asserts with `assert_eq!(a.version(), b.version())`.

Ruled (50): as stated. Capture the concurrent sibling before the sync,
assert the relation at the derivation, regenerate with `cargo run -p
before --example fuzz_seeds`, and commit every changed seed file. The
negative control is the acceptance's own: the relation assertion fails
against the parent's derivation (record it), and
`committed_seeds_match_the_live_derivation` is red until regeneration.
Land before the seed-replay leg (fuzz-guests-pins-38) so that leg's
first green run is over the regenerated seeds.

### fuelscape-pipeline-23 (medium, correctness): ruling 50

Resolution: On the `[Version, Version]` arm replace the two self-pairs with perturbed twins that defeat the equality rung while keeping the shape (the family and the same family after one tick on its first leaf, or dense(t) crossed with dense at the next ramp point), or drop them and rely on the three committed pair generators already present; do the same for the `[Party, Party]` self-pairs on rows with an early-exit predicate. State at `overlay_inputs` which rows own an equality short-circuit and that self-pairs are reserved for rows without one (version_cmp, version_concurrent, party_covers, the contains rows). Rewrite families.rs:91-92 to describe what the arm does. Acceptance: a committed test in a families.rs sibling `tests.rs`: for every `[Version, Version]` and `[Party, Party]` roster row, every `overlay_inputs` point has `inputs[0] != inputs[1]` unless the row is in an explicit allowlist of rows without an equality rung in the measured kernel.
Construction: Build `Plan { base_seed: 0x5eed, samples_per_column: 1, max_bytes: 64 }`, call `overlay_inputs` on the version_join row at 64, take the largest "dense × self" point, and run `(op.measure)(&mut Guest::new(), &fam.inputs, 2)`; compare its fuel with the `version_eq` row measured on `inputs[0]` at the same size. The two agree up to the clone's constant and both sit far below the "jump_pair" point at the same total size, which is the join sweep's cost.

Ruled (50): perturbed twins (the family after one tick on its first
leaf, or dense crossed with dense at the next ramp point) on every
two-operand row whose kernel opens with an equality rung, on both the
`[Version, Version]` and `[Party, Party]` arms; the allowlist of rows
without a short-circuit is explicit at `overlay_inputs`; the committed
distinctness test asserts every other overlay point has distinct
operands. `families.rs:91-92` is rewritten to what the arm does. The
compact datasets and the SVG gallery carry the old points; regenerating
them is a fuelscape survey (hours) and is not run in this lane: report
that the committed datasets still carry the self-pair points and leave
their regeneration to the coordinator.

### skyline-query-9 (low, verification-gap): ruling 10 (moved here from the suites lane)

Resolution: Either instrument or narrow. To instrument: a multi-scale fuel fit (the fuzzfit harness already counts wasm fuel for `ff_version_rank`) over the doc's tight construction at three or more scales past 65 KiB, judged against the `M(n) · log n` model with `M ≈ n log n`; or a deterministic check that records `meter_product`'s operand widths per tree level on that construction and holds the per-level product widths to the model's telescoping. To narrow: state in the public contract only what committed instruments pin and move the quasilinear-tier remark to a decision record. Acceptance: either a committed cell or test whose operands' parked sums exceed 4,000 words at every scale, named from integral.rs in place of the "[derived; ...]" bracket and rostered, or the public `# Complexity` no longer carries the unwitnessed clause.
Construction: Build the doc's own worst case: `Θ(log |v|)` armings whose parked widths grow as `4,000 · 2^i` words, each banked ahead of a trailing window of span `Θ(|v|)`, at |v| of about 65 KiB, 130 KiB, and 260 KiB packed; run `Version::rank` under the fuzzfit fuel harness; fit the exponent across the three scales against `n log^2 n`. A settle that re-ran an NTT-tier product once per tree level without telescoping reads the extra `log` here and in no committed counter.

Ruled (10): instrument, by the multi-scale fuel fit in the fuzzfit
harness at three or more scales past 65 KiB over the doc's own tight
construction, judged against `M(n) · log n`. The public clause stands.
The band joins the vocabulary before the lane's one calibration run so
its pin carries the same wasmtime and toolchain provenance as every other
band's. If the fit reads the extra log where the model says it should
not, or fails to read it where the doc says it is tight, that is a
finding about the doc's argument: report the readings, do not re-word
the contract.

## Hazards and stops

- One calibration run, at the end. A band that reads red at the current
  pin after a criterion change, before the re-pin, is expected and is
  what the re-pin commit attributes; a band that reads red after the
  re-pin is a finding to report, not a second calibration.
- The fuzzfit bands' provenance (wasmtime version, guest toolchain, the
  release profile) is part of the pin. Every one of those you touch
  (fuzzfit-strategies-1's profile, deps-9's lockfile sweep if rebased in)
  is named in the re-pin commit.
- `fuzzfit/harness/src/ops.rs:764` and `:858`: the gate lane owns the two
  cast removals; rebase over them.
- The wasm32 guest's exports are a fixed ABI the harness reads; adding
  `pin_panicked` or a memory read changes the harness, not the pins'
  meaning. Any pin whose outcome changes for a reason other than the
  planted construction is a stop.
- No `just fuzz` smoke, no bench, no wall-time measurement in this lane.
