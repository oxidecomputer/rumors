/-
The O pump layer, chain tier: twins over `FamOKO` of the base chain
and window layer — the count-versus-trace bounds, the descent base
(`absorb_deliverO`), the tower chains (`tower_deliverO`/
`tower_noblockO`/`top_blockedO`), the generic tower-state invariants,
and the four pump windows.

# Shape

Every statement is the base lemma's with the bundle swapped to
`FamOKO` — the tower/fins/rootret rows are order-independent and the
position packages (`DescSupply`/`AscCover`) are family-generic, so the
proofs are transcriptions consuming the Ord/Pump.lean toolkit. The
absorber-touching lemmas consume `absorb_stuckO`, whose starved-arm
offsets (`ord.absorb.wirePhase`) stay symbolic under `omega` — no
per-assignment dispatch at any call site here.

ONE statement moves (the recorded deviation of this unit, see
`wire0_windowO`): the leaf-wire window's request-feed slack is
`1 - ord.absorb.wirePhase` — the base `+ 1` at a reply-first absorber
(which consumes the wire first, so its request intake may trail by a
block), `+ 0` at a query-first absorber (which consumes the request
first, so a wire send is covered only if the request feed already
reaches it). With the base `+ 1` slack the query-first request-starved
arm pins `rcv wire = rcv leafRequests` and the send is one past the
window — the base statement is not provable there, and the O site
layer must discharge the tighter feed fact at query-first assignments.
Every other window keeps the base hypothesis list verbatim.

Chain (ord, stage D): the pump layer; consumed by the O master
induction and the O drain ladder. Base mirror:
Proofs/Sched/Weave/Window.lean. Map: PROGRESS.md §13.
-/
import StreamingMirror.Ord.Pump
import StreamingMirror.Proofs.Counting

namespace StreamingMirror.Ord

open Model
open Sched

variable (sk : Skel) (ord : OrdMap)

-- ======================================================== the chains

/-- A channel-side count never exceeds its owner's whole-trace total,
O twin of `count_le_owner`. -/
theorem count_le_ownerO {P : List (List Ev)} (hfam : FamOKO sk ord P)
    {fut : List Ev}
    {st : MState} (h : WCountP sk P fut st) (c : Chan) (b : Bool)
    {M : Nat} (hM : (if b then sndOwner sk c else rcvOwner sk c) = M)
    {T : List Ev} (hT : P[M]? = some T) :
    (proj c b st.out).length ≤ (proj c b T).length := by
  obtain ⟨r, pre, hr, hpre, hsub⟩ := cell_of_owner sk h hT
  rw [out_proj_ownerO sk ord hfam h c b hM hT hr hpre hsub, hpre,
    proj_append, List.length_append]
  omega

/-- An interior tower's level output never exceeds its resolution
count, O twin of `level_snd_le`: the tower rows are the base rows. -/
theorem level_snd_leO {P : List (List Ev)} (hfam : FamOKO sk ord P)
    {fut : List Ev}
    {st : MState} (h : WCountP sk P fut st) {p : Party} {top j : Nat}
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
  have hcount := count_le_ownerO sk ord hfam h (Chan.level p j) true
    (M := asmIdx sk p j) (by simpa using sndOwner_level sk hnz)
    (famOKO_asm_procs sk ord hfam htop h1 (by omega))
  rw [sndCount_eq_proj]
  calc (proj (Chan.level p j) true st.out).length
      ≤ (proj (Chan.level p j) true (asmEvents sk (p, j))).length :=
        hcount
    _ = (sk.asmResList p j).length := by
        rw [← hout, (asm_totals sk (p, j)).2.2, seg_len]

/-- The absorber's level-0 output never exceeds the leaf-request
total, O twin of `level0_snd_le`. -/
theorem level0_snd_leO {P : List (List Ev)} (hfam : FamOKO sk ord P)
    {fut : List Ev}
    {st : MState} (h : WCountP sk P fut st) :
    sndCount (Chan.level Party.I 0) st.out ≤ sk.totalLeafReqs := by
  have hcount := count_le_ownerO sk ord hfam h (Chan.level Party.I 0) true
    (M := 2 + sk.rootH) (by simp [sndOwner]) (famOKO_absorb sk ord hfam)
  rw [sndCount_eq_proj]
  calc (proj (Chan.level Party.I 0) true st.out).length
      ≤ (proj (Chan.level Party.I 0) true
          (absorbEventsO sk ord)).length := hcount
    _ = sk.totalLeafReqs := by rw [(absorb_totalsO sk ord).2.2, seg_len]

/-- The absorber's level output never outruns its request intake, O
twin of `absorb_out_le_req`: under BOTH orders each level-0 send
follows its block's leaf-request receive. -/
theorem absorb_out_le_reqO {P : List (List Ev)} (hfam : FamOKO sk ord P)
    {fut : List Ev}
    {st : MState} (h : WCountP sk P fut st) :
    sndCount (Chan.level Party.I 0) st.out
      ≤ rcvCount Chan.leafRequests st.out := by
  obtain ⟨r, pre, hr, hpre, hsub⟩ :=
    cell_of_owner sk h (famOKO_absorb sk ord hfam)
  have hRc : rcvCount Chan.leafRequests st.out
      = (proj Chan.leafRequests false pre).length := by
    rw [rcvCount_eq_proj,
      out_proj_ownerO sk ord hfam h _ false (M := 2 + sk.rootH)
        (by simp [rcvOwner]) (famOKO_absorb sk ord hfam) hr hpre hsub]
  have hOc : sndCount (Chan.level Party.I 0) st.out
      = (proj (Chan.level Party.I 0) true pre).length := by
    rw [sndCount_eq_proj,
      out_proj_ownerO sk ord hfam h _ true (M := 2 + sk.rootH)
        (by simp [sndOwner]) (famOKO_absorb sk ord hfam) hr hpre hsub]
  cases r with
  | nil =>
      rw [List.append_nil] at hpre
      obtain ⟨-, ht2, ht3⟩ := absorb_totalsO sk ord
      rw [hpre] at ht2 ht3
      rw [hRc, hOc, ht2, ht3, seg_len, seg_len]
      exact Nat.le_refl _
  | cons e₀ rest₀ =>
      obtain ⟨t, htN, hshape⟩ := absorb_cell_shapeO sk ord hpre (by simp)
      rcases hshape with ⟨-, -, hc2, hc3⟩ | ⟨-, -, hc2, hc3⟩
        | ⟨-, -, hc2, hc3⟩ <;> rw [hRc, hOc] <;> omega

/-- Nobody sends the responder's phantom level-0 channel, O twin of
`levelR0_snd_zero`. -/
theorem levelR0_snd_zeroO (hwf : sk.wellFormed = true)
    {P : List (List Ev)} (hfam : FamOKO sk ord P) {fut : List Ev}
    {st : MState} (h : WCountP sk P fut st) :
    sndCount (Chan.level Party.R 0) st.out = 0 := by
  have hge := (wf_rootH hwf).2
  have hM : sndOwner sk (Chan.level Party.R 0)
      = asmIdx sk Party.R 1 :=
    sndOwner_level sk (by simp)
  have hT := famOKO_asmR sk ord hfam (Nat.le_refl 1) (by omega)
  have hcount := count_le_ownerO sk ord hfam h (Chan.level Party.R 0) true
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

/-- ABSORBER DELIVERY, O twin of `absorb_deliver`: at a pump fixpoint
a drained absorber with its wire and request feeds present through `c`
has produced `c` level-0 returns — both orders consume both feeds, so
the starved arms close with the phase offset symbolic. -/
theorem absorb_deliverO (hwf : sk.wellFormed = true)
    {P : List (List Ev)} (hfam : FamOKO sk ord P) {fut : List Ev}
    {st : MState} (h : WCountP sk P fut st) (hfix : step sk st = none)
    {c : Nat} (hcN : c ≤ sk.totalLeafReqs)
    (hw : c ≤ sndCount (Chan.wire Party.R 0) st.out)
    (hq : c ≤ sndCount Chan.leafRequests st.out)
    (hdrain : sndCount (Chan.level Party.I 0) st.out
      ≤ rcvCount (Chan.level Party.I 0) st.out) :
    c ≤ sndCount (Chan.level Party.I 0) st.out := by
  have hcap := wf_capLevel hwf
  rcases absorb_stuckO sk ord hfam h hfix with
    ⟨hW, hL, hV⟩ | ⟨hWt, hLW, hVW, hsw⟩ | ⟨hLt, hWL, hVL, hsq⟩
    | ⟨hVt, hWV, hLV, hblk⟩
  · omega
  · omega
  · omega
  · rw [cap_level] at hblk
    omega

/-- TOP BLOCKING IS ABSURD, O twin of `top_blocked`: the two tower
tops can never be the blocked window. -/
theorem top_blockedO (hwf : sk.wellFormed = true)
    {P : List (List Ev)} (hfam : FamOKO sk ord P) {fut : List Ev}
    {st : MState} (h : WCountP sk P fut st) (hfix : step sk st = none)
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
      have hT := famOKO_asmI sk ord hfam (by omega) (Nat.le_refl _)
      have hcount := count_le_ownerO sk ord hfam h Chan.rootret true
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
    rcases rootret_stuckO sk ord hfam h hfix (by omega) with
      h1 | ⟨h0, hs0⟩
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
      have hT := famOKO_asmR sk ord hfam (by omega) (Nat.le_refl _)
      have hcount := count_le_ownerO sk ord hfam h Chan.rootrets true
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
    rcases fin_stuckO sk ord hfam h hfix (by omega) with
      ⟨ha, hb⟩ | ⟨ha, hb, hc⟩ | ⟨ha, hb, hc⟩
    · omega
    · omega
    · omega

/-- ASCENT, O twin of `tower_noblock`: a blocked level window is
absurd — every consumer above, covered by the ascent package, drains
what the window carries, all the way to the root returns. The tower
rows are order-independent, so the climb is the base one over the O
bundle. -/
theorem tower_noblockO (hwf : sk.wellFormed = true)
    {P : List (List Ev)} (hfam : FamOKO sk ord P) {fut : List Ev}
    {st : MState} (h : WEdgeP sk P fut st) (hfix : step sk st = none)
    {p : Party} {top : Nat}
    (htop : p = Party.I ∧ top = sk.rootH
      ∨ p = Party.R ∧ top = sk.rootH - 1)
    (hroot : 1 ≤ sndCount Chan.rootres st.out)
    (j : Nat) (h1 : 1 ≤ j) (hjt : j ≤ top)
    (hcov : AscCover sk st p j top)
    (hself : asks p j = true →
      sndCount (Chan.level p (j - 1)) st.out
        < sk.dsBefore (j - 1)
            (sndCount (Chan.upper p (j - 1)) st.out)
          + sk.capLevel)
    (hblk : rcvCount (asmLevelChan (p, j)) st.out + sk.capLevel
      ≤ sndCount (asmLevelChan (p, j)) st.out) : False := by
  have hcap := wf_capLevel hwf
  have hge2 := (wf_rootH hwf).2
  -- the phantom base: nobody sends the responder's level 0
  by_cases hR1 : p = Party.R ∧ j = 1
  · obtain ⟨rfl, rfl⟩ := hR1
    have hz : sndCount (asmLevelChan (Party.R, 1)) st.out = 0 :=
      levelR0_snd_zeroO sk ord hwf hfam h.toWCountP
    omega
  have hIdx := famOKO_asm_procs sk ord hfam htop h1 hjt
  rcases asm_stuckO sk ord hfam h.toWCountP hfix h1 hIdx with
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
          ≤ sk.totalLeafReqs := level0_snd_leO sk ord hfam h.toWCountP
      have htot : sk.pendsBefore Party.I 1
          (sk.asmResList Party.I 1).length = sk.totalLeafReqs :=
        pendsBefore_answerer_leaf (hna := rfl)
      omega
    · have hS : sndCount (asmLevelChan (p, j)) st.out
          ≤ (sk.asmResList p (j - 1)).length :=
        level_snd_leO sk ord hfam h.toWCountP htop (by omega) (by omega)
      have htot := pends_total_prod hwf (p := p) (j := j)
        (by omega)
        (by rcases htop with ⟨-, ht⟩ | ⟨-, ht⟩ <;> omega)
      omega
  · -- res-starved: the position facts refute the shut window
    cases hask : asks p j with
    | true =>
        -- asker: `hself` caps the level below under the very count
        -- the starvation pins to the window
        have h2 : 2 ≤ j := by
          rcases Nat.lt_or_ge j 2 with hj2 | hj2
          · exfalso
            have hj1 : j = 1 := by omega
            subst hj1
            cases p with
            | I => rw [show asks Party.I 1 = false from rfl] at hask
                   cases hask
            | R => exact hR1 ⟨rfl, rfl⟩
          · exact hj2
        rw [asmResChan_asker hask] at hres
        rw [show asmLevelChan (p, j) = Chan.level p (j - 1) from rfl]
          at hLp hblk
        rw [asmResChan_asker hask] at hLp
        have hpbR : sk.pendsBefore p j
            (rcvCount (Chan.upper p (j - 1)) st.out)
            = sk.dsBefore (j - 1)
                (rcvCount (Chan.upper p (j - 1)) st.out) :=
          pendsBefore_asker hask h2 _
        have hpbS : sk.pendsBefore p j
            (sndCount (Chan.upper p (j - 1)) st.out)
            = sk.dsBefore (j - 1)
                (sndCount (Chan.upper p (j - 1)) st.out) :=
          pendsBefore_asker hask h2 _
        have hmono := pendsBefore_mono sk p j hres
        have hs := hself hask
        omega
    | false =>
        -- answerer: `Φ` says the level below is short of the very
        -- allocation the starvation pins to the window
        obtain ⟨hphi, -⟩ := hcov j (Nat.le_refl j) hjt hask
        rw [asmResChan_answerer hask] at hres
        rw [show asmLevelChan (p, j) = Chan.level p (j - 1) from rfl]
          at hLp hblk
        rw [asmResChan_answerer hask] at hLp
        have hmono := pendsBefore_mono sk p j hres
        omega
  · -- level-starved against blocked: the window is both shut and dry
    omega
  · -- out-blocked: ascend
    by_cases hjtop : j = top
    · subst hjtop
      exact top_blockedO sk ord hwf hfam h.toWCountP hfix htop hroot
        hoblk
    · have hjlt : j < top := Nat.lt_of_le_of_ne hjt hjtop
      have hout := asmOutChan_of_lt sk htop hjlt
      rw [hout, cap_level] at hoblk
      refine tower_noblockO hwf hfam h hfix htop hroot (j + 1) (by omega)
        (by omega) (fun g hg1 hg2 hna => hcov g (by omega) hg2 hna)
        ?_ hoblk
      -- re-establish `hself` one stage up: the asker above sits over
      -- THIS answerer, whose stuck pins plus the package bound its
      -- output under the allocation line
      intro hask1
      have hna : asks p j = false := by
        have hs := asks_succ p j
        rw [hask1] at hs
        simpa using hs.symm
      obtain ⟨hphi, hp1⟩ := hcov j (Nat.le_refl j) hjt hna
      rw [asmResChan_answerer hna] at hRl hR1' hOp
      rw [asmResChan_answerer hna] at hLp
      rw [show asmLevelChan (p, j) = Chan.level p (j - 1) from rfl]
        at hLp
      rw [hout] at hOp
      have hwedge := wedge_rcvd_le_sentO sk ord hfam h (Chan.lower p j)
      rw [Nat.add_sub_cancel]
      rcases Nat.lt_or_ge (rcvCount (Chan.lower p j) st.out)
          (sndCount (Chan.lower p j) st.out) with hlt | hge
      · -- a step behind its walk: `P1` bounds the output directly
        omega
      · -- consumed everything sent: `Φ` kills the pins outright
        exfalso
        have hRe : rcvCount (Chan.lower p j) st.out
            = sndCount (Chan.lower p j) st.out := by omega
        rw [hRe] at hLp
        have hwl := wedge_rcvd_le_sentO sk ord hfam h
          (Chan.level p (j - 1))
        omega
termination_by top - j

/-- DESCENT, O twin of `tower_deliver`: at a pump fixpoint a drained
interior tower with descent supplies through demand `c` has produced
`c` outputs — bottoming at `absorb_deliverO`, which serves both
orders. -/
theorem tower_deliverO (hwf : sk.wellFormed = true)
    {P : List (List Ev)} (hfam : FamOKO sk ord P) {fut : List Ev}
    {st : MState} (h : WCountP sk P fut st) (hfix : step sk st = none)
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
  have hIdx := famOKO_asm_procs sk ord hfam htop h1 (by omega)
  obtain ⟨j', rfl⟩ : ∃ j', j = j' + 1 := ⟨j - 1, by omega⟩
  have hsup1 : c ≤ sndCount (asmResChan (p, j' + 1)) st.out := hsup.1
  have hsup2 : DescSupply sk st p j'
      (sk.pendsBefore p (j' + 1) c) := hsup.2
  rcases asm_stuckO sk ord hfam h hfix h1 hIdx with
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
          have hdel := absorb_deliverO sk ord hwf hfam h hfix hc₀N
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
      have hdel := tower_deliverO hwf hfam h hfix htop (j'' + 1)
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

/-- A tower's output never outruns its resolutions, O twin of
`asm_out_le_res`. -/
theorem asm_out_le_resO {P : List (List Ev)} (hfam : FamOKO sk ord P)
    {fut : List Ev}
    {st : MState} (h : WCountP sk P fut st) {p : Party} {top j : Nat}
    (htop : p = Party.I ∧ top = sk.rootH
      ∨ p = Party.R ∧ top = sk.rootH - 1)
    (h1 : 1 ≤ j) (hjt : j ≤ top) :
    sndCount (sk.asmOutChan (p, j)) st.out
      ≤ rcvCount (asmResChan (p, j)) st.out := by
  have hIdx := famOKO_asm_procs sk ord hfam htop h1 hjt
  obtain ⟨r, pre, hr, hpre, hsub⟩ := cell_of_owner sk h hIdx
  obtain ⟨hro, hlo, hoo⟩ := asm_owners sk p h1
  have hRc : rcvCount (asmResChan (p, j)) st.out
      = (proj (asmResChan (p, j)) false pre).length := by
    rw [rcvCount_eq_proj,
      out_proj_ownerO sk ord hfam h _ false (by simpa using hro)
        hIdx hr hpre hsub]
  have hOc : sndCount (sk.asmOutChan (p, j)) st.out
      = (proj (sk.asmOutChan (p, j)) true pre).length := by
    rw [sndCount_eq_proj,
      out_proj_ownerO sk ord hfam h _ true (by simpa using hoo)
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
allocation, O twin of `asm_lvl_le_pends`. -/
theorem asm_lvl_le_pendsO {P : List (List Ev)} (hfam : FamOKO sk ord P)
    {fut : List Ev}
    {st : MState} (h : WCountP sk P fut st) {p : Party} {top j : Nat}
    (htop : p = Party.I ∧ top = sk.rootH
      ∨ p = Party.R ∧ top = sk.rootH - 1)
    (h1 : 1 ≤ j) (hjt : j ≤ top) :
    rcvCount (asmLevelChan (p, j)) st.out
      ≤ sk.pendsBefore p j (rcvCount (asmResChan (p, j)) st.out) := by
  have hIdx := famOKO_asm_procs sk ord hfam htop h1 hjt
  obtain ⟨r, pre, hr, hpre, hsub⟩ := cell_of_owner sk h hIdx
  obtain ⟨hro, hlo, hoo⟩ := asm_owners sk p h1
  have hRc : rcvCount (asmResChan (p, j)) st.out
      = (proj (asmResChan (p, j)) false pre).length := by
    rw [rcvCount_eq_proj,
      out_proj_ownerO sk ord hfam h _ false (by simpa using hro)
        hIdx hr hpre hsub]
  have hLc : rcvCount (asmLevelChan (p, j)) st.out
      = (proj (asmLevelChan (p, j)) false pre).length := by
    rw [rcvCount_eq_proj,
      out_proj_ownerO sk ord hfam h _ false (by simpa using hlo)
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

/-- A tower's output allocation is covered by its level intake, O twin
of `asm_pends_le_out`: `out i` departs only after item `i`'s pends are
consumed. -/
theorem asm_pends_le_outO {P : List (List Ev)} (hfam : FamOKO sk ord P)
    {fut : List Ev}
    {st : MState} (h : WCountP sk P fut st) {p : Party} {top j : Nat}
    (htop : p = Party.I ∧ top = sk.rootH
      ∨ p = Party.R ∧ top = sk.rootH - 1)
    (h1 : 1 ≤ j) (hjt : j ≤ top) :
    sk.pendsBefore p j (sndCount (sk.asmOutChan (p, j)) st.out)
      ≤ rcvCount (asmLevelChan (p, j)) st.out := by
  have hIdx := famOKO_asm_procs sk ord hfam htop h1 hjt
  obtain ⟨r, pre, hr, hpre, hsub⟩ := cell_of_owner sk h hIdx
  obtain ⟨hro, hlo, hoo⟩ := asm_owners sk p h1
  have hLc : rcvCount (asmLevelChan (p, j)) st.out
      = (proj (asmLevelChan (p, j)) false pre).length := by
    rw [rcvCount_eq_proj,
      out_proj_ownerO sk ord hfam h _ false (by simpa using hlo)
        hIdx hr hpre hsub]
  have hOc : sndCount (sk.asmOutChan (p, j)) st.out
      = (proj (sk.asmOutChan (p, j)) true pre).length := by
    rw [sndCount_eq_proj,
      out_proj_ownerO sk ord hfam h _ true (by simpa using hoo)
        hIdx hr hpre hsub]
  cases r with
  | nil =>
      rw [List.append_nil] at hpre
      obtain ⟨-, ht2, ht3⟩ := asm_totals sk (p, j)
      rw [hpre] at ht2 ht3
      rw [hLc, hOc, ht2, ht3, seg_len, seg_len]
      exact Nat.le_refl _
  | cons e₀ rest₀ =>
      obtain ⟨idx, hidxN, hshape⟩ :=
        asm_cell_shape sk (p, j) hpre (by simp)
      rcases hshape with ⟨-, -, hc2, hc3⟩
        | ⟨tlv, rest, -, htl, -, -, hc2, hc3⟩ | ⟨-, -, hc2, hc3⟩
      · have hc2' : (proj (asmLevelChan (p, j)) false pre).length
            = sk.pendsBefore p j idx := hc2
        rw [hLc, hOc, hc3, hc2']
        exact Nat.le_refl _
      · have htl' : sk.pendsBefore p j idx ≤ tlv := htl
        rw [hLc, hOc, hc3, hc2]
        exact htl'
      · have hc2' : (proj (asmLevelChan (p, j)) false pre).length
            = sk.pendsBefore p j (idx + 1) := hc2
        rw [hLc, hOc, hc3, hc2']
        exact pendsBefore_mono sk p j (Nat.le_succ idx)

/-- A send never outruns its window, O twin of
`wedge_snd_le_rcv_cap`: the emitted stream respected E2 at every
position. -/
theorem wedge_snd_le_rcv_capO {P : List (List Ev)}
    (hfam : FamOKO sk ord P)
    {fut : List Ev} {st : MState} (h : WEdgeP sk P fut st) (c : Chan) :
    sndCount c st.out ≤ rcvCount c st.out + sk.cap c := by
  cases hz : sndCount c st.out with
  | zero => omega
  | succ q =>
      have hcanon := wproj_canonP sk h.toWCountP c true
        (hfam.snd_owned) (hfam.canon c true)
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

/-- THE UPPER WINDOW, O twin of `upper_window`: at a pump fixpoint the
asker above has consumed every resolution before the one the walk is
about to send. -/
theorem upper_windowO (hwf : sk.wellFormed = true)
    {P : List (List Ev)} (hfam : FamOKO sk ord P) {fut : List Ev}
    {st : MState} (h : WEdgeP sk P fut st) (hfix : step sk st = none)
    {p : Party} {top hh k : Nat}
    (htop : p = Party.I ∧ top = sk.rootH
      ∨ p = Party.R ∧ top = sk.rootH - 1)
    (hasks : asks p (hh + 1) = true)
    (hht : hh + 1 ≤ top)
    (hk : k < sk.stageLen hh)
    (hsnd : sndCount (Chan.upper p hh) st.out = k)
    (hdesc : DescSupply sk st p hh (sk.pendsBefore p (hh + 1) k))
    (hcov : AscCover sk st p (hh + 2) top)
    (hroot : 1 ≤ sndCount Chan.rootres st.out) :
    k ≤ rcvCount (Chan.upper p hh) st.out := by
  have hcap := wf_capLevel hwf
  have hge2 := (wf_rootH hwf).2
  have hres : asmResChan (p, hh + 1) = Chan.upper p hh :=
    asmResChan_asker hasks
  have hIdx := famOKO_asm_procs sk ord hfam htop (by omega) hht
  have hRk : rcvCount (Chan.upper p hh) st.out ≤ k := by
    have := wedge_rcvd_le_sentO sk ord hfam h (Chan.upper p hh)
    omega
  have hstuck := asm_stuckO sk ord hfam h.toWCountP hfix
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
      have hdel := tower_deliverO sk ord hwf hfam h.toWCountP hfix htop
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
      exact top_blockedO sk ord hwf hfam h.toWCountP hfix htop hroot
        hoblk
    · have hout' := asmOutChan_of_lt sk htop
        (show hh + 1 < top from by omega)
      rw [hout', cap_level] at hoblk
      have hna2 : asks p (hh + 2) = false := by
        have hs := asks_succ p (hh + 1)
        rw [show hh + 1 + 1 = hh + 2 from by omega, hasks] at hs
        simpa using hs
      exact tower_noblockO sk ord hwf hfam h hfix htop hroot
        (hh + 2) (by omega) (by omega) hcov
        (fun hask => absurd hask (by rw [hna2]; simp)) hoblk

/-- THE LOWER WINDOW, O twin of `lower_window`: at a pump fixpoint the
answerer has consumed every resolution before the one the walk is
about to send. -/
theorem lower_windowO (hwf : sk.wellFormed = true)
    {P : List (List Ev)} (hfam : FamOKO sk ord P) {fut : List Ev}
    {st : MState} (h : WEdgeP sk P fut st) (hfix : step sk st = none)
    {p : Party} {top hh d : Nat}
    (htop : p = Party.I ∧ top = sk.rootH
      ∨ p = Party.R ∧ top = sk.rootH - 1)
    (hna : asks p hh = false)
    (h1 : 1 ≤ hh) (hht : hh < top)
    (hd : d < (sk.asmResList p hh).length)
    (hsnd : sndCount (Chan.lower p hh) st.out = d)
    (hp1 : d ≤ sk.dsBefore hh (sndCount (Chan.upper p hh) st.out)
      + sk.capLevel + 1)
    (hdesc : DescSupply sk st p (hh - 1) (sk.pendsBefore p hh d))
    (hcov : AscCover sk st p (hh + 1) top)
    (hroot : 1 ≤ sndCount Chan.rootres st.out) :
    d ≤ rcvCount (Chan.lower p hh) st.out := by
  have hcap := wf_capLevel hwf
  have hge2 := (wf_rootH hwf).2
  obtain ⟨hh', rfl⟩ : ∃ hh', hh = hh' + 1 := ⟨hh - 1, by omega⟩
  have hdesc' : DescSupply sk st p hh'
      (sk.pendsBefore p (hh' + 1) d) := hdesc
  have hres : asmResChan (p, hh' + 1) = Chan.lower p (hh' + 1) :=
    asmResChan_answerer hna
  have hIdx := famOKO_asm_procs sk ord hfam htop (by omega) (by omega)
  have hRd : rcvCount (Chan.lower p (hh' + 1)) st.out ≤ d := by
    have := wedge_rcvd_le_sentO sk ord hfam h (Chan.lower p (hh' + 1))
    omega
  have hstuck := asm_stuckO sk ord hfam h.toWCountP hfix
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
      have hdel := absorb_deliverO sk ord hwf hfam h.toWCountP hfix hc₀N
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
      have hdel := tower_deliverO sk ord hwf hfam h.toWCountP hfix htop
        (hh'' + 1)
        (sk.pendsBefore p (hh'' + 1 + 1)
          (rcvCount (Chan.lower p (hh'' + 1 + 1)) st.out))
        (by omega) (by omega) hcN'
        (descSupply_mono sk hmono hdesc') hdrain'
      rw [hout'] at hdel
      omega
  · -- out-blocked: either the resolution is already consumed, or the
    -- ascent refutes the block
    by_cases hgoal : d ≤ rcvCount (Chan.lower p (hh' + 1)) st.out
    · exact hgoal
    · exfalso
      have hout' := asmOutChan_of_lt sk htop hht
      rw [hout'] at hOp
      rw [hout', cap_level] at hoblk
      refine tower_noblockO sk ord hwf hfam h hfix htop hroot
        (hh' + 1 + 1) (by omega) (by omega)
        (fun g hg1 hg2 hna => hcov g (by omega) hg2 hna)
        ?_ hoblk
      -- the asker above sits over THIS walk: its stuck consumer is a
      -- step behind the unsent `d`, and `hp1` is the walk's own
      -- schedulable overhang bound
      intro _
      rw [Nat.add_sub_cancel]
      omega

/-- THE LEAF-WIRE WINDOW, O twin of `wire0_window`: at a pump fixpoint
the absorber has consumed every leaf wire before the one the walk is
about to send.

THE RECORDED STATEMENT DEVIATION of this unit: the request-feed slack
is `1 - ord.absorb.wirePhase`, not the base `+ 1`. A reply-first
absorber consumes the wire first, so its request intake may trail the
wire intake by one block and the feed only needs to reach `w - 1`; a
query-first absorber consumes the request first, so its request-starved
arm pins the two intakes EQUAL and a wire send at `w` is covered only
if the request feed already reaches `w`. At `ord.absorb = .replyFirst`
the hypothesis is definitionally the base one; the base statement is
not provable at query-first (the request-starved arm leaves
`w ≤ rcv + 1`), so the O site layer owes the tighter feed fact there. -/
theorem wire0_windowO (hwf : sk.wellFormed = true)
    {P : List (List Ev)} (hfam : FamOKO sk ord P) {fut : List Ev}
    {st : MState} (h : WEdgeP sk P fut st) (hfix : step sk st = none)
    {w : Nat} (hw : w < sk.totalLeafReqs)
    (hsnd : sndCount (Chan.wire Party.R 0) st.out = w)
    (hreq : w ≤ sndCount Chan.leafRequests st.out
      + (1 - ord.absorb.wirePhase))
    (hcov : AscCover sk st Party.I 1 sk.rootH)
    (hroot : 1 ≤ sndCount Chan.rootres st.out) :
    w ≤ rcvCount (Chan.wire Party.R 0) st.out := by
  have hcap := wf_capLevel hwf
  rcases absorb_stuckO sk ord hfam h.toWCountP hfix with
    ⟨hW, hL, hV⟩ | ⟨hWt, hLW, hVW, hsw⟩ | ⟨hLt, hWL, hVL, hsq⟩
    | ⟨hVt, hWV, hLV, hblk⟩
  · omega
  · omega
  · omega
  · exfalso
    rw [cap_level] at hblk
    exact tower_noblockO sk ord hwf hfam h hfix (Or.inl ⟨rfl, rfl⟩)
      hroot 1 (Nat.le_refl 1) (by have := (wf_rootH hwf).2; omega)
      hcov (fun hask => absurd hask (by decide)) hblk

/-- THE LEAF-REQUEST WINDOW, O twin of `leafreq_window`: at a pump
fixpoint the absorber has consumed every leaf request before the one
the walk is about to send — the base hypothesis list verbatim; the
wire feed covers the request under both orders. -/
theorem leafreq_windowO (hwf : sk.wellFormed = true)
    {P : List (List Ev)} (hfam : FamOKO sk ord P) {fut : List Ev}
    {st : MState} (h : WEdgeP sk P fut st) (hfix : step sk st = none)
    {q : Nat} (hq : q < sk.totalLeafReqs)
    (hsnd : sndCount Chan.leafRequests st.out = q)
    (hwire : q ≤ sndCount (Chan.wire Party.R 0) st.out)
    (hcov : AscCover sk st Party.I 1 sk.rootH)
    (hroot : 1 ≤ sndCount Chan.rootres st.out) :
    q ≤ rcvCount Chan.leafRequests st.out := by
  have hcap := wf_capLevel hwf
  rcases absorb_stuckO sk ord hfam h.toWCountP hfix with
    ⟨hW, hL, hV⟩ | ⟨hWt, hLW, hVW, hsw⟩ | ⟨hLt, hWL, hVL, hsq⟩
    | ⟨hVt, hWV, hLV, hblk⟩
  · omega
  · omega
  · omega
  · exfalso
    rw [cap_level] at hblk
    exact tower_noblockO sk ord hwf hfam h hfix (Or.inl ⟨rfl, rfl⟩)
      hroot 1 (Nat.le_refl 1) (by have := (wf_rootH hwf).2; omega)
      hcov (fun hask => absurd hask (by decide)) hblk

end StreamingMirror.Ord
