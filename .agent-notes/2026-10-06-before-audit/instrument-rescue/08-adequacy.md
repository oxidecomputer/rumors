<!-- CAVEAT LECTOR: written by Claude (Opus 5.5) as the adequacy lane's cataloguer for the instrument rescue, from the lane's explore branch, its records, and the auditor's scratch directory, checked against `main`; for Finch's review. -->

# 08. The adequacy lane's instruments

## Summary

The adequacy lane (L8) audited the evidence rather than the code, so what it
built measures or calibrates the committed instruments. None of it is an
independent model of the crates' semantics. It built:

- a census of what the committed generators draw;
- a mutation campaign over both crates, with all 282 survivors classified;
- calibrations of four committed instruments: the wasm32 pins, the surface
  check, the asymptotics fold pins, and the detached workspaces' test time
  limits;
- two target-dependence probes and a branch-coverage map;
- small tests and probes for diagnosing individual survivors, and a harness
  that bisects a counter's drift across commits.

This file gives full entries for 15 instruments, ordered by my judgment of
value, and one line each for six items already carried by ready branches or
by `main`. The three I rank highest:

1. **The generator census** (entry 1): deterministic, diffable histograms of
   what the generators produce, and the cheapest source of floors for the
   relational classes no committed test watches.
2. **The wasm32 pin injection table** (entry 2): the only demonstration that
   the 32-bit pins fail on the narrowings their docs name (7 of 11 do), with
   a list of where they cannot.
3. **The gamma-window counterexample test** (entry 3): a finished
   0.05-second test for a contract no committed test exercises at unaligned
   starts.

Two facts bear on any rescue from this lane:

- **Several parts live only in the session scratchpad**, under
  `/private/tmp`, which does not survive a reboot (README, "Files"): the
  wasm32 injection table and its runner, the survivor splitter, the coverage
  splitter with its per-module files and merged profile, the bisect driver
  with its commit list, and the full caught, missed, and unviable outcome
  lists (verified, listing of `<scratch>/auditor-l8/`). Each entry says where
  its parts live.
- **Several probes relied on code inserted by hand into an untracked copy of
  the crates** (`l8/probe/` in the lane's worktree), which has since been
  rebuilt clean: the normalize shrink flag, the split-writer hit flag, the
  four reach-recorder sites, the libtest invocation test, and the window
  test's mutant switch. That code survives nowhere (verified: I searched the
  probe copy, the scratchpad, and the lane records). Only the logs of its
  runs remain.

**How I worked.** I read the branch's whole diff from its base
(`58285ca51..0acb87bdc`, nine signed commits) and the auditor's scratch
tools. I compared the branch's 41-file vendored `num-bigint` mechanically
with the crates.io copy: one line differs (verified). I read the lane's
reports, notes, findings, survivor index, and the briefs for unbuilt work in
full; the per-group survivor diff files and the briefs already built as ready
branches I counted or skimmed. I checked claims against `main` at
`dbc169291` with git and text searches. I used no box runs: the records hold
a runtime for every instrument that runs, and the integrator's rerun
reproduced the census at `main` (reported, `00-baseline.md` section 1).

**Conventions.** Every claim is marked *verified* (I checked it against a
file, a log, or the tree), *reported* (a record says so, and I name it), or
*inferred*. Paths without a prefix are relative to
`.agent-notes/2026-10-06-before-audit/`. `<scratch>` is the session
scratchpad,
`/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad`.
A `#n` is entry `n` of `QUESTIONS.md`. The committed suite's reach is in
`instrument-rescue/00-baseline.md`, cited by section.

## Already carried by ready branches or `main`

- **#57** (`audit/detached-nextest-timeouts`): a terminating time limit in
  each detached workspace, from the lane's brief. The lane's 200-second
  sleep test, which showed that no limit applied
  (`<scratch>/auditor-l8/w32/nextest-timeout-probe.log`), is superseded by
  the builder's hang injections in all four workspaces (reported, #57).
- **#66** (`fix/suanpan-normalize-width-shrink`): the exhaustive
  `normalization_is_exact_across_boundary_digits`, from the lane's normalize
  brief. It subsumes the lane's reachability probe
  `l8/l8_normalize_probe.rs`, in which 2 of 19 public constructions reached
  the width-shrinking loop (verified, `<scratch>/auditor-l8/normalize-probe.log`),
  detected by a flag that survives nowhere.
- **#78** (`audit/rank-decode-reader`): the scripted-reader property and the
  deterministic interrupt test, from the lane's rank-decode brief (reported).
- **#82** (`audit/range-minima-near-boundaries`): the three-form value
  generator and two exact cases through `tick` and `min_ticks`. The exact
  cases carry the values of the lane's two kernel-level counterexamples
  (`l8/range_minima_witness.rs`: a drop that crosses a `2^64` boundary beside
  a `2^200` one, and a `2^200` boundary lowered by 5) in public-API form
  (verified, #82's test docs). The coordinator declined the reviewer's
  suggested kernel-level case (reported, #82).
- **#83** (`audit/trait-coherence-and-hole-subtracts`): laws for `Hash`
  against the byte view, `Debug` against `Display`, and `ClockForks` hints,
  from the lane's trait brief; and the deletion of `hole_subtracts`, from its
  simplification brief (reported).
- **`main`, `ddfabe4cb`**: the worst-case pin declares
  `count_display × heap` `TARGET_DEPENDENT`, answering the lane's finding
  that the cell's ranking depends on the architecture (verified, the
  commit). The probe behind that finding is entry 13.

Two lane briefs were not built: the disjoint-families generator, dropped as a
reach gap without a failure (reported, `STATE.md` queue item 8), and the
lone-prefix padding tests, held until you document an error precedence
(entry 7).

## Entries, by value

### 1. The generator census

- **What it is.** A test-only measurement module: seven census functions
  that print histograms over any iterator of oracle or production values, a
  fixed-seed `sample` helper that draws from a proptest strategy without
  shrinking, and five `#[ignore]`d tests applying them to the committed
  strategies. It asserts nothing.
  `explore/l8-adequacy:crates/before/src/testing/l8_census.rs` at
  `0acb87bdc`, 485 lines, registered by two lines in `testing.rs` (verified).
- **What it reaches or checks.** Per value: depth, nodes, leaves, the widest
  base and widest root-to-leaf height in bits, nonzero interior bases, root
  kind, and encoded size. Per pair: the causal order of two versions, the
  region relation of two parties, and whether projecting a version by a
  party leaves it unchanged, empties it, or neither. Per trace: length, final
  population, operation mix, operations whose two operands resolve to one
  clock, the deepest party and version, and the widest height. Per family:
  how often the receiver and items are pairwise disjoint, by arity
  (verified, source). Its relation predicates are the recursive tree
  oracle's (`partial_cmp`, `is_disjoint`, `covers`), independent of
  production; encoded sizes go through the bridge to production's
  `encoded_bits` (verified, source). Its readings are the rescue's baseline:
  `00-baseline.md` sections 2.1 to 2.4 and 2.7 restate them.
- **Coverage beyond the committed suite.** The only committed census of
  generated values, `generator_classes_stay_under_mass`, holds floors for
  base-width classes and full depth (verified, `testing/generators/tests.rs`).
  The verdict matrix requires every verdict class, but over its constructed
  pool, not over generated pairs (`00-baseline.md` section 2.5). No committed
  test measures the relations of generated pairs, projection outcomes, trace
  degeneracy, population, or family success. This census is the source of
  the statements that independent version pairs are never equal (0 of
  20,000), that no arbitrary family drives a successful `join_all` of two or
  more items (0 of 17,781), and that organic heights never exceed 6 bits
  (verified, the logs).
- **Evidence.** It caught no defect; it measures. Its family census produced
  the disjoint-families brief, which was dropped (reported). Its output is
  deterministic: a rerun (`<scratch>/auditor-l8/census2.log`) matches
  `lanes/l8-adequacy/round-1/census-baseline.txt` on every histogram line,
  and its two family runs agree (verified, diff). The integrator's rerun at
  `main` reproduced every block (reported, `00-baseline.md` section 1).
- **Fold-in cost.** It fits as more floors beside
  `generator_classes_stay_under_mass`, for classes that have mass: disjoint
  and equal party pairs (13.9% and 3.1%), each ordered version relation,
  populations of three or more, and joins of two distinct clocks. Classes
  with no mass, equal version pairs and multi-input family success, cannot
  take a floor without a generator change, which was the dropped brief's
  choice. The work:
  - replace the printed histograms with floors and one summary line, as the
    committed census does;
  - give each test a doc comment stating its floor, and remove the lane tags
    from its prose;
  - read degenerate operations from the trace driver instead of re-deriving
    them. The census recomputes `optrace::apply`'s indexing and the organic
    drivers' `i % len` picks. Both match `main` today (verified,
    `optrace.rs:263-305` and `diff_ops/tests.rs:247-256`), but if either
    driver changed, the census would measure a schedule the tests no longer
    run, and nothing would fail.

  At 20,000 draws in a debug build on the box, the tests took 35.2 s
  (pairs), 9.0 s (versions), 3.9 s (traces), 3.6 s (parties), and 23.7 s
  (families) (verified, the logs), against nextest's 300-second limit. At the
  committed census's 3,000 draws they would take a few seconds (inferred, by
  proportion). No new dependencies.
- **Overlaps.** It extends the committed census floors and replaces nothing.
  #74 adds census floors for its own co-generated tick strategies
  (reported). The integrator built a family relation census on top of it for
  the baseline (reported, `00-baseline.md` section 1). Other lanes measured
  their own generators with their own scripts (reported, `instruments.md`);
  nobody has run this census on them, though its module doc says how.
- **Dependencies.** None. #74 adds strategies without changing existing ones
  (verified, its diff of the generator files is insertions only).
- **Value, in one sentence.** It makes generator reach a deterministic
  reading, and puts floors for the relational classes the differentials
  depend on a few lines away, at almost no runtime.

### 2. The wasm32 pin injection table and its driver

- **What it is.** A calibration harness for the wasm32 pins, in three parts:
  - `l8/w32.sh` (16 lines, `explore/l8-adequacy` at `0acb87bdc`) builds the
    guest and the harness from the probe copy and runs the pins a nextest
    filter selects;
  - `w32/injections.py` (about 70 lines) lists eleven injections, each an
    exact-count string swap in production source with the pin expected to
    fail, plus four trap probes;
  - `w32/run-injections.sh` (35 lines) applies each to a freshly rebuilt
    probe copy, runs its pin on the box, and appends a verdict to
    `w32/verdicts.txt`.

  The table and runner live only in `<scratch>/auditor-l8/w32/` (verified).
- **What it reaches or checks.** Each injection is the narrowing or wrong
  route a pin's doc names, or its nearest site: three decode-length
  narrowings, two cursor-position narrowings on the compare path, a writer
  length held in `usize`, the rank fraction length, the rank arithmetic
  route and the accumulator's sign, the fork count, and suanpan's digit
  landing (verified, the table). Each of the seven value pins at the base
  gets at least one. The oracle is the pin's own assertion. The trap probes
  insert `assert!(position < 2^32)` and `assert!(position < 2^20)` at the two
  read sites the compare pin passes through.
- **Coverage beyond the committed suite.** No committed test shows a wasm32
  pin failing on the defect it names. The harness's own
  `harness_outcomes_are_live` shows only that the protocol can report a
  pass, a failure, and a trap (verified, `pins.rs:32-44`). The pins are the
  audit's instrument of record for 32-bit targets, and this table is their
  only known-bad set.
- **Evidence.** Seven of the eleven are caught (each pin exits 100). Four
  pass: `compare-reader-open`, `compare-gamma-window`, `join-live-usize`, and
  `rank-arith-route` (verified, `verdicts.txt`). The lane traced each miss to
  the pin's input: the compare pin never places a live bit at or past
  `2^32`, the join path uses the writer's length only modulo 8, and the
  contiguous rank route is correct at the pin's gap (reported,
  `findings/wasm32-pins-calibration.md`). Those findings became #92's
  compare-pin doc and index entry, #63's rewrite of the rank-route doc, and
  the follow-up on narrowed cursor positions (reported). One correction: the
  record calls the `2^20` probe a positive control, but it passed too, so the
  probes show that no position at or past `2^20` reaches either site, not
  that the sites run at all. The conclusion that the pin cannot see a
  narrowed position holds either way (verified, the four logs; the
  conclusion inferred).
- **Fold-in cost.** No shared instrument hosts a calibration of a detached
  workspace. Each injection rebuilds the guest, and each took between 31 and
  121 seconds end to end (verified, `verdicts.txt`), about 11 minutes for all
  eleven, so the set cannot be one nextest test. A committed form would be a
  `just` recipe outside the landing check that applies each swap to a
  scratch copy and requires its pin to fail, or compile-time switches in
  production source like the survey's run 1 (reported, survey section 1.9).
  The table is Python; no crate dependency. All fifteen swaps apply at `main`
  with their expected counts (verified, my count script,
  `<scratch>/rescue-l8/scripts/inj_applies.py`). On ready tips, #43 removes
  the fork-count site and #63 removes both rank-route sites (verified), and
  the pins that #28, #29, #48, #50, and #54 add have no injection.
- **Overlaps.** #92's builder placed a deciding leaf at bit `2^32 + 3` in a
  guest probe, and #31's reviewer probed trap outcomes (reported, the
  ranker's probe handoff); neither is committed. It would stand beside the
  pins as their adequacy check.
- **Dependencies.** None to run at `main`. #58 restructures the pins'
  protocol but keeps every test name the filters select (verified, `pins.rs`
  on its tip).
- **Value, in one sentence.** It is the one demonstration that the 32-bit
  pins fail on the narrowings they claim to catch, and it lists where they
  cannot.

### 3. The gamma-window counterexample test

- **What it is.** One unit test, `l8_witness_window_unaligned_long_codes`,
  in `explore/l8-adequacy:l8/window_witness.rs` at `0acb87bdc` (38 lines).
  It is identical to
  `lanes/l8-adequacy/round-2/addendum-bits-party/witness/window_witness.rs`
  (verified, checksums), and was run from the probe copy inside
  `bits/tests.rs`.
- **What it reaches or checks.** 42 codes: six values whose gamma codes are
  59 to 63 bits long, each written after 1 to 7 leading bits and followed by
  16 filler bits. It checks that `BitsReader::gamma_from_window` returns the
  exact value and end position, so each code's last bits come from the
  window's ninth loaded byte (verified, source). The oracle is production's
  `BitsWriter::write_gamma`, which the codec suites check against canonical
  bytes separately (inferred).
- **Coverage beyond the committed suite.** The committed window tests call
  the window on long codes only at position 0, and at unaligned positions
  only on a two-bit stream (verified, `bits/tests.rs:604-679`). The test-only
  `ReferenceBitsReader` also decodes gamma codes through the window (verified,
  `bits/reader/reference.rs:48-56`), but the three mutants of the ninth-byte
  merge at `window.rs:47` survive the whole suite (reported, survivor index),
  so no committed assertion depends on that merge (inferred). This test
  checks the window against the writer, independently of both readers that
  use it.
- **Evidence.** It kills the campaign's three `gamma/window.rs:47`
  survivors: the unmutated window gets all 42 codes right, and the three
  mutants get 39, 26, and 20 wrong (verified, `witness/window.log`). Those
  survivors are unreachable from production today, because borsh's stream
  reader buffers at most 7 bits past the position (reported, survivor index).
  `follow-ups.md` already names this file as a ready regression test.
- **Fold-in cost.** Move it into `bits/tests.rs` beside `gamma_window_edge`,
  drop its lane prefix and the mutant-switch print, and state its invariant
  in the doc comment. It runs in 0.05 seconds (verified, log). No
  dependencies; the window's code at `main` is the base's (verified).
- **Overlaps.** It stands beside `gamma_window_edge` and
  `gamma_window_declines_conservatively`. No other lane's instrument tests
  the window.
- **Dependencies.** None.
- **Value, in one sentence.** It is the cheapest fold-in of the lane: a
  finished test for the window's general contract, in place before any
  caller with a fuller buffer relies on it.

### 4. The classified mutation survivors

- **What it is.** A record, not code. `l8/records/survivors/INDEX.md` (196
  lines) classifies all 282 survivors of the campaign (entry 5), and eight
  `diffs/<group>.md` files give each survivor's cargo-mutants name, enclosing
  function, and exact diff (verified, 282 entries). It is on
  `explore/l8-adequacy` at `0acb87bdc`, byte-identical to
  `lanes/l8-adequacy/round-2/addendum-bits-party/survivors/` (verified,
  checksums). The full outcome lists, 3,041 caught, 282 missed, and 596
  unviable, exist only in `<scratch>/auditor-l8/survivors/raw-*/` (verified,
  line counts).
- **What it reaches or checks.** Every mutant cargo-mutants generates in both
  crates' production code. The classes and their counts (reported,
  `INDEX.md`; they sum to 282, verified):
  - a reachable value change no test detects: 21;
  - trait behavior callers rely on (`Hash`): 3;
  - trait spelling only (`Debug` text, a delegating operator, an advisory
    `size_hint`): 41;
  - cost only: 96;
  - equivalent on every input: 69;
  - unreachable: 41;
  - instrument or test support compiled into the crate: 10;
  - undecided: 1.

  Each equivalent and unreachable entry carries a one-line argument, and
  each cost-only entry names the meter expected to see it.
- **Coverage beyond the committed suite.** The repository records nothing
  about which mutants its suite misses (verified, entry 5). The record holds
  three things no committed instrument does:
  - a baseline against which a later campaign shows a weakened test as a
    newly surviving mutant;
  - 96 cost-only mutants, a ready calibration set for the board and the
    fuzz-fit bands. Only six have run against any cost instrument (the
    survey's five, section 1.9, and entry 12's fold mutant), and none against
    the board (reported);
  - 110 arguments that a mutant is equivalent or unreachable: claims about
    the code that no test states, such as `alignment_fits` being constant on
    a 64-bit host.
- **Evidence.** Every brief the lane wrote came from it. After the ready
  branches land, its 21 reachable-value survivors fall to 2, the padding pair
  of entry 7 (reported, the index's status section and survey section 2.3).
  The fuel bands catch two cost-only survivors and nothing catches three
  (reported, survey section 1.9). It describes `main`'s code as well as the
  base's: their production sources differ only in suanpan's `digits/add.rs`,
  where all 41 mutants were caught (verified). `main`'s suite has gained
  tests from two landed fixes, which may kill some survivors (not measured).
- **Fold-in cost.** Committing it as a baseline raises a question under your
  rule that no mechanism may accept known failures. The equivalent and
  unreachable entries state positively why a mutant cannot change behavior,
  which reads as a declared model. The trait-spelling, cost-only, and
  held-by-decision entries are accepted gaps, and a committed list that a
  later campaign must match would accept them. Mutant names carry line and
  column (`crates/before/src/version.rs:170:9`), so any edit to a mutated
  file renames them; a comparison would have to match by file, function, and
  replacement (verified, the raw lists). It has no runtime of its own.
- **Overlaps.** Survey section 2.3 summarizes it; `follow-ups.md` carries its
  window and cost-only items.
- **Dependencies.** #43, #63, #66, #67, #78, #82, and #83 each kill or delete
  survivors (reported, the index's status section).
- **Value, in one sentence.** It answers whether the suite is still as strong
  as it was without re-deriving 282 classifications, and its cost-only class
  is the first calibration set the cost instruments lack.

### 5. The mutation campaign tooling

- **What it is.** Scripts for running cargo-mutants on the box, on
  `explore/l8-adequacy` at `0acb87bdc`:
  - `l8/mutants-common.sh` (15 lines): the shared flags, seed, and
    exclusions;
  - `l8/campaign.sh` (29 lines): eight named groups, one output directory
    each;
  - `l8/export_survivors.py` (30 lines): every missed mutant of an output
    directory as one Markdown file with its diff;
  - `l8/cpu.sh` (16 lines): the summed CPU of the campaign's process tree,
    using illumos's `ptree`.

  A fifth script, `split-survivors.sh` (37 lines), fetches the outcome lists
  and splits survivors by module; it lives only in `<scratch>/auditor-l8/`
  (verified).
- **What it reaches or checks.** cargo-mutants 27.1.0 over
  `crates/before/src` (excluding `src/testing/` and test files) and
  `crates/suanpan/src`: 3,919 mutants in eight groups, each judged by its
  crate's whole nextest suite with `--all-features` under
  `PROPTEST_RNG_SEED=8008` (verified, the scripts and outcome counts; the
  version reported). The oracle is the committed suite. The landing check's
  other legs, the board, the fuzz-fit bands, the wasm32 pins, and the surface
  check, do not run per mutant (verified, the flags).
- **Coverage beyond the committed suite.** The repository has no
  mutation-testing recipe or configuration (verified: nothing in the
  justfile, `.config/`, or CI). The other lanes' mutants were hand-chosen to
  calibrate their own instruments (reported, `instruments.md`); this is the
  only campaign over every mutation cargo-mutants applies.
- **Evidence.** The suite killed 3,041 of 3,323 viable mutants, 91.5%
  (verified, from the outcome counts). Entry 4 is its output.
- **Fold-in cost.** It is not a nextest instrument. A recipe would carry
  `mutants-common.sh`'s settings; the scripts hard-code the auditor's
  worktree path, and `cpu.sh` runs only on illumos. A full run took about 11
  hours from launch to completion, with two processes sharing the box, at
  about 17 seconds per mutant under a load of 50 to 190 (reported, the lane's
  notes and status file). cargo-mutants is a tool install, not a crate
  dependency. The fixed seed makes a run reproducible but hides how detection
  varies by seed: #82's property catches one of its mutants on 16 of 20 fresh
  seeds (reported, #82).
- **Overlaps.** It checks every other instrument at once and replaces none;
  the branch-coverage map (entry 9) is a coarser view of the same question.
- **Dependencies.** None.
- **Value, in one sentence.** It is the only total measure of what the
  committed suite misses, too slow for every commit but suited to a phase
  boundary or a release.

### 6. The `usize` signature census

- **What it is.** `l8/usize_census.py` (61 lines, `explore/l8-adequacy` at
  `0acb87bdc`). It reads `before`'s rustdoc JSON, built `--all-features` as
  the surface check builds it, and lists every public function-like item
  whose inputs or output mention `usize` (verified, source). Its output is
  only in `<scratch>/auditor-l8/usize-census.txt`.
- **What it reaches or checks.** All of `before`'s public items, including
  those behind the `meter` feature. The output lists 30 items, of which 9 are
  production API: seven `Iterator::size_hint` impls, `From<usize> for Count`,
  and `TryFrom<Count> for usize`. The other 21 are `meter`-feature test
  support (verified, the output). It does not read suanpan.
- **Coverage beyond the committed suite.** The surface check reconciles
  functions, trait impls, and items against the board, but looks at no
  signature's types (reported, survey section 2.2; consistent with
  `surfacecheck/src/check/tests.rs`, verified). Your `usize`-invariance rule
  treats a new `usize` parameter or stored quantity as a possible defect, and
  nothing would report one today.
- **Evidence.** It found no public production parameter typed `usize`
  outside standard conversions and iterator hints (verified, the output). It
  caught nothing. suanpan's `reserve_digits(usize)`, which #28 replaces, was
  outside its reach.
- **Fold-in cost.** It belongs in `surfacecheck` as one more census,
  reconciled both ways like the trait-impl census: a committed list of the
  items allowed to mention `usize`, failing on any addition or orphan. That
  means porting 61 lines of Python over raw JSON to Rust over the parsed
  rustdoc types the check already holds (inferred). Its runtime is negligible
  beside the rustdoc JSON build the surface leg already performs (inferred).
  No new dependency. suanpan has no surface check (reported, survey section
  2.2), so its signatures stay unwatched either way.
- **Overlaps.** It reads the same data as the surface check's censuses. The
  wasm32 pins check width behavior at run time; this would check it at the
  API.
- **Dependencies.** None. #43 changes the fork hints' values, not their
  signatures.
- **Value, in one sentence.** It turns a rule you stated, which nothing
  checks, into a reviewed event for every new `usize` in `before`'s public
  signatures, at negligible runtime.

### 7. The lone-prefix padding tests

- **What it is.** Two counterexample tests and a driver on
  `explore/l8-adequacy` at `0acb87bdc`: `l8/span_witness.rs` (18 lines),
  `l8/clock_witness.rs` (12 lines), and `l8/witness.sh` (7 lines). The test
  brief
  `lanes/l8-adequacy/round-2/addendum-bits-party/briefs/test-span-lone-endpoint-padding.md`
  holds the polished pair, which the lane also ran on #67's tip (verified).
- **What it reaches or checks.** A one-byte buffer holding only the first
  field of a two-field encoding, `Span`'s lower endpoint (`Version::new()`)
  or `Clock`'s party (`Party::seed()`), with a set padding bit after the
  marker. The tests require `Decode::TrailingBits` for it, and `Truncated`
  for the clean one-byte buffer (verified, source). The oracle is the
  expected error, stated by hand.
- **Coverage beyond the committed suite.** No committed test decodes a lone
  first field with bad padding: both `>` to `>=` survivors, at
  `span/wire.rs:136` and `party/io.rs:29`, pass the suite (reported,
  survivor index).
- **Evidence.** At the base both tests pass, and each fails under its
  survivor with `Err(Truncated)` (verified, `witness/span-*.log` and
  `witness/clock-*.log`). On #67's tip, which merges both checks into
  `Bits::padded_len`, both pass, and both fail when that guard's `<=` becomes
  `<` (verified, `witness/padded-len-67.log`).
- **Fold-in cost.** Two unit tests, in `span/tests.rs` and `clock/tests.rs`,
  about 0.05 seconds each (verified, logs). But the input is malformed twice
  over, short and badly padded, and `Decode`'s documented contract permits
  either error. #67's review reached the same conclusion for this pattern,
  and the coordinator held the tests until you document a precedence
  (reported, `STATE.md` queue item 8 and survey section 2.3). Folding them in
  would commit to a choice the rustdoc does not promise.
- **Overlaps.** The codecs lane's specification codec enforces only `Span`'s
  documented precedence and does not catch these survivors (reported, survey
  section 2.1).
- **Dependencies.** #67 merges the two sites; the tests apply to it unchanged
  (verified, log).
- **Value, in one sentence.** They are worth folding in only if you decide a
  caller may read `Truncated` as "read more"; otherwise they would fix an
  error order you have not promised.

### 8. The probe copy and its hit recorder

- **What it is.** Tooling that marks production lines and records which tests
  reach them, without touching the source tree the mutation campaign copies.
  On `explore/l8-adequacy` at `0acb87bdc` (verified):
  - `l8/mkprobe.sh` (47 lines) and `l8/mkprobe-at.sh` (36 lines) build an
    untracked workspace, `l8/probe/`, holding copies of `before` and
    `suanpan` from the worktree or from any commit, under a root manifest
    trimmed to the two crates;
  - `l8/probe-macro.rs.inc` and `l8/probe-module.rs.inc` (8 and 25 lines)
    add an `l8_probe!("name")` macro that writes the first hit of each named
    line per test process, with the test's name, to the file `L8_PROBE_OUT`
    names.
- **What it reaches or checks.** Any line a maintainer marks, under the whole
  suite. Four sites were marked and recorded: the two domination fast paths
  in `range_minima/boundary.rs`, reached by 9 and 26 tests, and two
  version-writer paths (`split_splice_continuation`, `take_left_wide_split`),
  reached by 16 and 56 (verified, `<scratch>/auditor-l8/probe-hits*.txt`).
  That separates a survivor whose line no test runs from one whose effect no
  test checks.
- **Coverage beyond the committed suite.** Nothing committed records which
  tests reach a line. Branch coverage says whether a line runs at all, and it
  cannot run on the box, whose toolchains lack the profiler runtime
  (reported, `lanes/l8-adequacy/round-1/coverage/NOTES.md`).
- **Evidence.** It showed that both `boundary.rs` survivors were reached and
  unchecked, which shaped the brief #82 implements. `mkprobe-at.sh` also
  drives the bisect harness (entry 10). The marked sites were edits to the
  untracked copy and survive nowhere (verified); only the hit tables remain.
- **Fold-in cost.** No shared instrument corresponds. It would be maintainer
  tooling under a scripts directory, changing nothing in `crates/`. The copy
  omits `fuzz/` and `fuzzfit/`, so two `fuzz_seeds` tests fail inside it
  (verified, `probe-hits-run.log`: 686 passed, 2 failed). The full suite ran
  in 34.8 seconds after the build (verified, the same log). The recorder is a
  `#[doc(hidden)] pub mod`, which the surface check cannot see, by its
  documented boundary (reported, `findings/instrument-calibrations.md`).
- **Overlaps.** The branch-coverage map (entry 9) answers the coarser
  question.
- **Dependencies.** None.
- **Value, in one sentence.** It is a profiler-free way to ask which tests
  reach a line, worth keeping only if mutation work recurs.

### 9. The branch-coverage map

- **What it is.** One llvm-cov run on the Mac, under your one-campaign
  exception, at `f27c324a` (production code identical to the base), with the
  report rebuilt by hand: `llvm-profdata merge` over 807 profile files and
  `llvm-cov export` over the 14 test binaries, because cargo-llvm-cov 0.8.7
  did not find binaries under the Mac's `build.build-dir` (reported,
  `lanes/l8-adequacy/round-1/coverage/NOTES.md`). `split-coverage.py` (79
  lines) splits the result into one file per production module, listing
  never-run lines and never-taken branch outcomes with their source text.
  The summary and notes are in the lane records; the script, the merged
  profile, the lcov file, and the per-module files exist only in
  `<scratch>/auditor-l8/` (verified).
- **What it reaches or checks.** Both crates' production code: 10,102 of
  10,324 lines and 1,576 of 1,652 branch outcomes ran (verified,
  `SUMMARY.txt`). LLVM counts conditions and `let ... else`, not match arms
  (reported).
- **Coverage beyond the committed suite.** No committed coverage measurement
  exists (verified, entry 5).
- **Evidence.** It located the rank decoder's reader paths (#78), the dead
  `hole_subtracts` (#83), `Rank::accumulate` running only in the wasm32 pins,
  the normalize shrink loop (#66), and the writer's early return that #67
  deletes (reported, the coverage notes).
- **Fold-in cost.** It cannot run on the box (reported), so a recipe would
  run on the Mac or in CI, needing `cargo-llvm-cov` and `llvm-tools`, and
  either a cargo-llvm-cov that honors `build.build-dir` or the manual merge.
  The instrumented test run took 102 seconds (reported). It is a periodic
  report, not a test.
- **Overlaps.** The mutation campaign is finer, since a covered line can
  still hide a survivor; the probe recorder says which tests reach a line.
- **Dependencies.** None.
- **Value, in one sentence.** It is a one-time map whose findings are all
  routed, useful again only as a periodic report on a machine other than the
  box.

### 10. The counter bisect harness

- **What it is.** `l8/seam.sh` (8 lines, `explore/l8-adequacy` at
  `0acb87bdc`) runs the committed test
  `skyline_min_ticks_stopping_boundary_is_flat_per_unit` in the probe copy
  and prints its `diff_large` reading. `seam/bisect.sh` (21 lines) bisects a
  list of 25 first-parent commits that touch either crate
  (`seam/commits.txt`), rebuilding the probe copy at each with
  `mkprobe-at.sh`. The driver and the list live only in
  `<scratch>/auditor-l8/seam/` (verified).
- **What it reaches or checks.** One counter that one committed test prints,
  across commits, without disturbing the worktree's checkout. It classifies
  each reading as old (7,867) or new (8,892) and stops on any other value
  (verified, source).
- **Coverage beyond the committed suite.** Nothing committed attributes a
  counter's movement to a commit.
- **Evidence.** Five builds attributed the reading's rise from 7,867 to
  8,892 to `fdd1bf47` (verified, `seam/bisect.txt`). The mechanism, a
  debug-only `clone().cmp_zero()` that #38 deletes, is reported
  (`findings/coordinator-leads.md` item 2). The attribution fed survey
  section 1.10.
- **Fold-in cost.** As a standing tool it would need the test, the reading's
  pattern, and the old and new values as parameters. `git bisect run`
  already performs the search; the side copy exists only because the
  mutation campaign was copying the worktree at the time (reported, the
  lane's notes). Runtime per step unmeasured.
- **Overlaps.** #95 compares two board runs exactly; this follows one focused
  reading across many commits.
- **Dependencies.** None.
- **Value, in one sentence.** It is low as an instrument; what is reusable
  amounts to a few lines of predicate script for `git bisect run`.

### 11. The surface check's end-to-end calibration

- **What it is.** `l8/surface.sh` (14 lines, `explore/l8-adequacy` at
  `0acb87bdc`) builds `before`'s rustdoc JSON from the probe copy with the
  pinned nightly and runs the surface check on it. Two injections were made
  by hand: a new public crate-root function, and `impl fmt::Binary for Count`
  (reported, `findings/instrument-calibrations.md`).
- **What it reaches or checks.** The surface check's whole path, from the
  rustdoc JSON of `before` itself through extraction to its findings.
- **Coverage beyond the committed suite.** The committed `check/tests.rs`
  demonstrates each finding category on synthetic surfaces, and
  `empty_extraction_trips_the_anchors` catches an extraction that finds
  nothing (verified, source). No committed test plants an item in a
  compiled crate and runs the extractor on its rustdoc JSON (inferred, from
  the test list).
- **Evidence.** Both injections fail with the expected finding, exit 1:
  "public function-like items with no board disposition and no exception
  (1): l8_unpriced_probe", and "reachable trait impls with no census pin and
  no exception (1): Count: impl core::fmt::Binary for Count" (verified,
  `<scratch>/auditor-l8/surface/*.log`). A `#[doc(hidden)]` public module
  passed unnoticed, which is the check's documented boundary (reported).
- **Fold-in cost.** A fixture crate with one planted function and impl,
  documented to JSON inside the surface check's tests, which would need the
  nightly toolchain those tests do not yet invoke (inferred). Runtime
  unmeasured.
- **Overlaps.** The committed synthetic tests cover the judgment; this would
  add only extraction.
- **Dependencies.** None.
- **Value, in one sentence.** It is low: the failure it would add, an
  extractor that drops an item rustdoc emitted, is partly caught by the
  anchors test.

### 12. The fold-pin calibration

- **What it is.** A calibration made of existing parts: the campaign's
  `balanced_try_fold` survivor that turns `weight += 1` into `weight *= 1`,
  applied in the probe copy, with readings taken from the committed
  asymptotics pins' own printed scan bits. No new code; logs in
  `<scratch>/auditor-l8/fold-*.log` only (verified).
- **What it reaches or checks.** The five fold entry points at arities 256
  and 1,024, on the pins' committed populations.
- **Coverage beyond the committed suite.** It shows that the five log-factor
  pins check only a floor. The mutant turns the balanced fold into a
  sequential one, so `Version::join_all`'s growth across the fourfold step
  rises from 5.82 to 15.75, and every pin passes (verified, the logs'
  readings and the arithmetic). That the board would catch it, at about 134
  scan bits per unit against a ceiling of 17, is inferred from arithmetic;
  nobody ran the board on it (reported, survey section 2.2).
- **Evidence.** No defect: a measured blind spot of the pins, which their
  design accepts.
- **Fold-in cost.** As a committed known-bad, it would need the board's
  acceptance sweep run on the mutant: unmeasured, and far beyond a nextest
  test (inferred).
- **Overlaps.** Entry 4's cost-only class contains it.
- **Dependencies.** None.
- **Value, in one sentence.** Its lesson is already recorded; as an artifact
  it matters only as one member of entry 4's cost-only calibration set.

### 13. The `num-bigint` formatting probe

- **What it is.** A detached workspace, `l8/nbprobe/` on
  `explore/l8-adequacy` at `0acb87bdc`: a 57-line driver, two manifests, and
  a vendored `num-bigint` 0.4.8 whose only change is `FAST_DIV_WIDE = false`
  (verified, by comparison with the crates.io source). Built twice, it prints
  the peak heap of formatting a `BigUint` in decimal, per unit of the board's
  `count_display` denominator, at about 90 widths from 64 bits to `2^19`
  (verified, source). The comparison table is only in
  `<scratch>/auditor-l8/nbprobe-compare.txt`.
- **What it reaches or checks.** Both settings of a target-conditional path
  in a dependency, on one host.
- **Coverage beyond the committed suite.** `main` declares the cell
  target-dependent and requires each such declaration to name its path in a
  comment (verified, `ddfabe4cb`); nothing executes the path to show the
  ranking really depends on it. This probe is the only executable
  demonstration.
- **Evidence.** Its x86 column reproduces the box's board readings, and on
  other architectures the pinned family wins by about 0.1% (reported,
  `findings/count-display-heap.md`). That finding became `ddfabe4cb`.
- **Fold-in cost.** It needs the vendored crate (about 15,000 lines) or a
  patch, for one fact about a dependency, and a committed test on one host
  can exercise only that host's setting (inferred). No shared instrument
  fits.
- **Overlaps.** None.
- **Dependencies.** None.
- **Value, in one sentence.** It is diagnostic only: its finding is settled
  on `main`, and it would matter again only if `num-bigint` changed its
  decimal conversion.

### 14. The split-writer hunt

- **What it is.** `l8/writer_hunt.rs` (42 lines, `explore/l8-adequacy` at
  `0acb87bdc`): an ignored test that ticks 300,000 random version and party
  pairs, drawn from a deeper recursive strategy (versions to depth 7, parties
  to depth 6), and stops at the first that sets a flag at
  `SplitOutput::splice_continuation`'s early return for an end just before a
  final leaf flag (verified, source). The flag, `L8_SPLIT_NARROW_HIT`, was
  inserted in the probe copy and survives nowhere (verified). A second
  generator, aimed at a wide left subtree beside a narrow two-level right
  one, also ran (reported, survivor index) and is not on the branch.
- **What it reaches or checks.** 600,000 ticks across the two runs, none
  reaching the return (verified, `witness/writer-hunt*.log`: "NO WITNESS in
  300000 cases" twice, in 336.6 and 98.8 seconds).
- **Coverage beyond the committed suite.** Only its negative result: random
  ticks never reach that return. #67 deletes the branch, arguing that no
  caller can reach it (reported, #67).
- **Evidence.** It supports #67's deletion.
- **Fold-in cost.** None sensible; its target disappears when #67 lands.
- **Overlaps.** The coverage map, which also found the return never runs.
- **Dependencies.** #67.
- **Value, in one sentence.** It has none once #67 lands; it records a search
  that found nothing.

### 15. The libtest invocation probe

- **What it is.** A one-test binary, `l8_env_probe`, that printed its
  arguments, `RUST_TEST_THREADS`, the available parallelism, and its thread
  name under nextest. Its output is `l8/records/witness/env-probe.log` on
  `explore/l8-adequacy`; its source survives nowhere (verified).
- **What it reaches or checks.** How nextest invokes a libtest binary on the
  box: `--exact <name> --nocapture`, `RUST_TEST_THREADS` unset, parallelism
  192, and the test on a thread named after it (verified, the log).
- **Coverage beyond the committed suite.** None; it records facts about the
  runner.
- **Evidence.** It grounded the lane's diagnosis of the `PeakAlloc` heap
  check's rare failure as libtest's main thread allocating inside the
  measurement window (reported, `findings/coordinator-leads.md` item 1). A
  later probe showed that single-threaded libtest does not close the race,
  and your ruling on question 91 replaces the allocator (reported, the
  survey's addendum).
- **Fold-in cost.** Nothing to fold in; it is three print statements to
  recreate (inferred).
- **Overlaps.** The libtest race probe in
  `<scratch>/builder-peakalloc-single-thread/`, which your ruling names as
  the fix's calibration (reported).
- **Dependencies.** None.
- **Value, in one sentence.** It has none as an instrument; its finding is
  recorded.

## Corrections to the records

1. The ranker's handoff attributes #38's value pool to this lane ("L8
   debug-assert scans (O2) value pool"). #38 follows the suanpan lane's
   observation O2 (verified: #38's entry, and the header of
   `coordinator-briefs/simplify-debug-assert-scans.md`). This lane's link to
   #38 is the bisection in entry 10, which attributed a touch rise to the
   debug check #38 deletes.
2. `findings/wasm32-pins-calibration.md` calls the `2^20` trap probe a
   positive control. It passed, so it shows that no position at or past
   `2^20` reaches the probed sites, not that they run (entry 2).
3. The family census found no multi-input success in 17,781 draws, not
   "about 17,700" (verified, `census-family.log`).
4. The campaign's final kill rate is 91.5% of viable mutants; the round-2
   report's 91.1% was taken partway (verified, entry 5).

## An observation noticed in passing

`ReferenceBitsReader`'s doc calls it "a deliberately bit-at-a-time bounded
reader", and it serves as the oracle in the reader tests, but its
`read_gamma` decodes through the window first and falls back to the bit loop
only when the window declines (verified, `bits/reader/reference.rs:9-56`).
Production's `BitsReader` does not call the window (verified, the callers of
`gamma_from_window`), so the reader tests still compare two different
decoders, and I found no common-mode failure. The doc overstates the
oracle's independence, which is a prose correction for the docs pass; it is
also why entry 3 is the window's only independent check. I have not acted on
it.

## What I could not assess

- The runtime of any folded form: every such figure above is inferred by
  proportion or marked unmeasured.
- Whether the board convicts any cost-only survivor: never run.
- Whether `usize_census.py` parses the current nightly's rustdoc JSON: not
  run, and the JSON format is versioned per nightly.
- The source of the second writer-hunt generator and of every probe inserted
  by hand: lost.
