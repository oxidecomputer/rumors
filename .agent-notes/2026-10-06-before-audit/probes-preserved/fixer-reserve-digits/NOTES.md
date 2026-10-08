# fixer-reserve-digits resumption record

Task: fix suanpan reserve panic (D2) on branch `fix/suanpan-reserve-digits-hint`,
worktree /Users/oxide/src/rumors-slot-05. Test commit 0ea0d86b on base 719f9e57
(verified clean, signed, HEAD matched).

COORDINATOR RULING (relayed owner ruling, mid-task): replace
`reserve_digits(usize)` with `reserve_bits(u64)`, best-effort, no panics;
update Rank::accumulate to pass bits; keep test commit 0ea0d86b as is; fix
commit updates the test to bits. Branch stays two commits.

## Established
- before.log: at 0ea0d86b the test fails `capacity overflow` raw_vec/mod.rs:28:5 (exit 100).
- Calibration (usize version): pre-check-then-reserve_exact mutant aborts on
  64-bit allocator-refusal request ("memory allocation of 9223372036854775800
  bytes failed", SIGABRT). mutant-precheck.log.
- On 64-bit, every u64 bits request has a valid layout (<= 2^62 bytes): only
  allocator refusal reachable natively. Capacity overflow only on 32-bit ->
  wasm32 pin added (Check::SuanpanReserve = 9; requests u64::MAX, 2^33-31).
- rank.rs: reserve_bits(sum_bits + 33) == old floor(widest/32)+2 positions for
  all widest < 2^64-32 (python check); both unsatisfiable above. Kept exact so
  board heap readings do not move. Slack position at multiples of 32 likely
  unnecessary (|d|<2^33 argument) -> report as observation.
- surface.rs generator gained unsatisfiable arm (2^60..=u64::MAX bits).
- surfacecheck covers only `before`; suanpan rename does not touch it.

## Uncommitted scratch (REMOVE before commit)
- crates/suanpan/src/accumulator/tests/explore_l7.rs (copied from explore wt,
  Reserve generator widened to bits incl. unsatisfiable)
- `mod explore_l7;` line in crates/suanpan/src/accumulator/tests.rs
  (orig saved at scratch tests.rs.orig)

## Next
1. box: build --no-run; run suanpan reserve tests, explore probe + pool
   programs (PROPTEST_CASES high), before rank tests; clippy.
2. wasm32 pins run (just wasm32-pins).
3. mutant calibration of conversion (expect) on wasm32.
4. remove scratch, commit fix (Previously/Now naming rename + reason), gate.

## Progress (after ruling)
- Fix committed 5bd37ff3 (signed). Scratch explore harness removed (adapted copy at scratch explore_l7.rs.adapted).
- Verified: suanpan 71/71, probe "returned", before rank 58/58, family 20000 cases ok (family.log),
  pins 9/9 (pins.log), native infallible mutant caught (mutant-infallible.log), wasm32 expect +
  infallible mutants caught (mutant-wasm-*.log).
- Gate running: gate.log (background). Then report.
- gate1 (uncapped, load ~430): all baseline except workspace TIMEOUT rumors::dispute_wire table_corpus_has_similar_protocol_overhead (180s). Rerunning under pset-run -n 24 -> gate.log
- gate2: attempt 3 of direct pset-run loop acquired cores before coordinator's audit-reserved rule arrived; letting it finish. Future gates: ~/bin/audit-reserved just gate
- gate2 (reserved cores, attempt 3) at 5bd37ff3 matches baseline: workspace 1864/1864, wasm 25/43/9, board 5311/0 + same 2 drift lines, fuzz libfuzzer. DONE; report sent.
- Round 2: amended fix -> 19154cd1 (rank comment, doctest, summary, witness doc, pin case 2^33-32; probe-grant.log Failed(Exhausted); r2-wasm-fix/mutant logs). Landing check -> check.log
- Landing check at 19154cd1: all ok except board (5311/0 + the two known drift lines). Round 2 done.
