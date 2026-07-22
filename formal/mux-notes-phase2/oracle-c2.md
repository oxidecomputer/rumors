# Adjudication lens: the C2 oracle, its capacity, and the per-party knowledge layer

Panel role: settle C2's positive half and its relationship to C1; define the
per-party knowledge formalization both conjectures need. Epistemic key as in
PROGRESS.md / MUX-PROGRESS.md: **[proven]** kernel-checked in the repo;
**[checked]** executable-gate-validated; **[derived]** paper argument here;
**[open]** known unknown. Every load-bearing claim carries a tag and a source.

Verdict up front:

1. **C2's positive half is TRUE, with C₀ = 1 per direction** (model-message
   units), in both corners (`.full`/schedulable and `.impl`/margin-0), by a
   construction that consumes the existing kernel machinery almost entirely
   and adds only bookkeeping-grade new lemmas. The phase-1 gap ("τ's wire
   projection arrives in consumption order" is not a lemma) is real for τ's
   **send** projection — and dissolves once the oracle serializes τ's
   **receive** projection instead, with each wire send split into a *stage*
   event (sender-local, at τ's send time) and a *push* event (at τ's receive
   time). §1. [derived, high confidence; Lean route mapped]
2. **C2's necessity half is exactly C1** and should be stated as a corollary
   schema over whatever class X the prove-C1 lens lands; the essential
   information gap has a crisp characterization: **what credits smuggle
   across is the per-stream E2 (back-pressure) edge family**, which the
   single pipe conflates into one aggregate edge. Every sufficient "third
   thing" is a transport for per-stream consumption evidence: explicit
   (credits), inferential (σ*'s FIFO-causal proofs), or omniscient (the
   skeleton). §2. [derived]
3. **The per-party knowledge definition**: a `PView` projection of `Skel`
   with a **role-dependent** fooling alphabet — the task's guess
   {D, M(absent), R-away} is right for the *asker* side of a scope and needs
   correction on the *answerer* side ({D, M(absent)} for held children,
   plus **free insertion of R children and leaf requests**, which P cannot
   see at all). View-equality is a *necessary* condition for
   indistinguishability, checkable in-model; *sufficiency* (a common
   concrete tree) is delegated per witness pair to a Rust proptest bridge in
   the `assert_valid` style. §3. [derived; bridge specified]
4. **If σ* is live, necessity-of-nonlocal-for-liveness dies but a strictly
   better theorem survives**: the oracle achieves liveness *and* full τ-grade
   overlap at C₀ = 1, while any live local strategy must idle out the
   announcement lag on fooling families — the missing skeleton bit is paid
   either in deadlock (work-conserving) or in idle time (σ*). Both C2
   versions are stated side-by-side for the synthesis. §4. [derived]

---

## 1. The oracle order: serializing τ onto one FIFO per direction

### 1.1 What τ already gives, and the precise shape of the gap

Given [proven]: τ = `Sched.schedule sk` (d5 corner) and `Sched.scheduleE sk`
(d6/encoder corner) are total (`merge_complete`, Weave/Final.lean:1229;
`merge_completeE`, FinalE.lean:198), injective (`schedule_inj`,
Numbering.lean:1672), edge-respecting (`schedule_e1`/`schedule_e2`,
Sched.lean:694/700; positional upgrade `schedule_e1_pos`, Numbering.lean:1614),
canonical per channel (`schedule_proj_canon`, Numbering.lean:1583: the n-th
receive on channel c IS `(c, false, n)`), pure functions of the full skeleton.
The cross-party surface is exactly the `wire(p,h)` family (MODEL.md §4, "the
pump's capacity-1 channel **is** the wire"; exposition.typ:241-248): direction
I→R carries `wire I rootH` plus `wire I h` for odd h; direction R→I carries
`wire R rootH` plus `wire R h` for even h — rootH/2 + 1 streams each
(lean-model map §1.3).

The naive C2 plan — "push wire frames in τ's send order" — hits the phase-1
gap, and the gap is **not closable as stated** [derived]: τ's send order and
τ's receive order genuinely differ across streams. τ respects per-wire-channel
E2 at cap 1 (rcv(c,n) ≺ snd(c,n+1)), so each stream's sender leads its
consumer by at most cap(1) + hand(1) = 2 frames (MODEL.md §4
stream/driver collapse), but **across** streams the sender may legally
produce a deep-stream frame before a shallow-stream frame that the receiver
must consume first — this cross-stream skew is precisely the pipelining the
protocol exists to have (exposition.typ:159-166 via model-doc map §9), and it
is the skew the empirical FIFO turned into head-of-line death
(deadlock-doc §1.3 [checked]). So "τ's send projection arrives in consumption
order" is *false* in general, not merely unproven. Any oracle that pushes in
τ-send order needs pipe + demux slack covering the skew (see §1.6 for that
variant); the sharp result comes from abandoning send order.

### 1.2 The construction: demand-order push with a stage/push split

**The oracle order.** For direction d (say I→R), define

> π_d(sk) := the subsequence of τ consisting of the **receive** events on
> d's wire channels, read as a list of (channel, seq) tags.

In Lean this is a filterMap over `schedule sk` — no sorting, no new
computation; totality of π_d over all of d's wire traffic is exactly
`merge_complete` + `schedule_inj`, and π_d restricted to any single channel
is seq order 0,1,2,… by `schedule_proj_canon` (receives are canonical). π_d
is a pure function of the full skeleton: exactly C2's "given BOTH sides' full
bidirectional dispute skeleton".

**The mux discipline (oracle strategy σ_orc).** Split every wire send into
two events:

- **stage(c,n)** — the producing walk/opener fires its send obligation into
  the stream's *staging cell* (the existing cap-1 wire channel state,
  reinterpreted as sender-side). All ledger program order (W, D1–D6,
  sequential-scope) gates on *stage*, exactly as it gates on `send wire`
  today. This is faithful to the Rust: the walk's send completes when the
  frame enters the pump's cap-1 output channel, not when bytes move
  (MODEL.md §4; rust-streaming map §2.3).
- **push(c,n)** — the mux moves a staged frame into the direction's FIFO
  pipe (capacity C). σ_orc pushes the frames of π_d **in π_d order**: it
  pushes the next π_d element when (i) it is staged, (ii) the pipe has room;
  otherwise it **idles**. Idling is allowed: C2's oracle is not
  work-conserving, and the charter's strategy definition permits idling
  (MUX-PROGRESS §1, §4 "strategically withholding").
- **deliver(c,n)** — the demux (shipped discipline: single reader, wire-order
  delivery into per-stream one-slot handoffs; MUX-PROGRESS §4 model
  decisions) moves the pipe head into stream c's receiver cell.
- **consume(c,n)** — the consuming walk's `recv wire`, now reading the
  receiver cell.

**Why the stage/push split is forced, and why it is the whole trick**
[derived]. Delaying the *send itself* to consumption time would drag the
sender's entire internal pipeline behind it: axiom W makes `lowerRes(c)` wait
on the wire yield, D1 makes queries wait on resolutions, and the
sequential-scope premise makes the next scope's prologue wait on everything
(MODEL.md §5–§6). That is §5D's "withholding stalls the sender" observation
turned inward. The split decouples them: the sender's *program* runs at full
τ speed against staging cells; only the *wire occupancy* is scheduled by the
oracle. And the staging cells need no new memory: they are the cap-1 wire
channels the model already has — under the oracle they hold a frame from
τ(snd) until τ(rcv), exactly the interval the unmuxed cap-1 wire channel held
it, so the sender-side back-pressure is **bit-for-bit the unmuxed model's**.

### 1.3 Validity: the refined schedule τ* respects every guard

Define the refined global schedule τ* from τ by a linear list transform:

- every non-wire event keeps its τ position;
- every wire send snd(c,n) is renamed stage(c,n) at its τ position;
- every wire receive rcv(c,n) at τ position t is expanded to the block
  ⟨push(c,n), deliver(c,n), consume(c,n)⟩ at positions t−2ε, t−ε, t.

τ* is a deterministic, computable function of sk (a map over the `schedule`
list). Guard-by-guard check that τ* is an execution of the muxed composed
system at pipe capacity C = 1 per direction [derived — each line names the
kernel lemma the Lean version consumes]:

1. **Sender program order into stage(c,n)**: predecessors of snd(c,n) keep
   their τ positions and stage keeps snd's position — unchanged, holds by
   τ's edge-respect (`schedule_e1/e2` + E3 via `trace_monotone`).
2. **Staging cell occupancy ≤ 1**: cell c holds frame n during
   [τ(snd(c,n)), τ(rcv(c,n))−2ε]. Frame n+1 is staged at τ(snd(c,n+1)) >
   τ(rcv(c,n)) by **E2 at wire cap 1** (`schedule_e2`). No overlap. This is
   the exact point where the artifact's cap-1 wire E2 becomes the theorem's
   load-bearing input.
3. **push(c,n) after stage(c,n)**: τ(rcv(c,n))−2ε > τ(snd(c,n)) by **E1**
   (`schedule_e1_pos`).
4. **Pipe FIFO + capacity 1**: pushes occur in π_d order by construction;
   between push(f_k) at τ(rcv f_k)−2ε and deliver(f_k) at τ(rcv f_k)−ε the
   pipe holds one frame, and push(f_{k+1}) sits at τ(rcv f_{k+1})−2ε >
   τ(rcv f_k) (τ positions distinct naturals, `schedule_inj`). Occupancy
   never exceeds 1. **C₀ = 1 suffices.**
5. **Deliver never blocks**: stream c's receiver cell was emptied at
   consume(c,n−1) = τ(rcv(c,n−1)) < τ(rcv(c,n))−ε (per-channel receive
   order is seq order, `schedule_proj_canon`). Under the oracle the one-slot
   handoffs never hold a frame across any other event — the shipped demux
   discipline is satisfied with zero effective buffering, so the theorem is
   robust across every reasonable demux variant (any discipline that can
   hand the head to a ready consumer). This is the formal shadow of §5A's
   "a frame arrives only when its stream's consumer is already waiting"
   (deadlock-doc map §2.5) — achieved here with **zero control messages**.
6. **Receiver program order at consume(c,n)**: consume sits at exactly
   τ(rcv(c,n)); all its τ-predecessors are placed at or before their old
   positions (stages don't move; only wire *receives* moved, and they moved
   by −ε within their own blocks). Holds by τ's edge-respect.
7. **Both directions at once**: the construction reads one global τ; blocks
   in different directions never share a channel, cell, or pipe; no
   cross-direction constraint exists beyond what τ already ordered. ✓
8. **Closes**: the walk's `recvClose wire` / terminal cascade sits after all
   of c's receives in the consumer trace (MODEL.md §5 step 5); at its τ*
   position the pipe and cells are empty of c-frames and the producer is
   done, so the close guard (extended to "…and no c-frame in flight") is
   open. In a stricter model the protocol's real `End(Stream)` markers —
   part of the frozen message set (rust-streaming map §1.4: explicit End
   frames existed on the old mux) — ride π at exactly these positions;
   either treatment works, the End-frame one is the honest one. [derived;
   flagged as bookkeeping]

Consequence [derived]: τ* is a complete run of the muxed system — an
**explicit termination witness** in the `replaySchedule` idiom (map §2.2),
reaching `Terminal` because τ was total (`merge_complete` /
`merge_completeE`). Termination of *every* run: the ρ argument (MODEL.md §7)
extends — push/deliver events are drawn from the finite per-direction frame
budget, which is a closed-form function of sk by the counting layer
(`wiresBefore_full`, `qsBefore_full`, Counting.lean; map §3.4), so no
unbounded loop is added and the standing §7 constraint is respected.

### 1.4 From a witness run to adversarial deadlock-freedom (Tier 2)

The charter's C2 wants deadlock-freedom of the composed system with the
oracle send orders fixed and everything else adversarial: endpoint
interleaving, commit choices, any ledger-legal publication linearization.
The existing Endgame architecture lifts [derived]:

At a hypothetical reachable stuck state, decode every component onto its
trace position (the existing `pends_sound`/`pends_cover` layer, map §3.7 —
note D4+W+D1 essentially force the per-scope publication order, MODEL.md §6,
which is what makes decoding adversarial states onto the canonical traces
possible; the mux components decode trivially: pushed frames are a prefix of
π_d, delivered a prefix of pushed). Rank all unperformed events by τ* and
take the least, e. Every τ*-predecessor of e is performed, so by the §1.3
guard table e's guard is open: if e is an endpoint event this is the existing
argmin argument verbatim (internal channels untouched); if e = stage, its
cell is free (item 2: the freeing push is a τ*-predecessor); if e = push,
its frame is staged, the pipe slot free, π-predecessors pushed (items 3,4);
if e = deliver, the cell is free (item 5); if e = consume, the frame is
delivered. The choice-point pillar (`walk_uncommitted_choosable`,
Progress.lean:295) and the close-cascade close out the same way as in
Endgame/EndgameE. Contradiction; no stuck state. Mux idling is not an
enabled-action claim, so the oracle's idling never masks a deadlock: stuck
means *nothing* can move, and we exhibited a mover.

**Exact consumption list for the Lean proof** (the deliverable the
formalize phase needs):

| consumed [proven] artifact | role in the mux theorem |
|---|---|
| `merge_complete` / `merge_completeE` | π_d total; τ* reaches Terminal |
| `schedule_inj` / `scheduleE_inj` | π_d duplicate-free; τ* positions distinct |
| `schedule_e1_pos` / `scheduleE_e1_pos` | push after stage (item 3) |
| `schedule_e2` / (E twin) | staging cells never overflow (item 2) |
| `schedule_proj_canon` / `scheduleE_proj_canon` | π_d per-channel = seq order (items 4,5) |
| `trace_monotone` / `trace_monotoneE` | endpoint program order embeds in τ (items 1,6) |
| `procs_snd_owned` / `procs_rcv_owned` | direction partition of the wire family well-defined |
| Counting totals (`wiresBefore_full` …) | per-direction frame budgets; ρ extension; End accounting |
| `pends_sound` / `pends_cover` | Tier-2 decode of adversarial states |
| `walk_uncommitted_choosable` (+ opener mirrors) | choice points never wedge |
| `run_reachable` / drain + `decide` idiom | kernel pins for the mux instances |

`weave_wedge`/`weaveE_wedge` are consumed only *inside* merge completeness —
the mux proof never touches the weave directly. **New work**: the `Mux/`
state extension (`pipe : Party → List (Chan × Nat)` per the phase-1
recommendation — do NOT extend `Chan`; lean-model map §5.5), the π_d
definition (a filterMap), the τ* list refinement and its edge-respect lemma
(mechanical, ε-block bookkeeping), the extended decode for mux components,
and the Endgame-style argmin re-run. Estimated shape: Pending/Endgame-scale
rework, no new mathematics. First milestone, nearly free: a `replayMux`
executable + kernel `decide` pins running τ* to Terminal on the six pinned
skeletons (jam, parentTrap, pyramid, boundaryProbe families) at C = 1.

**Statement sketches** (both corners):

```lean
-- Mux/Model.lean: composed system = Model + per-direction FIFO + cells.
-- Wire sends become stage; new actions muxPush (strategy-gated),
-- muxDeliver (head into per-stream cell), consume reads the cell.
def demandOrder (sk : Skel) (d : Party) : List (Chan × Nat) :=
  (Sched.schedule sk).filterMap fun (c, side, n) =>
    if !side && c.isWireOf d then some (c, n) else none

-- MuxDeadlockFree sk ax σI σR C : Prop — stuck-freedom of the composed
-- system, adversarial interleaving + committed choice, mux push order
-- fixed by the σ's (which may idle).
theorem mux_oracle_deadlock_free_d5 (hwf : sk.wellFormed) (hs : sk.schedulable) :
    MuxDeadlockFree sk .full (demandPusher (demandOrder sk I))
                              (demandPusher (demandOrder sk R)) 1
theorem mux_oracle_deadlock_free (hwf : sk.wellFormed)
    (hm0 : ∀ s, sk.dCount s ≤ sk.capLevel) :
    MuxDeadlockFree sk .impl (demandPusherE …) (demandPusherE …) 1
```

### 1.5 Least capacity C₀

**C₀ = 1 per direction, in model-message units (one message = one reply
frame), and 1 is least**: a FIFO pipe of capacity 0 is not a pipe (the
rendezvous degenerate case also completes under the oracle — push/deliver
fuse — but it is outside the model's asynchronous-channel vocabulary).
**Units caveat, per the charter's model decisions** (MUX-PROGRESS §4): model
messages are replies; a provision reply's byte size is unbounded (the §5A
W = 1 unit-mismatch discontinuity, deadlock-doc map §2.7 [checked]). C₀ = 1
is a statement about message-counted capacity; the byte-denominated question
is out of the model's scope and must be recorded as a scope limitation in
the theorem's docstring, echoing "W = 1 is the unique sound reply-denominated
window."

Capacity monotonicity for the mux needs no new assumption for the oracle as
defined: σ_orc pushes at most one frame ahead regardless of C, so larger C
changes nothing; eager variants (push as early as staged+room, keeping π
order) remain covered by the Tier-2 argmin since π order alone is what the
argument uses — [derived, but re-check in Lean: the eager variant lets cells
hold frames across other events, which item 5 must then get from the argmin
rather than from block-locality].

### 1.6 The send-order variant, and what it teaches

A second oracle — push in τ-**send** order with pipe capacity ≈ Σ wire caps
(rootH/2 + 1 per direction, 17 at Rust constants) and per-stream receiver
cells — corresponds to the phase-1 observation that naive per-stream demux
reservation is bounded (MUX-PROGRESS §4, finding 1). Per-stream in-flight is
bounded by cap+hand = 2 under τ's E2, so counting says the reserved slots
never overflow along τ; but the *adversarial* version of this variant needs
its own argmin pass and is genuinely shakier off the τ path (a receiver cell
can hold an early-pushed frame while its consumer is internally gated). I
record it as [open, not needed]: demand-order/C₀ = 1 is strictly sharper.
Its value is diagnostic: it shows the shipped mux's failure was **not**
insufficient capacity but severed per-stream E2 — with either withholding
(demand order) or reservation (per-stream slots), 1–17 slots suffice; without
either, no constant suffices (deadlock-doc §4 [checked]).

### 1.7 Framing hook (GKM)

In Genest–Kuske–Muscholl terms: `merge_complete` + τ's cap-1 wire E2-respect
is precisely a kernel-proven **existential 1-boundedness** of the protocol's
MSC family per wire channel (∃ linearization with every wire channel ≤ 1 in
flight); §1.3 lifts it to existential 1-boundedness *of the single-FIFO
serialization*. C1 is then exactly the GKM implementability gap: the family
is existentially 1-bounded but (conjecturally) **no locally computable
linearization achieves the bound** — the shipped work-conserving mux behaved
as if the family were *universally* bounded, which the empirical stall
refutes [checked]. This framing is worth one paragraph in the final theorem
docs; it names why C2-positive and C1 are not in tension.

---

## 2. Necessity of nonlocal information

### 2.1 The corollary schema

Let X be the strategy class the prove-C1 lens lands (expected: deterministic
**work-conserving** local strategies — must push some staged frame whenever
the pipe has room). C1(X): ∀ (σ_I, σ_R) ∈ X, ∃ sk well-formed, schedulable,
**realizability-checked** (MODEL.md §2 soundness note — mandatory for
impossibility, see §3.4) on which the muxed session sticks.

Then, as corollaries of C1(X) + §1:

- **(Weak, definitional)** The oracle map `sk ↦ demandOrder sk` is not
  X-realizable: no strategy pair in X reproduces (or matches the safety of)
  the demand order on all of X's fooling family — immediate, since σ_orc is
  deadlock-free there and X members are not. This is the charter's "its
  dependence on remote information is essential; that necessity is exactly
  C1" (MUX-PROGRESS §1), and it should land in Lean as a two-line corollary,
  not a theorem with content.
- **(Sharp, information-level — the version worth proving)** For the fooling
  pair (sk₁, sk₂) that defeats a given σ ∈ X: the two skeletons have equal
  P-views (§3) and equal σ-observable prefixes up to the divergence step,
  and `demandOrder sk₁`, `demandOrder sk₂` diverge at a position that
  references a frame whose demand rank depends on child labels **not yet
  announced** on the wire at that step. I.e. the impossibility is located in
  the *announcement lag*, not in computational weakness of σ. This form is
  what connects C1's proof object to C2's necessity claim and survives
  statement-strength audit.

### 2.2 The information-gap characterization (the "mysterious third thing")

**Claim [derived]: the essential missing information is the per-stream E2
edge family — per-stream consumption events — and every sufficient signal is
a transport for it.**

Evidence, both directions:

- *Necessity direction*: the empirical cycle [checked, deadlock-doc §1.3,
  §3] is exactly the model's per-stream E2 replaced by aggregate pipe E2 +
  flush-paced acks ("the local mirror is credit-paced with credit = 1 on
  every edge, and the wire session silently replaced consumption-paced
  credits with flush-paced acks plus a shared FIFO. The deadlock is the
  difference between those two semantics" — deadlock-doc §3). The cap-1
  collapse experiment shows capacity conflation creates real DAG cycles
  (PROGRESS.md:106-122 [checked]).
- *Sufficiency direction*: restoring per-stream E2 by any means restores the
  proven unmuxed model. Explicitly: (a) **credits** (§5A) make each wire
  stream observationally the cap-1 channel it replaces — the composed system
  then refines the model whose two flagship theorems are kernel-proven
  [derived; §5A's own soundness argument plus the §1.3 technique]; out of
  charter scope as a mechanism (new messages), in scope as the boundary
  marker. (b) **The oracle** replaces evidence with knowledge: π_d *is* the
  receive timeline, i.e. the full future E2 edge set, precomputed from sk.
  (c) **σ\*** replaces receipt with *inference*: under FIFO delivery and
  positional pairing, received reply frames (queries ride inside reply
  frames as `Query` reactions — MODEL.md §2; there is no separate question
  wire) prove consumption of the specific own-frames that causally enabled
  them.

So the signal lattice, ordered by strength, with status:

| signal ξ added to local observation | sufficient for liveness? | status |
|---|---|---|
| full remote skeleton at start | yes, C₀ = 1, full overlap | §1 [derived, Lean-ready] |
| per-stream consumption acks (credits) | yes | [derived, §5A + refinement argument]; out of scope by charter — boundary marker |
| remote dispute frontier / labels one level ahead | computes *demand* one level out, but **cannot compute π** (τ is merge-emergent; all static/closed-form designs refuted, PROGRESS.md §4 [checked]); whether some non-π safe order becomes computable is | [open] |
| nothing (existing frames only), **idling allowed** = σ\* | the campaign hinge | [open — probe/refute-C1's burden] |
| nothing, work-conserving | conjecturally no — C1's class | [open — prove-C1's burden] |

### 2.3 What the wire already announces (inputs the other lenses need)

Answering the coordinator's sub-questions from this lens's evidence:

- **Does every receiver-side branching that affects consumption order get
  announced?** The label vector of a scope's children is fully determined at
  the answerer when it emits the scope's reply frame (it holds the asker's
  listing + its own tree), and the reply frame always flows — even when
  every child is M, the scope still has exactly one wire op (MODEL.md §2,
  §5: per D child wire+res+queries, per R child wire only, plus the parent
  summary; an all-M scope still yields its reply). The asker's own listing
  announced its side first. So label information is *eventually* on the
  wire; the sender's uncertainty is confined to the **unannounced frontier**
  (labels of children of scopes whose reply frames haven't yet crossed).
  [derived from MODEL.md §2/§5]
- **Are provision-run absorptions order-known though silent?** Silent, yes —
  an absorbed Supply and an R-reply's supplies generate zero reverse
  traffic; no consumption receipt ever exists for them. But absorption is
  **unconditional under margin 0**: the FAN counting lemma (MODEL.md §8;
  "Asm sends never block", queues.rs:73-74) means Absorb/asker-side
  absorption needs no downstream capacity, so a demand-proof discipline may
  legitimately treat delivered provision frames as consumed-on-delivery.
  **Caution for the refute lens**: this unconditionality is a margin-0
  (.impl) fact; in the `.full`/schedulable+2 corner assembler towers can
  hold, and silent frames' consumption is then capacity-gated — σ*'s
  soundness argument is likely corner-dependent. [derived]
- **Same-stream head-block finite under sound demand-proofs?** Under the
  oracle: head-block never occurs at all (§1.3 item 5). Under σ*: reduces to
  the symmetric-composition bottoming-out question — not settled here;
  flagged as refute-C1's central burden (an all-idle incomplete state with
  every demand unproven would be a *deadlock* of σ*, since idling muxes plus
  blocked processes = stuck).

---

## 3. Per-party knowledge, Lean-ready

### 3.1 Ground truth per child, and the corrected fooling alphabet

Fix a scope s with asker A and answerer W (roles alternate by height parity:
`asks`, Skel.lean:51; MODEL.md §3). Ground truth per child of s, from
MODEL.md §2 + the directionality note:

| who holds the child | skeleton label | wire appearance |
|---|---|---|
| both, hashes differ | **D** | W's reply: `Query(nonempty)` |
| both, equal | dropped (M) | W's reply: `Match` — zero channel ops |
| A only (W lacks) | **R** | W's reply: `Query(empty)`; A supplies subtree |
| W only (A lacks) | dropped (M) | W's reply: `Supply`, absorbed — zero ops |

Hence the **role-dependent fooling alphabet** for a party P whose tree is
held fixed:

- **P = A (asker of s)**: each P-held child ranges over **{D, R, M-absent}**
  (peer differs / peer lacks / peer equal). Children P lacks cannot be
  added to sk from P's side of the fence: a W-only child is M-dropped. So
  the task's {D, M(absent), R-away} is **confirmed for the asker side** —
  with "R-away" = the peer-as-answerer lacks it.
- **P = W (answerer of s)**: each P-held child ranges over **{D, M-absent}**
  only (peer equal and peer absent both collapse to M-dropped — a Supply
  reaction is zero channel ops, so the model literally cannot tell them
  apart). R is *impossible* for a P-held child (R means the answerer — P —
  lacks it). **Additionally, R children are free insertions**: any child P
  lacks that the (varying) peer holds becomes an R child P will only learn
  of from the asker's listing. This is the answerer-side wedge the task's
  alphabet misses.
- **Height 1 / leaves** (MODEL.md §2: leaves never disputed): for P = I
  (always the h1 answerer), `leafReqs` at a held h1 scope counts leaves I
  lacks — **free insertion** for fixed I-tree, bounded by fan. For P = R,
  each held leaf ∈ {requested, absent}.

So the fooling *moves* on sk with P fixed: at asker-role scopes flip held
children among {D(subskel consistent with P's subtree), R, absent}; at
answerer-role scopes flip held children between {D(…), absent} and
insert/delete R children and leaf requests freely (within fan and
well-formedness). Every move must keep `wellFormed` and (for C1 instances)
`schedulable`.

### 3.2 The P-projection (`PView`)

```lean
-- Mux/Knowledge.lean
inductive PKid where
  | held    (sub : PView)   -- a D child P holds: P participates below
  | heldCut                 -- an R child P holds (asker role): subtree
                            -- exists in P's tree, absent from Skel
inductive PView where
  | node (height : Nat) (kids : List PKid) (leafReqs : Option Nat)
-- Skel.view (P : Party) : Option PView — recursion on height:
--   at a scope where P is answerer: kids := [held (view c) | c D-child];
--     R children OMITTED (invisible to P at session start).
--   at a scope where P is asker:   kids := [held (view c) | c D-child]
--     ++interleaved [heldCut | c R-child]  (relative radix order kept —
--     sk ids are BFS so per-scope kid order is the radix order,
--     MODEL.md §2 / Skel.lean:73-107).
--   leafReqs at h=1: recorded for P = R (bounded by held leaves),
--     dropped to `none` for P = I (free insertion ⇒ not P-determined)…
```

…with one deliberate asymmetry to adjudicate in cross-examination: for
P = I, `leafReqs` is *not* view-determined (free insertion), so the view
must erase it; for P = R it is bounded but its exact value still depends on
which held leaves the peer matches — also not fully determined; safest is to
erase `leafReqs` from both views and let concrete witnesses carry it.
Similarly the *count and positions of omitted children* are not recorded —
only the relative order of visible ones — because sk has no radix gaps.

**Definition.** `Indist P sk sk' : Prop := sk.view P = sk'.view P`.
Decidable, kernel-`decide`-friendly on pinned instances.

### 3.3 What Indist is and is not — and why the strategy signature collapses

> **Correction (phase-4 F3, 2026-07-21).** The necessity claim in the
> next paragraph is **falsified by §3.1's own ground-truth table**: with
> P's tree held fixed, a P-held child at an asker-role scope ranges over
> {D, R-cut, M-absent} as a function of the *peer's* tree, and the view
> encodes those labels distinctly (`held` vs `heldCut` vs omission). So
> "same P-tree realizes both" does **not** imply equal views: `Indist`
> (and the landed `LocalEq`) is strictly FINER than session-start
> indistinguishability, not implied by it. Consequences: the landed
> nonlocality refutations survive a fortiori (their witness pair differs
> only in `leafReqs`, erased under any honest coarsening), but the
> `LocalStrategy` class is larger than charter-local, and the asker-side
> fooling moves of §3.1 are unusable through this view. The
> charter-honest re-grounding (the `PView` class binding statements to
> the honest grain) is owned by the σ*-causal track per Finch's
> statement-faithfulness ruling; `LocalEq` remains the landed controls'
> vocabulary (Strategy.lean's `LocalEq` docstring records the same
> residue).

`Indist` is a **necessary** condition for "some single P-tree realizes
both" — same P-tree ⇒ same held-child structure at every commonly-reached
scope ⇒ equal views [derived; the proptest below makes it empirical]
**[REFUTED — see the correction above: the held-child *labels* are
peer-determined, so same-tree pairs need not be view-equal]**. It is
**not sufficient**: view-equal skeletons might demand incompatible P-trees
(radix-gap arithmetic, path-compression constraints — MODEL.md §2 soundness
note says unrealizable skeletons exist). This does not weaken C1, because of
a collapse the panel should adopt:

> A real strategy is σ : (own tree, observed trace) → choice. For a fooling
> pair built from a **common concrete tree T** (i.e. sk₁ = skel(T,U),
> sk₂ = skel(T,U′)), σ's tree argument is *constant* across the pair, so on
> the pair σ acts as a function of the trace alone.

Hence the Lean statement of C1 never needs a tree type: quantify over
σ : Trace → Choice per party; `Indist` + the Rust bridge certify that the
instantiating skeleton pair is same-T-realizable, so the Lean-level
trace-only quantification *covers* every real tree-conditioned strategy on
that pair. This also disposes of the worry that strategies could condition
on tree content below R cuts or on matched content: they may, but not
distinguishably within a common-T pair (and payload-erasure, MODEL.md §1,
guarantees the *protocol's* behavior can't either).

Mid-session knowledge, for both conjectures' statements: `Obs P` = P's
projection of the run — its own commits/fires plus its receives *in order*
(FIFO makes delivered order known; MUX-PROGRESS §1's definition of local
information). A strategy is σ : Obs-prefix → Option push, evaluated at
push opportunities.

### 3.4 The Rust proptest bridge (assumption/theorem interface, README style)

Per concrete fooling pair used by any impossibility theorem, commit:

1. **Witness data**: concrete trees T, U, U′ (proptest-regressions-style
   seeds; candidates: the wide-tree seeds in `tests/pairwise.proptest-regressions`
   and `tests/shadow_validity.proptest-regressions`, per MUX-PROGRESS §2),
   with the fooling move applied in a remote-only region (flip peer child
   equal↔differing↔absent per §3.1's alphabet).
2. **Extraction check**: run both sessions (`b·c` recipe, in-memory link,
   `run_to_quiescence`) with the instrumented trace; extract each session's
   dispute skeleton from the trace (a trace→Skel decoder is new but small —
   the trace already records scopes, resolutions with pending counts, and
   radix order; deadlock-doc map §5.1); assert byte-equality with the Lean
   `Skel` literals sk₁, sk₂ (pretty-printed via `Repr`).
3. **View check (in Lean)**: `example : Indist P sk₁ sk₂ := by decide`.
4. **Realizability discipline**: MODEL.md §2/§9 — for *positive* theorems
   unrealizable skeletons only enlarge the verified set, but a C1 fooling
   pair is an ∃-witness and **must** pass step 2; a Lean-only pair that
   fails extraction is a non-finding. Record this as a campaign rule next to
   AUDIT-NOTES A2/A3.

This is exactly the shape of the existing ledger bridge ("axioms transcribe
`assert_valid`, proptest-verified on every scheduled run", MODEL.md §6), so
it inherits the artifact's trust posture without new machinery.

---

## 4. If σ* is live: what C2 becomes

### 4.1 What dies

If refute-C1 exhibits σ* (demand-lockstep: push only demand-proven frames,
else idle) deadlock-free at C = 1 with symmetric composition bottoming out,
then C1 **as literally stated** is false (idling is a legal local strategy
under the charter's definition), and with it C2's "dependence on remote
information is essential *for deadlock-freedom*". H-a survives only as
C1(work-conserving) — still valuable, since the shipped bottom-most-ready
mux is work-conserving and the class is the natural one.

### 4.2 What survives: overlap/latency necessity ("pay the bit either way")

The oracle theorem (§1) is unconditional on the probe and gives strictly
more than liveness: τ* executes **every event at its τ position** — the
composed system retains the full concurrency structure of the independent-
stream model (staging behaves bit-for-bit like the unmuxed cap-1 wire, §1.2),
with the single pipe adding only per-direction transit serialization that
any one-pipe design pays. Call this **full overlap**.

σ*, by contrast, may push a frame only when its demand-proof has *arrived*.
For consecutive same-stream D-replies the proof is the reverse-direction
reply frame whose reactions the peer fired on consuming the predecessor —
one causal wire hop per reply, competing for the reverse pipe. For provision
runs the proof is absorption-unconditionality, which (§2.3) is a margin-0
argument. So on a flat fan-F family (F sibling D scopes on one stream), σ*
spaces F pushes by F reverse hops where the oracle spaces them by zero: the
in-model separation is a wait-for chain of length Θ(F) crossing directions
that τ* simply does not have; in wall-clock terms it is §5A's measured W = 1
figure — "a thousand-leaf divergence pays ~10³ RTTs where V1 pays ~4"
(deadlock-doc §5A [checked-adjacent]). σ* is, in effect, **W = 1 credits
with the credit inferred instead of sent** — same liveness mechanism, same
serialization price, zero wire cost.

And the necessity half survives in latency form [derived, needs the probe's
exact σ* proof rule to finalize]: on a §3 fooling pair, any *live* local
strategy must idle at the divergence point until the announcing frame
arrives (pushing either way risks deadlock on one twin — that is the fooling
argument re-aimed at the push σ* would need for overlap), while the oracle
pushes immediately on the correct twin. The unannounced skeleton bit is paid
either in deadlock (work-conserving) or in idle hops (idling): information
is conserved. This is H-c made precise, and it is the theorem I recommend
the synthesis adopt **regardless of the probe verdict**, since it degrades
gracefully: if σ* is unsound the idle-hops branch is vacuous and classic C2
necessity returns via §2.1.

### 4.3 Both versions, for the synthesis to pick

- **V1 (C1 lands for class X ⊇ work-conserving):** C2 as chartered.
  Positive: `mux_oracle_deadlock_free(_d5)` at C₀ = 1 (§1.4). Necessity:
  §2.1's sharp corollary over X's fooling family; minimal-signal
  characterization §2.2 as a named companion theorem/remark.
- **V2 (σ* live):** C2 restated: *the oracle achieves deadlock-freedom AND
  full overlap at C₀ = 1; no local strategy — idling or not — achieves both
  liveness and full overlap; the minimal signal for locally achieving both
  is per-stream consumption evidence (= what credits carry), and σ* is its
  zero-wire-cost, one-hop-lagged inferential form.* Plus C1(work-conserving)
  retained with the shipped mux as the concrete instance.
- Oracle **latency-optimality** (is demand-order push optimal among safe
  single-FIFO orders at given C?) — plausible via a greedy exchange argument,
  genuinely [open]; propose as a stretch goal, not a statement of record.

---

## 5. Obstructions and honest gaps

1. **Tier-2 is a sketch, not a proof.** The §1.4 argmin lift is architected
   on the existing Endgame/pends machinery and every guard is enumerated in
   §1.3, but the τ*-refinement edge-respect lemma and the extended decode
   are real Lean work (Pending/Endgame-scale). The Tier-1 witness
   (`replayMux` to Terminal + kernel pins) is cheap and should land first.
2. **Stream closes.** The model's `recvClose` reads producer state
   shared-memory-style; over a mux this either stays an abstraction (closes
   carry no scheduling choice) or `End(Stream)` frames — already in the
   frozen message set — join π at forced positions. Decide once, in the
   model-fixing lens; both are workable (§1.3 item 8).
3. **Eager-push variants** of the oracle need the argmin (not block-locality)
   for item 5; flagged in §1.5. The C₀ = 1 lockstep oracle is the one fully
   checked here.
4. **Units.** C₀ = 1 is reply-denominated; byte-boundedness is out of model
   scope (the W = 1 unit-mismatch echo). The theorem docstring must say so
   or it overclaims against the Rust reality.
5. **View sufficiency is delegated.** `Indist` is necessary-only; each
   fooling pair owes a Rust same-T witness (§3.4). An impossibility proof
   whose pair fails extraction is void — this is a standing realizability
   rule, not a footnote.
6. **σ*'s corner-dependence.** The absorption-unconditionality input to σ*
   holds under margin 0; in the `.full`/+2 corner silent frames' consumption
   is capacity-gated (§2.3). The probe should test σ* in both corners.
7. **leafReqs erasure** in `PView` (§3.2) is a modeling choice made here
   unilaterally; cross-examination should confirm no C1 gadget needs
   view-visible leaf counts.
8. **The send-order variant** (§1.6) is stated [open]; nothing downstream
   depends on it.
