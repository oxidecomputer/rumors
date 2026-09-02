<!-- CAVEAT LECTOR: review packet for lane p1-conformance, written by Claude (Fable 5.1) for Finch under triage/WORKFLOW.md. The goal, rulings, stack position, acceptance rows, fresh-eyes rounds, and stops are the coordinator's record; nothing in this packet is authored or endorsed by Finch. Read with the ground rules in .agent-notes/README.md. -->

# Review packet: p1-conformance

## Goal

The backend conformance suite and the window census suites exist to show
that a memory budget binds a reconciliation session, by differencing a
budgeted run against a floor run. At the base commit both difference two
identical runs: a 64 KiB budget sits below the window solve's flat
decode-fan pre-charge, so every capacity floors at one and the ceiling can
never fail; beneath that, the suite convicts fewer clauses than the
`Backend` docs claim and transcribes three obligations it never checks.
This lane pairs every ceiling with a floor that proves the counter counts,
resizes the budgets only after the floor has been seen to fail, and makes
every conviction the `Backend` docs attribute to the suite true by name.

## Rulings landed

- T6: every repair to the backend conformance suite lands in one lane,
  the census liveness floor first so that it sizes `LOCAL_BUDGET` above
  the flat decode-fan pre-charge before anything else is resized; the
  three transcribed obligations (decode-slot padding, residency across the
  erase/assume re-tag, monotonicity between grid points) become pointwise
  checks in the suite rather than prose admissions.
- T18: `window_census` guards its peak measurement with the same lock
  `decode_alloc.rs` uses, so `cargo test` is a supported entry for it.
- T26: `tests-resource-link-window-20` lands exactly as its Resolution
  states and is judged by its Acceptance; a lane agent that must deviate
  stops.
- T28: `tests-resource-link-window-25` and `-28` land per their stated
  Resolution and are reviewed as the lane's diff.

## Stack position

- Base: `0926fe32`
- Parent: `main`
- Children: none

## Acceptance table

Every row is one the coordinator ran itself against the worktree at the
named commit; an entry whose acceptance does not hold is not in this
table. The negative controls (the census floor failing at 64 KiB before
the resize; each by-name control failing by its name) are re-run here
once.

| entry | commit | acceptance command | decisive output |
|---|---|---|---|
| `conformance-28` | `4b8f3c00` then `a29692e9` | `cargo nextest run -p rumors --all-features --no-capture -E 'test(local_backend_conforms) \| test(materializing_backend_conforms) \| test(a_pre_charge_only_budget_fails_the_liveness_floor)'` | `census at budget 275248 B: floor run peaked at 14496 B (widest capacity 1), budgeted run at 14576 B (widest capacity 4)`; `budget 4194304 B: floor 193707 B, budgeted 194740 B (widest capacity 44)`; `3 tests run: 3 passed` |
| (same, negative control) | `aef30fba` | `LOCAL_BUDGET` swapped to `64 * 1024`, same command restricted to `local_backend_conforms`, restored (diff empty) | panics `the budgeted window admitted nothing above the floor: both runs peaked at 14496 B`; `0 passed, 1 failed` |
| `conformance-24`, `-25`, `-30`, `-31`, `-40`, `-41`, `-42` | `c977d6a5`, `0d7a5b84`, `8ab9af2d` | `cargo nextest run -p rumors --all-features --no-fail-fast -E 'test(/^conformance::backend::tests::/) \| binary(seed_liveness)'` | `33 tests run: 33 passed` |
| `conformance-24` (both directions) | `8ab9af2d` | the two presence controls by exact name | `a_parent_conjured_from_an_empty_group_fails_the_presence_check` PASS, `an_interior_parent_answering_none_fails_the_presence_check` PASS |
| `conformance-40` (negative control) | `aef30fba` | constructed-leaf `fan_slot_excess` term zeroed, `unpriced_slot_padding_fails_the_pointwise_check` run, restored (diff empty) | control FAILS: only `underpriced walked leaf:` fires, expected substring `underpriced leaf: ` absent |
| `conformance-42` | `0d7a5b84` | `tail -1 proptest-regressions/conformance/backend/tests.txt` | `cc aac7f574… # shrinks to fan = 0, bound = 609969, delta = 176464` |
| `tests-resource-link-window-19`, `-20` | `f8d82b6e`, `aef30fba` | `cargo nextest run -p rumors --all-features --no-capture -E 'binary(window_census)'` | `budget 2097152, divergence 20000: peak 182214, generations 48002+99576, overhead 34636, widest capacity 91`; `budget 0 … widest capacity 1`; `5 tests run: 5 passed`; `grep admitted tests/window_census.rs` empty (T139) |
| `-20` (negative control) | `aef30fba` | `TIGHT_BUDGET` swapped to `64 * 1024`, same command, restored (diff empty) | panics `budget 65536 resolves to the serialization floor at 21024 messages a side`; `4 passed, 1 failed` |
| `tests-resource-link-window-25`, `-28` | `a3e4594b` | `cargo nextest run -p rumors --all-features --no-capture -E 'binary(window_corners)'` | `asymmetric catch-up at budget 0: 8 hops`, `reverse … 8 hops`, `zero budget at 2000 mutual divergence: 523 hops`; `6 tests run: 6 passed` |
| `-28` (negative control) | `aef30fba` | the race awaited before the `join!`, same command, restored (diff empty) | panics `no racing commit landed after the greeting: the left replica holds 8096 messages and the right 8096`; `1 failed` |
| all | `aef30fba` | `just gate` (lane log `gate-aef30fba.log`) | `gate: clean in 229s`; test leg `1831 tests run: 1831 passed, 4 skipped` |

## Fresh-eyes rounds

Two rounds.

**Round 1** (surface correctness with meter validity folded in), against
`969ced5a`. The reviewer traced every `should_panic` string to its
emitter, every knob to the control that needs it, the census lock to
every differencing site, and the hop floor's derivation. Six repairs
landed in `8ab9af2d`:

- the parent-presence check was two-sided with a control for one side
  only; the lane verified by instrumentation that no zero-fan group ever
  reached `parent` in the honest suites, then constructed the case
  (`run` presents an empty group at rest; a knob answers it with a
  conjured node) so the check's other direction has a control that fails
  by name;
- a lost line continuation left an eighteen-space run inside the
  `underpriced leaf` message, so the padding control was matching the
  walked-leaf site instead of the constructed-leaf site its testdoc
  names; literal fixed, control pinned to the `underpriced leaf: `
  prefix;
- `TIGHT_BUDGET`'s doc claimed an enforcement the code lacked;
- the honest slot padding was derived from the checker's own formula;
  it is now pinned against hand-written 64-bit layout numbers (the
  reviewer's arithmetic was eight bytes off on the listing slot, which
  the pin caught by failing to compile: the reference slot is 65 bytes,
  not 73);
- one doc sentence said a priced gap's endpoints "still ascend" when
  they price equal;
- the completeness message printed the constant union rather than the
  computed one.

The round's first finding, the dead peak ceiling, became stop 1 and was
ruled (T139, `aef30fba`).

**Round 2** (operational validity and interaction with the tree),
against `aef30fba`: no defect in the change. The
reviewer re-checked each round-1 repair against the head (each closes
its finding; the layout pin's `REFERENCE_SLOT_BYTES = 65` matches an
independent decomposition), traced that every by-name control collects
every ledger violation into one panic so reordering or renaming a check
cannot mask a name, confirmed the conformance session never reaches the
codec's raw asserts (faults surface as `Error::Violation` into `fail`),
checked every reader of the widened constants and helpers across
`src`, `tests`, the justfile, and the docs, and confirmed the T139
retirement leaves nothing dangling. Four prose repairs landed as the
final commit: a stale annotation row, measured widths quoted in
constant docs restated as the properties the floors assert, the premise
behind the census's checked subtraction stated at `overhead`, and the
retag control's doc naming both paths by which its name fires. The
rounds stopped here: the findings had shifted from "this is wrong" to
"you might consider".

Reviewer notes not acted on: several floors (`window_granted > 1`,
`measured >= 2`, the growth witness) have their negative controls only
as reversible mutations recorded in commit messages and re-run by the
coordinator, not as committed tests; the coordinator judged
session-level `should_panic` controls for arithmetic floors not worth
their cost, and records the mutations in the acceptance table instead.
Observed capacities quoted in constant docs ("a widest capacity of
four") are prose calibration, not pins. The conformance ceiling is
non-vacuous but loose (measured admittance is a small fraction of the
budget), noted for a later round.

## Stops

1. **Ruled: T139.** The window census's peak-differencing ceiling
   compared two identical peaks at every budget (the local backend
   retains every decoded handle, so the peak instant is the commit join
   regardless of window); retired as decoration, the conformance census
   owning the admittance claim. Landed in `aef30fba`.

2. **Judgment call for your ruling: `Materializing`'s definition
   changed.** `conformance-40` asks that the honest `Materializing`
   suite "pass unchanged" once slot padding is priced. The lane priced
   the padding by giving `MaterializedNode` a sixteen-byte alignment
   (`#[repr(align(16))]`) and routing the derived excess through a knob,
   rather than adding a fourth backend type whose census would cost
   about 0.7 GiB per run. The honest test passes unchanged; the type it
   tests did not. Consequence the reviewer names: no committed backend
   other than `Local` now witnesses zero excess for a pointer-aligned
   handle, and `Local`'s excess is zero by construction. Annotated at
   `src/conformance/backend/tests.rs:367`. Recommendation: accept; the
   entry's purpose (a control that fails when padding is unpriced) is
   met, and the alternative buys a witness of a case the type system
   already makes trivial.

## Reading order

### deviations from a stated resolution

- tests-resource-link-window-19 (T18) at `tests/window_census.rs:111` ([hunk](#hunk-48))

### new tests and negative controls

- conformance-28 (T6) at `src/conformance/backend.rs:857` ([hunk](#hunk-20))
- conformance-28 (T6) at `src/conformance/backend/tests.rs:181` ([hunk](#hunk-28))
- conformance-24 (T6) at `src/conformance/backend/tests.rs:236` ([hunk](#hunk-30))
- conformance-24 (T6) at `src/conformance/backend/tests.rs:768` ([hunk](#hunk-42))
- conformance-24 (T6) at `src/conformance/backend/tests.rs:788` ([hunk](#hunk-42))
- conformance-24 (T6) at `src/conformance/backend/tests.rs:799` ([hunk](#hunk-42))
- conformance-24 (T6) at `src/conformance/backend/tests.rs:822` ([hunk](#hunk-42))
- conformance-25 (T6) at `src/conformance/backend/tests.rs:811` ([hunk](#hunk-42))
- conformance-31 (T6) at `src/conformance/backend/tests.rs:835` ([hunk](#hunk-42))
- conformance-31 (T6) at `src/conformance/backend/tests.rs:851` ([hunk](#hunk-42))
- conformance-30 (T6) at `src/conformance/backend/tests.rs:865` ([hunk](#hunk-42))
- conformance-40 (T6) at `src/conformance/backend/tests.rs:902` ([hunk](#hunk-43))
- conformance-41 (T6) at `src/conformance/backend/tests.rs:912` ([hunk](#hunk-43))
- conformance-42 (T6) at `src/conformance/backend/tests.rs:1069` ([hunk](#hunk-44))
- tests-resource-link-window-19 (T18) at `tests/window_census.rs:40` ([hunk](#hunk-47))
- tests-resource-link-window-20 (T26) at `tests/window_census.rs:175` ([hunk](#hunk-48))
- tests-resource-link-window-28 (T28) at `tests/window_corners.rs:227` ([hunk](#hunk-56))

### production edits

- conformance-25 (T6) at `src/conformance/backend.rs:14` ([hunk](#hunk-3))
- conformance-40 (T6) at `src/conformance/backend.rs:62` ([hunk](#hunk-4))
- conformance-24 (T6) at `src/conformance/backend.rs:73` ([hunk](#hunk-5))
- conformance-40 (T6) at `src/conformance/backend.rs:115` ([hunk](#hunk-6))
- conformance-40 (T6) at `src/conformance/backend.rs:128` ([hunk](#hunk-6))
- conformance-40 (T6) at `src/conformance/backend.rs:144` ([hunk](#hunk-6))
- conformance-41 (T6) at `src/conformance/backend.rs:103` ([hunk](#hunk-6))
- conformance-24 (T6) at `src/conformance/backend.rs:204` ([hunk](#hunk-7))
- conformance-40 (T6) at `src/conformance/backend.rs:346` ([hunk](#hunk-8))
- conformance-41 (T6) at `src/conformance/backend.rs:382` ([hunk](#hunk-9))
- conformance-24 (T6) at `src/conformance/backend.rs:435` ([hunk](#hunk-10))
- conformance-25 (T6) at `src/conformance/backend.rs:495` ([hunk](#hunk-11))
- conformance-24 (T6) at `src/conformance/backend.rs:534` ([hunk](#hunk-12))
- conformance-24 (T6) at `src/conformance/backend.rs:542` ([hunk](#hunk-12))
- conformance-24 (T6) at `src/conformance/backend.rs:566` ([hunk](#hunk-13))
- conformance-24 (T6) at `src/conformance/backend.rs:617` ([hunk](#hunk-14))
- conformance-42 (T6) at `src/conformance/backend.rs:714` ([hunk](#hunk-15))
- conformance-42 (T6) at `src/conformance/backend.rs:749` ([hunk](#hunk-16))
- conformance-31 (T6) at `src/conformance/backend.rs:797` ([hunk](#hunk-17))
- conformance-28 (T6) at `src/conformance/backend.rs:822` ([hunk](#hunk-18))
- conformance-24 (T6) at `src/conformance/backend.rs:821` ([hunk](#hunk-18))
- conformance-28 (T6) at `src/conformance/backend.rs:835` ([hunk](#hunk-19))
- conformance-28 (T6) at `src/conformance/backend.rs:862` ([hunk](#hunk-20))
- conformance-28 (T6) at `src/conformance/backend.rs:868` ([hunk](#hunk-20))
- conformance-30 (T6) at `src/conformance/backend.rs:883` ([hunk](#hunk-21))
- conformance-24 (T6) at `src/conformance/backend.rs:918` ([hunk](#hunk-22))
- conformance-24 (T6) at `src/conformance/backend.rs:965` ([hunk](#hunk-22))
- conformance-31 (T6) at `src/conformance/backend.rs:927` ([hunk](#hunk-22))
- conformance-30 (T6) at `src/conformance/backend.rs:977` ([hunk](#hunk-22))
- conformance-30 (T6) at `src/conformance/backend.rs:978` ([hunk](#hunk-22))
- conformance-24 (T6) at `src/conformance/backend.rs:994` ([hunk](#hunk-23))
- conformance-24 (T6) at `src/conformance/backend.rs:1080` ([hunk](#hunk-24))
- conformance-31 (T6) at `src/conformance/backend.rs:1139` ([hunk](#hunk-25))
- conformance-40 (T6) at `src/tree/mirror/streaming/window.rs:154` ([hunk](#hunk-45))
- conformance-40 (T6) at `src/tree/mirror/streaming/window.rs:175` ([hunk](#hunk-46))

### tests and prose

- conformance-42 (T6) at `proptest-regressions/conformance/backend/tests.txt:7` ([hunk](#hunk-2))
- conformance-42 (T6) at `src/conformance/backend/tests.rs:11` ([hunk](#hunk-26))
- conformance-24 (T6) at `src/conformance/backend/tests.rs:103` ([hunk](#hunk-27))
- conformance-28 (T6) at `src/conformance/backend/tests.rs:158` ([hunk](#hunk-28))
- conformance-28 (T6) at `src/conformance/backend/tests.rs:164` ([hunk](#hunk-28))
- conformance-41 (T6) at `src/conformance/backend/tests.rs:142` ([hunk](#hunk-28))
- conformance-28 (T6) at `src/conformance/backend/tests.rs:155` ([hunk](#hunk-28))
- conformance-40 (T6) at `src/conformance/backend/tests.rs:195` ([hunk](#hunk-29))
- conformance-24 (T6) at `src/conformance/backend/tests.rs:232` ([hunk](#hunk-30))
- conformance-24 (T6) at `src/conformance/backend/tests.rs:219` ([hunk](#hunk-30))
- conformance-25 (T6) at `src/conformance/backend/tests.rs:228` ([hunk](#hunk-30))
- conformance-24 (T6) at `src/conformance/backend/tests.rs:247` ([hunk](#hunk-31))
- conformance-31 (T6) at `src/conformance/backend/tests.rs:251` ([hunk](#hunk-31))
- conformance-40 (T6) at `src/conformance/backend/tests.rs:320` ([hunk](#hunk-32))
- conformance-41 (T6) at `src/conformance/backend/tests.rs:291` ([hunk](#hunk-32))
- conformance-42 (T6) at `src/conformance/backend/tests.rs:273` ([hunk](#hunk-32))
- conformance-42 (T6) at `src/conformance/backend/tests.rs:278` ([hunk](#hunk-32))
- conformance-40 (T6) at `src/conformance/backend/tests.rs:337` ([hunk](#hunk-32))
- conformance-42 (T6) at `src/conformance/backend/tests.rs:304` ([hunk](#hunk-32))
- conformance-28 (T6) at `src/conformance/backend/tests.rs:355` ([hunk](#hunk-33))
- conformance-40 (T6) at `src/conformance/backend/tests.rs:369` ([hunk](#hunk-33))
- conformance-28 (T6) at `src/conformance/backend/tests.rs:359` ([hunk](#hunk-33))
- conformance-41 (T6) at `src/conformance/backend/tests.rs:462` ([hunk](#hunk-34))
- conformance-41 (T6) at `src/conformance/backend/tests.rs:474` ([hunk](#hunk-35))
- conformance-42 (T6) at `src/conformance/backend/tests.rs:490` ([hunk](#hunk-36))
- conformance-24 (T6) at `src/conformance/backend/tests.rs:523` ([hunk](#hunk-37))
- conformance-25 (T6) at `src/conformance/backend/tests.rs:558` ([hunk](#hunk-38))
- conformance-24 (T6) at `src/conformance/backend/tests.rs:592` ([hunk](#hunk-39))
- conformance-31 (T6) at `src/conformance/backend/tests.rs:610` ([hunk](#hunk-40))
- conformance-24 (T6) at `src/conformance/backend/tests.rs:640` ([hunk](#hunk-41))
- conformance-31 (T6) at `src/conformance/backend/tests.rs:848` ([hunk](#hunk-42))
- conformance-40 (T6) at `src/conformance/backend/tests.rs:901` ([hunk](#hunk-43))
- conformance-42 (T6) at `src/conformance/backend/tests.rs:1036` ([hunk](#hunk-44))
- conformance-42 (T6) at `src/conformance/backend/tests.rs:1093` ([hunk](#hunk-44))
- tests-resource-link-window-19 (T18) at `tests/window_census.rs:19` ([hunk](#hunk-47))
- tests-resource-link-window-20 (T26) at `tests/window_census.rs:60` ([hunk](#hunk-47))
- tests-resource-link-window-19 (T18) at `tests/window_census.rs:23` ([hunk](#hunk-47))
- tests-resource-link-window-20 (T139) at `tests/window_census.rs:11` ([hunk](#hunk-47))
- tests-resource-link-window-20 (T139) at `tests/window_census.rs:60` ([hunk](#hunk-47))
- tests-resource-link-window-20 (T139) at `tests/window_census.rs:59` ([hunk](#hunk-47))
- tests-resource-link-window-20 (T26) at `tests/window_census.rs:102` ([hunk](#hunk-48))
- tests-resource-link-window-20 (T26) at `tests/window_census.rs:137` ([hunk](#hunk-48))
- tests-resource-link-window-20 (T26) at `tests/window_census.rs:142` ([hunk](#hunk-48))
- tests-resource-link-window-20 (T26) at `tests/window_census.rs:157` ([hunk](#hunk-48))
- tests-resource-link-window-20 (T26) at `tests/window_census.rs:199` ([hunk](#hunk-48))
- tests-resource-link-window-19 (T18) at `tests/window_census.rs:192` ([hunk](#hunk-48))
- tests-resource-link-window-20 (T139) at `tests/window_census.rs:191` ([hunk](#hunk-48))
- tests-resource-link-window-20 (T139) at `tests/window_census.rs:152` ([hunk](#hunk-48))
- tests-resource-link-window-19 (T18) at `tests/window_census.rs:125` ([hunk](#hunk-48))
- tests-resource-link-window-19 (T18) at `tests/window_census.rs:221` ([hunk](#hunk-49))
- tests-resource-link-window-19 (T18) at `tests/window_census.rs:289` ([hunk](#hunk-50))
- tests-resource-link-window-19 (T18) at `tests/window_census.rs:347` ([hunk](#hunk-51))
- tests-resource-link-window-20 (T26) at `tests/window_corners.rs:40` ([hunk](#hunk-52))
- tests-resource-link-window-20 (T26) at `tests/window_corners.rs:39` ([hunk](#hunk-52))
- tests-resource-link-window-25 (T28) at `tests/window_corners.rs:118` ([hunk](#hunk-53))
- tests-resource-link-window-25 (T28) at `tests/window_corners.rs:136` ([hunk](#hunk-54))
- tests-resource-link-window-28 (T28) at `tests/window_corners.rs:196` ([hunk](#hunk-55))
- tests-resource-link-window-28 (T28) at `tests/window_corners.rs:218` ([hunk](#hunk-56))
- tests-resource-link-window-28 (T28) at `tests/window_corners.rs:240` ([hunk](#hunk-57))

## The change

<a id="hunk-1"></a>
### .agent-notes/2026-09-01-holistic-review-rumors/triage/annotations/p1-conformance.tsv `@@ -0,0 +1,114 @@`

```diff
@@ -0,0 +1,114 @@
+# p1-conformance lane annotations: path, line (new side of `git diff 0926fe32...HEAD`), entry id, ruling, note.
+# Written by Claude (Fable 5.1) as the lane's implementer for Finch's review; not authored or endorsed by Finch.
+src/conformance/backend.rs	822	conformance-28	T6	The entry: `check`'s admittance ceiling had no floor, and at 64 KiB the budgeted run was the floor run. The `# Panics` list now names the floor clause, and `check` prints both peaks and both widest capacities so a failure's log carries the diagnosis.
+src/conformance/backend.rs	835	conformance-28	T6	`run` returns the session's `SessionStats` beside the peak (read from a `Recorder` shared with the client side; both sides exchange the same size pair, so both resolve the same window). This is how the floor learns the widest capacity the real session was granted, rather than re-solving from corpus sizes with version bytes guessed.
+src/conformance/backend.rs	857	conformance-28	T6	The liveness floor, first of the two. Premise: a budget that binds widens some stage past the one-scope floor, and a wider stage holds at least one more node value in flight at the session's peak instant than the floor does, so the census must read strictly higher; the minimum is one node value, so the assertion is a strict inequality with no margin. Negative control: at 64 KiB this fails with both peaks at 14496 B (recorded verbatim in 4b8f3c00); at 275248 B the budgeted peak is 14576 B. This premise holds here because the conformance session has no commit join: the peak is the walk's in-flight plus the assembled output. It does not hold in `tests/window_census.rs` (see that file's rows).
+src/conformance/backend.rs	862	conformance-28	T6	The second half of the floor, the resolution's alternative form: the session itself reports a widest capacity above one. It sits after the peak floor so a 64 KiB failure names the peaks, and it separates the two diagnoses (the solve did not widen versus the census did not see a widening). Its message is phrased for the case the peak floor passed and this did not.
+src/conformance/backend.rs	868	conformance-28	T6	Plain subtraction replaces `saturating_sub`: the floor above guarantees the difference is positive, so a saturating read of zero can no longer hide a regression.
+src/conformance/backend/tests.rs	158	conformance-28	T6	`LOCAL_BUDGET`, derived from the term it must clear rather than a literal: the flat decode-fan pre-charge under in-memory pricing plus 64 KiB, the resolution's suggested form. Measured once: at the suite's corpus (1536 a side) it lands a widest capacity of 4 and the census peak moves from 14496 B to 14576 B. Landed after the floor was committed failing at 64 KiB (4b8f3c00), then resized (a29692e9), so the history shows the order the brief requires.
+src/conformance/backend/tests.rs	164	conformance-28	T6	The testdoc restated from the measured behaviour: the budgeted run widens the window past the floor and its peak exceeds the floor run's, which is what the body now asserts. The old text claimed a budget that binds, which was false.
+src/conformance/backend/tests.rs	181	conformance-28	T6	Negative control, committed: `check(Local, SUPPLY_DECODE_ENVELOPE_BYTES)` is a budget spent entirely on the pre-charge, so every capacity floors at one and the peak floor fails by name. I chose the pre-charge itself over the brief's 64 KiB because it is the exact boundary of the premise (monotone: any smaller budget floors too), so the control needs no magic number; the 64 KiB failure is recorded in the commit message instead.
+src/conformance/backend/tests.rs	355	conformance-28	T6	`MATERIALIZING_BUDGET` re-checked against its own flat term through the same floor: at 4 MiB the budgeted run peaks 1033 B above the floor run (193707 B to 194740 B) with a widest capacity of 45, so the floor holds; the doc now states that rather than asserting the budget binds.
+tests/window_census.rs	19	tests-resource-link-window-19	T18	The module doc no longer rests correctness on nextest's process model: every test body holds the census lock, so the suite is correct under any runner's threading.
+tests/window_census.rs	40	tests-resource-link-window-19	T18	`CENSUS_LOCK`, the same shape as decode_alloc's meter lock: the node census is process-global and every test here constructs nodes. The guard clears a poisoned lock because a `should_panic` test poisons it by design and the poison guards nothing.
+tests/window_census.rs	60	tests-resource-link-window-20	T26	`TIGHT_BUDGET` measured, not handed: 2 MiB, a calibration observed once to widen the window at 21024 messages a side (capacities sum 569 over 33 stages, the session reporting a widest capacity of 91), where 64 KiB and the 209712 B pre-charge both floor every capacity at one. The constant's doc states the one property the code holds, widening past the floor at the derived capacities and in the session's report, and says the number is a calibration, not a bound.
+tests/window_census.rs	102	tests-resource-link-window-20	T26	`reconcile` returns both sides' `Gossiped`, so the admittance test can read the window the real session was granted.
+tests/window_census.rs	111	tests-resource-link-window-19	T18	The differencing arithmetic hoisted into one site. I return a small named struct rather than the resolution's `(overhead, after)` tuple because -20 also needs the session's report from the same call; three anonymous fields would be unreadable at the call sites. Not a deviation in substance: one site, both quantities.
+tests/window_census.rs	137	tests-resource-link-window-20	T26	`checked_sub(...).expect(...)` replaces `saturating_sub`: a peak below the two resting generations is a broken measurement and now fails instead of reading zero. This is the one remaining site; the floor test's copy was folded into `overhead`.
+tests/window_census.rs	142	tests-resource-link-window-20	T26	The census log line now carries the widest capacity the session reported, so the log shows the two arms ran different windows (1 versus 91) even where their peaks agree.
+tests/window_census.rs	157	tests-resource-link-window-20	T26	The fixture's own liveness, hoisted so the control below can drive it without a session: the derived capacities must sum past one per stage. Premise: the solve floors every stage at one when the budget is at or below the pre-charge, so a sum equal to the height count means the two arms would run the identical window.
+tests/window_census.rs	175	tests-resource-link-window-20	T26	Negative control, committed and cheap (no session): the fixture at a budget equal to the pre-charge fails the liveness assertion by name. The brief's literal `TIGHT_BUDGET = 64 * 1024` mutation was also run once and restored; it fails at the same assertion in 0.00 s (recorded verbatim in f8d82b6e).
+tests/window_census.rs	199	tests-resource-link-window-20	T26	The real-session floor: `window_granted > 1` from the budgeted arm's `Gossiped`. Premise: a session whose solve widened any stage reports that stage's capacity as its widest, so a report of one means the session ran at the serialization floor whatever the test-side solve said.
+tests/window_census.rs	192	tests-resource-link-window-19	T18	The lock taken at the top of every test body, this one and the four others (the other four bodies likewise); the version-bound tests construct nodes too.
+tests/window_census.rs	347	tests-resource-link-window-19	T18	The floor test now reads through `overhead` instead of spelling the before/reset/reconcile/peak/after arithmetic a second time.
+tests/window_corners.rs	40	tests-resource-link-window-20	T26	`window_corners.rs:174` re-measured with the census: 64 KiB floors every capacity at this population (4048 a side), so the doc's greeting-derived window never existed. `GROWTH_BUDGET` is 512 KiB, past the pre-charge with a widest capacity of 23 here; the doc states the property (clears the pre-charge, widens past the floor) and the test holds the session to it (line 218), so the literal cannot silently go stale.
+tests/window_corners.rs	118	tests-resource-link-window-25	T28	The hop floor. Premise: completion depends on a reply to a delivered message, so no transfer avoids one request and its reply, two causally chained one-way hops; the floor is that minimum, not the 8 hops observed. A `hops` returning 0 fails it by reading; both catch-up tests measure 8.
+tests/window_corners.rs	136	tests-resource-link-window-25	T28	The same floor at the reverse-direction site, same premise.
+tests/window_corners.rs	196	tests-resource-link-window-28	T28	The testdoc restated to what the body now witnesses: the window is derived past the floor, at least one racing commit is shown to have landed after the greeting's snapshot, and the follow-up converges the snapshots.
+tests/window_corners.rs	218	tests-resource-link-window-28	T28	The racing session is held to have widened (`window_granted > 1`), the guard that keeps `GROWTH_BUDGET` honest: if the pre-charge ever grows past it, this fails rather than the doc silently describing a derivation that does not run.
+tests/window_corners.rs	227	tests-resource-link-window-28	T28	The mid-session-growth witness, the resolution's first form: every racing commit is on the left replica, only those before the greeting's snapshot reached the right one, so `left > right` exactly when some commit landed mid-session. Negative control: awaiting `race` to completion before the `join!`, run once and restored, fails here with 8096 = 8096 (recorded verbatim in a3e4594b).
+tests/window_corners.rs	240	tests-resource-link-window-28	T28	The finish is snapshot equality rather than length equality: `Snapshot` already derives `PartialEq`, so finding 27's form fell out of the witness for free, as the brief allowed.
+src/conformance/backend.rs	204	conformance-24	T6	`fail`: every failure path in `run` and its helpers panics through this, appending the violations still pending on the ledger. This is the resolution's surfacing of pending violations in the `converged` message, generalized to the lost-root and short-session failures too, because a swallowed root assembly (conformance-31's control) dies at the corpus builder before `check` reads the ledger.
+src/conformance/backend.rs	435	conformance-24	T6	The presence clause: `parent.is_some() != (fan > 0)` records `parent contract: fan N yielded Some/None`, both directions. Only parent calls made through the charged decorator reach this check; see the row at line 916 for why that matters.
+src/conformance/backend.rs	534	conformance-24	T6	The containment clause of `leaves`: the yielded leaf's height-H prefix must equal the walked prefix. It has content only when the walked prefix is not the root's empty one, which is why `run` also walks from each of the root's children (line 1002).
+src/conformance/backend.rs	542	conformance-24	T6	The order clause of `leaves`: strictly ascending leaf prefixes, tracked against the previous yield inside the same closure that counts and prices.
+src/conformance/backend.rs	617	conformance-24	T6	The order clause of `assemble`: strictly ascending run prefixes. The `BTreeMap` lookup already caught a wrong prefix; it could not see a wrong order.
+src/conformance/backend.rs	918	conformance-24	T6	Judgment call, reported: `run` also folds each corpus through the default `Convert` assembly over the charged backend, first. The reference backend's own `assemble` is the default fold over itself, so its `parent` calls never pass through `Charged::parent`; the `PARENT_DROPS` control was spent there and convicted only as a short run, exactly the witness's finding. Driving the default fold over `Charged` brings every interior group of a resting corpus through the presence, price, and aggregate checks, and makes the by-name conviction the resolution asks for reachable. It is coverage the session alone did not give (the session assembles only disputed groups).
+src/conformance/backend.rs	965	conformance-24	T6	Convergence and completeness fail through `fail`, so a ledger violation recorded before a non-converging or short session is named in the panic rather than masked by it.
+src/conformance/backend/tests.rs	103	conformance-24	T6	`Knob::take`: a decrementing read for knobs that count faults to inject rather than bytes to add. The guard still restores the honest value on drop.
+src/conformance/backend/tests.rs	232	conformance-24	T6	`PARENT_DROPS`: interior parent calls answered with `Ok(None)`, spent one per call.
+src/conformance/backend/tests.rs	219	conformance-24	T6	`LEAVES_SWAP` and (line 219) `WALK_ESCAPES`: the walk yields its first two leaves swapped, or re-tags its first leaf's prefix outside every subtree but the root's (top bit of the first path byte flipped). The escape knob is beyond the resolution's listed knobs; the brief's negative-control rule requires a committed known-bad artifact for the containment check, and without the escape there is none.
+src/conformance/backend/tests.rs	247	conformance-24	T6	`ASSEMBLE_SWAP`: the assembly yields its first two nodes swapped.
+src/conformance/backend/tests.rs	523	conformance-24	T6	The presence fault injected at the reference backend's `parent`, spent on the first interior call.
+src/conformance/backend/tests.rs	592	conformance-24	T6	The escape and (line 560) the swap applied to the reference walk's output; `swapped_head` (line 607) and `escaped` (line 625) are the two small adaptors, shared with the assembly's swap.
+src/conformance/backend/tests.rs	768	conformance-24	T6	Negative control: the presence check fires by name. Its doc explains why the knob is spent by the default fold over the decorator (line 916 of backend.rs).
+src/conformance/backend/tests.rs	788	conformance-24	T6	Negative control: the walk's order check fires by name; count and prices are unchanged by a swap, so nothing else would.
+src/conformance/backend/tests.rs	799	conformance-24	T6	Negative control: the walk's containment check fires by name, at the sub-root walk (every leaf is inside the root's empty prefix).
+src/conformance/backend/tests.rs	822	conformance-24	T6	Negative control: the assembly's order check fires by name, in the multi-run regime (the root assembly yields one node, so it has nothing to swap).
+src/conformance/backend.rs	495	conformance-25	T6	`children` priced pointwise at the widest fan a child can have (`len.min(FAN)`), the monotone cap `check_assembled` argues; `underpriced child` when the measurement (plus reference-slot padding, conformance-40) exceeds it. Restricted to the price: the yielded child's own fan is invisible here.
+src/conformance/backend.rs	14	conformance-25	T6	The module doc's pointwise bullet now includes exploded children and the widest-fan cap; the bulk-seams bullet names the order, containment, and presence clauses; the end-to-end bullet names completeness (conformance-30).
+src/conformance/backend/tests.rs	228	conformance-25	T6	`CHILDREN_SLACK`, applied at line 524 to interior children only: leaf-height children re-enter `Charged::leaves` through the reference walk and are caught there for an incidental reason, as the witness found; the control's subject is the check on exploded interior references.
+src/conformance/backend/tests.rs	811	conformance-25	T6	Negative control: 64 KiB of slack on every exploded interior row fails `underpriced child` by name; the honest suites pass.
+src/conformance/backend.rs	927	conformance-31	T6	`assemble_runs` (line 1075): each corpus's sorted leaves driven through the backend's bulk assembly at the height under the root, one node per one-byte prefix, up to `FAN` runs per corpus, the wire decoder's regime; the multi-run boundary of `Local::assemble` is covered by `local_backend_conforms`. Before the root assembly, so a fault felt at the root is already named. `sorted_leaves` (line 1032) and `corpus` (line 1095) are the old `corpus` split so the same leaves feed all three assemblies.
+src/conformance/backend.rs	1139	conformance-31	T6	The corpus builder's `expect`s fail through `fail`: the swallowed-root case dies here with `unassembled run` in the message instead of masking it, the witness's exact failure.
+src/conformance/backend/tests.rs	251	conformance-31	T6	`ASSEMBLED_DROPS` and (line 247) `ASSEMBLE_RETAGS`: one-based positions of the assembled node to swallow or to re-tag to its neighbor's prefix (`neighbor`, line 633, flips the last prefix byte's low bit; the root prefix has no neighbor and is left alone). Applied at lines 577 and 584.
+src/conformance/backend/tests.rs	835	conformance-31	T6	Negative control: a swallowed assembled node fails `unassembled run` by name, through the sub-root assembly's leftover and, at the root, through the surfaced corpus failure.
+src/conformance/backend/tests.rs	851	conformance-31	T6	Negative control: a re-tagged assembled node fails `unsupplied assembly` by name.
+src/conformance/backend.rs	883	conformance-30	T6	`run` takes the count of shared messages both corpora omit (zero in `check`): the symmetric loss the two sides' agreement cannot see, so the completeness oracle has a committed known-bad artifact. `pub(super)` so the control can drive it. I chose this over wrapping both roots because the loss must be in both corpora before the greeting, and the corpus builder is where that is one line.
+src/conformance/backend.rs	977	conformance-30	T6	The completeness oracle: the reconciled root holds `COMMON + 2 * DIVERGENT` messages, the resolution's first form. Fails on the honest runs never; fails by name on any session that loses a leaf on both sides.
+src/conformance/backend/tests.rs	865	conformance-30	T6	Negative control: one shared message left out of both corpora converges (equal hashes) and fails `converged short` by name.
+src/conformance/backend.rs	115	conformance-40	T6	`fan_slot_excess`: the decode-fan pair's padding for this backend's leaf handle beyond `FAN_SLOT_BYTES`, the window's in-memory figure; saturating, because a handle narrower than a pointer is overcharged, the safe direction. Folded into the leaf comparisons at lines 342 and 548 (`underpriced leaf` and `underpriced walked leaf` now say how much padding they added).
+src/conformance/backend.rs	128	conformance-40	T6	`reference_slot_excess`, per height: the query slot `(u8, node)` and the resolution slot `(u8, Resolve<erased>)` beyond the in-memory padding; the listing slot holds a hash and pads the same for every backend. Folded into the child comparison at line 494, where the per-level references are priced, as the resolution asks once conformance-25 prices them.
+src/conformance/backend.rs	144	conformance-40	T6	The decomposition pinned to the window's own constant by a const assertion: the two slots' padding plus the two handles plus the listing slot equals `REFERENCE_SLOT_BYTES`. A quantity computable two ways gets a committed comparison.
+src/tree/mirror/streaming/window.rs	154	conformance-40	T6	`REFERENCE_SLOT_BYTES` and (line 175) `FAN_SLOT_BYTES` become `pub(crate)` so the suite can subtract them; crate-internal, inside the ruling.
+src/conformance/backend/tests.rs	369	conformance-40	T6	Judgment call, reported: the reference backend's node is made `#[repr(align(16))]` and its price gains the derived padding (`PRICED_SLOT_PADDING`, honest at `SLOT_PADDING`, line 303, the larger of the fan-slot and reference-slot excess), rather than adding a fourth backend type for the control. The resolution asks for a `#[repr(align(16))]` wrapper node whose `node_bytes` omits the padding, asserted to fail by name; `MaterializedNode` is that wrapper node, and the knob is how it omits the padding. A new backend type would instantiate the whole protocol tower again (documented at +0.7 GiB of rustc memory) or exercise only leaf construction; this way the honest run exercises the excess at every check. `Local` is untouched and `Materializing`'s honest test passes unchanged; its definition changed, which is the part to rule on if the acceptance's 'unchanged' meant the definition.
+src/conformance/backend/tests.rs	320	conformance-40	T6	A const assertion that the derived padding is positive, so the control cannot pass vacuously if the alignment attribute is ever dropped.
+src/conformance/backend/tests.rs	902	conformance-40	T6	Negative control: the padding priced at zero fails `underpriced leaf` at the first leaf, the message naming the decode-slot padding.
+src/conformance/backend.rs	103	conformance-41	T6	`Measure::measure_erased`, the oracle for the erase direction, as the amendment directs; the trait is crate-internal.
+src/conformance/backend.rs	382	conformance-41	T6	`erase` and `assume` measure the re-tagged handle, record `re-tagged node changed residency` when it differs from the carried bytes, and charge what they measure. The comment states the trait clause the untouched census peak rests on, and that it is checked here rather than relied on.
+src/conformance/backend/tests.rs	142	conformance-41	T6	`measure_erased` for `Local` (and line 600 for `Materializing`): the same shape as `measure` over the erased handle.
+src/conformance/backend/tests.rs	291	conformance-41	T6	`ASSUME_SLACK` and (line 287) `ERASE_SLACK`, applied at lines 448 and 439: the row grows across each re-tag. The erase knob is beyond the resolution's named `ASSUME_SLACK`; the erase direction's check needs its own known-bad artifact.
+src/conformance/backend/tests.rs	912	conformance-41	T6	Negative controls, one per direction (line 866 for erase): each fails by name with the direction in the message.
+src/conformance/backend.rs	714	conformance-42	T6	`BOUND_DENSE_CEILING` raised from 64 to 4096: pure arithmetic, so dense costs nothing, and every bound a session at the suite's scale can evaluate now has grid neighbors. The doc says what the dense sweep covers and why it is cheap.
+src/conformance/backend.rs	749	conformance-42	T6	`node_bytes_monotone`'s doc states that the bound dimension is a sample and that the family is held by the property test in this module's tests.
+src/conformance/backend/tests.rs	273	conformance-42	T6	`PRICED_BOUND_STEP`: the header shrinks by `STEP_DROP` above a threshold. `STEP_THRESHOLD` (line 293) and `STEP_DROP` (line 297) derive from `BOUND_SWEEP_CEILING`: the threshold is the midpoint of the grid's widest gap (between the neighbors of 2^19 and 2^20) so no adjacent grid pair straddles it, and the drop is that gap's width, the largest drop across which the gap's two endpoints still ascend. With those, a uniformly drawn `(bound, delta)` pair straddles the step with probability about one in eight, so the property convicts in 256 cases with certainty for practical purposes even before the seed replays.
+src/conformance/backend/tests.rs	278	conformance-42	T6	`PRICED_BOUND_DIP` and `DIP_BOUND` (101): a one-byte point dip at a bound off every power of two, inside the dense sweep; the acceptance's 'point dip at a non-grid bound below the raised dense ceiling', committed as a control (line 1028) rather than argued.
+src/conformance/backend/tests.rs	1036	conformance-42	T6	The `proptest!` over `(fan, bound, delta)` for both backends, the resolution's strategy verbatim, each case holding the serialization lock. The seed file at proptest-regressions/conformance/backend/tests.txt is committed; `tests/seed_liveness.rs` accepts its location.
+src/conformance/backend/tests.rs	1069	conformance-42	T6	Negative control: the step fails the property by name (the runner's panic carries the assertion's message). The knob is set per case because `Knob::set` takes the lock the honest cases also take.
+src/conformance/backend/tests.rs	1093	conformance-42	T6	The resolution's other half of the demonstration: the same step passes the grid sweep, committed as a passing test, which is what shows the property test is not decoration.
+proptest-regressions/conformance/backend/tests.txt	7	conformance-42	T6	The shrunk failing case the step control wrote on its first run: bound 609969 below the threshold 786432, bound plus delta above it. Committed so the control replays deterministically.
+src/conformance/backend/tests.rs	11	conformance-42	T6	Imports for the property test and the crate-internal items the tests now reach (`FAN`, `WindowConfig`, `run`, the slot-excess functions, the sweep ceiling); private items of the parent module are visible to its tests module, so nothing was widened for them.
+tests/window_census.rs	23	tests-resource-link-window-19	T18	Imports for the lock, the pre-charge accessor, and `Gossiped`.
+src/conformance/backend.rs	62	conformance-40	T6	Imports for `size_of`, `Local`, `Resolve`, the two slot constants, and `typed`, all used by the padding derivation.
+src/conformance/backend.rs	1080	conformance-24	T6	Fresh-eyes repair (item 1): the presence check's `Some`-at-zero-fan direction had no reachable input. Verified by an instrumented run of both honest suites: zero zero-fan groups reach `Charged::parent` (the session prunes only against what the peer knows, so a mixed scope keeps a child, and no resolution here comes back empty). I constructed rather than reduced: `run` presents `parent` with an empty group at rest, the trait's stated case, so the direction has a reachable input and its own control. Called at line 920.
+src/conformance/backend.rs	821	conformance-24	T6	Fresh-eyes repair (item 1): `check`'s `# Panics` lists the empty-group direction.
+src/conformance/backend/tests.rs	236	conformance-24	T6	Fresh-eyes repair (item 1): `PARENT_CONJURES`, applied at line 528: an empty group answered with a node conjured from a leaf re-tagged at the parent's height (the tag is phantom; the check on trial reads only presence). Negative control at line 776 expects `parent contract: fan 0 yielded Some`.
+src/conformance/backend.rs	346	conformance-40	T6	Fresh-eyes repair (item 2): the `underpriced leaf` literal had lost its line continuation and carried a run of spaces, so the slot-padding control was matching the walked-leaf site instead. Restored the continuation.
+src/conformance/backend/tests.rs	901	conformance-40	T6	Fresh-eyes repair (item 2): the control expects `underpriced leaf: `, a prefix only the constructed-leaf site emits (the walk's site says `underpriced walked leaf`), so dropping `fan_slot_excess` from the constructed-leaf check alone fails it. Observed: 6144 lines of `underpriced leaf: measured N B plus 8 B of decode-slot padding, node_bytes priced N B`.
+src/conformance/backend/tests.rs	337	conformance-40	T6	Fresh-eyes repair (item 4): `SLOT_PADDING` pinned against an independent hand-written layout, gated on a 64-bit pointer. The reviewer's arithmetic gave `REFERENCE_SLOT_BYTES = 73` with the listing slot at 33 bytes; measured, `Hash` is 24 bytes so the listing slot is 25 and the constant is 65 (the pin at 73 failed at compile time, which is the pin working). Fan pair 48 to 80, query slot 16 to 48, resolution slot 24 to 48 with 16 of padding either way, padding 8: all as the reviewer computed.
+src/conformance/backend/tests.rs	304	conformance-42	T6	Fresh-eyes repair (item 5): the gap's endpoints price equal across `STEP_DROP`, not ascending; the sentence now says so and names the non-strict comparison that accepts it.
+src/conformance/backend.rs	978	conformance-30	T6	Fresh-eyes repair (item 6): the completeness message no longer calls the constant the corpora's union (false by `omitted` in the control run); it says what the corpora are built to hold.
+tests/window_census.rs	11	tests-resource-link-window-20	T139	Ruling T139: the module doc names the backend conformance suite's census as the owner of the window's byte-admittance claim and states what this census pins instead (content overhead at the floor, the version-bound claims, and that a tight budget widens the session's window).
+tests/window_census.rs	60	tests-resource-link-window-20	T139	Ruling T139 (and fresh-eyes item 3): `TIGHT_BUDGET`'s doc states the one property the code holds, widening past the floor at the derived capacities and in the session's report; the 'far below what admits the whole divergence' clause is gone with the ceiling it described.
+tests/window_census.rs	191	tests-resource-link-window-20	T139	Ruling T139: the admittance test is now the widening test. Deleted as decoration: the `admitted` ceiling, the floor arm it differenced against, and `HANDLES_PER_SCOPE` and `ASSEMBLY_FAN_HANDLES`, which fed only it; `TRANSIENT_SLACK` and `overhead` stay for the content-overhead pin. The doc says what the body asserts and why no peak is differenced here (this census's peak is the commit join at every budget). Renamed for what it measures.
+tests/window_census.rs	152	tests-resource-link-window-20	T139	Ruling T139: the fixture-liveness doc and message speak of the budgeted session rather than differenced arms.
+src/conformance/backend/tests.rs	155	conformance-28	T6	Fresh-eyes round 2 (item 2): `LOCAL_BUDGET`'s doc no longer quotes an observed width (the widest capacity of four); it states what the floor asserts, a window wider than the floor's and a moved census peak.
+src/conformance/backend/tests.rs	359	conformance-28	T6	Fresh-eyes round 2 (item 2): `MATERIALIZING_BUDGET`'s doc drops 'tens of scopes wide' for the property the floor holds.
+src/conformance/backend/tests.rs	848	conformance-31	T6	Fresh-eyes round 2 (item 4): the retag control's doc covers both cases, an absent neighbor (the re-tagged node is the unsupplied one) and a neighbor with a run (the re-tagged node consumes it and is convicted as a length mismatch; the honest neighbor that follows is the unsupplied one), which is the case at this corpus.
+tests/window_census.rs	59	tests-resource-link-window-20	T139	Fresh-eyes round 2 (item 2): `TIGHT_BUDGET`'s doc drops 'between a dozen and a hundred' and says the 2 MiB is an observed calibration, not a bound; the floors pin no width.
+tests/window_census.rs	125	tests-resource-link-window-19	T18	Fresh-eyes round 2 (item 3): `overhead`'s doc states the premise its checked subtraction rests on (both old generations alive when the last side commits) and that a session breaking it fails the instrument visibly rather than vacuously.
+tests/window_corners.rs	39	tests-resource-link-window-20	T26	Fresh-eyes round 2 (item 2): `GROWTH_BUDGET`'s doc drops 'a few dozen scopes wide' for the property the guard asserts.
+src/conformance/backend.rs	73	conformance-24	T6	The `use` block: `Local`, `Resolve`, and the two slot constants for the padding derivation (conformance-40), `Recorder` and `SessionStats` for the widened-window floor (conformance-28), and `typed` for the in-memory layout the reference padding is measured against. Nothing else in the block moved.
+src/conformance/backend.rs	566	conformance-24	T6	The walk closure's yielded pair now uses the leaf's own prefix name (`leaf_prefix`), because the walked prefix `prefix` stays in scope for the containment check a few lines up; a rename, no behaviour change.
+src/conformance/backend.rs	797	conformance-31	T6	`check`'s doc restated to list the drives `run` now performs before the session: the default fold, the sub-root and root assemblies, and the root and per-child walks.
+src/conformance/backend.rs	994	conformance-24	T6	`walk` fails through `fail` so a pending violation is named; `walk_children` (new) explodes the root through `Charged::children` and walks each child at its own prefix, the drive that gives the containment clause content (conformance-24) and prices every exploded child (conformance-25); `corpus` is split into `sorted_leaves` here and the root assembly below (conformance-31).
+src/conformance/backend/tests.rs	195	conformance-40	T6	`Materializing`'s doc gains the paragraph on its wider-than-pointer node: the slot padding is the backend's to price and `PRICED_SLOT_PADDING` prices it honestly at rest. Doc only.
+src/conformance/backend/tests.rs	462	conformance-41	T6	`erase` grows the row by `ERASE_SLACK` across the re-tag, the erase-direction fault; the comment names both re-tag knobs.
+src/conformance/backend/tests.rs	474	conformance-41	T6	`assume` grows the row by `ASSUME_SLACK` across the re-tag, the resolution's named fault.
+src/conformance/backend/tests.rs	490	conformance-42	T6	`node_bytes` prices the slot padding through `PRICED_SLOT_PADDING` (conformance-40) and applies the two bound-lying faults after the fan dip: the point dip at `PRICED_BOUND_DIP` (two bytes, so the adjacent-bound comparison sees a strict fall) and the step above `PRICED_BOUND_STEP`; the comment says which check each is invisible to.
+src/conformance/backend/tests.rs	558	conformance-25	T6	`children` inflates interior rows only by `CHILDREN_SLACK`; the comment states why leaf-height children stay honest (they re-enter the walk's own leaf check through `leaves`).
+src/conformance/backend/tests.rs	610	conformance-31	T6	`assemble` composes the assembly faults on the default fold's output: the swallow (`ASSEMBLED_DROPS`, a positional `filter_map`), the re-tag to a neighbor (`ASSEMBLE_RETAGS`), and the head swap (`ASSEMBLE_SWAP`), beside the existing skip and slack; the comment lists all five.
+src/conformance/backend/tests.rs	640	conformance-24	T6	`measure_erased` for `Materializing` (conformance-41), and the three helpers the faults share: `swapped_head` (the head-swap adaptor for the walk and the assembly), `escaped` (a leaf prefix moved out of every subtree but the root's), and `neighbor` (a prefix's sibling one bit over, the root left alone).
+src/tree/mirror/streaming/window.rs	175	conformance-40	T6	`FAN_SLOT_BYTES` becomes `pub(crate)` so the suite can subtract it from the real pair's padding; crate-internal, inside the ruling.
+tests/window_census.rs	221	tests-resource-link-window-19	T18	The census lock taken at the top of the version-bound test's body; it constructs nodes too.
+tests/window_census.rs	289	tests-resource-link-window-19	T18	The census lock taken at the top of the wide-frontier test's body; it constructs nodes too.
```

*(review record; no annotation expected)*

<a id="hunk-2"></a>
### proptest-regressions/conformance/backend/tests.txt `@@ -0,0 +1,7 @@`

```diff
@@ -0,0 +1,7 @@
+# Seeds for failure cases proptest has generated in the past. It is
+# automatically read and these particular cases re-run before any
+# novel cases are generated.
+#
+# It is recommended to check this file in to source control so that
+# everyone who runs the test benefits from these saved cases.
+cc aac7f574d1db3722ce6e1b8e514ae6ea5f95b1bf786cf116225dd147aeacd2f1 # shrinks to fan = 0, bound = 609969, delta = 176464
```

<!-- annotation -->
> **conformance-42** (T6), line 7:
>
> The shrunk failing case the step control wrote on its first run: bound 609969 below the threshold 786432, bound plus delta above it. Committed so the control replays deterministically.

<a id="hunk-3"></a>
### src/conformance/backend.rs `@@ -11,17 +11,29 @@`

```diff
@@ -11,17 +11,29 @@
 //!   arguments over a fan and version-bound grid before any session
 //!   runs — the property that keeps the window's quantile evaluation an
 //!   upper bound.
-//! - **Pointwise**: every node the session assembles is measured (via
-//!   [`Measure`]) against the cost function at that node's actual fan and
-//!   version bounds. An underpriced node fails the run by name.
+//! - **Pointwise**: every node the session constructs, assembles, walks,
+//!   or explodes is measured (via [`Measure`]) against the cost function
+//!   at that node's actual fan and version bounds, or, where the fan
+//!   is invisible, at the widest fan the node can have, which
+//!   monotonicity makes an upper bound on the price at its own. The
+//!   measurement carries the slot padding the window's own constants
+//!   leave to the backend (a node aligned wider than a pointer pads the
+//!   decode-fan and reference slots it sits in), and a node re-tagged
+//!   across heights is re-measured. An underpriced node fails the run
+//!   by name.
 //! - **Bulk seams**: the backend's own [`leaves`](Backend::leaves) and
 //!   [`assemble`](Backend::assemble) overrides — the paths the wire codec
-//!   runs — are delegated to, their yields priced on the same census and
-//!   held to the walked or assembled node's aggregates.
+//!   runs — are delegated to, their yields priced on the same census,
+//!   held to the walked or assembled node's aggregates, and held to the
+//!   clauses the trait states for them: a walk stays inside the walked
+//!   prefix and ascends, an assembly yields one node per run in run
+//!   order, and [`parent`](Backend::parent) answers a real child with a
+//!   parent and an empty group with none.
 //! - **End to end**: identical divergent corpora reconcile once at the
 //!   zero-budget floor and once under a stated budget, with every live
 //!   node value's measured bytes on a census ledger. The peak difference
-//!   — the bytes the *window* itself admitted — must fit the budget.
+//!   — the bytes the *window* itself admitted — must fit the budget, and
+//!   the reconciled root must hold the corpora's whole union.
 //!
 //! # Accounting premises
 //!
```

<!-- annotation -->
> **conformance-25** (T6), line 14:
>
> The module doc's pointwise bullet now includes exploded children and the widest-fan cap; the bulk-seams bullet names the order, containment, and presence clauses; the end-to-end bullet names completeness (conformance-30).

<a id="hunk-4"></a>
### src/conformance/backend.rs `@@ -47,6 +59,7 @@`

```diff
@@ -47,6 +59,7 @@
 //! the [`Link`](crate::link::Link) boundary.
 
 use std::collections::BTreeMap;
+use std::mem::size_of;
 use std::pin::pin;
 use std::sync::atomic::{AtomicUsize, Ordering};
 use std::sync::{Arc, Mutex};
```

<!-- annotation -->
> **conformance-40** (T6), line 62:
>
> Imports for `size_of`, `Local`, `Resolve`, the two slot constants, and `typed`, all used by the padding derivation.

<a id="hunk-5"></a>
### src/conformance/backend.rs `@@ -60,13 +73,14 @@ use crate::{`

```diff
@@ -60,13 +73,14 @@ use crate::{
     message::Message,
     tree::{
         mirror::streaming::{
-            self, Backend, BoxNodeStream, ErasedNode, Leaf, Node, NodeStream, Root,
+            self, Backend, BoxNodeStream, ErasedNode, Leaf, Local, Node, NodeStream, Root,
             convert::Convert,
-            materialized,
-            window::{FAN, WindowConfig},
+            materialized::{self, Resolve},
+            stats::{Recorder, SessionStats},
+            window::{FAN, FAN_SLOT_BYTES, REFERENCE_SLOT_BYTES, WindowConfig},
         },
         typed::{
-            Hash, Path, Prefix,
+            self, Hash, Path, Prefix,
             height::{self, Height, S, Z},
         },
     },
```

<!-- annotation -->
> **conformance-24** (T6), line 73:
>
> The `use` block: `Local`, `Resolve`, and the two slot constants for the padding derivation (conformance-40), `Recorder` and `SessionStats` for the widened-window floor (conformance-28), and `typed` for the in-memory layout the reference padding is measured against. Nothing else in the block moved.

<a id="hunk-6"></a>
### src/conformance/backend.rs `@@ -82,8 +96,60 @@ use crate::{`

```diff
@@ -82,8 +96,60 @@ use crate::{
 pub(crate) trait Measure: Backend<Node<Z>: Leaf> {
     /// The actual resident bytes of one node value, measured.
     fn measure<H: Height>(node: &Self::Node<H>) -> usize;
+
+    /// The actual resident bytes of one height-erased node value,
+    /// measured: the oracle for the re-tag clause in the
+    /// [`erase`](Backend::erase) direction.
+    fn measure_erased(node: &Self::Erased) -> usize;
+}
+
+/// Bytes one decode-fan slot pads around a leaf handle of `B` beyond
+/// what the window charges for the in-memory handle's slot.
+///
+/// The window prices a fan slot at `node_bytes(0, bound)` plus
+/// [`FAN_SLOT_BYTES`], the pair's padding under the pointer-class
+/// handle, and leaves a wider-aligned handle's extra padding to the
+/// backend's own price: this is that extra, the obligation the leaf
+/// checks fold into their comparison. Never negative: a handle narrower
+/// than a pointer is overcharged, which is the safe direction.
+const fn fan_slot_excess<B: Backend<Node<Z>: Leaf>>() -> usize {
+    (size_of::<(Prefix<Z>, B::Node<Z>)>() - size_of::<B::Node<Z>>()).saturating_sub(FAN_SLOT_BYTES)
 }
 
+/// Bytes the per-level reference slots pad around a height-`H` handle
+/// of `B` beyond what the window charges for the in-memory handle's.
+///
+/// A held reference sits in a query slot `(u8, node)` and a resolution
+/// slot `(u8, Resolve<erased>)`; the window prices both at
+/// [`REFERENCE_SLOT_BYTES`], derived for the pointer-class handle, and
+/// leaves a wider layout's extra padding to the backend's price. The
+/// listing slot carries a hash, not a handle, so it pads the same for
+/// every backend.
+const fn reference_slot_excess<B: Backend<Node<Z>: Leaf>, H: Height>() -> usize {
+    let real = (size_of::<(u8, B::Node<H>)>() - size_of::<B::Node<H>>())
+        + (size_of::<(u8, Resolve<B::Erased>)>() - size_of::<B::Erased>());
+    real.saturating_sub(LOCAL_REFERENCE_PADDING)
+}
+
+/// The padding the window's reference-slot constant carries for the
+/// in-memory handle: the query and resolution slots less the handle in
+/// each.
+const LOCAL_REFERENCE_PADDING: usize = (size_of::<(u8, typed::Node<Z>)>()
+    - size_of::<typed::Node<Z>>())
+    + (size_of::<(u8, Resolve<<Local as Backend>::Erased>)>()
+        - size_of::<<Local as Backend>::Erased>());
+
+// The decomposition above is the window constant's own: the two
+// handle-bearing slots' padding, the two handles, and the listing slot.
+const _: () = assert!(
+    LOCAL_REFERENCE_PADDING
+        + size_of::<typed::Node<Z>>()
+        + size_of::<<Local as Backend>::Erased>()
+        + size_of::<(u8, Hash)>()
+        == REFERENCE_SLOT_BYTES,
+    "the reference-slot padding decomposes the window's constant",
+);
+
 /// The process-global byte census the charged decorator maintains.
 mod ledger {
     use super::{AtomicUsize, Mutex, Ordering};
```

<!-- annotation -->
> **conformance-40** (T6), line 115:
>
> `fan_slot_excess`: the decode-fan pair's padding for this backend's leaf handle beyond `FAN_SLOT_BYTES`, the window's in-memory figure; saturating, because a handle narrower than a pointer is overcharged, the safe direction. Folded into the leaf comparisons at lines 342 and 548 (`underpriced leaf` and `underpriced walked leaf` now say how much padding they added).

<!-- annotation -->
> **conformance-40** (T6), line 128:
>
> `reference_slot_excess`, per height: the query slot `(u8, node)` and the resolution slot `(u8, Resolve<erased>)` beyond the in-memory padding; the listing slot holds a hash and pads the same for every backend. Folded into the child comparison at line 494, where the per-level references are priced, as the resolution asks once conformance-25 prices them.

<!-- annotation -->
> **conformance-40** (T6), line 144:
>
> The decomposition pinned to the window's own constant by a const assertion: the two slots' padding plus the two handles plus the listing slot equals `REFERENCE_SLOT_BYTES`. A quantity computable two ways gets a committed comparison.

<!-- annotation -->
> **conformance-41** (T6), line 103:
>
> `Measure::measure_erased`, the oracle for the erase direction, as the amendment directs; the trait is crate-internal.

<a id="hunk-7"></a>
### src/conformance/backend.rs `@@ -129,6 +195,23 @@ mod ledger {`

```diff
@@ -129,6 +195,23 @@ mod ledger {
     }
 }
 
+/// Fail the run for `reason`, with every contract violation still
+/// pending on the ledger appended.
+///
+/// A by-name report is never masked by the failure that followed it: a
+/// backend fault first recorded on the ledger and then felt as a lost
+/// root or a short session names itself in the panic.
+fn fail(reason: &str) -> ! {
+    let pending = ledger::take_violations();
+    if pending.is_empty() {
+        panic!("{reason}");
+    }
+    panic!(
+        "{reason}; contract violations pending:\n{}",
+        pending.join("\n"),
+    );
+}
+
 /// The byte-charging census decorator.
 ///
 /// Delegates every operation to the wrapped backend — the bulk
```

<!-- annotation -->
> **conformance-24** (T6), line 204:
>
> `fail`: every failure path in `run` and its helpers panics through this, appending the violations still pending on the ledger. This is the resolution's surfacing of pending violations in the `converged` message, generalized to the lost-root and short-session failures too, because a swallowed root assembly (conformance-31's control) dies at the corpus builder before `check` reads the ledger.

<a id="hunk-8"></a>
### src/conformance/backend.rs `@@ -253,12 +336,15 @@ where`

```diff
@@ -253,12 +336,15 @@ where
         // The pointwise contract at the leaf seam: after construction has
         // had its chance to persist the payload, the cost function at
         // `children = 0` and the node's own bounds must cover what the
-        // handle keeps resident — this is the price the session budget
-        // charges every decode-fan slot.
+        // handle keeps resident, its decode-fan slot's padding included
+        // — this is the price the session budget charges every
+        // decode-fan slot.
+        let padding = fan_slot_excess::<N::Backend>();
         let priced = <N::Backend as Backend>::node_bytes(0, bound_bytes(&node));
-        if measured > priced {
+        if measured + padding > priced {
             ledger::violation(format!(
-                "underpriced leaf: measured {measured} B, node_bytes priced {priced} B",
+                "underpriced leaf: measured {measured} B plus {padding} B of decode-slot \
+                 padding, node_bytes priced {priced} B",
             ));
         }
         Ok(Self::wrap(node, measured))
```

<!-- annotation -->
> **conformance-40** (T6), line 346:
>
> Fresh-eyes repair (item 2): the `underpriced leaf` literal had lost its line continuation and carried a run of spaces, so the slot-padding control was matching the walked-leaf site instead. Restored the continuation.

<a id="hunk-9"></a>
### src/conformance/backend.rs `@@ -282,18 +368,37 @@ where`

```diff
@@ -282,18 +368,37 @@ where
     type Erased = ChargedNode<B::Erased>;
     type Error = B::Error;
 
-    // Both conversions settle the wrapper's ledger entry and open an
-    // identical one around the re-tagged handle: the running total dips by
-    // one node's bytes between the two calls and never rises, so the
-    // census peak is untouched.
+    // Both conversions settle the wrapper's ledger entry and open one
+    // around the re-tagged handle at its measured size: the running
+    // total dips by one node's bytes between the two calls and never
+    // rises, so the census peak is untouched exactly when the re-tag
+    // leaves residency unchanged. That is the trait's re-tag clause (a
+    // tag forgotten and restored "without changing the value";
+    // `assume::<H>(erase::<H>(node)) == node`), checked here rather than
+    // relied on: a re-tag that changes residency is recorded by name and
+    // charged at what it measures.
     fn erase<H: Height>(node: Self::Node<H>) -> Self::Erased {
         let bytes = node.bytes;
-        ChargedNode::wrap(B::erase(node.into_inner()), bytes)
+        let erased = B::erase(node.into_inner());
+        let measured = B::measure_erased(&erased);
+        if measured != bytes {
+            ledger::violation(format!(
+                "re-tagged node changed residency: typed {bytes} B, erased {measured} B",
+            ));
+        }
+        ChargedNode::wrap(erased, measured)
     }
 
     fn assume<H: Height>(erased: Self::Erased) -> Self::Node<H> {
         let bytes = erased.bytes;
-        ChargedNode::wrap(B::assume(erased.into_inner()), bytes)
+        let node = B::assume::<H>(erased.into_inner());
+        let measured = B::measure::<H>(&node);
+        if measured != bytes {
+            ledger::violation(format!(
+                "re-tagged node changed residency: erased {bytes} B, assumed {measured} B",
+            ));
+        }
+        ChargedNode::wrap(node, measured)
     }
 
     fn node_bytes(children: usize, version_bound: usize) -> usize {
```

<!-- annotation -->
> **conformance-41** (T6), line 382:
>
> `erase` and `assume` measure the re-tagged handle, record `re-tagged node changed residency` when it differs from the carried bytes, and charge what they measure. The comment states the trait clause the untouched census peak rests on, and that it is checked here rather than relied on.

<a id="hunk-10"></a>
### src/conformance/backend.rs `@@ -323,6 +428,14 @@ where`

```diff
@@ -323,6 +428,14 @@ where
             .map(|(radix, child)| (radix, child.map(ChargedNode::into_inner)))
             .collect();
         let parent = self.inner.parent(prefix, children).await?;
+        // The presence contract: at least one real child yields a
+        // parent, and a group with no real child yields none.
+        if parent.is_some() != (fan > 0) {
+            ledger::violation(format!(
+                "parent contract: fan {fan} yielded {}",
+                if parent.is_some() { "Some" } else { "None" },
+            ));
+        }
         Ok(parent.map(|node| {
             let measured = B::measure(&node);
             // The pointwise contract: the cost function evaluated at this
```

<!-- annotation -->
> **conformance-24** (T6), line 435:
>
> The presence clause: `parent.is_some() != (fan > 0)` records `parent contract: fan N yielded Some/None`, both directions. Only parent calls made through the charged decorator reach this check; see the row at line 916 for why that matters.

<a id="hunk-11"></a>
### src/conformance/backend.rs `@@ -370,6 +483,24 @@ where`

```diff
@@ -370,6 +483,24 @@ where
             while let Some(child) = children.next().await {
                 yield child.map(|(prefix, node)| {
                     let measured = B::measure(&node);
+                    // The pointwise contract at the explosion: an exploded
+                    // child is one of the held references the window
+                    // prices per depth, sitting in the reference slots
+                    // whose padding beyond the pointer-class layout is
+                    // the backend's to price. Its own fan is invisible
+                    // here, so the price is taken at the widest fan it
+                    // can have (its leaf count, capped at the radix),
+                    // which the contract's monotonicity in fan makes an
+                    // upper bound on the price at its own fan.
+                    let padding = reference_slot_excess::<B, H>();
+                    let priced = B::node_bytes(node.len().min(FAN), bound_bytes(&node));
+                    if measured + padding > priced {
+                        ledger::violation(format!(
+                            "underpriced child: measured {measured} B plus {padding} B of \
+                             reference-slot padding, node_bytes at the widest fan prices \
+                             {priced} B",
+                        ));
+                    }
                     (prefix, ChargedNode::wrap(node, measured))
                 });
             }
```

<!-- annotation -->
> **conformance-25** (T6), line 495:
>
> `children` priced pointwise at the widest fan a child can have (`len.min(FAN)`), the monotone cap `check_assembled` argues; `underpriced child` when the measurement (plus reference-slot padding, conformance-40) exceeds it. Restricted to the price: the yielded child's own fan is invisible here.

<a id="hunk-12"></a>
### src/conformance/backend.rs `@@ -389,20 +520,38 @@ where`

```diff
@@ -389,20 +520,38 @@ where
         stream! {
             let mut walked = 0usize;
             let mut failed = false;
+            let mut previous: Option<Prefix<Z>> = None;
             let mut leaves = pin!(self.inner.leaves(prefix, node.into_inner()));
             while let Some(leaf) = leaves.next().await {
                 failed |= leaf.is_err();
-                yield leaf.map(|(prefix, leaf)| {
+                yield leaf.map(|(leaf_prefix, leaf)| {
                     walked += 1;
+                    // The containment and order clauses the trait states
+                    // for an override: every yielded prefix extends the
+                    // walked one, and the walk ascends strictly.
+                    if Prefix::<H>::containing(&Path::from(leaf_prefix)) != prefix {
+                        ledger::violation(format!(
+                            "escaped leaf walk: a leaf at {leaf_prefix:?} yielded \
+                             outside the walked prefix {prefix:?}",
+                        ));
+                    }
+                    if let Some(before) = previous.replace(leaf_prefix)
+                        && before >= leaf_prefix
+                    {
+                        ledger::violation(format!(
+                            "unordered leaf walk: {leaf_prefix:?} yielded after {before:?}",
+                        ));
+                    }
                     let measured = B::measure(&leaf);
                     // The pointwise contract at the walk: a walked leaf
                     // is a fan slot the session budget prices at
-                    // `children = 0`.
+                    // `children = 0`, its slot's padding included.
+                    let padding = fan_slot_excess::<B>();
                     let priced = B::node_bytes(0, bound_bytes(&leaf));
-                    if measured > priced {
+                    if measured + padding > priced {
                         ledger::violation(format!(
-                            "underpriced walked leaf: measured {measured} B, \
-                             node_bytes priced {priced} B",
+                            "underpriced walked leaf: measured {measured} B plus {padding} B \
+                             of decode-slot padding, node_bytes priced {priced} B",
                         ));
                     }
                     // Every version bound under the walked node is in its
```

<!-- annotation -->
> **conformance-24** (T6), line 534:
>
> The containment clause of `leaves`: the yielded leaf's height-H prefix must equal the walked prefix. It has content only when the walked prefix is not the root's empty one, which is why `run` also walks from each of the root's children (line 1002).

<!-- annotation -->
> **conformance-24** (T6), line 542:
>
> The order clause of `leaves`: strictly ascending leaf prefixes, tracked against the previous yield inside the same closure that counts and prices.

<a id="hunk-13"></a>
### src/conformance/backend.rs `@@ -414,7 +563,7 @@ where`

```diff
@@ -414,7 +563,7 @@ where
                              the walked node's aggregate answers {aggregate} B",
                         ));
                     }
-                    (prefix, ChargedNode::wrap(leaf, measured))
+                    (leaf_prefix, ChargedNode::wrap(leaf, measured))
                 });
             }
             // The len aggregate is exact, so a completed walk returns it.
```

<!-- annotation -->
> **conformance-24** (T6), line 566:
>
> The walk closure's yielded pair now uses the leaf's own prefix name (`leaf_prefix`), because the walked prefix `prefix` stays in scope for the containment check a few lines up; a rename, no behaviour change.

<a id="hunk-14"></a>
### src/conformance/backend.rs `@@ -454,9 +603,20 @@ where`

```diff
@@ -454,9 +603,20 @@ where
         stream! {
             let mut assembled = pin!(assembled);
             let mut failed = false;
+            let mut previous: Option<Prefix<H>> = None;
             while let Some(item) = assembled.next().await {
                 failed |= item.is_err();
                 yield item.map(|(prefix, node)| {
+                    // The order clause the trait states for an override:
+                    // nodes come one per run, in run order, so their
+                    // prefixes ascend strictly.
+                    if let Some(before) = previous.replace(prefix)
+                        && before >= prefix
+                    {
+                        ledger::violation(format!(
+                            "unordered assembly: a node at {prefix:?} yielded after {before:?}",
+                        ));
+                    }
                     let run = runs
                         .lock()
                         .expect("the run ledger mutex is not poisoned")
```

<!-- annotation -->
> **conformance-24** (T6), line 617:
>
> The order clause of `assemble`: strictly ascending run prefixes. The `BTreeMap` lookup already caught a wrong prefix; it could not see a wrong order.

<a id="hunk-15"></a>
### src/conformance/backend.rs `@@ -545,9 +705,13 @@ where`

```diff
@@ -545,9 +705,13 @@ where
     }
 }
 
-/// Every version bound up to this many bytes is swept pairwise: the
-/// small encodings real sessions exchange, where an off-by-one hides.
-const BOUND_DENSE_CEILING: usize = 64;
+/// Every version bound up to this many bytes is swept pairwise.
+///
+/// Far past the encodings the suite's corpora exchange (tens of bytes),
+/// so a dip at any one bound a session at this scale can evaluate is a
+/// grid point's neighbor. Dense costs nothing here: the cost function
+/// is pure arithmetic.
+const BOUND_DENSE_CEILING: usize = 4096;
 
 /// The sweep's largest version bound: 1 MiB, far past any canonical
 /// encoding the suite's corpus scale reaches, sampled at powers of two.
```

<!-- annotation -->
> **conformance-42** (T6), line 714:
>
> `BOUND_DENSE_CEILING` raised from 64 to 4096: pure arithmetic, so dense costs nothing, and every bound a session at the suite's scale can evaluate now has grid neighbors. The doc says what the dense sweep covers and why it is cheap.

<a id="hunk-16"></a>
### src/conformance/backend.rs `@@ -581,6 +745,12 @@ fn sweep_bounds() -> Vec<usize> {`

```diff
@@ -581,6 +745,12 @@ fn sweep_bounds() -> Vec<usize> {
 /// out of release, so the suite sweeps the grid: every adjacent fan pair
 /// up to the radix ([`FAN`]), crossed with [`sweep_bounds`]'s version
 /// bounds, and every adjacent bound pair at each swept fan.
+///
+/// The fan dimension is exhaustive; the bound dimension is a sample of
+/// the family, dense where sessions evaluate and sparse above. A dip
+/// that begins and recovers strictly between two sparse grid points
+/// passes the sweep; the family itself (any bound, any increase) is
+/// held by a property test over each backend in this module's tests.
 fn node_bytes_monotone<B>()
 where
     B: Measure + Clone,
```

<!-- annotation -->
> **conformance-42** (T6), line 749:
>
> `node_bytes_monotone`'s doc states that the bound dimension is a sample and that the family is held by the property test in this module's tests.

<a id="hunk-17"></a>
### src/conformance/backend.rs `@@ -624,8 +794,12 @@ const DIVERGENT: usize = 1_024;`

```diff
@@ -624,8 +794,12 @@ const DIVERGENT: usize = 1_024;
 ///
 /// Sweeps the cost function for monotonicity, then builds two corpora
 /// sharing [`COMMON`] messages and diverging by [`DIVERGENT`] more on
-/// each side, walks each corpus through the bulk leaf seam, and
-/// reconciles them twice: once at the zero-budget floor, once under
+/// each side, drives each corpus's sorted leaves through the default
+/// fold and through the bulk assembly at a sub-root height (many runs)
+/// and at the root (one),
+/// walks each corpus through the bulk leaf walk at the root and, after
+/// exploding the root, at each child's own prefix, and reconciles the
+/// corpora twice: once at the zero-budget floor, once under
 /// `budget_bytes`.
 ///
 /// Checks in one process must not overlap: the census ledger is
```

<!-- annotation -->
> **conformance-31** (T6), line 797:
>
> `check`'s doc restated to list the drives `run` now performs before the session: the default fold, the sub-root and root assemblies, and the root and per-child walks.

<a id="hunk-18"></a>
### src/conformance/backend.rs `@@ -639,10 +813,18 @@ const DIVERGENT: usize = 1_024;`

```diff
@@ -639,10 +813,18 @@ const DIVERGENT: usize = 1_024;
 ///
 /// - the cost function dips anywhere on the swept fan and version-bound
 ///   grid;
-/// - any constructed, walked, or assembled node was underpriced;
-/// - a bulk seam mis-answered an aggregate;
+/// - any constructed, walked, exploded, or assembled node was
+///   underpriced;
+/// - a bulk seam mis-answered an aggregate, yielded out of order or
+///   outside its prefix, merged, split, or swallowed a run, or
+///   `parent` answered a real child with no parent or an empty group
+///   with one;
+/// - the stated budget resolves to the serialization floor, or the
+///   budgeted run's census peak does not exceed the floor run's (the
+///   admittance ceiling would otherwise compare two identical runs);
 /// - the window's measured byte admittance exceeded the budget; or
-/// - the session failed to converge the corpora.
+/// - the session failed to converge the corpora, or converged them to
+///   less than their union.
 pub(crate) async fn check<B>(backend: B, budget_bytes: usize)
 where
     B: Measure + Clone,
```

<!-- annotation -->
> **conformance-28** (T6), line 822:
>
> The entry: `check`'s admittance ceiling had no floor, and at 64 KiB the budgeted run was the floor run. The `# Panics` list now names the floor clause, and `check` prints both peaks and both widest capacities so a failure's log carries the diagnosis.

<!-- annotation -->
> **conformance-24** (T6), line 821:
>
> Fresh-eyes repair (item 1): `check`'s `# Panics` lists the empty-group direction.

<a id="hunk-19"></a>
### src/conformance/backend.rs `@@ -650,8 +832,13 @@ where`

```diff
@@ -650,8 +832,13 @@ where
 {
     node_bytes_monotone::<B>();
 
-    let floor_peak = run(backend.clone(), WindowConfig::Budget(0)).await;
-    let budget_peak = run(backend, WindowConfig::Budget(budget_bytes)).await;
+    let (floor_peak, floor_stats) = run(backend.clone(), WindowConfig::Budget(0), 0).await;
+    let (budget_peak, budget_stats) = run(backend, WindowConfig::Budget(budget_bytes), 0).await;
+    eprintln!(
+        "conformance census at budget {budget_bytes} B: floor run peaked at {floor_peak} B \
+         (widest capacity {}), budgeted run at {budget_peak} B (widest capacity {})",
+        floor_stats.window_granted, budget_stats.window_granted,
+    );
 
     let violations = ledger::take_violations();
     assert!(
```

<!-- annotation -->
> **conformance-28** (T6), line 835:
>
> `run` returns the session's `SessionStats` beside the peak (read from a `Recorder` shared with the client side; both sides exchange the same size pair, so both resolve the same window). This is how the floor learns the widest capacity the real session was granted, rather than re-solving from corpus sizes with version bytes guessed.

<a id="hunk-20"></a>
### src/conformance/backend.rs `@@ -660,7 +847,25 @@ where`

```diff
@@ -660,7 +847,25 @@ where
         violations.join("\n"),
     );
 
-    let admitted = budget_peak.saturating_sub(floor_peak);
+    // The liveness floor under the admittance ceiling. A budget that
+    // binds widens some stage past the one-scope serialization floor, and
+    // a wider stage holds more node values in flight at the session's
+    // peak instant than the floor does, so the least the census can read
+    // is one node value more: equal peaks mean the two runs were one run
+    // and the ceiling below could not fail.
+    assert!(
+        budget_peak > floor_peak,
+        "the budgeted window admitted nothing above the floor: both runs peaked at \
+         {floor_peak} B, so the budget does not bind at this corpus scale",
+    );
+    assert!(
+        budget_stats.window_granted > 1,
+        "the budget resolves to the serialization floor (widest capacity {}) yet the \
+         census moved: the peak difference is not the window's",
+        budget_stats.window_granted,
+    );
+
+    let admitted = budget_peak - floor_peak;
     assert!(
         admitted <= budget_bytes,
         "widening the window from the floor admitted {admitted} measured \
```

<!-- annotation -->
> **conformance-28** (T6), line 857:
>
> The liveness floor, first of the two. Premise: a budget that binds widens some stage past the one-scope floor, and a wider stage holds at least one more node value in flight at the session's peak instant than the floor does, so the census must read strictly higher; the minimum is one node value, so the assertion is a strict inequality with no margin. Negative control: at 64 KiB this fails with both peaks at 14496 B (recorded verbatim in 4b8f3c00); at 275248 B the budgeted peak is 14576 B. This premise holds here because the conformance session has no commit join: the peak is the walk's in-flight plus the assembled output. It does not hold in `tests/window_census.rs` (see that file's rows).

<!-- annotation -->
> **conformance-28** (T6), line 862:
>
> The second half of the floor, the resolution's alternative form: the session itself reports a widest capacity above one. It sits after the peak floor so a 64 KiB failure names the peaks, and it separates the two diagnoses (the solve did not widen versus the census did not see a widening). Its message is phrased for the case the peak floor passed and this did not.

<!-- annotation -->
> **conformance-28** (T6), line 868:
>
> Plain subtraction replaces `saturating_sub`: the floor above guarantees the difference is positive, so a saturating read of zero can no longer hide a regression.

<a id="hunk-21"></a>
### src/conformance/backend.rs `@@ -669,8 +874,17 @@ where`

```diff
@@ -669,8 +874,17 @@ where
 }
 
 /// One controlled-divergence reconciliation; returns the ledger's peak
-/// measured bytes above the resting corpora.
-async fn run<B>(backend: B, window: WindowConfig) -> usize
+/// measured bytes above the resting corpora, with the session's own
+/// account of itself (the window it resolved, above all).
+///
+/// `omitted` shared messages are left out of *both* corpora: zero for
+/// every honest run, and the completeness control's fault, a loss the
+/// two sides' agreement cannot see.
+pub(super) async fn run<B>(
+    backend: B,
+    window: WindowConfig,
+    omitted: usize,
+) -> (usize, SessionStats)
 where
     B: Measure + Clone,
     B::Error: std::fmt::Debug,
```

<!-- annotation -->
> **conformance-30** (T6), line 883:
>
> `run` takes the count of shared messages both corpora omit (zero in `check`): the symmetric loss the two sides' agreement cannot see, so the completeness oracle has a committed known-bad artifact. `pub(super)` so the control can drive it. I chose this over wrapping both roots because the loss must be in both corpora before the greeting, and the corpus builder is where that is one line.

<a id="hunk-22"></a>
### src/conformance/backend.rs `@@ -692,40 +906,83 @@ where`

```diff
@@ -692,40 +906,83 @@ where
         .map(|payload| (right_clock.tick().clone(), 2 << 32 | payload))
         .collect();
 
-    let left = corpus(&charged, common.iter().chain(&left_tail)).await;
-    let right = corpus(&charged, common.iter().chain(&right_tail)).await;
+    let shared = common.iter().skip(omitted);
+    let left_leaves = sorted_leaves::<B>(shared.clone().chain(&left_tail)).await;
+    let right_leaves = sorted_leaves::<B>(shared.chain(&right_tail)).await;
+
+    // The default fold over the charged backend: the path a backend
+    // without an `assemble` override runs, and the one that brings every
+    // interior group of a resting corpus through `parent`'s own checks
+    // (presence, price, aggregates). First, so the first interior
+    // `parent` call of a run is one those checks see.
+    fold_default(&charged, left_leaves.clone()).await;
+    fold_default(&charged, right_leaves.clone()).await;
+    empty_group(&charged).await;
+
+    // The bulk assembly boundary in the regime the wire decoder runs it:
+    // many maximal same-prefix runs per stream, one node each, in run
+    // order. A one-byte prefix partitions each corpus into up to `FAN`
+    // runs at this scale. Before the corpora assemble at the root, so a
+    // fault felt there is already named on the ledger.
+    assemble_runs(&charged, left_leaves.clone()).await;
+    assemble_runs(&charged, right_leaves.clone()).await;
+
+    let left = corpus(&charged, left_leaves).await;
+    let right = corpus(&charged, right_leaves).await;
 
-    // The bulk walk seam, exercised the way the wire encoder runs it:
-    // every leaf of each corpus once. Before the baseline resets, so the
-    // walk's checks land on the ledger while its transient charges stay
-    // out of the session's differenced peak.
+    // The bulk walk boundary, exercised the way the wire encoder runs it:
+    // every leaf of each corpus once from the root, then once more from
+    // each of the root's children at the child's own prefix, where the
+    // walk's containment clause has content and the explosion prices
+    // every child. Before the baseline resets, so the checks land on
+    // the ledger while the transient charges stay out of the session's
+    // differenced peak.
     walk(&charged, &left).await;
     walk(&charged, &right).await;
+    walk_children(&charged, &left).await;
+    walk_children(&charged, &right).await;
 
     // The corpora are what exists regardless of the window; measure the
     // session's own admittance above them. The greeting's sizes need no
     // stating: they are the roots' own aggregates.
     ledger::reset_peak();
 
-    let client = materialized::Handshaking::start(charged.clone(), left).window(window);
+    // The client's recorder is read afterwards: the two sides exchange
+    // the same pair of sizes, so both resolve the same window.
+    let stats = Recorder::default();
+    let client = materialized::Handshaking::start(charged.clone(), left)
+        .window(window)
+        .stats(stats.clone());
     let server = materialized::Handshaking::start(charged, right).window(window);
     let (ours, theirs) = streaming::mirror(client, server)
         .await
-        .expect("the conformance session reconciles");
+        .unwrap_or_else(|error| fail(&format!("the conformance session reconciles: {error:?}")));
     let peak = ledger::peak();
 
-    let converged = match (&ours.root, &theirs.root) {
-        (Some(left_root), Some(right_root)) => left_root.hash() == right_root.hash(),
-        _ => false,
+    // Convergence is agreement on one root; completeness is that root
+    // holding the corpora's whole union, which agreement alone cannot
+    // show when both sides lose the same leaves.
+    let (converged, reconciled) = match (&ours.root, &theirs.root) {
+        (Some(left_root), Some(right_root)) => {
+            (left_root.hash() == right_root.hash(), left_root.len())
+        }
+        _ => (false, 0),
     };
-    assert!(
-        converged,
-        "the conformance session must converge both corpora to one root",
-    );
-    peak
+    if !converged {
+        fail("the conformance session must converge both corpora to one root");
+    }
+    let union = COMMON + 2 * DIVERGENT;
+    if reconciled != union {
+        fail(&format!(
+            "the conformance session converged short: the root holds {reconciled} \
+             messages; the corpora are built to hold {union} between them",
+        ));
+    }
+    (peak, stats.snapshot())
 }
 
-/// Drain one corpus's bulk leaf walk so the walk seam's checks run.
+/// Drain one corpus's bulk leaf walk from the root so the walk's checks
+/// run.
 async fn walk<B>(charged: &Charged<B>, corpus: &Root<Charged<B>>)
 where
     B: Measure + Clone,
```

<!-- annotation -->
> **conformance-24** (T6), line 918:
>
> Judgment call, reported: `run` also folds each corpus through the default `Convert` assembly over the charged backend, first. The reference backend's own `assemble` is the default fold over itself, so its `parent` calls never pass through `Charged::parent`; the `PARENT_DROPS` control was spent there and convicted only as a short run, exactly the witness's finding. Driving the default fold over `Charged` brings every interior group of a resting corpus through the presence, price, and aggregate checks, and makes the by-name conviction the resolution asks for reachable. It is coverage the session alone did not give (the session assembles only disputed groups).

<!-- annotation -->
> **conformance-24** (T6), line 965:
>
> Convergence and completeness fail through `fail`, so a ledger violation recorded before a non-converging or short session is named in the panic rather than masked by it.

<!-- annotation -->
> **conformance-31** (T6), line 927:
>
> `assemble_runs` (line 1075): each corpus's sorted leaves driven through the backend's bulk assembly at the height under the root, one node per one-byte prefix, up to `FAN` runs per corpus, the wire decoder's regime; the multi-run boundary of `Local::assemble` is covered by `local_backend_conforms`. Before the root assembly, so a fault felt at the root is already named. `sorted_leaves` (line 1032) and `corpus` (line 1095) are the old `corpus` split so the same leaves feed all three assemblies.

<!-- annotation -->
> **conformance-30** (T6), line 977:
>
> The completeness oracle: the reconciled root holds `COMMON + 2 * DIVERGENT` messages, the resolution's first form. Fails on the honest runs never; fails by name on any session that loses a leaf on both sides.

<!-- annotation -->
> **conformance-30** (T6), line 978:
>
> Fresh-eyes repair (item 6): the completeness message no longer calls the constant the corpora's union (false by `omitted` in the control run); it says what the corpora are built to hold.

<a id="hunk-23"></a>
### src/conformance/backend.rs `@@ -736,18 +993,48 @@ where`

```diff
@@ -736,18 +993,48 @@ where
     };
     let mut leaves = pin!(charged.clone().leaves(Prefix::<height::Root>::new(), root));
     while let Some(leaf) = leaves.next().await {
-        leaf.expect("corpus leaves walk at rest");
+        leaf.unwrap_or_else(|error| fail(&format!("corpus leaves walk at rest: {error:?}")));
     }
 }
 
-/// Assemble one corpus through the charged backend, leaves in path order.
+/// Explode one corpus's root into its children and drain each child's
+/// bulk leaf walk at the child's own prefix.
+///
+/// The explosion prices every child; the walk's containment clause has
+/// content only when the walked prefix is not the root's empty one.
+async fn walk_children<B>(charged: &Charged<B>, corpus: &Root<Charged<B>>)
+where
+    B: Measure + Clone,
+    B::Error: std::fmt::Debug,
+{
+    let Some(root) = corpus.root.clone() else {
+        return;
+    };
+    let mut children = pin!(
+        charged
+            .clone()
+            .children::<height::UnderRoot>(Prefix::<height::Root>::new(), root)
+    );
+    while let Some(child) = children.next().await {
+        let (prefix, child) = child
+            .unwrap_or_else(|error| fail(&format!("corpus children explode at rest: {error:?}")));
+        let mut leaves = pin!(charged.clone().leaves(prefix, child));
+        while let Some(leaf) = leaves.next().await {
+            leaf.unwrap_or_else(|error| {
+                fail(&format!("corpus subtree leaves walk at rest: {error:?}"))
+            });
+        }
+    }
+}
+
+/// One corpus's leaves, constructed through the charged backend and
+/// sorted into path order.
 // The inline pair type is clearer than a name coined only to satisfy the
 // lint.
 #[allow(clippy::type_complexity)]
-async fn corpus<B>(
-    charged: &Charged<B>,
+async fn sorted_leaves<B>(
     messages: impl Iterator<Item = &(Version, u64)>,
-) -> Root<Charged<B>>
+) -> Vec<(Prefix<Z>, ChargedNode<B::Node<Z>>)>
 where
     B: Measure + Clone,
     B::Error: std::fmt::Debug,
```

<!-- annotation -->
> **conformance-24** (T6), line 994:
>
> `walk` fails through `fail` so a pending violation is named; `walk_children` (new) explodes the root through `Charged::children` and walks each child at its own prefix, the drive that gives the containment clause content (conformance-24) and prices every exploded child (conformance-25); `corpus` is split into `sorted_leaves` here and the root assembly below (conformance-31).

<a id="hunk-24"></a>
### src/conformance/backend.rs `@@ -758,10 +1045,84 @@ where`

```diff
@@ -758,10 +1045,84 @@ where
         let path = Path::for_leaf(version);
         let leaf = <ChargedNode<B::Node<Z>> as Leaf>::leaf(version.clone(), message)
             .await
-            .expect("corpus leaves construct at rest");
+            .unwrap_or_else(|error| fail(&format!("corpus leaves construct at rest: {error:?}")));
         leaves.push((Prefix::from(path), leaf));
     }
     leaves.sort_by_key(|(prefix, _)| *prefix);
+    leaves
+}
+
+/// Fold sorted leaves up to the height just under the root through the
+/// default level-by-level assembly over the charged backend, and drain
+/// it: every interior group passes through [`Charged::parent`].
+#[allow(clippy::type_complexity)]
+async fn fold_default<B>(charged: &Charged<B>, leaves: Vec<(Prefix<Z>, ChargedNode<B::Node<Z>>)>)
+where
+    B: Measure + Clone,
+    B::Error: std::fmt::Debug,
+{
+    let mut folded = pin!(<height::UnderRoot as Convert>::assemble::<Charged<B>>(
+        charged.clone(),
+        Box::pin(futures_stream::iter(leaves.into_iter().map(Ok))),
+    ));
+    while let Some(node) = folded.next().await {
+        node.unwrap_or_else(|error| fail(&format!("corpus leaves fold at rest: {error:?}")));
+    }
+}
+
+/// Present `parent` with an empty group, the trait's stated case of a
+/// scope that resolved to nothing at all, which must answer `None`.
+///
+/// Driven at rest because the session over this suite's corpora never
+/// produces one: a mixed scope keeps at least one unpruned child, and
+/// no resolution here comes back empty. Without this drive the
+/// presence check's second direction has no reachable input.
+async fn empty_group<B>(charged: &Charged<B>)
+where
+    B: Measure + Clone,
+    B::Error: std::fmt::Debug,
+{
+    let answered = charged
+        .clone()
+        .parent::<height::UnderRoot>(Prefix::<height::Root>::new(), Vec::new())
+        .await
+        .unwrap_or_else(|error| fail(&format!("an empty group assembles at rest: {error:?}")));
+    drop(answered);
+}
+
+/// Drive sorted leaves through the charged backend's bulk assembly at
+/// the height just under the root, one node per one-byte prefix, and
+/// drain it: the multi-run regime, held to account by the assembly's
+/// own checks.
+#[allow(clippy::type_complexity)]
+async fn assemble_runs<B>(charged: &Charged<B>, leaves: Vec<(Prefix<Z>, ChargedNode<B::Node<Z>>)>)
+where
+    B: Measure + Clone,
+    B::Error: std::fmt::Debug,
+{
+    let mut assembled = pin!(
+        charged
+            .clone()
+            .assemble::<height::UnderRoot>(Box::pin(futures_stream::iter(
+                leaves.into_iter().map(Ok)
+            )))
+    );
+    while let Some(node) = assembled.next().await {
+        node.unwrap_or_else(|error| fail(&format!("corpus runs assemble at rest: {error:?}")));
+    }
+}
+
+/// Assemble one corpus's sorted leaves through the charged backend into
+/// its root.
+#[allow(clippy::type_complexity)]
+async fn corpus<B>(
+    charged: &Charged<B>,
+    leaves: Vec<(Prefix<Z>, ChargedNode<B::Node<Z>>)>,
+) -> Root<Charged<B>>
+where
+    B: Measure + Clone,
+    B::Error: std::fmt::Debug,
+{
     // The spans are bound to a local so their borrowed join endpoints
     // outlive the fold's iterator.
     let bounds: Vec<Span<'_>> = leaves.iter().map(|(_, leaf)| leaf.span()).collect();
```

<!-- annotation -->
> **conformance-24** (T6), line 1080:
>
> Fresh-eyes repair (item 1): the presence check's `Some`-at-zero-fan direction had no reachable input. Verified by an instrumented run of both honest suites: zero zero-fan groups reach `Charged::parent` (the session prunes only against what the peer knows, so a mixed scope keeps a child, and no resolution here comes back empty). I constructed rather than reduced: `run` presents `parent` with an empty group at rest, the trait's stated case, so the direction has a reachable input and its own control. Called at line 920.

<a id="hunk-25"></a>
### src/conformance/backend.rs `@@ -775,12 +1136,11 @@ where`

```diff
@@ -775,12 +1136,11 @@ where
     let root = assembled
         .next()
         .await
-        .expect("a non-empty corpus assembles a root")
-        .expect("corpus assembly is infallible at rest");
-    assert!(
-        assembled.next().await.is_none(),
-        "one sorted run assembles exactly one root",
-    );
+        .unwrap_or_else(|| fail("a non-empty corpus assembles a root"))
+        .unwrap_or_else(|error| fail(&format!("corpus assembly is infallible at rest: {error:?}")));
+    if assembled.next().await.is_some() {
+        fail("one sorted run assembles exactly one root");
+    }
     Root {
         ceiling,
         root: Some(root.1),
```

<!-- annotation -->
> **conformance-31** (T6), line 1139:
>
> The corpus builder's `expect`s fail through `fail`: the swallowed-root case dies here with `unassembled run` in the message instead of masking it, the witness's exact failure.

<a id="hunk-26"></a>
### src/conformance/backend/tests.rs `@@ -7,20 +7,29 @@ use std::sync::atomic::{AtomicUsize, Ordering};`

```diff
@@ -7,20 +7,29 @@ use std::sync::atomic::{AtomicUsize, Ordering};
 use std::sync::{Mutex, MutexGuard, PoisonError};
 
 use async_stream::stream;
-use futures::{StreamExt, stream as futures_stream};
+use futures::{Stream, StreamExt, stream as futures_stream};
+use proptest::prelude::*;
 
 use before::Span;
 
-use super::{Charged, Measure, check, ledger};
+use super::{
+    BOUND_SWEEP_CEILING, Charged, Measure, check, fan_slot_excess, ledger, node_bytes_monotone,
+    reference_slot_excess, run,
+};
 use crate::{
     Version,
     message::Message,
     tree::{
         mirror::streaming::{
-            Backend, BoxNodeStream, ErasedNode, Leaf, Local, Node, NodeStream, convert::Convert,
+            Backend, BoxNodeStream, ErasedNode, Leaf, Local, Node, NodeStream,
+            convert::Convert,
+            window::{
+                FAN, FAN_SLOT_BYTES, REFERENCE_SLOT_BYTES, SUPPLY_DECODE_ENVELOPE_BYTES,
+                WindowConfig,
+            },
         },
         typed::{
-            self, Hash, Prefix,
+            self, Hash, Path, Prefix,
             height::{Height, S, Z},
         },
     },
```

<!-- annotation -->
> **conformance-42** (T6), line 11:
>
> Imports for the property test and the crate-internal items the tests now reach (`FAN`, `WindowConfig`, `run`, the slot-excess functions, the sweep ceiling); private items of the parent module are visible to its tests module, so nothing was widened for them.

<a id="hunk-27"></a>
### src/conformance/backend/tests.rs `@@ -87,6 +96,18 @@ impl Knob {`

```diff
@@ -87,6 +96,18 @@ impl Knob {
         self.cell.load(Ordering::Relaxed)
     }
 
+    /// Spend one unit of the knob's value: true while it was positive.
+    ///
+    /// For knobs that count faults to inject rather than bytes to add;
+    /// the guard restores the honest value on drop regardless.
+    fn take(&self) -> bool {
+        self.cell
+            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
+                value.checked_sub(1)
+            })
+            .is_ok()
+    }
+
     /// Move the knob off its honest value for one test's lifetime.
     fn set(&'static self, value: usize) -> Dishonest {
         let serial = serialized();
```

<!-- annotation -->
> **conformance-24** (T6), line 103:
>
> `Knob::take`: a decrementing read for knobs that count faults to inject rather than bytes to add. The guard still restores the honest value on drop.

<a id="hunk-28"></a>
### src/conformance/backend/tests.rs `@@ -117,18 +138,49 @@ impl Measure for Local {`

```diff
@@ -117,18 +138,49 @@ impl Measure for Local {
         // tree: its shallow size is everything it keeps resident.
         std::mem::size_of_val(node)
     }
+
+    fn measure_erased(node: &Self::Erased) -> usize {
+        std::mem::size_of_val(node)
+    }
 }
 
+/// The stated budget the in-memory check runs under.
+///
+/// It must clear the flat decode-fan pre-charge the window solve takes
+/// off every budget before widening any stage
+/// ([`SUPPLY_DECODE_ENVELOPE_BYTES`], the in-memory pricing of that
+/// term) with room left for dispute scopes; a budget at or below the
+/// pre-charge resolves to the serialization floor, and `check`'s
+/// liveness floor fails it by name. The 64 KiB above the pre-charge is
+/// what widens the suite's corpora past the floor: the floor holds
+/// only that the window is wider than the floor's and that the census
+/// peak moved, never a particular width.
+const LOCAL_BUDGET: usize = SUPPLY_DECODE_ENVELOPE_BYTES + 64 * 1024;
+
 /// The in-memory backend's pointer-priced account holds end to end.
 ///
 /// `Local` is the trivial case — handles into a resident tree — so the
 /// suite's pointwise check reduces to the pointer-size constant, and the
-/// end-to-end census confirms the window's byte admittance under a tight
-/// budget that genuinely binds at this scale.
+/// end-to-end census confirms the window's byte admittance under
+/// [`LOCAL_BUDGET`]: the budgeted run widens the window past the floor
+/// and its census peak exceeds the floor run's.
 #[test]
 fn local_backend_conforms() {
     let _serial = serialized();
-    pollster::block_on(check(Local, 64 * 1024));
+    pollster::block_on(check(Local, LOCAL_BUDGET));
+}
+
+/// A budget that covers only the flat decode-fan pre-charge fails the
+/// liveness floor by name.
+///
+/// It leaves nothing for dispute scopes: the solve floors every capacity
+/// at one, the budgeted run is the floor run, and the census reads one
+/// peak twice, which the admittance ceiling alone would pass vacuously.
+#[test]
+#[should_panic(expected = "admitted nothing above the floor")]
+fn a_pre_charge_only_budget_fails_the_liveness_floor() {
+    let _serial = serialized();
+    pollster::block_on(check(Local, SUPPLY_DECODE_ENVELOPE_BYTES));
 }
 
 /// A materializing reference backend, shaped like a database row store.
```

<!-- annotation -->
> **conformance-28** (T6), line 158:
>
> `LOCAL_BUDGET`, derived from the term it must clear rather than a literal: the flat decode-fan pre-charge under in-memory pricing plus 64 KiB, the resolution's suggested form. Measured once: at the suite's corpus (1536 a side) it lands a widest capacity of 4 and the census peak moves from 14496 B to 14576 B. Landed after the floor was committed failing at 64 KiB (4b8f3c00), then resized (a29692e9), so the history shows the order the brief requires.

<!-- annotation -->
> **conformance-28** (T6), line 164:
>
> The testdoc restated from the measured behaviour: the budgeted run widens the window past the floor and its peak exceeds the floor run's, which is what the body now asserts. The old text claimed a budget that binds, which was false.

<!-- annotation -->
> **conformance-28** (T6), line 181:
>
> Negative control, committed: `check(Local, SUPPLY_DECODE_ENVELOPE_BYTES)` is a budget spent entirely on the pre-charge, so every capacity floors at one and the peak floor fails by name. I chose the pre-charge itself over the brief's 64 KiB because it is the exact boundary of the premise (monotone: any smaller budget floors too), so the control needs no magic number; the 64 KiB failure is recorded in the commit message instead.

<!-- annotation -->
> **conformance-41** (T6), line 142:
>
> `measure_erased` for `Local` (and line 600 for `Materializing`): the same shape as `measure` over the erased handle.

<!-- annotation -->
> **conformance-28** (T6), line 155:
>
> Fresh-eyes round 2 (item 2): `LOCAL_BUDGET`'s doc no longer quotes an observed width (the widest capacity of four); it states what the floor asserts, a window wider than the floor's and a moved census peak.

<a id="hunk-29"></a>
### src/conformance/backend/tests.rs `@@ -142,6 +194,12 @@ fn local_backend_conforms() {`

```diff
@@ -142,6 +194,12 @@ fn local_backend_conforms() {
 /// knob's value, so the knob's honest resting value is the real header
 /// and anything less underprices — what the lying tests opt into
 /// through the knob's guard.
+///
+/// Its node is aligned wider than a pointer ([`MaterializedNode`]), so
+/// the decode-fan and reference slots the session keeps it in pad
+/// beyond the in-memory layout the window's slot constants derive from;
+/// that padding ([`SLOT_PADDING`]) is the backend's to price, and
+/// [`PRICED_SLOT_PADDING`] is the knob that prices it honestly at rest.
 #[derive(Clone, Copy, Debug)]
 struct Materializing;
 
```

<!-- annotation -->
> **conformance-40** (T6), line 195:
>
> `Materializing`'s doc gains the paragraph on its wider-than-pointer node: the slot padding is the backend's to price and `PRICED_SLOT_PADDING` prices it honestly at rest. Doc only.

<a id="hunk-30"></a>
### src/conformance/backend/tests.rs `@@ -156,6 +214,27 @@ static WALK_SLACK: Knob = Knob::new(0);`

```diff
@@ -156,6 +214,27 @@ static WALK_SLACK: Knob = Knob::new(0);
 /// Leaves the bulk walk silently drops: honest at zero.
 static WALK_SKIPS: Knob = Knob::new(0);
 
+/// Whether the bulk walk yields its first two leaves in swapped order:
+/// honest at zero, unordered above it.
+static LEAVES_SWAP: Knob = Knob::new(0);
+
+/// Whether the bulk walk re-tags its first leaf's prefix outside the
+/// walked subtree: honest at zero, escaping above it.
+static WALK_ESCAPES: Knob = Knob::new(0);
+
+/// Extra bytes the exploded interior child rows keep resident (leaf
+/// children stay honest, so the walk's leaf check is silent): honest at
+/// zero, over-holding above it.
+static CHILDREN_SLACK: Knob = Knob::new(0);
+
+/// Interior `parent` calls (a fan of at least one real child) answered
+/// with `Ok(None)`, spent one per call: honest at zero.
+static PARENT_DROPS: Knob = Knob::new(0);
+
+/// Whether an empty group presented to `parent` is answered with a
+/// conjured node instead of `None`: honest at zero.
+static PARENT_CONJURES: Knob = Knob::new(0);
+
 /// Extra bytes bulk-assembled rows keep resident: honest at zero,
 /// over-holding above it.
 static ASSEMBLE_SLACK: Knob = Knob::new(0);
```

<!-- annotation -->
> **conformance-24** (T6), line 232:
>
> `PARENT_DROPS`: interior parent calls answered with `Ok(None)`, spent one per call.

<!-- annotation -->
> **conformance-24** (T6), line 219:
>
> `LEAVES_SWAP` and (line 219) `WALK_ESCAPES`: the walk yields its first two leaves swapped, or re-tags its first leaf's prefix outside every subtree but the root's (top bit of the first path byte flipped). The escape knob is beyond the resolution's listed knobs; the brief's negative-control rule requires a committed known-bad artifact for the containment check, and without the escape there is none.

<!-- annotation -->
> **conformance-25** (T6), line 228:
>
> `CHILDREN_SLACK`, applied at line 524 to interior children only: leaf-height children re-enter `Charged::leaves` through the reference walk and are caught there for an incidental reason, as the witness found; the control's subject is the check on exploded interior references.

<!-- annotation -->
> **conformance-24** (T6), line 236:
>
> Fresh-eyes repair (item 1): `PARENT_CONJURES`, applied at line 528: an empty group answered with a node conjured from a leaf re-tagged at the parent's height (the tag is phantom; the check on trial reads only presence). Negative control at line 776 expects `parent contract: fan 0 yielded Some`.

<a id="hunk-31"></a>
### src/conformance/backend/tests.rs `@@ -163,6 +242,18 @@ static ASSEMBLE_SLACK: Knob = Knob::new(0);`

```diff
@@ -163,6 +242,18 @@ static ASSEMBLE_SLACK: Knob = Knob::new(0);
 /// Leaves bulk assembly silently drops: honest at zero.
 static ASSEMBLE_SKIPS: Knob = Knob::new(0);
 
+/// Whether bulk assembly yields its first two nodes in swapped order:
+/// honest at zero, unordered above it.
+static ASSEMBLE_SWAP: Knob = Knob::new(0);
+
+/// The one-based position of the assembled node bulk assembly folds and
+/// then swallows: honest at zero (none swallowed).
+static ASSEMBLED_DROPS: Knob = Knob::new(0);
+
+/// The one-based position of the assembled node bulk assembly re-tags to
+/// its neighbor's prefix: honest at zero (none re-tagged).
+static ASSEMBLE_RETAGS: Knob = Knob::new(0);
+
 /// Bytes subtracted from every node's `version_bytes` answer: honest at
 /// zero, deflating the aggregate above it.
 static VERSION_DEFLATE: Knob = Knob::new(0);
```

<!-- annotation -->
> **conformance-24** (T6), line 247:
>
> `ASSEMBLE_SWAP`: the assembly yields its first two nodes swapped.

<!-- annotation -->
> **conformance-31** (T6), line 251:
>
> `ASSEMBLED_DROPS` and (line 247) `ASSEMBLE_RETAGS`: one-based positions of the assembled node to swallow or to re-tag to its neighbor's prefix (`neighbor`, line 633, flips the last prefix byte's low bit; the root prefix has no neighbor and is left alone). Applied at lines 577 and 584.

<a id="hunk-32"></a>
### src/conformance/backend/tests.rs `@@ -176,6 +267,79 @@ static LEAF_VERSION_INFLATE: Knob = Knob::new(0);`

```diff
@@ -176,6 +267,79 @@ static LEAF_VERSION_INFLATE: Knob = Knob::new(0);
 /// honest at zero, a monotonicity dip above it.
 static PRICED_DIP: Knob = Knob::new(0);
 
+/// The version bound above which [`Materializing::node_bytes`] prices
+/// [`STEP_DROP`] fewer header bytes: honest at zero (no step), a
+/// step-shaped monotonicity dip above it.
+static PRICED_BOUND_STEP: Knob = Knob::new(0);
+
+/// The one version bound at which [`Materializing::node_bytes`] prices
+/// one byte less than at the bound before it: honest at zero (no dip),
+/// a point dip above it.
+static PRICED_BOUND_DIP: Knob = Knob::new(0);
+
+/// The bound the point-dip control dips at: inside the dense sweep and
+/// off every power of two and its neighbors, so only a sweep dense
+/// through it compares across the dip.
+const DIP_BOUND: usize = 101;
+
+/// The slot padding [`Materializing::node_bytes`] prices: honest at the
+/// real [`SLOT_PADDING`], lying below it.
+static PRICED_SLOT_PADDING: Knob = Knob::new(SLOT_PADDING);
+
+/// Extra bytes a row keeps after [`Backend::assume`] re-tags it: honest
+/// at zero, a residency-changing re-tag above it.
+static ASSUME_SLACK: Knob = Knob::new(0);
+
+/// Extra bytes a row keeps after [`Backend::erase`] re-tags it: honest
+/// at zero, a residency-changing re-tag above it.
+static ERASE_SLACK: Knob = Knob::new(0);
+
+/// The threshold bound the step control places strictly inside the
+/// sweep grid's widest gap, between the neighbors of the two largest
+/// powers of two: no adjacent grid pair straddles it, so the grid alone
+/// cannot see the step.
+const STEP_THRESHOLD: usize = (BOUND_SWEEP_CEILING / 2 + 1 + BOUND_SWEEP_CEILING - 1) / 2;
+
+/// The header bytes the step drops: the width of that gap, the largest
+/// drop the gap's two grid endpoints tolerate; across it they price
+/// equal, which the sweep's non-strict comparison accepts.
+const STEP_DROP: usize = (BOUND_SWEEP_CEILING - 1) - (BOUND_SWEEP_CEILING / 2 + 1);
+
+/// The bytes the session's slots pad around a [`MaterializedNode`]
+/// beyond the in-memory layout the window prices: the larger of the
+/// decode-fan slot's and the reference slots' excess, so one price
+/// covers both.
+const SLOT_PADDING: usize = {
+    let fan = fan_slot_excess::<Materializing>();
+    let reference = reference_slot_excess::<Materializing, Z>();
+    if fan > reference { fan } else { reference }
+};
+
+// The control below zeroes the priced padding; it convicts only if the
+// real padding is positive, which the node's alignment guarantees.
+const _: () = assert!(
+    SLOT_PADDING > 0,
+    "a node aligned wider than a pointer pads its slots",
+);
+
+// The same padding written out from the layouts by hand, independently
+// of the suite's excess functions, so an over-estimate there cannot
+// hide behind a price that covers it. On a 64-bit target: `Prefix<Z>`
+// is 34 bytes at alignment 2, the in-memory handle one 8-byte `Arc`,
+// and `Hash` 24 bytes, so the window's decode-fan pair is 48 bytes
+// (`FAN_SLOT_BYTES` 40) and its query, resolution, and listing slots
+// 16 + 24 + 25 bytes (`REFERENCE_SLOT_BYTES` 65). `MaterializedNode` is
+// 32 bytes at alignment 16: the fan pair grows to 80 bytes (excess
+// 80 - 32 - 40 = 8), the query slot to 48 (16 of padding against the
+// handle's 8, excess 8), and the resolution slot keeps 16 of padding
+// either way (24 - 8 and 48 - 32); the padding is 8. Layouts differ at
+// other pointer widths, so the pin is gated.
+const _: () = assert!(
+    std::mem::size_of::<usize>() != 8
+        || (FAN_SLOT_BYTES == 40 && REFERENCE_SLOT_BYTES == 65 && SLOT_PADDING == 8),
+    "the derived slot padding matches the hand-computed 64-bit layout",
+);
+
 /// The one fan [`PRICED_DIP`] carves the dip at: an arbitrary interior
 /// value the monotonicity sweep's adjacent-fan comparisons must cross.
 const DIP_FAN: usize = 7;
```

<!-- annotation -->
> **conformance-40** (T6), line 320:
>
> A const assertion that the derived padding is positive, so the control cannot pass vacuously if the alignment attribute is ever dropped.

<!-- annotation -->
> **conformance-41** (T6), line 291:
>
> `ASSUME_SLACK` and (line 287) `ERASE_SLACK`, applied at lines 448 and 439: the row grows across each re-tag. The erase knob is beyond the resolution's named `ASSUME_SLACK`; the erase direction's check needs its own known-bad artifact.

<!-- annotation -->
> **conformance-42** (T6), line 273:
>
> `PRICED_BOUND_STEP`: the header shrinks by `STEP_DROP` above a threshold. `STEP_THRESHOLD` (line 293) and `STEP_DROP` (line 297) derive from `BOUND_SWEEP_CEILING`: the threshold is the midpoint of the grid's widest gap (between the neighbors of 2^19 and 2^20) so no adjacent grid pair straddles it, and the drop is that gap's width, the largest drop across which the gap's two endpoints still ascend. With those, a uniformly drawn `(bound, delta)` pair straddles the step with probability about one in eight, so the property convicts in 256 cases with certainty for practical purposes even before the seed replays.

<!-- annotation -->
> **conformance-42** (T6), line 278:
>
> `PRICED_BOUND_DIP` and `DIP_BOUND` (101): a one-byte point dip at a bound off every power of two, inside the dense sweep; the acceptance's 'point dip at a non-grid bound below the raised dense ceiling', committed as a control (line 1028) rather than argued.

<!-- annotation -->
> **conformance-40** (T6), line 337:
>
> Fresh-eyes repair (item 4): `SLOT_PADDING` pinned against an independent hand-written layout, gated on a 64-bit pointer. The reviewer's arithmetic gave `REFERENCE_SLOT_BYTES = 73` with the listing slot at 33 bytes; measured, `Hash` is 24 bytes so the listing slot is 25 and the constant is 65 (the pin at 73 failed at compile time, which is the pin working). Fan pair 48 to 80, query slot 16 to 48, resolution slot 24 to 48 with 16 of padding either way, padding 8: all as the reviewer computed.

<!-- annotation -->
> **conformance-42** (T6), line 304:
>
> Fresh-eyes repair (item 5): the gap's endpoints price equal across `STEP_DROP`, not ascending; the sentence now says so and names the non-strict comparison that accepts it.

<a id="hunk-33"></a>
### src/conformance/backend/tests.rs `@@ -186,12 +350,23 @@ const ROW_HEADER: usize = 64;`

```diff
@@ -186,12 +350,23 @@ const ROW_HEADER: usize = 64;
 /// The bytes one child entry occupies in a materialized row.
 const ROW_ENTRY: usize = 24;
 
-/// The stated budget every materializing check runs under: the rows
-/// make it genuinely binding at the suite's corpus scale.
+/// The stated budget every materializing check runs under.
+///
+/// The rows make this backend's flat decode-fan pre-charge several
+/// times the in-memory one (each fan slot carries a header and bounds,
+/// not a pointer); 4 MiB clears it with room for dispute scopes, and
+/// `check`'s liveness floor holds the window to being wider than the
+/// floor's, never to a particular width, while the rows keep the
+/// admitted bytes a real fraction of the budget.
 const MATERIALIZING_BUDGET: usize = 4 * 1024 * 1024;
 
 /// A node value that owns its simulated row.
+///
+/// Aligned wider than a pointer, as a handle carrying a 16-byte row id
+/// would be: the shape the window's slot constants say owes its extra
+/// slot padding to its own price.
 #[derive(Clone, Debug)]
+#[repr(align(16))]
 struct MaterializedNode<N> {
     inner: N,
     row: Vec<u8>,
```

<!-- annotation -->
> **conformance-28** (T6), line 355:
>
> `MATERIALIZING_BUDGET` re-checked against its own flat term through the same floor: at 4 MiB the budgeted run peaks 1033 B above the floor run (193707 B to 194740 B) with a widest capacity of 45, so the floor holds; the doc now states that rather than asserting the budget binds.

<!-- annotation -->
> **conformance-40** (T6), line 369:
>
> Judgment call, reported: the reference backend's node is made `#[repr(align(16))]` and its price gains the derived padding (`PRICED_SLOT_PADDING`, honest at `SLOT_PADDING`, line 303, the larger of the fan-slot and reference-slot excess), rather than adding a fourth backend type for the control. The resolution asks for a `#[repr(align(16))]` wrapper node whose `node_bytes` omits the padding, asserted to fail by name; `MaterializedNode` is that wrapper node, and the knob is how it omits the padding. A new backend type would instantiate the whole protocol tower again (documented at +0.7 GiB of rustc memory) or exercise only leaf construction; this way the honest run exercises the excess at every check. `Local` is untouched and `Materializing`'s honest test passes unchanged; its definition changed, which is the part to rule on if the acceptance's 'unchanged' meant the definition.

<!-- annotation -->
> **conformance-28** (T6), line 359:
>
> Fresh-eyes round 2 (item 2): `MATERIALIZING_BUDGET`'s doc drops 'tens of scopes wide' for the property the floor holds.

<a id="hunk-34"></a>
### src/conformance/backend/tests.rs `@@ -285,9 +460,11 @@ impl Backend for Materializing {`

```diff
@@ -285,9 +460,11 @@ impl Backend for Materializing {
 
     // Erasure re-tags the store's handle; the resident row rides along
     // unchanged, so the census this backend exists to exercise sees no
-    // movement from either conversion.
+    // movement from either conversion -- until the re-tag-lying knobs
+    // ([`ERASE_SLACK`], [`ASSUME_SLACK`]) grow the row across it.
     fn erase<H: Height>(node: Self::Node<H>) -> Self::Erased {
-        let MaterializedNode { inner, row } = node;
+        let MaterializedNode { inner, mut row } = node;
+        row.resize(row.len() + ERASE_SLACK.get(), 0);
         MaterializedNode {
             inner: inner.into_untyped(),
             row,
```

<!-- annotation -->
> **conformance-41** (T6), line 462:
>
> `erase` grows the row by `ERASE_SLACK` across the re-tag, the erase-direction fault; the comment names both re-tag knobs.

<a id="hunk-35"></a>
### src/conformance/backend/tests.rs `@@ -295,7 +472,8 @@ impl Backend for Materializing {`

```diff
@@ -295,7 +472,8 @@ impl Backend for Materializing {
     }
 
     fn assume<H: Height>(erased: Self::Erased) -> Self::Node<H> {
-        let MaterializedNode { inner, row } = erased;
+        let MaterializedNode { inner, mut row } = erased;
+        row.resize(row.len() + ASSUME_SLACK.get(), 0);
         MaterializedNode {
             inner: typed::Node::from_untyped(inner),
             row,
```

<!-- annotation -->
> **conformance-41** (T6), line 474:
>
> `assume` grows the row by `ASSUME_SLACK` across the re-tag, the resolution's named fault.

<a id="hunk-36"></a>
### src/conformance/backend/tests.rs `@@ -304,15 +482,29 @@ impl Backend for Materializing {`

```diff
@@ -304,15 +482,29 @@ impl Backend for Materializing {
 
     fn node_bytes(children: usize, version_bound: usize) -> usize {
         let priced = std::mem::size_of::<MaterializedNode<typed::Node<Z>>>()
+            + PRICED_SLOT_PADDING.get()
             + PRICED_HEADER.get()
             + ROW_ENTRY * children
             + version_bound;
-        // The monotonicity-lying knob: a dip at one fan, invisible to
-        // every check that does not compare across it.
-        if children == DIP_FAN {
+        // The monotonicity-lying knobs: a dip at one fan, invisible to
+        // every check that does not compare across it, and a step in
+        // the bound, invisible to a grid whose points all sit on one
+        // side of it.
+        let priced = if children == DIP_FAN {
             priced.saturating_sub(PRICED_DIP.get())
         } else {
             priced
+        };
+        let priced = if version_bound != 0 && version_bound == PRICED_BOUND_DIP.get() {
+            priced.saturating_sub(2)
+        } else {
+            priced
+        };
+        let step = PRICED_BOUND_STEP.get();
+        if step != 0 && version_bound > step {
+            priced.saturating_sub(STEP_DROP)
+        } else {
+            priced
         }
     }
 
```

<!-- annotation -->
> **conformance-42** (T6), line 490:
>
> `node_bytes` prices the slot padding through `PRICED_SLOT_PADDING` (conformance-40) and applies the two bound-lying faults after the fan dip: the point dip at `PRICED_BOUND_DIP` (two bytes, so the adjacent-bound comparison sees a strict fall) and the step above `PRICED_BOUND_STEP`; the comment says which check each is invisible to.

<a id="hunk-37"></a>
### src/conformance/backend/tests.rs `@@ -326,6 +518,22 @@ impl Backend for Materializing {`

```diff
@@ -326,6 +518,22 @@ impl Backend for Materializing {
         S<H>: Height,
     {
         let fan = children.iter().filter(|(_, child)| child.is_some()).count();
+        // The presence-lying knob: an interior fan answered with no
+        // parent, the fault the trait's `parent` clause forbids.
+        if fan > 0 && PARENT_DROPS.take() {
+            return Ok(None);
+        }
+        // The other direction of the presence lie: a group with no real
+        // child answered with a node conjured from nothing (a leaf
+        // re-tagged at the parent's height; the tag is phantom, and the
+        // check on trial reads only the answer's presence).
+        if fan == 0 && PARENT_CONJURES.get() != 0 {
+            let conjured = typed::Node::from_untyped(typed::untyped::Node::leaf(
+                Version::new(),
+                Message::new(0),
+            ));
+            return Ok(Some(MaterializedNode::wrap(conjured, ROW_HEADER)));
+        }
         let children = children
             .into_iter()
             .map(|(radix, child)| (radix, child.map(|child| child.inner)))
```

<!-- annotation -->
> **conformance-24** (T6), line 523:
>
> The presence fault injected at the reference backend's `parent`, spent on the first interior call.

<a id="hunk-38"></a>
### src/conformance/backend/tests.rs `@@ -347,8 +555,13 @@ impl Backend for Materializing {`

```diff
@@ -347,8 +555,13 @@ impl Backend for Materializing {
             while let Some(child) = children.next().await {
                 yield child.map(|(prefix, node)| {
                     // A lazily loaded row: header and bounds, its child
-                    // table not yet materialized.
-                    let row = ROW_HEADER + bounds_of(&node);
+                    // table not yet materialized. The over-holding knob
+                    // ([`CHILDREN_SLACK`]) inflates interior rows only:
+                    // leaf-height children re-enter the walk's own leaf
+                    // check through `leaves`, and the control's subject
+                    // is the check on exploded interior references.
+                    let slack = if H::HEIGHT > 0 { CHILDREN_SLACK.get() } else { 0 };
+                    let row = ROW_HEADER + bounds_of(&node) + slack;
                     (prefix, MaterializedNode::wrap(node, row))
                 });
             }
```

<!-- annotation -->
> **conformance-25** (T6), line 558:
>
> `children` inflates interior rows only by `CHILDREN_SLACK`; the comment states why leaf-height children stay honest (they re-enter the walk's own leaf check through `leaves`).

<a id="hunk-39"></a>
### src/conformance/backend/tests.rs `@@ -361,20 +574,29 @@ impl Backend for Materializing {`

```diff
@@ -361,20 +574,29 @@ impl Backend for Materializing {
         node: Self::Node<H>,
     ) -> impl NodeStream<Self, Z> {
         // The reference bulk walk: the default explosion behind knobs
-        // that drop leaves ([`WALK_SKIPS`]) and inflate the yielded rows
-        // ([`WALK_SLACK`]) — honest at rest, the negative controls'
+        // that drop leaves ([`WALK_SKIPS`]), inflate the yielded rows
+        // ([`WALK_SLACK`]), re-tag the first leaf out of the walked
+        // subtree ([`WALK_ESCAPES`]), and swap the first two leaves
+        // ([`LEAVES_SWAP`]) — honest at rest, the negative controls'
         // subject when set.
-        H::explode::<Self>(
+        let walked = H::explode::<Self>(
             self,
             Box::pin(futures_stream::once(async move { Ok((prefix, node)) })),
         )
         .skip(WALK_SKIPS.get())
-        .map(|item| {
+        .enumerate()
+        .map(|(index, item)| {
             item.map(|(prefix, mut leaf)| {
                 leaf.row.resize(leaf.row.len() + WALK_SLACK.get(), 0);
+                let prefix = if index == 0 && WALK_ESCAPES.get() != 0 {
+                    escaped(prefix)
+                } else {
+                    prefix
+                };
                 (prefix, leaf)
             })
-        })
+        });
+        swapped_head(LEAVES_SWAP.get() != 0, walked)
     }
 
     fn assemble<'a, H: Convert>(
```

<!-- annotation -->
> **conformance-24** (T6), line 592:
>
> The escape and (line 560) the swap applied to the reference walk's output; `swapped_head` (line 607) and `escaped` (line 625) are the two small adaptors, shared with the assembly's swap.

<a id="hunk-40"></a>
### src/conformance/backend/tests.rs `@@ -382,16 +604,30 @@ impl Backend for Materializing {`

```diff
@@ -382,16 +604,30 @@ impl Backend for Materializing {
         leaves: BoxNodeStream<'a, Self, Z>,
     ) -> impl NodeStream<Self, H> + 'a {
         // The reference bulk assembly: the default fold behind knobs
-        // that drop supplied leaves ([`ASSEMBLE_SKIPS`]) and inflate the
-        // assembled rows ([`ASSEMBLE_SLACK`]) — honest at rest, the
-        // negative controls' subject when set.
+        // that drop supplied leaves ([`ASSEMBLE_SKIPS`]), inflate the
+        // assembled rows ([`ASSEMBLE_SLACK`]), swallow one assembled
+        // node ([`ASSEMBLED_DROPS`]), re-tag one to its neighbor's
+        // prefix ([`ASSEMBLE_RETAGS`]), and swap the first two
+        // ([`ASSEMBLE_SWAP`]) — honest at rest, the negative controls'
+        // subject when set.
         let supplied: BoxNodeStream<'a, Self, Z> = Box::pin(leaves.skip(ASSEMBLE_SKIPS.get()));
-        H::assemble(self, supplied).map(|item| {
-            item.map(|(prefix, mut node)| {
-                node.row.resize(node.row.len() + ASSEMBLE_SLACK.get(), 0);
-                (prefix, node)
+        let assembled = H::assemble(self, supplied)
+            .enumerate()
+            .filter_map(|(index, item)| async move {
+                (index + 1 != ASSEMBLED_DROPS.get()).then_some((index, item))
             })
-        })
+            .map(|(index, item)| {
+                item.map(|(prefix, mut node)| {
+                    node.row.resize(node.row.len() + ASSEMBLE_SLACK.get(), 0);
+                    let prefix = if index + 1 == ASSEMBLE_RETAGS.get() {
+                        neighbor(prefix)
+                    } else {
+                        prefix
+                    };
+                    (prefix, node)
+                })
+            });
+        swapped_head(ASSEMBLE_SWAP.get() != 0, assembled)
     }
 }
 
```

<!-- annotation -->
> **conformance-31** (T6), line 610:
>
> `assemble` composes the assembly faults on the default fold's output: the swallow (`ASSEMBLED_DROPS`, a positional `filter_map`), the re-tag to a neighbor (`ASSEMBLE_RETAGS`), and the head swap (`ASSEMBLE_SWAP`), beside the existing skip and slack; the comment lists all five.

<a id="hunk-41"></a>
### src/conformance/backend/tests.rs `@@ -399,6 +635,49 @@ impl Measure for Materializing {`

```diff
@@ -399,6 +635,49 @@ impl Measure for Materializing {
     fn measure<H: Height>(node: &Self::Node<H>) -> usize {
         std::mem::size_of_val(node) + node.row.len()
     }
+
+    fn measure_erased(node: &Self::Erased) -> usize {
+        std::mem::size_of_val(node) + node.row.len()
+    }
+}
+
+/// `inner` with its first two items in swapped order when `swap` holds:
+/// the order-lying adaptor behind the walk's and the assembly's controls.
+fn swapped_head<T>(swap: bool, inner: impl Stream<Item = T>) -> impl Stream<Item = T> {
+    stream! {
+        let mut inner = pin!(inner);
+        if swap {
+            let first = inner.next().await;
+            let second = inner.next().await;
+            for item in [second, first].into_iter().flatten() {
+                yield item;
+            }
+        }
+        while let Some(item) = inner.next().await {
+            yield item;
+        }
+    }
+}
+
+/// A leaf prefix moved out of every subtree but the root's: its first
+/// path byte flipped at the top bit.
+fn escaped(prefix: Prefix<Z>) -> Prefix<Z> {
+    let mut path = <[u8; 32]>::from(prefix);
+    path[0] ^= 0x80;
+    Prefix::from(path)
+}
+
+/// The prefix's neighbor at its own height, one radix bit over on its
+/// last byte; the root prefix has no neighbor and is returned as is.
+fn neighbor<H: Height>(prefix: Prefix<H>) -> Prefix<H> {
+    let depth = prefix.as_bytes().len();
+    if depth == 0 {
+        return prefix;
+    }
+    let mut path = [0u8; 32];
+    path[..depth].copy_from_slice(prefix.as_bytes());
+    path[depth - 1] ^= 1;
+    Prefix::<H>::containing(&Path::from(Prefix::<Z>::from(path)))
 }
 
 /// An honestly priced materializing backend passes the whole suite.
```

<!-- annotation -->
> **conformance-24** (T6), line 640:
>
> `measure_erased` for `Materializing` (conformance-41), and the three helpers the faults share: `swapped_head` (the head-swap adaptor for the walk and the assembly), `escaped` (a leaf prefix moved out of every subtree but the root's), and `neighbor` (a prefix's sibling one bit over, the root left alone).

<a id="hunk-42"></a>
### src/conformance/backend/tests.rs `@@ -477,6 +756,117 @@ fn lossy_bulk_assembly_fails_the_len_check() {`

```diff
@@ -477,6 +756,117 @@ fn lossy_bulk_assembly_fails_the_len_check() {
     pollster::block_on(check(Materializing, MATERIALIZING_BUDGET));
 }
 
+/// A `parent` that answers a fan of real children with no parent is
+/// convicted by name at the presence check.
+///
+/// The knob spends itself on the first interior `parent` call of the
+/// run, which is the default fold's over the charged decorator, where
+/// the check lives; this backend's own bulk assembly folds through its
+/// `parent` directly, where the same fault is felt only as a short run.
+#[test]
+#[should_panic(expected = "parent contract: fan")]
+fn an_interior_parent_answering_none_fails_the_presence_check() {
+    let _dishonest = PARENT_DROPS.set(1);
+    pollster::block_on(check(Materializing, MATERIALIZING_BUDGET));
+}
+
+/// A `parent` that answers an empty group with a node is convicted by
+/// name at the presence check's other direction, on the empty group the
+/// run presents at rest.
+#[test]
+#[should_panic(expected = "parent contract: fan 0 yielded Some")]
+fn a_parent_conjured_from_an_empty_group_fails_the_presence_check() {
+    let _dishonest = PARENT_CONJURES.set(1);
+    pollster::block_on(check(Materializing, MATERIALIZING_BUDGET));
+}
+
+/// A bulk walk that yields two leaves out of path order is convicted by
+/// name at the walk's order check: count and prices are unchanged, so
+/// nothing else would notice.
+#[test]
+#[should_panic(expected = "unordered leaf walk")]
+fn a_swapped_walk_fails_the_order_check() {
+    let _dishonest = LEAVES_SWAP.set(1);
+    pollster::block_on(check(Materializing, MATERIALIZING_BUDGET));
+}
+
+/// A bulk walk that yields a leaf outside the walked prefix is convicted
+/// by name at the walk's containment check, which has content at the
+/// sub-root walk the run drives (every leaf is inside the root's empty
+/// prefix).
+#[test]
+#[should_panic(expected = "escaped leaf walk")]
+fn an_escaping_walk_fails_the_containment_check() {
+    let _dishonest = WALK_ESCAPES.set(1);
+    pollster::block_on(check(Materializing, MATERIALIZING_BUDGET));
+}
+
+/// A `children` override whose exploded interior rows over-hold memory
+/// is convicted by name at the explosion's pointwise check.
+///
+/// The walk's leaf check never sees an interior child, so this check is
+/// the only one that prices the references the window prices per depth.
+#[test]
+#[should_panic(expected = "underpriced child")]
+fn overholding_children_fail_the_pointwise_check() {
+    let _dishonest = CHILDREN_SLACK.set(BULK_OVERHOLD);
+    pollster::block_on(check(Materializing, MATERIALIZING_BUDGET));
+}
+
+/// Bulk assembly that yields two nodes out of run order is convicted by
+/// name at the assembly's order check, in the multi-run regime the run
+/// drives at a sub-root height (the root assembly has one node to
+/// order).
+#[test]
+#[should_panic(expected = "unordered assembly")]
+fn a_swapped_assembly_fails_the_order_check() {
+    let _dishonest = ASSEMBLE_SWAP.set(1);
+    pollster::block_on(check(Materializing, MATERIALIZING_BUDGET));
+}
+
+/// Bulk assembly that folds a run and then swallows its node is
+/// convicted by name: the run it supplied never yielded a node.
+///
+/// The same fault at the root assembly costs the corpus its root, and
+/// the by-name report rides in that failure rather than being masked by
+/// it.
+#[test]
+#[should_panic(expected = "unassembled run")]
+fn a_swallowed_assembled_node_fails_the_run_check() {
+    let _dishonest = ASSEMBLED_DROPS.set(1);
+    pollster::block_on(check(Materializing, MATERIALIZING_BUDGET));
+}
+
+/// Bulk assembly that yields a node at a prefix other than its own run's
+/// is convicted by name.
+///
+/// When no run supplied the re-tagged prefix, the re-tagged node itself
+/// arrives unsupplied. When the neighbor has a run (the case at this
+/// corpus, where nearly every one-byte prefix does), the re-tagged node
+/// consumes that run and is convicted as a `bulk-assembled len`
+/// mismatch, and the honest neighbor that follows finds its run gone:
+/// `unsupplied assembly` fires either way.
+#[test]
+#[should_panic(expected = "unsupplied assembly")]
+fn a_re_tagged_assembled_node_fails_the_run_check() {
+    let _dishonest = ASSEMBLE_RETAGS.set(1);
+    pollster::block_on(check(Materializing, MATERIALIZING_BUDGET));
+}
+
+/// A loss both sides share is invisible to their agreement and caught
+/// by the completeness oracle: the reconciled root must hold the
+/// corpora's whole union.
+///
+/// Driven at the run level with one shared message left out of both
+/// corpora, the fault a symmetric backend defect produces and the only
+/// shape the two-sided convergence check cannot see.
+#[test]
+#[should_panic(expected = "converged short")]
+fn a_symmetric_loss_fails_the_completeness_check() {
+    let _serial = serialized();
+    pollster::block_on(run(Materializing, WindowConfig::Budget(0), 1));
+}
+
 /// A deflated `version_bytes` answer is caught by the assembly seam's floor.
 ///
 /// The run's own leaf encodings and the node's two bounds are all in
```

<!-- annotation -->
> **conformance-24** (T6), line 768:
>
> Negative control: the presence check fires by name. Its doc explains why the knob is spent by the default fold over the decorator (line 916 of backend.rs).

<!-- annotation -->
> **conformance-24** (T6), line 788:
>
> Negative control: the walk's order check fires by name; count and prices are unchanged by a swap, so nothing else would.

<!-- annotation -->
> **conformance-24** (T6), line 799:
>
> Negative control: the walk's containment check fires by name, at the sub-root walk (every leaf is inside the root's empty prefix).

<!-- annotation -->
> **conformance-24** (T6), line 822:
>
> Negative control: the assembly's order check fires by name, in the multi-run regime (the root assembly yields one node, so it has nothing to swap).

<!-- annotation -->
> **conformance-25** (T6), line 811:
>
> Negative control: 64 KiB of slack on every exploded interior row fails `underpriced child` by name; the honest suites pass.

<!-- annotation -->
> **conformance-31** (T6), line 835:
>
> Negative control: a swallowed assembled node fails `unassembled run` by name, through the sub-root assembly's leftover and, at the root, through the surfaced corpus failure.

<!-- annotation -->
> **conformance-31** (T6), line 851:
>
> Negative control: a re-tagged assembled node fails `unsupplied assembly` by name.

<!-- annotation -->
> **conformance-30** (T6), line 865:
>
> Negative control: one shared message left out of both corpora converges (equal hashes) and fails `converged short` by name.

<!-- annotation -->
> **conformance-31** (T6), line 848:
>
> Fresh-eyes round 2 (item 4): the retag control's doc covers both cases, an absent neighbor (the re-tagged node is the unsupplied one) and a neighbor with a run (the re-tagged node consumes it and is convicted as a length mismatch; the honest neighbor that follows is the unsupplied one), which is the case at this corpus.

<a id="hunk-43"></a>
### src/conformance/backend/tests.rs `@@ -499,6 +889,41 @@ fn inflated_leaf_version_bytes_fails_the_walk_check() {`

```diff
@@ -499,6 +889,41 @@ fn inflated_leaf_version_bytes_fails_the_walk_check() {
     pollster::block_on(check(Materializing, MATERIALIZING_BUDGET));
 }
 
+/// A backend whose node is aligned wider than a pointer and whose price
+/// omits the slot padding that alignment costs is convicted by name at
+/// the first leaf it constructs.
+///
+/// The window's slot constants derive from the in-memory layout and
+/// leave a wider layout's extra padding to the backend's price; the
+/// leaf check folds that excess into its comparison, so pricing the
+/// node value alone falls short by exactly the padding.
+#[test]
+#[should_panic(expected = "underpriced leaf: ")]
+fn unpriced_slot_padding_fails_the_pointwise_check() {
+    let _dishonest = PRICED_SLOT_PADDING.set(0);
+    pollster::block_on(check(Materializing, MATERIALIZING_BUDGET));
+}
+
+/// A re-tag that grows the node's residency on the way back to a typed
+/// handle is convicted by name: the measurement after `assume` differs
+/// from the bytes the erased handle carried.
+#[test]
+#[should_panic(expected = "re-tagged node changed residency: erased")]
+fn a_residency_changing_assume_fails_the_re_tag_check() {
+    let _dishonest = ASSUME_SLACK.set(BULK_OVERHOLD);
+    pollster::block_on(check(Materializing, MATERIALIZING_BUDGET));
+}
+
+/// A re-tag that grows the node's residency on the way to an erased
+/// handle is convicted by name: the measurement after `erase` differs
+/// from the bytes the typed handle carried.
+#[test]
+#[should_panic(expected = "re-tagged node changed residency: typed")]
+fn a_residency_changing_erase_fails_the_re_tag_check() {
+    let _dishonest = ERASE_SLACK.set(BULK_OVERHOLD);
+    pollster::block_on(check(Materializing, MATERIALIZING_BUDGET));
+}
+
 /// A cost function with a dip between quantile evaluation points is
 /// caught by the monotonicity sweep before any session runs.
 ///
```

<!-- annotation -->
> **conformance-40** (T6), line 902:
>
> Negative control: the padding priced at zero fails `underpriced leaf` at the first leaf, the message naming the decode-slot padding.

<!-- annotation -->
> **conformance-41** (T6), line 912:
>
> Negative controls, one per direction (line 866 for erase): each fails by name with the direction in the message.

<!-- annotation -->
> **conformance-40** (T6), line 901:
>
> Fresh-eyes repair (item 2): the control expects `underpriced leaf: `, a prefix only the constructed-leaf site emits (the walk's site says `underpriced walked leaf`), so dropping `fan_slot_excess` from the constructed-leaf check alone fails it. Observed: 6144 lines of `underpriced leaf: measured N B plus 8 B of decode-slot padding, node_bytes priced N B`.

<a id="hunk-44"></a>
### src/conformance/backend/tests.rs `@@ -586,3 +1011,86 @@ fn ledger_settles_over_clone_and_drop() {`

```diff
@@ -586,3 +1011,86 @@ fn ledger_settles_over_clone_and_drop() {
         "the peak persists after handles settle",
     );
 }
+
+/// One case of the bound-monotonicity property: the price at `bound`
+/// does not exceed the price at any larger bound, at one fan.
+fn assert_monotone_in_bound<B: Backend<Node<Z>: Leaf>>(fan: usize, bound: usize, delta: usize) {
+    let there = bound.saturating_add(delta);
+    let here_priced = B::node_bytes(fan, bound);
+    let there_priced = B::node_bytes(fan, there);
+    assert!(
+        here_priced <= there_priced,
+        "node_bytes must be monotone in version bound: bound {bound} prices {here_priced} B, \
+         bound {there} prices {there_priced} B, at fan {fan}",
+    );
+}
+
+proptest! {
+    /// The in-memory cost function is monotone in the version bound over
+    /// the whole family: for any bound and any increase, at any fan, the
+    /// price does not fall.
+    ///
+    /// The sweep in `check` samples adjacent grid points; this covers
+    /// the pairs between them.
+    #[test]
+    fn local_node_bytes_is_monotone_in_the_version_bound(
+        fan in 0..=FAN,
+        bound in 0..=BOUND_SWEEP_CEILING,
+        delta in 0..=BOUND_SWEEP_CEILING,
+    ) {
+        let _serial = serialized();
+        assert_monotone_in_bound::<Local>(fan, bound, delta);
+    }
+
+    /// The materializing cost function is monotone in the version bound
+    /// over the whole family: for any bound and any increase, at any fan,
+    /// the price does not fall.
+    ///
+    /// The sweep in `check` samples adjacent grid points; this covers
+    /// the pairs between them.
+    #[test]
+    fn materializing_node_bytes_is_monotone_in_the_version_bound(
+        fan in 0..=FAN,
+        bound in 0..=BOUND_SWEEP_CEILING,
+        delta in 0..=BOUND_SWEEP_CEILING,
+    ) {
+        let _serial = serialized();
+        assert_monotone_in_bound::<Materializing>(fan, bound, delta);
+    }
+
+    /// A step-shaped dip strictly inside a grid gap fails the
+    /// bound-monotonicity property by name: the case the grid sweep
+    /// cannot see, which the sibling control shows it passing.
+    ///
+    /// The knob is set for each case's lifetime; the failing case's seed
+    /// is committed, so the conviction replays deterministically.
+    #[test]
+    #[should_panic(expected = "monotone in version bound")]
+    fn a_step_dip_fails_the_monotonicity_property(
+        fan in 0..=FAN,
+        bound in 0..=BOUND_SWEEP_CEILING,
+        delta in 0..=BOUND_SWEEP_CEILING,
+    ) {
+        let _dishonest = PRICED_BOUND_STEP.set(STEP_THRESHOLD);
+        assert_monotone_in_bound::<Materializing>(fan, bound, delta);
+    }
+}
+
+/// A point dip at one bound inside the dense sweep is caught by the
+/// sweep's adjacent-bound comparison before any session runs: the
+/// price at [`DIP_BOUND`] falls one byte below the bound before it.
+#[test]
+#[should_panic(expected = "monotone in version bound")]
+fn a_point_dip_fails_the_dense_sweep() {
+    let _dishonest = PRICED_BOUND_DIP.set(DIP_BOUND);
+    pollster::block_on(check(Materializing, MATERIALIZING_BUDGET));
+}
+
+/// The same step-shaped dip passes the grid sweep: every adjacent grid
+/// pair sits on one side of the threshold, so the sweep is a sample of
+/// the family and the property test above is what holds the family.
+#[test]
+fn a_step_dip_hides_from_the_grid_sweep() {
+    let _dishonest = PRICED_BOUND_STEP.set(STEP_THRESHOLD);
+    node_bytes_monotone::<Materializing>();
+}
```

<!-- annotation -->
> **conformance-42** (T6), line 1036:
>
> The `proptest!` over `(fan, bound, delta)` for both backends, the resolution's strategy verbatim, each case holding the serialization lock. The seed file at proptest-regressions/conformance/backend/tests.txt is committed; `tests/seed_liveness.rs` accepts its location.

<!-- annotation -->
> **conformance-42** (T6), line 1069:
>
> Negative control: the step fails the property by name (the runner's panic carries the assertion's message). The knob is set per case because `Knob::set` takes the lock the honest cases also take.

<!-- annotation -->
> **conformance-42** (T6), line 1093:
>
> The resolution's other half of the demonstration: the same step passes the grid sweep, committed as a passing test, which is what shows the property test is not decoration.

<a id="hunk-45"></a>
### src/tree/mirror/streaming/window.rs `@@ -151,7 +151,7 @@ const KEY_DEPTH: usize = 32;`

```diff
@@ -151,7 +151,7 @@ const KEY_DEPTH: usize = 32;
 /// pointer-class node handles; a backend whose `Node` demands a wider
 /// layout pads the real slots beyond this constant and owes that padding
 /// to its own `node_bytes` price.
-const REFERENCE_SLOT_BYTES: usize = std::mem::size_of::<(u8, typed::Node<Z>)>()
+pub(crate) const REFERENCE_SLOT_BYTES: usize = std::mem::size_of::<(u8, typed::Node<Z>)>()
     + std::mem::size_of::<(u8, Resolve<<Local as Backend>::Erased>)>()
     + std::mem::size_of::<(u8, typed::Hash)>();
 
```

<!-- annotation -->
> **conformance-40** (T6), line 154:
>
> `REFERENCE_SLOT_BYTES` and (line 175) `FAN_SLOT_BYTES` become `pub(crate)` so the suite can subtract them; crate-internal, inside the ruling.

<a id="hunk-46"></a>
### src/tree/mirror/streaming/window.rs `@@ -172,7 +172,7 @@ const LEAF_REQUEST_BYTES: usize = 40;`

```diff
@@ -172,7 +172,7 @@ const LEAF_REQUEST_BYTES: usize = 40;
 /// whose `Node<Z>` demands wider alignment pads the real slot beyond
 /// `node_bytes + FAN_SLOT_BYTES` and owes that padding to its own
 /// `node_bytes` price.
-const FAN_SLOT_BYTES: usize =
+pub(crate) const FAN_SLOT_BYTES: usize =
     std::mem::size_of::<(Prefix<Z>, typed::Node<Z>)>() - std::mem::size_of::<typed::Node<Z>>();
 
 /// Worst-case bytes the decode fans of one session keep resident, under
```

<!-- annotation -->
> **conformance-40** (T6), line 175:
>
> `FAN_SLOT_BYTES` becomes `pub(crate)` so the suite can subtract it from the real pair's padding; crate-internal, inside the ruling.

<a id="hunk-47"></a>
### tests/window_census.rs `@@ -7,38 +7,61 @@`

```diff
@@ -7,38 +7,61 @@
 //! atomic replace), the reconciled generation, the session's output tree
 //! (transiently coexisting with both at the commit join), and the
 //! window's in-flight work. The first three are content and scale with
-//! the divergence; only the fourth is the window's to bound. These tests
-//! isolate it by differencing runs of the *identical* divergence under
-//! different budgets, then hold it against the admittance the derived
-//! capacities state.
+//! the divergence; only the fourth is the window's to bound, and its
+//! byte admittance is owned by the backend conformance suite's census
+//! (`rumors::conformance::backend`), whose session has no commit join.
+//! Here the census pins what a session at the floor holds above its
+//! generations (the commit's double-existence, bounded by the content),
+//! the version bounds a reconciled tree assembles against the pair
+//! bound the greeting priced, and that a tight budget widens the
+//! session's window past the serialization floor.
 //!
-//! The census is process-global, which is sound here because nextest runs
-//! each test in its own process.
+//! The census is process-global, so every test body holds
+//! [`CENSUS_LOCK`]: the suite is correct under any runner's threading,
+//! not only nextest's process-per-test model.
+
+use std::sync::{Mutex, MutexGuard, PoisonError};
 
 use rand::rngs::SmallRng;
 use rand::{RngCore, SeedableRng};
-use rumors::testing::{node_census, node_census_reset, window_capacities};
-use rumors::{Peer, Rumors};
+use rumors::testing::{
+    node_census, node_census_reset, supply_decode_envelope_bytes, window_capacities,
+};
+use rumors::{Gossiped, Peer, Rumors};
+
+/// Serializes the tests in this binary.
+///
+/// The node census is process-global: tests overlapping in one process
+/// (plain `cargo test` runs tests as threads) would reset and read each
+/// other's peaks, and every test here constructs nodes. Each test holds
+/// this lock for its whole body through [`census_locked`]; a
+/// `should_panic` test poisons it by design, and the poison guards no
+/// invariant, so the guard clears it and continues.
+static CENSUS_LOCK: Mutex<()> = Mutex::new(());
+
+/// Take the census lock for one test's lifetime.
+fn census_locked() -> MutexGuard<'static, ()> {
+    CENSUS_LOCK.lock().unwrap_or_else(PoisonError::into_inner)
+}
 
 /// Per-stream in-memory link buffering: far above every transfer here, so
 /// link backpressure never shapes residency.
 const LINK_CAPACITY: usize = 8 * 1024 * 1024;
 
-/// A budget that binds at test scale: a few scopes per level.
-const TIGHT_BUDGET: usize = 64 * 1024;
+/// A budget under which the window widens past the serialization floor.
+///
+/// It must clear the flat decode-fan pre-charge the window solve takes
+/// off every budget before widening any stage (about 210 KB under the
+/// in-memory pricing, [`supply_decode_envelope_bytes`]), so that some
+/// stage resolves wider than one scope: [`fixture_capacities`] holds
+/// that of the derived capacities, and the widening test holds it of
+/// the session's own report. Neither pins a width; the 2 MiB is a
+/// calibration observed to widen at [`DIVERGENT_WIDE`], not a bound.
+const TIGHT_BUDGET: usize = 2 * 1024 * 1024;
 
 /// Messages each side originates beyond the common prefix.
 const DIVERGENT_WIDE: usize = 20_000;
 
-/// Handles one buffered scope can pin at most: a full fan of child
-/// references plus its own bookkeeping.
-const HANDLES_PER_SCOPE: usize = 256 + 2;
-
-/// Handles the assembly fan queues can hold beyond the window: one full
-/// fan per active level (their capacity is a correctness floor the window
-/// never scales; see the window module docs).
-const ASSEMBLY_FAN_HANDLES: usize = 33 * 256;
-
 /// Transient slack: conversion buffers, in-hand replies, and the join's
 /// working set, all bounded per session rather than per divergence.
 const TRANSIENT_SLACK: usize = 8 * 1024;
```

<!-- annotation -->
> **tests-resource-link-window-19** (T18), line 19:
>
> The module doc no longer rests correctness on nextest's process model: every test body holds the census lock, so the suite is correct under any runner's threading.

<!-- annotation -->
> **tests-resource-link-window-19** (T18), line 40:
>
> `CENSUS_LOCK`, the same shape as decode_alloc's meter lock: the node census is process-global and every test here constructs nodes. The guard clears a poisoned lock because a `should_panic` test poisons it by design and the poison guards nothing.

<!-- annotation -->
> **tests-resource-link-window-20** (T26), line 60:
>
> `TIGHT_BUDGET` measured, not handed: 2 MiB, a calibration observed once to widen the window at 21024 messages a side (capacities sum 569 over 33 stages, the session reporting a widest capacity of 91), where 64 KiB and the 209712 B pre-charge both floor every capacity at one. The constant's doc states the one property the code holds, widening past the floor at the derived capacities and in the session's report, and says the number is a calibration, not a bound.

<!-- annotation -->
> **tests-resource-link-window-19** (T18), line 23:
>
> Imports for the lock, the pre-charge accessor, and `Gossiped`.

<!-- annotation -->
> **tests-resource-link-window-20** (T139), line 11:
>
> Ruling T139: the module doc names the backend conformance suite's census as the owner of the window's byte-admittance claim and states what this census pins instead (content overhead at the floor, the version-bound claims, and that a tight budget widens the session's window).

<!-- annotation -->
> **tests-resource-link-window-20** (T139), line 60:
>
> Ruling T139 (and fresh-eyes item 3): `TIGHT_BUDGET`'s doc states the one property the code holds, widening past the floor at the derived capacities and in the session's report; the 'far below what admits the whole divergence' clause is gone with the ceiling it described.

<!-- annotation -->
> **tests-resource-link-window-20** (T139), line 59:
>
> Fresh-eyes round 2 (item 2): `TIGHT_BUDGET`'s doc drops 'between a dozen and a hundred' and says the 2 MiB is an observed calibration, not a bound; the floors pin no width.

<a id="hunk-48"></a>
### tests/window_census.rs `@@ -74,64 +97,109 @@ fn send_random(rumors: &Rumors<u64>, n: usize, rng: &mut SmallRng) {`

```diff
@@ -74,64 +97,109 @@ fn send_random(rumors: &Rumors<u64>, n: usize, rng: &mut SmallRng) {
     rumors.send_all((0..n).map(|_| rng.next_u64())).unwrap();
 }
 
-/// One reconciliation session over a roomy in-memory link.
-fn reconcile(a: &Rumors<u64>, b: &Rumors<u64>) {
+/// One reconciliation session over a roomy in-memory link: both sides'
+/// accounts of it, `a`'s first.
+fn reconcile(a: &Rumors<u64>, b: &Rumors<u64>) -> (Gossiped, Gossiped) {
     pollster::block_on(async {
         let (mut left, mut right) = rumors::link::memory_with_capacity(LINK_CAPACITY);
         let (a_result, b_result) = tokio::join!(a.gossip(&mut left), b.gossip(&mut right));
-        a_result.expect("gossip a");
-        b_result.expect("gossip b");
-    });
+        (a_result.expect("gossip a"), b_result.expect("gossip b"))
+    })
+}
+
+/// The census of one session over a fresh divergence.
+struct Overhead {
+    /// Handles the session held at peak above its two resting
+    /// generations: content double-existence plus window in-flight.
+    handles: usize,
+    /// Handles the reconciled generation holds at rest.
+    after: usize,
+    /// The left side's account of the session.
+    session: Gossiped,
 }
 
-/// The session-peak handles a run holds above its two resting
-/// generations: content double-existence plus window in-flight.
-fn overhead(budget: usize, divergent: usize) -> usize {
+/// Reconcile a fresh `divergent`-message divergence under `budget` and
+/// difference the census peak against the two resting generations.
+///
+/// The differencing rests on both old generations being alive when the
+/// last side commits its fresh output, so the peak covers both resting
+/// generations at once; a session that dropped one side's old
+/// generation before the other's output finished assembling would read
+/// a peak below their sum and fail here visibly, not vacuously.
+fn overhead(budget: usize, divergent: usize) -> Overhead {
     let (left, right) = diverged(budget, divergent);
     let before = node_census().live;
     node_census_reset();
-    reconcile(&left, &right);
+    let (session, _) = reconcile(&left, &right);
     let peak = node_census().peak;
     let after = node_census().live;
-    let overhead = peak.saturating_sub(before + after);
+    let handles = peak
+        .checked_sub(before + after)
+        .expect("peak covers both resting generations");
     eprintln!(
         "budget {budget}, divergence {divergent}: peak {peak}, \
-         generations {before}+{after}, overhead {overhead}",
+         generations {before}+{after}, overhead {handles}, \
+         widest capacity {}",
+        session.stats.window_granted,
     );
-    overhead
+    Overhead {
+        handles,
+        after,
+        session,
+    }
 }
 
-/// Window-attributable residency stays inside the derived admittance.
+/// The per-height capacities `budget` derives for the widening test's
+/// session, held to the fixture's own liveness.
 ///
-/// The identical divergence runs once at the zero-budget floor and once
-/// at a budget that binds at test scale; the content components (both
-/// generations and the output tree) are the same trees in both runs, so
-/// the peak difference is the window's own buffering — which must stay
-/// inside what the derived capacities admit: each scope a full fan of
-/// handles, plus the assembly fans and bounded per-session slack. The
-/// admittance is denominated in the same capacities `sync_memory_budget`
-/// derives, so a regression that buffers past the window moves the
-/// measurement, not the bound.
-#[test]
-fn window_attributable_residency_stays_inside_admittance() {
+/// Some stage must resolve wider than the one-scope serialization floor,
+/// or the budgeted session would run the floor's window.
+fn fixture_capacities(budget: usize) -> Vec<usize> {
     // The sizes the session itself will exchange: both replicas hold the
     // common prefix plus their own divergence when they reconcile.
     let session_len = (1_024 + DIVERGENT_WIDE) as u64;
-    let capacities = window_capacities(session_len, session_len, TIGHT_BUDGET);
-    let admitted: usize = capacities.iter().sum::<usize>() * HANDLES_PER_SCOPE
-        + ASSEMBLY_FAN_HANDLES
-        + TRANSIENT_SLACK;
-    let floor = overhead(0, DIVERGENT_WIDE);
-    let windowed = overhead(TIGHT_BUDGET, DIVERGENT_WIDE);
-    eprintln!(
-        "admittance {admitted} (capacities sum {})",
-        capacities.iter().sum::<usize>(),
+    let capacities = window_capacities(session_len, session_len, budget);
+    assert!(
+        capacities.iter().sum::<usize>() > capacities.len(),
+        "budget {budget} resolves to the serialization floor at {session_len} messages a \
+         side: every capacity is one, so a budgeted session would run the floor's window",
     );
+    capacities
+}
+
+/// A budget that covers only the flat decode-fan pre-charge leaves
+/// nothing for dispute scopes, so the solve floors every capacity at one
+/// and the fixture's liveness check fails it by name.
+#[test]
+#[should_panic(expected = "resolves to the serialization floor")]
+fn a_pre_charge_only_budget_fails_the_fixture_liveness() {
+    let _census = census_locked();
+    fixture_capacities(supply_decode_envelope_bytes());
+}
+
+/// A tight budget widens the session's window past the serialization
+/// floor, at the derived capacities and in the session itself.
+///
+/// The budget's derived capacities must sum past one per stage, and the
+/// real session over the same population must report a widest capacity
+/// above one: the solve and the session agree that the budget bound a
+/// window wider than the floor. What that window admits in bytes is the
+/// backend conformance suite's claim to hold (its census has no commit
+/// join, so a wider window moves its peak); this census's peak is the
+/// commit join at every budget, so nothing here differences peaks.
+#[test]
+fn a_tight_budget_widens_the_session_window() {
+    let _census = census_locked();
+    let capacities = fixture_capacities(TIGHT_BUDGET);
+    let windowed = overhead(TIGHT_BUDGET, DIVERGENT_WIDE);
+    eprintln!("capacities sum {}", capacities.iter().sum::<usize>());
+    // The real session widened, not only the test-side solve: the
+    // budgeted side reports the widest capacity it was granted.
     assert!(
-        windowed <= floor + admitted,
-        "widening the window from the floor added {} handles at peak; \
-         the derived capacities admit {admitted}",
-        windowed.saturating_sub(floor),
+        windowed.session.stats.window_granted > 1,
+        "the budgeted session ran at the serialization floor (widest capacity {}); \
+         the budget does not bind at this population",
+        windowed.session.stats.window_granted,
     );
 }
 
```

<!-- annotation -->
> **tests-resource-link-window-20** (T26), line 102:
>
> `reconcile` returns both sides' `Gossiped`, so the admittance test can read the window the real session was granted.

<!-- annotation -->
> **tests-resource-link-window-19** (T18), line 111:
>
> The differencing arithmetic hoisted into one site. I return a small named struct rather than the resolution's `(overhead, after)` tuple because -20 also needs the session's report from the same call; three anonymous fields would be unreadable at the call sites. Not a deviation in substance: one site, both quantities.

<!-- annotation -->
> **tests-resource-link-window-20** (T26), line 137:
>
> `checked_sub(...).expect(...)` replaces `saturating_sub`: a peak below the two resting generations is a broken measurement and now fails instead of reading zero. This is the one remaining site; the floor test's copy was folded into `overhead`.

<!-- annotation -->
> **tests-resource-link-window-20** (T26), line 142:
>
> The census log line now carries the widest capacity the session reported, so the log shows the two arms ran different windows (1 versus 91) even where their peaks agree.

<!-- annotation -->
> **tests-resource-link-window-20** (T26), line 157:
>
> The fixture's own liveness, hoisted so the control below can drive it without a session: the derived capacities must sum past one per stage. Premise: the solve floors every stage at one when the budget is at or below the pre-charge, so a sum equal to the height count means the two arms would run the identical window.

<!-- annotation -->
> **tests-resource-link-window-20** (T26), line 175:
>
> Negative control, committed and cheap (no session): the fixture at a budget equal to the pre-charge fails the liveness assertion by name. The brief's literal `TIGHT_BUDGET = 64 * 1024` mutation was also run once and restored; it fails at the same assertion in 0.00 s (recorded verbatim in f8d82b6e).

<!-- annotation -->
> **tests-resource-link-window-20** (T26), line 199:
>
> The real-session floor: `window_granted > 1` from the budgeted arm's `Gossiped`. Premise: a session whose solve widened any stage reports that stage's capacity as its widest, so a report of one means the session ran at the serialization floor whatever the test-side solve said.

<!-- annotation -->
> **tests-resource-link-window-19** (T18), line 192:
>
> The lock taken at the top of every test body, this one and the four others (the other four bodies likewise); the version-bound tests construct nodes too.

<!-- annotation -->
> **tests-resource-link-window-20** (T139), line 191:
>
> Ruling T139: the admittance test is now the widening test. Deleted as decoration: the `admitted` ceiling, the floor arm it differenced against, and `HANDLES_PER_SCOPE` and `ASSEMBLY_FAN_HANDLES`, which fed only it; `TRANSIENT_SLACK` and `overhead` stay for the content-overhead pin. The doc says what the body asserts and why no peak is differenced here (this census's peak is the commit join at every budget). Renamed for what it measures.

<!-- annotation -->
> **tests-resource-link-window-20** (T139), line 152:
>
> Ruling T139: the fixture-liveness doc and message speak of the budgeted session rather than differenced arms.

<!-- annotation -->
> **tests-resource-link-window-19** (T18), line 125:
>
> Fresh-eyes round 2 (item 3): `overhead`'s doc states the premise its checked subtraction rests on (both old generations alive when the last side commits) and that a session breaking it fails the instrument visibly rather than vacuously.

<a id="hunk-49"></a>
### tests/window_census.rs `@@ -150,6 +218,7 @@ fn window_attributable_residency_stays_inside_admittance() {`

```diff
@@ -150,6 +218,7 @@ fn window_attributable_residency_stays_inside_admittance() {
 /// the slack to reality.
 #[test]
 fn version_bounds_stay_inside_the_priced_pair_bound() {
+    let _census = census_locked();
     let (left, right) = diverged(TIGHT_BUDGET, DIVERGENT_WIDE);
 
     // A third concurrent history: interior joins over two parties stay
```

<!-- annotation -->
> **tests-resource-link-window-19** (T18), line 221:
>
> The census lock taken at the top of the version-bound test's body; it constructs nodes too.

<a id="hunk-50"></a>
### tests/window_census.rs `@@ -217,6 +286,7 @@ fn bootstrap_from(provider: &Rumors<u64>) -> Rumors<u64> {`

```diff
@@ -217,6 +286,7 @@ fn bootstrap_from(provider: &Rumors<u64>) -> Rumors<u64> {
 /// exactly here; the bound-covering aggregate holds.
 #[test]
 fn wide_concurrent_frontiers_stay_inside_the_exchanged_bound() {
+    let _census = census_locked();
     // Doubling generations: every fork halves a *different* interval, so
     // party intervals stay shallow and stamps stay small — the many
     // frontiers accumulate in the join, not in any one leaf.
```

<!-- annotation -->
> **tests-resource-link-window-19** (T18), line 289:
>
> The census lock taken at the top of the wide-frontier test's body; it constructs nodes too.

<a id="hunk-51"></a>
### tests/window_census.rs `@@ -271,17 +341,13 @@ fn wide_concurrent_frontiers_stay_inside_the_exchanged_bound() {`

```diff
@@ -271,17 +341,13 @@ fn wide_concurrent_frontiers_stay_inside_the_exchanged_bound() {
 /// of it.
 #[test]
 fn floor_overhead_is_bounded_by_content() {
-    let (left, right) = diverged(0, DIVERGENT_WIDE);
-    let before = node_census().live;
-    node_census_reset();
-    reconcile(&left, &right);
-    let peak = node_census().peak;
-    let after = node_census().live;
-    let overhead = peak.saturating_sub(before + after);
-    eprintln!("floor: peak {peak}, generations {before}+{after}, overhead {overhead}");
+    let _census = census_locked();
+    let floor = overhead(0, DIVERGENT_WIDE);
     assert!(
-        overhead <= after + TRANSIENT_SLACK,
-        "floor session held {overhead} handles above its generations, \
-         more than one output tree ({after}) can explain",
+        floor.handles <= floor.after + TRANSIENT_SLACK,
+        "floor session held {} handles above its generations, \
+         more than one output tree ({}) can explain",
+        floor.handles,
+        floor.after,
     );
 }
```

<!-- annotation -->
> **tests-resource-link-window-19** (T18), line 347:
>
> The floor test now reads through `overhead` instead of spelling the before/reset/reconcile/peak/after arithmetic a second time.

<a id="hunk-52"></a>
### tests/window_corners.rs `@@ -28,6 +28,17 @@ const DELAY: Duration = Duration::from_millis(10);`

```diff
@@ -28,6 +28,17 @@ const DELAY: Duration = Duration::from_millis(10);
 /// Roomy per-stream pipe buffering: only round-trip structure is measured.
 const LINK_CAPACITY: usize = 8 * 1024 * 1024;
 
+/// A budget under which the greeting's sizes derive a window wider than
+/// the serialization floor.
+///
+/// The window solve takes a flat decode-fan pre-charge (about 210 KB
+/// under the in-memory pricing) off every budget before widening any
+/// stage, so a budget below it derives the floor whatever the sizes
+/// say; 512 KiB clears it with room for dispute scopes at the growth
+/// test's population, and the test holds the session to having widened
+/// past the floor, never to a particular width.
+const GROWTH_BUDGET: usize = 512 * 1024;
+
 /// Build a bootstrapped pair, then commit `left_extra`/`right_extra`
 /// further messages on the respective sides, all under `budget`.
 fn pair(
```

<!-- annotation -->
> **tests-resource-link-window-20** (T26), line 40:
>
> `window_corners.rs:174` re-measured with the census: 64 KiB floors every capacity at this population (4048 a side), so the doc's greeting-derived window never existed. `GROWTH_BUDGET` is 512 KiB, past the pre-charge with a widest capacity of 23 here; the doc states the property (clears the pre-charge, widens past the floor) and the test holds the session to it (line 218), so the literal cannot silently go stale.

<!-- annotation -->
> **tests-resource-link-window-20** (T26), line 39:
>
> Fresh-eyes round 2 (item 2): `GROWTH_BUDGET`'s doc drops 'a few dozen scopes wide' for the property the guard asserts.

<a id="hunk-53"></a>
### tests/window_corners.rs `@@ -100,6 +111,14 @@ fn asymmetric_catch_up_is_ladder_bound_at_the_floor() {`

```diff
@@ -100,6 +111,14 @@ fn asymmetric_catch_up_is_ladder_bound_at_the_floor() {
         "a one-common-message catch-up must cost ladder hops, not waves: \
          {measured} hops",
     );
+    // The floor is the one exchange no transfer avoids: completion
+    // depends on a reply to a delivered message, two causally chained
+    // one-way hops.
+    assert!(
+        measured >= 2,
+        "a catch-up cannot complete without one request and its reply: \
+         {measured} hops",
+    );
 }
 
 /// The same catch-up serves in the other direction: the large side
```

<!-- annotation -->
> **tests-resource-link-window-25** (T28), line 118:
>
> The hop floor. Premise: completion depends on a reply to a delivered message, so no transfer avoids one request and its reply, two causally chained one-way hops; the floor is that minimum, not the 8 hops observed. A `hops` returning 0 fails it by reading; both catch-up tests measure 8.

<a id="hunk-54"></a>
### tests/window_corners.rs `@@ -113,6 +132,11 @@ fn asymmetric_catch_up_is_direction_independent() {`

```diff
@@ -113,6 +132,11 @@ fn asymmetric_catch_up_is_direction_independent() {
         measured <= 12,
         "catch-up direction must not change the dispute price: {measured} hops",
     );
+    assert!(
+        measured >= 2,
+        "a catch-up cannot complete without one request and its reply: \
+         {measured} hops",
+    );
 }
 
 /// The zero-budget floor keeps its promise at wide mutual divergence:
```

<!-- annotation -->
> **tests-resource-link-window-25** (T28), line 136:
>
> The same floor at the reverse-direction site, same premise.

<a id="hunk-55"></a>
### tests/window_corners.rs `@@ -165,13 +189,17 @@ fn one_byte_pipes_at_the_floor_stay_live() {`

```diff
@@ -165,13 +189,17 @@ fn one_byte_pipes_at_the_floor_stay_live() {
 
 /// A set that grows mid-session cannot break the session it grows under.
 ///
-/// The window derives from the sizes exchanged at the greeting; commits
-/// racing the session make those sizes stale in the direction of more
-/// population, which may only serialize. The racing session completes,
-/// and the next session converges whatever it missed.
+/// The window derives from the sizes exchanged at the greeting (a
+/// budget past the flat pre-charge, so the derivation widens it past
+/// the floor); commits racing the session make those sizes stale in
+/// the direction of more population, which may only serialize. The
+/// racing session completes, at least one racing commit is witnessed
+/// to have landed after the greeting's snapshot (the left replica has
+/// grown past what the session gave the right one), and the next
+/// session converges whatever it missed.
 #[test]
 fn growth_during_a_session_only_serializes() {
-    let (left, right) = pair(64 * 1024, 2_048, 2_000, 2_000);
+    let (left, right) = pair(GROWTH_BUDGET, 2_048, 2_000, 2_000);
     let racer = left.clone();
     pollster::block_on(async {
         let (mut a, mut b) = rumors::link::memory_with_capacity(LINK_CAPACITY);
```

<!-- annotation -->
> **tests-resource-link-window-28** (T28), line 196:
>
> The testdoc restated to what the body now witnesses: the window is derived past the floor, at least one racing commit is shown to have landed after the greeting's snapshot, and the follow-up converges the snapshots.

<a id="hunk-56"></a>
### tests/window_corners.rs `@@ -184,8 +212,24 @@ fn growth_during_a_session_only_serializes() {`

```diff
@@ -184,8 +212,24 @@ fn growth_during_a_session_only_serializes() {
         };
         let (left_result, right_result, ()) =
             tokio::join!(left.gossip(&mut a), right.gossip(&mut b), race);
-        left_result.expect("gossip left under concurrent growth");
+        let left_result = left_result.expect("gossip left under concurrent growth");
         right_result.expect("gossip right");
+        assert!(
+            left_result.stats.window_granted > 1,
+            "the racing session ran at the serialization floor (widest capacity {}): the \
+             greeting's sizes derived no window for the race to stale",
+            left_result.stats.window_granted,
+        );
+        // The mid-session-growth witness: a commit that landed after the
+        // greeting's snapshot is on the left replica and was never
+        // offered to the right one, so the left has grown past the right.
+        assert!(
+            left.snapshot().len() > right.snapshot().len(),
+            "no racing commit landed after the greeting: the left replica holds {} \
+             messages and the right {}, so the session raced nothing",
+            left.snapshot().len(),
+            right.snapshot().len(),
+        );
 
         let (mut a, mut b) = rumors::link::memory_with_capacity(LINK_CAPACITY);
         let (left_result, right_result) = tokio::join!(left.gossip(&mut a), right.gossip(&mut b));
```

<!-- annotation -->
> **tests-resource-link-window-28** (T28), line 218:
>
> The racing session is held to have widened (`window_granted > 1`), the guard that keeps `GROWTH_BUDGET` honest: if the pre-charge ever grows past it, this fails rather than the doc silently describing a derivation that does not run.

<!-- annotation -->
> **tests-resource-link-window-28** (T28), line 227:
>
> The mid-session-growth witness, the resolution's first form: every racing commit is on the left replica, only those before the greeting's snapshot reached the right one, so `left > right` exactly when some commit landed mid-session. Negative control: awaiting `race` to completion before the `join!`, run once and restored, fails here with 8096 = 8096 (recorded verbatim in a3e4594b).

<a id="hunk-57"></a>
### tests/window_corners.rs `@@ -193,8 +237,8 @@ fn growth_during_a_session_only_serializes() {`

```diff
@@ -193,8 +237,8 @@ fn growth_during_a_session_only_serializes() {
         right_result.expect("follow-up gossip right");
     });
     assert_eq!(
-        left.snapshot().len(),
-        right.snapshot().len(),
+        left.snapshot(),
+        right.snapshot(),
         "the follow-up session converges everything the race added",
     );
 }
```

<!-- annotation -->
> **tests-resource-link-window-28** (T28), line 240:
>
> The finish is snapshot equality rather than length equality: `Snapshot` already derives `PartialEq`, so finding 27's form fell out of the witness for free, as the brief allowed.

