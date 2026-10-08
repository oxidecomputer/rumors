<!-- CAVEAT LECTOR: written by Claude (Opus 5.5) as the instrument rescue's integrator, collecting the corrections sections 01 to 09 found in the audit's records, and its own; for Finch's review. -->

# 18. Corrections to the audit's records

The cataloguers checked the records they relied on, and found statements
that are wrong or that claim more than the evidence shows. This section
collects them in one list, grouped by the record they correct. Each item
names the section that found it; that section gives the evidence and its
mark. Items marked *verified here* I checked again myself.

None of these changes a ready branch. Three change a conclusion the owner
might act on, and they come first.

## Corrections that change a conclusion

1. **The specification codec does catch the two padding survivors, at low
   rates.** Survey section 2.1 says the codecs lane's harness enforces only
   span's documented precedence and so misses the survivors at
   `span/wire.rs:136` and `party/io.rs:29`. The harness admits only the
   first-detected error class in three places (a clock's or span's first
   field, every borsh stream verdict, a rank header past the format bound),
   which is stricter than `Decode`'s contract. On the distinguishing input
   it rejects the survivors' `Truncated`, and its generators produce that
   input once in 703 clock inputs and once in 10,000 span inputs (06,
   entry 1). Folding it in as written would pin today's undocumented
   detection order (decision 2 in the main file).
2. **A wasm32 differential over suanpan histories exists.** Survey section
   2.2 says none exists and that random small traces cannot reach the
   32-bit defects. The stability-width fixer built one that varies the
   query width rather than the value's width, and it failed 498 of 4,800
   cases at the defect's base (09, A14).
3. **`main`'s `min_ticks` exceeds the board's heap ceiling at sizes the board
   does not sample.** On code identical to `main`'s, 177 of 756 readings of
   right spines exceed `1,024 + 20` bytes per input byte, up to 307.67
   (09, A4). Question 87 stopped the fix; this is the widest measurement of
   that open defect.

## The integrator's baseline (`00-baseline.md`)

Each is corrected in the baseline itself.

4. Section 4.1 said no committed test states `min_ticks` as a floor over
   histories. `min_ticks_floors_every_history` (`version/tests.rs:682`)
   states the weaker floor of the whole history's total ticks, on final
   clocks only (03; verified here, the source at `main`).
5. Sections 2.6 and 2.7 relayed "committed queries hold at most two holes".
   The committed random grid property reaches three concurrent hole bounds
   in 8 up-polar checks per 256 cases, on two cells only (05).
6. Section 3 said the bridge writes versions through `BitsWriter`. It writes
   a tree stream through `BitsWriter`, then converts it to production's
   stored form through production's own `VersionWriter`
   (`testing::version::from_tree_stream`) (02; verified here, `bridge.rs`
   and `testing/version.rs`).
7. Sections 2.6 and 2.7 relayed that suanpan's surface generator never parks
   digits at the representation's bound. It reaches a digit of magnitude
   `2^33 − 1` in 202 of 1,000 programs, but such digits meet `normalize` in
   only 2.2% of `normalize` steps and a shift in 1.7% of shifts (07). The
   same "never" appears in `follow-ups.md`, under "Bound-digit states
   reaching other operations".
8. Section 3 described the function-space oracle's limit only as
   `GRID_N = 32`. That cap governs trace replay; the differential table's
   legs assert a grid under 64 levels instead, and every scan visits `2^g`
   cells (verified here, `function.rs`). Sections 01 and 02 each cite one
   of the two limits; both hold.

## The integrator's handoff notes

9. The ranker's handoff attributed #38's value pool to the adequacy lane. It
   follows the suanpan lane's observation O2 (08). The handoff notes are
   working files in the scratchpad and were not corrected; this entry is
   the correction.

## `instruments.md` and the instrument survey

10. The identity generators reach depth 80, not "about 120": `GEN_DEPTH = 80`
    caps every cut point (verified here,
    `explore/l1-identity:crates/before/tests/audit_l1/gen.rs`). Section 01
    measured depth from 65 to 80 in 23% of draws.
11. The spans lane's "depth 17 to 64 for 338 of 771 shaped versions" quotes a
    histogram bucket label; the generator caps depth at 40 (05).
12. Survey section 2.1's "the algebra probe caught none of 18 survivors" says
    little: 15 of the 18 change no value by the adequacy lane's own
    classification (02).
13. Survey section 2.1 and `instruments.md` cite stale commits for four
    lanes (the survey says so itself); the sections read every branch at its
    tip.

## The lanes' records

### Identity (01)

14. M26 was not unique to the deep probe: five `amp_board_smoke` tests catch
    it at the base.
15. Observation O8's "32 or more two-child branches in 42%" misreads a
    bucket: 17 or more in 42%, 33 or more in 20.5%; depth above 32 is
    46.75%, not 48%.
16. The fork-step scan cost is about 10,000 + 6d bits, not 4,000 + 6d; still
    linear.
17. The cost probe's `is_disjoint` row reads 0.00 at every size: both
    operands own `[0, 1/2)`, so the comparison stops at the first leaf, and
    the row measures nothing.

### Algebra (02)

18. `census1.log` predates the probe's 900-split, depth-300 tier, so the
    records' census numbers describe an earlier four-tier generator. Section
    02 re-measured at the tip.
19. Only 9 of the probe's 20 round-1 kills came from its model; production
    debug assertions made the other 11.
20. In the deepest size tier, 97.7% of fold draws and 64.0% of first-party
    draws read an exhausted tape, so the deepest operands are rarely folded.

### Events (03)

21. The calibration's "committed" filter re-admitted four lane probes whose
    names contain `min_ticks`, so the committed set was 68 tests, not 72.
    The list of escapes is unaffected.
22. The defect record says the jump-entered heap breach disappears at a jump
    of `2^16`; at 98,304 levels that control reads 26.27 bytes per input
    byte, above the ceiling, from the plain spine's doubling.
23. #74's entry calls one of its mutants "M21"; it is #74's own label for a
    different defect from the lane's M21. M19 was not rerun after #74's
    second round changed its palette (also survey section 5, item 7).

### Codecs (06)

24. The coverage record's party depth above 8 is 14.6%, not 15.6%.

### Suanpan (07)

25. The pool model did not catch M6: both logged runs show it passing. The
    failure the records cite belonged to the explore touch property and was
    a pricing false alarm, fixed 40 minutes later.
26. Uncompacted cancellation is as rare in the pool model (0.10% of steps)
    as in the committed surface property (0.14%); the pool model's advantage
    is depth, operand histories, width, and extreme digits.

### Adequacy (08)

27. `findings/wasm32-pins-calibration.md` calls the `2^20` trap probe a
    positive control, but it passed, so it shows only that no position at or
    past `2^20` reaches the probed sites.
28. The family census found no multi-input success in 17,781 draws, not
    "about 17,700".
29. The campaign's final kill rate is 91.5% of viable mutants; the round-2
    report's 91.1% was taken partway.

### Build and review probes (09)

30. #81's ready entry declines 09.A1 "per your caution", and #83's narrowing
    removed 09.A2's laws under the coordinator's scope correction. Under
    notice 88 both are already-built instruments, and both rank in the top
    five here.

## The instruments' own defects

These are faults in instruments, to repair in any fold-in.

31. 05.1's antichain properties return without checking anything in 2% to
    8% of cases; a fold-in should construct at least one hole directly.
32. 05.3's doc comment claims the census and witness oracles agree, but it
    never calls the witness oracle; 05.21, that oracle, is dead code.
33. 05.14's doc comment claims a set intersection the code does not
    enumerate.
34. 02.8's totals line sums three of six counters and prints zeros.
35. 06.4's failing-reader oracle accepts either outcome, the same weakness
    #78's reviewer found and repaired in #78.
36. 06.8 carries a `prop_assume!` that never rejects.
37. 03.4 leaks its deep trees with `mem::forget` instead of dropping them on
    its large-stack thread.
38. 09.A6's grouping mislabels ceilings that share a value.

## Committed prose found wrong in passing

Each is a correction toward the code for the second documentation pass.

39. `version/measure/tests.rs` lines 198 to 201 and 588 to 589 say
    `arb_magnitude` tops out near `2^128`; it reaches 514 bits. The
    conclusion still holds, since no arbitrary pair defers (04).
40. `ReferenceBitsReader`'s doc calls it "a deliberately bit-at-a-time
    bounded reader", but its `read_gamma` tries the window first (08).
41. The committed `Count` conversion test compares `Count`'s conversion with
    the same `num-bigint` call it wraps, so it checks the wrapper, not the
    ranges (04, entry 8; 04.8 is the independent check).
42. The September triage's `rank-20` exercise has no committed successor;
    the consolidation `193a14744` deleted it (04, entry 2; 04.2 restores
    it).
