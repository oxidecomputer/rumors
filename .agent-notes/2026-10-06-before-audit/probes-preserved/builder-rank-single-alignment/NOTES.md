# builder-rank-single-alignment: resumption notes

Task: branch `simplify/rank-single-alignment`, worktree /Users/oxide/src/rumors-slot-20,
base 19154cd1 (tip of unlanded fix/suanpan-reserve-digits-hint). Brief:
/Users/oxide/src/rumors/.agent-notes/2026-10-06-before-audit/coordinator-briefs/simplify-rank-single-alignment.md

## Established
- HEAD verified 19154cd1, clean.
- num-bigint 0.4.8 `biguint_shl` narrows only `shift / BITS` to usize (expect "capacity overflow").
  So u64 shift works on wasm32 for gaps < 2^37.
- Exponent bound: decode exp = fraction bits read (groups held in a Vec, so < memory);
  parse exp = fraction.len() of an in-memory str; Version::rank / pair_rank exp = max_depth,
  and `VersionSubtreeReader::descend` pushes one path entry per internal node read from the
  stream, so depth <= stored bits; add/sub/sum normalize, so result exp <= max operand exp.
  => on wasm32 every reachable gap < 2^35 < 2^37. Inference confirmed.
- Native: alignment always fits, so accumulate never runs on 64-bit; board rows cannot move from routing.
- Fuel: fuzzfit bands (crates/before/fuzzfit/harness/src/bands.rs) re-derivable byte-reproducibly
  via `just fuzzfit-calibrate`; plan to run at parent and tip and diff.
- Memory: no committed instrument; plan temporary probe in wasm32-pins harness run_raw
  printing memory pages after each check (reverted).
- Wall-clock: common.md forbids; coordinator brief asks runtime -> dispute in report.

## Next
1. Parent measurements on box (board acceptance + worst-cases, fuzzfit-calibrate, pins memory probe).
2. Implement; tip measurements; calibrate defect injection; landing check.

## Background jobs
(none yet)

## Progress (update 1)
- Temporary probe in crates/before/wasm32-pins/harness/src/lib.rs (prints PROBE ... pages=) -- MUST REVERT before commit.
- Source edits done locally (uncommitted): rank.rs `aligned` helper replaces alignment_fits+accumulate;
  pins.rs test renamed rank_arithmetic_crosses_the_usize_gap_boundary; guest checks.rs + protocol doc restated.
- Kept `.clone() <<` and `a + &b` exactly as native route (zero native movement). Observation for report:
  `&self.num << gap` would avoid a clone transient (heap drop, possible worst-case flip) -- separate change.
- before no longer calls reserve_bits after this (coordination note for fix/suanpan-reserve-digits-hint).
- Parent measurement: background job, local log measure-parent.log; box outputs ~/src/rumors-slot-20/target/meas-parent/.
  script at box target/measure.sh (copy in scratch).
- Native calibration tests: rank_triple_laws_on_seeded_ranks, rank_sum_equals_the_pairwise_fold, rank_known_values.
- Parent artifacts in scratch meas-parent/. Parent pins probe pages: 50207, 50207, 42014, 58399 (cases 1-4).
  Parent refit already differs from committed bands.rs (145+/144-): pre-existing, compare refit-vs-refit.
- First tip run was STALE for wasm (edits made during parent build had older mtimes); saved as meas-tip-stale/.
  Touched edited files; tip rerun in background -> measure-tip.log, box target/meas-tip/.
  Validity check: pins PASS line must show new name rank_arithmetic_crosses_the_usize_gap_boundary.
- WIP commit 0e3753b4 (probe removed). Untracked dump bin crates/before/fuzzfit/harness/src/bin/dump_rank_fuel.rs: DELETE before final.
- Valid tip readings: board acceptance + worst-cases byte-identical; pins memory pages identical (50207,50207,42014,58399).
- Fuel per-sample (parent vs tip, same 15429 rows): ff_rank_add all 5143 higher +45..75 (mean 56.3);
  ff_rank_checked_sub accepted 1217/9069 higher +50..72, rest equal; rejected arm equal.
  #[inline]: no change. #[inline(always)]: +30..60 / +0..52. Mechanism (wasm disasm): helper out-of-line +
  num-bigint to_usize expect no longer provably dead (old guard proved gap<2^32) + different clone lowering.
  Decision: keep plain helper; report. Refit tip ff_rank_add slope .315056 int 2.459400 wa .172719 wb .403490;
  checked_sub wa .462441 wb .496821. Within REFIT_TOLERANCE 0.7 and ENFORCE_MARGIN 0.2.
- Next: native+wasm calibration injections, fmt/clippy/wasm32-pins, amend commit msg, landing check.
- Calibration: A (b shifted by self gap): native 6 FAIL (rank_triple_laws_on_seeded_ranks, rank_sum_equals_the_pairwise_fold,
  algebraic_laws rank_triple_laws, rank_cmp_agrees..., harmonic_rank..., deeper_hoisted_window...), pin case 1 WrongValue.
  B (b gap `as usize as u64`): pin case 2 WrongValue (my self-inverting prediction refuted: normalization changes sum exp).
  Logs in scratch calib-A/, box target/calib-B/.
- Final commit 66ebbcb0 (signed G). Landing check running: scratch landing-check.log; box logs target/audit-check-logs/.
- DONE: landing check matches baseline (pre-board-pin form). Logs scratch landing-logs/. Report next.
