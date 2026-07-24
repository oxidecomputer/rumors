/-
The O ancestor telescope: the rolling context of an assignment-ordered
site and the coverage/descent packages read off it.

The E telescope's three structural collapses hold verbatim — the
parent is always pending (counts carry no splice discriminant), every
spine rung is a `base` link, and `P1` closes from margin 0 alone. The
per-ancestor owner filters spell their scope-suffix tails `walkSegO`
(the assignment-ordered blocks), and every count read off them rides
Ord/Site.lean's pins, which conclude against the d5 totals through the
projection bridges — so the ladders and packages are TeleE.lean's
proofs with the O names swapped in.

One genuinely new pair: `phi_of_spineO`/`ascCover_of_spineO`, twins of
Ctx.lean's spine assembly over `FamOKO` — the base pair consumes
`FamOK`, which the query-first absorber assignments cannot inhabit, so
the four bundle-consuming calls inside (`asm_out_le_res`,
`asm_pends_le_out`, `absorb_out_le_req`, `wedge_rcvd_le_sent`) swap to
their landed O twins; the spine induction itself is untouched.

Chain (ord, stage D exit): edge respect; consumed by Ord/Master.lean's
ready sites. Base mirror: TeleE.lean (+ Ctx.lean's spine assembly).
Map: PROGRESS.md §13.
-/
import StreamingMirror.Ord.Site

namespace StreamingMirror.Ord

open Model
open Sched

variable (sk : Skel) (ord : OrdMap)

-- ============================================= deep-window count pins

/-- A deep O stage parked at its window start has emitted exactly the
resolutions before the window (cf. `deep_lowerE`). -/
theorem deep_lowerO (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCountP sk (procsO sk ord) fut st) {g c : Nat}
    (hgr : g < sk.rootH) (hc : c ≤ sk.stageLen g)
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk g)
      = walkSegO sk ord g c (sk.stageLen g)) :
    sndCount (lowerOut (wpk g)) st.out = sk.dsBefore g c := by
  have hfl := futLen_walkSegO_res sk ord hc (Nat.le_refl _) hfil
  have hpin := lower_snd_pinO sk ord hwf h hgr
  have hmono := dsBefore_mono sk g hc
  omega

/-- A deep O stage parked at its window start has emitted exactly the
summaries before the window (cf. `deep_upperE`). -/
theorem deep_upperO (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCountP sk (procsO sk ord) fut st) {g c : Nat}
    (hgr : g < sk.rootH) (hc : c ≤ sk.stageLen g)
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk g)
      = walkSegO sk ord g c (sk.stageLen g)) :
    sndCount (upperOut (wpk g)) st.out = c := by
  have hfl := futLen_walkSegO_upper sk ord hc hfil
  have hpin := upper_snd_pinO sk ord hwf h hgr
  omega

/-- A deep O stage parked at its window start has emitted exactly the
wires before the window (cf. `deep_wireE`). -/
theorem deep_wireO (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCountP sk (procsO sk ord) fut st) {g c : Nat}
    (hgr : g < sk.rootH) (hc : c ≤ sk.stageLen g)
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk g)
      = walkSegO sk ord g c (sk.stageLen g)) :
    sndCount (wireOut (wpk g)) st.out = sk.wiresBefore g c := by
  have hfl := futLen_walkSegO_wire sk ord hc (Nat.le_refl _) hfil
  have hpin := wire_snd_pinO sk ord hwf h hgr
  have hmono := wiresBefore_mono sk g hc
  omega

/-- A deep O stage parked at its window start has emitted exactly the
queries before the window (cf. `deep_qE`). -/
theorem deep_qO (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCountP sk (procsO sk ord) fut st) {g c : Nat}
    (h1 : 1 ≤ g)
    (hgr : g < sk.rootH) (hc : c ≤ sk.stageLen g)
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk g)
      = walkSegO sk ord g c (sk.stageLen g)) :
    sndCount (askedOut (wpk g)) st.out = sk.qsBefore g c := by
  have hfl := futLen_walkSegO_q sk ord hc (Nat.le_refl _) hfil
  have hpin := asked_snd_pinO sk ord hwf h h1 hgr
  have hmono := qsBefore_mono sk g hc
  omega

-- ================================= the spine assembly over the O bundle

/-- `Φ` from the spine chain, O twin of `phi_of_spine` (Ctx.lean): the
level supply below an answerer stage stays strictly inside the
in-flight allocation cut. The spine induction is the base one; the
four family-bundle consumers inside swap to their `FamOKO` twins. -/
theorem phi_of_spineO {P : List (List Ev)} (hfam : FamOKO sk ord P)
    {fut : List Ev}
    {st : MState} (h : WEdgeP sk P fut st)
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
      have hO := asm_out_le_resO sk ord hfam h.toWCountP htop
        (show 1 ≤ g + 1 by omega) (show g + 1 ≤ top by omega)
      rw [hout, hres] at hO
      have hwr := wedge_rcvd_le_sentO sk ord hfam h (Chan.upper p g)
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
      have hpo := asm_pends_le_outO sk ord hfam h.toWCountP htop hg1
        (show g ≤ top by omega)
      have houtg : sk.asmOutChan (p, g) = Chan.level p g :=
        asmOutChan_of_lt sk htop (by omega)
      rw [houtg,
        show asmLevelChan (p, g) = Chan.level p (g - 1) from rfl]
        at hpo
      have hwlg := wedge_rcvd_le_sentO sk ord hfam h
        (Chan.level p (g - 1))
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
      have hO := asm_out_le_resO sk ord hfam h.toWCountP htop
        (show 1 ≤ g + 1 by omega) (show g + 1 ≤ top by omega)
      rw [hout, hres] at hO
      have hwr := wedge_rcvd_le_sentO sk ord hfam h (Chan.upper p g)
      have hpo1 := asm_pends_le_outO sk ord hfam h.toWCountP htop
        (show 1 ≤ g + 1 by omega) (show g + 1 ≤ top by omega)
      rw [hout,
        show asmLevelChan (p, g + 1) = Chan.level p g from rfl]
        at hpo1
      have hwlg2 := wedge_rcvd_le_sentO sk ord hfam h (Chan.level p g)
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
      have hor := absorb_out_le_reqO sk ord hfam h.toWCountP
      have hwr := wedge_rcvd_le_sentO sk ord hfam h Chan.leafRequests
      show sndCount (Chan.level Party.I 0) st.out
        < sk.pendsBefore Party.I 1
            (sndCount (Chan.lower Party.I 1) st.out)
      omega

/-- The ascent coverage from a ladder of spine links and P1 facts, O
twin of `ascCover_of_spine`. -/
theorem ascCover_of_spineO {P : List (List Ev)} (hfam : FamOKO sk ord P)
    {fut : List Ev}
    {st : MState} (h : WEdgeP sk P fut st) {p : Party} {top : Nat}
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
  exact ⟨phi_of_spineO sk ord hfam h htop (hlink g hjg hgt hna) hgt hna,
    hp1 g hjg hgt hna⟩

-- ==================================== the rolling ancestor telescope

/-- The rolling O ancestor context of a site (cf. `AncTeleE`): the
in-flight coordinates of every stage above `h`, with the future's
per-ancestor owner filters in the assignment-ordered shape — query
residue, the remaining kid chunks, then the PENDING parent ahead of
the `walkSegO` scope suffix. -/
structure AncTeleO (h : Nat) (A j t : Nat → Nat) (fut : List Ev) :
    Prop where
  rng : ∀ G, h < G → G < sk.rootH →
    A G < sk.stageLen G
      ∧ j G < sk.nChildren G (sk.stageScope G (A G))
  isD : ∀ G, h + 2 ≤ G → G < sk.rootH →
    sk.childIsD G (sk.stageScope G (A G)) (j G) = true
  coh : ∀ G, h + 1 ≤ G → G + 1 < sk.rootH →
    A G = sk.wiresBefore (G + 1) (A (G + 1)) + j (G + 1)
  fil : ∀ G, h < G → G < sk.rootH →
    fut.filter (fun e => evOwner sk e == walkIdx sk G)
      = (chunkQ sk G (A G) (j G)).drop (t G)
        ++ (List.range' (j G + 1)
              (sk.nChildren G (sk.stageScope G (A G)) - (j G + 1))).flatMap
             (childChunk sk (wpk G) (A G))
        ++ ((upperOut (wpk G), true, A G) : Ev)
          :: walkSegO sk ord G (A G + 1) (sk.stageLen G)

/-- An in-flight O ancestor's count pins, read off the telescope: the
summary count is the scope index outright (cf. `ancTele_countsE`). -/
theorem ancTele_countsO (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (hW : WCountP sk (procsO sk ord) fut st) {h : Nat}
    {A j t : Nat → Nat}
    (hanc : AncTeleO sk ord h A j t fut) {G : Nat} (hG : h < G)
    (hGr : G < sk.rootH)
    (hD : sk.childIsD G (sk.stageScope G (A G)) (j G) = true) :
    sndCount (upperOut (wpk G)) st.out = A G
      ∧ sndCount (lowerOut (wpk G)) st.out
        = sk.dsBefore G (A G) + dRank sk (wpk G) (A G) (j G) + 1 := by
  obtain ⟨hA, hj⟩ := hanc.rng G hG hGr
  exact anc_position_countsO sk ord hwf hW hGr hA hj hD
    (futLen_ancO_upper sk ord hA (hanc.fil G hG hGr))
    (futLen_ancO_lower sk ord hA hj hD (hanc.fil G hG hGr))

/-- An in-flight O ancestor's `P1` overhang fact, from margin 0 (cf.
`ancTele_p1E`). -/
theorem ancTele_p1O (hwf : sk.wellFormed = true)
    (hm0 : ∀ s, sk.dCount s ≤ sk.capLevel) {fut : List Ev}
    {st : MState}
    (hW : WCountP sk (procsO sk ord) fut st) {h : Nat}
    {A j t : Nat → Nat}
    (hanc : AncTeleO sk ord h A j t fut) {p : Party} {G : Nat}
    (hG : h < G)
    (hGr : G < sk.rootH) (hna : asks p G = false)
    (hD : sk.childIsD G (sk.stageScope G (A G)) (j G) = true) :
    sndCount (Chan.lower p G) st.out
      ≤ sk.dsBefore G (sndCount (Chan.upper p G) st.out)
        + sk.capLevel + 1 := by
  obtain ⟨hA, hj⟩ := hanc.rng G hG hGr
  exact p1_of_ancO sk ord hwf hm0 hW hna hGr hA hj hD
    (futLen_ancO_upper sk ord hA (hanc.fil G hG hGr))
    (futLen_ancO_lower sk ord hA hj hD (hanc.fil G hG hGr))

-- ====================================================== the ladders

/-- The O spine ladder above a site: every rung is a `base` link — the
pending parents keep every covered summary count strictly inside its
cut (cf. `ancTele_ladderE`). -/
theorem ancTele_ladderO (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (hW : WCountP sk (procsO sk ord) fut st) {h : Nat}
    {A j t : Nat → Nat}
    (hanc : AncTeleO sk ord h A j t fut) {p : Party} {k : Nat}
    (hp : asks p h = false)
    (hcoh0 : h + 1 < sk.rootH →
      k = sk.wiresBefore (h + 1) (A (h + 1)) + j (h + 1))
    (hup0 : sndCount (Chan.upper p h) st.out = k) :
    ∀ m, h + 2 + 2 * m < sk.rootH →
      SpineLink sk st p (h + 2 + 2 * m) := by
  intro m hr
  cases m with
  | zero =>
      have hnaG : asks p (h + 2) = false := by
        rw [asks_add_two]
        exact hp
      obtain ⟨hA2, hj2⟩ := hanc.rng (h + 2) (by omega) hr
      have hD2 := hanc.isD (h + 2) (by omega) hr
      obtain ⟨-, hcl2⟩ := ancTele_countsO sk ord hwf hW hanc
        (show h < h + 2 by omega) hr hD2
      rw [← lowerOut_eq_of_answerer hnaG] at hcl2
      obtain ⟨hA1, hj1⟩ := hanc.rng (h + 1) (by omega) (by omega)
      have hcohl := hanc.coh (h + 1) (by omega) (by omega)
      refine spineLink_base_at sk hwf hnaG hr hA2 hj2 hD2
        (t := j (h + 1)) ?_ ?_ hcl2
      · rw [← hcohl]
        exact hj1
      · rw [← hcohl, ← hcoh0 (by omega)]
        exact hup0
  | succ m' =>
      -- the rung's floor stage is itself a covered ancestor
      have hnag : asks p (h + 2 + 2 * m') = false := by
        have hs := asks_add_two_mul p h (m' + 1)
        rw [show h + 2 * (m' + 1) = h + 2 + 2 * m' from by omega] at hs
        rw [hs]
        exact hp
      have hnaG : asks p (h + 2 + 2 * m' + 2) = false := by
        rw [asks_add_two]
        exact hnag
      have hgr : h + 2 + 2 * m' < sk.rootH := by omega
      have hrG : h + 2 + 2 * m' + 2 < sk.rootH := by omega
      have hDg := hanc.isD (h + 2 + 2 * m') (by omega) hgr
      obtain ⟨hcu, -⟩ := ancTele_countsO sk ord hwf hW hanc
        (show h < h + 2 + 2 * m' by omega) hgr hDg
      rw [← upperOut_eq_of_answerer hnag] at hcu
      obtain ⟨hA2, hj2⟩ := hanc.rng (h + 2 + 2 * m' + 2) (by omega) hrG
      have hD2 := hanc.isD (h + 2 + 2 * m' + 2) (by omega) hrG
      obtain ⟨-, hcl2⟩ := ancTele_countsO sk ord hwf hW hanc
        (show h < h + 2 + 2 * m' + 2 by omega) hrG hD2
      rw [← lowerOut_eq_of_answerer hnaG] at hcl2
      obtain ⟨hA1, hj1⟩ := hanc.rng (h + 2 + 2 * m' + 1) (by omega)
        (by omega)
      have hcohl : A (h + 2 + 2 * m' + 1)
          = sk.wiresBefore (h + 2 + 2 * m' + 2)
              (A (h + 2 + 2 * m' + 2)) + j (h + 2 + 2 * m' + 2) :=
        hanc.coh (h + 2 + 2 * m' + 1) (by omega) (by omega)
      have hcohg : A (h + 2 + 2 * m')
          = sk.wiresBefore (h + 2 + 2 * m' + 1)
              (A (h + 2 + 2 * m' + 1)) + j (h + 2 + 2 * m' + 1) :=
        hanc.coh (h + 2 + 2 * m') (by omega) (by omega)
      refine spineLink_base_at sk hwf hnaG hrG hA2 hj2 hD2
        (t := j (h + 2 + 2 * m' + 1)) ?_ ?_ hcl2
      · simp only [Nat.add_eq, Nat.mul_eq]
        rw [← hcohl]
        exact hj1
      · simp only [Nat.add_eq, Nat.mul_eq]
        rw [← hcohl, ← hcohg]
        exact hcu

/-- The O leaf ladder: the absorber bottoms stage 1, and every higher
odd stage is a `base` rung off its covered floor (cf.
`ancTele_ladder_leafE`). -/
theorem ancTele_ladder_leafO (hwf : sk.wellFormed = true)
    {fut : List Ev} {st : MState}
    (hW : WCountP sk (procsO sk ord) fut st)
    {A j t : Nat → Nat} (hanc : AncTeleO sk ord 0 A j t fut)
    (hr : 1 < sk.rootH) {k i0 : Nat} (hk : k < sk.stageLen 0)
    (hcoh0 : k = sk.wiresBefore 1 (A 1) + j 1)
    (hi0 : i0 < sk.nChildren 0 (sk.stageScope 0 k))
    (hreq0 : sndCount Chan.leafRequests st.out
      = sk.wiresBefore 0 k + i0) :
    ∀ m, 1 + 2 * m < sk.rootH →
      SpineLink sk st Party.I (1 + 2 * m) := by
  obtain ⟨hA1, hj1⟩ := hanc.rng 1 (by omega) hr
  have hD1 : sk.childIsD 1 (sk.stageScope 1 (A 1)) (j 1) = true :=
    parent_slot_isD sk hwf hr hk hA1 hj1 hcoh0 (by omega)
  intro m hrm
  cases m with
  | zero =>
      obtain ⟨-, hcl⟩ := ancTele_countsO sk ord hwf hW hanc (by omega)
        hr hD1
      refine spineLink_absorb_at sk hwf hr hA1 hj1 hD1 (i0 := i0)
        ?_ hcl ?_
      · rw [← hcoh0]
        exact hi0
      · rw [← hcoh0]
        exact hreq0
  | succ m' =>
      -- a base rung: floor stage 1 + 2 * m', covered (or the site's
      -- own parent stage when m' = 0, whose D flag is hD1)
      have hg1r : 1 + 2 * m' < sk.rootH := by omega
      have hrG : 1 + 2 * m' + 2 < sk.rootH := by omega
      have hDg : sk.childIsD (1 + 2 * m')
          (sk.stageScope (1 + 2 * m') (A (1 + 2 * m')))
          (j (1 + 2 * m')) = true := by
        rcases Nat.eq_zero_or_pos m' with rfl | hm
        · exact hD1
        · exact hanc.isD (1 + 2 * m') (by omega) (by omega)
      have hnag : asks Party.I (1 + 2 * m') = false := by
        have hs := asks_add_two_mul Party.I 1 m'
        rw [hs]
        rfl
      have hnaG : asks Party.I (1 + 2 * m' + 2) = false := by
        rw [asks_add_two]
        exact hnag
      obtain ⟨hA2, hj2⟩ := hanc.rng (1 + 2 * m' + 2) (by omega) hrG
      have hD2 : sk.childIsD (1 + 2 * m' + 2)
          (sk.stageScope (1 + 2 * m' + 2) (A (1 + 2 * m' + 2)))
          (j (1 + 2 * m' + 2)) = true :=
        hanc.isD (1 + 2 * m' + 2) (by omega) hrG
      obtain ⟨-, hcl2⟩ := ancTele_countsO sk ord hwf hW hanc
        (show 0 < 1 + 2 * m' + 2 by omega) hrG hD2
      rw [← lowerOut_eq_of_answerer hnaG] at hcl2
      obtain ⟨hAm, hjm⟩ := hanc.rng (1 + 2 * m' + 1) (by omega)
        (by omega)
      have hcohl : A (1 + 2 * m' + 1)
          = sk.wiresBefore (1 + 2 * m' + 2) (A (1 + 2 * m' + 2))
            + j (1 + 2 * m' + 2) :=
        hanc.coh (1 + 2 * m' + 1) (by omega) (by omega)
      obtain ⟨hcu, -⟩ := ancTele_countsO sk ord hwf hW hanc
        (show 0 < 1 + 2 * m' by omega) hg1r hDg
      rw [← upperOut_eq_of_answerer hnag] at hcu
      have hcohg : A (1 + 2 * m')
          = sk.wiresBefore (1 + 2 * m' + 1) (A (1 + 2 * m' + 1))
            + j (1 + 2 * m' + 1) :=
        hanc.coh (1 + 2 * m') (by omega) (by omega)
      refine spineLink_base_at sk hwf hnaG hrG hA2 hj2 hD2
        (t := j (1 + 2 * m' + 1)) ?_ ?_ hcl2
      · simp only [Nat.add_eq, Nat.mul_eq]
        rw [← hcohl]
        exact hjm
      · simp only [Nat.add_eq, Nat.mul_eq]
        rw [← hcohl, ← hcohg]
        exact hcu

-- ============================================ coverage assemblies

/-- The ascent coverage of an interior O site (cf. `ancTele_covE`):
base rungs from the ladder, overhangs from margin 0, assembled through
the O spine pair. -/
theorem ancTele_covO (hwf : sk.wellFormed = true)
    (hm0 : ∀ s, sk.dCount s ≤ sk.capLevel) {fut : List Ev}
    {st : MState}
    (hW : WEdgeP sk (procsO sk ord) fut st) {h : Nat}
    {A j t : Nat → Nat}
    (hanc : AncTeleO sk ord h A j t fut) {k : Nat}
    (hcoh0 : h + 1 < sk.rootH →
      k = sk.wiresBefore (h + 1) (A (h + 1)) + j (h + 1))
    (hup0 : sndCount (Chan.upper ((wpk h).1) h) st.out = k) :
    AscCover sk st ((wpk h).1) (h + 2) (wtop sk h) := by
  refine ascCover_of_spineO sk ord (famOKO_procsO sk ord hwf) hW
    (wpk_htop sk h) ?_ ?_
  · intro G hG2 hGt hna
    have hGr : G < sk.rootH := answerer_lt_rootH sk hwf hGt hna
    have hpar := asks_false_parity hna (asks_wpk_self h)
    obtain ⟨m, rfl⟩ : ∃ m, G = h + 2 + 2 * m :=
      ⟨(G - h - 2) / 2, by omega⟩
    exact ancTele_ladderO sk ord hwf hW.toWCountP hanc
      (asks_wpk_self h) hcoh0 hup0 m hGr
  · intro G hG2 hGt hna
    have hGr : G < sk.rootH := answerer_lt_rootH sk hwf hGt hna
    exact ancTele_p1O sk ord hwf hm0 hW.toWCountP hanc (by omega) hGr
      hna (hanc.isD G (by omega) hGr)

/-- The O leaf sites' ascent coverage (cf. `ancTele_cov_leafE`). -/
theorem ancTele_cov_leafO (hwf : sk.wellFormed = true)
    (hm0 : ∀ s, sk.dCount s ≤ sk.capLevel) {fut : List Ev}
    {st : MState}
    (hW : WEdgeP sk (procsO sk ord) fut st) {A j t : Nat → Nat}
    (hanc : AncTeleO sk ord 0 A j t fut) (hr : 1 < sk.rootH)
    {k i0 : Nat} (hk : k < sk.stageLen 0)
    (hcoh0 : k = sk.wiresBefore 1 (A 1) + j 1)
    (hi0 : i0 < sk.nChildren 0 (sk.stageScope 0 k))
    (hreq0 : sndCount Chan.leafRequests st.out
      = sk.wiresBefore 0 k + i0) :
    AscCover sk st Party.I 1 sk.rootH := by
  have hev := (wf_rootH hwf).1
  obtain ⟨hA1, hj1⟩ := hanc.rng 1 (by omega) hr
  have hD1 : sk.childIsD 1 (sk.stageScope 1 (A 1)) (j 1) = true :=
    parent_slot_isD sk hwf hr hk hA1 hj1 hcoh0 (by omega)
  refine ascCover_of_spineO sk ord (famOKO_procsO sk ord hwf) hW
    (Or.inl ⟨rfl, rfl⟩) ?_ ?_
  · intro G hG1 hGt hna
    have hGr : G < sk.rootH := by
      rcases Nat.lt_or_ge G sk.rootH with h' | h'
      · exact h'
      · exfalso
        have hG : G = sk.rootH := by omega
        subst hG
        simp [asks, hev] at hna
    have hodd : G % 2 = 1 := by
      simp only [asks, beq_eq_false_iff_ne, ne_eq] at hna
      omega
    obtain ⟨m, rfl⟩ : ∃ m, G = 1 + 2 * m := ⟨(G - 1) / 2, by omega⟩
    exact ancTele_ladder_leafO sk ord hwf hW.toWCountP hanc hr hk hcoh0
      hi0 hreq0 m hGr
  · intro G hG1 hGt hna
    have hGr : G < sk.rootH := by
      rcases Nat.lt_or_ge G sk.rootH with h' | h'
      · exact h'
      · exfalso
        have hG : G = sk.rootH := by omega
        subst hG
        simp [asks, hev] at hna
    rcases Nat.lt_or_ge G 2 with hG2 | hG2
    · have hG1' : G = 1 := by omega
      subst hG1'
      exact ancTele_p1O sk ord hwf hm0 hW.toWCountP hanc (by omega) hGr
        hna hD1
    · exact ancTele_p1O sk ord hwf hm0 hW.toWCountP hanc (by omega) hGr
        hna (hanc.isD G (by omega) hGr)

-- ================================= descent packages from the context

/-- The O upper site's descent package (cf. `descSupply_upper_of_ctxE`):
at the scope tail the whole subtree is emitted, so every deep window
sits at a clean boundary cursor. -/
theorem descSupply_upper_of_ctxO (hwf : sk.wellFormed = true)
    {fut : List Ev} {st : MState}
    (hW : WCountP sk (procsO sk ord) fut st) {h k : Nat}
    (h1 : 1 ≤ h) (hhr : h < sk.rootH) (hk : k < sk.stageLen h)
    (hasks : asks ((wpk h).1) (h + 1) = true) {X : Nat}
    (hXW : sk.wiresBefore h k ≤ X)
    (hXle : X ≤ sk.stageLen (h - 1))
    (hdeep : ∀ g', g' < h →
      fut.filter (fun e => evOwner sk e == walkIdx sk g')
        = walkSegO sk ord g' (descIdx sk g' (h - 1 - g') X)
            (sk.stageLen g'))
    (hlowh : sk.dsBefore h k ≤ sndCount (lowerOut (wpk h)) st.out)
    (hq1 : h = 1 →
      sk.qsBefore 1 k ≤ sndCount Chan.leafRequests st.out) :
    DescSupply sk st ((wpk h).1) h
      (sk.pendsBefore ((wpk h).1) (h + 1) k) := by
  have hcle : ∀ g', g' < h →
      descIdx sk g' (h - 1 - g') X ≤ sk.stageLen g' := by
    intro g' hg'
    refine descIdx_le_stageLen sk hwf ?_ ?_
    · rw [show g' + (h - 1 - g') = h - 1 from by omega]
      omega
    · rw [show g' + (h - 1 - g') = h - 1 from by omega]
      exact hXle
  have hkX : ∀ g', g' < h →
      descIdx sk g' (h - g') k ≤ descIdx sk g' (h - 1 - g') X := by
    intro g' hg'
    rw [show h - g' = h - 1 - g' + 1 from by omega, descIdx_succ,
      show g' + (h - 1 - g') + 1 = h from by omega]
    exact descIdx_mono sk g' (h - 1 - g') hXW
  refine descSupply_upper_site sk hwf h1 hhr hasks ?_ ?_ ?_
  · intro g hg1 hgh hna_g
    by_cases hgh' : g = h
    · subst hgh'
      rw [Nat.sub_self, descIdx_zero, lowerOut_eq_of_answerer hna_g]
      exact hlowh
    · have hlt : g < h := by omega
      rw [lowerOut_eq_of_answerer hna_g,
        deep_lowerO sk ord hwf hW (by omega) (hcle g hlt)
          (hdeep g hlt)]
      exact dsBefore_mono sk g (hkX g hlt)
  · intro g hg2 hasker_g
    have hna_g : asks ((wpk h).1) g = false := by
      have hs := asks_succ ((wpk h).1) g
      rw [hasker_g] at hs
      simpa using hs.symm
    rw [upperOut_eq_of_answerer hna_g,
      deep_upperO sk ord hwf hW (by omega) (hcle g (by omega))
        (hdeep g (by omega))]
    exact hkX g (by omega)
  · intro _
    have hk0 : descIdx sk 0 h k ≤ descIdx sk 0 (h - 1) X := by
      have hx := hkX 0 h1
      rw [Nat.sub_zero] at hx
      exact hx
    constructor
    · have hd0 := hdeep 0 h1
      rw [Nat.sub_zero] at hd0
      have hc0 := hcle 0 h1
      rw [Nat.sub_zero] at hc0
      rw [show Chan.wire Party.R 0 = wireOut (wpk 0) from rfl,
        deep_wireO sk ord hwf hW (by omega) hc0 hd0]
      exact wiresBefore_mono sk 0 hk0
    · by_cases h1' : h = 1
      · subst h1'
        have hpeel : descIdx sk 0 1 k = sk.wiresBefore 1 k :=
          descIdx_peel sk 0 0 k
        rw [hpeel,
          ← qs_wires sk hwf (Nat.le_refl 1) hhr (Nat.le_of_lt hk)]
        exact hq1 rfl
      · have h2 : 2 ≤ h := by omega
        have hd1 := hdeep 1 (by omega)
        rw [show h - 1 - 1 = h - 2 from by omega] at hd1
        have hc1 := hcle 1 (by omega)
        rw [show h - 1 - 1 = h - 2 from by omega] at hc1
        rw [show Chan.leafRequests = askedOut (wpk 1) from rfl,
          deep_qO sk ord hwf hW (Nat.le_refl 1) (by omega) hc1 hd1,
          qs_wires sk hwf (Nat.le_refl 1) (by omega) hc1]
        refine Nat.le_trans (wiresBefore_mono sk 0 hk0) ?_
        refine Nat.le_of_eq ?_
        have hp := descIdx_peel sk (h - 2) 0 X
        rw [show h - 2 + 1 = h - 1 from by omega] at hp
        exact congrArg (sk.wiresBefore 0) hp

/-- The O lower site's descent package (cf. `descSupply_lower_of_ctxE`). -/
theorem descSupply_lower_of_ctxO (hwf : sk.wellFormed = true)
    {fut : List Ev} {st : MState}
    (hW : WCountP sk (procsO sk ord) fut st)
    {h k i : Nat} (h1 : 1 ≤ h) (hhr : h < sk.rootH)
    (hk : k < sk.stageLen h)
    (hi : i ≤ sk.nChildren h (sk.stageScope h k))
    (hna : asks ((wpk h).1) h = false)
    (hdeep : ∀ g', g' < h →
      fut.filter (fun e => evOwner sk e == walkIdx sk g')
        = walkSegO sk ord g'
            (descIdx sk g' (h - 1 - g') (sk.wiresBefore h k + i))
            (sk.stageLen g'))
    (hq1 : h = 1 →
      sk.wiresBefore 0 (sk.wiresBefore 1 k + i)
        ≤ sndCount Chan.leafRequests st.out) :
    DescSupply sk st ((wpk h).1) (h - 1)
      (sk.pendsBefore ((wpk h).1) h
        (sk.dsBefore h k + dRank sk (wpk h) k i)) := by
  have hXle : sk.wiresBefore h k + i ≤ sk.stageLen (h - 1) := by
    have h1' := wiresBefore_succ sk hk
    have h2' := wiresBefore_mono sk h
      (show k + 1 ≤ sk.stageLen h from hk)
    have h3' := wiresBefore_total sk hwf h1 hhr
    omega
  have hcle : ∀ g', g' < h →
      descIdx sk g' (h - 1 - g') (sk.wiresBefore h k + i)
        ≤ sk.stageLen g' := by
    intro g' hg'
    refine descIdx_le_stageLen sk hwf ?_ ?_
    · rw [show g' + (h - 1 - g') = h - 1 from by omega]
      omega
    · rw [show g' + (h - 1 - g') = h - 1 from by omega]
      exact hXle
  refine descSupply_lower_site sk hwf hna h1 hhr hk hi ?_ ?_ ?_
  · intro g hg1 hgh hna_g
    rw [lowerOut_eq_of_answerer hna_g,
      deep_lowerO sk ord hwf hW (by omega) (hcle g (by omega))
        (hdeep g (by omega))]
    exact Nat.le_refl _
  · intro g hg2 hasker_g
    have hna_g : asks ((wpk h).1) g = false := by
      have hs := asks_succ ((wpk h).1) g
      rw [hasker_g] at hs
      simpa using hs.symm
    rw [upperOut_eq_of_answerer hna_g,
      deep_upperO sk ord hwf hW (by omega) (hcle g (by omega))
        (hdeep g (by omega))]
    exact Nat.le_refl _
  · intro _
    constructor
    · have hd0 := hdeep 0 h1
      rw [Nat.sub_zero] at hd0
      have hc0 := hcle 0 h1
      rw [Nat.sub_zero] at hc0
      rw [show Chan.wire Party.R 0 = wireOut (wpk 0) from rfl,
        deep_wireO sk ord hwf hW (by omega) hc0 hd0]
      exact Nat.le_refl _
    · by_cases h1' : h = 1
      · subst h1'
        exact hq1 rfl
      · have h2 : 2 ≤ h := by omega
        have hd1 := hdeep 1 (by omega)
        rw [show h - 1 - 1 = h - 2 from by omega] at hd1
        have hc1 := hcle 1 (by omega)
        rw [show h - 1 - 1 = h - 2 from by omega] at hc1
        rw [show Chan.leafRequests = askedOut (wpk 1) from rfl,
          deep_qO sk ord hwf hW (Nat.le_refl 1) (by omega) hc1 hd1,
          qs_wires sk hwf (Nat.le_refl 1) (by omega) hc1]
        refine Nat.le_of_eq ?_
        have hp := descIdx_peel sk (h - 2) 0
          (sk.wiresBefore h k + i)
        rw [show h - 2 + 1 = h - 1 from by omega] at hp
        exact congrArg (sk.wiresBefore 0) hp

end StreamingMirror.Ord
