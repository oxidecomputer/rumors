<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch and for the next before-triage coordinator session, at the end of the 2026-09-02/03 session; not authored, audited, or endorsed by Finch. Read with `WORKFLOW.md` (the procedure), `rulings.md` (the authority, rulings 1 to 114), `../../STATUS.md` (the dashboard), and `../../merge-queue.md` (the ordered record and the cross-plan rules). -->

# The coordinator's live journal (the before triage's landing)

This file is the coordinator's durable state, rewritten at every event
(a launch, an agent's report, a review round, a packet, a stop, a merge)
before the next action is taken. A coordinator session that has just
compacted, or a fresh one, reads it first and resumes from it; nothing
that matters lives only in a transcript.

## Recovery procedure (read first after a compaction or a restart)

1. Read `WORKFLOW.md`, then `rulings.md` from ruling 107 on, then this
   file whole, then `../../STATUS.md` and `../../merge-queue.md`.
2. Find the running agents: `ListAgents` lists this session's subagents
   by id; the map from id to lane and last instruction is
   `<scratchpad>/coordinator/agents.tsv` (the scratchpad is
   `/private/tmp/claude-506/-Users-oxide-src-rumors/836822ce-635e-48a7-b2e5-196967bded33/scratchpad`).
   An agent that is `running` is working; one that has completed with a
   "waiting on the monitor" sign-off is parked on a background run and
   needs waking: find its wrapper with `pgrep -f "on-illumos.sh
   /Users/oxide/src/before-<lane>"`, arm a background wait on that pid
   (`while kill -0 $P; do sleep 20; done`), and when it exits send the
   agent the verdict lines from its log under `<scratchpad>/<lane>/`.
3. Every lane's branch state is in git: `git -C /Users/oxide/src/before-<lane>
   log --oneline main..HEAD` and `git status`. The per-lane table below
   says what each is awaiting; the agent's own report, if it landed, is
   the last message in its task output file
   (`<scratchpad>/../tasks/<agent id>.output`, a JSONL transcript: read
   only its last entry, never the whole file).
4. The rumors coordinator session is `rumors-6e` (`ListAgents` peers);
   the cross-plan rules are the queue header; message it only for the
   three events rule 7 names.
5. Then continue the per-lane "next" column below. When in doubt, a stop
   goes into the morning brief rather than a guess.

## Overnight rules (Finch's authorization of 2026-09-03, evening)

- No merge to `main`, no push, no snapshot re-accept, no re-pin outside a
  ruling's explicit sanction, no root-file edit beyond what a brief names.
- Any deviation from a stated Resolution is a stop, never a coordinator
  acceptance; any moved pin, snapshot, rendered panel, or public
  contract is a stop. A stop halts its lane only; it goes into the
  morning brief as a numbered item with a recommendation.
- Three review rounds per lane, then the lane is a finding. At most four
  lanes building at once; the box load watched (hold above ~150).
- Lanes in scope tonight: `p1-board` (from `main`), `p1-fuzz` and
  `p1-survivors` (stacked on the gate lane's final tip), plus review
  rounds, repairs, and packets for those and for the gate lane. Nothing
  else launches.
- This journal, `STATUS.md`, `agents.tsv`, and the queue are updated
  before the next action at every event.

## Rumors' root-file lines tonight (from its session, for rebases)

- `p1-proptest-ci` (on its branch): justfile `proptest_ci_cases`,
  `test-release`, `all`'s line; `.config/nextest.toml` release profile;
  ci.yml proptest job; `tools/caselint` (new).
- `p1-generators` (running): justfile `test`, `test-all`, `test-release`
  recipe lines gain the two zero-reject variables; ci.yml test jobs
  likewise; `tools/caselint` extended. Note: before's gate lane touched
  the `test` recipe's comment (a blank line for `just --list`), so that
  rebase may conflict on adjacent lines; the coordinator resolves it.
- `p1-collision-mode`: a `test-collision` recipe and one `ci` line; a
  root `AGENTS.md` paragraph.
- `p2-commit-path`: README.md regenerated. None touch the fuzz or
  fuzzfit workspaces, their lockfiles, or the `fuzz-build`/`fuzzfit`
  recipe lines.

## Morning brief for Finch

(rewritten as the night proceeds: packets ready in merge order; stops as
a numbered block with recommendations; findings routed; judgment calls
flagged)

- Packets ready, in merge order: `p2-widths`, `p2-surface` (one stop:
  the `fuelscape.js` caption), `p2-generators` (any order).
- Stops: none new yet.

## What to read first

`WORKFLOW.md`, then `rulings.md` from ruling 107 on (the cross-plan rulings made while landing), then `briefs/README.md`, then this file. The rumors triage runs in the same workspace from its own session; every rule shared with it is in `../../merge-queue.md`'s header and is binding.

## Merged to main

- `p1-proptest-cases` (`e27ba5e6`, ruling 109) and `p1-harness` (`5b0a17d4`, rulings 4, 38, 50, 51, 108): both retired, ledger shas written.

## Lanes with worktrees, in the order they should reach Finch

Worktrees under `/Users/oxide/src/before-<lane>`, branches `before/<lane>`, all rebased onto a recent `main`; every commit made while the 1Password agent was refusing is unsigned and is re-signed by the merge step's rebase (`--exec 'git commit --amend --no-edit --no-verify -S'`). The merge step also compiles the rebased tip (default and all features) before any fast-forward (`WORKFLOW.md`, "The merge step's last check").

1. **`p2-widths`** (tip `e0af2efb`, 51 commits, rulings 33, 34, 39, 41, 104, 109, 110): complete after three repair rounds; the coordinator's acceptance runs all pass; the coordinator's gate of record at the tip is green but for `fuzz` (`<scratchpad>/p2-widths/coord-gate-final.log`), both feature-set compiles pass, and 51 commits are identical under the rebase. Packet meta is drafted and filled at `<scratchpad>/p2-widths/meta.md` (acceptance table and rounds written). The lane agent completed annotation coverage at Finch's word (206 rows, 0 unannotated, 0 stale, verified by the coordinator), and the packet is built, rendered (the coordinator scratchpad's `p2-widths.html`), and listed in the queue ahead of the surface packet: packet head `ba36e970`. Two items in its "For Finch's eye" section (the bridge's third door; party-23's table choice). Its one justfile hunk (`fuzz-test`) and `p1-gate`'s justfile edits rebase over each other.
2. **`p2-surface`** (tip `8a2aa4d7`, 44 commits, rulings 42, 44, 46, 47, 48, 49, 98, 104, 105, 109, 110 part 1): complete after three rounds plus a scoped fourth on the visitor, which did not converge in three (recorded for the packet's rounds section, already written in `<scratchpad>/p2-surface/meta.md` with the acceptance table, the rendering section for ruling 89, and one stop: the `fuelscape.js` x-axis caption still says "total input bytes"). The coordinator's gate of record at the tip is green but for `fuzz`, both feature-set compiles pass, and every dump and dataset was verified to move only in `size_measure` (two also in `contract`). The lane agent completed annotation coverage at Finch's word (334 rows, 0 unannotated, 0 stale, verified by the coordinator), and the packet is built, rendered (the coordinator scratchpad's `p2-surface.html`), and listed in the queue: packet head `cad0b75d`. Its registry step (ruling 43) is held for a second launch after `p1-suites`.
3. **`p1-gate`** (tip `0335b7fd`, 21 commits on the merged rumors gate lane, rulings 16, 18, 24, 25, 28, 50, 106, 107, 112, 114): the agent was mid-task when the session paused, on ruling 114: bisecting the fuzzfit lock's sweep to confirm `bytes` alone moved the guest's fuel, converging every swept crate upward (root `Cargo.lock` brought to the newest version any lock resolved: bytes 1.12.1 and the rest), explaining the mechanism Finch asked for ("what would have changed it?"), then its gate. Its first review round's repairs are landed. Next: read its report; run a second fresh-eyes round; the coordinator's gate at its tip; the packet. Detached scratch worktrees of its own sit under `<scratchpad>/p1-gate/` (`before-p1-gate-presweep`, `before-p1-gate-calib`, `before-p1-gate-precodec`, `bisect`): remove each with `git worktree remove` (or `git worktree prune` after deleting the directories) and delete their `~/src/` and `~/build/` twins on the box. Its last instruction (sent as the session paused) was to finish the upward convergence, the four verification legs, and the one gate, then report; the generators lane's band finding is to be read into its mechanism paragraph. If its report did not land, resume it with that message's content.
4. **`p2-generators`** (tip `41eb5857`, 11 commits, rulings 111, 113): complete; the four spellings return nothing; the whole before crate runs at 4000 cases under zero reject budgets with no failure; packet built over the widening and listed in the queue (packet head `e84a74a3`, rendered at the coordinator scratchpad's `p2-generators.html`). Its runs surfaced the `ff_party_decode` band finding (in `new-findings.md`, routed to `p1-fuzz`), which reframes the gate lane's fuel stop: the band is too tight for the family, and the bytes downgrade only exposed it at the default count.

## Live state of the overnight run (rewritten at every event; last 2026-09-03 03:55 UTC)

| Lane | Branch tip | Agent state | Awaiting | Next |
|---|---|---|---|---|
| p1-board | `2fe750ac` (base main, launched 2026-09-03 03:55 UTC) | running (launch message sent) | its report | acceptance re-runs, fresh-eyes round 1, repairs, gate, packet |
| p1-gate | see above | parked on its final verification chain (a background wait is armed in this session; after a compaction, re-arm per the recovery procedure) | its final report: upward convergence, four legs, one gate, the bytes mechanism | rebase, second fresh-eyes round, coordinator gate, packet |
| p1-fuzz | not created | not launched | the gate lane's final tip (stacks on it) | worktree from that tip; launch with the brief plus the handoffs in its brief and the band refit (new-findings.md) |
| p1-survivors | not created | not launched | the gate lane's final tip (stacks on it) | worktree from that tip; launch |
| p2-widths, p2-surface, p2-generators | packets ready | done | Finch's word | merge in the morning |

## What comes next, in order

- Wake or relaunch the two mid-task agents (gate, generators) if their reports did not land; a relaunch reads the brief, the launch messages in this session's transcript are the pattern (worktree, base, box mechanism through the wrapper with `CARGO_BUILD_JOBS=32 NEXTEST_TEST_THREADS=32`, no `pset-run`, annotations, in-turn polling).
- Packets for widths and surface once their gates read green but for `fuzz`; then Finch reads; merge on his word, widths first (surface's `Cargo.toml`/`Cargo.lock` root touches and the gate lane's lockfile touches rebase over each other).
- After `p1-gate` merges: launch `p1-fuzz` stacked on it (its brief carries three handoffs: the fuzz crate's test recipe now exists as `fuzz-test` from the widths lane, so fold it into the ruling-15 legs; the `call1_with_panic` channel; the guest-closure provenance pin from ruling 114), then `p1-survivors`; after widths and harness: `p1-board`, then `p1-suites`, then `p2-rows` (its brief carries the JOIN_WIDE_TOOTH attribution, the deep rows, and the meter suite's dev-profile statement), then `p2-cures`.
- `new-findings.md` indexes every finding without a ledger row and where it went.

## Open questions and stops for Finch

- The mechanism behind the fuel movement (`bytes` 1.11.1 vs 1.12.1 on the party decode path): owed by the gate lane's report, and read against the generators lane's finding that the `ff_party_decode` band breaches on main's own tree at 4000 cases (the band, not the dependency, is the root cause; the dependency shifted the constant).
- `fuelscape.js`'s x-axis caption under ruling 110 (the surface packet's stop).
- Twenty-two unsigned note-only commits on `main` ahead of `origin/main` (both sessions', signing outages); a re-signing rewrite of `main` before any push, announced in the queue with the other session paused.
- gate-legs-1 (ruling 24's coverage reproduction) waits for CI's next red (ruling 112).
- Whether the three memory terminals should all pin the `(Trapped, None)` genre (only rank does; a fuzz-lane item).

## Machine notes

- The box syncs through git (ruling 112; the skill's wrapper pushes HEAD and checks it out, then rsyncs the working tree). Gates run in the general pool with the two caps; `pset-run` only for wall-time measurements. The `fuzz` leg is expected red on the box (no libFuzzer port) and counts as clean when sole.
- Box directories to retire when their lanes merge: `~/src/before-<lane>` and `~/build/before-<lane>` for p1-gate, p2-widths, p2-surface, p2-generators, plus the presweep pair above. Retiring a Mac worktree resolves its forge build directory first (none of these lanes built on the Mac).
- The two shared note files (`STATUS.md`, `merge-queue.md`) are edited only when clean, edit and commit in one command (queue rule 5).
