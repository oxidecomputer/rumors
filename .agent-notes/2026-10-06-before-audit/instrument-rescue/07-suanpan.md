<!-- CAVEAT LECTOR: written by Claude (Opus 5.5) as the cataloguer for the suanpan lane (L7) in the owner's instrument rescue (question 99), from the branch, the lane records, the auditor's scratch logs, and two census runs on ox-east-1; for the integrator and for Finch's review. -->

# Instrument rescue: the suanpan lane (L7)

The suanpan lane built a value oracle over programs on a pool of
accumulators, a touch-cost property and a search that tried to break it,
wasm fuel measurements of `before` operations on sparse inputs, 32-bit
cases for the wasm32 executor, three design prototypes, and calibration
tooling. Most of its checks have reached ready branches. What remains is
the value oracle, the fuel measurements of six operations the committed
ladder leaves out, the record of which instrument kills which hand-made
defect, and a handful of cheap or purely diagnostic pieces.

## Summary

**Source.** The branch `explore/l7-suanpan` at `5d5e33471`, read against its
merge base with `main`, `0bdeb588d`: 22 commits, one of them a merge of
`main` at `041d868a5`; 22 files; 2,402 added lines (verified,
`git diff --stat`). I read every added file in full with `git show`. The
lane's auditor also left calibration tooling outside the branch, in the
session scratchpad's `auditor-l7/` directory. I read that too and catalogue
it below. That directory lives under `/private/tmp` and does not survive a
reboot.

**Notation.** `#n` is entry `n` of `QUESTIONS.md`, which names its branch.
Every claim is marked *verified* (I checked it), *reported* (a record says
so; the record is named), or *inferred* (my reasoning, not checked by a
run).

**Baseline.** `instrument-rescue/00-baseline.md` did not exist while I wrote
this. Where an entry needed a committed baseline, I measured the committed
generator myself with the same census code as the explore instrument. I
used two of the three box runs allowed, both from the scratch worktree
`/Users/oxide/src/rumors-rescue-l7` at `5d5e33471`:

- *Run 1* measured three properties' run times at their default case
  counts, and took a census of 1,000 programs each from the pool model's
  generator and from the committed surface generator. The surface
  generator at `5d5e33471` is identical to `main`'s (verified, `git diff`).
- *Run 2* took the same census of #37's generator, which carries the pool
  model's generator forward.

Each census draws from proptest's deterministic runner, so its counts
reproduce exactly. The census code is in the session scratchpad at
`rescue-l7/census.diff`, and the logs are `rescue-l7/run1.log` and
`rescue-l7/run2.log`.

**Count.** 26 instruments: 11 with full entries, ordered by my judgment of
value, and 15 carried by ready branches, one line each.

**Top three.**

1. *The pool model's exact-oracle checks* (entry 1). #37 already carries its
   generator, so about 360 lines of checks would put a value oracle over
   operands with their own histories, values of thousands of digits, and
   heavily cancelled states that no committed value test reaches.
2. *The fuel matrix's six further operations* (entry 2). #52's ladder holds
   three of `before`'s linear operations to committed fuel readings; the
   matrix measured join, meet, `partial_cmp`, tick by two parties, and the
   projection comparison as well, all of which the fuzz-fit guest already
   exports.
3. *The mutant schema* (entry 3). It is the lane's only record of which
   instrument kills which hand-made defect, and it exists only in
   `/private/tmp`.

**Corrections to the records.**

1. *M6.* The pool model's inventory entry (`lanes/l7-suanpan/round-2/inventory.md`)
   and the round-1 coverage record say the pool model caught M6 (the small
   stability threshold lowered from 3 to 2) once threshold-biased values
   were added. Both logged runs show it passing M6: `calibrate.out`, before
   those values existed, and `calibrate2.out`, at 2,000 cases after
   `df0002298` added them (`mut/M6.log`: "PASS [ 35.160s] ...
   l7_pool_programs_match_the_oracle"). The failure in `calibrate2.out`
   belongs to the explore touch property, and its minimal input is an
   owned-clone subtraction, the pricing error that `c100e028f` fixed about
   40 minutes later. That failure was a false alarm, not a kill (verified,
   logs and commit times). Nothing depends on the claim: the committed
   suite catches M6 (verified, `calibrate_existing.out`).
2. *Uncompacted cancellation.* The records name uncompacted cancellation
   across steps as the pool model's distinctive reach, because it compares
   only where a program asks. The census shows such states are about as
   rare in the pool model (0.10% of steps leave two or more redundant
   digits) as in the surface property (0.14%), because suanpan's trim
   removes most cancellation as it happens. The pool model's advantage is
   depth (runs of up to 1,969 redundant digits, against 33), operand
   histories, width, and extreme digits (verified, run 1).
3. *Bound digits in the surface property.* The follow-up "Bound-digit states
   reaching other operations" (`follow-ups.md`) says the surface model
   never parks digits at the representation's bound, so other operations
   "never see such states". The surface property does reach a digit of
   magnitude `2^33 − 1` in 202 of 1,000 programs, through ordinary
   deposits. It reaches them rarely where they matter: 2.2% of its
   `normalize` steps and 1.7% of its shifts, against 12.7% and 13.6% in the
   pool model (verified, run 1). "Rarely" is accurate; "never" is not.

**What I could not assess.**

- The run time of the pool model's checks folded onto #37's generator:
  estimated, not measured (entry 1).
- Whether landing case 5 catches a narrowing the committed cases miss: no
  mutant was ever run against it (entry 5).
- The adversary's run time at its recorded scale: its logs carry no times
  (entry 4).
- The readout census over `before`'s suite: never run, and I did not use
  my third box run on it, because #64 tests every class directly (entry 7).
- The run time of the fuel-ladder extension: estimated from the ladder
  prototype's timing (entry 2).

## Carried by ready branches

- **The touch-bound property** (`metered::l7_touches_are_linear_in_table_work`,
  with its work pricing `metered::work` and `meter_program`): #37,
  `audit/suanpan-touch-bound`, as `touches_stay_linear_in_priced_work` in
  `crates/suanpan/src/accumulator/tests/metered/touch_bound.rs`, with derived
  constants `K = 15` and `D = 60` and the committed control
  `remnant_dropping_trim_exceeds_the_bound` (verified). It left one thing
  behind: the explore version ran every step through the pool model's step
  function, whose inline value checks are how it caught M1 (a stability
  answer) and N1 (`normalize`'s width) (verified, `mut/M1.log`,
  `mut/N1.test__l7_touches_are_lin.log`). #37 checks touches only. Those
  checks are entry 1.
- **The pool model's generator** (swarm weights; word, shift, and width
  strategies; values near `k · 2^(32d)`; extreme-digit parking; gap rounds):
  #37's `touch_bound.rs` ports it nearly verbatim, adds cancelled chains,
  and spreads gap tops over bit lengths up to `2^14` digits (verified, by
  reading). Entry 1 measures both generators.
- **The fuzz-fit ladder prototype** (`crates/before/fuzzfit/harness/tests/explore_l7.rs`,
  `l7_zero_range_fuel_ladder`): #52, `audit/suanpan-zero-range-fuel-ladder`,
  as `harness/tests/zero_range_ladder.rs` with 72 pinned cells, a ±2% band,
  a liveness floor, value checks, and control flatness (verified, diff and
  `Operation` enum; details reported, #52's entry).
- **The host-side family builders** (`wasm32-pins/guest/src/l7_builders.rs`,
  as `include!`d by the two native test files): #52's
  `harness/src/zero_range_ladder/families.rs` (verified).
- **The known-bad variant K1** (a repeated zero-range lookup): #52's band
  documentation and calibration (reported, #52's entry). #86 deletes the map
  it amplifies.
- **Prototype W** (feature `l7-proto-w`, `digits/written.rs` with a positional
  invariant check): #86, `fix/suanpan-zero-range-bitset`, built from scratch,
  with the model test `written_positions_match_an_ordered_set` against a
  `BTreeSet` (verified).
- **Prototype O1** (feature `l7-o1`): #89,
  `simplify/suanpan-comparison-fixed-point` (reported, #89's entry).
- **The repeated-comparison probe** (`metered::l7_probe_repeated_comparison_cost`):
  #89's exact touch test `comparison_skips_only_a_write_back_that_restores_its_digits`
  (verified).
- **The extreme high-part probe** (`metered::l7_probe_extreme_high_parts` and
  `l7_check_readout`): #64, `audit/suanpan-readout-class-table`, as
  `readout_high_part_costs_one_touch_in_every_carry_class`, over every
  reachable final-carry class at an even and an odd width (verified name;
  coverage reported, #64's entry).
- **The native zero-shift probe** (`probes::l7_probe_zero_shift_history`): #48,
  `fix/suanpan-zero-shift`, as the property
  `shifting_zero_takes_constant_space_whatever_its_stored_form` in
  `representation.rs` (verified).
- **wasm32 zero-shift cases 7 and 8** (`zz_l7_suanpan_zero_shift_representations`):
  #48's `suanpan_shifts_zero_onto_an_unaddressable_digit_position`, cases 0
  and 1 of `Check::SuanpanZeroShift` (verified).
- **The native stability-width probe** (`probes::l7_probe_stability_huge_width_compacts`):
  #50, `fix/suanpan-stability-width`, as the witness
  `maximum_adjustment_width_still_compacts`, which also stores a value below
  the cancelling top (verified).
- **wasm32 stability-width case 9** (`zz_l7_suanpan_stability_width_compacts`):
  #50's `suanpan_stability_query_compacts_and_declines_past_the_usize_digit_index`,
  at three widths straddling the boundary, with the value at digit 2 so that
  a truncating repair fails too (verified).
- **The reserve probe** (`probes::l7_probe_reserve_digits_extreme`): #28,
  `fix/suanpan-reserve-digits-hint`, as `reserve_bits_is_value_neutral`,
  `reserve_bits_ignores_unsatisfiable_requests`, and the wasm32 pin
  `suanpan_ignores_unsatisfiable_reservations` (verified).
- **wasm32 landing cases 10, 11, and 12** (`zz_l7_suanpan_add_shifted_landings`):
  #54, as cases 5 and 6 of `suanpan_rejects_unaddressable_digit_landings`
  (explore cases 10 and 12). Case 11 repeats the route of `main`'s case 2
  (reported, `lanes/l7-suanpan/round-3/MB2-add-shifted-wasm32-landings.md`).

## Full entries

### 1. The pool model's exact-oracle checks

- **What it is.** A stateful differential property,
  `l7_pool_programs_match_the_oracle`, with its step function `apply`, its
  after-step observer `observe`, and the contract checks `check_stable` and
  `check_conversions`, in
  `explore/l7-suanpan:crates/suanpan/src/accumulator/tests/explore_l7.rs`,
  lines 1 to 711 at `5d5e33471` (verified). A program has 1 to 119 steps
  over a pool of three accumulators, each paired with a `num_bigint::BigInt`.
  Steps come in 19 kinds, and each case enables or disables each kind
  through a swarm weight vector. The generator (lines 509 to 685) is
  carried by #37; the checks are not.
- **What it reaches or checks.**
  - *The oracle.* `num_bigint` arithmetic, independent of suanpan's digit
    code. The only shared code is the test module's readout helpers,
    `assert_value` and `from_limbs` (verified).
  - *After every step, for every member, without mutation:* both readouts
    against the oracle; suanpan's private representation invariants;
    known-zero soundness; `stored_bits() == 32 · stored_digit_count()`; the
    documented bound `|value| < 2.01 · 2^stored_bits()`
    (`crates/suanpan/src/accumulator.rs:352` at `main`); and the exact
    largest value a digit count can hold, `(2^33 − 1) · Σ 2^(32i)`.
  - *At the step that asks:* `normalize` leaves the exact width, grows by at
    most one digit, and leaves a zero known. `cmp_zero` agrees with the
    oracle, and `Equal` implies a known zero. A `Some` stability answer has
    the oracle's sign, is never `Equal`, exceeds `2^bits`, and at
    digit-aligned widths exceeds every accumulator operand of that width. A
    `None` answer leaves at most two digits above the requested width.
    Asked with another member's `stored_bits`, adding or subtracting that
    member's actual value keeps the sign. Every primitive conversion agrees
    with the oracle, and a rejection returns the value with its `Debug`
    representation unchanged.
  - *Census against the committed surface property* (verified, run 1, 1,000
    programs each). *Redundant digits* are stored digits beyond the
    value's exact width, left by cancellation no comparison has yet
    compacted.

    | Regime | Pool model | Committed surface property |
    |---|---|---|
    | Steps | 58,249 | 40,775 |
    | Operand uses whose operand holds two or more recorded zero ranges | 4,165 of 12,500 (33%) | 0 of 23,153 |
    | Operand uses with the receiver as its own operand | 1,047 | 0 |
    | Operand uses whose operand holds two or more redundant digits | 13 | 0 |
    | Steps leaving more than 512 stored digits | 6,576 (11%) | 0 |
    | Largest stored digit count | 5,112 | 116 |
    | `normalize` steps on a value holding a digit of magnitude `2^33 − 1` | 372 of 2,936 (12.7%) | 36 of 1,621 (2.2%) |
    | `<<=` steps on such a value | 410 of 3,013 (13.6%) | 54 of 3,183 (1.7%) |
    | Steps leaving two or more redundant digits | 61 (0.10%) | 58 (0.14%) |
    | Longest redundant run | 1,969 digits | 33 digits |
    | Steps leaving two or more recorded zero ranges | 19,043 (33%) | 14,788 (36%) |
    | Most recorded zero ranges at once | 66 | 11 |

  - *Predicates no committed test states* (verified, by grep at `main`):
    - the `2.01` bound in public terms: the surface property checks
      `bits() <= stored_bits() + 2`, which allows four times
      `2^stored_bits()`;
    - the exact representable maximum;
    - the stability operand clause against an evolved pool member: the
      committed `stored_width_stability_is_sound` uses constructed probe
      operands;
    - a `Debug`-preserving rejection on arbitrary states: committed
      `primitives.rs` checks it only at each type's range boundaries.

    In the digit form, suanpan's private invariant check bounds every digit
    below `2^33`, which implies the `2.01` bound (inferred, by summing the
    digit bound). The public check adds the small form's width arithmetic in
    `stored_digit_count`.
- **Coverage beyond the committed suite.**
  - *Operands with histories.* The surface property builds every operand
    fresh from one primitive, one limb stream, or one cancelled high word.
    Its operands never hold two or more zero ranges, never alias the
    receiver, and never carry uncompacted cancellation (table above).
  - *Width.* Values to 5,112 digits, against 116.
  - *Extreme digits.* Extreme digits meet `normalize` and shifts six to
    eight times as often. The pool model parks extremes of both signs at
    digits 0 to 11, then negates, shifts, compares, normalizes, and lends
    them as operands. This is the gap the follow-up "Bound-digit states
    reaching other operations" names (see correction 3).
  - *What it does not add.* It reaches deeper uncompacted cancellation, but
    not more of it (correction 2). It also lacks some of the surface
    property's breadth: twelve typed primitive operands and shift counts,
    the consuming `+` and `-` forms with primitives, and an accumulator
    operand built at the small form's `2^96` limit. It would stand beside
    the surface property, not replace it.
- **Evidence.**
  - *Mutants* (verified, the auditor's logs at 2,000 cases: `calibrate.out`
    and `mut/`). It fails M1 (comparison threshold 3 to 2), M2 (stale range
    after a write), M3 (range rebuild without clearing), M4 (negative
    readout complement), M5 (`i128::MIN` conversion), M7 (stability margin
    `f + 2` to `f + 1`, through the `None` width clause), M9 (collapse
    leaving the final digit), and N1 (`normalize.rs:74`, `*` to `/`). On M1
    and N1, nextest killed it at the 180-second limit then in force; its
    output holds 834 and 1,854 assertion panics from its own checks, so it
    had found a failure and was still shrinking it (verified: the timeouts
    and panics; inferred: the shrinking). It passes M6 (correction 1). M8
    trips production's own `debug_assert`, so it calibrates nothing.
  - *Against the committed suite.* The committed suite also fails M1 to M7
    and M9 (verified, `calibrate_existing.out`). N1 was the pool model's one
    unique catch, and #66's exhaustive test now catches it (reported, #66's
    entry). Against `main` plus the ready branches, it has no known unique
    catch.
  - *Runs.* A 20,000-case run passed (verified, `long1.log`).
- **Fold-in cost.**
  - *Where.* #37 already carries the generator, so the natural fold-in is a
    second property over #37's `arb_program`: keep an oracle per member
    beside #37's pool, apply each step to both, and run `observe` after
    every part of every step (inferred). The checks to port are lines 147
    to 506: `max_magnitude`, `check_stable`, the oracle half of `apply`,
    `check_conversions`, and `observe`, about 360 lines (verified line
    numbers).
  - *Adjustments.* #28 replaces `reserve_digits(usize)` with
    `reserve_bits(u64)`. #86 replaces the zero-range invariants that
    `assert_invariants` checks. `Convert` becomes an observation, since #37
    has no such step. #37 parks only negative extremes; positive ones arise
    by negation.
  - *Reach of that fold-in* (verified, run 2; #37's generator, 1,000
    programs, counting each part of an embedded pattern as a step). It
    keeps every regime above and adds many uncompacted states: 5,302 of
    95,187 steps (5.6%) leave two or more redundant digits; 2,637 of 35,766
    comparisons (7.4%) read such a value; 1,346 steps hold a zero stored
    redundantly, against 5 in the pool model; values reach 16,382 digits.
    The oracle would then check values in regimes neither committed value
    property reaches.
  - *Run time.* The pool model takes 3.72 s at the default 256 cases
    (verified, run 1: debug build, one test at a time, box load about 21).
    Over #37's wider generator I estimate 5 to 15 s (inferred: the census's
    readouts alone took 5.6 s for 1,000 programs), well inside nextest's
    300-second limit.
  - *Dependencies and docs.* No new dependencies: `num-bigint` and
    `proptest` already build in suanpan's tests (verified, run 1). The step
    kinds and their fields need doc comments, and the property's comment
    must state its invariant. `before`'s validation index has no suanpan
    entry (verified, grep at `main`), so the rationale belongs in suanpan's
    test module docs, where #37 put its own.
- **Overlaps.** It shares its generator with #37. It overlaps the committed
  `complete_surface_matches_bigint` in readouts, conversions, `normalize`,
  and stability, over a different input distribution, so it would be a
  second, independent check. The committed exhaustive
  `representation_invariants_hold_exhaustively` covers every short
  schedule; this samples long ones.
- **Dependencies.** #37 (the generator), #28 (the reserve API), and #86 (the
  invariants). #48 makes zero shifts take constant space; the pool model
  checks values, not space, so nothing in it breaks (inferred).
- **Value, in one sentence.** It is the only value oracle that drives
  suanpan with operands carrying their own histories, values of thousands
  of digits, and extreme digits, and on #37's generator it would check
  values in heavily cancelled states no committed test reaches, for about
  360 lines and a few seconds per run.

### 2. The fuel matrix's operations that the committed ladder leaves out

- **What it is.** A wasm fuel probe in the `wasm32-pins` workspace, at
  `5d5e33471` (verified):
  - the guest dispatcher `l7_fuel` and the suanpan-only workloads
    `l7_suanpan`, in `crates/before/wasm32-pins/guest/src/checks.rs`, about
    120 lines;
  - the protocol variant `Check::L7Fuel = 9`;
  - a fuel-metering engine, `run_fueled`, in the harness library, 27 lines;
  - the ignored harness test `zz_l7_fuel_matrix`;
  - the guest-side family builders, `l7_builders.rs`, 229 lines.
- **What it reaches or checks.** For six families (F1, ±S, and OS, each with
  a control that records no zero ranges) at `r` = 64 to 16,384, it reports
  the fuel each operation adds over a baseline: suanpan's update loop
  alone, `Version::decode`, `Version::new() <= v`, `partial_cmp` against an
  empty version, join and meet with a one-tick version, tick by the seed
  party and by a half party, the projection comparison
  `v.project(&half) <= v`, and `min_ticks`. It asserts only that each probe
  returns.
  - Readings are in [`F1b-design-evidence.md`](../lanes/l7-suanpan/round-2/F1b-design-evidence.md)
    (reported). I checked one cell against its raw log: ±S decode at
    `r = 64` is 6,664,878 fuel over 32,292 bytes, 206.4 per byte, matching
    the table's 206 (verified, `auditor-l7/m3/current-f2.log`).
  - Under the ordered zero-range map, every operation grows per byte on ±S
    and OS across the five sizes: join from 261 to 375, tick by a half
    party from 342 to 589, the projection comparison from 515 to 868, and
    suanpan alone from 1,617 to 3,538 fuel per limb. The controls are flat,
    and the W prototype is flat everywhere (reported, F1b).
- **Coverage beyond the committed suite.** #52's ladder measures decode, `<=`
  against a lowered operand, and `min_ticks` (verified, its `Operation`
  enum). It does not measure join, meet, `partial_cmp`, tick, or the
  projection comparison, each documented as linear in input bytes, nor
  suanpan's update cost apart from `before`.
  - These operations drive accumulators that decode and `<=` do not. On the
    OS control, tick, the projection comparison, and `min_ticks` grew under
    the map while decode and `<=` stayed flat, because their accumulators
    receive the odd value `D` alone (reported, F1b's caveat).
  - So a later suanpan change that brought back per-range cost on those
    accumulators' paths could pass all 72 ladder cells and still break the
    linear bounds of join, tick, or the projection comparison (inferred).
  - The board cannot see this work: its counters never count map work, and
    its slope limit of 1.15 passes a logarithmic factor (reported,
    [`F1c-machinery-zero-range-fuel-ladder.md`](../lanes/l7-suanpan/round-3/F1c-machinery-zero-range-fuel-ladder.md)).
- **Evidence.** It measured F1's breach of `before`'s linear bounds on every
  operation it covers, and design W's repair (reported, F1b). It caught no
  defect beyond F1. No mutant was run against it; K1 was run only against
  the fuzz-fit ladder (reported, F1c).
- **Fold-in cost.**
  - *Where.* Add `Operation` variants to #52's `zero_range_ladder.rs`, with
    kernels over exports the fuzz-fit guest already has: `ff_version_join`,
    `ff_version_meet`, `ff_version_cmp`, `ff_version_tick`, and
    `ff_version_project` followed by `ff_version_le` (verified, all
    exported by `crates/before/fuzzfit/guest/src/lib.rs` at `main`). Then
    re-run `just fuzzfit-calibrate-ladder`.
  - *A trap to avoid.* Compare against the ladder's lowered operand, never
    `Version::new()`. F1c showed that a comparison with the empty version
    has a constant-time answer, so pinning today's traversal would commit
    removable work. The matrix's `<=` probe has that flaw (reported, F1c),
    and its `partial_cmp` probe compares against the same empty version
    (inferred to share it).
  - *Run time.* Six operations add up to 144 cells to the ladder's 72
    (inferred). The ladder prototype's 90 cells took 16 s (reported, F1c),
    so I estimate 25 to 40 s more, split across tests (inferred).
  - *What not to keep.* The suanpan-only workloads do not fit the fuzz-fit
    guest, which exports `before` operations only, so they would need a new
    export (inferred). `run_fueled` duplicates the fuzz-fit harness's fuel
    engine, and `Check::L7Fuel = 9` collides with the variant number that
    #28, #31, #48, and #50 claim (reported, #28's and #54's entries).
- **Overlaps.** It extends #52 over the same families, sizes, and guest. The
  fuzz-fit bands meter random programs, not chosen families. The fuelscape
  measures fuel but enforces nothing.
- **Dependencies.** #52 (the ladder) and #86 (W). New cells should be
  pinned after #86 lands; otherwise W must re-pin them at once (inferred).
- **Value, in one sentence.** It would extend the ladder's committed
  linearity check from three of `before`'s operations to six more that fold
  through suanpan accumulators, using guest exports that already exist.

### 3. The mutant schema and its calibration scripts

- **What it is.** Tooling outside the branch, in the session scratchpad's
  `auditor-l7/` directory (verified):
  - `mutate.py` (60 lines) applies or reverts one named exact-string swap,
    and refuses unless the source text matches exactly once;
  - `calibrate.sh` and `calibrate_f.sh` apply a mutant, run a nextest
    filter on the box, revert, and stop unless `git diff --quiet` passes;
  - the kill logs `calibrate*.out`, `mut/`, and `r3/mut*.log`.

  The schema names 27 mutants: M1 to M9, M12 to M16, Z1 to Z5, N1 to N3,
  K1, and the 32-bit narrowings L3S, L3D, L3W, and L3H. It also holds three
  repair swaps (U1Fa to U1Fc) and a feature switch (WDEF), which are not
  mutants.
- **What it reaches or checks.** Hand-chosen semantic defects: decision
  thresholds (M1, M6, M7); zero-range bookkeeping (M2, M3, M12 to M16, Z1
  to Z5); readout and conversion (M4, M5); recentering (M8); collapse (M9);
  `normalize`'s shrink loop (N1 to N3); a doubled map lookup (K1); and
  shift or index narrowings that change nothing on 64-bit hosts (L3S, L3D,
  L3W, L3H).
- **Coverage beyond the committed suite.** It is not a check; it measures
  checks.
  - Several of its mutants are of forms `cargo-mutants` does not generate:
    a dropped statement block (M14, M15), an added redundant lookup (K1),
    and inserted `as usize as u64` casts (L3S, L3H, L3D) (inferred from the
    mutants' text).
  - Its kill matrix records which instrument catches each. For example, the
    committed suite misses M14 and M15, and only the touch property catches
    them (verified, `calibrate_existing.out` and
    `calibrate_touch_split2.out`).
- **Evidence.** Its runs back the lane's calibration claims: MB1's (M14,
  M15), the cross-check of the adequacy lane's Z and N survivors, MB2's
  (L3S, L3H, L3D, L3W), and F1c's (K1) (verified, logs). One record
  misreads it (correction 1).
- **Fold-in cost.**
  - Twelve mutants target `zero_ranges.rs`, which #86 deletes (M2, M3, M12,
    M14 to M16, Z1 to Z5, K1); they cannot apply after it lands. The rest
    target files that #86 and #89 also change (inferred, from those
    branches' file lists).
  - Paths are hard-coded to the auditor's worktree.
  - A fold-in would re-derive each string against `main`, then commit the
    schema as known-bad demonstrations, for example as a just recipe beside
    the adequacy lane's survivor export. No cost to the suite's run time.
- **Overlaps.** The adequacy lane's `cargo-mutants` survivor list and
  export; each ready branch's own calibration record, such as #37's
  committed control.
- **Dependencies.** #86 and #89 change the code most mutants target.
- **Value, in one sentence.** It is the lane's only record of which
  instrument kills which hand-made defect, and it lives in `/private/tmp`,
  so it must be copied into the lane records before a reboot if the owner
  wants it kept.

### 4. The hill-climbing touch adversary

- **What it is.** The ignored test `metered::l7_adversarial_touch_search`
  and its helper `scaled`, `explore_l7.rs` lines 889 to 990, about 100
  lines (verified).
  - Per seed it draws a swarm weight vector and 60 random steps.
  - It then makes a set number of mutations (replace, insert, or remove a
    step, or repeat a segment one to eight times), keeping each that does
    not lower the program's touches per unit of work.
  - Finally it multiplies every shift and park index of the best program by
    2, 4, and 8, and prices it again.
- **What it reaches or checks.** It asserts nothing; it prints ratios.
  - Over 12 seeds of 30,000 mutations, the best ratio was 9.631 (seed 4).
    Ten seeds' best programs read the same ratio at every scale, and seeds
    0 and 2 fall: 4.05 to 1.75 and 5.53 to 1.44 (verified,
    `adversary3.log`).
  - The best programs stack `+= i128::MAX`, which the explore pricing
    charges one unit. Under #37's pricing, which charges 128-bit updates
    four units, that family would read about 2.4 (reported, MB1; not
    re-run).
- **Coverage beyond the committed suite.** It searches for programs that
  maximize cost per unit of work, and tests whether that cost grows with
  width by rescaling. #37 samples programs and checks a fixed bound;
  nothing committed searches toward the bound.
- **Evidence.** It found no width-dependent cost and caught nothing. It
  was never calibrated: nobody checked whether it finds a dropped-remnant
  mutant faster than random sampling does.
- **Fold-in cost.** Port onto #37's `Step` and `Step::work`, replacing the
  explore pricing, which charges `i128` updates one unit and owned-clone
  operands once. Keep it ignored and document it as a search, or run it
  from a just recipe. Its run time at the recorded scale is unmeasured; its
  default, 4 seeds of 2,000 mutations, is far cheaper.
- **Overlaps.** #37's property and its derivation of `K`.
- **Dependencies.** #37.
- **Value, in one sentence.** A tool for re-measuring the margin of #37's
  constant when suanpan's cost table changes, not a regression check.

### 5. wasm32 landing cases 5 and 6

- **What it is.** Guest cases 5 and 6 in `suanpan_landing`
  (`crates/before/wasm32-pins/guest/src/checks.rs`), and the harness test
  `zz_l7_suanpan_limb_index_past_usize_max`, which also re-runs case 4
  (verified).
  - Case 5 streams `2^31` zero limbs and then a 1, at shift 0. The 1's limb
    index fits a 32-bit `usize`, but its digit position, `2^32`, does not,
    so correct code panics.
  - Case 6 repeats case 4's stream and classifies the outcome, reporting
    `WrongBytes` when the 1 wrapped to digit 0.
- **What it reaches or checks.** Case 5 traps as required (verified,
  `auditor-l7/wasm3-fix.log`).
- **Coverage beyond the committed suite.**
  - `main`'s cases reach digit `2^32` through a shift plus a small limb
    index (case 1) and through a limb index of `2^32` (case 4) (verified,
    `main`'s guest). No committed case, and neither of #54's, reaches a
    limb index between `2^31` and `2^32` at shift 0, where the index is
    addressable and only twice it is not.
  - A narrowing that converts the limb index to `usize` with a panic on
    overflow, but doubles it with wrapping arithmetic, would trap on case 4
    and pass `main`'s pin, while case 5 would return (inferred; no such
    mutant was ever run).
  - Case 6 adds nothing a committed check needs; it diagnosed the
    limb-index wrap before the fix. The test's case 4 assertion repeats
    `main`'s case 4.
- **Evidence.** It demonstrated the limb-index wrap (D1) before the fix
  (verified, `wasm2.log` and `wasm3-fix.log`). Case 5 has caught nothing.
- **Fold-in cost.** One more case in the loop of `main`'s
  `suanpan_rejects_unaddressable_digit_landings`. Cases 4, 5, and 6 took
  70.2 s together (verified, `wasm3-fix.log`); case 5 streams half as many
  limbs as case 4, so about 14 s (inferred).
- **Overlaps.** `main`'s case 4, and #54's cases.
- **Dependencies.** #31 and #54 both edit that loop; land this after them.
- **Value, in one sentence.** A cheap extra boundary for the 32-bit landing
  pin, at an index no committed case reaches, though the defect it would
  catch is contrived.

### 6. The offset-comparison accounting probe

- **What it is.** Three parts at `5d5e33471` (verified):
  - process-global counters, `crates/before/src/testing/l7_compare.rs`, 39
    lines;
  - a hook in production code, in `Anchor::compare_offset_to`
    (`crates/before/src/version/range_minima/anchor.rs`), 12 lines under
    `cfg(all(test, feature = "touch-meter"))`;
  - the probe `l7_compare_offset_accounting`
    (`crates/before/src/testing/meter/board/l7_compare_probe.rs`, 86 lines),
    which ticks every board family with a cross bundle at four sizes, and
    14 constructed left chains.
- **What it reaches or checks.** It asserts nothing. Per family and size it
  prints the calls, the touches spent inside the comparison, the digits of
  its operands and of the gap, and the largest single call.
  - Ten board families reach the function. Touches per operand unit stay
    between 1.00 and 2.49 at every size. On MirrorWide every call costs
    exactly 3 touches while the gap grows from 16 to 125 digits (reported,
    [`L1-offset-comparison-composition.md`](../lanes/l7-suanpan/round-3/L1-offset-comparison-composition.md)).
  - The left chains make no calls: the comparison runs only after a raise
    or a closed pre-scan (reported).
- **Coverage beyond the committed suite.** It attributes tick's touches to
  one function and normalizes them by that function's operands; the board
  judges tick's total touches per input byte. No committed instrument
  states the per-call cost.
- **Evidence.** It answered a lead with "no defect". The auditor concluded
  that every failure it could construct here is one the board's tick cells
  already fail on (reported). Not calibrated.
- **Fold-in cost.** It needs a per-function hook in production code, which
  would be a new kind of instrument for `before`; the board meters whole
  operations. Folding it in means a board meter keyed by call site, or a
  property bounding touches per operand unit across the families. The
  probe's run time is unmeasured.
- **Overlaps.** The board's tick cells.
- **Dependencies.** None.
- **Value, in one sentence.** Low: a white-box reading that confirmed a cost
  argument whose consequence the board already enforces.

### 7. The readout carry census

- **What it is.** A suanpan feature, `l7-high-log`, that makes
  `Digits::read_digits` append each readout whose final carry is 2 or −3,
  or negative over an all-zero low part, to the file named by
  `L7_HIGH_LOG`, tagged with the test's name
  (`crates/suanpan/src/accumulator/digits/read.rs`, 27 lines, and
  `Cargo.toml`) (verified).
- **What it reaches or checks.** It counts; it asserts nothing. Over
  suanpan's committed suite it found no readout with carry −2 over a zero
  low part; carries of 2 and −3 appeared only in two randomized tests, and
  negative carries over a zero low part only as −1 (verified,
  `auditor-l7/r3/leads1.log`). Concurrent test processes interleave some
  log lines, so only per-class counts are reliable (reported, and visible in
  the log).
- **Coverage beyond the committed suite.** None now: #64 tests every
  reachable class directly. What remains is the ability to ask which
  classes another suite reaches, `before`'s for example, which was never
  run (reported, round-3 report).
- **Evidence.** It found the gap that #64 closed.
- **Fold-in cost.** A feature in production code that writes files does not
  fit the project's conventions; a test-only per-class counter, like the
  touch meter, would (inferred). Worth it only if someone wants the
  `before` census.
- **Overlaps.** #64.
- **Dependencies.** #27 changes the high-part code in `read.rs`, and #64
  stacks on it.
- **Value, in one sentence.** None beyond diagnosis, since #64 tests every
  class.

### 8. Prototype T (position tags)

- **What it is.** A suanpan feature, `l7-proto`: a tag per digit position
  that lets a write skip the zero-range map unless it lands on an untagged
  position at or below the old top, with a tag invariant added to
  `assert_invariants`. About 60 lines across `digits.rs`, `sign.rs`,
  `normalize.rs`, `digits/tests.rs`, and `zero_ranges.rs` (verified).
- **What it reaches or checks.** It is a design alternative, not a check.
  It passed suanpan's 70 tests and the pool model at 2,000 cases
  (reported, `proto2.log`). It flattens F1 but still grows on ±S and OS,
  because it removes only the first of the map's three costs (reported,
  F1b).
- **Coverage beyond the committed suite.** None. #86 deletes the map that T
  patches. As a partial fix, it shows that the F1 family alone cannot tell
  a partial repair from a full one; #52's ladder already covers that with
  its ±S and OS families.
- **Evidence.** Design evidence for F1b; it caught nothing.
- **Fold-in cost.** Not applicable.
- **Overlaps.** Prototype W, now #86.
- **Dependencies.** None.
- **Value, in one sentence.** None for the committed suite; it is design
  history.

### 9. The native size probe

- **What it is.** `l7_probe_version_sizes`
  (`crates/before/tests/explore_l7.rs`, 22 lines, feature `meter`), which
  prints each fuel family's encoded bytes and leaf count at `r` = 64 to
  16,384 (verified).
- **What it reaches or checks.** It asserts nothing; it supplied the
  denominators of the fuel tables.
- **Coverage beyond the committed suite.** None: #52's ladder measures each
  cell's encoded length as its denominator (verified, the ladder's `Cell`
  docs).
- **Evidence.** It supplied F1's and F1b's byte counts.
- **Fold-in cost.** Not applicable.
- **Overlaps.** #52.
- **Dependencies.** None.
- **Value, in one sentence.** None; #52 records the same numbers.

### 10. The touch replay probe

- **What it is.** `metered::l7_replay_touch_repro1` (`explore_l7.rs` lines
  795 to 869), which replays one hard-coded, shrunk 17-step program and
  prints touches, work, and widths at each step (verified).
- **What it reaches or checks.** It asserts nothing. It diagnosed a false
  failure caused by the process-global touch meter under concurrent tests
  (reported, round-1 `NOTES.md`).
- **Coverage beyond the committed suite.** None.
- **Evidence.** It exposed that harness mistake; it caught no defect.
- **Fold-in cost.** Not applicable.
- **Overlaps.** #37's failure messages, which name the failing step and its
  readings.
- **Dependencies.** None.
- **Value, in one sentence.** None.

### 11. Scratch drafts and table scripts

- **What it is.** In the session scratchpad's `auditor-l7/` (verified, read):
  drafts of branch code (`adversary.rs`, `guest_fuel.rs`,
  `probes_native.rs`); scripts that applied those drafts to the worktree
  (`fuel_patch.py`, `wasm_patch.py`); and formatters that turned fuel logs
  into the F1b and F1c tables (`m3/table.py`, `m3/mdtable.py`,
  `r3/table.py`).
- **What it reaches or checks.** Nothing; the drafts' committed forms are
  entries above.
- **Coverage beyond the committed suite.** None.
- **Evidence.** The formatters produced the F1b and F1c tables.
- **Fold-in cost.** Not applicable.
- **Overlaps.** Entries 2 and 4, and the probes carried by #28 and #48.
- **Dependencies.** None.
- **Value, in one sentence.** None; the tables they produced are in the lane
  records.
