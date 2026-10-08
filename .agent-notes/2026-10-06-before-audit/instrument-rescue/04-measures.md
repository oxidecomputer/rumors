<!-- CAVEAT LECTOR: written by Claude (Opus 5.5) as the measures-lane cataloguer for the instrument rescue, from the explore branch, the lane's records, and two runs on ox-east-1; for Finch's review. -->

# 04. The measures lane (L4): every instrument it built

## Summary

The measures lane audited `Rank`, `Ranked`, `Count`, and the version
measures (`rank`, `distance`, `lag`, `rank_cmp`, `encode_rank`). Its explore
branch, `explore/l4-measures` at `2c82c7bac` on base `58285ca51`, adds eight
signed commits and 1,369 lines across eight files (verified, `git log` and
`git diff --stat`). Two more artifacts live outside the branch, in the
auditor's scratch directory (`<scratchpad>/auditor-l4/`): a patch of
integrator mutants, and the logs of mutants whose source was not kept
(verified).

This file gives 17 instruments a full entry. Two more are already carried by
ready branches and get one line each:

- The wasm32 `Sum` cases (prototype cases 5 to 8) became #29,
  `audit/wasm32-rank-sum-pin`, test
  `rank_sum_straddles_the_usize_alignment_limit` (verified, ready entry and
  branch diff).
- The panic message recorded after a wasm32 trap (the message half of the
  prototype's `run_diagnosed`) became #31, `audit/wasm32-trap-diagnosis`
  (verified).

The lane's other deliverables are prose and simplifications, not
instruments, and are carried too: the width-invariant routing by #63, the
normalization prose and two other prose items by #85, and the checked
`span * 4` by #72 (verified, ready entries).

What the lane's instruments add, in brief:

- **Reach.** The multi-scale partition generator (entry 1) drives arbitrary
  inputs through the integrator's deferral path and produces exact rank ties
  between distinct versions at freezing scale. No committed arbitrary or
  organic pair does either (my census, below).
- **Independent oracles.** The flat sweep oracle (entry 3) computes rank,
  distance, lag, and the sign of `rank_cmp` without the paper's recursion or
  a sampling grid, in time linear in the leaf count, so it checks inputs
  3,000 levels deep. The rational `Rank` oracle (entry 5) reaches integral and
  even-integer ranks, which the committed `RANK_TRIPLE` driver never builds.
- **Cost coverage the committed suite lost.** The September fix that made
  rank sums linear in every summand order came with a meter for its
  adversarial order, and the consolidation of `before`'s resource
  instruments deleted that meter (verified, entry 2). The lane's sum-order
  probe is the only instrument built that exercises that order today.
- **Evidence.** No semantic harness of the lane caught a defect, and the
  committed suite also killed every mutant the auditor built (reported,
  coverage record). The `Rank` value property (entry 6) exercises one
  operator the whole committed suite misses: the adequacy campaign's
  survivor at `rank.rs:989`, which replaces `AddAssign<&Rank>` with `()`,
  would fail it (inferred by reading; not run).

### The measurements this file rests on

I made two runs on ox-east-1, a build and a test run, in a scratch worktree
detached at the explore tip with a census module added to each explore file.
The logs are `<scratchpad>/rescue-l4/box1-build.log` and `box2-run.log`, and
the census code is `<scratchpad>/rescue-l4/rescue-census.patch`, which applies
at the explore tip. Between the explore base and `main` (`dbc169291`), the
lane's production files, the shared generators, the trace driver, and the
bridge are unchanged; the code diff touches only the board, two lines of an
exhaustive-suite test, the wasm32 pins, and suanpan's limb index (verified,
`git diff --stat 58285ca51 main`). So the
census describes `main`'s generators as well. All 15 tests in the run passed,
at a load of 28 to 37 on 192 hardware threads (verified).

For each ordered pair, the census runs `rank`, `distance`, and `rank_cmp`, the
operations the lane's own coverage test runs, and records whether the
integrator froze (`FREEZE_HITS` rose), deferred a nonzero parked height
(`DEFER_HITS`, the lane's tap, entry 7), deferred three or more times, or
found an exact rank tie between distinct versions (verified, census source):

| Generator (deterministic runner) | Pairs | Froze | Deferred | Deferred 3+ | Distinct-version rank ties | Equal pairs | Depth p50 / p90 / max | Height bits p50 / max |
|---|---|---|---|---|---|---|---|---|
| L4 `arb_leaves(64)` with `arb_related` | 2,000 | 86.5% | 50.8% | 26.1% | 29.1% | 9.7% | 12 / 86 / 130 | 2,019 / 2,209 |
| Committed `arb_oracle_version` pairs (`arbitrary_trees_agree` and the law drivers) | 2,000 | 24.0% | 0 | 0 | 0 | 0 | 3 / 3 / 4 | 128 / 513 |
| Committed organic histories, every ordered pair (`organic_histories_agree`) | 6,342 | 0 | 0 | 0 | 1.6% | 34.8% | 2 / 4 / 7 | at most 6 (00-baseline §2.4) |

The L4 row reproduces the auditor's recorded census exactly: 1,730 froze,
1,015 deferred, 522 deferred three or more times, and 581 tied, of 2,000
(reported, box log `sweep3-none.log`, which I read). The lane's deep setting
(`L4_DEEP_MAX=3000`, `L4_K_MAX=6000`) keeps those rates while reaching about
3,000 levels and 6,000-bit heights (reported, `deep-1.log`: 86% froze, 53%
deferred, 27% tied).

For `Rank` values, 20,000 pairs from each generator:

| Generator | Integral, nonzero | Even integral | Zero | Equal-value pairs | Same-exponent pairs | Exponent p50 / max | Numerator bits p50 / max |
|---|---|---|---|---|---|---|---|
| L4 `arb_q` with `arb_related_q`, after its parse route | 33.3% | 24.5% | 6.8% | 30.0% | 43.0% | 9 / 5,172 | 135 / 940 |
| Committed `stream_rank`, the `RANK_TRIPLE` driver, copied verbatim | 0 | 0 | 0 | 0 | 0.01% | 50,050 / 99,996 | 64 / 16,832 |

`stream_rank` forces every numerator odd and draws exponents uniformly below
100,000, so it yields an integral rank once in 100,000 draws and an even
integer never (verified, source). My census takes pairs from one word stream
instead of one stream per seed, which leaves each draw's distribution
unchanged. Committed text tests do reach integral ranks, since
`canonical_rank_text` generates integers up to 129 bits (verified, source);
the arithmetic, order, hash, and encoding laws do not.

Every lane test runs in 8.3 seconds or less at its default case count (256
for the proptests), against nextest's 300-second limit on `main` (verified,
my run; 00-baseline §8). Each entry gives its own figure.

## Entries, in order of my judgment of value

### 1. The multi-scale partition generator and its related-pair modes

- **What it is.** A proptest generator in
  `explore/l4-measures:crates/before/src/version/measure/explore_l4.rs` at
  `2c82c7bac`, about 230 lines: `SplitAt`, `depths_of`, `Step`, `apply`,
  `heights_of`, `arb_k`, `arb_step`, `arb_split`, `arb_leaves`,
  `arb_related`, and the larger `arb_leaves_large` (verified).
  - A version is a dyadic partition grown from one leaf by repeated splits
    of a random, first, last, or second-to-last leaf, optionally after a
    same-side spine of 32 to `L4_DEEP_MAX` splits (default 120). Its absolute
    heights come from a script of twelve step kinds: small moves, `±2^k`,
    `2^k − 1 + s`, one full 32-bit digit at a digit boundary, plateaus,
    drops to zero, returns to an earlier height, `±(2^k − 2^(k/2))`, bit
    flips, and near-cancellations. The width `k` comes from five bands: 0 to 69, 250 to
    329, 500 to 699, 900 to 1,199, and 2,000 to `L4_K_MAX` (default 2,200).
  - The second version is independent, perturbed (same partition, heights
    moved), refined (leaves split, children near the parent's height), a
    single-leaf edit, the mirror image (leaf order reversed, so the area is
    equal), or a rotation of heights among leaves of one depth (also equal
    area).
- **What it reaches or checks.** It reaches freezing in 86.5% of pairs,
  deferral in 50.8%, three or more deferrals in 26.1%, exact rank ties
  between distinct versions in 29.1%, and equal pairs in 9.7%, with depth up
  to 130, up to 175 leaves, heights up to 2,209 bits, and encodings up to
  58,888 bytes (verified, census). The deep setting reaches about 3,000
  levels and 6,000-bit heights (reported). It checks nothing itself; it is
  input.
- **Coverage beyond the committed suite.** 00-baseline §2.7 lists equal
  independent pairs, and anything deeper than 4 under an oracle, among the
  regimes the committed generators never produce; this generator produces
  both. The census adds two regimes the baseline does not measure: no
  committed arbitrary or organic pair defers, and none ties in rank at
  freezing scale (verified). The committed suite reaches deferral and
  freeze-regime ties only through named shapes: the deferral pool, arming
  trains and their mirrors, the cancellation and zero-drift families, the
  jump and concurrent pairs, and one 800-level staircase mirror for
  `Ranked::cmp` (verified, `version/measure/tests.rs` and
  `version/tests.rs`). The committed laws on `Ranked`'s order, its encoding
  order, prefix-freedom, the valuation identity, strict monotonicity, and
  the distance and lag identities (`laws::VERSION_PAIR` and `VERSION_SOLO`)
  run only on the arbitrary and organic drivers (verified,
  `algebraic_laws.rs`'s module doc). No in-tree driver has evaluated them on
  a deferring pair or on a tie at freezing scale.
- **Evidence.** It caught no defect. Through the flat-oracle property
  (entry 10) it carried 70,000 cases plus 600 deep ones without a failure
  (reported, coverage record). The deep-spine prefix mattered once: the
  harness missed mutant M25 (a width read without its scale) until the
  prefix was added (verified, `mut-5.log` reads `M25 suite=100 harness=0`;
  `sweep3-M25.log` shows the harness failing after). The committed suite
  also killed all 12 integrator mutants (reported, `sweep2-suite-*` logs).
- **Fold-in cost.** Move the strategies into `testing::generators` beside
  `arb_oracle_version`. Then add the generator as a third input for the law
  drivers in `algebraic_laws/tests.rs` and for `assert_single` and
  `assert_pair` in `version/measure/tests.rs`, which already check the tree
  oracle and the composed forms; that fold-in needs no new oracle. Building
  and measuring 2,000 pairs takes 8.3 seconds (verified), and 256 cases
  through the lane's predicates take 3.2 seconds (verified). At depth 130
  the recursive tree oracle is inside its envelope, which warns only of
  spines thousands of levels deep, and a committed test already runs it at
  depth 512 (verified, its "Operating envelope" doc; 00-baseline §2.7). The
  deep setting builds versions through the recursive bridge, so after the
  stack guard's removal (the owner's ruling on question 68) it must run on a
  thread with an explicit stack size, as that ruling prescribes. No new
  dependencies. Each strategy needs a rustdoc stating its reach, and a
  census floor beside `generator_classes_stay_under_mass` would keep the
  freeze, deferral, and tie rates from collapsing unnoticed; a deferral
  floor needs the tap in entry 7.
- **Overlaps.** Entries 7, 10, 12, and 13 draw from it, and entry 9's
  probes ran through entry 10. The algebra lane's
  tape generators reach depth 300 and heights past the word size, with
  correlated pairs aimed at collapse cascades (reported, `instruments.md`);
  I did not measure their freeze or deferral rates. Its mirror and rotation
  modes generalize the committed tie families to arbitrary shapes
  (observation O4, reported). It would stand beside the committed
  generators and replace none.
- **Dependencies.** None for the default setting. The deep setting depends
  on the stack-guard removal, which is ruled but not yet a branch.
- **Value, in one sentence.** It is the only generator built that sends
  arbitrary inputs, rather than named shapes, through the deferral path and
  the exact-tie path, so feeding it to the law drivers would extend every
  committed version law to a regime no in-tree driver reaches.

### 2. The `Rank` sum-order cost probe

- **What it is.** A cost probe, `l4_rank_sum_order_cost` in
  `explore_l4_rank.rs`, about 60 lines (verified). It builds the wide integer
  `2^W − 1` and the `k = 2·√W` unit fractions `2^−1` through `2^−k`, sums
  them in four orders (wide then ascending exponents, ascending then wide,
  wide then descending, and the wide value before every unit), and prints
  suanpan digit touches per byte of value content at `W` = `2^12`, `2^14`,
  `2^16`, and `2^18`. It asserts nothing.
- **What it reaches or checks.** Wide-then-ascending reads 0.53, 0.50, 0.48,
  and 0.47 touches per content byte across the four sizes; ascending-then-wide
  falls from 0.30 to 0.22; the interleaved order holds at 0.49 to 0.50
  (verified, `sumcost-1.log`). The cost is flat or falling over a 64-fold
  range of `W`.
- **Coverage beyond the committed suite.** The September triage closed
  `rank-20` as "make rank sums linear in their combined value content
  regardless of summand order, and exercise ascending fractional scales
  after a wide numerator" (verified,
  `2026-09-16-before-triage-plan/checklist.md`). That exercise was
  `rank_sum_mixed_envelope` in `tests/meter.rs`, added by `4bf25c154` and
  deleted by the consolidation `193a14744` (verified, `git log -S`). The
  board's `rank_sum` row, the only remaining sum meter, adds one family rank
  and then small integers (verified, `board/ops.rs`), so no later summand
  has a larger exponent and the held value never shifts. A regression of
  `sum_iter`'s amortized shift to shifting by exactly the gap (the auditor's
  mutant E8) would therefore pass every committed instrument (inferred). In
  this probe's wide-then-ascending order, that regression shifts the whole
  `W`-bit value once per fraction, about `k·W/32` digit touches, against
  about `3W/8` content bytes (the wide value's `W/8` plus the fractions'
  `k²/16 = W/4`). That is about `k/12` touches per byte: roughly 11, 21, 43,
  and 85 across the four sizes, doubling at each step, where the code reads
  0.5 today (inferred, by arithmetic; the content sizes match the logged
  1,560 to 98,496 bytes). The `Sum` rustdoc promises `O(k + n)` time
  (verified, `rank.rs`).
- **Evidence.** It caught nothing, and the closed fix holds (reported). Its
  sensitivity to E8 was never run: `mut-E8.log` holds no cost line
  (verified).
- **Fold-in cost.** It can be a focused check in `tests/meter/`, with a
  ceiling on touches per content byte and a liveness floor at the four
  sizes; the probe takes 0.32 seconds (verified). Or it can be a board
  family whose `rank_sum` operands ascend in exponent, which adds cells and
  may move the worst-case ranking, so that path stops for the owner under
  the audit's rules. The ceiling must come from the mechanism (each
  full-width shift at least doubles the occupied span), not from today's
  reading. E8 is the known-bad input that shows the check can fail. No new
  dependencies.
- **Overlaps.** The board's `rank_sum` row, which uses a different order,
  and the lane's `Sum` value properties (entries 13 and 14), which check
  values only. I found nothing similar in the other lanes' inventories.
- **Dependencies.** None. #28 and #63 change the reservation in
  `Rank::accumulate`, not `sum_iter`, and #85 changes only `sum_iter`'s
  comment (verified, branch diffs).
- **Value, in one sentence.** It is the only instrument built that exercises
  a closed fix's adversarial summand order, which the consolidation dropped
  and the board cannot see.

### 3. The flat sweep oracle for the version measures

- **What it is.** An oracle in `explore_l4.rs`, about 120 lines: `Leaf`,
  `tree_of`, `version_of`, `leaves_of`, `flat_rank`, `segments`,
  `flat_pair`, `rank_is`, and `to_unsigned`, plus the cross-check
  `l4_flat_oracle_agrees_with_tree_oracle` (verified).
- **What it reaches or checks.** A version is a list of `(depth, height)`
  leaves. The rank is `Σ h·2^(S−d) / 2^S` at `S`, the maximum depth. The pair
  measures merge two partitions at their common scale and integrate
  `|a − b|` (distance), `max(0, b − a)` (`a.lag(b)`), and `a − b` (the sign
  of `rank_cmp`) with plain `BigInt` arithmetic. `rank_is` compares a
  production rank with `num / 2^exp` by cross-multiplication and also
  requires the stored normal form: a zero numerator only with exponent zero,
  and an odd numerator whenever the exponent is positive. The oracle shares
  no code with the tree oracle, the function oracle's grid, or the
  integrator. It does build production versions through the bridge
  (`from_oracle_version`), which writes through production's `BitsWriter`
  (00-baseline §3), and its arithmetic uses num-bigint, as production does.
  The cross-check agrees with the tree oracle's rank on 500 shallow cases in
  0.56 seconds (verified).
- **Coverage beyond the committed suite.** The committed oracles for these
  measures are the recursive tree oracle, which overflows on deep inputs by
  design; the Riemann sum, whose grid has `2^g` points and so covers only
  small inputs; and the composed forms, which use production's own join,
  meet, and rank (verified, 00-baseline §3 and `oracles/function.rs`). The
  flat sweep is linear in the leaf count and does not recurse, so it judged
  600 cases with spines up to 3,000 levels deep (reported). 00-baseline
  §2.7 records that nothing generated deeper than 128 is checked against any
  committed oracle.
- **Evidence.** Through entry 10 it killed all 12 integrator value mutants,
  the same 12 the committed suite kills (reported). The three
  schedule-forcing probes (entry 9) pass under it (verified, logs). It
  caught no defect.
- **Fold-in cost.** It would join `testing::oracles` as a third model of
  versions, beside `tree` and `function`. The algebra lane's leaf-list model
  uses the same representation, `(depth, height)` leaves with absolute
  heights, and implements canonicalization, join, meet, the causal relation,
  projection, and a canonical encoder, but no measures (verified,
  `explore/l2-algebra:crates/before/tests/l2_probe/model.rs`). The two
  belong together, and that encoder would let deep versions be built
  without the recursive bridge (inferred). Runtime: 256 cases take 3.2
  seconds (verified). In the deep setting, 600 cases took 102.5 seconds
  outside nextest, and a larger run hit nextest's former 180-second limit
  (verified, box logs `run4.log` and `deep-1.log`; the case counts, 600 and
  1,500, are reported in the lane's `NOTES.md`). The validation index needs
  an entry for it. No new dependencies.
- **Overlaps.** The algebra lane's leaf-list model, which it complements.
  Within the lane, entry 10 is its property, and entries 6 and 13 reuse its
  `rank_is` and `flat_rank`. It would stand beside the committed oracles as
  an independent second check, and replace none.
- **Dependencies.** None for the default setting; the deep setting as in
  entry 1.
- **Value, in one sentence.** It is the only oracle built for rank,
  distance, lag, and `rank_cmp` that is independent of the paper's
  recursion and still runs on inputs thousands of levels deep.

### 4. The `Ranked::cmp` tie-cost probe

- **What it is.** A cost probe, `l4_ranked_cmp_tie_cost` in `explore_l4.rs`,
  about 65 lines, run on a thread with a 1 GiB stack because its mirror
  passes through the recursive oracle (verified).
- **What it reaches or checks.** It hangs a committed shape and its mirror
  under a fresh root fork, which gives two distinct versions of exactly
  equal rank, for arming trains (`n` = 4, 8, 16, 32) and plateau punctures
  (`s` = 200 to 1,600). It measures touches and densified digits per byte
  for `Ranked::cmp` and for the two rank folds, and asserts that the ranks
  tie and the verdict falls to the version bytes. Arming trains read 4.68,
  5.00, 5.24, and 5.36 touches per byte for the comparison (1.07, 1.05, and
  1.02 times per doubling) against 2.98 to 3.50 for two rank folds; plateau
  punctures read a flat 4.17 against 2.30 (verified, `tiecost-1.log`).
- **Coverage beyond the committed suite.** The September triage closed
  `rank-33` as "give `Ranked::cmp` its attainable worst-case contract and
  exercise the exact comparison on the settle-triggering families"
  (verified, checklist). At `main`, the board's `ranked_cmp` row compares
  each family's version with that version plus one seed tick (verified,
  `FamilyData::version2`'s doc), which never ties in rank, and no meter
  under `tests/meter/` calls `Ranked::cmp` or `rank_cmp` (verified, grep).
  The committed suite checks the values of exact ties (mirrored arming
  trains for `rank_cmp`, the staircase mirror for `Ranked::cmp`) but not
  their cost.
- **Evidence.** It caught nothing; the closed fix holds (reported).
- **Fold-in cost.** A focused check in `tests/meter/settle_flatness.rs`,
  bounding the comparison's touches relative to two rank folds at four
  sizes, with a floor. Or a board pair family whose second version is the
  mirror, reaching `rank_cmp`, `ranked_cmp`, `distance`, and `lag`, which
  adds cells and may move the worst-case ranking, a stop for the owner. The
  probe takes 5.9 seconds (verified). It already runs on an explicit-stack
  thread, the pattern the stack-guard ruling prescribes. No new
  dependencies.
- **Overlaps.** The committed tie value tests, and entry 1's mirror mode,
  which produces ties for values.
- **Dependencies.** None.
- **Value, in one sentence.** It is the only instrument built that meters
  the settle-to-zero comparison that a closed fix bounds.

### 5. The rational `Rank` oracle and its generators

- **What it is.** An oracle and a generator in `explore_l4_rank.rs`, about
  170 lines: `Q` (with `cmp`, `add`, `sub`, and `rank`), `render`,
  `arb_num`, `arb_exp`, `arb_q`, and `arb_related_q` (verified).
- **What it reaches or checks.** `Q` holds a raw `(num, exp)` meaning
  `num / 2^exp`, compares by cross-multiplication, and never normalizes.
  `render` writes the canonical binary text of `num / 2^exp` without
  production code, and `Q::rank` builds the production rank by parsing that
  text. Numerators include zero, small values, `2^k` and `2^k ± 1` at
  `k` = 31 to 33, 63 to 65, 127 to 129, 191 to 192, and 255 to 256, random
  values up to 80 bytes, shifted values, and shifted runs of ones; exponents
  include 0, word boundaries, and values up to 5,000. The related modes are
  an unnormalized respelling, a low-bit flip, a tiny offset far below the
  value, the same magnitude class with a different word count, and the same
  class with an independent mantissa. The census reach is in the table
  above: a third of ranks integral, a quarter even integers, and 30% of
  pairs equal in value (verified).
- **Coverage beyond the committed suite.** The committed rank references
  are `alignment_cmp`, which judges order only; the pairwise fold, which is
  production's own `+`; the laws, which relate production to itself; and the
  text round trip, which compares parsing with display (verified,
  `version/tests.rs`). The text round trip would pass a parser and printer
  that err consistently, except at the handful of values the committed
  tests fix (four parsed, three displayed) (inferred, by reading
  `rank_text_known_values_and_boundaries` and `rank_known_values`). The rational oracle judges values of `+`, `checked_sub`, `Sum`,
  and order by arithmetic that never touches production, and `render` maps
  values to text independently of both parser and printer. Its inputs are
  the regime `RANK_TRIPLE`'s driver never builds: integers, zero, and equal
  values with different spellings.
- **Evidence.** The auditor's mutant R2 (comparing raw numerators within a
  magnitude class) survived the first version of the generator and was
  caught once the same-class independent-mantissa mode was added; the
  committed suite catches R2 in 26 tests (reported, coverage record). It
  caught no defect.
- **Fold-in cost.** `Q` and `render`, about 60 lines, into
  `testing::oracles`; `arb_q` and `arb_related_q` into `testing::generators`.
  Then a second `RANK_TRIPLE` driver beside `stream_rank` would put every
  rank law, including #83's `rank_debug_is_display`, on these inputs.
  20,000 census pairs took 6.3 seconds (verified), so 256 cases cost well
  under a second. No new dependencies.
- **Overlaps.** `stream_rank` complements it: huge exponents and odd
  numerators there, small exponents and integers here. The codecs lane's
  specification codec checks rank bytes against a specification; this
  oracle checks the order those bytes induce. Entries 2, 6, 13, and 14 use
  it.
- **Dependencies.** None. It would also drive #83's new law once #83 lands.
- **Value, in one sentence.** It gives the rank laws the inputs they never
  see: integers, even integers, zero, and equal values spelled differently.

### 6. The `Rank` value property

- **What it is.** A property, `check_values` with `l4_rank_values_agree` in
  `explore_l4_rank.rs`, about 90 lines, 256 cases by default (`L4_CASES`)
  (verified).
- **What it reaches or checks.** For each pair: `parse(render(q))` equals the
  `from_raw` route and is in normal form; `Ord`, `Eq`, and `Hash` on equal
  pairs; `&a + &b`, owned `b + a`, and `a += &b` against the oracle's sum;
  `checked_sub` and `saturating_sub` in both arms; encoding order, also with
  random suffixes of up to 5 bytes; the decode round trip; `encode_to`
  against `encode`; `Display` and `Debug` against `render`; and `Display`
  under three widths without precision and three width-and-precision
  pairs, each in left, right, and center alignment, with the default fill,
  `*`, and the multibyte `é`, against `str`'s formatting of the same text.
- **Coverage beyond the committed suite.** Formatter flags are tested on one
  committed value, `101.01` (`rank_formatting_behaves_as_text`), and by #83's
  law on one flag combination (verified). `AddAssign<&Rank>` is called by no
  committed test: the adequacy campaign lists it as survivor rank-42, missed
  by the whole suite (verified, survivor `diffs/rank.md`). No committed test
  asserts `Rank`'s normal form directly; it holds by construction through
  `from_raw` and through the decoder's `debug_assert!` (verified, grep).
- **Evidence.** It killed R1 and R3 to R7, and R2 after the generator was
  strengthened; 40,000 cases passed in all (reported). It would kill
  rank-42, because it compares `acc += &b` with the oracle's `a + b` and the
  mutant leaves `acc` equal to `a` (inferred; not run).
- **Fold-in cost.** A property in `version/tests.rs` beside the rank text
  tests, over entry 5's generator. The flag check could instead become a
  `RANK_TRIPLE` law ("`Display` with flags equals `str` formatting of the
  plain rendering"). 256 cases take 0.17 seconds (verified). Its `Debug`
  check duplicates #83's law.
- **Overlaps.** `RANK_TRIPLE`'s codec, order, and cross-path laws, on other
  inputs; #83's `rank_debug_is_display`.
- **Dependencies.** None.
- **Value, in one sentence.** It alone tests `Rank`'s formatter flags as a
  family and exercises `AddAssign<&Rank>`.

### 7. The `DEFER_HITS` tap and the lane's coverage census

- **What it is.** A test-only counter in
  `crates/before/src/version/measure/integral.rs` (four added lines: a
  thread-local beside `FREEZE_HITS`, incremented in `defer_parked`), and the
  diagnostic `l4_generator_coverage` in `explore_l4.rs` (about 30 lines),
  which prints the freeze, deferral, three-or-more deferral, tie, and
  frozen-tie counts over 2,000 cases (verified).
- **What it reaches or checks.** It observes whether the integrator defers.
  The census takes 8.3 seconds (verified).
- **Coverage beyond the committed suite.** Every committed liveness floor in
  the measure tests counts freezes (verified, grep for `FREEZE_HITS` in
  `version/measure/tests.rs`). Freezing does not imply deferral: 24% of
  committed arbitrary pairs freeze and none defers (verified, census). A
  change that stopped the deferral pool or the arming trains from deferring
  would therefore satisfy every committed floor while the deferred reduction
  went unexercised (inferred). For example,
  `parked_cancellation_settles_and_defers_exactly` says the "settlement and
  deferral paths it exists to drive" are alive, but its floor counts
  freezes (verified). 00-baseline §8 notes that the committed census
  measures shape and relation mix, not semantic events such as these.
- **Evidence.** It is a diagnostic and never fails. It produced the reach
  numbers this file uses.
- **Fold-in cost.** Keep the four tap lines in `integral.rs` beside
  `FREEZE_HITS`, add deferral floors where a test claims deferral, and turn
  the census into a floor test for entry 1's generator. A floor test of a
  few hundred cases would take about a second (inferred from 8.3 seconds
  for 2,000). It changes a production file, under `#[cfg(test)]`.
- **Overlaps.** The adequacy lane's census tooling measures shapes, not
  integrator events (00-baseline §1 and §8). Entry 9 needs the same kind of
  hook.
- **Dependencies.** None.
- **Value, in one sentence.** It is the only way to show that a test reaches
  the deferred-height reduction rather than merely a freeze.

### 8. The `Count` boundary sweep

- **What it is.** An exhaustive test, `l4_count_boundaries` in
  `explore_l4_rank.rs`, about 60 lines, 0.05 seconds (verified).
- **What it reaches or checks.** It takes 0, 1, and `MAX − 1`, `MAX`, and
  `MAX + 1` of `u8`, `u16`, `u32`, `u64`, `u128`, and `usize`, 20 values, and
  builds each from its limbs through public arithmetic. For each: the limbs
  equal the value's 64-bit digits and the iterator's exact size is honest;
  `Display` equals the decimal; `parse` round-trips; each `TryFrom`
  succeeds exactly when the value is at most the type's `MAX`, judged by a
  `BigUint` comparison; `From<u128>` and `From<usize>` round-trip; and
  `checked_sub(1)` behaves at the neighbors. It also rejects ten
  non-canonical texts.
- **Coverage beyond the committed suite.** The committed
  `unsigned_conversions_match_their_ranges` compares `Count`'s conversion
  with num-bigint's `try_from` on the same value (verified), and `Count`'s
  conversion is that same call (verified, `count.rs`), so the committed test
  checks the wrapper, not the ranges. Its generator, `arb_magnitude`, cannot
  draw `MAX − 1` or `MAX` of `u8`, `u16`, `u32`, or `u128`, and draws each
  of their `MAX + 1` values about once in 27,000 draws (inferred from the
  strategy: only the `(2j + 1)·2^k` arm reaches `2^8`, `2^16`, `2^32`, and
  `2^128`, at weight `1/13 · 1/4 · 1/512`). The committed rejection test,
  `count_rejects_noncanonical_decimal`, covers every class in the sweep's
  list but two: a `0x` prefix and a non-ASCII digit (the Arabic-Indic `١`)
  (verified, both lists).
- **Evidence.** It caught nothing.
- **Fold-in cost.** It drops into `count/tests.rs` as it is. On a 64-bit
  host the `usize` row duplicates the `u64` row (observation O8 discusses
  the 32-bit difference). No new dependencies.
- **Overlaps.** The committed conversion, limb, and text properties, which
  sample other values.
- **Dependencies.** None.
- **Value, in one sentence.** It checks every conversion boundary
  exhaustively against an independent judge, which the committed property
  can neither reach nor judge independently.

### 9. The schedule-independence probes E13, E14, and E15

- **What it is.** Three environment-selected switches in the integrator that
  force it to always defer, always freeze, or never freeze. Their source was
  not kept: it is absent from the saved patch, the branch, and the box
  (verified). Logs show the flat-oracle property passing under each
  (verified, `mutlogs/mut-E13.log`, `mut-E14.log`, `mut-E15.log`).
- **What it reaches or checks.** It shows that the integrator's values do
  not depend on its freeze and deferral schedule, at 1,024 cases each
  (reported, coverage record).
- **Coverage beyond the committed suite.** No committed test varies the
  schedule. The adequacy lane classified the survivors at
  `version/measure/integral.rs:230` (two), `:253`, and `min_ticks.rs:69`
  (two) as cost-only on the strength of this result (verified, survivor
  `INDEX.md`: "values schedule-independent; lane L4 checked all three
  schedules").
- **Evidence.** Each probe passes, as it must (reported).
- **Fold-in cost.** Each switch is one condition in `integral.rs`, and they
  would need rebuilding. As a committed check they would become a
  metamorphic property: a forced schedule computes the same rank,
  distance, lag, and `rank_cmp` as the natural one. That needs a test-only
  schedule override in a production file. The brief excludes new
  construction; this would be reconstruction of an instrument the lane
  built and ran.
- **Overlaps.** Entry 11 holds the value mutants of the same integrator;
  entry 7 needs a similar test-only hook.
- **Dependencies.** None.
- **Value, in one sentence.** It is the evidence behind five survivor
  classifications, and as a property it would check the freeze and
  deferral machinery against the path that uses neither.

### 10. The pair predicate set over the flat oracle

- **What it is.** A property, `check_pair` with
  `l4_measures_agree_with_flat_oracle` in `explore_l4.rs`, about 130 lines,
  256 cases by default (verified).
- **What it reaches or checks.** On entry 1's pairs: rank of both versions;
  distance, lag, and `rank_cmp` in both orders; `Rank::cmp`; `Ranked::cmp`
  equal to rank order then byte order; `Ranked::encode` in that order, also
  with two fixed suffix pairs appended; `encode_rank` equal to
  `rank().encode()` and ordered under suffixes; causal order implying
  strict rank order, strict `Ranked` order, and a one-sided lag; distance
  zero exactly on equal versions; and the valuation law on production join
  and meet. Every value is judged against entry 3.
- **Coverage beyond the committed suite.** Most of these predicates are
  committed laws: `rank_is_a_valuation`, `rank_strictly_monotone`,
  `distance_symmetric`, `distance_separates`, `lag_halves_sum_to_distance`,
  `lag_zero_iff_dominated`, `ranked_orders_by_rank_then_bytes`,
  `ranked_encoding_orders_like_ord`, `version_encoding_is_prefix_free`, and
  `ranked_carries_own_rank` (verified, law names). Order under suffixes
  follows from prefix-freedom and lexicographic order (inferred). What
  remains its own is exactness against entry 3, in entry 1's regime.
- **Evidence.** 70,000 cases plus 600 deep ones passed (reported); it killed
  the 12 integrator mutants, M25 only after the deep prefix (verified,
  logs).
- **Fold-in cost.** As the property joining entries 1 and 3 in
  `version/measure/tests.rs`, keeping only the exactness checks once entry 1
  feeds the law drivers. 256 cases take 3.2 seconds (verified).
- **Overlaps.** The committed laws above, and `assert_pair`.
- **Dependencies.** None for the default setting.
- **Value, in one sentence.** It holds little of its own beyond entries 1
  and 3, whose combination it is.

### 11. The integrator mutant patch

- **What it is.** `mutants-integral.patch` in `<scratchpad>/auditor-l4/`,
  6,458 bytes (verified). It adds `l4_mutant(name)`, which reads `L4_MUT`,
  and 12 value mutants switched by name: M2 parks before settling; M3, M4,
  and M31 drop or misplace deferred widths; M5 merges on the wrong side; M6
  and M29 drop a jump's shift; M9 skips the orientation reversal; M15 skips
  a lone deferral; M23 loses a deferred height's sign; M25 reads a width
  without its scale; M30 skips a negative opening (verified, patch text;
  the descriptions are reported, coverage record).
- **What it reaches or checks.** One build serves all 12 mutants, selected
  at run time.
- **Coverage beyond the committed suite.** None as a check: it is a
  calibration set, and the committed suite kills all 12 (reported).
- **Evidence.** For every mutant, both the flat-oracle property and the
  committed suite fail (verified, `mut-5.log`: each line reads
  `suite=100 harness=100`, except M25 at `harness=0` before the deep
  prefix).
- **Fold-in cost.** It applies at the explore tip and not at `main`, because
  its context includes the `DEFER_HITS` lines (verified, `git apply --check`
  on both trees). As committed known-bad demonstrations it would need
  test-only switches in a production file; the survey cut that kind of
  check for instruments without a failure (reported, survey section 4). Its
  practical use is to calibrate entries 1, 3, and 10 when they are folded
  in.
- **Overlaps.** The adequacy campaign's cargo-mutants survivors, none of
  which changes an integrator value (verified, survivor `INDEX.md`).
- **Dependencies.** Entry 7's tap, for the patch to apply.
- **Value, in one sentence.** It is a ready calibration set for any future
  integrator instrument.

### 12. The integrator cost search

- **What it is.** A cost probe, `l4_cost_search` in `explore_l4.rs`, about
  60 lines, over `arb_leaves_large` (up to 400 splits and 200 height steps)
  and its related pairs; 300 samples; 5.7 seconds (verified). For `rank`,
  `distance`, `lag`, and `rank_cmp` it prints the worst touches and
  densified digits per input byte, binned by input size. It asserts
  nothing.
- **What it reaches or checks.** The worst reading is 4.0 touches per byte,
  on one- and two-byte inputs; above that it reads at most 3.6, and 0.6 to
  1.4 on 0.3 to 2.4 MB inputs in the deep setting; densified digits stay at
  most 0.002 per byte (verified, `cost-1.log` and `cost-2.log`, which print
  both quantities times 1,000).
- **Coverage beyond the committed suite.** The board meters chosen families
  against a ceiling of 22 touches per byte (00-baseline §5.1). The fuzz-fit
  bands meter random programs, but by their own doc never build wide
  magnitudes (00-baseline §5.3). So this is the only random search over
  wide multi-scale shapes. Touches do not count `BigUint` work, so its flat
  readings say nothing about total cost (observation O5, reported).
- **Evidence.** It caught nothing.
- **Fold-in cost.** A metered property over entry 1's generator with a
  touch ceiling, run in its own process because the meters are global
  (nextest gives each test one). No new dependencies.
- **Overlaps.** The board's `rank`, `distance`, `lag`, and `rank_cmp` rows;
  the fuzz-fit bands.
- **Dependencies.** None.
- **Value, in one sentence.** It searches random shapes, at magnitudes that
  neither the board's chosen families nor the fuzz-fit programs build, but
  it only prints what it finds.

### 13. The seven-route rank property

- **What it is.** A property, `l4_version_rank_is_the_sum_of_leaf_areas` in
  `explore_l4_rank.rs`, about 35 lines, 0.96 seconds (verified).
- **What it reaches or checks.** For a partition from `arb_leaves(40)`, the
  version's rank equals the flat oracle and equals each of: the `Sum` of its
  per-leaf areas `h / 2^d`, each built by parsing independently rendered
  text, in a shuffled order; the pairwise fold of the same areas; distance
  to the empty version; lag from the empty version; decode of encode; parse
  of display; and `ranked().rank()`. All seven agree on `Eq`, `Hash`, `Ord`,
  encoding, and text.
- **Coverage beyond the committed suite.** No committed test compares a
  version's rank with a sum of its leaf areas (verified, grep), so the first
  two routes tie the integrator to `Rank` arithmetic for the first time.
  Distance and lag against the empty version are committed on the product
  family (`arbitrary_factors_embed_their_product_in_exact_rank`, verified),
  and the codec, text, and `Ranked` routes are laws on depth at most 4.
- **Evidence.** It caught nothing.
- **Fold-in cost.** A property in `version/measure/tests.rs` over entry 1's
  generator, using entry 5's renderer or `Rank::from_raw` for the areas.
- **Overlaps.** Entry 3, the committed laws named above.
- **Dependencies.** None.
- **Value, in one sentence.** It gives every version's rank a second route
  through `Rank` arithmetic, cheaply.

### 14. The `Sum` property over any order

- **What it is.** A property, `l4_rank_sum_any_order` in
  `explore_l4_rank.rs`, about 20 lines, 0.23 seconds (verified).
- **What it reaches or checks.** For 0 to 9 ranks from `arb_q` in a shuffled
  order, `Sum<&Rank>` and `Sum<Rank>` equal the rational oracle's sum, and
  the pairwise fold equals `Sum`.
- **Coverage beyond the committed suite.** The committed
  `rank_sum_equals_the_pairwise_fold` sums up to 24 `stream_rank` values
  against production's own fold (verified). This adds an independent
  reference and entry 5's integral, zero, and equal inputs.
- **Evidence.** It caught nothing.
- **Fold-in cost.** A second strategy for the committed property.
- **Overlaps.** The committed property; entry 5.
- **Dependencies.** None.
- **Value, in one sentence.** Its value is low: it adds a second reference
  for a property the committed suite already states.

### 15. The wasm32 `Sum` footprint and replay cases (prototype cases 9 to 12)

- **What it is.** Guest cases 9 to 12 of `rank_arithmetic` in
  `crates/before/wasm32-pins/guest/src/checks.rs`, about 35 lines, driven
  one at a time by `l4_rank_sum_cases_diagnosed` with `L4_CASE` (verified).
- **What it reaches or checks.** Case 9 computes only `[deep, half].sum()`
  and case 10 only `[one, deep].sum()`, where `deep` is the existing pin's
  512 MiB-numerator rank. Case 11 replays `Sum`'s two accumulator calls for
  `[deep, half]` directly, and case 12 replays them after an exact
  `reserve_digits`. Case 9 traps with no panic message at 42,014 pages
  (2.56 GiB), case 10 passes, case 11 traps the same way, and case 12
  passes within the same 42,014 pages (verified, `wasm-2.log` and
  `wasm-3b.log`). Together they establish observation O6: on wasm32 a
  deep-first `Sum` aborts on allocation where `+` and the small-first order
  succeed, because amortized growth of a 1 GiB digit vector requests
  2 GiB.
- **Coverage beyond the committed suite.** No committed instrument measures
  memory on wasm32, and #29 tests only the small-first orders (verified). The
  owner ruled O6 an observation, not a defect, and an input to suanpan's
  growth-policy design under the suanpan lane's F1 (verified, the ruling in
  `observations.md`).
- **Evidence.** They are the reproduction of record for O6.
- **Fold-in cost.** Not as a passing test, since cases 9 and 11 trap today
  and the ruling accepts that. As a design check for the growth-policy
  work, cases 9 and 11 should pass after a growth change. Case 12 calls
  `reserve_digits`, which #28 renames to `reserve_bits(u64)` (verified).
  Cases 5 to 10 took 120 seconds together, and cases 11 and 12 about 15
  seconds each (verified, logs).
- **Overlaps.** #29's cases 5 to 7 (small-first sums); #28's
  `suanpan_ignores_unsatisfiable_reservations`; the suanpan lane's F1 work.
- **Dependencies.** #28 for the rename, and #31 to read the trap's message.
- **Value, in one sentence.** It is the acceptance check for any suanpan
  growth-policy change that aims to fix O6, not a regression test.

### 16. The wasm32 memory-size readout

- **What it is.** About four lines of `run_diagnosed` in
  `crates/before/wasm32-pins/harness/src/lib.rs` that read the guest
  memory's size in 64 KiB pages after a run (verified).
- **What it reaches or checks.** The high-water memory of any pin's guest.
- **Coverage beyond the committed suite.** #31 carries the panic-message
  half and reads guest memory only to fetch that message (verified, branch
  diff); nothing reports memory size. #29's entry states that its case 7
  peaks at 50,207 pages and "leaves room for one more 512 MiB numerator",
  but nothing checks that headroom (verified, ready entry).
- **Evidence.** It produced every page count in this file and in O6.
- **Fold-in cost.** Add the page count to the harness's result, so a pin can
  state its headroom and a memory regression reads as a stated failure
  rather than an allocation trap. The owner's ruling that neither crate
  promises an input size limits how much of this is wanted (verified,
  #53's entry).
- **Overlaps.** #31's message record; no other lane's `wasm32-pins` changes
  read memory size (verified, grep over the L6, L7, and L8 explore
  branches).
- **Dependencies.** #31, whose harness result it would extend.
- **Value, in one sentence.** It is a cheap diagnostic for the 32-bit
  checks' memory margins, of low value under the input-size ruling.

### 17. The `Rank` mutants R1 to R7 and E8

- **What it is.** Environment-selected mutants in `rank.rs` whose source was
  not kept (verified); their logs remain on the box under
  `~/src/rumors-audit-l4-measures/target/l4logs/mut-R*.log` and `mut-E8.log`
  (verified). From the coverage record (reported): R1 normalizes off by one;
  R2 compares raw numerators within a magnitude class; R3 swaps center
  padding; R4 drops the last fraction bit; R5 misplaces the first summand's
  exponent; R6 is off by one in precision; R7 returns zero on a
  `checked_sub` underflow; E8 shifts a sum's held value without doubling,
  which preserves values.
- **What it reaches or checks.** It calibrated entry 6.
- **Coverage beyond the committed suite.** None as a check.
- **Evidence.** Entry 6 kills R1 and R3 to R7, and R2 after strengthening;
  the committed suite kills R2 in 26 tests (reported).
- **Fold-in cost.** Rebuilding from the descriptions. E8 matters most: it is
  the known-bad input that entry 2's fold-in needs to show it can fail.
- **Overlaps.** The adequacy campaign's rank survivors, which are different
  mutants.
- **Dependencies.** None.
- **Value, in one sentence.** Their value is low, except for E8, which entry
  2's fold-in needs.

## Findings outside the catalogue

Each has a disposition; none changed anything.

1. **Committed test docs state a false premise.** `version/measure/tests.rs`
   says at lines 198 to 201 (`deferral_pool`) that "`arb_magnitude` tops out
   near 2^128, under half of that, so arbitrary trees do not reach the
   deferred-height reduction", and at lines 588 to 589
   (`arbitrary_arming_trains_agree`) that "`arb_magnitude`'s 128-bit ceiling
   keeps the arbitrary-tree sweep from ever arming" (verified, at `main`).
   `arb_magnitude` reaches 514 bits (00-baseline §2.1), and 24% of
   arbitrary pairs freeze (verified, census). The conclusion still holds,
   since none of 2,000 arbitrary pairs deferred. Disposition: a correction
   toward the code, for the coordinator's second documentation pass.
2. **The September `rank-20` exercise has no committed successor.** Entry 2
   gives the evidence. Disposition: for the owner; entry 2 is the
   candidate that restores it.
3. **The committed `Count` conversion test shares its oracle with the
   implementation.** Entry 8 gives the evidence. Disposition: for the owner,
   with entry 8 as the candidate.

## What I could not assess

- The algebra lane's tape generators' freeze and deferral rates, for the
  overlap in entry 1 (not measured).
- The cost probes' sensitivity to E8 and the schedule switches, and entry
  6's kill of rank-42: these would be mutation runs, which the brief's box
  allowance does not cover. Each is marked inferred where it appears.
- I used two of the three allowed box runs. My scratch worktree is removed;
  its box mirror, `~/src/rumors-rescue-l4` (765 MB), remains for the
  coordinator to retire.
