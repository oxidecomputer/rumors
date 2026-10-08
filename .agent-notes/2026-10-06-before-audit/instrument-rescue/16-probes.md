<!-- CAVEAT LECTOR: written by Claude (Opus 5.5) as the instrument rescue's integrator, generated from one ranked list of every instrument in sections 01 to 09; for Finch's review. -->

# 16. Probes and one-off tools

These are one-off tools and settled investigations. They are listed so
that nothing the audit built goes uncounted; none adds coverage to fold
in.

Each row condenses the linked entry in sections 01 to 09, which gives the
full account and marks every figure as verified, reported, or inferred; a
row's figures carry the linked entry's marks. "Rank" orders this category;
"Overall" is the instrument's position in the single ranking in
[`../instrument-rescue.md`](../instrument-rescue.md), section 3, and both
rank by the coverage an instrument adds against the work of folding it in.
`#n` is entry `n` of `QUESTIONS.md`; "slot 23", "slot 26", "slot 33",
"slot 45", "slot 46", and "slot 47" are ruled branches not yet ready. Where two lanes
built overlapping instruments, [`17-overlaps.md`](17-overlaps.md) says which
belong together or which subsumes which.

This category holds 13 of the 159 rows.

| Rank | Overall | Instrument | Adds beyond `main` and the ready branches | Evidence | Fold-in work and runtime | Depends on |
|---:|---:|---|---|---|---|---|
| 1 | 124 | [09.A28](09-build-and-review-probes.md) Probe of the integral's dense-span bound | #72's derived bound is attained within one. | 2,063 against a bound of 2,064. | A `debug_assert!` would need `S` threaded through. | #72. |
| 2 | 134 | [09.A32](09-build-and-review-probes.md) Board recipe interrupt behavior on a toy justfile | Regression evidence for #93's trap. | Above. | No harness for recipes exists. | #93. |
| 3 | 141 | [05.20](05-spans.md) Tie-order experiment | The `overlay.rs:142` swap is equivalent for placement and filters. | Supports the equivalence. | None. | None. |
| 4 | 148 | [08.14](08-adequacy.md) Split-writer hunt | Random ticks never reach an early return #67 deletes. | Two runs of 300,000 found nothing. | None. | #67. |
| 5 | 149 | [08.15](08-adequacy.md) libtest invocation probe | How nextest invokes a test binary on the box. | Grounded the heap-race diagnosis. | None. | None. |
| 6 | 151 | [07.8](07-suanpan.md) Prototype T (position tags) | A partial design for F1. | Design evidence only. | Not applicable. | None. |
| 7 | 152 | [07.10](07-suanpan.md) Touch replay probe | Diagnosed a shared-meter false failure. | Above. | Not applicable. | None. |
| 8 | 153 | [03.15](03-events.md) Bridge round trip on one pair | None beyond every bridged differential. | A sanity check. | Not worth folding. | None. |
| 9 | 154 | [02.12](02-algebra.md) Parallel long-run copies `par00` to `par15` | A way to run 02.2 long. | 54,400 cases. | None; `PROPTEST_CASES` serves. | None. |
| 10 | 155 | [02.15](02-algebra.md) `run_on_tree.sh` | Runs the probe on an uncommitted merge. | Used once. | None. | None. |
| 11 | 156 | [02.16](02-algebra.md) `extract.py` | Converts survivor records to patches. | Mangled one patch. | None. | None. |
| 12 | 157 | [07.11](07-suanpan.md) Suanpan scratch drafts and table scripts | Drafts whose final forms are other entries. | Produced the F1 tables. | None. | None. |
| 13 | 158 | [09.A35](09-build-and-review-probes.md) Instruments superseded by a ruling or design | Strict serde tests (`9123a4d8`), protocol sweeps (`789936bc`), a panic classifier, an adapted probe. | Superseded. | None. | None. |
