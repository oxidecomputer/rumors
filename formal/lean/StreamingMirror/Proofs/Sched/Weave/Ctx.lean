/-
Weave pump-progress, the position layer (PROGRESS.md §7 3b, step (f)
of the pump case-tree): what a walk's own trace position pins at a
pump-facing emission. The first brick is the splice-aware prefix
bound: when a walk's cell heads at its scope-`k` parent summary, its
emitted prefix already carries every resolution of the earlier scopes
— the §5 splice only ever ADDS the current scope's resolutions in
front of the parent. This is the own-walk component of the descent
supply (`DescSupply`'s top level); the cross-walk components (the
completed-subtree boundary memberships along the coverage telescope)
and the ascent coverage are the remaining CtxOK obligations, built by
the weave-order induction (see the design of record in PROGRESS.md).
-/
import StreamingMirror.Proofs.Sched.Weave.Window

namespace StreamingMirror.Sched

open Model

variable (sk : Skel)

/-- The prefix of a walk cut at its scope-`k` parent summary carries
the resolutions of all earlier scopes. -/
theorem walk_prefix_lower {pk : Party × Nat} {k : Nat}
    {pre rest : List Ev}
    (hsplit : walkEvents sk pk
      = pre ++ (upperOut pk, true, k) :: rest) :
    sk.dsBefore pk.2 k ≤ (proj (lowerOut pk) true pre).length := by
  unfold walkEvents at hsplit
  rw [List.range_eq_range'] at hsplit
  obtain ⟨t, -, htN, p₂, r₂, hblock, hr₂, hpre, hr⟩ :=
    prefix_flatMap _ 0 hsplit (by simp)
  rw [Nat.zero_add] at htN
  rw [Nat.sub_zero] at hpre
  -- the head is block t's sole parent event, so t = k
  have hmem : ((upperOut pk, true, k) : Ev) ∈ scopeBlock sk pk t := by
    rw [hblock]
    refine List.mem_append_right _ ?_
    cases r₂ with
    | nil => exact absurd rfl hr₂
    | cons x r₃ =>
        have hx : x = (upperOut pk, true, k) := by
          have := congrArg (fun l : List Ev => l[0]?) hr
          simpa using this.symm
        rw [hx]
        exact List.mem_cons_self ..
  have hmp : ((upperOut pk, true, k) : Ev)
      ∈ proj (upperOut pk) true (scopeBlock sk pk t) :=
    List.mem_filter.2 ⟨hmem, by simp⟩
  rw [proj_block_upper, seg_one] at hmp
  have htk : t = k := by
    have h := List.mem_singleton.1 hmp
    simpa using (congrArg (fun e : Ev => e.2.2) h).symm
  subst htk
  -- the closed blocks before t carry their full resolution segments
  have hrun : proj (lowerOut pk) true
      ((List.range t).flatMap (scopeBlock sk pk))
      = seg (lowerOut pk) true (sk.dsBefore pk.2 0)
          (sk.dsBefore pk.2 t - sk.dsBefore pk.2 0) :=
    proj_flatMap_seg t
      (fun i hi => proj_block_res sk pk (by omega))
      (fun i hi => by
        have := dsBefore_succ sk (h := pk.2) (k := i) (by omega)
        omega)
  rw [hpre, proj_append, List.length_append, ← List.range_eq_range',
    hrun, seg_len]
  have h0 : sk.dsBefore pk.2 0 = 0 := rfl
  omega

-- ================================================ the spine telescope

/-- The spine linking chain feeding the ascent coverage's `Φ` fact.

Indexed by an answerer stage `g + 2`, each link relates the ancestor
walk two stages down to the stage's allocation cut: `base` when that
walk's summary count sits strictly inside the cut (a pre-splice
ancestor, or the emission's own unsent summary), `step` when the
splice has fired below — the summary count touches the cut, the lower
walk's pends line equals its resolution count (the splice identity),
and the chain continues two stages down. The initiator side bottoms
out below stage 2 at `absorbBase`: the stage-1 allocation cut is fed
by the absorber, so the unsent leaf request stands in for the unsent
summary (`p` is a uniform parameter, hence the equation hypothesis).
Layer D reads each link off the worklist tail's `descIdx` windows
(`align_kids_suffix`). -/
inductive SpineLink (st : MState) (p : Party) : Nat → Prop
  | base (g : Nat)
      (hlt : sndCount (Chan.upper p g) st.out
        < sk.pendsBefore p (g + 2)
            (sndCount (Chan.lower p (g + 2)) st.out)) :
      SpineLink st p (g + 2)
  | step (g : Nat) (hg1 : 1 ≤ g)
      (hle : sndCount (Chan.upper p g) st.out
        ≤ sk.pendsBefore p (g + 2)
            (sndCount (Chan.lower p (g + 2)) st.out))
      (hpb : sk.pendsBefore p (g + 1)
            (sndCount (Chan.upper p g) st.out)
          = sndCount (Chan.lower p g) st.out)
      (prev : SpineLink st p g) :
      SpineLink st p (g + 2)
  | absorbBase (hp : p = Party.I)
      (hlt : sndCount Chan.leafRequests st.out
        < sk.pendsBefore p 1 (sndCount (Chan.lower p 1) st.out)) :
      SpineLink st p 1

/-- `Φ` from the spine chain: the level supply below an answerer
stage stays strictly inside the in-flight allocation cut.

Downward induction along the links. At a `base` link the producer
asker above the lower walk is capped by that walk's summary count
outright — resolution consumption cannot pass an unsent summary. At a
`step` link, if the producer consumed the summary touching the cut,
its pends line (`asm_pends_le_out`) plus the splice identity force
the answerer below to have delivered everything it was sent — which
the induction hypothesis (its own `Φ`, again through
`asm_pends_le_out`) refutes. At the `absorbBase` bottom the producer
is the absorber, capped request-side (`absorb_out_le_req`) by the
unsent leaf request strictly inside the cut. -/
theorem phi_of_spine (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WEdge sk fut st)
    {p : Party} {top : Nat}
    (htop : p = Party.I ∧ top = sk.rootH
      ∨ p = Party.R ∧ top = sk.rootH - 1)
    {j : Nat} (hsl : SpineLink sk st p j) :
    j ≤ top → asks p j = false →
      sndCount (Chan.level p (j - 1)) st.out
        < sk.pendsBefore p j (sndCount (Chan.lower p j) st.out) := by
  induction hsl with
  | base g hlt =>
      intro hjt hna
      have hasker : asks p (g + 1) = true := by
        have hs := asks_succ p (g + 1)
        rw [show g + 1 + 1 = g + 2 from rfl, hna] at hs
        simpa using hs.symm
      have hout : sk.asmOutChan (p, g + 1) = Chan.level p (g + 1) :=
        asmOutChan_of_lt sk htop (by omega)
      have hres : asmResChan (p, g + 1) = Chan.upper p g := by
        have hr := asmResChan_asker (j := g + 1) hasker
        simpa using hr
      have hO := asm_out_le_res sk (famOK_procs sk hwf) h.toWCountP htop
        (show 1 ≤ g + 1 by omega) (show g + 1 ≤ top by omega)
      rw [hout, hres] at hO
      have hwr := wedge_rcvd_le_sent sk (famOK_procs sk hwf) h (Chan.upper p g)
      show sndCount (Chan.level p (g + 1)) st.out
        < sk.pendsBefore p (g + 2)
            (sndCount (Chan.lower p (g + 2)) st.out)
      omega
  | step g hg1 hle hpb _prev ih =>
      intro hjt hna
      have hasker : asks p (g + 1) = true := by
        have hs := asks_succ p (g + 1)
        rw [show g + 1 + 1 = g + 2 from rfl, hna] at hs
        simpa using hs.symm
      have hnag : asks p g = false := by
        have hs := asks_succ p g
        rw [hasker] at hs
        simpa using hs.symm
      -- the answerer below has NOT delivered everything it was sent
      have hphi := ih (by omega) hnag
      have hpo := asm_pends_le_out sk (famOK_procs sk hwf) h.toWCountP htop hg1
        (show g ≤ top by omega)
      have houtg : sk.asmOutChan (p, g) = Chan.level p g :=
        asmOutChan_of_lt sk htop (by omega)
      rw [houtg,
        show asmLevelChan (p, g) = Chan.level p (g - 1) from rfl]
        at hpo
      have hwlg := wedge_rcvd_le_sent sk (famOK_procs sk hwf) h (Chan.level p (g - 1))
      have hth : sndCount (Chan.level p g) st.out
          < sndCount (Chan.lower p g) st.out := by
        rcases Nat.lt_or_ge (sndCount (Chan.level p g) st.out)
            (sndCount (Chan.lower p g) st.out) with hlt' | hge'
        · exact hlt'
        · exfalso
          have hmono := pendsBefore_mono sk p g hge'
          omega
      -- the producer above
      have hout : sk.asmOutChan (p, g + 1) = Chan.level p (g + 1) :=
        asmOutChan_of_lt sk htop (by omega)
      have hres : asmResChan (p, g + 1) = Chan.upper p g := by
        have hr := asmResChan_asker (j := g + 1) hasker
        simpa using hr
      have hO := asm_out_le_res sk (famOK_procs sk hwf) h.toWCountP htop
        (show 1 ≤ g + 1 by omega) (show g + 1 ≤ top by omega)
      rw [hout, hres] at hO
      have hwr := wedge_rcvd_le_sent sk (famOK_procs sk hwf) h (Chan.upper p g)
      have hpo1 := asm_pends_le_out sk (famOK_procs sk hwf) h.toWCountP htop
        (show 1 ≤ g + 1 by omega) (show g + 1 ≤ top by omega)
      rw [hout,
        show asmLevelChan (p, g + 1) = Chan.level p g from rfl]
        at hpo1
      have hwlg2 := wedge_rcvd_le_sent sk (famOK_procs sk hwf) h (Chan.level p g)
      show sndCount (Chan.level p (g + 1)) st.out
        < sk.pendsBefore p (g + 2)
            (sndCount (Chan.lower p (g + 2)) st.out)
      rcases Nat.lt_or_ge (sndCount (Chan.level p (g + 1)) st.out)
          (sndCount (Chan.upper p g) st.out) with hcase | hcase
      · omega
      · exfalso
        have hOe : sndCount (Chan.level p (g + 1)) st.out
            = sndCount (Chan.upper p g) st.out := by omega
        rw [hOe, hpb] at hpo1
        omega
  | absorbBase hp hlt =>
      intro _ _
      subst hp
      have hor := absorb_out_le_req sk (famOK_procs sk hwf) h.toWCountP
      have hwr := wedge_rcvd_le_sent sk (famOK_procs sk hwf) h Chan.leafRequests
      show sndCount (Chan.level Party.I 0) st.out
        < sk.pendsBefore Party.I 1
            (sndCount (Chan.lower Party.I 1) st.out)
      omega

-- ================================================= the ladder rungs

/-- A pre-splice ancestor position yields its stage's spine link.

The answerer's cut sits at its in-flight D slot's resolution; one
`dRank_succ` step and `pends_cut_mid` convert it to the wire prefix
of the slot AFTER the in-flight one, and the ancestor two stages
down — still mid-scope inside that slot's subtree — keeps its summary
strictly below by `spine_nest`. -/
theorem spineLink_base_at (hwf : sk.wellFormed = true) {st : MState}
    {p : Party} {g : Nat}
    (hna : asks p (g + 2) = false) (hg2r : g + 2 < sk.rootH)
    {A2 i2 : Nat} (hA2 : A2 < sk.stageLen (g + 2))
    (hi2 : i2 < sk.nChildren (g + 2) (sk.stageScope (g + 2) A2))
    (hD2 : sk.childIsD (g + 2) (sk.stageScope (g + 2) A2) i2 = true)
    {t : Nat}
    (ht : t < sk.nChildren (g + 1)
        (sk.stageScope (g + 1) (sk.wiresBefore (g + 2) A2 + i2)))
    (hup : sndCount (Chan.upper p g) st.out
      = sk.wiresBefore (g + 1) (sk.wiresBefore (g + 2) A2 + i2) + t)
    (hlow : sndCount (Chan.lower p (g + 2)) st.out
      = sk.dsBefore (g + 2) A2 + dRank sk (wpk (g + 2)) A2 i2 + 1) :
    SpineLink sk st p (g + 2) := by
  have hB : sk.wiresBefore (g + 2) A2 + i2 < sk.stageLen (g + 1) :=
    kid_index_lt sk hwf (by omega) hg2r hA2 hi2
  have hdr : dRank sk (wpk (g + 2)) A2 (i2 + 1)
      = dRank sk (wpk (g + 2)) A2 i2
        + (if sk.childIsD (g + 2) (sk.stageScope (g + 2) A2) i2
            then 1 else 0) :=
    dRank_succ sk (wpk (g + 2)) A2 i2
  rw [hD2, if_pos rfl] at hdr
  refine SpineLink.base g ?_
  rw [hlow, Nat.add_assoc, ← hdr,
    pends_cut_mid sk hwf hna (by omega) hg2r hA2 (by omega), hup]
  exact spine_nest sk hB ht

/-- The ascent coverage from a ladder of spine links and P1 facts:
`Φ` per answerer stage is `phi_of_spine` at that stage's link. -/
theorem ascCover_of_spine (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WEdge sk fut st) {p : Party} {top : Nat}
    (htop : p = Party.I ∧ top = sk.rootH
      ∨ p = Party.R ∧ top = sk.rootH - 1) {j : Nat}
    (hlink : ∀ G, j ≤ G → G ≤ top → asks p G = false →
      SpineLink sk st p G)
    (hp1 : ∀ G, j ≤ G → G ≤ top → asks p G = false →
      sndCount (Chan.lower p G) st.out
        ≤ sk.dsBefore G (sndCount (Chan.upper p G) st.out)
          + sk.capLevel + 1) :
    AscCover sk st p j top := by
  intro g hjg hgt hna
  exact ⟨phi_of_spine sk hwf h htop (hlink g hjg hgt hna) hgt hna,
    hp1 g hjg hgt hna⟩
