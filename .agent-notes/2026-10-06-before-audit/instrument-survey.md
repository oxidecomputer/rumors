<!-- CAVEAT LECTOR: written by Claude (Opus 5.5) as the surveyor for the audit's final phase, from the audit's records and two runs on ox-east-1; for Finch's review. -->

# The instrument survey: candidates ranked by a named failure

This survey answers one question for every candidate instrument the audit
collected. The question is the one your caution set:

> Name a concrete failure (a defect, a false pass, a false failure, or a
> regression) that this candidate catches and the committed instruments
> miss, and point at where that failure was constructed or observed.

A candidate with such a failure is *eligible* for a proposal branch, and
section 1 ranks those. A candidate whose value is reach, independence, or
extra safety, with no failure behind it, is *listed* in section 2 with
what it reaches and why no failure is known. Section 3 maps what the
validation index and the crates' `# Testing` sections leave out. Section 4
says where this rule cut the older survey design in `instruments.md`, and
section 5 lists what I found wrong in the brief and in the records it
cites.

**Counts.** Ten candidates are eligible. 37 are listed: 12 explore-branch
instruments in section 2.1 and 25 collected or design candidates in
section 2.2, where a few entries group two or three related candidates.

**Base.** `main` at `599bf7d3c`. Explore branches were read with
`git show`; none was checked out. A `#n` is entry `n` of `QUESTIONS.md`,
which names its branch and worktree.

**Provenance marks.** Every claim carries one of three marks:

- *verified*: I checked it myself, against the code at `main`, a branch
  tip, a log file, or a run of my own;
- *reported*: an agent's record says so, and I did not re-check it. The
  record is named;
- *inferred*: my reasoning from what I read, not checked by a run.

**Runs.** The brief allowed four runs on ox-east-1, only for claims that
reading cannot settle. I used two, both in my scratch worktree
`/Users/oxide/src/rumors-survey` at `main`:

- *Run 1* compiled the record's five cost-only mutation survivors into
  the fuzz-fit guest, one at a time, and ran the fuzz-fit suite, to test
  the claim that only wall time sees them (1.9). Four variants covered
  the five, because the two `writer.rs:363` survivors behave identically.
  The switch was a compile-time constant read from an environment
  variable, so the unmutated build carries no extra instructions, and its
  baseline passed 25 of 25.
- *Run 2* built rustdoc with warnings denied for the two in-scope
  detached workspaces (section 2.2).

Their scripts, logs, and the reverted scaffold are beside this file in the
session scratchpad's `surveyor/` directory: `survey-fuel.sh`,
`run1-fuel.log`, `scaffold.diff`, `survey-docs.sh`, `run2-docs.log`, and
`bands_drift.py` for one comparison in section 2.2.

**How the ranking orders them.** By the consequence of the failure: an
asymptotic regression passing every cost instrument first, then a value
defect that a pin claims to catch and cannot, then movement the gate's own
instruments hide, then recurring false failures, then narrower blind spots
and constant factors. Each entry states its dependencies, so the
coordinator can sequence a blocked one later without re-ranking.

---

## 1. Eligible candidates, ranked

### 1.1 A time meter over `before`'s `num-bigint` work

**The failure.** On `fix/before-min-ticks-heap` (fix commit `1a31f8d1`),
`Version::min_ticks` reads big-integer limbs quadratically on a
constructed input, and every board counter read the same as at `main`.

- *Where it was constructed.* The final reviewer's probe
  (`reviewer-min-ticks-final/reviewer_probe.rs`) builds a left comb of `k`
  teeth under an outer minimum whose offset is about `2^(64k)`. Limbs read
  by the suspended-record storage, cross-prefix shape (verified from the
  logs `run1-branch.log` and `run2-parent.log`):

  | k | input bytes | branch | parent |
  |---|---|---|---|
  | 250 | 4,314 | 125,751 | 251 |
  | 500 | 8,626 | 501,501 | 501 |
  | 1,000 | 17,251 | 2,003,001 | 1,001 |
  | 2,000 | 34,501 | 8,006,001 | 2,001 |

  The parent is the demonstration commit `08573e159` for D1 (the events
  lane's `min_ticks` heap defect), which changes no `min_ticks` code, so
  its column is `main`'s implementation (verified). The two columns come
  from hooks at different call sites (the branch's record push and pop;
  the parent's contribution store), so they count the same quantity
  through different code. The answers are identical in both
  runs (verified: `answer_bits` and `answer_low` agree in every line both
  logs print in full; the branch log truncates one line).
- *Why the committed instruments miss it.* suanpan's touch meter counts
  accumulator digit work only (`crates/suanpan/src/accumulator.rs:42-49`,
  verified), the scan meter counts bits read from encoded streams, and the
  `min_ticks` modules call neither meter (verified by grep). Big-integer
  arithmetic there is invisible to the board. That the board read every
  touch and scan identical is reported (question 87).

**The smallest change.** Add the reviewer's two comb shapes
(cross-prefix and same-prefix) as `min_ticks` families to the fuel ladder
that #52 adds to the fuzz-fit harness (`zero_range_ladder`): four sizes,
each cell pinned within the ladder's ±2% band. Wasm fuel counts every
instruction, so the ladder sees `num-bigint` work with no hook at any call
site, and it covers `main`'s own `num-bigint` use in `min_ticks`'s heights
along the way. A limb counter charged at each call site, as the probe did,
is the alternative; it misses any call site nobody hooks.

**Cost.** Eight more ladder cells, each measured in a fresh guest on
inputs under 100 KB that are linear at `main`. I did not time them;
#52's ladder of 72 cells runs inside the fuzz-fit leg today (reported).

**Dependencies.** Question 87: options 2 and 3 build this instrument. If
you choose option 1 (stop the `min_ticks` work), the one constructed
failure is on an abandoned design, and the candidate moves to section 2.
It also needs #52 (ready), on which #86 and #89 stack.

### 1.2 A 32-bit compare pin whose input reaches past bit `2^32`

**The failure.** Two narrowings of a read cursor's position on 32-bit
targets pass every wasm32 pin, including the one whose doc says it
catches them.

- *Where it was constructed.* The adequacy lane's calibration
  (`lanes/l8-adequacy/round-1/findings/wasm32-pins-calibration.md`,
  reported) injected `(position as usize) / 8` for `(position / 8) as
  usize` at `bits/reader.rs:148`, and the same narrowing at
  `bits/reader/gamma/window.rs:28`. All 8 pins passed both.
- *What the pin claims.* `version_compare_crosses_the_usize_position_boundary`
  says (verified, `crates/before/wasm32-pins/harness/tests/pins.rs:72-73`):
  "Narrowing a cursor position before dividing it into a byte index would
  wrap the second input and misorder it against the smaller version."
- *Why it cannot catch them* (reported, the lane's trap probes): the large
  operand has exactly `2^32` live bits, so every live bit's position is
  below `2^32`; only the end position is not.
- *Still missed.* No ready branch changes this pin's input or doc
  (verified: the diffs of #29, #31, #54, #58, #63, and #85 against `main`).
  No other instrument runs on a 32-bit target.

**The smallest change.** Grow the existing `Check::VersionCompare`
synthesis so a decisive bit lies at or past `2^32`, with different data at
the wrapped offset: a stream of at least `2^29 + 2` bytes. Extending the
existing check adds no protocol variant, so it does not collide with the
renumbering that #28, #31, #48, and #50 need. If the guest cannot afford
the allocation, correct the doc to what the pin does guard (a `2^32`
*length* on the comparison path), and record that cursor positions are
unpinned.

**Cost.** The pin already takes 37 to 196 seconds across the audit's
landing logs (verified), and a grown input is about the same size. At
`main` the wasm32-pins workspace has no nextest time limit; #57 gives it
20 minutes (verified).

**Dependencies.** None. Land after #58 if a variant is added after all.

### 1.3 Run the worst-case ranking pin even when acceptance fails

**The failure.** On any commit whose board acceptance fails, ranking
drift goes unreported, and it appears later, attributed to the next
commit whose acceptance passes.

- *Mechanism* (verified). The gate's `_gate-board` recipe is
  `amp-board-acceptance worst-cases-pin`, and the landing check runs `just
  amp-board-acceptance worst-cases-pin`
  (`coordinator/audit-check.new2`). `just` stops at the first failing
  recipe, so a failing acceptance run skips the pin.
- *Where it was observed* (reported, `demonstrator-min-ticks-heap/NOTES.md`,
  round 2). The D1 demonstration's tree had one failing cell by design. Run on
  its own, the pin found 64 drift lines: the 2 expected `version_min_ticks`
  heap flips, 48 touch flips on 24 other operations, 12 tie changes, and
  the 2 known `count_display` lines. The in-leg run would have shown only
  that cell; the demonstrator saw the rest only because the coordinator
  told it to run the two commands separately. Every fix branch's
  failing-test commit fails the board by design, so this recurs.

**The smallest change.** Give `_gate-board` a body that runs both recipes
whatever the first returns and fails if either failed; make the same
change in the landing check's board leg. The holistic review's
`board-ops-render-19` proposed a larger form, folding the pin into the
acceptance run so the grid is swept once; it would fix this too.

**Cost.** Nothing when acceptance passes, since both already run. When it
fails, one more sweep of the grid, which I did not time.

**Dependencies.** None.

### 1.4 A committed check that each board ceiling equals its stated rule

**The failure.** Board ceilings drift from the derivations their docs
state, and a 49% rise in a release-profile heap reading passed
unreported.

- *At `main`* (verified, `crates/before/src/testing/meter/board/ceilings.rs`):
  - `COMB_SCATTER_PROJECTION_HEAP_BYTES_PER_IO_BYTE = 3.0` says both
    projection spellings "measure a flat 2.0 B/B" (lines 115-119), and
    still names the "split skyline builder" that `63d01d903` deleted.
  - `QUERY_EVALUATION_HEAP_BYTES_PER_INPUT_BYTE = 152.0` says it is "the
    largest release-profile reading with 25% headroom, rounded up"
    (lines 150-152).
- *The readings* (reported, `reviewer-ceiling-rules/NOTES.md`, with
  logs). COMB reads 2.982 B/B, first at `63d01d903` by bisection; the
  reviewer traced it to an allocator doubling step, not a worse worst
  case. QUERY's deciding reading is 86.59, which gives 109 by the stated
  rule. FOLD's rule gives 16 against 17.
- *Why nothing caught it.* No instrument compares a ceiling with its
  rule, and a reading below its ceiling can move freely. QUERY's ceiling
  sits 39% above its own rule, so its deciding reading could rise 75%
  before failing, against 25% under the rule.

**The smallest change.** As question 65's recommendation describes: each
ceiling carries its rule's inputs as data beside the constant, and
`run_acceptance` recomputes the rule from the readings it already holds
and fails on a mismatch in either direction. The ox-east-1 box is the
target of record, and `TARGET_DEPENDENT` covers cells whose readings
differ by host.

**Cost.** No extra sweep; arithmetic over readings the acceptance run
already holds.

**Dependencies.** Question 65 decides the rule. #53 and slot 26's
`audit/board-ceiling-rules` restate the rules this check would enforce.

### 1.5 `bounded_corpus_manifest_snapshot` times out under load

**The failure.** Two passing `rumors` tests fail the gate when the box is
busy; the first also fails the landing check's snapshot leg.

- *Where it was observed* (verified, by searching every log in the
  session scratchpad). `rumors
  tree::mirror::streaming::remote::codec::tests::bounded_corpus_manifest_snapshot`
  timed out at 180.0 to 181.5 seconds in at least 11 distinct runs: three
  of the coordinator's rechecks (`coordinator/recheck/rumors-slot-02.log`,
  `recheck/void/rumors.log`, `recheck/void/rumors-slot-09.log`), three of
  `reviewer-deep-surfaces`' runs, two of `reviewer-touch-bound`'s (one a
  rerun of the timed-out tests), and one each of `builder-rank-sum-pin`,
  `builder-read-high-part`, and `fixer-zero-shift`. Passing runs took 65.8
  to 169.7 seconds.
- *The second test* (verified, same search). `rumors::dispute_wire
  table_corpus_has_similar_protocol_overhead` timed out in about 11
  distinct full-gate runs across `reviewer-deep-surfaces`,
  `builder-rank-sum-pin`, `fixer-reserve-digits`,
  `builder-trap-diagnosis`, `builder-read-high-part`, and
  `builder-one-sweep`, including two reruns of the test alone
  (`builder-trap-diagnosis/dispute-isolated.log`,
  `builder-one-sweep/rerun-timeout.log`). It passed in 92.1 to 156.5
  seconds. The landing check's snapshot filter does not run it.
- *Why.* The root `.config/nextest.toml` terminates any test at 180
  seconds, and these tests need a third to most of that budget even when
  they pass.

**The smallest change.** A per-test override in `.config/nextest.toml`
that gives these two tests a longer `terminate-after`, keeping the global
limit for everything else. Making them cheaper is a `rumors` change,
outside the audit's scope.

**Cost.** None.

**Dependencies.** Your ruling, since both tests belong to `rumors`.
Section 5 notes the related false claim in that config file.

### 1.6 `PeakAlloc` heap checks free of libtest's main thread

**The failure.** A passing heap check fails at random under load.

- *Where it was observed* (verified). `resident_space_comparison_is_scoped_by_shape`
  (`crates/before/tests/representation_space.rs:56`) failed with "the
  measured value must release every allocation it owns", `left: 4729`,
  `right: 4609`, on two branches:
  `builder-join-all-multiplicity/gate-1-legs/workspace.log` and
  `builder-rank-decode-reader/landing1-logs/tests.log`. The adequacy lane
  counted five other recorded runs that pass (reported).
- *Mechanism* (reported, `lanes/l8-adequacy/round-2/findings/coordinator-leads.md`,
  item 1, from libtest 1.97.1's source). nextest runs one test per
  process, but libtest still spawns a thread for it. Its main thread makes
  two first insertions into empty collections right after the spawn; when
  it is preempted there, those allocations land inside the measurement
  window of a process-wide counter.
- *Same exposure* (verified that each installs `PeakAlloc` as the global
  allocator): `crates/before/tests/meter.rs` and
  `crates/before/tests/amp_board_smoke.rs`, whose
  `retaining_results_does_not_allocate_measured_heap` expects a heap
  reading of exactly zero (reported).

**The smallest change.** Run libtest single-threaded for these binaries
(`RUST_TEST_THREADS=1`), set where every nextest invocation inherits it.
That costs nothing, since each process runs one test anyway. Per-thread
attribution would be more robust but needs a maintained counting
allocator under the no-`unsafe` rule, and none has been vetted.

The lane proposes, as the known-bad check, a background thread
allocating during a measurement, which must still read exact. That
calibration fits only per-thread attribution; with `RUST_TEST_THREADS=1`
it fails by design. For the cheap fix, the meaningful check is that each
of the three binaries asserts the variable is set, because the race
itself is too rare to reproduce on demand.

**Cost.** None.

**Dependencies.** None.

### 1.7 An exact comparison of two board runs

**The failure.** Twice, a fix stopped on fitted-exponent rises that were
changes in fixed allocation, not growth, and each stop sent the fix
back for another round of probes (reported, the fixers' notes).

- *Where it was observed.*
  - The mask variant of W, suanpan's zero-range fix (#86): the fixer's
    comparison counted 334 cells whose rendered heap exponent changed,
    314 of them rises of 0.01 to 0.03 (verified,
    `fixer-zero-range-bitset/board-diff-mask.txt`). Exact byte
    probes then showed smaller fixed costs and constant 112- or 224-byte
    steps (reported, the same notes; you ruled the rises acceptable).
  - The `min_ticks` records design: the fixer stopped on rises such as
    `reveal-hifloor` 0.19 to 0.28 and `tooth-tail` 0.80 to 0.91
    (verified, `fixer-min-ticks-heap/NOTES.md`). The coordinator
    attributes these to fixed-part changes too (reported).
- *Why it happens* (reported, `STATE.md`). With heap ≈ `a + b·n`,
  shrinking `a` raises the two-point log-log slope toward 1 without any
  asymptotic change. Every comparison is an agent's own script over the
  board's rendered text, which rounds exponents to 0.01 and per-byte
  readings to 0.1 (verified: `board-diff-mask.txt` compares strings such
  as `('0.99', 1.1)`).
- *Why nothing committed helps.* The board has no comparison mode
  (verified, `crates/before/examples/amp_board.rs`: render, `acceptance`,
  `worst-cases`, `worst-cases-check`).

**The smallest change.** A board mode that writes each cell's exact
readings at every ladder size to a file, and a compare mode that reads
two such files and prints, per cell and currency, the difference at each
size, classified as unchanged, a constant shift, or growing. The shard
processes already send exact samples to the parent (verified, child mode
in `amp_board.rs`), so this needs no new metering.

**Cost.** A developer tool; no gate time.

**Dependencies.** None.

### 1.8 A board row for neutral and hole-free `Query::coverage`

**The failure.** #61 changes `Query::coverage`'s heap for bounded queries
without holes, for neutral queries, and for polar queries whose holes
need no clamp, and no committed instrument registers the change.

- *Where it was observed* (reported, #61's ready entry and its
  reviewer). Across the board's 15,933 cells, every heap reading is
  identical between #61's parent and tip, because the board's coverage
  operations build only shapes whose clamp is still needed.
- *Consequence.* A change that reverted #61's saving, or any heap
  regression on those shapes, would pass every committed instrument, and
  the saving cannot be locked in by a ceiling as your rule for
  improvements requires.

**The smallest change.** One board operation variant over a bounded,
hole-free query (the coordinator's suggestion is
`after(v) & before(w)`), judged by the existing heap ceiling.

**Cost.** One more operation across the version families; not measured.

**Dependencies.** #61 (ready).

### 1.9 Cost-only survivors that every meter and the fuel bands miss

**The failure.** Three cost-only mutants pass every committed instrument.
The clearest makes every `PackedU64Stack::pop` decode its width one bit
at a time.

- *Where they were constructed.* The adequacy campaign's survivors
  (reported, the round-2 addendum's survivor index):
  - `bits/stack/packed_u64.rs:63`, `quick < 62` to `quick > 62`: every
    pop takes the bit-at-a-time loop. The stack holds the version
    writer's lengths, the party writer's positions, `range_minima`'s
    boundaries, and the tick pre-scan's suspended records (verified by
    grep).
  - `bits/reader/words.rs:78`, `read_word_opt` returning `None`, and
    `>=` to `<` in the same function: the reader's optional buffer
    top-up is skipped, or adds a zero word past the end.
- *Verified here (run 1).* I compiled each into the fuzz-fit guest
  through a compile-time switch that costs no instructions when off. The
  guest's hash changed for each variant, so each mutant was built in. All
  25 fuzz-fit tests pass under each of the three, the fuel-band
  enforcement included. The unmutated baseline also passes 25 of 25.
- *Why every counter misses them* (inferred from reading). The mutants
  do the same bit work through more calls, and the touch and scan meters
  count bits and digits, not calls, so no counting meter can see them.
  Fuel can, but the bands accept up to `10^0.2`, about 1.58 times, above
  each band's worst calibrated residual (`ENFORCE_MARGIN`,
  `crates/before/fuzzfit/harness/src/bands.rs:71`, verified), and these
  paths are a small share of any generated program's fuel.
- *The record's other two survivors are caught.* The two
  `bits/writer.rs:363` survivors, which make `splice_storage` copy whole
  bytes bit by bit, fail three fuel-band tests (verified, run 1, for the
  `>` to `==` form; the `>` to `<` form skips the same copies, inferred),
  for example: "ABOVE BAND (asymptotic regression): ff_clock_fork at 129 bits
  consumed 5889 fuel; the pinned law predicts ~10^3.211 +0.354/-0.304".

**The smallest change.** A cell in #52's ±2% fuel ladder for one
operation whose fuel these pops dominate. Nobody has constructed such a
family, so that construction comes first. The work is a constant factor
of at most 64 per pop, and the `words.rs` pair's extra cost is
unmeasured and may be negligible, so this ranks low under your ruling
that cost findings are asymptotic first.

**Cost.** Four ladder cells once the family exists.

**Dependencies.** #52.

### 1.10 `STOPPING_DIFF_BAND` in units of hops

**The failure, as the adequacy lane states it.** The band admitted a 13%
rise in the touch difference it guards, 7,867 to 8,892 at 1,024 hops,
from `fdd1bf47` (reported, the lane's bisection,
`coordinator-leads.md` item 2). The band is the record ±25%
(`crates/before/tests/meter/version_scaling/minimum_boundaries.rs:246`,
verified), which admits about 1.9 extra touches per hop.

**Where I dispute it.** The lane says the band "admitted a
one-touch-per-hop rise its message says it exists to catch". Its message
says it exists to catch "a per-hop read of the surviving boundary's
width" (verified, lines 313-317). The surviving boundary is wide, at
least three digits by the shape's description, so a width read costs at
least three touches per hop and exceeds the slack (inferred from the
test's prose; not measured). The rise that passed was one touch per hop
from a debug-only `clone().cmp_zero()` (verified in `fdd1bf47`'s diff),
which release builds never run, and #38 deletes it. So the band did what
its message promises; what it missed is a one-touch-per-hop rise of
debug-only work.

**The smallest change**, if you want such rises caught: a ceiling of the
record plus `k / 2`. **Cost:** none. **Dependencies:** land after #38,
whose deletion restores the 7,867 reading, then re-derive the record.

I rank it last because the failure is debug-only work. Strike it if you
don't want dev-profile meters to flag work that release builds never do.

---

## 2. Listed candidates

Each entry says what the candidate reaches or checks and why no failure is
known. Grouped by source.

### 2.1 The explore-branch instruments

Every cited commit exists on its branch and holds the cited paths
(verified). Four branches have moved past the commit `instruments.md`
cites: `explore/l1-identity` to `346a82ac9`, `explore/l2-algebra` to
`6748bcd41`, `explore/l7-suanpan` to `5d5e33471`, and
`explore/l8-adequacy` to `0acb87bdc`. The later commits add drafts,
records, round-3 probes, and a reusable census (verified, `git log`).

| Lane | Instrument | What it reaches or checks | Why no failure is known |
|---|---|---|---|
| Identity (L1) | Interval-set model, party generators, histories, overlay, deep probe (`crates/before/tests/audit_l1/`) | A model sharing no code with production or the tree oracle; parties to depth about 120 and 128 bytes, against depth 4 and 4 bytes committed (reported) | Its three unique catches are covered. M19 and M20 fail under #40 and #81 (reported, their ready entries); M26 was already caught at the base by `amp_board_smoke` (reported, #75's reviewer, across all 12 test binaries). |
| Algebra (L2) | Leaf-list model with tape generators (`crates/before/tests/l2_probe/`) | Random topology to depth 300; heights on the 63-bit boundaries; correlated pairs forcing collapse cascades (reported) | It caught none of 18 algebra survivors and 2 tie-order mutants (reported, `lanes/l2-algebra/round-2/report.md`); the committed suite caught all 20 of its own mutants. |
| Events (L3) | Co-generated tick pairs (`l3_probe.rs`) | Nested lookahead sites, wide and multiple pre-scans | Built: #74 catches M6, M8, M15, and M21 (reported). M19 was caught in #74's first round by an out-of-bounds panic and not re-run after round 2 changed the strategies' palette (reported, `builder-events-cogen/NOTES.md`); #74's own index paragraph names it. |
| Events (L3) | `min_ticks` floor and tightness over histories (machinery brief MB-2) | Tests the definition of `min_ticks` against histories instead of against another computation | Its calibration shifted production alone by ±1, and the committed differentials kill those shifts too (reported: they are not among the events lane's escapes). The class it exists for, one definition wrong in production and both oracles at once, was never constructed. `STATE.md` still queues it; the rule cuts it. |
| Events (L3) | Generator-reach, counter-search, and heap-search diagnostics | Histograms and maxima only | Diagnostics; they always pass. The heap search peaked at 43% of the board's ceiling (reported). |
| Measures (L4) | Integrator and `Rank` harnesses (`explore_l4.rs`, `explore_l4_rank.rs`) | 70,000 integrator cases plus spines to depth 3,000 and heights to 6,000 bits; three freeze schedules | The committed suite caught all 12 integrator and 6 `Rank` mutants (reported). Its two wasm32 instruments were built as #29 and #31. |
| Spans (L5) | Grid model, shaped and organic generators, exhaustive 4-cell cube (`audit_l5.rs`) | `Before` and concurrent verdicts at 9.6 to 12.6% against 1.5 to 2.4% committed; depth to 64 (reported) | The committed suite caught all 17 non-equivalent mutants (reported). Its doc defects went to the docs branch. |
| Codecs (L6) | Specification codec and probes (`l6_spec.rs`, `l6_probes.rs`, `tests/l6_resource.rs`) | All six wire types, every entry point, 300,000 cases per type, error class checked for applicability (reported) | The committed suite caught all 17 mutations (reported). It enforces only span's documented error precedence and checks first-detected order informationally, so it does not catch the two padding-precedence survivors (section 2.3). Its 32-bit growth probes backed a defect your ruling dissolved (no input-size promise, #53). |
| Suanpan (L7) | Pool model over three accumulators (`explore_l7.rs`) | Arbitrary operand histories, extreme digits meeting `normalize` and shifts, swarm-weighted mixes | Its one unique catch, survivor `normalize.rs:74`, fails #66's exhaustive test (reported, #66's ready entry). |
| Suanpan (L7) | Hill-climbing touch adversary (`#[ignore]`) | Searches programs for touches per unit of work | Best ratio 9.6 over 12 seeds, no width dependence (reported). Its property form was built as #37. |
| Suanpan (L7) | Wasm fuel instrument and builders | Fuel on sparse families; 32-bit landings | Built as #52's ladder and #54's pins. |
| Adequacy (L8) | Generator census (`l8_census.rs`), `usize` signature census, survivor export, window witness | Histograms of what the committed generators draw; public signatures mentioning `usize` | Measurement, not checks. The window witness kills three survivors that no production caller can reach (section 2.3); it is already in `follow-ups.md`. |

### 2.2 The coordinator's collected candidates without a failure

- **Remnant insertion in the zero-range map.** No fuel-ladder family splits
  a zero range, because every zero run is one digit (reported, #52). W
  (#86) deletes the map, so nothing remains to cover.
- **The touch-bound property's read-only operations.**
  `signed_magnitude`, `scaled_signed_magnitude`, and `TryFrom` are outside
  #37's generator (reported). #27 and #64 pin the readout's touches on
  constructed rows, and the follow-up list records that its reviewer
  found no failure a random-history version would catch.
- **A wasm32-versus-native differential runner over suanpan traces.** I
  found no record naming a failure. suanpan's four 32-bit defects (#28,
  #48, #50, #54) all need widths past `2^32` bits, which random small
  traces cannot reach.
- **A late-first-split COMB family.** The coordinator infers up to about
  4 B/B, above COMB's ceiling of 3. If built and confirmed, that is a
  defect finding about the code, not an instrument; it is cheap to
  construct, and you may want it constructed before question 65 sets
  COMB's rule.
- **nextest LEAK flags.** They appear in four landing logs (verified) and
  never fail a test. The adequacy lane's mechanism, sibling processes
  holding pipe ends between spawn and exec, is reported. A
  `leak-timeout` setting would quiet the output.
- **Calibrating the board against the cost-only survivors.** No
  cost-only survivor is known to pass the board, because none has been
  run against it. The lane's arithmetic puts the quadratic fold mutant at
  about 134 scan bits per unit against a ceiling of 17 (reported, not
  run). The five survivors no counting meter sees are settled in 1.9.
- **Carry-ripple families and fragmented masks**, named in
  `instruments.md`'s cost area. I found no record of a failure either
  catches; the suanpan lane's touch adversary found no width-dependent
  cost (reported).
- **Rustdoc builds for the detached workspaces.** No gate or
  landing-check leg documents wasm32-pins or surfacecheck (verified). Run
  2 built both, public and private, with warnings denied: all four pass
  without a warning (verified). rustdoc never renders integration tests,
  so such a leg would not check the pins' doc comments in
  `harness/tests/pins.rs`, the ones 1.2 finds overclaiming.
- **`RED_ZONE`'s rule.** `STRIDE * max_frame_bytes < RED_ZONE` is stated
  in `recurse.rs`'s doc and checked by nothing (verified), but the guard
  serves test helpers only, and no stack overflow has been observed.
- **Termination outside nextest, and fuel budgets per wasm32 check.**
  Doctests, the board and surface `cargo run` steps, and
  `fuelscape-verify` have no time limit (verified, justfile). No hang has
  been observed in any of them; #57 bounds the detached nextest runs.
- **Fuzz-fit band drift.** A fresh calibration differs from the committed
  bands in 145 lines (reported, `builder-one-sweep` and the
  rank-alignment reviewer). The refit was taken at `d5e80103`, an
  ancestor of `main` that differs from it in two commits under the
  measured crates. I compared the two files: the refit's band
  ceilings at each band's largest calibrated size sit higher than the
  committed ones by up to 0.066 in `log₁₀` fuel (`ff_version_min_ticks`,
  about 16%), and by more than 0.01 for 8 of 50 bands, all within the
  enforcement margin of 0.2 (verified, `bands_drift.py`). The cost rose a
  few percent since the last re-pin without anyone recording it, but
  nothing failed or passed falsely.
- **`harmonic` understating the rank fold's worst touch case.**
  `jump-rising-spine` beats it by 1.28 to 1.32 times (reported), and its
  commit `08573e159` re-pins the four rank-fold touch rows (verified). No
  ceiling is breached by either family, and no regression is known that
  the weaker family would hide. The family arrives with the D1
  demonstration, so it lands or not with question 87.
- **A rule that every representation threshold be reachable at board
  sizes.** The one failure, a 23-bit prefix field past which D1 recurred
  while the board passed, was on a design you rejected; your ruling
  for `min_ticks` already requires "no field has a capacity the board
  cannot reach". No unreached threshold is known at `main`; finding one
  would take an enumeration nobody has done.
- **Pinning `usize` in public signatures in `surfacecheck`.** No public
  parameter or stored quantity in `before` is a `usize` outside standard
  conversions and iterator hints (reported, the adequacy lane's census).
  suanpan, where `reserve_digits(usize)` was such a parameter (#28), has
  no surface check at all.
- **Measuring cost on a 32-bit target.** The 32-bit growth defects were
  dissolved by your ruling that neither crate promises an input size.
- **Stack safety at `opt-level = 0`.** You declined it (question 68,
  identity lane MB3).
- **The wasm32 join pin's doc.** It names a writer-length wrap that the
  writer's design makes value-neutral on that path (reported, the
  adequacy lane's calibration). A doc correction, not an instrument;
  #63 already rewrites the rank-route pin's doc.
- **Generator-reach floors, a known-bad demonstration for every
  instrument, and the survivor list as a baseline.** No failure is named
  for any. Known-bad demonstrations appear in section 1 wherever a failure
  exists.
- **The predicates: history independence and order independence of
  folds.** The algebra lane showed tie order changes no verdict in all
  four `CursorSet` walks (reported); no failure is known.
- **`arb_party_family`'s skew.** Multi-input `join_all` never succeeds
  in the arbitrary families (reported); already dropped under your
  caution.

### 2.3 Mutation survivor classes no instrument covers

From the campaign's final index
(`lanes/l8-adequacy/round-2/addendum-bits-party/survivors/INDEX.md`; 282
survivors of 3,919 mutants, reported):

- **G, two of 21 uncovered by decision.** `span/wire.rs:136` and
  `party/io.rs:29` change only which `Decode` variant a doubly malformed
  input returns. They stay unbuilt unless you document a precedence
  (#67's note). The other G survivors are covered by #66, #78, and #82
  (reported).
- **T, 41, by decision.** `Debug` text, delegating operator forms, and
  advisory `size_hint`s. #83 covers `Debug for Count`; #43 deletes the
  fork-iterator pair.
- **U, 41.** Unreachable today. The three `gamma/window.rs:47` survivors
  change values inside the window's general contract, which no
  production caller reaches; `follow-ups.md` holds their ready-made
  regression test.
- **C, 96, cost only.** The board is inferred to see most of them
  (above); 1.9 covers the three that nothing sees. `positions.rs:46`
  (every adjacent tag push stores a record) is marked "board heap at
  depth, unverified" and remains so.
- **E (69), I (10), and the one undecided survivor**, which #67 deletes.

---

## 3. The verification map

What the validation index
(`crates/before/src/testing/validation_index.rs`) and the crates'
`# Testing` sections leave out at `main`. I list them and write none.

### The validation index

Two gaps the brief names are closed by ready branches (verified from
their diffs):

- *The 32-bit boundary pins.* #31 adds an entry, with a triage line for
  an abort that carries no panic message.
- *The zero-range fuel ladder.* #52 adds a paragraph beside the fuzz-fit
  bands; #86 and #89 carry it.
- #74 adds the co-generated tick pairs.

Still missing after every ready branch (verified by searching the index
for each):

1. **The worst-case ranking pin** (`worst-cases-pin` against
   `WORST_RANKINGS`). The index's board paragraph never mentions it, and
   it is the only instrument that turns a ranking flip into a failure.
2. **The generator census floors** (`testing/generators/tests.rs`), which
   fail when a generator stops producing a shape class.
3. **The stack-safety tests at depth**: the `recurse` guard,
   `clock/tests.rs`'s `deep_tree_*_stack_safety` tests, and those #34
   and #75 add.
4. **The heap checks outside the board** that read `PeakAlloc` directly:
   `tests/representation_space.rs` and the heap pins in `tests/meter.rs`
   and `tests/amp_board_smoke.rs`. The focused-checks paragraph names
   `tests/meter.rs` for its axes, not for its heap readings, and says
   nothing of the libtest race in 1.6.
5. **The compile-time trait assertions** (`testing/auto_traits.rs`) and
   **the fuelscape islands test** (`testing/fuelscape_islands.rs`).
6. **Every suanpan instrument.** The index covers only `before`, and
   suanpan has no index of its own: the touch meter and its metered
   tests, the differential, surface-model, representation, primitive, and
   witness suites under `accumulator/tests/`,
   `tests/amortized_sequences.rs`, #37's touch-bound property, and #66's
   exhaustive normalize test.
7. **A false pointer.** The index's last section says the shared
   scaffolding (generators, the oracle bridge, the RNG, the op-trace
   driver) "is indexed in [`super`]'s module doc". At `main`,
   `testing.rs`'s module doc is five lines and indexes none of it.

### The `# Testing` sections

- **`before`** has a `## Testing` subsection in its crate docs
  (`crates/before/src/lib.rs:408-413`, verified): one paragraph naming the
  semantic instruments. It omits the resource instruments (the board, the
  focused meters, the fuel bands, the ranking pin), the 32-bit pins, and
  the stack-safety tests, and it does not invite contributions to gaps.
- **suanpan** has no testing section (verified: its crate docs have
  `# Optional metering` only).

### Documentation builds of the detached workspaces

No gate or landing-check leg runs rustdoc in the in-scope detached
workspaces, wasm32-pins and surfacecheck (verified, the justfile and the
landing check). Both document cleanly today, public and private, with
warnings denied (verified, run 2), so the gap has no failure and is listed
in section 2.2.

## 4. Where the rule cut `instruments.md`'s scope

- **Sequence, step 1.** The older design runs every survivor against every
  explore instrument and the census on every generator. Both are cut, as
  the brief and notice 88 say. I made one narrow exception: run 1 ran
  five existing survivors against one committed instrument, the fuel
  bands, because a candidate's premise ("only wall time sees them")
  depended on it. I wrote no new mutant: each variant is one of the
  campaign's own diffs behind a compile-time switch.
- **Generator coverage and oracles.** Every explore-branch generator and
  independent model is listed, not proposed: none catches a failure the
  committed suite plus the ready branches misses (section 2.1).
- **Predicates.** Multiplicity was built (#40, #81). History independence
  and order independence of folds are listed.
- **Cost instruments.** The sparse-height family and wasm fuel were built
  (#52), and the hill-climbing adversary became #37's property. Carry
  ripples and fragmented masks are listed. The `num-bigint` meter (1.1)
  is the one new cost instrument with a failure behind it.
- **Target dependence.** The compare pin's reach (1.2) is eligible. Telling
  a panic from an abort is built (#31). Measuring 32-bit cost is cut by
  your input-size ruling, `opt-level = 0` by your answer to question 68,
  and the `usize` signature pin for lack of a failure.
- **Checks on the evidence itself.** Generator-reach floors, a known-bad
  demonstration for every instrument, and the survivor list as a
  baseline are all cut to the cases with a failure: the `PeakAlloc`
  liveness assertion (1.6) and the ceiling rule check (1.4).
- **Outside `instruments.md`: the dispatch queue.** Item 1 of `STATE.md`'s
  queue (the `min_ticks` histories, MB-2) is cut by the same rule, since
  its failure class was never constructed. The coordinator may want to
  retire that entry.

## 5. What I found wrong in the brief and the records

1. **"Five C-class survivors invisible to every counting meter ... only
   wall time sees them"** (the brief, `STATE.md`, and the addendum's C
   table). Wasm fuel is a deterministic counter too, and the gate's fuel
   bands catch both `writer.rs:363` survivors (verified, run 1). Three
   survivors remain invisible, and only they are in 1.9.
2. **The `STOPPING_DIFF_BAND` lead.** The band did not miss what its
   message says it catches; it missed a one-touch-per-hop rise of
   debug-only work (1.10).
3. **The `PeakAlloc` calibration.** The proposed background-thread
   calibration fits only the per-thread fix, not the cheap
   `RUST_TEST_THREADS=1` one (1.6).
4. **The snapshot timeout's frequency.** `STATE.md` calls
   `bounded_corpus_manifest_snapshot` the snapshot leg's main
   false-failure source; the logs hold at least 11 timed-out runs of it,
   against two `PeakAlloc` failures, and a second `rumors` test times out
   as often in full-gate runs (1.5).
5. **The root `.config/nextest.toml`** says "The slowest honest tests
   today are the proptest suites, all finishing well under 60 seconds".
   Two `rumors` tests contradict it: `bounded_corpus_manifest_snapshot`
   (65.8 to 169.7 seconds when it passes) and `rumors::dispute_wire
   table_corpus_has_similar_protocol_overhead` (92.1 to 156.5 seconds),
   and both time out at its 180-second limit (verified, 1.5). The
   sentence also uses "honest" and "today". The file is the workspace's,
   but the tests that falsify it are `rumors`', outside the audit's
   scope, so I leave the correction to you.
6. **The validation index's pointer** to a scaffolding index in
   `testing.rs`'s module doc that does not exist (section 3, item 7).
7. **#74's mutant list.** Its ready entry names M6, M8, M15, and M21; the
   events lane's list of escapes names M6, M8, M15, and M19. M19 was
   caught in #74's first round and not re-run after the second round's
   palette change (reported). Low risk, since the census floor holds the
   regime that triggers it, but unverified at the tip.
8. **The `harmonic` item has no record of its own.** I reconstructed it
   from the D1 demonstrator's notes and the re-pins in `08573e159`
   (section 2.2).
9. **The detached-workspace docs item** names only wasm32-pins.
   surfacecheck has the same gap. Both build clean today (run 2), and
   a doc leg would not reach the pins' own doc comments, which live in
   an integration test rustdoc never renders.
10. **The representation-threshold item** cites a failure on a design you
    already rejected, with the principle already in your ruling.
11. **`instruments.md`'s inventory commits are stale** for four lanes
    (section 2.1); nothing depends on it, but the triage should read the
    tips.
