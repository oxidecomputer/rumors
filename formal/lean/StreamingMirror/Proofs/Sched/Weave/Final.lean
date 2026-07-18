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

-- ================================================ weave positions (φ)

/-- First position of an event in a list (length if absent): the
argmin's potential is `evIdx` into the final weave output. -/
def evIdx (x : Ev) : List Ev → Nat
  | [] => 0
  | e :: l => if e = x then 0 else evIdx x l + 1

/-- A present event is found at its position. -/
theorem evIdx_getElem? {x : Ev} :
    ∀ {l : List Ev}, x ∈ l → l[evIdx x l]? = some x := by
  intro l
  induction l with
  | nil => intro hx; cases hx
  | cons e l ihl =>
      intro hx
      unfold evIdx
      by_cases he : e = x
      · rw [if_pos he, he]
        rfl
      · rw [if_neg he]
        have hx' : x ∈ l := by
          rcases hx with _ | ⟨_, hx'⟩
          · exact absurd rfl he
          · exact hx'
        simpa using ihl hx'

/-- `evIdx` finds the FIRST occurrence: no position is earlier. -/
theorem evIdx_le {x : Ev} :
    ∀ {l : List Ev} {i : Nat}, l[i]? = some x → evIdx x l ≤ i := by
  intro l
  induction l with
  | nil => intro i hi; simp at hi
  | cons e l ihl =>
      intro i hi
      unfold evIdx
      by_cases he : e = x
      · rw [if_pos he]
        omega
      · rw [if_neg he]
        match i with
        | 0 =>
            simp only [List.getElem?_cons_zero, Option.some.injEq] at hi
            exact absurd hi he
        | i + 1 =>
            simp only [List.getElem?_cons_succ] at hi
            have := ihl hi
            omega

/-- With no duplicates, `evIdx` is THE position. -/
theorem evIdx_unique {x : Ev} :
    ∀ {l : List Ev} {i : Nat}, l.count x ≤ 1 → l[i]? = some x →
      i = evIdx x l := by
  intro l
  induction l with
  | nil => intro i _ hi; simp at hi
  | cons e l ihl =>
      intro i hcnt hi
      unfold evIdx
      by_cases he : e = x
      · rw [if_pos he]
        match i with
        | 0 => rfl
        | i + 1 =>
            exfalso
            simp only [List.getElem?_cons_succ] at hi
            have hxl : x ∈ l := List.mem_of_getElem? hi
            have h1 : 1 ≤ l.count x := List.one_le_count_iff.2 hxl
            rw [List.count_cons, if_pos (by simp [he])] at hcnt
            omega
      · rw [if_neg he]
        match i with
        | 0 =>
            simp only [List.getElem?_cons_zero, Option.some.injEq] at hi
            exact absurd hi he
        | i + 1 =>
            simp only [List.getElem?_cons_succ] at hi
            have hcnt' : l.count x ≤ 1 := by
              rw [List.count_cons] at hcnt
              omega
            have := ihl hcnt' hi
            omega

/-- An embedded pair occupies ordered positions. -/
theorem pair_sublist_pos {a b : Ev} :
    ∀ {W : List Ev}, ([a, b]).Sublist W →
      ∃ i j : Nat, i < j ∧ W[i]? = some a ∧ W[j]? = some b := by
  intro W
  induction W with
  | nil => intro hs; cases hs
  | cons w W ihW =>
      intro hs
      cases hs with
      | cons _ hs' =>
          obtain ⟨i, j, hij, hi, hj⟩ := ihW hs'
          exact ⟨i + 1, j + 1, by omega, by simpa using hi,
            by simpa using hj⟩
      | cons_cons _ hs' =>
          have hb : b ∈ W := (List.singleton_sublist.1 hs')
          obtain ⟨j, hj⟩ := List.mem_iff_getElem?.1 hb
          exact ⟨0, j + 1, by omega, rfl, by simpa using hj⟩

/-- Duplicate-free carrier: an embedded pair's `evIdx`es are strictly
ordered. -/
theorem pos_lt_of_pair {W : List Ev} (hcnt : ∀ e : Ev, W.count e ≤ 1)
    {a b : Ev} (hs : ([a, b]).Sublist W) : evIdx a W < evIdx b W := by
  obtain ⟨i, j, hij, hi, hj⟩ := pair_sublist_pos hs
  rw [← evIdx_unique (hcnt a) hi, ← evIdx_unique (hcnt b) hj]
  exact hij

/-- The final weave output carries each event at most once: its
projections are canonical, and a canonical stream is duplicate-free. -/
theorem wfinal_count_le_one (hwf : sk.wellFormed = true)
    (hsched : sk.schedulable = true) :
    ∀ e : Ev, (wFinal sk).out.count e ≤ 1 := by
  intro ⟨c, b, n⟩
  have hcanon := wproj_canon sk hwf
    (wfinal_wedge sk hwf hsched).toWCount c b
  have hfilter : (wFinal sk).out.count (c, b, n)
      = (proj c b (wFinal sk).out).count (c, b, n) := by
    unfold proj
    exact (List.count_filter (by simp)).symm
  rw [hfilter, hcanon, count_canon]
  split <;> omega

-- ============================================== canonical membership

/-- Membership in a canon stream pins the fields and bounds the seq. -/
theorem mem_canon_lt {c' : Chan} {b' : Bool} {n' : Nat} {c : Chan}
    {b : Bool} {m : Nat} (hm : ((c', b', n') : Ev) ∈ canon c b m) :
    c' = c ∧ b' = b ∧ n' < m := by
  unfold canon at hm
  obtain ⟨t, ht, he⟩ := List.mem_map.1 hm
  have htl := List.mem_range.1 ht
  injection he with h1 h2
  injection h2 with h2a h2b
  exact ⟨h1.symm, h2a.symm, by omega⟩

/-- Every seq below the total is present in the canon stream. -/
theorem canon_mem {c : Chan} {b : Bool} {n m : Nat} (h : n < m) :
    ((c, b, n) : Ev) ∈ canon c b m := by
  unfold canon
  exact List.mem_map.2 ⟨n, List.mem_range.2 h, rfl⟩

/-- A seg's head-cons unrolling. -/
theorem seg_cons (c : Chan) (b : Bool) (lo k : Nat) :
    seg c b lo (k + 1) = (c, b, lo) :: seg c b (lo + 1) k := by
  unfold seg
  rw [List.range_succ_eq_map, List.map_cons, List.map_map]
  refine congrArg₂ List.cons (by rw [Nat.add_zero]) ?_
  apply List.map_congr_left
  intro t _
  show (c, b, lo + Nat.succ t) = (c, b, lo + 1 + t)
  rw [Nat.add_succ, Nat.succ_add]

/-- Two seqs in order embed as an ordered pair in the canon stream. -/
theorem pair_sublist_canon {c : Chan} {b : Bool} {m n M : Nat}
    (hmn : m < n) (hnM : n < M) :
    ([((c, b, m) : Ev), (c, b, n)]).Sublist (canon c b M) := by
  have hmem : ((c, b, n) : Ev) ∈ seg c b (m + 1) (M - m - 1) := by
    unfold seg
    exact List.mem_map.2 ⟨n - (m + 1), List.mem_range.2 (by omega),
      by rw [show m + 1 + (n - (m + 1)) = n from by omega]⟩
  have hsl : ([((c, b, m) : Ev), (c, b, n)]).Sublist
      (seg c b m (M - m)) := by
    rw [show M - m = (M - m - 1) + 1 from by omega, seg_cons]
    exact List.Sublist.cons_cons _ (List.singleton_sublist.2 hmem)
  have hsplit : canon c b M = seg c b 0 m ++ seg c b m (M - m) := by
    rw [canon_eq_seg]
    have hs := seg_append c b 0 m (M - m)
    rw [show m + (M - m) = M from by omega, Nat.zero_add] at hs
    exact hs.symm
  rw [hsplit]
  exact hsl.trans (List.sublist_append_right _ _)

-- ======================================== positional count reading

/-- If a prefix's side count exceeds `n`, the seq-`n` event sits inside
the prefix: canonical streams are read off by counts. -/
theorem mem_take_of_count {W : List Ev} {c : Chan} {b : Bool}
    (hcanon : proj c b W = canon c b (proj c b W).length)
    {k n : Nat} (hn : n < (proj c b (W.take k)).length) :
    ∃ j, j < k ∧ W[j]? = some (c, b, n) := by
  have hpref : proj c b (W.take k) <+: proj c b W :=
    (List.take_prefix k W).filter _
  have hcp : proj c b (W.take k)
      = canon c b (proj c b (W.take k)).length := by
    refine prefix_canon (m := (proj c b W).length) ?_
    rw [← hcanon]
    exact hpref
  have hmem : ((c, b, n) : Ev) ∈ proj c b (W.take k) := by
    rw [hcp]
    exact canon_mem hn
  have hmemt : ((c, b, n) : Ev) ∈ W.take k :=
    (List.mem_filter.1 hmem).1
  obtain ⟨j, hj⟩ := List.mem_iff_getElem?.1 hmemt
  have hjl : j < (W.take k).length :=
    (List.getElem?_eq_some_iff.1 hj).1
  have hjk : j < k := by
    rw [List.length_take] at hjl
    omega
  refine ⟨j, hjk, ?_⟩
  rw [List.getElem?_take, if_pos hjk] at hj
  exact hj

/-- `mem_take_of_count`, read through the send counter. -/
theorem mem_take_snd {W : List Ev} {c : Chan}
    (hcanon : proj c true W = canon c true (proj c true W).length)
    {k n : Nat} (hn : n < sndCount c (W.take k)) :
    ∃ j, j < k ∧ W[j]? = some (c, true, n) :=
  mem_take_of_count hcanon (sndCount_eq_proj c (W.take k) ▸ hn)

/-- `mem_take_of_count`, read through the receive counter. -/
theorem mem_take_rcv {W : List Ev} {c : Chan}
    (hcanon : proj c false W = canon c false (proj c false W).length)
    {k n : Nat} (hn : n < rcvCount c (W.take k)) :
    ∃ j, j < k ∧ W[j]? = some (c, false, n) :=
  mem_take_of_count hcanon (rcvCount_eq_proj c (W.take k) ▸ hn)

-- ============================================= trace-search helpers

/-- A positive emitted count names an emitting trace. -/
theorem emittedCount_pos {q : Ev → Bool} :
    ∀ {ts rs : List (List Ev)}, 1 ≤ emittedCount q ts rs →
      ∃ t ∈ ts, ∃ e ∈ t, q e = true := by
  intro ts
  induction ts with
  | nil => intro rs h; cases rs <;> simp [emittedCount] at h
  | cons t ts iht =>
      intro rs h
      cases rs with
      | nil => simp [emittedCount] at h
      | cons r rs =>
          have hsplit : emittedCount q (t :: ts) (r :: rs)
              = ((t.take (t.length - r.length)).filter q).length
                + emittedCount q ts rs := rfl
          rw [hsplit] at h
          rcases Nat.eq_zero_or_pos
              (((t.take (t.length - r.length)).filter q).length) with
            hz | hpos
          · rw [hz, Nat.zero_add] at h
            obtain ⟨t', ht', e, he, hq⟩ := iht h
            exact ⟨t', List.mem_cons_of_mem _ ht', e, he, hq⟩
          · cases hf : (t.take (t.length - r.length)).filter q with
            | nil => rw [hf] at hpos; simp at hpos
            | cons e es =>
                have hmem : e ∈ (t.take (t.length - r.length)).filter q := by
                  rw [hf]
                  exact List.mem_cons_self ..
                obtain ⟨hmt, hq⟩ := List.mem_filter.1 hmem
                exact ⟨t, List.mem_cons_self .., e,
                  List.take_subset _ _ hmt, hq⟩

/-- Every emitted event belongs to some trace. -/
theorem mem_some_trace {fut : List Ev} {st : MState}
    (h : WCount sk fut st) {e : Ev} (he : e ∈ st.out) :
    ∃ T ∈ procs sk, e ∈ T := by
  have hcnt := wcount_out_glued sk h (fun x => x == e)
  have hpos : 1 ≤ emittedCount (fun x => x == e) (procs sk)
      (manFilters sk fut ++ st.rem) := by
    rw [← hcnt]
    have hm : e ∈ st.out.filter (fun x => x == e) :=
      List.mem_filter.2 ⟨he, by simp⟩
    have := List.length_pos_of_mem hm
    omega
  obtain ⟨T, hT, e', he', hbeq⟩ := emittedCount_pos hpos
  have : e' = e := by simpa using hbeq
  exact ⟨T, hT, this ▸ he'⟩

/-- Minimum of a Nat image over a non-empty list. -/
theorem exists_min_image {α : Type _} (f : α → Nat) :
    ∀ {l : List α}, l ≠ [] → ∃ a ∈ l, ∀ b ∈ l, f a ≤ f b := by
  intro l
  induction l with
  | nil => intro h; exact absurd rfl h
  | cons a l ihl =>
      intro _
      cases l with
      | nil =>
          refine ⟨a, List.mem_cons_self .., fun b hb => ?_⟩
          rcases hb with _ | ⟨_, hb⟩
          · exact Nat.le_refl _
          · cases hb
      | cons c l' =>
          obtain ⟨m, hm, hmin⟩ := ihl (by simp)
          by_cases hf : f a ≤ f m
          · refine ⟨a, List.mem_cons_self .., fun b hb => ?_⟩
            rcases hb with _ | ⟨_, hb⟩
            · exact Nat.le_refl _
            · exact Nat.le_trans hf (hmin b hb)
          · refine ⟨m, List.mem_cons_of_mem _ hm, fun b hb => ?_⟩
            rcases hb with _ | ⟨_, hb⟩
            · omega
            · exact hmin b hb

/-- Mirror of `Forall2.exists_rel_right`: a right member at an index
names its left partner. -/
theorem Forall2.exists_rel_left {α β : Type _} {R : α → β → Prop} :
    ∀ {la : List α} {lb : List β}, Forall2 R la lb →
      ∀ {i : Nat} {b : β}, lb[i]? = some b →
        ∃ a, la[i]? = some a ∧ R a b
  | _, _, .nil, i, _, hb => by simp at hb
  | _, _, .cons hab t, i, b, hb => by
      match i with
      | 0 =>
          simp only [List.getElem?_cons_zero, Option.some.injEq] at hb
          subst hb
          exact ⟨_, rfl, hab⟩
      | i + 1 =>
          simp only [List.getElem?_cons_succ] at hb
          obtain ⟨a, ha, hr⟩ := Forall2.exists_rel_left t hb
          exact ⟨a, by simpa using ha, hr⟩

-- ================================================== the blame layer

/-- A seq below its side's projection total is present in the list. -/
theorem proj_mem_of_lt {W : List Ev} {c : Chan} {b : Bool}
    (hcanon : proj c b W = canon c b (proj c b W).length)
    {n : Nat} (hn : n < (proj c b W).length) : ((c, b, n) : Ev) ∈ W := by
  have hm : ((c, b, n) : Ev) ∈ canon c b (proj c b W).length :=
    canon_mem hn
  rw [← hcanon] at hm
  exact (List.mem_filter.1 hm).1

/-- Freshness at the merge: an event at or past its side's schedule
count was never emitted. -/
theorem not_mem_schedule_of_count (hwf : sk.wellFormed = true)
    {c : Chan} {b : Bool} {n : Nat}
    (hle : (proj c b (schedule sk)).length ≤ n) :
    ((c, b, n) : Ev) ∉ schedule sk := by
  intro hmem
  obtain ⟨ms, hms⟩ := schedule_proj_canon sk hwf c b
  have hmemp : ((c, b, n) : Ev) ∈ proj c b (schedule sk) :=
    List.mem_filter.2 ⟨hmem, by simp⟩
  have hlen : (proj c b (schedule sk)).length = ms := by
    rw [hms]
    unfold canon
    rw [List.length_map, List.length_range]
  rw [hms] at hmemp
  have := (mem_canon_lt hmemp).2.2
  omega

/-- The blame step: an unemitted protocol event sits in its trace's
final remainder, and that remainder's head — a stalled head — sits at
or before it in the weave. -/
theorem blame_head (hwf : sk.wellFormed = true)
    (hsched : sk.schedulable = true) {g : Ev}
    (hgW : g ∈ (wFinal sk).out) (hg_not : g ∉ schedule sk) :
    ∃ h', h' ∈ (finalState sk).rem.filterMap List.head?
      ∧ evIdx h' (wFinal sk).out ≤ evIdx g (wFinal sk).out := by
  obtain ⟨T', hT', hgT⟩ :=
    mem_some_trace sk (wfinal_wedge sk hwf hsched).toWCount hgW
  obtain ⟨r', hr'mem, pre', hpre', hsub'⟩ :=
    (trace_monotone sk).exists_of_mem_left hT'
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
      · have hpair : ([h', g] : List Ev).Sublist (wFinal sk).out := by
          have h1 : ([h', g] : List Ev).Sublist (h' :: rest') :=
            List.Sublist.cons_cons _ (List.singleton_sublist.2 hg_r)
          have h2 : (h' :: rest').Sublist T' :=
            hpre' ▸ List.sublist_append_right _ _
          exact (h1.trans h2).trans
            (all_sublist_wfinal sk hwf hsched T' hT')
        exact Nat.le_of_lt
          (pos_lt_of_pair (wfinal_count_le_one sk hwf hsched) hpair)

-- ============================================== merge completeness

/-- MERGE COMPLETENESS (§7 3b, closed): under well-formedness and
schedulability the merge drains every trace — the fixpoint's
remainders are all empty.

The stall refutation: were any remainder non-empty, every non-empty
remainder's head is disabled at the fixpoint. Rank the heads by weave
position (`evIdx` into the drained weave — a full topological order by
`all_sublist_wfinal`) and look at the minimum. A starved receive
blames the send with its own count as seq; a jammed send blames the
receive its cap window awaits. Either blocker exists (the weave's own
edge-respect bounds each side's total by the other), is unemitted
(canonical freshness), and so sits in ITS trace's remainder at or
after that trace's head — a head strictly below the minimum in weave
order, by E1/E2 in the weave. Contradiction. -/
theorem merge_complete (hwf : sk.wellFormed = true)
    (hsched : sk.schedulable = true) :
    ((finalState sk).rem.all List.isEmpty) = true := by
  by_contra hcon
  rw [Bool.not_eq_true, List.all_eq_false] at hcon
  obtain ⟨r₀, hr₀mem, hr₀ne⟩ := hcon
  -- shared facts
  have hge2 := (wf_rootH hwf).2
  have hwedge := wfinal_wedge sk hwf hsched
  have hcnt1 := wfinal_count_le_one sk hwf hsched
  have hsubW := all_sublist_wfinal sk hwf hsched
  have hWcanon : ∀ (c' : Chan) (b' : Bool),
      proj c' b' (wFinal sk).out
        = canon c' b' (proj c' b' (wFinal sk).out).length :=
    fun c' b' => wproj_canon sk hwf hwedge.toWCount c' b'
  have hminv := schedule_inv sk
  have hfix : step sk (finalState sk) = none :=
    mergeN_fixpoint sk (totalEvents sk)
      ⟨[], fun _ => 0, fun _ => 0, procs sk⟩ (Nat.le_refl _)
  have hscan : scan sk (finalState sk).sent (finalState sk).rcvd
      (finalState sk).rem = none := by
    unfold step at hfix
    cases hs : scan sk (finalState sk).sent (finalState sk).rcvd
        (finalState sk).rem with
    | none => rfl
    | some pr => rw [hs] at hfix; simp at hfix
  -- prefix counts never exceed the whole
  have htake_le : ∀ (c' : Chan) (b' : Bool) (k : Nat),
      (proj c' b' ((wFinal sk).out.take k)).length
        ≤ (proj c' b' (wFinal sk).out).length :=
    fun c' b' k => ((List.take_sublist k _).filter _).length_le
  -- the weave bounds each side's total by the other
  have hRS : ∀ c' : Chan, (proj c' false (wFinal sk).out).length
      ≤ (proj c' true (wFinal sk).out).length := by
    intro c'
    rcases Nat.eq_zero_or_pos
        (proj c' false (wFinal sk).out).length with hz | hpos
    · omega
    · have hmem : ((c', false,
          (proj c' false (wFinal sk).out).length - 1) : Ev)
          ∈ (wFinal sk).out :=
        proj_mem_of_lt (hWcanon c' false) (by omega)
      obtain ⟨k, hk⟩ := List.mem_iff_getElem?.1 hmem
      have he1 := hwedge.e1_hist k c' _ hk
      rw [sndCount_eq_proj] at he1
      have := htake_le c' true k
      omega
  have hSR : ∀ c' : Chan, (proj c' true (wFinal sk).out).length
      ≤ (proj c' false (wFinal sk).out).length + sk.cap c' := by
    intro c'
    rcases Nat.eq_zero_or_pos
        (proj c' true (wFinal sk).out).length with hz | hpos
    · omega
    · have hmem : ((c', true,
          (proj c' true (wFinal sk).out).length - 1) : Ev)
          ∈ (wFinal sk).out :=
        proj_mem_of_lt (hWcanon c' true) (by omega)
      obtain ⟨k, hk⟩ := List.mem_iff_getElem?.1 hmem
      have he2 := hwedge.e2_hist k c' _ hk
      rw [rcvCount_eq_proj] at he2
      have := htake_le c' false k
      omega
  -- the minimum stalled head
  have hne : (finalState sk).rem.filterMap List.head? ≠ [] := by
    cases hr₀ : r₀ with
    | nil => rw [hr₀] at hr₀ne; simp at hr₀ne
    | cons e0 rest0 =>
        intro hnil
        have hmem : e0 ∈ (finalState sk).rem.filterMap List.head? :=
          List.mem_filterMap.2 ⟨r₀, hr₀mem, by rw [hr₀]; rfl⟩
        rw [hnil] at hmem
        cases hmem
  obtain ⟨estar, hstar_mem, hmin⟩ :=
    exists_min_image (fun e => evIdx e (wFinal sk).out) hne
  obtain ⟨rs, hrs_mem, hrs_head⟩ := List.mem_filterMap.1 hstar_mem
  obtain ⟨rest, hrs⟩ := List.head?_eq_some_iff.1 hrs_head
  obtain ⟨is, his⟩ := List.mem_iff_getElem?.1 hrs_mem
  have hdis : enabled sk (finalState sk).sent (finalState sk).rcvd
      estar = false :=
    scan_none_heads sk hscan (i := is) (by rw [his, hrs])
  obtain ⟨Tstar, hTstar, preM, hpreM, hsubM⟩ :=
    Forall2.exists_rel_left hminv.rem_struct his
  have hTstar_mem : Tstar ∈ procs sk := List.mem_of_getElem? hTstar
  have hestar_W : estar ∈ (wFinal sk).out := by
    refine (hsubW Tstar hTstar_mem).subset ?_
    rw [hpreM, hrs]
    exact List.mem_append_right _ (List.mem_cons_self ..)
  obtain ⟨c, b, n⟩ := estar
  cases b with
  | false =>
      -- STARVED RECEIVE: blame the send at the current count
      have hsent : (finalState sk).sent c
          = sndCount c (schedule sk) := hminv.sent_eq c
      simp only [enabled, decide_eq_false_iff_not, Nat.not_lt] at hdis
      have hstarve : sndCount c (schedule sk) ≤ n := by omega
      have hnW : n < (proj c false (wFinal sk).out).length := by
        have hm : ((c, false, n) : Ev)
            ∈ proj c false (wFinal sk).out :=
          List.mem_filter.2 ⟨hestar_W, by simp⟩
        rw [hWcanon c false] at hm
        exact (mem_canon_lt hm).2.2
      have hsW : sndCount c (schedule sk)
          < (proj c true (wFinal sk).out).length := by
        have := hRS c
        omega
      have hgW : ((c, true, sndCount c (schedule sk)) : Ev)
          ∈ (wFinal sk).out :=
        proj_mem_of_lt (hWcanon c true) hsW
      have hg_not : ((c, true, sndCount c (schedule sk)) : Ev)
          ∉ schedule sk :=
        not_mem_schedule_of_count sk hwf
          (Nat.le_of_eq (sndCount_eq_proj c _).symm)
      obtain ⟨h', hh'pool, hh'le⟩ :=
        blame_head sk hwf hsched hgW hg_not
      -- the receive at the current count is in the weave
      have hrW : ((c, false, sndCount c (schedule sk)) : Ev)
          ∈ (wFinal sk).out :=
        proj_mem_of_lt (hWcanon c false) (by omega)
      -- E1 in the weave: the blocker precedes that receive
      have hk1 := evIdx_getElem? hrW
      have he1 := hwedge.e1_hist _ c (sndCount c (schedule sk)) hk1
      obtain ⟨j, hjlt, hjget⟩ := mem_take_snd (hWcanon c true) he1
      have hgle := evIdx_le hjget
      -- that receive is at or before the head in the weave
      have hminstar := hmin h' hh'pool
      rcases Nat.eq_or_lt_of_le hstarve with heq | hlt2
      · rw [heq] at hgle hjlt hh'le
        omega
      · have hpair := pair_sublist_canon (c := c) (b := false)
          hlt2 hnW
        have hcanonsub : (canon c false
            (proj c false (wFinal sk).out).length).Sublist
            (wFinal sk).out := by
          rw [← hWcanon c false]
          exact List.filter_sublist
        have hordered := pos_lt_of_pair hcnt1 (hpair.trans hcanonsub)
        omega
  | true =>
      -- JAMMED SEND: blame the receive the cap window awaits
      have hrcvd : (finalState sk).rcvd c
          = rcvCount c (schedule sk) := hminv.rcvd_eq c
      simp only [enabled, decide_eq_false_iff_not, Nat.not_lt] at hdis
      have hjam : rcvCount c (schedule sk) + sk.cap c ≤ n := by omega
      have hnW : n < (proj c true (wFinal sk).out).length := by
        have hm : ((c, true, n) : Ev)
            ∈ proj c true (wFinal sk).out :=
          List.mem_filter.2 ⟨hestar_W, by simp⟩
        rw [hWcanon c true] at hm
        exact (mem_canon_lt hm).2.2
      have hrlt : rcvCount c (schedule sk)
          < (proj c false (wFinal sk).out).length := by
        have := hSR c
        omega
      have hgW : ((c, false, rcvCount c (schedule sk)) : Ev)
          ∈ (wFinal sk).out :=
        proj_mem_of_lt (hWcanon c false) hrlt
      have hg_not : ((c, false, rcvCount c (schedule sk)) : Ev)
          ∉ schedule sk :=
        not_mem_schedule_of_count sk hwf
          (Nat.le_of_eq (rcvCount_eq_proj c _).symm)
      obtain ⟨h', hh'pool, hh'le⟩ :=
        blame_head sk hwf hsched hgW hg_not
      -- E2 in the weave at the jammed send's own position
      have hkstar := evIdx_getElem? hestar_W
      have he2 := hwedge.e2_hist _ c n hkstar
      obtain ⟨j, hjlt, hjget⟩ := mem_take_rcv (hWcanon c false)
        (k := evIdx ((c, true, n) : Ev) (wFinal sk).out)
        (n := rcvCount c (schedule sk)) (by omega)
      have hgle := evIdx_le hjget
      have hminstar := hmin h' hh'pool
      omega
