# Mux model, fixed: every modeling decision, Lean-ready

Role: model-fixer, phase-2 adjudication panel. Charter of record:
`/Users/oxide/src/rumors-mux/formal/MUX-PROGRESS.md` §1–§2 (message set
FROZEN, credits out of scope, mux model to be fixed here). Protocol spec of
record: `/Users/oxide/src/rumors-mux/formal/MODEL.md`. Empirical anatomy:
`design/streaming-wire-deadlock.md` via the deadlock-doc and rust-streaming
maps. Lean artifact facts via the lean-model map.

Epistemic tags as in PROGRESS.md: **[proven]** kernel-checked in the repo;
**[checked]** validated executably; **[derived]** paper argument here;
**[open]** known unknown. Decisions that change what the theorems *mean* are
flagged **[decision-for-Finch]** and indexed in §7.

Summary of the fix, one paragraph: the mux is a **separate state component**
wrapped around the untouched base model — a per-direction bounded FIFO
`pipe : Party → List Chan` between a sender-side per-stream outbox and the
existing `chan` cells reinterpreted (for wire channels only) as the
receiver-side demux slots. The `Chan` inductive, the obligation machine, the
AxMode ledgers, and 18 of the 23 action arms are consumed verbatim.
Strategies are total functions of (full skeleton, own-machine action
history), with locality imposed as an *invariance hypothesis* under a
skeleton-indistinguishability relation rather than by minting a view type.
The sender observes flush-paced receipts only — its own pushes — never
remote delivery or consumption. Push choice is atomic-at-writability
(matching `Mux::next`'s poll-at-write discipline), the demux is the shipped
FIFO-head/block-on-full, and capacity is denominated in messages (= scope
replies) with the §5A unit-mismatch stated as a scope limitation. H-a is
stated over the work-conserving class; C1 as literally chartered is at risk
from σ* and the statement templates are built so that either outcome of the
hinge lands as a theorem plus a kernel-checked control.

---

## 1. The mux state component

### 1.1 Anchoring against the Rust anatomy

The thing being modeled is the old single-pipe transport
(`remote/session/`, mux worktree): per direction, logical wire streams
serialized into one ordered FIFO; sender side, per-stream one-slot
`WriteRequest` handoffs feeding a mux that picks the bottom-most ready
stream at write time (`outgoing.rs:199-224`); receiver side, a sole demux
reader routing each frame into that stream's one-slot handoff and
**awaiting the send** (`incoming.rs:86-90`) — head-of-line. Receipts fire
on flush, not on remote consumption (`outgoing.rs:44-74`). [checked, from
the rust-streaming map §1.4/§4.2]

The model surface to serialize is exactly the `wire p h` family — the only
cross-party channels (MODEL.md §4, "the pump's capacity-1 channel **is**
the wire"; exposition.typ:241-248). Direction I→R carries `wire I rootH`
plus `wire I h` for odd h; direction R→I carries `wire R rootH` plus
`wire R h` for even h (incl. `wire R 0` → absorb) — rootH/2 + 1 streams per
direction. Everything else (`asked`, `upper`, `lower`, `level`,
`leafRequests`, root plumbing) is endpoint-internal and must be left alone.
[proven-adjacent: read off Model.lean:28-67]

**Decision: the opening wire messages (`wire p rootH`) route through the
mux.** The old mux carries stream 0 (opening question/reply) on the same
pipe (signal.rs stream index 0 = `UnderRoot`); only link-transport moved
the opening onto the control stream. Since the C1 instance is the old mux,
the openings are muxed. [derived; flagged §7.6]

### 1.2 The state and alphabet (Lean definitions)

Do NOT touch the `Chan` inductive (phase-1 rule: it ripples through the
23-way Preserve analysis, `allChans`/`sentOf`/`recvdOf`). Instead:

```lean
namespace Mux
open StreamingMirror

/-- One observation on a party's machine (see §2 for the alphabet argument). -/
inductive MObs
  | act (a : Action)        -- an endpoint protocol action this party executed
  | pushed (h : Nat)        -- own mux serialized a frame of wire(self, h): the flush receipt
  | delivered (h : Nat)     -- own demux delivered an incoming frame on stream h
  deriving DecidableEq, Repr

structure MuxState where
  base   : State                 -- the untouched base model state
  outbox : Chan → Nat            -- sender-side per-stream one-slot handoff (WriteRequest slot);
                                 -- invariant: zero off the wire family, ≤ 1 on it
  pipe   : Party → List Chan     -- pipe p = frames in flight p → p.other, head = oldest;
                                 -- invariant: entries are `Chan.wire p _` only
  hist   : Party → List MObs     -- per-machine observation history (newest last)

inductive MAction
  | base (a : Action)            -- one of the 23 base actions, wire sends/closes rerouted (§1.3)
  | push (p : Party)             -- mux move: strategy-chosen outbox frame → pipe p
  | deliver (p : Party)          -- demux move: pipe-p head → receiver-side wire cell
  deriving DecidableEq, Repr
```

Frame identity: entries of `pipe` are bare `Chan` tags, not `Chan × Nat`.
The seq component the phase-1 note floated is *derivable* — payloads are
opaque and identity is positional everywhere in the model (MODEL.md §4
"items are opaque… state is occupancy"), the sender pushes each stream's
frames in order, and the pipe is FIFO, so the n-th occurrence of `wire p h`
in the cumulative push history is seq n by the same canonical-numbering
argument `Numbering.lean` already proves for channel-sides
(`schedule_proj_canon`). Storing the Nat would add redundant state, a
seq-correctness invariant to maintain, and kernel-reduction weight in every
`decide`. Deviation from the §4 findings-note sketch (`muxQ : Party → List
(Chan × Nat)`), justified as above; reversible if a proof turns out to want
the tags. [derived]

### 1.3 The extended `apply`

`Mux.apply (sk : Skel) (ax : AxMode) (C : Nat) (σI σR : Strategy) :
MAction → MuxState → Option MuxState`, with:

- **`.base a`, a not touching a wire channel** (18 of 23 arms): delegate —
  `(Model.apply sk ax a s.base).map (fun t => { s with base := t,
  hist := recordAct s a })`. Verbatim reuse; one lifting lemma
  `apply_base_lift` makes every base preservation proof transportable.
- **Wire *send* arms rerouted to the outbox** — exactly three: `iopenFire`
  (opening yield), `ropenFire` on its wire obligation, `walkFire` when
  `obligChan pk o` is a wire channel. Guard `s.outbox c == 0` replaces
  `s.base.chan c < 1`; effect increments `outbox c` instead of `chan c`;
  the same `fireOblig`/`normWalk` helpers apply to `s.base` unchanged, so
  the committed-choice discipline and every AxMode ledger are consumed
  as-is. The walk still commits and blocks exactly as MODEL.md §5 requires
  — a full outbox holds the committed op, the "cannot skip ahead" device.
- **Wire *receive* arms untouched**: `walkRecvWire`, `ropenRecv`,
  `absorbRecvWire` keep reading `s.base.chan c`, which for wire channels
  now denotes the **receiver-side demux one-slot handoff**. This
  reinterpretation is the whole trick: the base cap-1 wire cell moves to
  the receiving machine, and the mux inserts outbox + pipe *behind* it.
  Receiver-side counting lemmas transport verbatim.
- **Wire *close* arms strengthened** — two: `walkCloseWire`,
  `absorbCloseWire`. Guard becomes
  `producerDone sk s.base c && s.base.chan c == 0 && s.outbox c == 0 &&
  !(s.pipe (wireParty c)).contains c` — a close is sound only when no
  frame of c can still arrive. Without the extension a receiver could
  close under an in-flight frame and manufacture spurious terminals.
  [derived; this is a soundness fix the wrapper *must* make, analogous to
  the phantom-walk lesson — it gets a must-fail regression pin]
- **`.push p`**: guard
  `(s.pipe p).length < C && σ_p sk (s.hist p) = some h &&
  s.outbox (wire p h) > 0`; effect: outbox −1, `pipe p := pipe p ++
  [wire p h]`, append `.pushed h` to `hist p`. Atomic
  choice-at-writability, no commit phase: the shipped mux re-polls its
  ready set each time the sink is writable (`Mux::next`, rust-streaming
  map §2.2), so choice and serialization coincide; and once serialized a
  frame cannot be retracted, which the FIFO append captures. A
  pick-early/committed-push variant is *weaker information* for σ (stale
  hist at fire time); H-a over atomic-push strategies is therefore the
  stronger impossibility and subsumes it. [derived]
- **`.deliver p`**: guard `s.pipe p = c :: rest && s.base.chan c == 0`;
  effect: pop, `chan c := 1`, append `.delivered h` to `hist p.other`.
  Head-only, blocks (is disabled) while the target cell is full — the
  shipped HOL discipline (§3).

`allMActions sk = (Model.allActions sk).map .base ++ [push I, push R,
deliver I, deliver R]`. Conservativity note transfers: omissions make
`mstuck` easier and `MuxDeadlockFree` harder (Statement.lean:127-131
argument, verbatim).

### 1.4 Lifting Reachable / stuck / terminal / run / drain

```lean
def Mux.init (sk) : MuxState := ⟨Model.init sk, fun _ => 0, fun _ => [], fun _ => []⟩

def Mux.terminal (sk) (s : MuxState) : Bool :=
  Model.terminal sk s.base && (s.pipe .I).isEmpty && (s.pipe .R).isEmpty
    && (wireChans sk).all (fun c => s.outbox c == 0)

def Mux.canStep … := (allMActions sk).any (fun a => (Mux.apply … a s).isSome)
def Mux.mstuck … := !terminal … && !canStep …
inductive Mux.Reachable … -- init + closure, verbatim pattern
def Mux.run / Mux.drain    -- verbatim copies of Controls.lean:197-260 pattern
theorem run_reachable / drain_reachable -- verbatim glue
```

The pipe/outbox emptiness conjuncts in `terminal` are redundant given flow
conservation (a base-terminal state has every wire recv fired), but stating
them conjunctively avoids needing that lemma before the definition exists;
prove the redundancy later as `terminal_drained` if a proof wants it.
`Deadlock = reachable non-terminal state where nothing moves` is then
exactly the charter §2 definition, and the composed system includes mux and
demux moves as required. The `run_reachable`/`decide` spine — the ITF
bridge and every kernel-checked negative control — copies over at the cost
of ~60 lines (lean-model map §5.2 estimate confirmed). [derived]

```lean
def MuxDeadlockFree (sk : Skel) (ax : AxMode) (C : Nat) (σI σR : Strategy) : Prop :=
  ∀ s, Mux.Reachable sk ax C σI σR s → Mux.mstuck sk ax C σI σR s = false
```

Note the quantifier posture this inherits, deliberately: strategies fix the
*push* choices, but the endpoint interleaving (which process runs, which
obligation a walk commits to among ledger-legal ones) stays fully
adversarial, exactly as in the base `DeadlockFree`. A strategy must survive
every local schedule; conversely a refutation of `MuxDeadlockFree` may pick
the endpoint schedule. This matches reality (tokio readiness order is
scheduler-dependent — rust-streaming map §2.4) and is load-bearing for the
H-a proof technique (§5.1). [derived; flagged §7.7]

### 1.5 What transports, precisely

- **Verbatim, zero new proof**: all of `Counting.lean` (pure functions of
  Skel); the Sched/Numbering/MInv stack (explicitly trace-generic —
  Sched.lean:297-303 "none of it looks inside a trace"); the Progress
  pillar (`walk_uncommitted_choosable` — commit guards are untouched);
  `wellFormed`/`schedulable`/instances/pins.
- **Via the lifting lemma**: the 18 untouched arms' Preserve cases, for
  any invariant whose conjuncts on those arms only mention `s.base`.
- **Not verbatim — the one real port**: `flowOk`. For wire channels the
  conservation equation becomes
  `outbox c + pipeCount p c + chan c + recvdOf c == sentOf c`, with
  `chan c ≤ 1`, `outbox c ≤ 1`, `(pipe p).length ≤ C`; non-wire conjuncts
  verbatim. A new `MInv := base-Inv-with-wire-flow-generalized` needs new
  preservation proofs for exactly 7 cases: 3 rerouted send arms, 2
  strengthened closes, push, deliver. Estimate ~400–700 lines following
  the guard-mirroring house style. The 23-way base Preserve is **not**
  redone. [derived]

---

## 2. The strategy interface

### 2.1 The type

```lean
/-- A send-order strategy: given the session skeleton and this machine's
    observation history, name the stream to push next, or idle. -/
def Strategy := Skel → List MObs → Option Nat
```

Total function ⇒ **determinism is free** (the charter's "deterministic
local-information-only" is a function, full stop). `none` = idle, which the
charter's definition of a local strategy permits — this is the door σ*
walks through, and H-a closes it again with a hypothesis (§2.4).

### 2.2 Locality as invariance, not a view type

The model has no per-party trees — the genuinely new definition flagged by
phase 1. Rather than minting a `SkelView` type (which would freeze one
representation of party knowledge into every statement), give σ the **full
skeleton** and impose locality as an invariance hypothesis over the
oracle-c2 panel's indistinguishability relation:

```lean
-- consumed from the oracle-c2 / refuter panels' definition of record:
def LocalEq (p : Party) (sk sk' : Skel) : Bool := …

def LocalStrategy (p : Party) (σ : Strategy) : Prop :=
  ∀ sk sk' tr, LocalEq p sk sk' = true → σ sk tr = σ sk' tr
```

Why this shape: (i) it is the standard Lean-friendly encoding — σ may
*read* remote structure but provably cannot *use* it; (ii) the C2 oracle is
then literally "a Strategy that is not LocalStrategy", so the necessity
corollary is a statement about one hypothesis; (iii) the fooling argument
for H-a is directly usable (two LocalEq skeletons, one run prefix, σ forced
to the same choice on both). The definition of `LocalEq` itself is where
C1's meaning lives — too coarse and C1 is trivially true, too fine and
strategies see remote information. I consume it, I do not fix it here.
**[decision-for-Finch §7.1]**, with two mandatory controls regardless of
its content: a kernel `decide` exhibiting two *distinct* skeletons that are
LocalEq (nondegeneracy — else `LocalStrategy` is vacuous), and a Rust
proptest bridging concrete tree pairs (same p-side tree, different
skeletons) to LocalEq, in the established `assert_valid` bridge style
(README.md assumption/theorem interface; the committed pairwise seeds are
candidate witnesses — MUX-PROGRESS.md §2 crux 1).

### 2.3 The observation alphabet, argued

`MObs` (§1.2) records, per machine: every base action the machine executed
(`.act`), its own completed pushes (`.pushed`), and its own demux's
deliveries of incoming frames (`.delivered`). The load-bearing exclusions
and inclusions:

- **The sender does NOT observe remote delivery or consumption.**
  RECOMMENDED and adopted. Ground truth: `WriteReceipt` fires on
  write+flush, not remote consumption (`outgoing.rs:44-74, 128-155`)
  [checked]. In the model the flush receipt IS the `.pushed` event — the
  push action's own completion. A consumption-receipt observation would be
  a reverse-direction signal carrying exactly the information credits
  carry (remote demux progress), i.e. a covert credit — admitting it would
  dissolve the charter's frozen-message-set boundary from inside the
  observation type. This exclusion is what makes the sender's ignorance of
  its own outgoing pipe's occupancy *honest*: p knows its pushes, never
  their drain. [derived; **decision-for-Finch §7.2** since it changes
  theorem meaning]
- **Everything on the machine is observable.** `.act` includes the
  machine's own protocol actions (commits, fires, internal sends/recvs),
  and `.delivered` its incoming demux events. Maximal locality strengthens
  H-a (impossibility against the best-informed local strategies) and costs
  H-b nothing (σ* uses a subset). It also makes σ well-defined without a
  state peek: a machine's full local state is a deterministic function of
  its own action history given its skeleton knowledge, so passing `hist`
  alone loses nothing — and what it *cannot* reconstruct (pipe occupancy,
  remote state) is exactly what it must not know. [derived]
- **FIFO delivery makes delivered-order known to the receiver** — the
  `.delivered` subsequence is the push order restricted to arrivals — and
  the *sender's* knowledge of the receiver's consumption order rides only
  on frames the receiver sent it (questions/reactions in `.delivered`
  frames) plus its own `.pushed` sequence under the FIFO discipline. This
  is precisely the causal budget the hinge question is about; the
  observation type neither adds to nor subtracts from it. [derived]

### 2.4 Work-conservation and the strategy classes

```lean
/-- σ never idles while it holds a pushable frame: whenever some outbox
    slot on p's machine is occupied, σ names an occupied one. -/
def WorkConserving (p : Party) (σ : Strategy) : Prop :=
  ∀ sk (s : MuxState), MuxReachableAny sk … s →
    (∃ h, s.outbox (Chan.wire p h) > 0) →
    ∃ h, σ sk (s.hist p) = some h ∧ s.outbox (Chan.wire p h) > 0
```

(Technically stated over reachable `(hist, outbox)` pairs; pipe room enters
at the push guard, so a WC strategy may still *wait* on a full pipe — it
may never *decline*.) The shipped mux is the concrete instance:

```lean
/-- Bottom-most-ready: deepest (lowest-height) occupied outbox slot wins —
    outgoing.rs:199-224's reverse-index poll. Memoryless: a function of the
    outbox occupancies, which are reconstructible from hist. -/
def bottomMostReady : Strategy := …
theorem bottomMostReady_wc : ∀ p, WorkConserving p bottomMostReady
theorem bottomMostReady_local : ∀ p, LocalStrategy p bottomMostReady
```

σ* (demand-lockstep) is `Strategy`-typed with `LocalStrategy` by
construction and `¬WorkConserving` on the H-a witness family — which is
itself the negative control showing WC is load-bearing (§5.5). The C2
oracle strategy `ofSchedule (oracleOrder sk) p` (push in τ's per-direction
wire projection order, idle when the next frame is not yet in the outbox)
is `Strategy`-typed and **not** `LocalStrategy` — pinned by a `decide` on a
LocalEq pair where the two oracle orders differ.

---

## 3. The demux discipline

**Fixed to the shipped one**: single logical reader per direction, FIFO
head only, deposit into the target stream's one-slot cell, disabled (=
blocked) while that cell is full (`incoming.rs:60-92`) [checked]. This is
`.deliver` as defined in §1.3. The theorems are stated *for this
discipline*; it appears as the definition, not a hypothesis — variants get
their own definitions if pursued.

Enumerated variants and robustness posture [derived]:

1. **Skip-scan** (deliver the earliest pipe entry whose target cell is
   free; per-stream order preserved, cross-stream HOL removed). The
   empirical cycle's link 4 dies, but the impossibility mechanism
   relocates rather than disappears: frames for a parked stream accumulate
   in the *pipe* (up to C) instead of jamming the *head*, and the pipe
   fills with undeliverable frames before the demanded frame can be
   pushed. H-a should survive with the pigeonhole moved from slot slack to
   pipe capacity and the witness family's provision count re-scaled;
   **conjectured robust, not claimed** — worth one executable probe, not a
   phase-3 theorem. Implementation: a second `deliver` definition, one
   constructor swap.
2. **Demand-driven demux** (deliver only when the consumer is already
   parked on the cell). Strictly less eager than shipped; it changes when
   cells fill but not what the sender knows, and the sender's information
   gap is where H-a lives. Conjectured equivalent for H-a; irrelevant for
   H-b/C2 (see below). Not pursued.
3. **Unbounded per-stream demux buffers** — the design doc's rejected
   option C. Makes C1 trivially false (deadlock-doc map §1.4 [checked]:
   the stall reproduces at 64 B and 16 MiB pipes; the cycle is endpoint
   demux state, not pipe capacity). Excluded by construction here: the
   receiver cell is the base model's cap-1 wire cell. The **boundedness of
   total endpoint demux state is thus structural in this model**, which is
   exactly how MUX-PROGRESS.md §4's "C1 must bound endpoint demux state or
   it is trivially false" caveat gets discharged. A `decide` control on an
   unbounded-cell variant (guard `chan c < BIG`) completing the H-a
   witness pins that the bound is load-bearing (§5.5).

**Robustness freebie for the positive theorems**: under σ* (and under the
C2 oracle), the intended proof shows every pushed frame's consumer demand
is already proven, so the head cell is always eventually drainable and
`deliver` never blocks persistently — the liveness argument never leans on
the discipline's eagerness. If that lemma lands, H-b/C2 are
discipline-insensitive across variants 1–2 *by inspection of the proof*,
and only H-a needs the discipline named. State this as a remark in the
module doc, not a theorem. [derived]

---

## 4. Capacity denomination

**Decision: messages, where one message = one scope-level reply = one
`wire p h` model send.** `C : Nat`, `1 ≤ C`, per direction; one shared
parameter for both directions (asymmetric capacities are a free
generalization nobody needs yet). [derived; **decision-for-Finch §7.3**]

Justification against the alternative (chunk counts):

- The model's channels are message-counted and payload-erased (MODEL.md
  §1 "Payloads", §4); message denomination is the only unit the existing
  counting layer speaks.
- The byte-unboundedness caveat, stated as the scope limitation it is: a
  reply that answers an R child is a whole-subtree provision run —
  unbounded bytes, unbounded frames in the Rust framing
  (`design/streaming-wire-deadlock.md` §4, §5A's unit-mismatch
  discontinuity: reply-denominated windows > 1 are unsound because "a
  grant denominated in replies can never be covered by a buffer
  denominated in frames"; W = 1 is the unique sound reply-denominated
  point). A message-denominated pipe slot therefore holds unboundedly
  many real bytes.
- Direction of soundness, made explicit in the statements' prose:
  message-denominated capacity is *generous to the mux*. For **H-a
  (impossibility)** this makes the theorem STRONGER — if every
  work-conserving strategy deadlocks even when whole replies ride in
  single slots, it deadlocks a fortiori under byte framing. For **H-b/C2
  (liveness)** it makes the theorems WEAKER than byte reality, and each
  positive statement carries the caveat verbatim: *"capacity is counted
  in scope replies; the byte-level soundness of holding one reply per
  slot is the §5A W = 1 structural argument (the actively decoded reply
  needs zero buffer because the consumer never parks mid-reply), assumed
  at the model boundary, not proven here."* Note the mitigation: under
  σ*/oracle a frame is pushed only against proven demand, so parked
  full-reply slots — where the unit mismatch bites — are exactly what
  the disciplines exclude. [derived]
- Chunk counts were considered and rejected: interior R children are
  *childless in Skel by construction* (MODEL.md §2 — the answerer lacks
  the whole subtree), so provision volume is not derivable from the
  skeleton; modeling `leafReqs`-derived chunk volume would require
  enriching `Skel` with subtree sizes, rippling through `wellFormed`, the
  counting layer, and the instance pins — disproportionate to what it
  buys, and it still would not make the pipe byte-accurate (frame bodies
  are arbitrary `Message<T>`). If a future campaign needs byte fidelity,
  the honest unit is bytes with a size oracle, which is a different
  model. [derived]

Echo worth pinning in prose: per-stream buffering between a sender's
choice and the consumer is outbox (1) + shared pipe (C) + receiver cell
(1) — at small C this reproduces the empirical "≈ 3 frames of per-stream
slack" (demux slot + ProxyResponses slot + in-flight decode) that the
regression shape had to beat [checked, deadlock-doc §1.2]. The witness
family's provision count scales as C + 2 + O(1) accordingly.

---

## 5. Statement templates, proof budgets, controls

Throughout: `ax = .impl` (the shipping encoder's order; H-a must indict
the *composition*, not a send-order the encoder doesn't have), margin-0
capacity hypothesis on `capLevel` as in the flagship, `decide` not
`native_decide` for every control.

### 5.1 H-a: impossibility over the work-conserving class

```lean
theorem workConserving_deadlocks
    (σI σR : Strategy)
    (hLI : LocalStrategy .I σI) (hLR : LocalStrategy .R σR)
    (hWI : WorkConserving .I σI) (hWR : WorkConserving .R σR)
    (C : Nat) (hC : 1 ≤ C) :
    ∃ sk : Skel, sk.wellFormed = true ∧ (∀ s, sk.dCount s ≤ sk.capLevel) ∧
      ¬ MuxDeadlockFree sk .impl C σI σR
```

The tree pair is "well-formed and schedulable, i.e. the un-muxed protocol
provably completes" per the charter — margin-0 subsumes schedulable
(`margin0_schedulable`), and `Sched.deadlock_free` on the same sk is the
in-context witness that only the mux is at fault.

**Proof technique — forced-run + choice-set-singleton lifting, NOT
per-σ decide.** σ is a function variable, so no single `decide` closes
this. But the refutation only needs ∃ one reachable stuck state, and the
endpoint interleaving is ours to choose (§1.4). The plan:

1. `hard C : Skel` — the regression shape parameterized by C: root fan
   C + 3-ish, first child (radix-first) disputed two levels deep, C + 2
   provision (R) children behind it on the same stream (deadlock-doc §5.3
   witness shape; wellFormed/margin-0 by construction, pinned by decide).
2. A concrete endpoint schedule (we pick every non-push action) under
   which, at every push decision point along the critical prefix, **the
   pusher's outbox has exactly one occupied slot** — the fatal provision
   frames become available strictly before the frame demand will later
   name (the deep answer cannot exist before the questions that mint it,
   which cannot exist before deliveries we sequence late). This is the
   empirical anatomy transcribed: the initiator flushes h30 answers
   "before the h29 questions that will demand h28 answers even exist"
   (§5D [checked]).
3. The lifting lemma: `WorkConserving p σ` + singleton occupied-outbox ⇒
   σ's choice at that point is forced. Hence every WC pair traverses the
   *same* run prefix — the run is strategy-independent. Small clean
   induction, generic in σ.
4. The prefix ends in a state where outbox + pipe + cell on the disputed
   stream's direction are saturated with undeliverable provisions and the
   consumer is parked on the assembler's positional `Pending` (links 1–6
   of the cycle); `decide` the concrete stuck fact at small C, or prove
   the parametric stuck state by the counting layer for general C.

Budget: the forced-run construction and stuck fact at C ∈ {1, 2, 3} are
kernel `decide`s in the Controls idiom (~600 lines incl. the family). The
lifting lemma ~300 lines. **General C is the phase-3 risk item**: a
parametric run needs either an induction constructing the schedule as a
function of C (moderate; the run is length-linear in C) or acceptance of
per-C anchors plus a [derived] general argument. Plan for the induction,
budget 1.5–3k lines, fall back to anchors with the gap recorded. [open at
general C; the rest derived]

**Honest gap to record**: step 2's singleton-choice property must be
*verified on the actual family*, not assumed — if at some point two
streams' outboxes are simultaneously occupied and one choice escapes the
trap, WC alone does not force the deadlock and the theorem needs either a
harder family or a per-choice-point case split (finite, decide-able, but
the tree can blow up: k choice points of width w = w^k branches; keep
k ≤ ~5). The executable simulator probe should map the choice points on
`hard 1` before any Lean is written.

### 5.2 H-b: σ* witness liveness at C = 1

```lean
def sigmaStar : Strategy := …   -- demand-lockstep; def consumed from the
                                 -- refute-C1 panel, must be computable and
                                 -- LocalStrategy by construction

theorem sigmaStar_deadlock_free
    (hwf : sk.wellFormed = true) (hm0 : ∀ s, sk.dCount s ≤ sk.capLevel) :
    MuxDeadlockFree sk .impl 1 sigmaStar sigmaStar
```

Proof technique: a genuine ∀-interleaving progress proof — the MInv-style
architecture: a mux invariant (§1.5) + τ-argmin ranking with the event
universe extended by push/deliver events (each `wire p h` seq n mints
`push(p,h,n)` and `deliver(p,h,n)` events sandwiched between the base
snd/rcv — the schedule-level site (b) vocabulary). The two new lemmas that
are the actual content:

- **Demand-proof soundness**: whenever σ* pushes, the receiver's
  consumption of that frame is not blocked-forever — the pushed frame's
  cell drains. This is the hinge (§8); if it fails, this theorem is
  unprovable and the failing skeleton is C1's fooling wedge.
- **Symmetric bottoming-out**: no reachable state has both directions
  idle-with-unproven-demands while non-terminal — demand-proofs are
  generated by protocol progress that σ* itself never blocks, induction
  along τ. [open — the campaign's hinge, reduced by this model to (i) a
  per-instance `decide` on jam/parentTrap/pyramid/`hard C` under
  σ*×σ* before proving (the drain technique runs any concrete strategy),
  and (ii) the two lemmas above]

Budget: comparable to an Endgame adaptation, 2–5k lines, *contingent on
the hinge closing*. Do the executable probe first: `Mux.drain` under
σ*×σ* on every pinned family is an afternoon and settles whether to
attempt the proof at all.

### 5.3 C2: oracle liveness (and what is explicitly not claimed)

```lean
def oracleOrder (sk : Skel) (p : Party) : List Nat :=
  (Sched.scheduleE sk).filterMap …   -- τ's wire projection, per direction
def ofSchedule (ord : List Nat) : Strategy := …  -- push in fixed order, idle if next absent

theorem oracle_deadlock_free
    (hwf : sk.wellFormed = true) (hm0 : ∀ s, sk.dCount s ≤ sk.capLevel) :
    MuxDeadlockFree sk .impl 1 (ofSchedule (oracleOrder sk .I))
                               (ofSchedule (oracleOrder sk .R))
```

The oracle exists kernel-proven (`scheduleE` + `merge_completeE` +
`scheduleE_e1_pos`); the new content is the **serialization lemma** — τ's
per-direction wire projection arrives in receiver-consumption order, so at
C = 1 the head is always the receiver's next need and no HOL forms. Site
(b) proof over the trace-generic merge machinery, embedded back into the
site (a) transition system by a replay argument (the `replaySchedule`
pattern, upgraded from gate-checked to a kernel lemma on the mux system).
Budget 1–3k lines. **Not claimed as a theorem**: C2's *performance* half
(full streaming overlap, H-c's latency contrast with σ*). Overlap is a
quantitative property with no existing formal vocabulary; it stays
executable-tier — muxprobe measures per-stream in-flight overlap and
lockstep stalls for σ* vs oracle on the pinned families, and H-c's
serialization-price claim is recorded [checked], not [proven].
**[decision-for-Finch §7.5]**

### 5.4 Necessity corollary

```lean
theorem necessity (C : Nat) (hC : 1 ≤ C) :
    (∀ σI σR, LocalStrategy .I σI → LocalStrategy .R σR →
       WorkConserving .I σI → WorkConserving .R σR →
       ∃ sk, sk.wellFormed = true ∧ margin0 sk ∧ ¬ MuxDeadlockFree sk .impl C σI σR)
  ∧ (∀ sk, sk.wellFormed = true → margin0 sk →
       MuxDeadlockFree sk .impl 1 (oracle …) (oracle …))
```

Pure conjunction of 5.1 and 5.3 — the corollary the charter wants, but
**class-relative**: "nonlocal information is necessary for
liveness-with-work-conservation." If σ* lands (5.2), C1 as literally
chartered is FALSE, and the sharp residual statement is H-c's: credits (or
the skeleton) are necessary for *liveness and overlap jointly*, where the
overlap half is executable-tier. The Genest–Kuske–Muscholl framing fits
this shape exactly — the protocol MSC family is existentially B-bounded
for B = 1 (C2/5.3: some linearization fits a 1-bounded pipe) while no
work-conserving locally-computable linearization achieves any bound
(5.1) — and σ*'s status decides whether "work-conserving" can be dropped.
The templates above are deliberately built so both outcomes are landable
theorems, not statement rewrites.

### 5.5 Negative-control discipline (every hypothesis pays rent)

Per house style, each new hypothesis/definition gets a kernel-`decide`
control showing it is load-bearing:

| hypothesis / def | control | shape |
|---|---|---|
| `WorkConserving` in 5.1 | an idling strategy survives `hard C` | `decide`: drain under σ* (or a hand-built withholding σ) on `hard 1` reaches terminal — if the hinge closes; else a weaker hand-idler on a sub-family |
| `LocalStrategy` in 5.1 | the oracle is not local | `decide`: LocalEq pair `(sk, sk')` with `oracleOrder sk ≠ oracleOrder sk'` |
| `LocalEq` nondegenerate | locality is not vacuous | `decide`: distinct sk, sk' with `LocalEq p sk sk' = true` |
| bounded receiver cell | option C escape is real | `decide`: unbounded-cell variant (`chan c < 2^32` guard) completes `hard 1` under `bottomMostReady` |
| FIFO-head demux | HOL is load-bearing at the witness | `decide`: skip-scan variant on `hard 1` — record which way it falls (informative either way; if it also jams, the robustness conjecture in §3.1 gets an anchor) |
| `1 ≤ C` | vacuity guard | C = 0 makes every push disabled; `decide` mstuck at init on any sk with a wire |
| shipped-mux faithfulness | the model reproduces the bug | `decide`: `bottomMostReady × bottomMostReady` on `hard 1` reaches mstuck — the model twin of the committed pairwise seed, and the deadlock doc §7 item 4's pre-scoped negative control ("the credit-less extended model should refute progress on §2's skeleton") |
| close-guard extension (§1.3) | must-fail pin | `decide`: the unstrengthened close admits a run to a bogus terminal with a frame in the pipe |

### 5.6 Size roll-up

| module | content | est. lines |
|---|---|---|
| `Mux/Basic.lean` | MuxState, MAction, apply, init/terminal/stuck/Reachable/run/drain | ~450 |
| `Mux/Strategy.lean` | MObs plumbing, Strategy, LocalStrategy, WorkConserving, bottomMostReady, ofSchedule, sigmaStar | ~350 |
| `Mux/Instances.lean` | `hard C` family, LocalEq witnesses, wellFormed/margin-0 pins | ~250 |
| `Mux/Controls.lean` | the §5.5 table | ~700 |
| `Mux/Invariant.lean` | MInv + 7-case preservation | ~700 |
| `Mux/Proofs/Impossibility.lean` | forced run, lifting lemma, general-C induction | 2–4k |
| `Mux/Proofs/SigmaStar.lean` | contingent on hinge | 2–5k |
| `Mux/Proofs/Oracle.lean` | serialization lemma + replay | 1–3k |

---

## 6. Build and module plan

- **Layout**: `formal/lean/StreamingMirror/Mux/{Basic, Strategy,
  Instances, Controls, Invariant}.lean`, proofs under
  `StreamingMirror/Mux/Proofs/`. Import spine: `Mux/Basic` imports
  `StreamingMirror.Model`; `Mux/Strategy` imports Basic; proofs
  additionally import `Proofs.Sched` (+ Numbering) for the Ev/merge
  vocabulary and `Proofs.Counting`. Nothing imports the Preserve/Weave
  stack unless MInv work wants a lemma. Add the `import
  StreamingMirror.Mux.…` lines to root `StreamingMirror.lean` — the root
  import list IS the build manifest (lean-model map §4).
- **Executable**: a NEW `[[lean_exe]] muxprobe` (root `MuxProbe.lean`)
  rather than growing `eventdag` — eventdag is 1700+ lines with a single
  responsibility (the base event DAG oracle); the mux probe's job is
  different (strategy × discipline × C matrices, σ*×σ* bottoming-out
  probes, overlap measurement for H-c's [checked] tier). One refactor to
  flag: extract `genSkel` and the pinned families from `EventDag.lean`
  into a shared importable module (`StreamingMirror/Gen.lean`) so muxprobe
  fuzzes the same distribution — small, mechanical, touches eventdag's
  imports only.
- **Gate additions**: `just muxprobe` (matrix over pinned families +
  fuzz seeds; exit nonzero on any deviation from the expected
  stall/complete matrix, which is committed alongside like README's
  instance matrix); `lake build` already covers the new modules via the
  root manifest; the 300-seed eventdag sweep discipline extends: any
  commit touching `Mux/Basic` or `Mux/Strategy` definitions runs the
  muxprobe matrix. Same trust posture: `decide` only on the statement
  path; muxprobe is gate-tier.
- **Rust proptest bridge points** (the assumption/theorem interface, per
  README house style):
  1. *Faithfulness*: the model's `bottomMostReady`+FIFO-head on `hard 1`
     jams (kernel) ⟷ the mux worktree's committed pairwise seed jams
     (already-committed regression, `tests/pairwise.proptest-regressions`)
     — plus a new proptest asserting the Lean `hard` shape is realizable
     as a tree pair (realizability check per MODEL.md §2 soundness note).
  2. *LocalEq*: proptest generating tree pairs with equal p-side trees,
     asserting their skeletons are LocalEq — the indistinguishability
     bridge (MUX-PROGRESS.md §2 crux 1).
  3. *If σ* lands*: a prototype demand-lockstep scheduler in the mux
     worktree run against the committed seeds must complete — checked-tier
     evidence that σ*'s information is really available at the Rust layer
     (flush receipts + received frames only).
  4. The existing `assert_valid` lockstep carries unchanged — base
     obligations and ledgers are untouched by construction.

---

## 7. Decisions-for-Finch (each changes what a theorem means)

1. **`LocalEq` definition** (§2.2) — the meaning of "local information";
   consumed from the oracle-c2/refuter panels; nondegeneracy control
   mandatory.
2. **Observation alphabet excludes remote consumption receipts** (§2.3) —
   flush-paced only, per the Rust. Admitting consumption receipts would
   smuggle credits into the observation type and likely flips C1.
3. **Capacity in messages** (§4) — byte unboundedness of provision runs
   stated as a scope limitation citing §5A's unit mismatch; strengthens
   H-a, weakens H-b/C2 relative to byte reality.
4. **H-a is stated over the work-conserving class** (§5.1) — C1 as
   literally chartered (all deterministic local strategies, idling
   allowed) is expected FALSE if σ* survives; the charter text should be
   amended to the trichotomy or the necessity corollary read
   class-relatively.
5. **H-c's overlap/performance half stays executable-tier** (§5.3) — no
   kernel vocabulary for streaming overlap; muxprobe measurements,
   recorded [checked].
6. **Opening wire messages route through the mux** (§1.1) — old-mux
   faithful.
7. **Endpoint interleaving stays adversarial** (§1.4) — a strategy must
   survive every local schedule, matching the base DeadlockFree posture;
   the alternative (strategy also controls local scheduling) would weaken
   H-a and un-match the Rust's tokio nondeterminism.

## 8. Position on the hinge (from the model-fixer's seat)

The hinge: is the receiver's consumption order a deterministic function of
information causally available to the sender at push time?

- **Per stream: yes, and the model makes it a definition.** Consumption on
  `wire p h` is positional in the consuming stage's BFS scope order
  (MODEL.md §5 per-scope prologue; positional pairing is the identity
  carrier). Every receiver-side branching that affects that order is
  announced: D/R labels ride the receiver's own query listings inside
  reply frames the sender receives; M children are dropped with zero
  channel ops on both sides (MODEL.md §2) so they cannot perturb any
  order; provision-run absorptions live inside the parent's reply frame,
  which itself flows and is positionally consumed — silent but
  order-irrelevant. [derived from MODEL.md §2/§5; this much I regard as
  settled]
- **Across streams and across directions: this is where H-b stands or
  falls, and it is NOT settled by the model — it is made decidable by
  it.** The residue is exactly the phase-1 findings' "reverse-direction
  symmetric coupling": a proven demand's consumer may be blocked on its
  own outgoing obligations into the reverse outbox, whose drain waits on
  the reverse σ*'s demand-proof, which rides a forward-direction frame —
  a potential proof-lag cycle that no per-stream determinism argument
  touches. Whether σ*×σ* bottoms out is, in this model, (i) a `Mux.drain`
  `decide` per pinned family (run before any proof), and (ii) the two
  named lemmas of §5.2 (demand-proof soundness, τ-induction
  bottoming-out). [open]
- **H-a I expect to land**: the forced-run technique needs only the
  empirical anatomy transcribed, and every ingredient (family, forced
  prefix, stuck decide, WC lifting) has an existing kernel-checked
  idiom to copy. The general-C induction is the one real risk.
