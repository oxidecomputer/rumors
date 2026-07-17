/-
Layer-D emission-site support (PROGRESS.md §7 3b (g)): the counting
route's cornerstone. `WCount.man_struct` says every manual trace is
its emitted prefix plus its owner filter of the future, and
`out_proj_owner` collapses an owned channel-side of `out` onto that
prefix — so an owned count is its whole-trace total minus the
future's share (`futLen`). Layer D computes `futLen` from the SYNTAX
of the remaining worklist (`align_kids_suffix` and the `descIdx`
windows); the lemmas here convert those reads into the count pins the
window lemmas' hypothesis packages consume (`hsnd`, `hroot`,
`DescSupply`, `SpineLink`).
-/
import StreamingMirror.Proofs.Sched.Weave.Ctx

namespace StreamingMirror.Sched

open Model

variable (sk : Skel)

-- ================================================== the count pin

/-- The future's share of an owner's channel-side. -/
def futLen (fut : List Ev) (M : Nat) (c : Chan) (b : Bool) : Nat :=
  (proj c b (fut.filter fun e => evOwner sk e == M)).length

/-- THE COUNT PIN: an owned channel-side's emitted count plus the
future's share is the whole-trace total. -/
theorem count_pin (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCount sk fut st) (c : Chan) (b : Bool)
    {M : Nat} (hM : (if b then sndOwner sk c else rcvOwner sk c) = M)
    (hMlt : M < manCount sk)
    {T : List Ev} (hT : (procs sk)[M]? = some T) :
    (proj c b st.out).length + futLen sk fut M c b
      = (proj c b T).length := by
  obtain ⟨r, pre, hr, hpre, hsub⟩ := cell_of_owner sk h hT
  have hout := out_proj_owner sk hwf h c b hM hT hr hpre hsub
  have hrf : fut.filter (fun e => evOwner sk e == M) = r := by
    have hlen : M < (manFilters sk fut).length := by
      unfold manFilters
      simpa using hMlt
    rw [List.getElem?_append_left hlen] at hr
    unfold manFilters at hr
    rw [List.getElem?_map, List.getElem?_range hMlt] at hr
    simpa using hr
  unfold futLen
  rw [hout, hpre, proj_append, List.length_append, hrf]

-- ============================================= manual trace lookups

/-- The stage-`h` walk sits at slot `walkIdx h`. -/
theorem procs_walk {h : Nat} (hh : h < sk.rootH) :
    (procs sk)[walkIdx sk h]? = some (walkEvents sk (wpk h)) := by
  unfold procs
  have hidx : walkIdx sk h = 2 + (sk.rootH - 1 - h) := rfl
  rw [hidx]
  simp only [List.cons_append, List.nil_append]
  rw [show 2 + (sk.rootH - 1 - h) = sk.rootH - 1 - h + 1 + 1
      from by omega,
    List.getElem?_cons_succ, List.getElem?_cons_succ,
    List.getElem?_append_left (by
      simp only [List.length_append, List.length_map,
        List.length_range, List.length_cons, List.length_nil]
      omega),
    List.getElem?_append_left (by
      simp only [List.length_append, List.length_map,
        List.length_range, List.length_cons, List.length_nil]
      omega),
    List.getElem?_append_left (by
      simp only [List.length_map, List.length_range]
      omega),
    List.getElem?_map, List.getElem?_map,
    List.getElem?_range (by omega)]
  simp only [Option.map_some]
  rw [show sk.rootH - 1 - (sk.rootH - 1 - h) = h from by omega]
  rfl

/-- The responder opener sits at slot 1. -/
theorem procs_ropen : (procs sk)[1]? = some (ropenEvents sk) := rfl

-- ============================================== whole-trace totals

/-- A walk's summary sends are the canonical run over its stage. -/
theorem walk_upper_total (pk : Party × Nat) :
    proj (upperOut pk) true (walkEvents sk pk)
      = canon (upperOut pk) true (sk.stageLen pk.2) := by
  unfold walkEvents
  refine proj_flatMap_canon (g := fun k => k) _ rfl
    (fun k _ => ?_) (fun k _ => by omega)
  have h1 : k + 1 - k = 1 := by omega
  rw [h1]
  exact proj_block_upper sk pk k

/-- A walk's resolution sends are the canonical run over its D-kid
total. -/
theorem walk_lower_total (pk : Party × Nat) :
    proj (lowerOut pk) true (walkEvents sk pk)
      = canon (lowerOut pk) true
          (sk.dsBefore pk.2 (sk.stageLen pk.2)) := by
  unfold walkEvents
  exact proj_flatMap_canon (g := sk.dsBefore pk.2) _ rfl
    (fun k hk => proj_block_res sk pk hk)
    (fun k hk => by rw [dsBefore_succ sk hk]; omega)

/-- A walk's wire sends are the canonical run over its kid total. -/
theorem walk_wire_total (pk : Party × Nat) :
    proj (wireOut pk) true (walkEvents sk pk)
      = canon (wireOut pk) true
          (sk.wiresBefore pk.2 (sk.stageLen pk.2)) := by
  unfold walkEvents
  exact proj_flatMap_canon (g := sk.wiresBefore pk.2) _ rfl
    (fun k hk => proj_block_wire sk pk hk)
    (fun k hk => by rw [wiresBefore_succ sk hk]; omega)

/-- The opener sends the root resolution exactly once. -/
theorem ropen_rootres_total :
    proj Chan.rootres true (ropenEvents sk)
      = [(Chan.rootres, true, 0)] := by
  unfold ropenEvents proj
  rw [List.filter_cons_of_neg (by simp), List.filter_cons_of_neg
    (by simp), List.filter_cons_of_pos (by simp)]
  rw [List.filter_eq_nil_iff.2]
  intro e he
  obtain ⟨j, -, rfl⟩ := List.mem_map.1 he
  simp

-- ==================================================== count pins

/-- The walk-owned send channels' pins, seen as `sndCount`. -/
theorem walk_snd_pin (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCount sk fut st) {hh : Nat}
    (hhr : hh < sk.rootH) (c : Chan)
    (hM : sndOwner sk c = walkIdx sk hh) :
    sndCount c st.out + futLen sk fut (walkIdx sk hh) c true
      = (proj c true (walkEvents sk (wpk hh))).length := by
  have hMlt : walkIdx sk hh < manCount sk := by
    unfold walkIdx manCount
    omega
  rw [sndCount_eq_proj]
  exact count_pin sk hwf h c true (by simpa using hM) hMlt
    (procs_walk sk hhr)

/-- The summary pin: emitted summaries plus the future's share is the
stage length. -/
theorem upper_snd_pin (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCount sk fut st) {hh : Nat}
    (hhr : hh < sk.rootH) :
    sndCount (upperOut (wpk hh)) st.out
        + futLen sk fut (walkIdx sk hh) (upperOut (wpk hh)) true
      = sk.stageLen hh := by
  have hp := walk_snd_pin sk hwf h hhr (upperOut (wpk hh)) rfl
  have hlen : (proj (upperOut (wpk hh)) true
      (walkEvents sk (wpk hh))).length = sk.stageLen hh := by
    rw [walk_upper_total]
    simp [canon, wpk]
  omega

/-- The resolution pin: emitted resolutions plus the future's share
is the stage's D-kid total. -/
theorem lower_snd_pin (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCount sk fut st) {hh : Nat}
    (hhr : hh < sk.rootH) :
    sndCount (lowerOut (wpk hh)) st.out
        + futLen sk fut (walkIdx sk hh) (lowerOut (wpk hh)) true
      = sk.dsBefore hh (sk.stageLen hh) := by
  have hp := walk_snd_pin sk hwf h hhr (lowerOut (wpk hh)) rfl
  have hlen : (proj (lowerOut (wpk hh)) true
      (walkEvents sk (wpk hh))).length
      = sk.dsBefore hh (sk.stageLen hh) := by
    rw [walk_lower_total]
    simp [canon, wpk]
  omega

/-- The wire pin: emitted wires plus the future's share is the
stage's kid total. -/
theorem wire_snd_pin (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCount sk fut st) {hh : Nat}
    (hhr : hh < sk.rootH) :
    sndCount (wireOut (wpk hh)) st.out
        + futLen sk fut (walkIdx sk hh) (wireOut (wpk hh)) true
      = sk.wiresBefore hh (sk.stageLen hh) := by
  have hM : sndOwner sk (wireOut (wpk hh)) = walkIdx sk hh := by
    have hwire : wireOut (wpk hh) = Chan.wire (wpk hh).1 hh := rfl
    rw [hwire]
    simp only [sndOwner]
    rw [if_neg (by omega)]
  have hp := walk_snd_pin sk hwf h hhr (wireOut (wpk hh)) hM
  have hlen : (proj (wireOut (wpk hh)) true
      (walkEvents sk (wpk hh))).length
      = sk.wiresBefore hh (sk.stageLen hh) := by
    rw [walk_wire_total]
    simp [canon, wpk]
  omega

-- =================================== position facts for the ascent

/-- `schedulable` in per-scope form: no stage counts more than
`capLevel + 2` D children. -/
theorem schedulable_dOf (hsched : sk.schedulable = true) {g : Nat}
    {A : Nat} (hA : A < sk.stageLen g) :
    sk.dOf g (sk.stageScope g A) ≤ sk.capLevel + 2 := by
  have hmem : sk.stageScope g A ∈ sk.scopesAt (g + 1) := by
    unfold Skel.stageScope
    have hlen : A < (sk.stageScopes g).length := hA
    rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hlen]
    exact List.getElem_mem _
  have hs : sk.stageScope g A < sk.scopes.length := (mem_scopesAt hmem).1
  unfold Skel.schedulable at hsched
  rw [List.all_eq_true] at hsched
  have hd := hsched (sk.stageScope g A) (List.mem_range.2 hs)
  simp only [decide_eq_true_eq] at hd
  unfold Skel.dOf
  split
  · omega
  · exact hd

/-- `P1` from a mid-scope position: with `schedulable`, a walk's
resolution sends overhang its summary allocation line by at most the
level window plus one. -/
theorem p1_of_position (hsched : sk.schedulable = true)
    {st : MState} {p : Party} {g A δ σ : Nat}
    (hA : A < sk.stageLen g)
    (hup : sndCount (Chan.upper p g) st.out = A + σ)
    (hlow : sndCount (Chan.lower p g) st.out
      = sk.dsBefore g A + δ + 1)
    (hcase : (σ = 0 ∧ δ + 2 ≤ sk.dOf g (sk.stageScope g A))
      ∨ (σ = 1 ∧ δ + 1 = sk.dOf g (sk.stageScope g A))) :
    sndCount (Chan.lower p g) st.out
      ≤ sk.dsBefore g (sndCount (Chan.upper p g) st.out)
        + sk.capLevel + 1 := by
  have hd := schedulable_dOf sk hsched hA
  rcases hcase with ⟨hσ, hδ⟩ | ⟨hσ, hδ⟩
  · subst hσ
    have hup' : sndCount (Chan.upper p g) st.out = A := by
      omega
    have hgoal : sk.dsBefore g (sndCount (Chan.upper p g) st.out)
        = sk.dsBefore g A := by
      rw [hup']
    rw [hgoal, hlow]
    omega
  · subst hσ
    have hgoal : sk.dsBefore g (sndCount (Chan.upper p g) st.out)
        = sk.dsBefore g A + sk.dOf g (sk.stageScope g A) := by
      rw [hup, dsBefore_succ sk hA]
    rw [hgoal, hlow]
    omega

/-- The splice identity: a post-splice ancestor's pends line meets its
resolution count — the `step` obligation of `SpineLink`. -/
theorem splice_link {st : MState} {p : Party} {g A : Nat}
    (hg1 : 1 ≤ g) (hasker : asks p (g + 1) = true)
    (hup : sndCount (Chan.upper p g) st.out = A + 1)
    (hlow : sndCount (Chan.lower p g) st.out = sk.dsBefore g (A + 1)) :
    sk.pendsBefore p (g + 1) (sndCount (Chan.upper p g) st.out)
      = sndCount (Chan.lower p g) st.out := by
  have hpb := pendsBefore_asker (sk := sk) hasker (by omega) (A + 1)
  rw [hup, hlow]
  simpa using hpb

-- ================================== the descent's two-stage step

/-- Two descent steps at once: an answerer-stage demand in cursor
form hands the same shape two stages down.

The demand `dsBefore (j+2) C` covers the answerer's resolutions, its
pends line converts to the wire cut two coordinates in
(`pendsBefore_answerer_ds`), the asker below consumes that against
the ancestor summaries, and its own pends line re-enters cursor form
(`pendsBefore_asker`) — the coverage telescope `C ↦ wiresBefore (j+2)
C ↦ wiresBefore (j+1) …` in one bite. -/
theorem descSupply_step (hwf : sk.wellFormed = true) {st : MState}
    {p : Party} {j C : Nat} (hna : asks p (j + 2) = false)
    (h1 : 1 ≤ j) (hjr : j + 2 < sk.rootH)
    (hres1 : sk.dsBefore (j + 2) C
      ≤ sndCount (Chan.lower p (j + 2)) st.out)
    (hres2 : sk.wiresBefore (j + 1) (sk.wiresBefore (j + 2) C)
      ≤ sndCount (Chan.upper p j) st.out)
    (hrec : DescSupply sk st p j
      (sk.dsBefore j
        (sk.wiresBefore (j + 1) (sk.wiresBefore (j + 2) C)))) :
    DescSupply sk st p (j + 2) (sk.dsBefore (j + 2) C) := by
  have hasker : asks p (j + 1) = true := by
    have hs := asks_succ p (j + 1)
    rw [show j + 1 + 1 = j + 2 from rfl, hna] at hs
    simpa using hs.symm
  refine ⟨?_, ?_⟩
  · rw [asmResChan_answerer hna]
    exact hres1
  · have hid1 : sk.pendsBefore p (j + 2) (sk.dsBefore (j + 2) C)
        = sk.wiresBefore (j + 1) (sk.wiresBefore (j + 2) C) :=
      pendsBefore_answerer_ds hwf hna (by omega) hjr C
    rw [hid1]
    refine ⟨?_, ?_⟩
    · rw [asmResChan_asker hasker]
      exact hres2
    · have hid2 : sk.pendsBefore p (j + 1)
          (sk.wiresBefore (j + 1) (sk.wiresBefore (j + 2) C))
          = sk.dsBefore j
              (sk.wiresBefore (j + 1) (sk.wiresBefore (j + 2) C)) :=
        pendsBefore_asker hasker (by omega) _
      rw [hid2]
      exact hrec

/-- The initiator descent's bottom: the stage-1 answerer hands its
wire cut to the absorber's two feeds. -/
theorem descSupply_base_I (hwf : sk.wellFormed = true) {st : MState}
    {C : Nat} (hjr : 1 < sk.rootH)
    (hres1 : sk.dsBefore 1 C
      ≤ sndCount (Chan.lower Party.I 1) st.out)
    (hwire : sk.wiresBefore 0 (sk.wiresBefore 1 C)
      ≤ sndCount (Chan.wire Party.R 0) st.out)
    (hreq : sk.wiresBefore 0 (sk.wiresBefore 1 C)
      ≤ sndCount Chan.leafRequests st.out) :
    DescSupply sk st Party.I 1 (sk.dsBefore 1 C) := by
  refine ⟨?_, ?_⟩
  · rw [asmResChan_answerer rfl]
    exact hres1
  · have hid : sk.pendsBefore Party.I 1 (sk.dsBefore 1 C)
        = sk.wiresBefore 0 (sk.wiresBefore 1 C) :=
      pendsBefore_answerer_ds hwf rfl (by omega) hjr C
    rw [hid]
    exact fun _ => ⟨hwire, hreq⟩

/-- The responder descent's bottom: the stage-2 answerer hands its
cut to the pend-free height-1 asker, below which nothing is owed. -/
theorem descSupply_base_R (hwf : sk.wellFormed = true) {st : MState}
    {C : Nat} (hjr : 2 < sk.rootH)
    (hres1 : sk.dsBefore 2 C
      ≤ sndCount (Chan.lower Party.R 2) st.out)
    (hres2 : sk.wiresBefore 1 (sk.wiresBefore 2 C)
      ≤ sndCount (Chan.upper Party.R 0) st.out) :
    DescSupply sk st Party.R 2 (sk.dsBefore 2 C) := by
  refine ⟨?_, ?_⟩
  · rw [asmResChan_answerer rfl]
    exact hres1
  · have hid : sk.pendsBefore Party.R 2 (sk.dsBefore 2 C)
        = sk.wiresBefore 1 (sk.wiresBefore 2 C) :=
      pendsBefore_answerer_ds hwf rfl (by omega) hjr C
    rw [hid]
    refine ⟨?_, ?_⟩
    · rw [asmResChan_asker rfl]
      exact hres2
    · have hz : sk.pendsBefore Party.R 1
          (sk.wiresBefore 1 (sk.wiresBefore 2 C)) = 0 :=
        pendsBefore_asker_one hwf rfl _
      rw [hz]
      exact fun hcon => Party.noConfusion hcon

/-- The root resolution is banked once the opener's future is past
it. -/
theorem rootres_pin (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCount sk fut st)
    (hsilent : futLen sk fut 1 Chan.rootres true = 0) :
    1 ≤ sndCount Chan.rootres st.out := by
  have hMlt : (1 : Nat) < manCount sk := by
    unfold manCount
    omega
  have hp := count_pin sk hwf h Chan.rootres true (M := 1) rfl hMlt
    (procs_ropen sk)
  rw [ropen_rootres_total] at hp
  rw [sndCount_eq_proj]
  simp only [List.length_cons, List.length_nil] at hp
  omega

end StreamingMirror.Sched
