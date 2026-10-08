<!-- CAVEAT LECTOR: written by Claude (Opus 5.5) as the instrument rescue's integrator, generated from one ranked list of every instrument in sections 01 to 09; for Finch's review. -->

# 15. Diagnostics and meta-instruments

These instruments measure or calibrate other instruments: censuses, mutant
sets, survivor records, and calibration harnesses. Their value is
calibration of whatever folds in, not new coverage of the crates. Almost
every mutant set edits production source and has no committed home;
decision 3 in the main file asks where such sets should live.

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

This category holds 42 of the 159 rows.

| Rank | Overall | Instrument | Adds beyond `main` and the ready branches | Evidence | Fold-in work and runtime | Depends on |
|---:|---:|---|---|---|---|---|
| 1 | 2 | [09.A1](09-build-and-review-probes.md) Exhaustive agreement test for the laws' multiplicity comparison | Every family of at most three parties of depth at most 2 (3,616 families), checked against the tree oracle; #81's sampled property passes a wrong comparison at 4,096 cases. | Fails every comparison mutant the reviewer tried; 0.42 s unmutated. | Apply one written diff to #81's tests. 0.42 s. | #81 (on #40). |
| 2 | 17 | [08.1](08-adequacy.md) Generator census (`l8_census.rs`) | The only measurement of generated pair relations, projection outcomes, trace degeneracy, populations, and family success; floors would hold those classes. | Deterministic; reproduced at `main`; source of the baseline. | Floors beside `generator_classes_stay_under_mass`, read degenerate operations from the driver. A few seconds at 3,000 draws. | None. |
| 3 | 36 | [04.7](04-measures.md) `DEFER_HITS` tap and coverage census | Shows a test reaches the deferred-height reduction rather than merely a freeze; committed floors count freezes only. | Source of the lane's reach numbers. | Four `cfg(test)` lines in `integral.rs` and deferral floors where tests claim deferral. | None. |
| 4 | 42 | [05.3](05-spans.md) Census validator (`l5_census_is_exact_against_a_finer_universe`) | An executable check of the exactness argument every exact-coverage test relies on, committed grid included. | Never calibrated; its doc claims a witness check it does not make. | About 60 lines with 05.2 or 05.4; correct the doc. 0.15 s. | None. |
| 5 | 52 | [09.A6](09-build-and-review-probes.md) Ceiling-rule auditor | Recomputes each board ceiling from its stated rule; nothing committed compares the two (survey 1.4). | Found COMB's and QUERY's drift; one grouping bug. | Survey 1.4's Rust form in `run_acceptance`, using the Python as reference. No extra sweep. | Slot 26's one-rule branch (on #53). |
| 6 | 53 | [09.A7](09-build-and-review-probes.md) libtest main-thread race probe | The only reproduction of the heap meters' race; refuted the single-thread fix. | Decided question 91's premise. | Planned as slot 47's calibration. 0.54 s. | Slot 47 (shared counting allocator). |
| 7 | 54 | [09.A5](09-build-and-review-probes.md) Fuel comparison between two commits | Makes every constant-factor fuel change visible and attributable per commit; the bands admit about 1.58-fold slack and nothing compares commits. | Found three unrecorded movements that no band failed. | A fuzz-fit capture and compare mode modeled on #95. Developer tool; no gate time. | None (#95 for a shared format). |
| 8 | 87 | [08.4](08-adequacy.md) Classified mutation survivors (282) | A baseline for later campaigns; 96 cost-only mutants as the cost instruments' first calibration set; 110 written equivalence and unreachability arguments. | Every lane-8 brief came from it. | A record; committing it as a baseline needs your ruling (section 1 of the main file). | #43, #63, #66, #67, #78, #82, #83 kill or delete survivors. |
| 9 | 88 | [08.5](08-adequacy.md) Mutation campaign tooling | The only total measure of what the suite misses (91.5% of viable mutants killed). | Produced 08.4. | A recipe for a phase boundary or release; about 11 hours a run. | None. |
| 10 | 89 | [09.A19](09-build-and-review-probes.md) Compile-time mutant switches (`option_env!`) for cost-only survivors | Showed three cost-only survivors escape every committed meter, fuel bands included; a template for switches that compile to nothing. | Above. | A template for committing mutants, or survey 1.9's ladder cell instead. | #52. |
| 11 | 90 | [09.A12](09-build-and-review-probes.md) Mutant schema for `Rank::decode`'s reader paths | Shows #78's oracle rejects a permissive window and accepts a correct early-stopping decoder. | Above. | A test-only copy of the decoder, about 100 lines. | #78. |
| 12 | 91 | [09.A16](09-build-and-review-probes.md) Mutant schema for suanpan's readout carry classes | 12 of 22 class mutants fail only #64's table. | All 22 fail #64. | A production switch with no committed home. | #64. |
| 13 | 92 | [09.A17](09-build-and-review-probes.md) Stack-recursion mutants and frame-size probes | Which deep test alone detects each recursion; why the depth is `2^18`. | PAMT passes at 100,000 levels and fails at `2^18`. | High: one aborting run per mutant. | #34, #75. |
| 14 | 93 | [09.A18](09-build-and-review-probes.md) Mutants of suanpan's written-position set | #86's calibration; the truncation mutants need about 130 of 256 cases. | All eight fail #86's model test. | A production switch with no committed home. | #86. |
| 15 | 94 | [07.3](07-suanpan.md) Suanpan mutant schema and calibration scripts (27 mutants) | Mutant forms cargo-mutants does not generate, and a kill matrix by instrument. | Backs MB1, MB2, F1c. | Re-derive against `main`; twelve target code #86 deletes. | #86, #89. |
| 16 | 95 | [03.6](03-events.md) Events calibration mutant set (M1 to M23) | A ready calibration set for the pre-scan, memo, and `min_ticks`; four escaped at the base. | The evidence behind #74. | The switch edits library code; patches are against the base. | None. |
| 17 | 96 | [01.12](01-identity.md) Identity mutant lists and calibration driver | 24 semantic mutants (duplicated error groups, loops turned recursive). | Backed MB1 to MB3. | Needs a known-bad mechanism the project lacks. | None. |
| 18 | 97 | [02.13](02-algebra.md) Algebra round-1 mutants and `mutate.py` | Nine are the only demonstration that 02.1's model, not production's assertions, detects errors. | Committed suite killed all 20. | No code; keep as 02.1's calibration. | None. |
| 19 | 98 | [04.11](04-measures.md) Integrator mutant patch (12 mutants) | A calibration set for 04.1, 04.3, and 04.10. | All 12 killed by both harness and suite. | Applies only at the explore tip. | 04.7's tap. |
| 20 | 99 | [04.17](04-measures.md) `Rank` mutants R1 to R7 and E8 (source not kept) | E8 is the known-bad 04.2's fold-in needs. | Calibrated 04.6. | Rebuild from the descriptions. | None. |
| 21 | 100 | [05.17](05-spans.md) Spans calibration mutants (M1 to M20, table only) | A known-bad list for the spans fold-ins. | Committed suite caught all 17 non-equivalent ones. | Rebuild from the table, about an hour. | None. |
| 22 | 101 | [06.10](06-codecs.md) Decoder mutation schemas (M1 to M17) | Data on thin committed checks: M6 rests on one test, M14 and M15 on two. | Every mutant failed a committed test. | Keep as data; the anchors rot. | None. |
| 23 | 102 | [09.A36](09-build-and-review-probes.md) Calibration sets for carried tests (24 sets) | The evidence behind each ready entry's calibration claims. | Reported per entry. | Needs a committed home for production mutants. | Their target branches. |
| 24 | 110 | [09.A21](09-build-and-review-probes.md) Board-cell bisection and allocator event log | A commit and a mechanism for any heap movement. | Traced COMB's rise to `63d01d903`. | A recipe; the log needs `unsafe` or question 91's hooks. | Question 91. |
| 25 | 111 | [08.8](08-adequacy.md) Probe copy and hit recorder | Which tests reach a marked line, without a profiler. | Shaped #82's brief. | Maintainer tooling; the marked sites are lost. | None. |
| 26 | 112 | [08.9](08-adequacy.md) Branch-coverage map | 97.8% of lines and 95.4% of branch outcomes; every finding routed. | Located five gaps. | Mac or CI only. | None. |
| 27 | 113 | [01.11](01-identity.md) Identity generator statistics printers | Two-child branch counts and unary chains, which the census lacks. | Produced observation O8. | Add two metrics to the census. | None. |
| 28 | 114 | [02.8](02-algebra.md) Writer-path census | Counts writer collapse paths and code widths. | Two flaws: a wrong totals line, first pair only. | Reach floors for 02.3. | 02.1. |
| 29 | 115 | [03.13](03-events.md) Events generator histograms | The only record of how rarely independent pairs reach tick's pre-scan regimes. | Numbers behind #74. | Its committed-generator row as a floor. | None. |
| 30 | 116 | [05.16](05-spans.md) Committed-population reach probe (spans) | The only measurement of the committed span population's placement mix. | Not applicable. | A placement-mix floor. | None. |
| 31 | 117 | [06.12](06-codecs.md) Codec reach histograms | Depth and violation rates of 06.5's inputs. | Produced 06.1's reach numbers. | Floors for 06.5 if folded. | 06.5. |
| 32 | 120 | [09.A31](09-build-and-review-probes.md) Scan for self-recursive test helpers | Retargeted at library code, a mechanical check of the no-recursion rule (direct calls only). | Led to question 68. | A small script. | Question 68. |
| 33 | 121 | [08.11](08-adequacy.md) Surface check end-to-end calibration | A planted item through the extractor. | Two of two caught. | A fixture crate and nightly in tests. | None. |
| 34 | 122 | [08.12](08-adequacy.md) Fold-pin calibration | The log-factor pins check only a floor. | A quadratic fold passes all five pins. | A board sweep on the mutant. | None. |
| 35 | 123 | [08.10](08-adequacy.md) Counter bisect harness | Attributes one reading's movement to a commit. | Attributed a touch rise to `fdd1bf47`. | A `git bisect run` predicate. | None. |
| 36 | 130 | [02.9](02-algebra.md) Stack-overflow calibration | A minimal recursion overflows at depth 100,000 on 2 MiB. | Abort verified. | Needs a child process. | None. |
| 37 | 139 | [06.14](06-codecs.md) Writer mutants and reach probes | Showed an early return cannot run. | Its finding is #67. | None. | None. |
| 38 | 140 | [02.14](02-algebra.md) Survivor and tie-order patches with `run.sh` | Tie order cannot change a verdict. | 3,200 cases each. | None. | None. |
| 39 | 143 | [03.14](03-events.md) Memo reach diagnostic | Superseded by #74's census. | Set the wide strategy's regime claims. | None. | None. |
| 40 | 145 | [05.19](05-spans.md) Spans behavioral histograms | Every reach number in the lane's records. | Not applicable. | Not as written: a global mutex. | None. |
| 41 | 146 | [07.7](07-suanpan.md) Readout carry census (`l7-high-log`) | Which readout classes a suite reaches. | Found the gap #64 closed. | A file-writing feature does not fit. | #27, #64. |
| 42 | 147 | [07.9](07-suanpan.md) Native size probe | Encoded sizes of the fuel families. | Supplied F1's denominators. | None; #52 records the same. | None. |
