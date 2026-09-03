<!-- CAVEAT LECTOR: written by Claude (the two triage coordinator sessions of 2026-09-02) for Finch as the shared merge queue of the before and rumors triages; not authored, audited, or endorsed by Finch. -->

# Merge queue for the before and rumors triage lanes

Both triages land lanes into `main` from the same primary worktree, and
their lanes meet at the workspace root (the justfile, `.github/`, the
manifests and lockfile, root `AGENTS.md`, `deny.toml`,
`rust-toolchain.toml`, `.cargo/`, `tools/`). This file is the order
Finch merges in and the record of what merged. The rules the two
coordinator sessions agreed:

1. One owner per shared root file per wave; the other plan's lane stacks
   on the owner's branch tip or waits for its merge.
2. A lane that touches a shared root file incidentally (a dependency
   pin, the lockfile) makes that edit its own last commit, so a
   cross-plan rebase conflicts in one small commit a coordinator
   resolves. A lane whose whole surface is root files (a gate lane)
   commits per entry as usual; stacking is the rule for it.
3. When one plan hardens an instrument the other retires, the retiring
   lane goes second.
4. A lane is appended below when its packet is ready for Finch, in
   declaration order: lane, branch, base, root files touched. Finch
   merges in that order. The merging session appends the merge SHA on
   the same line; the other session rebases its stacks the same day.
5. Primary-worktree hygiene in `/Users/oxide/src/rumors`: commits with
   explicit pathspecs only, never a bare `git add -A`; no rebase,
   checkout, stash, or reset there; lane work happens in lane
   worktrees; `main` is fast-forwarded only when `git status` shows the
   other session has nothing staged. Any rewrite of `main` (an identity
   repair, say) happens at Finch's word with the other session paused,
   and is announced here first.
6. No per-session lane cap on the illumos box (Finch's ruling, relayed
   by the rumors session on 2026-09-02): each session keeps its
   concurrently building lanes to what the load bears, holding a launch
   while the box's one-minute load average is above about 150 on its 192
   threads. Every gate runs inside an exclusive processor set,
   `pset-run -n 40 -- just gate` through the skill's wrapper, at most two
   concurrent psets per session, so a gate's tests cannot be starved into
   nextest's per-test limit by another build; non-gate builds run unbound
   in the general pool. A leg that still fails only by the 180 s limit
   inside a pset is a finding about the test, not about load, and is
   reported as such. Wall-time measurements under `pset-run` stay one at
   a time, announced here first. Before builds nothing on the Mac; rumors
   benches run on the Mac against their committed baselines.
7. The two coordinator sessions message each other (Finch's
   authorization; he sees every exchange) for three events: a merge to
   `main` landed, a lane launching that edits a shared root file, a
   ruling that changes a shared instrument.

Joint work at the end: publication prep (rumors T79: license headers on
every file in the workspace, `publish = false`, crate metadata) touches
both plans' trees and runs last, as one lane, after both triages' code
lanes have closed.

## Merged (root files touched)

- rumors: `2cb24d45` swarm example deleted; root `Cargo.toml`
  dev-dependencies and `Cargo.lock` pruned.
- rumors: the conformance lane (`c1477615` and its series); no root
  files.
- `27ac8d92` (Finch): illumos-scoped clippy allow at
  `crates/before/src/lib.rs`.
- rumors: `p1-memwatch`, main at `d631cda6` (`6b258a5f`, `34eb563e`,
  `d631cda6`); `tools/memwatch` deleted, twelve justfile recipes
  unwrapped (`test`, `test-all`, `doctest`, `citecheck`, `fuzz-build`,
  `bench-build`, `fuzzfit-build`, `fuzzfit`, `fuelscape-test`,
  `surface-totality`, `coverage-kernel`, `coverage-kernel-branch`),
  `ci.yml` and root `Cargo.toml` comments only.

- rumors: `c4d1778c` the gate lane (`f1d7773c` through `9f19d098`, packet
  `c4d1778c`); root files: justfile (recipes, `ci` and `all`, header),
  `.github/workflows/ci.yml` (pin order; cargo-mutants@27.1.0 kept for
  the `before` mutants lane), `Cargo.toml` (`bytes` serde to
  dev-dependencies), `Cargo.lock` (one line), `tools/testdoc`.

- rumors: `12aa9a8d` the codec lane (`1b7f3de4`, `28055b77`, packet
  `12aa9a8d`); no root files.

- rumors: `a3200a2e` the renderer lane (`f0d829a2` through `896705ce`, packet
  `a3200a2e`); root files: `Cargo.toml` (`cbor-diag` pinned by rev),
  `Cargo.lock` (source string only), `tools/digestshare`, `AGENTS.md`
  (one snapshot re-accept class); all 22 wire snapshots re-accepted
  under T138.

- before: `e27ba5e6` the proptest-cases lane (`7892c60c` through `f60734bb`,
  packet `e27ba5e6`); no root files.

- rumors: `3fab5759` the harness-tests lane (`7d978fcf` through `2d3906cf`,
  packet `3fab5759`); root files: `.config/nextest.toml` (one comment),
  `design/rumors-frame-fuzz.md` (one paragraph), `Cargo.lock`. The
  inter-process disruption family is deleted (T143); `tests/common/tcp.rs`
  stays for the TCP link suites.

- rumors: `59608589` the harness-crate lane (twelve commits over the codec merge;
  packet `59608589`); no root files.

- rumors: `9fbd9c16` the walk lane (`bed0eeb1` through `302c1f82`, packet
  `9fbd9c16`); no root files.

- rumors: `44a26d70` the causality lane (seven code commits, packet `44a26d70`); root
  files: `Cargo.toml` (`rand_chacha` dev-dependency), `Cargo.lock`.

## Ready for Finch, in merge order

- rumors `p1-gate`: branch `triage/p1-gate`, packet head `5cd25b55`, last
  code commit `b06000df`, base `d631cda6` (main). Root files: justfile,
  `.github/workflows/ci.yml`, `Cargo.toml` (`bytes` dev-dependency),
  `Cargo.lock`, `tools/testdoc`. `before/p1-gate` stacks on this.

- before `p1-harness`: branch `before/p1-harness`, packet head `d1979bf6`,
  last code commit `e9a1fd6d`, base `main`. Root files: justfile (the
  `test` recipe line and two recipe comments). Also
  `crates/before/Cargo.toml` (a `[[test]]` entry). Packet:
  `.agent-notes/2026-09-01-holistic-review-before/triage/reviews/p1-harness.md`.
  Children: `before/p2-widths` stacks on it next.

## Announcements

- OPEN on `main` (found 2026-09-03 by the before session, relayed to the
  rumors session as it handed off; absent from its HANDOFF.md): merged
  `main` fails `cargo check --locked --workspace --all-targets` (the
  `just check` recipe, default features) in
  `src/tree/mirror/streaming/remote/proxy/tests.rs`: `LeftFailure`,
  `RightFailure` (line 251), `PayloadCodec`, `PayloadDepthLimit` (265,
  271) are used without a `use` in that file. Last touched by the
  rumors p2-walk commit `bed0eeb1`, merged over the harness-crate
  lane's rewrite. The gate's all-features legs do not see it; `just
  check` does. The rumors successor session owns the repair; the before
  lanes' packets note it where their own `just check` reads red for
  this reason alone.

- before: `before/p1-gate` launched, stacked on rumors `triage/p1-gate` at
  `b06000df`; it edits the justfile (the mutants-list and workflowlint
  legs go; the wasm32-pins leg joins CI; the lockfile audit derives its
  list), `.github/workflows/ci.yml` (cargo-mutants install removed; the
  wasm32-pins leg), `deny.toml`, `rust-toolchain.toml`,
  `.cargo/mutants.toml` (deleted), `tools/mutantcheck*` and
  `tools/workflowlint` (deleted), root `AGENTS.md` (the mutant-exclusions
  paragraph), and the six lockfiles. It merges after the rumors gate
  lane; before's harness packet follows the seven rumors packets in the
  queue.

- before: `before/p1-harness` launched from `27ac8d92`; edits
  `crates/before/tests/meter.rs` and, as its own last commit, one line of
  the justfile's `test` recipe (which the rumors memwatch lane is also
  rewriting; that lane owns the line and this lane rebases over it).
