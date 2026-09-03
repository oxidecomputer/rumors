<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch, from the triage sessions of 2026-09-02, as the operating procedure for landing the rulings in triage/rulings.md through local review packets while the rumors triage lands its own lanes in the same workspace; not authored, audited, or endorsed by Finch. Read with the ground rules in ../../README.md. -->

# Landing the before triage beside the rumors triage

The loop that turns a ruling into merged code is the one the rumors
triage wrote and runs: read
`../../2026-09-01-holistic-review-rumors/triage/WORKFLOW.md` first, in
full. Its roles (coordinator, lane agent, fresh-eyes reviewers, Finch),
its annotation format, its packet, its stops, its "what never happens",
and its merge procedure all apply here unchanged, and its `review.py` is
the one packet tool for both plans (run it with `--repo` pointing at the
lane worktree; it treats any `triage/annotations/` or `triage/reviews/`
path as the review record). This document adds only what is specific to
the before plan: where its record lives, how its lanes stack, how the two
plans share the workspace, and where its lanes build.

## The record

- The dashboard: `.agent-notes/STATUS.md`, shared with the rumors triage,
  current state only, rewritten in place at every lane state change
  (launch, review round, packet, merge, stop) by the session whose lane
  changed; the merge queue stays the ordered record.

- Rulings: `rulings.md`; the ledger: `ledger.tsv` with `ledger.py`.
- Briefs: `briefs/`, one per lane; `briefs/README.md` maps lanes to
  rulings, files, and launch order; `briefs/PROSE.md` is the prose
  standard every lane and every fresh-eyes pass applies (ruling 105).
- Annotations: `annotations/<lane>.tsv`, written by the lane agent in its
  worktree, one row per changed region, committed with the lane.
- Packets: `reviews/<lane>.md`, built by the coordinator, committed on the
  lane branch, answered by Finch with `>> finch:` lines in the Markdown.
  For reading, the coordinator renders the packet to HTML with pandoc
  (`pandoc -s -f gfm -t html5 -H <stylesheet> --metadata title=...`), the
  stylesheet being the one the rumors session's packets use (annotation
  blocks as amber quotes, diff lines colored), and opens it in the
  browser; the Markdown on the branch stays the record.

## Names

Lane branches are `before/<lane>` (the rumors plan's are `triage/<lane>`;
the two plans' lane names coincide, so the prefix is what keeps them
apart). Lane worktrees are `/Users/oxide/src/before-<lane>`; the basename
is also the lane's directory on the illumos box, so it is never reused
for a different base. Each lane's scratchpad is its own subdirectory of
the session scratchpad, named for the lane.

## Stacks

A lane that must land after another branches from that lane's branch,
never from `main`, and its packet is diffed against that branch. A child
never merges before its parent; when the parent merges, the child is
rebased onto `main` the same day and its packet rebuilt; a lane with two
parents waits for both to merge and then branches from `main`. The
stacks, from `briefs/README.md`'s launch orders:

- **The gate stack.** The rumors lane branch `triage/p1-gate` (ruling
  107) → `before/p1-gate` → `before/p1-fuzz`; `before/p1-gate` →
  `before/p1-survivors`. Later, `p5-judge` and `p5-scanners` follow the
  gate lanes for the justfile and lockfiles (and `p1-fuzz` for
  `bands.rs`, `p2-surface` for the extractor).
- **The harness stack** (every lane in it edits `tests/meter.rs`, so they
  are serialized): `main` → `before/p1-harness` → `before/p1-board` →
  `before/p1-suites` → `before/p2-rows` → `before/p2-cures` (after
  `p2-widths` has merged). `p6-harness` and the six board lanes come
  last, `p6-harness` first among them.
- **Independent from `main`:** `p2-widths`, `p2-surface`; the P3, P4, P5,
  P7, P8, and remaining P6 lanes each base on `main` once every lane in
  its Follows column has merged.

## The merge step's last check

A rebase can drop a line silently: a `use` block rewritten by the other
plan, a helper deleted under a test that still calls it. Neither shows
in a range-diff that reads "identical" for every commit, because the
range-diff compares the lane's commits to themselves, not the merged tree
to a compiler. So the coordinator's merge step, after the final rebase
onto `main` and before the fast-forward, compiles the rebased tip on the
box: `cargo check --locked --workspace --all-targets` under default
features and under `--all-features`. A red tip is never fast-forwarded,
even when the red file is the other plan's: it is reported to that
session and to Finch, and the merge waits for the repair. A tip that
moved since the packet Finch read is compiled again; the packet's
range-diff against the reviewed head shows him the difference.

## Sharing the workspace with the rumors plan

The two plans' code never overlaps: no rumors lane touches
`crates/before`, and no before lane touches the root crate's `src/` or
`tests/`. They meet only at the workspace root: the justfile, `.github/`,
`Cargo.toml` and `Cargo.lock`, root `AGENTS.md`, `deny.toml`,
`rust-toolchain.toml`, `.cargo/`, and `tools/`. Rules, proposed to the
rumors session on 2026-09-02 and recorded here as agreed or amended:

1. One owner per shared root file per wave. The other plan's lane stacks
   on the owner's branch or waits for its merge. In wave 1 the rumors
   gate lane owns `ci.yml`, the justfile header and the `ci` and `gate`
   recipe lines, the root lockfile, and `tools/testdoc`.
2. A lane's edits to a shared root file go in its own last commit, so a
   cross-plan rebase conflicts in one small commit the coordinator
   resolves; Finch never resolves a conflict.
3. When one plan hardens an instrument the other retires, the retiring
   lane goes second: `tools/digestshare` (ruling 79) after the rumors
   renderer lane, `tools/workflowlint` (ruling 106) after the rumors gate
   lane.
4. A merge queue at `.agent-notes/merge-queue.md`: one line per lane
   ready for merge, in declaration order, naming the lane, its branch,
   its base, and the root files it touches. Finch merges in that order;
   the merging session appends the merge SHA; the other session rebases
   its stacks the same day.
5. Primary-worktree hygiene in `/Users/oxide/src/rumors`, which both
   sessions commit notes from: commits with explicit pathspecs only
   (`git commit -- <paths>`), never a bare `git add -A`; no rebase,
   checkout, stash, or reset there; lane rebases happen in lane
   worktrees; `main` is fast-forwarded only when `git status` shows the
   other session has nothing staged.
6. Box budget: each session runs at most four concurrently building lanes
   on the illumos box and at most one wall-time measurement at a time,
   under `pset-run`, announced in the queue file first. The before plan
   builds nothing on the Mac.
7. Messages between the two coordinator sessions (Finch authorized the
   direct channel; every exchange is visible to him in both sessions) are
   for three events only: a merge to `main` landed (SHA, root files
   touched); a lane launching that edits a shared root file; a ruling
   that changes a shared instrument. A message from the other session is
   a report: corroborate it against the branches before acting on it.

## Where a lane builds

Every build, test, and gate of a before lane runs on the illumos box
(`ox-east-1`, per the `building-on-illumos` skill), never on the Mac:
Finch ruled that a clean `just gate` on the box is the gate of record and
that the Mac runs no gate. The skill's wrapper syncs the lane worktree to
`~/src/<worktree basename>` on the box and runs one command there with a
build directory of its own (`~/build/<basename>`), so lanes and plans do
not collide; cargo runs `--locked` there so the lockfile stays the Mac's;
nothing is edited or committed on the box. The clock guard from the
skill (rsync preserves mtimes; a box clock ahead of the Mac yields a
green build of stale code) is checked before a lane's first build; the
box runs chrony and Finch owns its clock. Two legs pin toolchain-derived
numbers and may fire on the box if its toolchains differ from the
pinned ones (`1.97.1` and `nightly-2026-06-30` are both installed there);
a lane reports such a leg with both numbers and re-pins nothing.
One leg is expected red on the box and counts as clean when it is the
only failure: `fuzz`, because libFuzzer has no illumos port
(`FuzzerPlatform.h` refuses the target); a lane quotes that line and
runs no fuzz build elsewhere (Finch's ruling, recorded in the rumors
workflow: fuzzing is CI's). Benchmarks whose committed baselines are the
Mac's are the coordinator's to schedule, on a quiet machine, once.

A lane's final gate run is backgrounded on the Mac side with its output
redirected to a log under the lane's scratchpad directory, polled with
short checks, and its verdict read from the log; never a foreground
demand, never piped through a filter. The polling happens inside the
lane's turn: a `sleep` loop in the foreground that exits when the log's
exit line appears. A lane never ends its turn to wait for a background
run's notification, because re-invocation on that notification is
unreliable and the coordinator is then the only thing that can wake it;
every launch prompt says so, and a lane that ends a turn waiting is
woken by the coordinator with the verdict's location. The lane retires its box build
directory after its final commit (`rm -rf ~/build/<basename>` over ssh);
the worktree stays for the coordinator.

## Effort

Lane agents that touch a harness, an instrument, or production code run
at high effort, as do fresh-eyes reviewers; the vocabulary and prose
sweeps (P3, the prose halves of P4) may run at medium. Lanes are
launched with the Agent tool, one at a time or in waves, never as a
scripted workflow: a lane's defining event is a stop that needs Finch's
ruling, which the coordinator relays and a script cannot pause for.
