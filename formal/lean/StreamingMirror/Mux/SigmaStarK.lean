/-
σ*ₖ: the K-parameterized window discipline and its strategy class
(T8-SPEC.md, clauses 4 and 5) — the demand rule of Mux/SigmaStar.lean
and Mux/Causal.lean generalized from demand-lockstep (arrears 1) to the
per-direction advertised parking depths of the K-deep transport
(`deliverStepK`, Mux/Proofs/WcImpossibilityK.lean).

# The demand rule at arrears K (T8-SPEC clause 5, transcribed)

Frame seqs are 0-based (`Sched.Ev` numbering): the next frame on stream
`h` is the `pushedCount tr h`-th, and it is *licensed* at gate depth K
iff

    pushedCount tr h < K  ∨  rcv(c, pushedCount tr h − K) ∈ inevitableA

— the first K frames of a stream ride free (they land in the
receiver's K parked cells even if it never consumes), and every later
frame waits until the receiver's consumption of its K-arrears
predecessor is derivable from the ANNOUNCED closure. In the spec's
1-based reading this is exactly "frame k licensed iff k ≤ K ∨
rcv(c, k−K) ∈ inevitableA". The arrears form is DERIVED, not chosen:
the K-deep demux guard is `chan c < recvDepth`, so a full parked cell
holds exactly K frames past the consumer, and Step 1 of the liveness
argument (the pipes-drain, Mux/Proofs/SigmaStarKLive.lean) needs the
head's push-time certificate to name precisely `rcv(c, recvdOf)` — any
looser arrears breaks the keystone contradiction, any tighter one is
not what the transport permits. At K = 1 the rule degenerates to the
landed causal demand rule (`demandedAK_one`; `sigmaStarK_one` pins the
whole strategy).

# The licensing predicate is CAUSAL (clause 5's audit rule)

`demandedAK` reads the session only through the announced view
(`aviewOf` — the parameters plus what the arrived frames have
determined) and the machine's own history: the closure is
`inevitableA`, the announced-closure of Mux/Causal.lean, never the
omniscient `inevitable`. An omniscient-closure statement would spec an
engine the implementation cannot compute and fails the T8 audit by
name.

# The strategy class (clause 4, transcribed)

`WindowDisciplined K p σ` is the class of strategies that send "in ANY
order permitted by the window discipline": at every realizable
observation of the K-composition,

- **gate** (clause 5's "gated only by"): every frame σ names is
  licensed — σ never outruns the window discipline;
- **progress** (clause 4's "performs some licensed push"): whenever at
  least one licensed frame exists, σ names one — σ never idles inside
  the window.

Which licensed frame σ picks is completely free — that freedom is the
point of the clause: T8 quantifies over the whole class, so the
implementation's frame ordering is provably impossible to get wrong.
The class is guarded by `KConsistentAny` (realizability under the
K-composition, mirroring `KWorkConserving`'s posture: a strategy is
constrained only where the composition can actually put it), which
makes the class as wide — and the ∀-class theorem as strong — as the
self-identifying inhabitants allow. Two inhabitants are kernel-pinned:

- `sigmaStarK`, the canonical least-frame selector (the σ*-causal
  selection order, `wireHeightsA` head-first);
- `sigmaLadderK`, the shipped priority ladder's shape — the
  `bottomMostReady` reverse-index poll (Mux/Instances.lean,
  outgoing.rs:199-224) run over the licensed set, deepest stream
  first — so the spec's "the shipped priority ladder is another
  instance" is a checked theorem (`sigmaLadderK_windowDisciplined`),
  not prose.

# Which depth gates which party

A party's frames park at its PEER's demux, so its gate depth is the
depth its peer advertised: the initiator's strategy is gated at `KR`
(`recvDepth KI KR .I`), the responder's at `KI`. T8
(Mux/Proofs/SigmaStarKLive.lean) therefore pairs
`WindowDisciplined KR .I σI` with `WindowDisciplined KI .R σR`.
-/
import StreamingMirror.Mux.Causal
import StreamingMirror.Mux.Proofs.WcImpossibilityK

namespace StreamingMirror.Mux

open Model

-- ================================================ the arrears-K demand rule

/-- Is the next frame on stream `h` licensed at gate depth `K`, from
announced information? The first K frames are unconditionally licensed
(they fit the receiver's parked cells); frame `n ≥ K` needs the
receiver's consumption of its K-arrears predecessor `rcv(c, n − K)` in
the announced closure — the module doc derives this exact form from
the `deliverStepK` guard. `demandedAK 1 = demandedA`
(`demandedAK_one`). -/
def demandedAK (K : Nat) (av : AView) (tr : List MObs) (h : Nat) : Bool :=
  decide (pushedCount tr h < K) ||
    (inevitableA av tr).contains
      (Chan.wire av.party h, false, pushedCount tr h - K)

/-- At gate depth 1 the arrears rule is the landed causal demand rule:
`n < 1` is `n = 0`, and the arrears-1 predecessor is the immediate
one. -/
theorem demandedAK_one (av : AView) (tr : List MObs) (h : Nat) :
    demandedAK 1 av tr h = demandedA av tr h := by
  rw [demandedAK, demandedA]
  congr 1
  cases pushedCount tr h with
  | zero => rfl
  | succ n => simp

/-- The licensed streams at observation `tr` for a party holding view
`av`, gated at depth `K`: history-held (`committedInHist`) and
licensed (`demandedAK`), enumerated in the view's stream order. A pure
function of `(av, tr)` — the causal carrier — which is what clause 5's
"gated only by an inference from its own tree and decoded frames"
means here. -/
def licensedA (K : Nat) (av : AView) (tr : List MObs) : List Nat :=
  (wireHeightsA av av.party).filter fun h =>
    committedInHist av.rootH tr h && demandedAK K av tr h

/-- Party `p`'s licensed streams under skeleton `sk`: `licensedA` at
`p`'s announced view. -/
def licensedK (sk : Skel) (K : Nat) (p : Party) (tr : List MObs) :
    List Nat :=
  licensedA K (aviewOf sk p tr) tr

-- ========================================================== the inhabitants

/-- The canonical selector core over a view: the first licensed held
stream in the view's order — `causalCore` at gate depth `K`. -/
def kCore (K : Nat) (av : AView) (tr : List MObs) : Option Nat :=
  (wireHeightsA av av.party).find? fun h =>
    committedInHist av.rootH tr h && demandedAK K av tr h

/-- The ladder selector core over a view: the DEEPEST licensed held
stream — the `bottomMostReady` reverse-index poll among licensed
frames. -/
def kLadderCore (K : Nat) (av : AView) (tr : List MObs) : Option Nat :=
  (wireHeightsA av av.party).reverse.find? fun h =>
    committedInHist av.rootH tr h && demandedAK K av tr h

/-- σ*ₖ, the canonical window-disciplined selector: the first licensed
held stream in the view's order (`rootH` first, then the walk stages
top down) — `sigmaStarCausal`'s selection rule with the demand rule at
gate depth `K`. `sigmaStarK 1 = sigmaStarCausal` (`sigmaStarK_one`). -/
def sigmaStarK (K : Nat) : Strategy := fun sk tr =>
  match partyOf tr with
  | none => none
  | some p => kCore K (aviewOf sk p tr) tr

/-- The priority-ladder inhabitant: the shipped mux's reverse-index
poll (`bottomMostReady`, Mux/Instances.lean — deepest ready stream
first) run over the LICENSED set. Minted so the spec's clause-4
sentence "the shipped priority ladder is another instance" is a
theorem (`sigmaLadderK_windowDisciplined`) rather than an intention. -/
def sigmaLadderK (K : Nat) : Strategy := fun sk tr =>
  match partyOf tr with
  | none => none
  | some p => kLadderCore K (aviewOf sk p tr) tr

/-- The K = 1 degeneration, definitional up to the `n < 1 ↔ n = 0`
spelling: σ*ₖ at the demand-lockstep depth IS the landed σ*-causal —
strategy-level, every skeleton, every history. -/
theorem sigmaStarK_one : sigmaStarK 1 = sigmaStarCausal := by
  funext sk tr
  rw [sigmaStarK, sigmaStarCausal]
  cases partyOf tr with
  | none => rfl
  | some p =>
      show kCore 1 (aviewOf sk p tr) tr = causalCore (aviewOf sk p tr) tr
      rw [kCore, causalCore]
      congr 1
      funext h
      rw [demandedAK_one]

-- ============================================== history attribution, K-side

/-- One K-variant step's history effect, arm-generic: the base and push
arms are the record harness's (definitionally shared), and the K-deep
deliver differs only in its guard, which the history shape never
reads. -/
private theorem applyK_hist_cases {sk : Skel} {ax : AxMode}
    {KI KR C : Nat} {σI σR : Strategy} {ma : MAction} {s₀ s₁ : MState}
    (hstep : applyK sk ax KI KR C σI σR ma s₀ = some s₁) (p : Party) :
    s₁.hist p = s₀.hist p
      ∨ (∃ b, s₁.hist p = s₀.hist p ++ [MObs.act b] ∧ actionParty b = p)
      ∨ (∃ o, s₁.hist p = s₀.hist p ++ [o] ∧ ∀ b, o ≠ MObs.act b) := by
  cases ma with
  | base a =>
      have hstep' : apply sk ax C σI σR (.base a) s₀ = some s₁ := hstep
      exact apply_hist_cases hstep' p
  | push q =>
      have hstep' : apply sk ax C σI σR (.push q) s₀ = some s₁ := hstep
      exact apply_hist_cases hstep' p
  | deliver q =>
      have hrec : ∀ (q₀ : Party) (o : MObs),
          s₁.hist = recordObs s₀.hist q₀ o →
          (p = q₀ → s₁.hist p = s₀.hist p ++ [o])
            ∧ (p ≠ q₀ → s₁.hist p = s₀.hist p) := by
        intro q₀ o hh
        have hpq : s₁.hist p
            = if p == q₀ then s₀.hist p ++ [o] else s₀.hist p := by
          rw [hh]; rfl
        constructor
        · intro hp
          rwa [if_pos (by simp [hp])] at hpq
        · intro hp
          rwa [if_neg (by simp [hp])] at hpq
      have hstep' : deliverStepK KI KR q s₀ = some s₁ := hstep
      unfold deliverStepK at hstep'
      split at hstep'
      next c rest _ =>
        split at hstep'
        · injection hstep' with hs₁
          have hh : s₁.hist
              = recordObs s₀.hist q.other (.delivered (wireHeight c)) := by
            rw [← hs₁]
          by_cases hp : p = q.other
          · exact Or.inr (Or.inr ⟨.delivered (wireHeight c),
              (hrec _ _ hh).1 hp, by intro b hb; cases hb⟩)
          · exact Or.inl ((hrec _ _ hh).2 hp)
        · cases hstep'
      next => cases hstep'

/-- Histories attribute correctly at every K-reachable state, with no
well-formedness hypothesis and any axiom mode: the wf-free
`histParty_reachable`, transported to the K composition. -/
theorem histParty_reachableK {sk : Skel} {ax : AxMode} {KI KR C : Nat}
    {σI σR : Strategy} {s : MState}
    (hr : KMReachable sk ax KI KR C σI σR s) :
    ∀ p a, MObs.act a ∈ s.hist p → actionParty a = p := by
  induction hr with
  | init =>
      intro p a hmem
      cases hmem
  | step ma hr' hstep ih =>
      intro p a hmem
      rcases applyK_hist_cases hstep p with heq | ⟨b, heq, hbp⟩
        | ⟨o, heq, hno⟩
      · rw [heq] at hmem
        exact ih p a hmem
      · rw [heq, List.mem_append] at hmem
        rcases hmem with hold | hnew
        · exact ih p a hold
        · rw [List.mem_singleton] at hnew
          injection hnew with hab
          rw [hab]
          exact hbp
      · rw [heq, List.mem_append] at hmem
        rcases hmem with hold | hnew
        · exact ih p a hold
        · rw [List.mem_singleton] at hnew
          exact absurd hnew.symm (hno a)

-- =========================================================== the class

/-- Observation realizability under the K-deep transport: some
K-composition run of some depths, capacity, and strategy pair puts
`tr` on machine `p` — the `Consistent`/`ConsistentImpl` pattern over
`KMReachable`, and the guard `WindowDisciplined` quantifies under
(mirroring `KWorkConserving`: a strategy is constrained only where the
K-composition can actually put it). -/
def KConsistentAny (p : Party) (sk : Skel) (tr : List MObs) : Prop :=
  ∃ (ax : AxMode) (KI KR C : Nat) (σI σR : Strategy) (s : MState),
    KMReachable sk ax KI KR C σI σR s ∧ s.hist p = tr

/-- `partyOf` pins the machine on any K-realizable trace: a hit names
the history's owner. -/
theorem partyOf_kConsistent {p : Party} {sk : Skel} {tr : List MObs}
    (hc : KConsistentAny p sk tr) {q : Party}
    (hq : partyOf tr = some q) : q = p := by
  obtain ⟨ax, KI, KR, C, σI, σR, s, hr, htr⟩ := hc
  rw [← htr] at hq
  rw [partyOf] at hq
  obtain ⟨o, ho, hsome⟩ := List.exists_of_findSome?_eq_some hq
  cases o with
  | act a =>
      have := histParty_reachableK hr p a ho
      simp only [Option.some.injEq] at hsome
      rw [← hsome, this]
  | pushed h => cases hsome
  | delivered h => cases hsome

/-- σ is window-disciplined for party `p` at gate depth `K`: at every
K-realizable observation, every frame it names is licensed (the GATE —
clause 5's "gated only by", the conjunct that keeps σ inside the
window discipline) and it names some frame whenever a licensed one
exists (PROGRESS — clause 4's "performs some licensed push", the
conjunct that keeps σ from idling inside the window). WHICH licensed
frame σ picks is free: T8 quantifies over this whole class (module
doc). -/
def WindowDisciplined (K : Nat) (p : Party) (σ : Strategy) : Prop :=
  ∀ (sk : Skel) (tr : List MObs), KConsistentAny p sk tr →
    (∀ h, σ sk tr = some h → h ∈ licensedK sk K p tr)
    ∧ (licensedK sk K p tr ≠ [] → (σ sk tr).isSome = true)

-- ================================================ the class inhabitation

/-- A licensed stream's `committedInHist` puts a wire commit on the
history, so the machine can identify itself. -/
private theorem partyOf_isSome_of_licensed {sk : Skel} {K : Nat}
    {p : Party} {s : MState} {h₀ : Nat}
    (hmem : h₀ ∈ licensedK sk K p (s.hist p)) :
    (partyOf (s.hist p)).isSome = true := by
  obtain ⟨-, hpred⟩ := List.mem_filter.mp hmem
  rw [Bool.and_eq_true] at hpred
  have hcm := hpred.1
  rw [show (aviewOf sk p (s.hist p)).rootH = sk.rootH from rfl,
    committedInHist_eq, decide_eq_true_eq] at hcm
  refine partyOf_isSome_of_commits (sk := sk) (h := h₀) ?_
  omega

/-- σ*ₖ inhabits its class: the canonical least-frame selector is
window-disciplined at its own gate depth, for both parties — the
spec's clause-4 "canonical scheduler is one instance", checked. -/
theorem sigmaStarK_windowDisciplined (K : Nat) (p : Party) :
    WindowDisciplined K p (sigmaStarK K) := by
  intro sk tr hc
  constructor
  · -- the gate: a named frame is a licensed-set member
    intro h hσ
    rw [sigmaStarK] at hσ
    cases hq : partyOf tr with
    | none =>
        rw [hq] at hσ
        cases hσ
    | some q =>
        rw [hq] at hσ
        have hqp : q = p := partyOf_kConsistent hc hq
        subst hqp
        have hσ' : kCore K (aviewOf sk q tr) tr = some h := hσ
        rw [kCore] at hσ'
        have hmem := List.mem_of_find?_eq_some hσ'
        have hpred := List.find?_some hσ'
        exact List.mem_filter.mpr ⟨hmem, hpred⟩
  · -- progress: a licensed member makes the selector fire
    intro hne
    obtain ⟨h₀, hh₀⟩ := List.exists_mem_of_ne_nil _ hne
    obtain ⟨ax, KI, KR, C, σI, σR, s, hr, htr⟩ := hc
    subst htr
    have hsome := partyOf_isSome_of_licensed hh₀
    obtain ⟨q, hq⟩ := Option.isSome_iff_exists.mp hsome
    have hqp : q = p :=
      partyOf_kConsistent ⟨ax, KI, KR, C, σI, σR, s, hr, rfl⟩ hq
    subst hqp
    obtain ⟨hmem, hpred⟩ := List.mem_filter.mp hh₀
    rw [sigmaStarK, hq]
    show (kCore K (aviewOf sk q (s.hist q)) (s.hist q)).isSome = true
    rw [kCore, List.find?_isSome]
    exact ⟨h₀, hmem, hpred⟩

/-- The shipped ladder inhabits the class: the `bottomMostReady`
reverse-index poll over the licensed set is window-disciplined — the
spec's clause-4 "the shipped priority ladder is another instance",
checked. With `sigmaStarK_windowDisciplined` this is why clause 4
exists: T8 covers every frame ordering the implementation could
choose, these two included. -/
theorem sigmaLadderK_windowDisciplined (K : Nat) (p : Party) :
    WindowDisciplined K p (sigmaLadderK K) := by
  intro sk tr hc
  constructor
  · -- the gate: a named frame is a licensed-set member
    intro h hσ
    rw [sigmaLadderK] at hσ
    cases hq : partyOf tr with
    | none =>
        rw [hq] at hσ
        cases hσ
    | some q =>
        rw [hq] at hσ
        have hqp : q = p := partyOf_kConsistent hc hq
        subst hqp
        have hσ' : kLadderCore K (aviewOf sk q tr) tr = some h := hσ
        rw [kLadderCore] at hσ'
        have hmem := List.mem_of_find?_eq_some hσ'
        have hpred := List.find?_some hσ'
        exact List.mem_filter.mpr ⟨List.mem_reverse.mp hmem, hpred⟩
  · -- progress: a licensed member makes the selector fire
    intro hne
    obtain ⟨h₀, hh₀⟩ := List.exists_mem_of_ne_nil _ hne
    obtain ⟨ax, KI, KR, C, σI, σR, s, hr, htr⟩ := hc
    subst htr
    have hsome := partyOf_isSome_of_licensed hh₀
    obtain ⟨q, hq⟩ := Option.isSome_iff_exists.mp hsome
    have hqp : q = p :=
      partyOf_kConsistent ⟨ax, KI, KR, C, σI, σR, s, hr, rfl⟩ hq
    subst hqp
    obtain ⟨hmem, hpred⟩ := List.mem_filter.mp hh₀
    rw [sigmaLadderK, hq]
    show (kLadderCore K (aviewOf sk q (s.hist q)) (s.hist q)).isSome = true
    rw [kLadderCore, List.find?_isSome]
    exact ⟨h₀, List.mem_reverse.mpr hmem, hpred⟩

end StreamingMirror.Mux
