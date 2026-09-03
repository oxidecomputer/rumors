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

## Ready for Finch, in merge order

- rumors `p1-gate`: branch `triage/p1-gate`, packet head `5cd25b55`, last
  code commit `b06000df`, base `d631cda6` (main). Root files: justfile,
  `.github/workflows/ci.yml`, `Cargo.toml` (`bytes` dev-dependency),
  `Cargo.lock`, `tools/testdoc`. `before/p1-gate` stacks on this.
- rumors `p1-renderer`: `triage/p1-renderer`, packet `ae3d9fc4`, code
  `fff43de9`, base `b8401660`. Root files: `Cargo.toml` (`cbor-diag`
  rev pin), `Cargo.lock`, `tools/digestshare`, `AGENTS.md`.
- rumors `p2-codec`: `triage/p2-codec`, packet `610d6108`, code
  `86f65f8f`, base `a6a79c39`. No root files.
- rumors `p1-harness-crate`: `triage/p1-harness-crate`, packet `5e2f3cde`,
  code `f6f3a0b6`, stacked on `p2-codec`. No root files.
- rumors `p1-harness-tests`: `triage/p1-harness-tests`, packet `9c8ce16c`,
  code `82cc0257`, base `51ccf03c`. Root files: `.config/nextest.toml`
  (one comment), `design/rumors-frame-fuzz.md`, `Cargo.lock`.
- rumors `p2-walk`: `triage/p2-walk`, packet `ca5c3290`, code `eb97720b`,
  base `b1e8a397`. No root files.
- rumors `p1-causality`: `triage/p1-causality`, packet `aa3e2aa1`, code
  `f1b4859c`, base `4f796995`. Root files: `Cargo.toml` (`rand_chacha`
  dev-dependency), `Cargo.lock`.

## Announcements

- before: `before/p1-harness` launched from `27ac8d92`; edits
  `crates/before/tests/meter.rs` and, as its own last commit, one line of
  the justfile's `test` recipe (which the rumors memwatch lane is also
  rewriting; that lane owns the line and this lane rebases over it).
