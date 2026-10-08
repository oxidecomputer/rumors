# auditor-l7 resumption record (lane L7: suanpan and its uses in before)

Read first after any compaction: briefs/common.md, briefs/auditor.md, briefs/lane-l7-suanpan.md (all under /Users/oxide/src/rumors/.agent-notes/2026-10-06-before-audit/), then this file. Verify claims below against the named logs/commits before relying on them.

Scratch dir (S): /private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/auditor-l7
Worktree: /Users/oxide/src/rumors-audit-l7-suanpan, branch explore/l7-suanpan (private), base 58285ca5.
Explore commits so far: 7d5fae48 (pool model + touch accounting), df000229 (threshold values, gap shifts, probes, adversary), d43fa15e (wasm32 wrap probe + fuel instrument), then "Add L7 touch replay probe", "Add L7 dense-base fuel control and encoded-size probe", a959e13c (zero-shift guest cases). All signed normally so far.

## How to run things

- Remote wrapper: /Users/oxide/.claude/skills/building-on-illumos/scripts/on-illumos.sh <worktree> 'unset CARGO_TARGET_DIR; ...' (syncs the local tree; never edit the tree while a mutation run is in flight).
- suanpan explore tests: `cargo nextest run --locked -p suanpan --features touch-meter -E "test(/l7_/)"` (NEVER --all-features: it enables the l7-proto, l7-proto-w, l7-o1, l7-high-log explore features); L7_CASES env sets proptest cases; L7_PRINT prints ratios.
- Long runs: use `cargo test --lib -- --test-threads 1 <filter>` (nextest kills at 180 s). NEVER run the touch property in the same process as other tests: the touch meter is process-global (a two-thread run produced a bogus failure, see long1.log).
- wasm32: `cd crates/before/wasm32-pins && cargo build --locked -p wasm32-pins-guest --release --target wasm32-unknown-unknown --target-dir ../../../target/wasm32-pins && cargo build --locked -p wasm32-pins-harness --tests --release && WASM32_PINS_GUEST_WASM=$PWD/../../../target/wasm32-pins/wasm32-unknown-unknown/release/wasm32_pins_guest.wasm cargo nextest run --locked --cargo-profile release --no-capture --run-ignored all -E "test(/zz_l7/)"`. Fuel readings test `zz_l7_fuel_readings` (~10 min; L7_DENSE=1 for the dense control).
- Mutation calibration: S/mutate.py <M> apply|revert (unique-string swaps; M1..M16), S/calibrate.sh <M...> (runs /l7_/ tests), S/calibrate_f.sh '<filterset>' <M...> (any filter). Each checks `git diff --quiet` after revert; commit everything first.

## Established (verified)

1. D1 (high, 32-bit): `Digits::apply_limbs` (crates/suanpan/src/accumulator/digits/add.rs:89-95) takes limb_index from `Iterator::enumerate`; on wasm32 release a nonzero limb at index 2^32 wraps to position 0 instead of the documented panic. Evidence: S/wasm1.log, S/wasm2.log (case4 Failed(WrongValue); case6 Failed(WrongBytes) = value reads back exactly 1; control case5 traps). Fix `(0u128..).zip(limbs)` verified: S/wasm3-fix.log (cases 4 and 6 trap; existing suanpan_rejects passes); reverted, diff clean. Record: S/deliverables/D1-limb-index-wrap.md (needs the fix-verification line updated to cite wasm3-fix.log).
2. D2 (low): `reserve_digits(usize::MAX)` panics "capacity overflow", no `# Panics` doc. Evidence: S/run3.log. Record: S/deliverables/D2-reserve-digits-panic.md.
3. Cost composition (finding, record not yet written): suanpan's per-update `O(log(W+1))` (BTreeMap in ZeroRanges, Many mode) surfaces in before's O(n)-documented `Version::decode` and comparison. Family: root base sum_{i<r} 4*2^(64i) (r-1 zero ranges after the first deposit) over 64r alternating +1/0 leaves. Fuel per byte (S/wasm2.log + sizes from S/sizes.log): decode 1487.0, 1568.9, 1654.4, 1661.8, 1747.6; compare 1648.3, 1729.3, 1814.6, 1821.9, 1907.7 at r = 64..16384 (bytes 3569..917489). suanpan word update: 371.5, 440.5, 509.5, 509.5, 578.5. Dense control (same widths/depths, no ranges; S/wasm4-dense.log): flat 181.5 per update; decode ~1091/leaf, compare ~1231/leaf, flat. before's crate page (crates/before/src/lib.rs:342-358) calls every asymptotic claim a hard guarantee.
4. Zero-shift representation dependence (question, not yet a defect): redundant zero `[-2^32, 1]` shifted 2^25 bits keeps 1,048,578 stored digits; known zero keeps 1 (S/run3.log). `Shl` doc (operators.rs:361) says panics only if "the result needs an unrepresentable working width"; `ShlAssign` (343) says "a nonzero contribution cannot fit". 32-bit guest cases 7/8 staged (commit a959e13c), NOT YET RUN.
5. Calibration (S/calibrate.out, S/calibrate2.out): pool model catches M1,M2,M3,M4,M5,M7,M9; M6 caught at 2000 cases after near-threshold values; M8 trips production debug_assert (not a calibration); touch property catches M12; M13 is +1 touch per range (asymptotically equivalent, expected to survive).
6. 20,000-case pool model run passed (S/long1.log, test l7_pool_programs_match_the_oracle ok). The touch result in that log is bogus (shared counter).
7. Reading-level conclusions: suanpan core arithmetic, recentering, sign scan, zero ranges, normalize, reads, conversions look correct; before's lattice switch reads, range_minima, place filter (documented k*p term), integral freeze/defer, min_ticks freeze, tick emission reads, prescan all price their width-linear calls by input or output. Rank::sum_iter comment cites a stale panic threshold ("shift / 32 > usize::MAX", rank.rs ~1031) vs suanpan's current landing-position wording: observation.

8. Touch-work bug fixed (c100e028: owned subtraction priced by right operand); 20,000-case serial run passes (S/long3.log, two "test result: ok" lines: the killed first job survived and also finished; both at c100e028).
9. 32-bit zero shift (S/wasm5-zeroshift.log): redundant zero traps, known zero passes. Q1 record: S/deliverables/Q1-zero-shift-representation.md. F1 record: S/deliverables/F1-cost-composition-log-factor.md. D1 record updated with fix verification and test brief.
10. Existing suanpan suite vs mutants (S/calibrate_existing.out): catches M1-M7, M9, M12, M16; MISSES M14 and M15 (range-split upper remnant dropped, One/Many branch). After adding GapRound pattern (c30c045f), the touch property catches M14 (216 touches vs work 5, one GapRound), M15 (718 vs 6), M16 (S/calibrate_touch_split2.out). So the touch property has a unique catch; the pool model has none among M1-M9 (existing suites catch them all).
11. Lesson: the box's `agent` account is shared with other auditors (L8 runs mutation campaigns on suanpan); never pattern-kill. CARGO_TARGET_DIR overrides must live under target/ (rsync --delete spares only /target/).

12. Adversarial touch search (S/adversary2.log, S/adversary3.log): best ratio 9.6 from `+= i128::MAX` priced at 1 unit; width-flat under x2..x8 shift scaling. Repeated-comparison probe: `2 * 2^2048` costs 6 touches per cmp_zero (O1).
13. Round 1 report delivered. Deliverables in S/deliverables/: D1, D2, F1, Q1, MB1, S1, observations.md, coverage.md. Explore branch head 880aa203 (all signed).

## Round 2 (in progress)

Coordinator items: (1) F1 evidence: measure remaining ops, prototype tag design (T), adversarial families, F1b record; owner ruled: goal is a suanpan fix keeping before linear unconditionally without weakening any guarantee; compare designs. (2) O1 constant-factor brief. (3) Q1 brief now unconditional (DONE: deliverables/Q1-fix-and-test-brief.md, flagged in STATUS). Then usize-width sweep (new contract clause), min_ticks family, adequacy-lane zero_ranges survivors, inventory of every explore instrument (auditor.md "Your worktree").

ALWAYS begin remote commands with `unset CARGO_TARGET_DIR; export NEXTEST_TEST_THREADS=24;`. NEVER use `--all-features` for suanpan on this branch (it enables the l7-proto, l7-proto-w, l7-o1 explore features); use `--features touch-meter` (+ the prototype feature wanted). The first Z-mutant run was void for this reason (logs moved to S/mut/void).

Prototypes on explore branch (features of suanpan): l7-proto = T (tags, commit 94f56560); l7-proto-w = W (written bitset, crates/suanpan/src/accumulator/digits/written.rs, 31571595 + ensure fast path c045c8ba = "W2"); l7-o1 = O1 (skip identity collapse). All validated with full suanpan suite + pool model (S/proto2.log, S/protoW1.log, S/o1.log). O1 also passes before's touch instruments (84 tests, S/o1_before.log); board acceptance with O1 still to run (must not sync while a mutation calibration is in flight).

Fuel matrix: families/ops in crates/before/wasm32-pins/guest/src/l7_builders.rs + checks.rs l7_fuel; harness test zz_l7_fuel_matrix; run the prebuilt binary target/wasm32-pins/release/deps/pins-c2e1fad8c0bd62a4 directly (cargo test holds the build lock), logs on box under ~/src/rumors-audit-l7-suanpan/target/l7m/<build>-f<family>.log; guests: target/wasm32-pins (current), -proto (T), -w (W v1, slow ensure), -w2 (W2). Sizes via S/build_r2.log L7SIZE lines. Table script S/m3/table.py (copies logs to S/m3). Families 2-5 done for current/T/W2: current grows (log), T grows less, W2 flat (dense +10-17%). F0/F1 (1M-leaf runs) were still running for current/proto/w/w2 at last check.

Draft records: S/deliverables/F1b-design-evidence.md (needs FILLED-IN-TABLES), S/deliverables/S2-comparison-fixed-point.md (needs BOARD-RESULT).

usize sweep finding: cmp_zero_stable_under early-returns None without compaction on 32-bit when bits >= 32*2^32 (usize::try_from of adjustment_high); probes added (native l7_probe_stability_huge_width_compacts, guest case 9 / zz_l7_suanpan_stability_width_compacts), NOT YET RUN (harness rebuild must wait until no matrix job uses the pins binary).

Adequacy survivors (zero_ranges.rs): classified by reading (equivalent: Debug, last(), insert guard, compact no-op/never-collapse; unreachable: take_below boundaries Z1/Z2; cost breaches: compact guards Z3/Z4/Z5). Calibration rerun in flight: S/calibrate_zn.out (Z1-Z5, N1-N3 normalize survivors) with the touch property + pool model.

Round 2 progress since: Z/N calibration valid run S/calibrate_zn.out: touch property catches Z3,Z4,Z5 (survivors of committed suite); Z1,Z2 survive (unreachable take_below boundaries); pool model catches N1 (normalize.rs:74, committed-suite survivor), N2,N3 survive. O1 board: 5311 green/0 red, worst-case only baseline drift (target/l7m/board-o1.log, worst-o1.log on box). U1-a verified both widths (S/batch3.log). Touch totals identical A/T/W on 1000 seeded programs; O1 never more (S/m3/touchcmp-*). min_ticks op 11 measured (box target/l7m/mt-<c3|p3|w3>-f<fam>.log, harness binary target/wasm32-pins-h2/...). Deliverables written: Q1-fix-and-test-brief, S2-comparison-fixed-point, U1-usize-width-sweep, inventory, F1b (tables pending: run S/m3/mdtable.py after copying all logs into S/m3, including mt-* renamed so the build label maps: mt-c3 -> current, mt-p3 -> proto, mt-w3 -> w2).

Round 2 report delivered (F1b tables filled; S2 board result recorded; inventory written).

## Next steps if resumed

- Merge any fix branches the coordinator names into explore/l7-suanpan and rerun the l7_ suites past them.
- Measure F1 on join/meet/tick if asked (extend guest modes in crates/before/wasm32-pins/guest/src/checks.rs `l7_fuel`).
- Construct a family that seeds min_ticks' `answer` with zero ranges.

## Background jobs

None running (checked `ps -u agent` on the box for this worktree's path).

## Round 3 (in progress, started 2026-10-07)

Request: write BRIEFS (no code) into /Users/oxide/src/rumors/.agent-notes/2026-10-06-before-audit/lanes/l7-suanpan/round-3/ plus report.md.
Owner ruling on F1b: adopt W. Order: (1) machinery branch first: deterministic counter or committed fuel pin growing on F1 families today; (2) W branch flattening it, constant tuned on range-free inputs (e.g. lowest level of marks in digits' spare bits), with model-based test of the bitset vs BTreeSet oracle.
Deliverables: item-1 machinery brief (builder), item-2 fix brief (fixer, stacked on 1), leads 1-4 evaluations/briefs, report.md.
Landing check is ~/bin/audit-reserved ~/bin/audit-check (never pset-run).
Lead analysis so far (from reading, unverified unless noted):
- Lead 2 (read.rs high part): reachable high parts {1,2} in each sign; 3 impossible; high-2 touches not pinned. Brief: add high-part-2 cases each sign to nonzero_high_parts_cost_one_touch_in_each_sign.
- Lead 3 (deposit_value huge shift on wasm32): positions u128, unpinned on wasm32; brief a guest case: small operand 1<<64 via add_shifted(32*(2^32-2)) and via <<=, expect trap.
- Lead 4 (U1-a): demonstrator = guest case 9 (fails Failed(WrongLength) at base) + native witness; fix keeps threshold in u64.
- Lead 1 (anchor compare_offset_to / tick memo REL_FOLLOWER): in progress; scratch S/r3/.
Next: finish lead 1; merge main into explore; measure item-1 base fuel readings at main; write briefs.

### Round 3 progress (2026-10-07)
- Merged main into explore (041d868a; conflict in wasm32-pins guest checks.rs resolved: main's case 4 kept, my cases 5-9 kept). Explore probes committed 55de61b8:
  - fuzzfit harness tests/explore_l7.rs: L7FUEL ladder (families 0-5, ops decode/le/min_ticks, sizes 64..4096; env L7_FAMILIES/L7_SIZES/L7_OPS). NOTE: its `le` op is `Version::new() <= v`, which has an O(1) answer in normal form (root base); item-1 brief must use a pair differing only at the last leaf instead.
  - before: testing/l7_compare.rs counters + anchor.rs compare_offset_to hook (cfg(all(test, feature="touch-meter"))); board/l7_compare_probe.rs test l7_compare_offset_accounting (all board cross families x 4 levels + left-chain-all/top constructed chains). Run alone (process-global counters).
  - suanpan feature l7-high-log: read.rs logs extreme carries (2, -3, negative with zero low part) to $L7_HIGH_LOG with thread name.
  - wasm32-pins guest cases 10 (add_shifted small operand at shift 32<<32) and 11 (stored operand 2^2048 at shift 32<<32); harness zz_l7_suanpan_add_shifted_landings.
- mutate.py new swaps: WDEF (suanpan default = l7-proto-w, for W guest builds), K1 (double Many-mode lookup), L3S/L3D/L3W (32-bit shift narrowing/wrap in add_shifted routes), U1Fa/b/c (U1-a fix). All dry-run clean.
- Plan: build guests A (target/fuzzfit), W (target/fuzzfit-w via WDEF), K1 (target/fuzzfit-k1); run harness explore_l7 binary with FUZZFIT_GUEST_WASM each; then lead runs.
- Written: lanes/l7-suanpan/round-3/U1a-stability-width-test-brief.md (fix verification PENDING).
- Box load ~336 at 17:45.
- Ladder v2 (explore 8e01405b; harness explore_l7 binary target/fuzzfit/release/deps/explore_l7-49d2f7cfb17c0969; guests target/fuzzfit{,-w,-k1}): S/r3/ladder2-all.log, table S/r3/ladder2-table.md (A exact fuel; W, K1 per byte). A sparse rows grow (F1 decode +12%, le +11%; ±S +46/46/60%; OS +17/44/56%); controls flat; K1 +6.6..29.9% sparse, controls identical; W sparse flat, controls +7.2..20.5% (F1 min_ticks -6.5%).
  Lessons: ff_version_min_ticks renders decimal (superlinear) -> use ff_count_from_version; `Version::new() <= v` is O(1)-answerable -> use operand with root base 0 + lowered end; OS control must densify D too.
- Lead 2 verified: S/r3/leads1.log STEP1 (l7_probe_extreme_high_parts passes; touches d+1/2d+1 as predicted) and STEP2 census (suanpan committed suite: c=2 and c=-3 reached by surface + stored_width_stability; c=-1 zero-low reached; c=-2 zero-low reached by NONE).
- Written: round-3/F1c (item 1), F1d (item 2), MB2 (lead 3, calibration PENDING), MB3 (lead 2), U1a (lead 4, fix verification PENDING).
- In flight: S/r3/leads2.log (lead-1 sweep l7_compare_offset_accounting; wasm32 cases 10/11 + case 9 at base). Next: mutant loop L3S/L3D/L3W and U1F on wasm32 (guest rebuild each; S/mutate.py), then lead-1 write-up, report.md.

### Round 3 complete (2026-10-07)
Deliverables in /Users/oxide/src/rumors/.agent-notes/2026-10-06-before-audit/lanes/l7-suanpan/round-3/: F1c (item 1), F1d (item 2), L1 (lead 1, no defect), MB3 (lead 2), MB2 (lead 3), U1a (lead 4; fix verified S/r3/u1f.log), inventory-additions.md. report.md was refused by the harness (subagent report files blocked): the report text went into the handback message.
Key evidence logs: S/r3/ladder2-all.log + ladder2-table.md (explore b9f688a3); S/r3/leads1.log (census, high-part probe; b9f688a3); S/r3/leads2.log (lead-1 accounting, wasm32 base; 5358b7f8); S/r3/mut-*.log, mut2-*.log, mut-base.log (lead-3 calibration); S/r3/u1f.log (U1-a fix).
Corrections made this round: case 2 of main's landing pin covers the stored-operand route (SMALL_SHIFT_MAX = 30), not the small-value route; `Version::new() <= v` is O(1)-answerable; ff_version_min_ticks renders decimal.
Wrapper: runs command under set -e; use `cmd || echo "status $?"` (common.md's `cmd; echo` form is wrong; reported).
No background jobs running. Explore head 5d5e3347, all signed.
