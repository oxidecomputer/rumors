# Campaign plan of record

Purpose: the resumable task deck and standing adjudications for the
deadlock-freedom campaign, so work can resume from a cold start even if
the session's task tracker is lost. Sibling of `PROGRESS.md` (the
detailed technical record); this file holds the *sequencing and
decisions*. Update both at checkpoints; retire entries here when they
land in PROGRESS.md as completed work.

Last updated: 2026-07-18.

## Working state (cold-start orientation)

- All formal work: worktree `~/.cache/rumors-worktrees/main`, branch
  `main`, commits path-limited. NEVER touch the user's checkout at
  `/Users/oxide/src/rumors` (branch `link-transport`, dirty with their
  work).
- Rust-side work: worktree `~/.cache/rumors-worktrees/parent-first`,
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
- **#15 (next; blocked by #12)**: mint the epilogue ledger
  (MODEL.md §5's documented placement) as an AxMode field (sweep all
  literals/destructures incl. #12's landed code); margin-0 adversarial
  validation sweeps (fuzz + boundary matrix) — the falsifiable check
  on the re-target; −2/−1 boundary runs to confirm the borrowed-slots
  story (documentation-grade); determine whether pdelay's drainAdv
  stalls at −2 are epilogue-legal; fix the incorrect D5 paragraph in
  MODEL.md (contradicts MODEL.md §5 and the code); reorganize
  Statement.lean flagship names. Def-touching: eventdag per commit.
- **#16 (blocked by #15)**: the implementation-facing theorem (see
  adjudication 2). Counting/edge-respect layer re-derived for the
  encoder-order schedule; #12's cursor-invariant + argmin architecture
  re-instantiates (epilogue ledger also fully pins per-walk order).
- **#17 (blocked by #15)**: parent-first re-scope — invert the d5
  check into the epilogue check (minted spelling), wire into
  `Trace::assert_valid`, proptests exercise it (passes on real
  traces), keep capacity-floor pins, README ledger row, full
  `just gate`.
- **#18 (blocked by #16)**: legibility pass. (1) Refactor/document the
  theorem statement so a human reviewer can verify the *claim* matches
  the desiderata and see explicitly which Rust proptest discharges
  which hypothesis. (1a) Refactor proof organization so the layer
  structure (counting → edge-respect → cursor invariant → argmin →
  statement) is trackable even where bodies stay dense; module docs
  state each layer's contract. No new mathematics.
- **#19 (blocked by #18)**: typeset exposition (typst default) for a
  technically competent reader with no codebase familiarity: the
  problem, the mechanism, the deadlock question and design space, then
  a human-friendly but real proof of the same theorem,
  cross-referenced into the (refactored) Lean artifact, with the
  epistemic frame (kernel-checked / fuzz-validated / assumed) explicit.

Serial order #18 → #19 is deliberate: the exposition's references must
point at the refactored organization.
