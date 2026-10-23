<!-- CAVEAT LECTOR: written by Claude (Opus 5.5) as the instrument evaluator (question 149), from the rescue catalogue, the explore branches and preserved probes, today's `main` (`8d95c51c2`, read in slot 64), and the pending branches; no tests were run; for Finch's review. -->

# Which rescued instruments to fold in

The rescue catalogue (`instrument-rescue.md`) ranks 159 already-built
instruments, largely on evidence of the form "this instrument fails under
mutant X". You have ruled that evidence uninformative, so this report judges
each instrument afresh, by what it checks and what that is worth against its
cost. Every row gets a verdict; the appendix lists all 159 in one table.

Two terms recur. To *fold in* an instrument is to port it into the committed
suite's shared modules (`testing::generators`, `testing::oracles`,
`testing::laws`, the board), as opposed to keeping it as a parallel harness.
The *fuel board* is the pending branch `audit/board-fuel` (#136), which adds
wasm fuel as the board's time currency, with one heap and one fuel ceiling per
cell.

Claims are marked *verified* (I read it in today's tree or a branch) or
*reported* (taken from the catalogue or another record).

## 1. How I judged

### 1.1 The axes

I used six axes. Each answers a question you would ask of a new test, and none
rests on a mutant.

1. **Contract.** Does the instrument state a promise the code makes, or a law
   that follows from one, which nothing committed states? Or does it check a
   stated promise in a regime nothing committed reaches? This is the
   doctrine's "tests are written because the code's promises call for them".
   Where a contract is the deciding question, I read the rustdoc rather than
   the catalogue.
2. **Reach.** Which part of the input space does it sample that the
   committed generators never or almost never produce? The baseline's list
   of unreached regimes still holds, because the shared generators changed
   only by #74's co-generator and #34's right spine; arbitrary values still
   stop at depth 4 (verified, `ARB_DEPTH = 4` at `testing/generators.rs:281`).
3. **Independence.** What does its judgment share with production and with
   the committed oracles? An oracle that shares a representation or a writer
   with the code shares its failures.
4. **Cost.** Lines to port, the work of fitting the shared modules, runtime
   on every run of `just gate`, and any maintenance it creates (calibration
   tables, numbers held in the tree, special wiring).
5. **Durability.** Is it coupled to internals that legitimately change, or to
   meters that are retiring? Does the code it targets still exist?
6. **Redundancy.** Did landed work already carry it, or does another rescued
   instrument do the same job better?

These suit this codebase because its correctness story rests on stated
contracts (rustdoc with `# Errors`, complexity, and space clauses), on laws
from the interval-tree-clock algebra, and on three oracle families with known
blind spots (the tree oracle shares production's recursion; the
function-space oracle cannot go past a few levels of depth; production checked
against itself catches only what a law states). An instrument earns its place
by filling one of those blind spots for a stated promise, at a cost that does
not crowd the suite.

### 1.2 Cost instruments, under the two-axis ruling

The board ends with heap measured natively and time as wasm fuel; touch and
scan retire. So an instrument built on touch or scan is worth what its
*input family* is worth on the fuel board. The meter is replaceable; a
white-box worst case for an operation is not. On the fuel board, a new family
gets a cell per applicable row in both ceiling tables, each bounded at 10%
over its own worst reading, plus the growth-exponent check, and one recipe
recalibrates them (verified, `cell_ceilings.rs`'s module doc on
`audit/board-fuel`). So a family needs no hand-derived constant. Its cost is
one recalibration on two hosts, table diffs to review, and board runtime.
Where re-expressing a family in fuel or heap makes sense, I say so; random
searches that only print (03.9, 04.12) do not re-express, because the board's
value is in constructed worst cases, not in random samples well under a
ceiling.

### 1.3 Mutant evidence, and one tension in the doctrine

I set aside every claim of the form "kills mutant X", for and against. I did
use failure *classes* in one place: to justify a new oracle mechanism, as
Principle 3 asks ("name a concrete, constructible failure it catches that
existing instruments miss"). Principle 6 says not to justify an individual
test by an imagined bug. I read these as applying at different scales: a new
mechanism must name the space it samples, which is a class of failures
existing oracles cannot see; an individual test needs only the promise it
states. If you meant Principle 3's rule to be phrased without failures too,
it could say "name the region of input space, or the code it is independent
of, that existing instruments cannot judge". That would cover the same cases
without inviting imagined bugs.

### 1.4 What changed under the catalogue

The catalogue describes `main` at `ce67ab083`. These changes alter verdicts
(all verified in `git log --first-parent` and the tree unless marked):

| Change | Effect on the catalogue |
|---|---|
| The `min_ticks` heap fix landed (`5d1d8aa27`, "records over merged references"), with the board families `jump-rising-spine`, `wide-step-pairs`, and `wide-step-spine`, and focused heap meters at two sizes per family | 03.5 and 09.A8 are carried. Question 87's framing is obsolete; 09.A4, 09.A15, and 03.7 no longer hold an open defect. |
| #83 landed squashed into one law commit (`dc2fe0d93`); `d653bb83` is not an ancestor of `main` | 09.A2 can no longer be restored by reverting `735b6b2e`; it is a hand port from `archive/rescue-d653bb83`. |
| #81 landed with a strengthened agreement property (`laws/tests/conservation.rs`) | 09.A1 still adds an exhaustive form beside it. |
| #74, #78, #43, #106, #110, #75, #34, #76, #61, #94, #51 landed | Several entries are carried or narrowed (see each). |
| #52 (the zero-range fuel ladder), #27, #64, #66 were dropped | 07.2 and 09.A9 lose their fold-in target; 07.3's and 09.A16's targets are gone. |
| #122 reworked suanpan; #86 replaced the zero-range map with a written-position set | 07.1 needs real porting; every zero-range mutant and invariant is gone. |
| #37 (touch-bound property) is pending, and the coordinator expects to propose dropping it (reported, `STATE.md`) | 07.1 should not ride #37's generator; 09.A3 and 07.4 go with #37. |
| The owner scheduled a fuel-board family driving `Sum` through a shift and through writes below the run (reported, `STATE.md`) | 04.2 is covered by that family. |
| The stack guard (`descend!`, `recurse.rs`) is still present (verified, seven files) | Recursive test helpers in fold-ins still route through `descend!` or iterate. |

## 2. Recommendations

Five strengths, from strongest:

- **2.1 Rescue now:** small, standalone, contract-called, cheap.
- **2.2 Rescue as a group:** larger fold-ins whose parts share a generator
  or oracle and should land together.
- **2.3 Rescue only with another change.**
- **2.4 Already carried.**
- **2.5 Decline.**

### 2.1 Rescue now

Each of these can be one commit on one branch. Together they add roughly 700
lines and about 12 seconds per run, most of it the cube (inferred from the
reported runtimes).

**05.4, the exhaustive boolean cube.**
- *Coverage.* `Coverage` is exact by its own definition: `Full` means every
  covered version is admitted, `Empty` none, `Partial` some and not others
  (verified, `causally/query.rs:51-59`). The committed exact checks enumerate
  the two-party grid of nine versions on two cells (reported). The cube
  enumerates all 16 versions with four cells of height 0 or 1, every ordered
  segment, and every single clause and same-polarity clause pair: 921,456
  verdicts on a lattice whose widest antichain is six, against three
  (reported).
- *Oracle.* Brute force over the 16 members suffices, because the cube is
  closed under join and meet (reported, re-derived by the cataloguer), so it
  needs no census.
- *Cost.* About 106 lines; 7 to 13 seconds in three recorded runs
  (reported). It belongs beside `coverage_is_exact_on_the_two_party_grid`,
  and can draw its versions from `testing::exhaustive`'s depth-2 corpus.
- *Verdict.* Rescue. It is the "exhaust the finite domain" form `AGENTS.md`
  asks for, applied to a contract the code states. Agrees with the
  catalogue (position 1).

**05.3, the census validator, retargeted.**
- *Coverage.* Every exact-coverage check, committed or rescued, rests on an
  argument: the enumerated set contains a witness for every verdict. #61's
  reviewer found an off-grid version inside the grid's interval (reported),
  so that argument is not obvious. The validator lifts two-cell pools to four
  cells and brute-forces every function inside each segment.
- *Cost.* About 60 lines, 0.15 seconds (reported). Its doc wrongly claims it
  compares against the witness oracle (reported); correct that.
- *Verdict.* Rescue with 05.4, pointed first at the committed grid's `brute`
  and the cube's brute force, then at the census if group G lands. I rank it
  higher than the catalogue (42) because it guards the committed oracle,
  not only a rescued one.

**05.6, span-to-query coverage agreement.**
- *Coverage.* A law nothing states: a segment query's coverage of another span
  is `Full` when the segment contains it, `Empty` when they do not meet, and
  `Partial` otherwise; `Query::from(&v)` admits exactly `v`. The committed
  tie covers membership only (reported).
- *Cost.* One `VERSION_TRIPLE` law of about 40 lines over the existing
  `span_candidates`; under a second.
- *Verdict.* Rescue. Agrees (8).

**08.3, the gamma window at unaligned starts (also 09.A23).**
- *Coverage.* The committed window tests decode long codes only at
  position 0 (verified, `bits/tests.rs:604-679`). This test decodes 59- to
  63-bit codes at every start from 1 to 7, so each code's last bits come from
  the ninth loaded byte. The window is `pub(crate)`, and borsh's reader never
  hands it a fuller buffer today (reported). Under Principle 1 the finding is
  judged by the contract clause, so reachability does not lower its value.
- *Oracle.* The production writer, which the codec suites check against
  canonical bytes.
- *Cost.* 38 lines, 0.05 seconds (reported).
- *Verdict.* Rescue. Agrees (10). The adequacy cataloguer also noticed that
  `ReferenceBitsReader`'s doc overstates its independence from the window
  (reported); that is a prose item for the second docs pass.

**04.8, the `Count` boundary sweep.**
- *Coverage.* The committed conversion test judges `Count`'s `TryFrom` by
  `num-bigint`'s `try_from` on the same value (verified,
  `count/tests.rs:91-106`), which is the call the implementation makes
  (reported). `arb_magnitude` cannot draw `MAX − 1` or `MAX` of `u8`, `u16`,
  `u32`, or `u128` (reported). The sweep takes 0, 1, and `MAX − 1`, `MAX`,
  `MAX + 1` for six types, judged by an independent `BigUint` comparison.
- *Cost.* About 60 lines, 0.05 seconds.
- *Verdict.* Rescue. Keep the `usize` row: it duplicates `u64` only on
  64-bit hosts. Agrees (15).

**03.1, `min_ticks` attained by a constructed history.**
- *Coverage.* `min_ticks` is "the minimum number of ticks that could have
  produced this version" and "a floor over all causal histories" (verified,
  `version.rs:316-322`). A minimum is attained, so attainment is half the
  contract. The committed suite checks attainment only on a single region
  and three fixed values (reported). The construction forks a seed down each
  node's path, ticks that node's base, rejoins, and checks that the result
  equals the drawn version and that `min_ticks` equals the ticks spent.
- *Cost.* About 80 lines; 0.16 seconds on arbitrary versions and 1.6 on
  #74's spine versions (reported); #74's `arb_spine_case` is on `main`
  (verified, `generators/co_generated.rs`). `realize` recurses on version
  depth, so it goes through `descend!` while the guard exists.
- *Verdict.* Rescue. Agrees (9).

**03.2, `min_ticks` bounded by each clock's causal past, narrowed.**
- *Coverage.* The committed `min_ticks_floors_every_history` checks the final
  clocks against the whole history's total ticks (verified,
  `version/tests.rs:807-825`). The rustdoc's floor is per history that
  yields the version, which for a clock is its own causal past. A per-clock
  ledger states that exactly, after every step, and gives about 40 times as
  many tight observations per history on the committed generator (reported).
- *Narrowed form.* Change the committed test to carry the ledger over
  `world_strategy`. Leave out foreign ticks (decision 7). Wide counts and
  `absorb` cannot go through the tree oracle's trace applier (reported), so
  they wait for 01.3 if you take it.
- *Cost.* A small change to one test; 0.59 seconds (reported).
- *Verdict.* Rescue narrowed. Agrees (7).

**09.A1, exhaustive agreement for the laws' multiplicity comparison.**
- *Coverage.* Every multiplicity law's verdict rests on the laws'
  `same_multiplicity`, built from public `without` and `join`. The committed
  property checks it against the tree oracle's on sampled, constructed
  families (verified, `laws/tests/conservation.rs:93`). This test exhausts
  every family of at most three nonempty normal-form parties of depth at
  most 2, 3,616 families (reported).
- *Oracle.* The tree oracle's recursion, independent of the layer
  construction.
- *Cost.* 64 lines, 0.42 seconds (reported). It uses `both_verdicts` and
  `owner_layers`, which `main` has (verified). Restate its doc comment as the
  domain it exhausts; today it explains itself by the weaker comparison it
  would reject.
- *Verdict.* Rescue (decision 9). Agrees (2).

**09.A26, fork-plan tails at every depth to 140.**
- *Coverage.* #43's tests check four depths (`usize::BITS + 1`, `+ 2`, 128,
  130) under names that claim the family (verified,
  `party/forks/tests.rs:189-266` and `310-357`). `AGENTS.md` asks that a
  family claim be a proptest or an exhausted domain. This exhausts depths 0
  to 140 and 0 to 40 shares before the end, and also checks the last share's
  identity, which #43 does not.
- *Cost.* Port its positioning to the plan's own `skip_shares` (verified
  present); 0.2 to 0.9 seconds (reported).
- *Verdict.* Rescue. I rank it above the catalogue (34).

**01.7 and 01.13, the zigzag at stack-safety depth, narrowed.**
- *Coverage.* The doctrine requires a committed deep-input test proving no
  traversal recurses on input depth. `deep_identity_stack_safety` drives a
  left and a right spine and their combs (verified, `clock/tests.rs:841-846`);
  no deep test alternates direction (verified, no zigzag builder outside the
  tick tests). An alternating spine is a different input family for that
  proof.
- *Narrowed form.* Add a `deep_zigzag_party` builder beside
  `deep_left_spine_party`, writing bits as it does, and put the zigzag cell in
  that test's loop. Leave out 01.7's explicit 256 KiB thread; whether the
  stack tests should fix their stack size, rather than rest on libtest's
  default (the depth constant's doc says a larger `RUST_MIN_STACK` weakens
  it, verified), is your call, and naturally part of question 68's work.
- *Cost.* A few lines, under a second.
- *Verdict.* Rescue narrowed. I rank it well above the catalogue (108).

**04.5, 04.14, and 04.6, rational `Rank` inputs, together.**
- *Coverage.* `RANK_TRIPLE`'s driver draws odd numerators at exponents below
  100,000, so it yields no integral rank, no even integer, and no zero in
  20,000 draws (reported). 04.5's `Q` holds `num / 2^exp` without
  normalizing, compares by cross-multiplication, and renders canonical text
  without production code. Its generators reach integers, zero, and equal
  values with different spellings.
- *Narrowed form.* (a) `Q`, `render`, `arb_q`, and `arb_related_q` into the
  shared modules, and a second `RANK_TRIPLE` driver over them (04.5);
  (b) `arb_q` as a second strategy for the committed
  `rank_sum_equals_the_pairwise_fold`, with `Q` as reference (04.14);
  (c) from 04.6, only a `RANK_TRIPLE` law that `Display` under width,
  precision, alignment, and fill equals `str` formatting of the plain text.
  09.A2's `rank_add_spellings_agree` covers `+=`, so 04.6 need not.
- *Cost.* About 150 lines; under a second each.
- *Verdict.* Rescue. Agrees (3); 04.6 and 04.14 narrowed.

**09.A2, the trait-impl laws #83 narrowed out (decision 4).**
- *Coverage.* `main` lacks (verified, by comparing law names with
  `archive/rescue-d653bb83`): `Count`'s addition in every spelling, `Rank`'s
  addition spellings, every owned and borrowed `OwnVersion` comparison cell
  against the materialized verdict, the size hints of the four shape walks
  bracketing what remains, `Ranked`'s and the atoms' `Debug`, and
  `Query::from(&q)` as a copy. These state `Iterator::size_hint`'s contract,
  the `Debug` formats the rustdoc documents (verified, `ranked.rs:324` and
  `causally/forms.rs:400-408`), and agreement among spellings of one
  operator, where an argument-order slip in a borrowed cell would answer the
  converse.
- *Cost.* A hand port of about 250 lines from the archived commit, since the
  squash removed the revert path; the laws ride existing drivers. Skip its
  two region-union conservation laws, which #81's multiplicity laws supersede.
- *Verdict.* Rescue. Agrees (4), at a higher port cost than the catalogue
  states.

**06.4, reader entry points under chunked, interrupting, and failing readers.**
- *Coverage.* Every reader-taking decoder promises to decode "from a
  reader" (verified, e.g. `version.rs:1057`), and `std::io::Read` permits short
  reads and `Interrupted`. Only `Rank::decode` meets such a reader today,
  through #78's `ScriptedReader` (verified, `version/tests.rs:1510`).
- *Narrowed form.* Drive `Party`, `Version`, `Clock`, `Span`, and `Ranked`
  decoding, and the borsh impls, through #78's reader with the slice decode as
  oracle, and use #78's exact failure oracle rather than 06.4's
  accept-either one.
- *Cost.* About 100 lines, under a second (reported).
- *Verdict.* Rescue. Above the catalogue (29).

**06.6, encoders under failing writers, narrowed (decision 6).**
- *Coverage.* `encode_to` promises "Returns any error reported by the writer"
  (verified, `version.rs:979-983`). The only committed test writer accepts one
  byte per call and never fails (reported).
- *Narrowed form.* Assert error propagation and exact bytes when the writer
  has room, under short writes and `Interrupted`; drop the "a prefix was
  written" assertion, which no `# Errors` section promises.
- *Cost.* About 80 lines, 0.25 seconds (reported).
- *Verdict.* Rescue narrowed.

**08.6, the `usize` signature census.**
- *Coverage.* Your rule treats a new `usize` in a public signature as a
  possible 32-bit defect. The surface check pins trait impls, including the
  three `usize` conversions (verified, `surfacecheck/src/census.rs:209-221`),
  but no signature's types.
- *Cost.* A port of 61 lines of Python into `surfacecheck` as a census
  reconciled both ways; negligible runtime beside the rustdoc build. It does
  not reach suanpan, which has no surface check.
- *Verdict.* Rescue. Agrees (16).

### 2.2 Rescue as a group

**Group E: deep generators into the existing drivers (no new oracle).**
The committed laws are oracle-free and the tree oracle handles depths of a
few hundred, so these generators multiply every committed law and
tree-oracle differential without a new model.

- **04.1, the multi-scale partition generator, with 04.7's deferral tap.**
  Deferral in 50.8% of pairs and exact rank ties between distinct versions in
  29.1%, against none in committed arbitrary or organic pairs (reported,
  census). Feed it to the law drivers and to `assert_single` and
  `assert_pair` in `version/measure/tests.rs`. Take the default setting only;
  the 3,000-level setting needs explicit-stack threads. 04.7's four lines add
  `DEFER_HITS` beside `FREEZE_HITS` (verified, `integral.rs:158`). The
  committed measure tests' own docs claim to drive "the parked, deferred, and
  settled pipeline" while their floors count freezes only (verified,
  `version/measure/tests.rs:395-410`), so the tap makes an existing claim
  checkable. I rank 04.7 higher than the catalogue (36) for that reason.
- **02.3 and 02.5, deep random topology and boundary heights.** Topology to
  depth 300 under six split strategies, and heights aimed at the writer's
  63/65-bit code boundary (reported). One topology generator with several
  height arms is the right shape (the catalogue's overlap section agrees).
  Give each component its own size budget: in the deepest tier, 97.7% of fold
  draws read an exhausted tape (reported).
- **01.1, deep party generators.** Depth to 80 against 4; `arb_disjoint(k)`
  produces accepted families of 2 to 10 arbitrary parties, where the committed
  families give none of arity 2 or more (reported, 0 of 17,781). With an alias
  inserted at a random position, it also reaches the late overlap that
  `follow-ups.md` says `arb_clock_family` never reaches (verified, listed).
  The function-space leg of a differential row must cap its operands' depth.
- **01.4, narrowed:** family laws over `arb_disjoint` (accept iff disjoint;
  multiplicity conserved on rejection at arities 2 to 10; every clock
  unchanged after a failed `sync_all`). Its exact-dealing check waits for
  group F.
- **05.9, narrowed:** its near-equal wide alphabets as one more height arm.
- **Floors** for each generator's stated regime (decision 8).
- *Cost.* About 600 lines of generators plus driver wiring. The law drivers'
  runtime on deep inputs is unmeasured; set case counts so each driver stays
  within a few seconds.
- *Verdict.* Rescue. Of the groups, this one adds the most coverage per line.

**Group D: co-generated tick inputs for the tick laws.**
- **03.3:** two laws, `min_ticks` rises by at most one per tick and by at
  most `a + b` under `ticks(a + b)`. Both follow from the definition:
  appending ticks to a minimal history reaches the successor. Add #74's
  co-generated cases as a third `VERSION_PARTY` driver; today the drivers use
  arbitrary and organic inputs only (verified,
  `algebraic_laws/tests.rs`).
- **03.4, narrowed:** a deeper tier of #74's spine strategy, hundreds of
  levels, inside that driver. Decline its 3,000-level form on a 1 GiB stack,
  whose runtime varies widely and whose shrinks exceeded the old time limit
  (reported).
- **09.A10, optional:** the memo's reuse property at unit level, against a
  vector model. It localizes a failure #74's end-to-end strategy would also
  see, and is coupled to an internal structure, so take it only if you want
  unit-level checks of `Memo`.
- *Cost.* Small; run the driver at a reduced case count as #74 does for its
  wide strategies (its spine property takes 12.7 seconds, reported).
- *Verdict.* Rescue 03.3 and 03.4 narrowed; 09.A10 at your preference.

**Group F: one flat model of versions and parties.**
- *Parts.* 02.1's leaf-list model and 04.3's flat sweep share one
  representation, `(depth, height)` leaves with absolute heights (reported);
  merge them, with 04.3's functions as the measures. Port 01.2's fork
  characterization onto the same `BigUint` positions, which removes its
  126-level cap. Use one canonical encoder: 02.6 now, or 06.1's if group H
  lands first. Fold 02.4's correlated pair families here, since they call the
  model.
- *Properties.* 02.2 split into one property per family (lattice, order,
  projection, shapes, folds, span placement), each a few seconds; today it is
  one 48-second property (reported). 04.10's exactness checks only. 01.5
  narrowed to exact `fork`, `forks`, array splits at counts other than 4, and
  clock-level `sync` after a partial overlap. 01.4's exact `sync_all`
  dealing. 01.6's share identity at counts past `usize`.
- *Why a third oracle.* The bridge builds versions through production's
  `VersionWriter` (verified, `testing/version.rs:38-39`), so a
  canonicalization error shared by the writer passes every tree-oracle
  differential, and the function-space oracle cannot judge inputs past a few
  levels. No committed oracle judges deep inputs independently of both the
  tree recursion and the writer.
- *Cost.* The largest group after H: roughly 900 lines of model and 600 of
  properties, with docs and a validation-index entry.
- *Verdict.* Rescue, after group E. I disagree with ranking 01.2 (27)
  separately from 02.1 (19); they belong in one module.

**Group G: exact coverage beyond the grid.**
- *Parts.* 05.2's census over the common refinement of group F's partitions,
  so 05.7's vector model is a view rather than a third version type. 05.3
  extended to the census. One shaped antichain property from 05.1, which
  must construct at least one hole (today 2% to 8% of cases check nothing,
  reported). One shaped query property from 05.5. 05.11 as a delta to the
  committed clause enums: operand order and the `!before` spelling.
- *Coverage.* Exact coverage with three or more concurrent holes, at depth
  and with heights of `2^64` and above: thousands of such checks per 256
  cases, against 8 committed, all on two cells (reported).
- *Cost.* Moderate. The shaped antichain property takes 26 seconds at its
  default count (reported); cut its cases to stay near 5.
- *Verdict.* Rescue after group F. If you decline F, the census can stand on
  the function-space oracle sampled at a shared grid instead (the
  catalogue's suggestion).

**Group H: the specification codec, staged.**
- *Stage 1.* 06.5's near-valid generators (model trees with edits), a
  totality property (no decoder panics on any of them), and one property that
  text, postcard, CBOR, borsh, and a chunked reader agree with the slice
  decode. Roughly 600 lines (inferred).
- *Stage 2.* 06.1's specification verdicts for all six formats, widened to
  every applicable error class (decision 2), about 1,200 more lines.
- *Why.* `Eq` and `Hash` are defined on canonical bytes (#83's laws hold
  them), so a decoder that accepts a non-canonical encoding breaks a public
  promise. The committed rejection tests generate non-canonical trees at
  depth 4 at most (reported).
- *Fold work.* Replace the hand-built runner and `L6_CASES` with
  `proptest!`, so failures leave seeds; make its eight recursive helpers
  iterative or route them through `descend!`; drop `allow(dead_code)`.
  06.12 survives only as stage 1's floors.
- *Verdict.* Rescue, staged. It is the broadest single check and the largest
  port; stage 1 alone is worth having.

**Group I: fuel-board families, after #136 lands, in one recalibration.**
- **06.3:** rejection rows for `Rank` and `Ranked` decoding. The board has
  truncated-decode rows for the other four types and none for these (verified,
  `board/ops.rs`).
- **04.4:** a pair family whose second version is the first's mirror, so the
  two have equal rank. `Ranked`'s ordering doc names exact ties as its worst
  case (verified, `ranked.rs:352-360`), and the board's `ranked_cmp` row
  compares a version with itself plus one tick, which never ties (reported).
- **01.8:** `party_without` with a deep receiver; the board meters it only
  from the seed (verified, `ops.rs:2262-2273`). And a fork step whose count
  width grows apart from the party's size; the fork plan's `BigUint` work is
  invisible to scan but visible to fuel.
- **07.2, re-expressed:** port #52's ±S and OS family builders (still on
  `audit/suanpan-zero-range-fuel-ladder`) as board families, so every row,
  join and tick included, sees accumulators with sparse written positions. No
  fuel-board family drives that state (verified, its family list), and #52's
  purpose was dropped on the premise that the fuel board replaces it.
- **09.A9, re-expressed:** its two wide-offset combs as `min_ticks` families,
  where an outer minimum's offset widens with the input.
- **04.2:** covered by the owner-scheduled `Sum` family; build that family in
  the same branch.
- *Cost.* Each family adds a cell per applicable row to both tables and grows
  board runtime about in proportion (61 families now).
- *Verdict.* Rescue as one branch.

**Group J: suanpan's value oracle over accumulators with histories (07.1).**
- *Coverage.* The committed surface property builds every operand fresh, from
  a primitive, shifted limbs, a small boundary, or a cancelled high word or top
  (verified, `surface.rs`'s `Operand`); no suanpan property drives a pool of
  accumulators carrying their own histories (verified). 07.1 reaches operands
  holding prior histories, aliasing, values to 5,112 digits, and extreme
  digits meeting `normalize` and shifts six to eight times as often
  (reported).
- *Narrowed form.* Port the explore branch's pool generator, not #37's, and
  drop every zero-range invariant (#86 removed the map). Keep the readout,
  representation, `2.01 · 2^stored_bits` bound, exact maximum, stability, and
  conversion checks.
- *Cost.* About 360 lines of checks plus 180 of generator, rewritten to
  #122's API; 3.7 seconds (reported).
- *Verdict.* Rescue. Its catalogue position (22) assumed #37's generator.

### 2.3 Rescue only with another change

- **06.15, CBOR byte-string bridging:** a test of a few lines, with #85's
  commit stating the bridging (verified, `d8d078cc5` on
  `docs/audit-corrections`). A documented promise should be tested.
- **09.A22, the `from_parts` counterexamples:** with #85's warning; better as
  doctests on the warning, or rows in `tests/stale_state.rs`.
- **01.3, the rule-respecting history driver, as an extension of
  `testing::optrace`:** add `forks(k).take(t)`, array splits, `join_all`,
  `sync_all`, and byte round trips to `Op`, give each trace applier the new
  kinds, and re-derive the function-space grid. Every organic consumer then
  sees multi-party operations and larger populations, and each trace can
  assert the crate page's promise that rule-respecting use never fails
  (reported). Moderate to high cost; after groups E and F. Decline its
  parallel-harness form.
- **06.2 (narrowed) and 07.5:** more 32-bit boundaries. 06.2 is the only
  borsh decode on a 32-bit target (verified: the guest builds `before` with
  `meter` only); restate its docs as a bit-position check and use a leaf near
  `2^31` bits to halve its 150 seconds (reported). 07.5's case 5 reaches a
  limb index that fits `usize` while its digit position does not, which no
  committed case reaches (verified, guest cases 1 to 6). Each adds tens of
  seconds to the wasm32-pins tests; take them only if you want more 32-bit
  boundaries.

### 2.4 Already carried

- **03.5 and 09.A8:** the jump-entered rising spine is a board family on
  `main` (verified, `registry.rs:1380`).
- **09.A7:** #110's `foreign_thread_requests_are_excluded` reproduces the
  libtest race deterministically (verified, `alloc-meter/src/tests.rs:262`).
- **01.9:** #106's exact-sizing properties cover `join`, `join_all`,
  `without`, the `forks` residual, and more (verified, `spare_capacity` call
  sites). The one uncovered reading, a party taken from a decoded clock
  holding the clock's buffer (observation O1), is a design question, not an
  instrument.
- **04.2:** the owner-scheduled `Sum` family (group I).

### 2.5 Decline

Each row states why it adds no meaningful coverage, or why its cost
outweighs what it adds.

**Superseded by a rescued instrument or by landed work.**

| ID | Name | Why |
|---|---|---|
| 01.10 | Overlay refinement property | Its added depth is on the party side; 01.1 feeding the committed `clock_shape_matches_the_oracle` row gives the same. |
| 01.11 | Identity generator statistics | Floors come from decision 8, built where a generator claims a regime. |
| 02.7 | Party region generator | 01.1 covers deep parties and masks. |
| 02.8 | Writer-path census | Only as 02.4's floor inside group F. |
| 04.10 | Pair predicates over the flat oracle | Only its exactness checks, in group F. |
| 04.13 | Seven-route rank property | Group F's oracle judges rank independently; the extra routes are committed laws. |
| 05.7 | Vector model as its own type | Only as a view in group G. |
| 05.8 | Deep span layouts | 02.3's topology replaces it. |
| 05.10 | Span verdicts against vectors | Group F checks `partial_cmp` independently at depth, which every committed span verdict derives from. |
| 05.12 | Projected-span views against vectors | Group F checks projection independently; its verdict mix is lopsided (reported). |
| 05.14 | Span algebra against vectors | Committed laws cover every spelling (reported). |
| 05.15 | Organic overlays | Shallow; other generators cover it. |
| 06.7 | Encoder agreement, pointwise model | Duplicates group F's byte comparisons. |
| 06.9 | First-detected-class model | Decision 2. |
| 08.7 | Lone-prefix padding tests | Decision 2. |
| 09.A5 | Fuel comparison between commits | The fuel board's exact compare covers fuel (verified, its `compare.rs`); a fuzz-fit dump mode is optional tooling. |
| 09.A6 | Ceiling-rule auditor | Per-cell ceilings are generated by the calibration recipe; the outer bounds' drift is #115's open question. |
| 09.A11, 04.16 | wasm32 memory readouts | #31 already reports an allocation abort as such; under the input-size ruling a memory ceiling per pin adds little. |
| 09.A14 | wasm32 stability differential | #50's strengthened case covers the defect; 70 seconds (reported). |
| 09.A33 | Differentials against replaced code | Keeps deleted implementations as oracles beside two committed ones. |

**Cost meters with no worthwhile fuel or heap form.**

| ID | Name | Why |
|---|---|---|
| 02.10, 02.11 | Fragmented-mask and carry-ripple families | Flat readings; restate the board's mask and cliff-comb families in another topology. |
| 03.7 | Plain rising spine | The fix landed; its verdict depended on where samples fall against powers of two (see section 5). |
| 03.8, 03.9 | Random tick heap and counter searches | Random inputs sit far under the ceilings (0.43 and 0.61, reported); the board's constructed families are where worst cases live. |
| 03.12 | Memo-dense heap families | The board's memo families build the same traffic. |
| 04.12 | Integrator cost search | Prints only; touch-based. |
| 05.13 | Carry-boundary touch family | Never failed; asserts nothing. |
| 05.18 | Refinement scan probe | #61 and #94 carry it. |
| 07.4 | Hill-climbing touch adversary | Built for #37's constant; goes with #37. |
| 07.6 | Offset-comparison accounting | Needs a per-function production hook; the board judges the whole operation. |
| 09.A3 | Touch-bound worst cases | Touch retires and #37 is likely dropped; suanpan-only workloads are not on the fuel board. |
| 09.A4 | `min_ticks` heap sweep | The defect is fixed and the families landed; see section 5 for the sampling question it raises. |
| 09.A15 | Real-scale stepped spine | Needs gigabytes; the fix landed. |
| 09.A25 | Heap sweeps for single operations | Lazy-input folds and between-sample sizes; low value, and you kept the lookahead (question 69). |

**Mutant sets and calibration (decision 3 moot).** 01.12, 02.13, 02.14,
03.6, 04.9, 04.11, 04.17, 05.17, 05.20, 06.10, 06.14, 07.3, 08.2, 08.4,
08.5, 08.8, 08.12, 09.A12, 09.A16, 09.A17, 09.A18, 09.A19, 09.A36. None has
a use that does not rest on mutants. 04.9's schedule switches could become a
metamorphic property, but only through a test-only schedule override in
`integral.rs`, and groups E and F reach deferral naturally.

**Diagnostics, one-off tools, and settled questions.**

| ID | Name | Why |
|---|---|---|
| 01.14 | Unoptimized deep-test configuration | Ruled out (question 68). |
| 02.9 | Stack-overflow calibration | #75's derivation states a stronger bound. |
| 02.12, 02.15, 02.16 | Long-run copies, runner, converter | Runners, not checks. |
| 03.10, 03.11 | Bushy strategy, second perturbation | #74's spine dominates; no measured contribution. |
| 03.13, 03.14 | Tick generator histograms and reach | #74's census floors hold the regimes. |
| 03.15 | Bridge round trip on one pair | Every bridged differential depends on it. |
| 04.15 | wasm32 `Sum` footprint cases | Observation O6, ruled not a defect. |
| 05.16, 05.19 | Span reach probe, histograms | Measurements; floors come from decision 8. |
| 05.21 | Witness heuristic | Dead code, not in the tree. |
| 06.8 | Writer copy-path harness | Internal APIs; reach not new (reported). |
| 06.11, 06.13, 06.16 | wasm32 rank stream, reader, `Vec` growth probes | Little beyond committed checks, or `std` behavior. |
| 07.7 to 07.11 | Readout census, prototype T, size probe, replay, drafts | Diagnosis and design history. |
| 08.1 | Generator census | Keep only the census functions a floor needs (decision 8). |
| 08.9, 08.10, 08.11, 08.13, 08.14, 08.15 | Coverage map, bisect harness, surface calibration, `num-bigint` probe, writer hunt, libtest probe | Periodic reports or settled findings. |
| 09.A13 | Swarm close bound for #82 | Its only evidence is a detection rate against one mutant. |
| 09.A20 | Third-party serde formats | Decision 10. |
| 09.A21 | Board-cell bisection and allocator log | Developer tooling with `unsafe`; `git bisect run` and #95 cover the need. |
| 09.A24 | Guest panic records | Probes of the test harness's own channel; #31's prose records them. |
| 09.A27 | Shifted zero through every spelling | The public cost model allows O(`A`) retained space for `+=` with `A` the operand's working width (verified, suanpan `lib.rs`); only `add_shifted` and `sub_shifted` promise O(1) for a zero. Take it only if you make that promise public for every spelling. |
| 09.A28 | Dense-span bound probe | Needs `S` threaded into `DensePart::new`; #72's proof stands. |
| 09.A29, 09.A30 | Compare pin past `2^32`, rank gap cases | No narrowing on those paths; duplicate #63's cases. |
| 09.A31 | Self-recursion scan | Question 68 removes the guard it distinguishes; #34 and #75 check the rule at run time. |
| 09.A32 | Board recipe interrupt probe | No home for recipe tests. |
| 09.A34, 09.A35 | Decoder-growth and superseded instruments | Your input-size and leniency rulings. |

## 3. Fold-in groupings and order

Order is by dependency first, then by coverage per unit of work.

1. **The small batch (section 2.1),** one branch, one commit per instrument:
   05.4 with 05.3, 05.6, 08.3, 04.8, 03.1, 03.2, 09.A1, 09.A26, 01.7 with
   01.13, the rational-rank set (04.5, 04.14, 04.6), 06.4, 06.6, 08.6.
2. **09.A2's laws,** a separate branch, since it is a hand port (decision 4).
3. **Group E,** deep generators with their floors. It is the base for D's
   deeper tier, F, and G.
4. **Group D,** co-generated tick laws.
5. **Group J,** suanpan's value oracle; independent, so it can run beside 3
   and 4.
6. **Group I,** fuel-board families, once #136 lands; it can share a branch
   with the owner-scheduled `Sum` family.
7. **Group F,** the flat model, after E.
8. **Group G,** exact coverage, after F.
9. **Group H,** the codec, staged; independent of F, but whichever lands
   first supplies the one encoder.
10. **Conditional:** 01.3's `optrace` extension after E and F; 06.15 and
    09.A22 with #85; 06.2 and 07.5 if you want them.

## 4. The open decisions

**Decision 2: does any decoder promise which error a doubly malformed input
returns?** I recommend promising nothing, which is option (a). `Decode`
already tells callers to handle any applicable variant, and only span
documents a precedence. A precedence for every decoder would be a public
commitment that constrains every future validator, for a benefit the
catalogue names only in principle ("`Truncated` means read more"). No caller
in `rumors` reads the variant that way that I know of; if its transport ever
frames messages incrementally, that is the time to promise it, with that
caller's tests. Under (a), widen 06.1's three narrowed verdicts to every
applicable class, and decline 06.9 and 08.7.

**Decision 4: do you want #83's narrowed laws back?** Yes, as a hand port
from `archive/rescue-d653bb83`. The squash the catalogue proposed has
happened, so the revert path is gone. Take the spellings, the `OwnVersion`
cells, the four size-hint brackets, the two `Debug` laws (their formats are
documented on `main`), and the borrowed-query copy; skip the two region-union
conservation laws, which #81 superseded.

**Decision 6: should encoders promise what a failing writer has received?**
No. Keep 06.6's assertions that the writer's error is returned, which
`encode_to` already promises, and that the bytes are exact when the writer has
room, under short writes and `Interrupted`. Leave the prefix unpromised until
a caller needs it.

**Decision 7: is a foreign tick in your model?** Leave it out. A foreign tick
is `Version::tick` with a party on a version that party's clock never held;
the floor still holds there, but it is not a `Clock` operation, and the
committed `world_strategy` already gives about 40 times as many tight
observations once the ledger is per clock (reported). Wide counts and `absorb`
belong to 01.3's `optrace` extension, if you take it, run production-only for
the wide counts.

**Decision 8: should every folded-in generator carry a reach floor?** Yes,
narrowly. A floor is a liveness check on a claim. Each folded generator's doc,
and the docs of the tests it feeds, will claim a regime: deferral for 04.1,
depth past 32 for 01.1 and 02.3, accepted families of arity 2 or more for
`arb_disjoint`, three or more concurrent holes for 05.1, writer cascades for
02.4. Floor exactly those, per generator, beside
`generator_classes_stay_under_mass`, using 08.1's census functions only where
a floor needs a metric. Do not build a census module over the committed
generators' relational classes for its own sake; a floor with no claim behind
it justifies itself.

**Decision 9: take the two instruments declined under the narrowed caution?**
Take 09.A1: it exhausts a finite domain for the comparison every
multiplicity law rests on, against an independent oracle, in half a second.
Decline 09.A13's swarm close bound: its only evidence is a detection rate
against one constructed mutant, and it opens no region of input space the
property lacks.

**Decision 10: eight dev-dependencies for real serde formats?** No. #51 tests
the serde contract against serde's own test deserializers; the third-party
quirks the probe found (byte keys from csv headers, sequences from csv and
MessagePack) are the leniency you ruled for and #51 states. If you want
assurance against those formats, run the probe once before a release; do not
commit it.

## 5. What I could not settle

- **Runtimes of the folded forms.** The law drivers over deep generators,
  group F's split properties, and group G's shaped properties at reduced
  counts are unmeasured; every figure above comes from the catalogue's runs
  of the unfolded instruments. Group E is where a measurement would most
  change a choice of case counts.
- **Whether the explore code still compiles at `main`.** I built nothing. The
  `before` instruments target files that changed little, but 07.1 needs a
  real port to #122's API, and 06.1's port estimate is the catalogue's.
- **The board's heap samples and powers of two.** A doubling buffer's peak
  heap sits just past a power of two, and the board samples each family at
  fixed sizes (reported); the `min_ticks` fix pinned two sizes per family
  (verified, `89232f092`). 09.A4's sweep found readings far over the ceiling
  only at such sizes, on the code before the fix (reported). Whether the
  fixed `min_ticks`, or any other operation, exceeds a ceiling at sizes the
  board skips is unknown. This is a question about the board's sampling, not
  a reason to rescue 09.A4: should each family's ladder include one size just
  past a power of two?
- **Observation O1.** A party taken from a decoded clock may still hold the
  clock's buffer (reported, before #106's exact sealing). It is a design
  question for you, outside any instrument.
- **09.A27.** It turns on whether suanpan should promise publicly that a zero
  operand costs constant space in every spelling, as its private module doc
  says.
- **The doctrine's two rules about failures** (section 1.3): whether you want
  Principle 3's "name a constructible failure" rephrased, so it no longer
  reads against Principle 6.

## Appendix: every row's verdict

*Position* is the catalogue's rank. Verdicts: **R** rescue (section 2.1),
**G:x** rescue in group x (2.2), **W** rescue only with another change
(2.3), **C** carried (2.4), **N** narrowed, **D** decline (2.5).

| ID | Name | Position | Verdict |
|---|---|---:|---|
| 01.1 | Party generators | 6 | G:E |
| 01.2 | Interval-set party model | 27 | G:F (merged into one model) |
| 01.3 | Rule-respecting history driver | 49 | W (as an `optrace` extension) |
| 01.4 | `join_all`/`sync_all` against the model | 48 | G:E and G:F, N |
| 01.5 | Set algebra, fork, splits, sync against the model | 47 | G:F, N |
| 01.6 | Wide-count fork properties | 35 | G:F |
| 01.7 | Deep probe on a 256 KiB stack | 108 | R, N (zigzag only) |
| 01.8 | Identity scan-cost probe | 65 | G:I (two families) |
| 01.9 | Identity heap-retention probe | 86 | C (#106; O1 is a question) |
| 01.10 | Overlay refinement property | 70 | D |
| 01.11 | Identity generator statistics | 113 | D |
| 01.12 | Identity mutant lists | 96 | D |
| 01.13 | Deep-party builders | 109 | R, N (zigzag builders only) |
| 01.14 | Unoptimized deep-test configuration | 138 | D |
| 02.1 | Leaf-list model | 19 | G:F |
| 02.2 | `algebra_matches_model` | 40 | G:F, N (split by family) |
| 02.3 | Deep random partitions | 14 | G:E |
| 02.4 | Correlated pair families | 39 | G:F |
| 02.5 | Boundary-height palette | 38 | G:E |
| 02.6 | Independent canonical encoders | 68 | G:F (unless H's encoder) |
| 02.7 | Party region generator | 37 | D |
| 02.8 | Writer-path census | 114 | D (only as 02.4's floor) |
| 02.9 | Stack-overflow calibration | 130 | D |
| 02.10 | Fragmented-mask cost families | 105 | D |
| 02.11 | Carry-ripple cost families | 107 | D |
| 02.12 | Long-run copies | 154 | D |
| 02.13 | Round-1 mutants | 97 | D |
| 02.14 | Survivor and tie-order patches | 140 | D |
| 02.15 | `run_on_tree.sh` | 155 | D |
| 02.16 | `extract.py` | 156 | D |
| 03.1 | `min_ticks` attainment | 9 | R |
| 03.2 | `min_ticks` per causal past | 7 | R, N |
| 03.3 | Tick claims #74 did not carry | 13 | G:D |
| 03.4 | Deep co-generated spines | 66 | G:D, N |
| 03.5 | Jump-entered rising spine (with 09.A8) | 80 | C (landed) |
| 03.6 | Events mutant set | 95 | D |
| 03.7 | Plain rising spine | 83 | D |
| 03.8 | Random tick heap search | 84 | D |
| 03.9 | Random counter search | 63 | D |
| 03.10 | Bushy strategy | 131 | D |
| 03.11 | Second late perturbation | 132 | D |
| 03.12 | Memo-dense heap families | 142 | D |
| 03.13 | Events generator histograms | 115 | D |
| 03.14 | Memo reach diagnostic | 143 | D |
| 03.15 | Bridge round trip | 153 | D |
| 04.1 | Multi-scale partition generator | 5 | G:E |
| 04.2 | `Rank` sum-order cost probe | 11 | C (owner-scheduled `Sum` family, G:I) |
| 04.3 | Flat sweep oracle | 20 | G:F |
| 04.4 | `Ranked::cmp` tie-cost probe | 28 | G:I (mirror-tie family) |
| 04.5 | Rational `Rank` oracle | 3 | R |
| 04.6 | `Rank` value property | 18 | R, N (formatter law) |
| 04.7 | `DEFER_HITS` tap and census | 36 | G:E, N (tap and floors) |
| 04.8 | `Count` boundary sweep | 15 | R |
| 04.9 | Schedule-independence probes | 103 | D |
| 04.10 | Pair predicates over the flat oracle | 72 | G:F, N (exactness only) |
| 04.11 | Integrator mutant patch | 98 | D |
| 04.12 | Integrator cost search | 64 | D |
| 04.13 | Seven-route rank property | 71 | D |
| 04.14 | `Sum` over any order | 73 | R, N (second strategy) |
| 04.15 | wasm32 `Sum` footprint cases | 127 | D |
| 04.16 | wasm32 memory-size readout | 57 | D |
| 04.17 | `Rank` mutants | 99 | D |
| 05.1 | Pairwise-concurrent holes | 24 | G:G, N |
| 05.2 | Exact coverage census | 25 | G:G |
| 05.3 | Census validator | 42 | R (retargeted), extended in G:G |
| 05.4 | Exhaustive boolean cube | 1 | R |
| 05.5 | Query exact coverage across worlds | 41 | G:G, N |
| 05.6 | Span-to-query agreement | 8 | R |
| 05.7 | Vector model and verdict oracles | 26 | G:G, N (as a view) |
| 05.8 | Deep span layouts | 44 | D |
| 05.9 | Grid worlds, near-equal wide heights | 43 | G:E, N (height arm) |
| 05.10 | Span verdicts against vectors | 45 | D |
| 05.11 | Clause model and folds | 46 | G:G, N (delta) |
| 05.12 | Projected-span views | 69 | D |
| 05.13 | Carry-boundary touch family | 106 | D |
| 05.14 | Span algebra against vectors | 75 | D |
| 05.15 | Organic overlays | 76 | D |
| 05.16 | Committed-population reach probe | 116 | D |
| 05.17 | Spans mutants | 100 | D |
| 05.18 | Refinement scan probe | 144 | D |
| 05.19 | Spans histograms | 145 | D |
| 05.20 | Tie-order experiment | 141 | D |
| 05.21 | Witness heuristic | 159 | D |
| 06.1 | Specification-codec differential | 21 | G:H (stage 2) |
| 06.2 | wasm32 borsh probe | 61 | W, N |
| 06.3 | Hostile `Rank`/`Ranked` rejection families | 31 | G:I |
| 06.4 | Misbehaving-reader probe | 29 | R, N (on #78's reader) |
| 06.5 | Near-valid encoding generators | 50 | G:H (stage 1) |
| 06.6 | Failing-writer probe | 30 | R, N |
| 06.7 | Encoder agreement, pointwise model | 67 | D |
| 06.8 | Writer copy-path harness | 74 | D |
| 06.9 | First-detected-class model | 118 | D |
| 06.10 | Decoder mutation schemas | 101 | D |
| 06.11 | wasm32 rank group-stream probe | 128 | D |
| 06.12 | Codec reach histograms | 117 | G:H, N (floors only) |
| 06.13 | wasm32 reader probe | 136 | D |
| 06.14 | Writer mutants and reach probes | 139 | D |
| 06.15 | CBOR framing probe | 77 | W (with #85) |
| 06.16 | wasm32 `Vec` growth diagnostic | 150 | D |
| 07.1 | Pool model's exact-oracle checks | 22 | G:J |
| 07.2 | Fuel matrix's further operations | 23 | G:I (as ±S and OS families) |
| 07.3 | Suanpan mutant schema | 94 | D |
| 07.4 | Hill-climbing touch adversary | 104 | D |
| 07.5 | wasm32 landing case 5 | 55 | W |
| 07.6 | Offset-comparison accounting | 133 | D |
| 07.7 | Readout carry census | 146 | D |
| 07.8 | Prototype T | 151 | D |
| 07.9 | Native size probe | 147 | D |
| 07.10 | Touch replay probe | 152 | D |
| 07.11 | Scratch drafts | 157 | D |
| 08.1 | Generator census | 17 | D (functions only, for floors) |
| 08.2 | wasm32 injection table | 51 | D |
| 08.3 | Gamma-window test (with 09.A23) | 10 | R |
| 08.4 | Classified survivors | 87 | D |
| 08.5 | Mutation campaign tooling | 88 | D |
| 08.6 | `usize` signature census | 16 | R |
| 08.7 | Lone-prefix padding tests | 119 | D |
| 08.8 | Probe copy and hit recorder | 111 | D |
| 08.9 | Branch-coverage map | 112 | D |
| 08.10 | Counter bisect harness | 123 | D |
| 08.11 | Surface check calibration | 121 | D |
| 08.12 | Fold-pin calibration | 122 | D |
| 08.13 | `num-bigint` formatting probe | 137 | D |
| 08.14 | Split-writer hunt | 148 | D |
| 08.15 | libtest invocation probe | 149 | D |
| 09.A1 | Exhaustive multiplicity agreement | 2 | R |
| 09.A2 | #83's narrowed laws | 4 | R (hand port) |
| 09.A3 | Touch-bound worst cases | 12 | D |
| 09.A4 | `min_ticks` heap sweep | 79 | D |
| 09.A5 | Fuel comparison between commits | 54 | D |
| 09.A6 | Ceiling-rule auditor | 52 | D |
| 09.A7 | libtest race probe | 53 | C (#110) |
| 09.A9 | `min_ticks` limb work on combs | 81 | G:I (as families) |
| 09.A10 | Memo reuse property | 32 | G:D (optional) |
| 09.A11 | wasm32 memory after each case | 56 | D |
| 09.A12 | `Rank::decode` mutant schema | 90 | D |
| 09.A13 | Swarm close bound | 33 | D |
| 09.A14 | wasm32 stability differential | 62 | D |
| 09.A15 | Real-scale stepped spine | 82 | D |
| 09.A16 | Readout class mutants | 91 | D |
| 09.A17 | Stack-recursion mutants | 92 | D |
| 09.A18 | Written-set mutants | 93 | D |
| 09.A19 | Compile-time mutant switches | 89 | D |
| 09.A20 | Third-party serde formats | 78 | D |
| 09.A21 | Board-cell bisection, allocator log | 110 | D |
| 09.A22 | `from_parts` counterexamples | 59 | W (with #85) |
| 09.A24 | Guest panic records | 60 | D |
| 09.A25 | Heap size sweeps | 85 | D |
| 09.A26 | Fork-plan tails to depth 140 | 34 | R |
| 09.A27 | Shifted zero through every spelling | 58 | D |
| 09.A28 | Dense-span bound probe | 124 | D |
| 09.A29 | Compare pin past `2^32` | 125 | D |
| 09.A30 | Rank alignment gap cases | 126 | D |
| 09.A31 | Self-recursion scan | 120 | D |
| 09.A32 | Board recipe interrupt probe | 134 | D |
| 09.A33 | Differentials against replaced code | 129 | D |
| 09.A34 | 32-bit decoder-growth instruments | 135 | D |
| 09.A35 | Superseded instruments | 158 | D |
| 09.A36 | Calibration sets for carried tests | 102 | D |
