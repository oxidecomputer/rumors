# Refuting C1: σ* (demand-lockstep) and its deadlock-freedom argument

Panel role: refute-C1. Assigned deliverable of the phase-2 adjudication
(MUX-PROGRESS.md §3, §5 log entry 2026-07-21).

**Verdict up front.** C1 as literally stated (MUX-PROGRESS.md §1: ∀ capacity
C, ∀ deterministic local strategy pairs, ∃ a schedulable tree pair that
deadlocks) is **FALSE** [derived, this document]. The witness is σ*
(demand-lockstep), a strategically idling scheduler that pushes a frame only
when the receiver's consumption of the frame's per-stream predecessor is
*provable* from locally observed evidence plus deterministic forward
derivation over the announced skeleton prefix. σ* is deadlock-free and
terminating at every pipe capacity C ≥ 1 per direction, on the `.impl`
hypothesis class (wellFormed + margin-0), with 1 demux slot per wire stream.
The refutation survives the charter's frozen message set: σ* adds **zero**
control messages, zero acks, zero credits — it spends *latency* (idling)
where credits spend *alphabet*.

The near-miss matters as much as the result: a purely *evidence-based*
demand-lockstep (certify consumption only from received frames) is refuted
by an explicit wedge — the invisible scope (§6) — and what rescues σ* is
that the model is payload-erased and every receiver branching is announced
one scope in arrears. If either of those failed, C1 would be true; they are
therefore the exact content of the hinge question, and both hold in the
model of record (MODEL.md §1 payload soundness, §2 label minting).

Epistemic key, as in PROGRESS.md: **[proven]** kernel-checked in the repo;
**[checked]** validated executably; **[derived]** paper argument in this
document; **[open]** known unknown.

---

## 0. The mux model consumed (decisions this argument fixes)

Per MUX-PROGRESS.md §2, confirmed against MODEL.md §4 and the Rust proxy
(rust-streaming.md §1.4, §4.2):

- **What is muxed.** Exactly the wire family: per direction rootH/2 + 1
  cap-1 streams (`wire(p, h)`), the only cross-party channels
  (MODEL.md §4 channel table; exposition.typ:241-248 via model-doc.md §2).
  All other channels stay endpoint-internal and untouched.
- **Pipe.** Per direction one FIFO of capacity C messages (message-counted,
  the working default; the byte-denominated caveat is out of scope and
  flagged in §5).
- **Send side.** A walk's committed wire fire becomes a *push*: it enters
  the pipe when the mux selects it; the mux may select any enabled committed
  wire obligation with pipe room, or idle. No sender-side outbox is modeled
  (the pump hand is the committed obligation itself, MODEL.md §5 committed
  choice). Robustness: adding a per-stream cap-1 outbox only adds slack;
  the proof in §3 is insensitive to it [derived].
- **Receive side (demux).** The shipped discipline (mandated by
  MUX-PROGRESS §4 model decisions): wire-order delivery of the pipe head
  into per-stream one-slot handoffs — the slot IS the model's cap-1 wire
  channel cell — blocking (head-of-line) when the slot is full. One slot
  per stream, 1 message deep.
- **Deadlock.** Reachable non-terminal state of the composed system where
  no process, mux, demux, or delivery can move; a mux that idles by
  strategy contributes no move. This is the `run_to_quiescence` Stalled
  twin (MODEL.md §7).
- **Strategy locality.** σ_p is a pure function of (own tree, observed
  trace). One modeling decision this argument *needs* and names loudly:
  **the observed trace includes frames delivered into p's own demux slots**
  (slot-peek), not only frames consumed by p's protocol processes. This is
  faithful to the Rust: the demux decodes every frame (signal byte and
  body) before routing it (rust-streaming.md §4.2, `incoming.rs:60-92`).
  The demux is p-local machinery; its knowledge is local knowledge. Where
  slot-peek is load-bearing is marked in §3.5. The sender does NOT observe
  delivery, consumption, pipe occupancy at the far end, or anything else
  not derivable from its own actions and arrived frames.
- **Protocol send order.** Both parties run the shipping encoder's
  deterministic per-scope order (D6/`.impl`: per D child c in child order —
  wire(c), res(c), asked(g) for g ∈ kids(c); R children wire-only; parent
  resolution last; MODEL.md §5–§6). C1 quantifies over strategies, so the
  refuter picks: honest D6 commits + σ* mux selection. Everything else
  (interleaving of recvs, assemblers, demux, deliveries) stays fully
  adversarial.
- **Hypothesis class.** wellFormed + margin-0 (∀ s, dCount s ≤ capLevel) —
  the `.impl` flagship's class (`Sched.deadlock_free`, EndgameE.lean:921
  [proven]), so `scheduleE`/`merge_completeE` are available as the τ of
  this argument.

---

## 1. σ* precisely

### 1.1 The observation structure

At any state, party p's observation O_p consists of:

1. **Own side**: the full local component state — every walk/asm/opener
   cursor and committed obligation, and the sequence of p's own pushes in
   pipe order (p executed them).
2. **Arrivals**: the sequence of frames delivered to p's endpoint (consumed
   by p's processes ∪ currently sitting in p's demux slots), in arrival
   order. FIFO per direction makes arrival order = ¬p's push order
   restricted to delivered prefix.
3. **Own tree** T_p, used only through the labels it mints (below).

O_p is monotone along any run, and σ*'s derived sets below are monotone in
O_p — proofs never retract [derived, by construction].

### 1.2 The announced skeleton A_p

The model quantifies over one global `Skel`; per-party knowledge is the
genuinely new definition (lean-model.md §5.5 item 4). Define A_p(O) — the
sub-skeleton *announced to p* — as the downward-closed scope set with
labels, built from two minting rules:

- **Own minting.** p is the answerer of scope σ iff p produces the reply
  frame about σ. The child labels of σ (D/R/M, and leafReqs at height 1)
  are computed by σ's answerer merging the counterparty listing with T_p
  (MODEL.md §2). So p knows kids(σ) with labels for every σ it answers, as
  soon as the parent's listing has arrived — in particular no later than
  the moment p generates the reply frame about σ.
- **Received minting.** For every scope σ answered by ¬p whose reply frame
  has arrived at p's endpoint (consumed or slot-peeked), p reads kids(σ)
  with labels off the frame: D and R rode as Query(nonempty)/Query(empty)
  reactions, and — the hinge's Match sub-question — **M children ride too**:
  they are dropped from the *skeleton* with zero channel ops (MODEL.md §2),
  but the Match/absorbed-Supply reactions are payload of the parent's reply
  frame, which always flows. An all-M scope is invisible in *channel
  traffic*, never in *frame content*: its parent's reply announces that its
  kid list is empty.

Latency of announcement: the labels of σ's children are available to the
*producer* of the frames those labels concern at production time (own
minting), and to the counterparty exactly one frame-arrival later (received
minting). Every quantity σ* ever needs (wire counts, asked counts = |kids(c)|
per D child c, pending counts, leafReqs) is a pure function of A_p labels —
this is MODEL.md §1's payload-independence: "the count and order of channel
operations depend only on each child's merge-join arm, never on payloads"
[proven-adjacent: model soundness premise, adversarially cross-checked in
the extraction].

### 1.3 Certified and inevitable events

Events are the existing vocabulary `(chan, side, seq)` (Sched.Ev), extended
with delivery events del(c, n) for wire channels (push = snd; deliver;
recv). The DAG on announced events: E1 (snd ≺ del ≺ rcv), E2 at the real
capacities (for wire channels, the slot's cap-1 E2: rcv(c, n) ≺ del(c, n+1);
and the *demand edge* rcv(c, n) ≺ snd(c, n+1) that σ* itself enforces), E3
(the D6 program order, MODEL.md §5–§6) — all computable within A_p.

**Certified_p(O)** — least set of global events closed under:

- (C-own) p's own performed events.
- (C-arr) arrival of frame (c, n) at p certifies snd(c, n) by ¬p, del of it,
  and — FIFO — snd/del of every frame ¬p pushed earlier (arrival order is
  push order).
- (C-prog) program-order back-closure on ¬p's side within A_p: a certified
  ¬p event certifies every E3/E1 predecessor in ¬p's trace (prologue recvs
  of that scope and all earlier scopes of that stage, the earlier
  publications of the same scope, the recvs those publications' W/D1/D2
  guards force). All positions computable from A_p by the counting layer's
  prefix sums (wiresBefore/qsBefore/pendsBefore — Counting.lean, consumable
  [proven]).
- (C-fifo) rcv(c, n) certified ⇒ rcv(c, m) certified for m < n (sole
  consumer, positional).

Soundness: everything in Certified_p happened in the true run [derived,
immediate — every rule is evidence- or self-grounded].

**Inevitable_p(O)** — least superset of Certified_p closed under:

- (I-step) e ∈ Inevitable_p if e is a **non-push** event of ¬p's side or a
  delivery event (recv, internal send, commit, close, del — anything whose
  enabledness is not gated by a mux strategy), every DAG-predecessor of e
  within A_p is in Inevitable_p, and e's channel guard is open at the
  occupancy computed from Inevitable_p counts.

The exclusion of pushes is the entire trick: pushes are strategy-gated, so
no forward derivation may assume them; everything else in this model fires
whenever enabled, and enabledness of a recv/close is *stable until fired*
(SPSC: occupancy of c is decreased only by c's sole consumer's own recv;
MODEL.md §4 "Verified SPSC"). Sends' guards are stable too (occupancy
increased only by the sole producer, who is the blocked party itself).

**Keystone Lemma (proof-rule soundness at stuck states).** In any reachable
stuck state s of the composed system, every event of Inevitable_p(O_p(s))
has been performed. *Proof.* Induction along the closure's construction
order. Take the first e added by (I-step) that is unperformed at s. Its
DAG-predecessors are earlier in closure order, hence performed; its guard
is therefore open at the true occupancies (the counting step: predecessors
performed pins sends ≥ n and recvs = n−1 exactly as in the kernel proof's
guard-history lemmas, scheduleE_e1/e2 shape); e is a non-push event, so it
is an *enabled action* at s — contradicting stuckness. Events added by the
Certified rules are performed by soundness. ∎ [derived; Lean-shaped: an
induction over a fuel-indexed closure, no reachability induction needed]

Note what the lemma does NOT say: it does not say inevitable events fire
promptly, or at all, in non-stuck states. It is used only under a stuckness
hypothesis, which breaks the circularity that sinks naive "the receiver
will surely get there" arguments.

### 1.4 The demand-proof relation and the strategy

**Demanded.** An unpushed frame f = (c, k) (k-th message on wire channel
c = wire(p, h), consumer Walk(¬p, h−1), or ROpen/Absorb at the ends) is
*proven-demanded* at O_p iff

> k = 1, **or** rcv(c, k−1) ∈ Certified_p(O) ∪ Inevitable_p(O).

Base case k = 1 is unconditional because every consumer's first operation
on its wire channel is the recv itself: a walk's scope-1 prologue opens
with recv wire (MODEL.md §5.1, "reply first, then query"; ROpen's first op
is its one recv; Absorb's per-request block opens with recv wire). An empty
slot plus a consumer whose next wire-channel op is that recv can never
head-of-line block anything.

**σ\* (demand-lockstep).** At every scheduling opportunity, p's mux pushes
the least (fixed total order: stream height descending, then seq — any
fixed order works; the proof never uses the tie-break) enabled committed
wire obligation that is proven-demanded, if the pipe has room; otherwise it
idles. Both parties run the same σ*; the definition is role-symmetric
because it is stated over stages and channels, and each party is
simultaneously asker on one parity and answerer on the other (MODEL.md §3)
— walks-both-roles symmetry costs nothing here, the demand rule never
mentions roles, only channels and their consumers' program shapes.

σ* is deterministic, computable (two finite closures over A_p plus prefix
sums), and local in exactly C1's sense: a pure function of (own tree,
observed trace) — with the observed trace including slot-peek per §0.

### 1.5 Every receiver choice point, classified

MODEL.md's apply has, on the receiving side of a frame, the following
branchings that affect consumption order; each with its announcement and
latency (this is the hinge's checklist):

| Receiver branching | What decides it | Announced where | Available to the sender when |
|---|---|---|---|
| Which reply the k-th frame on c must be | positional pairing | nothing to announce: skeleton-static (wf_bfs_aligned) | always (counting layer) |
| D/R/M labels of σ's children (σ answered by p) | p's own merge | p mints them | at p's own frame-production time (zero latency) |
| D/R/M labels of σ's children (σ answered by ¬p) | ¬p's merge | Query/Match reactions **inside** ¬p's reply frame about σ | on arrival of that one frame (one-frame latency) |
| asked-send count of consumer scope j (= Σ |kids(c)| over its D children) | labels minted by ¬p | rides ¬p's wire(c) frames of scope j itself | after scope j's own frames arrive (one-scope arrears) |
| provision-run absorption (R-child supply; M absorbed Supply) | consumer absorbs silently, zero channel ops | the *existence* is announced in the parent frame's reactions; the absorption itself needs no announcement because it is not a channel op — consumption order is unaffected | zero latency (order-known: nothing to order) |
| Match children (all-M scopes) | zero channel ops both sides | kid-list emptiness rides the parent reply frame | one-frame latency; the *consumption* of the all-M scope's own reply frame produces **no** reverse traffic — see §6 |
| leafReqs counts (height 1) | minted by the initiator (always the answerer at height 1, MODEL.md §4) | inside I's leaf-parent reply frames | R has them before it produces the supplies that need them |
| parent-resolution pending counts (asker d, answerer d+r) | labels as above | same frames | same |
| cross-height cursor interleaving (which of ~17 walks moves next) | adversarial scheduler | **never announced — and never needs to be** | n/a: σ*'s proofs are per-stream facts (rcv(c,k−1)) that are scheduler-independent; the keystone lemma quantifies over all interleavings |

The last row answers phase 1's "suspect residue" (MUX-PROGRESS §4, the
crux entry): cross-height interleaving is real nondeterminism, but σ*
never needs to predict it. Demand proofs are assertions about a single
stream's consumption prefix, and those are monotone, schedule-independent
facts. The reverse-direction coupling (answerers blocked on the reverse
pipe feeding back into question consumption) is handled not by prediction
but by the stuck-state minimality argument of §3.4.

---

## 2. The theorem and its proof

**Theorem (σ* deadlock-freedom) [derived].** Let sk be wellFormed with
margin 0 (∀ s, dCount s ≤ capLevel), both parties conforming with the D6
encoder order, both muxes running σ*, pipes of any capacity C ≥ 1 per
direction, demux into per-stream cap-1 slots. Then no reachable state of
the composed system is stuck, and every maximal run ends Terminal.

Terminology for the proof: a *withheld push* is a committed wire obligation
whose frame is not proven-demanded at its sender's current observation
(the mux idles on it). τ below is `scheduleE sk` — the kernel-proven total,
injective, E1/E2/E3-respecting timestamp of every event of the completed
session, which exists on this hypothesis class (`merge_completeE`,
`scheduleE_inj`, `scheduleE_e1_pos` [proven]). τ is used only in this
meta-proof; neither strategy computes it.

### 2.1 Step 1: at any stuck state, both pipes are empty

Suppose stuck s has a nonempty pipe, direction p→¬p, head frame g. If g's
slot is free, the delivery is an enabled action — not stuck. So the slot
holds an unconsumed frame; per-stream FIFO makes g = (c', m+1) and the slot
frame (c', m), with rcv(c', m) unperformed. At g's push time σ* required
rcv(c', m) ∈ Certified_p ∪ Inevitable_p; both sets are monotone in O_p, so
rcv(c', m) ∈ Inevitable_p(O_p(s)) ∪ Certified_p(O_p(s)) still. By the
Keystone Lemma (and Certified-soundness), rcv(c', m) is performed at s —
contradiction. ∎

This is the formal content of "σ* never head-of-line blocks": INV-A, the
invariant that every pushed frame's per-stream predecessor-consumption was
certified at push time, plus keystone soundness, drains every pipe at every
stuck candidate. Note the proof allows the transient two-in-flight state
(slot holds (c,k−1) merely *inevitable*, pipe holds (c,k)) — σ* pipelines
that much — and stuckness is still refuted because inevitability at a stuck
state collapses to performedness.

### 2.2 Step 2: the chase — stuckness forces a withheld push

Suppose stuck s (pipes now empty). If no committed wire obligation is
withheld: every uncommitted publisher with obligations left has a choosable
commit (`walk_uncommitted_choosable`, Progress.lean:295 [proven] — holds in
mode .impl), so all publishers are committed-or-done; a committed non-wire
obligation's guard, a recv, a close, a delivery: we show something is
enabled by the chase below with no withheld-push case, landing in the
kernel theorem's territory; concretely, s with pipes empty and slot
occupancies read as wire-channel occupancies *is* a state of the unmuxed
cap-1 model up to the two-in-flight transients, which are absent when pipes
are empty. The chase makes this self-contained without a run-mapping:

Pick any unperformed event and walk backwards along unperformed
DAG-predecessors. τ decreases strictly along DAG edges (edge-respect
[proven]), so the walk terminates at an unperformed event e all of whose
DAG-predecessors are performed. Then e's guard is open (counting: for
recv(ch, n), producer order gives sends ≥ n and consumer order gives
recvs = n−1, occupancy ≥ 1; for send(ch, n), the E2 predecessor
rcv(ch, n−cap) performed gives occupancy ≤ cap−1; for closes, producer-done
plus emptiness; for wire recvs, pipes-empty means the frame — whose snd and
del are among the performed predecessors — sits in the slot). If e is not a
push, e is an enabled action — not stuck. So e is a wire snd whose sender
has it committed (D6 forces the commit order; the decode layer
`pends_sound`/`pends_cover` [proven] is the machinery aligning the
committed obligation with the pending event) — a **withheld push**, since
an enabled non-withheld push is a mux move. ∎

So a stuck state exhibits a nonempty set W of withheld pushes across both
parties, pipes empty, everything else disabled.

### 2.3 Step 3: the τ-least withheld push has its predecessor consumed

Let f* = snd(c, k) ∈ W with τ(f*) minimal over both parties; sender p,
consumer stage Walk(¬p, h−1) =: W′. If k = 1, f* is proven-demanded by
definition — contradiction. So k ≥ 2, and (c, k−1) was pushed (per-channel
producer order: the walk fired it before committing (c, k)) and delivered
(pipes empty). Claim: rcv(c, k−1) is **performed** at s.

Suppose not: (c, k−1) sits in the slot. W′ has not reached scope (k−1)'s
prologue recv. If W′ is exactly at that recv, it is enabled — not stuck.
So W′ is blocked mid-scope j ≤ k−2, on either:

- a withheld push g of scope j: but every publication of scope j precedes
  rcv(c, j+1) (E3) which precedes rcv(c, k−1) (consumer order) which
  precedes snd(c, k) = f* (the cap-1 wire E2 edge — present in the DAG that
  τ respects regardless of the muxed run's transients). So τ(g) < τ(f*),
  contradicting minimality; or
- an internal op (send into a full lowerRes/asked/upperRes/level cell, or a
  starved recv): run the §2.2 chase from it. Every event reached is a DAG
  ancestor of scope j's completion, hence τ-below f* by the same chain. The
  chase terminates at an enabled action (not stuck) or a withheld push
  τ-below f* (contradiction).

Hence rcv(c, k−1) is performed at s. ∎

This is where the reverse-direction symmetric coupling is discharged: the
chase freely crosses sides (an assembler starving for level items whose
subtree completions need frames of the *other* direction), and every
crossing rides a DAG edge, so τ keeps decreasing; both directions' withheld
pushes were pooled in W, so minimality applies globally.

### 2.4 Step 4: the sender can prove it — knowledge coverage

It remains to contradict "f* withheld": show
rcv(c, k−1) ∈ Certified_p ∪ Inevitable_p at O_p(s).

First, coverage of A_p. The closure that derives rcv(c, k−1) runs over W′'s
scopes 1..k−2 (their completions) and the internal network they touch. The
label data needed: for each such scope j, its own kid labels (minted by p —
p answers those scopes, since W′ consumes p's frames; zero latency) and the
asked-send counts |kids(c′)| for scope j's D children c′ — minted by ¬p,
riding ¬p's own scope-j publication frames wire(c′). Every such frame's snd
is τ-below f* (§2.3's chain), so at s it is *not* withheld (minimality),
i.e. pushed; pipes are empty, so it is delivered to p's endpoint; by
slot-peek-or-consumption it is in O_p. So A_p covers everything the closure
mentions. The same argument covers the deeper frames (both directions)
feeding the assembler towers under scope j's parent-resolution guards:
their snd events are DAG-ancestors of rcv(c, k−1), τ-below f*, hence
performed, and p either performed them itself or has received them.

Second, the derivation. Within A_p, p replays ¬p's side by the (I-step)
closure: all of ¬p's non-push events up through W′'s scope-(k−2) completion
and the prologue recvs of scope k−1 have their predecessors grounded in
Certified_p (p's own pushes; ¬p's observed pushes; C-prog back-closure) and
open guards at closure-computed occupancies. The closure is exactly the
model's own apply-guard structure restricted to non-push events, so it
reaches rcv(c, k−1). Hence rcv(c, k−1) ∈ Inevitable_p(O_p(s)):
**f\* is proven-demanded — contradiction.** No stuck state exists. ∎

One subtlety worth pinning: the derivation does *not* need ¬p's withheld
scope-(k−1) publications, nor any event τ-above f*. Demand for (c, k) is
only about the consumption of (c, k−1) — σ* deliberately does not wait for
scope (k−1)'s *completion* (that would be circular: completion of scope
k−1 can require frames τ-above f*). This is the exact reason the demand
relation is pinned to the prologue recv and not to scope completion, and it
is what lets σ* pipeline one frame ahead of the consumer's working scope.

### 2.5 Termination

ρ′(s) = unfired protocol operations + undelivered pushed frames. Every
transition (commit counted as in MODEL.md §7; push fires a send; delivery
decrements the second summand; recv/close fire ops) strictly decreases ρ′
or is a commit counted once per obligation; idling is not a transition. So
all runs are finite; by deadlock-freedom every maximal run ends Terminal.
Fairness-free, exactly as MODEL.md §7. ∎

### 2.6 What the proof consumes (candidate axioms/lemmas for the Lean bridge)

1. `scheduleE` totality/injectivity/edge-respect on wellFormed+margin-0
   (`merge_completeE`, `scheduleE_inj`, `scheduleE_e1_pos`,
   `scheduleE_e2`) [proven] — used as the well-founded measure only.
2. The counting layer's prefix sums and per-channel totals
   (Counting.lean) [proven] — guard-openness from performed predecessors,
   and σ*'s position arithmetic.
3. `walk_uncommitted_choosable` + the decode layer (`pends_sound`,
   `pends_cover`) [proven] — commit availability and committed-obligation ↔
   pending-event alignment.
4. Payload-independence of channel-op counts and order (MODEL.md §1)
   [assumed, Rust-anchored] — what makes A_p sufficient for the closure.
5. Label announcement: A_p as defined is derivable from the frame log
   (labels ride reply frames; Match reactions included) [assumed — needs a
   Rust proptest bridge: reconstruct the skeleton prefix from a frame
   transcript; see §7].
6. FIFO pipes; demux slot-peek observability [modeling decisions, §0].
7. SPSC guard stability (sole producer/consumer per channel side —
   `procs_snd_owned`/`procs_rcv_owned` [proven]).

Notably NOT consumed: capacity monotonicity (the artifact's standing
assumption). Early drafts of this argument routed through "muxed runs embed
in a cap-2-wires unmuxed model"; the keystone + chase formulation replaced
that embedding, and the assumption dropped out. The Lean statement need not
import it.

---

## 3. The tight capacity bound

**C = 1 is sufficient** — that is the theorem, and the proof is uniform in
C ≥ 1: nothing in §2 uses the pipe bound except FIFO order and "pipes empty
at stuck states", both of which hold at every finite C. Larger C only gives
σ* more room to push already-proven frames; the demand rule keeps
per-stream in-flight at ≤ 2 (one in the slot, at most one behind it whose
predecessor-consumption is inevitable-not-yet), so σ* never *uses* more
than min(C, #streams) pipe cells anyway [derived].

**Is C = 1 necessary?** The question dissolves at C = 0: a rendezvous pipe
(push completes only simultaneously with delivery into the slot) makes the
composed system *literally* the unmuxed cap-1 model — a push is enabled
exactly when the destination slot is free, senders never occupy shared
medium, head-of-line blocking is structurally impossible, and
`Sched.deadlock_free` applies verbatim with **no strategy needed at all**
[derived]. But C = 0 is not a mux in the charter's sense (no real byte
transport completes writes only on remote consumption; flush-paced
completion is the empirical problem statement, deadlock-doc §1.1), so the
honest statement is: **the muxed problem exists only for C ≥ 1, and σ*
solves it for all C ≥ 1**. There is no interesting lower capacity
threshold — the difficulty of the mux problem is monotone-increasing in C
for work-conserving strategies (more room to run ahead of demand) and flat
for σ*.

**Demux slot count: 1 per stream matters, but only for overlap, not
existence.** The proof uses the slot in two places: the k = 1 base case
(an empty slot plus consumer-first-op-is-recv grounds the induction) and
the one-frame lookahead (demand = predecessor *consumption*, not scope
completion). With zero slots (demux hands directly to the consumer when it
is at its recv — the "demux waits for consumer readiness" alternative
discipline of MUX-PROGRESS §2), σ* adapts: the demand rule strengthens to
"the consumer's arrival at the prologue recv of scope k is
certified-or-inevitable" (i.e. scope k−1 *completion* inevitable). The
§2.4 coverage argument still closes — scope k−1's completion events are
DAG-ancestors of rcv(c, k), and the same τ-minimality argument grounds the
labels — but the transient two-in-flight pipelining is lost and every push
waits a full scope rather than a prologue [derived, one notch less
confident: the completion-inevitability closure touches events τ-above
some withheld pushes in the *general* (non-stuck) case, which is fine for
the stuck-state proof but makes the latency analysis worse]. Slots deeper
than 1 buy nothing for liveness and are unsound to *grant* beyond one
reply anyway in the byte world (the W = 1 unit-mismatch discontinuity,
deadlock-doc §2.7 — echoed here as the reason the model's
messages-as-units choice is load-bearing and must be stated as a scope
limitation of the theorem).

Summary for the statement: fix C ≥ 1 arbitrary, slots = 1/stream. σ*
works. C1's negation needs only ∃C; we get ∀C ≥ 1.

---

## 4. The price σ* pays (the H-c question)

Where the streaming design's overlap survives, and where it dies
[derived throughout]:

1. **Per-stream question pipelining: capped at ~2 frames in flight.** The
   unmuxed protocol already caps a stream at cap-1 + producer's hand
   (MODEL.md §4); σ* holds slot + 1 pipe frame. So *within a stream*, σ*
   is within one frame of the shipped design's own discipline. Not the
   loss center.

2. **Cross-scope prefetch inside a stream: lost when proofs lag.** The
   sender may push (c, k) only after deriving rcv(c, k−1). When scope k−2
   of the consumer stage is D-heavy, the derivation needs |kids(c′)|
   for its D children — labels minted by the *peer*, riding the peer's
   scope-(k−2) frames. Those frames cross the reverse pipe on the peer's
   own demand-locked clock. Worst case (alternating D-chains on both
   sides): one reverse-frame arrival per pushed frame per stream — the
   per-stream ~1 RTT serialization of the W = 1 credit floor
   (deadlock-doc §2.7: "a thousand-leaf divergence pays ~10³ RTTs where
   V1 pays ~4"). This is the real cost and it is exactly H-c's shape.

3. **But the forward-derivation escape is large.** Wherever the closure
   needs no *new* labels, demand proofs are instantaneous and local:
   - **all-M and R (provision) scopes**: consumer ops are prologue recvs +
     a pending-0 parent resolution; inevitability derives with zero
     reverse evidence. Provision runs pipeline at full pipe speed under
     σ*. The empirical wedge's shape (deadlock-doc §1.2: six-plus
     provisions queued behind one deep dispute) flows *without any round
     trip* — σ* fixes the historical stall not by throttling the
     provisions but by proving them consumable.
   - Already-announced D regions (labels arrived earlier for other
     reasons) likewise.
   So the serialization price is proportional to the *fresh-dispute
   frontier* (D-children whose labels have not yet crossed), not to tree
   size or provision volume. Contrast the credit design, which pays its
   window discipline on every reply regardless.

4. **Critical-path latency: asymptotically no worse than the oracle.**
   The protocol's critical path is depth·RTT under any transport — each
   level's questions causally require the previous level's answers
   (design/streaming-latency-serialization.md, cited at deadlock-doc
   §2.7). σ*'s proof-lag adds O(1) reverse-arrivals per D-scope *on the
   critical descent*, a constant factor on depth·RTT, not a new
   asymptotic term. What C2's oracle buys over σ* is therefore NOT
   latency class but **bandwidth utilization at C > 1**: the oracle can
   keep the pipe full with frames the receiver will consume in exactly
   pipe order (τ's wire projection); σ* keeps the pipe only as full as
   its proof frontier allows, and idles otherwise. On wide trees with
   many concurrently provable streams the gap narrows (σ* interleaves
   streams freely); on a single deep fresh dispute it is ~2× the frames
   per RTT of the oracle.

5. **A smarter proven-demand relation recovers more.** Two upgrades that
   preserve the proof shape: (a) *batched inevitability* — when scopes
   k−1..k+j−1 of the consumer stage are all forward-derivable (M/R runs),
   push the whole run of j frames at once (the closure already licenses
   it); (b) *speculative labels bounded by fan* — the sender knows an
   upper bound on asked counts (≤ F) and could push one extra frame
   whenever the derivation succeeds under EVERY label assignment
   consistent with announced constraints; this is a strictly weaker
   demand notion, still local, still sound (the closure quantifies over
   the finite label space). How much of the D-frontier lag (b) removes is
   a quantitative question for the simulator probe [open]. The residual
   irreducible wait is the one the fooling intuition was pointing at: a
   frame whose consumability genuinely depends on a peer branching not
   yet announced under ANY consistent labeling must wait for the
   announcement — σ* just shows that waiting there is always safe and
   always finite.

Connecting to H-c as stated: H-c's claim "the oracle achieves
deadlock-freedom AND full streaming overlap, so credits/independence are
necessary for liveness+performance jointly, not for liveness alone" is
**directionally confirmed but should be weakened**: the joint-necessity
holds for *pipe utilization and constant factors*, not for the latency
asymptotics, and the price is localized to fresh-dispute frontiers rather
than uniform per-stream lockstep. The sharp residual claim worth proving:
no local strategy achieves the oracle's pipe utilization on adversarial
fresh-dispute chains — that is a performance-impossibility, the true
surviving kernel of C1's intuition, and a good candidate for the
"mysterious third thing" question (what credits buy is exactly the
fresh-frontier labels, one RTT earlier).

---

## 5. The obstruction that almost was (and what would make C1 true)

The strongest attack on demand-lockstep, stated as sharply as I can make
it, because it defines the boundary of the refutation:

**The invisible-scope wedge.** Let scope σ_{k−1} (consumer stage S(¬p,
h−1)) have zero D and zero R children — all-M, a dispute that dissolves
into matches and absorbed supplies. Its consumption produces ZERO channel
operations toward p: W′ consumes (c, k−1), recvs its asked, fires one
internal pending-0 parent resolution, and arrives at scope k's prologue
needing (c, k) — **and no frame p will ever receive is causally after
rcv(c, k−1) until p itself pushes (c, k)**. Any strategy whose demand
proofs are *evidence-only* (certification solely by received frames —
rule C-arr/C-prog without I-step) starves here: p waits forever for an
announcement that does not exist; the composed evidence-only lockstep has
a reachable state where all demands are unproven and the session is
incomplete. The same wedge appears in weaker form wherever a scope's tail
is invisible (the D6 epilogue: res + asked + parent after the last wire)
and the *next* reverse frame is itself demand-locked behind p's push —
a genuine proof-deadlock, two muxes waiting for each other's evidence.

Why σ* survives it: the (I-step) closure derives the invisible events
instead of observing them. That derivation needs exactly three protocol
properties, each of which is therefore *load-bearing for the refutation
and a boundary of C1's falsity*:

1. **Payload-independence** (MODEL.md §1): the receiver's channel-op
   counts and order depend only on merge-join arms (labels), never on
   contents. If any receiver branching consumed private-tree content
   beyond the labels — e.g. a consumption order that depended on hash
   values — the closure could not run and the wedge would be fatal:
   **C1 would be true**, with the invisible scope as the fooling gadget
   (two peer trees identical in announced labels, differing in the
   hidden branch, forcing opposite consumption orders — unprovable at
   push time, and pushing wrong jams a slot).
2. **One-scope-arrears announcement** (§1.5 table): every label a demand
   proof needs rides a frame that is τ-below the frame being justified.
   The τ-minimality argument (§2.4) is what turns "announced eventually"
   into "announced in time". If the protocol had a label that only rode
   frames τ-ABOVE its first use in a demand proof — an announcement that
   necessarily arrives too late — that would be the intrinsic obstruction
   of my brief's §5, and C1's proof. I looked for one systematically
   (§1.5 enumerates every choice point); the D6 epilogue's asked-counts
   came closest, and they miss being a wedge only because the counts for
   scope j ride scope j's own frames while demand proofs only ever need
   counts for scopes ≤ k−2, two scopes behind the pushed frame. This
   two-scope gap is structural (prologue-first consumption + per-channel
   FIFO), not accidental — but it is fragile: a protocol variant whose
   walks consumed asked BEFORE wire (query-first prologue) would need
   scope k−1's own labels to prove rcv(c, k−1)'s asked-recv enabled,
   and those labels ride frames that may be legitimately withheld.
   Check against any future prologue reordering.
3. **Determinism of the receiver network given inputs** (Kahn shape;
   fixed D6 encoder order): the closure's derived events happen in the
   real run in every schedule. Adversarial *interleaving* is fine (the
   keystone lemma quantifies over it); adversarial *implementation
   order* within the ledger-legal poset would break the exact-position
   arithmetic of C-prog. σ* as defined is a strategy for the shipping
   encoder, which is what C1 needs refuted; the ∀-implementations
   version of σ* would have to certify against the poset's worst case
   [open, believed adaptable — the counting layer is order-free].

So the honest statement of the boundary: **C1 is false for this protocol
because the protocol happens to announce every consumption-order-relevant
branching one scope before it is needed, and to be payload-independent in
its control flow. C1's truth for "natural" mux classes survives only as
(H-a): work-conserving strategies, which cannot idle at the fresh-dispute
frontier, are killed by the §4-parametric family (deadlock-doc §1.6) — a
claim this document does not adjudicate but whose proof machinery
(pigeonhole over pipe + slot state on the wedge shape) is untouched by
σ*'s existence.**

---

## 6. Lean-readiness

Site (a) of lean-model.md §5.2 (separate state component, `Chan` untouched),
consuming the existing artifact per §2.6's list.

### 6.1 The mux transition system

```lean
-- StreamingMirror/Mux/Model.lean
structure MuxState where
  base : StreamingMirror.State          -- slots = the existing wire cells
  pipe : Party → List (Chan × Nat)      -- frames p has pushed, FIFO
  log  : Party → List (Chan × Nat)      -- frames delivered to p, arrival order
  -- (log p) ++ (pipe ¬p ∘ …) reconstruct ¬p's push order; kept explicit
  -- so Obs is a projection, not a history argument.

inductive MuxAction
  | protoAct (a : Action)               -- every non-wire-send Model.apply action
  | push (p : Party)                    -- σ-gated: walkFire/openFire on a wire chan
  | deliver (p : Party)                 -- pipe head → wire cell if empty

def Obs (p : Party) (s : MuxState) : ObsT := …  -- own components + log p (slot-peek)

def MuxModel.apply (sk C) (σI σR : ObsT → Option (Chan × Nat)) :
    MuxAction → MuxState → Option MuxState
  -- push p: requires (s.pipe p).length < C, a committed wire obligation f
  --         with σ_p (Obs p s) = some f; fires the walk's fireOblig into pipe
  -- deliver p: pipe head (c,n), s.base.chan c = 0 → chan c := 1, log ¬p ++ [(c,n)]
```

### 6.2 σ* as near-Lean pseudocode

```lean
-- knowledge: the announced sub-skeleton, a *projection of sk* justified by
-- bridge axiom (B5): announced sk p obs = scopes whose parent-reply frame
-- index appears in obs.log or was produced by p (a counting-layer predicate,
-- decidable, no payloads needed — the model never stores labels in frames,
-- so A_p is DEFINED via sk + frame indices, and the axiom says the real
-- system can compute the same projection from frame contents).
def announced (sk) (p) (obs) : Nat → Bool := …

def certified  (sk p obs) : List Ev := …   -- C-own/C-arr/C-prog/C-fifo, fuel = |events|
def inevitable (sk p obs) : List Ev := …   -- I-step closure over non-push events

def provenDemanded (sk p obs) (c k) : Bool :=
  k == 1 || (Ev.rcv c (k-1)) ∈ certified … ∪ inevitable …

def sigmaStar (sk p) (obs) : Option (Chan × Nat) :=
  (enabledCommittedWire obs).filter (provenDemanded sk p obs) |>.min? byHeightThenSeq
```

Everything is a fold over finite lists with `DecidableEq` — `decide`-friendly
at pin sizes.

### 6.3 Statements

```lean
-- the refutation of C1 as literally stated:
theorem sigmaStar_deadlock_free (sk : Skel) (hwf : sk.wellFormed = true)
    (hm0 : ∀ s, sk.dCount s ≤ sk.capLevel) (C : Nat) (hC : 1 ≤ C) :
    ∀ s, Mux.Reachable sk .impl sigmaStar sigmaStar C s →
      Mux.stuck sk .impl sigmaStar sigmaStar C s = false

-- C1 (transcribed) and its refutation as a corollary:
theorem c1_false : ¬ C1Statement := ⟨1, sigmaStar, sigmaStar, …⟩
```

### 6.4 Kernel-decided vs induction

- **Kernel `decide`**: σ* runs to Terminal on every pin (jam, pdelay,
  pyramid margins, smokeChain) — the Controls.lean drain/run technique
  copies verbatim (drain with a σ*-driver, ~60 lines, per lean-model.md
  §2.3); the negative control "the shipped bottom-most-ready mux (or any
  work-conserving instance) sticks on the wedge shape" as a stuck replay —
  giving the H-a/H-b contrast as paired kernel facts before any induction
  exists. Also: evidence-only-σ* sticks on an all-M instance (§5's wedge as
  a kernel theorem — worth minting even though it refutes a strategy nobody
  ships, because it pins WHY the closure is needed).
- **Induction**: keystone lemma (closure-order induction, no reachability
  needed); pipes-empty-at-stuck (INV-A + keystone); the chase (well-founded
  on τ via `scheduleE` + edge-respect lemmas); coverage (§2.4 — the heavy
  one: needs per-event "τ-below f* ⇒ label-carrying frame delivered", built
  from the counting layer + FIFO). Estimated the largest new proof; the
  Preserve 23-way induction is NOT redone (σ*'s invariant is its own small
  `MuxInv`, flowOk-architecture).
- **Bridge axioms → Rust proptests**: (B5) announced-skeleton
  reconstruction from a frame transcript (new proptest against the trace
  infrastructure); slot-peek observability (the demux decodes frames —
  cite incoming.rs, no test needed, a modeling note); FIFO pipe (transport
  property, conformance-adjacent).

### 6.5 Statement-strength audit hooks (phase 4)

The formal C1Statement must quantify strategies as
`σ : ObsT → Option (Chan × Nat)` with ObsT including slot-peek — if the
panel rules slot-peek out of "local information", σ* needs the no-peek
variant (§3's zero-slot adaptation, demand = scope-completion
inevitability) and the theorem should be proved in THAT form to make the
refutation robust to the definitional choice [open — my recommendation:
prove the no-peek form, strictly stronger, believed to hold].

---

## 7. Verdict, and what this does to the trichotomy

- **H-b: CONFIRMED [derived].** σ* refutes C1 as literally stated, at every
  C ≥ 1, message set frozen, both sides symmetric, with a complete informal
  proof (§2) whose only consumed unproven inputs are the model's own
  standing premises plus the two new bridge items (label announcement,
  slot-peek). The crux answer: **yes — the receiver's consumption order is
  a deterministic function of causally-sender-available information**, not
  because every branching is *predictable*, but because (i) every branching
  is announced one scope in arrears of its first relevance (§1.5), and
  (ii) branchings never affect *whether* the per-stream next frame is
  consumable, only the internal work between consumptions, which is
  forward-derivable from announced labels (payload-independence). The
  suspect residues from phase 1 both discharge: cross-height cursor
  interleaving never needs predicting (demand facts are per-stream and
  schedule-independent), and the reverse-direction coupling is broken by
  global τ-minimality over both parties' withheld sets (§2.3–2.4).
- **H-a: NOT ADJUDICATED here, PLAUSIBLE.** σ* idles; nothing in this
  document bears on the ∀-work-conserving claim except positively: the
  fresh-dispute frontier where σ* *must* idle (§4.2, §5) is exactly where a
  work-conserving scheduler is forced to push something unproven, and the
  §1.6/§4 parametric family (no constant buffer covers a subtree-sized
  provision behind a first-radix deep dispute) is the candidate pigeonhole.
  Recommend the prove-C1 panel restate C1 with work-conservation as a
  hypothesis; the two panels' outputs are then consistent, not conflicting.
- **H-c: WEAKEN.** The serialization price is real but localized
  (fresh-dispute frontiers; provisions and matches pipeline free); the
  latency asymptotic (depth·RTT) is shared with the oracle; the oracle's
  strict advantage is pipe utilization / constant factors. "Credits or
  independence necessary for liveness+performance jointly" should become:
  necessary for *oracle-grade overlap*; a natural signal strictly weaker
  than the remote skeleton that recovers it is precisely the fresh-frontier
  labels one RTT early — which is what credits smuggle (they prove slot
  readiness without the sender deriving it). That is a concrete candidate
  answer to the charter's "mysterious third thing".

### Honest gaps, ranked

1. **§2.4's coverage argument is the load-bearing novelty** and has no
   kernel-checked analogue yet. The step "every label the closure needs
   rides a frame τ-below f*" was verified by hand against the D6 per-scope
   order and the prologue direction (wire before asked); it should be
   machine-checked early (a decidable per-skeleton check over the pins
   BEFORE attempting the induction), because a single protocol corner
   where a needed label rides a τ-above frame flips the verdict to C1-true
   (§5.2). [open — highest-value next probe]
2. **The chase's guard-openness at committed choice**: I claim
   pends_sound/pends_cover align committed obligations with pending
   events in the muxed system as they do unmuxed; the muxed system adds
   push/deliver events those lemmas never saw. Believed routine (the
   decode layer is per-process-local), unverified. [open]
3. **Openers/absorb/finishers**: §2's cases were spot-checked (k = 1
   grounding, absorb's wire-first block, RFinish counts) but not
   exhaustively enumerated the way the 23-way Preserve analysis would;
   a Lean transcription will force it. [open, low risk]
4. **Message-counted capacity**: byte-denominated pipes reintroduce the
   unit-mismatch (a provision run of unbounded bytes in one model
   message); the theorem must carry the scope limitation, as MUX-PROGRESS
   §4 already flags. σ*'s byte-world analogue would need the demand rule
   at frame granularity with runs, which is exactly where W = 1's
   structural soundness argument lives — compatible, not proven. [open]
5. **If the panel strips slot-peek from locality**, the no-peek σ* variant
   is sketched (§3, §6.5) but its coverage argument was not re-derived at
   full rigor. [open]

### One-line summary for the coordinator

C1 as chartered is false: demand-lockstep with forward derivation over the
announced skeleton is a local, frozen-alphabet, capacity-1 witness; the
impossibility survives only with work-conservation added (H-a), and the
"mysterious third thing" has a name — one-RTT-early fresh-frontier labels.
