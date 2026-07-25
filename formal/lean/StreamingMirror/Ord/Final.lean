/-
The drained O weave: merge completeness for the O trace family at
every assignment of the two-point dequeue-order class.

# Shape

`Final.lean`'s drain ladder is generic over `FamOK` + `ManRows`, but
`FamOK.pumps` pins the pump suffix to the reply-first `weavePumps`, so
the query-first-absorber assignments cannot inhabit it. This file twins
the ladder over `FamOKO` (Ord/Pump.lean; pump suffix `weavePumpsO`,
inhabited by `procsO` at EVERY assignment): the walk/opener totals ride
`ManRows` (proj-shaped, unconditional at `procsO`), the tower drain and
jam climb consume the O stuck trichotomies, and the absorber's drain
case dispatches through `absorb_stuckO` — whose starved arms close by
`omega` with the `ord.absorb.wirePhase` offsets symbolic, so nothing
here dispatches on the assignment.

The capacity hypothesis appears exactly once: `weaveO_wedge`'s margin-0
input. Everything downstream is the base argument verbatim — the
drained witness, the τ ranking, the blame step, and the stall
refutation are placement-independent once edge-respect of the witness
is in hand.

Chain (ord, stage D exit → E): merge completeness; consumed by
Ord/Pending's tau layer and the O endgame. Base mirror: FinalE.lean.
Map: PROGRESS.md §13.
-/
import StreamingMirror.Ord.Master
import StreamingMirror.Ord.Window

namespace StreamingMirror.Ord

open Model
open Sched

variable (sk : Skel) (ord : OrdMap)

-- ======================================= manual traces are all out

/-- With no future left, every manual trace's projection is whole, O
twin of `man_proj_full`: the output's channel-side projection IS the
trace's. -/
theorem man_proj_fullO {P : List (List Ev)} (hfam : FamOKO sk ord P)
    {st : MState}
    (h : WCountP sk P [] st) (c : Chan) (b : Bool) {M : Nat}
    (hM : (if b then sndOwner sk c else rcvOwner sk c) = M)
    (hMlt : M < manCount sk) {T : List Ev}
    (hT : P[M]? = some T) :
    proj c b st.out = proj c b T := by
  have hlen : (manFilters sk ([] : List Ev)).length = manCount sk := by
    unfold manFilters
    rw [List.length_map, List.length_range]
  have hlt : M < (manFilters sk ([] : List Ev)).length := by omega
  have hr : (manFilters sk ([] : List Ev) ++ st.rem)[M]?
      = some ((manFilters sk ([] : List Ev))[M]) := by
    rw [List.getElem?_append_left hlt]
    exact List.getElem?_eq_getElem hlt
  have hnil : (manFilters sk ([] : List Ev))[M] = [] :=
    manFilters_nil_mem sk (List.getElem_mem hlt)
  rw [hnil] at hr
  obtain ⟨pre, hpre, hsub⟩ :=
    Forall2.rel_of_getElem? (wcount_glue sk h) hT hr
  have hcore := out_proj_ownerO sk ord hfam h c b hM hT hr hpre hsub
  rw [hcore, hpre, List.append_nil]

/-- A walk-owned channel's send count at a drained-manual state is the
walk's whole-trace total, O twin of `walk_count_full` (the total is
the BASE walk trace's — `ManRows` is proj-shaped against it). -/
theorem walk_count_fullO {P : List (List Ev)} (hfam : FamOKO sk ord P)
    (hman : ManRows sk P) {st : MState}
    (h : WCountP sk P [] st) {hh : Nat} (hhr : hh < sk.rootH)
    (c : Chan) (hc : sndOwner sk c = walkIdx sk hh) :
    sndCount c st.out = (proj c true (walkEvents sk (wpk hh))).length := by
  have hMlt : walkIdx sk hh < manCount sk := by
    unfold walkIdx manCount
    omega
  obtain ⟨T, hT, hproj⟩ := hman.walk hhr
  rw [sndCount_eq_proj,
    man_proj_fullO sk ord hfam h c true (by simpa using hc) hMlt hT,
    hproj]

/-- The resolution feed of every assembler is fully sent once the
walks are drained, O twin of `asm_res_full`: the walk totals meet the
assembler's demand exactly. -/
theorem asm_res_fullO (hwf : sk.wellFormed = true)
    {P : List (List Ev)} (hfam : FamOKO sk ord P) (hman : ManRows sk P)
    {st : MState}
    (h : WCountP sk P [] st) {p : Party} {top j : Nat}
    (htop : p = Party.I ∧ top = sk.rootH
      ∨ p = Party.R ∧ top = sk.rootH - 1)
    (h1 : 1 ≤ j) (hjt : j ≤ top) :
    sndCount (asmResChan (p, j)) st.out = (sk.asmResList p j).length := by
  have hge2 := (wf_rootH hwf).2
  have hjr : j ≤ sk.rootH := by
    rcases htop with ⟨-, ht⟩ | ⟨-, ht⟩ <;> omega
  cases hask : asks p j with
  | true =>
      have hna : asks p (j - 1) = false := by
        have hs := asks_succ p (j - 1)
        rw [show j - 1 + 1 = j from by omega, hask] at hs
        cases hp : asks p (j - 1)
        · rfl
        · rw [hp] at hs
          simp at hs
      have hpk : (wpk (j - 1)).1 = p := wpk_fst_of_answerer hna
      have hup : Chan.upper p (j - 1) = upperOut (wpk (j - 1)) := by
        unfold upperOut
        rw [hpk]
        rfl
      have hcnt := walk_count_fullO sk ord hfam hman h (hh := j - 1)
        (by omega) (Chan.upper p (j - 1)) rfl
      rw [asmResChan_asker hask, hcnt, hup, walk_upper_total,
        asmResList_asker_length hask]
      unfold canon
      rw [List.length_map, List.length_range]
      show sk.stageLen (j - 1) = (sk.scopesAt j).length
      unfold Skel.stageLen Skel.stageScopes
      rw [show j - 1 + 1 = j from by omega]
  | false =>
      have hjlt : j < sk.rootH := by
        rcases htop with ⟨hpI, ht⟩ | ⟨-, ht⟩
        · rcases Nat.lt_or_ge j sk.rootH with hlt | hge
          · exact hlt
          · exfalso
            have hj : j = sk.rootH := by omega
            subst hj hpI
            have heven := (wf_rootH hwf).1
            simp [asks, heven] at hask
        · omega
      have hpk : (wpk j).1 = p := wpk_fst_of_answerer hask
      have hlow : Chan.lower p j = lowerOut (wpk j) := by
        unfold lowerOut
        rw [hpk]
        rfl
      have hcnt := walk_count_fullO sk ord hfam hman h (hh := j) hjlt
        (Chan.lower p j) rfl
      rw [asmResChan_answerer hask, hcnt, hlow, walk_lower_total,
        answerer_resList_total hwf hask h1 hjlt]
      show (canon _ _ _).length = _
      unfold canon
      rw [List.length_map, List.length_range]
      rfl

/-- The absorber's wire feed is fully sent once the walks drain, O
twin of `wire0_full` (the feed is the stage-0 WALK's send side — the
absorber's own dequeue order never enters). -/
theorem wire0_fullO (hwf : sk.wellFormed = true)
    {P : List (List Ev)} (hfam : FamOKO sk ord P) (hman : ManRows sk P)
    {st : MState}
    (h : WCountP sk P [] st) :
    sndCount (Chan.wire Party.R 0) st.out = sk.totalLeafReqs := by
  have hge2 := (wf_rootH hwf).2
  have hw : Chan.wire Party.R 0 = wireOut (wpk 0) := rfl
  have hcnt := walk_count_fullO sk ord hfam hman h (hh := 0) (by omega)
    (Chan.wire Party.R 0)
    (by simp only [sndOwner]; rw [if_neg (by omega)])
  rw [hcnt, hw, walk_wire_total]
  show (canon _ _ _).length = _
  unfold canon
  rw [List.length_map, List.length_range]
  exact wiresBefore_full_leaf hwf

/-- The absorber's request feed is fully sent once the walks drain, O
twin of `leafreq_full`. -/
theorem leafreq_fullO (hwf : sk.wellFormed = true)
    {P : List (List Ev)} (hfam : FamOKO sk ord P) (hman : ManRows sk P)
    {st : MState}
    (h : WCountP sk P [] st) :
    sndCount Chan.leafRequests st.out = sk.totalLeafReqs := by
  have hge2 := (wf_rootH hwf).2
  have hq : Chan.leafRequests = askedOut (wpk 1) := rfl
  have hcnt := walk_count_fullO sk ord hfam hman h (hh := 1) (by omega)
    Chan.leafRequests rfl
  rw [hcnt, hq, walk_asked_total]
  show (canon _ _ _).length = _
  unfold canon
  rw [List.length_map, List.length_range]
  exact qsBefore_full_leaf hwf

/-- The root resolution is sent once ropen drains, O twin of
`rootres_full`. -/
theorem rootres_fullO {P : List (List Ev)} (hfam : FamOKO sk ord P)
    (hman : ManRows sk P) {st : MState}
    (h : WCountP sk P [] st) :
    sndCount Chan.rootres st.out = 1 := by
  obtain ⟨T, hT, hprojT⟩ := hman.ropen
  have hproj := man_proj_fullO sk ord hfam h Chan.rootres true
    (M := 1) rfl (by unfold manCount; omega) hT
  rw [sndCount_eq_proj, hproj, hprojT]
  unfold ropenEvents
  rw [proj_cons_ne_side (by simp), proj_cons_ne_chan (by simp),
    proj_cons_self]
  have hmap : proj Chan.rootres true
      ((List.range sk.rootPending).map fun j =>
        (Chan.asked Party.R (sk.rootH - 2), true, j)) = [] := by
    refine List.filter_eq_nil_iff.2 fun e he => ?_
    obtain ⟨q, -, rfl⟩ := List.mem_map.1 he
    simp
  rw [hmap]
  rfl

-- ==================================================== the jam climb

/-- A jam on an assembler's level feed forces a jam on its output, O
twin of `level_jam_up`: the O stuck trichotomy's other arms clash with
the drained resolution feed, the feed totals, or the jam itself. -/
theorem level_jam_upO (hwf : sk.wellFormed = true)
    {P : List (List Ev)} (hfam : FamOKO sk ord P) (hman : ManRows sk P)
    {st : MState}
    (h : WCountP sk P [] st) (hfix : step sk st = none)
    {p : Party} {top j : Nat}
    (htop : p = Party.I ∧ top = sk.rootH
      ∨ p = Party.R ∧ top = sk.rootH - 1)
    (h1 : 1 ≤ j) (hjt : j ≤ top)
    (hle : sndCount (asmLevelChan (p, j)) st.out
      ≤ sk.pendsBefore p j (sk.asmResList p j).length)
    (hjam : rcvCount (asmLevelChan (p, j)) st.out
        + sk.cap (asmLevelChan (p, j))
      ≤ sndCount (asmLevelChan (p, j)) st.out) :
    rcvCount (sk.asmOutChan (p, j)) st.out
        + sk.cap (sk.asmOutChan (p, j))
      ≤ sndCount (sk.asmOutChan (p, j)) st.out := by
  have hcap : 1 ≤ sk.cap (asmLevelChan (p, j)) := cap_pos hwf _
  have hres := asm_res_fullO sk ord hwf hfam hman h htop h1 hjt
  have hIdx := famOKO_asm_procs sk ord hfam htop h1 hjt
  rcases asm_stuckO sk ord hfam h hfix h1 hIdx with
    ⟨hr, hl, ho⟩ | ⟨hr, hl, ho, hsr⟩
      | ⟨hr, h1r, hlo, hl, ho, hsl⟩ | ⟨hr, h1r, hl, ho, hblk⟩
  · -- exhausted: the level feed is complete, contradicting the jam
    rw [hl] at hjam
    omega
  · -- res-starved: the walks have drained
    omega
  · -- level-starved: outright clash with the jam
    omega
  · exact hblk

/-- No assembler's level feed is jammed at a drained-manual pump
fixpoint, O twin of `chain_no_jam`: the jam would climb the tower and
block the root returns. -/
theorem chain_no_jamO (hwf : sk.wellFormed = true)
    {P : List (List Ev)} (hfam : FamOKO sk ord P) (hman : ManRows sk P)
    {st : MState}
    (h : WCountP sk P [] st) (hfix : step sk st = none)
    (hroot : 1 ≤ sndCount Chan.rootres st.out)
    {p : Party} {top : Nat}
    (htop : p = Party.I ∧ top = sk.rootH
      ∨ p = Party.R ∧ top = sk.rootH - 1) :
    ∀ d j, 1 ≤ j → j ≤ top → top - j ≤ d →
      sndCount (asmLevelChan (p, j)) st.out
        ≤ sk.pendsBefore p j (sk.asmResList p j).length →
      rcvCount (asmLevelChan (p, j)) st.out
          + sk.cap (asmLevelChan (p, j))
        ≤ sndCount (asmLevelChan (p, j)) st.out → False := by
  have hge2 := (wf_rootH hwf).2
  intro d
  induction d with
  | zero =>
      intro j h1 hjt hd hle hjam
      have hj : j = top := by omega
      subst hj
      exact top_blockedO sk ord hwf hfam h hfix htop hroot
        (level_jam_upO sk ord hwf hfam hman h hfix htop h1 hjt hle hjam)
  | succ d ihd =>
      intro j h1 hjt hd hle hjam
      have hup := level_jam_upO sk ord hwf hfam hman h hfix htop h1 hjt
        hle hjam
      rcases Nat.lt_or_ge j top with hlt | hge
      · -- the jammed output is the level feed one tower up
        have hjr : j < sk.rootH := by
          rcases htop with ⟨-, ht⟩ | ⟨-, ht⟩ <;> omega
        have hout : sk.asmOutChan (p, j) = Chan.level p j :=
          asmOutChan_of_lt sk htop hlt
        have hnext : asmLevelChan (p, j + 1) = Chan.level p j := rfl
        rw [hout] at hup
        have hle' : sndCount (asmLevelChan (p, j + 1)) st.out
            ≤ sk.pendsBefore p (j + 1)
                (sk.asmResList p (j + 1)).length := by
          rw [hnext, pends_total_prod hwf (by omega : 2 ≤ j + 1)
            (by omega : j + 1 - 1 < sk.rootH)]
          show sndCount (Chan.level p j) st.out
            ≤ (sk.asmResList p (j + 1 - 1)).length
          rw [show j + 1 - 1 = j from by omega]
          exact level_snd_leO sk ord hfam h htop h1 hlt
        refine ihd (j + 1) (by omega) (by omega) (by omega) hle' ?_
        rw [hnext]
        exact hup
      · have hj : j = top := by omega
        subst hj
        exact top_blockedO sk ord hwf hfam h hfix htop hroot hup

/-- No assembler's OUTPUT is jammed either, O twin of `no_out_jam`:
one climb step in. -/
theorem no_out_jamO (hwf : sk.wellFormed = true)
    {P : List (List Ev)} (hfam : FamOKO sk ord P) (hman : ManRows sk P)
    {st : MState}
    (h : WCountP sk P [] st) (hfix : step sk st = none)
    (hroot : 1 ≤ sndCount Chan.rootres st.out)
    {p : Party} {top j : Nat}
    (htop : p = Party.I ∧ top = sk.rootH
      ∨ p = Party.R ∧ top = sk.rootH - 1)
    (h1 : 1 ≤ j) (hjt : j ≤ top)
    (hjam : rcvCount (sk.asmOutChan (p, j)) st.out
        + sk.cap (sk.asmOutChan (p, j))
      ≤ sndCount (sk.asmOutChan (p, j)) st.out) : False := by
  rcases Nat.lt_or_ge j top with hlt | hge
  · have hjr : j < sk.rootH := by
      rcases htop with ⟨-, ht⟩ | ⟨-, ht⟩ <;>
        have := (wf_rootH hwf).2 <;> omega
    have hout : sk.asmOutChan (p, j) = Chan.level p j :=
      asmOutChan_of_lt sk htop hlt
    have hnext : asmLevelChan (p, j + 1) = Chan.level p j := rfl
    rw [hout] at hjam
    have hle' : sndCount (asmLevelChan (p, j + 1)) st.out
        ≤ sk.pendsBefore p (j + 1)
            (sk.asmResList p (j + 1)).length := by
      rw [hnext, pends_total_prod hwf (by omega : 2 ≤ j + 1)
        (by omega : j + 1 - 1 < sk.rootH)]
      show sndCount (Chan.level p j) st.out
        ≤ (sk.asmResList p (j + 1 - 1)).length
      rw [show j + 1 - 1 = j from by omega]
      exact level_snd_leO sk ord hfam h htop h1 hlt
    refine chain_no_jamO sk ord hwf hfam hman h hfix hroot htop
      (top - (j + 1)) (j + 1) (by omega) (by omega) (by omega) hle' ?_
    rw [hnext]
    exact hjam
  · have hj : j = top := by omega
    subst hj
    exact top_blockedO sk ord hwf hfam h hfix htop hroot hjam

-- ==================================================== the tower drain

/-- One drain step, O twin of `asm_counts_step`: resolution feed
drained, level feed complete from below, no jam above — the trichotomy
collapses to exhaustion. -/
theorem asm_counts_stepO (hwf : sk.wellFormed = true)
    {P : List (List Ev)} (hfam : FamOKO sk ord P) (hman : ManRows sk P)
    {st : MState}
    (h : WCountP sk P [] st) (hfix : step sk st = none)
    (hroot : 1 ≤ sndCount Chan.rootres st.out)
    {p : Party} {top j : Nat}
    (htop : p = Party.I ∧ top = sk.rootH
      ∨ p = Party.R ∧ top = sk.rootH - 1)
    (h1 : 1 ≤ j) (hjt : j ≤ top)
    (hlvl : sk.pendsBefore p j (sk.asmResList p j).length
      ≤ sndCount (asmLevelChan (p, j)) st.out) :
    rcvCount (asmResChan (p, j)) st.out = (sk.asmResList p j).length
    ∧ rcvCount (asmLevelChan (p, j)) st.out
        = sk.pendsBefore p j (sk.asmResList p j).length
    ∧ sndCount (sk.asmOutChan (p, j)) st.out
        = (sk.asmResList p j).length := by
  have hres := asm_res_fullO sk ord hwf hfam hman h htop h1 hjt
  have hIdx := famOKO_asm_procs sk ord hfam htop h1 hjt
  rcases asm_stuckO sk ord hfam h hfix h1 hIdx with
    ⟨hr, hl, ho⟩ | ⟨hr, hl, ho, hsr⟩
      | ⟨hr, h1r, hlo, hl, ho, hsl⟩ | ⟨hr, h1r, hl, ho, hblk⟩
  · exact ⟨hr, hl, ho⟩
  · -- res-starved: the walks have drained
    omega
  · -- level-starved: the feed below is complete
    have hmono := pendsBefore_mono sk p j hr
    omega
  · -- out-blocked: no jam above
    exact (no_out_jamO sk ord hwf hfam hman h hfix hroot htop h1 hjt
      hblk).elim

/-- The absorber drains, O twin of `absorb_counts_full`: its feeds are
the drained walks, and its output cannot jam without blocking the
towers above. The starved arms of `absorb_stuckO` close by `omega`
with the `wirePhase` offsets symbolic — the contradiction lives on the
starved channel itself. -/
theorem absorb_counts_fullO (hwf : sk.wellFormed = true)
    {P : List (List Ev)} (hfam : FamOKO sk ord P) (hman : ManRows sk P)
    {st : MState}
    (h : WCountP sk P [] st) (hfix : step sk st = none)
    (hroot : 1 ≤ sndCount Chan.rootres st.out) :
    rcvCount (Chan.wire Party.R 0) st.out = sk.totalLeafReqs
    ∧ rcvCount Chan.leafRequests st.out = sk.totalLeafReqs
    ∧ sndCount (Chan.level Party.I 0) st.out = sk.totalLeafReqs := by
  have hge2 := (wf_rootH hwf).2
  have hw := wire0_fullO sk ord hwf hfam hman h
  have hl := leafreq_fullO sk ord hwf hfam hman h
  rcases absorb_stuckO sk ord hfam h hfix with
    ⟨h1, h2, h3⟩ | ⟨h1, h2, h3, h4⟩ | ⟨h1, h2, h3, h4⟩ | ⟨h1, h2, h3, h4⟩
  · exact ⟨h1, h2, h3⟩
  · -- wire-starved: the stage-0 walk has drained
    omega
  · -- request-starved: the stage-1 walk has drained
    omega
  · -- out-blocked: the jam would climb the initiator tower
    exfalso
    have hbase : asks Party.I 1 = false := rfl
    have htot : sk.pendsBefore Party.I 1
        (sk.asmResList Party.I 1).length = sk.totalLeafReqs :=
      pendsBefore_answerer_leaf hbase
    have hle : sndCount (asmLevelChan (Party.I, 1)) st.out
        ≤ sk.pendsBefore Party.I 1 (sk.asmResList Party.I 1).length := by
      rw [htot]
      exact level0_snd_leO sk ord hfam h
    exact chain_no_jamO sk ord hwf hfam hman h hfix hroot
      (Or.inl ⟨rfl, rfl⟩)
      (sk.rootH - 1) 1 (by omega) (by omega) (by omega) hle h4

/-- Tower drain, bottom-up, O twin of `asm_counts_full`: with the base
level feed complete, every assembler in the tower reaches its
totals. -/
theorem asm_counts_fullO (hwf : sk.wellFormed = true)
    {P : List (List Ev)} (hfam : FamOKO sk ord P) (hman : ManRows sk P)
    {st : MState}
    (h : WCountP sk P [] st) (hfix : step sk st = none)
    (hroot : 1 ≤ sndCount Chan.rootres st.out)
    {p : Party} {top : Nat}
    (htop : p = Party.I ∧ top = sk.rootH
      ∨ p = Party.R ∧ top = sk.rootH - 1)
    (hbase : sk.pendsBefore p 1 (sk.asmResList p 1).length
      ≤ sndCount (asmLevelChan (p, 1)) st.out) :
    ∀ j, 1 ≤ j → j ≤ top →
      rcvCount (asmResChan (p, j)) st.out = (sk.asmResList p j).length
      ∧ rcvCount (asmLevelChan (p, j)) st.out
          = sk.pendsBefore p j (sk.asmResList p j).length
      ∧ sndCount (sk.asmOutChan (p, j)) st.out
          = (sk.asmResList p j).length := by
  have hge2 := (wf_rootH hwf).2
  have main : ∀ m, m + 1 ≤ top →
      rcvCount (asmResChan (p, m + 1)) st.out
          = (sk.asmResList p (m + 1)).length
      ∧ rcvCount (asmLevelChan (p, m + 1)) st.out
          = sk.pendsBefore p (m + 1) (sk.asmResList p (m + 1)).length
      ∧ sndCount (sk.asmOutChan (p, m + 1)) st.out
          = (sk.asmResList p (m + 1)).length := by
    intro m
    induction m with
    | zero =>
        intro h1t
        exact asm_counts_stepO sk ord hwf hfam hman h hfix hroot htop
          (by omega) h1t hbase
    | succ m ihm =>
        intro ht
        have hprev := (ihm (by omega)).2.2
        have hlt : m + 1 < top := by omega
        have hjr : m + 1 < sk.rootH := by
          rcases htop with ⟨-, ht'⟩ | ⟨-, ht'⟩ <;> omega
        have hout : sk.asmOutChan (p, m + 1) = Chan.level p (m + 1) :=
          asmOutChan_of_lt sk htop hlt
        have hlvl : sk.pendsBefore p (m + 1 + 1)
            (sk.asmResList p (m + 1 + 1)).length
            ≤ sndCount (asmLevelChan (p, m + 1 + 1)) st.out := by
          rw [pends_total_prod hwf (by omega) (by omega)]
          show (sk.asmResList p (m + 1 + 1 - 1)).length
            ≤ sndCount (Chan.level p (m + 1)) st.out
          rw [show m + 1 + 1 - 1 = m + 1 from by omega, ← hout, hprev]
          exact Nat.le_refl _
        exact asm_counts_stepO sk ord hwf hfam hman h hfix hroot htop
          (by omega) ht hlvl
  intro j h1 hjt
  obtain ⟨m, rfl⟩ : ∃ m, j = m + 1 := ⟨j - 1, by omega⟩
  exact main m hjt

/-- The initiator tower's totals, base fed by the absorber; O twin of
`asmI_counts`. -/
theorem asmI_countsO (hwf : sk.wellFormed = true)
    {P : List (List Ev)} (hfam : FamOKO sk ord P) (hman : ManRows sk P)
    {st : MState}
    (h : WCountP sk P [] st) (hfix : step sk st = none)
    (hroot : 1 ≤ sndCount Chan.rootres st.out) :
    ∀ j, 1 ≤ j → j ≤ sk.rootH →
      rcvCount (asmResChan (Party.I, j)) st.out
          = (sk.asmResList Party.I j).length
      ∧ rcvCount (asmLevelChan (Party.I, j)) st.out
          = sk.pendsBefore Party.I j (sk.asmResList Party.I j).length
      ∧ sndCount (sk.asmOutChan (Party.I, j)) st.out
          = (sk.asmResList Party.I j).length := by
  have habs := absorb_counts_fullO sk ord hwf hfam hman h hfix hroot
  refine asm_counts_fullO sk ord hwf hfam hman h hfix hroot
    (Or.inl ⟨rfl, rfl⟩) ?_
  rw [pendsBefore_answerer_leaf rfl]
  show sk.totalLeafReqs ≤ sndCount (Chan.level Party.I 0) st.out
  rw [habs.2.2]
  exact Nat.le_refl _

/-- The responder tower's totals, O twin of `asmR_counts`: its phantom
base pends nothing. -/
theorem asmR_countsO (hwf : sk.wellFormed = true)
    {P : List (List Ev)} (hfam : FamOKO sk ord P) (hman : ManRows sk P)
    {st : MState}
    (h : WCountP sk P [] st) (hfix : step sk st = none)
    (hroot : 1 ≤ sndCount Chan.rootres st.out) :
    ∀ j, 1 ≤ j → j ≤ sk.rootH - 1 →
      rcvCount (asmResChan (Party.R, j)) st.out
          = (sk.asmResList Party.R j).length
      ∧ rcvCount (asmLevelChan (Party.R, j)) st.out
          = sk.pendsBefore Party.R j (sk.asmResList Party.R j).length
      ∧ sndCount (sk.asmOutChan (Party.R, j)) st.out
          = (sk.asmResList Party.R j).length := by
  refine asm_counts_fullO sk ord hwf hfam hman h hfix hroot
    (Or.inr ⟨rfl, rfl⟩) ?_
  rw [pendsBefore_asker_one hwf rfl]
  exact Nat.zero_le _

/-- The fins drain, O twin of `fin_counts_full`: the root resolution
arrives and every root return is consumed. -/
theorem fin_counts_fullO (hwf : sk.wellFormed = true)
    {P : List (List Ev)} (hfam : FamOKO sk ord P) (hman : ManRows sk P)
    {st : MState}
    (h : WCountP sk P [] st) (hfix : step sk st = none)
    (hroot : 1 ≤ sndCount Chan.rootres st.out) :
    rcvCount Chan.rootres st.out = 1
    ∧ rcvCount Chan.rootrets st.out = sk.rootPending := by
  have hge2 := (wf_rootH hwf).2
  have heven := (wf_rootH hwf).1
  rcases fin_stuckO sk ord hfam h hfix (by omega) with
    h1 | ⟨ha, hb, hc⟩ | ⟨ha, hb, hc⟩
  · exact h1
  · -- rootres-starved: ropen has drained
    have := rootres_fullO sk ord hfam hman h
    omega
  · -- rootrets-starved: the responder top has drained
    exfalso
    have hR := (asmR_countsO sk ord hwf hfam hman h hfix hroot
      (sk.rootH - 1) (by omega) (Nat.le_refl _)).2.2
    have hout : sk.asmOutChan (Party.R, sk.rootH - 1)
        = Chan.rootrets := by
      unfold Skel.asmOutChan
      rw [if_neg (by simp), if_pos (by simp)]
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
    rw [hout, asmResList_asker_length hasks, hpend] at hR
    omega

/-- The floating root return fires, O twin of `rootret_fired`. -/
theorem rootret_firedO (hwf : sk.wellFormed = true)
    {P : List (List Ev)} (hfam : FamOKO sk ord P) (hman : ManRows sk P)
    {st : MState}
    (h : WCountP sk P [] st) (hfix : step sk st = none)
    (hroot : 1 ≤ sndCount Chan.rootres st.out) :
    rcvCount Chan.rootret st.out = 1 := by
  have hge2 := (wf_rootH hwf).2
  have heven := (wf_rootH hwf).1
  rcases rootret_stuckO sk ord hfam h hfix (by omega) with h1 | ⟨h0, hs0⟩
  · exact h1
  · exfalso
    have hI := (asmI_countsO sk ord hwf hfam hman h hfix hroot sk.rootH
      (by omega) (Nat.le_refl _)).2.2
    have hout : sk.asmOutChan (Party.I, sk.rootH) = Chan.rootret := by
      unfold Skel.asmOutChan
      rw [if_pos (by simp)]
    have hasks : asks Party.I sk.rootH = true := by
      simp [asks, heven]
    have hlen1 : (sk.asmResList Party.I sk.rootH).length = 1 := by
      rw [asmResList_asker_length hasks, wf_root_stage hwf]
      rfl
    rw [hout, hlen1] at hI
    omega

-- ========================================== cells are literally empty

/-- A drained assembler's cell is empty, O twin of `asm_sublist`: the
trace is all out (the tower rows are order-independent — only the
bundle and the freshness law swap for their O twins). -/
theorem asm_sublistO {P : List (List Ev)} (hfam : FamOKO sk ord P)
    {st : MState}
    (h : WCountP sk P [] st) {p : Party} {top j : Nat}
    (htop : p = Party.I ∧ top = sk.rootH
      ∨ p = Party.R ∧ top = sk.rootH - 1)
    (h1 : 1 ≤ j) (hjt : j ≤ top)
    (hcnt : rcvCount (asmResChan (p, j)) st.out
        = (sk.asmResList p j).length
      ∧ rcvCount (asmLevelChan (p, j)) st.out
          = sk.pendsBefore p j (sk.asmResList p j).length
      ∧ sndCount (sk.asmOutChan (p, j)) st.out
          = (sk.asmResList p j).length) :
    (asmEvents sk (p, j)).Sublist st.out := by
  obtain ⟨hro, hlo, hoo⟩ := asm_owners sk p h1
  have hIdx := famOKO_asm_procs sk ord hfam htop h1 hjt
  obtain ⟨r, pre, hr, hpre, hsub⟩ := cell_of_owner sk h hIdx
  cases r with
  | nil =>
      rw [List.append_nil] at hpre
      exact hpre ▸ hsub
  | cons e₀ rest₀ =>
      exfalso
      obtain ⟨c₀, b₀, n₀⟩ := e₀
      have hmem : ((c₀, b₀, n₀) : Ev) ∈ asmEvents sk (p, j) := by
        rw [hpre]
        exact List.mem_append_right _ (List.mem_cons_self ..)
      unfold asmEvents at hmem
      obtain ⟨idx, hidx, he⟩ := List.mem_flatMap.1 hmem
      have hidxlt : idx < (sk.asmResList p j).length :=
        List.mem_range.1 hidx
      rw [asmBlock_eq] at he
      rcases he with _ | ⟨_, he⟩
      · -- the resolution receive: seq below the drained res total
        have hno := cell_not_outO sk ord hfam h (asmResChan (p, j)) false
          (by simpa using hro) hIdx hr hpre hsub
          (List.mem_cons_self ..)
        rw [← rcvCount_eq_proj, hcnt.1] at hno
        omega
      · rcases List.mem_append.1 he with he | he
        · -- a pending level receive: seq below the drained pends total
          obtain ⟨hc, hb, hlon, hhi⟩ := mem_seg he
          subst hc hb
          have hno := cell_not_outO sk ord hfam h (asmLevelChan (p, j))
            false (by simpa using hlo) hIdx hr hpre hsub
            (List.mem_cons_self ..)
          rw [← rcvCount_eq_proj, hcnt.2.1] at hno
          have hlon' : sk.pendsBefore p j idx ≤ n₀ := hlon
          have hhi' : n₀ < sk.pendsBefore p j idx + sk.pendAt p j idx :=
            hhi
          have hstep : sk.pendsBefore p j (idx + 1)
              = sk.pendsBefore p j idx + sk.pendAt p j idx :=
            pendsBefore_succ sk (by omega)
          have hmono : sk.pendsBefore p j (idx + 1)
              ≤ sk.pendsBefore p j (sk.asmResList p j).length :=
            pendsBefore_mono sk p j (by omega)
          omega
        · -- the output send: seq below the drained out total
          rcases he with _ | ⟨_, he⟩
          · have hno := cell_not_outO sk ord hfam h (sk.asmOutChan (p, j))
              true (by simpa using hoo) hIdx hr hpre hsub
              (List.mem_cons_self ..)
            rw [← sndCount_eq_proj, hcnt.2.2] at hno
            omega
          · cases he

/-- The drained O absorber's cell is empty, O twin of `absorb_sublist`:
the head's identity transfers through the block permutation (the
assignment only swaps the two receives inside a block, never their
seqs), so the three freshness clashes are the base ones. -/
theorem absorb_sublistO {P : List (List Ev)} (hfam : FamOKO sk ord P)
    {st : MState}
    (h : WCountP sk P [] st)
    (hcnt : rcvCount (Chan.wire Party.R 0) st.out = sk.totalLeafReqs
      ∧ rcvCount Chan.leafRequests st.out = sk.totalLeafReqs
      ∧ sndCount (Chan.level Party.I 0) st.out = sk.totalLeafReqs) :
    (absorbEventsO sk ord).Sublist st.out := by
  have hIdx := famOKO_absorb sk ord hfam
  obtain ⟨r, pre, hr, hpre, hsub⟩ := cell_of_owner sk h hIdx
  cases r with
  | nil =>
      rw [List.append_nil] at hpre
      exact hpre ▸ hsub
  | cons e₀ rest₀ =>
      exfalso
      have hmem : e₀ ∈ absorbEventsO sk ord := by
        rw [hpre]
        exact List.mem_append_right _ (List.mem_cons_self ..)
      unfold absorbEventsO at hmem
      obtain ⟨q, hq, he⟩ := List.mem_flatMap.1 hmem
      have hqlt := List.mem_range.1 hq
      have he' := (absorbBlockO_perm ord q).subset he
      rcases he' with _ | ⟨_, he'⟩
      · have hno := cell_not_outO sk ord hfam h (Chan.wire Party.R 0)
          false (by simp [rcvOwner]) hIdx hr hpre hsub
          (List.mem_cons_self ..)
        rw [← rcvCount_eq_proj, hcnt.1] at hno
        omega
      · rcases he' with _ | ⟨_, he'⟩
        · have hno := cell_not_outO sk ord hfam h Chan.leafRequests
            false (by simp [rcvOwner]) hIdx hr hpre hsub
            (List.mem_cons_self ..)
          rw [← rcvCount_eq_proj, hcnt.2.1] at hno
          omega
        · rcases he' with _ | ⟨_, he'⟩
          · have hno := cell_not_outO sk ord hfam h
              (Chan.level Party.I 0) true (by simp [sndOwner]) hIdx hr
              hpre hsub (List.mem_cons_self ..)
            rw [← sndCount_eq_proj, hcnt.2.2] at hno
            omega
          · cases he'

/-- The drained fins' cell is empty, O twin of `fin_sublist`. -/
theorem fin_sublistO {P : List (List Ev)} (hfam : FamOKO sk ord P)
    {st : MState}
    (h : WCountP sk P [] st) (hge : 1 ≤ sk.rootH)
    (hcnt : rcvCount Chan.rootres st.out = 1
      ∧ rcvCount Chan.rootrets st.out = sk.rootPending) :
    (finEvents sk).Sublist st.out := by
  have hIdx := famOKO_fin sk ord hfam hge
  obtain ⟨r, pre, hr, hpre, hsub⟩ := cell_of_owner sk h hIdx
  cases r with
  | nil =>
      rw [List.append_nil] at hpre
      exact hpre ▸ hsub
  | cons e₀ rest₀ =>
      exfalso
      have hmem : e₀ ∈ finEvents sk := by
        rw [hpre]
        exact List.mem_append_right _ (List.mem_cons_self ..)
      unfold finEvents at hmem
      rcases hmem with _ | ⟨_, he⟩
      · have hno := cell_not_outO sk ord hfam h Chan.rootres false
          (by simp [rcvOwner]) hIdx hr hpre hsub
          (List.mem_cons_self ..)
        rw [← rcvCount_eq_proj, hcnt.1] at hno
        omega
      · obtain ⟨q, hq, hqe⟩ := List.mem_map.1 he
        have hqlt := List.mem_range.1 hq
        subst hqe
        have hno := cell_not_outO sk ord hfam h Chan.rootrets false
          (by simp [rcvOwner]) hIdx hr hpre hsub
          (List.mem_cons_self ..)
        rw [← rcvCount_eq_proj, hcnt.2] at hno
        omega

/-- The fired floating root return's cell is empty, O twin of
`rootret_sublist`. -/
theorem rootret_sublistO {P : List (List Ev)} (hfam : FamOKO sk ord P)
    {st : MState}
    (h : WCountP sk P [] st) (hge : 1 ≤ sk.rootH)
    (hcnt : rcvCount Chan.rootret st.out = 1) :
    ([((Chan.rootret, false, 0) : Ev)]).Sublist st.out := by
  have hIdx := famOKO_rootret sk ord hfam hge
  obtain ⟨r, pre, hr, hpre, hsub⟩ := cell_of_owner sk h hIdx
  cases r with
  | nil =>
      rw [List.append_nil] at hpre
      exact hpre ▸ hsub
  | cons e₀ rest₀ =>
      exfalso
      have hmem : e₀ ∈ ([((Chan.rootret, false, 0) : Ev)]) := by
        rw [hpre]
        exact List.mem_append_right _ (List.mem_cons_self ..)
      rcases hmem with _ | ⟨_, he⟩
      · have hno := cell_not_outO sk ord hfam h Chan.rootret false
          (by simp [rcvOwner]) hIdx hr hpre hsub
          (List.mem_cons_self ..)
        rw [← rcvCount_eq_proj, hcnt] at hno
        omega
      · cases he

-- ============================================ the drained O weave state

/-- The O weave, run to the merge fixpoint: the class's potential
carrier at this assignment. -/
def wFinalO : MState := wPump sk (weaveStateO sk ord)

/-- Edge-respect survives the final pump. -/
theorem wfinalO_wedge (hwf : sk.wellFormed = true)
    (hm0 : ∀ s, sk.dCount s ≤ sk.capLevel) :
    WEdgeP sk (procsO sk ord) [] (wFinalO sk ord) :=
  wEdge_pump sk (weaveO_wedge sk ord hwf hm0)

/-- The final O weave state is a merge fixpoint. -/
theorem wfinalO_fix : step sk (wFinalO sk ord) = none :=
  wPump_fixpoint sk _

/-- EVERY O trace is a sublist of a drained-manual pump fixpoint's
output (cf. `all_sublist_finalE`; the case analysis is `procsO`'s,
with the absorber arm through the O drain). -/
theorem all_sublist_finalO (hwf : sk.wellFormed = true) {st : MState}
    (h : WCountP sk (procsO sk ord) [] st) (hfix : step sk st = none) :
    ∀ T ∈ procsO sk ord, T.Sublist st.out := by
  have hge2 := (wf_rootH hwf).2
  have hfam := famOKO_procsO sk ord hwf
  have hman := manRows_procsO sk ord
  have hroot : 1 ≤ sndCount Chan.rootres st.out := by
    have := rootres_fullO sk ord hfam hman h
    omega
  intro T hT
  simp only [procsO, List.mem_append, List.mem_cons,
    List.not_mem_nil, or_false, List.mem_map] at hT
  rcases hT with ((((rfl | rfl) | ⟨pk, hpk, rfl⟩) | rfl) | ⟨pk, hpk, rfl⟩)
    | rfl | rfl
  · -- iopen
    exact man_sublist sk h (M := 0) (by unfold manCount; omega) rfl
  · -- ropen
    exact man_sublist sk h (M := 1) (by unfold manCount; omega)
      (procsO_ropen sk ord)
  · -- a walk, in its assigned prologue order
    obtain ⟨i, hi, rfl⟩ := hpk
    have hilt : i < sk.rootH := List.mem_range.1 hi
    exact man_sublist sk h
      (M := walkIdx sk (sk.rootH - 1 - i))
      (by unfold walkIdx manCount; omega)
      (procsO_walk sk ord (by omega))
  · -- the absorber, in its assigned order
    exact absorb_sublistO sk ord hfam h
      (absorb_counts_fullO sk ord hwf hfam hman h hfix hroot)
  · -- an assembler
    unfold Skel.asmKeys at hpk
    rcases List.mem_append.1 hpk with hk | hk
    · obtain ⟨q, hq, rfl⟩ := List.mem_map.1 hk
      have hqlt : q < sk.rootH := List.mem_range.1 hq
      exact asm_sublistO sk ord hfam h (Or.inl ⟨rfl, rfl⟩) (by omega)
        (by omega)
        (asmI_countsO sk ord hwf hfam hman h hfix hroot (q + 1)
          (by omega) (by omega))
    · obtain ⟨q, hq, rfl⟩ := List.mem_map.1 hk
      have hqlt : q < sk.rootH - 1 := List.mem_range.1 hq
      exact asm_sublistO sk ord hfam h (Or.inr ⟨rfl, rfl⟩) (by omega)
        (by omega)
        (asmR_countsO sk ord hwf hfam hman h hfix hroot (q + 1)
          (by omega) (by omega))
  · -- the floating rootret receive
    exact rootret_sublistO sk ord hfam h (by omega)
      (rootret_firedO sk ord hwf hfam hman h hfix hroot)
  · -- fins
    exact fin_sublistO sk ord hfam h (by omega)
      (fin_counts_fullO sk ord hwf hfam hman h hfix hroot)

/-- THE O WEAVE IS TOTAL: every O trace rides inside the final O weave
output. -/
theorem all_sublist_wfinalO (hwf : sk.wellFormed = true)
    (hm0 : ∀ s, sk.dCount s ≤ sk.capLevel) :
    ∀ T ∈ procsO sk ord, T.Sublist (wFinalO sk ord).out :=
  all_sublist_finalO sk ord hwf
    (wfinalO_wedge sk ord hwf hm0).toWCountP (wfinalO_fix sk ord)

-- ================================================= canonical carrier

/-- O weave-state projections are canonical. -/
theorem wprojO_canon (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCountP sk (procsO sk ord) fut st) (c : Chan)
    (b : Bool) :
    proj c b st.out = canon c b (proj c b st.out).length := by
  refine wproj_canonP sk h c b ?_ (procsO_canon sk ord c b)
  cases b
  · exact procsO_rcv_owned sk ord hwf
  · exact procsO_snd_owned sk ord hwf

/-- The final O weave output carries each event at most once. -/
theorem wfinalO_count_le_one (hwf : sk.wellFormed = true)
    (hm0 : ∀ s, sk.dCount s ≤ sk.capLevel) :
    ∀ e : Ev, (wFinalO sk ord).out.count e ≤ 1 := by
  intro ⟨c, b, n⟩
  have hcanon := wprojO_canon sk ord hwf
    (wfinalO_wedge sk ord hwf hm0).toWCountP c b
  have hfilter : (wFinalO sk ord).out.count (c, b, n)
      = (proj c b (wFinalO sk ord).out).count (c, b, n) := by
    unfold proj
    exact (List.count_filter (by simp)).symm
  rw [hfilter, hcanon, count_canon]
  split <;> omega

/-- Freshness at the O merge: an event at or past its side's schedule
count was never emitted. -/
theorem not_mem_scheduleO_of_count (hwf : sk.wellFormed = true)
    {c : Chan} {b : Bool} {n : Nat}
    (hle : (proj c b (scheduleO sk ord)).length ≤ n) :
    ((c, b, n) : Ev) ∉ scheduleO sk ord := by
  intro hmem
  obtain ⟨ms, hms⟩ := scheduleO_proj_canon sk ord hwf c b
  have hmemp : ((c, b, n) : Ev) ∈ proj c b (scheduleO sk ord) :=
    List.mem_filter.2 ⟨hmem, by simp⟩
  have hlen : (proj c b (scheduleO sk ord)).length = ms := by
    rw [hms]
    unfold canon
    rw [List.length_map, List.length_range]
  rw [hms] at hmemp
  have := (mem_canon_lt hmemp).2.2
  omega

-- ================================================== the blame layer

/-- The blame step over the O weave: an unemitted protocol event sits
in its trace's final remainder, and that remainder's head sits at or
before it in the O weave. -/
theorem blame_headO (hwf : sk.wellFormed = true)
    (hm0 : ∀ s, sk.dCount s ≤ sk.capLevel) {g : Ev}
    (hgW : g ∈ (wFinalO sk ord).out) (hg_not : g ∉ scheduleO sk ord) :
    ∃ h', h' ∈ (finalStateO sk ord).rem.filterMap List.head?
      ∧ evIdx h' (wFinalO sk ord).out ≤ evIdx g (wFinalO sk ord).out := by
  obtain ⟨T', hT', hgT⟩ :=
    mem_some_trace sk (wfinalO_wedge sk ord hwf hm0).toWCountP hgW
  obtain ⟨r', hr'mem, pre', hpre', hsub'⟩ :=
    (trace_monotoneO sk ord).exists_of_mem_left hT'
  have hg_r : g ∈ r' := by
    rcases List.mem_append.1 (hpre' ▸ hgT) with hg | hg
    · exact absurd (hsub'.subset hg) hg_not
    · exact hg
  cases r' with
  | nil => cases hg_r
  | cons h' rest' =>
      refine ⟨h',
        List.mem_filterMap.2 ⟨h' :: rest', hr'mem, rfl⟩, ?_⟩
      rcases hg_r with _ | ⟨_, hg_r⟩
      · exact Nat.le_refl _
      · have hpair : ([h', g] : List Ev).Sublist (wFinalO sk ord).out := by
          have h1 : ([h', g] : List Ev).Sublist (h' :: rest') :=
            List.Sublist.cons_cons _ (List.singleton_sublist.2 hg_r)
          have h2 : (h' :: rest').Sublist T' :=
            hpre' ▸ List.sublist_append_right _ _
          exact (h1.trans h2).trans
            (all_sublist_wfinalO sk ord hwf hm0 T' hT')
        exact Nat.le_of_lt
          (pos_lt_of_pair (wfinalO_count_le_one sk ord hwf hm0) hpair)

-- ============================================== merge completeness

/-- MERGE COMPLETENESS at every assignment of the two-point class:
under well-formedness and margin 0 the O merge drains every trace —
the fixpoint's remainders are all empty. The stall refutation is
`merge_complete`'s, ranked by O weave position. -/
theorem merge_completeO (hwf : sk.wellFormed = true)
    (hm0 : ∀ s, sk.dCount s ≤ sk.capLevel) :
    ((finalStateO sk ord).rem.all List.isEmpty) = true := by
  by_contra hcon
  rw [Bool.not_eq_true, List.all_eq_false] at hcon
  obtain ⟨r₀, hr₀mem, hr₀ne⟩ := hcon
  -- shared facts
  have hge2 := (wf_rootH hwf).2
  have hwedge := wfinalO_wedge sk ord hwf hm0
  have hcnt1 := wfinalO_count_le_one sk ord hwf hm0
  have hsubW := all_sublist_wfinalO sk ord hwf hm0
  have hWcanon : ∀ (c' : Chan) (b' : Bool),
      proj c' b' (wFinalO sk ord).out
        = canon c' b' (proj c' b' (wFinalO sk ord).out).length :=
    fun c' b' => wprojO_canon sk ord hwf hwedge.toWCountP c' b'
  have hminv := scheduleO_inv sk ord
  have hfix : step sk (finalStateO sk ord) = none :=
    mergeN_fixpoint sk (totalEventsO sk ord)
      ⟨[], fun _ => 0, fun _ => 0, procsO sk ord⟩ (Nat.le_refl _)
  have hscan : scan sk (finalStateO sk ord).sent (finalStateO sk ord).rcvd
      (finalStateO sk ord).rem = none := by
    unfold step at hfix
    cases hs : scan sk (finalStateO sk ord).sent
        (finalStateO sk ord).rcvd (finalStateO sk ord).rem with
    | none => rfl
    | some pr => rw [hs] at hfix; simp at hfix
  -- prefix counts never exceed the whole
  have htake_le : ∀ (c' : Chan) (b' : Bool) (k : Nat),
      (proj c' b' ((wFinalO sk ord).out.take k)).length
        ≤ (proj c' b' (wFinalO sk ord).out).length :=
    fun c' b' k => ((List.take_sublist k _).filter _).length_le
  -- the O weave bounds each side's total by the other
  have hRS : ∀ c' : Chan, (proj c' false (wFinalO sk ord).out).length
      ≤ (proj c' true (wFinalO sk ord).out).length := by
    intro c'
    rcases Nat.eq_zero_or_pos
        (proj c' false (wFinalO sk ord).out).length with hz | hpos
    · omega
    · have hmem : ((c', false,
          (proj c' false (wFinalO sk ord).out).length - 1) : Ev)
          ∈ (wFinalO sk ord).out :=
        proj_mem_of_lt (hWcanon c' false) (by omega)
      obtain ⟨k, hk⟩ := List.mem_iff_getElem?.1 hmem
      have he1 := hwedge.e1_hist k c' _ hk
      rw [sndCount_eq_proj] at he1
      have := htake_le c' true k
      omega
  have hSR : ∀ c' : Chan, (proj c' true (wFinalO sk ord).out).length
      ≤ (proj c' false (wFinalO sk ord).out).length + sk.cap c' := by
    intro c'
    rcases Nat.eq_zero_or_pos
        (proj c' true (wFinalO sk ord).out).length with hz | hpos
    · omega
    · have hmem : ((c', true,
          (proj c' true (wFinalO sk ord).out).length - 1) : Ev)
          ∈ (wFinalO sk ord).out :=
        proj_mem_of_lt (hWcanon c' true) (by omega)
      obtain ⟨k, hk⟩ := List.mem_iff_getElem?.1 hmem
      have he2 := hwedge.e2_hist k c' _ hk
      rw [rcvCount_eq_proj] at he2
      have := htake_le c' false k
      omega
  -- the minimum stalled head
  have hne : (finalStateO sk ord).rem.filterMap List.head? ≠ [] := by
    cases hr₀ : r₀ with
    | nil => rw [hr₀] at hr₀ne; simp at hr₀ne
    | cons e0 rest0 =>
        intro hnil
        have hmem : e0 ∈ (finalStateO sk ord).rem.filterMap List.head? :=
          List.mem_filterMap.2 ⟨r₀, hr₀mem, by rw [hr₀]; rfl⟩
        rw [hnil] at hmem
        cases hmem
  obtain ⟨estar, hstar_mem, hmin⟩ :=
    exists_min_image (fun e => evIdx e (wFinalO sk ord).out) hne
  obtain ⟨rs, hrs_mem, hrs_head⟩ := List.mem_filterMap.1 hstar_mem
  obtain ⟨rest, hrs⟩ := List.head?_eq_some_iff.1 hrs_head
  obtain ⟨is, his⟩ := List.mem_iff_getElem?.1 hrs_mem
  have hdis : enabled sk (finalStateO sk ord).sent
      (finalStateO sk ord).rcvd estar = false :=
    scan_none_heads sk hscan (i := is) (by rw [his, hrs])
  obtain ⟨Tstar, hTstar, preM, hpreM, hsubM⟩ :=
    Forall2.exists_rel_left hminv.rem_struct his
  have hTstar_mem : Tstar ∈ procsO sk ord := List.mem_of_getElem? hTstar
  have hestar_W : estar ∈ (wFinalO sk ord).out := by
    refine (hsubW Tstar hTstar_mem).subset ?_
    rw [hpreM, hrs]
    exact List.mem_append_right _ (List.mem_cons_self ..)
  obtain ⟨c, b, n⟩ := estar
  cases b with
  | false =>
      -- STARVED RECEIVE: blame the send at the current count
      have hsent : (finalStateO sk ord).sent c
          = sndCount c (scheduleO sk ord) := hminv.sent_eq c
      simp only [enabled, decide_eq_false_iff_not, Nat.not_lt] at hdis
      have hstarve : sndCount c (scheduleO sk ord) ≤ n := by omega
      have hnW : n < (proj c false (wFinalO sk ord).out).length := by
        have hm : ((c, false, n) : Ev)
            ∈ proj c false (wFinalO sk ord).out :=
          List.mem_filter.2 ⟨hestar_W, by simp⟩
        rw [hWcanon c false] at hm
        exact (mem_canon_lt hm).2.2
      have hsW : sndCount c (scheduleO sk ord)
          < (proj c true (wFinalO sk ord).out).length := by
        have := hRS c
        omega
      have hgW : ((c, true, sndCount c (scheduleO sk ord)) : Ev)
          ∈ (wFinalO sk ord).out :=
        proj_mem_of_lt (hWcanon c true) hsW
      have hg_not : ((c, true, sndCount c (scheduleO sk ord)) : Ev)
          ∉ scheduleO sk ord :=
        not_mem_scheduleO_of_count sk ord hwf
          (Nat.le_of_eq (sndCount_eq_proj c _).symm)
      obtain ⟨h', hh'pool, hh'le⟩ :=
        blame_headO sk ord hwf hm0 hgW hg_not
      -- the receive at the current count is in the O weave
      have hrW : ((c, false, sndCount c (scheduleO sk ord)) : Ev)
          ∈ (wFinalO sk ord).out :=
        proj_mem_of_lt (hWcanon c false) (by omega)
      -- E1 in the O weave: the blocker precedes that receive
      have hk1 := evIdx_getElem? hrW
      have he1 := hwedge.e1_hist _ c (sndCount c (scheduleO sk ord)) hk1
      obtain ⟨j, hjlt, hjget⟩ := mem_take_snd (hWcanon c true) he1
      have hgle := evIdx_le hjget
      -- that receive is at or before the head in the O weave
      have hminstar := hmin h' hh'pool
      rcases Nat.eq_or_lt_of_le hstarve with heq | hlt2
      · rw [heq] at hgle hjlt hh'le
        omega
      · have hpair := pair_sublist_canon (c := c) (b := false)
          hlt2 hnW
        have hcanonsub : (canon c false
            (proj c false (wFinalO sk ord).out).length).Sublist
            (wFinalO sk ord).out := by
          rw [← hWcanon c false]
          exact List.filter_sublist
        have hordered := pos_lt_of_pair hcnt1 (hpair.trans hcanonsub)
        omega
  | true =>
      -- JAMMED SEND: blame the receive the cap window awaits
      have hrcvd : (finalStateO sk ord).rcvd c
          = rcvCount c (scheduleO sk ord) := hminv.rcvd_eq c
      simp only [enabled, decide_eq_false_iff_not, Nat.not_lt] at hdis
      have hjam : rcvCount c (scheduleO sk ord) + sk.cap c ≤ n := by omega
      have hnW : n < (proj c true (wFinalO sk ord).out).length := by
        have hm : ((c, true, n) : Ev)
            ∈ proj c true (wFinalO sk ord).out :=
          List.mem_filter.2 ⟨hestar_W, by simp⟩
        rw [hWcanon c true] at hm
        exact (mem_canon_lt hm).2.2
      have hrlt : rcvCount c (scheduleO sk ord)
          < (proj c false (wFinalO sk ord).out).length := by
        have := hSR c
        omega
      have hgW : ((c, false, rcvCount c (scheduleO sk ord)) : Ev)
          ∈ (wFinalO sk ord).out :=
        proj_mem_of_lt (hWcanon c false) hrlt
      have hg_not : ((c, false, rcvCount c (scheduleO sk ord)) : Ev)
          ∉ scheduleO sk ord :=
        not_mem_scheduleO_of_count sk ord hwf
          (Nat.le_of_eq (rcvCount_eq_proj c _).symm)
      obtain ⟨h', hh'pool, hh'le⟩ :=
        blame_headO sk ord hwf hm0 hgW hg_not
      -- E2 in the O weave at the jammed send's own position
      have hkstar := evIdx_getElem? hestar_W
      have he2 := hwedge.e2_hist _ c n hkstar
      obtain ⟨j, hjlt, hjget⟩ := mem_take_rcv (hWcanon c false)
        (k := evIdx ((c, true, n) : Ev) (wFinalO sk ord).out)
        (n := rcvCount c (scheduleO sk ord)) (by omega)
      have hgle := evIdx_le hjget
      have hminstar := hmin h' hh'pool
      omega

/-- Every O trace embeds in `scheduleO` in order, with the merge
completeness discharged: the corollary Ord/Pending's `hrem`-threaded
tau layer (`trace_sublistO`, `tau_le_of_pendO`) consumes through
`merge_completeO` at the endgame. -/
theorem trace_sublistO' (hwf : sk.wellFormed = true)
    (hm0 : ∀ s, sk.dCount s ≤ sk.capLevel) :
    ∀ T ∈ procsO sk ord, T.Sublist (scheduleO sk ord) := by
  intro T hT
  obtain ⟨r, hr, pre, hpre, hsub⟩ :=
    (trace_monotoneO sk ord).exists_of_mem_left hT
  have hempty : r = [] := by
    have := List.all_eq_true.1 (merge_completeO sk ord hwf hm0) r hr
    cases r with
    | nil => rfl
    | cons a l => simp at this
  rw [hempty, List.append_nil] at hpre
  exact hpre ▸ hsub

end StreamingMirror.Ord
