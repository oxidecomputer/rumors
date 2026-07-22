# Adjudication brief: prove-C1 (the impossibility side)

Role: construct the strongest true impossibility. Verdict up front, in the
trichotomy's terms (MUX-PROGRESS.md §4, "the campaign's hinge"):

- **H-b stands: I could not break σ\*.** After a systematic attack at all
  four assigned joints, demand-lockstep survives, and I can give a positive
  argument sketch for its deadlock-freedom at C = 1 per direction. C1 **as
  literally stated** (∀ pairs of deterministic local strategies, idling
  allowed) is therefore, on my analysis, **false**. [derived — one honest
  gap, §3.2, named as such]
- **H-a stands and is airtight: every work-conserving pair deadlocks, for
  every C**, on an explicit one-parameter family `prov C` (width ~C) whose
  jamming run is *forced* — the strategy is consulted only at states where
  its enabled-push set is a singleton, so no fooling/pigeonhole argument is
  even needed. The shipped bottom-most-ready mux is work-conserving, so
  this is the theorem that explains the empirical bug. Kernel-decidable at
  concrete C in the Controls.lean idiom; parametric in C by a uniform run
  script + counting arithmetic. [derived, high confidence]
- **The exact frontier** between the two: whether the strategy may wait an
  *unbounded number of its own scheduling opportunities* for new
  observations. Any strategy with bounded patience (in scheduler visits)
  jams on `prov C` stretched by the patience bound; σ\*'s patience is
  unbounded but *observation-conditioned*. That frontier is, I claim, the
  right formal reading of the charter's C1 (§4.1).
- On the hinge question itself: **the receiver's consumption ORDER is a
  deterministic function of causally-available information; the
  consumption OCCURRENCE of silent runs is not observable but is
  inevitability-derivable.** Derivability is enough for an idling scheduler
  and useless to a work-conserving one — that asymmetry is exactly where
  the class splits. (§2.3, §3.1)

Epistemic key as in PROGRESS.md: [proven] kernel-checked in-repo /
[checked] executable evidence in-repo / [derived] paper argument here /
[open] known unknown.

Model fixings assumed throughout (each is a phase-2 decision this brief
takes a position on, flagged where load-bearing):

- **M1 (mux/demux).** Per direction one FIFO pipe, capacity C messages
  (message = one model reply frame). Demux delivers the pipe head into the
  model's existing cap-1 `wire(p,h)` cell for that stream (these are the
  "17 one-slot handoffs" of the Rust demux — deadlock-doc map §1.1); blocks
  head-of-line if the cell is full (MUX-PROGRESS §2, matching
  `incoming.rs:86-90`). The per-stream cells are RETAINED: per the
  boundedness caveat (MUX-PROGRESS §4), C1 is trivially false without a
  bound on endpoint demux state, and trivially true is not on offer either —
  the cells are the shipped discipline.
- **M2 (what a strategy is).** σ_p : (p's tree, p's observed trace) →
  Option Frame, deterministic. Observed trace = p's own pushes + frames
  DELIVERED by p's demux (not frames still in p's pipe behind a blocked
  head), in order. §7 records the sensitivity of σ\* to this choice.
- **M3 (deadlock is σ-relative).** Deadlock(σ_I, σ_R) = reachable state,
  non-terminal, where no process/demux action is enabled AND neither σ
  prescribes an admissible push. This must be the definition: if
  σ-refusal did not count, the always-idle strategy would be vacuously
  "deadlock-free" and C1 trivially false; if the mux "can move" whenever
  any frame is enabled regardless of σ, C1 is about nothing. Under M3,
  idling strategies carry a real liveness obligation — which is exactly
  σ\*'s proof burden (§3.2).
- **M4 (units).** Message-counted capacities, the working default
  (MUX-PROGRESS §2). §4.4 notes the byte-unit transfer direction: the
  model's one-frame-per-reply makes the impossibility *harder* to
  construct than Rust reality (provision runs are many wire frames per
  model frame — deadlock-doc §4), so C1-WC transfers to Rust a fortiori;
  σ\*'s C = 1 does not transfer without byte pacing [open].

---

## 1. The information anatomy the whole brief rests on

Three structural facts, all read off MODEL.md, do all the work below.

**F1 — a party's entire input is the other party's wire frames.** The
cross-party surface is exactly the `wire(p,h)` family (MODEL.md §4, "the
pump's capacity-1 channel is the wire"; exposition: only wires cross).
Every process on q's side is deterministic given its inputs (Kahn shape:
blocking send/recv in fixed program order, nothing observes occupancy —
the latency doc's monotonicity premise, deadlock-doc map §4.7). Hence
**q's complete behavior is a function of (q's tree, the delivered prefix
of p's pushes, q's internal schedule)** — and by Kahn confluence the
internal schedule affects timing only, never which channel operations
occur or their per-channel order. [derived from MODEL.md §4–5 + the named
Kahn/capacity-monotonicity assumption already in the artifact]

**F2 — q's tree enters only through announced labels.** Control flow is
payload-independent (MODEL.md §1: count and order of channel ops depend
only on each child's merge-join arm). The arms are D/R/M. D and R arms
surface as Query reactions *inside reply frames that flow on the wire*;
M arms (Match, absorbed Supply) cost zero channel ops on both sides
(MODEL.md §2) — they are dropped from the skeleton and, critically, they
do not change the frame count of any stream: frames-per-scope is a
function of D/R labels alone (one wire send per D or R child, MODEL.md
§5 step 2). So the parent scope's reply frame always flows, and its
M-children are consumption-order-inert. **Every branch of q's machinery
that affects which channel ops q performs is announced in some frame q
sends or has already received.** leafReqs counts ride the h1-level
queries the same way; pending counts are derived (MODEL.md §4, resolution
pending counts = f(labels), asymmetric by role but both computable from
labels). [derived; the full enumeration is §3.1]

**F3 — per-stream consumption is positional and sequential.** A stage
consumes its wire channel in BFS scope order, one frame per scope,
prologue-recv before publications before the next recv; scope σ+1's
prologue requires all of σ's obligations fired (MODEL.md §5 steps 1–5,
the sequential-scope premise). So *which* frame q needs next on stream S
is never in doubt — only *when* q will be ready for it, which is gated by
(a) q's completed publications for the previous scope, including q's own
reverse-direction wire sends (the symmetric coupling), and (b) q's
assembler drains, whose pending arithmetic is F2-announced.

F1+F2+F3 give the sender a *simulation principle*: p can compute q's full
consumption schedule on every stream, as a function of the frames p has
pushed, up to the frontier where q's announcements have not yet reached p
— and the unknown region is exactly "subtrees whose parent replies from q
are still in flight or unsent."

---

## 2. Steelman of σ\* (H-b)

### 2.1 σ\* stated precisely

σ\* ("demand-lockstep") on side p maintains a **certified set** K_p of
events of the (unmuxed-model) event vocabulary (`Ev = Chan × side × seq`,
Proofs/Sched.lean) it can prove *realized or inevitable*:

- **Base:** p's own fired events; every frame delivered to p (and, by E1
  and per-channel FIFO, everything positionally upstream of it on both
  sides of its channel).
- **Internal closure:** an event of q's internal machinery (or a q-side
  delivery) whose DAG predecessors (E1 message edges, E2 back-pressure
  edges at the *muxed* capacities, E3 program order) are all in K_p is
  added — justified by F1/F2: given its inputs, q's machinery performs it
  in every maximal run (Kahn confluence; enabled internal actions cannot
  be permanently refused in a maximal run without producing a stuck state
  chargeable elsewhere).
- **Peer-push closure:** q's push of frame g is added when p's *simulation
  of σ\*-q* — run on the sub-trace of q-observations p can itself certify
  — prescribes g. Deterministic and monotone (more certified observations
  never retract a prescription), so this is a conservative lower bound on
  q's actual pushes. This closure is legitimate precisely because C1
  quantifies over *pairs*: the refuting witness is the pair (σ\*, σ\*),
  and each side may assume the other's strategy. [key move]

**Push rule.** σ\*-p pushes frame f = (S, k) iff:
(i) the consumption of (S, k−1) is in K_p **using only frames delivered
or ahead of f in p's pipe** (so f's handoff is empty at f's arrival, or
provably will empty without anything behind f); and
(ii) every frame p previously pushed satisfies (i) inductively (maintained
by construction). Otherwise idle.

Note what σ\* does *not* need: it never needs to know which single stream
q needs next (no such singleton exists — q runs ~ROOT_H/2 stages
concurrently, rust-streaming map §2.3); it only needs its own chosen push
order to be absorbable. The sender controls the interleaving; the burden
is absorption, not clairvoyance.

### 2.2 Walk through the empirical stall shape

The stall (deadlock-doc map §1.2–1.3): responder's opening reply disputes
root child #1 (first in radix order, deep) and requests ≥ 6 others; the
initiator's stream carries [deep answer, provision, provision, …]; the
eager mux flushes all of it; the responder's positional assembly gates
everything behind child #1's Pending slot; the needed deeper (h28) answer
is already flushed *behind* the provisions; demux head-of-line closes the
six-link cycle.

σ\*-I on the same skeleton:

1. Push frame #1 (the deep answer). Its Query reactions announce I's
   grandchild disputes; R's need for #1 was proven at session start
   (positional: R's walk is at scope 1, its `asked` fed by ROpen — all
   F2-derivable).
2. Frames #2, #3 (provisions): consumption of #1 and #2 by R is
   *inevitability-derivable*: scope 1's publications complete given only
   already-pushed frames (its lowerRes/asked cells are empty first time
   around), scope 2 is a degenerate R-scope with pending-0 resolution
   (MODEL.md §4: asker-side pending = dCount = 0). Push.
3. Frame #4: R's walk will park in `upper.send()` at scope 3 — the
   upperRes cell holds scope 2's resolution while Asm waits on scope 1's
   Pending slot, which requires the deep subtree, which requires I's h28
   answers. **All of this is computable by I** from announced labels
   (scope 1's dCount, the pending arithmetic) and FIFO order. σ\*-I can
   certify consumption of #3 but NOT of #4-and-beyond until the deep
   subtree completes. So it pushes #4 (handoff will take it) and **idles
   the provision stream** thereafter.
4. R's h29 questions arrive; I answers; σ\*-I pushes the h28 answers —
   the pipe is free because provisions #5.. were withheld. The subtree
   completes; its completion is derivable by I from its own delivered
   answers plus R's announced labels; therefore Asm's drain, the walk's
   unparking, and consumptions #4, #5, … enter K_I; the remaining
   provisions flow. Session completes.

**Why it survives here:** every gating fact in the six-link cycle —
positional assembly, pending counts, D/R labels, park points — is either
announced in-band (F2) or derived from FIFO positions (F3). The cycle's
"information trapped on the remote side" (the charter's phrase for what
credits delocalize) turns out to be *derivable from traffic the protocol
already carries*. The one thing that is genuinely never announced —
consumption of silent provision runs — is exactly the thing σ\* replaces
with inevitability derivation. [derived]

### 2.3 The positive claim for the panel

**Claim (H-b, for the record; positive-half ownership belongs to
refute-C1/C2):** with M1–M3, the pair (σ\*, σ\*) is deadlock-free at
C = 1 per direction on every wellFormed, margin-0 skeleton. Argument
shape: strong induction over the unmuxed event DAG (acyclic under
`schedulable` — [checked] both directions, PROGRESS.md §5; kernel-adjacent
via `merge_complete` giving a total linearization τ). Invariant: the
joint certified sets (K_I, K_R) and the realized-event set grow to cover
the DAG; at every incomplete quiet state the DAG-minimal unrealized event
is either internal (fires), a delivery (handoff provably empty by its E2
predecessor being certified), or a push whose rule-(i) obligation is its
E2 predecessor — a DAG predecessor, hence certified by the induction
hypothesis. Same-stream head-blocks are finite by §3.4. The genuinely
hard step is §3.2's fixpoint lemma; its status is the brief's main
honest gap.

---

## 3. The attack, joint by joint

### 3.1 Joint (i): receiver-side branchings — the enumeration

Every receiver choice point in `Model.apply` (Model.lean:330, 23 arms;
MODEL.md §5), classified. "Announced" means: derivable by the counterparty
at the relevant push time from frames sent/received plus F1–F3.

| # | choice point (apply arm) | what branches | classification |
|---|---|---|---|
| 1 | `walkRecvWire`/`walkRecvAsked` prologue | nothing — fixed order | canonical (MODEL.md §5.1) |
| 2 | scope cursor advance (`normWalk`) | which scope next | canonical: BFS positional; the scope list is f(labels), announced in parent replies (F2, F3) |
| 3 | `walkCommit` among obligations | publication linearization | **silent but consumption-confluent**: the *set* of scope-σ publications gates the next wire recv, not their order; and under `.impl` D4+D6 force essentially the macro's own order anyway (MODEL.md §6, D4 note). Residual: transient cell occupancy differs by order — bounded by cap 1 and drained before the scope boundary either way [derived] |
| 4 | D/R/M arm per child | reaction labels | **announced**: D/R ride as Query reactions in the flowing reply; M = zero channel ops on both sides, zero frames, consumption-order-inert; per-stream frame counts = f(D/R only) (F2; MODEL.md §2, §5) |
| 5 | `asmRecvRes`/`asmRecvLevel` | pending fills, drain order | announced-derived: pending = f(labels) (MODEL.md §4), order positional |
| 6 | `absorb` loop | leafReqs iterations | announced: leafReqs ride the h1 queries inside the leaf-parent-level replies (F2) |
| 7 | close cascade (`recvClose`, `producerDone`) | when streams end | derived: per-channel session totals are pure functions of Skel (Counting.lean full-prefix lemmas); Rust carries explicit End frames besides |
| 8 | **the receiver's own mux σ_q** | push timing of reverse frames | **NOT announced** — the only genuinely silent branching that affects consumption (a q-walk parked on an unpushed wire send blocks its stream's consumption). Dissolved for H-b by fixing the pair: σ\* assumes the peer runs σ\* and simulates it (§2.1 peer-push closure). This is also why C1 restricted to *robust* strategies (must work against every peer strategy in the class) would be a different — and easier to prove — theorem [open, flagged for the panel] |
| 9 | provision-run absorptions | none (order positional) | **silent occurrence, known order**: no reverse traffic ever (pure-supply replies generate zero publications), but occurrence is inevitability-derivable from announced pendings (§2.2 step 3) — the sharp instance of the hinge |

Conclusion for the hinge: rows 1–7 and 9 say the consumption *order* is a
deterministic function of causally available information — sub-question
answered YES. Row 9 says the *occurrence* of silent absorptions is not
observable, only derivable. Row 8 says the composition question (joint
ii) is where the real risk lives.

### 3.2 Joint (ii): does the symmetric composition ground out?

The feared state: both sides idle, all demands unproven, session
incomplete. Split by whether traffic is in flight.

**Quiet states (pipes empty, all delivered frames consumed).** Then each
side's knowledge is the full announcement prefix of a completed exchange
prefix. Session incomplete ⇒ the unmuxed event DAG restricted to
unrealized events is nonempty and acyclic ⇒ it has a minimal element u,
all of whose DAG predecessors are realized. If u is internal or a
delivery: enabled, fires (not a stuck state). If u is a send by p: its E2
predecessor (consumption of the previous frame on u's stream, cap 1) is
*realized*; realized events on q's side lie on gating paths whose every
scope was announced — because those subtrees *completed*, hence all their
frames were exchanged, hence (quiet) delivered to p. So p's simulation
certifies the E2 predecessor and rule (i) fires: σ\*-p pushes u. **No
quiet stuck state exists.** [derived, and I consider this step solid]

**In-flight states.** Here p may need an announcement that is currently
in q's pipe or handoffs. Waiting is safe iff its arrival is guaranteed,
i.e. p's own demux pipeline drains. p's handoffs drain unless a p-consumer
is parked on a send σ\*-p is withholding — the potential proof-level
cycle. The mutual-soundness argument breaks every instance I could
construct (§3.3 gives the sharpest near-miss). The general statement is:

> **Lemma L2 (joint epistemic grounding) [open in detail, derived in
> shape].** Define the joint operator F(K_I, K_R) = one application of
> the three closure rules on each side plus the realized-event updates of
> a maximal run in which each σ\* pushes whenever its rule fires. Then
> the least fixpoint of F covers the full event DAG of any wellFormed,
> margin-0 skeleton.

Proof shape: strong induction over the DAG, simultaneously for both
sides' certified sets ("s certifies u" for each side s), descending
through the closure rules; monotonicity of the calculus (proofs cite only
true, stable facts) makes the double recursion well-founded on the finite
acyclic DAG. **The honest gap:** the peer-push closure's simulation runs
on a *lower bound* of the peer's observations; showing the lower bound
suffices at every DAG step — that the simulated σ\*-q prescription for a
needed frame never waits on evidence only the *real* q has — is exactly
one more instance of the same induction, and I have not closed it to
proof grade on paper. If L2 fails, the failing DAG-minimal event is by
construction a receiver choice invisible-until-too-late, i.e. **the
fooling wedge and C1's proof** — MUX-PROGRESS §4's dichotomy lands
exactly here. My considered judgment after targeted counterexample search
(§3.3): L2 holds. Confidence: medium-high.

### 3.3 Joint (iii): in-flight ambiguity — the sharpest near-miss

Attempted wedge: p must choose between f1 (stream A) and f2 (stream B);
the discriminating announcement g is in flight to p; g sits in p's pipe
*behind* a frame u destined for handoff A, which is full because A's
consumer walk is parked on a send x that σ\*-p is withholding *pending
g's content*. Then p never sees g, the walk never unparks: deadlock.

Why it is unreachable under the pair (σ\*, σ\*): q pushed u before g. But
σ\*-q's rule (i) for u requires certifying that handoff A empties using
only frames ahead of u — and handoff A is occupied precisely because
σ\*-p is withholding x, which σ\*-q's simulation of σ\*-p reproduces
(deterministic, conservative). So sound σ\*-q would have pushed g (whose
handoff obligation is independent) *instead of* u, or idled. The
configuration requires one side to have pushed an uncertified frame —
excluded by induction. [derived] The general form of this rescue is
rule (i)'s "frames ahead of f" side condition: it makes every pipe
prefix self-justifying, so no needed announcement can be trapped behind
an unabsorbable frame. Can the sender always safely wait, then? Yes:
waiting is unsafe only if it stalls the arrival of the discriminator,
and the discriminator's path to p is clear by the same invariant.

### 3.4 Joint (iv): same-stream head-block finiteness

If σ\*-p pushes (S, k+1) while (S, k) still occupies the handoff, rule
(i) certified (S, k)'s consumption from frames *ahead of* (S, k+1) plus
q-internal progress plus simulated σ\*-q pushes. Nothing behind (S, k+1)
in the pipe is cited; hence the head-block resolves after finitely many
q-side events, each itself certified — induction over the (finite,
acyclic) cited sub-DAG. Every pipe wait under sound proofs is finite; ρ
(MODEL.md §7) bounds the total run. [derived]

### 3.5 Verdict on σ\*

I could not break it. All four joints resolve; the residue is L2's
formalization (§3.2), which I judge true. Per my role instructions, the
deliverable therefore includes this steelman *as the refutation of
C1-as-stated*, plus the salvage below, which I argue is the statement the
charter actually needs.

---

## 4. The salvageable impossibility: C1-WC

### 4.1 Statement and why it is the right reading of the charter

**Definition (work-conserving).** A strategy σ_p is work-conserving iff
whenever the mux is scheduled with pipe room and a nonempty set of
committed-enabled wire sends, σ_p prescribes pushing one of them (it may
choose which; it may not idle).

**Theorem C1-WC [target].** For every pipe capacity C ≥ 1 there is a
skeleton `prov C` — wellFormed, margin-0 (hence schedulable; inside BOTH
flagship theorems' hypothesis classes, so the unmuxed protocol provably
completes on it) — such that for *every* pair of work-conserving local
strategies (σ_I, σ_R), the muxed composition under M1–M3 reaches a stuck
non-terminal state. Under either axiom corner (`.impl`/D6 or `.full`/D5).

**Corollary (bounded patience).** The same family, unchanged, defeats
every strategy that may idle at most B consecutive scheduling
opportunities while its observations are unchanged: the adversary
schedules the mux B+1 times per fill step. So the impossibility frontier
is precisely *observation-conditioned unbounded idling* — the one liberty
σ\* uses. [derived]

Why this is the honest formal reading of the charter (MUX-PROGRESS §1):
the charter's question is whether "altering local send-order scheduling
based on existing information" suffices. Read literally, "send-order
strategies" includes idling and σ\* answers YES (C1 false). But (a) the
charter's concrete referent — the shipped mux, §5D's "sender-side
scheduling alone" — is work-conserving, and §5D's informal argument is
exactly C1-WC's mechanism ("scheduling cannot reorder bytes already
flushed"); (b) the author expected both conjectures true, and C1-WC + H-b
+ H-c is the trichotomy that makes both *spirits* true: no scheduler that
keeps working can survive, an idling scheduler survives only by paying
the serialization price C2 prices out; (c) M3 shows the literal statement
is definition-sensitive in a way the work-conserving statement is not.
Recommendation to the panel: state C1 as C1-WC + the bounded-patience
corollary, and record σ\* as the constructive refutation of the literal
form, with its capacity bound (C = 1) as the charter demands of a
refutation.

### 4.2 The adversarial family `prov C`

One-parameter generalization of the committed regression shape
(deadlock-doc map §1.2, §5.3), at model scale rootH = 4:

```
prov C : Skel
  rootH = 4, fan = C + 5, capLevel = 1
  scope 0 : root, h4, D, kids = [1 .. C+5]
  scope 1 : h3, D, kids = [C+6]        -- first in BFS/radix order: the deep dispute
  scopes 2 .. C+5 : h3, R, childless    -- the provision wall (C+4 of them)
  scope C+6 : h2, D, kids = [C+7]
  scope C+7 : h1, D, leafReqs = 1
```

wellFormed: ids BFS, kids ascending/one-height-down, kid multiset exact,
BFS alignment (scopesAt 3 flatMap kids = [C+6] = scopesAt 2, etc.) ✓.
Margin-0: every dCount ≤ 1 = capLevel ✓. Rust-shape correspondence: root
fan ≥ 7 with the first radix child deep-disputed and ≥ 6 whole-subtree
provisions behind it — the exact committed-seed shape, with the provision
count now scaling as C + 4. For C > F − 5 (Rust F = 256), replace the
single wide root by a stage of width ~C/F whose first scope is
deep-disputed — provisions on one stream aggregate across parent scopes
(deadlock-doc §4: "reply-count backlog is only fan-bounded at the opening
level"), so the wall's length is unbounded at fixed fan. [derived]

Role assignment (rootH even ⇒ I asks h4, R asks h3/h1): R's opening reply
announces the root labels (1 D + (C+4) R, lacked by R). I's stream
`wire(I,3)` then owes C + 5 frames: frame 1 = the reply about scope 1
(carrying I's Query reaction for scope C+6), frames 2..C+5 = pure-supply
provisions. The deep chain bottoms in `wire(I,1)` frames (the "h28
answers" of the empirical stall).

### 4.3 The forced run

Fix any work-conserving (σ_I, σ_R). The adversary schedules:

1. Opening exchange (IOpen listing, R's opening reply) — pushes forced,
   pipes empty, singleton enabled sets.
2. I's Walk(I,3) processes the root scope. Committed choice + the ledgers
   force the publication order: wire 1, res 1, asked(C+6) (D4: no later
   wire before the sole D sibling is resolved and has sent its queries —
   MODEL.md §6), then wires 2..C+5 in child order (per-channel order,
   MODEL.md §5.3), parent resolution last (D6) or immediately after
   asked(C+6) (D5) — either way the wire wall's order is invariant.
3. Adversary runs I's mux eagerly, R's side lazily. During this window
   I's committed-enabled push set is the **singleton** {next provision}:
   the only other I-walks await input that does not exist yet.
   Work-conservation forces the push whenever the pipe has room. R is
   scheduled just enough to keep the pipe draining: R's Walk(R,2)
   consumes frame 1 (publishing its `wire(R,2)` frame about scope C+6 —
   σ_R's push also forced, singleton, empty pipe), then scope 2, then
   scope 3 — where it **parks in `upper.send()`**: Asm(R) holds scope 1's
   resolution (pending 1, its slot fillable only by the C+6 subtree
   return, which transitively needs `wire(I,1)` frames), the upperRes
   cell holds scope 2's pending-0 resolution, scope 3's send blocks.
   Identical to empirical links 1–2 [checked shape, derived transfer].
4. Absorption budget on stream (I,3): 3 consumed + 1 handoff + C pipe =
   C + 4 frames. I has C + 5 to push. Work-conservation has I fill the
   pipe (frames 5..C+4) and block with frame C+5 committed in hand.
5. Now let R's `wire(R,2)` frame reach I: Walk(I,1) consumes it and its
   asked (sent in step 2), and commits its `wire(I,1)` frame — **behind a
   permanently full pipe**.
6. Final state check: I's mux blocked (pipe full; head = frame 5, handoff
   holds frame 4); R's demux blocked head-of-line; Walk(R,2) parked in
   upper.send; Asm(R) waiting on a level item; Walk(R,0) waiting on
   `wire(I,1)`; R's mux has nothing committed (its next frames need the
   deep exchange); I's walks parked on committed wire sends; every
   `recvClose` guard fails (producers not done). No process, demux, or
   mux action enabled; neither σ prescribes an admissible push (pipe
   full / nothing enabled). Non-terminal. **Stuck.** [derived; kernel
   plan §5]

The strategies were consulted only at singleton-enabled states, so the
run script is strategy-independent given work-conservation: no fooling
argument, no P-indistinguishability needed. (For the record, the
indistinguishability the *bounded-patience* corollary needs is trivial:
during the fill window I's observations are constant, so a patience-B
strategy must push within B schedulings of the same singleton.)

### 4.4 Robustness of the theorem

- **Axiom corner:** the wall's order is forced under both D5 and D6
  (step 2); the jam is at the receiver's Asm, indifferent to the sender's
  parent placement. State for `.impl`, check `.full` too. [derived]
- **Demux variants** (MUX-PROGRESS §2 requires this): with
  wait-for-consumer-readiness demux (no handoffs), absorption budget
  drops to 3 + C and the same family jams at m = C + 3. With per-stream
  buffers of depth k, budget = 3 + k + C: family jams at m = C + k + 3 —
  the impossibility must (and does) scale the wall with *total endpoint
  demux state + C*, per the boundedness caveat [checked, in-repo: the
  64 B/16 MiB invariance and the HANDOFF=1024 masking, deadlock-doc §1.4,
  §4].
- **Units:** in bytes the theorem only strengthens — one model provision
  = a subtree-sized frame run (deadlock-doc §4), so the wall needs fewer
  replies at any byte capacity. [derived]
- **Both-sides quantification:** σ_R is also forced throughout (its
  pushes are singletons on an empty pipe); no clever responder strategy
  can help, because R's problem is inbound HOL, which no local *send*
  policy touches — the frozen message set forbids the receiver from
  saying anything it wouldn't already say. [derived]

### 4.5 What survives of C2's necessity corollary — the mysterious third thing

C1-as-stated was meant to make C2's nonlocality *essential*. With σ\*
standing, the necessity claim must sharpen, and it sharpens informatively:

- Credits are NOT necessary for liveness. The signal strictly weaker than
  the full remote skeleton that suffices (the charter's "mysterious third
  thing") is: **the announcement prefix the protocol already carries,
  plus FIFO positional arithmetic, plus inevitability closure** — i.e.
  nothing new on the wire at all. What credits smuggle across is not
  information but *computation and timing*: an O(1)-local, in-the-moment
  proof of handoff emptiness, where σ\* must run a whole-peer simulation
  and must idle wherever inevitability is not yet derivable.
- Work-conservation is exactly what credits restore: a credit-mux is
  work-conserving over *credited* frames and never jams (deadlock-doc
  §2.5's W = 1 argument). So the trichotomy's engineering content: you
  need credits (or independence) to be simultaneously deadlock-free,
  work-conserving, and pipelined — necessity for liveness+performance
  jointly (H-c), not liveness alone. C1-WC is the "liveness under
  work-conservation" leg, kernel-checkable.

---

## 5. Lean-readiness

### 5.1 Model extension (site (a) of the lean-model map §5.2; do NOT touch `Chan`)

```lean
abbrev Frame := Chan × Nat                    -- wire channel + seq (positional identity)

structure MuxSt where
  pipe : Party → List Frame                    -- FIFO, ≤ C per direction
  hand : Party → Option Frame                  -- committed push awaiting pipe room

structure MState (sk : Skel) where
  base : Model.State                           -- walks/asms/openers/absorb unchanged
  mux  : MuxSt
-- wire cells base.chan (wire p h) are retained as the cap-1 per-stream handoffs (M1)

inductive MuxAct
  | intra (a : Action)      -- every non-wire Model.apply arm, verbatim
  | commitWire (p) (f)      -- walkFire on a wire channel re-targeted into mux.hand p
  | push (p)                -- hand → pipe when |pipe p| < C  (σ_p gates WHICH frame commits)
  | deliver (p)             -- head of pipe p → wire cell if empty; else disabled (HOL)

def Obs := List ObsEv                          -- own pushes ++ delivered frames, in order
def Strategy := Skel → Obs → Option Frame       -- per-party knowledge: see §7 (open defn)
def WorkConserving (σ : Strategy) : Prop :=
  ∀ sk obs s, enabledPushes sk s ≠ [] → room s → σ sk obs ∈ enabledPushes sk s
```

Copy verbatim: `run`/`drain`/`run_reachable`/`drain_reachable` +
`decide` (Controls.lean idiom, lean-model map §2.3); the phantom-key
guards (obstacle 1); a small `muxFlowOk` invariant on the `flowOk`
template (occupancy = pushes − deliveries) if needed. Keep K and
skeletons tiny for kernel anchors (obstacle 2).

### 5.2 Statements

```lean
def prov (C : Nat) : Skel := ⟨…as §4.2…, 4, C+5, 1⟩

theorem prov_wellFormed (C) : (prov C).wellFormed = true            -- decide small C; induction ∀C
theorem prov_margin0   (C) : ∀ s, (prov C).dCount s ≤ (prov C).capLevel

-- the run script is strategy-independent given WC (singleton-enabled lemma):
def provRun (C : Nat) : List MuxAct                                  -- §4.3's schedule, length O(C)

theorem wc_impossibility (C : Nat) (σI σR : Strategy)
    (hI : WorkConserving σI) (hR : WorkConserving σR) :
    ∃ s, MuxReachable (prov C) .impl C σI σR s ∧
         muxStuck (prov C) s = true ∧ muxTerminal (prov C) s = false

-- kernel anchors (the Controls pattern, decide, maxRecDepth ≤ 16000):
theorem prov1_stuck : (match muxRun (prov 1) .impl 1 (provRun 1) with
  | some s => muxStuck (prov 1) s && !muxTerminal (prov 1) s | none => false) = true
theorem prov1_not_deadlockFree : ∀ σI σR, WC σI → WC σR →
    ¬ MuxDeadlockFree (prov 1) 1 σI σR            -- glue: singleton lemma + run_reachable
```

Proof skeleton for `wc_impossibility`: (a) `enabled_singleton` — at each
fill-window state of `provRun C`, `enabledPushes = [next provision]`
(structural: only Walk(I,3) holds a commitment; decide at small C,
counting-layer prefix sums (`wiresBefore_full` etc.) for the ∀C index
arithmetic); (b) WC + singleton ⇒ the run replays under any (σI, σR); (c)
closed-form final-state description + `muxStuck` evaluation (an
occupancy-arithmetic lemma: 3 + 1 + C < C + 5). Consumed machinery:
Skel/wellFormed/margin-0 stack, Counting.lean totals, Controls
run/decide; the Preserve/Weave stack is NOT needed. Where induction is
genuinely needed vs decide: `decide` closes C ∈ {1,2,3} completely; the
∀C statement needs one induction on the fill counter — the arithmetic is
linear and self-contained.

For H-b (stated for completeness; owner: refute-C1/C2 panels):

```lean
def sigmaStar : Strategy := …    -- certified-set fixpoint; THE hard definition [open]
theorem sigmaStar_deadlockFree (sk) (hwf) (hm0) :
    MuxDeadlockFree sk 1 sigmaStar sigmaStar
-- consumes: merge_complete/τ as the DAG linearization witness, Numbering for
-- positional vocabulary, the L2 fixpoint lemma (§3.2) as the core induction.
```

### 5.3 Executable probes before proving

Run the eventdag `capOne` knob output on `prov 1..3` analogues
(anticipated experiment, lean-model map §3.8); wire a mux `drainAdv` to
watch the forced run jam; realizability bridge: a Rust proptest in the
`assert_valid` style exhibiting tree pairs realizing `prov C` for small C
(the committed seeds already realize the C≈3 shape [checked]).

---

## 6. The GKM framing, briefly

In Genest–Kuske–Muscholl terms: the protocol's MSC family is
*existentially 1-bounded per stream* (the W = 1 fact — the unique sound
reply-denominated window, deadlock-doc §2.7 — is exactly an existential
bound witness). C2 = the existential bound's witnessing linearization (τ)
is computable from the global skeleton. C1-WC = no *work-conserving*
local linearization achieves the bound: work-conservation forces
universally-bounded behavior, and the family is not universally bounded
(the provision wall's backlog is unbounded in C). σ\* = the existential
witness is locally reconstructible because the protocol's own messages
carry the full causal structure (GKM's "control data" is already
in-band). The frame slots in cleanly and could name the Lean definitions
(`existentiallyBounded`, `universallyBounded`) if the panel wants the
connection recorded.

---

## 7. Obstructions and open questions (the honest ledger)

1. **L2 (joint epistemic grounding) is [open] at proof grade.** The
   quiet-state half is solid; the in-flight half rests on the
   conservative-simulation argument (§3.2), whose "lower bound suffices"
   step I could not close on paper. If false, C1-as-stated is TRUE and
   the failing DAG-minimal event is the fooling wedge — the refuter panel
   should attack exactly there.
2. **M2 sensitivity.** σ\*'s calculus assumes observation =
   demux-delivered frames. If observation is narrowed to consumer-consumed
   frames the calculus adapts (derive from consumed) but the proofs
   thicken; the panel must fix M2 before Lean.
3. **Per-party knowledge has no model-level definition** (the model has
   only the shared Skel — lean-model map §5.5 obstacle 4). Both `Strategy`'s
   first argument and any indistinguishability statement need the new
   `LocalObs` definition + a Rust proptest bridge for concrete witness
   tree pairs. This is the genuinely new formal object of the campaign.
4. **Realizability of `prov C` for large C** (fan > 256 forces the
   multi-scope variant; radix-trie path compression constraints
   unverified) — [open]; check before claiming Rust transfer beyond the
   committed-seed scale.
5. **Byte-denominated capacities**: C1-WC transfers a fortiori; σ\* at
   C = 1 *reply* does not transfer to a byte pipe without pacing —
   consistent with the charter's decision to state message-counting as a
   scope limitation.
6. **Strategy-vs-walk-linearization boundary**: I folded the walk's
   committed-choice order into "the strategy" where the ledgers leave
   slack; under D4+D6 the slack is nil on `prov C`, so C1-WC is
   insensitive — but the panel should fix the boundary in the model
   definition (does σ control `walkCommit` picks?) for cleanliness.
7. **Robust-strategy variant** (row 8 of §3.1): C1 for strategies that
   must be deadlock-free against *every* peer strategy in the class may
   be provable even against idling — σ\*'s peer-simulation move is
   unavailable there. Worth stating as a third theorem if the panel wants
   an unconditional impossibility that survives σ\*.

