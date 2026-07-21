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

## 4. Findings

Dated entries accumulate here as phases complete; refuted approaches are
recorded with their refutations, in the PROGRESS.md tradition of keeping
the negative space on the record.

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
