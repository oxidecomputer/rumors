# Campaign plan of record

Purpose: the resumable task deck and standing adjudications for the
deadlock-freedom campaign, so work can resume from a cold start even if
the session's task tracker is lost. Sibling of `PROGRESS.md` (the
detailed technical record); this file holds the *sequencing and
decisions*. Update both at checkpoints; retire entries here when they
land in PROGRESS.md as completed work.

Last updated: 2026-07-18.

## Working state (cold-start orientation)

- All formal work: worktree `~/src/rumors-worktrees/main`, branch
  `main`, commits path-limited. NEVER touch the user's checkout at
  `/Users/oxide/src/rumors` (branch `link-transport`, dirty with their
  work). (Worktrees relocated from `~/.cache/rumors-worktrees/` on
  2026-07-18 for durability, per user instruction.)
- Rust-side work: worktree `~/src/rumors-worktrees/parent-first`,
  branch `parent-first` (base 3fc9117a). Merges only when the
  implementation-facing theorem succeeds, together with the proof.
- Gates: `just lean` ends "Build completed successfully" before every
  commit (check output text, not exit status). The eventdag sweep
  (`cd formal/lean && lake exe eventdag eventdag-out 300`, fuzz sweep
  OK in output text) before every def-touching commit; once per
  checkpoint otherwise. No sorries in commits.
- Accumulated Lean traps: recorded in PROGRESS.md per-phase; read
  before writing proofs.
- Harness bug (observed 2026-07-18): background-shell completion and
  Monitor events sometimes never fire without user interaction. Forks
  launching long sweeps must poll — bounded background waits, checking
  the output file each wake — never park on a notification or Monitor
  event alone.

## Standing adjudications (user decisions, 2026-07-17/18)

1. **Finding #7 resolution**: the d5 (parent-early) ledger is a weave
   property, not an encoder property; the encoder's epilogue placement
   is deliberate and safe under the capacity discipline. Full analysis:
   `design/parent-placement.md` (parent-first branch).
2. **Theorem re-target**: the implementation-facing theorem is
   deadlock freedom under the encoder's per-walk order (epilogue
   placement, minted as a ledger) with arbitrary cross-process
   interleaving; walk channels at capacity 1; assembler capacity ≥ max
   per-scope dispute count (**margin 0**, the shipping FAN ≥ kids
   discipline — deliberately not the tight −2 floor, so the
   borrowed-slots invariant never enters the proof).
3. **Capacity monotonicity** for wider production configs is assumed
   informally (Kahn rationale: per-walk order fixed ⇒ processes
   deterministic ⇒ added buffer capacity only relaxes back-pressure).
   Record as a named assumption wherever the theorem is documented.
4. **The d5 theorem is kept** as the capacity-universal counterpart
   (the priced alternative encoder design point); the implementation-
   facing theorem owns the flagship names in Statement.lean.
5. **Rust proptests** mirror the minted local invariants on the
   parent-first branch, maintaining the chain: proptested local
   invariants ⇒ formally proven global theorem.

## The deck

- **#12 (DONE, 2026-07-18)**: d5 endgame closed. `Sched.deadlock_free`
  and `Sched.progress` (Proofs/Endgame.lean:966/:728) proven on main
  (`56844fbf`, record `5cbb42f1`), axioms independently verified as
  `[propext, Classical.choice, Quot.sound]` only. Notable: the planned
  Reachable-induction cursor invariant proved unnecessary — the
  committed-arm mirrors already pin performed prefixes statically; the
  endgame is per-family decode lemmas + the τ-least argmin + a close
  cascade. Residual (non-load-bearing): "terminal ⟹ all channels
  drained" corollary not minted (small assembly from Counting.lean
  totals if ever wanted).
- **#15 (DONE, 2026-07-18)**: the `d6` (epilogue) ledger minted —
  `AxMode.d6` + `AxMode.impl` (d6 instead of d5, all else as `.full`);
  guard verbatim + the Rust `assert_valid` spelling in PROGRESS.md §8;
  the pillar generalized with `hmode : ax.d5 = false ∨ ax.d6 = false`
  (d5/d6 assert opposite corners and are never combined); theorems
  renamed `progress_d5`/`deadlock_free_d5`, flagship names reserved
  for the `.impl` theorem; MODEL.md D5 paragraph corrected + D6 added,
  README ledger table re-rowed. Validation: margin-0 `.impl`
  adversarial drains asserted in runFuzz (hard error on stall) + pins
  + boundary matrix; sub-margin stalls required ≥ 1 (hypothesis
  load-bearing). 5a SETTLED: pdelay stalls under `.impl` itself — the
  −2 floor fails adversarially even for the encoder's per-walk order
  (it is poll-schedule-specific); margin 0 confirmed as the theorem
  hypothesis. 5b CONFIRMED in-model (stuck-state accounting: full
  buffers + consumer hand + producer hands). Sweep outcomes in
  PROGRESS.md §8.
- **#16 (DONE, 2026-07-19)**: the implementation-facing FLAGSHIP is
  proven. `Sched.deadlock_free : sk.wellFormed = true →
  (∀ s, sk.dCount s ≤ sk.capLevel) → DeadlockFree sk AxMode.impl`
  (Proofs/EndgameE.lean, via `Sched.progress`), axioms
  `[propext, Classical.choice, Quot.sound]` only; `schedulable`
  subsumed by margin 0. Both design-space corners now carry
  kernel-checked theorems (`deadlock_free_d5` unchanged); the complete
  proof-route record is PROGRESS.md §9 items 1–5. #18 is unblocked.
  Campaign record of the route (kept for the narrative doc): the
  implementation-facing theorem (see adjudication 2). Landed:
  `Sched.scheduleE` + `EventDag.schedCandidateE` (encoder-order trace
  layer, cross-checked and replay-validated under `.impl` at margin 0
  across the full gate); the EWEAVE both sides
  (`Proofs/Sched/WeaveE.lean` `weaveGoE`/`weaveE` with kernel
  anchors; `EventDag.weaveOrderE`; margin-0 validity + transcription
  gate-asserted on pins and all acyclic fuzz seeds; `pdelay` pinned
  rejected sub-margin / valid at margin 0); unit 1 as a
  projection-equality bridge (`proj_walkEventsE_eq` etc. — the E
  order projects identically per channel-side, so proj-based
  counting bricks transfer by rewrite; plus `walkEventsE_perm`/
  `totalEventsE_eq` for non-proj totals). Remaining units in
  PROGRESS.md §9: (2) the eweave master induction (U-sites at the
  scope tail via margin 0 + tower drainage — the bulk and the new
  content), (3) `merge_completeE` (Final.lean argmin re-instantiated),
  (4) endgame (Pending decodes under d6 mirrors + argmin/cascade at
  `.impl`, flagship names; `schedulable` dropped — implied by
  margin 0).
- **#17 (DONE, 2026-07-18)**: parent-first re-scoped to the epilogue
  invariant — `Trace::assert_parent_last` wired into
  `Trace::assert_valid` as the seventh check (commit `4407590b` on
  parent-first), exercised positively by all streaming proptests and
  negatively per-arm; the d5 probe kept as `assert_parent_early`
  (unwired, documented design-space record); capacity-floor pins
  intact; gate clean modulo the two known mux-deadlock failures at
  the branch base (fixed on link-transport).
- **#18 (DONE, 2026-07-19)**: legibility pass, no new mathematics,
  axioms re-verified unchanged. Statement.lean is now the audit
  document (both corners side by side; per-ledger English; the
  explicit statement→Rust chain naming `Trace::assert_parent_last`,
  `FAN = 256`, and the capacity pins; the transcription boundary; a
  named "Assumed, not proven" section). New `Proofs/Map.lean` carries
  the proof map (shared foundation, the five-stage per-corner chain,
  the E/d5 mirror table, the epistemic frame); all 38 `Proofs/`
  modules close with a uniform "Chain:" postscript. Record:
  PROGRESS.md §10. #19 is unblocked.
- **#19 (blocked by #18)**: typeset exposition (typst default) for a
  technically competent reader with no codebase familiarity: the
  problem, the mechanism, the deadlock question and design space, then
  a human-friendly but real proof, cross-referenced into the
  (refactored) Lean artifact, with the epistemic frame (kernel-checked
  / fuzz-validated / assumed) explicit. CENTRAL FRAME (Finch,
  2026-07-18): present the TWO satisfying regimes as first-class peers
  — parent-early/d5 (capacity-universal, the priced alternative;
  `deadlock_free_d5`) and parent-late/epilogue (what the Rust
  exercises; margin-0 floor; the flagship `deadlock_free` under
  `.impl`) — including the discovery narrative, the borrowed-slots
  mechanism, why −2 is poll-schedule-specific while margin 0 is
  robust, and the criticality-ordering insight.

- **#20 (blocked by #19, written LAST)**: typst companion — the full
  development narrative, from Claude's own perspective, first person.
  Division of labor: #19 is the WHAT (the system and theorems); #20 is
  the close history of HOW the verification took form. PRIMARY
  EMPHASIS (Finch): the proof development itself is the spine — the
  Quint era; the Lean transcription and two-tier executable/kernel
  discipline; the model's structure and why; the proof architecture
  as it evolved (edge layers, counting routes, telescopes, windows,
  master inductions, precedence, the endgame and its
  cursor-invariant-unnecessary surprise); the finding-#7 arc as a
  verification story; the d6 mint and margin-0/borrowed-slots
  analysis; the E-side re-derivation's systematic refunds;
  proof-engineering texture (trap lists, hard vs mechanical, where
  scouting saved effort); the proptest⇒theorem chain to the Rust.
  SECONDARY (one section, not the spine): the collaboration process
  (coordinator+forks, checkpoints, adjudication points, operational
  failure modes). Sources: PROGRESS.md + its git history (each
  checkpoint commit a dated belief snapshot), PLAN.md, design/ docs,
  both branches' git logs — and, EXPLICITLY AUTHORIZED by Finch
  (2026-07-18), the Claude session transcripts on this machine
  (~/.claude/projects/-Users-oxide-src-rumors/*.jsonl and sibling
  project dirs) for process details predating this session or lost to
  its compactions; mine them with targeted subagent sweeps, not
  wholesale reads. Distinguish lived from reconstructed narrative.

Serial order #18 → #19 → #20 is deliberate: the exposition's
references must point at the refactored organization, and the
narrative must include the exposition's own production.
