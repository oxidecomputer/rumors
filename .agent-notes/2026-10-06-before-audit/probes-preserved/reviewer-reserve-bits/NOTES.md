# reviewer-reserve-bits: resumption record

Task: review `fix/suanpan-reserve-digits-hint` (0ea0d86b test, 5bd37ff3 fix) on base 719f9e57.
Worktree /Users/oxide/src/rumors-slot-05, verified at 5bd37ff3, clean, base notes-only vs 455e97de.
Commit nothing. All experiments REVERTED; `git diff --exit-code` clean confirmed before gate.

## Established
- No caller of reserve_digits remains outside .agent-notes (git grep).
- Base test (0ea0d86b): FAIL `capacity overflow` raw_vec/mod.rs:28:5, exit 100 (base-test.log).
- malloc(2^58), malloc(2^62) -> NULL on Mac arm64 and box (malloc_probe.py). 64-bit test reaches
  allocator refusal deterministically on both; if an allocator ever satisfied it, test still passes
  (asserts value only): verdict not environment-dependent, only coverage would be lost.
- Rank::accumulate arithmetic: ceil((w+33)/32) == floor(w/32)+2 for w <= 2^64-34; differs (2^59 vs
  2^59+1) only for w >= 2^64-32, both unsatisfiable. Commit msg "within 32 bits of u64::MAX" ok.
- Headroom comment (rank.rs:514-516) rationale unsupported: probe (reviewer_probe.rs.saved, probe2.log)
  2M carry-heavy add/sub cases, digit buffer len never > ceil(w/32). Proof sketch: top-position total
  bounded < 2^33 (no carry out). Fixer's own NOTES call the slack "likely unnecessary".
- Doctest in accumulator.rs:208-209: stored_bits is working width, never reflects reservations ->
  second assert vacuous as evidence of "ignored".
- wasm32 allocator-refusal regime: temp pin bits=2^33-32 passes on fix; infallible mutant traps
  (wasm-fix.log, wasm-mutant.log). Not pinned in branch -> suggestion.
- Other branch audit/wasm32-trap-diagnosis is now 6fd48431 (amended from 4dcc0caa, reflog).
  git merge-tree 5bd37ff3 6fd48431: CONFLICT protocol/src/lib.rs and guest/src/checks.rs; pins.rs
  auto-merges. Keeping both `= 9` -> E0081. Harness/guest share protocol enum; recipe rebuilds guest.

## Jobs
- gate: background task bckzs5nbu, log scratch/gate.log (HEAD in gate.head).

## Next
- prose pass line by line; wait gate; compare vs baseline; report.

## Update (gate waiting)
- python check: old==new for all w < 2^64-32; 32 mismatches at w >= 2^64-32 (2^59+1 vs 2^59).
- Prose verdicts drafted: BLOCK rank.rs:514-516 carry rationale (provably unused margin).
  Non-blocking: doctest 2nd assert vacuous (stored_bits = working width); witnesses.rs "two ends of the
  range" imprecise (unaddressable boundary is lower: LA57 2^56 B, AArch64 2^52 B); summary line
  "if the allocator can provide it" omits addressability; wasm32 allocator-refusal pin (2^33-32) suggestion;
  truncating-narrowing mutant unobservable (residual).
- Monitor task b2zmrm47w watches gate.log.

## Verification switch
- Coordinator: landing check replaces just gate. Stopped my queued gate (remote pid 24559, only child sleep 15, never started; log gate-queued-stopped.log).
- Landing check: background task bvay7n4f4, log check.log.
r2 probe: guest checks.rs bits==1 branch + pins.rs reviewer_probe_grant -- REVERT

## Round 2 (tip 19154cd1)
- All five items resolved; grant probe reproduced Failed(Exhausted) (r2-grant.log), reverted; pin passes at clean tip (r2-pin.log). Verdict: approve.
