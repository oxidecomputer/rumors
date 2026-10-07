<!-- CAVEAT LECTOR: written by Claude (Opus 5.5) from a run on ox-east-1 on 2026-10-06; the stream logs are in the coordinator's session scratchpad under coordinator/baseline-455e97de-gate-logs. -->

# Baseline on ox-east-1 at `455e97de`

The baseline below was recorded at `455e97de`. It still holds at `main`: the
fixes landed on `main` since then change no leg's verdict and no test count.
Each landed fix ran the full gate on reserved cores, and each matched this
record.

Every later "this fails" claim in the audit is judged against this record. A
failure that also occurs here is pre-existing, and a branch must reproduce it
exactly, with nothing new beside it.

The run used the toolchain channel pinned in `rust-toolchain.toml` (1.97.1) and
the justfile's pinned nightly (`nightly-2026-06-30`), both installed on the box.
It ran on a working tree identical to `455e97de`'s; the commit was made from
that tree after the run.

## How the gate is invoked on the box

```
on-illumos.sh <worktree> 'unset CARGO_TARGET_DIR; just gate'
```

The remote wrapper exports `CARGO_TARGET_DIR=~/build/<worktree>`, but the
justfile's `docs` recipe reads `target/doc`. Under the wrapper's default, the
workspace leg fails at `docs` (`fuelscape-assets: no such file or directory:
target/doc`) for a reason unrelated to the tree. Unsetting the variable builds
in the synced tree's own `target/`, which the wrapper's rsync excludes and so
never deletes. illumos's `env` has no `-u` flag, so `unset` is the portable
spelling.

## `just gate`

| Leg | Verdict | Detail |
|---|---|---|
| audit | ok | |
| internal-docs | ok | |
| docsrs | ok | |
| doctest | ok | |
| surface | ok | |
| workspace | ok | `test-all`: 1863 tests run, 1863 passed, 2 skipped |
| wasm | ok | fuzzfit 25/25, fuelscape 43/43, wasm32-pins 8/8 |
| fuzz | failed | `libfuzzer-sys` 0.4.13 does not compile on illumos (`FuzzerPlatform.h:72: #error "Support for your platform has not been implemented"`). The crate depends on it unconditionally, so the leg stops at its clippy step. The fuzz workspace is out of the audit's scope. |
| board | failed | `amp-board-acceptance` passes: 5311 green, 0 red. `worst-cases-pin` reports drift for `count_display × heap`: pinned worst `hugeleaf`; live worst `pure-comb` at the default scale and `memo-comb` at the acceptance scale. |

## Landing check

Branches land on the landing check (`~/bin/audit-check`, described in the
README) rather than the full gate. Its baseline was recorded at `9a90b767`,
whose code is `455e97de`'s plus the landed limb-index and board-pin fixes,
with the corrected script, which judges every command in a leg and refuses
to run if it cannot see a failing first command. The invocation:

```
on-illumos.sh <worktree> 'unset CARGO_TARGET_DIR; export NEXTEST_TEST_THREADS=24; ~/bin/audit-reserved ~/bin/audit-check'
```

| Leg | Verdict | Detail |
|---|---|---|
| tests | ok | `before` and `suanpan`: 759 run, 759 passed, 1 skipped; `rumors` snapshot filter: 142 run, 142 passed |
| lints | ok | `just gate-lints` and both clippy runs clean |
| docs | ok | private-items and docs.rs builds clean; doctests: 193 passed (`before`), 3 passed (`suanpan`) |
| surface | ok | 14 tests passed; 211 public function-like items = 134 board-covered + 77 excepted |
| board | ok | 5311 green, 0 red; `worst-case pin: clean (270 pinned rows verified at 2 sampling scales)` |
| wasm | ok | wasm32-pins 8/8, fuzzfit 25/25, fuelscape 43/43 |

A branch's landing check must match this table, with counts moved only by
the tests the branch adds or removes. A branch based before the board-pin
fix (`ddfabe4c`) instead shows the board leg failing with exactly the two
`count_display × heap` drift lines described under `just gate`, and nothing
else beside them; its `before` and `suanpan` count is two lower.

## Open items

- The justfile's `docs` recipe assumes the build directory is `target/`, which
  any exported `CARGO_TARGET_DIR` breaks. The owner ruled to record this as a
  finding and change nothing during the audit.
