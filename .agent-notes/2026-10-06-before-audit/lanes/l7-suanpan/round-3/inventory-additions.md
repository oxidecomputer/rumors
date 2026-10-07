<!-- CAVEAT LECTOR: written by Claude (Opus 5.5), auditor-l7, round 3. -->

# Inventory additions: `explore/l7-suanpan`, round 3 (head `5d5e3347`)

These entries extend `round-2/inventory.md`. The branch now also merges main at `041d868a`, which brings in the limb-index fix and its pin.

**1. Zero-range fuel ladder, `crates/before/fuzzfit/harness/tests/explore_l7.rs`** (`55de61b8`, `8e01405b`, `b9f688a3`). Briefed as `F1c-machinery-zero-range-fuel-ladder.md`.

- Checks: Wasmtime fuel for decode, `<=`, and `min_ticks` on six host-built families (F1, ±S, OS, each with a range-free control) at `r` = 16 to 4,096. It prints readings, and checks values against the native build: decode re-encodes, both `<=` answers hold, and `min_ticks` renders equal at `r <= 64`.
- Reach: families with thousands of recorded zero ranges, which no committed generator or board family builds, in the one currency that sees map work.
- Calibration: the known-bad K1 raises every sparse decode and `<=` cell, and the ±S and OS `min_ticks` cells, by 6.6% to 29.9%, and leaves every control cell identical. The W prototype flattens every sparse row.
- Env: `L7_FAMILIES`, `L7_SIZES`, `L7_OPS`.

**2. Builders, `crates/before/wasm32-pins/guest/src/l7_builders.rs`** (`8e01405b`, `b9f688a3`). These are shared by item 1, the wasm32 fuel probes, and `crates/before/tests/explore_l7.rs`.

- New: `l7_family_version_lowered`, the comparison operand (root base 0, last bottom node replaced by a zero leaf).
- Changed: family 5, the OS control, now densifies `O` as well as `S`.
- Round 2's OS-control readings predate this change.

**3. Offset-comparison accounting** (`55de61b8`, `5358b7f8`). Not briefed: the board's tick cells already enforce the composed cost (`L1-offset-comparison-composition.md`).

- Parts: `crates/before/src/testing/l7_compare.rs` (counters), a hook in `Anchor::compare_offset_to` under `cfg(all(test, feature = "touch-meter"))`, and the probe `l7_compare_offset_accounting` in `crates/before/src/testing/meter/board/l7_compare_probe.rs`.
- Checks: nothing is asserted. Per family and size, it prints calls, touches inside the function, operand and gap widths, and tick's total.
- Reach: every board cross family at levels 0 through 3, plus two constructed chains, which make zero calls.
- Calibration: none run. It is a white-box reading, not a pass or fail check.

**4. Readout carry census, suanpan feature `l7-high-log`** (`55de61b8`). The evidence behind `MB3-readout-high-part-classes.md`.

- Checks: it appends each readout whose final carry is 2 or -3, or negative over a zero low part, to `$L7_HIGH_LOG`, tagged with the test name.
- Reach: it measured suanpan's committed suite.
- Caveat: concurrent test processes interleave some lines, so the counts are reliable only per class, not per line.

**5. Extreme high-part probe, `l7_probe_extreme_high_parts`**, in `crates/suanpan/src/accumulator/tests/explore_l7.rs`, `metered` module (`b9f688a3`). Briefed as MB3.

- Checks: five constructions covering the readout classes no touch pin covers, with exact values and touches in both readouts, at 64 and 128 digits.
- Calibration: none run by me. MB3 gives the builder two swaps.

**6. Huge-shift landing cases**: guest cases 10, 11, and 12 in `suanpan_landing`, and the harness test `zz_l7_suanpan_add_shifted_landings` (`55de61b8`, `5d5e3347`). Briefed as MB2 (cases 10 and 12).

- Calibration:
  - L3S fails case 10 only.
  - L3H fails case 12 only.
  - L3D and L3W fail case 11 and main's case 2.
- Case 11 duplicates case 2's route.

**7. Superseded.** `zz_l7_suanpan_limb_index_past_usize_max`, with guest cases 5 and 6, was the round-1 demonstration of the limb-index wrap: it asserts the trap, failed on the old base, and should pass since main's fix (merged at `041d868a`; inferred: case 4 traps in this round's runs, but I did not rerun this test). It now duplicates main's committed case 4. Retire it with the branch.

Scratch tools, not on the branch: `S/mutate.py` gains the swaps WDEF (suanpan default feature `l7-proto-w`, used to build a W guest without a direct dependency), K1, L3S, L3D, L3W, L3H, and U1Fa through U1Fc. Each reverts cleanly (verified with `git diff --quiet`).
