<!-- CAVEAT LECTOR: written by Claude (Opus 5.5), reviewer review-board-fuel-2, round 2 of audit/board-fuel at 971e39a23. -->

## 1. Verdict

**Changes required, but only three small prose fixes.** Each has an exact repair below, so the coordinator can apply them under the "exact repair" ruling without a third round.

All five round-1 findings are resolved in substance. The mechanisms the fixes introduced hold up when tested:
- **The tree id fails safely.** When `git stash create` cannot take the index lock, it exits 1 with no output. The recipe then passes `AMP_BOARD_SOURCE_TREE=""`, and `source_tree()` refuses the empty name. The builder's `calibrate-B-2.log` records exactly that refusal.
- **The calibration round trip is exact.** The fuel table re-derives byte-exactly from the tip's capture for every recomputable cell.
- **No host paths leak into the guest.** It embeds no path under the home directory, and the tables are not linked into it.
- **The middle commit stands alone.** `d8be5c459` compiles with all features, tests, and examples.

The three blocking items are all rustdoc or test-doc accuracy, which the reviewer brief treats as blocking. Two of them were introduced by the fixes.

## 2. Blocking findings, most severe first

**F1. Two new test doc comments give the architecture rationale backwards.**
- `crates/before/src/testing/meter/board/cell_ceilings/tests.rs:119-121` (`heap_table_names_the_architectures_of_record`) and `:240-242` (`recalibration_refuses_architectures_off_the_record`) both say a table calibrated from fewer architectures "would hold for an architecture whose readings it never saw".
- The opposite is true. `recalibrate` writes `archs` from the architectures it actually measured, and `cell_ceilings(currency, arch)` matches on that list. So a table calibrated from fewer architectures *stops* holding for the omitted one, whose runs are then judged against the board-wide ceiling alone. That is round 1's B2 failure.
- Repair for `:119-121`: "The judge applies a table only on the architectures it names, so a heap table naming fewer than the board of record runs on would leave the others judged against the board-wide ceiling alone."
- Repair for `:240-242`: "A table calibrated from fewer would not hold on the omitted architecture, whose runs would face the board-wide ceiling alone; one calibrated from more would name an architecture the record does not."
- The same test has two more problems:
  - `:131` `assert_eq!(archs_of_record(Currency::Heap), HEAP_ARCHS)` is a tautology, because `archs_of_record` returns `&HEAP_ARCHS` by definition.
  - `:126`'s skip of empty tables is dead code, since no table is empty.
  - Its name and summary say "heap", but it loops over every table.
  - Suggestion: rename it to cover every table, and delete the skip and the final assert.
- The sibling rustdoc on `recalibrate` (`cell_ceilings.rs:266-269`, "so a table never holds for an architecture whose readings it never saw") is not false. Its first clause, though, describes something the code guarantees by construction, not something the exact-cover check adds. Non-blocking.

**F2. The calibration-to-judgment property claims more than its fixture can show.**
- At `cell_ceilings/tests.rs:316-319`, the doc says calibration and judgment "must agree on units, intercept, and rounding".
- The fixture declares no model, and its `denom_bytes == exp_denom_bytes`. So a derivation that normalized by trend units instead of constant units would still pass. Both sides do call one shared `constant_units`, so this is common-mode by construction rather than tested.
- Repair, either of:
  - drop "units," from the sentence, or
  - give the probe samples a `Model` whose `constant_units` differ from both denominators, which makes the claim true.

**F3. Round 1's B5 is only partly resolved: "the rendered board says so".**
- At `cell_ceilings.rs:59-60`, the doc still says that when heap falls back to its outer bound, "the rendered board says so".
- `render.rs` `ceiling_text` (around lines 85-96) then prints only `heap <= 1024 + 15 * units bytes`. It says so only by omitting the per-cell clause, which is round 1's exact observation. The fuel half of B5 is fixed.
- Repair, either of:
  - change the doc to "and the rendered preamble states that bound alone", or
  - make `ceiling_text` name the missing table for the measured architecture.

## 3. Prose findings against Part IV

Blocking prose: F1 through F3 above. The rest are non-blocking:
- **`justfile:861-863`.** "`git stash create` records them without touching the index" is inaccurate. It rewrites the index's stat cache under `index.lock`, and that write is what fails under contention. I'd say "without changing what is staged or the stash list", and add one clause on the lock failure and the resulting refusal.
- **`ceilings.rs:72-80` (`FUEL_INTERCEPT`).** The figures are hand-measured and their method is unstated. Over the 5,469 cells I could recompute from the tip's capture, I get median 1.12 and p90 3.10 at 1,000, and 1.26 and 32.5 at zero. The doc says 1.13 and 3.3, and 1.29 and 34. "About 870 instructions" I could not reproduce. Either state the method, or round to what survives recalibration.
- **`board-fuel/harness/src/lib.rs` module doc.** "[`provenance`] names everything a fuel reading depends on besides the code under test" overclaims. The `fuel` profile and the remap flags also move fuel, and provenance does not record them.
- **The `amp-board-fuel-build` comment and recipe (`justfile:935-944`).** Since the fold, cargo passes workspace members to rustc as relative paths. The current Mac guest embeds `crates/before/src/...` and no checkout path, so the `{{justfile_directory()}}=/rumors` remap is a no-op. It is harmless, but the comment presents it as needed.
- **Lines left unwrapped** by this round's edits:
  - `examples/amp_board.rs:46`
  - `board/shard.rs` module doc, around lines 29 and 38 (the first is round 1's line, still overlong)
  - `board/shard.rs:434`
  - `board.rs:123`
  - `ceilings.rs:80`
  - `justfile:930`
  - `Cargo.toml:251`, an orphaned short line, "dependencies included. A per-package"
- **Commit `d8be5c459`.** "A verdict names the currencies it requires: the native board heap, scan, and touch, the fuel board fuel." reads as a garbled list. Suggest "the native board requires heap, scan, and touch; the fuel board, fuel." Commits `971e39a23` and `2f6110706` are clean. No roster IDs or branch names remain.

## 4. Non-blocking suggestions

1. **Fuel recalibration is coupled to a two-host heap capture.**
   - `amp-board-calibrate` runs the native heap step first. With no capture, that step refuses: the run covers aarch64 alone, not both architectures of record. The fuel step never runs.
   - So the advice in `check_guest`'s refusal ("recalibrate with `just amp-board-calibrate`", harness `lib.rs:113`) leads straight into the heap refusal. A Wasmtime bump or a cost-table change, which moves only fuel, still needs an x86_64 capture from the box.
   - Options: a fuel-only recipe, or a message that names the two-host procedure.
2. **The restored model guard is untested and narrower in reach.**
   - `Ladder::assert_models_override` is correct per operation and currency, across the whole ladder.
   - It now runs only inside `accept_with`, so `board::run` (the debugging view and the smoke suite) no longer exercises it, where the old per-sample check ran everywhere.
   - No test shows it fires. A `#[should_panic]` test over a synthetic ladder carrying a no-op model would be a fixture, not a broken implementation.
3. **Untested harness logic.** `guest_rustc`, `check_paths`, and `render_costs` have no tests. The tool members also get no `-D warnings` rustdoc build, which is consistent with the other tool members but leaves the harness's intra-doc links unchecked.
4. **No time bound on the fuel leg.**
   - `FUEL_TANK = u64::MAX / 2` is effectively unbounded, and no nextest limit applies to `cargo run`, so a looping guest hangs the leg. The native board behaves the same way.
   - A finite tank would turn a hang into a trap. Any finite value would be a new constant to derive, which is why I list it only as an option.
5. **One duplicated writer remains.** The capture writer `write_atomically` is still duplicated between `examples/amp_board.rs` and `board-fuel/harness/src/main.rs`. Round 1's table-writer duplication is resolved by `write_recalibrations`.
6. **Concurrent gate streams can print a spurious error.** The gate's `board` and `board-fuel` streams each run `git stash create` concurrently even without `--capture`. A collision prints `error: could not write index` and is otherwise harmless.
7. **Outside this branch, inferred and not verified.** The fuzz-fit guest (`justfile:632`) builds without path remapping. Its registry and rust-src panic paths therefore differ in length between hosts, which by the builder's own finding shifts fuel. Fuzz-fit is outside the audit's scope; I'm routing this to you rather than investigating.
8. **Told, not verified.** The guest comment says the owner accepts the `unsafe extern` declaration. I found no ruling of record. The builder's notes cite a coordinator call.

## 5. What I checked and found sound

- **B1.** `Ladder::accept(required)` refuses an unmeasured required currency before rendering. The native runner requires `[Heap, Scan, Touch]` and the harness requires `[Fuel]`. A test covers both `accept` and `judge_with`.
- **B2.**
  - The committed heap table names `["aarch64", "x86_64"]`.
  - `recalibrate` refuses any other architecture set: the missing-capture case and the extra-architecture case are both tested.
  - It refuses captures from another tree or a debug build, missing rows, and other sizes; each refusal is tested against a capture that otherwise merges.
  - The procedure is documented as numbered steps.
- **B3.** Every requested test exists and passes:
  - the derivation rule and its clamp
  - the larger-reading merge
  - the refusals
  - the tighter-of-two check and its labels
  - the merge's refusal of fuel from other deals
  - `BuildStamp` round trips, including `target=wasm32-`
  - the green-then-red property
- **B4.** The rule is stated accurately in `cell_ceilings`, `board.rs`, `validation_index.rs`, and the heap commit's subject.
- **B5.**
  - The heap ceiling is an intra-doc link.
  - The `costs.rs` mechanism is now correct: a new operator changes the operator count that provenance records.
  - `multiplication_units` names the 32-digit and 256-digit thresholds, which are 128 bytes and 1 KiB on wasm32.
- **The `count_display` claim holds where it matters.** Its fuel model is sound at the ladder's sizes: the large families reach about 10 to 27 KB, past both thresholds. They read exponents of 0.85 to 1.02 against the bound of 1.15, while a quadratic cost would read about 1.26 on that axis.
- **Tree id.** Clean and dirty trees yield the correct tree, and a held lock leads to refusal, never a wrong name.
- **Capture format v2.** The header parses on `rsplit_once('\t')`, so the stamp and tree split cleanly. `compare` keys on builds and allows differing trees. No v1 references remain.
- **Guest rustc.** It is read from `producers`. The committed provenance carries `rustc 1.97.1 (8bab26f4f 2026-07-14)`.
- **`check_paths` and the remaps.** The guest holds only `/cargo/registry/src` and `/rustc/<hash>/library` absolute paths, and nothing from `/Volumes/forge`.
- **The tables cannot move fuel.** None of their strings appear in the guest, so `heap.rs` changing mid-recipe cannot shift a fuel reading.
- **The deleted provenance test.** It ran `check_provenance` (the harness's build-time rustc) only inside `amp-board-fuel`, just before acceptance. `check_guest` performs the same check, and a stronger one, since it reads the guest's own rustc. Nothing that file checked is now unchecked.
- **The rebase.**
  - `tool_members` lists both board-fuel crates, and `manifestlint` exits 0 at the tip.
  - `[profile.fuel]`'s comment covers both guests.
  - The absence of a nextest override is correct: the guest has `test = false`, the harness has no tests, and the leg runs under `cargo run`.
- **Work units.** Every work product is formed in `u64` before it is multiplied. `digits * digits` cannot overflow, because `digits` is below 2³².
- **Thread cap.** `available_parallelism().min(24)`.
- **Spawn ordering.** The argument holds: every unrun shard follows a failed one.

## 6. Evidence I ran

- **Local focused tests at the tip** (task prompt permits; `nice -n 10`, `CARGO_BUILD_JOBS=4`): `cargo nextest run -p before --all-features --locked -E 'test(/cell_ceilings::|shard::tests|ladder::tests|compare::tests|board::tests/)'` gave `42 tests run: 42 passed, 717 skipped`, exit 0.
- **Middle commit.** A scratch worktree at `d8be5c459`, running `cargo check -p before --all-features --tests --examples --locked`, exited 0. The worktree, its `target/`, and its resolved build directory `/Volumes/forge/build/f4/03002c5dd3d467` are all removed.
- **Lock probe** in a throwaway repo, git 2.50.1:
  - clean tree: `HEAD^{tree}`, exit 0
  - dirty tree: the stash commit's tree, exit 0
  - dirty tree with `index.lock` held: `error: could not write index`, status 1
  - recipe form under the lock: `AMP=` (empty), command exit 0
- **Fuel re-derivation.** Run against `builder-board-fuel/final-fuel-mac.capture` (header `tree=efa84118…`, equal to the tip's tree) and the committed `fuel.rs`: `cells 5729 exact 5469 differ 260`. The 260 are the model rows plus one flat-denominator cell per operation, the same set as round 1.
- **`./tools/manifestlint` at the tip:** exit 0.
- **Landing check.** I did not run one, since the task said one was running. `scratchpad/landing-59/check2.log` (20:15) reads `audit-check: clean in 492s`, with surface, board, lints, docs, wasm, and tests all ok. The log does not name its SHA. The landing check does not run the fuel leg. The builder reports the fuel leg at 5 red, the known cells, with identical captures on the Mac and the box.
- **The branch is untouched.** `git -C /Users/oxide/src/rumors-slot-59 status --short` is empty, and HEAD is `971e39a23`.

Files:
- `/Users/oxide/src/rumors-slot-59/crates/before/src/testing/meter/board/cell_ceilings/tests.rs`
- `/Users/oxide/src/rumors-slot-59/crates/before/src/testing/meter/board/cell_ceilings.rs`
- `/Users/oxide/src/rumors-slot-59/crates/before/src/testing/meter/board/render.rs`
- `/Users/oxide/src/rumors-slot-59/justfile`
- `/Users/oxide/src/rumors-slot-59/crates/before/board-fuel/harness/src/lib.rs`

My notes, diff, and scripts are in `/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/review-board-fuel-2/`.