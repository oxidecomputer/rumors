# fixer-wasm32-growth: resumption record

Branch fix/before-wasm32-buffer-growth, worktree /Users/oxide/src/rumors-slot-14.
Task prompt: fix every input-driven buffer growth that panics past 2^30 on 32-bit;
owner ruling option A (succeed if fits, else Decode::Io(OutOfMemory)); no new variant.

## Established
- Rebased 60d534ed onto main d6d22b4c -> 932d2350 (signed G). Conflict only in
  checks.rs imports (kept both `core::fmt::{self, Write}` and `core::iter`).
  pins.rs auto-merged; suanpan case 4 and new checks are independent fns.

## Next
- Confirm demo fails on rebased base (4 new pins tests).
- Survey growth sites.
- BASE CONFIRMED at 932d2350 (run-base.log, load ~375): borsh past FAIL Trapped(UnreachableCodeReached) 87.5s;
  rank past FAIL Trapped(UnreachableCodeReached) 90.9s; borsh control PASS 91.7s; rank control PASS 216.9s (SLOW, not killed).

## Design decisions so far
- Plain `try_reserve` is NOT enough: std amortized growth asks max(2*cap, req); 2*cap > isize::MAX -> CapacityOverflow
  even when req fits (auditor's VecGrowthBoundary probe shows exactly this). Shared helper must clamp to max capacity.
- std 1.97.1 default_read_to_end: small_probe_read uses infallible extend_from_slice -> resuming std read_to_end
  past the limit could PANIC. Do not resume std read_to_end. Take<R> overrides only read/read_buf (no slice fast path).
- Rank zero-group counting: NOT doing (would make exp unbounded by memory on 32-bit -> breaks sum_iter premise,
  arithmetic panics reachable). Report as proposal.
- read_to_end decoders (Party/Version/Clock/Ranked/Span::decode): keep std read_to_end as fast path (slice
  exact reserve -> no board/fuel movement), continue with own fallible loop only when std reports OOM with
  buf full past half max (std's growth refusal state). Own-loop-only would move heap (2x cap retained in Bytes).
- Scope: decoders fallible (ruling). Operations (BitsWriter, BitStack in ops, tick frames, suanpan digits):
  report as proposal, NOT implemented (no error channel; large blast radius).
- Decoder sites so far: D1 rank groups, D2 rank BitSink mantissa, D3 rank image extend, D4 StreamBitsReader.bytes,
  D5 Ranked borsh rank_bytes, D6 CanonicalLimbs (borsh+serde Count), D7 read_to_end x5, D8 validator BitStacks
  (party frames, version open, admit path/left_was_leaf). TODO check: read_gamma_slow, text parsers, clock/span decode.
- Residual (report): num-bigint from_bytes_be/BigUint::new exact infallible allocs -> abort at true exhaustion.

## Inventory (survey done)
FIX (before-owned decoder buffers -> new private module src/growth.rs, fallible clamped growth -> Decode::Io(OOM)):
 rank.rs groups/BitSink/image; borsh StreamBitsReader.bytes, Ranked rank_bytes; count.rs CanonicalLimbs (borsh+serde Count);
 read_to_end x5 (party/version/clock/ranked/span::decode) via growth::read_to_end (std fast path + takeover);
 BitStack::try_push for validators: party/io/validate/frames.rs, version/io/validate.rs `open`, admit.rs path/left_was_leaf.
RULED OUT: gamma decode (exact set_bit alloc), text parsers (exact allocs), VersionRegionReader path on lo
 (lo in memory; path <= lo bits/3 < 1 GiB on 32-bit), admission stacks can't reach limit (<= input/2) but made fallible anyway.
OUTSTANDING (stop+propose): suanpan Digits Vec<i64> resize doubling: validator height, admission diff, Ranked rank fold.
 Height > 2^32 bits (input ~512 MiB) panics on wasm32 even after fix. Needs suanpan API (try_reserve_digits / fallible deposit).
 Verify post-fix: VersionBorshWideLeaf k=2^32+16 (auditor's original) expected Trapped (scratch pin).
RESIDUAL: num-bigint exact infallible allocs (from_bytes_be, BigUint::new, set_bit) abort at true exhaustion.
OPS proposal: BitsWriter/BitStack/tick frames etc. infallible doubling in operations.

## Implementation (uncommitted in slot-14 as of this entry)
- src/growth.rs (+ growth/tests.rs), lib.rs mod; rank.rs, borsh_impls.rs, count.rs, serde_impls.rs, bits/stack/bit.rs
  (+tests extended w/ TryPush), party/io/validate{,/frames}.rs, version/io/validate{,/admit}.rs, 5x read_to_end,
  error.rs Decode::Io doc, 6 decoder Errors docs.
- focused1.log: clippy -p before all-features OK; focused nextest running (bg).
- Scratch probes: probes.py (apply / --remove) in this dir: S1 reader k=2^32-1, S2 borsh k=2^32 (expect trap: suanpan),
  S2b k=2^32+16, S3 party chain 2^32+64 (expect Exhausted=OOM after fix), S6 count 2^27+1 limbs (expect Passed).
  Failure::Exhausted == OOM in scratch probes.
## Cost measurement plan
- Board: parent readings == coordinator/audit-check-baseline.log (a9bf84f0 tree == parent outside pins). Diff my landing check board section vs it.
- Fuel: run `just fuzzfit-calibrate` at fix and at parent (git archive HEAD~1 into target/parent-src on box); diff bands.rs.
- focused1.log: clippy OK, 187/187 focused tests pass incl. 6 growth tests + bit_stack model.
- Probes APPLIED (uncommitted) in slot-14; run-fix.log running (4 doubling_limit + scratch). REMOVE with probes.py --remove before commit; verify git diff of wasm32-pins empty.
- commit-msg.txt drafted.
- run-fix.log (first fix): borsh past PASS; rank past FAIL Failed(DecodeRejected) (OOM: clamp to 2 GiB + separate image copy);
  S1 reader Exhausted, S6 count Exhausted, S3 party Exhausted, S2/S2b DecodeRejected (not trap! cause unknown).
- REVISION: growth ladder (std doubling, then steps cap/1(clamped)|/2|/4|/8 via try_reserve_exact); read takeover on any
  full-buffer OOM (dropped >max/2 condition; test below-half removed); rank groups pushed straight into image
  (try_extend_from_slice removed). Added scratch check 93 (S2why) naming the error. run-fix2.log running.
- run-fix2.log (revised fix): 164 host tests pass; ALL 4 doubling_limit pins PASS (rank past 390.8s under load ~950);
  S1 reader k=2^32-1 Passed; S6 count 2^27+1 Passed; S3 party 2^32+64 Exhausted (OOM, expected: 1:1 stack);
  S2/S2b/S2why borsh k=2^32 and 2^32+16 Passed (my suanpan prediction WRONG for sparse heights: apply_limbs skips zero limbs).
- run-s7.log: S7 dense leaf height 2^(2^32+64)-1 via borsh: Trapped(UnreachableCodeReached); k=1000 Passed.
  => suanpan Digits doubling in validator height: DEMONSTRATED outstanding site (location inferred by elimination).
- Probes removed; probes.diff saved (final version incl. S7).
- COMMITTED fd32007d (signed). Landing check started -> landing.log
- Background: landing.log (landing check at fd32007d); fuel.log (ssh: git archive 932d2350/fd32007d into
  ~/src/rumors-slot-14/target/fuel-{parent,fix}, just fuzzfit-calibrate, diff bands -> target/fuel-*-bands.rs).
- Board parent readings: coordinator/audit-check-baseline.log (a9bf84f0 == parent outside pins). main moved notes-only.
- landing.log at fd32007d: lints FAILED (doclint summary >220 chars in growth/tests.rs, fixed uncommitted); board matches baseline (2 drift lines); readings moved only rank*/ranked* heap (board.diff). fuel.log: party_decode x1.30, version x1.13, clock x1.15. Trying cold spill in BitStack::try_push -> fuel2.log
- fuel2.log: cold spill restores fuel within ~1% of parent. AMENDED -> d0a6205f (signed). Landing check #2 -> landing2.log
- DONE: tip d0a6205f (signed). landing2.log: only board failed, 5311/0 + exactly 2 drift lines; tests 762 (757+5),
  snapshots 142, doctests 193/3, surface 14, wasm32-pins 12, fuzzfit 25, fuelscape 43. Leg logs in landing2-legs/.
  Board readings identical between fd32007d and d0a6205f; moved vs parent only rank*/ranked* heap (board.diff).
  Fuel bands: fuel-parent-bands.rs vs fuel-fix2-bands.rs (cold spill) within ~1%.
- Next: report (SubagentHandback).

## REBUILD (owner ruling: accept 32-bit panic past 1 GiB; document the limit; drop machinery)
- Superseded fix commit, kept reachable via this note (and reflog): d0a6205f30e0c3ba1c56dcb1e1fd1d49abf34b91
  "Grow before's decoder buffers fallibly past the doubling limit" (growth.rs, BitStack::try_push cold spill,
  CanonicalLimbs::push fallible, rank decode_stream restructure, 5 growth tests).
- OBSERVATION for a possible future heap branch: rank decode_stream restructure (append fraction groups directly
  to the numerator's byte image instead of a separate groups Vec + extend copy). Board heap vs parent:
  rank_decode/serde/borsh 132 rows each, 78 down / 53 up, mean -0.65 B/B, range [-4.0, +0.9], max 17.0->17.0;
  ranked_decode max 11.6->10.0, ranked_serde 10.0->8.4 (all down). Reviewer: allocates the image eagerly even
  for integral-only ranks (cliff 3.3->3.7). Would need: build image lazily on first group.
- Plan: reset to 932d2350; amend pins commit (past-limit -> assert Trapped(UnreachableCodeReached));
  docs commit (crate-level canonical section + links; suanpan digit storage note).
- trapcause.log: hook diverting 'capacity overflow' panics changes past-limit traps to MemoryOutOfBounds (rank & borsh); deliberate panic stays Unreachable => past-limit traps ARE capacity overflow. Probe removed (trapcause.diff).
- Rank restructure V2 (lazy Option image, zero-integral empty seed, in-place LE materialization) uncommitted; run rank1.log (clippy, rank tests, board, wasm32-pins)
- rank1.log: restructure V2: 452 heap readings down, 0 up; worst-case tie drift for rank_{decode,serde,borsh} default scale (improvement) -> re-pin; RANK_ ceiling 6.0 -> 5.0 (max 4.6 -> 3.7, x1.25 rounded up). Running MUTANT (no integral bytes) -> mutant.log
- mutant.log: integral-dropping mutant killed by 7 rank tests (incl. rank_wire_decode_matches_slice_reference). Restored. Re-pinned worst.rs default rank_{decode,serde,borsh} heap tie; RANK_ ceiling 5.0. Run rank2.log (rank tests, board, wasm32-pins)
- REBUILT: faba9766 pins, 15e1ae7c rank heap, 73cf4e70 docs. Landing check -> landing3.log
- landing3.log: matches baseline (only board, 2 drift lines). Legs in landing3-legs/. DONE.
- ROUND 2: old docs commit 73cf4e70 (patch docs-73cf4e70.patch), rank 15e1ae7c (patch saved). Rebuilding: pins faba9766 -> new (F1,F2), cherry-pick rank (N4 msg), docs (F1,F3,N1,N2,N3).
- ROUND 2 rebuilt: 8208efae pins, 022ae6fc rank (diff == 15e1ae7c), 5e23aeca docs. Landing -> landing4.log
- landing4: matches baseline; legs landing4-legs/. DONE round 2.

## ROUND 3 (owner: promise no input sizes anywhere in before/suanpan)
- Dropped (kept reachable on fix/before-wasm32-buffer-growth and via this note):
  pins 8208efae2..., docs 5e23aeca... (full SHAs below). Earlier: faba9766, 73cf4e70, 15e1ae7c, d0a6205f.
- Keep rank heap commit 022ae6fc content -> new branch simplify/before-rank-decode-image on current main.
8208efaeb6400fe0ad598e30fab840d329788b8e
5e23aecaf158ef7b344efa0fd4e9bb261f065303
022ae6fcedbd0f3af7022b8492c7c69410ff0255
- New branch simplify/before-rank-decode-image @ main 9f0c1134; cherry-pick -> 85a7f36a (content == 022ae6fc). Board at main -> board-main.log
- Branch: 0423eb1f rank, 0465f21f Decode::Io doc, 33fd20af ceiling doc. Landing -> landing5.log
- landing5 CLEAN; board vs main: see report. DONE round 3.
- ROUND 4 (prose repair): pre-repair SHAs 0465f21f / 33fd20af; rebuilding commits 2,3 with reviewer repair.diff
- ROUND 4 done: 4c8e5045 (Io doc), 38b4d9a5 (ceiling doc), tree == 33fd20af + repair.diff (b74d2662). Doc builds both status 0 (docs-round4.log).
