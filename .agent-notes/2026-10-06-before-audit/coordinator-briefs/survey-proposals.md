<!-- CAVEAT LECTOR: builder briefs written by the coordinator (Claude Opus 5.5) from the instrument survey (`instrument-survey.md`, section 1). -->

# Builder briefs: proposal branches from the instrument survey

Each branch implements one eligible candidate from `instrument-survey.md`
section 1, which names the failure it catches. Read your candidate's section
there in full before starting. These are proposals: they wait for the owner's
review and never land on their own.

The owner's caution governs every branch: build the smallest change that
catches the named failure, and nothing beyond it. Extend the shared
instruments (`testing::meter`, the board, `justfile` recipes) rather than
adding parallel harnesses.

## Shared rules

- Base: the slot's HEAD as created (stated per branch). Verify it and a
  clean tree first; stop if either differs.
- Every branch carries a known-bad demonstration: a reversible swap that
  reproduces the named failure, which the new instrument must fail and the
  old one must pass. Report both outcomes verbatim and restore the swap
  (`git diff` empty afterwards).
- No performance values or counts in tree prose; readings go in commit
  messages. Every new test or check has a doc comment stating what it
  protects.
- Box runs only via `~/.claude/skills/building-on-illumos/scripts/on-illumos.sh
  <worktree> '<cmd>'`, with `unset CARGO_TARGET_DIR` first; outputs only under
  the worktree. About four focused runs, then one landing check at the tip
  (the coordinator's `~/bin/audit-reserved ~/bin/audit-check`), run in the
  background to a file and polled.
- `main` at the base includes the board-pin fix `ddfabe4c` (check with `git
  merge-base --is-ancestor`): expect the post-board-pin baseline, 759 tests,
  board 5311 green with pins clean, plus your own tests.
- Never use EnterWorktree; launch no services; delete nothing outside your
  worktree; signed commits, imperative messages naming the component.
- Scratch: `<session scratchpad>/builder-<branch-suffix>/`.

## 40. `proposal/worst-case-pin-always` (survey 1.3), base `main`

**Failure.** `_gate-board` runs `amp-board-acceptance worst-cases-pin`; `just`
stops at the first failing recipe, so whenever acceptance fails, ranking drift
goes unreported and surfaces later, attributed to the next passing commit.
**Change.** Give `_gate-board` a body that runs both recipes whatever the first
returns, and fails if either failed, reporting each. Keep the recipe's comment
accurate. (The coordinator updates its own landing-check script separately.)
**Known-bad.** With one acceptance cell made to fail (lower one ceiling by
swap) and one `WORST_RANKINGS` row flipped (swap), the old recipe reports only
the acceptance failure; the new one reports both.

## 41. `proposal/peakalloc-single-thread` (survey 1.6), base `main`

**Failure.** `resident_space_comparison_is_scoped_by_shape` failed `4729 !=
4609` on two branches: libtest's main thread allocates inside the measurement
window of a process-wide counter when preempted after spawning the test thread.
`tests/meter.rs` and `tests/amp_board_smoke.rs` have the same exposure.
**Change.** Run libtest single-threaded for these binaries
(`RUST_TEST_THREADS=1`), set where every nextest invocation inherits it (find
the right place: the workspace's cargo or nextest configuration; confirm the
detached workspaces are unaffected or covered). Each of the three binaries
asserts the variable is set, with a message naming the race.
**Known-bad.** With the variable unset, each binary's assertion fails. The race
itself is too rare to reproduce on demand; say so in the commit message rather
than building a reproduction.

## 42. `proposal/wasm32-compare-pin-reach` (survey 1.2), base `main`

**Failure.** Narrowing a cursor position before dividing it into a byte index
(`(position as usize) / 8` for `(position / 8) as usize` at `bits/reader.rs:148`,
and the same at `bits/reader/gamma/window.rs:28`) passes every wasm32 pin,
including `version_compare_crosses_the_usize_position_boundary`, whose doc
says it catches exactly that: its large operand's live bits all sit below
`2^32`.
**Change.** First try growing the existing `Check::VersionCompare` synthesis so
a decisive bit lies at or past `2^32` with different data at the wrapped
offset, without adding a protocol variant. If the guest cannot afford it,
instead correct the pin's doc to what it guards and record in the validation
index that cursor positions are unpinned on 32-bit; report which path you took
and why.
**Known-bad.** Each of the two narrowings, applied by swap, must fail the pin
(path one), or the doc must no longer claim to catch them (path two).

## 43. `proposal/board-exact-compare` (survey 1.7), base `main`

**Failure.** Comparing two board runs means diffing rendered text that rounds
exponents to 0.01 and per-byte readings to 0.1. Twice this stopped fixes on
fitted-exponent rises that were constant shifts (the W mask's 314 rises; the
`min_ticks` records' rises), and an exponent-keyed rule missed a 40% flat rise
(`version_min_ticks × wide-arming`, 2.9 → 4.0 B/B).
**Change.** In `crates/before/examples/amp_board.rs`: a mode that writes every
cell's exact readings at every sample size to a file (the shard processes
already send exact samples), and a compare mode that reads two such files and
prints, per cell and currency, the difference at each size, classified as
unchanged, a constant shift, or growing with input, with the growth's slope.
A developer tool: no gate leg. Document both modes where the board's other
modes are documented.
**Known-bad.** On a synthetic pair of files (or two real captures with a swap
between them), the compare mode reports a constant shift as constant, a
proportional change as growing, and an identical pair as unchanged; a rounded
text diff of the same pair misclassifies at least the constant shift.

## 44. `proposal/board-neutral-coverage` (survey 1.8), base #61 (`simplify/span-refine-partial`, `89afe1d4`)

**Failure.** #61 changes `Query::coverage`'s heap for bounded queries without
holes, neutral queries, and polar queries whose holes need no clamp, and all
15,933 board heap readings stay identical: the board's coverage operations
build only shapes whose clamp is still needed. A regression on those shapes
passes every instrument.
**Change.** One board operation variant over a bounded, hole-free query (for
example `after(v) & before(w)`), registered like its siblings and judged by
the existing heap ceiling. Re-pin worst cases only where the new row requires
it, with the movement in the commit message.
**Known-bad.** With #61's change reverted by swap, the new row's heap reading
differs from the tip's (report both); report whether the existing ceiling
fails on the revert, and if not, say so plainly rather than tightening the
ceiling, which is the owner's question 65.
