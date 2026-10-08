# builder-wasm32-compare-pin-reach: resumption notes

Task: brief 42 in `.agent-notes/2026-10-06-before-audit/coordinator-briefs/survey-proposals.md`;
survey 1.2; calibration `lanes/l8-adequacy/round-1/findings/wasm32-pins-calibration.md`.
Worktree `/Users/oxide/src/rumors-slot-42`, branch `proposal/wasm32-compare-pin-reach`,
base `8b28bbd81` (verified clean, board-pin fix ancestor).

## Established by reading (base tree)

- Site 1 `bits/reader.rs` `from_storage`: `(position / 8) as usize`. Reached only
  through `BitsReader::at(bits, position)` with position != 0: PayloadRange::decode_payload,
  VersionTreeReader::at/seek (tick, prescan, raise, splice), writer.rs copies.
- `Version::partial_cmp` -> order.rs `compare` -> `VersionRegionReader::open` ->
  `VersionTreeReader::new` -> `Bits::reader` -> `BitsReader::at(self, 0)`; forward reads only.
  So no compare input reaches site 1 with a large position.
- Site 2 `bits/reader/gamma/window.rs`: module is `#[cfg(any(test, feature = "borsh"))]`;
  guest builds `before` with `meter` only. Not compiled into the guest. Callers: borsh
  streaming decode, test-only reference reader.
- => path one (grow VersionCompare) cannot catch either narrowing. Take path two (doc
  correction + index note), after measuring.
- What the compare pin does guard (to verify): VersionRegionReader's `len` (regions.rs:102
  `len: version.stored_len(),`): narrowed to usize wraps 2^32 -> 0, walk misses its end.
- #31 (`cc9e0ecd`) adds a wasm32-pins paragraph to validation_index.rs at the same spot:
  textual conflict expected; flag in report.

## Runs planned (focused, compare pin only)

1. narrowing #1 at site 1 -> expect pass.
2. narrowing #2 at site 2 + compile_error! in window.rs -> guest builds (window absent), pin passes.
3. grown probe (A = node(leaf(H), leaf(H+1)) vs B = leaf(H), H = 2^(2^31) - 1, right leaf
   flag at bit 2^32 + 3) + trap assert!(position < 1<<32) at site 1 + narrowing #1 -> expect pass.
4. len narrowing at regions.rs -> expect compare pin fails at FIRST_WIDE (doc claim evidence).
Then doc edit, commit, landing check.

## Results

(none yet)
- run1 (narrowing #1, old pin): PASS [38.113s]; log run1-narrow-reader.log.
- run3 (grown probe + trap position<2^32 at site 1 + narrowing #1): probe PASS [36.119s],
  old pin PASS [39.852s]; log run3-grown-probe.log. Probe reverted.
- run2 (narrowing #2 + compile_error! in window.rs): guest built, pin PASS [48.450s];
  `cargo check -p before --features borsh` fails with the compile_error (control). Reverted.
- run4 (regions.rs len narrowed `as usize as u64`): FAIL at (536870913, 0),
  Trapped(UnreachableCodeReached). Reverted. Tree clean.
- Decision: path two (doc correction + index note). Next: edit pins.rs doc + validation_index, fmt, commit, landing check.
- commit 5180c60ab (signed G). Landing check running in background -> landing.log
- landing check clean (873 s), all counts match baseline; compare pin 52.558s in landing wasm leg. DONE; report sent.
- amended per review -> da54e600b (G); fmt/doclint/both rustdoc builds ok (amend-docs.log)
