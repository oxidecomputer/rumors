# Cross-examination: the pro-C1 brief (prove-c1.md)

Role: adversarial cross-examiner of
`scratchpad/adjudication/prove-c1.md`, armed with `refute-c1.md`,
`probe.md`, and the probe's three minimized traces. Required reading
completed in the charter's order (MUX-PROGRESS.md, the four maps,
MODEL.md in full).

Epistemic key as in PROGRESS.md: **[proven]** kernel-checked in-repo /
**[checked]** executable evidence in-repo or in the probe / **[derived]**
paper argument, here meaning *my own re-derivation*, not the brief's /
**[open]** known unknown. Where I say "verified" below, I mean I re-derived
the step myself from MODEL.md and the Lean maps, not that I trusted the
brief.

## Verdict up front

**The impossibility survives, at the work-conserving class, with
repairs.** No fatal finding. The core theorem (C1-WC: every
work-conserving local strategy pair deadlocks on an explicit skeleton
family, both axiom corners) is correct; I verified the witness family's
well-formedness and margin-0, the ledger-forced publication order, the
singleton-enabled-set claims at every strategy consultation, the
absorption arithmetic, and the final-state stuckness enumeration, and the
probe's minimized traces independently exhibit the stuck states at C ∈
{1, 2} [checked]. The brief's deliberate abandonment of the
fooling/pigeonhole route — consulting the strategies only at
singleton-enabled states — is its strongest feature and it holds up:
there is no P-indistinguishability definition to attack because none is
used, and that is *why* the ∀-strategies quantifier goes through.

Four repairs are needed before Lean statements are minted, one of which
strengthens the theorem substantially:

1. The statement is quantifier-weak and the pigeonhole framing
   misidentifies the mechanism: the probe's capacity-flatness data plus my
   re-derivation show a **single fixed skeleton defeats every WC pair at
   every capacity C** (∃sk ∀C, not ∀C ∃sk(C)); the jam is FIFO burial
   plus permanent slot occupation, not pipe-capacity exhaustion.
2. The bounded-patience corollary is **ill-typed under the brief's own
   M2**: a deterministic pure function of (tree, trace) cannot "idle B
   times then push" at an unchanged observation. The "exact frontier"
   claim must be restated.
3. A one-line D5 transcription error (parent placement relative to the
   asked send) — immaterial to the jam, but the Lean will catch it loudly.
4. The realizability direction flips for impossibility results: MODEL.md
   §2's "unrealizable skeletons only enlarge the verified set" soundness
   argument protects *positive* theorems only. The fixed-witness
   restatement (repair 1) happens to solve this too, because the
   committed regression seeds already realize that shape [checked].

C1 as literally chartered remains refuted by σ\* on the current evidence;
that concession is consistent across all three briefs and its one open
gap (prove-c1's L2 ≈ refute-c1's §2.4 coverage) does not touch C1-WC in
either direction.

---

## 1. What I verified, step by step (the parts that survive)

### 1.1 The witness family `prov C` is inside the theorem class [derived]

Checked against `Skel.wellFormed`'s conjunct list (lean-model map §1.1;
MODEL.md §2):

- Heights/parities: rootH = 4 even; root h4 kind D ✓. Scope 1 (h3, D,
  kids [C+6]) one height down ✓; scopes 2..C+5 (h3, R) childless with no
  leafReqs ✓ (non-D ⇒ childless); scope C+6 (h2, D, kids [C+7]) ✓; scope
  C+7 (h1, D, leafReqs 1) childless, leafReqs only at height-1 D ✓.
- Fan: root has C+5 kids = fan ✓ (fan is a per-skeleton parameter; see
  finding F4 on what that costs at the Rust boundary).
- BFS alignment: scopesAt(4)=[0] flatMap kids = [1..C+5] = scopesAt(3);
  scopesAt(3) flatMap kids = [C+6] = scopesAt(2); = [C+7] = scopesAt(1);
  scopesAt(1) flatMap kids = [] = scopesAt(0) ✓.
- Margin 0: dCount ∈ {1,1,1,0} ≤ capLevel = 1 everywhere ✓ — inside
  `Sched.deadlock_free`'s hypothesis class, hence the unmuxed protocol
  provably completes on it [proven, consumed].

### 1.2 The publication order is ledger-forced [derived]

At Walk(I,3)'s root scope, under `.impl` = {W, D1, D2, D3, D4, D6}
(MODEL.md §6): the only choosable first obligation is wire(child 1) (W
blocks res; D1 blocks asked; D4 blocks wires 2.. until the sole D sibling
is resolved *and* has sent its queries; D6 blocks the parent until last).
Then res 1 (W satisfied, everything else still blocked), then asked(C+6)
(D1 satisfied), then wires 2..C+5 in child order (per-channel child order
is program structure, MODEL.md §5.3), parent last (D6). Each commit point
is a **singleton**; the committed-choice machinery (MODEL.md §5,
"commitment is what captures that a real program cannot skip ahead")
means the walk holds exactly one obligation at a time. So neither the
walk-linearization slack (brief's §7.6 worry) nor the mux's frame choice
ever has more than one option on this family. Same check passes for
Walk(R,2)'s scope 1 (wire, res, asked, parent) and for the degenerate R
scopes (parent only, immediately choosable under both D5 and D6).

### 1.3 The singleton-enabled-set claims [derived, the load-bearing step]

The brief's central move — "the strategies were consulted only at
singleton-enabled states" — survives scrutiny, but it is worth recording
*why*, because it rests on an adversary power that must be stated in the
model: **the adversary may delay demux deliveries** (a demux step is an
ordinary action under interleaving nondeterminism; only at the terminal
stuck state must no delivery be enabled). During the fill window:

- I-side committed-enabled wire sends: Walk(I,3) holds exactly one
  (§1.2); Walk(I,1) is in phase 0 and *cannot commit* because its wireIn
  (`wire R 2`) is undelivered — the adversary parks R's pushed frame in
  the R→I pipe until the I→R pipe is full. IOpen is done; Absorb produces
  `level`, not wire. Singleton ✓.
- R-side consultations (opening reply; the scope-C+6 question frame):
  Walk(R,0) is in phase 0; ROpen's wire is done; singleton ✓. Note σ_R is
  forced *to push* by work-conservation — the adversary needs R's frame
  in flight so Walk(R,2) can progress to its park, and WC guarantees it.

Then step 5 (deliver R's frame; Walk(I,1) recv wire, recv asked — the
asked was sent in §1.2's forced order; commit wire) is *mandatory*, not
optional: if those actions were skipped, they would be enabled at the end
and the state would not be stuck. The brief's run includes them. ✓

### 1.4 The absorption arithmetic and the park [derived + checked]

Walk(R,2) processes scopes 1..C+5 positionally (MODEL.md §5: sequential
scopes; prologue recv wire then recv asked). Scope 1's parent resolution
(asker-side pending = dCount = 1, MODEL.md §4's asymmetric counts) is
consumed by Asm(R,3), which enters phase 1 needing a level(R,2) item that
transitively requires the whole C+6 subtree — i.e. `wire(I,1)` frames.
Scope 2's pending-0 resolution then fills the cap-1 upperRes cell; scope
3's parent send blocks. Quiescence *forces* exactly this park position:
if Asm(R,3) had not consumed scope 1's resolution, `asmRecvRes` would be
enabled at the end. So consumed = 3, slot = 1, pipe = C; wall = C+4
provisions + 1 dispute reply = C+5 > C+4. The probe's stuck states pin
the identical anatomy at rootH = 6 (upperRes(R,4) = 1, slot wire(I,5) =
1, walk(R,4) at scope #2 committed `parent`, walk(I,1) committed
`wire 0`, pipe holding provisions —
`probe/trace_minimal_w4_C1.txt`, `trace_regression_bottom_C2.txt`)
[checked].

### 1.5 Final-state stuckness [derived + checked]

I enumerated every process against MODEL.md §5/§7 guards: both walks with
committed wires blocked on pipe room; Walk(R,2) blocked on the full
upperRes cell; Walk(R,0)/Absorb starved on empty slots; ROpen blocked
mid-asked-quota on the full asked cell; every Asm in phase 0 on an empty
resolution cell or phase 1 on a starved level channel; RFinish short of
rootPending; all recvCloses fail `producerDone`; demux R head destined
for the full slot; pipe R→I empty; mux R has no committed wire (so WC
prescribes nothing — the stuck state does not even need M3's contested
σ-refusal clause, only the charter's own "no process, mux, or demux can
move"). Non-terminal trivially. The probe's simulator, whose enabledness
check is exhaustive over the transcribed `allActions`, classifies the
concrete instances stuck with zero fuel ambiguity [checked].

---

## 2. Findings, ordered by severity

### F1 [repairable, and a strengthening] — the quantifier order is weak and the pigeonhole misidentifies the mechanism

The brief states C1-WC as **∀C ∃ prov C** with a witness whose wall
scales as C+4, and frames the proof as beating an absorption budget
"3 + 1 + C" (§4.3 step 4, §4.4). The probe's data contradicts the
*framing* (not the theorem): the minimal deadlocking width of the
regression family is **w = 4 flat across C ∈ {1..16}** (probe §3 finding
2) [checked]. I re-derived why, and it matters for the statement:

At large C the pipe never fills. The jam mechanism is: frames 1..w+1 of
the wall are pushed (WC-forced, room available); the consumer parks after
consuming 3; the slot holds frame 4; **frame 5 sits in the pipe
permanently** — deliverable never (slot full until the deep subtree
completes, which needs frames that don't exist yet). When the deep
question arrives and the sender's deep reply becomes committed-enabled,
work-conservation **forces the push into the poisoned FIFO**, burying the
one frame the receiver needs behind an undeliverable provision. Capacity
never enters: the load-bearing bounds are (i) the cap-1 per-stream slot
and (ii) FIFO commit-no-retract. This is exactly MUX-PROGRESS §4's
boundedness caveat ("the cycle is demux head-of-line + flush-paced
receipts, not raw pipe capacity" [checked, in-repo: the 64 B / 16 MiB
invariance]) and the probe's own recommendation ("no pigeonhole over pipe
capacity is needed or appropriate").

Consequently the theorem should be restated **∃sk ∀C ∀ WC pairs** with a
*fixed* witness (the regression shape at w = 4, rootH = 6 — the probe's
minimal instance), proved by the same singleton-forced-run technique: the
adversary withholds R's question frames in the R→I pipe until the wall is
pushed (all consultations singletons, as in §1.3), then the deep push is
forced behind the permanent resident. I walked this variant's final state
through the same enumeration as §1.5; it is stuck at every C [derived].
The C-scaled `prov C` family remains a fine *secondary* instance showing
the sender-side blocking variant of the wedge, and the demux-variant
budgets of §4.4 transfer to it; but the flagship statement should not
carry a capacity-indexed witness, because (a) it is strictly weaker, (b)
it invites the reader to think the pipe bound is load-bearing when the
artifact's own record says it is not, and (c) the fixed witness dissolves
finding F4 (realizability) for free.

Minor arithmetic in the same area: §4.4's consumer-readiness variant
"jams at m = C + 3" is stated in the scaled framing; under the fixed
witness it becomes "jams at wall ≥ consumed + 1", again
capacity-independent. Recompute after the restatement.

### F2 [repairable] — the bounded-patience corollary is ill-typed under M2; the "exact frontier" claim must be restated

M2 defines a strategy as a **deterministic pure function**
σ_p : (tree, observed trace) → Option Frame. The corollary (§4.1) defines
patience-B strategies as those that "may idle at most B consecutive
scheduling opportunities **while its observations are unchanged**". But a
pure function consulted twice at the same (tree, trace) returns the same
answer: under M2, a strategy either idles at a given observation
*forever* or pushes at the *first* consultation. "Idle B times, then
push" requires a consultation counter, which is not in M2's observed
trace. So the class {bounded-patience, B ≥ 1} \ {work-conserving} is
**empty** as typed, and the corollary's adversary ("schedules the mux B+1
times per fill step") schedules a function that cannot change its mind.

Two repairs, panel's choice:

- (a) Extend M2 so strategies observe scheduling ticks (or their own idle
  count). Then the corollary holds as argued — I checked that the
  adversary can keep observations otherwise frozen through the fill
  window (deliveries withheld; remote actions are unobservable), so each
  B+1-consultation block forces one push. But this is a *model
  extension* the panel must adopt explicitly, and it weakens the clean
  "pure function of local information" reading of the charter.
- (b) Keep M2 pure and restate the frontier without time: on the witness
  family, any strategy that **pushes an enabled-but-uncertified frame at
  one of the adversary-reachable fill observations** jams (same run);
  any strategy that idles there cannot be hurt at that state, because
  idling forces the adversary — on pain of the state not being stuck —
  to eventually fire an enabled delivery, which changes the observation
  and leaks the discriminator (I verified there is always exactly such a
  pending delivery at the idle states of this family: R's question frame
  in the R→I pipe). Under (b) the honest frontier statement is:
  **work-conservation is precisely the property of never letting the
  observation change before pushing**; "patience" is not a spectrum for
  pure strategies but a binary per-observation choice, and σ\* sits on
  the idle side of it at exactly the fill observations.

Either way, §4.1's sentence "the impossibility frontier is precisely
observation-conditioned unbounded idling" needs to be re-derived in the
chosen vocabulary before it goes into a Lean statement. This does not
touch the main theorem: WC strategies push at every consultation by
definition, pure or not.

### F3 [repairable, one line] — D5 corner transcription error in the forced run

§4.3 step 2 claims the parent resolution lands "immediately after
asked(C+6) (D5)". Wrong: D5 requires the parent **before any further wire
or query** once every D child is resolved (MODEL.md §6, D5; the weave
"pins the floating parent immediately after the scope's final
resolution", PROGRESS.md §5 via model-doc §6). After res 1 — the only D
child's resolution — the asked(C+6) send is a query and is therefore
D5-blocked until the parent fires. Correct D5 order: wire 1, res 1,
**parent**, asked(C+6), wires 2..C+5. Immaterial to the theorem (the
parent is an intra-party upperRes send; the wire wall's order and the
R-side jam are unchanged — I re-checked Asm(I,4) merely moves to phase 1
earlier and stays disabled), but it is exactly the kind of slip the
`wkChoosable` transcription will surface, and the brief's claim "the
wire wall's order is invariant" deserves to be stated with the corrected
D5 order next to it.

### F4 [repairable, scope hygiene] — realizability flips direction for impossibilities

MODEL.md §2's soundness argument — honest walks are deterministic given
trees, unrealizable skeletons only *enlarge* the verified set — is a
one-way street: it makes positive theorems conservative and does nothing
for negative ones. A C1-WC witness skeleton that no tree pair realizes
proves nothing about the charter's "there exists a tree pair" (§1). The
brief knows this (§5.3's proptest bridge, §7.4's [open]), but the theorem
statement in §5.2 carries no realizability hypothesis and the prose
claims Rust transfer "a fortiori" in units without flagging that the
*witness* needs the bridge too. Under F1's fixed-witness restatement this
is nearly free: the committed regression seeds
(`tests/pairwise.proptest-regressions`) realize root fan ≥ 7 with the
first radix child deep-disputed and ≥ 6 provisions behind it — the exact
shape — deterministically in ~20 ms [checked, in-repo]. Recommendation:
state the model-level theorem over Skel as-is, and mint a separate,
explicitly-bridged corollary "realized by the committed seed pair" for
the Rust claim; drop or bracket the large-C `prov C` realizability
question (fan > 256 multi-scope wall) as [open] prior art unless someone
needs it.

### F5 [repairable, definitional nit] — fix the commit-control boundary in the WC definition

`WorkConserving` quantifies over "committed-enabled wire sends" with
commits made by the model's nondeterministic `walkCommit`. On the witness
family commits are ledger-forced singletons (§1.2), so the theorem is
insensitive — but the Lean statement must fix whether σ gates commits
(the probe's σ\* fuses commit+fire; an idling strategy sabotaged by
adversarial wire commits would be parked forever holding a frame it
refuses to push). Recommendation, matching the probe's model of record:
commits stay adversarial, σ gates **pushes only**, and the C1-WC theorem
notes commit-forcing on the witness family as a lemma
(`commit_singleton`). The brief flags this itself (§7.6); it should be
resolved, not flagged, before Lean.

---

## 3. The four assigned attack joints, answered

**(i) The P-indistinguishability definition — is the fooling pair
realizable, does the trace stay identical?** Vacated by design: the main
theorem uses no fooling pair and no indistinguishability definition. The
run script consults the strategies only where their decision set is a
singleton, which I verified at every consultation on both sides (§1.3),
including the walkCommit layer under both axiom corners (§1.2 + F3's
correction). The only surviving indistinguishability-shaped obligations
are (a) inside the patience corollary — where "observations constant
through the fill window" is trivially true but the corollary itself needs
F2's repair — and (b) in the brief's §7.2/§7.3 side questions
(M2-narrowing, per-party knowledge), which are panel-level model fixings,
not gaps in this proof. Note the asymmetry this leaves for the *record*:
if L2 fails and C1-as-stated is resurrected, THAT proof will need the
tree-level fooling machinery and will inherit the full realizability
burden; C1-WC never does.

**(ii) The pigeonhole accounting — does it correctly charge the 17
handoff slots and per-stream slack?** The accounting is family-specific
and correct as far as it goes: it charges exactly one slot (the wall
stream's), the parked consumer's 3 consumed frames (forced by quiescence,
§1.4), and the C pipe cells — it does not need and does not use the other
16 slots, which is right, because the jam is single-stream HOL, not
multi-slot exhaustion. Two corrections. First, per F1, the charge to the
pipe's C cells is *inessential*: the capacity-uniform mechanism (slot
occupation + FIFO burial) kills every C with a fixed wall, so the
"per-stream slack + pipe" budget should be recast as "per-stream slack +
**one permanent pipe resident**", which is what the empirical record
(≈3 frames/stream slack, buffer-size invariance) actually says. Second,
the "3 consumed" is not a universal slack constant but the park position
of this family (scope 3, forced by the pending-1/pending-0 resolution
pattern through the cap-1 upperRes cell); a Lean statement should derive
it from the skeleton (`parkIndex sk`) rather than hard-code 3, or the
next skeleton variant will silently break the arithmetic.

**(iii) The strategy-class definition — is work-conservation doing hidden
work? would a 1-step-idle strategy evade it?** Work-conservation is doing
*exactly* the work it should, and the probe isolates it beautifully: the
'cert' policy — work-conserving but preferring certified frames whenever
any exist — deadlocks the same ~30% of skeletons, wedging precisely at
states where the pipe has room, a frame is enabled, and *no* enabled
frame is certified (probe §3 finding 4) [checked]. So frame choice is not
the missing ingredient; the forced push at proof-less states is. That is
the honest formalization of §5D's "sender-side scheduling alone"
(deadlock-doc map §4.1), and the shipped bottom-most-ready mux is in the
class (rust-streaming map §2.2). A 1-step-idle strategy: under pure M2
such a strategy cannot exist (F2); under tick-extended M2 it is defeated
by the corollary's B+1-consultation adversary, which I checked goes
through once the type repair lands. What the class does NOT cover —
unbounded observation-conditioned idlers — is not hidden work but the
theorem's entire point, and M3 keeps that residual class honest by
charging idle-refusal states as deadlocks unless the idler can prove
liveness (σ\*'s burden, owned by refute-c1).

**(iv) Consistency with the probe — does the impossibility class provably
exclude σ\*?** Yes, definitionally and mechanistically. Definitionally: at
the probe's wedge states (room + enabled + nothing certified), WC must
push; σ\* by definition idles; so σ\* ∉ WC, and σ\* is also outside any
tick-extended bounded-patience class (its idling at the fill observations
is unbounded in consultations, bounded only by observation *changes*).
Mechanistically, I traced both strategies through the fixed witness at
large C: σ\*'s demand rule withholds exactly the fatal frame (provision
w+1, whose predecessor-consumption is gated behind the parked walk and
derivable by neither certification nor push-free inevitability), so its
own pipe history never contains an undeliverable resident, and the
k = 1 base case then pushes the deep reply into a clean pipe — while WC
is forced to create the resident and then forced to bury the deep reply
behind it. The probe's "σ\* idles exactly where eager dies" (24
idling skeletons ⊇ all 20 eager-deadlocking ones, zero
eager-deadlocks without σ\*-idling) is the empirical mirror of this
complementarity [checked]. One caveat inherited from the probe, correctly
flagged there and in prove-c1 §3.2/§7.1: the probe's σ\* is omniscient
and the sampled runs are not a proof of H-b; but C1-WC is independent of
whether σ\* ultimately stands — if L2/coverage fails, C1-as-stated
becomes true and C1-WC is subsumed, not contradicted.

Also checked for consistency: the probe's ~30% random-skeleton deadlock
rate across five WC policies at five capacities, the identical
deadlocking sets across policies (wedge is skeleton-intrinsic), and the
one-to-one mapping of the probe's stuck anatomy onto the empirical
six-link cycle (deadlock-doc map §1.3) — all corroborate, none
contradict, the brief's H-a [checked].

---

## 4. Recommended statement for the panel (post-repair shape)

```lean
-- fixed witness, capacity-uniform, both corners:
def wedge : Skel := ⟨…regression shape, rootH := 6, w := 4…⟩   -- probe's minimal
theorem wedge_wellFormed : wedge.wellFormed = true
theorem wedge_margin0    : ∀ s, wedge.dCount s ≤ wedge.capLevel

theorem wc_impossibility (C : Nat) (hC : 1 ≤ C) (σI σR : Strategy)
    (hI : WorkConserving σI) (hR : WorkConserving σR) :
    ∃ s, MuxReachable wedge .impl C σI σR s ∧
         muxStuck wedge s = true ∧ muxTerminal wedge s = false
-- proof: commit_singleton (ledger-forced order) + push_singleton (adversary
-- withholds R→I deliveries through the wall) + resident lemma (slot + one
-- pipe frame permanently unabsorbable) + burial (WC forces the deep push
-- behind the resident) + closed-form stuck-state decode; decide anchors at
-- C ∈ {1,2,3} in the Controls idiom (the probe's traces are the run scripts).
```

with (a) the `prov C` family retained as the sender-blocking secondary
instance and the demux-variant robustness lemmas, (b) the Rust corollary
bridged explicitly through the committed seeds, (c) the frontier
statement re-derived per F2's chosen repair, and (d) M3 footnoted as
unnecessary for this theorem (the stuck state satisfies the charter's
strategy-free deadlock definition — a robustness worth advertising, since
M3 was the brief's most contestable modeling decision).

## 5. Residuals for the panel (not findings against this brief)

- L2 / §2.4-coverage (the σ\* soundness gap) remains the campaign's live
  hinge; both briefs locate it identically and refute-c1's proposed
  decidable per-skeleton probe ("every needed label rides a τ-below
  frame") is the cheapest next discriminator. C1-WC is insensitive to its
  outcome.
- The per-party knowledge definition (`LocalObs`) is still the genuinely
  new formal object; C1-WC needs only its trivial fragment (WC never
  inspects observations at singleton states), which is another argument
  for landing C1-WC first.
- H-c's pricing question is out of this model's reach (probe §5); nothing
  in prove-c1's H-c remarks (§4.5) is load-bearing for the impossibility.

## 6. Summary judgment

- **Fatal findings: none.**
- **Repairable: F1 (restate ∃sk ∀C with the fixed witness; recast the
  budget as slot + permanent resident), F2 (patience corollary ill-typed
  under pure M2; frontier restated), F3 (D5 order slip), F4
  (realizability hypothesis / seed-bridged corollary), F5 (fix the
  commit-control boundary).**
- **Verdict: C1-WC survives cross-examination and, after F1, comes out
  stronger than briefed — a single realizable skeleton defeats every
  work-conserving pair at every capacity, under both axiom corners, by a
  strategy-consultation-free forced run. C1 as literally chartered stays
  refuted by σ\* pending L2/coverage; the trichotomy's H-a leg is ready
  for phase 3.**
