/-
Weave pump-progress, the chain layer (PROGRESS.md §7 3b, edge-respect
step (e) of the pump case-tree): the four pump-window discharges. At a
manual send into a pump-facing channel (`upper`/`lower`/leaf-wire/
`leafRequests`), the consumer's stuck trichotomy (`asm_stuck`,
`absorb_stuck`, `fin_stuck`, `rootret_stuck`) is refuted case by case:

- Starved on the very channel being sent: the producer's count IS the
  seq about to go out — pure arithmetic.
- Level-starved: DESCEND into the supplier tower (`tower_deliver`,
  recursion downward, bottoming at the absorber or at the pend-free
  height-1 asker). Each supplier's own res starvation is refuted by
  the descent supply package (`DescSupply`), out-blocking purely by
  the caller's starvation fact, exhaustion by the supply/demand
  totals.
- Out-blocked: ASCEND into the consumer stack (`tower_noblock`,
  recursion upward, topping at `rootret`/fins). Each consumer's res
  starvation is refuted by the ascent coverage package (`AscSupply`),
  level-starvation purely against the blocking fact, exhaustion by
  totals plus the count-versus-trace bound.

The supply packages are hypotheses here; establishing them at every
weave position is the CtxOK tree induction (step (f)). `Skel.
schedulable` and the §5 splice placement live INSIDE those packages'
eventual proofs, not in this file.
-/
import StreamingMirror.Proofs.Sched.Weave.Pump
import StreamingMirror.Proofs.Counting

namespace StreamingMirror.Sched

open Model

variable (sk : Skel)

-- ==================================================== small helpers

/-- Asking alternates with level parity. -/
theorem asks_succ (p : Party) (j : Nat) :
    asks p (j + 1) = !asks p j := by
  rcases Nat.mod_two_eq_zero_or_one j with hm | hm <;>
    cases p <;> simp [asks, Nat.add_mod, hm]

/-- `wellFormed` puts at least one slot in every level window. -/
theorem wf_capLevel {sk : Skel} (hwf : sk.wellFormed = true) :
    1 ≤ sk.capLevel := by
  unfold Skel.wellFormed at hwf
  simp only [Bool.and_eq_true, decide_eq_true_eq] at hwf
  exact hwf.1.2

/-- Every channel window has at least one slot. -/
theorem cap_pos {sk : Skel} (hwf : sk.wellFormed = true) (c : Chan) :
    1 ≤ sk.cap c := by
  unfold Skel.cap
  cases c <;> first
    | exact wf_capLevel hwf
    | exact Nat.le_refl 1

/-- Level windows carry the level capacity. -/
theorem cap_level (p : Party) (j : Nat) :
    sk.cap (Chan.level p j) = sk.capLevel := rfl

/-- An asker's resolution channel is the upper from one stage down. -/
theorem asmResChan_asker {p : Party} {j : Nat}
    (hasks : asks p j = true) :
    asmResChan (p, j) = Chan.upper p (j - 1) := by
  unfold asmResChan
  rw [if_pos hasks]

/-- An answerer's resolution channel is its own lower. -/
theorem asmResChan_answerer {p : Party} {j : Nat}
    (hna : asks p j = false) :
    asmResChan (p, j) = Chan.lower p j := by
  unfold asmResChan
  rw [if_neg (by simp [hna])]

/-- Away from the two top slots, a tower's output is its level. -/
theorem asmOutChan_level {p : Party} {j : Nat}
    (hI : ¬(p = Party.I ∧ j = sk.rootH))
    (hR : ¬(p = Party.R ∧ j = sk.rootH - 1)) :
    sk.asmOutChan (p, j) = Chan.level p j := by
  have hI' : ¬(((p, j).fst == Party.I
      && (p, j).snd == sk.rootH) = true) := by
    simp only [Bool.and_eq_true, beq_iff_eq]
    exact fun hc => hI ⟨hc.1, hc.2⟩
  have hR' : ¬(((p, j).fst == Party.R
      && (p, j).snd == sk.rootH - 1) = true) := by
    simp only [Bool.and_eq_true, beq_iff_eq]
    exact fun hc => hR ⟨hc.1, hc.2⟩
  unfold Skel.asmOutChan
  rw [if_neg hI', if_neg hR']

/-- Interior level channels are sent by their tower. -/
theorem sndOwner_level {p : Party} {j : Nat}
    (hnz : ¬(p = Party.I ∧ j = 0)) :
    sndOwner sk (Chan.level p j) = asmIdx sk p j := by
  simp only [sndOwner]
  rw [if_neg hnz]

/-- The tower slot read, steered by the party's top. -/
theorem asm_procs {p : Party} {top j : Nat}
    (htop : p = Party.I ∧ top = sk.rootH
      ∨ p = Party.R ∧ top = sk.rootH - 1)
    (h1 : 1 ≤ j) (hjt : j ≤ top) :
    (procs sk)[asmIdx sk p j]? = some (asmEvents sk (p, j)) := by
  rcases htop with ⟨hp, ht⟩ | ⟨hp, ht⟩ <;> subst hp
  · exact procs_asmI sk h1 (by omega)
  · exact procs_asmR sk h1 (by omega)

/-- A channel-side count never exceeds its owner's whole-trace total. -/
theorem count_le_owner (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCount sk fut st) (c : Chan) (b : Bool)
    {M : Nat} (hM : (if b then sndOwner sk c else rcvOwner sk c) = M)
    {T : List Ev} (hT : (procs sk)[M]? = some T) :
    (proj c b st.out).length ≤ (proj c b T).length := by
  obtain ⟨r, pre, hr, hpre, hsub⟩ := cell_of_owner sk h hT
  rw [out_proj_owner sk hwf h c b hM hT hr hpre hsub, hpre,
    proj_append, List.length_append]
  omega

/-- An interior tower's level output never exceeds its resolution
count: the count-versus-trace bound through the owner collapse. -/
theorem level_snd_le (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCount sk fut st) {p : Party} {top j : Nat}
    (htop : p = Party.I ∧ top = sk.rootH
      ∨ p = Party.R ∧ top = sk.rootH - 1)
    (h1 : 1 ≤ j) (hjt : j < top) :
    sndCount (Chan.level p j) st.out ≤ (sk.asmResList p j).length := by
  have hI : ¬(p = Party.I ∧ j = sk.rootH) := by
    rcases htop with ⟨rfl, ht⟩ | ⟨rfl, ht⟩
    · rintro ⟨-, hj⟩; omega
    · simp
  have hR : ¬(p = Party.R ∧ j = sk.rootH - 1) := by
    rcases htop with ⟨rfl, ht⟩ | ⟨rfl, ht⟩
    · simp
    · rintro ⟨-, hj⟩; omega
  have hout : sk.asmOutChan (p, j) = Chan.level p j :=
    asmOutChan_level sk hI hR
  have hnz : ¬(p = Party.I ∧ j = 0) := by rintro ⟨-, hj⟩; omega
  have hcount := count_le_owner sk hwf h (Chan.level p j) true
    (M := asmIdx sk p j) (by simpa using sndOwner_level sk hnz)
    (asm_procs sk htop h1 (by omega))
  rw [sndCount_eq_proj]
  calc (proj (Chan.level p j) true st.out).length
      ≤ (proj (Chan.level p j) true (asmEvents sk (p, j))).length :=
        hcount
    _ = (sk.asmResList p j).length := by
        rw [← hout, (asm_totals sk (p, j)).2.2, seg_len]

/-- The absorber's level-0 output never exceeds the leaf-request
total. -/
theorem level0_snd_le (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCount sk fut st) :
    sndCount (Chan.level Party.I 0) st.out ≤ sk.totalLeafReqs := by
  have hcount := count_le_owner sk hwf h (Chan.level Party.I 0) true
    (M := 2 + sk.rootH) (by simp [sndOwner]) (procs_absorb sk)
  rw [sndCount_eq_proj]
  calc (proj (Chan.level Party.I 0) true st.out).length
      ≤ (proj (Chan.level Party.I 0) true (absorbEvents sk)).length :=
        hcount
    _ = sk.totalLeafReqs := by rw [(absorb_totals sk).2.2, seg_len]

/-- Nobody sends the responder's phantom level-0 channel. -/
theorem levelR0_snd_zero (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCount sk fut st) :
    sndCount (Chan.level Party.R 0) st.out = 0 := by
  have hge := (wf_rootH hwf).2
  have hM : sndOwner sk (Chan.level Party.R 0)
      = asmIdx sk Party.R 1 :=
    sndOwner_level sk (by simp)
  have hT := procs_asmR sk (Nat.le_refl 1) (by omega)
  have hcount := count_le_owner sk hwf h (Chan.level Party.R 0) true
    (by simpa using hM) hT
  have hne : sk.asmOutChan (Party.R, 1) ≠ Chan.level Party.R 0 := by
    unfold Skel.asmOutChan
    split
    · simp
    · split
      · simp
      · simp
  have hnil : proj (Chan.level Party.R 0) true
      (asmEvents sk (Party.R, 1)) = [] := by
    unfold proj
    rw [List.filter_eq_nil_iff]
    intro e he
    simp only [Bool.and_eq_true, decide_eq_true_eq, beq_iff_eq,
      not_and]
    intro hc hb
    exact hne
      (((asmEvents_support sk (Party.R, 1) e he).1 hb).symm.trans hc)
  rw [hnil] at hcount
  simp only [List.length_nil, Nat.le_zero] at hcount
  rw [sndCount_eq_proj]
  exact hcount

/-- A tower's whole-sweep pending total is its supplier's resolution
count: consumer level demand = producer output supply, at every
interior boundary. -/
theorem pends_total_prod {sk : Skel} (hwf : sk.wellFormed = true)
    {p : Party} {j : Nat} (h2 : 2 ≤ j) (hjr : j - 1 < sk.rootH) :
    sk.pendsBefore p j (sk.asmResList p j).length
      = (sk.asmResList p (j - 1)).length := by
  have hflip : asks p j = !asks p (j - 1) := by
    have hs := asks_succ p (j - 1)
    rw [show j - 1 + 1 = j from by omega] at hs
    exact hs
  cases hask : asks p j with
  | true =>
      have hna : asks p (j - 1) = false := by
        rw [hask] at hflip
        simpa using hflip.symm
      have h0 := pendsBefore_asker_full hwf (p := p) (j := j - 1)
        (by rw [show j - 1 + 1 = j from by omega]; exact hask)
        (by omega)
      rw [show j - 1 + 1 = j from by omega] at h0
      rw [h0, asmResList_answerer_length hna]
  | false =>
      have hasker : asks p (j - 1) = true := by
        rw [hask] at hflip
        simpa using hflip.symm
      have h0 := pendsBefore_answerer_full hwf (p := p) (j := j - 2)
        (by rw [show j - 2 + 2 = j from by omega]; exact hask)
        (by omega)
      rw [show j - 2 + 2 = j from by omega] at h0
      rw [h0, asmResList_asker_length hasker]
      unfold Skel.stageLen Skel.stageScopes
      rw [show j - 2 + 1 = j - 1 from by omega]

-- ================================================== supply packages
-- The position facts the chains consume, packaged per tower. The
-- CtxOK tree induction (step (f)) establishes these at every weave
-- position; here they are hypotheses.

/-- Descent supplies below a tower: each supplier's resolutions are
present through the demand the level above hands down, bottoming at
the absorber's wire and request feeds. -/
def DescSupply (st : MState) (p : Party) : Nat → Nat → Prop
  | 0, c => p = Party.I →
      c ≤ sndCount (Chan.wire Party.R 0) st.out
        ∧ c ≤ sndCount Chan.leafRequests st.out
  | j + 1, c =>
      c ≤ sndCount (asmResChan (p, j + 1)) st.out
        ∧ DescSupply st p j (sk.pendsBefore p (j + 1) c)

/-- Descent supplies weaken with the demand. -/
theorem descSupply_mono {st : MState} {p : Party} :
    ∀ {j c c'}, c' ≤ c → DescSupply sk st p j c →
      DescSupply sk st p j c'
  | 0, _, _, hle, hsup => fun hp =>
      ⟨Nat.le_trans hle (hsup hp).1, Nat.le_trans hle (hsup hp).2⟩
  | j + 1, _, _, hle, hsup =>
      ⟨Nat.le_trans hle hsup.1,
        descSupply_mono (pendsBefore_mono sk p (j + 1) hle) hsup.2⟩

/-- Ascent coverage above a channel: every tower from `j` to the top
has resolutions present whose pending allocation covers everything its
level-in channel has so far carried. -/
def AscSupply (st : MState) (p : Party) (j top : Nat) : Prop :=
  ∀ j', j ≤ j' → j' ≤ top →
    ∃ r, r ≤ sndCount (asmResChan (p, j')) st.out
      ∧ sndCount (asmLevelChan (p, j')) st.out
          ≤ sk.pendsBefore p j' r

-- ======================================================== the chains

/-- ABSORBER DELIVERY: at a pump fixpoint a drained absorber with its
wire and request feeds present through `c` has produced `c` level-0
returns. -/
theorem absorb_deliver (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCount sk fut st) (hfix : step sk st = none)
    {c : Nat} (hcN : c ≤ sk.totalLeafReqs)
    (hw : c ≤ sndCount (Chan.wire Party.R 0) st.out)
    (hq : c ≤ sndCount Chan.leafRequests st.out)
    (hdrain : sndCount (Chan.level Party.I 0) st.out
      ≤ rcvCount (Chan.level Party.I 0) st.out) :
    c ≤ sndCount (Chan.level Party.I 0) st.out := by
  have hcap := wf_capLevel hwf
  rcases absorb_stuck sk hwf h hfix with
    ⟨hW, hL, hV⟩ | ⟨hWt, hLW, hVW, hsw⟩ | ⟨hLt, hWL, hVL, hsq⟩
    | ⟨hVt, hWV, hLV, hblk⟩
  · omega
  · omega
  · omega
  · rw [cap_level] at hblk
    omega

/-- TOP BLOCKING IS ABSURD: the two tower tops (`rootret`, the fins'
`rootrets`) can never be the blocked window — their consumers drain
everything the root resolution allows. -/
theorem top_blocked (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCount sk fut st) (hfix : step sk st = none)
    {p : Party} {top : Nat}
    (htop : p = Party.I ∧ top = sk.rootH
      ∨ p = Party.R ∧ top = sk.rootH - 1)
    (hroot : 1 ≤ sndCount Chan.rootres st.out)
    (hblk : rcvCount (sk.asmOutChan (p, top)) st.out
        + sk.cap (sk.asmOutChan (p, top))
      ≤ sndCount (sk.asmOutChan (p, top)) st.out) : False := by
  have hge2 := (wf_rootH hwf).2
  have hev := (wf_rootH hwf).1
  rcases htop with ⟨rfl, rfl⟩ | ⟨rfl, rfl⟩
  · -- initiator top: the floating rootret receive
    have hout : sk.asmOutChan (Party.I, sk.rootH) = Chan.rootret := by
      unfold Skel.asmOutChan
      rw [if_pos (by simp)]
    rw [hout] at hblk
    have hasks : asks Party.I sk.rootH = true := by
      simp [asks, hev]
    have hsndle : sndCount Chan.rootret st.out ≤ 1 := by
      have hT := procs_asmI sk (by omega) (Nat.le_refl _)
      have hcount := count_le_owner sk hwf h Chan.rootret true
        (M := asmIdx sk Party.I sk.rootH) (by rfl) hT
      have htot := (asm_totals sk (Party.I, sk.rootH)).2.2
      rw [hout] at htot
      rw [sndCount_eq_proj]
      calc (proj Chan.rootret true st.out).length
          ≤ (proj Chan.rootret true
              (asmEvents sk (Party.I, sk.rootH))).length := hcount
        _ = (sk.asmResList Party.I sk.rootH).length := by
            rw [htot, seg_len]
        _ = 1 := by
            rw [asmResList_asker_length hasks, wf_root_stage hwf]
            rfl
    have hcapr : sk.cap Chan.rootret = 1 := rfl
    rw [hcapr] at hblk
    rcases rootret_stuck sk hwf h hfix (by omega) with h1 | ⟨h0, hs0⟩
    · omega
    · omega
  · -- responder top: the fins' root returns
    have hout : sk.asmOutChan (Party.R, sk.rootH - 1)
        = Chan.rootrets := by
      unfold Skel.asmOutChan
      rw [if_neg (by simp), if_pos (by simp)]
    rw [hout] at hblk
    have hasks : asks Party.R (sk.rootH - 1) = true := by
      have hodd : (sk.rootH - 1) % 2 = 1 := by omega
      simp [asks, hodd]
    have hpend : (sk.scopesAt (sk.rootH - 1)).length
        = sk.rootPending := by
      have halign := wf_bfs_aligned hwf
        (h := sk.rootH - 1) (by omega)
      rw [show sk.rootH - 1 + 1 = sk.rootH from by omega,
        wf_root_stage hwf] at halign
      have hlen := congrArg List.length halign
      simp only [List.flatMap_cons, List.flatMap_nil,
        List.append_nil] at hlen
      unfold Skel.rootPending
      omega
    have hsndle : sndCount Chan.rootrets st.out ≤ sk.rootPending := by
      have hT := procs_asmR sk (by omega) (Nat.le_refl _)
      have hcount := count_le_owner sk hwf h Chan.rootrets true
        (M := asmIdx sk Party.R (sk.rootH - 1)) (by rfl) hT
      have htot := (asm_totals sk (Party.R, sk.rootH - 1)).2.2
      rw [hout] at htot
      rw [sndCount_eq_proj]
      calc (proj Chan.rootrets true st.out).length
          ≤ (proj Chan.rootrets true
              (asmEvents sk (Party.R, sk.rootH - 1))).length := hcount
        _ = (sk.asmResList Party.R (sk.rootH - 1)).length := by
            rw [htot, seg_len]
        _ = sk.rootPending := by
            rw [asmResList_asker_length hasks, hpend]
    have hcapr : sk.cap Chan.rootrets = 1 := rfl
    rw [hcapr] at hblk
    rcases fin_stuck sk hwf h hfix (by omega) with
      ⟨ha, hb⟩ | ⟨ha, hb, hc⟩ | ⟨ha, hb, hc⟩
    · omega
    · omega
    · omega

/-- Below its party's top, a tower's output is its level channel. -/
theorem asmOutChan_of_lt {p : Party} {top j : Nat}
    (htop : p = Party.I ∧ top = sk.rootH
      ∨ p = Party.R ∧ top = sk.rootH - 1)
    (hjt : j < top) :
    sk.asmOutChan (p, j) = Chan.level p j := by
  refine asmOutChan_level sk ?_ ?_
  · rintro ⟨hpI, hjr⟩
    rcases htop with ⟨-, ht⟩ | ⟨hpR, -⟩
    · omega
    · rw [hpI] at hpR
      cases hpR
  · rintro ⟨hpR, hjr⟩
    rcases htop with ⟨hpI, -⟩ | ⟨-, ht⟩
    · rw [hpR] at hpI
      cases hpI
    · omega

/-- ASCENT: a blocked level window is absurd — every consumer above,
covered by the ascent package, drains what the window carries, all the
way to the root returns. -/
theorem tower_noblock (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCount sk fut st) (hfix : step sk st = none)
    {p : Party} {top : Nat}
    (htop : p = Party.I ∧ top = sk.rootH
      ∨ p = Party.R ∧ top = sk.rootH - 1)
    (hroot : 1 ≤ sndCount Chan.rootres st.out)
    (j : Nat) (h1 : 1 ≤ j) (hjt : j ≤ top)
    (hsup : AscSupply sk st p j top)
    (hblk : rcvCount (asmLevelChan (p, j)) st.out + sk.capLevel
      ≤ sndCount (asmLevelChan (p, j)) st.out) : False := by
  have hcap := wf_capLevel hwf
  have hge2 := (wf_rootH hwf).2
  -- the phantom base: nobody sends the responder's level 0
  by_cases hR1 : p = Party.R ∧ j = 1
  · obtain ⟨rfl, rfl⟩ := hR1
    have hz : sndCount (asmLevelChan (Party.R, 1)) st.out = 0 :=
      levelR0_snd_zero sk hwf h
    omega
  have hIdx := asm_procs sk htop h1 hjt
  rcases asm_stuck sk hwf h hfix h1 hIdx with
    ⟨hRe, hLe, hOe⟩ | ⟨hRl, hLp, hOp, hres⟩
    | ⟨hRl, hR1', hLlo, hLhi, hOp, hlv⟩ | ⟨hRl, hR1', hLp, hOp, hoblk⟩
  · -- exhausted: demand total = supply total bounds the window shut
    by_cases hj1 : j = 1
    · subst hj1
      have hpI : p = Party.I := by
        cases p
        · rfl
        · exact absurd ⟨rfl, rfl⟩ hR1
      subst hpI
      have hS : sndCount (asmLevelChan (Party.I, 1)) st.out
          ≤ sk.totalLeafReqs := level0_snd_le sk hwf h
      have htot : sk.pendsBefore Party.I 1
          (sk.asmResList Party.I 1).length = sk.totalLeafReqs :=
        pendsBefore_answerer_leaf (hna := rfl)
      omega
    · have hS : sndCount (asmLevelChan (p, j)) st.out
          ≤ (sk.asmResList p (j - 1)).length :=
        level_snd_le sk hwf h htop (by omega) (by omega)
      have htot := pends_total_prod hwf (p := p) (j := j)
        (by omega)
        (by rcases htop with ⟨-, ht⟩ | ⟨-, ht⟩ <;> omega)
      omega
  · -- res-starved: the coverage package reaches past the window
    obtain ⟨r, hr1, hr2⟩ := hsup j (Nat.le_refl j) hjt
    have hmono := pendsBefore_mono sk p j
      (show r ≤ rcvCount (asmResChan (p, j)) st.out from
        Nat.le_trans hr1 hres)
    omega
  · -- level-starved against blocked: the window is both shut and dry
    omega
  · -- out-blocked: ascend
    by_cases hjtop : j = top
    · subst hjtop
      exact top_blocked sk hwf h hfix htop hroot hoblk
    · have hjlt : j < top := Nat.lt_of_le_of_ne hjt hjtop
      have hout := asmOutChan_of_lt sk htop hjlt
      rw [hout, cap_level] at hoblk
      exact tower_noblock hwf h hfix htop hroot (j + 1) (by omega)
        (by omega) (fun j' hj1' hj2' => hsup j' (by omega) hj2')
        hoblk
termination_by top - j

/-- DESCENT: at a pump fixpoint a drained interior tower with descent
supplies through demand `c` has produced `c` outputs. -/
theorem tower_deliver (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCount sk fut st) (hfix : step sk st = none)
    {p : Party} {top : Nat}
    (htop : p = Party.I ∧ top = sk.rootH
      ∨ p = Party.R ∧ top = sk.rootH - 1)
    (j c : Nat) (h1 : 1 ≤ j) (hjt : j < top)
    (hcN : c ≤ (sk.asmResList p j).length)
    (hsup : DescSupply sk st p j c)
    (hdrain : sndCount (sk.asmOutChan (p, j)) st.out
      ≤ rcvCount (sk.asmOutChan (p, j)) st.out) :
    c ≤ sndCount (sk.asmOutChan (p, j)) st.out := by
  have hcap := wf_capLevel hwf
  have hge2 := (wf_rootH hwf).2
  have hIdx := asm_procs sk htop h1 (by omega)
  obtain ⟨j', rfl⟩ : ∃ j', j = j' + 1 := ⟨j - 1, by omega⟩
  have hsup1 : c ≤ sndCount (asmResChan (p, j' + 1)) st.out := hsup.1
  have hsup2 : DescSupply sk st p j'
      (sk.pendsBefore p (j' + 1) c) := hsup.2
  rcases asm_stuck sk hwf h hfix h1 hIdx with
    ⟨hRe, hLe, hOe⟩ | ⟨hRl, hLp, hOp, hres⟩
    | ⟨hRl, hR1', hLlo, hLhi, hOp, hlv⟩ | ⟨hRl, hR1', hLp, hOp, hoblk⟩
  · -- exhausted: the whole demand was met
    omega
  · -- res-starved: the descent supply feeds the next resolution
    omega
  · -- level-starved: the supplier below must deliver — recurse
    by_cases hco : c ≤ sndCount (sk.asmOutChan (p, j' + 1)) st.out
    · exact hco
    have hRc : rcvCount (asmResChan (p, j' + 1)) st.out ≤ c := by
      omega
    have hmono := pendsBefore_mono sk p (j' + 1) hRc
    rcases Nat.eq_zero_or_pos j' with rfl | hj'pos
    · simp only [Nat.zero_add] at *
      cases p with
      | R =>
          -- the height-1 asker pends nothing: starvation is absurd
          have hz := pendsBefore_asker_one hwf
            (p := Party.R) (hasks := rfl)
            (rcvCount (asmResChan (Party.R, 1)) st.out)
          omega
      | I =>
          -- the absorber delivers
          have hlc : asmLevelChan (Party.I, 1)
              = Chan.level Party.I 0 := rfl
          rw [hlc] at hLlo hLhi hlv
          have hc₀N : sk.pendsBefore Party.I 1
              (rcvCount (asmResChan (Party.I, 1)) st.out)
              ≤ sk.totalLeafReqs := by
            have htot : sk.pendsBefore Party.I 1
                (sk.asmResList Party.I 1).length
                = sk.totalLeafReqs :=
              pendsBefore_answerer_leaf (hna := rfl)
            have := pendsBefore_mono sk Party.I 1 hRl
            omega
          have hpair := hsup2 rfl
          have hdel := absorb_deliver sk hwf h hfix hc₀N
            (Nat.le_trans hmono hpair.1)
            (Nat.le_trans hmono hpair.2) hlv
          omega
    · obtain ⟨j'', rfl⟩ : ∃ j'', j' = j'' + 1 :=
        ⟨j' - 1, by omega⟩
      have hout' := asmOutChan_of_lt sk htop
        (show j'' + 1 < top from by omega)
      have hlc : asmLevelChan (p, j'' + 1 + 1)
          = Chan.level p (j'' + 1) := rfl
      rw [hlc] at hLlo hLhi hlv
      have hcN' : sk.pendsBefore p (j'' + 1 + 1)
          (rcvCount (asmResChan (p, j'' + 1 + 1)) st.out)
          ≤ (sk.asmResList p (j'' + 1)).length := by
        have htot : sk.pendsBefore p (j'' + 1 + 1)
            (sk.asmResList p (j'' + 1 + 1)).length
            = (sk.asmResList p (j'' + 1)).length :=
          pends_total_prod hwf (p := p)
            (j := j'' + 1 + 1) (by omega)
            (by rcases htop with ⟨-, ht⟩ | ⟨-, ht⟩ <;> omega)
        have := pendsBefore_mono sk p (j'' + 1 + 1) hRl
        omega
      have hdrain' : sndCount (sk.asmOutChan (p, j'' + 1)) st.out
          ≤ rcvCount (sk.asmOutChan (p, j'' + 1)) st.out := by
        rw [hout']
        exact hlv
      have hdel := tower_deliver hwf h hfix htop (j'' + 1)
        (sk.pendsBefore p (j'' + 1 + 1)
          (rcvCount (asmResChan (p, j'' + 1 + 1)) st.out))
        (by omega) (by omega) hcN'
        (descSupply_mono sk hmono hsup2) hdrain'
      rw [hout'] at hdel
      omega
  · -- out-blocked against drained: the window has slack
    have hpos := cap_pos hwf (sk.asmOutChan (p, j' + 1))
    omega
termination_by j

-- ==================================== generic tower-state invariants
-- Free consequences of the cell shapes at ANY weave state: the pieces
-- of the ascent coverage that need no position facts. (The residue —
-- the strict coverage at a blocked boundary — is where the §5 splice
-- and `Skel.schedulable` bite, in the CtxOK layer.)

/-- A tower's output never outruns its resolutions. -/
theorem asm_out_le_res (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCount sk fut st) {p : Party} {top j : Nat}
    (htop : p = Party.I ∧ top = sk.rootH
      ∨ p = Party.R ∧ top = sk.rootH - 1)
    (h1 : 1 ≤ j) (hjt : j ≤ top) :
    sndCount (sk.asmOutChan (p, j)) st.out
      ≤ rcvCount (asmResChan (p, j)) st.out := by
  have hIdx := asm_procs sk htop h1 hjt
  obtain ⟨r, pre, hr, hpre, hsub⟩ := cell_of_owner sk h hIdx
  obtain ⟨hro, hlo, hoo⟩ := asm_owners sk p h1
  have hRc : rcvCount (asmResChan (p, j)) st.out
      = (proj (asmResChan (p, j)) false pre).length := by
    rw [rcvCount_eq_proj,
      out_proj_owner sk hwf h _ false (by simpa using hro)
        hIdx hr hpre hsub]
  have hOc : sndCount (sk.asmOutChan (p, j)) st.out
      = (proj (sk.asmOutChan (p, j)) true pre).length := by
    rw [sndCount_eq_proj,
      out_proj_owner sk hwf h _ true (by simpa using hoo)
        hIdx hr hpre hsub]
  cases r with
  | nil =>
      rw [List.append_nil] at hpre
      obtain ⟨ht1, -, ht3⟩ := asm_totals sk (p, j)
      rw [hpre] at ht1 ht3
      rw [hRc, hOc, ht1, ht3, seg_len, seg_len]
      exact Nat.le_refl _
  | cons e₀ rest₀ =>
      obtain ⟨idx, hidxN, hshape⟩ :=
        asm_cell_shape sk (p, j) hpre (by simp)
      rcases hshape with ⟨-, hc1, -, hc3⟩
        | ⟨tlv, rest, -, -, -, hc1, -, hc3⟩ | ⟨-, hc1, -, hc3⟩ <;>
        omega

/-- A tower's level intake never outruns its resolutions' pending
allocation. -/
theorem asm_lvl_le_pends (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCount sk fut st) {p : Party} {top j : Nat}
    (htop : p = Party.I ∧ top = sk.rootH
      ∨ p = Party.R ∧ top = sk.rootH - 1)
    (h1 : 1 ≤ j) (hjt : j ≤ top) :
    rcvCount (asmLevelChan (p, j)) st.out
      ≤ sk.pendsBefore p j (rcvCount (asmResChan (p, j)) st.out) := by
  have hIdx := asm_procs sk htop h1 hjt
  obtain ⟨r, pre, hr, hpre, hsub⟩ := cell_of_owner sk h hIdx
  obtain ⟨hro, hlo, hoo⟩ := asm_owners sk p h1
  have hRc : rcvCount (asmResChan (p, j)) st.out
      = (proj (asmResChan (p, j)) false pre).length := by
    rw [rcvCount_eq_proj,
      out_proj_owner sk hwf h _ false (by simpa using hro)
        hIdx hr hpre hsub]
  have hLc : rcvCount (asmLevelChan (p, j)) st.out
      = (proj (asmLevelChan (p, j)) false pre).length := by
    rw [rcvCount_eq_proj,
      out_proj_owner sk hwf h _ false (by simpa using hlo)
        hIdx hr hpre hsub]
  cases r with
  | nil =>
      rw [List.append_nil] at hpre
      obtain ⟨ht1, ht2, -⟩ := asm_totals sk (p, j)
      rw [hpre] at ht1 ht2
      rw [hRc, hLc, ht1, ht2, seg_len, seg_len]
      exact Nat.le_refl _
  | cons e₀ rest₀ =>
      obtain ⟨idx, hidxN, hshape⟩ :=
        asm_cell_shape sk (p, j) hpre (by simp)
      rcases hshape with ⟨-, hc1, hc2, -⟩
        | ⟨tlv, rest, -, htl, hth, hc1, hc2, -⟩ | ⟨-, hc1, hc2, -⟩
      · have hc2' : (proj (asmLevelChan (p, j)) false pre).length
            = sk.pendsBefore p j idx := hc2
        rw [hLc, hRc, hc1, hc2']
        exact Nat.le_refl _
      · have hth' : tlv < sk.pendsBefore p j (idx + 1) := hth
        rw [hLc, hRc, hc1, hc2]
        omega
      · have hc2' : (proj (asmLevelChan (p, j)) false pre).length
            = sk.pendsBefore p j (idx + 1) := hc2
        rw [hLc, hRc, hc1, hc2']
        exact Nat.le_refl _

/-- A send never outruns its window: the emitted stream respected E2
at every position, so the count is within `cap` of the receipts. -/
theorem wedge_snd_le_rcv_cap (hwf : sk.wellFormed = true)
    {fut : List Ev} {st : MState} (h : WEdge sk fut st) (c : Chan) :
    sndCount c st.out ≤ rcvCount c st.out + sk.cap c := by
  cases hz : sndCount c st.out with
  | zero => omega
  | succ q =>
      have hcanon := wproj_canon sk hwf h.toWCount c true
      have hmem : ((c, true, q) : Ev) ∈ proj c true st.out := by
        rw [hcanon]
        have hlen : (proj c true st.out).length = q + 1 := by
          rw [← sndCount_eq_proj, hz]
        rw [hlen]
        unfold canon
        exact List.mem_map.2 ⟨q, List.mem_range.2 (by omega), rfl⟩
      have hmem' : ((c, true, q) : Ev) ∈ st.out :=
        (List.mem_filter.1 hmem).1
      obtain ⟨k, hk⟩ := List.mem_iff_getElem?.1 hmem'
      have hguard := h.e2_hist k c q hk
      have htake : rcvCount c (st.out.take k) ≤ rcvCount c st.out := by
        rw [rcvCount_eq_proj, rcvCount_eq_proj]
        exact ((List.take_sublist k st.out).filter _).length_le
      omega

-- ================================================ the four windows

/-- THE UPPER WINDOW: at a pump fixpoint the asker above has consumed
every resolution before the one the walk is about to send. -/
theorem upper_window (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WEdge sk fut st) (hfix : step sk st = none)
    {p : Party} {top hh k : Nat}
    (htop : p = Party.I ∧ top = sk.rootH
      ∨ p = Party.R ∧ top = sk.rootH - 1)
    (hasks : asks p (hh + 1) = true)
    (hht : hh + 1 ≤ top)
    (hk : k < sk.stageLen hh)
    (hsnd : sndCount (Chan.upper p hh) st.out = k)
    (hdesc : DescSupply sk st p hh (sk.pendsBefore p (hh + 1) k))
    (hasc : AscSupply sk st p (hh + 2) top)
    (hroot : 1 ≤ sndCount Chan.rootres st.out) :
    k ≤ rcvCount (Chan.upper p hh) st.out := by
  have hcap := wf_capLevel hwf
  have hge2 := (wf_rootH hwf).2
  have hres : asmResChan (p, hh + 1) = Chan.upper p hh :=
    asmResChan_asker hasks
  have hIdx := asm_procs sk htop (by omega) hht
  have hRk : rcvCount (Chan.upper p hh) st.out ≤ k := by
    have := wedge_rcvd_le_sent sk hwf h (Chan.upper p hh)
    omega
  have hstuck := asm_stuck sk hwf h.toWCount hfix
    (show 1 ≤ hh + 1 by omega) hIdx
  rw [hres, show asmLevelChan (p, hh + 1) = Chan.level p hh from rfl]
    at hstuck
  rcases hstuck with
    ⟨hRe, hLe, hOe⟩ | ⟨hRl, hLp, hOp, hres'⟩
    | ⟨hRl, hR1', hLlo, hLhi, hOp, hlv⟩ | ⟨hRl, hR1', hLp, hOp, hoblk⟩
  · -- exhausted: everything is consumed
    have hN : (sk.asmResList p (hh + 1)).length = sk.stageLen hh := by
      rw [asmResList_asker_length hasks]
      rfl
    omega
  · -- starved on this very channel: the seq about to go out IS the
    -- send count
    omega
  · -- level-starved: the supplier below delivers
    exfalso
    rcases Nat.eq_zero_or_pos hh with rfl | hhpos
    · have hz := pendsBefore_asker_one hwf (p := p)
        (by exact hasks) (rcvCount (Chan.upper p 0) st.out)
      simp only [Nat.zero_add] at *
      omega
    · obtain ⟨hh', rfl⟩ : ∃ hh', hh = hh' + 1 := ⟨hh - 1, by omega⟩
      have hmono := pendsBefore_mono sk p (hh' + 1 + 1) hRk
      have hout' := asmOutChan_of_lt sk htop
        (show hh' + 1 < top from by omega)
      have hcN' : sk.pendsBefore p (hh' + 1 + 1)
          (rcvCount (Chan.upper p (hh' + 1)) st.out)
          ≤ (sk.asmResList p (hh' + 1)).length := by
        have htot : sk.pendsBefore p (hh' + 1 + 1)
            (sk.asmResList p (hh' + 1 + 1)).length
            = (sk.asmResList p (hh' + 1)).length :=
          pends_total_prod hwf (p := p) (j := hh' + 1 + 1)
            (by omega)
            (by rcases htop with ⟨-, ht⟩ | ⟨-, ht⟩ <;> omega)
        have := pendsBefore_mono sk p (hh' + 1 + 1) hRl
        omega
      have hdrain' : sndCount (sk.asmOutChan (p, hh' + 1)) st.out
          ≤ rcvCount (sk.asmOutChan (p, hh' + 1)) st.out := by
        rw [hout']
        exact hlv
      have hdel := tower_deliver sk hwf h.toWCount hfix htop
        (hh' + 1)
        (sk.pendsBefore p (hh' + 1 + 1)
          (rcvCount (Chan.upper p (hh' + 1)) st.out))
        (by omega) (by omega) hcN'
        (descSupply_mono sk hmono hdesc) hdrain'
      rw [hout'] at hdel
      omega
  · -- out-blocked: the ascent refutes it
    exfalso
    by_cases htopc : hh + 1 = top
    · rw [htopc] at hoblk
      exact top_blocked sk hwf h.toWCount hfix htop hroot hoblk
    · have hout' := asmOutChan_of_lt sk htop
        (show hh + 1 < top from by omega)
      rw [hout', cap_level] at hoblk
      exact tower_noblock sk hwf h.toWCount hfix htop hroot
        (hh + 2) (by omega) (by omega)
        (fun j' hj1' hj2' => hasc j' (by omega) hj2') hoblk

/-- THE LOWER WINDOW: at a pump fixpoint the answerer has consumed
every resolution before the one the walk is about to send. -/
theorem lower_window (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WEdge sk fut st) (hfix : step sk st = none)
    {p : Party} {top hh d : Nat}
    (htop : p = Party.I ∧ top = sk.rootH
      ∨ p = Party.R ∧ top = sk.rootH - 1)
    (hna : asks p hh = false)
    (h1 : 1 ≤ hh) (hht : hh < top)
    (hd : d < (sk.asmResList p hh).length)
    (hsnd : sndCount (Chan.lower p hh) st.out = d)
    (hdesc : DescSupply sk st p (hh - 1) (sk.pendsBefore p hh d))
    (hasc : AscSupply sk st p (hh + 1) top)
    (hroot : 1 ≤ sndCount Chan.rootres st.out) :
    d ≤ rcvCount (Chan.lower p hh) st.out := by
  have hcap := wf_capLevel hwf
  have hge2 := (wf_rootH hwf).2
  obtain ⟨hh', rfl⟩ : ∃ hh', hh = hh' + 1 := ⟨hh - 1, by omega⟩
  have hdesc' : DescSupply sk st p hh'
      (sk.pendsBefore p (hh' + 1) d) := hdesc
  have hres : asmResChan (p, hh' + 1) = Chan.lower p (hh' + 1) :=
    asmResChan_answerer hna
  have hIdx := asm_procs sk htop (by omega) (by omega)
  have hRd : rcvCount (Chan.lower p (hh' + 1)) st.out ≤ d := by
    have := wedge_rcvd_le_sent sk hwf h (Chan.lower p (hh' + 1))
    omega
  have hstuck := asm_stuck sk hwf h.toWCount hfix
    (show 1 ≤ hh' + 1 by omega) hIdx
  rw [hres,
    show asmLevelChan (p, hh' + 1) = Chan.level p hh' from rfl]
    at hstuck
  rcases hstuck with
    ⟨hRe, hLe, hOe⟩ | ⟨hRl, hLp, hOp, hres'⟩
    | ⟨hRl, hR1', hLlo, hLhi, hOp, hlv⟩ | ⟨hRl, hR1', hLp, hOp, hoblk⟩
  · -- exhausted
    omega
  · -- starved on this very channel
    omega
  · -- level-starved: the supplier below delivers
    exfalso
    have hmono := pendsBefore_mono sk p (hh' + 1) hRd
    rcases Nat.eq_zero_or_pos hh' with rfl | hh'pos
    · -- the absorber delivers
      simp only [Nat.zero_add] at *
      have hpI : p = Party.I := by
        cases p with
        | I => rfl
        | R => rw [show asks Party.R 1 = true from rfl] at hna
               cases hna
      subst hpI
      have hc₀N : sk.pendsBefore Party.I 1
          (rcvCount (Chan.lower Party.I 1) st.out)
          ≤ sk.totalLeafReqs := by
        have htot : sk.pendsBefore Party.I 1
            (sk.asmResList Party.I 1).length
            = sk.totalLeafReqs :=
          pendsBefore_answerer_leaf (hna := rfl)
        have := pendsBefore_mono sk Party.I 1 hRl
        omega
      have hpair := hdesc' rfl
      have hdel := absorb_deliver sk hwf h.toWCount hfix hc₀N
        (Nat.le_trans hmono hpair.1)
        (Nat.le_trans hmono hpair.2) hlv
      omega
    · obtain ⟨hh'', rfl⟩ : ∃ hh'', hh' = hh'' + 1 :=
        ⟨hh' - 1, by omega⟩
      have hout' := asmOutChan_of_lt sk htop
        (show hh'' + 1 < top from by omega)
      have hcN' : sk.pendsBefore p (hh'' + 1 + 1)
          (rcvCount (Chan.lower p (hh'' + 1 + 1)) st.out)
          ≤ (sk.asmResList p (hh'' + 1)).length := by
        have htot : sk.pendsBefore p (hh'' + 1 + 1)
            (sk.asmResList p (hh'' + 1 + 1)).length
            = (sk.asmResList p (hh'' + 1)).length :=
          pends_total_prod hwf (p := p) (j := hh'' + 1 + 1)
            (by omega)
            (by rcases htop with ⟨-, ht⟩ | ⟨-, ht⟩ <;> omega)
        have := pendsBefore_mono sk p (hh'' + 1 + 1) hRl
        omega
      have hdrain' : sndCount (sk.asmOutChan (p, hh'' + 1)) st.out
          ≤ rcvCount (sk.asmOutChan (p, hh'' + 1)) st.out := by
        rw [hout']
        exact hlv
      have hdel := tower_deliver sk hwf h.toWCount hfix htop
        (hh'' + 1)
        (sk.pendsBefore p (hh'' + 1 + 1)
          (rcvCount (Chan.lower p (hh'' + 1 + 1)) st.out))
        (by omega) (by omega) hcN'
        (descSupply_mono sk hmono hdesc') hdrain'
      rw [hout'] at hdel
      omega
  · -- out-blocked: the ascent refutes it
    exfalso
    have hout' := asmOutChan_of_lt sk htop hht
    rw [hout', cap_level] at hoblk
    exact tower_noblock sk hwf h.toWCount hfix htop hroot
      (hh' + 1 + 1) (by omega) (by omega)
      (fun j' hj1' hj2' => hasc j' (by omega) hj2') hoblk

/-- THE LEAF-WIRE WINDOW: at a pump fixpoint the absorber has consumed
every leaf wire before the one the walk is about to send. -/
theorem wire0_window (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCount sk fut st) (hfix : step sk st = none)
    {w : Nat} (hw : w < sk.totalLeafReqs)
    (hsnd : sndCount (Chan.wire Party.R 0) st.out = w)
    (hreq : w ≤ sndCount Chan.leafRequests st.out + 1)
    (hasc : AscSupply sk st Party.I 1 sk.rootH)
    (hroot : 1 ≤ sndCount Chan.rootres st.out) :
    w ≤ rcvCount (Chan.wire Party.R 0) st.out := by
  have hcap := wf_capLevel hwf
  rcases absorb_stuck sk hwf h hfix with
    ⟨hW, hL, hV⟩ | ⟨hWt, hLW, hVW, hsw⟩ | ⟨hLt, hWL, hVL, hsq⟩
    | ⟨hVt, hWV, hLV, hblk⟩
  · omega
  · omega
  · omega
  · exfalso
    rw [cap_level] at hblk
    exact tower_noblock sk hwf h hfix (Or.inl ⟨rfl, rfl⟩) hroot
      1 (Nat.le_refl 1) (by have := (wf_rootH hwf).2; omega)
      hasc hblk

/-- THE LEAF-REQUEST WINDOW: at a pump fixpoint the absorber has
consumed every leaf request before the one the walk is about to
send. -/
theorem leafreq_window (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCount sk fut st) (hfix : step sk st = none)
    {q : Nat} (hq : q < sk.totalLeafReqs)
    (hsnd : sndCount Chan.leafRequests st.out = q)
    (hwire : q ≤ sndCount (Chan.wire Party.R 0) st.out)
    (hasc : AscSupply sk st Party.I 1 sk.rootH)
    (hroot : 1 ≤ sndCount Chan.rootres st.out) :
    q ≤ rcvCount Chan.leafRequests st.out := by
  have hcap := wf_capLevel hwf
  rcases absorb_stuck sk hwf h hfix with
    ⟨hW, hL, hV⟩ | ⟨hWt, hLW, hVW, hsw⟩ | ⟨hLt, hWL, hVL, hsq⟩
    | ⟨hVt, hWV, hLV, hblk⟩
  · omega
  · omega
  · omega
  · exfalso
    rw [cap_level] at hblk
    exact tower_noblock sk hwf h hfix (Or.inl ⟨rfl, rfl⟩) hroot
      1 (Nat.le_refl 1) (by have := (wf_rootH hwf).2; omega)
      hasc hblk

end StreamingMirror.Sched
