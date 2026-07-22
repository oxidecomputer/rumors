# SYNTHESIS — adjudication of record for the mux conjectures

Synthesis judge, phase-2 adjudication (MUX-PROGRESS.md §3, log 2026-07-21).
Inputs: five stage-A briefs (prove-c1, refute-c1, oracle-c2, model-fixer,
probe), two cross-examinations (attack-prove, attack-refute), the
coordinator eventdag note, the four phase-1 maps, and MODEL.md read in
full. Epistemic key as in PROGRESS.md: **[proven]** kernel-checked in-repo;
**[checked]** executable evidence (probe transcription tier or in-repo
gates); **[derived]** paper argument; **[open]** known unknown. Where the
panel disagreed, the ruling is argued, not counted.

**Frozen record (marker added 2026-07-22).** This document is the
phase-2 ruling as issued, kept verbatim with its errors and their
in-place supersession markers — the wrong projection call in §1.3 is
part of the record. Of the input briefs it cites: refute-c1, oracle-c2,
attack-prove, attack-refute remain alongside this file as frozen
records; prove-c1, model-fixer, probe, and the synthesis duplicate were
consigned to git history — MUX-PROGRESS §5's 2026-07-22 estate entry
names the last commit at which the full set was present. Every adopted
decision from the consigned briefs is restated in this document where
it is cited.

---

## 1. Verdicts

### 1.1 C1 as literally chartered: FALSE [derived, two named conditions]

C1 (MUX-PROGRESS §1: for every capacity C and every pair of deterministic
local send-order strategies there is a schedulable tree pair on which the
muxed session deadlocks) is refuted by σ* — demand-lockstep with forward
derivation (refute-c1 §1–§2): push a frame only when the receiver's
consumption of its per-stream predecessor is in Certified ∪ Inevitable,
where Certified is evidence-grounded (own events, arrivals, FIFO/program
back-closure) and Inevitable is the closure over the peer's **non-push**
events with DAG-open guards. σ* adds zero control messages, is
deterministic and local (a function of own tree + observed trace under the
§2.3 observation ruling), and is deadlock-free at **every** C ≥ 1 per
direction on the `.impl`/margin-0 class.

Decisive arguments, in order of load:

1. **The structural half is settled [checked].** The probe's omniscient σ*
   is terminal in 2,150/2,150 runs (215 skeletons × C ∈ {1,2} × 5
   interleavings, `.impl`), 174/174 under the strict certificate, 81/81
   under `.full` on schedulable non-margin-0 pins, with zero bottom-outs
   of the symmetric composition and zero fuel exhaustion (probe §4). The
   single shared FIFO per direction is not, by itself, fatal to any
   idling scheduler. Any true C1 must be informational, not structural.
2. **The informational half closes on paper [derived], by three pillars
   the cross-examiner independently re-derived and confirmed
   (attack-refute §2, §4):** (i) *self-containment* — neither closure ever
   cites an unperformed push by either side, so every demand proof is
   grounded in traffic already committed, which is why C = 1 suffices and
   why σ* halts the provision wall at exactly the right frame
   (frame-for-frame agreement with prove-c1's independent arithmetic);
   (ii) *global τ-minimality* — at a stuck candidate the τ-least withheld
   push (both parties pooled, τ = `scheduleE` [proven]) has its per-stream
   predecessor consumed (Step 3), and every label its demand proof needs
   rides a frame τ-below it, hence already pushed and delivered (Step 4);
   this is what discharges the reverse-direction symmetric coupling the
   phase-1 findings flagged; (iii) *one-scope-arrears announcement* — the
   §1.5/attack-refute-§2 enumeration of every receiver choice point is
   complete: D/R labels ride Query reactions inside reply frames that
   always flow (MODEL.md §2: even an all-M scope has exactly one wire op);
   M children are zero-channel-op on both sides (MODEL.md §2) and hence
   consumption-order-inert; asked-quota counts for consumer scope j ride
   scope j's own frames while demand proofs only ever need counts for
   scopes ≤ k−2 (prologue-wire-first consumption, MODEL.md §5.1, plus
   per-channel FIFO); provision absorptions are silent in occurrence but
   positional in order and I-step-derivable.

The two named conditions on this verdict:

- **Condition A (proof repair, mechanical).** The Keystone Lemma as
  written in refute-c1 §1.3 is unsound in its delivery case
  (attack-refute F1, CONFIRMED): DAG-predecessors-performed does not give
  pipe-head position, so an inevitable forward delivery parked behind a
  blocked head is unperformed yet unenabled. The repair is verified by
  hand: run the induction over the **push-time derivation tree**; FIFO
  ancestry makes every cited forward delivery a pre-head push (delivered
  at the stuck state), leaving only peer non-push events, where the
  counting/enabledness argument is valid. Steps 2–4 are unaffected
  (pipes-empty makes the delivery case vacuous). The Lean statement is the
  repaired form; the original phrasing must not be attempted.
- **Condition B (the residual risk).** The Step-4 coverage induction is
  the refutation's load-bearing novelty and has **zero executable
  validation** — the probe's σ* is omniscient (probe caveat 3;
  attack-refute F4). The causal (A_p-limited) σ* must be implemented in
  the probe and swept over label-latency-maximizing families
  (alternating-parity fresh-dispute chains, all-M tails, the no-peek
  gadget of attack-refute F2), plus the decidable per-state Step-4
  invariant ("every needed label rides a τ-below frame"), **before** any
  σ* Lean work. This is the one place a C1-true reversal can still hide;
  if it wedges, the failing skeleton is the fooling wedge and C1's proof,
  and the theorem suite below is built so that outcome lands as a theorem
  too (§4, stage-0 gate).

Reconciliation of the two σ* formulations: prove-c1's σ* simulates the
peer's strategy (peer-push closure) and carries the open lemma L2
("lower-bound observations suffice"); refute-c1's σ* excludes **all**
pushes from the Inevitable closure and never simulates the peer, so L2 is
moot (attack-refute §6). **The object of record is refute-c1's σ*.**
prove-c1's version is retained only as commentary on why pair
quantification matters (its row 8).

Domain ruling: the refutation is stated on `.impl` + margin-0 — the
shipping encoder's kernel-proven class (`Sched.deadlock_free` [proven]).
The charter's "schedulable" domain belongs to the `.full`/D5 corner that
no shipping encoder follows; the port (τ := `schedule`, D5's guards also
totally order per-scope publications) looks mechanical but is unargued
(attack-refute F3) and is recorded [open] as a stretch item, not a
condition on the verdict. The probe's `.full` sweep (81/81) is the
checked-tier evidence that nothing structural lurks there.

### 1.2 C1 restated over the work-conserving class (C1-WC): TRUE [derived + checked, Lean-ready]

The impossibility the charter was reaching for is real, and after
cross-examination it is **stronger than briefed** (attack-prove F1):

> **One fixed skeleton defeats everything.** There exists a single
> wellFormed, margin-0, tree-realizable skeleton `wedge` (the committed
> regression shape: root fan ≥ 5 with the first radix child deep-disputed
> and w = 4 whole-subtree provisions behind it on the same stream,
> rootH = 6 — the probe's minimal instance) such that for every pipe
> capacity C ≥ 1 and **every** pair of work-conserving strategies — local
> or not — the muxed composition reaches a stuck non-terminal state.

**[Superseded (phase 4), witness numbers: the LANDED `wedge` literal
(Mux/Instances.lean) is the 6-provision, root-fan-7
committed-regression shape — w = 4/fan ≥ 5 above describes the probe's
minimal instance, which stage 1 did not transcribe. The theorem is an
∃-witness, unaffected; the realizability bridge of record is
`src/tree/mirror/streaming/tests/wedge.rs` (deterministic pair pinned
to the literal; the committed proptest seeds realize the jam mechanism,
not the byte-exact shape).]**

Decisive arguments:

1. **The forced run needs no fooling argument** (prove-c1 §4.3, verified
   step-by-step by attack-prove §1): the adversary schedules endpoints and
   withholds R→I deliveries so that every strategy consultation happens at
   a state whose enabled-push set is a **singleton**; work-conservation
   forces the push; the strategies are never meaningfully consulted. No
   P-indistinguishability, no realizability-of-fooling-pairs burden, no
   pigeonhole. This is why the ∀-strategies quantifier goes through — and
   why the locality hypotheses can be **dropped** from the statement
   (attack-prove §3(iii): the probe's certificate-aware policy is
   effectively omniscient and dies the same way [checked]).
2. **The mechanism is slot occupation + FIFO burial under
   commit-no-retract, not pipe exhaustion** [checked + derived]: the
   probe's minimal deadlocking width is w = 4 flat across C ∈ {1..16}
   (probe §3 finding 2), matching the in-repo 64 B/16 MiB invariance
   (deadlock-doc §1.4). The consumer parks on positional assembly
   (`Pending` gates on the first disputed child, MODEL.md §4 pending
   counts); the slot holds the next provision; one provision becomes a
   permanent pipe resident; when the deep reply finally becomes
   committed-enabled, work-conservation forces its push **behind** the
   resident. Stuck-state enumeration against MODEL.md §5/§7 guards was
   done independently by the cross-examiner and pinned by the probe's
   minimized traces [checked].
3. **The class provably excludes σ*, definitionally and mechanistically**
   (attack-prove §3(iv)): at the probe's wedge states (pipe room + enabled
   frames + nothing certified) WC must push and σ* idles; on the pool, σ*
   idles on a superset of exactly the eager-deadlocking skeletons
   (24 ⊇ 20, zero eager deadlocks without σ*-idling) [checked]. **The
   right to idle, not frame choice, is the entire frontier.**

Repairs adopted from attack-prove: F1 (fixed witness, ∃sk ∀C ∀WC-pairs;
budget recast as park-position + slot + one permanent resident, with the
park position derived from the skeleton, not hard-coded); F2 (the
bounded-patience corollary is ill-typed for pure strategies — **dropped**
as a statement; the frontier is restated: for pure strategies, patience is
a per-observation binary, and work-conservation is precisely "push before
your observation changes"); F3 (D5 order slip corrected: parent precedes
the asked send in that corner — immaterial to the jam, transcription will
enforce); F4 (realizability direction flips for impossibilities — the
model-level theorem is stated over Skel, and the Rust claim lands as a
separate corollary bridged through the committed seeds, which realize the
exact shape [checked, in-repo]); F5 (commits stay adversarial, σ gates
pushes only; `commit_singleton` on the family as a named lemma — in fact
discharged generally by the `.impl` commit-totality strengthening,
attack-refute §2: W/D1/D4/D6 totally order each scope's publications, so
at every commit point exactly one obligation is choosable).

`prov C` (prove-c1 §4.2) is retained as the secondary, sender-side-blocking
instance and for the demux-variant budget lemmas; it is not the flagship.

### 1.3 C2: positive half TRUE at C₀ = 1 per direction; necessity half class-relative [derived; machinery kernel-proven; C=1 probe-checked]

**Positive half.** **[Superseded in stage 3 (P2 + track E; see the
MUX-PROGRESS log): every π_d-primacy claim in this paragraph and the
two below is REFUTED or retired. π_d — run exactly as specified here —
deadlocks a generator instance at every capacity
(`static_oracle_jams`, kernel); "the demand-order pusher is its
precomputed form" is false (the state-feedback oracle is live where
π_d jams, so they are different objects); π-eligibility fails, and its
D4 engine argument below with it; and the landed oracle of record is
neither π_d nor the state-feedback fallback but the static SEND
projection (`sendProj`, Oracle/Order.lean) — the direction this
section's "false for the send projection in general" ruled out. The
prose is retained as the adjudication's history.]**

The oracle of record is the **demand-order pusher**:
π_d(sk) = the subsequence of τ (= `scheduleE sk`) consisting of the
receive events on direction d's wire channels (a filterMap; totality =
`merge_completeE`, per-channel seq order = `scheduleE_proj_canon`, all
[proven]) — a pure function of the full bidirectional skeleton, exactly
C2's chartered input. `ofSchedule (π_d)` pushes the next π_d frame when it
is committed and the pipe has room, else idles. Deadlock-free at every
C ≥ 1, with C₀ = 1 sufficient — capacity 1, not merely "small constant"
(probe §7 finding 3 [checked]: the state-feedback oracle is live at C = 1
on everything tested, and the demand-order pusher is its precomputed
form).

The phase-1 gap ("τ's wire projection arrives in consumption order" is
not a lemma) is adjudicated per oracle-c2 §1.1: it is **false for the
send projection** in general — cross-stream skew is the pipelining the
protocol exists to have — and **dissolves for the receive projection**,
which is consumption order per-channel by construction. What remains in
the harness of record (§2.1, no staging cell) is one genuinely new lemma,
**π-eligibility**: when the τ-least unperformed event is a wire push, all
π-earlier frames of its direction are already pushed. Its engine is D4:
within a scope, every earlier D sibling's queries precede any later wire
(MODEL.md §6, D4), and the consumer's park point is always at a D child
whose asked chain therefore fired before any frame the oracle withholds —
the deep descents π needs next are never gated behind a withheld wire.
[derived here; named as the risk item of the oracle module, with the
argmin fallback below]

Fallback recorded: if π-eligibility resists, the **state-feedback oracle**
(the probe's 'exit' certificate: push when the frame's pipe-exit is
provable from committed traffic, ground truth) lands with the same
argmin machinery minus σ*'s coverage step — Steps 1–3 + the repaired
keystone with omniscient grounding, no locality anywhere. Either form
satisfies C2's charter; the π_d form is primary because it is literally
"a function of both skeletons".

C2's positive half is deliberately **not** routed through σ*: it must
survive a Condition-B reversal, and it does — nothing in it consumes the
coverage induction.

**Necessity half.** As chartered ("its dependence on remote information is
essential; that necessity is exactly C1") the corollary dies with
C1-literal and is restated class-relatively:

> Nonlocal information is necessary for **liveness under
> work-conservation** (T3 ∧ T5, a two-line conjunction), and not for
> liveness alone (σ*).

**The mysterious third thing has a name.** The signal strictly weaker than
the full remote skeleton that suffices is: the announcement prefix the
protocol already carries + FIFO positional arithmetic + inevitability
closure — i.e. *nothing new on the wire at all*. What credits smuggle
across is not information but **per-stream consumption evidence one hop
early** — an O(1)-local, in-the-moment proof of slot-drain (the per-stream
E2 edge family, which the single pipe conflates — oracle-c2 §2.2), where
σ* must run a whole-peer derivation and idle where derivation lags. σ* is
W = 1 credits with the credit inferred instead of sent. Credits (or
independence) are necessary only for liveness + work-conservation +
pipelining jointly.

**H-c (the price).** WEAKENED and demoted to executable tier. The probe's
fair-rounds metric puts σ* at 0.99× the unmuxed baseline (max 1.03)
[checked]: the model is message-counted, payload-erased, and latency-free,
which erases exactly where §5A's cost analysis lives, and the probe's σ*
paid zero informational lag besides. H-c's steep-price claim is currently
**unformalizable in the artifact's vocabulary** (probe §5); refute-c1's
"constant factor on depth·RTT" is downgraded to directional [open]
(attack-refute F9). No quantitative overlap claim enters any statement of
record; muxprobe measures overlap/lockstep-stalls for the record at
[checked] tier, and a hop-metered model extension or Rust measurement is
the (out-of-phase-3) path to more.

**GKM framing, for the docs**: the protocol's MSC family is existentially
1-bounded per direction (merge_completeE + wire-cap-1 E2 [proven] lifts to
the single-FIFO serialization via π_d), and not universally bounded
(work-conservation forces universally-bounded behavior; the wedge refutes
it). σ* shows even "no locally computable existential linearization" fails
as an impossibility — the protocol's own frames carry the full causal
structure in-band — so the true residue is exactly work-conservation.
**[Phase-4 caution: the "lifts via π_d" clause rode the refuted
π-eligibility argument. Existential 1-boundedness now rests on the
SEND-projection serialization (`oracle_deadlock_free` at C = 1), but
that re-derivation has not been written; treat GKM 1-boundedness as
[derived]-pending until phase 6 re-derives it from the send
projection.]**

### 1.4 The hinge, answered

*Is the receiver's consumption order a deterministic function of
information causally available to the sender at push time?*

**Per stream: yes, definitionally** — consumption is positional in BFS
scope order (MODEL.md §5); every order-affecting branching is announced
one scope in arrears (D/R labels in flowing reply frames; M-children
zero-op and inert; leafReqs riding h1 queries; asked-counts two scopes
behind need). **Occurrence of silent consumptions: no, but
inevitability-derivable** — provision-run absorptions generate no reverse
traffic ever, and the evidence-only demand-lockstep is genuinely refuted
by the all-M invisible scope (refute-c1 §5); what rescues σ* is deriving
non-push events forward instead of waiting for evidence, which is licensed
by payload-independence (MODEL.md §1) and Kahn determinism. **Across
streams and directions: yes at stuck states**, via global τ-minimality —
never by prediction. Derivability suffices for an idling scheduler and is
useless to a work-conserving one, which must push regardless: that
asymmetry is exactly where the impossibility class splits, and it is the
theorem-shaped content of the campaign.

---

## 2. The mux model of record

Adopting model-fixer's design with three adjudicated changes (2.1, 2.2,
2.6). Site (a) of the lean-model map §5.2: a separate wrapper; the `Chan`
inductive, obligation machine, AxMode ledgers, and 18 of 23 apply arms are
consumed verbatim; the run/drain/decide spine copies (~60 lines).

### 2.1 Topology: hand + pipe(C) + per-stream slot(1) — no staging cell

**Ruling.** The harness has, per stream, per direction: the committed
obligation in hand (MODEL.md §5's blocked-sender-holds-item device), the
shared bounded FIFO pipe of capacity C, and the base model's cap-1 wire
cell reinterpreted as the **receiver-side demux slot**. Model-fixer's
separate sender-side outbox (and oracle-c2's stage/push split, which is
the same buffer) is **not** adopted into the harness of record.

Reasons: (i) *Rust faithfulness* — the old mux's producers await the
`WriteReceipt`, which fires on write+flush (outgoing.rs:44-74 [checked]),
so the producer's program order gates on the push itself; the WriteRequest
slot is the hand, not an independent stage a producer walks away from. A
staged harness would give every strategy — including the WC class under
indictment — slack the shipped system never had. (ii) *Validation* — the
probe's calibrated transcription, both cross-examinations' arithmetic
(wall widths, park positions, singleton consultations), and all committed
traces live in this topology. (iii) *One harness* — the necessity
corollary (T7) is only clean if T3 and T5 quantify over the same pipes.

Cost accepted: oracle-c2's τ* replay (its §1.3, where the serialization
lemma is definitional) does not transfer — in this topology a withheld
push parks its producer, so τ* is not a run. C2's proof is the argmin
route (§1.3 above). The staged-mux variant and its full-overlap property
are recorded as a design remark (it is the ORACLE-side design freedom a
future latency campaign could formalize), not phase-3 scope — consistent
with H-c's demotion to executable tier.

### 2.2 State and actions

```lean
namespace Mux
inductive MObs
  | act (a : Action) | pushed (h : Nat) | delivered (h : Nat)
  deriving DecidableEq, Repr

structure MuxState where
  base : State                 -- wire cells = receiver-side demux slots
  pipe : Party → List Chan     -- FIFO, head oldest; entries Chan.wire p _ only
  hist : Party → List MObs

inductive MAction
  | base (a : Action)          -- non-wire arms verbatim; wire recv arms read the slot
  | push (p : Party)           -- σ-gated: fire a committed wire obligation into pipe p
  | deliver (p : Party)        -- pipe-p head → slot if empty; else disabled (HOL)
```

- Pipe entries are bare `Chan` tags; seq is positional (canonical-numbering
  argument, `schedule_proj_canon` idiom) — model-fixer's justification
  adopted.
- `push p` guard: some p-process holds a committed obligation on
  `wire p h`, `σ_p sk (hist p) = some h`, `(pipe p).length < C`; effect:
  `fireOblig`/`normWalk` on base, append tag, record `.pushed h`.
  `walkCommit` stays **adversarial**; σ gates pushes only (attack-prove
  F5). Under `.impl` this costs nothing: the ledgers totally order each
  scope's publications, so commits are forced (`commit_totality`,
  attack-refute §2 — mint as a lemma; it also discharges prove-c1's
  obstruction #6 and makes the probe's fused commit+fire a WLOG).
- `deliver p` guard: `pipe p = c :: rest ∧ base.chan c == 0`; effect: pop,
  `chan c := 1`, record `.delivered` to `p.other`. Shipped discipline
  (incoming.rs:60-92): FIFO head only, block-on-full. Variants (skip-scan,
  demand-driven demux) get decide-tier controls, not theorems
  (model-fixer §3 posture adopted).
- Wire `recvClose` guards strengthened: `producerDone ∧ chan c == 0 ∧
  outboxless-pipe-clear` — i.e. no c-frames in the producer's pipe
  (attack-refute F8; probe implements this). Must-fail regression pin.
- `terminal` = base terminal ∧ both pipes empty. `mstuck`, `Reachable`,
  `run`/`drain` + reachability glue: verbatim Controls.lean pattern.
- `MuxDeadlockFree sk ax C σI σR := ∀ s, Reachable … s → mstuck … s = false`.
  Endpoint interleaving fully adversarial (model-fixer decision 7,
  adopted); idling is not a move, so an idler carries a real liveness
  obligation (M3, adopted — with the note that T3's stuck states satisfy
  even the strategy-free reading, a robustness worth advertising).
- Opening wires route through the mux (model-fixer decision 6, adopted —
  old-mux faithful, signal.rs stream 0).

### 2.3 Observation ruling: slot-peek IN

`.delivered` fires at demux delivery, not at consumer consumption.
**Ratified** (attack-refute F2, reversing refute-c1 §6.5's recommendation
to prove the no-peek form): peek is load-bearing — the label-carrying
frames Step 4 needs can sit delivered-but-unconsumed in slots whose
consumer walks are parked on τ-above withheld pushes; without peek,
coverage fails, and the no-peek variant is plausibly FALSE (the
two-height mutual proof-starvation gadget). Defensible under the
charter's "everything received" and faithful to the Rust demux, which
decodes every frame before routing (incoming.rs:60-92). In the
payload-erased model, `delivered h` + the skeleton projection carries
exactly what decoded content would; bridge axiom B5 (announced-skeleton
reconstruction from a frame transcript) carries the correspondence to
Rust as a proptest. Escalated as decision-for-Finch #1.

Excluded from observation: remote delivery, remote consumption, own-pipe
occupancy drain — a consumption receipt is a covert credit and would
dissolve the frozen-message-set charter from inside the observation type
(model-fixer §2.3, adopted; decision-for-Finch #2).

### 2.4 Strategy interface and locality

```lean
def Strategy := Skel → List MObs → Option Nat        -- none = idle
def LocalEq (p : Party) (sk sk' : Skel) : Bool := …  -- PView equality (oracle-c2 §3.2)
def LocalStrategy (p) (σ) : Prop :=
  ∀ sk sk' tr, LocalEq p sk sk' → Consistent sk tr → Consistent sk' tr →
    σ sk tr = σ sk' tr
```

- Locality as invariance under `LocalEq`, not a view type (model-fixer
  §2.2, adopted), **with the `Consistent` guard added by this synthesis**:
  without it, an unreachable trace that announces the skeletons'
  difference would break σ*'s invariance vacuously-wrongly. `Consistent
  sk tr` = tr is a p-observation prefix of some reachable run under sk
  (decidable via the existing run machinery on pins; a Prop for the
  general statement).
- `PView` per oracle-c2 §3.1–3.2 with the corrected role-dependent fooling
  alphabet (asker-held children range over {D, R, M-absent}; answerer-held
  over {D, M-absent} with **free insertion of R children and leafReqs**),
  and leafReqs erased from both views. Mandatory controls: LocalEq
  nondegeneracy [decide]; the oracle is not local [decide on a LocalEq
  pair with differing π_d]; Rust proptest bridging same-p-tree pairs to
  LocalEq (committed pairwise seeds as candidate witnesses).
- The tree-argument collapse (oracle-c2 §3.3) is adopted: fooling pairs
  built from a common concrete tree make σ trace-only on the pair, so no
  tree type is ever needed in Lean; every impossibility witness pair owes
  a Rust same-tree extraction check (a standing realizability rule — a
  pair that fails extraction is a non-finding).
- `WorkConserving p σ` := at any reachable state where p holds ≥ 1
  committed wire obligation and the pipe has room, σ names one of them
  (may choose which; may not idle). `bottomMostReady` (outgoing.rs
  reverse-index poll) is the pinned concrete instance, with
  `bottomMostReady_wc` and `bottomMostReady_local`.

### 2.5 Capacity denomination

Messages (= scope replies), one shared C per direction, `1 ≤ C`
(model-fixer §4, adopted; decision-for-Finch #5). Every positive statement
carries the scope limitation verbatim: byte-level soundness of one-reply
slots is §5A's W = 1 structural argument, assumed at the model boundary.
Direction of soundness: generous-to-the-mux, so impossibility transfers to
Rust a fortiori; liveness claims are weaker than byte reality.

### 2.6 The hypothesis class

All statements of record: `.impl` + `wellFormed` + margin-0
(∀ s, dCount s ≤ capLevel) — the shipping encoder's proven class, inside
`Sched.deadlock_free` [proven], so every witness skeleton carries its own
in-context proof that only the mux is at fault. The `.full`/schedulable
corner: probe-checked for σ*-omniscient [checked]; ports recorded [open].
Notably, margin-0 is also what makes absorption unconditional (the FAN
counting lemma, MODEL.md §8) — oracle-c2's warning that σ*'s
absorption-on-delivery reasoning is corner-dependent is thereby scoped
into the hypothesis class rather than hedged in prose.

Capacity monotonicity (the artifact's standing assumption) is consumed by
**no** theorem of record — refute-c1's keystone+chase formulation dropped
it, and the build must keep it that way (a regression in this is a
finding).

---

## 3. The theorem suite of record

All controls kernel `decide` (never `native_decide`); statements on the
`.impl`+margin-0 class per §2.6. "Consumes" lists existing [proven]
machinery.

### T0. Harness + instances (Mux/Basic, Mux/Strategy, Mux/Instances)

Definitions of §2; `wedge : Skel` (regression shape, rootH = 6, w = 4,
fan ≥ 5 root) **[Superseded (phase 4), witness numbers — see §1.2's
marker: the landed `wedge` literal (Mux/Instances.lean) is 6
provisions, root fan 7]** and `prov C` (secondary family); pins:

```lean
theorem wedge_wellFormed : wedge.wellFormed = true := by decide
theorem wedge_margin0    : ∀ s, wedge.dCount s ≤ wedge.capLevel := by decide
```

Technique: decide. Size: ~450 + 350 + 250 lines. Negative control for the
close-guard extension: the unstrengthened close admits a run to a bogus
terminal with a frame in flight [decide, must-fail pin].

### T1. `commit_totality` (the `.impl` forced-order lemma)

```lean
theorem commit_totality (hwf : sk.wellFormed = true) :
    ∀ s, Reachable sk .impl s → ∀ pk, uncommittedPhase2 s pk →
      ∃! o, wkChoosable sk .impl pk (s.walk pk) o
```

W/D1/D4/D6 + per-channel child order pin the unique choosable obligation.
Consumes: `walk_uncommitted_choosable` (existence half). Technique: guard
case analysis, no reachability induction beyond what the decode layer
gives. Size: ~300–600. Serves T3 (singleton consultations), T5/T6 (decode
alignment), and retires the probe's commit+fire fusion as WLOG.

### T2. Keystone lemma, repaired form (Mux/Proofs/Chase)

```lean
-- Inevitable_p over push-time derivation trees; forward-delivery citations
-- discharged by FIFO ancestry, I-step restricted to non-push peer events,
-- guards in POSITIONAL form (E2-predecessor membership — attack-refute F6).
theorem keystone (hm0 …) (hstuck : mstuck … s = true) :
    ∀ e ∈ inevitableAt (pushTime f) …, performedAt s e
```

Plus the chase (Step-2/3 analogues): τ-well-founded descent over
unperformed events, guard-openness by counting, termination at an enabled
action or a withheld push. Consumes: `scheduleE` totality/injectivity/
edge-respect (`merge_completeE`, `scheduleE_inj`, `scheduleE_e1_pos`,
`scheduleE_e2`), Counting.lean prefix sums, `pends_sound`/`pends_cover`,
`procs_snd_owned`/`procs_rcv_owned` (SPSC stability). Technique:
closure-order induction (no reachability induction) + τ-argmin. Size:
~800–1,500. Shared by T5 and T6; built once.

### T3. C1-WC (the impossibility) — flagship, lands first

```lean
theorem wc_impossibility (C : Nat) (hC : 1 ≤ C) (σI σR : Strategy)
    (hWI : WorkConserving .I σI) (hWR : WorkConserving .R σR) :
    ¬ MuxDeadlockFree wedge .impl C σI σR
```

No locality hypotheses (stronger: even omniscient WC dies). Proof
skeleton: (a) `commit_singleton` on the family (via T1 or per-state
decide); (b) `push_singleton` — along the forced run (adversary withholds
R→I deliveries through the wall) every consultation has a singleton
enabled-push set; (c) WC + singleton ⇒ the fixed run script replays for
any pair; (d) resident lemma — slot + one permanently unabsorbable pipe
frame; (e) burial — WC forces the deep reply behind the resident (C ≥ 2)
or the pipe-full park (C = 1); (f) closed-form stuck decode. ∀C via the
two-case split: the C = 1 and C ≥ 2 final states are each uniform in C
(guards are `length < C`, monotone; the run length is constant — no
C-induction at all). Kernel anchors: `wedge` stuck replays at C ∈ {1,2,3}
in the Controls idiom (the probe's minimized traces are the run scripts).
Consumes: T0, T1, Counting totals. Technique: decide anchors + one
generic-in-σ replay lemma. Size: ~800–1,500 (down from prove-c1's 2–4k —
the fixed witness removed the C-induction).

Rust corollary (separate statement, explicitly bridged): `wedge` is
realized by the committed seed pair (`tests/pairwise.proptest-regressions`)
[checked]; large-C `prov C` realizability stays [open] and bracketed.

Controls: bottomMostReady × bottomMostReady jams `wedge` [decide — the
faithfulness pin, the deadlock doc §7-item-4 negative control];
a hand-built idling strategy completes `wedge` [decide — WC is
load-bearing]; unbounded-slot variant completes `wedge` under
bottomMostReady [decide — bounded demux state is load-bearing; the option-C
escape]; skip-scan demux on `wedge` [decide — informative either way];
C = 0 vacuity [decide].

### T4. σ* liveness (H-b; the refutation of C1-literal) — gated on stage-0

```lean
def sigmaStar : Strategy := …   -- refute-c1 §6.2: certified/inevitable closures
                                 -- over A_p, provenDemanded, height-then-seq tie-break
theorem sigmaStar_deadlock_free (hwf) (hm0) (C : Nat) (hC : 1 ≤ C) :
    MuxDeadlockFree sk .impl C sigmaStar sigmaStar
theorem sigmaStar_local (p) : LocalStrategy p sigmaStar
theorem c1_literal_false : ¬ C1Statement   -- witness ⟨sigmaStar, sigmaStar, 1⟩
```

Proof skeleton: INV-A (every pushed frame's predecessor-consumption
certified-or-inevitable at push time) + T2 keystone ⇒ pipes empty at stuck
states (Step 1); chase ⇒ a withheld push exists (Step 2); global
τ-minimality ⇒ its per-stream predecessor is consumed (Step 3); coverage —
every label the derivation needs rides a frame τ-below f*, hence pushed
and delivered, hence in A_p by slot-peek; the I-step closure replays the
ancestor set positionally (Step 4). Consumes: T2, Counting, Numbering,
`scheduleE` stack, B5 bridge axiom. Technique: the one large genuinely new
induction (coverage). Size: ~2,000–4,000. Wording fixes folded in:
demand-edge as membership not run-order (F7); the per-stream in-flight
bound stated as "slot + forward-derivable silent horizon" (F5).

Controls: σ* completes every pin (jam+m0, pdelay+m0, pyramid margins,
smokeChain, `wedge`, `prov 1..3`) [decide, drain with the σ*-driver];
**evidence-only σ* starves on an all-M instance** [decide — pins why the
Inevitable closure is needed; refutes a strategy nobody ships, minted
because it marks C1's boundary].

### T5. C2 positive half (the oracle)

```lean
def demandOrder (sk : Skel) (d : Party) : List (Chan × Nat) :=
  (Sched.scheduleE sk).filterMap fun (c, side, n) =>
    if !side && c.isWireOf d then some (c, n) else none
def ofSchedule (ord : List (Chan × Nat)) : Strategy := …

theorem oracle_deadlock_free (hwf) (hm0) (C : Nat) (hC : 1 ≤ C) :
    MuxDeadlockFree sk .impl C
      (ofSchedule (demandOrder sk .I)) (ofSchedule (demandOrder sk .R))
theorem oracle_not_local :
    ∃ sk sk', LocalEq p sk sk' = true ∧ demandOrder sk p ≠ demandOrder sk' p
```

Proof skeleton: τ-argmin (T2's chase) + **π-eligibility** (the τ-least
ready push is π-front; engine = D4's queries-before-later-wires + the
positional park-point structure). Fallback if π-eligibility resists: the
state-feedback oracle (probe's 'exit' certificate, omniscient grounding —
Steps 1–3 only, no coverage). Consumes: T2, `merge_completeE`,
`scheduleE_proj_canon`, `scheduleE_inj`, Counting, `pends_*`,
`walk_uncommitted_choosable`. Technique: argmin induction; Tier-1 first —
kernel decide drains of `ofSchedule (demandOrder …)` to Terminal on all
pins at C = 1. Size: ~1,500–3,000. Scope note in the docstring: capacity
in replies; §5A W = 1 caveat verbatim. NOT claimed: overlap/latency
optimality (executable tier only).

### T6. Necessity corollary

```lean
theorem necessity (C : Nat) (hC : 1 ≤ C) :
    (∀ σI σR, WorkConserving .I σI → WorkConserving .R σR →
        ¬ MuxDeadlockFree wedge .impl C σI σR)
  ∧ (∀ sk, sk.wellFormed = true → margin0 sk →
        MuxDeadlockFree sk .impl 1 (ofSchedule …) (ofSchedule …))
```

Two lines from T3 + T5. The prose of record states the class-relativity
and names the third thing (§1.3).

### T7. Stretch / recorded-open items (not phase-3 commitments)

- `.full`/schedulable ports of T4/T5 (mechanical-looking, unargued).
- The no-peek σ* question: probe the F2 gadget; if it wedges, mint the
  starvation control and record "peek is load-bearing" as a theorem-tier
  fact; do not attempt the no-peek liveness proof.
- The staged-mux (stage/push split) variant + τ* replay + full-overlap
  remark (oracle-c2 §1.2–1.3) — future latency campaign.
- Robust-strategy C1 (must survive every peer in the class; σ*'s pair
  move unavailable) — prove-c1 §7.7; worth a statement only if Finch wants
  an unconditional impossibility that survives σ*.

---

## 4. Phase-3 build plan

**Stage 0 — pre-Lean probe gates (blocking; one agent, the existing probe
codebase).** (P1) Implement the causal A_p-limited σ* (two finite
closures over announced positions) and sweep: all pins, 300 random
margin-0, alternating-parity fresh-dispute chains, all-M tails, the F2
no-peek gadget; plus the decidable Step-4 invariant check ("every needed
label rides a τ-below frame") per skeleton. (P2) Run `ofSchedule (π_d)`
exactly (not the state-feedback proxy) on pins + fuzz at C ∈ {1, 2}.
(P3) Mechanical re-verification of the `wedge` choice-point map
(singleton consultations) — attack-prove did it by hand; make it a probe
assertion. (P4) Construct and run the no-peek gadget.
**Gate:** if P1 wedges anywhere, C1-literal flips to TRUE with that
skeleton as the fooling wedge; T4 is replaced by the wedge theorem
(fooling machinery + LocalEq, realizability-bridged); T3/T5/T6 are
unaffected. If P2 wedges, T5 falls back to the state-feedback oracle.

**Stage 1 — harness (serial, one agent).** T0 modules
(`Mux/{Basic,Strategy,Instances}.lean`), root-manifest imports,
transcription-parity checks against the probe (same action enumeration
order so drains replicate bit-for-bit). ~1,050 lines.

**Stage 2 — parallel worktree agents.**
- **A (impossibility):** `Mux/Controls.lean` (full §3 control table,
  ~700) + T1 `commit_totality` + T3 `wc_impossibility` (~1,100–2,100).
  Depends on Stage 1 only.
- **B (shared infra):** T2 keystone + chase (`Mux/Proofs/Chase.lean`,
  ~800–1,500). Depends on Stage 1 only.
- **C (executable):** `[[lean_exe]] muxprobe` (strategy × discipline × C
  matrices, σ*×σ* bottoming probes, overlap measurement for the H-c
  record; extract `genSkel`/pins into a shared `StreamingMirror/Gen.lean`);
  `just muxprobe` gate wiring with a committed expected stall/complete
  matrix.
- **D (Rust bridges, in the mux worktree):** wedge-realizability proptest
  (committed seeds ↔ Lean `wedge` literal, trace→Skel decoder); LocalEq
  same-tree-pair proptest; B5 announced-skeleton reconstruction proptest.
  All in the `assert_valid` bridge style.

**Stage 3 — parallel, after B lands.**
- **E:** T5 oracle (`Mux/Proofs/Oracle.lean`, ~1,500–3,000), Tier-1 decide
  pins first, then π-eligibility + argmin.
- **F:** T4 σ* (`Mux/Proofs/SigmaStar.lean`, ~2,000–4,000 + `MuxInv`
  preservation ~400–700 on the flowOk template: wire conservation
  `pipeCount + chan + recvdOf = sentOf`, 5 new arms) — **gated on P1
  green**.

**Stage 4 — closure (serial).** T6 necessity; `sigmaStar_local` /
`oracle_not_local` / nondegeneracy controls; MUX-PROGRESS.md §4 findings
entries + charter amendment text for Finch; statement-strength audit round
in the house style (surface → operational → interaction → assumption);
AUDIT-NOTES.md updates (§6 below).

Gate discipline throughout: `lake build` green + kernel `decide` only on
statement paths + muxprobe matrix on any `Mux/{Basic,Strategy}` def
change + the 300-seed eventdag sweep unchanged.

Total new Lean: ~7k–13k lines, of which the two real risks are T4's
coverage induction (mitigated by the stage-0 gate) and T5's π-eligibility
(mitigated by the state-feedback fallback).

---

## 5. Decisions for Finch

Only judgment calls that change what the theorems MEAN; everything else
above is made here.

1. **Observation = slot-peek** (frames count as observed at demux
   delivery, pre-consumption). Changes what "local information" means in
   C1; the refutation is conditional on it, and the no-peek variant is
   plausibly false. Recommended: ADOPT (charter's "everything received";
   incoming.rs decodes before routing). If you rule observation =
   consumed-only, C1-literal reopens and the F2 gadget is the first test.
2. **Consumption receipts stay out of the observation type** (flush-paced
   `.pushed` only). Admitting them smuggles credits into "local" and
   likely flips C1 by definition. Recommended: ADOPT (matches
   WriteReceipt semantics).
3. **Charter amendment.** C1 as literally written is refuted (pending the
   stage-0 gate); the campaign's headline becomes the trichotomy: C1-WC
   true (one realizable skeleton kills every work-conserving pair at every
   capacity), σ* the constructive refutation of the literal form at C = 1,
   C2 true at C₀ = 1 with necessity read class-relatively, H-c demoted to
   executable tier. Approve rewriting MUX-PROGRESS §1's conjecture text to
   this resolution (the alternative — keeping C1-literal as a target —
   means betting on the stage-0 gate failing).
4. **Domain of the refutation** = `.impl` + margin-0 (the shipping
   encoder's proven class), with the `.full`/schedulable port recorded
   [open]. Accept the per-corner reading of the charter's "schedulable"?
5. **Capacity denominated in messages** (= replies), byte soundness
   assumed at the boundary via §5A's W = 1 argument. "C = 1 suffices"
   means one *reply* slot of unbounded bytes; impossibility transfers to
   Rust a fortiori, liveness does not without byte pacing.

## 6. Alignment findings (for AUDIT-NOTES.md)

1. **A1 (confirmed, sharpened).** MODEL.md §1 lists termination under
   "Proved about the model"; the Lean artifact has no kernel termination
   theorem — termination stands at the executable tier
   (`replaySchedule` gate + kernel completion pins). MODEL.md §1(ii)
   should say so, or the theorem should be minted.
2. **`schedulable ⟺ event-DAG-acyclic` is gate-checked, not
   kernel-proven** (Skel.lean docstring is honest; any prose that calls it
   proven misaligns). The mux campaign now leans on the acyclicity
   direction via `scheduleE` — no new exposure, but worth a line.
3. **Payload-independence's weight increased.** MODEL.md §1's "count and
   order of channel ops depend only on each child's merge-join arm" was an
   extraction premise; it is now the load-bearing boundary of C1's falsity
   (refute-c1 §5: if any receiver branching consumed content beyond
   labels, C1 flips true). It gains a Rust proptest (bridge B5:
   reconstruct the announced skeleton from a frame transcript) — record it
   as a promoted assumption.
4. **MODEL.md's scope statement needs a cross-reference once T3 lands**:
   "the pump's capacity-1 channel is the wire" is true of
   `mirror_connected` only; the single-pipe transport the model omits is
   now formally indicted by `wc_impossibility`. A reader must not read
   `DeadlockFree` as covering the old remote transport; MODEL.md §1's
   "Explicitly not modeled" should name the Mux/ subtree as the place
   where it now IS modeled.
5. **Capacity monotonicity is consumed by no mux theorem of record**
   (σ*'s final formulation dropped it; the probe's early embedding remark
   leaned on it, superseded). Keep it that way — any reappearance in the
   build is a finding.
6. **Probe transcription deviation, reconciled:** the probe fuses
   walkCommit+walkFire for σ* while the model of record keeps commits
   adversarial; `commit_totality` (T1) proves the fusion WLOG under
   `.impl`. Recorded so the probe is not read as modeling a different
   system.
7. **prove-c1's D5 description slip** (parent placement relative to the
   asked send; attack-prove F3) — MODEL.md is correct and the brief was
   wrong; no artifact change, but the transcription must use MODEL.md §6's
   order.
8. **The three `native_decide` cross-validation pins** remain the
   only non-kernel trust on the positive side (documented in README);
   the mux suite adds none.
