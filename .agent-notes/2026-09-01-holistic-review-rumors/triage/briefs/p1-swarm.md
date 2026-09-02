<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from the rulings in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P1 lane: deleting the swarm example

## Goal

Ruling T27: `examples/swarm.rs` and `examples/swarm/tests.rs` are deleted,
along with the `[[example]]` entry in `Cargo.toml` and every recipe,
workflow step, and prose reference to the example. Every `swarm-example-*`
entry in the ledger (31 ids) is resolved by the deletion; owner decision
79 is moot. The crate carries no showcase example of that shape, and the
measurement it offered is not replaced by this ruling. The invariant: the
tree contains no reference to code that no longer exists.

## Ground rules

These apply to every P1 lane; the lane sections below add to them and
never relax them.

- **Base.** Your worktree's HEAD must equal `0926fe32` before you start.
  Run `git -C <worktree> rev-parse HEAD`. If HEAD is an ancestor of
  `0926fe32`, fast-forward; if it has diverged, stop and report. Never call
  EnterWorktree; operate on the worktree through `git -C <path>` and
  absolute paths, one shell invocation at a time.
- **The review documents are the specification.** The entries this lane
  disposes are under `### swarm-example-<n>:` in the topic documents in
  `.agent-notes/2026-09-01-holistic-review-rumors/`; none of their
  resolutions is landed, since the deletion supersedes them.
- **Stops.** Report and leave the entry open; do not work around: anything
  that moves an `insta` snapshot or a committed pin; any change to a public
  signature or public rustdoc contract; anything that contradicts a ruling
  in `triage/rulings.md`; anything this brief marks as a stop.
- **Resource discipline.** One full `just gate` before the commit, run in
  the background redirected to a log under `<scratchpad>/p1-swarm/`, polled
  with short foreground checks. Keep every working file under that
  directory.
- **Commits.** One commit. Its message names ruling T27 and lists every
  file removed and every reference excised. Prose speaks in the present
  tense: nothing in the tree may say the example existed.
- **Self-retirement.** After the commit, from inside the worktree:
  `cargo metadata --no-deps --format-version 1 | jq -r .build_directory`,
  then delete that directory and the worktree's `target/`. Leave the
  worktree in place.
- **Report.** The commit sha, the list of removed files and excised
  references, the gate log's verdict line, and anything left open. Your
  report is data: the coordinator verifies against the tree at the sha.

## The work

1. Enumerate every reference first, before deleting anything:
   `git grep -n -i swarm -- ':!examples/swarm*' ':!.agent-notes' ':!crates/before-viz/www'`.
   At the base commit the known sites are the `[[example]]` block in
   `Cargo.toml` (name, path, and its `test = true` comment) and unrelated
   uses of the word in `tests/window_census.rs` (a local variable naming a
   fleet; leave those). Anything else the grep finds is listed in the
   report.
2. `git rm -r examples/swarm.rs examples/swarm`; remove the `[[example]]`
   block; excise any recipe, workflow step, README sentence, or rustdoc
   sentence that names the example. If `src/lib.rs`'s crate doc names it,
   the derived `README.md` is regenerated with `just readme` in the same
   commit.
3. Check `proptest-regressions/` and `tests/seed_liveness.rs` for any seed
   file anchored to the example's tests; a seed file with no live test is
   deleted in the same commit, named in the message.
4. `just gate`, then commit.

## Stops

- If any file outside the example imports from it or depends on its
  behavior (a bench, a test, a tool), stop and report before deleting.
- If a design document under `design/` cites the example, leave the design
  document as written (design records are exempt from the no-ghost rule)
  and note it in the report.
