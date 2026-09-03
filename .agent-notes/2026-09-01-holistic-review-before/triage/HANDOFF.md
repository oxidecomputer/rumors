<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch and for the next before-triage coordinator session, at the end of the 2026-09-02/03 session; not authored, audited, or endorsed by Finch. Read with `WORKFLOW.md` (the procedure), `rulings.md` (the authority, rulings 1 to 114), `../../STATUS.md` (the dashboard), and `../../merge-queue.md` (the ordered record and the cross-plan rules). -->

# Handoff: the before triage's landing, end of the first session

## What to read first

`WORKFLOW.md`, then `rulings.md` from ruling 107 on (the cross-plan rulings made while landing), then `briefs/README.md`, then this file. The rumors triage runs in the same workspace from its own session; every rule shared with it is in `../../merge-queue.md`'s header and is binding.

## Merged to main

- `p1-proptest-cases` (`e27ba5e6`, ruling 109) and `p1-harness` (`5b0a17d4`, rulings 4, 38, 50, 51, 108): both retired, ledger shas written.

## Lanes with worktrees, in the order they should reach Finch

Worktrees under `/Users/oxide/src/before-<lane>`, branches `before/<lane>`, all rebased onto a recent `main`; every commit made while the 1Password agent was refusing is unsigned and is re-signed by the merge step's rebase (`--exec 'git commit --amend --no-edit --no-verify -S'`). The merge step also compiles the rebased tip (default and all features) before any fast-forward (`WORKFLOW.md`, "The merge step's last check").

1. **`p2-widths`** (tip `e0af2efb`, 51 commits, rulings 33, 34, 39, 41, 104, 109, 110): complete after three repair rounds; the coordinator's acceptance runs all pass; the coordinator's gate of record at the tip is green but for `fuzz` (`<scratchpad>/p2-widths/coord-gate-final.log`), both feature-set compiles pass, and 51 commits are identical under the rebase. Packet meta is drafted and filled at `<scratchpad>/p2-widths/meta.md` (acceptance table and rounds written). The lane agent completed annotation coverage at Finch's word (206 rows, 0 unannotated, 0 stale, verified by the coordinator), and the packet is built, rendered (the coordinator scratchpad's `p2-widths.html`), and listed in the queue ahead of the surface packet: packet head `ba36e970`. Two items in its "For Finch's eye" section (the bridge's third door; party-23's table choice). Its one justfile hunk (`fuzz-test`) and `p1-gate`'s justfile edits rebase over each other.
2. **`p2-surface`** (tip `8a2aa4d7`, 44 commits, rulings 42, 44, 46, 47, 48, 49, 98, 104, 105, 109, 110 part 1): complete after three rounds plus a scoped fourth on the visitor, which did not converge in three (recorded for the packet's rounds section, already written in `<scratchpad>/p2-surface/meta.md` with the acceptance table, the rendering section for ruling 89, and one stop: the `fuelscape.js` x-axis caption still says "total input bytes"). The coordinator's gate of record at the tip is green but for `fuzz`, both feature-set compiles pass, and every dump and dataset was verified to move only in `size_measure` (two also in `contract`). The lane agent completed annotation coverage at Finch's word (334 rows, 0 unannotated, 0 stale, verified by the coordinator), and the packet is built, rendered (the coordinator scratchpad's `p2-surface.html`), and listed in the queue: packet head `cad0b75d`. Its registry step (ruling 43) is held for a second launch after `p1-suites`.
3. **`p1-gate`** (tip `0335b7fd`, 21 commits on the merged rumors gate lane, rulings 16, 18, 24, 25, 28, 50, 106, 107, 112, 114): the agent was mid-task when the session paused, on ruling 114: bisecting the fuzzfit lock's sweep to confirm `bytes` alone moved the guest's fuel, converging every swept crate upward (root `Cargo.lock` brought to the newest version any lock resolved: bytes 1.12.1 and the rest), explaining the mechanism Finch asked for ("what would have changed it?"), then its gate. Its first review round's repairs are landed. Next: read its report; run a second fresh-eyes round; the coordinator's gate at its tip; the packet. A detached worktree of its own sits at `<scratchpad>/p1-gate/before-p1-gate-presweep` (remove with `git worktree remove`; its box directories `~/src/before-p1-gate-presweep` and `~/build/before-p1-gate-presweep` too).
4. **`p2-generators`** (tip `8d7b3175` plus uncommitted work, rulings 111, 113): the agent was mid-sweep of the seven `prop_filter` sites when the session paused (its worktree carried six uncommitted files; its earlier packet at `8d7b3175` is withdrawn and is rebuilt after the widened sweep). Next: its report, a light coordinator read, packet.

## What comes next, in order

- Wake or relaunch the two mid-task agents (gate, generators) if their reports did not land; a relaunch reads the brief, the launch messages in this session's transcript are the pattern (worktree, base, box mechanism through the wrapper with `CARGO_BUILD_JOBS=32 NEXTEST_TEST_THREADS=32`, no `pset-run`, annotations, in-turn polling).
- Packets for widths and surface once their gates read green but for `fuzz`; then Finch reads; merge on his word, widths first (surface's `Cargo.toml`/`Cargo.lock` root touches and the gate lane's lockfile touches rebase over each other).
- After `p1-gate` merges: launch `p1-fuzz` stacked on it (its brief carries three handoffs: the fuzz crate's test recipe now exists as `fuzz-test` from the widths lane, so fold it into the ruling-15 legs; the `call1_with_panic` channel; the guest-closure provenance pin from ruling 114), then `p1-survivors`; after widths and harness: `p1-board`, then `p1-suites`, then `p2-rows` (its brief carries the JOIN_WIDE_TOOTH attribution, the deep rows, and the meter suite's dev-profile statement), then `p2-cures`.
- `new-findings.md` indexes every finding without a ledger row and where it went.

## Open questions and stops for Finch

- The mechanism behind the fuel movement (`bytes` 1.11.1 vs 1.12.1 on the party decode path): owed by the gate lane's report.
- `fuelscape.js`'s x-axis caption under ruling 110 (the surface packet's stop).
- Twenty-two unsigned note-only commits on `main` ahead of `origin/main` (both sessions', signing outages); a re-signing rewrite of `main` before any push, announced in the queue with the other session paused.
- gate-legs-1 (ruling 24's coverage reproduction) waits for CI's next red (ruling 112).
- Whether the three memory terminals should all pin the `(Trapped, None)` genre (only rank does; a fuzz-lane item).

## Machine notes

- The box syncs through git (ruling 112; the skill's wrapper pushes HEAD and checks it out, then rsyncs the working tree). Gates run in the general pool with the two caps; `pset-run` only for wall-time measurements. The `fuzz` leg is expected red on the box (no libFuzzer port) and counts as clean when sole.
- Box directories to retire when their lanes merge: `~/src/before-<lane>` and `~/build/before-<lane>` for p1-gate, p2-widths, p2-surface, p2-generators, plus the presweep pair above. Retiring a Mac worktree resolves its forge build directory first (none of these lanes built on the Mac).
- The two shared note files (`STATUS.md`, `merge-queue.md`) are edited only when clean, edit and commit in one command (queue rule 5).
