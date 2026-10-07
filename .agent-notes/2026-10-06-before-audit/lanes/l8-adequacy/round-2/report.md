<!-- CAVEAT LECTOR: the round-2 report of the L8 (adequacy) auditor, Claude Opus 5.5, filed and condensed by the coordinator because the harness refuses report files from subagents. Measurements are the auditor's claims unless the coordinator's notes say otherwise. -->

# Lane L8, round 2: report

The explore branch is `explore/l8-adequacy` at `4684d9ad`. The sibling
directories hold the briefs, the leads write-up, and the survivor index with
per-group diffs.

## Campaign

cargo-mutants 27.1.0 over the full nextest suite of `before` or `suanpan`
(`--all-features`, `PROPTEST_RNG_SEED=8008`), excluding `src/testing/` and
test files. Exit code 2 means "completed, some mutants missed"; no group
timed out.

| group | mutants | caught | missed | unviable | status at hand-back |
|---|---:|---:|---:|---:|---|
| count.rs (pilot) | 57 | 41 | 3 | 13 | done |
| version | 1,596 | 1,301 | 79 | 216 | done |
| rank | 341 | 271 | 47 | 23 | done |
| span | 306 | 103 | 29 | 174 | done |
| rest | 113 | 45 | 36 | 32 | done |
| suanpan | 411 | 355 | 30 | 26 | done |
| bits | 628 | 249 | 10 | 10 | 269 of 628 written |
| party | 467 | 193 | 15 | 69 | 277 of 467 written |

Over the 3,370 written outcomes the suite kills 91.1% of viable mutants.
The recipe for classifying the rest of bits and party is at the end of
`survivors/INDEX.md`.

## Classification of the 249 survivors written

G (a reachable value change no test detects) 21; T* (trait behavior callers
rely on) 3; T (trait spelling only) 39; C (cost only) 84; E (equivalent) 53;
U (unreachable) 38; I (instrument) 10; ? 1. Per-group counts and every
survivor's diff are under `survivors/`.

## Briefs

- `briefs/machinery-range-minima-near-boundaries.md` (new): 2 G in
  `Boundary::lower_wide`'s fast paths, witnessed through `RangeMinima`'s own
  operations, not yet through a public-API input.
- `briefs/machinery-rank-decode-reader.md` (amended): 11 G in the rank
  decoder's `Interrupted` guards and length check; requires bounded reads so
  retry-forever mutants fail rather than hang.
- `briefs/test-span-lone-endpoint-padding.md` (extended): 2 survivors where
  a length check precedes the first field's padding check (`span/wire.rs:136`,
  `party/io.rs:29`); the base returns `TrailingBits`, the mutant `Truncated`.
- `briefs/machinery-suanpan-normalize-bound.md` (unchanged): 6 G.
- `briefs/machinery-trait-impl-coherence.md` (amended): narrowed to `Hash`
  content dependence (3 T*); drops the operator, `Debug`, and `size_hint`
  laws.
- `briefs/machinery-disjoint-families.md` (new): rebalances
  `arb_party_family`, whose multi-input `join_all` never succeeds (0 of about
  17,700 draws at arity 2 to 17). A reach gap with no demonstrated failure.

## The four leads (`findings/coordinator-leads.md`)

1. The `PeakAlloc` flake comes from libtest's main thread allocating inside
   the measurement window, not from a concurrent test. Cheapest fix:
   `RUST_TEST_THREADS=1` for nextest runs.
2. `STOPPING_DIFF_BAND`'s rise (7,872 to 8,892) is `fdd1bf47`'s debug-only
   `clone().cmp_zero()` in `Boundary::from_positive`. The ×1.25 ceiling
   admitted a one-touch-per-hop rise its message says it exists to catch;
   a ceiling in hops would have caught it.
3. nextest LEAK flags are harmless: on illumos a just-spawned sibling holds
   the test's pipe ends until it execs. Remedy: `leak-timeout = "1s"`.
4. `arb_party_family`'s skew: see the disjoint-families brief.

## Judgment

Mutation testing cannot see cost regressions (84 C survivors, against an
uncalibrated board; the one calibrated case, a quadratic fold, passed the
floor-only asymptotics pins), meter flakiness, loose band ceilings, or
generator reach. Recommended, in order: fix the `PeakAlloc` race,
re-denominate `STOPPING_DIFF_BAND` in hops, calibrate the board against a
handful of C mutants, and run the wasm32 pins against the value-changing
`accumulate` mutants. Beyond these, diminishing returns.

## Coordinator's notes

- The `?` survivor (`version/io/writer.rs:255`) is the early return that
  `simplify/codec-cleanups` (#67) deletes; its reviewer verified the branch
  dead structurally.
- The padding brief's two survivors change only which `Decode` variant a
  doubly malformed input returns. `Decode`'s contract permits either, as the
  codecs review of #67 found for the same pattern; they are not built unless
  the owner documents a precedence.
- The `accumulate` survivors disappear with `simplify/rank-single-alignment`
  (#63), which deletes `Rank::accumulate`.
