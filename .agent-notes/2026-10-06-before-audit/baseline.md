<!-- CAVEAT LECTOR: written by Claude (Opus 5.5) from the final gate on the Mac at `main` 3aa015900, 2026-10-25. The gate's log is in the coordinator's session scratchpad under final-gate/. -->

# Baseline at `main` `3aa015900`

Every later "this fails" claim is judged against this record. A failure that
also occurs here is pre-existing, and a branch must reproduce it exactly,
with nothing new beside it.

The toolchains are the channel pinned in `rust-toolchain.toml` (1.97.1) and
the justfile's pinned nightly (`nightly-2026-06-30`).

## `just gate` on the Mac (aarch64)

`just gate` in a worktree detached at `3aa015900`: clean in 417 s, every
stream ok.

| Stream | Detail |
|---|---|
| workspace | `test-all`: 2057 run, 2057 passed, 2 skipped |
| doctest | every crate's doctests passing |
| board, board-fuel | 6612 green, 0 red, heap and fuel |
| suanpan-fuel | 34 passed |
| focused-fuel | 50 passed |
| surface | 230 public function-like items = 134 board-covered + 96 excepted |
| wasm, fuzz, internal-docs, docsrs, audit | ok |

The fuzz stream builds against its committed lockfile (`--locked`). Board
captures are format v4; earlier captures do not load.
