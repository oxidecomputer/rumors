# audit-rank-fuel: resumption record

Task: decide whether `before`'s rank ops meet `O(M(n))` on freeze-parade-like
inputs; candidate 1 is `rank_sum` (`O(k + n)`). Worktree
`/Users/oxide/src/rumors-slot-56`, detached at `e0956b25d` (verified HEAD,
clean tree at start). ox-east-1 is down: local builds under `nice -n 10`,
`-j 8`, <= 8 test threads (task prompt overrides common.md's box-only rule).

## Established (by reading)

- No commit on main touches measure/rank/suanpan between merge-base
  `d1aa90e04` and `bf2ba264`; all differences are the #122 branch's.
- `freeze` replaces `segment_width` with `Accumulator::new()` at both bases.
- freeze-parade FZ(k): left spine depth S = 64k sets the scale; the block of
  2k wide leaves (values ~2^(P+2+bitlen k), P = 290) sits at depth ~2+log2 k,
  so its regions' weight_shift is ~S. In-pair delta 2^P (no freeze), cross-pair
  delta 1 fires a freeze: k-1 freezes. No deferral (parked ~ drift width).
- Products in FZ(k): each freeze does one single-digit product
  (~300-bit height x one 32-bit digit). O(1) each, O(k) total: NOT the cause.
- Suspected mechanism (H1): after each freeze, the next `interval` writes
  `segment_width.add_shifted_limbs(~S, [1])` into a FRESH accumulator, whose
  `Digits::add_at` does `digits.resize(pos+1, 0)`: zero-fills ~S/32 digits
  (8 bytes each) per freeze => Theta(k * S) = Theta(n^2) bytes. Not touch-
  counted, not peak-heap visible. `DeferredIntegral::push` has the same
  pattern (`self.width = Accumulator::new()`).
- Scope's fuel/byte numbers fit a + b*k exactly (913 + 83.8*(k/256)), i.e. a
  pure quadratic term, not Toom-3 n^1.465.

## Open

- Account for the 4x gap: fit says ~64 k^2 fuel, fill alone is ~16 k^2 bytes.
  Look for realloc copies, read-side growth.
- rank_sum: accumulator adds at fixed position; touch flat. Check allocator /
  conversion / drop costs.
- White-box: families with many deferrals at deep positions; pair driver.

## Next

Read suanpan value/operand/operators/read paths; then build a scratch
instrument (counting allocator + fill counter + product width log).

## Milestone 1 (probe built and run)

- Probe: uncommitted `crates/before/examples/rank_fuel_probe.rs` (copy in
  scratch `rank_fuel_probe.rs`) + counter patch `probe-counters.patch`
  (suanpan PROBE_FILLED in Digits::add_at resize; before PROBE_PRODUCTS in
  SparseWidth::add_product). Build:
  `nice -n 10 cargo build --locked --release -j 8 -p before --example rank_fuel_probe --features meter,touch-meter`
  Run: `target/release/examples/rank_fuel_probe <family> <lo_exp> <hi_exp> <ops>`
- fz-rank.tsv: FZ(k) k=2^4..2^14: touch/B flat 0.831, peak/B ~0.98 flat,
  products = k-1 single-digit (h<=307 bits x 32), sum_model64 = 5k (linear),
  filled digits/B doubles per doubling (16 k^2 bytes total).
- Scope's fuel/B = 913 + 8*filled/B at every rung within 3 fuel (k=512 is
  the scope's 50,124-byte rung; my earlier k=256 guess was wrong).
  => quadratic fuel term IS the zero-fill (1 fuel/byte memory.fill).

## Milestone 2 (mechanisms established)

- Candidate 2 ROOT CAUSE: fresh-accumulator refill. integral.rs:272
  (`segment_width = Accumulator::new()` per freeze) and deferred.rs:62
  (`width = Accumulator::new()` per deferral); next write at shift ~S does
  `digits.resize(pos+1, 0)`. Attribution (attrib.tsv): FZ all segment site;
  dense-suffix 2:1 segment:deferred; promo-rearm both; freeze-pos segment;
  arming-train both. All quadratic. Products linear or <= ~0.2 M(n).
- Fix prototype (probe-plus-fixproto.patch, uncommitted): Digits::reset clears
  [min(lowest_written, highest) ..= highest]; integrator uses reset() at both
  sites. fixproto.tsv: all fills flat, alloc/B flat, touch +0.01/B.
- Calibration: unbalanced split mutant (`lo + 1`) -> sum64/B doubles per
  doubling on arming-train-n, exceeds M64/B at k=256 (mutant-split.tsv);
  reverted, diff identical (pre-/post-mutant.patch).
- Candidate 1: rank_sum touch/fill/alloc/peak flat on FZ, DS, WA. New:
  ZeroRanges BTreeMap probes: comb-rank adversary (alternating zero digits)
  tree_log/B +0.551 per doubling over 13 doublings => Theta(k log R).
  dense-suffix tree size grows (2..127), 0.86 probes/B. rank.rs Sum doc says
  O(k + n); suanpan documents add as O(L log(W+1) + G).
- Background: bf72jseei runs suanpan suite + before measure tests on the fix
  prototype -> fixproto-suanpan.log, fixproto-before.log. Expect the
  metered reset touch pin (metered.rs:440-447) to fail (held+1 -> held).

## Next

Write deliverables: defect record + test brief + fix note (zero-fill),
question on M(n), rank_sum log-factor record, inventory; then revert
worktree to clean (git diff empty) after saving patches.

## Milestone 3 (fuel + allocation witnesses; worktree restored)

- Worktree restored to clean HEAD e0956b25d (no tracked diff, no untracked
  files). Probe example moved to scratch rank_fuel_probe.rs; scratch test to
  scratch_rank_fill.rs. Patches: probe-plus-fixproto.patch (probe counters
  + fix prototype), fix-only.patch, seg-only.patch.
- Fuel (fuelprobe/, wasmtime 48.0.5, fresh instance per reading; fuel.tsv):
  base FZ rank fuel/B 995..3594 (k 512..16384) reproduces scope within 0.2%;
  fixed guest flat 903..907. FZ distance base 1173..1850, fixed ~1170 flat.
  comb-rank rank_sum 558 -> 1351 (m 16..32768); dense-suffix rank_sum
  371,371 (no tree) -> 546 at tree onset (k=64) -> 937.
- alloc-meter bytes_allocated (scratch-fill-{base,segonly,full}.log):
  FZ rank 512->1024: base 4386968/50124 -> 17162392/100173 (x1.958);
  seg-only and full flat. DS rank 2048->4096: base x1.570, seg-only x1.307,
  full x1.000. => FZ band witnesses segment site; DS(2048->4096) band
  witnesses deferral site independently.
- Fix prototype test check (before reverting): suanpan 94/95 pass, only
  metered.rs:442 reset pin moves 65 -> 64; before measure tests 90/90.
- Fuel build dir for scratch workspace lands in /Volumes/forge/build/<hash>
  of the fuelprobe path: report for retirement.

## Next

Write deliverables in scratch: defect-rank-refill.md, defect-rank-sum-log.md,
simplification brief (Integrator::one), observations.md, coverage.md; then
handback.

## Round 1 complete (handed back)

Deliverables written: defect-rank-refill.md (D1, medium), defect-rank-sum-log.md
(D2, low, blocked on an owner ruling), simplification-integrator-one.md,
observations.md, coverage.md. Worktree clean at e0956b25d; no commits made.
Open questions: which M(n) (D1); which Sum repair (D2). Next if resumed:
examine the O(n)-documented accumulator users for the zero-range log factor
(observation 3); prototype D2 option 2 and test the residual; re-read the
ranked_* fuel after D1's fix.
