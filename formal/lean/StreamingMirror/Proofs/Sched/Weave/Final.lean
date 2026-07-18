/-
The drained weave (PROGRESS.md §7 3b, closing): the weave's final
state, pumped to the merge fixpoint, emits EVERY event — each trace of
`procs` is a sublist of its output. This is the totality the argmin
needs: the weave order is a full topological order of the event DAG,
so "position in the weave" is a potential defined on every event a
stalled merge could ever blame.

# Shape

`weave_wedge` gives edge-respect with an empty future, so every MANUAL
trace is already fully emitted (`man_proj_full`); what remains is that
the pump towers drain. That is a stall refutation at the pump fixpoint
(`wfinal_fix`), from the stuck trichotomies (`asm_stuck` &c.), in two
sweeps:

- No jam anywhere (`chain_no_jam`): a jam on an assembler's level feed
  forces — through its own trichotomy, whose starved arms clash with
  the walks' drained totals or with the jam itself — a jam on its own
  output, climbing until `top_blocked` kills it at the root returns.
- Upward drain (`asm_counts_full`): with the feed below complete and
  no jam above, each trichotomy collapses to its exhausted arm; the
  induction climbs from the absorber (whose feeds are the drained
  walks) through both towers to fins and the floating root return.

The cells are then literally empty (`cell_not_out` against the totals
per head shape), giving `all_sublist_wfinal`.
-/
import StreamingMirror.Proofs.Sched.Weave.Master

namespace StreamingMirror.Sched

open Model

variable (sk : Skel)

-- ============================================ the drained weave state

/-- The weave, run to the merge fixpoint: the potential's carrier. -/
def wFinal : MState := wPump sk (weaveState sk)

/-- Edge-respect survives the final pump. -/
theorem wfinal_wedge (hwf : sk.wellFormed = true)
    (hsched : sk.schedulable = true) : WEdge sk [] (wFinal sk) :=
  wEdge_pump sk (weave_wedge sk hwf hsched)

/-- The final weave state is a merge fixpoint. -/
theorem wfinal_fix : step sk (wFinal sk) = none :=
  wPump_fixpoint sk _

-- ======================================= manual traces are all out

/-- With no future left, every manual trace's projection is whole: the
output's channel-side projection IS the trace's. -/
theorem man_proj_full (hwf : sk.wellFormed = true) {st : MState}
    (h : WCount sk [] st) (c : Chan) (b : Bool) {M : Nat}
    (hM : (if b then sndOwner sk c else rcvOwner sk c) = M)
    (hMlt : M < manCount sk) {T : List Ev}
    (hT : (procs sk)[M]? = some T) :
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
  have hcore := out_proj_owner sk hwf h c b hM hT hr hpre hsub
  rw [hcore, hpre, List.append_nil]

/-- A walk-owned channel's send count at a drained-manual state is the
walk's whole-trace total. -/
theorem walk_count_full (hwf : sk.wellFormed = true) {st : MState}
    (h : WCount sk [] st) {hh : Nat} (hhr : hh < sk.rootH)
    (c : Chan) (hc : sndOwner sk c = walkIdx sk hh) :
    sndCount c st.out = (proj c true (walkEvents sk (wpk hh))).length := by
  have hMlt : walkIdx sk hh < manCount sk := by
    unfold walkIdx manCount
    omega
  rw [sndCount_eq_proj,
    man_proj_full sk hwf h c true (by simpa using hc) hMlt
      (procs_walk sk hhr)]

/-- The resolution feed of every assembler is fully sent once the
walks are drained: the walk totals meet the assembler's demand
exactly. -/
theorem asm_res_full (hwf : sk.wellFormed = true) {st : MState}
    (h : WCount sk [] st) {p : Party} {top j : Nat}
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
      have hcnt := walk_count_full sk hwf h (hh := j - 1) (by omega)
        (Chan.upper p (j - 1)) rfl
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
      have hcnt := walk_count_full sk hwf h (hh := j) hjlt
        (Chan.lower p j) rfl
      rw [asmResChan_answerer hask, hcnt, hlow, walk_lower_total,
        answerer_resList_total hwf hask h1 hjlt]
      show (canon _ _ _).length = _
      unfold canon
      rw [List.length_map, List.length_range]
      rfl

/-- The absorber's wire feed is fully sent once the walks drain. -/
theorem wire0_full (hwf : sk.wellFormed = true) {st : MState}
    (h : WCount sk [] st) :
    sndCount (Chan.wire Party.R 0) st.out = sk.totalLeafReqs := by
  have hge2 := (wf_rootH hwf).2
  have hw : Chan.wire Party.R 0 = wireOut (wpk 0) := rfl
  have hcnt := walk_count_full sk hwf h (hh := 0) (by omega)
    (Chan.wire Party.R 0)
    (by simp only [sndOwner]; rw [if_neg (by omega)])
  rw [hcnt, hw, walk_wire_total]
  show (canon _ _ _).length = _
  unfold canon
  rw [List.length_map, List.length_range]
  exact wiresBefore_full_leaf hwf

/-- The absorber's request feed is fully sent once the walks drain. -/
theorem leafreq_full (hwf : sk.wellFormed = true) {st : MState}
    (h : WCount sk [] st) :
    sndCount Chan.leafRequests st.out = sk.totalLeafReqs := by
  have hge2 := (wf_rootH hwf).2
  have hq : Chan.leafRequests = askedOut (wpk 1) := rfl
  have hcnt := walk_count_full sk hwf h (hh := 1) (by omega)
    Chan.leafRequests rfl
  rw [hcnt, hq, walk_asked_total]
  show (canon _ _ _).length = _
  unfold canon
  rw [List.length_map, List.length_range]
  exact qsBefore_full_leaf hwf

/-- The root resolution is sent once ropen drains. -/
theorem rootres_full (hwf : sk.wellFormed = true) {st : MState}
    (h : WCount sk [] st) :
    sndCount Chan.rootres st.out = 1 := by
  have hproj := man_proj_full sk hwf h Chan.rootres true
    (M := 1) rfl (by unfold manCount; omega) (procs_ropen sk)
  rw [sndCount_eq_proj, hproj]
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

/-- A jam on an assembler's level feed forces a jam on its output: the
stuck trichotomy's other arms clash with the drained resolution feed,
the feed totals, or the jam itself. -/
theorem level_jam_up (hwf : sk.wellFormed = true) {st : MState}
    (h : WCount sk [] st) (hfix : step sk st = none)
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
  have hres := asm_res_full sk hwf h htop h1 hjt
  have hIdx := asm_procs sk htop h1 hjt
  rcases asm_stuck sk hwf h hfix h1 hIdx with
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
fixpoint: the jam would climb the tower and block the root returns. -/
theorem chain_no_jam (hwf : sk.wellFormed = true) {st : MState}
    (h : WCount sk [] st) (hfix : step sk st = none)
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
      exact top_blocked sk hwf h hfix htop hroot
        (level_jam_up sk hwf h hfix htop h1 hjt hle hjam)
  | succ d ihd =>
      intro j h1 hjt hd hle hjam
      have hup := level_jam_up sk hwf h hfix htop h1 hjt hle hjam
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
          exact level_snd_le sk hwf h htop h1 hlt
        refine ihd (j + 1) (by omega) (by omega) (by omega) hle' ?_
        rw [hnext]
        exact hup
      · have hj : j = top := by omega
        subst hj
        exact top_blocked sk hwf h hfix htop hroot hup

/-- No assembler's OUTPUT is jammed either: one climb step in. -/
theorem no_out_jam (hwf : sk.wellFormed = true) {st : MState}
    (h : WCount sk [] st) (hfix : step sk st = none)
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
      exact level_snd_le sk hwf h htop h1 hlt
    refine chain_no_jam sk hwf h hfix hroot htop
      (top - (j + 1)) (j + 1) (by omega) (by omega) (by omega) hle' ?_
    rw [hnext]
    exact hjam
  · have hj : j = top := by omega
    subst hj
    exact top_blocked sk hwf h hfix htop hroot hjam

-- ==================================================== the tower drain

/-- One drain step: resolution feed drained, level feed complete from
below, no jam above — the trichotomy collapses to exhaustion. -/
theorem asm_counts_step (hwf : sk.wellFormed = true) {st : MState}
    (h : WCount sk [] st) (hfix : step sk st = none)
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
  have hres := asm_res_full sk hwf h htop h1 hjt
  have hIdx := asm_procs sk htop h1 hjt
  rcases asm_stuck sk hwf h hfix h1 hIdx with
    ⟨hr, hl, ho⟩ | ⟨hr, hl, ho, hsr⟩
      | ⟨hr, h1r, hlo, hl, ho, hsl⟩ | ⟨hr, h1r, hl, ho, hblk⟩
  · exact ⟨hr, hl, ho⟩
  · -- res-starved: the walks have drained
    omega
  · -- level-starved: the feed below is complete
    have hmono := pendsBefore_mono sk p j hr
    omega
  · -- out-blocked: no jam above
    exact (no_out_jam sk hwf h hfix hroot htop h1 hjt hblk).elim

/-- The absorber drains: its feeds are the drained walks, and its
output cannot jam without blocking the towers above. -/
theorem absorb_counts_full (hwf : sk.wellFormed = true) {st : MState}
    (h : WCount sk [] st) (hfix : step sk st = none)
    (hroot : 1 ≤ sndCount Chan.rootres st.out) :
    rcvCount (Chan.wire Party.R 0) st.out = sk.totalLeafReqs
    ∧ rcvCount Chan.leafRequests st.out = sk.totalLeafReqs
    ∧ sndCount (Chan.level Party.I 0) st.out = sk.totalLeafReqs := by
  have hge2 := (wf_rootH hwf).2
  have hw := wire0_full sk hwf h
  have hl := leafreq_full sk hwf h
  rcases absorb_stuck sk hwf h hfix with
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
      exact level0_snd_le sk hwf h
    exact chain_no_jam sk hwf h hfix hroot (Or.inl ⟨rfl, rfl⟩)
      (sk.rootH - 1) 1 (by omega) (by omega) (by omega) hle h4

/-- Tower drain, bottom-up: with the base level feed complete, every
assembler in the tower reaches its totals. -/
theorem asm_counts_full (hwf : sk.wellFormed = true) {st : MState}
    (h : WCount sk [] st) (hfix : step sk st = none)
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
        exact asm_counts_step sk hwf h hfix hroot htop (by omega) h1t
          hbase
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
        exact asm_counts_step sk hwf h hfix hroot htop (by omega) ht
          hlvl
  intro j h1 hjt
  obtain ⟨m, rfl⟩ : ∃ m, j = m + 1 := ⟨j - 1, by omega⟩
  exact main m hjt

/-- The initiator tower's totals, base fed by the absorber. -/
theorem asmI_counts (hwf : sk.wellFormed = true) {st : MState}
    (h : WCount sk [] st) (hfix : step sk st = none)
    (hroot : 1 ≤ sndCount Chan.rootres st.out) :
    ∀ j, 1 ≤ j → j ≤ sk.rootH →
      rcvCount (asmResChan (Party.I, j)) st.out
          = (sk.asmResList Party.I j).length
      ∧ rcvCount (asmLevelChan (Party.I, j)) st.out
          = sk.pendsBefore Party.I j (sk.asmResList Party.I j).length
      ∧ sndCount (sk.asmOutChan (Party.I, j)) st.out
          = (sk.asmResList Party.I j).length := by
  have habs := absorb_counts_full sk hwf h hfix hroot
  refine asm_counts_full sk hwf h hfix hroot (Or.inl ⟨rfl, rfl⟩) ?_
  rw [pendsBefore_answerer_leaf rfl]
  show sk.totalLeafReqs ≤ sndCount (Chan.level Party.I 0) st.out
  rw [habs.2.2]
  exact Nat.le_refl _

/-- The responder tower's totals: its phantom base pends nothing. -/
theorem asmR_counts (hwf : sk.wellFormed = true) {st : MState}
    (h : WCount sk [] st) (hfix : step sk st = none)
    (hroot : 1 ≤ sndCount Chan.rootres st.out) :
    ∀ j, 1 ≤ j → j ≤ sk.rootH - 1 →
      rcvCount (asmResChan (Party.R, j)) st.out
          = (sk.asmResList Party.R j).length
      ∧ rcvCount (asmLevelChan (Party.R, j)) st.out
          = sk.pendsBefore Party.R j (sk.asmResList Party.R j).length
      ∧ sndCount (sk.asmOutChan (Party.R, j)) st.out
          = (sk.asmResList Party.R j).length := by
  refine asm_counts_full sk hwf h hfix hroot (Or.inr ⟨rfl, rfl⟩) ?_
  rw [pendsBefore_asker_one hwf rfl]
  exact Nat.zero_le _

/-- The fins drain: the root resolution arrives and every root return
is consumed. -/
theorem fin_counts_full (hwf : sk.wellFormed = true) {st : MState}
    (h : WCount sk [] st) (hfix : step sk st = none)
    (hroot : 1 ≤ sndCount Chan.rootres st.out) :
    rcvCount Chan.rootres st.out = 1
    ∧ rcvCount Chan.rootrets st.out = sk.rootPending := by
  have hge2 := (wf_rootH hwf).2
  have heven := (wf_rootH hwf).1
  rcases fin_stuck sk hwf h hfix (by omega) with
    h1 | ⟨ha, hb, hc⟩ | ⟨ha, hb, hc⟩
  · exact h1
  · -- rootres-starved: ropen has drained
    omega
  · -- rootrets-starved: the responder top has drained
    exfalso
    have hR := (asmR_counts sk hwf h hfix hroot (sk.rootH - 1)
      (by omega) (Nat.le_refl _)).2.2
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

/-- The floating root return fires. -/
theorem rootret_fired (hwf : sk.wellFormed = true) {st : MState}
    (h : WCount sk [] st) (hfix : step sk st = none)
    (hroot : 1 ≤ sndCount Chan.rootres st.out) :
    rcvCount Chan.rootret st.out = 1 := by
  have hge2 := (wf_rootH hwf).2
  have heven := (wf_rootH hwf).1
  rcases rootret_stuck sk hwf h hfix (by omega) with h1 | ⟨h0, hs0⟩
  · exact h1
  · exfalso
    have hI := (asmI_counts sk hwf h hfix hroot sk.rootH (by omega)
      (Nat.le_refl _)).2.2
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

/-- Membership in a seg pins the channel, side, and seq window. -/
theorem mem_seg {c' : Chan} {b' : Bool} {n' : Nat} {c : Chan}
    {b : Bool} {lo n : Nat}
    (hm : ((c', b', n') : Ev) ∈ seg c b lo n) :
    c' = c ∧ b' = b ∧ lo ≤ n' ∧ n' < lo + n := by
  unfold seg at hm
  obtain ⟨t, ht, he⟩ := List.mem_map.1 hm
  have htl := List.mem_range.1 ht
  injection he with h1 h2
  injection h2 with h2a h2b
  exact ⟨h1.symm, h2a.symm, by omega, by omega⟩

/-- A manual trace at a drained-manual state is entirely out. -/
theorem man_sublist {st : MState} (h : WCount sk [] st) {M : Nat}
    (hMlt : M < manCount sk) {T : List Ev}
    (hT : (procs sk)[M]? = some T) : T.Sublist st.out := by
  refine wcount_done_man_sublist sk h T ?_
  have hTt : ((procs sk).take (manCount sk))[M]? = some T := by
    rw [List.getElem?_take, if_pos hMlt]
    exact hT
  exact List.mem_of_getElem? hTt

/-- A drained assembler's cell is empty: the trace is all out. -/
theorem asm_sublist (hwf : sk.wellFormed = true) {st : MState}
    (h : WCount sk [] st) {p : Party} {top j : Nat}
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
  have hIdx := asm_procs sk htop h1 hjt
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
        have hno := cell_not_out sk hwf h (asmResChan (p, j)) false
          (by simpa using hro) hIdx hr hpre hsub
          (List.mem_cons_self ..)
        rw [← rcvCount_eq_proj, hcnt.1] at hno
        omega
      · rcases List.mem_append.1 he with he | he
        · -- a pending level receive: seq below the drained pends total
          obtain ⟨hc, hb, hlon, hhi⟩ := mem_seg he
          subst hc hb
          have hno := cell_not_out sk hwf h (asmLevelChan (p, j)) false
            (by simpa using hlo) hIdx hr hpre hsub
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
          · have hno := cell_not_out sk hwf h (sk.asmOutChan (p, j))
              true (by simpa using hoo) hIdx hr hpre hsub
              (List.mem_cons_self ..)
            rw [← sndCount_eq_proj, hcnt.2.2] at hno
            omega
          · cases he

/-- The drained absorber's cell is empty. -/
theorem absorb_sublist (hwf : sk.wellFormed = true) {st : MState}
    (h : WCount sk [] st)
    (hcnt : rcvCount (Chan.wire Party.R 0) st.out = sk.totalLeafReqs
      ∧ rcvCount Chan.leafRequests st.out = sk.totalLeafReqs
      ∧ sndCount (Chan.level Party.I 0) st.out = sk.totalLeafReqs) :
    (absorbEvents sk).Sublist st.out := by
  have hIdx := procs_absorb sk
  obtain ⟨r, pre, hr, hpre, hsub⟩ := cell_of_owner sk h hIdx
  cases r with
  | nil =>
      rw [List.append_nil] at hpre
      exact hpre ▸ hsub
  | cons e₀ rest₀ =>
      exfalso
      have hmem : e₀ ∈ absorbEvents sk := by
        rw [hpre]
        exact List.mem_append_right _ (List.mem_cons_self ..)
      unfold absorbEvents at hmem
      obtain ⟨q, hq, he⟩ := List.mem_flatMap.1 hmem
      have hqlt := List.mem_range.1 hq
      rcases he with _ | ⟨_, he⟩
      · have hno := cell_not_out sk hwf h (Chan.wire Party.R 0) false
          (by simp [rcvOwner]) hIdx hr hpre hsub
          (List.mem_cons_self ..)
        rw [← rcvCount_eq_proj, hcnt.1] at hno
        omega
      · rcases he with _ | ⟨_, he⟩
        · have hno := cell_not_out sk hwf h Chan.leafRequests false
            (by simp [rcvOwner]) hIdx hr hpre hsub
            (List.mem_cons_self ..)
          rw [← rcvCount_eq_proj, hcnt.2.1] at hno
          omega
        · rcases he with _ | ⟨_, he⟩
          · have hno := cell_not_out sk hwf h (Chan.level Party.I 0)
              true (by simp [sndOwner]) hIdx hr hpre hsub
              (List.mem_cons_self ..)
            rw [← sndCount_eq_proj, hcnt.2.2] at hno
            omega
          · cases he

/-- The drained fins' cell is empty. -/
theorem fin_sublist (hwf : sk.wellFormed = true) {st : MState}
    (h : WCount sk [] st) (hge : 1 ≤ sk.rootH)
    (hcnt : rcvCount Chan.rootres st.out = 1
      ∧ rcvCount Chan.rootrets st.out = sk.rootPending) :
    (finEvents sk).Sublist st.out := by
  have hIdx := procs_fin sk hge
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
      · have hno := cell_not_out sk hwf h Chan.rootres false
          (by simp [rcvOwner]) hIdx hr hpre hsub
          (List.mem_cons_self ..)
        rw [← rcvCount_eq_proj, hcnt.1] at hno
        omega
      · obtain ⟨q, hq, hqe⟩ := List.mem_map.1 he
        have hqlt := List.mem_range.1 hq
        subst hqe
        have hno := cell_not_out sk hwf h Chan.rootrets false
          (by simp [rcvOwner]) hIdx hr hpre hsub
          (List.mem_cons_self ..)
        rw [← rcvCount_eq_proj, hcnt.2] at hno
        omega

/-- The fired floating root return's cell is empty. -/
theorem rootret_sublist (hwf : sk.wellFormed = true) {st : MState}
    (h : WCount sk [] st) (hge : 1 ≤ sk.rootH)
    (hcnt : rcvCount Chan.rootret st.out = 1) :
    ([((Chan.rootret, false, 0) : Ev)]).Sublist st.out := by
  have hIdx := procs_rootret sk hge
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
      · have hno := cell_not_out sk hwf h Chan.rootret false
          (by simp [rcvOwner]) hIdx hr hpre hsub
          (List.mem_cons_self ..)
        rw [← rcvCount_eq_proj, hcnt] at hno
        omega
      · cases he

-- ============================================= the full permutation

/-- EVERY trace is a sublist of a drained-manual pump fixpoint's
output: the openers and walks because the future is spent, the pumps
by the drain induction. -/
theorem all_sublist_final (hwf : sk.wellFormed = true) {st : MState}
    (h : WCount sk [] st) (hfix : step sk st = none) :
    ∀ T ∈ procs sk, T.Sublist st.out := by
  have hge2 := (wf_rootH hwf).2
  have hroot : 1 ≤ sndCount Chan.rootres st.out := by
    have := rootres_full sk hwf h
    omega
  intro T hT
  simp only [procs, List.mem_append, List.mem_cons,
    List.not_mem_nil, or_false, List.mem_map] at hT
  rcases hT with ((((rfl | rfl) | ⟨pk, hpk, rfl⟩) | rfl) | ⟨pk, hpk, rfl⟩)
    | rfl | rfl
  · -- iopen
    exact man_sublist sk h (M := 0) (by unfold manCount; omega) rfl
  · -- ropen
    exact man_sublist sk h (M := 1) (by unfold manCount; omega)
      (procs_ropen sk)
  · -- a walk
    obtain ⟨i, hi, rfl⟩ := hpk
    have hilt : i < sk.rootH := List.mem_range.1 hi
    exact man_sublist sk h
      (M := walkIdx sk (sk.rootH - 1 - i))
      (by unfold walkIdx manCount; omega)
      (procs_walk sk (by omega))
  · -- absorb
    exact absorb_sublist sk hwf h
      (absorb_counts_full sk hwf h hfix hroot)
  · -- an assembler
    unfold Skel.asmKeys at hpk
    rcases List.mem_append.1 hpk with hk | hk
    · obtain ⟨q, hq, rfl⟩ := List.mem_map.1 hk
      have hqlt : q < sk.rootH := List.mem_range.1 hq
      exact asm_sublist sk hwf h (Or.inl ⟨rfl, rfl⟩) (by omega)
        (by omega)
        (asmI_counts sk hwf h hfix hroot (q + 1) (by omega) (by omega))
    · obtain ⟨q, hq, rfl⟩ := List.mem_map.1 hk
      have hqlt : q < sk.rootH - 1 := List.mem_range.1 hq
      exact asm_sublist sk hwf h (Or.inr ⟨rfl, rfl⟩) (by omega)
        (by omega)
        (asmR_counts sk hwf h hfix hroot (q + 1) (by omega) (by omega))
  · -- the floating rootret receive
    exact rootret_sublist sk hwf h (by omega)
      (rootret_fired sk hwf h hfix hroot)
  · -- fins
    exact fin_sublist sk hwf h (by omega)
      (fin_counts_full sk hwf h hfix hroot)

/-- THE WEAVE IS TOTAL: every trace of `procs` rides inside the final
weave output — the completeness witness's carrier holds every event of
the protocol. -/
theorem all_sublist_wfinal (hwf : sk.wellFormed = true)
    (hsched : sk.schedulable = true) :
    ∀ T ∈ procs sk, T.Sublist (wFinal sk).out :=
  all_sublist_final sk hwf
    (wfinal_wedge sk hwf hsched).toWCount (wfinal_fix sk)
