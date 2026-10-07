# Simplification brief S1: four self-contained tidy-ups in the tick lane

For a builder. Each item is behavior-preserving, touches no public signature or format, and is
one commit. Line numbers are at `58285ca5` (verified with `git show 58285ca5:<path>`).

## 1. One outline for the three `ticks` docs (prose toward the code)

- Current: `crates/before/src/party.rs:243-246` opens "Advances `version` by `n` events for this
  [`Party`]" with no final period, and says "`n` sequential [`tick`](Self::tick)s", but the
  parameter is `k` (`party.rs:269`). `crates/before/src/clock.rs:122-128` says "by `n` events" and
  "byte-identical to `n` sequential ticks" and then "The count `k` is any unsigned number"; the
  parameter is `k` (`clock.rs:145`). `Version::ticks` (`version.rs:252-256`) uses `k` throughout.
- Proposed: name the count `k` in all three, end the first sentence with a period, and give the
  three siblings one outline varying only in the receiver (Part IV, "Parallel things get parallel
  prose"). Do not change the complexity islands or examples.
- Why more obviously correct: the prose names the parameter the reader passes.
- Coverage: doctests unchanged; `just gate`'s docs legs.

## 2. Move `memo.rs`'s inline test module to a sibling file

- Current: `crates/before/src/version/tick/memo.rs:156-183`, an inline `#[cfg(test)] mod tests { … }`
  holding `selected_out_of_order_values_cross_block_boundaries`.
- Proposed: `#[cfg(test)] mod tests;` plus `tick/memo/tests.rs`, per AGENTS.md "Writing tests".
- Coverage: the same test runs unchanged.

## 3. Delete the dead `let _ = matched;` bindings

- Current: `crates/before/src/version/tick.rs:672-673` and `:776-777`:
  `debug_assert!(matched, "…"); let _ = matched;`. `debug_assert!` expands to
  `if cfg!(debug_assertions) { assert!(…) }`, which still names `matched` in release builds, so
  no unused-variable warning needs suppressing. (Also reported 2026-09-01; still present.)
- Proposed: delete the two `let _ = matched;` lines; confirm `cargo clippy -p before
  --all-targets --all-features --release -- -D warnings` stays clean.
- Coverage: none needed (no behavior).

## 4. Say why tick's party always owns a region

- Current: `crates/before/src/version/tick.rs:151-154` and `:173-176`, maintainer `# Panics`:
  "The party must own at least one region." This reads as a caller obligation, but the party
  grammar has no empty spelling: tags are `00` (owned) and three branch tags with present children
  (`party/io/validate.rs:37-52`, `party/io/reader.rs:268-272`), so every decodable `Party` owns a
  region.
- Proposed: keep the canonical-stream panic, and replace the region sentence with one stating that
  a `Party` always owns a region (its encoding has no empty form), so this cannot fire.
- Coverage: none needed (prose).
