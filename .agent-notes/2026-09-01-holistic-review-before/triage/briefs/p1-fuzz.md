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

- **Base.** Your worktree's HEAD must equal `0fc1921e` before you start.
  Run `git -C <worktree> rev-parse HEAD`. If HEAD is an ancestor of
  `0fc1921e`, fast-forward; if it has diverged, stop and report. Never call
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
   `DETERMINISTIC_MARGIN`) each with its synthetic red case, then the
   vocabulary extension and one `just fuzzfit-calibrate` re-pin as the
   last commit of the lane, attributed in full.

Re-pin discipline for step 3: one calibration run, at the end, after
every criterion change has landed with its own red case; never iterate
on the calibration. The bands file carries a provenance note binding the
pin to the wasmtime version and the guest toolchain; state both in the
re-pin commit. `p1-gate` may bump wasmtime in the detached lockfiles
(deps-9); if it has landed, rebase before calibrating so the provenance
note names the version you ran.

Files shared: `fuzzfit/harness/src/ops.rs` (the gate lane removes two
casts at :764 and :858; rebase over that small commit); `bands.rs` (the
suites lane's ruling 10 adds a rank fuel fit past 65 KiB; that lane is
told to land after you).

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
