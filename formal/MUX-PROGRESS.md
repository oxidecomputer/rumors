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

(None yet. Dated entries accumulate here as phases complete; refuted
approaches are recorded with their refutations, in the PROGRESS.md
tradition of keeping the negative space on the record.)

## 5. Log

- **2026-07-21** Campaign opened. Charter fixed (§1): message set
  frozen, credits out of scope, both conjectures to be settled in Lean
  atop the completed deadlock-freedom artifact. Worktree
  `rumors-mux` (branch `mux-conjectures` off main) created;
  `lake build` verified green at 238 jobs before any new work.
  Phase 1 readers dispatched.
