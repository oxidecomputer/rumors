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
  `p1-survivors` (stacked on the gate lane's final tip), and, at Finch's
  word in the night (ruling 115), `p8-tagwalk` (stacked on the same tip;
  the fuzz lane holds its calibration until it has rebased onto the
  tagwalk tip), plus review rounds, repairs, and packets for those and
  for the gate lane. Nothing else launches.
- This journal, `STATUS.md`, `agents.tsv`, and the queue are updated
  before the next action at every event.
- Every `just gate` on the box runs under a shared mutex directory (`mkdir ~/gate.lock` in a retry loop, a lock older than 45 minutes removed as a dead gate's, `rmdir` on exit, all in one remote command), agreed with the rumors session after three gates launched into one second and drove the load to 460; targeted runs and builds stay outside it. The rumors session writes it into queue rule 6.
- Shared note files (`merge-queue.md`, `STATUS.md`) are edited and committed under a mutex directory `.agent-notes/.editlock` (`mkdir` in a retry loop, a lock older than five minutes removed, `rmdir` on exit, all in one command with the clean check inside), agreed with the rumors session after a clean-check race carried its hunk in a before commit; on macOS the lock's age is `stat -f %m`.
- Around 08:00 UTC the account ran out of usage credits: every running agent (the board lane mid-poll, the fuzz reviewer mid-read) died with HTTP 429 and no report; Finch resolved it in the afternoon and asked for the agents to be revived, which a `SendMessage` to each dead agent's id does (it resumes from its transcript). Nothing in any branch was lost: both trees were clean at their tips.
- The 1Password signing agent hangs rather than refuses tonight (a
  `git commit -S` blocks until the command timeout), so every note commit
  for the rest of the night is made with `--no-gpg-sign` under a
  `timeout`, and all of them join the re-signing rewrite of `main` the
  morning brief lists. Lane commits are re-signed at their merge rebase.

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

(rewritten as the night proceeds)

- Packets ready, in merge order: `p2-widths`, `p2-surface` (one stop:
  the `fuelscape.js` caption), `p2-generators` (any order), then
  `p1-gate`, then its children `p8-tagwalk` and `p1-survivors` (packet head `f9ec31d7`) (packet head `3316c4ed`), then
  `p1-gate` (packet head `75e0bc91`; one stop: the syn 2/3 roster).
- Merge-order coupling with the rumors plan (2): `p1-fuzz` (child of p1-gate via p8-tagwalk) merges before the rumors `p1-collision-mode` lane, whose branch carries an eleventh fork-chain seed (`cc d6731723`, 152 bits, in the refit set now) that reads red until the band refit lands. A twelfth (`cc a072497c`, 136 bits, 13767 fuel against the old band; from the rumors `p2-vanish-liveness` gate, 2026-09-03 afternoon) was handed to the fuzz lane to add and replay against the refit bands; the gate mutex on the rumors side now carries `~/gate.lock/pid`, so a lock whose pid is dead is stale before the 45-minute rule.
- Merge-order coupling with the rumors plan: `p2-generators` merges before the rumors `p1-generators` lane's last commit (the zero reject budgets on the gate's test recipes and CI), since before's pre-widening sites abort under those budgets; the widened lane's whole-crate run at 4000 cases under zero budgets had no rejection abort. The two long properties that hit nextest's 180 s limit at 4000 cases are `p1-suites`' (new-findings.md).
- Stops (numbered):
  1. **`syn` 2 and 3 in the root lock after the upward convergence.** serde_derive 1.0.229 and thiserror-impl 2.0.20 (the newest) require syn 3; async-stream-impl, pollster-macro, and tracing-attributes (each the newest of its line) still require syn 2; all build-time proc-macro edges. cargo deny bans the duplicate, so the gate lane's audit stream is red for this alone. Options: (a) roster `syn@2` in deny.toml naming the three holdouts (deps-2's own design for a duplicate that cannot converge yet; verified `bans ok`; the entry reads unmatched when the holdouts move); (b) hold serde_derive at 1.0.228 and thiserror-impl at 2.0.18. Recommendation: (a), the coda names the direction and the duplicate is build-time only.
  3. **Ruling 11's residual fit does not work as ruled (board lane).** Built and measured: with a materiality fraction of 1/8 the residual trend flips the gap tripwire in both windows, and on the release board it turns 588 of 2096 committed cells red, all "heap exponent" and only those. Mechanism: a linear cell reading `a + c·n` with its fixed term `a` under the allowance leaves a residual `c·n − (8192 − a)` whose log-log trend is steeper than 1 by construction; the trend stays under 1.15 only while the shortfall is at most about 0.4 of the smallest fitted residual, and the board's heap cells sit far outside that. Options: (a) judge the heap trend by an intercept-fitting (affine) model, which is ruling 73's leg (currently placed in `p4-ghosts`), and drop the residual trend; (b) re-derive the allowance so the fixed term is measured per cell; (c) a materiality fraction near 1, which is tuning, not a guarantee. Recommendation: (a), landed in the board lane's successor rather than `p4-ghosts`. The gap tripwire stays committed asserting today's verdict; the fit sits on `before/p1-board-residual-fit` (`cedeb289`).
  6. **`span_dominance`'s `WORST_RANKINGS` rows moved when its probe was corrected** (the dominance kernel dropped the end stream under the old probe, so the row's touch floor rested on a false premise; probing with the span's ceiling forces the full sweep). New argmaxes: touch staircase 2.39/B to dense 5.33/B at both scales, scan promo-rearm to dense-suffix (an 8.00/B tie) at the default scale, heap hugeleaf's reading 1.06/1.19 to 1.00/B (argmax unchanged). These are the honest readings for the row; only the two ruling-12 rows were allowed to move by design, so the lane did not re-pin. Recommendation: re-pin the two dominance rows with this attribution; the board stream reads red on the drift until then.
  7. (being resolved in the third round: no under edge, per-row over-side ceilings committed at the parent's readings with attribution; returns here only if a tick cell still reads red) **The tick rows' "2 to 5x kept calibration" was never what the release board reads.** A committed check of the band (carried on the cell, judged) reads 5 cells red at the default scale and 6 at the top: `version_tick` and `clock_tick` on nested-wide at 1.78x (under), `version_ticks` on reveal-comb 5.05x, reveal-hifloor 5.00x, dominated-undercut 6.96x, comb-scatter 5.22x (over). Not widened. Options: re-derive the band from what each walk records (the `ticks` row's count loop reads more; nested-wide's wide payloads raise the live-bits floor), or state the calibration per row (`version_tick`/`clock_tick` versus `version_ticks`) rather than one band for the trio. The lane holds until ruled; the board stream reads red on these cells meanwhile.
  4. **Confirmations from the board lane** (each landed with a stated reason; reverse any you read differently): `causally_contains` oriented by verdict (the covered operand is the probe) rather than the literal `since(&w).contains(&v)`, so dense-suffix keeps certifying; two concurrent pairings (jump-pair, concurrent-pair) keep root-code floors, so "every family" is met on 27 of 29; `party_covers` uses the re-decode twin rather than `join(a, b).covers(&b)`; meter-adequacy-7's scan-reading evidence is structurally impossible (the compare is below every metered primitive), the bench mirror being the only witness; the determinism dive's tick leg (a compile-time zero) dropped; `query_coverage` keeps a positive touch not-applicable because its walk legitimately stops early.
  5. **board-ops-render-19's justfile and CI half** (the `board` stream runs `amp-board-acceptance` alone since the pin is checked inside it; the `worst-cases-pin` CI step deleted; the recipe a view) is the gate lane's file this wave: a small follow-up commit on `before/p1-gate` or the next justfile owner. The justfile's board comment still lists segments among the counters (same owner).
  2. (resolved under the coda: the four crates came up in every lock at `6a4d255e`, the meter suite's 302 MEASURED lines unchanged) **Four crates the first sweep lowered outside the fuzzfit lock**: dashu-int and dashu-base 0.5.1 to 0.5.0, borsh 1.8.0 to 1.6.1, num-modular 0.6.5 to 0.6.4, cfg_aliases 0.2.2 to 0.2.1. dashu is the arithmetic backend the meters price, so a bump may move limb readings. Under the coda they go up; the lane held them for your word. Recommendation: bring them up in one commit judged by the gate and the meter suite; any moved reading is attributed or is a stop.
- Finding, corrected in the night: the bisect to `5d167a63` stands, but the per-bit attribution to `IdReader::tag` was wrong: `Party::decode` parses through `DsiCursor::read_bit` (the `dsi_bitstream` reader) and never enters `IdReader`, on both sides of that commit, and the tag-pair change (landed, a real win elsewhere: fork kernels about 18% less fuel per bit, covers and disjoint 8 to 10%, join 6%, nothing higher, the meter suite byte-identical) moved `ff_party_decode` by zero. What inside `5d167a63` moved the decode kernel is still unnamed (candidates: the frame stack, the buffered reader's construction, the `IdFrame` Vec's growth on deep chains); the earlier text follows. A `git bisect` over the guest names `5d167a63` (2026-08-18, the crate-owned `BitsView` replacing bitvec's bit slice): `IdReader::tag` now reads each id node's two tag bits through `BitsView::bit`, an `assert!`, a bounds-checked byte index, and a shift, all in `u64` on a wasm32 guest, about 101 fuel per bit against the law's 53, and a deep fork chain's id is nearly all tag nodes. The band was pinned on 2026-08-04 and never refit; a refit moves 36 kernels by more than 0.05 dex in both directions. bytes moves the kernel by one fuel unit (`Bytes::from(Vec)` initializing in place). Routed: the refit to `p1-fuzz`; the per-bit cost of `BitsView::bit` on the id walk to `p8-performance` as a measured trade (a byte cursor for the tag walk). The gate lane's wasm stream reads red on its branch by the committed seed until `p1-fuzz` refits the band; the upward convergence of the locks stands under ruling 114's coda.
- The gate lane's audit failure is stop 1 above (last updated 2026-09-03 04:18 UTC); its report also named the library commit behind the fuel breach (below) (last updated 2026-09-03 04:15 UTC).

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

## Live state of the overnight run (rewritten at every event; last 2026-09-03 21:22 UTC)

| Lane | Branch tip | Agent state | Awaiting | Next |
|---|---|---|---|---|
| p1-board | `00948f7b` (37 commits on main `2fe750ac`; third round landed, report received and corroborated against its logs: 2096 green / 0 red at both scales; the span_dominance pin drift the only board red; gate ok but board and fuzz) | fresh-eyes round 3 running on the round's eleven commits (agent in `agents.tsv`); coordinator acceptance at the tip done and green but for the drift (`p1-board/coord/acceptance-00948f7b.log`: both compiles exit 0; 126 board tests passed; `amp-board-acceptance` 2096 green / 0 red at both scales, exit 1 on the span_dominance drift alone; `worst-cases-pin` the same three lines); the meta's acceptance table and stops are updated for the third round | the round's report | rounds section, packet (`<scratchpad>/p1-board/meta.md`) with stops 3 (ruling 11's fit) and 6 (the dominance re-pin) plus a new stop: the shard wire tag `PROTOCOL` moved v5 to v6 in `20e58098` (the cell carries one field where the band carried two; internal to the board example's shard fan-out, `meter/board/shard.rs`, no committed snapshot names it); the lane's other two deviations (the 20x to 10x probe move as its own commit; round 2's D1 test history) are packet notes |
| p1-gate | `90f7dd9c` code, packet head `75e0bc91` (children rebase onto `90f7dd9c` with `git rebase --onto 90f7dd9c fdd47747 <child>` when each reports; unsigned commits re-signed at merge) | packet ready | Finch: the syn roster (stop 1 in the packet), then his merge word after the three P2 packets | on merge: ledger shas for its entries; then rebase the three children onto main |
| p1-fuzz | `8073fada` (22 commits; the twelfth seed `cc a072497c` committed in `9d2be228`, the cold sentry green on all twelve, `p1-fuzz/sentry-twelve.log`, the coordinator's re-run at the tip in `coord/fuzzfit-8073fada.log`; before that `d0c14638`, 20 commits on the tagwalk tip `47288c3e`; the calibration `517e3f89` re-pinned 75 band keys, 49 before, on one run after two refusals on `ff_version_eq`'s dead floor; `3fcc31cd` routes fuelscape's i64 rows through the harness's one call path; `b9474419` adds the eleventh seed) | the coordinator's acceptance at the tip is green (`p1-fuzz/coord/acceptance-d0c14638.log`: both compiles exit 0; `just fuzzfit` 28 passed with eleven seeds; `fuelscape-test` 43; `wasm32-pins` 50) and both red-first plants read red on the Mac (`coord/plants-d0c14638.log`; the scratch worktree is removed, its forge dir deleted); the packet meta is drafted at `<scratchpad>/p1-fuzz/meta.md` (acceptance table, four stops, for-Finch items; rounds section empty); fresh-eyes round 1 resumed after the credit outage (agent in `agents.tsv`) | round-1 review landed (fifteen findings, no shipped-code bug; recorded in the meta's rounds section); nine repairs and one uncommitted attribution sweep (`ForkChain` weight zeroed, `ff_party_decode`'s fit compared) sent to the lane agent (the message of 21:20 UTC in this session; its content is summarized in the meta) | the lane's repair report, then fresh-eyes round 2 | acceptance table into `<scratchpad>/p1-fuzz/meta.md` (drafted from the lane's two reports: the pre-calibration report at `f297c49e` and the calibration report at `d0c14638`, both in its task output), rounds 2 and 3, packet, queue entry after `p8-tagwalk`. Stops to carry: REFIT_TOLERANCE 0.7 unchanged (the lane derives 0.55 from the printed 0.459); `ff_party_without`'s success arm bimodal under the fork-chain family (slope 0.714 to 0.146, ceiling at 128 bits up 0.8 dex), not recalibrated; the `ff_version_eq` mirror routing (identity and unequal pairs measured for the differential, left out of the fit) is the lane's own call, reversible; `ff_rank_decode` pinned constant over 128..1896 bits from 86 samples (a rank-codec emission is a candidate); skyline-query-9 (needs a meter Shape generator: another lane); the span and query kernels' vocabulary extension. Post-merge follow-up: the handoff's third fuzz leg (the fuzz crate's unit tests, one process per test, red-first) wires into the gate's `fuzz` stream only once `p2-widths`' `fuzz-test` recipe and this lane share a tree; the lane left the stream line at `fuzz-build fuzz-replay`. The lane's own gate ran at `3fcc31cd` (ten seeds): every stream ok but audit (the syn stop) and fuzz (the port). |
| p1-survivors | `984a015a` code, packet head `f9ec31d7` (child of p1-gate; unsigned, re-signed at merge) | packet ready | Finch's word, after the gate lane | on merge: ledger shas for codec-bits-15, skyline-watermark-24, suanpan-40, suanpan-tests-25, suanpan-39 (dup) |
| p8-tagwalk | `47288c3e` code, packet head `3316c4ed` (child of p1-gate; unsigned commits re-signed at merge) | packet ready | Finch's word, after the gate lane | on merge: ledger note (a finding, no ledger row of its own); p1-fuzz calibrates on `47288c3e` |
| p2-widths, p2-surface, p2-generators | packets ready | done | Finch's word | merge in the morning |

## What comes next, in order

- Wake or relaunch the two mid-task agents (gate, generators) if their reports did not land; a relaunch reads the brief, the launch messages in this session's transcript are the pattern (worktree, base, box mechanism through the wrapper with `CARGO_BUILD_JOBS=32 NEXTEST_TEST_THREADS=32`, no `pset-run`, annotations, in-turn polling).
- Packets for widths and surface once their gates read green but for `fuzz`; then Finch reads; merge on his word, widths first (surface's `Cargo.toml`/`Cargo.lock` root touches and the gate lane's lockfile touches rebase over each other).
- After `p1-gate` merges: launch `p1-fuzz` stacked on it (its brief carries three handoffs: the fuzz crate's test recipe now exists as `fuzz-test` from the widths lane, so fold it into the ruling-15 legs; the `call1_with_panic` channel; the guest-closure provenance pin from ruling 114), then `p1-survivors`; after widths and harness: `p1-board`, then `p1-suites`, then `p2-rows` (its brief carries the JOIN_WIDE_TOOTH attribution, the deep rows, and the meter suite's dev-profile statement), then `p2-cures`.
- `new-findings.md` indexes every finding without a ledger row and where it went.

## Open questions and stops for Finch

- The fuel movement is settled: the gate lane replayed the shrunk program under bytes 1.11.1 (13603 to 13743 fuel) and 1.12.1 (13602 to 13742); the dependency moves the kernel by one fuel unit, and the shape (a chain of forks from a common root with sparse ticks, a 136-bit party id) is out of band. Root cause: the `ff_party_decode` band's law does not hold for that family; the refit is `p1-fuzz`'s calibration (ruling 14), with the seeds from both lanes. Ruling 114's upward convergence stands on its own merits (Finch's general rule), not as the fix.
- `fuelscape.js`'s x-axis caption under ruling 110 (the surface packet's stop).
- Twenty-two unsigned note-only commits on `main` ahead of `origin/main` (both sessions', signing outages); a re-signing rewrite of `main` before any push, announced in the queue with the other session paused.
- gate-legs-1 (ruling 24's coverage reproduction) waits for CI's next red (ruling 112).
- Whether the three memory terminals should all pin the `(Trapped, None)` genre (only rank does; a fuzz-lane item).

## Machine notes

- The box syncs through git (ruling 112; the skill's wrapper pushes HEAD and checks it out, then rsyncs the working tree). Gates run in the general pool with the two caps; `pset-run` only for wall-time measurements. The `fuzz` leg is expected red on the box (no libFuzzer port) and counts as clean when sole.
- Box directories to retire when their lanes merge: `~/src/before-<lane>` and `~/build/before-<lane>` for p1-gate, p2-widths, p2-surface, p2-generators, plus the presweep pair above. Retiring a Mac worktree resolves its forge build directory first (none of these lanes built on the Mac).
- The two shared note files (`STATUS.md`, `merge-queue.md`) are edited only when clean, edit and commit in one command (queue rule 5).
