<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P6 lane: fuzz targets, the fuzz-fit guest and harness, the wasm32 pins

## Goal

The detached fuzz workspaces' per-module entries, landed per their Resolutions inside an approved roster, under rulings 14 to 17 (the vocabulary, the seed replay, the pins), 50, 77 (the framing file), and 88.

## Awaiting individual ruling

The coordinator is walking these mediums with Finch; nothing below lands for them until the ruling is appended here: fuzz-guests-pins-29, fuzzfit-bands-19, fuzzfit-strategies-11, fuzzfit-strategies-16.

## Roster summary

1 ruled (1 low); 4 medium awaiting ruling; 37 pending roster approval (23 low, 14 nit).

## Ground rules

These apply to every lane; the lane sections below add to them and
never relax them.

- **Base.** Your worktree's HEAD must equal `5328537c` before you start.
  Run `git -C <worktree> rev-parse HEAD`. If HEAD is an ancestor of
  `5328537c`, fast-forward; if it has diverged, stop and report. Never call
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
- **No lower bound on performance is pinned anywhere (ruling 88).** Every
  touch, scan, limb, or heap pin is a ceiling: a reading over it fails; a
  reading under it is an improvement that lands by tightening the ceiling
  with its attribution in the commit. The only floors are liveness floors
  derived from a mechanism's irreducible work, never from a measured
  reading. An entry whose Resolution asks for an exact pin or a measured
  floor is read as a ceiling plus any mechanism-derived floor it names.

## Ordering

Every P6 lane runs after the P1 to P5 and P7 lanes that touch its files have landed on main, or rebases onto them before its final gate run; the coordinator names the base SHA at launch. Lows and nits inside this lane's approved roster are swept without a question to Finch; every high and medium has, or awaits, an individual ruling. A change that would alter a rendered `before` doc panel is a stop (ruling 89). Follows `p1-fuzz` and `p2-widths` (the wasm32 pins). Owns `crates/before/fuzz/**`, `crates/before/fuzzfit/**`, `crates/before/wasm32-pins/**`.

## Members

### fuzz-guests-pins-12 (low, simplification): ruling 92

`drive_groups!` duplicates the in-tree `organic_drive!` selection macro arm for arm

- Owner-gated: yes: adds an exported macro to the `laws` instrument feature

Resolution: Move the eighteen selection arms into `laws.rs` beside `for_each_law_group!` as a macro taking the environment expression and the assertion macro (`assert!` here, `assert_laws!` in the tests), and have both consumers invoke it; fix the pool-index choice once. Acceptance: one spelling of the eighteen arms in the tree; a signature added to `for_each_law_group!` breaks the build in exactly one place.

Approved roster (92, decision 72): lands per the quoted Resolution inside this lane.

## Mediums awaiting individual ruling

Listed with their Resolution so the lane knows the files they touch; not landed until ruled.

### fuzz-guests-pins-29 (medium, verification): awaiting individual ruling

The rank pins observe only `0 < r < 1` and `r == r.clone()`, which a decoder that drops the seam bit satisfies

- Owner-gated: no

Resolution: Give each rank pin a witness that sees the seam bit inside the memory budget: decode a second synthesized stream identical except for the deep bit (drop the first stream's bytes first) and assert strict order between the two; for `pin_rank_add` and `pin_rank_checked_sub` assert the algebraic inverse (`sum.checked_sub(&small) == Some(big)`, `diff + small == minuend`) or compare `sum.encode()` against a synthesized expected stream; for `pin_version_rank` bracket with tight bounds `rank(leaf(h)) < r < rank(leaf(h + d))` rather than `rank(1)`. Replace `r != r.clone()` with a check that exercises `Clone` and `Eq` on the limb arm and can fail (compare the clone's `encode()` to the original's). Acceptance: a guest whose `synth_rank` clears the bit at `exp` reads red on every rank decode pin; a `pin_rank_add` whose result lacks the 2^-(2^32) term reads red; the committed guest stays green within the 4 GiB budget. Construction: In `synth_rank`, change `for e in [65, exp]` to `for e in [65]` (standing in for a decoder that drops the seam bit) and run `just wasm32-pins`: `rank_decode_below_backend_capacity`, `rank_decode_at_backend_byte_capacity`, `rank_decode_at_usize_exp_boundary`, `rank_decode_at_backend_bit_capacity`, `rank_decode_past_backend_bit_capacity`, the `rank_add_*` pins, and the `rank_checked_sub_*` pins all still return `Value(0)`; only `rank_roundtrip_past_backend_bit_capacity` notices.

Awaiting individual ruling: the coordinator is walking the P6 mediums with Finch now. Do not land this entry and do not choose among its alternatives; when the ruling arrives it is appended to this brief by the coordinator.

### fuzzfit-bands-19 (medium, verification): awaiting individual ruling

Fuel determinism, the instrument's stated foundation, has no committed test; the only check is a probe binary no recipe runs

- Owner-gated: no for the test; deleting `probe.rs` afterwards is the owner's call (removal of an instrument)

Resolution: Add `fuel_is_deterministic_across_fresh_guests` to `tests/enforce.rs`: run one fixed program (e.g. `build(&Family::Escalation { depth }, seed)` from `ESCALATION_REPLAYS[0]`, or the first bootstrap program) through `run_program` twice, each with its own fresh `Guest`, and `assert_eq!` the two `Vec<Sample>` including fuel (derive `PartialEq, Eq` on `Sample`). Then retire `probe.rs`, whose remaining unique function the test owns, or reword its doc so it no longer claims to be the determinism proof. Acceptance: the committed test fails if two fresh-guest replays differ in any sample's fuel; demonstrated red under a deliberate break (skip `ff_regs_reserve` on the second instance so a reallocation lands inside a measured call) before the probe is removed; `just fuzzfit` green after. Construction: Grep the tests for a second `run_program` on the same program or any comparison of two `Measured.fuel` values from separate guests: none exists. A failure the test would catch: a `static mut` counter read inside a guest kernel, or a pooled slot that is not reset to the initial image; the suite today stays green while replays diverge.

Awaiting individual ruling: the coordinator is walking the P6 mediums with Finch now. Do not land this entry and do not choose among its alternatives; when the ruling arrives it is appended to this brief by the coordinator.

### fuzzfit-strategies-11 (medium, verification): awaiting individual ruling

The mirror omits join and meet's empty-operand rungs, so O(1) steps enter the fitted cloud and the `ff_version_join`/`ff_version_meet` liveness floors are about 2.8 decades wide

- Owner-gated: no

Resolution: extend the predicate for `VersionJoin` and `VersionMeet` to `va == *vb || va.is_empty() || vb.is_empty()`, reword the comments at 610-611 and 623 to name all three rungs, and note beside `Step::identity` that the empty rungs' liveness is owned by `empty_operands_answer_without_a_walk`; add a sanity.rs pin that `Mirror::step` reports identity for an aliasing pair, a byte-equal distinct-buffer pair, and an empty-operand pair, and not for a concurrent pair; then `just fuzzfit-calibrate` and commit the re-pin with the movement annotated (the deterministic stream is unchanged; only which steps are sampled moves). Acceptance: after the re-pin, `ff_version_join` and `ff_version_meet` `width_below` fall into the range the other pair kernels occupy (below 1.0 decade), their `samples` counts drop by the excluded mass, the new sanity pin is green, and the enforcement suite is green at the new pin.

Awaiting individual ruling: the coordinator is walking the P6 mediums with Finch now. Do not land this entry and do not choose among its alternatives; when the ruling arrives it is appended to this brief by the coordinator.

### fuzzfit-strategies-16 (medium, verification): awaiting individual ruling

The escalation replays and the bootstrap stream carry reach claims with no committed floor; the builder truncates silently when a budget binds

- Owner-gated: no

Resolution: have `B` count refusals (every early return in `tick`/`fork`/`party_fork`/`party_forks`/`clock_dup`/`party_dup`/`join_all_versions`/`version_of` and every `if room()` guard that skips an emission) and expose it (`pub fn build_reporting(family, seed) -> (Vec<Op>, u32)` with `build` delegating); in sanity.rs assert zero refusals for both `ESCALATION_REPLAYS` entries and for every bootstrap program, so a cap that binds on a corpus of record fails by name; optionally pin each replay's key roster (each band key the doc names present with a maximum denominator above a stated floor, one tick per level being the irreducible growth). For the bootstrap stream, assert every sample is sub-floor and the small-band kernels' maximum denominator lies within a stated distance below `FIT_FLOOR_BITS`, replacing the manual procedure at 157-158. Acceptance: temporarily lowering `ESCALATION_BUDGET.max_ops` to 7000, or setting `BOOTSTRAP_MAX_ROUNDS` to 3 or 40, reads red by name in `just fuzzfit`; at the committed values it is green, and the doc at 96-111 points at the witness.

Awaiting individual ruling: the coordinator is walking the P6 mediums with Finch now. Do not land this entry and do not choose among its alternatives; when the ruling arrives it is appended to this brief by the coordinator.

## Roster members pending Finch's approval

Lows and nits no ruling has reached, placed here by the files they touch. Land only after the coordinator confirms the roster is approved.

### fuzz-guests-pins-11 (low, verification): roster: pending Finch's approval

One-byte length prefixes cap every decoded law-target operand at 255 bytes

- Owner-gated: no

Resolution: Widen the prefix to `u16` (little-endian, saturated at the remainder) in both targets and regenerate the seeds, or state at the framing why 255 bytes is the intended operand ceiling for the law target. Acceptance: a regenerated seed carrying a canonical clock over 255 bytes decodes in `fuzz_laws` and drives every group; the seed set's `u8::try_from` becomes `u16`.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuzz-guests-pins-18 (low, simplification): roster: pending Finch's approval

The fuzz-fit guest hand-expands its register-file accessors, split borrows, and verdict tables, and the copies already drift

- Owner-gated: no

Resolution: A `Slot` trait (`from_val`, `as_ref`, `as_mut`) implemented for the six slot types gives one generic `take::<T>`, `with::<T>`, `with_mut::<T>`, `take_range::<T>`, and `with_two_mut::<A, B>`; name the return-code tables as functions (`ordering_code`, `placement_code`, `dominance_code`, `precedence_code`) with the encoding in their doc; delete `decimal_digest` in favor of `ShapeDigest` fed the text's bytes, and name `FNV_OFFSET`, `FNV_PRIME`, `NONNEGATIVE_MASK`; route the four clock kernels through `with_c`. Acceptance: `split_at_mut` appears once; `Some(Some(Val::C(c)))` appears only in the helpers; each verdict encoding is spelled once; `cbf2_9ce4` appears once in the guest; `just fuzzfit` bands unchanged, since nothing measured changes.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuzz-guests-pins-25 (low, verification): roster: pending Finch's approval

wasm32-pins lacks the target-dir redirect its sibling workspaces carry against wasmtime-generated sources under `crates/`

- Owner-gated: no

Resolution: Add `crates/before/wasm32-pins/.cargo/config.toml` with `[build] target-dir = "../../../target/wasm32-pins"` (matching `wasm32pins_target`) and the same rationale; the harness fallback path (finding 32) then has one definition to agree with. Acceptance: on a host with no `~/.cargo/config.toml` build-dir override, `just wasm32-pins && just doclint` is green and `find crates/before/wasm32-pins -name '*.rs' -path '*/target/*'` is empty. Construction: On a host without a build-dir override, `just wasm32-pins-build` then `./tools/doclint crates`: wasmtime's OUT_DIR sources under `crates/before/wasm32-pins/target` are linted.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuzz-guests-pins-3 (low, documentation): roster: pending Finch's approval

The fuzz run commands are spelled three ways, two have drifted, and the README misstates the gate's toolchain

- Owner-gated: no

Resolution: Replace Cargo.toml:3-9 with one pointer to `just fuzz-build` / `just fuzz`; in the README keep only the crash-reproduction line the recipe does not cover and point at the recipe for the rest; correct README.md:8-9 to say the gate builds the targets on nightly and only the smoke runs at `just all` cadence; point justfile:51-52 at the README or drop the cross-reference. Acceptance: one seeded, `--target`-bearing spelling of the invocation remains (the justfile); the README's gate sentence agrees with justfile:468.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuzz-guests-pins-31 (low, simplification): roster: pending Finch's approval

The wasm32 guest's failure codes are bare negative literals, and the harness says the guest's docs key them

- Owner-gated: no

Resolution: Introduce named codes in the guest (`DECODE_REJECTED`, `BYTES_DIFFER`, `LENGTH_UNADDRESSABLE`, and so on) shared across exports, and either re-export them for the pins to assert on or correct the harness doc to say the guest's code names key them. Acceptance: `grep -E 'return -[0-9]+' crates/before/wasm32-pins/guest/src/lib.rs` is empty; the harness doc sentence is true.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuzz-guests-pins-34 (low, simplification): roster: pending Finch's approval

`BUILD_CAP_BYTES` and the `*_build_cap` test-name family name a cap two commits removed; three pins spell its coordinate as literals

- Owner-gated: no

Resolution: Rename the constant for the coordinate it is (for example `STRADDLE_COORDINATE_BYTES` for 67_108_864, so `at` reads as the coordinate itself and `below`/`past` as `- 1`/`+ 1`), rename the test family `*_below_straddle`/`*_at_straddle`/`*_past_straddle`, restate lines 14-16 as a present-tense definition (the byte count at which a buffer's bit count reaches 2^29, where a `usize`-denominated 32-bit bit count would bind), use the constant at 47-48, 61-62, 76-77 with a `fn live_bits(n: u64) -> i64 { 8 * n - 8 }` helper, and drop the five trailing commas. Acceptance: `grep -rn -i 'build_cap\|build cap' crates/before/wasm32-pins` returns nothing; `grep -c '67_108_86' pins.rs` is 1; the pins' assertions are byte-identical before and after.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuzz-guests-pins-36 (low, documentation): roster: pending Finch's approval

One join-emit pin's doc gives a different size for the same operand than its sibling

- Owner-gated: no

Resolution: "~50 MB and ~512 MB" (or "~48 MiB and ~488 MiB"). Acceptance: the two join-emit docs quote the same size for the same operand in the same unit.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuzz-guests-pins-6 (low, verification): roster: pending Finch's approval

The differential's composite allowance admits every `(NotCanonical, TrailingBits)` pair on `Ranked` and `Span`

- Owner-gated: no

Resolution: Condition the allowance on evidence that the composite check was reached: parse both components with the borsh prefix readers, run the pair check, and allow `(NotCanonical, TrailingBits)` only when that composed spelling rejects at `Stage::Pair` with a nonempty remainder; otherwise require exact genre agreement. Acceptance: a borsh `Span` reader mutated to wrap `hi`'s `TrailingBits` as `NotCanonical` diverges on the committed `span_crossed_padding` seed; the committed reader agrees on every seed. Construction: In `borsh_impls.rs`'s `Span::deserialize_reader`, map the cursor's `Decode::TrailingBits` (line 285) to `NotCanonical` before the pair check, then run `fuzz_decode_differential` on `seeds/fuzz_decode_differential/span_crossed_padding`: raw `Span::decode` says `TrailingBits`, borsh says `NotCanonical`, `composite = true` admits it.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuzz-guests-pins-8 (low, documentation): roster: pending Finch's approval

`fuzz_decode_ops`'s module doc and flavour-1 comment describe only flavour 0's framing

- Owner-gated: no

Resolution: Module doc: "flavour 0: the remainder is an op script, one op per byte, over the decoded clock; flavour 1: the remainder is a `Version` message the decoded clock compares against and receives". Line 48: "Decode a Clock, then compare against and receive a Version decoded from the remainder." Mirror the two-flavour sentence in `fuzz_seed_set.rs:33-35`, Cargo.toml:68, and README.md:36-37. Acceptance: every description of the framing names both flavours and matches both `match` arms.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuzz-guests-pins-9 (low, verification): roster: pending Finch's approval

`drive_clock` swallows `join`/`sync` errors its own construction proves impossible

- Owner-gated: no

Resolution: `.expect("a forked or re-split child is disjoint from its origin")` on both, with the comment rewritten as the one-line proof (every stash entry is a fork or sync re-split of `clock`, so overlap here is a `Party::fork`/`sum_split` defect). Acceptance: a `Party::fork` deliberately returning an aliased party crashes `cargo +nightly fuzz run fuzz_decode_ops seeds/fuzz_decode_ops` on the `clock_then_ops` seed (ops `[0, 1, 3, 5, 2, 4, 6, 7]` drive fork, sync, then join). Construction: Locally make `Party::fork` return `self.dangerously_alias()` and replay the seed: today the run stays green because `join`'s `Err` is pushed back and `sync`'s `Err(Overlap)` is dropped.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuzzfit-bands-11 (low, simplification): roster: pending Finch's approval

Four shared computations are each spelled twice: bucket medians and their thresholds, the deterministic stream loop, and the `Fit`-to-`Band` transcription

- Owner-gated: no

Resolution: One `pub fn bucket_medians(samples) -> Vec<(f64, f64)>` (and a bucket-key helper) in `fit.rs`, consumed by `curve.rs` after its thin-bucket filter and by `diag.rs` for the key; `MIN_BUCKETS` and `MIN_DECADES` declared once (`fit.rs`, public) and imported by `curve.rs`. Expose the deterministic stream as an iterator or add a family predicate to `for_each_deterministic_program` so `diag` consumes it (keeping its skip of filtered-out families). Compose `Band { kernel, rejected, fit: Fit }` (or a `Band::of(kernel, rejected, Fit)` constructor), delete both `band_of`s, and factor the two `writeln!` bodies into one `fn band_source(&Band) -> String`. Acceptance: one `total_cmp` median expression and one `BUCKETS_PER_DECADE).floor()` expression in the crate; `grep -rn 'TestRunner::deterministic' harness/src` returns only drive.rs; `grep -c 'slope: {:.6}' calibrate.rs` is 1; `just fuzzfit-calibrate` on unchanged code produces byte-identical data modulo any deliberate layout change.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuzzfit-bands-12 (low, verification): roster: pending Finch's approval

The noisy-linear tripwire's jitter is bucket-correlated, so its doc overstates what it proves

- Owner-gated: no

Resolution: Jitter by within-bucket index (expose `i` from `sampled`, or hash `d` so parity is balanced within every bucket), then tighten the assertion to a bound that demonstrates median insulation (e.g. `excess.abs() < 0.05`); or reword the comment to say the jitter is a size-correlated step and the assertion is against the allowance. Acceptance: every bucket holds half its samples at 2x, and the asserted bound is well under `SLOPE_ALLOWANCE`.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuzzfit-bands-15 (low, simplification): roster: pending Finch's approval

`Guest::call_i64` and `Op::returns_i64` form a second call path the untyped `call` already covers

- Owner-gated: no

Resolution: Delete `call_i64` and `Op::returns_i64`; make drive.rs:47-51 a single `guest.call(op.kernel(), &args)`; switch the six fuelscape call sites to `call`. Acceptance: both detached workspaces build; `just fuzzfit` and `just fuelscape-test` pass with the bands untouched, and `just fuzzfit-calibrate` on the changed code produces no diff (fuel cannot move, and the empty diff is the check).

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuzzfit-bands-8 (low, verification): roster: pending Finch's approval

`fit` and `fit_constant` have no direct tests; the fitter's headline claims are untested families

- Owner-gated: no

Resolution: Add proptests to `fit/tests.rs`: (a) a noise-free `fuel = 10^b · d^a` corpus over at least two decades recovers (a, b) within 1e-9 with both widths near 0; (b) the same corpus plus k spikes of x10 on random samples leaves the slope within 1e-6 of a while `width_above` is about 1 (raw OLS moves the slope); (c) samples spanning under a decade, or fewer than three buckets, classify constant with slope 0 and intercept the mean; (d) sub-floor samples are dropped when at least two floored remain and kept otherwise; (e) `fit_constant` returns the mean level and panics on a floored sample (`#[should_panic]`). Acceptance: the five properties committed and green; replacing the bucket medians at fit.rs:138-147 with raw OLS fails (b) by name. Construction: Replace fit.rs:138-147 with a plain OLS over `logs` and run the harness unit tests: nothing in `fit/tests.rs` or `curve/tests.rs` fails.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuzzfit-bands-9 (low, verification): roster: pending Finch's approval

`SHAPE_EXEMPT`'s justification (the point leg catches a degenerate fold) is argued inline, never pinned

- Owner-gated: no

Resolution: Either (a) an arithmetic tripwire in `curve/tests.rs` binding the argument to the constants: assert `log10(max_fold / (2 · log2 max_fold)) > width_above + ENFORCE_MARGIN` for both fold bands, and assert in the prefix leg that at least one `join_all`/`meet_all` step above the detection width is judged; or (b) a guest control kernel that left-folds the same registers, judged against the pinned `join_all` band and required to read `Above` at the budget width. Acceptance: a committed test fails if the fold ceilings widen or `max_fold` shrinks past the point where a left fold reads `InBand`, or if the prefix stops exercising wide folds. Construction: With a balanced-versus-left model of `n / (2 · log2 n)`: at n = 32 the excess is about 3.2x = 0.51 decades < 0.555 (`join_all` ceiling plus margin), in band; at n = 64, 5.3x = 0.73 decades, `Above`. Whether the 256-program prefix contains a `join_all` at n >= 64 is not determinable from code (`ScatterFold` draws `clocks` in 8..=1024 and ladders at doubling widths below it).

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuzzfit-strategies-12 (low, simplification): roster: pending Finch's approval

`Mirror::step` duplicates whole arms that differ by one method call

- Owner-gated: no

Resolution: extract small helpers on `Mirror` (`take_versions(src, n) -> Result<(u64, Vec<Version>), Malformed>` for the folds; `with_clock(c, f)` for tick/send/fork; a `version_pair_bits(a, b)`; `stage_bits()`), keeping one arm per `Op` so the exhaustive match still documents the ABI. Acceptance: `Mirror::step` roughly halves with no arm losing its comment; `just fuzzfit` green; the deterministic stream unchanged.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuzzfit-strategies-14 (low, simplification): roster: pending Finch's approval

`decimal_digest` and the ABI return codes are duplicated by hand between guest and harness

- Owner-gated: no

Resolution: factor the return codes, the digest, and ideally the kernel-name strings into one shared module both crates include (`#[path]` from a sibling `abi.rs`, or a dependency-free `fuzzfit-abi` crate); drop the mirrored-by-hand prose. Acceptance: one definition of `decimal_digest` and `ERR_OP` in the workspace; the `min_ticks` differential still passes.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuzzfit-strategies-18 (low, documentation): roster: pending Finch's approval

Family docs and arm comments misdescribe what the constructions do

- Owner-gated: no

Resolution: 181: rename to `jitter` or "Extra ticks per level drawn from 0..=this, on top of one"; 200: "High teeth tick toward `2^magnitude ± 1`; teeth past the tick budget stay at zero"; 207: "Which lane descends each level: the seed's (true) or the first fork's (false); both lanes fork every level so both ids deepen, and the pair walks opposite halves of the id tree", with the inline comment at 995 restated the same way; 300: "a fork whose child is joined into a sink of earlier children (the success join, growing round over round)"; 1866-1869: reword to match arm 5 (overlap likely, both arms sample, the mirror predicts each). Acceptance: each field doc and arm comment describes the construction beside it.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuzzfit-strategies-2 (low, simplification): roster: pending Finch's approval

`Op::returns_i64` and the `call_i64` branch are a second call path `Guest::call` already covers

- Owner-gated: no

Resolution: delete `Op::returns_i64` (ops.rs:251-254) and the branch; always `guest.call`. Retiring `wasm::Guest::call_i64` itself belongs to the wasm.rs partition (fuelscape calls it at six sites). Acceptance: drive.rs has one call site and the `min_ticks` differential (`expect == decimal_digest`) still passes in `just fuzzfit`.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuzzfit-strategies-20 (low, verification): roster: pending Finch's approval

The register-appetite premise behind `REGS_RESERVE` is stated in prose but not pinned over generated programs

- Owner-gated: no

Resolution: in `programs_respect_the_budget`, compute the largest register index the program writes (`dst` fields; `dst + n - 1` for `PartyForks`) and assert `max_index + 1 <= 2 * budget.max_ops + budget.max_forks` (the premise the const assert encodes), or expose `REGS_RESERVE` and assert directly against it. Acceptance: the sanity suite fails by name when an `Op` variant or builder path allocates past the documented bound; the const assert and this test together cover both halves of the argument.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuzzfit-strategies-21 (low, simplification): roster: pending Finch's approval

Guards that can never fire in the gate: three in `fork_balanced`, `any_program`'s non-empty filter, and `B::push`'s release-compiled-out `debug_assert`

- Owner-gated: no

Resolution: reduce `fork_balanced` to one interleaving pass per doubling with the single `next.len() < n` guard, keeping `truncate` (or give `n = 0` an explicit early return) and stating the doubling invariant in the doc comment; delete the `prop_filter`; delete the `debug_assert` or promote it to `assert!` if the owner wants the emission site named on a breach. Acceptance: `generation_is_deterministic` and `programs_respect_the_budget` green with the deterministic corpus byte-identical (`for_each_deterministic_program` yields the same programs; the refit staleness check confirms since the stream is the same); `grep -n prop_filter strategies.rs` empty.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuzzfit-strategies-22 (low, simplification): roster: pending Finch's approval

`construct` is an 840-line match with a partial domain and five repeated epilogues

- Owner-gated: no

Resolution: one function per family with `construct` reduced to dispatch; a shared `spine_epilogue(b, pools, seed, cur, rank: bool, cross_tick: bool)` and `join_chain(b, shares) -> Option<Reg>`; consider a `Coupled` sub-enum returned by `reduced_family` and taken by `construct`, with `Family::Independent { .. }` and `Family::Coupled(Coupled)` at the top level so the `unreachable!` dissolves; each family function then has a rustdoc home for the construction comments now attached to match arms. Acceptance: `construct` or its replacement fits on a screen; `grep -c unreachable! strategies.rs` is 0; `generation_is_deterministic` and the enforce staleness cross-check confirm the emitted programs are unchanged.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuzzfit-strategies-4 (low, simplification): roster: pending Finch's approval

The driver's snapshot table restates `Op::kernel` through `u8` tags and an `unreachable!`

- Owner-gated: no

Resolution: in ops.rs add `pub enum Kind { Version, Party, Clock, Rank }` with `fn snapshot_op(self, reg: Reg) -> Op` (the three `*Encode` ops and `RankDisplay`); `live_regs() -> Vec<(Reg, Kind)>`; the driver becomes `let op = kind.snapshot_op(reg); guest.call(op.kernel(), &op.args())`; sanity.rs matches on `Kind`. Acceptance: no `ff_` string literal in drive.rs; `grep -n unreachable! drive.rs tests/sanity.rs` is empty; `just fuzzfit` green.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuzz-guests-pins-13 (nit, simplification): roster: pending Finch's approval

`fuzz_parse` compares the `Clock` round-trip by `encode()` while its siblings compare by `==`

Where: `crates/before/fuzz/fuzz_targets/fuzz_parse.rs:53-63`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): `assert_eq!(again, clock, ..)`, or a comment saying why not

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuzz-guests-pins-23 (nit, documentation): roster: pending Finch's approval

`ff_party_forks`'s doc says the kernel "replaces `src`"; it mutates `src` in place

Where: `crates/before/fuzzfit/guest/src/lib.rs:987-988`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): "(the source in `src` keeps its remainder share; the iterator borrows it)"

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuzz-guests-pins-28 (nit, simplification): roster: pending Finch's approval

Repeated prologue and epilogue fragments in the wasm32 guest; a slice-dispatch `unreachable!` in the harness

Where: `crates/before/wasm32-pins/guest/src/lib.rs:102-106`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): `addressable` and `rank_observations` helpers; a generic `call<P: WasmParams>`

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuzz-guests-pins-30 (nit, verification): roster: pending Finch's approval

`synth_rank_ladder`'s layout check is a `debug_assert_eq!` compiled out of the only profile built (release with overflow checks only).

Where: `crates/before/wasm32-pins/guest/src/lib.rs:552-552`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): `assert_eq!` with a message, or a negative return code.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuzz-guests-pins-5 (nit, simplification): roster: pending Finch's approval

The six-type wire roster is spelled three times across the decode targets

Where: `crates/before/fuzz/fuzz_targets/fuzz_decode.rs:31-86`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): One `round_trip!` macro; one roster list for both differentials

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuzzfit-bands-14 (nit, simplification): roster: pending Finch's approval

Em-dashes in `//` comments and assert strings; past-tense incident narration at two declaration sites

Where: `crates/before/fuzzfit/harness/src/wasm.rs:131-135`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): `--` or colons at eight sites; reword two past-tense comments positively

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuzzfit-bands-22 (nit, simplification): roster: pending Finch's approval

The band key is a bare `(&str, bool)` tuple with the `" [err]"` rendering repeated eleven times

Where: `crates/before/fuzzfit/harness/tests/enforce.rs:56-66`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): A `BandKey` struct with `Display`; one `violation` fn

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuzzfit-bands-23 (nit, simplification): roster: pending Finch's approval

Two dead guards: `bands_are_pinned` is subsumed by the roster parity test, and the small-band `BelowFloor` arm is unreachable

Where: `crates/before/fuzzfit/harness/tests/enforce.rs:158-164`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Delete `bands_are_pinned`; make `BelowFloor` unreachable or unrepresentable

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuzzfit-bands-25 (nit, simplification): roster: pending Finch's approval

Idiom nits: qualified paths beside imports, magic numbers, an expect message that asserts rather than names

Where: `crates/before/fuzzfit/harness/tests/enforce.rs:369-371`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Imports; `10f64.powf(1.0 / BUCKETS_PER_DECADE)`; named `NOP_CEILING` and `PROGRESS_EVERY`

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuzzfit-bands-28 (nit, simplification): roster: pending Finch's approval

The pure `judge_against` tripwire lives in an integration file titled "Generator sanity"; `bands` has no sibling tests

Where: `crates/before/fuzzfit/harness/tests/sanity.rs:1-3`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Move the pure tripwire to `src/bands/tests.rs`; retitle sanity.rs

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuzzfit-bands-29 (nit, verification): roster: pending Finch's approval

`prop_assert!(step.denom_bits >= 1)` in `programs_are_well_formed` cannot fail: `Step` is built only with `denom_bits.max(1)`.

Where: `crates/before/fuzzfit/harness/tests/sanity.rs:51-51`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Delete, or assert that no live operand encodes to zero bits.

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuzzfit-bands-3 (nit, documentation): roster: pending Finch's approval

`Band.constant`'s doc invites the converse reading; three pinned bands have slope 0 and `constant: false`

Where: `crates/before/fuzzfit/harness/src/bands.rs:157-158`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): "Whether the band was constant-classified: too little denominator span or too few buckets for a slope estimate (see `fit::fit`) ...

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuzzfit-strategies-10 (nit, documentation): roster: pending Finch's approval

`Malformed` is documented as never a `before` bug, but the decode and parse arms map before's own round-trip failures to it

Where: `crates/before/fuzzfit/harness/src/ops.rs:314-320`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): either reword the doc ("a register-file or stage violation: a generator bug, or a `before` round-trip failure surfacing through a stale stage") or ...

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

### fuzzfit-strategies-9 (nit, simplification): roster: pending Finch's approval

Idiom nits across `ops.rs` and `strategies.rs`

Where: `crates/before/fuzzfit/harness/src/ops.rs:273-276`. Nit row (the full record is in `evidence/`, under the partition or sweep report named by the id's prefix).

Resolution (nit row): Apply the listed renames and named selectors; `Builder` for `B`

Roster note: lands only once the coordinator confirms this lane's roster is approved; then per the quoted Resolution, under the rulings this brief names. Report rather than choose if the Resolution offers alternatives, would move a public signature, or would change a rendered `before` doc panel (ruling 89: a deliberate ruling is required for that).

