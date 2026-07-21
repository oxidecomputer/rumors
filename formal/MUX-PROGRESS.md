# The mux conjectures: design of record

Target: settle, building on the deadlock-freedom artifact (README.md,
MODEL.md, PROGRESS.md), whether the streaming mirror protocol *needs*
flow-control credits or true channel independence to avoid deadlock over
a single bounded transport channel — or whether a sufficiently clever
local send-order schedule of the protocol's *existing* messages suffices.
This document is the design of record for that campaign: the problem
statement as agreed, the model decisions and their status, what has been
tried and refuted, and the workflow for the parts still open. Read it
before writing any mux-conjecture Lean. Companion docs: MODEL.md (the
protocol model this builds on), PROGRESS.md (the deadlock-freedom proof
whose machinery — the invariant, the counting layer, the schedule τ and
its weave — this campaign consumes).

Epistemic key, as in PROGRESS.md: **[proven]** = in the repo,
kernel-checked; **[checked]** = validated executably on a pinned matrix,
not yet a theorem; **[derived]** = paper argument in this document;
**[open]** = known unknown.

## 1. The problem statement (the charter)

The message set is FROZEN: the mux may not add control messages of any
kind. Cooperative flow-control credits are explicitly out of scope — it
is already understood that credits resolve the deadlock by delocalizing
channel-capacity information that is otherwise trapped on the remote
side, and the link-transport branch already solves the problem that way
(a transport with credits inside: QUIC, HTTP/2, separate TCP streams).
The purpose of this campaign is to determine whether such augmentations
are *necessary*:

> Do you need flow-control credits or true channel independence (or some
> more mysterious third thing), or is it surprisingly possible to achieve
> deadlock-freedom merely by altering local send-order scheduling based
> on existing information afforded by the protocol?

Two conjectures, both expected true by the author, both to be proved or
refuted in Lean on top of the existing artifact:

- **C1 (impossibility) [open].** For every pipe capacity C and every
  pair of deterministic local-information-only send-order strategies
  (σ_I, σ_R), there exists a tree pair — well-formed and schedulable,
  i.e. one the un-muxed protocol provably completes — on which the muxed
  session deadlocks. "Local information" means: the side's own full
  tree, plus the trace of every action it has observed so far in the
  session (its own sends and everything received, in order) — and
  nothing held by the remote party that has not yet reached it.
- **C2 (oracle existence; necessity of nonlocal information) [open].**
  There exists an oracular send-order function which, given BOTH sides'
  full bidirectional dispute skeleton, produces deadlock-free send
  orders for both parties over the bounded pipes — conjecturally with
  small constant capacity. Its dependence on remote information is
  essential; that necessity is exactly C1, so it should land as a
  corollary. If C1 is instead refuted, the refutation must be a
  constructive witness strategy plus a tight bound on the minimum pipe
  capacity that makes it work.

The most interesting residual question, whichever way C1 falls: is there
a natural signal strictly weaker than the full remote skeleton that
suffices for a deadlock-free schedule (the "mysterious third thing")? A
positive answer sharpens both theorems and names precisely *what*
information credits smuggle across.

**Resolution (2026-07-21, phase-2 adjudication — MUX-ADJUDICATION.md is
the ruling of record; the original conjecture text above is retained as
history).** The conjectures resolve as a trichotomy:

- **C1 as literally stated: FALSE** [derived, two named conditions] —
  refuted by σ*, "demand-lockstep with forward derivation": push a frame
  only when the receiver's consumption of its per-stream predecessor is
  Certified ∪ Inevitable. σ* adds zero control messages, is
  deterministic and local, and is deadlock-free at *every* C ≥ 1 per
  direction on the `.impl`/margin-0 class. The right to *idle* — not
  frame choice — is the whole frontier. Conditions: (A) the Keystone
  Lemma restated over push-time derivation trees; (B) the causal
  (A_p-limited) σ* probe sweep is a blocking stage-0 gate — if it
  wedges, C1 flips TRUE with that skeleton as the fooling wedge, and
  the suite is built so that outcome also lands as a theorem.
- **C1-WC: TRUE** [derived + checked, Lean-ready] — one fixed,
  tree-realizable skeleton (`wedge`, the regression shape at w = 4,
  rootH = 6, realized by the committed proptest seeds) defeats every
  *work-conserving* strategy pair — locality hypotheses dropped, even
  omniscient WC dies — at every C ≥ 1, via a forced run whose every
  strategy consultation is singleton-enabled (no fooling argument, no
  pigeonhole). Mechanism: cap-1 slot occupation + FIFO burial under
  commit-no-retract; capacity-flat [checked, w = 4 across C = 1..16].
- **C2 positive: TRUE at C₀ = 1** per direction (message = reply
  units) — the oracle of record pushes in `demandOrder sk d` = the
  *receive*-event projection of the kernel-proven τ = `scheduleE` onto
  direction d's wire channels (the send projection is FALSE in general:
  cross-stream skew is the protocol's pipelining). Necessity is
  class-relative: nonlocal information is necessary for liveness *under
  work-conservation*, not for liveness alone.
- **The mysterious third thing, named**: the announcement prefix the
  protocol already carries + FIFO positional arithmetic + the
  inevitability closure — nothing new on the wire. What credits smuggle
  is per-stream consumption evidence one hop early (the per-stream E2
  edge family the single pipe conflates): *computation and timing, not
  information*. σ* is W = 1 credits inferred instead of sent.
- **H-c (the performance price) demoted to executable tier**: in the
  payload-erased, latency-free model σ* costs 0.99× the unmuxed
  baseline; the real price lives in what the model erases (bytes, RTTs,
  causal proof-lag). No quantitative overlap claim enters any theorem.

Standing rulings adopted with the panel's recommendations (overridable
by Finch; each changes what "local information" means): observation =
slot-peek (frames observed at demux delivery, pre-consumption — the
charter's "everything received", faithful to incoming.rs decoding every
frame before routing; the no-peek variant is plausibly false via the
two-height mutual proof-starvation gadget); consumption receipts stay
OUT of the observation (flush-paced `pushed` only — admitting them
smuggles credits in via the observation type); theorem domain =
`.impl` + margin-0 (the shipping encoder's kernel-proven class), the
`.full`/schedulable port recorded [open]; capacity denominated in
messages (= replies), with the §5A W = 1 byte-soundness caveat stated in
every positive theorem's docstring.

## 2. The mux model [open — to be fixed by the adjudication phase]

Replace the independent per-channel transport with, per direction, a
single bounded FIFO pipe of capacity C. Working definitions, each a
modeling decision the adjudication phase must confirm against MODEL.md's
channel structure and the original Rust proxy before any Lean is
written:

- **Mux (sender side).** Whenever the side's protocol processes have
  send obligations enabled, the mux picks which enabled protocol message
  enters the pipe next — a pure function of (own tree, observed trace).
- **Demux (receiver side).** Delivers the pipe-head message into its
  target per-channel queue (the queues the protocol's consumers already
  read, at their existing model capacities); if the target queue is
  full, the demux blocks — head-of-line. Alternative disciplines (e.g.
  demux waits for consumer readiness) must be considered and the theorem
  stated for the discipline matching the original Rust proxy, with
  robustness across reasonable variants argued or proven.
- **Both sides muxed**; the impossibility quantifies over both
  strategies. Deadlock = reachable non-terminal state of the composed
  system where no process, mux, or demux can move.
- **Capacity units** (messages vs bytes) [open]: to be chosen and
  justified; messages-counted is the working default since the model's
  channels are message-counted.

Known modeling cruxes:

- **Indistinguishability lives at the tree level; the model has only
  skeletons.** C1's fooling argument needs "two remote trees the local
  side cannot distinguish from its trace prefix". Either characterize
  same-local-tree skeleton pairs inside the model, or exhibit concrete
  tree pairs and bridge with a Rust proptest in the established
  `assert_valid` style (README.md's assumption/theorem interface). The
  wide-tree regression seeds (`tests/pairwise.proptest-regressions`,
  `tests/shadow_validity.proptest-regressions`, per
  design/streaming-wire-deadlock.md) are candidate concrete witnesses.
- **Where boundedness lives.** The receiver's existing per-channel
  queues absorb reordering slack in addition to the pipe's C; the
  impossibility must beat pipe + queue slack combined. Whether total
  queue capacity is bounded by depth (fixed) or grows with the width
  the adversary uses is a load-bearing question for the pigeonhole.
- **C2's positive half may be mostly built.** The existing progress
  proof constructs a valid global schedule τ of all events (PROGRESS.md
  §3–5, the weave). "Oracle emits in receiver-consumption order" should
  serialize τ onto a small-constant-capacity pipe; what remains is the
  serialization argument, not the schedule's existence.

## 3. Campaign plan

Phases, each a coordinator-driven workflow; the coordinator keeps proof
work out of its own context and distills results here:

1. **Understand [in progress].** Parallel readers map the Lean artifact
   (model, theorems, reusable machinery), MODEL.md/PROGRESS.md (channel
   families, axioms, τ), design/streaming-wire-deadlock.md (the
   empirical wait cycle; §5's single-socket design of record; §8's Link
   contract), and the Rust reality (channel inventory, consumption
   determinism, the old proxy's discipline). Durable findings land in §4
   below.
2. **Adjudicate.** Independent prover/refuter panels attack C1 and C2 on
   paper, adversarially cross-examined, before any Lean: fix the mux
   model, settle the cruxes above, and emit precise Lean-ready theorem
   statements. Stop condition: the panel agrees on statements and proof
   skeletons, or produces a concrete refuting schedule for C1.
3. **Formalize.** Lean modules (a `Mux/` subtree in the existing lake
   package) built by worktree agents; iterate to `lake build` green with
   no `sorry`, `decide`-not-`native_decide` for controls, matching the
   existing artifact's trust posture.
4. **Verify.** Adversarial review rounds in the house style
   (surface correctness → operational validity → interaction effects →
   assumption verification), plus statement-strength audit: do the
   formal statements actually capture the charter's informal claims?
   Rust proptest bridge for any new model-level assumption.
4b. **T10: capacity monotonicity** (per Finch, 2026-07-21; scheduled
   after track F merges — it touches the same shared proof layers B and
   G refactored, and F is the last in-flight consumer; runs parallel to
   phase-4 review). Target: `DeadlockFree` (and termination) at every
   pointwise-widened capacity vector κ ≥ κ₀ — closing the [derived]
   Kahn-argument gap (AUDIT-NOTES A7) between the floor-capacity
   theorems and the deployed Window = 65536 configuration, and
   licensing instantiation at arbitrary minimum-satisfying channel
   bounds (per-family: widen levels, keep wires, any mix). Structure:
   scoping audit first — classify every capacity mention in EndgameE's
   stuck-state analysis and the trace lemmas it consumes as MONOTONE
   (enabled-at-floor ⇒ enabled-wide; disabled-wide ⇒ the floor fact) or
   EXACT (Sched E2 / borrowed-slots arithmetic, which does NOT
   transfer) — then build by whichever route the audit licenses:
   (1) commutation/deferral simulation (`applyW κ` beside `apply`,
   never touching it; wide runs reorder-map to floor runs; diamond
   lemmas over the 23 arms) — self-contained, medium-heavy, yields a
   reusable run-reordering theorem; or (2) the InvPW route if the
   enabledness core proves monotone-in-κ throughout (G's elastic
   theorem — the κ = ∞ wire corner proven capacity-blind — is the
   evidence this may be cheap). A7 then resolves by theorem, as A1 did.
5. **Legibility** (per Finch, 2026-07-21; precedes the docs pass).
   Make the theorem statements clear, clean, and human-auditable, in
   the tradition of the base artifact's Statement.lean pass: a
   `Mux/Statement.lean` audit surface collecting the suite's final
   statements in definitions small enough to audit by reading them,
   with the "what a skeptical reader must read, and need not read"
   contract stated; names revisited for the reader rather than the
   prover; hypotheses stated in their weakest honest form; every
   statement's docstring carrying its charter provenance (which
   conjecture it settles, which controls pin its hypotheses) and the
   byte-denomination scope caveat where applicable; a proof-map
   update; doclint/testdoc clean.
6. **Document** (per Finch, 2026-07-21). EXTEND `doc/narrative.typ` —
   the campaign history stays first-person and discovery-ordered; the
   mux campaign becomes its next chapter (the conjectures as posed, the
   panel, the σ\* reversal, the executable tier refuting the static
   oracle before Lean could, the no-peek surprise, the wedge theorem,
   the eager-absorption design consequence). REWRITE
   `doc/exposition.typ` — a coherent exposition over one strictly
   linearly ordered in discovery order: organize around the resolved
   trichotomy (eagerness is fatal on one small tree; patience with
   inference suffices; omniscience buys only time), with the theorems,
   the third-thing characterization (announcement prefix + FIFO
   arithmetic + inevitability closure; credits carry computation and
   timing, not information), and the engineering consequence (the
   single-socket design) each placed where understanding wants them,
   not where we found them.

## 4. Findings

Dated entries accumulate here as phases complete; refuted approaches are
recorded with their refutations, in the PROGRESS.md tradition of keeping
the negative space on the record.

### 2026-07-21 — Stage-3 track F landed and merged: T4 CLOSED

**`sigmaStar_deadlock_free` is kernel-checked** (zero sorry, 261 jobs
green at merge): σ\* × σ\* is deadlock-free on every well-formed
margin-0 skeleton at every C ≥ 1. Step 4 — `closure_coverage`, the
campaign's one genuinely new induction — closed by stage-indexing the
closure by τ (each event enters the sender's closure at its own τ
stage; performed wire sends are push-grounded with pipes-empty making
arrival grounding complete; everything else I-steps in off its
strictly-τ-below past), eliminating any saturation lemma. Per the
stage-0 lesson the proof never consults shared-FIFO order (slot E2 edge
only), and capacity monotonicity is consumed nowhere. Supporting stack
~6,200 lines including the Steps extraction (per-arm InvL/count/hand
facts restating the monoliths' bullets without touching them) and the
strategy-generic `sinv_reachable` (T5 inherits it). Companions:
`C1Statement` minted verbatim from §1; **`c1_omniscient_false`
unconditional**; `wedge_sigmaStar_deadlock_free` (σ\* completes the
impossibility witness); `wedge_evidence_starves` [decide] (the
Inevitable closure is load-bearing). HONEST RESIDUE:
**`sigmaStar_local` is [open] at kernel tier** — σ\* runs the
full-skeleton closure, and its locality is exactly the A_p-sufficiency
statement (coverage of the announced projection at ALL reachable
observations, not just stuck states); probe-checked 4,970/4,970, so
`c1_literal_false` carries σ\*'s locality as an explicit NAMED
hypothesis with the gap stated precisely in C1.lean's module doc
(`wireHeights`/`committedInHist`/`rootH` are LocalEq-invariant by
construction; `inevitable`/`scheduleE` not shown invariant). Phase 4
must adjudicate: push A_p-sufficiency to kernel tier, or accept the
named hypothesis with the probe bridge. FINDING (track F caught a
track-B bug): the landed `MuxInv` was UNSATISFIABLE as stated —
unguarded `delivered_eq` breaks at the phantom `wire I 0`
(Nat-subtraction alias onto walk (R,0)'s consumer count); fixed by
`allChans`-guarding both count fields + `pushed_mem` +
`evUniv_wire_mem`. T8 not attempted (budget); its stub records the
per-direction (K_I, K_R) parameterization.

### 2026-07-21 — Phase 1 (understand) complete

- **The mux surface is exactly the wire family.** The only cross-party
  channels are `wire(p,h)`: rootH/2 + 1 ≈ 17 cap-1 streams per direction
  at Rust rootH = 32 (MODEL.md's channel table; exposition.typ). Every
  other channel family (queries, resolutions, level returns, asked,
  leafRequests) is endpoint-internal plumbing. "The channels being muxed"
  is therefore a small statically-known set, and naive per-stream demux
  reservation is bounded (≤ 17 slots/direction) — bounded endpoint state
  is not by itself the obstruction. [proven-adjacent: read off the model]
- **Prior art inside the repo, both directions.**
  design/streaming-wire-deadlock.md §5D argues C1's core informally for
  the *shipped eager* mux: the peer's demand order is a function of the
  peer's tree, unknowable until its questions arrive, by which time
  answers were already flushed; mid-reply withholding breaks the
  atomic-reply decode; "this route needs a receiver-driven signal
  anyway." §5A is a full credit-mux design of record (reserved signal
  bytes, W = 1 structural soundness argument) — out of scope by charter
  (§1), retained as the boundary marker: W = 1 reply is the unique sound
  reply-denominated window (the unit-mismatch discontinuity), which any
  impossibility statement should echo. [derived, in-repo]
- **The crux for C1, now precisely named.** §5D does not obviously rule
  out a *strategically withholding* scheduler. Candidate refutation σ*
  ("demand-lockstep"): push exactly the frame the receiver's
  deterministic consumption schedule demands next, else idle. It stands
  or falls on one question: *is the receiver's consumption order a
  deterministic function of information causally available to the sender
  at push time* (own tree + questions received + answers already pushed,
  FIFO delivery making delivered-order known)? Every receiver
  skeleton-choice seems to be announced in its own questions; the
  suspect residue is cross-height cursor interleaving and the reverse
  direction's symmetric coupling (answerers blocked on the reverse pipe
  feed back into question consumption). If σ* is sound, C1 is FALSE with
  a delightful small-capacity witness; if a receiver choice is provably
  invisible-until-too-late, that choice IS the fooling wedge and C1's
  proof. Phase 2 adjudicates exactly this. [open — the campaign's hinge]
- **C2's positive core already exists, kernel-proven.** τ =
  `Sched.schedule`/`scheduleE`: total, injective, edge-respecting global
  timestamp of all events, a pure function of the full skeleton;
  completeness under `schedulable` (`merge_complete`) and margin-0
  (`merge_completeE`); `replaySchedule` runs it to terminal. Remaining
  C2 work: per-direction wire projection + single-FIFO embedding;
  "τ's wire projection arrives in consumption order" is NOT yet a lemma
  and is real work. [proven core / open embedding]
- **Model decisions queued for phase 2** (with reader recommendations):
  mux as a *separate state component* (`muxQ : Party → List (Chan × Nat)`
  or a distinct event alphabet) — do NOT extend the `Chan` inductive
  (ripples through the 23-way Preserve analysis); demux discipline fixed
  to the shipped one (wire-order delivery into per-stream one-slot
  handoffs, single blocking reader) with robustness across variants
  argued; capacity units: model messages = replies (byte-unboundedness
  of supply runs is the §5A unit-mismatch — state as scope limitation or
  model chunk counts); the trace alphabet σ observes (protocol frames
  only; flush receipts?); per-party knowledge as a projection of Skel —
  the model has NO private trees, this is the genuinely new definition,
  with a Rust proptest bridge for concrete witness tree pairs.
- **Boundedness caveat for C1's statement.** The empirical stall
  reproduces identically at 64 B and 16 MiB transport buffers: the cycle
  is demux head-of-line + flush-paced receipts, not raw pipe capacity.
  C1 must bound total endpoint demux state alongside pipe capacity, or
  it is trivially false (unbounded per-stream demux buffers — the
  design doc's rejected option C — absorb any misordering). [checked,
  in-repo]
- **Instruments available.** EventDag executable oracle (blameProbe,
  weakPotential, capOne knob — capacity-collapse experiments were
  anticipated); the Quint simulator; pinned adversarial skeletons (jam,
  parentTrap, pyramid d, boundaryProbe families); the regression shape
  (root fan ≥ 7, first radix child deep-disputed, ≥ 6 whole-subtree
  provisions behind it, ≈ 3 frames/stream slack) with committed seeds;
  `run_to_quiescence`'s no-wake Stalled witness as the operational
  deadlock definition. Kernel-checked stuck-run technique (Controls.lean
  run/drain + decide) copies to any new mux transition system.
- **Alignment audit opened** (AUDIT-NOTES.md, per Finch's standing
  request): A1 termination may be witness-checked rather than
  kernel-proven while MODEL.md §1 lists it under "proved" — to verify;
  A2/A3 documented-in-repo gaps recorded as campaign rules.

## 5. Log

- **2026-07-21** Campaign opened. Charter fixed (§1): message set
  frozen, credits out of scope, both conjectures to be settled in Lean
  atop the completed deadlock-freedom artifact. Worktree
  `rumors-mux` (branch `mux-conjectures` off main) created;
  `lake build` verified green at 238 jobs before any new work.
  Phase 1 readers dispatched.
- **2026-07-21** Phase 1 complete (4 parallel readers over the Lean
  artifact, MODEL/PROGRESS docs, the deadlock design doc, and the Rust
  on both branches; ~710k tokens). Findings distilled into §4; the
  campaign's hinge identified: whether the receiver's consumption order
  is causally computable at the sender (σ* demand-lockstep) or provably
  not (the fooling wedge). AUDIT-NOTES.md opened. Phase 2 (adjudicate)
  dispatched: five independent lenses — prove-C1, refute-C1, C2-oracle,
  model-fixing, and an executable simulator probe — then adversarial
  cross-examination and synthesis into Lean-ready statements.
- **2026-07-21** Phase 2 complete (8 agents, ~1.68M tokens; the probe's
  Python transcription passed all 21 calibration gates, then ran ~2150
  muxed sessions). Verdicts distilled into §1's Resolution;
  MUX-ADJUDICATION.md committed as the ruling of record, including the
  full theorem suite T0–T6 with proof skeletons, controls, and the
  staged build plan (~7k–13k new Lean lines, two named risks each with
  a fallback). Both cross-examinations returned zero fatal findings;
  the Keystone delivery-case repair and the slot-peek dependency are
  incorporated as conditions. Alignment findings A5–A8 recorded.
  Dispatched next: stage 0 (blocking causal-σ* probe gates P1–P4) and
  stage 1 (the Mux/ Lean harness) in parallel.
- **2026-07-21** Latency analysis landed and merged (MUX-LATENCY.md +
  addendum; harness pinned in mux-notes-phase2/latency/). Baseline
  (independent links) = (L+2)·δ, depth-only, probe-exact. σ\* (K = 1)
  adds the width term: expected 1.84× (max 4.8×) on the random pool,
  unbounded in width — class degradation depth·RTT → scopes·RTT via
  the widest frontier level (levels pipeline; the widest is paid once);
  silent runs pipeline free, so chains and the historical wedge cost
  σ\* zero RTTs; maximizing shape is wide shallow combs. C = 1 is
  stop-and-wait for every scheduler including the oracle: C₀ = 1 is
  liveness-only. **The K-dial law** ([derived] then [checked],
  probe-exact 54/54 with both corners reproducing): pacing K+1
  frames/RTT per frontier stream; T ≈ (L+2)·δ +
  2·⌈max(0, P\*−K+1)/(K+1)⌉·δ; round-trip parity with multi-link iff
  K ≥ P\*+1; residual hyperbolic below, no cliff. Sizing: the dial
  covers the widest frontier LEVEL (dispute-density × fan); the default
  Window::scopes() = fan² zeroes the term short of ~fan³-scope
  divergence. Conclusion of record: single-socket σ\*ₖ at advertised
  windows matches multi-link exactly in round trips in the model;
  residuals are byte HOL (chunk-bounded, K-independent) and TCP
  loss-recovery coupling — loss isolation and boringness are what
  multi-link still buys. K > 1 liveness remains T8 [pending kernel].
- **2026-07-21** Stage-2 track B landed and merged (~1,900 lines,
  kernel-only): **T2 `keystone`** (over push-time derivation trees, the
  F1 repair route; delivery events excluded from the closure vocabulary
  — the broken case is unstatable by construction) and **`chase`** (at
  any stuck non-terminal state with pipes empty, the τ-least
  unperformed event is a withheld wire push with all DAG predecessors
  performed), over the new `MuxInv` ground-fact interface with
  `muxInv_init`. Enabling refactor: `InvL` (the flow-free local
  invariant) minted so the decode layer runs at muxed states with
  frames in flight. Five argued deviations from refute-c1 §2 recorded
  in module docs; `MuxInv` preservation is stage-F's obligation. Two
  A+B integration fixes at merge (InvL projection at
  `commit_totality`'s call site; `mem_enabledPushes_intro` rename) —
  build green at 250 jobs. **Stage 3 dispatched on three worktree
  tracks**: E (T5 state-feedback oracle + `static_oracle_jams` + T9
  controls + T6 necessity), F (T4 σ\* — the coverage induction — +
  T7 companions; T8 σ\*ₖ stretch), G (ρ-decrease termination closing
  AUDIT-NOTES A1 + the elastic-parking simulation theorem +
  `wc_impossibility_K`).
- **2026-07-21** Stage-2 track D landed and merged (6 bridge tests
  green; full gate green except the two historical single-pipe stall
  replays, verified failing identically at the base commit — this
  main-based branch still carries the old transport, by design): wedge
  realizability (deterministic tree pair whose trace-decoded skeleton
  equals `wedge(32)` structurally; the committed integration seeds pin
  the jam mechanism, not the literal shape — adjudication F4 repair
  honored), LocalEq soundness + nondegeneracy (free-insertion
  invisibility at asserted 100% frequency), and B5 announced-skeleton
  reconstruction from payload-erased transcripts. Alignment findings:
  A10 (global publication order draws tokio RNG — payload-independence
  is honest per channel only) recorded; A5 wording qualified.
- **2026-07-21** Eager-absorption feasibility landed and merged
  (design/eager-absorption.md): verdict MODERATE (receiver) / INVASIVE
  (sender), no blockers, and the plumbing crux DISSOLVED — supplied-run
  construction is already demux-driven pre-cursor (`decode_reply` in
  per-stream pumps → `Convert::assemble` → `Backend::parent`, unlinked
  handles; the `Scope` FIFO is the context ledger). Context-registration
  causality verified arm-by-arm across the message vocabulary and named;
  proptest specified. Version-bound filtering is entirely sender-side at
  answer time, so eager absorption cannot resurrect redacted content.
  Receiver half ≈ 360 lines (widen three cap-1 proxy gates to K + a
  Window-style knob; parked descriptor = the existing `Reply` handle);
  sender σ\*ₖ inference ≈ 1–2.2k lines whose liveness spec IS T8 —
  house posture: the theorem lands first.
- **2026-07-21** Stage-2 track A landed and merged (`lake build` green
  at 245 jobs, zero sorry, kernel decide only): **T3
  `wc_impossibility` in full ∀C generality** — every work-conserving
  strategy pair deadlocks `wedge` at every C ≥ 1, via the σ-free
  forced-run executor (deliver-R-last as the withholding adversary), a
  b-bounded replay lift to arbitrary WC pairs (push guards monotone in
  C), and four kernel anchors (C ∈ {1,2,3} pipe-full parks + the
  capacity-blind `noHands` burial certificate covering all C ≥ 4 — no
  C-induction). T1 `commit_totality` proven (unique choosable
  obligation at every `.impl`-reachable state; Mathlib-free spelled-out
  uniqueness). Full controls suite: `wedge_not_deadlockFree`,
  `wedge_idler_completes` (work-conservation load-bearing),
  `wedge_unboundedSlot_completes` (one-slot demux state load-bearing —
  the formal echo of the eager-absorption design), C = 0 vacuity, and
  the F8 must-fail pins. Boundary finding recorded as AUDIT-NOTES A9:
  F8's conjunct is vacuous on well-formed skeletons (BFS alignment
  forbids in-flight frames at close) — it hardens the `mstuck`
  totality boundary, not a reachable protocol state.
- **2026-07-21** Suite extended by Finch's direction (T8 pair, the
  K-deep parking regime): `sigmaStarK_deadlock_free` — σ\*ₖ (the
  inferred-credit scheme at parking depth K in logical replies) is
  deadlock-free for every K ≥ 1 under K-deep demux parking with
  fan-bounded control residue; and `wc_impossibility_K` — the
  widened-wedge family (provisions scaled past K) kills every
  work-conserving pair at every fixed K. Scheduled after T4/T3
  respectively (each reuses its parent's machinery). Motivating design
  insight (Finch): conversion of wire frames to logical replies at
  arrival is a CUSTODY change, not a semantics change — the Backend is
  already a handle-based streaming constructor (`backend.rs`: nodes are
  cheap handles; `parent()` folds radix groups bottom-up), so supplied
  runs absorb at line rate into unlinked subtrees with O(1)-handle
  parked descriptors, and linking stays at cursor time. K-deep
  reply-denominated windows become sound with no new abstraction;
  the RTT frontier law improves from 1 to K scopes/RTT/stream.
  Feasibility assessment of the Rust refactor dispatched (plumbing: the
  construction coroutines currently race walk-owned channels; the
  version-bound filtering site; violation-check placement).
- **2026-07-21** Stage 0 complete: **the causal σ\* survived P1** —
  4,970/4,970 runs Terminal (497 skeletons: 9 pins incl. wedge, 88
  adversarial-family, 400 random margin-0; C ∈ {1,2}; 5 interleavings;
  symmetric composition), causality structurally enforced (the strategy
  is a pure function of own pushes + arrivals; skeleton access through
  an announced-set view that faults on overreach), zero soundness
  violations vs the omniscient certificate. Condition B discharged;
  C1-literal stays FALSE; T4 unblocked. P2 independently reconfirms the
  static-oracle failure (π-eligibility FALSE, minimal 11-scope
  counterexample recorded; state-feedback fallback verified Terminal on
  all 8 wedging skeletons). P3 mechanizes the wedge singleton-
  consultation property at C ∈ {1,2,3} with decide-anchor traces
  dumped. P4 REVERSES a panel expectation: no-peek causal σ\* also
  survives (3,470/3,470 incl. the F2 family) — slot-peek stands as a
  modeling decision, NOT a demonstrated liveness necessity; the
  refutation is stronger than adjudicated. Lean-relevant lesson from
  one fixed probe artifact: the Inevitable closure must gate deliveries
  by the slot E2 edge only — importing shared-FIFO head order into the
  derivation manufactures spurious HOL wedges. Ambiguities resolved to
  least-information readings, flagged as Lean divergence risks in
  STAGE0-GATES.md.
- **2026-07-21** Stage-2 track C landed and merged (`muxprobe` exe,
  golden 252-row matrix, `just muxprobe` in the formal tier; commit
  consultations 3,524/3,524 singleton — the probe-fusion WLOG is
  executably confirmed). **FINDING, T5-altering:** the static oracle is
  executably FALSE — `ofSchedule(π_d)`, τ's receive projection pushed
  as a precomputed list, deadlocks 4/25 random margin-0 skeletons
  (witness `genSkelM0 2` pinned as the `rand2` matrix instance;
  capacity- and interleaving-flat; independently cross-confirmed in the
  Python probe). Mechanism: the static order demands a wire frame whose
  producer walk is parked on a query into the full cap-1 `asked`
  channel, which drains only after that very frame, while a ready
  provision the absorber needs sits refused — π-eligibility fails as
  drafted. T5's oracle of record becomes the STATE-FEEDBACK form (the
  adjudication's named fallback, which completes everything tested);
  the failed static form is retained as a new negative control to
  formalize (`static_oracle_jams` on the pinned witness): even full
  skeleton knowledge does not make a NON-ADAPTIVE schedule live —
  adaptivity, not information, is the liveness ingredient. This
  sharpens the trichotomy and is a statement-strength lesson: the
  receive-projection argument was [derived] and wrong on a corner the
  executable tier caught before any Lean was written.
- **2026-07-21** Stage 1 landed (three commits, `lake build` green at
  241 jobs, kernel-only trust): `Mux/Basic.lean` (the harness of
  record — hand + pipe(C) of Chan tags + demux slots, no staging cell,
  F8-strengthened wire close), `Mux/Strategy.lean` (observations with
  slot-peek, `WorkConserving`, `LocalEq`/`LocalStrategy`),
  `Mux/Instances.lean` (the `wedge` literal + T0 pins). Bonus beyond
  plan: `wedge_bottomMostReady_jams` — the shipped discipline's jam on
  wedge at C = 1 is already kernel-decided (~70 forced steps, also
  verified jamming at C = 2). Deviations recorded in code comments
  (MObs/Strategy placement, margin-0 soundness bridge, viewEnc token
  serialization). Stage 2 dispatched: four parallel worktree tracks —
  A: Mux controls + `commit_totality` (T1) + `wc_impossibility` (T3);
  B: the repaired Keystone + τ-chase infrastructure (T2, shared by both
  stage-3 theorems); C: the `muxprobe` executable tier + gate wiring;
  D: the Rust proptest bridges (wedge realizability, LocalEq soundness,
  B5 announced-skeleton reconstruction). Stage-0 causal-σ* gate still
  running.
- **2026-07-21** Stage-3 track G landed (three commits, `lake build`
  green at 253 jobs, zero sorry, kernel decide only; every named
  theorem on the three standard axioms). Three independent theorems:
  (1) **Termination minted** (`Proofs/Termination.lean`, closing
  AUDIT-NOTES A1 by remedy (i)): `Model.rho` + `rho_decreases` (the
  23-case strict decrease; the `cAdj` committed-slot device prices the
  phantom corners, so the only hypothesis is the `asmLevelsOk`
  inductive invariant), `terminating` (run length ≤ ρ(init) — BMC
  completeness at depth ρ(init)+1 per instance is now a theorem),
  `maximal_run_terminal`(`_d5`), and `greedy_run_terminal`
  (drain-to-Terminal at explicit fuel ρ(init)). MODEL.md §1(ii)/§7
  updated. (2) **Elastic parking is deadlock-free by simulation**
  (`Mux/Elastic.lean`): `elastic_deadlock_free` — under per-direction
  UNBOUNDED reply parking, every pushes-when-nonempty pair
  (`EWorkConserving`, the widest honest class) is live at every
  C ≥ 1 on the well-formed margin-0 class, as a corollary of the
  flagship progress engine: at a stuck state pipes drain, hands empty,
  and the parked base state satisfies exactly `InvPW` — the progress
  lemma's hypothesis WEAKENED to conservation-without-caps
  (`progress_of_inv` re-typed; its argmin argument never consumed
  `chan ≤ cap`, and `InvP.weak` restores all callers). The adjudicated
  "projects to a reachable BASE state" phrasing is FALSE as literally
  drafted — multi-parked states are base-unreachable — and the honest
  route is this invariant-interface reduction; the `EMuxInv`
  preservation sweep is carried as an explicit hypothesis (the stage-F
  `MuxInv` obligation's elastic twin, to be adapted from that landing;
  `eMuxInv_init` is its base case), matching the Chase's interface
  posture. Kernel pin: `wedge_elastic_completes` (the unbounded-slot
  control as a first-class semantics). (3) **`wc_impossibility_K`**
  (`Mux/Proofs/WcImpossibilityK.lean`): at per-direction depths — the
  single-socket advertisement, `deliver .I` gated by `KR`,
  `deliver .R` by `KI` — `wedgeW (KR+5)` kills every `KWorkConserving`
  pair for KR ∈ {1,2,3} anchored, ∀ KI ≥ 1 GENUINELY (the burial is
  directional; the stuck certificate's reverse-deliver conjunct is
  pipe-emptiness, and the executor's floor-depth reverse deliveries
  lift by guard monotonicity), ∀ C ≥ 1 via T3's two-shape split
  (12 kernel anchors, calibration perfectly uniform: pipe-full parks
  at b ∈ {1,2,3}, `noHands` burial at b = 4, reverse pipe empty
  everywhere). `(KI, KR) = (1,1)` degenerates to the record harness
  (`deliverStepK_one`); KR ≥ 4 open at theorem tier (each depth needs
  its own kernel replay), covered at [derived] tier by the widened
  family.
- **2026-07-21** T10 secondary: `elastic_deadlock_free` is now
  UNCONDITIONAL — the `EMuxInv` seam is discharged by
  `eMuxInv_reachable` (the stage-F sweep's elastic twin, assembled
  from the Steps files; deliver arm trivial as predicted). Repair en
  route: the landed `flow_wire` was unguarded and unsatisfiable past
  walk (R,0)'s first wire receive (the track-F `delivered_eq` bug,
  reproduced) — now `allChans`-guarded, plus the `pipe_wire` content
  field the deliver arm needs (mux-notes-phase2/t10-audit.md §4).
