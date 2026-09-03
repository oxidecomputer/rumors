<!-- CAVEAT LECTOR: written by Claude (the two triage coordinator sessions of 2026-09-02) for Finch as the shared merge queue of the before and rumors triages; not authored, audited, or endorsed by Finch. -->

# Merge queue for the before and rumors triage lanes

The current-state dashboard of both triages is `STATUS.md` beside this
file; this file is the ordered record it links to.

Coordinator sessions (the names `ListAgents` shows and `SendMessage`
takes): before is `rumors-74`, rumors is `rumors-6e`. A session whose
address changes writes the new one here. Each session's live journal is
its triage's `triage/HANDOFF.md`, with a recovery procedure at the top.

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
   other session has nothing staged. Every edit-and-commit of this
   file or `STATUS.md` runs under one mutex shared by both sessions,
   since the clean check and the commit are seconds apart and a commit
   by the other session in between sweeps the edit into it: in one
   command, `mkdir .agent-notes/.editlock` in a retry loop (a lock older
   than five minutes is stale and removed), the clean check (no
   uncommitted or staged change to the file), the edit, the commit with
   the pathspec (`--no-gpg-sign` when the agent refuses), and `rmdir` on
   exit; the directory is untracked and never added. Any rewrite of
   `main` (an identity repair, say) happens at Finch's word with the
   other session paused, and is announced here first. This file and `STATUS.md` are edited by
   both sessions in the primary worktree: before editing either, check
   it carries no uncommitted change from anyone (`git diff --quiet` and
   `git diff --cached --quiet` on the path), wait and retry if it does,
   then edit and commit in one command, so neither session's commit
   carries the other's hunks; when the signing agent refuses inside that
   command, the same command falls back to `--no-gpg-sign` (the commit
   is re-signed in the rewrite above) rather than leave the file dirty.
6. No per-session lane cap on the illumos box (Finch's ruling, relayed
   by the rumors session on 2026-09-02): each session keeps its
   concurrently building lanes to what the load bears, holding a launch
   while the box's one-minute load average is above about 150 on its 192
   threads. Every build and gate runs in the general pool, with each
   lane's parallelism bounded so a few lanes cannot oversubscribe the
   box: `CARGO_BUILD_JOBS=32` and `NEXTEST_TEST_THREADS=32` exported in
   the remote command (a gate's own concurrent streams multiply these,
   so a gate is the heaviest thing a lane runs). No exclusive processor
   set for a gate or a build: a set reserves cores a gate leaves idle
   through its serial phases and takes them from everyone else (Finch's
   observation). `pset-run` is for wall-time measurements only, one at a
   time, announced here first. A leg that fails only by nextest's 180 s
   per-test limit at a one-minute load under about 150 is a finding
   about the test, reported as such; above that, the stream is re-run
   once the load is down. Gates serialize across both sessions through
   one mutex on the box, because a load or `pgrep` check before a
   launch cannot close the race between two lanes launching in the
   same second: in one remote command, `mkdir ~/gate.lock` in a retry
   loop (a lock older than 45 minutes is a dead gate's and is removed),
   then `just gate` (or any whole-suite run), then `rmdir` on exit;
   targeted runs and builds stay outside the lock, so at most one gate
   runs on the box at a time. Before builds nothing on the Mac; rumors
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

- before: `5b0a17d4` the harness lane (31 code commits from `5a6be5c0`, packet and
  queue notes); root files: justfile (the `test` recipe line and two
  recipe comments); `crates/before/Cargo.toml` (`[[test]] meter` with
  `required-features`).

## Ready for Finch, in merge order

- rumors `p2-commit-path`: branch `triage/p2-commit-path`, packet head
  `14d0d5fb`, last code commit `5a4559e1`, base `1336daa0` (main). Root
  files: README.md (regenerated from the crate doc). Its commits are
  unsigned (the lock protocol); the merge rebase re-signs.

- before `p2-widths`: branch `before/p2-widths`, packet head `ba36e970`,
  last code commit `d5e63e93`, base `main`. Root files: justfile (one
  recipe, `fuzz-test`, and its line in `all`). Merges before
  `p2-surface`. Packet:
  `.agent-notes/2026-09-01-holistic-review-before/triage/reviews/p2-widths.md`.

- before `p2-surface`: branch `before/p2-surface`, packet head `cad0b75d`,
  last code commit `8a2aa4d7`, base `main`. Root files: `Cargo.toml`
  (workspace `proc-macro2` and `syn`), `Cargo.lock` (surface-scan's
  entries), justfile (one comment paragraph). One stop (the
  `fuelscape.js` caption). Packet:
  `.agent-notes/2026-09-01-holistic-review-before/triage/reviews/p2-surface.md`.

- before `p2-generators`: branch `before/p2-generators`, packet head `e84a74a3`,
  last code commit `3cb24804`, base `main`. No root files. Test files
  only; merges in any order among the before packets (the rumors
  caselint roots widen after it). Packet:
  `.agent-notes/2026-09-01-holistic-review-before/triage/reviews/p2-generators.md`.

- before `p1-gate`: branch `before/p1-gate`, packet head `75e0bc91`, last code
  commit `6a4d255e`, base `main`. Root files: justfile, `.github/`
  (ci.yml, dependabot.yml, pages.yml), `deny.toml`, root `AGENTS.md`,
  `tools/` (lockcheck new; mutantcheck and workflowlint deleted),
  `.cargo/mutants.toml` deleted, root `Cargo.lock` and the five detached
  locks (upward convergence). One stop (the `syn` 2/3 roster); its gate
  reads red on audit for that stop and on wasm for the committed fuel
  seed until `p1-fuzz` refits. Merges after the three P2 packets; its
  children `p1-fuzz`, `p8-tagwalk`, `p1-survivors` follow. Packet:
  `.agent-notes/2026-09-01-holistic-review-before/triage/reviews/p1-gate.md`.

- before `p8-tagwalk`: branch `before/p8-tagwalk`, packet head `3316c4ed`, last
  code commit `ddd5684d`, base `before/p1-gate` (`90f7dd9c`): a child, merges
  after the gate lane. No root files. Packet:
  `.agent-notes/2026-09-01-holistic-review-before/triage/reviews/p8-tagwalk.md`.

## Announcements

- rumors: `p1-envelope` launched from `0fad870c` (Finch's word,
  2026-09-03 night); root files: `examples/envelope_sim.rs` deleted,
  `results/` cleaned, README.md regenerated if the crate doc's envelope
  figures move; no justfile or ci.yml line (no recipe names the example).

- before: `before/p1-gate` now also touches the root `Cargo.lock` (`bytes`
  1.11.1 to 1.12.1, ruling 114: the lock convergence goes upward to the
  release the fuel bands were calibrated against); one line beyond the
  lockfile touches already announced.

- REPAIRED at `9a7e898e` (rumors; the walk's dropped imports, helper, and
  aliases restored; the causality suite's call to the deleted
  `FaultPlan::is_clean` replaced by `!= FaultPlan::NONE`; both checks,
  clippy, and the affected suites green on the box). Both merge steps now
  compile the rebased tip before any fast-forward. The original report:
  OPEN on `main` (found 2026-09-03 by the before session, relayed to the
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
