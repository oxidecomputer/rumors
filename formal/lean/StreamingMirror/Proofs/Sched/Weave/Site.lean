/-
Window-site assembly (PROGRESS.md §7 3b, layer D): the per-site
wrappers that turn the counting layer's pins into the window lemmas'
hypothesis packages. The `hsnd` family here is the first tier: each
window site's emitted count equals the seq of the event being
emitted, read as total minus the site's collapsed `futLen` share
(`Emit.lean`'s site pins), with the channel bridged from the pins'
`wpk` spelling to the windows' party-indexed spelling.
-/
import StreamingMirror.Proofs.Sched.Weave.Emit

namespace StreamingMirror.Sched

open Model

variable (sk : Skel)

-- ================================================= the hsnd wrappers

/-- The summary site's `hsnd`: the walk has emitted exactly the
summaries of the scopes before its current one. -/
theorem upper_site_hsnd (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCount sk fut st) {p : Party} {hh k : Nat}
    (hna : asks p hh = false) (hhr : hh < sk.rootH)
    (hk : k < sk.stageLen hh)
    (hfu : futLen sk fut (walkIdx sk hh) (upperOut (wpk hh)) true
      = sk.stageLen hh - k) :
    sndCount (Chan.upper p hh) st.out = k := by
  have hch : upperOut (wpk hh) = Chan.upper p hh := by
    rw [show upperOut (wpk hh) = Chan.upper (wpk hh).1 hh from rfl,
      wpk_fst_of_answerer hna]
  have hpin := upper_snd_pin sk hwf h hhr
  rw [hch] at hpin hfu
  omega

/-- The resolution site's `hsnd`: the walk has emitted exactly the
resolutions before its current slot's. -/
theorem lower_site_hsnd (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCount sk fut st) {p : Party} {hh k i : Nat}
    (hna : asks p hh = false) (hhr : hh < sk.rootH)
    (hfu : futLen sk fut (walkIdx sk hh) (lowerOut (wpk hh)) true
      = sk.dsBefore hh (sk.stageLen hh)
        - (sk.dsBefore hh k + dRank sk (wpk hh) k i))
    (hbnd : sk.dsBefore hh k + dRank sk (wpk hh) k i
      < sk.dsBefore hh (sk.stageLen hh)) :
    sndCount (Chan.lower p hh) st.out
      = sk.dsBefore hh k + dRank sk (wpk hh) k i := by
  have hch : lowerOut (wpk hh) = Chan.lower p hh := by
    rw [show lowerOut (wpk hh) = Chan.lower (wpk hh).1 hh from rfl,
      wpk_fst_of_answerer hna]
  have hpin := lower_snd_pin sk hwf h hhr
  rw [hch] at hpin hfu
  omega

/-- The leaf-wire site's `hsnd`: the stage-0 walk has emitted exactly
the wires before its current slot's. -/
theorem wire0_site_hsnd (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCount sk fut st) {k i : Nat}
    (hr : 0 < sk.rootH)
    (hfu : futLen sk fut (walkIdx sk 0) (wireOut (wpk 0)) true
      = sk.wiresBefore 0 (sk.stageLen 0) - (sk.wiresBefore 0 k + i))
    (hbnd : sk.wiresBefore 0 k + i
      < sk.wiresBefore 0 (sk.stageLen 0)) :
    sndCount (Chan.wire Party.R 0) st.out = sk.wiresBefore 0 k + i := by
  have hch : wireOut (wpk 0) = Chan.wire Party.R 0 := rfl
  have hpin := wire_snd_pin sk hwf h hr
  rw [hch] at hpin hfu
  omega

/-- The leaf-request site's `hsnd`: the stage-1 walk has emitted
exactly the requests before its current feed cursor's. -/
theorem leafreq_site_hsnd (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCount sk fut st) {K i t : Nat}
    (hr : 1 < sk.rootH)
    (hfu : futLen sk fut (walkIdx sk 1) (askedOut (wpk 1)) true
      = sk.qsBefore 1 (sk.stageLen 1)
        - (sk.qsBefore 1 K + qSum sk (wpk 1) K i + t))
    (hbnd : sk.qsBefore 1 K + qSum sk (wpk 1) K i + t
      < sk.qsBefore 1 (sk.stageLen 1)) :
    sndCount Chan.leafRequests st.out
      = sk.qsBefore 1 K + qSum sk (wpk 1) K i + t := by
  have hch : askedOut (wpk 1) = Chan.leafRequests := rfl
  have hpin := asked_snd_pin sk hwf h (Nat.le_refl 1) hr
  rw [hch] at hpin hfu
  omega

-- ============================================== position/P1 assembly

/-- `P1` at a lower-emission site: the emitting walk's own stage,
where the resolution being sent has no `+1` yet (`σ = 0` — the
spliced summary follows the last D resolution in the chunk). -/
theorem p1_of_lower_site (hsched : sk.schedulable = true)
    {st : MState} {p : Party} {g A i : Nat} (hA : A < sk.stageLen g)
    (hi : i < sk.nChildren g (sk.stageScope g A))
    (hD : sk.childIsD g (sk.stageScope g A) i = true)
    (hup : sndCount (Chan.upper p g) st.out = A)
    (hlow : sndCount (Chan.lower p g) st.out
      = sk.dsBefore g A + dRank sk (wpk g) A i) :
    sndCount (Chan.lower p g) st.out
      ≤ sk.dsBefore g (sndCount (Chan.upper p g) st.out)
        + sk.capLevel + 1 := by
  have hd := schedulable_dOf sk hsched hA
  have hdr : dRank sk (wpk g) A i + 1
      ≤ sk.dOf g (sk.stageScope g A) :=
    dRank_succ_le_dOf sk (wpk g) hi hD
  rw [hup, hlow]
  omega

/-- The in-flight ancestor's count pins: an ancestor parked at scope
`A`, D slot `jD`, has emitted `A` summaries plus the splice and
exactly the resolutions through slot `jD`'s.

The `futLen` hypotheses are `futLen_anc_upper`/`futLen_anc_lower`'s
conclusions; the subtraction exactness rides on `dRank_succ_le_dOf`
through the allocation line of scope `A + 1`. -/
theorem anc_position_counts (hwf : sk.wellFormed = true)
    {fut : List Ev} {st : MState} (h : WCount sk fut st)
    {g A jD : Nat} (hgr : g < sk.rootH) (hA : A < sk.stageLen g)
    (hjD : jD < sk.nChildren g (sk.stageScope g A))
    (hD : sk.childIsD g (sk.stageScope g A) jD = true)
    (hfu : futLen sk fut (walkIdx sk g) (upperOut (wpk g)) true
      = sk.stageLen g - A
        - (if lastDOf sk g A == some jD then 1 else 0))
    (hfl : futLen sk fut (walkIdx sk g) (lowerOut (wpk g)) true
      = sk.dsBefore g (sk.stageLen g)
        - (sk.dsBefore g A + dRank sk (wpk g) A jD + 1)) :
    sndCount (upperOut (wpk g)) st.out
        = A + (if lastDOf sk g A == some jD then 1 else 0)
      ∧ sndCount (lowerOut (wpk g)) st.out
        = sk.dsBefore g A + dRank sk (wpk g) A jD + 1 := by
  have hupp := upper_snd_pin sk hwf h hgr
  have hlop := lower_snd_pin sk hwf h hgr
  have hdr : dRank sk (wpk g) A jD + 1
      ≤ sk.dOf g (sk.stageScope g A) :=
    dRank_succ_le_dOf sk (wpk g) hjD hD
  have hds := dsBefore_succ sk hA
  have hmono : sk.dsBefore g (A + 1)
      ≤ sk.dsBefore g (sk.stageLen g) :=
    dsBefore_mono sk g hA
  refine ⟨?_, by omega⟩
  by_cases hbe : lastDOf sk g A = some jD
  · have hb : (lastDOf sk g A == some jD) = true := by simp [hbe]
    rw [hb, if_pos rfl] at hfu ⊢
    omega
  · have hb : (lastDOf sk g A == some jD) = false := by simp [hbe]
    rw [hb, if_neg (by simp)] at hfu ⊢
    omega

/-- `P1` at a covered ancestor: its in-flight D slot pins the counts
in `p1_of_position`'s exact shape, spliced or not.

Post-splice (`lastDOf = some jD`) the slot is the last D and
`dRank_lastD` closes the allocation exactly; pre-splice the slot sits
strictly below the last D and `dRank_below_lastD` leaves the two-slot
slack `schedulable` caps. -/
theorem p1_of_anc (hwf : sk.wellFormed = true)
    (hsched : sk.schedulable = true) {fut : List Ev} {st : MState}
    (h : WCount sk fut st) {p : Party} {g A jD : Nat}
    (hna : asks p g = false) (hgr : g < sk.rootH)
    (hA : A < sk.stageLen g)
    (hjD : jD < sk.nChildren g (sk.stageScope g A))
    (hD : sk.childIsD g (sk.stageScope g A) jD = true)
    (hfu : futLen sk fut (walkIdx sk g) (upperOut (wpk g)) true
      = sk.stageLen g - A
        - (if lastDOf sk g A == some jD then 1 else 0))
    (hfl : futLen sk fut (walkIdx sk g) (lowerOut (wpk g)) true
      = sk.dsBefore g (sk.stageLen g)
        - (sk.dsBefore g A + dRank sk (wpk g) A jD + 1)) :
    sndCount (Chan.lower p g) st.out
      ≤ sk.dsBefore g (sndCount (Chan.upper p g) st.out)
        + sk.capLevel + 1 := by
  have hchu : upperOut (wpk g) = Chan.upper p g := by
    rw [show upperOut (wpk g) = Chan.upper (wpk g).1 g from rfl,
      wpk_fst_of_answerer hna]
  have hchl : lowerOut (wpk g) = Chan.lower p g := by
    rw [show lowerOut (wpk g) = Chan.lower (wpk g).1 g from rfl,
      wpk_fst_of_answerer hna]
  obtain ⟨hcu, hcl⟩ :=
    anc_position_counts sk hwf h hgr hA hjD hD hfu hfl
  rw [hchu] at hcu
  rw [hchl] at hcl
  by_cases hbe : lastDOf sk g A = some jD
  · have hb : (lastDOf sk g A == some jD) = true := by simp [hbe]
    rw [hb, if_pos rfl] at hcu
    have hdl : dRank sk (wpk g) A jD + 1
        = sk.dOf g (sk.stageScope g A) :=
      dRank_lastD sk hbe
    exact p1_of_position sk hsched hA hcu hcl (Or.inr ⟨rfl, hdl⟩)
  · have hb : (lastDOf sk g A == some jD) = false := by simp [hbe]
    rw [hb, if_neg (by simp)] at hcu
    obtain ⟨j, hj, -⟩ := lastDOf_isSome_of_D sk hD hjD
    have hne : jD ≠ j := fun he => hbe (by rw [he]; exact hj)
    have hbl : dRank sk (wpk g) A jD + 2
        ≤ sk.dOf g (sk.stageScope g A) :=
      dRank_below_lastD sk hj hD hne
    exact p1_of_position sk hsched hA hcu hcl (Or.inl ⟨rfl, hbl⟩)

-- ================================================= the ladder rungs

/-- A post-splice ancestor position yields its stage's spine link,
consuming the link two stages down.

The summary count touches the allocation cut (`spine_nest` gives the
non-strict side), and the splice identity `splice_link` supplies the
step's pends equation from the post-splice pins. -/
theorem spineLink_step_at (hwf : sk.wellFormed = true) {st : MState}
    {p : Party} {g : Nat} (hg1 : 1 ≤ g)
    (hna : asks p (g + 2) = false) (hg2r : g + 2 < sk.rootH)
    {A2 i2 : Nat} (hA2 : A2 < sk.stageLen (g + 2))
    (hi2 : i2 < sk.nChildren (g + 2) (sk.stageScope (g + 2) A2))
    (hD2 : sk.childIsD (g + 2) (sk.stageScope (g + 2) A2) i2 = true)
    {t : Nat}
    (ht : t < sk.nChildren (g + 1)
        (sk.stageScope (g + 1) (sk.wiresBefore (g + 2) A2 + i2)))
    (hup : sndCount (Chan.upper p g) st.out
      = sk.wiresBefore (g + 1) (sk.wiresBefore (g + 2) A2 + i2) + t
        + 1)
    (hlow : sndCount (Chan.lower p g) st.out
      = sk.dsBefore g
          (sk.wiresBefore (g + 1) (sk.wiresBefore (g + 2) A2 + i2)
            + t + 1))
    (hlow2 : sndCount (Chan.lower p (g + 2)) st.out
      = sk.dsBefore (g + 2) A2 + dRank sk (wpk (g + 2)) A2 i2 + 1)
    (prev : SpineLink sk st p g) :
    SpineLink sk st p (g + 2) := by
  have hasker : asks p (g + 1) = true := by
    have hs := asks_succ p (g + 1)
    rw [show g + 1 + 1 = g + 2 from rfl, hna] at hs
    simpa using hs.symm
  have hB : sk.wiresBefore (g + 2) A2 + i2 < sk.stageLen (g + 1) :=
    kid_index_lt sk hwf (by omega) hg2r hA2 hi2
  have hdr : dRank sk (wpk (g + 2)) A2 (i2 + 1)
      = dRank sk (wpk (g + 2)) A2 i2
        + (if sk.childIsD (g + 2) (sk.stageScope (g + 2) A2) i2
            then 1 else 0) :=
    dRank_succ sk (wpk (g + 2)) A2 i2
  rw [hD2, if_pos rfl] at hdr
  have hpb := splice_link sk hg1 hasker hup hlow
  refine SpineLink.step g hg1 ?_ hpb prev
  rw [hlow2, Nat.add_assoc, ← hdr,
    pends_cut_mid sk hwf hna (by omega) hg2r hA2 (by omega), hup]
  exact spine_nest sk hB ht

/-- The leaf sites' stage-1 spine link: the emitted leaf request sits
strictly inside the stage-1 allocation cut, bottoming the initiator
ladder at the absorber. -/
theorem spineLink_absorb_at (hwf : sk.wellFormed = true)
    {st : MState} (hr : 1 < sk.rootH) {K1 i1 : Nat}
    (hK1 : K1 < sk.stageLen 1)
    (hi1 : i1 < sk.nChildren 1 (sk.stageScope 1 K1))
    (hD1 : sk.childIsD 1 (sk.stageScope 1 K1) i1 = true)
    {i0 : Nat}
    (hi0 : i0 < sk.nChildren 0
        (sk.stageScope 0 (sk.wiresBefore 1 K1 + i1)))
    (hlow : sndCount (Chan.lower Party.I 1) st.out
      = sk.dsBefore 1 K1 + dRank sk (wpk 1) K1 i1 + 1)
    (hreq : sndCount Chan.leafRequests st.out
      = sk.wiresBefore 0 (sk.wiresBefore 1 K1 + i1) + i0) :
    SpineLink sk st Party.I 1 := by
  have hB : sk.wiresBefore 1 K1 + i1 < sk.stageLen 0 :=
    kid_index_lt sk hwf (Nat.le_refl 1) hr hK1 hi1
  have hdr : dRank sk (wpk 1) K1 (i1 + 1)
      = dRank sk (wpk 1) K1 i1
        + (if sk.childIsD 1 (sk.stageScope 1 K1) i1
            then 1 else 0) :=
    dRank_succ sk (wpk 1) K1 i1
  rw [hD1, if_pos rfl] at hdr
  refine SpineLink.absorbBase rfl ?_
  rw [hlow, Nat.add_assoc, ← hdr,
    pends_cut_mid sk hwf (p := Party.I) rfl (Nat.le_refl 1) hr hK1
      (by omega), hreq]
  exact spine_nest sk hB hi0

end StreamingMirror.Sched
