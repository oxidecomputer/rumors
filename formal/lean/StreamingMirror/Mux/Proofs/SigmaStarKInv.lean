/-
The K-variant preservation sweep (T8's invariant layer): the
strategy-generic ground facts and the σ*ₖ-class push certificates,
along `KMReachable` — the stage-F pattern (Mux/Proofs/SigmaStarInv.lean)
re-assembled for the K-deep transport, per the F/E/T10 house pattern.

# What is genuinely new here, and what is shared

The K composition (`applyK`, Mux/Proofs/WcImpossibilityK.lean) shares
the base and push arms with the record harness DEFINITIONALLY, so the
landed per-arm sweep transfers verbatim once the occupancy bound is a
parameter (`SInvB`, the commit that generalized the stage-F sweep):

- the ground facts at the K bound are `MuxInvB (kcap KI KR sk)` — wire
  cells bounded by the RECEIVING party's advertised depth (the
  per-direction parking bounds, as ground facts), internal channels by
  the base capacity as always;
- the only new preservation obligation is the K-deep deliver arm
  (`sinvK_deliver`), and its whole content is one inequality: the
  depth guard `chan < recvDepth` lands the frame within the parked
  bound — `SInvB.deliver_shape` does the rest;
- the receive ledger (`RecvLedger`, Mux/Proofs/CausalCoverage.lean) is
  bound-free; its K sweep re-dispatches the landed base/push lemmas
  and re-assembles the deliver arm through `RecvLedger.obs_assemble`.

# The K push certificates (INV-A at the advertised depths)

`PushProvenAK`: every recorded push past the free window carried its
arrears-K license at its own push-time observation prefix, at the gate
depth the party's sends were advertised (`recvDepth KI KR p` — the
initiator gated at K_R, the responder at K_I). Unlike the landed
`PushProvenA` (σ*-causal-specific), the K certificates are a CLASS
invariant: any `WindowDisciplined` pair maintains them, because the
class's gate conjunct is exactly the certificate at push time. This is
where T8's clause-4 quantification enters the induction — the
preservation proof consults the class, never a concrete selector.
-/
import StreamingMirror.Mux.SigmaStarK
import StreamingMirror.Mux.Proofs.CausalCoverage

namespace StreamingMirror.Mux

open Model

variable {sk : Skel}

-- ==================================================== the K occupancy bound

/-- The K-variant occupancy bound: a wire cell parks up to the
RECEIVING party's advertised depth (`deliverStepK`'s guard); every
internal channel keeps the base capacity. -/
def kcap (KI KR : Nat) (sk : Skel) : Chan → Nat
  | .wire q _ => recvDepth KI KR q
  | c => sk.cap c

/-- The base capacity is within the K bound at advertised depths ≥ 1:
wire caps are 1 and the depths dominate them; internal channels are
untouched. The hypothesis the shared base-arm sweep threads. -/
theorem cap_le_kcap {KI KR : Nat} (hKI : 1 ≤ KI) (hKR : 1 ≤ KR)
    (sk : Skel) : ∀ c, sk.cap c ≤ kcap KI KR sk c := by
  intro c
  cases c
  case wire q h =>
      show 1 ≤ recvDepth KI KR q
      cases q
      · exact hKR
      · exact hKI
  all_goals exact Nat.le_refl _

/-- The K-variant ground facts: `MuxInvB` at the per-direction parking
bound. -/
abbrev MuxInvK (KI KR : Nat) (sk : Skel) (s : MState) : Prop :=
  MuxInvB (kcap KI KR sk) sk s

/-- The K-variant strategy-generic invariant: K ground facts plus the
(bound-free) history decode. -/
abbrev SInvK (KI KR : Nat) (sk : Skel) (s : MState) : Prop :=
  SInvB (kcap KI KR sk) sk s

-- ======================================================= the K deliver arm

/-- A K-deep delivery preserves the K invariant: the depth guard lands
the frame within the receiving party's parked bound
(`SInvB.deliver_shape` at the K bound). -/
theorem sinvK_deliver {KI KR : Nat} {p : Party} {s s' : MState}
    (hstep : deliverStepK KI KR p s = some s')
    (hm : SInvK KI KR sk s) : SInvK KI KR sk s' := by
  unfold deliverStepK at hstep
  split at hstep
  next c rest hp =>
      split at hstep
      case isFalse => cases hstep
      case isTrue hg =>
        obtain ⟨g, rfl⟩ := hm.mux.pipe_mem_wire (p := p)
          (c := c) (by rw [hp]; exact List.mem_cons_self ..)
        injection hstep with hs'
        subst hs'
        refine SInvB.deliver_shape hp ?_ hm
        show s.base.chan (Chan.wire p g) + 1 ≤ recvDepth KI KR p
        omega
  next => cases hstep

-- ======================================================== the K-step sweep

/-- Every enabled K-variant step preserves the K invariant, under every
strategy pair: base and push arms are the record harness's
(definitionally shared, swept at the K bound), the deliver arm is
`sinvK_deliver`. -/
theorem sinvK_step {KI KR : Nat} (hwf : sk.wellFormed = true)
    (hKI : 1 ≤ KI) (hKR : 1 ≤ KR) {C : Nat} {σI σR : Strategy}
    {ma : MAction} {s s' : MState}
    (hstep : applyK sk .impl KI KR C σI σR ma s = some s')
    (hm : SInvK KI KR sk s) : SInvK KI KR sk s' := by
  cases ma with
  | base a =>
      have hstep' : applyBase sk .impl a s = some s' := hstep
      exact sinv_base hwf (cap_le_kcap hKI hKR sk) hstep' hm
  | push p =>
      have hstep' : apply sk .impl C σI σR (.push p) s = some s' := hstep
      exact (sinv_push hwf hstep' hm).1
  | deliver p =>
      exact sinvK_deliver hstep hm

/-- The K invariant holds at every K-reachable state: the stage-F
induction at the per-direction parking bound. -/
theorem sinvK_reachable {KI KR : Nat} (hwf : sk.wellFormed = true)
    (hKI : 1 ≤ KI) (hKR : 1 ≤ KR) {C : Nat} {σI σR : Strategy}
    {s : MState}
    (hr : KMReachable sk .impl KI KR C σI σR s) : SInvK KI KR sk s := by
  induction hr with
  | init =>
      refine ⟨muxInvB_init _ sk, ?_, ?_⟩
      · intro p a hmem
        cases hmem
      · intro p h
        rw [show (init sk).hist p = [] from rfl, holdsWire_init]
        rfl
  | step a hr' hstep ih => exact sinvK_step hwf hKI hKR hstep ih

-- ============================================== the receive ledger, K-side

/-- Every K-variant step preserves the receive ledger: the landed
base/push lemmas re-dispatched (the arms are shared), the K deliver
re-assembled — its receipt is not an `.act` and the chan bump is
invisible to consumer counts. -/
theorem recvLedger_stepK {KI KR : Nat} (hwf : sk.wellFormed = true)
    {C : Nat} {σI σR : Strategy} {ma : MAction} {s s' : MState}
    (hstep : applyK sk .impl KI KR C σI σR ma s = some s')
    (hm : SInvK KI KR sk s) (hrl : RecvLedger sk s) :
    RecvLedger sk s' := by
  cases ma with
  | base a =>
      have hstep' : applyBase sk .impl a s = some s' := hstep
      exact recvLedger_base hwf hstep' hm hrl
  | push q =>
      have hstep' : apply sk .impl C σI σR (.push q) s = some s' := hstep
      simp only [apply] at hstep'
      split at hstep'
      next h₀ _ => exact recvLedger_push hwf hstep' hm hrl
      next => cases hstep'
  | deliver q =>
      have hstep' : deliverStepK KI KR q s = some s' := hstep
      unfold deliverStepK at hstep'
      split at hstep'
      next c rest hpp =>
          split at hstep'
          case isFalse => cases hstep'
          case isTrue h0 =>
            injection hstep' with hs'
            refine RecvLedger.obs_assemble (q₀ := q.other)
              (o := .delivered (wireHeight c)) hrl
              (fun a hc => by cases hc) (fun q' g hmc => ?_)
              (by rw [← hs'])
            rw [← hs']
            exact recvdOf_chan_blind _ _
      next => cases hstep'

/-- The receive ledger holds at every K-reachable state. -/
theorem recvLedger_reachableK {KI KR : Nat} (hwf : sk.wellFormed = true)
    (hKI : 1 ≤ KI) (hKR : 1 ≤ KR) {C : Nat} {σI σR : Strategy}
    {s : MState}
    (hr : KMReachable sk .impl KI KR C σI σR s) : RecvLedger sk s := by
  induction hr with
  | init =>
      refine ⟨?_, ?_⟩
      · intro p h hmem
        show ownRecvs sk.rootH p [] h ≤ _
        rw [show ownRecvs sk.rootH p [] h = 0 from rfl]
        omega
      · intro p h hne
        exact absurd rfl hne
  | step ma hr' hstep ih =>
      exact recvLedger_stepK hwf hstep (sinvK_reachable hwf hKI hKR hr') ih

-- ============================================ the K push certificates

/-- The K-gated push certificates (INV-A at the advertised depths):
every recorded push past the free window carried its arrears-K license
at its own push-time observation prefix — gate depth
`recvDepth KI KR p`, the depth the party's sends were advertised. A
CLASS invariant: any `WindowDisciplined` pair maintains it (module
doc). -/
def PushProvenAK (KI KR : Nat) (sk : Skel) (s : MState) : Prop :=
  ∀ p i h, (s.hist p)[i]? = some (.pushed h) →
    recvDepth KI KR p ≤ pushedCount ((s.hist p).take i) h →
    (Chan.wire p h, false,
        pushedCount ((s.hist p).take i) h - recvDepth KI KR p)
      ∈ inevitableA (aviewOf sk p ((s.hist p).take i)) ((s.hist p).take i)

/-- Extending a history by one observation keeps every K certificate,
provided the new observation carries its own (the `certsA_snoc`
trichotomy at gate depth `K`). -/
private theorem certsAK_snoc {K : Nat} {p : Party} {tr : List MObs}
    {o : MObs}
    (hcert : ∀ i h, tr[i]? = some (.pushed h) →
      K ≤ pushedCount (tr.take i) h →
      (Chan.wire p h, false, pushedCount (tr.take i) h - K)
        ∈ inevitableA (aviewOf sk p (tr.take i)) (tr.take i))
    (hnew : ∀ h, o = .pushed h → K ≤ pushedCount tr h →
      (Chan.wire p h, false, pushedCount tr h - K)
        ∈ inevitableA (aviewOf sk p tr) tr) :
    ∀ i h, (tr ++ [o])[i]? = some (.pushed h) →
      K ≤ pushedCount ((tr ++ [o]).take i) h →
      (Chan.wire p h, false, pushedCount ((tr ++ [o]).take i) h - K)
        ∈ inevitableA (aviewOf sk p ((tr ++ [o]).take i))
            ((tr ++ [o]).take i) := by
  intro i h hget hcnt
  rcases Nat.lt_trichotomy i tr.length with hlt | heq | hgt
  · rw [List.getElem?_append_left hlt] at hget
    rw [List.take_append_of_le_length (Nat.le_of_lt hlt)] at hcnt ⊢
    exact hcert i h hget hcnt
  · subst heq
    rw [List.getElem?_concat_length] at hget
    injection hget with hget
    rw [List.take_append_of_le_length (Nat.le_refl _),
      List.take_length] at hcnt ⊢
    exact hnew h hget hcnt
  · rw [List.getElem?_eq_none (by
      rw [List.length_append, List.length_cons, List.length_nil]
      omega)] at hget
    cases hget

/-- Every K-composition step under a `WindowDisciplined` pair preserves
the K push certificates: non-push observations are neutral, and a push
observation carries the license the class's GATE conjunct guarantees
at push time — the point where T8's clause-4 quantification enters the
induction. -/
theorem pushProvenAK_step {KI KR : Nat} (hwf : sk.wellFormed = true)
    {C : Nat} {σI σR : Strategy}
    (hWI : WindowDisciplined KR .I σI)
    (hWR : WindowDisciplined KI .R σR)
    {ma : MAction} {s s' : MState}
    (hr : KMReachable sk .impl KI KR C σI σR s)
    (hstep : applyK sk .impl KI KR C σI σR ma s = some s')
    (hm : SInvK KI KR sk s) (hp : PushProvenAK KI KR sk s) :
    PushProvenAK KI KR sk s' := by
  have hgen : ∀ (q₀ : Party) (o : MObs),
      (∀ h, o = .pushed h →
        recvDepth KI KR q₀ ≤ pushedCount (s.hist q₀) h →
        (Chan.wire q₀ h, false,
            pushedCount (s.hist q₀) h - recvDepth KI KR q₀)
          ∈ inevitableA (aviewOf sk q₀ (s.hist q₀)) (s.hist q₀)) →
      s'.hist = recordObs s.hist q₀ o →
      PushProvenAK KI KR sk s' := by
    intro q₀ o hnew hh
    have hrec : ∀ q, s'.hist q
        = if q == q₀ then s.hist q ++ [o] else s.hist q := by
      intro q
      rw [hh]
      rfl
    intro q i h
    rw [hrec]
    by_cases hq : q = q₀
    · subst hq
      rw [if_pos (by simp)]
      exact certsAK_snoc (hp q) hnew i h
    · rw [if_neg (by simp [hq])]
      exact hp q i h
  cases ma with
  | base a =>
      have hstep' : applyBase sk .impl a s = some s' := hstep
      obtain ⟨-, b, -, hs'⟩ := applyBase_inv hstep'
      refine hgen (actionParty a) (.act a) ?_ (by rw [hs'])
      intro h hcon
      cases hcon
  | deliver q =>
      have hstep' : deliverStepK KI KR q s = some s' := hstep
      unfold deliverStepK at hstep'
      split at hstep'
      next c rest hpp =>
          split at hstep'
          case isFalse => cases hstep'
          case isTrue h0 =>
            injection hstep' with hs'
            refine hgen q.other (.delivered (wireHeight c)) ?_
              (by rw [← hs'])
            intro h hcon
            cases hcon
      next => cases hstep'
  | push q =>
      have hstep' : apply sk .impl C σI σR (.push q) s = some s' := hstep
      obtain ⟨-, h, hσ, hh⟩ := sinv_push hwf hstep' hm
      have hKC : KConsistentAny q sk (s.hist q) :=
        ⟨.impl, KI, KR, C, σI, σR, s, hr, rfl⟩
      have hlic : h ∈ licensedK sk (recvDepth KI KR q) q (s.hist q) := by
        cases q with
        | I => exact (hWI sk (s.hist .I) hKC).1 h hσ
        | R => exact (hWR sk (s.hist .R) hKC).1 h hσ
      obtain ⟨-, hpred⟩ := List.mem_filter.mp hlic
      rw [Bool.and_eq_true] at hpred
      have hdem := hpred.2
      refine hgen q (.pushed h) ?_ hh
      intro h' heq hcnt
      have heq' : h = h' := by injection heq
      subst heq'
      rw [demandedAK, Bool.or_eq_true] at hdem
      rcases hdem with hlt | hmem
      · exfalso
        rw [decide_eq_true_eq] at hlt
        omega
      · exact (List.contains_iff_mem ..).mp hmem

/-- The K push certificates hold along every `WindowDisciplined`-pair
run. -/
theorem pushProvenAK_reachable {KI KR : Nat} (hwf : sk.wellFormed = true)
    (hKI : 1 ≤ KI) (hKR : 1 ≤ KR) {C : Nat} {σI σR : Strategy}
    (hWI : WindowDisciplined KR .I σI)
    (hWR : WindowDisciplined KI .R σR) {s : MState}
    (hr : KMReachable sk .impl KI KR C σI σR s) :
    PushProvenAK KI KR sk s := by
  induction hr with
  | init =>
      intro p i h hget
      rw [show (init sk).hist p = [] from rfl] at hget
      cases i <;> cases hget
  | step a hr' hstep ih =>
      exact pushProvenAK_step hwf hWI hWR hr' hstep
        (sinvK_reachable hwf hKI hKR hr') ih

end StreamingMirror.Mux
