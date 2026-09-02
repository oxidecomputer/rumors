<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch, from the triage sessions of 2026-09-02, as the operating procedure for landing the rulings in triage/rulings.md through GitHub pull requests; not authored, audited, or endorsed by Finch. Read with the ground rules in ../../README.md. -->

# Landing the triage through pull requests

This is the procedure a fresh session follows to turn the rulings in
`rulings.md` into merged code, one lane at a time, with Finch reviewing
every change as a pull request. It is written to be handed to a session
that has read nothing else: the first section says what to read, the rest
says what to do. The briefs under `briefs/` are the per-lane inputs; this
document is the loop around them.

## What to read first

1. `TRIAGE.md`: the phases, the dispositions, and the ledger's columns.
2. `rulings.md`, whole: every decision Finch made, numbered T1 through
   T132. A brief cites rulings by number; the ruling is the authority when
   the two disagree.
3. `briefs/README.md`: the lanes, their base, their independence, and the
   launch order. Then the brief of the lane being run.
4. `ledger.py summary` and `ledger.py check`: the state of the record. A
   row is pending until its `sha` column names a merged commit.

## Roles

- **The coordinator** (the session reading this) writes nothing into the
  code itself. It launches lane agents, supervises them, verifies every
  acceptance claim against the tree, runs the fresh-eyes loop, opens the
  pull request from the lane's branch, keeps the stacks rebased, and
  writes the ledger. It asks Finch only the questions in "Stops" below.
- **The lane agent** implements one brief in one worktree, commits per
  entry, and writes the annotation file described under "The
  self-review": its own account of every change, in its own words, at the
  moment it made the change. The annotations are the implementer's
  perspective, never a reviewer's.
- **Fresh-eyes reviewers** are agents that have read neither the brief nor
  the lane's transcript. They review the diff before the pull request
  opens and their findings go back to the lane agent as repair items.
  Their reports never become the annotations; they improve the change the
  annotations describe.
- **Finch** reviews the pull request on GitHub, rules on stops, and
  merges. Nothing merges to `main` at anyone else's word.

## Identity

Pull requests, review comments, and pushes are made by a separate GitHub
identity provisioned for Claude (`GITHUB-APP-SETUP.md`), never by Finch's
account. Every pull request body opens with the CAVEAT LECTOR header. If
the token is missing, lanes still run and commit in their worktrees; no
pull request opens and no branch is pushed until it exists.

## One lane, start to finish

### 1. Launch

- Check the disk (`df -h /Volumes/forge /`) and trim dead caches before
  a wave of more than three builders.
- Create the worktree from the base the brief names:
  `git -C /Users/oxide/src/rumors worktree add ../rumors-<lane> -b
  <lane-branch> <base>`. Stacked lanes branch from the parent lane's
  branch, not from `main` (see "Stacks").
- Launch the lane agent with the brief's path, the worktree path, its
  scratchpad subdirectory, and the annotation-file requirement below. The
  brief carries every other ground rule; do not restate them, and do not
  add mechanism the brief lacks without also stating the goal it serves.

### 2. Implementation

The lane agent works per its brief. Two additions to the briefs as
written:

- **Annotation file.** Alongside its commits, the agent maintains
  `<worktree>/.agent-notes/<review>/triage/annotations/<lane>.tsv`
  (created by the agent, committed with the lane) with one row per
  changed region: `path`, `line` (in the final tree), `entry id`,
  `ruling`, and `note`. The note is the agent's own reason for the change
  at that site, written when it makes the change: what the entry claimed,
  what it did about it, and anything it would tell a reviewer standing at
  that line (a judgment call, an alternative it rejected, a nit it swept
  and why the sweep was the right shape). One row per nit too; a nit
  without a row is a change with no justification. This file is the
  source of the inline review comments, so it is written for Finch.
- **Commit granularity.** One commit per entry or per logical unit, the
  message naming the entry ids and the ruling, so the ledger's `sha`
  column and `git log` agree.

### 3. Acceptance verification

The lane's report is data. For every entry the report claims landed, the
coordinator runs the entry's Acceptance against the worktree at the
reported sha (the command and its decisive output, not the agent's
paraphrase), and records the result. An entry whose acceptance does not
hold goes back to the lane agent as a repair item, never into the ledger.
The negative controls the briefs require (the known-bad artifact failing
the repaired instrument) are re-run here, once.

### 4. Fresh-eyes review

Before the pull request opens, at least one fresh-eyes reviewer reads
the diff cold, with a brief that names the invariants the lane's entries
protect but not the resolutions, and with the instruction to dispute
rather than confirm. Escalate across rounds as the doctrine says: surface
correctness first, then operational validity, then interaction with the
rest of the tree, then the assumptions the change rests on. A round's
findings are classified by the coordinator:

- a defect in the change: back to the lane agent as a repair item, which
  lands as a further commit with its own annotation rows;
- a defect in the brief or a ruling: a stop (below);
- taste: recorded in the pull request body under "Reviewer notes not
  acted on", with the reason, never silently dropped.

Stop iterating when a round's findings shift from "this is wrong" to
"you might consider". Cap the rounds at three per lane; a lane that has
not converged by then is a finding about the lane, reported to Finch.

### 5. Opening the pull request

- Sign the outgoing commits if the identity's convention requires it
  (see `GITHUB-APP-SETUP.md`); push the branch with the Claude identity.
- Open the pull request as a draft with `gh pr create --draft --base
  <parent>` where `<parent>` is `main` for an independent lane and the
  parent lane's branch for a stacked one.
- Body, in this order: the CAVEAT LECTOR header; the goal in two or
  three sentences; the rulings landed, by number; the acceptance table
  (entry id, commit, acceptance command, decisive output); the fresh-eyes
  rounds (how many, what changed); "Reviewer notes not acted on"; stops,
  if any; the stack position (parent and children).
- Then the self-review: one review submitted with `gh api` as a single
  request whose `comments` array is generated from the annotation file,
  one comment per row, each prefixed with the entry id and ruling and
  carrying the agent's note verbatim. The review event is `COMMENT`;
  never `APPROVE`. Order the comments by risk: deviations from a stated
  resolution first, then new tests and their negative controls, then
  production edits, then prose, then nits grouped per file.
- Mark the pull request ready for review only when no stop is open. A
  pull request with an open stop stays a draft and carries the label
  `triage:stop` and a top comment naming the stop.

### 6. Review and merge

- Finch reviews on GitHub. His review comments are the next repair
  round: the coordinator relays them to the lane agent (or a successor
  with the brief and the annotation file), which lands the repairs as
  further commits with annotation rows, and the coordinator re-verifies
  acceptance for anything the repair touched.
- Merge is Finch's, and it is a rebase-merge (never a squash), so the
  per-entry commits and their shas survive onto `main`.
- After the merge: the coordinator writes each entry's `sha` into the
  ledger (the sha on `main`), runs `ledger.py check`, commits the ledger,
  rebases every child of the merged branch and retargets its pull request
  (see "Stacks"), and retires the worktree per the machine notes (resolve
  the forge build directory from inside the worktree, remove the
  worktree, delete both caches).

## Stacks

A lane that depends on another's commits branches from that lane's branch
and its pull request targets that branch. The dependencies are stated in
`briefs/README.md` (envelope on gate; collision mode on harness-crate; P2
codec on harness-crate; P2 peer on commit-path). Rules:

- A child never merges before its parent.
- When a parent merges, the coordinator rebases every child onto `main`
  the same day, force-pushes the child's branch (the branch is Claude's,
  so this is inside the standing authorization), and retargets the pull
  request to `main`. A stack is never more than one merge behind.
- A large lane splits into a stack of pull requests, one per commit group
  the brief's ordering already names (the conformance suite, the collision
  mode, the harness crate). Each pull request in the stack is reviewable
  on its own; the size cap is one logical unit of review, roughly a
  change Finch can read in one sitting.

## Stops

A stop is anything a ruling reserves for Finch: a moved `insta` snapshot
or committed pin, a public signature or public rustdoc contract change
the ruling did not name, a contradiction with a ruling, a deviation from
a stated resolution, a fresh-eyes finding that indicts the brief, or a
stop the brief itself names (the collision mode's four open questions,
for example). A stop is reported to Finch as a numbered block, each item
re-grounded with enough context to rule on without scrollback and with a
recommendation. The entry stays `open` in the ledger, the pull request
stays a draft with the `triage:stop` label, and nothing else in the lane
waits on it unless the brief says so.

## What never happens

- No prose authored by Claude posts under Finch's GitHub account.
- No pull request is approved or merged by Claude.
- No snapshot is re-accepted by Claude; a moved snapshot is a stop.
- No `defer` is written without a ruling naming its home (T4).
- No ledger `sha` is written from a report; only from a merged commit
  whose acceptance the coordinator ran.
- No worktree is force-removed; no cache is deleted before its forge
  directory has been resolved from inside the worktree.

## Resource discipline for the coordinator

At most four builders at once; check the disk before each wave. One
`just gate` run per lane commit series, backgrounded to a log under the
lane's scratchpad directory and polled, never a foreground demand.
Fresh-eyes rounds are reads, not builds; they do not run the suite.
Timing measurements a brief asks for are made once, load reported, never
iterated.
