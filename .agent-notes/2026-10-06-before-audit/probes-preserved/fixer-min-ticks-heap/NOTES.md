# fixer-min-ticks-heap: resumption record

Task: fix D1 (min_ticks heap) on branch fix/before-min-ticks-heap, worktree
/Users/oxide/src/rumors-slot-16. Base verified: HEAD ba094bbe (signed, Good), clean at start.
Briefs read: common.md, fixer.md, README, baseline.md, defect-D1, fix-note-D1, demonstrator NOTES.
Remote wrapper runs under `set -e`: use `cmd || echo x=$?` to continue after failures.
Board: export AMP_BOARD_SHARDS=24 (audit-check does; default is 192 shards).

## Established
- Parent board (ba094bbe), parent-board.log: 5401 green / 1 red; RED version_min_ticks x
  jump-rising-spine heap 124.9/B (5011->10011), 125.0/B (20011->40011), 177.2/B small (61->111).
- segvec 0.2.0 is the only crates.io segmented vec: last release 2023-07-31, 418 recent dl,
  internal get_unchecked_mut unsafe, pop never frees. Decision: hand-roll safe Vec<Vec<T>> stack.
- Stored format is a leaf-delta stream (PayloadKind::Delta zigzag codes); each step's
  height_change is exactly one encoded delta -> cost argument for the i32 freeze holds per delta.
- Plan for mech 2: freeze inside HeightPrefixes::add_leaf (already reads live once per leaf, so
  no extra touches) when the offset doesn't fit i32; predicate owned by contributions.rs.
- Plan for mech 1: ChunkStack<T> (Vec<Vec<T>>, top never reallocates) in range_minima, used for
  Boundaries::payloads. ZST payload (tick walk, P=()) never allocates (Vec<()> cap = MAX).

## Scratch (MUST remove before commit)
- crates/before/src/version/measure/fixer_probe.rs (copy in scratch dir) and the
  `#[cfg(test)] mod fixer_probe;` lines appended to crates/before/src/version/measure.rs.

## Background jobs
- parent-probe-worst.log: parent worst-cases map + probe (dev lib test).

## Next
- Read parent probe; implement mech 2 alone -> probe+board; mech 1 alone; both.

## Progress (update 2)
- Parent probe (parent-probe-worst.log, dev lib test, peak_alloc like the board): 756 readings,
  value ok everywhere. Parent worst: jump-2^31..2^287 at 2^16+2 levels: 244-308 B/B (spill slots
  doubling); plain rising 39.4 B/B at 2^k+2 (RED off-board); step-2^64/2^65 RED 29.4 B/B at 2^k+2
  (14.7 steady) -- a red family at the parent beyond D1's description.
- Parent worst map in parent-worst.log: version_min_ticks heap worst jump-rising-spine 124.9/125.0,
  runner-up propagate-seam 14.22 (default) / 14.25 (acceptance).
- Patches: mech1.patch (range_minima ChunkedStack), mech2.patch (min_ticks freeze + docs).
  Worktree state for run 1: mech2 only (mech1 reversed; chunked_stack files show D = intent-to-add).
- Running: mech2-run.log (probe + board acceptance).
- Next: mech1 only (git apply -R mech2.patch; git apply mech1.patch) -> mech1-run.log; then both.

## Progress (update 3)
- Board deterministic: demonstrator's parent log vs mine: 0 moved readings.
- Mech2 EAGER (freeze every leaf whose live offset leaves i32; mech2-eager.patch, mech2-eager-run.log):
  JR 20.2/20.2/31.0 (still RED, now mech-1 sawtooth); but dominated-undercut heap e 0.00 -> 0.70.
- Mech2 LAZY (owner's literal guidance: freeze only when a STORED contribution's offset leaves i32;
  HeightPrefixes now owns live L + width freeze (advance), narrow(leaf,total) at
  ContributionStore::store; mech2.patch, mech2-run.log, mech2-compare.txt): same JR numbers; STILL
  dominated-undercut heap 0.1->2.6 (e 0.00->0.70), jump-pair 0.0->1.4, cancelling-chain 0.0->1.3,
  freeze-pos 1.2->2.1, propagate-seam 14.2->16.3. Cause: a frozen component is permanent (part of the
  F chain), while the spill slot it replaces was recycled on settle. => growth-rate change = STOP
  condition per task brief. Plan: finish mech1-only + both measurements, then report, likely no commit.
- Mech1 now also chunks Boundaries::wide_values (step-2^64 RED at parent comes from Vec doubling).
- Running: mech1-run.log (no exit= line; wait for wrapper process to end).

## Progress (update 4)
- Mech1 only (payloads + wide_values chunked; mech1-run.log, mech1-compare.txt): JR unchanged
  (125.7/125.2/187.8 -- spills dominate); plain rising 39.4 -> 14.05 at 65538 levels; step-2^64
  29.38 -> 18.84; board moves 13 (ascend-cliff 11.4->9.4, propagate-seam small 17.3->20.9,
  JR small 177.2->187.8, ...). Small-size rises: geometric chunk edges at 4,12,28,...,2^k-4 hold
  ~2h vs Vec's 1.5h-3h; all under ceiling via intercept. chunked_stack tests pass (3/3).
- peak_alloc 0.3.0 realloc = alloc+copy+free (verified in source).
- Running: both-run.log (probe, before+suanpan nextest, board acceptance, worst-cases).
- Decision forming: growth-rate change from mech2 => stop & report, no commit (fixer.md: leave
  branch at test commit). Save final patches in scratch: mech1.patch, mech2.patch (lazy),
  mech2-eager.patch.

## Final (update 5)
- Both (both-run.log, both-compare.txt, both-board.log, both-worst.log): board 5402 green / 0 red;
  JR 14.3/14.1/23.9. STOP conditions all from mech2: seam_plunge touch 29752 > ceiling 26945
  (skyline_min_ticks_descending_boundary_is_flat_per_unit); heap exponent changes dominated-undercut
  0.00->0.70, jump-pair 0.02->1.00, cancelling-chain 0.06->0.82; propagate-seam heap 14.2->17.0
  becomes worst (flip at both scales). Suite 760/761 pass; probe values 0 wrong.
- Worktree restored to ba094bbe, clean. Patches in scratch apply cleanly: both/mech1/mech2.
- Verdict: STOPPED, no commit; landing check not run (branch unchanged from demonstrator's).

## Round 2 (coordinator relayed owner ruling a5dfe5a8: land both.patch, raise seam_plunge ceiling)
- Verified ruling text in defect-D1 file (commit a5dfe5a8).
- Found before committing: seam_plunge's DESCENDING_PARK_SURPLUS_BAND (1514, 2523) on
  control-minus-plunge also breaks: parent +2021, both.patch -1060 (needs negative floor; tripwire
  doc inverted). Ruling cites "about 10%" (vs ceiling); vs parent reading the rise is +52.5%
  (19503 -> 29752). My first report omitted the parent reading.
- Full meter diffs parent vs both (parent-meter.log, fix-meter.log, meter-diff.txt):
  seam_plunge 19503/21524 -> 29752/28692; seam_stop 28361/19469 -> 35876/29703;
  freeze_position 187990 -> 172017; promotion_rearm 880062 -> 848074; reveal_comb 22725 -> 20726.
- Variant both-deferred.patch (leaf offset added after observe_leaf; narrow does no total
  subtraction): seam_plunge 22572 (< ceiling 26945), control 21524 (= parent), surplus -1048 (band
  still fails); promotion_rearm 768023; reveal_comb 20476; freeze_position small 71017 touches <
  73328 bytes => one-touch-per-byte liveness floor fails. Board 5402 green; probe 0 wrong.
- STOPPED again, nothing committed; worktree clean at ba094bbe.

## Observation (inferred, not measured): inline prefix field overflow
- StoredContribution's prefix field is 23 bits. Each narrowing freeze adds a prefix, at most one per
  63 input bits, so past 2^23 prefixes (>= ~66 MB of stored input under narrowing) every new
  contribution spills again (48-byte slot each, recycled). Payload-heavy families at that scale
  (e.g. a rising spine of 2^32 steps, ~8.5 B/level) could exceed the heap ceiling. Unmeasured.

## Round 3 (owner re-ruling d3020960: land both-deferred + conditions)
- Applied both-deferred.patch; added WS(b,d) family (wide-step-spine, b=31, depth 600), per-leaf
  floor in tests/meter/version_scaling.rs min_ticks_family_run, surplus band (-1310,-786) + doc,
  lowered 3 ceilings, worst.rs re-pins (version_min_ticks heap -> propagate-seam; wide-step-spine
  in VERSION_STREAM_FAMILIES, clock_split_array, clock_borsh_deserialize ties), item-4 inline
  prefix bound + min_ticks/tests.rs. Phase attribution via scratch FIXERPHASE (removed).
- Swaps (all reverted, verified): dead meter -> floor fails (runA.log); residue read -> ceiling
  fires (runB.log), with ceiling off -> band fires -6168 < -1310 (runB2.log); spill prefix 0 ->
  both min_ticks tests fail (runB.log).
- r3.log: clippy clean both sets; 764/764 pass (1 leaky: coincident_span, unrelated);
  pin.log: only the 2 count_display drift lines. Board r2.log: 5493 green / 0 red.
- Commit message drafted: commit-msg.txt. NOT yet committed.
- Running: l3-long.log (graft l3_probe.rs + mod line in tick.rs; MUST remove before commit).
- Next: remove graft (verify tick.rs diff empty), commit signed, landing check in background.
- COMMITTED d1b5ff23 (signed Good). Landing check -> landing.log
- AMENDED (doclint summary length) -> new tip; landing-1.log = first check (lints failed: doclint)
- Landing check at 38202ca6 (landing.log): all legs ok except board = 5493 green + 2 count_display drift lines. tests 764/764 (+142 rumors), docs 193+3, wasm 8/25/43. DONE; report next.

## Round 4 (review round 1 repairs)
- Base verified 38202ca6 clean. Saved fix-r0.patch (ba094bbe..38202ca6), fix-r0-msg.txt, demo-msg.txt.
- Plan: reset --hard ba094bbe; amend demo (worst.rs comments/doc, B5); apply -3 fix-r0.patch;
  B3 (live = Accumulator::new()), B4 band patch, B2 seam_stop band, prose items; measure once;
  commit; debug pass; one landing check. 38202ca6 recoverable via reflog/patch.
- R1: demo 08573e15, fix 7222d49b (both G). Landing -> landing.log
- Landing at 7222d49b: all ok except board (5493 green + 2 count_display lines). DONE.

## Round 5: single-regime redesign (owner ruling a3e4df50, question 55)
- Base 7222d49b verified clean. Steps: branch archive/min-ticks-reanchor 7222d49b; invariant in
  redesign.md FIRST (stop if it fails); cost derivation; build; mutants a/b/c; debug pass;
  one real-scale probe (reviewer_probe.final.rs PROBE REAL); thresholds list; measure; commit
  demo + one fix; one landing check.
- Redesign built (uncommitted): RangeMinima MinimumClient trait (suspend/displace/cross/resume),
  Armed/Close enums, Boundary returned on cross/met, Chain client in minima.rs, contributions.rs and
  min_ticks/tests.rs deleted, WS family removed (ws-family.patch). rd1.log: clippy clean; 755/761
  non-meter pass (values OK, invariant (B) checked in range_minima property). Meters: reveal_comb
  small 11,366 -> 330,335 touches (29x): circulating wide boundary read per pop (k*b vs input k+b).
  Cost-argument premise false: RangeMinima moves boundaries without reading them. Running board
  (rd-board.log) to get exponents across the ladder.
- STOPPED: board rd-board.log: version_min_ticks x reveal-comb touch e 1.97 (219.4 -> 840.1/B; parent
  6.9/B e 1.00). Cost premise false (boundaries circulate unread in RangeMinima; chain reads per pop).
  Spike archived: archive/min-ticks-chain-spike 966a263e (G). Fix branch back at 7222d49b, clean.
  Not run (pending ruling): mutants, real-scale probe, landing check. Report sent with options.

## Round 6: variable-width records (owner ruling 386379bf)
- Base 7222d49b verified. Derivation: redesign.md Part II. Built (uncommitted): RangeMinima without P
  (suspend/retire notifications), SuspendedMinima (minima/suspended.rs, PackedU64Stack, deltas vs
  outer neighbor, explicit top), Minimum record, narrow -> MinimumHeight, contributions.rs deleted,
  inline_prefixes entry gone, min_ticks/tests.rs retargeted (WS closed form), ChunkedStack ZST
  handling+test removed. Running rv1.log (clippy, full tests dev, meter --no-capture).
- rv1: clippy clean, 764/764 dev, meters identical to 7222d49b. rv2: board 5493 green; probe REAL
  70MB 14.49 B/B (1.02 GB), 91MB 11.49 B/B (1.05 GB); JR 2.77 B/B. STOP: exponent rises vs parent:
  touch dominated-undercut .85->.88 & hoisted-window 1.00->1.01 (re-anchoring itself, present since
  both.patch), comb-scatter .90->.91 & wide-tooth-comb .88->.89 (B3), heap bigroot .98->1.00 (records).
  Archived archive/min-ticks-variable-records; fix branch back at 7222d49b clean. Mutants not run.
- Round 7 (no re-anchoring): branch archive/min-ticks-records from 08573e15; probe grafted (meter.rs.orig3, tests/fixer_probe.rs). Running rr1.log.
- rr1: clippy clean; 762/762 dev; meters identical to parent; board 5402 green; zero touch/scan
  moves vs parent; 88 heap moves; probe REAL 4.34 / 3.34 B/B (305 MB). STOP: heap exponent rises
  vs parent: bigroot .98->1.00, cancelling-chain .06->.07, memo-fanout .93->.95, reveal-hifloor
  .19->.28, tooth-tail .80->.91 (both scales). mech1 (ChunkedStack) alone does not cause them.
  Archived archive/min-ticks-records; fix branch at 7222d49b. Mutants/landing not run.
- Round 8 (path 2): branch archive/min-ticks-records-chunked from 61f55dcb; heap probe scratch (heap-probe.patch) -> hp.log
- inverted encoding (top whole, rest vs inner neighbor) applied; running rc1.log (tests dev, meters, board, worst, probe)
- Path 2 done. hp.log probe: bigroot 0 records; tooth-tail 2 records w/ 2k-16k-bit offsets;
  reveal-hifloor up to 1024 records, offsets <=4000 bits. Inverted encoding (archive
  min-ticks-records-chunked) changed only cancelling-chain (.07 -> .05). Remaining rises:
  bigroot .98->1.00, memo-fanout .93->.95, reveal-hifloor .19->.28, tooth-tail .80->.91.
  STOP & report. Fix branch at 7222d49b.
- Round 9: archive/min-ticks-records-final from 61f55dcb (origin form) + in-place push/pop + single-copy decode + fold comment. Running rf1.log
- rf1: clippy clean, 762/762 dev, meters identical (plus band test), board 5402 green, 88 heap-only moves (rf1-vs-parent.txt). Step 1: tooth-tail .91->.88, reveal-hifloor .28->.25 (main .80/.19). Re-pin min_ticks heap -> propagate-seam both sections. Running mutants mu.log (remote-side swaps).
- Committed 1a31f8d1 (G) on fix/before-min-ticks-heap atop 08573e15. Mutants mu2.log all caught. Landing check -> landing-final.log
- Landing (landing-final.log): surface/docs/lints/wasm/tests ok; board 5402 green x3; worst-cases-pin fails only on the 2 pre-existing count_display x heap drift lines (same at parent). Reported.
