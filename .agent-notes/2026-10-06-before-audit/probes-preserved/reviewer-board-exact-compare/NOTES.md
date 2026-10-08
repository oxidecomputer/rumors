# reviewer-board-exact-compare: resumption notes

Task: review proposal/board-exact-compare in /Users/oxide/src/rumors-slot-43 at d703a35f
(base 8b28bbd81). Verified HEAD d703a35f, clean, signed (G). Box runs allowed: 3. No commits.

## Established
- trend_units extraction: identical expression (judge.rs); board 5311 green + pin clean in
  builder's landing.log (= landing-2-race.log prefix) at d703a35f. Only failure:
  representation_space.rs:56 4729 vs 4609 (PeakAlloc race); run-race-alone.log passes.
  7e3f9ca2f differs from d703a35f only in one test doc comment (git diff verified); its
  landing (landing-1-doclint.log) shows tests ok (count not printed).
- Demo: B->A 33 uneven/1 constant; rendered heap exponent max move 0.26 over 34 cells
  (heapexp.py). A->C: 2 growing; meet-shade rendered e 0.00 unchanged.
- Truncation proptest catches an end-line-optional parser only ~63% per 256-case run
  (prefix_sim.py).
- Doc inaccuracies: "sign never changes" (code allows growth from zero); opposite-sign
  shift bullet claims always uneven (false when rate dominates at first growth size);
  validation_index "or only shifted" overclaims and mixes tense.

## Temporary edits (restore with `git apply -R run1.patch` after formatting? NO: fmt ran
before saving, so run1.patch is the exact diff; restore via `git apply -R run1.patch`,
then confirm `git diff` empty)

## Runs
- run1: clippy + compare tests + probes -> run1.log (background)
- run2 (planned): end-line-optional mutant; exhaustive test must fail.
