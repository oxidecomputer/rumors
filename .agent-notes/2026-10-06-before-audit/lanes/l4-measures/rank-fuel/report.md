## Verdict

**Candidate 2 (`rank`/`distance`/`lag` on `freeze-parade`): the cost is real, but it isn't multiplication.**
- Every freeze replaces the integrator's segment-width accumulator with a fresh one, and every deferral replaces the deferred-width accumulator the same way.
- The next width write lands near the tree's maximum depth `S`, so `suanpan` zero-fills a new digit buffer from digit 0 up to `S/32` each time.
- That adds up to `Θ(n²)` bytes of zero-fill on linear input. The products stay well inside `O(M(n))`.
- Under the backend reading of `M(n)` (`num-bigint`'s real multiplication) this breaches the documented bound. Under a schoolbook reading (`M(n) = n²`) it stays inside it.
- Either way, the argument in `integral.rs:91-93` that each input delta pays for its own work is false, and a committed band claims "rank is linear on the freeze parade" when it isn't.
- Severity: medium. Root cause isolated, repair prototyped and measured.

**Candidate 1 (`rank_sum`): a real but mild breach.**
- `Sum` is documented as `O(k + n)`, but measures as `Θ(k log R)`.
- The log comes from `suanpan`'s zero-range map (`ZeroRanges`, a `BTreeMap`). It records a range for every zero 32-bit digit run inside a summand. Once two ranges coexist, every later write pays a tree lookup.
- `suanpan`'s own cost table already documents this log factor (`log(W + 1)`), so `rank.rs` contradicts the model it is built on.
- Severity: low. Which repair is wanted is an open question for the owner.

Confirmed defects: 1 medium, 1 low. Two questions. One simplification brief. I judge the rank-time question at diminishing returns. The one lead with real yield left is outside this lane (observation 3 below).

Base check: HEAD was `e0956b25d` at start and is at end. The worktree is clean (empty `git status`), nothing committed, so no unsigned SHAs.

Departures from `common.md`, all on your instruction: everything ran on this Mac under `nice -n 10`, `-j 8`, 8 test threads. I used wasm fuel and `alloc-meter` as meters, and no wall time.

## D1: rank integration refills a zero prefix at every freeze and deferral (medium)

Record, test brief and fix note: `/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/audit-rank-fuel/defect-rank-refill.md`

**Mechanism.** `integral.rs:272` (`self.segment_width = Accumulator::new();`) and `deferred.rs:62` (`self.width = Accumulator::new();`):
- The next write goes in at `weight_shift ≈ S`.
- `Digits::add_at` then runs `self.digits.resize(position + 1, 0)` (`suanpan/.../digits.rs:205`), refilling the buffer from digit 0.
- The comments beside both sites say replacement avoids `reset()`'s scan. In fact it just moves that cost into the next write.
- Touch doesn't see the refill, since the read skips the prefix. Peak heap doesn't either, since each old buffer is freed first.
- The bug is identical at `bf2ba264`, where the scope measured.

**What the scope's fuel slope reflects.** Its fuel per byte equals a constant plus 8 fuel per zero-filled digit per byte, within 3 fuel at every rung. The constant (913) is fitted at the first rung only. The 8 is not fitted: Wasmtime charges `memory.fill` 1 fuel per byte, and a digit is 8 bytes. My own fresh-instance fuel readings reproduce the scope's within 0.2%.

| `k` | Scope fuel/B | My base fuel/B | With the repair |
|---|---|---|---|
| 512 | 997 | 995.4 | 906.3 |
| 1,024 | 1,081 | 1,079.5 | 906.6 |
| 4,096 | 1,587 | 1,582.5 | 906.7 |
| 16,384 | 3,598 | 3,593.9 | 906.8 |

The scope's multiplication hypothesis doesn't hold. On `freeze-parade` every product is one 32-bit digit times a height of at most 307 bits, `k − 1` of them. Across ten family ladders, the products' modelled total stays below 0.28 of one full-width multiplication, with no upward trend. A deliberately bad reduction order makes the same model double per doubling, so the model can detect a breach.

**Who else is hit.** `freeze-pos`, `promo-rearm`, `dense-suffix` and `arming-train` show the same quadratic refill. `lone-freeze` and `weight-comb` are clean.

**Failing tests to build.** Two bands in `crates/before/tests/meter/version_scaling/skip_adequacy.rs`, each checking allocation volume per byte with `alloc_meter`'s `bytes_allocated` and the existing `assert_flat`. Readings are exact and deterministic:
- `FZ(512)` against `FZ(1024)` fails at base: `4386968/50124 -> 17162392/100173`, ×1.958.
- `DS(2048)` against `DS(4096)` fails at base (×1.570). It still fails (×1.307) with only the segment site fixed, so it independently checks the deferral site.

**Repair.**
- In `suanpan`, `Digits::reset` clears from `min(lowest_written, highest_nonzero)` instead of from 0. Every digit below `lowest_written` is already zero by an invariant the crate states and tests.
- The integrator calls `reset()` at both sites instead of building a fresh accumulator.

I prototyped it, measured it, and reverted it:
- Fill, allocation volume and fuel are all flat on every affected family.
- `suanpan`: 94 of 95 tests pass. The one failure is the intended drop in a pinned count: `metered.rs:442`, left 64 / right 65.
- `before`: 90 of 90 measure and rank tests pass.
- Touch rises 0.831 → 0.841 per byte, far under the ceiling of 24. No ranking flips. API and formats are untouched.
- Base the fix on #122, since both edit `digits.rs` and the integrator.

## D2: `Rank`'s `Sum` pays `log R` per summand (low)

Record: `/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/audit-rank-fuel/defect-rank-sum-log.md`

**Fuel.**
- `comb-rank`, my adversary (alternating zero digits): 558 → 1,351 fuel per byte over a 2048× range of `m`.
- The board's own `dense-suffix` cell: 371 per byte while no tree exists, 546 exactly where the tree forms (`p = 64`), then up to 937.

**Deterministic counts.** Tree operations per byte are flat at 0.551, while the tree depth paid per byte rises by +0.551 per doubling over 13 doublings. Touch, fill and allocation volume are flat. That explains the scope's 779 → 1,314 reading. Its dip at the top is consistent with the B-tree's step structure (inferred, not measured).

The test brief depends on your ruling, because no existing meter sees this cost.

## Questions

1. Does `M(n)` in the rank docs mean the backend's multiplication or schoolbook `n²`? Two passages in the crate assume the backend reading: `meter.rs:1899-1902` treats a schoolbook charge as exceeding the bound, and `deferred.rs:130` speaks of the backend's bound. I recommend fixing D1 under either reading.
2. Which `Sum` repair? (a) Restate the bound. (b) Stop `suanpan` recording zero ranges inside one operand's span. That helps every accumulator caller, but a residual case may remain, which I haven't verified. (c) Take `Sum` off the zero-range bookkeeping entirely; that moves the board's `rank_sum` touch floor. I recommend (b), plus (a)'s restatement for the residual.

## Simplification brief

`Integrator::one` exists only to stream a single limb of value 1. Passing `[1]` directly does the same without the field. Behavior-preserving, two lines:
`/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/audit-rank-fuel/simplification-integrator-one.md`

## Observations

`/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/audit-rank-fuel/observations.md`
1. The board's heap column records peak only. Allocation volume would have caught D1, and `alloc-meter` already reports it.
2. The scope's other flagged `ranked_*` exponents go through the same integrator. I infer they share D1's cause; I measured only `rank`, `distance` and `lag`.
3. Lead for other lanes: the zero-range tree's per-write log may also sit inside `O(n)`-documented accumulator users (`min_ticks`, `tick`, projection, placement, `range_minima`, the region reader). Not examined.
4. The scope's multiplication hypothesis is refuted, with the calibration evidence above.
5. The `usize` reasoning in `rank.rs:1019-1027` is sound; no finding.
6. The misleading comments at D1's two sites are covered by D1's fix note.

## Coverage, instruments, resumption

Coverage table, instrument inventory, blind spots and how to resume:
`/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/audit-rank-fuel/coverage.md`

All instruments live in the scratch directory; none is committed:
- the probe example `rank_fuel_probe.rs`, applied with `probe-only.patch`
- the wasm fuel probe in `fuelprobe/`
- `scratch_rank_fill.rs`, the allocation-volume prototype of the demonstrator's bands
- the patches `fix-only.patch` and `seg-only.patch`

All of them, and the logs, are in `/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/audit-rank-fuel/`; `NOTES.md` there is current.

Blind spots: I ran neither the board nor the landing check, and I didn't prototype `Sum` repair (b).

Build directories to retire:
- `/Volumes/forge/build/ee/0b6b3d09301937` (the worktree's)
- `/Volumes/forge/build/7d/5f64abf5a03aae` (the fuel probe's)
- `fuelprobe/target` in the scratch directory

Process note: one `cargo test --no-run` failed because my untracked probe example built only against the patched tree. Cargo builds examples with tests. I moved the example to scratch and reran. Nothing was lost, and the tree was restored to a zero-line diff each time.