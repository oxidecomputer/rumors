<!-- CAVEAT LECTOR: written by Claude (Opus 5.5), reviewer review-one-workspace, of build/one-workspace at 2fe38aa07. -->

# Review of `build/one-workspace` (round 1)

## 1. Verdict: changes required

The fold itself works:
- Production resolution is untouched.
- The guests compile with the same flags as at `main`.
- Both re-pins reproduce byte for byte.
- The supply-chain picture is unchanged.

But one change in behavior blocks: by deleting the detached workspaces, the branch silently drops the wasm32-pins and fuelscape nextest time limits. Four pieces of untouched prose now describe the old layout. And two claims are false: one in the commit message and its comments, one in a justfile comment.

**What I ran.** I did not run `audit-check`: your prompt says a landing check is already running at the tip, so I left `rumors-slot-62` alone. Everything below ran locally on this Mac under `nice -n 10` with `CARGO_BUILD_JOBS=4`, in two scratch worktrees: one at the tip `2fe38aa07` and one at `main` `00167a4d9`. Your prompt allowed local runs, which overrides `common.md`'s box-only rule. Both worktrees have been removed, after I resolved and deleted their forge build dirs (`0d/8833bca2dc125b`, `ec/330338d3d6152b`, `c2/2ce051b2fd1f0a`). Slot-62 is clean at `2fe38aa07`.

## 2. Blocking findings, most severe first

### B1. The wasm32-pins and fuelscape suites lose their nextest hang limits and fall back to the root's 300 s
Four files are now dead: `crates/before/wasm32-pins/.config/nextest.toml` (20 periods, 1200 s), `crates/before-fuelscape/.config/nextest.toml` (9 periods, 540 s), and the fuzzfit and surfacecheck copies. Nextest reads configuration only from the workspace root.

- **Demonstrated at the tip.** I appended invalid TOML to `crates/before/wasm32-pins/.config/nextest.toml` and ran `cargo nextest list -p wasm32-pins-protocol`. It exited 0 from both the repo root and the crate directory, so the file is never read.
- **Control at `main`.** The same corruption gives exit 96: `error: failed to parse nextest config at .../wasm32-pins/.config/nextest.toml`. Both files are restored.
- **Why it matters.** Commit `2f69f5a43` sized these limits from measured runs under load. wasm32-pins' `rank_arithmetic_straddles_the_usize_alignment_limit` took 577 s on its slowest passing run (median 103 s); fuelscape's slowest test took 247 s. On a loaded box the wasm leg will now kill a passing test.
- **Fix.** Add per-package `[[profile.default.overrides]]` with `slow-timeout` in the root `.config/nextest.toml`:
  - `package(wasm32-pins-harness)`: terminate after 20 periods
  - `package(before-fuelscape)`: terminate after 9 periods

  Then delete the four orphaned files and rewrite `.config/nextest.toml:18-25`, which still says each detached workspace has its own file.

### B2. Untouched prose that is now false about the layout
- `deny.toml:1-7` says the sweep covers "exactly the graph Cargo.lock records", and that fuzzfit, fuelscape, and surfacecheck are detached "with their own lockfiles". Neither is true now: the lock records the tool graphs, and the bans check excludes them.
- `rust-toolchain.toml:1-4` names "the fuzz, fuzzfit, fuelscape, and surfacecheck workspaces".
- `.config/nextest.toml:18-25` (see B1).

### B3. Plain `cargo` at the root changed scope, and no commit says so
`main` had no `default-members`, so plain `cargo build`, `test`, `clippy`, and `run` at the root selected `rumors` alone. Now they select all seven non-tool members (`Cargo.toml:26-34`). The fold commit's message reads as if `default-members` already existed.

`default-members = ["."]` keeps `main`'s behavior exactly and still leaves the tool members out. I checked that the wider scope breaks nothing: no two default members share an example or binary name, and `window_tradeoff` belongs only to `rumors`. So this is a ruling for you or the owner: keep the old scope with that one-line change, or keep the new scope and attribute it in the commit message.

### B4. "Workspace-wide builds never compile it" is false
This wording appears at `justfile:596` and `justfile:663`. `cargo <cmd> --workspace` without `tool_members` does compile Wasmtime: my control `cargo tree --workspace` at the tip includes `wasmtime`. The member manifests' wording ("ordinary builds", meaning the default members) is accurate; these two comments should say the same.

The same gap reaches editors, which I infer from rust-analyzer's documented defaults rather than tested: rust-analyzer runs both its check and its build-script pass with `--workspace`. Opening this repo in Zed or VS Code will now check Wasmtime and Cranelift and run their build scripts. The repo commits no editor configuration. Whether to add one (`rust-analyzer.toml`, or `.zed/settings.json` overriding the check command) is the owner's call. As things stand, the answer to question 1 is "no" for the editor path.

## 3. Prose against Part IV (none of these touch rustdoc of record)
- **`justfile:82-85` (should fix).** The claim that the streams "reach different target directories, never wait on one another's build lock" is false on this Mac. `~/.cargo/config.toml` sets `build.build-dir = /Volumes/forge/build/{workspace-path-hash}`, and cargo takes `.cargo-build-lock` per profile inside that directory.
  - Demonstrated: a `cargo check --target-dir target/lockB` started during a `cargo build --target-dir target/lockA` printed `Blocking waiting for file lock on build directory` and waited 13.3 s.
  - The doc, doctest, docs.rs, and surface-json legs already shared the root build dir at `main`. The fold adds the tool legs' debug-profile clippy and nextest runs (fuelscape-test, surface-totality, fuzzfit and wasm32-pins clippy) to the workspace stream's lock.
  - On the box, which sets no `build-dir`, the claim holds.
  - Fix: state the premise ("where the build dir is the target dir"), or pass `--config build.build-dir=<target-dir>/build` in the tool recipes.
- **`Cargo.toml:249-255` (should fix).** On the pinned nightly, `rustc --print target-spec-json --target wasm32-unknown-unknown` reports `"panic-strategy": "abort"`. So `panic = "abort"` changes nothing in the measured wasm, and "a per-package override could not set `panic`" is not the real reason for a separate profile. The real reason is that `codegen-units = 1` must reach every crate in the guest (both `before` and `suanpan` show `-C codegen-units=1`), and a per-package override cannot do that without also changing every release build.
  - Separately, "The fuel guests'" (plural) does not fit this branch: only the fuzz-fit guest uses the profile.
- **`justfile:358` (minor).** "(measured: 84 crates vs 459)" does not reproduce. cargo-deny reports 177 crates with no package selection and 184 with `--workspace`, at both `main` and the tip, so it was already stale. The branch rewrote this paragraph and carried the number forward; re-measure it or cut it.
- **`justfile:350` (minor).** "shipped members" also covers `rumors-testkit`, which ships nothing. "Default members" is the exact term.
- **`crates/before/fuzzfit/harness/src/bands.rs:31-32` (minor).** The doc paragraph was not re-wrapped; line 32 is about 120 columns.
- **`Cargo.toml:79` (minor).** `fuzzfit-harness` sits between `flate2` and `dsi-bitstream`. The table was already unsorted at `main`, but this entry is out of place even locally.
- **Hand-maintained enumerations (minor).** The list of tool members appears in prose in `Cargo.toml:18-24`, `justfile:67-70`, `justfile:406-408`, and `crates/before/AGENTS.md:17-18`, and twice in code (`members` minus `default-members`, and `tool_members`).

## 4. Non-blocking suggestions
1. **Make `default-members` the single list.** Derive `tool_members` in the justfile from `cargo metadata --no-deps` (the workspace members minus the default members). Or add a gate check that the two partition `members`; today they do exactly (7 + 7 = 14), but nothing enforces it.
2. **Give wasm32-pins its own profile.** `[profile.wasm32-pins]` would inherit `release` with an explicit `overflow-checks = false` and carry the deleted manifest's reason: compiler overflow traps would hide the wrong-value failures the pins exist to catch. Today the guest depends on cargo's default. Its flags are identical at base and tip, but a future `overflow-checks = true` in `[profile.release]` would silently change what the pins test.
3. **Correct the advisory fix version.** Fixing all 13 advisories needs Wasmtime 48.0.7 or later, not 48.0.6 as the builder reports: RUSTSEC-2026-0378 and -0379 (published 2026-10-19) list `>=48.0.7`. All 13 postdate the 2026-10-06 baseline, so `baseline.md`'s "audit ok" is stale for `main` too.
4. **A pre-existing calibrator quirk** (outside this branch's scope). `fuzzfit-calibrate` writes a trailing blank line at both `main` and the tip, which `fmt --check` rejects, so every re-pin needs a hand trim.

## 5. Evidence (what I checked and found sound)
- **Q1, Wasmtime avoidance.** Every `--workspace` invocation in the justfile carries `tool_members`. All other cargo calls name packages with `-p` or run in the fuzz workspace, and nothing outside the justfile uses `--workspace`. With `--target all -e normal,build,dev` and with and without `--all-features`, `cargo tree` over both the default members and `--workspace $TM` found 0 instances of wasmtime, cranelift, rand 0.8, or any tool crate. The full-workspace control has wasmtime.
- **Q2, lint and test coverage.** The tool recipes' clippy and nextest selections match the old detached runs package for package. `cargo fmt --all --check -v` reaches every tool crate, so the per-recipe `fmt` lines' removal loses nothing. Nothing is linted or tested twice. `manifestlint` passes and now covers the tool manifests. The build-lock question is in section 3.
- **Q3, guests.** I diffed the `cargo build -v` rustc flags for both guests at base and tip, ignoring metadata hashes: all 24 crates (fuzz-fit guest) and 27 (pins guest at `main`; 26 at the tip, after `syn` 3 drops) have identical features, `opt-level`, `codegen-units`, `panic`, `strip`, `debug-assertions`, and target. There is no `[profile.release]` in the root, so the branch leaves `release` unchanged. `fuel` is used at every producer and consumer of the fuzz-fit guest: `fuzzfit_guest_wasm`, the calibrate and fuelscape recipes, and the `wasm.rs` fallbacks. Only `bytes` and `thiserror` among the moved crates compile into the guest.
- **Q4, lockfile.**
  - Every (name, version) in `main`'s root lock is still present with the same source and checksum.
  - The resolved feature tree of the non-tool workspace is byte-identical between base and tip: 1190 lines, and 1192 with `--all-features`.
  - On the tool side, every move is down to the root's version (for example `cc` 1.4.6 to 1.2.62, `shlex` 2.0.1 to 1.3.0, `tempfile` 3.27.0 to 3.23.0); none moves up, and no package is new to every lock.
  - Holding Wasmtime at 48.0.5 keeps this attribution clean. The tree states it where a maintainer looks: `PINNED_RUSTC`'s doc calls a Wasmtime bump in `Cargo.lock` a re-pin event.
- **Q5, re-pins.** The commit messages' figures match the diffs: commit 1 moves 39 of 45 main bands and 5 of 5 small bands, with `ff_clock_version` at (-0.081, +0.302); commit 3 moves 23 of 45 and 4 of 5, at most 0.0044. Fresh calibrations reproduce `d018f452f` (at `main`, with the old lockfile) and `2fe38aa07` (at the tip) byte for byte, apart from the trailing blank line, from a different checkout path. The printed evidence also matches: excess +0.000, floor gap 0.169, refit divergence 0.338.
- **Q7, supply chain.** `cargo audit --no-fetch` against the advisory database as updated on 2026-10-20 finds the same 13 Wasmtime 48.0.5 advisories at `main` (in the fuzzfit, fuelscape, and wasm32-pins locks) and at the tip (in the root lock): no new advisory and none resolved. The root lock, which was clean at `main`, now carries them. `cargo deny --workspace $TM check bans` passes at the tip and at `main`; the control without exclusions fails with 9 duplicates, so the exclusion is doing real work.

Scratch logs and scripts are in `/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/review-one-workspace/`; the resumption record is `NOTES.md` there.