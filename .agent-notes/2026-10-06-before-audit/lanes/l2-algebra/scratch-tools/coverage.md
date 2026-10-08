# Coverage record: lane L2, version algebra

Revision: explore branch `explore/l2-algebra` at `d78c6129` (base `58285ca5`;
production code unchanged from base). Every run was on ox-east-1 through
`on-illumos.sh /Users/oxide/src/rumors-audit-l2-algebra 'unset CARGO_TARGET_DIR; …'`.
Logs are in this directory.

## What each contract clause rests on

| Clause | Committed instruments | What I added (explore only) |
|---|---|---|
| join/meet are pointwise max/min, canonical | `lattice/tests.rs` (oracle + pointwise walk; families, arb depth 4, organic, wide grids, staircases, exhaustive small scope); `diff_ops`; laws | model differential on deep, wide, correlated pairs: all 6 operator cells per direction, `|=`/`&=` owned and borrowed, chained results |
| encoding no longer than both operands | `assert_output_fits_inputs` inside every lattice differential (bits) | bit and byte bounds on every probe pair, decomposition families included; proof sketch below |
| lattice laws incl. distributivity | `laws::VERSION_PAIR`, `VERSION_TRIPLE` | implied by model equality (the model is pointwise max/min) |
| all comparison paths agree, incl. shared storage | `order/tests.rs` (`assert_verdicts`), laws | every spelling (owned/borrowed mixes, `concurrent` both ways) on model pairs, on clones (`ptr_eq` path), re-decodes, and span-decoded endpoints sharing one allocation |
| Hash consistent with Eq | law `version_eq_implies_hash_eq` | hash on every equal pair (clone, re-decode) |
| folds equal pairwise application | `laws::VERSION_LIST`, `VERSION_AND_LIST` (arity 0..=17, depth-4 pools), `tests/fold_skeleton.rs` | `join_all`/`meet_all`/`span_all` owned and borrowed, `Sum`, `collect`, up to 40 items from a pool with clones, re-decodes, adjacent repeats, empty |
| projection is restriction | `diff_ops` (3-way), `laws::VERSION_PARTY*`, `own/tests.rs` | materialization (`to_version`, `From`) and every view comparison cell (view/version both orders, view/view, `concurrent`) against model restriction, parties to depth 300 |
| shape iterators render the step function | `diff_ops` shape rows, `shape/tests.rs` | exact rises and depths for `shape`, `combine` (N = 0, 2, 3), `Clock::shape`, `Party::shape` |
| no recursion on input depth | `clock/tests.rs` deep tests (depth 100k) | depth 100k for shape walks, concurrent hull, `Sum`/`collect`, masked views (see machinery brief) |
| span placement verdicts (L5's theme) | L5 | `place`/`dominance`/`precedence`/`contains` vs model relations over point spans (shared and unshared endpoints) and hulls |

## Runs

- `algebra_matches_model`, 256 cases per run, passing at every revision.
  Latest: `run9.log`, exit 0, 48 s.
- Long runs: `long1.log` (16 × 2000 = 32,000 cases at `2e557419`, all PASS)
  and `long2.log` (16 × 1400 = 22,400 cases at `d78c6129`, all PASS,
  exit 0).
  Command:
  `PROPTEST_CASES=… PROPTEST_MAX_SHRINK_ITERS=… nice -n 10 cargo nextest run --profile high-count -p before --locked --test l2_probe --test-threads 16 --no-fail-fast --run-ignored only -E "test(/^par/)"`.
- Census (`census1.log`, 2,000 deterministic tapes): relation outcomes Less
  399, Equal 522, Greater 312, concurrent 767. Depth of `a`: 0 (241), 1–4
  (401), 5–16 (642), 17–64 (357), 65+ (359). Families evenly spread. Writer
  paths among the 2,000 joins: direct wide collapse 375, cascade wide 329,
  narrow cascade after a split 167; meets similar. Projections: all four
  relations, 1,298 of 2,000 non-trivial.

## Calibration (the probe can fail)

- 20 string-swap mutants across `lattice.rs` (switch-delta arithmetic, side
  sign), `io/writer.rs` (cascade guard, wide take-left flag, split-stream
  take-left, split-stream direct collapse, direct depth check, left-is-leaf),
  `projection.rs` (leave-ownership step, unowned sign, block-skip difference
  sign, block-skip height integrators, skip boundary, others-deepest),
  `order.rs` (`<` on equal), `version.rs` (fold merged–merged arm, `span_all`
  hull arm), `version/shape.rs` (refinement tie), and `overlay.rs` (pair
  tie). The probe killed all 20 (`mut/round1/summary.txt`,
  `mut/round2-probe/summary.txt`). Each kill is a model assertion or a
  production debug assertion on the mutated path. After every mutant,
  `git diff --quiet` confirmed the tree was restored (`restored=True` in each
  summary).
- The committed `before` suite (`--all-features`, excluding my probes) also
  killed all 20 (`mut/round2-suite/`; each log names its first failing
  committed test). So the probe catches no first-order mutant the committed
  suite misses, and I propose no new semantic instrument.
- The deep-surface test: a one-frame-per-level recursion over the same
  depth-100k version aborts with a stack overflow on a 2 MiB thread
  (`run7.log`).

## Cost readings (`cost1.log`, `cost2.log`; touch and scan meters, sizes ×1, ×2, ×4, ×8)

- Carry ripple (heights alternating `2^W` and `2^W − 1`, `W` equal to the
  leaf count): join, meet, span, `partial_cmp`, `<=`, `>=`, `concurrent`,
  `join_all`, and every masked comparison stay flat per input bit
  (for example join: 0.285, 0.287, 0.287, 0.288 touches per bit).
- Fragmented masks (comb parties with `n` regions inside one `2^n`-wide
  plateau): every masked comparison is flat per bit. `project` grows linearly
  per input bit, as `O(|input| + |output|)` predicts for an output of about
  2n² bits: its scan count tracks the output. That matches the documented
  `|result| = O(|self|^2)`.
- Subadditivity of the encoding. Every output boundary is an input boundary,
  so union leaves and internal nodes number at most those of the two inputs
  together, less the shared root. At each boundary the output delta lies
  between the two input deltas, since pointwise max and min are 1-Lipschitz.
  Zigzag-gamma length depends only on magnitude and grows with it. So each
  output code is no longer than an input code at the same boundary, and
  `bits(a ∘ b) < bits(a) + bits(b)` strictly. The byte bound follows. A pair
  with disjoint deep supports (a's structure on the left half, b's on the
  right) attains it to within a few bits. This argument is mine; the probe
  checks the inequality on every case.

## Residual blind spots

- 32-bit: not executed beyond the committed `wasm32-pins` (join output and
  comparison past 2^32 bits). The box has no 32-bit native target installed.
  By reading the code: every byte index in `BitsWriter` and the readers is a
  `u64` position divided by 8 before narrowing, and all narrowing casts in
  the writer act on values of at most 64.
- Random sizes stop near 900 splits and depth 300. Depth 100k is reached
  only for spine shapes.
- Debug builds only. There is no `unsafe`, and release builds only remove
  checks.
- I mutated by hand, at 20 sites. The adequacy lane's cargo-mutants
  survivors in `src/version/{lattice,order,projection,own,overlay,shape}.rs`,
  `src/fold.rs`, and `src/shape.rs` would sharpen this.
- The accumulator's internals (suanpan, L7), `range_minima` (used only by
  `tick`, L3), and `place/filter` (`causally`, L5) were only exercised
  through this lane's entry points.

## How to resume

- Probe: `cargo nextest run -p before --locked --test l2_probe` (add
  `--run-ignored only -E "test(/^par/)" --profile high-count` with
  `PROPTEST_CASES` for long runs; `-E "test(census)" --no-capture` for the
  census).
- Cost: `cargo nextest run -p before --features touch-meter,scan-meter --locked --test l2_cost --run-ignored only --no-capture`.
- Mutants: `python3 mutate.py <mutants.json> '<remote command>' <outdir>`
  from this directory. It restores each file and checks `git diff --quiet`.
