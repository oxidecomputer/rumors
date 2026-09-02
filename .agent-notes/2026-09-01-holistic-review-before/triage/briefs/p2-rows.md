<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P2 lane: the red-first rows

## Goal

Rulings 1 and 2 fix the order of every cost and space cure: the breaching
shape lands as a committed instrument that reads red, then the cure turns
it green, measured at the parent. This lane lands the instruments and
nothing else. Each row below is a two-scale envelope row in the unified
harness (`tests/meter.rs`, ruling 4) and, where the operation has a board
row, a board family, committed asserting the contract the crate docs
promise, so that at your base it fails and the commit message names the
breach and its readings. The invariant restored: every over-bound
operation the review demonstrated is a failing test in the tree before
anyone touches its kernel, and `tests/meter.rs`'s header names exactly
those operations as under repair.

A red row is a deliberately failing test. The gate must still pass: mark
each red row `#[ignore = "under repair: <finding id>; see the meter
header"]` with its readings in the doc comment, roster the ignored set in
one place the header cites, and give `p2-cures` the un-ignore as its
first edit per row. Report if the ignore attribute would hide the row
from any roster totality check (`tests-other-24` notes the scanners
cannot see `#[ignore]`); in that case add the row to the totality
check's expected-ignored list by reference and say so.

## Ground rules

These apply to every P2 lane; the lane sections below add to them and
never relax them.

- **Base.** Your worktree's HEAD must equal the SHA the coordinator names
  at launch: a main commit carrying the landed `p1-harness` lane (the
  unified envelope harness, ruling 4), at or above `bba0e31a`. Do not
  start from a base without it. Run `git -C <worktree> rev-parse HEAD`.
  If HEAD is an ancestor of the named SHA, fast-forward; if it has
  diverged, stop and report. Never call EnterWorktree; operate on the
  worktree through `git -C <path>` and absolute paths, one shell
  invocation at a time.
- **The review documents are the specification.** Each member entry below
  quotes its Resolution and Acceptance verbatim from the class document in
  `.agent-notes/2026-09-01-holistic-review-before/`; the entry's full
  record (evidence, construction, witness outcome) is under `### <id>:`
  there (`#### <id>:` in `simplification.md`; nits in `documentation.md`,
  `simplification.md`, and `verification.md` are one-line table rows) and
  in `evidence/`. Line anchors are at the reviewed commit `9e5784fb`, and
  the tree outside `.agent-notes/` is byte-identical at your base, so they
  hold; re-anchor from the quoted evidence, never from a line number
  alone, once your own commits move the file.
- **The rulings govern.** `triage/rulings.md` records Finch's decisions.
  Where a quoted Resolution offers alternatives, the ruling named beside
  it picks one; where the ruling amends the Resolution, the amendment is
  stated under the quote and wins. Where a quoted Resolution and the lane
  goal come apart, the goal wins, and the discrepancy is reported.
- **Typed references, never strings (ruling 43).** Wherever this lane
  touches `meter/registry.rs`, a family roster, `TRIPWIRE_ROSTER`, the
  surface rosters, or any test that names another test, file, or line:
  reasons, pins, and enforcement homes are expressed as references the
  compiler resolves (function items, registered law names, `Shape` and
  `Op` values), never as strings naming a test function, a file, or a
  line number. A lane that sees a cleaner idiomatic shape for a roster is
  authorized to adopt it and reports the reshaping in its diff. Finch's
  words: "please make these instruments impossible to drift in the
  future. I *really don't like* the pattern of hard-coded strings and
  Rust source locations embedded in tests; the way these family rosters
  ended up is not really to my taste, but I haven't had time to make it
  more idiomatic and obviously correct. If you see a good way to clean it
  up, please do."
- **Stops.** Report and leave the entry open; do not work around: anything
  that moves an `insta` snapshot or a committed pin the brief does not
  name as moving; any change to a public signature or public rustdoc
  contract the resolution does not name (the meter surface under
  `any(test, feature = "meter")` is instrument surface by ruling 8 and
  not public API, but a change there lands with the `rumors` test update
  in the same commit); anything that contradicts a ruling; any deviation
  from the stated resolution; anything this brief marks as a stop. A stop
  on one entry does not block the others.
- **Negative controls.** Every repaired or added instrument lands with a
  committed demonstration that a known-bad artifact fails it. The
  constructions in `evidence/witness.md` and each entry's Construction
  line are those artifacts; convert each into a committed test
  (`should_panic`, an asserted `Err`, a judge test over synthetic samples,
  or a reversible mutation whose observed failure the commit message
  records verbatim). Ruling 20 waived it for four P1 band docs and nowhere in P2.
- **Resource discipline.** Build with `cargo nextest run --no-run` before
  running; during iteration run only the binaries the entry names; never
  iterate on a timing measurement; launch no daemons or services. One full
  `just gate` per agent, before the final commit, run in the background
  redirected to a log under `<scratchpad>/<lane>/` and polled with short
  foreground checks (the foreground command cap is ten minutes; a
  foreground gate run cannot finish). Keep every working file and evidence
  log under that directory, never loose at the scratchpad top level.
- **Commits.** One commit per logical unit, its message describing the
  change and naming the entry ids and the ruling. Every re-pin is measured
  at the parent and its movement named in the commit. Commit every
  proptest seed file that appears. Prose speaks in the present tense: no
  reference to code that no longer exists, no dated rationale at a
  declaration site. Comments use spaced double-hyphens, never em-dashes;
  every test has a doc comment stating its invariant. Commit with a
  descriptive message before finishing.
- **Never delete anything outside your worktree.** If the disk fills
  (ENOSPC), stop and report; it is the coordinator's problem, not a
  reason to trim caches you do not own.
- **Self-retirement.** After the final commit, from inside the worktree:
  `cargo metadata --no-deps --format-version 1 | jq -r .build_directory`,
  then delete that directory and the worktree's `target/`. Leave the
  worktree in place.
- **Report.** For each entry: landed, stopped, or open; the commit sha(s);
  the acceptance evidence (the command and its decisive output, verbatim).
  Then anything left open and why, and every deviation from a stated
  resolution, explicitly. Your report is data: the coordinator verifies
  each entry's Acceptance against the tree at the reported sha before the
  ledger records it. Report what you could not do rather than working
  around it.

## Ordering inside the lane

1. crate-root-40 and prose-hygiene-1 first: the header restated to what
   the suite pins and which operations are under repair, so every later
   red row has its place in the list.
2. The envelope rows, one commit each, red at your base: skyline-coding-9
   (join), rank-33 (`ranked_cmp`), skyline-sweep-place-masked-5 with
   codec-bits-29 (the masked dual family and the scan charge),
   span-causally-36 with span-causally-24, skyline-sweep-place-masked-21,
   and span-causally-28 (the multi-hole rows), skyline-fill-grow-2 (the
   memo heap), rank-20 (ascending `sum_ranks`).
3. The witnesses that are not red today but pin an order the cure must
   preserve or improve: skyline-coding-29's closed-form limb witness, the
   shape-walk rows and depth pin (meter-adequacy-1, party-12,
   crate-root-37, clock-9, recursion-6).
4. The board families, rebased onto `p1-board` if it has landed: the
   join family, the memo families in the tick group, the shape walks.

Every reading at your base is recorded in the row's doc comment and the
commit message. Do not attempt any cure here: a row that you find you can
turn green with a one-line change is still red-first, and the change is
`p2-cures`' commit.

## Members

### crate-root-40 (medium, claim): ruling 1

(no Resolution line found)

Ruled (1): the header names the operations under repair with their
finding ids, as the resolution states, and nothing else changes in the
hard-guarantee sentence in `lib.rs`. In this lane the list is the full
set of rows this lane lands red; `p2-cures` removes each as its cure
lands, and the list is empty when that lane closes.

### prose-hygiene-1 (medium, documentation): ruling 45

Resolution: rewrite lines 4-11 to state what the suite pins (exact counter
envelopes at measured ×1.25, the limb floors, the committed known-bad
kernels) without the status narrative; restate the scenario docs at 428-429,
440-441, 465-466 and 983-985 from the envelope-table comments (iterative
sweep; validate-plus-wrap decode; the fused tick), dropping "today" and every
recursion or quadratic description. Acceptance: no scenario doc in the file
describes a mechanism its own pinned row contradicts, and `grep -n today
crates/before/tests/meter.rs` returns nothing.

Ruled (45): as stated, in the same commit as crate-root-40 (both
rewrite lines 4-11). The four scenario docs are restated from their
table comments; `grep -n today crates/before/tests/meter.rs` returns
nothing.

### skyline-coding-9 (high, claim): ruling 1, the instrument half

Resolution: the class is decided (owner ruling 1, 2026-09-02, `triage/rulings.md`): cure it, and land the instrument first. One structural cure: emit topology flags and payload codes into two streams and interleave once in `finish()`; every repair then becomes an O(1) truncation of the topology stream plus, for absorb, one 1-bit truncation of the payload stream, and the held-leaf discipline, `lens`, and `extract_code` dissolve (`continue_verbatim` would de-interleave its source range by walking it, still linear). A smaller cure to try first (the reviewer's suggestion after the ruling, unverified): after `cascade` truncates the pair, the sibling's code is already the stream's tail, so hold it in place rather than extracting it, and make the next flush a no-op while the held code is still the tail; `extract_code` is then needed only when bits were appended after the held code, and each cascade step is O(1) plus the flag truncation. Whether this composes with `lens` and `continue_verbatim` decides between it and the two-stream builder. Restating the contract as Θ(output + Σ re-anchored widths) is off the table under the ruling. The instrument lands first, before either cure: a two-scale flatness pin on the construction below (scan bits, and peak heap), plus a committed known-bad demonstrator if the builder is rewritten. Acceptance: a committed test under `scan-meter` builds `b(d)` = the text `(0, T_{d-1}, (0, 0, 1))` iterated from `T_0 = 0` and `a = Shape::Hugeleaf.packed1(10·d)`, measures `meter::scan_bits()` around `&a | &b` at `d` and `2d`, and asserts the per-input-bit scan cost stays within ×1.25 (it reads about ×2 per doubling today by the trace); the same pair added to `assert_emits` still matches the oracle byte for byte; the island contracts and the build.rs/emit.rs cost prose agree with the code.
Construction: `fn spine_of_pairs(d: usize) -> Version { let mut t = String::from("0"); for _ in 0..d { t = format!("(0, {t}, (0, 0, 1))"); } t.parse().unwrap() }` (canonical: every node has a zero-base child, no equal sibling leaves; preorder heights 0, then 0,1 repeated). `fn join_scan(d: usize) -> (u64, u64) { let a = Shape::Hugeleaf.packed1(10 * d).version(); let b = spine_of_pairs(d); let bits = (a.encode().len() + b.encode().len()) as u64 * 8; meter::reset_scan_bits(); std::hint::black_box(&a | &b); (meter::scan_bits(), bits) }`. Expected under the doc's claim: scan/bits flat from `d` to `2d`. Expected from the trace: about `2·d·(20d)` scan bits against about `30d` input bits, so the per-bit ratio doubles per doubling.

Ruled (1): this lane lands the instrument only: the two-scale flatness
pin on `&a | &b` with `a = Shape::Hugeleaf.packed1(10·d)` and `b =
spine_of_pairs(d)` under `scan-meter` (scan bits and peak heap per input
bit across `d` to `2d`, asserting ×1.25), red at your base with the
readings recorded (the trace predicts about ×2 per doubling), and the
same pair added to `assert_emits`. Where join has a board row, register
the shape as a board family too. The cure (the held-tail candidate, the
two-stream builder as fallback) is `p2-cures`'.

### rank-33 (high, claim): ruling 29, the instrument half

Resolution: Restore the contract at ops.rs:1350 to the pair measures' form (`O(M(|self| + |other|)) · log(|self| + |other|))` time, `O(|self| + |other|)` space, mirroring `version_distance`'s committed contract) and regenerate fuelscape/ranked_cmp.json through the compactor so the island and fuelscape-verify agree; add the plateau_puncture and wide_arming pair families to the `ranked_cmp` OpSpec so the fit itself confronts the superlinear shapes; rewrite ranked.rs:19-22 to what is true (one co-sweep instead of two folds and a compare, a constant, with the integrator's ledger and settle tree as its transient). If a cheaper order kernel is wanted, note that a sign-only fold cannot be linear on exact ties (the answer-embedded product must be resolved), so a domination-certificate early exit can only improve the non-tie case; the public worst case stays M-bound either way. Acceptance: the island text reads the M-bound contract and fuelscape-verify passes on the regenerated JSON; a committed two-scale meter row `ranked_cmp × PlateauPuncture` (`Shape::PlateauPuncture.packed2(w, d)` against `Version::new()`, at (w, d) and (2w, 2d)) whose limb and touch readings track `a.rank()`'s rather than a flat per-byte line.

Ruled (29): this lane lands the `ranked_cmp × PlateauPuncture` row at
(w, d) and (2w, 2d). Commit it asserting the contract the docs state
today (a flat per-byte line), so it reads red with the M-bound readings
recorded; `p2-cures` restates the contract and flips the assertion to
"limb and touch readings track `a.rank()`'s". The OpSpec families and the
fuelscape regeneration are `p2-cures`'.

### skyline-sweep-place-masked-5 (high, claim): ruling 30, the instrument half

Resolution: guard each peek with the depth test the skip already implies: `self.a.depth() > a_bound && self.a.peek_flip() > a_bound` (and the `b` twin). Since `peek_flip() <= depth()`, the guard is a necessary condition and changes no step; when it holds, `a` is the unique deepest slot, so if the skip does not engage `advance_set` steps `a` this same round and pops exactly the run the peek read, and every peek is then amortized against an immediate pop. (Caching the flip level in `LeafCursor` at open and step time has the same effect.) State the amortization premise the caller must keep in `peek_flip`'s doc (overlay.rs:347-354). Acceptance: a committed two-scale band on the family below (fuel per packed byte under the deterministic wasm fuel meter, or the bench judge's wall exponent) reads flat across a doubling of `(r, L)`: red at HEAD, green after the guard; the existing `masked_cmp_*` rows and `masked_cmp_hole_depth_band` unmoved.

Ruled (30): the two-scale band on the entry's `(r, L)` family, red at
your base. The currency it reads is the scan charge codec-bits-29 adds
(64 scan bits per word inside `trailing_ones`), so land that charge
first in this lane; the deterministic scan reading replaces the entry's
"fuel or wall exponent" alternatives. The guard is `p2-cures`'.

### codec-bits-29 (medium, claim): ruling 30, the instrument half

Resolution: Cache the flip level in `LeafCursor`: recompute `len - trailing_ones()` once after each `descend`/`step` (that one scan is bounded by the run the next `step` pops, so it amortizes to O(1) per plateau) and have `peek_flip` return the cached `u64`. Then (a) add the peek to masked.rs's cost argument and to `trailing_ones`'s doc ("callers that peek repeatedly must cache"); (b) register the dual family (a parked right run of depth `r` in one operand, `n` plateaus inside its interval in the other, the a-mask unowned there) for `masked_cmp` and `project`; (c) give the board a currency that sees it, either `scan::record_bits_u64(64 * words)` inside `trailing_ones` or the bench judge's two-scale ratio on the new cells. Acceptance: a committed test builds the dual shape at `(r, n)` and `(2r, 2n)` and asserts the walk's stack-word reads stay within a flat per-input-bit band across the doubling (about 4x today, about 2x with the cached flip level); the masked and projection cost arguments name the peek and its amortization.
Construction: version `v`: root = node(left = L1, right = leaf h = 0); L_k = node(left = leaf h = 1, right = L_{k+1}) for k = 1..r, with L_{r+1} = leaf h = 0 (every sibling pair is leaf/internal or of distinct heights, so canonical). v's preorder leaves have paths 00, 010, ..., 0 1^(r-1) 0, then the parked leaf 0 1^r (r trailing ones), then 1. Party `p` = "(0, 1)" (unowned on [0, 1/2)). Version `w`: the same spine to depth r + 1 with L_{r+1} replaced by a subtree of n leaves with alternating heights 0/1, all inside v's parked leaf's interval. Run `(&v / &p).partial_cmp(&w)`: after r lockstep advances `a` is parked and `b` advances n - 1 times; each advance runs `block_skip`, whose first conjunct is true, so `self.a.peek_flip()` scans about r / 64 words and then compares false against `a_bound >= r + 1`. Count words read in `trailing_ones` (a temporary counter) at r = n = 8192 and r = n = 16384 against input bits; expect the ratio to approach 4 rather than 2. For `project`, replace `w` by a party alternating owned/unowned across n leaves inside the same interval and call `(&v / &p2).to_version()`.

Ruled (30): the scan charge (`scan::record_bits_u64(64 * words)`
inside `trailing_ones`, clause (c)) and the dual family registered for
`masked_cmp` and `project` (clause (b)) are this lane's, with the
two-scale test on the dual shape at `(r, n)` and `(2r, 2n)` committed
red (the entry predicts about 4× today). The guard chosen by ruling 30
is the depth test, not the cached flip level; clause (a)'s doc changes
land with the guard in `p2-cures`. The scan charge moves every committed
scan reading that passes through `trailing_ones`: measure each at the
parent and attribute the movement in the commit, as ruling 30's
instrument-first order requires; a reading that moves for another reason
is a stop.

### span-causally-36 (high, claim): ruling 31, the instrument half

Resolution: instruments before cures. First land a deterministic `scan-meter` row in tests/meter.rs's `placement` module shaped as below, measured at (k, |hi|), (2k, |hi|), (k, 2|hi|), asserting the marginal cost of doubling |hi| is independent of k (within the crate's slack); commit it red. Then replace the per-hole loop with one fused membership walk over the polarity's deciding clamp end: for `Down`, `Empty <=> !filter::admits(clamped_hi.view().live(), self.holes.iter().map(|h| (h.at.view().live(), P::hole_demand(h.strict))))` (with the crossed-clamp test kept as `le(clamped_lo, clamped_hi)`, or reduced to `!le(floor, ceiling)` since `floor <= hi` and `lo <= ceiling` are established by the fused walk returning `Partial`); dually `clamped_lo` for `Up`; a new sealed method naming which clamp end decides replaces `hole_covers` (its only caller). `admits` returns false exactly when some hole's subtraction holds on the probe (filter.rs:221-230), which is the `any(hole_covers)` predicate. Then restate the `# Complexity` text to the traversals the code performs (the fused walk's own hole factor is span-causally-24). Add a many-hole fuelscape variant for the rendered island. Acceptance: the meter row goes green; the doc says what the code does; `coverage_is_exact_on_the_two_party_grid`, `coverage_clamp_refinement_is_exact`, and the new pin of span-causally-35 stay green.

Ruled (31): the deterministic `scan-meter` row in the placement module
at (k, |hi|), (2k, |hi|), (k, 2|hi|), asserting the marginal cost of
doubling |hi| is independent of k, committed red. The fused walk, the
contract restatement, and the many-hole fuelscape variant are
`p2-cures`'.

### span-causally-24 (medium, claim): ruling 31, the instrument half

Resolution: owner decision, then wording. (a) Restate the contracts with the hole-count factor: "linear in the bits decoded (each stream once); per-interval work proportional to the number of live holes, `O(k · (|probe| + |self|))` in operations", and replace "linear time" at causally.rs:85-86, query.rs:28-30, polarity.rs:211-212 with the actual payoff of the polarity restriction (a polynomial decision procedure with an exact verdict; the SAT reduction is span-causally-33). (b) Keep the linear promise, which needs an indexed per-hole design. In both cases add a k-scaling instrument in the touch (accumulator) currency or fuel, never scan bits. Also correct filter.rs:46-48 (outside this partition). Acceptance: a touch-currency row with probe `v` fixed and `Q_k = until(&h_1) & ... & until(&h_k)` at k in {8, 64}; under (a) it pins the ratio ~8 as the declared model; under (b) it asserts the difference is bounded by the added holes' bits and fails today. The public docs no longer say "linear time" without the hole-count clause.

Ruled (31): option (a)'s instrument: the touch-currency row with the
probe fixed and `Q_k = until(&h_1) & ... & until(&h_k)` at k in {8, 64},
pinning the ratio (about 8) as the declared per-interval model the
restated contract will name. This row is not red: it pins the model the
cure keeps. The wording changes are `p2-cures`'.

### skyline-sweep-place-masked-21 (medium, claim): ruling 31, the instrument half

Resolution: either (a) restate the bound accurately at filter.rs:37-48 and the public docs it feeds (scan bits O(|v| + Σ|b_i|); folds O(k|v| + Σ|b_i|); sign reads and advance bookkeeping O(k · (|v| + Σ|b_i|))), with the accurate comparison (the fusion saves k-1 probe decodes and pays a k factor on the bounds' boundaries), and pin the exponent in k with a two-scale touch row; or (b) restructure: read only pairs whose cursor stepped this round (a bound step changes one pair; a probe step changes all k, which the composition also pays; `Directions::fold` is idempotent on an unchanged sign), and pick the deepest slot with a heap keyed by depth, reaching Θ(k|v| + Σ|b_i| · log k); the single-bound identity rows are unmoved because every interval of a two-stream overlay is dirty. Acceptance: a committed tests/meter.rs row builds k and 2k `Demand::NotBefore` holes with pairwise-disjoint leaf-boundary sets against the probe `Version::new()` (below every hole, so nothing exits early), measures `touch_ops` over `filter::admits`, and asserts the ratio sits under the stated exponent, with a liveness floor of k touches (one read per bound on the first interval).

Ruled (31): option (a)'s row: k and 2k `Demand::NotBefore` holes with
pairwise-disjoint leaf-boundary sets against the probe `Version::new()`,
measuring `touch_ops` over `filter::admits`, asserting the ratio sits
under the stated exponent, with the liveness floor of k touches. The
restated bound at `filter.rs:37-48` and the public docs is `p2-cures`'.

### span-causally-28 (low, verification-gap): ruling 31, the instrument half

Resolution: add an enforced row (a `scan-meter` or touch row in tests/meter.rs) whose operands carry k pairwise-concurrent holes with k scaling, declaring the k·m model at the cell; either add a many-hole fuelscape variant or relabel the existing island's contract to the one-hole shape it samples. Optionally fuse the survive filter through `filter::admits` with per-bound verdicts. Acceptance: an enforced row exists whose reading quadruples when k doubles at fixed bound size, and the island's contract describes what its operands measure.

Ruled (31): the enforced row whose operands carry k pairwise-concurrent
holes with k scaling, declaring the `k·m` model at the cell; the reading
quadruples when k doubles at fixed bound size. The island relabeling or
the many-hole fuelscape variant is `p2-cures`' (with span-causally-36's).
The optional survive-filter fusion is not taken.

### skyline-fill-grow-2 (medium, claim): ruling 2, the instrument half

Resolution: (1) Instrument first: add a `peak_heap` reading to `memo_resolution_cost`'s `tick_run` (the `PeakAlloc` harness the tick rows use) at two scales for `MemoChain(distinct)` and `MemoComb`, judged per input byte; or promote those two families to the board's tick group so `MAX_HEAP_BYTES_PER_INPUT_BYTE` judges them with their own `(event, id)` pair. (2) If the reading is red, either declare a family model at the constant the owner ratifies (as `ASCEND_CLIFF_TICK_HEAP_BYTES_PER_INPUT_BYTE` does) or cure, measuring first: store narrow links and suspended heads and keepers at machine width (the `Boundary::Word | Wide(Accumulator)` trade `MinWeb` already makes), boxing only the wide ones. (3) Rewrite fill.rs:130-134 to state what holds: `PreFrames` holds bits and slot deltas; the suspend stack (`SuspendedLevel`) holds one head and one keeper accumulator per level with a recorded-but-unresolved first child, moved rather than copied, so their digits count toward the O(n + m) live total; and `Memo::links` holds one accumulator per nonzero link. Acceptance: a committed two-scale heap reading exists for both families under the board ceiling or a declared model whose derivation names the per-level struct cost; the heap paragraph names `SuspendedLevel` and no longer says "never an accumulator per open site-nesting level".

Ruled (2): the two-scale peak-heap row on `MemoComb` and
`MemoChain(distinct)` under the tick harness, judged per input byte
against the board ceiling, recording the suspend-stack depth and the link
count at the peak so the two contributors are separated; committed red
(the witness read 104.7 and 98.6 B/B for the comb at d = 1000 and 2000,
53.6 and 50.5 B/B for the chain at k = 1000 and 2000). Also promote the
two families to the board's tick group if the board's ceiling is the
judge of record there; say which home you chose. The representation cure
and the `fill.rs:130-134` restatement are `p2-cures`'.

### rank-20 (medium, claim): ruling 32, the instrument half

Resolution: Instrument first: a two-scale touch row (W fixed at, say, a 2^20-bit counter rank; n in {64, 128} spine ranks 1/2^k summed in ascending k) asserting touches per content bit flat across the two scales; it reads red today (touches double with n while content barely moves). Then cure with geometric headroom: when a summand exceeds the held exponent by `gap`, shift by `max(gap, held_bits)` and carry the surplus as exponent headroom, so the held width at least doubles per rescale and total rescale touches telescope to O(final held width); `from_num`'s `trailing_zeros`/`shr` already strip the headroom at the end (every summand entered at a shift at least the headroom). Rewrite 1011-1018 to the true bound, add `# Complexity` to both `impl Sum` blocks, and correct the two pins' adversarial-order prose. Acceptance: the new row is committed red then green with touches at most c · Σ‖r_i‖ at both scales; `RANK_SUM_MIXED` unchanged or re-pinned with attribution; `rank_sum_equals_the_pairwise_fold` and `rank_cross_path_normalization` still hold.

Ruled (32): the two-scale touch row (a 2^20-bit counter rank held, n
in {64, 128} spine ranks summed ascending), asserting touches per content
bit flat across the two scales, committed red with the readings (the
entry predicts about 2.1M and 4.2M touches). The headroom cure, the
`# Complexity` on both `impl Sum` blocks, and the pins' prose are
`p2-cures`'.

### skyline-coding-29 (medium, claim): ruling 36, the witness half

Resolution: owner ruling on a ratified model; two consistent options. Cure: carry `span` (and `drop`) up the spine instead of re-summing them at each level (fold the small `entry_step` into the moved child summary in place; an `Accumulator` per flowing summary keeps the fold amortized O(1) across carry cliffs, and only a printed base pays a magnitude read), measure at the parent on `SKYLINE_RENDER_*` and the mirror-wide cells, then retire the declared model and the liveness pin together as ceilings.rs:425-429 prescribes. Accept: replace "superlinear, subquadratic"/"n log n" with the derived class Θ(|self| + depth × max interior summary width + k log k) in ops.rs and the island, state both mechanisms in text.rs's render doc, delete the "genuinely quadratic still reads red" sentence, re-point asymptotics.rs:45-46 at a sentence that exists, and add the closed-form witness below so the instrument pins the order rather than a scale-bound fit. Acceptance: either the mirror-wide cells and `render_merge_superlinearity_is_alive` both read linear after the cure (the pin flips red and is retired in the same commit), or the `version_display`/`version_fromstr` contracts, the text.rs render doc, and the ceilings.rs prose all state the same derived class and a committed check `render_limb_ops(s) >= s * s.div_ceil(64)` holds at three doublings.
Construction: under `limb-meter`, for `s in [1024, 2048, 4096]` assert `render_limb_ops(s) as usize >= s * s.div_ceil(64)` (each of the `s` spine levels' span sum costs at least `⌈s/64⌉` limb ops); as an order witness, the doubling ratios `render_limb_ops(2s) / render_limb_ops(s)` increase with `s` and exceed 3.5 by `s = 4096`, where an `n log n` mechanism would hold near `2·(1 + 1/log2 s) ≈ 2.2`.

Ruled (36): cure, then retire the model. This lane lands the
closed-form witness (`render_limb_ops(s) >= s * s.div_ceil(64)` at three
doublings under `limb-meter`) and records the doubling ratios at your
base in its doc comment; it is green today and pins the order the cure
must beat (the cure makes it read linear and `p2-cures` retires it with
the model and the liveness pin). The carry-up cure and the prose are
`p2-cures`'.

### meter-adequacy-1 (medium, verification-gap): ruling 37

Resolution: give the four walks an enforcing home. The cheapest is a board
row group (`version_shape`, `party_shape`, `clock_shape`, `shape_combine`)
draining the iterators under the scan floor; the family bundles already
supply the operands, and the tiling test then reclassifies the four NA
entries as priced. The alternative is four fuzz-fit `Op` variants (the guest
kernels `ff_version_shape`, `ff_party_shape`, `ff_clock_shape`,
`ff_shape_combine` already exist) plus a re-pin. Separately, state
`shape::combine`'s bound in its rustdoc, since the crate docs promise a
Big-O on every operation. The sweep's proposed keyword pin on NA reason
text is not recommended: `BOARD_NOT_APPLICABLE` is `&[(&str, &str)]`, so the
tiling test judges membership only, and a string-content lint would be a
convention held in a regex. Acceptance: a committed board or fuzz-fit test
that reads red under the construction below, and the four NA entries gone.
Construction: regress `crate::shape::Plateaus::next` to re-scan the stored
stream from position 0 on every item (quadratic drain). Run `just gate`:
the board has no shape row, `tests/meter.rs` has no shape scenario, the
fuzz-fit program vocabulary has no shape op, and `fuelscape-test` asserts
sampler correctness only, so the gate stays green.

Ruled (37): board rows for the four public shape walks as their
enforcing home, with the entry's construction (a `Plateaus::next` that
re-opens the stream per item) as the reversible-mutation negative control
recorded in the commit message.

### party-12 (medium, verification-gap): ruling 37

Resolution: Add a closed-form pin in party/tests.rs: for `Party::seed()` and `k` over a ladder (1..=64, 1023, 1024, 1025, 2^16), every yielded share and the residual read `encoded_bits() == 2 + 2·d` with `d ∈ {⌊log₂(k+1)⌋, ⌈log₂(k+1)⌉}`, and the count of shares at the deeper level equals `2·(k+1) − 2^⌈log₂(k+1)⌉` (the complete-tree leaf split), so the whole tiling is fixed. Commit the 3:1 split as a known-bad witness (a `cfg(test)` `Split` variant, or an inline transcription of `Split::next` with `count - (count / 4).max(1)`) and assert the pin convicts it. Cite the pin by name at forks.rs:16-17 and :154. Acceptance: the new test fails when `left_count` is `count - (count / 4).max(1)` and passes on `count.div_ceil(2)`.
Construction: change forks.rs:57 to `let left_count = count - (count / 4).max(1);` (a 3:1 split, depth about `2.4·log₂ k`). `forks_matches_from_array` still holds (both forms run the same `Split`), the reunion and partial-drop laws hold (the shares remain a disjoint tiling), `party_forks_max_saturates_without_panic` terminates, and at the island's `k = 8` the extra work is roughly ×1.6 fuel, under the band's `width_above + ENFORCE_MARGIN` ceiling. Nothing committed fails.

Ruled (37): the closed-form depth pin over the ladder, with the 3:1
split committed as the known-bad witness the pin convicts (a `cfg(test)`
`Split` variant or an inline transcription), cited by name at
`forks.rs:16-17` and `:154`. Under ruling 43 the citation is a reference
where the doc can carry one (an intra-doc link to the test), not a bare
string.

### crate-root-37 (low, claim): ruling 37, with ruling 44 on the prose

Resolution: State the walk's auxiliary space in the module doc (one path stack per input, `O(depth)` bits, heap-resident past 64 levels, freed at drop) and amend party.rs:481 to "allocates nothing per item" (likewise version.rs:738-740 and the clock door if they carry the clause). Add one heap-metered scenario per shape door to tests/meter.rs on the deep spine families so the auxiliary-space claim has a pin. Acceptance: under `PeakAlloc`, draining `deep_left_spine_party(64).shape()` reads zero heap delta and `deep_left_spine_party(65).shape()` a nonzero one; the prose states the `O(depth)`-bit path stack; a committed envelope row bands the deep-spine drains.
Construction: In a `PeakAlloc`-instrumented test: `let p = deep_left_spine_party(65); reset peak; p.shape().count(); assert_eq!(peak_delta, 0)` fails: `IdLeafCursor::open` pushes 65 path bits and `BitStack::push` allocates its first `words` entry at the 65th push.

Ruled (37, 44): the heap-metered scenario per shape door on the deep
spine families lands here (zero heap delta at depth 64, nonzero at 65).
The prose: "nothing allocates" and "allocates nothing per item" are
elided (ruling 44), and the module doc states the `O(depth)`-bit path
stack only because the row you land pins it; if the row does not land,
the sentence does not either.

### clock-9 (low, verification-gap): ruling 37

Resolution: Owner's call. If the dispositions stand, no change; if not, add fuzzfit bands for `ff_clock_shape` (and the party/version shape kernels) and `ff_clock_sync_all` at the next `just fuzzfit-calibrate`, and add an orbit-style size pin for `forks(k)` shares (every share of `Party::seed().forks(k)` reads at most `2 + 2·⌈log2 (k + 1)⌉` encoded bits) beside the iterated-fork affine chain pin. Acceptance: a `shape` walk that re-opens its version cursor per fragment fails the gate; the share-size pin trips when a share exceeds the logarithmic bound.
Construction: wrap `Overlay::next` so it re-opens the version walk from the start on every call (the yielded fragments are unchanged); run `just gate`: the laws pass and only the unenforced fuelscape would show the bend.

Ruled (37): the dispositions do not stand; the enforcing home is the
board rows meter-adequacy-1 lands (not fuzzfit bands, which are the fuzz
lane's vocabulary) plus the orbit-style share-size pin for `forks(k)`
beside the iterated-fork affine chain pin. The construction (an
`Overlay::next` that re-opens the walk) is the negative control.

### recursion-6 (low, verification-gap): ruling 37

Resolution: Extend `deep_tree_query_and_causal_stack_safety` (or add a sibling)
with the four shape doors over the deep clock, asserting item counts against
their closed forms (a depth-d left spine has d + 1 plateaus or regions):
`late.shape().count()`, `clock.party().shape().count()`,
`clock.shape().count()`, `combine([&early, &late]).count()`. Acceptance: a
committed test drives each public shape iterator over a depth-100k structure.

Ruled (37): as stated; a depth-100k drive of each public shape
iterator with item counts asserted against their closed forms.

### envelopes-b-20 (low, claim): roster: pending Finch's approval

Resolution: for the `/8` floors, restate the premise so it yields the constant (for example: every consumed code costs at least one touch, a code spans at most eight input bytes per touch it costs, and the payload is the whole input) or lower the constant to what the stated premises support (`input / 64`); for the per-byte rank floors, derive them per family from the code structure (leaf count plus the wide codes' digit counts) or relabel them as measured-basis tripwires. Acceptance: each floor's doc reproduces its constant from its premises; a hand computation of PP(500, 500)'s irreducible touches is at or above its asserted floor.
Construction: not a runtime failure today. Demonstration for the composition: `(input / 8) / 8 = input / 64`. For the rank floor, a settle that delegates the whole wide × dense product to the backend and touches each of x's ~500 digits once plus ~500 leaf folds reads roughly 1,000-2,000 touches on a ~4.6 KB PP(500, 500) operand and trips `touches >= bytes` at 5720 while being strictly cheaper.

Roster note: touch floors whose stated premises do not reach the
asserted constant; each floor's doc reproduces its constant from its
premises or the constant drops to what the premises support, and the
per-byte rank floors derive per family or are relabeled measured-basis
tripwires. Lands with the row work if Finch approves the roster.

### envelopes-b-28 (nit, correctness): roster: pending Finch's approval

Resolution: rewrite as `fused < met + joined + cmp` and print all four readings (or assert `fused >= cmp` first with its own message). Acceptance: no bare `-` between counter readings in the range.
Construction: any hypothetical `span` fast path that skips the classifying `partial_cmp` yields `fused` below `cmp`'s early-exiting prefix; the current line panics on overflow before reaching the assertion.

Roster note: an unchecked counter subtraction that panics on overflow
before its assertion; rewritten as `fused < met + joined + cmp` with all
four readings printed. Lands if approved.

### version-core-1 (nit, claim): roster: pending Finch's approval

Resolution: one test-only assertion, `assert!(Version::new().view().ptr_eq(Version::new().view()))`, with the `Party::seed()` twin; or drop the parenthetical and let the `static` speak for itself. Acceptance: a committed test reads red when `EMPTY_STREAM` becomes a `const`, or the comment no longer claims cross-call sharing.

Roster note: a cross-call clone-identity claim with no pin; one
test-only `ptr_eq` assertion for `Version::new()` and the `Party::seed()`
twin, or the parenthetical dropped. Lands if approved.

### board-families-floors-judge-14 (low, claim): roster: pending Finch's approval

Resolution: Drop the universal clause and state the conditional ("positive wherever either operand stores a nonzero delta; a delta-free pair such as the single-leaf hugeleaf column declares NA"), or, if every pair family is meant to carry a live touch floor, pin it with a test over `FamilyId::board()`'s version pairs and give hugeleaf a counterpart that stores a delta. Acceptance: the doc asserts no property of every committed family, or a committed test iterating the board's version pairs asserts `touch_pair_fold` is `Liveness::Floor` on each and passes.
Construction: Build the hugeleaf bundle (`FamilyData::build(FamilyId::Hugeleaf, 1.0, 0)`), decode `version` and `version2`, and assert `matches!(touch_pair_fold(&v, &w), Liveness::NotApplicable { .. })`: it holds today, contradicting the sentence.

Roster note: `touch_pair_fold`'s doc overclaims a live touch floor on
every committed pair family (the hugeleaf column declares NA); either the
sentence becomes conditional or a committed test over the board's version
pairs asserts the floor on each and hugeleaf gains a delta-storing
counterpart. Lands with the board-family work if approved; say which.

### board-ops-render-31 (nit, correctness): roster: pending Finch's approval

Resolution: `assert!(input_bytes > 0, "a board cell charges against at least one byte")` in the three `Cell` constructors (or in `measure` before the fit); optionally `debug_assert!(!slope.is_nan())` at the end of `trend`. Acceptance: a unit test constructing a `Sample` pair with `exp_denom_bytes: 0` through `evaluate` panics at the guard rather than returning a green cell.

Roster note: a zero denominator makes the exponent fit `NaN` and the
leg reads green on unbounded growth; an `assert!(input_bytes > 0)` in the
`Cell` constructors with a unit test through `evaluate`. Lands with the
board-family work if approved.

## Hazards and stops

- Every row lands red and stays red until `p2-cures`; turning one green
  here, by any means, is a stop.
- The scan charge inside `trailing_ones` (codec-bits-29) moves committed
  scan readings; each movement is measured at the parent and attributed
  in that commit, and a reading that moves for any other reason is a
  stop. The 480 B `MASKED_CMP_HOLE` heap pin never widens (ruling 24).
- `tests/meter.rs` is yours after `p1-harness`; `p1-board` and
  `p1-suites` may still be landing their band-doc and satellite edits
  there. Rebase before your gate run and report any conflict rather than
  resolving it by dropping either side's rows.
- Board families change every family's `Coverage::Board { cells }` count
  and the smoke suite's per-family counts; re-state those counts in the
  same commit with the attribution, as board-ops-render-9's acceptance
  describes for its own case.
- No bench judge, no wall-time measurement; the deterministic counters
  are the readings of record.
