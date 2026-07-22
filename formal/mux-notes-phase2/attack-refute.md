# Cross-examination of the σ* refutation (refute-c1.md)

**Frozen record (2026-07-22 estate audit):** kept deliberately — the cross-examination whose F1 repair route the keystone formalizes (cited from the proofs by finding number). Claims herein carry the epistemic status they had when written; where later work superseded one, the supersession lives in MUX-PROGRESS §4 or an in-place marker, and this file is otherwise not updated.


Panel role: adversarial cross-examiner, phase-2 adjudication
(MUX-PROGRESS.md §3, log 2026-07-21). Target:
`adjudication/refute-c1.md` (σ*/demand-lockstep, the H-b witness).
Ammunition: `adjudication/prove-c1.md`, `adjudication/probe.md`, worked
from MODEL.md directly (not the maps) for every load-bearing check.

Epistemic key as in PROGRESS.md: **[proven]** kernel-checked in-repo;
**[checked]** executable evidence in-repo/probe; **[derived]** paper
argument in this document; **[open]** known unknown.

## 0. Verdict up front

**σ* survives in its essentials.** After independently re-walking every
receiver choice point, attempting to construct all-idle states,
stress-testing the demand-proof calculus under in-flight reordering,
re-deriving the C = 1 accounting, and hand-running the argument on
adversarial families the probe's generator would not produce, I could
not break the refutation. C1 as literally chartered (∀C, ∀ deterministic
local strategy pairs, idling allowed, observation = everything received
at the endpoint) is false, witnessed by σ*, **conditional on three
repairs/rulings**, none of which I judge fatal:

1. The **Keystone Lemma's proof is broken as written** (its delivery
   case is unsound); the lemma as stated is too strong. The repair is
   identified and mechanical (§3, F1). [CONFIRMED, repairable]
2. **Slot-peek is load-bearing, not cosmetic.** The refutation stands
   only under the observation model where demux-decoded-but-unconsumed
   frames count as observed. The no-peek variant that refute-c1 §6.5
   recommends proving instead is **unsupported and plausibly false** — I
   exhibit a candidate wedge shape against it (§3, F2). The panel must
   ratify the observation definition; under the charter's own words
   ("everything received") slot-peek is defensible, so this is a
   dependency, not a defect. [CONFIRMED dependency]
3. The theorem covers only the `.impl`/margin-0 corner; the charter's
   "schedulable" domain under `.full`/D5 is unaddressed (§3, F3).
   [statement-level]

Nothing I found attacks the three structural pillars: (i) the closure's
exclusion of **both sides'** future pushes (self-containment — every
demand proof is grounded in traffic already committed, which is why
C = 1 works and why σ* stops exactly before recreating the provision
flood, §4.5); (ii) the global-τ-minimality argument that grounds the
symmetric composition (§4.4); (iii) one-scope-arrears label announcement
(§2, verified choice point by choice point, including the all-M
invisible scope and the absorb/opener ends). Where refute-c1 and
prove-c1 disagree in formulation, the refuter's version is the stronger
one: its Inevitable-closure never simulates the peer's strategy, so
prove-c1's honest gap L2 ("lower bound suffices" for the peer-push
simulation, prove-c1 §3.2) simply does not arise for this σ*. [derived]

---

## 1. What was reviewed and how

- refute-c1.md in full; every numbered step of §2 re-derived from
  MODEL.md §§2–8 rather than taken from the document's own citations.
- The §1.5 choice-point table re-built independently from MODEL.md §5
  (obligation machine) and §4 (channel wiring), *then* diffed against
  the document (assignment: do not trust the enumeration under review).
- prove-c1.md and probe.md read in full; consistency cross-checks in §6.
- Hand-constructed families beyond the probe's generator (rootH ≤ 6,
  fan ≤ 7 random): childless-D/all-M chains, the provision wall at
  parametric width, alternating-parity fresh-dispute chains, extreme fan
  at margin 0, and a no-peek starvation gadget (§5).

## 2. Independent walk of the receiver choice points

I reconstructed the role/announcement structure from MODEL.md before
comparing with refute-c1 §1.5. Summary of my own derivation, with the
two places the document's table needed verification most:

- **Who mints what, re-derived.** Walk stage S(p,h) processes scopes σ
  at height h+1 and answers σ's children: its `wire(c)` sends are p's
  replies about the children c (MODEL.md §3–§5). The reply about c
  carries one reaction per merge position — Supply/Match (M),
  Query(empty) (R), Query(nonempty) (D) — so **kids(c)'s D/R/M labels
  are minted by c's answerer at reply-production time and ride that one
  frame** (MODEL.md §2 labels; §5 step 2). Roles alternate per level, so
  for the demand arithmetic of stream c = wire(p,h) with consumer
  W′ = Walk(¬p, h−1): the labels of the scopes W′ *consumes* are minted
  by p (zero latency), and the asked-quota counts for W′'s scope j
  (Σ|kids(c′)| over j's D children) are minted by ¬p and ride **W′'s own
  scope-j publications** on the reverse direction. This confirms the
  table's rows 2–4 and the "two-scope-arrears" structure: demand for
  (c,k) is rcv(c,k−1), whose E3 predecessors are scope k−2's *complete*
  publication set (sequential-scope premise, MODEL.md §5.4), so the
  freshest labels a demand proof ever needs concern scopes ≤ k−2, whose
  carrying frames are E3-ancestors of rcv(c,k−1). [derived, confirms
  refute-c1 §1.5/§2.4]
- **All-M / childless-D scopes** (the §5 "invisible scope"): a scope
  whose children are all M appears in the skeleton as a childless D
  scope (wellFormed permits childless D at height ≥ 2; M-children are
  dropped, MODEL.md §2). Its consumer's ops are prologue recvs plus one
  pending-0 parent resolution; the I-step closure derives them with zero
  reverse evidence, and the *existence/emptiness* of the kid list is
  announced in the parent reply, minted by the pushing side itself (the
  pusher answers the scopes its stream's consumer processes). Verified:
  the evidence-only wedge is real and the closure genuinely dissolves
  it. [derived, confirms]
- **Provision (R) scopes**: consumer-side pending = dCount = 0 on the
  asker side (MODEL.md §4 resolution counts), so absorption is silent
  but its *order* is fully positional and its *occurrence* is
  I-step-derivable — confirming probe §6's answer and the table's row 5.
  [derived + checked]
- **leafReqs / Absorb / openers**: leafReqs minted by I (the height-1
  answerer) riding wire(I,1) frames, which necessarily precede R's
  wire(R,0) supplies about the same requests (R consumes them first);
  Absorb's per-request block opens with recv wire, grounding k = 1; both
  opening wires are frame 1 of their channels, hence never withheld.
  Spot-verified — I concur with refute-c1's own gap #3 that a Lean
  transcription must still enumerate these exhaustively, and found no
  problem in the spots. [derived, low residual risk]
- **Cross-height cursor interleaving**: never announced and never
  needed — demand facts are per-stream, monotone, schedule-independent;
  the stuck-state lemmas quantify over interleavings. Verified through
  the Step-1/Step-3 checks below. [derived, confirms]
- **A strengthening the document missed** (also discharges prove-c1
  obstruction #6): under `.impl`, the ledger guards *totally order* each
  scope's publications. All wire sends of a stage share one channel
  (per-channel child order), likewise lowerRes and asked; cross-channel,
  W forces wire(i) ≺ res(i), D1 forces res(i) ≺ queries(i), D4 forces
  queries(i) ≺ wire(i+1) for every earlier D sibling, D6 pins the parent
  last (MODEL.md §6). So at every commit point exactly one obligation is
  choosable: **the honest D6 linearization is forced, not "picked by the
  refuter"** — σ*'s theorem is robust to leaving `walkCommit`
  adversarial, and refute-c1 §0's "the refuter picks honest D6 commits"
  can be deleted as a hypothesis. [derived]

Conclusion of the walk: the §1.5 enumeration is **complete and correct**
for the model of record; no silent receiver branching was found beyond
those the document names, and each named one is classified correctly.

---

## 3. Findings, ordered by severity

### F1. The Keystone Lemma's delivery case is unsound as written — CONFIRMED, repairable

Statement under attack (refute-c1 §1.3): *"In any reachable stuck state
s, every event of Inevitable_p(O_p(s)) has been performed."* Proof
sketch given: take the first unperformed I-step event in closure order;
predecessors performed ⇒ guard open ⇒ it is an enabled action ⇒
contradicts stuckness.

**The gap.** I-step admits *delivery* events del(c,n) whose DAG
predecessors are snd(c,n) (∈ C-own, performed) and rcv(c,n−1) (slot E2).
Predecessors-performed gives: frame in the pipe, slot free. It does
**not** give *pipe-head position* — and the demux delivers only the
head. The DAG has no cross-stream pipe-order edges, so the closure can
contain an unperformed del(c₄,r) sitting *behind* a blocked head
(c″,j): del(c₄,r) is inevitable, unperformed, and **not enabled**. The
proof's "e is an enabled action" fails for exactly this case, and the
case is reachable in the lemma's intended use site (Step 1 considers a
nonempty pipe with a blocked head — precisely the configuration that
manufactures such members). Failure scenario, concretely: stuck-candidate
s with head g′=(c″,j), slot(c″) full; p pushed (c₄,r) after g′ with
rcv(c₄,r−1) already inevitable; the closure at s contains del(c₄,r);
the induction selects it first (closure order is construction order,
which nothing constrains); the enabledness claim is false; the induction
collapses. The lemma as stated is at best unproven; I believe it is
actually **false** for the full set at s in adversarial closure orders,
though no *stuck reachable* counterexample follows (that is the point —
the proof, not the theorem, is what breaks). [derived]

**Why it is repairable (repair verified by hand).** Step 1 needs only:
rcv(c′,m) ∈ Certified ∪ Inevitable at the *push time t* of the head g
⇒ performed at s. Run the keystone induction over the **time-t
derivation tree** instead of the full set at s:

- Every forward delivery cited at time t has snd ∈ C-own(O_p(t)), i.e.
  the frame was pushed **before g**; at s, g is the head, so every
  pre-g frame is delivered — all cited forward dels are *performed* at
  s. [the FIFO ancestry argument]
- Every reverse (¬p→p) send/del cited enters only via C-arr = already
  arrived = delivered. Performed.
- What remains unperformed in the time-t tree is therefore **only ¬p
  non-push events**; the tree is finite and downward-closed in the DAG
  (acyclic on this class — margin-0 ⇒ scheduleE exists [proven]), so it
  has a minimal unperformed member, all of whose predecessors are
  performed; the counting argument (producer sends ≥ n, sole consumer's
  recvs = n−1, SPSC guard stability, MODEL.md §4) opens its guard; it
  is a non-push protocol action, hence enabled — contradiction with
  stuckness. ∎ [derived]

Monotonicity is then used only to say the time-t derivation remains a
valid derivation at s (closures are monotone in O_p), not to re-run the
induction over the bigger set. **Action for the Lean statement:**
restate the keystone as a property of push-time derivations (or ban
forward-del citations from I-step and add the FIFO-ancestry lemma
separately); do not attempt the lemma as currently phrased — it will
not prove. Downstream steps are unaffected: Steps 2–4 operate after
pipes-empty is established, where the delivery case is vacuous (every
performed snd has a performed del). [derived]

### F2. Slot-peek is load-bearing; the recommended no-peek variant is unsupported and plausibly FALSE — CONFIRMED dependency

Where peek is used: §2.4's coverage cites frames "delivered to p's
endpoint... by slot-peek-or-consumption". I verified this is not
removable: the label-carrying reverse frames needed for f*'s demand
proof are DAG-ancestors of rcv(c,k−1) — but **their receipts by p are
not**. Their consumers are *other walks of p*, which may legitimately be
parked mid-scope on their own withheld pushes with τ **above** f*
(consistent with f* being τ-least). At such a stuck candidate the
needed frame sits **delivered-but-unconsumed** in p's slot. With peek,
C-arr covers it and coverage closes. Without peek, C-arr covers consumed
frames only; the labels are unreadable; §2.4 fails — and I see no
alternative derivation: the content is what is needed, and deriving
"the frame will eventually be consumed" does not reveal content to a
no-peek observer. Candidate wedge shape for no-peek σ* [open, not fully
constructed]: two stages of p, heights h and h−2; Walk(p,h−2) committed
mid-scope on a wire send whose demand proof needs labels riding a frame
parked in Walk(p,h−2)'s *own* input slot-queue region; Walk(p,h)'s
τ-least withheld push needs labels riding the frame parked at
Walk(p,h−2)'s slot — mutual proof-starvation across heights of the same
party, with the discriminators delivered but sealed. A two-height
dispute chain with fan ≥ 2 and alternating D-placement looks
sufficient; constructing and probing it should precede any attempt to
prove the no-peek form.

Consequences: (a) refute-c1 §6.5's recommendation ("prove the no-peek
form, strictly stronger, believed to hold") should be **reversed** —
prove the with-peek form and surface the observation definition to the
panel as a modeling decision; (b) the C1 statement must define observed
trace as *frames decoded at the endpoint's demux* (faithful to
`incoming.rs:60-92`, which decodes before routing [checked, in-repo]),
and the refutation should be advertised as conditional on that reading.
Under the charter's letter — "the trace of every action it has observed
so far (its own sends and everything received, in order)"
(MUX-PROGRESS §1) — delivered-to-endpoint is a fair reading of
"received", so I rule this a dependency, not a fatality. If the panel
rules observation = consumed-only, **σ*'s liveness is open again and my
wedge shape is the first thing to test**. [derived]

### F3. Corner and domain coverage — statement-level repair

The theorem is stated for `.impl` + margin-0 only. The charter's domain
is "well-formed and schedulable, i.e. one the un-muxed protocol provably
completes" (MUX-PROGRESS §1). Two readings: (i) the shipping encoder's
proven class is margin-0 (`Sched.deadlock_free` [proven]) — then σ*'s
class matches and the C1 statement should *say* margin-0 + D6; (ii) if
the panel keeps "schedulable" (the `.full`/D5 corner,
`Sched.deadlock_free_d5` [proven]), σ* must be ported: τ := `schedule`
(merge_complete [proven]) is available, and D5's guards also totally
order each scope's publications (W/D1/D4 as in §2 plus D5's
parent-first-after-last-D placement), so the port looks mechanical — but
it is currently **unargued** in refute-c1, and the probe's `.full`
sweep (81/81 terminal, probe §4) used the omniscient σ*, not the causal
one. Fix the statement or write the port. [derived / open]

### F4. The refutation's novel core has zero executable validation

Probe caveat 3 is explicit: the probe's σ* certificates read **global
state**; "a causal-simulation σ* (fed only by own trace + skeleton) was
not implemented, and a tree-local σ* cannot even be expressed in this
model" [checked, probe §8]. So the 2,150/2,150-terminal evidence
supports the *structural* half (idling with full knowledge is live at
C = 1) and says nothing about §2.4's coverage — the exact step refute-c1
itself ranks as its load-bearing novelty (gap #1). Before Lean:
implement the A_p-limited causal σ* in the probe harness (the closure is
two finite fixpoints over announced positions — directly codable) and
sweep, with emphasis on shapes that maximize label latency: deep
alternating-parity D-chains where every scope's asked-quota labels ride
the immediately preceding reverse frame, childless-D (all-M) tails, and
the no-peek variant on the F2 gadget. Also run the decidable per-state
check refute-c1 proposes ("τ-least-withheld demand derivable from causal
observation at every reachable σ*-state") — that is Step 4 as a testable
invariant. [derived recommendation]

### F5. Internal inconsistency: the "per-stream in-flight ≤ 2" claim

§3 claims "the demand rule keeps per-stream in-flight at ≤ 2"; §4.3
claims provision runs "pipeline at full pipe speed"; §4.5(a) offers
batched pushes as an *upgrade*. The demand rule as defined already
licenses unbounded same-stream lookahead on silent (M/R) runs:
rcv(c,k) for a silent scope is I-step-derivable before (c,k) is even
delivered, so (c,k+1), (c,k+2)… become proven-demanded in sequence.
The ≤ 2 bound is false as stated (harmless to the theorem — nothing
uses it — but it will mislead the Lean invariant design and the H-c
latency analysis). Fix the side-claim; the correct bound is "≤ slot +
the forward-derivable silent horizon". [derived]

### F6. Monotonicity formulation hazard in I-step

I-step's guard clause is phrased as "channel guard open at the occupancy
computed from Inevitable_p counts". An occupancy-mutable formulation is
not obviously monotone in O_p (more derived sends can *close* a computed
send guard), and §2.1 leans on monotonicity. The positional formulation
— guard = membership of the E2 predecessor (rcv(ch, n−cap)) in the set —
is monotone by construction and coincides with occupancy at the real
capacities. The Lean definition must use the positional form. [derived]

### F7. "Demand edge" mischaracterization (wording)

§1.3 lists "the demand edge rcv(c,n) ≺ snd(c,n+1) that σ* itself
enforces". σ* enforces *membership* (rcv(c,n) ∈ Certified ∪ Inevitable
at push time), not run order — pushed frames routinely precede their
predecessor's consumption (that is the tolerated two-in-flight
transient, §2.1). The edge is sound in the DAG only because it *is* the
unmuxed cap-1 wire E2 edge that τ respects. Reword before Lean, or the
formalizer will try to prove a false run-order invariant. [derived]

### F8. Mux close-guard spec gap

The muxed `recvClose wire` guard needs "producer fired all ops ∧ slot
empty ∧ **no c-frames in the producer's pipe**" (the probe implements
exactly this, probe §2; refute-c1 never states it). Without it,
Terminal/stuck classification drifts at the session tail. One line in
the model definition; flagged so it is not lost. [derived]

### F9. H-c commentary overclaims its epistemic grade

§4.4's "σ*'s proof-lag adds O(1) reverse-arrivals per D-scope on the
critical descent — a constant factor on depth·RTT, not a new asymptotic
term" is plausible but unmetered, and the probe explicitly finds H-c
**unpriceable in the current model** (message-counted, latency-free —
probe §5). Downgrade the H-c verdict from [derived] to
[derived-shape / open-quantitatively]; the panel should not carry
"constant factor" into any statement without a hop-metered extension or
Rust measurement. [derived]

---

## 4. Load-bearing steps that SURVIVED the attack (verification notes)

### 4.1 Step 1 (pipes empty at stuck states) — sound after F1's repair

The slot/FIFO bookkeeping is right: the head g and the slot-resident
frame are same-stream consecutive (sole producer, in-order dels), so
INV-A applies to exactly the right pair. With the keystone restated per
F1, the contradiction goes through. The transient "slot holds k−1
merely-inevitable while pipe holds k" is genuinely tolerated and
genuinely collapses at stuck states. [derived]

### 4.2 Step 2 (stuckness forces a withheld push) — sound

Chase termination: finite acyclic DAG (margin-0 ⇒ scheduleE exists
[proven]). Guard-openness from performed predecessors: re-derived for
recv/send/close; the delivery sub-case cannot arise (pipes empty ⇒
performed snd ⟹ performed del, so the chase passes through dels to
snds). Terminal event a wire snd ⇒ committed: by the `.impl`
total-order result of §2 above, an uncommitted-or-differently-committed
walk would expose an earlier unperformed program-order predecessor,
contradiction; uncommitted publishers can always commit
(`walk_uncommitted_choosable` [proven], applicable — `.impl` has
d5 = false). Pipe room: pipes empty. So "unperformed, all-preds-done,
non-enabled" ⇒ withheld. [derived]

### 4.3 Step 3 (τ-least withheld push has consumed predecessor) — sound

I probed the slot logic the document glosses: (c,k−1) in the slot forces
rcv(c,k−2) performed (dels in order require the slot emptied), so W′ is
blocked inside scope k−2's body, exactly where the (a)/(b) case split
lands. Case (a)'s DAG chain (scope-j publication ≺ rcv(c,j+1) ≺ … ≺
rcv(c,k−1) ≺_{E2,cap-1} snd(c,k)) is all-DAG-edges; τ respects the DAG
regardless of muxed-run transients — the argument never needs the run to
respect τ. Case (b)'s chase stays τ-below f* because every reached event
is a DAG-ancestor of scope-j completion, including the cross-party and
cross-direction hops (assembler → level → deeper walks → other-direction
wire recvs → the *other* party's withheld pushes, which are pooled in W
— global minimality applies). The reverse-direction symmetric coupling
is genuinely discharged here, not hand-waved. [derived]

### 4.4 Step 4 (coverage) — sound at stuck states; the two-scope gap is real

Re-derivation: rcv(c,k−1) performed (Step 3) ⇒ its full E1/E2/E3
ancestor set is performed ⇒ every ¬p push in it is delivered (pipes
empty) ⇒ certified by C-arr (with peek, F2); every needed label rides
one of those frames (my §2 minting walk); every needed forward del has
snd ∈ C-own; the I-step closure then replays the ancestor set in DAG
order with positional guards that match the E2 edges tautologically
(ancestors under E1∪E2∪E3 are downward-closed, so "guard-open" is
literally "E2-predecessor in set"). The document's own caveat — demand
is pinned to the prologue recv, NOT scope completion, precisely so the
derivation never touches events τ-above f* — checks out and is the
step's crux; I confirm the circularity it dodges is real (scope-(k−1)
completion can require frames τ-above f*). [derived]

### 4.5 The self-containment property and C = 1 — sound, and the best thing in the document

Verified: **neither closure ever cites an unperformed push by either
side** (I-step derives non-push events only; snds enter via C-own =
p's performed pushes or C-arr = ¬p's arrived pushes). Hence every
demand proof is grounded entirely in traffic already committed to the
pipe or delivered — the pushed head always drains without any further
strategy cooperation from anyone. This is what makes the theorem uniform
in C ≥ 1, it is structurally identical to the probe's 'exit' certificate
(close under everything except further pushes — probe §4 [checked],
confluence-validated 0/8,695 disagreements), and it is exactly why σ*
stops at the right frame of the provision wall: the wall's scope-m
completions eventually require Asm's pending fill, which requires the
*sender's own future deep pushes*, which the closure refuses to cite —
so demand fails and σ* idles with the pipe free for the deep answers.
I re-ran this arithmetic on `prov C` by hand and it agrees
frame-for-frame with prove-c1 §2.2 (push through frame 4, withhold 5+).
The two documents' independent agreement on the mechanism is itself
evidence. [derived + checked]

### 4.6 Base case, termination, all-idle grounding — sound

k = 1 unconditional: every wire consumer's first channel op is the recv
(walk prologue reply-first, MODEL.md §5.1; ROpen; Absorb's block head),
and a first frame's slot is trivially free — it can never be a blocking
head. ρ′ termination: standard, delivery counted, idling not a
transition. All-idle states: any candidate is a stuck state; Steps 2–4
exhibit a proven-demanded withheld push at it; σ_p's decision is a
function of O_p, which is unchanged while nothing moves, so "would push
later" is not a confound — the demand proof succeeds *now* or the state
was not stuck. [derived]

## 5. Adversarial families constructed by hand (probe-blind spots)

1. **All-M / childless-D chains** (invisible scopes): closure derives
   the silent consumptions from own-minted labels; no reverse traffic
   needed. Survives. [derived]
2. **Provision wall `prov C`** (width parametric in C): σ* withholds at
   the exact budget frame; deep answers overtake; completes. Survives,
   agrees with prove-c1. [derived]
3. **Alternating-parity fresh-dispute chain** (every demand proof needs
   labels off the immediately preceding reverse frame): maximal
   serialization, no deadlock — at any stuck candidate the needed label
   frame is τ-below and delivered. This is the latency worst case, not a
   liveness wedge. Survives; belongs in the causal-σ* probe sweep (F4).
   [derived]
4. **Extreme fan at margin 0**: level cells never gate (capLevel ≥
   dCount, the FAN counting lemma, MODEL.md §8); the assembler coupling
   enters demand proofs only through cell-freeing recvs, all derivable.
   Survives. [derived]
5. **No-peek starvation gadget** (F2): the one shape that threatens a
   variant — but the variant, not the definition under review. [open]

## 6. Consistency with the other briefs

- **prove-c1**: its σ* uses a peer-push simulation closure and carries
  the open lemma L2; refute-c1's σ* excludes all pushes from I-step and
  needs no peer simulation — L2 is moot for the object under review.
  The two briefs' walk-throughs of the regression shape agree exactly.
  prove-c1's obstruction #6 (who owns walkCommit) is discharged by the
  `.impl` commit-totality result (§2). prove-c1's C1-WC (H-a) is
  untouched by anything here and remains the right companion statement.
- **probe**: supports the structural half [checked]; does not touch
  §2.4 (F4); its finding that certificate verdicts are functions of
  (skeleton, committed traffic) is the executable shadow of the
  self-containment property (§4.5) and raises confidence that the causal
  closure, once implemented, will match the omniscient one wherever
  A_p has caught up.

## 7. Recommendation to the panel

State the results as three theorems:

1. **C1-literal is FALSE** — σ* (with-peek observation model, `.impl` +
   margin-0, any C ≥ 1), with F1's keystone repair folded into the proof
   plan and F2's observation definition ratified as a model decision
   (with the Rust demux citation as its anchor). If the schedulable/
   `.full` domain is kept in the charter statement, require the D5 port
   (F3) before calling C1 settled.
2. **C1-WC (work-conserving) is TRUE** [target] — prove-c1's family;
   probe-supported; this is the theorem that explains the shipped bug
   and preserves the charter's spirit.
3. **H-c weakened**: oracle advantage = pipe utilization/constant
   factors, unpriceable in the current model (probe §5); do not enshrine
   quantitative claims (F9). The "mysterious third thing" candidate —
   fresh-frontier labels one RTT early — survives review as the sharp
   residual question.

Ordered work before Lean: (1) causal-σ* probe implementation + the
Step-4 invariant check on the F4 families; (2) keystone restatement;
(3) panel ruling on observation (peek vs no-peek) and domain (margin-0
vs schedulable); (4) only then the coverage induction, which remains the
largest genuinely new proof and — after this review — the only place a
C1-true reversal could still hide.
