<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch, from the triage sessions of 2026-09-02, as the operating procedure for landing the rulings in triage/rulings.md through local review packets; not authored, audited, or endorsed by Finch. Read with the ground rules in ../../README.md. -->

# Landing the triage through local review packets

This is the procedure a fresh session follows to turn the rulings in
`rulings.md` into merged code, one lane at a time, with Finch reviewing
every change locally, in Zed, without anything leaving the machine. It is
written to be handed to a session that has read nothing else: the first
section says what to read, the rest says what to do. The briefs under
`briefs/` are the per-lane inputs; this document is the loop around them.

## What to read first

1. `TRIAGE.md`: the phases, the dispositions, and the ledger's columns.
2. `rulings.md`, whole: every decision Finch made, numbered. A brief
   cites rulings by number; the ruling is the authority when the two
   disagree.
3. `briefs/README.md`: the lanes, their base, their independence, and the
   launch order. Then the brief of the lane being run.
4. `review.py --help`: the packet tool this procedure uses.
5. `ledger.py summary` and `ledger.py check`: the state of the record. A
   row is pending until its `sha` column names a commit on `main`.

## Roles

- **The coordinator** (the session reading this) writes nothing into the
  code itself. It launches lane agents, supervises them, verifies every
  acceptance claim against the tree, runs the fresh-eyes loop, builds the
  review packet from the lane's branch, relays Finch's replies as repairs,
  keeps the stacks rebased, and writes the ledger. It asks Finch only the
  questions in "Stops" below.
- **The lane agent** implements one brief in one worktree, commits per
  entry, and writes the annotation file described under "Annotations":
  its own account of every change, in its own words, at the moment it
  made the change. The annotations are the implementer's perspective,
  never a reviewer's.
- **Fresh-eyes reviewers** are agents that have read neither the brief nor
  the lane's transcript. They review the diff before the packet is built,
  and their findings go back to the lane agent as repair items. Their
  reports never become the annotations; they improve the change the
  annotations describe.
- **Finch** reads the packet, replies inline, rules on stops, and merges.
  Nothing merges to `main` at anyone else's word.

## Annotations

Alongside its commits, the lane agent maintains
`.agent-notes/<review>/triage/annotations/<lane>.tsv` in its worktree,
committed with the lane, with one row per changed region:

    path <TAB> line <TAB> entry id <TAB> ruling <TAB> note

`line` is a line number in the final tree of the lane (the new side of
the diff); a deletion is annotated at the line that follows it, and a
row at line 0 annotates every hunk of its file (a deleted file, a
regenerated lockfile). The note
is the agent's own reason for the change at that site, written when it
makes the change: what the entry claimed, what it did about it, and
anything it would tell a reviewer standing at that line (a judgment call,
an alternative it rejected, a nit it swept and why the sweep was the
right shape). One row per nit too; a nit without a row is a change with
no justification. The file is written for Finch: plain English, no
roster tags beyond the entry id and ruling. A note may span lines only
as one TSV field (use `\n` literally; the tool unescapes it).

## The packet

`review.py packet` renders one branch's changes against its base into
`triage/reviews/<lane>.md`, committed on the lane branch:

1. A header (CAVEAT LECTOR), the goal in two or three sentences, the
   rulings landed by number, and the stack position (parent and
   children). The coordinator writes these from the brief.
2. The acceptance table: entry id, commit, the acceptance command, and
   its decisive output, one row per entry the lane claims. Every row is
   one the coordinator ran itself.
3. The fresh-eyes rounds: how many, what each changed, and "Reviewer
   notes not acted on" with the reason for each.
4. Stops, if any, as a numbered block with recommendations.
5. The annotated diff: every hunk of `git diff <base>...<head>`, in file
   order, and under each hunk the annotation rows that fall inside it,
   each as a comment block naming the entry id, the ruling, and the note.
   Hunks with no annotation are flagged `(no annotation)`, which is a
   defect the coordinator sends back before the packet goes to Finch.

Regions ordered by risk are the reader's first pass: the packet's table
of contents lists deviations from a stated resolution first, then new
tests and their negative controls, then production edits, then prose,
then nits grouped per file, each entry linking to its hunk.

Finch reviews the packet in Zed and replies inline: a line beginning
`>> finch:` anywhere under a hunk or a section. `review.py replies`
extracts every reply with its hunk and the annotation it sits under, as a
numbered repair list the coordinator hands to the lane agent. A reply
that rules on a stop is transcribed into `rulings.md` by the coordinator
in the same turn.

## One lane, start to finish

### 1. Launch

- Check the disk (`df -h /Volumes/forge /`) and trim dead caches before
  a wave of more than three builders.
- Create the worktree from the base the brief names:
  `git -C /Users/oxide/src/rumors worktree add ../rumors-<lane> -b
  <lane-branch> <base>`. Stacked lanes branch from the parent lane's
  branch, not from `main` (see "Stacks").
- Lane commits use the repository's default identity and signing, like
  every other commit; nothing is configured per worktree or per lane.
- Launch the lane agent with the brief's path, the worktree path, its
  scratchpad subdirectory, and the annotation requirement above. The
  brief carries every other ground rule; do not restate them, and do not
  add mechanism the brief lacks without also stating the goal it serves.

### 2. Implementation

The lane agent works per its brief: one commit per entry or per logical
unit, the message naming the entry ids and the ruling, so the ledger's
`sha` column and `git log` agree; the annotation file kept current with
every commit.

### 3. Acceptance verification

The lane's report is data. For every entry the report claims landed, the
coordinator runs the entry's Acceptance against the worktree at the
reported sha (the command and its decisive output, not the agent's
paraphrase), and records the result for the packet. An entry whose
acceptance does not hold goes back to the lane agent as a repair item,
never into the packet's table as landed. The negative controls the briefs
require (the known-bad artifact failing the repaired instrument) are
re-run here, once.

### 4. Fresh-eyes review

Before the packet is built, at least one fresh-eyes reviewer reads the
diff cold, with a brief that names the invariants the lane's entries
protect but not the resolutions, and with the instruction to dispute
rather than confirm. Escalate across rounds as the doctrine says: surface
correctness first, then operational validity, then interaction with the
rest of the tree, then the assumptions the change rests on. A round's
findings are classified by the coordinator:

- a defect in the change: back to the lane agent as a repair item, which
  lands as a further commit with its own annotation rows;
- a defect in the brief or a ruling: a stop (below);
- taste: recorded in the packet under "Reviewer notes not acted on",
  with the reason, never silently dropped.

Stop iterating when a round's findings shift from "this is wrong" to
"you might consider". Cap the rounds at three per lane; a lane that has
not converged by then is a finding about the lane, reported to Finch.

### 5. The packet goes to Finch

- Build it: `review.py packet --base <parent> --head <lane-branch>
  --annotations <tsv> --meta <meta.md> --out triage/reviews/<lane>.md`,
  where `meta.md` holds the coordinator-written sections (header, goal,
  rulings, acceptance table, rounds, stops, stack). Commit it on the lane
  branch.
- Tell Finch the branch, the packet path, and the risk-ordered table of
  contents. Open the packet in Zed for him if in a narrated session (one
  file at a time).
- A lane with an open stop is announced as such; its packet carries the
  stop block at the top and the lane is not offered for merge until the
  stop is ruled.

### 6. Review, repair, merge

- Finch replies in the packet. The coordinator runs `review.py replies`,
  hands the list to the lane agent (or a successor with the brief and
  the annotation file), which lands the repairs as further commits with
  annotation rows. The coordinator re-verifies acceptance for anything a
  repair touched, rebuilds the packet, and shows Finch `git range-diff
  <base> <old-head> <new-head>` so the round's change is exact.
- Merge happens at Finch's explicit word, per lane, after he has read
  the packet; the coordinator runs it: a rebase of the lane branch onto
  `main`, then a fast-forward of `main`, so the per-entry commits
  survive. Never a squash. A lane commit that lacks a signature or
  carries a stray identity is amended in the same rebase
  (`--exec 'git commit --amend --no-edit --reset-author -S'`).
- After the merge: the coordinator writes each entry's `sha` into the
  ledger (the sha on `main`), runs `ledger.py check`, commits the ledger,
  rebases every child of the merged branch (see "Stacks"), and retires
  the worktree per the machine notes (resolve the forge build directory
  from inside the worktree, remove the worktree, delete both caches).

## Stacks

A lane that depends on another's commits branches from that lane's branch
and its packet is diffed against that branch. The dependencies are stated
in `briefs/README.md` (envelope on gate; collision mode on harness-crate;
P2 codec on harness-crate; P2 peer on commit-path). Rules:

- A child never merges before its parent.
- When a parent merges, the coordinator rebases every child onto `main`
  the same day and rebuilds its packet against `main`. A stack is never
  more than one merge behind.
- A large lane splits into a stack of packets, one per commit group the
  brief's ordering already names (the conformance suite, the collision
  mode, the harness crate). Each packet is reviewable on its own; the
  size cap is one logical unit of review, roughly a change Finch can read
  in one sitting.

## Stops

A stop is anything a ruling reserves for Finch: a moved `insta` snapshot
or committed pin, a public signature or public rustdoc contract change
the ruling did not name, a contradiction with a ruling, a deviation from
a stated resolution, a fresh-eyes finding that indicts the brief, or a
stop the brief itself names (the collision mode's four open questions,
for example). A stop is reported to Finch as a numbered block, each item
re-grounded with enough context to rule on without scrollback and with a
recommendation, both in the packet and in the message that announces it.
The entry stays `open` in the ledger and nothing else in the lane waits
on it unless the brief says so.

## What never happens

- Nothing is pushed to any remote by Claude under this procedure; the
  review record is the packet and the branch, on this machine.
- No lane is merged without Finch's explicit, per-lane word after he
  has read its packet.
- No snapshot is re-accepted by Claude; a moved snapshot is a stop.
- No `defer` is written without a ruling naming its home (T4).
- No ledger `sha` is written from a report; only from a commit on `main`
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

## Effort and orchestration

Lane agents that touch a harness or production code (all of P1 and P2,
the API pass, the commit path) run at high effort: the failure mode of a
cheap agent there is a plausible test that passes for the wrong reason.
Fresh-eyes reviewers run at high effort too; a reviewer below the
author's effort confirms rather than disputes. Mechanical lanes whose
oracle is a regenerating grep or a deletion (the swarm and envelope
deletions, the em-dash, vocabulary, and import sweeps, the retired-prose
sweeps) may run at medium. The coordinator stays at high.

Lanes are launched with the Agent tool in waves of at most four, never
as a scripted workflow: a lane's defining event is a stop that needs
Finch's ruling mid-lane, which a background script cannot pause for, and
per-lane supervision (early intervention, verification by artifact,
relaying findings between concurrent lanes) belongs to the coordinator.
A scripted workflow fits only fixed-shape fan-outs with no stops
expected: several fresh-eyes reviewers over one packet with escalating
briefs, merged mechanically, or a P3 sweep whose acceptance is a grep
count. Their merged output is still verified against the diff before it
reaches Finch.
