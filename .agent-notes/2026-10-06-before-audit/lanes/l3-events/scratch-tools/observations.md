# L3 observations (draft; each verified by reading at 58285ca5 unless marked)

## O-1. `ticks` docs name the count `n` where the parameter is `k`

- `crates/before/src/party.rs:243` — "Advances `version` by `n` events for this [`Party`]" has no
  final period and names `n`; the parameter is `k` (party.rs:269) and the next sentence says
  "`n` sequential [`tick`](Self::tick)s".
- `crates/before/src/clock.rs:122-128` — "Advances this [`Clock`] by `n` events ... byte-identical to
  `n` sequential ticks", then "The count `k` is any unsigned number"; the parameter is `k`.
- `Version::ticks` (version.rs:252-256) uses `k` throughout: the three sibling docs should share
  one outline (Part IV, "Parallel things get parallel prose").
- Kind: prose correction toward the code; no behavior change.

## O-2. `memo.rs` carries an inline test module

- `crates/before/src/version/tick/memo.rs:156-183` — `#[cfg(test)] mod tests { ... }` inline,
  against the repo convention of a sibling `tests.rs` (AGENTS.md, "Writing tests").
- Kind: self-contained move; behavior-preserving.

## O-3. `let _ = matched;` after `debug_assert!(matched, ..)` is dead

- `crates/before/src/version/tick.rs:672-673` and `:776-777`. `debug_assert!` expands to an
  `if cfg!(debug_assertions)` that still names `matched`, so no unused-variable warning arises
  in release; the binding is residue (first reported 2026-09-01 as skyline-fill-grow-15, still
  present).
- Kind: trivial cleanup.

## O-4. Internal `# Panics` on `TickWalk::tick`/`ticks` names a state the Party grammar excludes

- `crates/before/src/version/tick.rs:151-154`, `:173-176`: "The party must own at least one
  region." Party tags are `00` (owned), `10`, `01`, `11` (branches with present children), so
  every decodable party owns a region (party/io/validate.rs:37-52). The clause is true but
  unreachable; it could say why it holds instead of reading as a caller obligation.
- Kind: maintainer prose.
