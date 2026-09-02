<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P2 lane: the cost and space cures

## Goal

Every row `p2-rows` committed red names an operation over a bound
`lib.rs` states as a hard guarantee, and rulings 1 and 2 make each a
defect to fix, never an exception to declare. This lane turns those rows
green: one cure per row, measured at the parent, its re-pins named in
the commit, the row un-ignored and asserting in the same commit as the
cure that makes it pass, and the meter header's under-repair list
shrinking by one each time. The invariant restored: every asymptotic and
transient-space claim on the crate's front page and in its per-operation
`# Complexity` sections holds absolutely, for every input shape,
including shapes reachable only through `decode`, text, or literal
construction.

## Ground rules

These apply to every P2 lane; the lane sections below add to them and
never relax them.

- **Base.** Your worktree's HEAD must equal the SHA the coordinator names
  at launch: a main commit carrying the landed `p2-rows` lane (every row
  this lane cures committed red) and `p1-harness`, at or above
  `bba0e31a`. Do not start from a base without them. Run
  `git -C <worktree> rev-parse HEAD`. If HEAD is an ancestor of the named
  SHA, fast-forward; if it has diverged, stop and report. Never call
  EnterWorktree; operate on the worktree through `git -C <path>` and
  absolute paths, one shell invocation at a time.
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

1. The masked skip's guard (skyline-sweep-place-masked-5, codec-bits-29)
   first: smallest cure, its row's currency already landed.
2. `Ranked::cmp`'s contract and fit families (rank-33), the ascending
   `sum_ranks` headroom (rank-20): both inside the rank module.
3. The multi-hole fusion and restatement (span-causally-36, -24, -25,
   -28, skyline-sweep-place-masked-21) as one series.
4. The memo-heap representation (skyline-fill-grow-2) after `p2-widths`
   has landed its `memo.rs` widening (ruling 33), rebasing onto it; the
   two edit the same struct fields.
5. The join re-anchor cure (skyline-coding-9): try the held-tail
   candidate first, measure, fall back to the two-stream builder only on
   measured evidence, never on anticipated complexity.
6. The render merge (skyline-coding-29), retiring the declared model, its
   liveness pin, and `p2-rows`' closed-form witness together once the
   mirror-wide cells read linear.
7. crate-root-40's list emptied and, last, the headline tightened per
   ruling 93 (crate-root-25, paper-fidelity-3): `lib.rs`'s sentence names
   the per-operation classes (linear for the core operations, near-linear
   for the n-ary folds, multiplication-bound where the answer is a wide
   integer), every operation's `# Complexity` section carrying its own
   bound. This is the lane's final commit.

Every cure: measure at the parent (the same `just test-all` MEASURED
lines and `just amp-board-acceptance` output kept under
`<scratchpad>/p2-cures/`), then after; every moved pin named with both
numbers. A cure whose row stays red after the change is a finding about
the cure: report the readings, do not widen the row.

## Members

### skyline-sweep-place-masked-5 (high, claim): ruling 30

Resolution: guard each peek with the depth test the skip already implies: `self.a.depth() > a_bound && self.a.peek_flip() > a_bound` (and the `b` twin). Since `peek_flip() <= depth()`, the guard is a necessary condition and changes no step; when it holds, `a` is the unique deepest slot, so if the skip does not engage `advance_set` steps `a` this same round and pops exactly the run the peek read, and every peek is then amortized against an immediate pop. (Caching the flip level in `LeafCursor` at open and step time has the same effect.) State the amortization premise the caller must keep in `peek_flip`'s doc (overlay.rs:347-354). Acceptance: a committed two-scale band on the family below (fuel per packed byte under the deterministic wasm fuel meter, or the bench judge's wall exponent) reads flat across a doubling of `(r, L)`: red at HEAD, green after the guard; the existing `masked_cmp_*` rows and `masked_cmp_hole_depth_band` unmoved.

Ruled (30): the depth-test guard on both cursors (`depth() > bound &&
peek_flip() > bound`), the same check in the projection loop, and the
amortization premise stated at `peek_flip`'s doc. The row `p2-rows`
landed is the acceptance: red at the parent, green after, the existing
`masked_cmp_*` rows and `masked_cmp_hole_depth_band` unmoved.

### codec-bits-29 (medium, claim): ruling 30

Resolution: Cache the flip level in `LeafCursor`: recompute `len - trailing_ones()` once after each `descend`/`step` (that one scan is bounded by the run the next `step` pops, so it amortizes to O(1) per plateau) and have `peek_flip` return the cached `u64`. Then (a) add the peek to masked.rs's cost argument and to `trailing_ones`'s doc ("callers that peek repeatedly must cache"); (b) register the dual family (a parked right run of depth `r` in one operand, `n` plateaus inside its interval in the other, the a-mask unowned there) for `masked_cmp` and `project`; (c) give the board a currency that sees it, either `scan::record_bits_u64(64 * words)` inside `trailing_ones` or the bench judge's two-scale ratio on the new cells. Acceptance: a committed test builds the dual shape at `(r, n)` and `(2r, 2n)` and asserts the walk's stack-word reads stay within a flat per-input-bit band across the doubling (about 4x today, about 2x with the cached flip level); the masked and projection cost arguments name the peek and its amortization.
Construction: version `v`: root = node(left = L1, right = leaf h = 0); L_k = node(left = leaf h = 1, right = L_{k+1}) for k = 1..r, with L_{r+1} = leaf h = 0 (every sibling pair is leaf/internal or of distinct heights, so canonical). v's preorder leaves have paths 00, 010, ..., 0 1^(r-1) 0, then the parked leaf 0 1^r (r trailing ones), then 1. Party `p` = "(0, 1)" (unowned on [0, 1/2)). Version `w`: the same spine to depth r + 1 with L_{r+1} replaced by a subtree of n leaves with alternating heights 0/1, all inside v's parked leaf's interval. Run `(&v / &p).partial_cmp(&w)`: after r lockstep advances `a` is parked and `b` advances n - 1 times; each advance runs `block_skip`, whose first conjunct is true, so `self.a.peek_flip()` scans about r / 64 words and then compares false against `a_bound >= r + 1`. Count words read in `trailing_ones` (a temporary counter) at r = n = 8192 and r = n = 16384 against input bits; expect the ratio to approach 4 rather than 2. For `project`, replace `w` by a party alternating owned/unowned across n leaves inside the same interval and call `(&v / &p2).to_version()`.

Ruled (30): the guard, not the cache (the resolution's cached flip
level is the alternative ruling 30 did not take). Clause (a) is yours:
the peek and its amortization named in `masked.rs`'s cost argument and
in `trailing_ones`'s doc. Clauses (b) and (c) landed in `p2-rows`.

### rank-33 (high, claim): ruling 29

Resolution: Restore the contract at ops.rs:1350 to the pair measures' form (`O(M(|self| + |other|)) · log(|self| + |other|))` time, `O(|self| + |other|)` space, mirroring `version_distance`'s committed contract) and regenerate fuelscape/ranked_cmp.json through the compactor so the island and fuelscape-verify agree; add the plateau_puncture and wide_arming pair families to the `ranked_cmp` OpSpec so the fit itself confronts the superlinear shapes; rewrite ranked.rs:19-22 to what is true (one co-sweep instead of two folds and a compare, a constant, with the integrator's ledger and settle tree as its transient). If a cheaper order kernel is wanted, note that a sign-only fold cannot be linear on exact ties (the answer-embedded product must be resolved), so a domination-certificate early exit can only improve the non-tie case; the public worst case stays M-bound either way. Acceptance: the island text reads the M-bound contract and fuelscape-verify passes on the regenerated JSON; a committed two-scale meter row `ranked_cmp × PlateauPuncture` (`Shape::PlateauPuncture.packed2(w, d)` against `Version::new()`, at (w, d) and (2w, 2d)) whose limb and touch readings track `a.rank()`'s rather than a flat per-byte line.

Ruled (29): the M-bound contract at `ops.rs:1350`, mirroring
`version_distance`; `fuelscape/ranked_cmp.json` regenerated through the
compactor so `fuelscape-verify` passes; the plateau_puncture and
wide_arming pair families added to the `ranked_cmp` OpSpec; `ranked.rs:
19-22` rewritten to the constant-factor fact. The `p2-rows` row's
assertion flips from flat-per-byte to "tracks `a.rank()`'s readings" in
this commit. The domination-certificate early exit is optional and not
taken here unless you measure a non-tie improvement; say so either way.

### rank-20 (medium, claim): ruling 32

Resolution: Instrument first: a two-scale touch row (W fixed at, say, a 2^20-bit counter rank; n in {64, 128} spine ranks 1/2^k summed in ascending k) asserting touches per content bit flat across the two scales; it reads red today (touches double with n while content barely moves). Then cure with geometric headroom: when a summand exceeds the held exponent by `gap`, shift by `max(gap, held_bits)` and carry the surplus as exponent headroom, so the held width at least doubles per rescale and total rescale touches telescope to O(final held width); `from_num`'s `trailing_zeros`/`shr` already strip the headroom at the end (every summand entered at a shift at least the headroom). Rewrite 1011-1018 to the true bound, add `# Complexity` to both `impl Sum` blocks, and correct the two pins' adversarial-order prose. Acceptance: the new row is committed red then green with touches at most c · Σ‖r_i‖ at both scales; `RANK_SUM_MIXED` unchanged or re-pinned with attribution; `rank_sum_equals_the_pairwise_fold` and `rank_cross_path_normalization` still hold.

Ruled (32): the geometric-headroom cure, `# Complexity` on both `impl
Sum` blocks, the two pins' "adversarial order" prose corrected, and
1011-1018 rewritten to the true bound. `RANK_SUM_MIXED` unchanged or
re-pinned with attribution; `rank_sum_equals_the_pairwise_fold` and
`rank_cross_path_normalization` stay green.

### span-causally-36 (high, claim): ruling 31

Resolution: instruments before cures. First land a deterministic `scan-meter` row in tests/meter.rs's `placement` module shaped as below, measured at (k, |hi|), (2k, |hi|), (k, 2|hi|), asserting the marginal cost of doubling |hi| is independent of k (within the crate's slack); commit it red. Then replace the per-hole loop with one fused membership walk over the polarity's deciding clamp end: for `Down`, `Empty <=> !filter::admits(clamped_hi.view().live(), self.holes.iter().map(|h| (h.at.view().live(), P::hole_demand(h.strict))))` (with the crossed-clamp test kept as `le(clamped_lo, clamped_hi)`, or reduced to `!le(floor, ceiling)` since `floor <= hi` and `lo <= ceiling` are established by the fused walk returning `Partial`); dually `clamped_lo` for `Up`; a new sealed method naming which clamp end decides replaces `hole_covers` (its only caller). `admits` returns false exactly when some hole's subtraction holds on the probe (filter.rs:221-230), which is the `any(hole_covers)` predicate. Then restate the `# Complexity` text to the traversals the code performs (the fused walk's own hole factor is span-causally-24). Add a many-hole fuelscape variant for the rendered island. Acceptance: the meter row goes green; the doc says what the code does; `coverage_is_exact_on_the_two_party_grid`, `coverage_clamp_refinement_is_exact`, and the new pin of span-causally-35 stay green.

Ruled (31): the fused membership walk over the deciding clamp end
replaces the per-hole loop; the `# Complexity` text is restated to the
traversals the code performs (linear in bits decoded, per-interval work
proportional to live holes); a many-hole fuelscape variant is added for
the rendered island (its survey run is the coordinator's). The three
named tests and span-causally-35's pin stay green; the `p2-rows` row goes
green.

### span-causally-24 (medium, claim): ruling 31

Resolution: owner decision, then wording. (a) Restate the contracts with the hole-count factor: "linear in the bits decoded (each stream once); per-interval work proportional to the number of live holes, `O(k · (|probe| + |self|))` in operations", and replace "linear time" at causally.rs:85-86, query.rs:28-30, polarity.rs:211-212 with the actual payoff of the polarity restriction (a polynomial decision procedure with an exact verdict; the SAT reduction is span-causally-33). (b) Keep the linear promise, which needs an indexed per-hole design. In both cases add a k-scaling instrument in the touch (accumulator) currency or fuel, never scan bits. Also correct filter.rs:46-48 (outside this partition). Acceptance: a touch-currency row with probe `v` fixed and `Q_k = until(&h_1) & ... & until(&h_k)` at k in {8, 64}; under (a) it pins the ratio ~8 as the declared model; under (b) it asserts the difference is bounded by the added holes' bits and fails today. The public docs no longer say "linear time" without the hole-count clause.

Ruled (31): option (a)'s wording at `causally.rs:85-86`, `query.rs:
28-30`, `polarity.rs:211-212`, and `filter.rs:46-48`: linear in the bits
decoded, per-interval work proportional to the number of live holes,
`O(k · (|probe| + |self|))` in operations; "linear time" without the
hole-count clause appears nowhere. The row landed in `p2-rows` pins the
declared model.

### skyline-sweep-place-masked-21 (medium, claim): ruling 31

Resolution: either (a) restate the bound accurately at filter.rs:37-48 and the public docs it feeds (scan bits O(|v| + Σ|b_i|); folds O(k|v| + Σ|b_i|); sign reads and advance bookkeeping O(k · (|v| + Σ|b_i|))), with the accurate comparison (the fusion saves k-1 probe decodes and pays a k factor on the bounds' boundaries), and pin the exponent in k with a two-scale touch row; or (b) restructure: read only pairs whose cursor stepped this round (a bound step changes one pair; a probe step changes all k, which the composition also pays; `Directions::fold` is idempotent on an unchanged sign), and pick the deepest slot with a heap keyed by depth, reaching Θ(k|v| + Σ|b_i| · log k); the single-bound identity rows are unmoved because every interval of a two-stream overlay is dirty. Acceptance: a committed tests/meter.rs row builds k and 2k `Demand::NotBefore` holes with pairwise-disjoint leaf-boundary sets against the probe `Version::new()` (below every hole, so nothing exits early), measures `touch_ops` over `filter::admits`, and asserts the ratio sits under the stated exponent, with a liveness floor of k touches (one read per bound on the first interval).

Ruled (31): option (a): the accurate bound at `filter.rs:37-48` and
the public docs it feeds, with the accurate comparison. Option (b)'s
heap-keyed restructure is not taken.

### span-causally-25 (low, claim): ruling 31

Resolution: rewrite the paragraph as three clauses once span-causally-24 and -36 are settled: atoms and named constructors `O(1)`; membership and coverage as fused walks over the probe(s) and every bound (with the hole-count factor the owner chooses); conjunction linear in the bounds plus one comparison per cross-side hole pair. Acceptance: the module summary, the conjoin island contract, and the `contains`/`coverage` islands agree on the hole-count dependence.

Ruled (31): the module summary as three clauses, with the hole-count
dependence ruling 31 fixed (per-interval work proportional to live
holes), landing after -24 and -36 in this series.

### span-causally-28 (low, verification-gap): ruling 31

Resolution: add an enforced row (a `scan-meter` or touch row in tests/meter.rs) whose operands carry k pairwise-concurrent holes with k scaling, declaring the k·m model at the cell; either add a many-hole fuelscape variant or relabel the existing island's contract to the one-hole shape it samples. Optionally fuse the survive filter through `filter::admits` with per-bound verdicts. Acceptance: an enforced row exists whose reading quadruples when k doubles at fixed bound size, and the island's contract describes what its operands measure.

Ruled (31): the row landed in `p2-rows`; here, either the many-hole
fuelscape variant (with span-causally-36's) or the island's contract
relabeled to the one-hole shape it samples. Take the variant so the
island describes the regime the row measures; report if the guest's
register machine cannot express the many-hole operand, in which case
relabel and say why.

### skyline-fill-grow-2 (medium, claim): ruling 2

Resolution: (1) Instrument first: add a `peak_heap` reading to `memo_resolution_cost`'s `tick_run` (the `PeakAlloc` harness the tick rows use) at two scales for `MemoChain(distinct)` and `MemoComb`, judged per input byte; or promote those two families to the board's tick group so `MAX_HEAP_BYTES_PER_INPUT_BYTE` judges them with their own `(event, id)` pair. (2) If the reading is red, either declare a family model at the constant the owner ratifies (as `ASCEND_CLIFF_TICK_HEAP_BYTES_PER_INPUT_BYTE` does) or cure, measuring first: store narrow links and suspended heads and keepers at machine width (the `Boundary::Word | Wide(Accumulator)` trade `MinWeb` already makes), boxing only the wide ones. (3) Rewrite fill.rs:130-134 to state what holds: `PreFrames` holds bits and slot deltas; the suspend stack (`SuspendedLevel`) holds one head and one keeper accumulator per level with a recorded-but-unresolved first child, moved rather than copied, so their digits count toward the O(n + m) live total; and `Memo::links` holds one accumulator per nonzero link. Acceptance: a committed two-scale heap reading exists for both families under the board ceiling or a declared model whose derivation names the per-level struct cost; the heap paragraph names `SuspendedLevel` and no longer says "never an accumulator per open site-nesting level".

Ruled (2): the row landed in `p2-rows`. The cure, measured at the
parent: `Memo::links` and `SuspendedLevel`'s head and keeper stored
word-or-wide (a machine-word arm and a boxed `Accumulator` arm, 16 bytes
per entry, a checked fold that spills on overflow) through one shared
type modeled on `MinWeb::Boundary`, which also settles the watermark
partition's question about boxing `Boundary::Wide`; the suspend stack
reserved once from the id tree's site-nesting depth; the deferred
first-child head considered for direct entry into its ledger slot at
suspend time; and `fill.rs:130-134` restated to name `SuspendedLevel`.
The expected result (the comb in the low twenties of B/B, a floor of a
few B/B) is a reading of the struct layouts, to be confirmed by the row,
not assumed. No family model is declared at any constant. Rebase onto
`p2-widths`' `memo.rs` widening first.

### skyline-coding-9 (high, claim): ruling 1

Resolution: the class is decided (owner ruling 1, 2026-09-02, `triage/rulings.md`): cure it, and land the instrument first. One structural cure: emit topology flags and payload codes into two streams and interleave once in `finish()`; every repair then becomes an O(1) truncation of the topology stream plus, for absorb, one 1-bit truncation of the payload stream, and the held-leaf discipline, `lens`, and `extract_code` dissolve (`continue_verbatim` would de-interleave its source range by walking it, still linear). A smaller cure to try first (the reviewer's suggestion after the ruling, unverified): after `cascade` truncates the pair, the sibling's code is already the stream's tail, so hold it in place rather than extracting it, and make the next flush a no-op while the held code is still the tail; `extract_code` is then needed only when bits were appended after the held code, and each cascade step is O(1) plus the flag truncation. Whether this composes with `lens` and `continue_verbatim` decides between it and the two-stream builder. Restating the contract as Θ(output + Σ re-anchored widths) is off the table under the ruling. The instrument lands first, before either cure: a two-scale flatness pin on the construction below (scan bits, and peak heap), plus a committed known-bad demonstrator if the builder is rewritten. Acceptance: a committed test under `scan-meter` builds `b(d)` = the text `(0, T_{d-1}, (0, 0, 1))` iterated from `T_0 = 0` and `a = Shape::Hugeleaf.packed1(10·d)`, measures `meter::scan_bits()` around `&a | &b` at `d` and `2d`, and asserts the per-input-bit scan cost stays within ×1.25 (it reads about ×2 per doubling today by the trace); the same pair added to `assert_emits` still matches the oracle byte for byte; the island contracts and the build.rs/emit.rs cost prose agree with the code.
Construction: `fn spine_of_pairs(d: usize) -> Version { let mut t = String::from("0"); for _ in 0..d { t = format!("(0, {t}, (0, 0, 1))"); } t.parse().unwrap() }` (canonical: every node has a zero-base child, no equal sibling leaves; preorder heights 0, then 0,1 repeated). `fn join_scan(d: usize) -> (u64, u64) { let a = Shape::Hugeleaf.packed1(10 * d).version(); let b = spine_of_pairs(d); let bits = (a.encode().len() + b.encode().len()) as u64 * 8; meter::reset_scan_bits(); std::hint::black_box(&a | &b); (meter::scan_bits(), bits) }`. Expected under the doc's claim: scan/bits flat from `d` to `2d`. Expected from the trace: about `2·d·(20d)` scan bits against about `30d` input bits, so the per-bit ratio doubles per doubling.

Ruled (1): cure, instrument first (done in `p2-rows`). Try the smaller
cure first: hold the re-anchored sibling's code in place at the stream's
tail rather than extracting and re-splicing it, with the next flush a
no-op while the held code is still the tail, so `extract_code` is needed
only when bits were appended after the held code; measure on the row and
on `assert_emits`. Whether it composes with `lens` and `continue_verbatim`
decides between it and the two-stream builder; abandon the smaller cure
only on measured evidence (the row still red, or a byte mismatch against
the oracle), never on anticipated complexity, and record the measurement
either way. The island contracts and the `build.rs`/`emit.rs` cost prose
end up agreeing with the code. Restating the contract as
Θ(output + Σ re-anchored widths) is off the table.

### skyline-coding-29 (medium, claim): ruling 36

Resolution: owner ruling on a ratified model; two consistent options. Cure: carry `span` (and `drop`) up the spine instead of re-summing them at each level (fold the small `entry_step` into the moved child summary in place; an `Accumulator` per flowing summary keeps the fold amortized O(1) across carry cliffs, and only a printed base pays a magnitude read), measure at the parent on `SKYLINE_RENDER_*` and the mirror-wide cells, then retire the declared model and the liveness pin together as ceilings.rs:425-429 prescribes. Accept: replace "superlinear, subquadratic"/"n log n" with the derived class Θ(|self| + depth × max interior summary width + k log k) in ops.rs and the island, state both mechanisms in text.rs's render doc, delete the "genuinely quadratic still reads red" sentence, re-point asymptotics.rs:45-46 at a sentence that exists, and add the closed-form witness below so the instrument pins the order rather than a scale-bound fit. Acceptance: either the mirror-wide cells and `render_merge_superlinearity_is_alive` both read linear after the cure (the pin flips red and is retired in the same commit), or the `version_display`/`version_fromstr` contracts, the text.rs render doc, and the ceilings.rs prose all state the same derived class and a committed check `render_limb_ops(s) >= s * s.div_ceil(64)` holds at three doublings.
Construction: under `limb-meter`, for `s in [1024, 2048, 4096]` assert `render_limb_ops(s) as usize >= s * s.div_ceil(64)` (each of the `s` spine levels' span sum costs at least `⌈s/64⌉` limb ops); as an order witness, the doubling ratios `render_limb_ops(2s) / render_limb_ops(s)` increase with `s` and exceed 3.5 by `s = 4096`, where an `n log n` mechanism would hold near `2·(1 + 1/log2 s) ≈ 2.2`.

Ruled (36): cure, then retire the model. Carry `span` and `drop` up the
spine (fold `entry_step` into the moved child summary in place, an
`Accumulator` per flowing summary), measure at the parent on the
`SKYLINE_RENDER_*` rows and the mirror-wide cells, then retire the
declared model, `render_merge_superlinearity_is_alive`, and `p2-rows`'
closed-form witness together in the commit where the cells read linear,
as `ceilings.rs:425-429` prescribes; `asymptotics.rs:45-46` re-pointed;
the class prose in `ops.rs`, the island, and `text.rs`'s render doc
states the derived bound with the "genuinely quadratic still reads red"
sentence deleted. The "Accept" branch of the resolution is not taken.

### crate-root-25 (medium, claim): ruling 93

Resolution: Reword to the bound the crate guarantees, for example "while every operation carries a documented, guaranteed time bound: linear in the encoded input for the core operations (tick, fork, join, comparison, the codecs), near-linear for the n-ary folds, and multiplication-bound only where the answer itself is a wide integer (rank and its relatives)"; then `just readme`. Acceptance: the opening paragraph names no bound any `# Complexity` section exceeds; the words "asymptotically linear" do not stand as a crate-wide promise while `fuelscape/version_rank.json` carries contract `O(M(|self|) · log |self|)`.
Construction: Textual: the rendered docs for `Version::rank` state `O(n (log n)^2)` in total input bytes on the same page set whose front page states "asymptotically linear"; the instruments that would fail a literal linear claim are already committed and green (`render_merge_superlinearity_is_alive`, `version_join_all_log_factor_is_alive`).

Ruled (93): once the five rows above are green, tighten the headline to the per-operation classes (linear for the core operations, near-linear for the n-ary folds, multiplication-bound where the answer is a wide integer) in this lane's final commit; every operation's `# Complexity` section carries its own bound.

### paper-fidelity-3 (medium, claim): ruling 93

Resolution: qualify the headline to what the roster supports: linear on the core operations (tick, join, meet, compare, fork, codec), a `log k` factor on the n-ary folds, `M(n) · log n` on the rank family, quadratic output on projection materialization, with each operation's `# Complexity` section as the contract of record. Acceptance: no sentence on the front page states a bound that any `contract:` row in the roster exceeds.

Ruled (93): the same headline change as crate-root-25, filed from
the paper-fidelity sweep; one commit lands both.

## Hazards and stops

- Nothing here lands before its `p2-rows` row is in your base, red. A row
  you cannot find is a stop.
- Every re-pin measured at the parent and named with both numbers; a
  committed pin that moves for a reason other than the cure it sits under
  is a stop, never a re-pin.
- `memo.rs` is shared with `p2-widths` (ruling 33's widening); land the
  memo-heap representation after that lane's commit is in your base.
- The render merge's declared model is retired only in the commit where
  the cells read linear; retiring it while any mirror-wide cell still
  reads superlinear is a stop.
- Public `# Complexity` text changes only where a ruling above names the
  new wording; any other public contract movement is a stop.
- No bench judge; the survey runs for fuelscape variants are the
  coordinator's.
