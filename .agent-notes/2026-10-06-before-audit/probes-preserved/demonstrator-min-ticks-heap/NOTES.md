# demonstrator-min-ticks-heap: resumption record

Task: focused failing test for D1 (min_ticks heap on jump-entered rising spine JR(d, 2^40))
in crates/before/tests/meter.rs. Worktree /Users/oxide/src/rumors-slot-16, branch
fix/before-min-ticks-heap, HEAD 5065caeb (verified at start).

## Established (by reading the tree at 5065caeb)

- CONFLICT in the task: "focused test in tests/meter.rs, no board family" vs "build with the
  meter registry's construction language". The construction language (ev_leaf, BitsWriter,
  testing::version::from_tree_stream) is crate-private; the only public entry is
  `registry::Shape::build*`. registry/tests.rs `every_shape_is_cited_by_a_family` requires every
  Shape to be cited by a FamilyId, and `every_family_reaches_board_operations` requires every
  FamilyId to be a board column. So a new Shape == a new board family.
- No public Version constructor from a tree or shape exists (oracle bridge is pub(crate)).
- tests/meter.rs runs in the dev profile (justfile test-all: nextest --all-features), with
  [profile.dev.package.before] opt-level = 2, debug assertions on.
- Board denominator for version_min_ticks: n = v.encode().len() (family.rs version()).
  Heap read: reset_peak, baseline = current, peak - baseline (measure.rs).
- Constants HEAP_INTERCEPT_BYTES, MAX_HEAP_BYTES_PER_INPUT_BYTE are pub at
  before::testing::meter::board.

## In progress

- Scratch dev-profile probe (uncommitted): crates/before/src/testing/meter/scratch_jr.rs plus a
  `#[cfg(test)] mod scratch_jr;` line in src/testing/meter.rs. MUST be removed after; verify with
  `git -C /Users/oxide/src/rumors-slot-16 status --short` empty.

## Next

- Report the conflict to the coordinator with the dev-profile readings and options.

## Background job

- Scratch build: local log build-scratch.log in this dir (on-illumos nextest --lib --no-run,
  dev profile, all features). Then run `-E 'test(scratch_jr_min_ticks_heap)' --no-capture`.

## Options to offer the coordinator

1. Board family (the brief's primary form): Shape + FamilyId; re-judges every version cell and may
   move worst-case rankings. Coordinator explicitly declined this for the demonstration.
2. Focused test inside the crate (lib test binary), using the private construction language
   directly. Needs a #[global_allocator] PeakAlloc in the lib test binary (none exists today), which
   would then wrap every lib test; or a heap reading via some other means. Not tests/meter.rs.
3. tests/meter.rs + a new public registry entry that is not a board family: requires relaxing
   `every_shape_is_cited_by_a_family` (instrument change, needs a ruling).
4. tests/meter.rs building the version by another public route: Version::decode of hand-written
   wire bytes (a second encoder in a test) -- not the construction language.

## Done

- Dev-profile scratch probe ran (run-scratch.log): JR(d, 2^40) over the ceiling at every size,
  readings identical to the auditor's release readings. Probe removed; worktree clean at 5065caeb.
- Stopped and reported the instruction conflict to the coordinator. Nothing committed.

## Round 2: coordinator chose option 1 (board family)

Stop rules: run board on base+family; if only version_min_ticks cells go red (and possibly
version_min_ticks's worst pin -> new family) commit; anything else red or any other ranking flip to
the new family -> stop, report verbatim, no re-pin. Verification: ~/bin/audit-check replaces
just gate; for this failing-test commit only board leg + registry tests + clippy/fmt.

Edits (uncommitted): jump_rising_spine(b, d) in testing/meter.rs; Shape::JumpRisingSpine (P2);
FamilyId::JumpRisingSpine "jump-rising-spine" appended to ALL (58) with VERSION_BUNDLE_CELLS;
board/family.rs constants JUMP_RISING_SPINE_BASE_DEPTH=8000, ENTRY_BITS=40; registry/tests.rs
ALL_SHAPES; meter/tests.rs proptest jump_rising_spine_decodes_canonically_as_a_rising_spine;
tests/verdict_matrix.rs arm (build2(40, 4)).

Jobs: focused.log, clippy.log (local scratch). Then board: run amp-board-acceptance and
worst-cases-pin as SEPARATE commands (just stops at first failing recipe).
- focused.log (run 1): clippy exit 0; my property failed at b=1,d=2 (16 vs 14 bits): gamma(2^b+2) is
  2b+1 bits only from b=2. Fixed: assert b >= 2, strategy b in 2..=300. Coordinator claimed run dead;
  verified it was alive (rustc pid 22583 at 10:45 CPU); did not duplicate.
- Running chain: focused-2.log -> board-acceptance.log -> worst-pin.log (poll files for exit=).
- focused-2.log: 74/74 pass. board-acceptance.log: 5401 green / 1 red (5402); only red is
  version_min_ticks x jump-rising-spine heap (124.9 / 125.0 / 177.2 B/B at base/top/small scales).
- worst-pin.log: 64 drift lines: 2 version_min_ticks heap flips (allowed), 48 touch flips on 24
  other ops (STOP condition), 12 tie-joins, 2 pre-existing count_display heap. STOPPED, not committed.
  Diff saved as jump-rising-spine-family.patch; worktree left dirty with the uncommitted family.

## Round 3: coordinator ruled option (a): re-pin and commit
- TEMPORARY scratch edit in board/worst.rs (margin precision, SCRATCH_ABS). MUST revert before commit:
  restore the `margin = if margin >= 100.0 {...} else {...},` block. Verify `git diff worst.rs` shows
  only the re-pins.
- Running: worst-cases-precise.log (just worst-cases). Then: re-pin WORST_RANKINGS, clippy, commit,
  worst-cases-pin alone, audit-check in background.
- Scratch renderer edit REVERTED (worst.rs matched HEAD before re-pin). Margins in worst-cases-precise.log
  (lines with SCRATCH_ABS). No exact ties. Near ties (<1%): all touch flips except rank-fold four
  (x1.28-1.32) and default-scale ranked_decode/serde/borsh (x1.0101).
- Re-pins applied in worst.rs (50 rows + VERSION_STREAM_FAMILIES + 4 inline tie lists), doc on WORST_RANKINGS.
- Running chain: clippy-2.log -> worst-pin-2.log -> focused-3.log. Then commit, then audit-check.
- clippy-2 exit 0; worst-pin-2: only the two baseline count_display lines; focused-3: 79/79.
- COMMITTED ba094bbe0e3932f5fe5d8dcbb468b86e3756d5e7 (signature G). Worktree clean.
- Running: audit-check.log (~/bin/audit-reserved ~/bin/audit-check). Poll for exit=. Then report.
- audit-check done (audit-check.log): board FAILED with only the version_min_ticks red cell (3 samples);
  all other legs ok; tests 758 (baseline 757 + my property). Pin drift not reached in-leg (just stops
  after acceptance fails); separate worst-pin-2.log shows only the 2 count_display lines. Task complete.
