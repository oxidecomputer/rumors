# auditor-l7 resumption record (lane L7: suanpan and its uses in before)

Read first after any compaction: briefs/common.md, briefs/auditor.md, briefs/lane-l7-suanpan.md (all under /Users/oxide/src/rumors/.agent-notes/2026-10-06-before-audit/), then this file. Verify claims below against the named logs/commits before relying on them.

Scratch dir (S): /private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/auditor-l7
Worktree: /Users/oxide/src/rumors-audit-l7-suanpan, branch explore/l7-suanpan (private), base 58285ca5.
Explore commits so far: 7d5fae48 (pool model + touch accounting), df000229 (threshold values, gap shifts, probes, adversary), d43fa15e (wasm32 wrap probe + fuel instrument), then "Add L7 touch replay probe", "Add L7 dense-base fuel control and encoded-size probe", a959e13c (zero-shift guest cases). All signed normally so far.

## How to run things

- Remote wrapper: /Users/oxide/.claude/skills/building-on-illumos/scripts/on-illumos.sh <worktree> 'unset CARGO_TARGET_DIR; ...' (syncs the local tree; never edit the tree while a mutation run is in flight).
- suanpan explore tests: `cargo nextest run --locked -p suanpan --all-features -E "test(/l7_/)"`; L7_CASES env sets proptest cases; L7_PRINT prints ratios.
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

## Next steps if resumed

- Merge any fix branches the coordinator names into explore/l7-suanpan and rerun the l7_ suites past them.
- Measure F1 on join/meet/tick if asked (extend guest modes in crates/before/wasm32-pins/guest/src/checks.rs `l7_fuel`).
- Construct a family that seeds min_ticks' `answer` with zero ranges.

## Background jobs

None running (checked `ps -u agent` on the box for this worktree's path).
