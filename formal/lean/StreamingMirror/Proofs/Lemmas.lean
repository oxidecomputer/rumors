/-
The shared lemma library under the proof stack: channel-occupancy
algebra (`bump`), state-update projections (`setWalk`/`setAsm`),
prefix-counting over `List.range`, and the congruence ("frame") lemmas
that let a per-action preservation proof dispatch every part of the
invariant the action cannot touch.

Conventions set here and used by every file under `Proofs/`:

- Proofs work against `InvP`, the Prop-level restatement of the
  executable `Inv` (`inv_iff`), so hypotheses arrive as `∀ pk ∈ …`
  quantifiers and equations rather than `&&`-chains.
- Flow reasoning is pointwise per channel: `sentOf_ext`/`recvdOf_ext`
  frame the channels an action's state delta cannot reach; the touched
  channel gets a bespoke argument in the action's own lemma.
- Preservation proofs never `subst` the successor state: `injection`
  names the equation `hs' : ⟨record⟩ = s'`. A component the action does
  not touch is dispatched by `rw [← hs']` on the goal followed by
  `exact hi.<field> …` — the record projections make the two sides
  definitionally equal. Only components the action genuinely changes
  get named `have` equations (always with explicit types: an inline
  `(by rw [← hs'])` argument elaborates against metavariables and
  silently degenerates). See `preserve_finRet` for the template.
-/
import Batteries
import StreamingMirror.Invariant

namespace StreamingMirror.Model

-- ======================================================== bump algebra

@[simp] theorem bump_self (f : Chan → Nat) (c : Chan) (d : Int) :
    bump f c d c = (Int.ofNat (f c) + d).toNat := by
  simp [bump]

@[simp] theorem bump_ne (f : Chan → Nat) {c c' : Chan} (d : Int)
    (h : c' ≠ c) : bump f c d c' = f c' := by
  simp [bump, h]

theorem bump_apply (f : Chan → Nat) (c c' : Chan) (d : Int) :
    bump f c d c' = if c' = c then (Int.ofNat (f c) + d).toNat else f c' := by
  by_cases h : c' = c <;> simp [h]

/-- Sending one message: occupancy at the touched channel. -/
theorem bump_one (f : Chan → Nat) (c : Chan) : bump f c 1 c = f c + 1 := by
  simp

/-- Receiving one message: occupancy at the touched channel. -/
theorem bump_neg_one (f : Chan → Nat) (c : Chan) :
    bump f c (-1) c = f c - 1 := by
  simp; omega

-- ============================================== state-update projections

@[simp] theorem setWalk_walk_self (s : State) (pk : Party × Nat)
    (ws : WalkSt) : (setWalk s pk ws).walk pk = ws := by
  simp [setWalk]

@[simp] theorem setWalk_walk_ne (s : State) {pk pk' : Party × Nat}
    (ws : WalkSt) (h : pk' ≠ pk) :
    (setWalk s pk ws).walk pk' = s.walk pk' := by
  simp [setWalk, h]

@[simp] theorem setAsm_asm_self (s : State) (pk : Party × Nat)
    (a : AsmSt) : (setAsm s pk a).asm pk = a := by
  simp [setAsm]

@[simp] theorem setAsm_asm_ne (s : State) {pk pk' : Party × Nat}
    (a : AsmSt) (h : pk' ≠ pk) :
    (setAsm s pk a).asm pk' = s.asm pk' := by
  simp [setAsm, h]

-- ================================================ prefix counting

/-- Counting a `< i` prefix inside `range n`: exactly `i` hits when
`i ≤ n`. The workhorse behind "a prefix-closed done-ledger's count is
its frontier" (`wkWireCount` at a committed `.wire i`). -/
theorem length_filter_range_lt {i n : Nat} (h : i ≤ n) :
    ((List.range n).filter (fun j => decide (j < i))).length = i := by
  induction n with
  | zero => have : i = 0 := by omega
            subst this; simp
  | succ n ih =>
      rw [List.range_succ, List.filter_append, List.length_append]
      by_cases hi : i ≤ n
      · have : ¬ n < i := by omega
        simp [ih hi, this]
      · have hin : i = n + 1 := by omega
        subst hin
        have : ∀ j ∈ List.range n, (decide (j < n + 1)) = true := by
          intro j hj
          simp [List.mem_range] at hj
          simp; omega
        rw [List.filter_eq_self.mpr this]
        simp [List.length_range]

/-- A pointwise-characterized ledger counts like its characterization. -/
theorem length_filter_congr {p q : Nat → Bool} {l : List Nat}
    (h : ∀ x ∈ l, p x = q x) :
    (l.filter p).length = (l.filter q).length := by
  rw [List.filter_congr h]

/-- Folding `acc + f i` is summing. Lets `wkQSum`-style folds reuse
`List.sum` lemmas. -/
theorem foldl_add_eq_sum (f : Nat → Nat) (l : List Nat) (n : Nat) :
    l.foldl (fun acc i => acc + f i) n = n + (l.map f).sum := by
  induction l generalizing n with
  | nil => simp
  | cons x xs ih => simp [ih, Nat.add_assoc]

/-- Folding a constant accumulator is the identity (the `+ 0` summand
simp leaves behind on an all-zero ledger). -/
theorem foldl_const (l : List Nat) (n : Nat) :
    l.foldl (fun acc _ => acc) n = n := by
  induction l generalizing n <;> simp [List.foldl_cons, *]

/-- A prefix-closed done-ledger with frontier `i` counts to exactly `i`:
everything below the frontier is done, the frontier is not, and done-ness
propagates downward. This is the committed-`.wire i` arm of `wkLocalOk`
(`i == wkWireCount`) reduced to its combinatorial core. -/
theorem length_filter_of_frontier {p : Nat → Bool} {i fan : Nat}
    (hif : i ≤ fan)
    (hlow : ∀ j < i, p j = true)
    (hfront : p i = false)
    (hclosed : ∀ j < fan, p j = true → j = 0 ∨ p (j - 1) = true) :
    ((List.range fan).filter p).length = i := by
  have habove : ∀ j, i ≤ j → j < fan → p j = false := by
    intro j
    induction j with
    | zero =>
        intro hij _
        have h0 : i = 0 := by omega
        exact h0 ▸ hfront
    | succ j ih =>
        intro hij hjf
        by_cases hi' : i ≤ j
        · cases hpj : p (j + 1) with
          | false => rfl
          | true =>
              rcases hclosed (j + 1) hjf hpj with h0 | hprev
              · exact absurd h0 (by omega)
              · have hj : p j = false := ih hi' (by omega)
                simp only [Nat.add_sub_cancel] at hprev
                rw [hj] at hprev
                cases hprev
        · have h1 : i = j + 1 := by omega
          exact h1 ▸ hfront
  have hpoint : ∀ j ∈ List.range fan, p j = decide (j < i) := by
    intro j hj
    rw [List.mem_range] at hj
    by_cases hji : j < i
    · simp [hlow j hji, hji]
    · simp [habove j (by omega) hj, hji]
  rw [List.filter_congr hpoint]
  exact length_filter_range_lt hif

-- ======================================== well-formedness extraction

/-- Per-scope fan bounds, extracted from `wellFormed`'s per-scope pass. -/
theorem wf_scope_bounds {sk : Skel} (hwf : sk.wellFormed = true)
    {i : Nat} (hi : i < sk.scopes.length) :
    (sk.scope i).kids.length ≤ sk.fan ∧ (sk.scope i).leafReqs ≤ sk.fan := by
  unfold Skel.wellFormed at hwf
  simp only [Bool.and_eq_true, List.all_eq_true, decide_eq_true_eq] at hwf
  have hper := hwf.1.1.1.1.2
  have hbody := hper i (List.mem_range.mpr hi)
  exact ⟨hbody.1.1.1.1.1.2, hbody.1.1.1.1.2⟩

/-- Root height bounds from `wellFormed`: even and at least 2 (the root
scope exists and every scope's height is positive). Both matter to the
channel wiring: evenness makes responder stage indices even (steering
`askedOut` at index 1 to the initiator), and positivity keeps the root
wire channels clear of the `wire _ 0` phantom keys. -/
theorem wf_rootH {sk : Skel} (hwf : sk.wellFormed = true) :
    sk.rootH % 2 = 0 ∧ 2 ≤ sk.rootH := by
  unfold Skel.wellFormed at hwf
  simp only [Bool.and_eq_true, List.all_eq_true, decide_eq_true_eq,
    beq_iff_eq] at hwf
  have hn := hwf.1.1.1.1.1.1.1.1
  have hh0 := hwf.1.1.1.1.1.1.1.2
  have hev := hwf.1.1.1.1.1.2
  have hper := hwf.1.1.1.1.2
  have hge1 := (hper 0 (List.mem_range.mpr hn)).1.1.1.1.1.1
  omega

/-- A stage cursor in range denotes a real scope id. -/
theorem stageScope_lt_scopes (sk : Skel) {h k : Nat}
    (hk : k < sk.stageLen h) :
    sk.stageScope h k < sk.scopes.length := by
  have hmem : sk.stageScope h k ∈ sk.stageScopes h := by
    unfold Skel.stageScope
    unfold Skel.stageLen at hk
    rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hk,
      Option.getD_some]
    exact List.getElem_mem hk
  unfold Skel.stageScopes Skel.scopesAt at hmem
  rw [List.mem_filter] at hmem
  exact List.mem_range.mp hmem.1

/-- The child count of any real stage scope is fan-bounded: the fact
that squeezes a walk's per-child ledgers into `List.range sk.fan`. -/
theorem nChildren_le_fan {sk : Skel} (hwf : sk.wellFormed = true)
    {h k : Nat} (hk : k < sk.stageLen h) :
    sk.nChildren h (sk.stageScope h k) ≤ sk.fan := by
  obtain ⟨hkids, hleaf⟩ := wf_scope_bounds hwf (stageScope_lt_scopes sk hk)
  unfold Skel.nChildren
  split
  · exact hleaf
  · exact hkids

-- ===================================================== Prop-level Inv

/-- The invariant, restated at the Prop level: what preservation proofs
consume and produce (`inv_iff`). The `flow` field carries both the
conservation equation and the capacity bound. -/
structure InvP (sk : Skel) (ax : AxMode) (s : State) : Prop where
  wk : ∀ pk ∈ sk.walkKeys, wkLocalOk sk ax s pk = true
  asm : ∀ pk ∈ sk.asmKeys, asmLocalOk sk s pk = true
  top : topLocalOk sk ax s = true
  flow : ∀ c ∈ allChans sk,
    s.chan c + recvdOf sk s c = sentOf sk s c ∧ s.chan c ≤ sk.cap c

theorem inv_iff (sk : Skel) (ax : AxMode) (s : State) :
    Inv sk ax s = true ↔ InvP sk ax s := by
  constructor
  · intro h
    simp only [Inv, Bool.and_eq_true, List.all_eq_true] at h
    obtain ⟨⟨⟨hwk, hasm⟩, htop⟩, hflow⟩ := h
    refine ⟨hwk, hasm, htop, fun c hc => ?_⟩
    rw [flowOk, List.all_eq_true] at hflow
    have := hflow c hc
    simpa using this
  · intro ⟨hwk, hasm, htop, hflow⟩
    simp only [Inv, Bool.and_eq_true, List.all_eq_true]
    refine ⟨⟨⟨hwk, hasm⟩, htop⟩, ?_⟩
    rw [flowOk, List.all_eq_true]
    intro c hc
    have := hflow c hc
    simpa using this

-- =============================================== flow frame (congruence)

/-- Everything `sentOf` can read, at observation granularity: the walk
fields enter only through the derived counts, and `committed` is not
among them — so a committed-choice update is invisible to every producer
count. An action whose delta avoids all of these leaves every `sentOf`
unchanged. -/
theorem sentOf_ext (sk : Skel) {s s' : State}
    (hasm : ∀ pk, s'.asm pk = s.asm pk)
    (hsc : ∀ pk, (s'.walk pk).scope = (s.walk pk).scope)
    (hph : ∀ pk, (s'.walk pk).phase = (s.walk pk).phase)
    (hwd : ∀ pk, (s'.walk pk).wireDone = (s.walk pk).wireDone)
    (hrd : ∀ pk, (s'.walk pk).resDone = (s.walk pk).resDone)
    (hqs : ∀ pk, (s'.walk pk).qSent = (s.walk pk).qSent)
    (hpd : ∀ pk, (s'.walk pk).parentDone = (s.walk pk).parentDone)
    (h1 : s'.iopenWire = s.iopenWire) (h2 : s'.iopenQuery = s.iopenQuery)
    (h3 : s'.ropenWire = s.ropenWire) (h4 : s'.ropenRes = s.ropenRes)
    (h5 : s'.ropenQ = s.ropenQ) (h6 : s'.absorbIdx = s.absorbIdx) :
    ∀ c, sentOf sk s' c = sentOf sk s c := by
  intro c
  cases c <;>
    simp [sentOf, wkWireSent, wkResSent, wkQSentTot, wkParentSent,
      wkWireCount, wkResCount, wkQSum, asmOutSent,
      hasm, hsc, hph, hwd, hrd, hqs, hpd, h1, h2, h3, h4, h5, h6]

/-- Everything `recvdOf` can read; the consumer-side twin of
`sentOf_ext`. -/
theorem recvdOf_ext (sk : Skel) {s s' : State}
    (hasm : ∀ pk, s'.asm pk = s.asm pk)
    (hsc : ∀ pk, (s'.walk pk).scope = (s.walk pk).scope)
    (hph : ∀ pk, (s'.walk pk).phase = (s.walk pk).phase)
    (h1 : s'.ropenGotWire = s.ropenGotWire)
    (h2 : s'.absorbIdx = s.absorbIdx) (h3 : s'.absorbPhase = s.absorbPhase)
    (h4 : s'.ifin = s.ifin) (h5 : s'.rfinGot = s.rfinGot)
    (h6 : s'.rfinGotRes = s.rfinGotRes) :
    ∀ c, recvdOf sk s' c = recvdOf sk s c := by
  intro c
  cases c <;>
    simp [recvdOf, wkWireRecvd, wkAskedRecvd, asmResRecvd, asmLevelRecvd,
      absorbWireRecvd, absorbAskedRecvd,
      hasm, hsc, hph, h1, h2, h3, h4, h5, h6]

/-- A committed-choice update is invisible to every producer count: the
flow layer of `walkCommit` for free. -/
theorem sentOf_setWalk_committed (sk : Skel) (s : State)
    (pk : Party × Nat) (co : Option Oblig) (c : Chan) :
    sentOf sk (setWalk s pk { s.walk pk with committed := co }) c
      = sentOf sk s c := by
  apply sentOf_ext <;>
    first
      | rfl
      | exact fun _ => rfl
      | (intro pk'
         by_cases h : pk' = pk
         · subst h; simp
         · simp [h])

/-- The consumer-side twin of `sentOf_setWalk_committed`. -/
theorem recvdOf_setWalk_committed (sk : Skel) (s : State)
    (pk : Party × Nat) (co : Option Oblig) (c : Chan) :
    recvdOf sk (setWalk s pk { s.walk pk with committed := co }) c
      = recvdOf sk s c := by
  apply recvdOf_ext <;>
    first
      | rfl
      | exact fun _ => rfl
      | (intro pk'
         by_cases h : pk' = pk
         · subst h; simp
         · simp [h])

-- ================================================= local frame lemmas

/-- `wkWireCount` reads the state only through `s.walk pk`. -/
theorem wkWireCount_congr (sk : Skel) {s s' : State}
    (pk : Party × Nat) (h : s'.walk pk = s.walk pk) :
    wkWireCount sk s' pk = wkWireCount sk s pk := by
  simp only [wkWireCount, h]

/-- `wkLocalOk` reads the state only through `s.walk pk`. -/
theorem wkLocalOk_congr (sk : Skel) (ax : AxMode) {s s' : State}
    (pk : Party × Nat) (h : s'.walk pk = s.walk pk) :
    wkLocalOk sk ax s' pk = wkLocalOk sk ax s pk := by
  simp only [wkLocalOk, wkWireCount_congr sk pk h, h]

/-- `asmLocalOk` reads the state only through `s.asm pk`. -/
theorem asmLocalOk_congr (sk : Skel) {s s' : State}
    (pk : Party × Nat) (h : s'.asm pk = s.asm pk) :
    asmLocalOk sk s' pk = asmLocalOk sk s pk := by
  simp only [asmLocalOk, h]

/-- Everything `topLocalOk` can read. -/
theorem topLocalOk_congr (sk : Skel) (ax : AxMode) {s s' : State}
    (h1 : s'.iopenWire = s.iopenWire) (h2 : s'.iopenQuery = s.iopenQuery)
    (h3 : s'.iopenCh = s.iopenCh) (h4 : s'.ropenGotWire = s.ropenGotWire)
    (h5 : s'.ropenWire = s.ropenWire) (h6 : s'.ropenRes = s.ropenRes)
    (h7 : s'.ropenQ = s.ropenQ) (h8 : s'.ropenCh = s.ropenCh)
    (h9 : s'.absorbIdx = s.absorbIdx) (h10 : s'.absorbPhase = s.absorbPhase)
    (h11 : s'.rfinGotRes = s.rfinGotRes) (h12 : s'.rfinGot = s.rfinGot) :
    topLocalOk sk ax s' = topLocalOk sk ax s := by
  simp only [topLocalOk, h1, h2, h3, h4, h5, h6, h7, h8, h9, h10, h11, h12]

end StreamingMirror.Model
