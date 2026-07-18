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

Chain (d5, stage B): provides the futLen forms and count pins to
Site.lean's packages; SiteE.lean re-exports them to the E side through
its projection bridges. Map: Proofs/Map.lean.
-/
import StreamingMirror.Proofs.Sched.Weave.Ctx

namespace StreamingMirror.Sched

open Model

variable (sk : Skel)

-- ================================================== the count pin

/-- The future's share of an owner's channel-side. -/
def futLen (fut : List Ev) (M : Nat) (c : Chan) (b : Bool) : Nat :=
  (proj c b (fut.filter fun e => evOwner sk e == M)).length

/-- THE COUNT PIN, family form: an owned channel-side's emitted count
plus the future's share is the whole-trace total, over any trace
family satisfying `FamOK`. -/
theorem count_pinP {P : List (List Ev)} (hfam : FamOK sk P)
    {fut : List Ev}
    {st : MState} (h : WCountP sk P fut st) (c : Chan) (b : Bool)
    {M : Nat} (hM : (if b then sndOwner sk c else rcvOwner sk c) = M)
    (hMlt : M < manCount sk)
    {T : List Ev} (hT : P[M]? = some T) :
    (proj c b st.out).length + futLen sk fut M c b
      = (proj c b T).length := by
  obtain ⟨r, pre, hr, hpre, hsub⟩ := cell_of_owner sk h hT
  have hout := out_proj_owner sk hfam h c b hM hT hr hpre hsub
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

/-- THE COUNT PIN, d5 corner: the family form at `procs`. -/
theorem count_pin (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCount sk fut st) (c : Chan) (b : Bool)
    {M : Nat} (hM : (if b then sndOwner sk c else rcvOwner sk c) = M)
    (hMlt : M < manCount sk)
    {T : List Ev} (hT : (procs sk)[M]? = some T) :
    (proj c b st.out).length + futLen sk fut M c b
      = (proj c b T).length :=
  count_pinP sk (famOK_procs sk hwf) h c b hM hMlt hT

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

-- ===================================== futLen over uniform segments

/-- `futLen` splits across a future's concatenation. -/
theorem futLen_append (fut₁ fut₂ : List Ev) (M : Nat) (c : Chan)
    (bb : Bool) :
    futLen sk (fut₁ ++ fut₂) M c bb
      = futLen sk fut₁ M c bb + futLen sk fut₂ M c bb := by
  unfold futLen
  rw [List.filter_append, proj_append, List.length_append]

/-- `futLen` through a computed owner filter: once layer D has read
the future's owner share off the worklist syntax, the share's
projection length is the whole `futLen`. -/
theorem futLen_of_filter {fut l : List Ev} {M : Nat}
    (hfil : fut.filter (fun e => evOwner sk e == M) = l)
    (c : Chan) (bb : Bool) :
    futLen sk fut M c bb = (proj c bb l).length := by
  unfold futLen
  rw [hfil]

private theorem seg_glue' (c : Chan) (bb : Bool) {x y z : Nat}
    (hxy : x ≤ y) (hyz : y ≤ z) :
    seg c bb x (y - x) ++ seg c bb y (z - y) = seg c bb x (z - x) := by
  have h1 : y = x + (y - x) := by omega
  have h2 : (y - x) + (z - y) = z - x := by omega
  calc seg c bb x (y - x) ++ seg c bb y (z - y)
      = seg c bb x (y - x) ++ seg c bb (x + (y - x)) (z - y) := by
        rw [← h1]
    _ = seg c bb x ((y - x) + (z - y)) := seg_append ..
    _ = seg c bb x (z - x) := by rw [h2]

private theorem chain_le' {g : Nat → Nat} :
    ∀ (n i : Nat), (∀ k, i ≤ k → k < i + n → g k ≤ g (k + 1)) →
      g i ≤ g (i + n)
  | 0, _, _ => Nat.le_refl _
  | n + 1, i, h => by
      have h1 : g i ≤ g (i + 1) := h i (Nat.le_refl i) (by omega)
      have h2 : g (i + 1) ≤ g (i + 1 + n) :=
        chain_le' n (i + 1) (fun k hk1 hk2 => h k (by omega) (by omega))
      rw [show i + 1 + n = i + (n + 1) from by omega] at h2
      exact Nat.le_trans h1 h2

/-- The window-anchored gluer: contiguous blocks whose projections
are consecutive segments concatenate to one segment over the window
(`proj_flatMap_seg` freed from its zero anchor). -/
theorem proj_flatMap_seg' {f : Nat → List Ev} {c : Chan}
    {bb : Bool} {g : Nat → Nat} :
    ∀ (n i : Nat),
      (∀ k, i ≤ k → k < i + n →
        proj c bb (f k) = seg c bb (g k) (g (k + 1) - g k)) →
      (∀ k, i ≤ k → k < i + n → g k ≤ g (k + 1)) →
      proj c bb ((List.range' i n).flatMap f)
        = seg c bb (g i) (g (i + n) - g i)
  | 0, i, _, _ => by
      simp [proj_nil, seg_zero]
  | n + 1, i, hseg, hmono => by
      have hrec := proj_flatMap_seg' n (i + 1)
        (fun k hk1 hk2 => hseg k (by omega) (by omega))
        (fun k hk1 hk2 => hmono k (by omega) (by omega))
      rw [List.range'_succ, List.flatMap_cons, proj_append,
        hseg i (Nat.le_refl i) (by omega), hrec,
        show i + 1 + n = i + (n + 1) from by omega]
      exact seg_glue' c bb (hmono i (Nat.le_refl i) (by omega))
        (by
          have h2 := chain_le' n (i + 1)
            (fun k hk1 hk2 => hmono k (by omega) (by omega))
          rw [show i + 1 + n = i + (n + 1) from by omega] at h2
          exact h2)

-- ============================== stage-window projections (walkSeg)

/-- A stage window's summary sends: one per scope, seqs `[a, b)`. -/
theorem walkSeg_proj_upper {h' a b : Nat} (hab : a ≤ b) :
    proj (upperOut (wpk h')) true (walkSeg sk h' a b)
      = seg (upperOut (wpk h')) true a (b - a) := by
  unfold walkSeg
  rw [proj_flatMap_seg' (g := fun k => k) (b - a) a
      (fun k hk1 hk2 => by
        rw [proj_block_upper, show k + 1 - k = 1 from by omega])
      (fun k _ _ => by omega),
    show a + (b - a) = b from by omega]

/-- A stage window's resolution sends: the `dsBefore` slice. -/
theorem walkSeg_proj_res {h' a b : Nat} (hab : a ≤ b)
    (hb : b ≤ sk.stageLen h') :
    proj (lowerOut (wpk h')) true (walkSeg sk h' a b)
      = seg (lowerOut (wpk h')) true (sk.dsBefore h' a)
          (sk.dsBefore h' b - sk.dsBefore h' a) := by
  unfold walkSeg
  rw [proj_flatMap_seg' (g := fun k => sk.dsBefore h' k) (b - a) a
      (fun k hk1 hk2 => by
        have hk : k < sk.stageLen h' := by omega
        exact proj_block_res sk (wpk h') hk)
      (fun k hk1 hk2 => by
        have hk : k < sk.stageLen h' := by omega
        have := dsBefore_succ sk (h := h') hk
        omega),
    show a + (b - a) = b from by omega]

/-- A stage window's wire sends: the `wiresBefore` slice. -/
theorem walkSeg_proj_wire {h' a b : Nat} (hab : a ≤ b)
    (hb : b ≤ sk.stageLen h') :
    proj (wireOut (wpk h')) true (walkSeg sk h' a b)
      = seg (wireOut (wpk h')) true (sk.wiresBefore h' a)
          (sk.wiresBefore h' b - sk.wiresBefore h' a) := by
  unfold walkSeg
  rw [proj_flatMap_seg' (g := fun k => sk.wiresBefore h' k) (b - a) a
      (fun k hk1 hk2 => by
        have hk : k < sk.stageLen h' := by omega
        exact proj_block_wire sk (wpk h') hk)
      (fun k hk1 hk2 => by
        have hk : k < sk.stageLen h' := by omega
        have := wiresBefore_succ sk (h := h') hk
        omega),
    show a + (b - a) = b from by omega]

/-- A stage window's query sends: the `qsBefore` slice. -/
theorem walkSeg_proj_q {h' a b : Nat} (hab : a ≤ b)
    (hb : b ≤ sk.stageLen h') :
    proj (askedOut (wpk h')) true (walkSeg sk h' a b)
      = seg (askedOut (wpk h')) true (sk.qsBefore h' a)
          (sk.qsBefore h' b - sk.qsBefore h' a) := by
  unfold walkSeg
  rw [proj_flatMap_seg' (g := fun k => sk.qsBefore h' k) (b - a) a
      (fun k hk1 hk2 => by
        have hk : k < sk.stageLen h' := by omega
        exact proj_block_q sk (wpk h') hk)
      (fun k hk1 hk2 => by
        have hk : k < sk.stageLen h' := by omega
        have := qsBefore_succ sk (h := h') hk
        omega),
    show a + (b - a) = b from by omega]

/-- A stage window's wire receives: one per scope, seqs `[a, b)`. -/
theorem walkSeg_proj_wireIn {h' a b : Nat} (hab : a ≤ b) :
    proj (wireIn (wpk h')) false (walkSeg sk h' a b)
      = seg (wireIn (wpk h')) false a (b - a) := by
  unfold walkSeg
  rw [proj_flatMap_seg' (g := fun k => k) (b - a) a
      (fun k hk1 hk2 => by
        rw [proj_block_wireIn, show k + 1 - k = 1 from by omega])
      (fun k _ _ => by omega),
    show a + (b - a) = b from by omega]

/-- A stage window's query receives: one per scope, seqs `[a, b)`. -/
theorem walkSeg_proj_askedIn {h' a b : Nat} (hab : a ≤ b) :
    proj (askedIn (wpk h')) false (walkSeg sk h' a b)
      = seg (askedIn (wpk h')) false a (b - a) := by
  unfold walkSeg
  rw [proj_flatMap_seg' (g := fun k => k) (b - a) a
      (fun k hk1 hk2 => by
        rw [proj_block_askedIn, show k + 1 - k = 1 from by omega])
      (fun k _ _ => by omega),
    show a + (b - a) = b from by omega]

-- ============================ futLen at a stage-window owner share

/-- `futLen` of the summaries a stage window still owes. -/
theorem futLen_walkSeg_upper {fut : List Ev} {h' a b : Nat}
    (hab : a ≤ b)
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk h')
      = walkSeg sk h' a b) :
    futLen sk fut (walkIdx sk h') (upperOut (wpk h')) true = b - a := by
  rw [futLen_of_filter sk hfil, walkSeg_proj_upper sk hab, seg_len]

/-- `futLen` of the resolutions a stage window still owes. -/
theorem futLen_walkSeg_res {fut : List Ev} {h' a b : Nat}
    (hab : a ≤ b) (hb : b ≤ sk.stageLen h')
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk h')
      = walkSeg sk h' a b) :
    futLen sk fut (walkIdx sk h') (lowerOut (wpk h')) true
      = sk.dsBefore h' b - sk.dsBefore h' a := by
  rw [futLen_of_filter sk hfil, walkSeg_proj_res sk hab hb, seg_len]

/-- `futLen` of the wires a stage window still owes. -/
theorem futLen_walkSeg_wire {fut : List Ev} {h' a b : Nat}
    (hab : a ≤ b) (hb : b ≤ sk.stageLen h')
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk h')
      = walkSeg sk h' a b) :
    futLen sk fut (walkIdx sk h') (wireOut (wpk h')) true
      = sk.wiresBefore h' b - sk.wiresBefore h' a := by
  rw [futLen_of_filter sk hfil, walkSeg_proj_wire sk hab hb, seg_len]

/-- `futLen` of the queries a stage window still owes. -/
theorem futLen_walkSeg_q {fut : List Ev} {h' a b : Nat}
    (hab : a ≤ b) (hb : b ≤ sk.stageLen h')
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk h')
      = walkSeg sk h' a b) :
    futLen sk fut (walkIdx sk h') (askedOut (wpk h')) true
      = sk.qsBefore h' b - sk.qsBefore h' a := by
  rw [futLen_of_filter sk hfil, walkSeg_proj_q sk hab hb, seg_len]

/-- `futLen` of the wire receives a stage window still owes. -/
theorem futLen_walkSeg_wireIn {fut : List Ev} {h' a b : Nat}
    (hab : a ≤ b)
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk h')
      = walkSeg sk h' a b) :
    futLen sk fut (walkIdx sk h') (wireIn (wpk h')) false = b - a := by
  rw [futLen_of_filter sk hfil, walkSeg_proj_wireIn sk hab, seg_len]

/-- `futLen` of the query receives a stage window still owes. -/
theorem futLen_walkSeg_askedIn {fut : List Ev} {h' a b : Nat}
    (hab : a ≤ b)
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk h')
      = walkSeg sk h' a b) :
    futLen sk fut (walkIdx sk h') (askedIn (wpk h')) false = b - a := by
  rw [futLen_of_filter sk hfil, walkSeg_proj_askedIn sk hab, seg_len]

-- ======================= mid-scope projections (splicedChunk runs)

private theorem proj_schunk_wire (h k : Nat) (lastD : Option Nat)
    (i : Nat) :
    proj (wireOut (wpk h)) true (splicedChunk sk h k lastD i)
      = seg (wireOut (wpk h)) true (sk.wiresBefore h k + i) 1 := by
  unfold splicedChunk
  rw [proj_cons_self, seg_one]
  by_cases hD : sk.childIsD h (sk.stageScope h k) i
  · rw [if_pos hD, proj_cons_ne_chan (by simp [lowerOut, wireOut]),
      proj_append]
    have hspl : proj (wireOut (wpk h)) true
        (if lastD == some i
          then [((upperOut (wpk h), true, k) : Ev)] else []) = [] := by
      split
      · rw [proj_cons_ne_chan (by simp [upperOut, wireOut]), proj_nil]
      · rfl
    have hq : proj (wireOut (wpk h)) true (chunkQ sk h k i) = [] :=
      proj_eq_nil fun e he h1 _ => by
        unfold chunkQ at he
        obtain ⟨t, -, rfl⟩ := List.mem_map.1 he
        simp only [askedOut, wireOut] at h1
        split at h1 <;> exact Chan.noConfusion h1
    rw [hspl, hq]
    rfl
  · rw [if_neg hD, proj_nil]

private theorem proj_schunk_res (h k : Nat) (lastD : Option Nat)
    (i : Nat) :
    proj (lowerOut (wpk h)) true (splicedChunk sk h k lastD i)
      = seg (lowerOut (wpk h)) true
          (sk.dsBefore h k + dRank sk (wpk h) k i)
          (dRank sk (wpk h) k (i + 1) - dRank sk (wpk h) k i) := by
  have hds := dRank_succ sk (wpk h) k i
  rw [show sk.childIsD (wpk h).2 (sk.stageScope (wpk h).2 k) i
      = sk.childIsD h (sk.stageScope h k) i from rfl] at hds
  unfold splicedChunk
  rw [proj_cons_ne_chan (by simp [wireOut, lowerOut])]
  by_cases hD : sk.childIsD h (sk.stageScope h k) i
  · rw [hD, if_pos rfl] at hds
    rw [show dRank sk (wpk h) k (i + 1) - dRank sk (wpk h) k i = 1
        from by omega]
    rw [if_pos hD, proj_cons_self, seg_one]
    have hspl : proj (lowerOut (wpk h)) true
        (if lastD == some i
          then [((upperOut (wpk h), true, k) : Ev)] else []) = [] := by
      split
      · rw [proj_cons_ne_chan (by simp [upperOut, lowerOut]), proj_nil]
      · rfl
    have hq : proj (lowerOut (wpk h)) true (chunkQ sk h k i) = [] :=
      proj_eq_nil fun e he h1 _ => by
        unfold chunkQ at he
        obtain ⟨t, -, rfl⟩ := List.mem_map.1 he
        simp only [askedOut, lowerOut] at h1
        split at h1 <;> exact Chan.noConfusion h1
    rw [proj_append, hspl, hq]
    rfl
  · have hDf : sk.childIsD h (sk.stageScope h k) i = false := by
      simpa using hD
    rw [hDf, if_neg (by simp)] at hds
    rw [show dRank sk (wpk h) k (i + 1) - dRank sk (wpk h) k i = 0
        from by omega]
    rw [if_neg hD, proj_nil, seg_zero]

private theorem proj_schunk_q (h k : Nat) (lastD : Option Nat)
    (i : Nat) :
    proj (askedOut (wpk h)) true (splicedChunk sk h k lastD i)
      = seg (askedOut (wpk h)) true
          (sk.qsBefore h k + qSum sk (wpk h) k i)
          (qSum sk (wpk h) k (i + 1) - qSum sk (wpk h) k i) := by
  have hqs := qSum_succ sk (wpk h) k i
  rw [show sk.qCount (wpk h).2 (sk.stageScope (wpk h).2 k) i
      = sk.qCount h (sk.stageScope h k) i from rfl] at hqs
  have hw : qSum sk (wpk h) k (i + 1) - qSum sk (wpk h) k i
      = sk.qCount h (sk.stageScope h k) i := by omega
  rw [hw]
  unfold splicedChunk
  rw [proj_cons_ne_chan (by
    unfold wireOut askedOut
    split <;> simp)]
  by_cases hD : sk.childIsD h (sk.stageScope h k) i
  · rw [if_pos hD, proj_cons_ne_chan (by
      unfold lowerOut askedOut
      split <;> simp)]
    have hspl : proj (askedOut (wpk h)) true
        (if lastD == some i
          then [((upperOut (wpk h), true, k) : Ev)] else []) = [] := by
      split
      · rw [proj_cons_ne_chan (by
          unfold upperOut askedOut
          split <;> simp), proj_nil]
      · rfl
    have hcq : chunkQ sk h k i
        = seg (askedOut (wpk h)) true
            (sk.qsBefore h k + qSum sk (wpk h) k i)
            (sk.qCount h (sk.stageScope h k) i) := rfl
    rw [proj_append, hspl, List.nil_append, hcq, proj_seg_self]
  · have hDf : sk.childIsD h (sk.stageScope h k) i = false := by
      simpa using hD
    have hq0 : sk.qCount h (sk.stageScope h k) i = 0 := by
      unfold Skel.qCount
      rw [if_pos (by simp [hDf])]
    rw [if_neg hD, proj_nil, hq0, seg_zero]

private theorem proj_schunk_upper (h k i : Nat) :
    proj (upperOut (wpk h)) true
        (splicedChunk sk h k (lastDOf sk h k) i)
      = if lastDOf sk h k == some i
          then [((upperOut (wpk h), true, k) : Ev)] else [] := by
  unfold splicedChunk
  rw [proj_cons_ne_chan (by simp [wireOut, upperOut])]
  by_cases hD : sk.childIsD h (sk.stageScope h k) i
  · rw [if_pos hD, proj_cons_ne_chan (by simp [lowerOut, upperOut]),
      proj_append]
    have hq : proj (upperOut (wpk h)) true (chunkQ sk h k i) = [] :=
      proj_eq_nil fun e he h1 _ => by
        unfold chunkQ at he
        obtain ⟨t, -, rfl⟩ := List.mem_map.1 he
        simp only [askedOut, upperOut] at h1
        split at h1 <;> exact Chan.noConfusion h1
    rw [hq, List.append_nil]
    split
    · rw [proj_cons_self, proj_nil]
    · rfl
  · have hDf : sk.childIsD h (sk.stageScope h k) i = false := by
      simpa using hD
    have hne : (lastDOf sk h k == some i) = false := by
      cases hlast : lastDOf sk h k with
      | none => rfl
      | some j =>
          by_cases hji : j = i
          · subst hji
            exact absurd (lastDOf_isD sk hlast).1 (by simp [hDf])
          · simpa using hji
    rw [if_neg hD, proj_nil, hne]
    rfl

/-- A mid-scope kid suffix's wire sends: one per remaining slot. -/
theorem chunks_proj_wire (h k : Nat) (lastD : Option Nat) (m i : Nat) :
    proj (wireOut (wpk h)) true
        ((List.range' i m).flatMap (splicedChunk sk h k lastD))
      = seg (wireOut (wpk h)) true (sk.wiresBefore h k + i) m := by
  rw [proj_flatMap_seg' (g := fun i' => sk.wiresBefore h k + i') m i
      (fun i' _ _ => by
        rw [proj_schunk_wire sk h k lastD i',
          show sk.wiresBefore h k + (i' + 1) - (sk.wiresBefore h k + i')
            = 1 from by omega])
      (fun i' _ _ => by omega),
    show sk.wiresBefore h k + (i + m) - (sk.wiresBefore h k + i) = m
      from by omega]

/-- A mid-scope kid suffix's resolution sends: the `dRank` slice. -/
theorem chunks_proj_res (h k : Nat) (lastD : Option Nat) (m i : Nat) :
    proj (lowerOut (wpk h)) true
        ((List.range' i m).flatMap (splicedChunk sk h k lastD))
      = seg (lowerOut (wpk h)) true
          (sk.dsBefore h k + dRank sk (wpk h) k i)
          (dRank sk (wpk h) k (i + m) - dRank sk (wpk h) k i) := by
  rw [proj_flatMap_seg'
      (g := fun i' => sk.dsBefore h k + dRank sk (wpk h) k i') m i
      (fun i' _ _ => by
        rw [proj_schunk_res sk h k lastD i',
          show sk.dsBefore h k + dRank sk (wpk h) k (i' + 1)
              - (sk.dsBefore h k + dRank sk (wpk h) k i')
            = dRank sk (wpk h) k (i' + 1) - dRank sk (wpk h) k i'
            from by omega])
      (fun i' _ _ => by
        have hds := dRank_succ sk (wpk h) k i'
        split at hds <;> omega),
    show sk.dsBefore h k + dRank sk (wpk h) k (i + m)
        - (sk.dsBefore h k + dRank sk (wpk h) k i)
      = dRank sk (wpk h) k (i + m) - dRank sk (wpk h) k i
      from by omega]

/-- A mid-scope kid suffix's query sends: the `qSum` slice. -/
theorem chunks_proj_q (h k : Nat) (lastD : Option Nat) (m i : Nat) :
    proj (askedOut (wpk h)) true
        ((List.range' i m).flatMap (splicedChunk sk h k lastD))
      = seg (askedOut (wpk h)) true
          (sk.qsBefore h k + qSum sk (wpk h) k i)
          (qSum sk (wpk h) k (i + m) - qSum sk (wpk h) k i) := by
  rw [proj_flatMap_seg'
      (g := fun i' => sk.qsBefore h k + qSum sk (wpk h) k i') m i
      (fun i' _ _ => by
        rw [proj_schunk_q sk h k lastD i',
          show sk.qsBefore h k + qSum sk (wpk h) k (i' + 1)
              - (sk.qsBefore h k + qSum sk (wpk h) k i')
            = qSum sk (wpk h) k (i' + 1) - qSum sk (wpk h) k i'
            from by omega])
      (fun i' _ _ => by
        have hqs := qSum_succ sk (wpk h) k i'
        omega),
    show sk.qsBefore h k + qSum sk (wpk h) k (i + m)
        - (sk.qsBefore h k + qSum sk (wpk h) k i)
      = qSum sk (wpk h) k (i + m) - qSum sk (wpk h) k i
      from by omega]

private theorem chunks_upper_some {h k j : Nat}
    (hlast : lastDOf sk h k = some j) :
    ∀ (m i : Nat),
      proj (upperOut (wpk h)) true
        ((List.range' i m).flatMap
          (splicedChunk sk h k (lastDOf sk h k)))
      = if i ≤ j ∧ j < i + m
          then [((upperOut (wpk h), true, k) : Ev)] else []
  | 0, i => by
      rw [if_neg (by omega)]
      rfl
  | m + 1, i => by
      rw [List.range'_succ, List.flatMap_cons, proj_append,
        proj_schunk_upper sk h k i, chunks_upper_some hlast m (i + 1)]
      by_cases hji : j = i
      · subst hji
        rw [show (lastDOf sk h k == some j) = true from by
            rw [hlast]; simp,
          if_pos rfl, if_neg (by omega), if_pos (by omega)]
        rfl
      · rw [show (lastDOf sk h k == some i) = false from by
            rw [hlast]; simpa using hji,
          if_neg (by simp), List.nil_append]
        by_cases hc : i + 1 ≤ j ∧ j < i + 1 + m
        · rw [if_pos hc, if_pos (by omega)]
        · rw [if_neg hc, if_neg (by omega)]

private theorem chunks_upper_nosplice {h k : Nat}
    (hlast : lastDOf sk h k = none) :
    ∀ (m i : Nat),
      proj (upperOut (wpk h)) true
        ((List.range' i m).flatMap
          (splicedChunk sk h k (lastDOf sk h k)))
      = []
  | 0, _ => rfl
  | m + 1, i => by
      rw [List.range'_succ, List.flatMap_cons, proj_append,
        proj_schunk_upper sk h k i, chunks_upper_nosplice hlast m (i + 1),
        show (lastDOf sk h k == some i) = false from by rw [hlast]; rfl,
        if_neg (by simp), List.nil_append]

/-- A kid suffix still holding the splice: exactly the one parent
summary rides it. -/
theorem chunks_proj_upper_covered {h k j i m : Nat}
    (hlast : lastDOf sk h k = some j) (hij : i ≤ j) (hjm : j < i + m) :
    proj (upperOut (wpk h)) true
        ((List.range' i m).flatMap
          (splicedChunk sk h k (lastDOf sk h k)))
      = [((upperOut (wpk h), true, k) : Ev)] := by
  rw [chunks_upper_some sk hlast m i, if_pos ⟨hij, hjm⟩]

/-- A kid suffix past the splice: the parent summary has left. -/
theorem chunks_proj_upper_past {h k j i m : Nat}
    (hlast : lastDOf sk h k = some j) (hji : j < i) :
    proj (upperOut (wpk h)) true
        ((List.range' i m).flatMap
          (splicedChunk sk h k (lastDOf sk h k)))
      = [] := by
  rw [chunks_upper_some sk hlast m i, if_neg (by omega)]

/-- An undisputed scope's kid suffix: no summary rides the kids (it
was emitted in the prologue). -/
theorem chunks_proj_upper_none {h k i m : Nat}
    (hlast : lastDOf sk h k = none) :
    proj (upperOut (wpk h)) true
        ((List.range' i m).flatMap
          (splicedChunk sk h k (lastDOf sk h k)))
      = [] :=
  chunks_upper_nosplice sk hlast m i

-- ========================= futLen at a mid-scope kid-suffix share

/-- `futLen` of the wires a mid-scope kid suffix still owes. -/
theorem futLen_chunks_wire {fut : List Ev} {h k i : Nat}
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk h)
      = (List.range' i (sk.nChildren h (sk.stageScope h k) - i)).flatMap
          (splicedChunk sk h k (lastDOf sk h k))) :
    futLen sk fut (walkIdx sk h) (wireOut (wpk h)) true
      = sk.nChildren h (sk.stageScope h k) - i := by
  rw [futLen_of_filter sk hfil, chunks_proj_wire, seg_len]

/-- `futLen` of the resolutions a mid-scope kid suffix still owes. -/
theorem futLen_chunks_res {fut : List Ev} {h k i : Nat}
    (hi : i ≤ sk.nChildren h (sk.stageScope h k))
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk h)
      = (List.range' i (sk.nChildren h (sk.stageScope h k) - i)).flatMap
          (splicedChunk sk h k (lastDOf sk h k))) :
    futLen sk fut (walkIdx sk h) (lowerOut (wpk h)) true
      = sk.dOf h (sk.stageScope h k) - dRank sk (wpk h) k i := by
  have hdt : dRank sk (wpk h) k (sk.nChildren h (sk.stageScope h k))
      = sk.dOf h (sk.stageScope h k) := dRank_total sk (wpk h) k
  rw [futLen_of_filter sk hfil, chunks_proj_res, seg_len,
    show i + (sk.nChildren h (sk.stageScope h k) - i)
      = sk.nChildren h (sk.stageScope h k) from by omega,
    hdt]

/-- `futLen` of the queries a mid-scope kid suffix still owes. -/
theorem futLen_chunks_q {fut : List Ev} {h k i : Nat}
    (hi : i ≤ sk.nChildren h (sk.stageScope h k))
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk h)
      = (List.range' i (sk.nChildren h (sk.stageScope h k) - i)).flatMap
          (splicedChunk sk h k (lastDOf sk h k))) :
    futLen sk fut (walkIdx sk h) (askedOut (wpk h)) true
      = sk.qOf h (sk.stageScope h k) - qSum sk (wpk h) k i := by
  have hqt : qSum sk (wpk h) k (sk.nChildren h (sk.stageScope h k))
      = sk.qOf h (sk.stageScope h k) := qSum_total sk (wpk h) k
  rw [futLen_of_filter sk hfil, chunks_proj_q, seg_len,
    show i + (sk.nChildren h (sk.stageScope h k) - i)
      = sk.nChildren h (sk.stageScope h k) from by omega,
    hqt]

/-- `futLen` of a still-spliced parent summary: exactly one. -/
theorem futLen_chunks_upper_covered {fut : List Ev} {h k j i : Nat}
    (hlast : lastDOf sk h k = some j) (hij : i ≤ j)
    (hjm : j < i + (sk.nChildren h (sk.stageScope h k) - i))
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk h)
      = (List.range' i (sk.nChildren h (sk.stageScope h k) - i)).flatMap
          (splicedChunk sk h k (lastDOf sk h k))) :
    futLen sk fut (walkIdx sk h) (upperOut (wpk h)) true = 1 := by
  rw [futLen_of_filter sk hfil,
    chunks_proj_upper_covered sk hlast hij hjm]
  rfl

/-- `futLen` of a departed parent summary: none left. -/
theorem futLen_chunks_upper_past {fut : List Ev} {h k j i : Nat}
    (hlast : lastDOf sk h k = some j) (hji : j < i)
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk h)
      = (List.range' i (sk.nChildren h (sk.stageScope h k) - i)).flatMap
          (splicedChunk sk h k (lastDOf sk h k))) :
    futLen sk fut (walkIdx sk h) (upperOut (wpk h)) true = 0 := by
  rw [futLen_of_filter sk hfil, chunks_proj_upper_past sk hlast hji]
  rfl

/-- `futLen` of an undisputed scope's summary over its kid suffix:
none. -/
theorem futLen_chunks_upper_none {fut : List Ev} {h k i : Nat}
    (hlast : lastDOf sk h k = none)
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk h)
      = (List.range' i (sk.nChildren h (sk.stageScope h k) - i)).flatMap
          (splicedChunk sk h k (lastDOf sk h k))) :
    futLen sk fut (walkIdx sk h) (upperOut (wpk h)) true = 0 := by
  rw [futLen_of_filter sk hfil, chunks_proj_upper_none sk hlast]
  rfl

-- ==================================== the query pin and root silence

/-- A walk's query sends are the canonical run over its stage's query
total. -/
theorem walk_asked_total (pk : Party × Nat) :
    proj (askedOut pk) true (walkEvents sk pk)
      = canon (askedOut pk) true (sk.qsBefore pk.2 (sk.stageLen pk.2)) := by
  unfold walkEvents
  exact proj_flatMap_canon (g := sk.qsBefore pk.2) _ rfl
    (fun k hk => proj_block_q sk pk hk)
    (fun k hk => by rw [qsBefore_succ sk hk]; omega)

/-- The query pin: emitted queries plus the future's share is the
stage's query total.

`h1` is essential: `askedOut (wpk 0)` is `leafRequests`, owned by the
stage-1 walk — the leaf stage never emits queries of its own. -/
theorem asked_snd_pin (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCount sk fut st) {hh : Nat}
    (h1 : 1 ≤ hh) (hhr : hh < sk.rootH) :
    sndCount (askedOut (wpk hh)) st.out
        + futLen sk fut (walkIdx sk hh) (askedOut (wpk hh)) true
      = sk.qsBefore hh (sk.stageLen hh) := by
  have hM : sndOwner sk (askedOut (wpk hh)) = walkIdx sk hh := by
    show sndOwner sk (if (wpk hh).2 < 2 then Chan.leafRequests
      else Chan.asked (wpk hh).1 ((wpk hh).2 - 2)) = walkIdx sk hh
    rw [show (wpk hh).2 = hh from rfl]
    by_cases h2 : hh < 2
    · rw [if_pos h2]
      have hone : hh = 1 := by omega
      rw [hone]
      rfl
    · rw [if_neg h2]
      simp only [sndOwner]
      rw [if_neg (by rintro ⟨-, habs⟩; omega),
        if_neg (by rintro ⟨-, habs⟩; omega),
        show hh - 2 + 2 = hh from by omega]
  have hp := walk_snd_pin sk hwf h hhr (askedOut (wpk hh)) hM
  have hlen : (proj (askedOut (wpk hh)) true
      (walkEvents sk (wpk hh))).length
      = sk.qsBefore hh (sk.stageLen hh) := by
    rw [walk_asked_total]
    simp [canon, wpk]
  omega

/-- The opener's future past the root queries carries no root
resolution: `rootres` is banked before any window site fires. -/
theorem feed_rootres_silent {fut : List Ev} {i₀ : Nat}
    (hfeed : fut.filter (fun e => evOwner sk e == 1)
      = ((ropenEvents sk).drop 3).drop i₀) :
    futLen sk fut 1 Chan.rootres true = 0 := by
  rw [futLen_of_filter sk hfeed]
  have hnil : proj Chan.rootres true (((ropenEvents sk).drop 3).drop i₀)
      = [] :=
    proj_eq_nil fun e he h1 _ => by
      have hmem : e ∈ (ropenEvents sk).drop 3 := List.mem_of_mem_drop he
      unfold ropenEvents at hmem
      simp only [List.drop_succ_cons, List.drop_zero] at hmem
      obtain ⟨j, -, rfl⟩ := List.mem_map.1 hmem
      exact Chan.noConfusion h1
  rw [hnil]
  rfl

-- =============================== the query chunk's mid-feed windows

/-- A partially delivered query chunk projects nil off its own
channel-side. -/
theorem chunkQ_drop_proj_ne (h k i t : Nat) {c : Chan} {b : Bool}
    (hc : ¬(c = askedOut (wpk h) ∧ b = true)) :
    proj c b ((chunkQ sk h k i).drop t) = [] :=
  proj_eq_nil fun e he h1 h2 => by
    have hmem : e ∈ chunkQ sk h k i := List.mem_of_mem_drop he
    unfold chunkQ at hmem
    obtain ⟨s, -, rfl⟩ := List.mem_map.1 hmem
    exact hc ⟨h1.symm, h2.symm⟩

/-- A whole query chunk projects nil off its own channel-side. -/
theorem chunkQ_proj_ne (h k i : Nat) {c : Chan} {b : Bool}
    (hc : ¬(c = askedOut (wpk h) ∧ b = true)) :
    proj c b (chunkQ sk h k i) = [] := by
  have hd := chunkQ_drop_proj_ne sk h k i 0 hc
  rwa [List.drop_zero] at hd

/-- A partially delivered query chunk's remaining queries: the seg
from the feed cursor to the chunk's end. -/
theorem chunkQ_drop_proj_q (h k i : Nat) {t : Nat}
    (ht : t ≤ sk.qCount h (sk.stageScope h k) i) :
    proj (askedOut (wpk h)) true ((chunkQ sk h k i).drop t)
      = seg (askedOut (wpk h)) true
          (sk.qsBefore h k + qSum sk (wpk h) k i + t)
          (sk.qCount h (sk.stageScope h k) i - t) := by
  have hcq : chunkQ sk h k i
      = seg (askedOut (wpk h)) true
          (sk.qsBefore h k + qSum sk (wpk h) k i)
          (sk.qCount h (sk.stageScope h k) i) := rfl
  rw [hcq, seg_drop _ _ _ _ _ ht, proj_seg_self]

-- ================================= the in-flight ancestor's pins

/-- An in-flight ancestor's future summary share: everything past
its current scope, plus the current summary unless the splice
already fired at the in-flight slot.

The feed cursor `t` cancels — the pin is insensitive to feed
progress, which is what keeps the ancestor form valid at every site
inside the in-flight kid's subtree. -/
theorem futLen_anc_upper {fut : List Ev} {g A jD t : Nat}
    (hA : A < sk.stageLen g)
    (hjD : jD < sk.nChildren g (sk.stageScope g A))
    (hD : sk.childIsD g (sk.stageScope g A) jD = true)
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk g)
      = (chunkQ sk g A jD).drop t
        ++ (List.range' (jD + 1)
              (sk.nChildren g (sk.stageScope g A) - (jD + 1))).flatMap
             (splicedChunk sk g A (lastDOf sk g A))
        ++ walkSeg sk g (A + 1) (sk.stageLen g)) :
    futLen sk fut (walkIdx sk g) (upperOut (wpk g)) true
      = sk.stageLen g - A
        - (if lastDOf sk g A == some jD then 1 else 0) := by
  obtain ⟨j, hlast, hij⟩ := lastDOf_isSome_of_D sk hD hjD
  have hjn := (lastDOf_isD sk hlast).2
  have hne : proj (upperOut (wpk g)) true
      ((chunkQ sk g A jD).drop t) = [] :=
    chunkQ_drop_proj_ne sk g A jD t (by
      rintro ⟨hc, -⟩
      simp only [askedOut, upperOut] at hc
      split at hc <;> exact Chan.noConfusion hc)
  rw [futLen_of_filter sk hfil, proj_append, proj_append, hne,
    walkSeg_proj_upper sk (show A + 1 ≤ sk.stageLen g by omega)]
  by_cases hjj : j = jD
  · subst hjj
    rw [chunks_proj_upper_past sk hlast (show j < j + 1 by omega),
      hlast, if_pos (beq_self_eq_true (some j))]
    simp only [List.nil_append, seg_len]
    omega
  · rw [chunks_proj_upper_covered sk hlast (show jD + 1 ≤ j by omega)
        (show j < jD + 1 + (sk.nChildren g (sk.stageScope g A)
          - (jD + 1)) by omega),
      hlast, if_neg (by simp [hjj])]
    simp only [List.nil_append, List.length_append, List.length_nil,
      List.length_cons, seg_len]
    omega

/-- An in-flight ancestor's future resolution share: everything past
the in-flight slot's own resolution. -/
theorem futLen_anc_lower {fut : List Ev} {g A jD t : Nat}
    (hA : A < sk.stageLen g)
    (hjD : jD < sk.nChildren g (sk.stageScope g A))
    (hD : sk.childIsD g (sk.stageScope g A) jD = true)
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk g)
      = (chunkQ sk g A jD).drop t
        ++ (List.range' (jD + 1)
              (sk.nChildren g (sk.stageScope g A) - (jD + 1))).flatMap
             (splicedChunk sk g A (lastDOf sk g A))
        ++ walkSeg sk g (A + 1) (sk.stageLen g)) :
    futLen sk fut (walkIdx sk g) (lowerOut (wpk g)) true
      = sk.dsBefore g (sk.stageLen g)
        - (sk.dsBefore g A + dRank sk (wpk g) A jD + 1) := by
  have hne : proj (lowerOut (wpk g)) true
      ((chunkQ sk g A jD).drop t) = [] :=
    chunkQ_drop_proj_ne sk g A jD t (by
      rintro ⟨hc, -⟩
      simp only [askedOut, lowerOut] at hc
      split at hc <;> exact Chan.noConfusion hc)
  rw [futLen_of_filter sk hfil, proj_append, proj_append, hne,
    chunks_proj_res sk g A (lastDOf sk g A) _ (jD + 1),
    walkSeg_proj_res sk (show A + 1 ≤ sk.stageLen g by omega)
      (Nat.le_refl _)]
  simp only [List.nil_append, List.length_append, seg_len]
  have hidx : jD + 1 + (sk.nChildren g (sk.stageScope g A) - (jD + 1))
      = sk.nChildren g (sk.stageScope g A) := by omega
  rw [hidx]
  have htot : dRank sk (wpk g) A (sk.nChildren g (sk.stageScope g A))
      = sk.dOf g (sk.stageScope g A) := dRank_total sk (wpk g) A
  have hds := dRank_succ sk (wpk g) A jD
  rw [show sk.childIsD (wpk g).2 (sk.stageScope (wpk g).2 A) jD
      = sk.childIsD g (sk.stageScope g A) jD from rfl, hD,
    if_pos rfl] at hds
  have hsc : sk.dsBefore g (A + 1)
      = sk.dsBefore g A + sk.dOf g (sk.stageScope g A) :=
    dsBefore_succ sk hA
  have hmono : sk.dsBefore g (A + 1) ≤ sk.dsBefore g (sk.stageLen g) :=
    dsBefore_mono sk g (by omega)
  have hle : dRank sk (wpk g) A jD + 1 ≤ sk.dOf g (sk.stageScope g A) :=
    dRank_succ_le_dOf sk (wpk g) hjD hD
  omega

-- ==================================== the emission sites' own pins

/-- The prologue summary site: an undisputed scope's future summary
share spans the scope and everything past it. -/
theorem futLen_site_upper_prologue {fut : List Ev} {h k : Nat}
    (hk : k < sk.stageLen h)
    (hlast : lastDOf sk h k = none)
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk h)
      = ((upperOut (wpk h), true, k) : Ev)
          :: ((List.range' 0
                (sk.nChildren h (sk.stageScope h k))).flatMap
                (splicedChunk sk h k (lastDOf sk h k))
              ++ walkSeg sk h (k + 1) (sk.stageLen h))) :
    futLen sk fut (walkIdx sk h) (upperOut (wpk h)) true
      = sk.stageLen h - k := by
  rw [futLen_of_filter sk hfil, proj_cons_self, proj_append,
    chunks_proj_upper_none sk hlast,
    walkSeg_proj_upper sk (show k + 1 ≤ sk.stageLen h by omega)]
  simp only [List.nil_append, List.length_cons, seg_len]
  omega

/-- The splice summary site: a disputed scope's future summary share
after the last resolution, before its feed chunk. -/
theorem futLen_site_upper_splice {fut : List Ev} {h k jL : Nat}
    (hk : k < sk.stageLen h)
    (hlast : lastDOf sk h k = some jL)
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk h)
      = ((upperOut (wpk h), true, k) : Ev)
          :: (chunkQ sk h k jL
              ++ (List.range' (jL + 1)
                    (sk.nChildren h (sk.stageScope h k)
                      - (jL + 1))).flatMap
                   (splicedChunk sk h k (lastDOf sk h k))
              ++ walkSeg sk h (k + 1) (sk.stageLen h))) :
    futLen sk fut (walkIdx sk h) (upperOut (wpk h)) true
      = sk.stageLen h - k := by
  have hqne : proj (upperOut (wpk h)) true (chunkQ sk h k jL) = [] :=
    chunkQ_proj_ne sk h k jL (by
      rintro ⟨hc, -⟩
      simp only [askedOut, upperOut] at hc
      split at hc <;> exact Chan.noConfusion hc)
  rw [futLen_of_filter sk hfil, proj_cons_self, proj_append,
    proj_append, hqne,
    chunks_proj_upper_past sk hlast (show jL < jL + 1 by omega),
    walkSeg_proj_upper sk (show k + 1 ≤ sk.stageLen h by omega)]
  simp only [List.nil_append, List.length_cons, seg_len]
  omega

/-- The resolution site: the walk's future resolution share past the
emitted one, its in-range bound, and its still-unsent summary. -/
theorem futLen_site_lower {fut : List Ev} {h k i : Nat}
    (hk : k < sk.stageLen h)
    (hi : i < sk.nChildren h (sk.stageScope h k))
    (hD : sk.childIsD h (sk.stageScope h k) i = true)
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk h)
      = ((lowerOut (wpk h), true,
            sk.dsBefore h k + dRank sk (wpk h) k i) : Ev)
          :: ((if lastDOf sk h k == some i
                then [((upperOut (wpk h), true, k) : Ev)] else [])
              ++ chunkQ sk h k i
              ++ (List.range' (i + 1)
                    (sk.nChildren h (sk.stageScope h k)
                      - (i + 1))).flatMap
                   (splicedChunk sk h k (lastDOf sk h k))
              ++ walkSeg sk h (k + 1) (sk.stageLen h))) :
    futLen sk fut (walkIdx sk h) (lowerOut (wpk h)) true
        = sk.dsBefore h (sk.stageLen h)
          - (sk.dsBefore h k + dRank sk (wpk h) k i)
      ∧ sk.dsBefore h k + dRank sk (wpk h) k i
          < sk.dsBefore h (sk.stageLen h)
      ∧ futLen sk fut (walkIdx sk h) (upperOut (wpk h)) true
        = sk.stageLen h - k := by
  obtain ⟨j, hlast, hij⟩ := lastDOf_isSome_of_D sk hD hi
  have hjn := (lastDOf_isD sk hlast).2
  have htot : dRank sk (wpk h) k (sk.nChildren h (sk.stageScope h k))
      = sk.dOf h (sk.stageScope h k) := dRank_total sk (wpk h) k
  have hds := dRank_succ sk (wpk h) k i
  rw [show sk.childIsD (wpk h).2 (sk.stageScope (wpk h).2 k) i
      = sk.childIsD h (sk.stageScope h k) i from rfl, hD,
    if_pos rfl] at hds
  have hsc : sk.dsBefore h (k + 1)
      = sk.dsBefore h k + sk.dOf h (sk.stageScope h k) :=
    dsBefore_succ sk hk
  have hmono : sk.dsBefore h (k + 1) ≤ sk.dsBefore h (sk.stageLen h) :=
    dsBefore_mono sk h (by omega)
  have hle : dRank sk (wpk h) k i + 1 ≤ sk.dOf h (sk.stageScope h k) :=
    dRank_succ_le_dOf sk (wpk h) hi hD
  have hidx : i + 1 + (sk.nChildren h (sk.stageScope h k) - (i + 1))
      = sk.nChildren h (sk.stageScope h k) := by omega
  refine ⟨?_, by omega, ?_⟩
  · have hspl : proj (lowerOut (wpk h)) true
        (if lastDOf sk h k == some i
          then [((upperOut (wpk h), true, k) : Ev)] else []) = [] := by
      split
      · rw [proj_cons_ne_chan (by simp [upperOut, lowerOut]), proj_nil]
      · rfl
    have hqne : proj (lowerOut (wpk h)) true (chunkQ sk h k i) = [] :=
      chunkQ_proj_ne sk h k i (by
        rintro ⟨hc, -⟩
        simp only [askedOut, lowerOut] at hc
        split at hc <;> exact Chan.noConfusion hc)
    rw [futLen_of_filter sk hfil, proj_cons_self, proj_append,
      proj_append, proj_append, hspl, hqne,
      chunks_proj_res sk h k (lastDOf sk h k) _ (i + 1),
      walkSeg_proj_res sk (show k + 1 ≤ sk.stageLen h by omega)
        (Nat.le_refl _)]
    simp only [List.nil_append, List.length_cons, List.length_append,
      seg_len]
    rw [hidx]
    omega
  · have hqne : proj (upperOut (wpk h)) true (chunkQ sk h k i) = [] :=
      chunkQ_proj_ne sk h k i (by
        rintro ⟨hc, -⟩
        simp only [askedOut, upperOut] at hc
        split at hc <;> exact Chan.noConfusion hc)
    rw [futLen_of_filter sk hfil,
      proj_cons_ne_chan (by simp [lowerOut, upperOut]),
      proj_append, proj_append, proj_append, hqne,
      walkSeg_proj_upper sk (show k + 1 ≤ sk.stageLen h by omega)]
    by_cases hji : j = i
    · rw [hji] at hlast
      rw [chunks_proj_upper_past sk hlast (show i < i + 1 by omega),
        hlast, if_pos (beq_self_eq_true (some i)), proj_cons_self,
        proj_nil]
      simp only [List.append_nil, List.length_append, List.length_cons,
        List.length_nil, seg_len]
      omega
    · rw [chunks_proj_upper_covered sk hlast
          (show i + 1 ≤ j by omega)
          (show j < i + 1 + (sk.nChildren h (sk.stageScope h k)
            - (i + 1)) by omega),
        hlast, if_neg (by simp [hji]), proj_nil]
      simp only [List.nil_append, List.length_cons, List.length_nil,
        List.length_append, seg_len]
      omega

/-- The wire site: at a kid-slot boundary the walk's future wire
share spans the remaining slots and every later scope, with its
in-range bound. -/
theorem futLen_site_wire {fut : List Ev} {h k i : Nat}
    (hk : k < sk.stageLen h)
    (hi : i < sk.nChildren h (sk.stageScope h k))
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk h)
      = (List.range' i
            (sk.nChildren h (sk.stageScope h k) - i)).flatMap
            (splicedChunk sk h k (lastDOf sk h k))
          ++ walkSeg sk h (k + 1) (sk.stageLen h)) :
    futLen sk fut (walkIdx sk h) (wireOut (wpk h)) true
        = sk.wiresBefore h (sk.stageLen h) - (sk.wiresBefore h k + i)
      ∧ sk.wiresBefore h k + i < sk.wiresBefore h (sk.stageLen h) := by
  have hws : sk.wiresBefore h (k + 1)
      = sk.wiresBefore h k + sk.nChildren h (sk.stageScope h k) :=
    wiresBefore_succ sk hk
  have hwm : sk.wiresBefore h (k + 1)
      ≤ sk.wiresBefore h (sk.stageLen h) :=
    wiresBefore_mono sk h (by omega)
  refine ⟨?_, by omega⟩
  rw [futLen_of_filter sk hfil, proj_append,
    chunks_proj_wire sk h k (lastDOf sk h k) _ i,
    walkSeg_proj_wire sk (show k + 1 ≤ sk.stageLen h by omega)
      (Nat.le_refl _)]
  simp only [List.length_append, seg_len]
  omega

/-- The query site: mid-feed, the walk's future query share past the
emitted request, with its in-range bound. -/
theorem futLen_site_q {fut : List Ev} {h K i t : Nat}
    (hK : K < sk.stageLen h)
    (hi : i < sk.nChildren h (sk.stageScope h K))
    (ht : t < sk.qCount h (sk.stageScope h K) i)
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk h)
      = (chunkQ sk h K i).drop t
          ++ (List.range' (i + 1)
                (sk.nChildren h (sk.stageScope h K)
                  - (i + 1))).flatMap
               (splicedChunk sk h K (lastDOf sk h K))
          ++ walkSeg sk h (K + 1) (sk.stageLen h)) :
    futLen sk fut (walkIdx sk h) (askedOut (wpk h)) true
        = sk.qsBefore h (sk.stageLen h)
          - (sk.qsBefore h K + qSum sk (wpk h) K i + t)
      ∧ sk.qsBefore h K + qSum sk (wpk h) K i + t
          < sk.qsBefore h (sk.stageLen h) := by
  have hqs := qSum_succ sk (wpk h) K i
  rw [show sk.qCount (wpk h).2 (sk.stageScope (wpk h).2 K) i
      = sk.qCount h (sk.stageScope h K) i from rfl] at hqs
  have hidx : i + 1 + (sk.nChildren h (sk.stageScope h K) - (i + 1))
      = sk.nChildren h (sk.stageScope h K) := by omega
  have htot : qSum sk (wpk h) K (sk.nChildren h (sk.stageScope h K))
      = sk.qOf h (sk.stageScope h K) := qSum_total sk (wpk h) K
  have hqm : qSum sk (wpk h) K (i + 1)
      ≤ qSum sk (wpk h) K (i + 1
          + (sk.nChildren h (sk.stageScope h K) - (i + 1))) :=
    chain_le' (g := fun s => qSum sk (wpk h) K s)
      (sk.nChildren h (sk.stageScope h K) - (i + 1)) (i + 1)
      (fun s _ _ => by
        have hstep := qSum_succ sk (wpk h) K s
        omega)
  rw [hidx] at hqm
  have hsuc : sk.qsBefore h (K + 1)
      = sk.qsBefore h K + sk.qOf h (sk.stageScope h K) :=
    qsBefore_succ sk hK
  have hqsm : sk.qsBefore h (K + 1)
      ≤ sk.qsBefore h (K + 1 + (sk.stageLen h - (K + 1))) :=
    chain_le' (g := fun s => sk.qsBefore h s)
      (sk.stageLen h - (K + 1)) (K + 1)
      (fun s hs1 hs2 => by
        have hstep := qsBefore_succ sk
          (show s < sk.stageLen h by omega)
        omega)
  rw [show K + 1 + (sk.stageLen h - (K + 1)) = sk.stageLen h
      from by omega] at hqsm
  refine ⟨?_, by omega⟩
  rw [futLen_of_filter sk hfil, proj_append, proj_append,
    chunkQ_drop_proj_q sk h K i (by omega),
    chunks_proj_q sk h K (lastDOf sk h K) _ (i + 1),
    walkSeg_proj_q sk (show K + 1 ≤ sk.stageLen h by omega)
      (Nat.le_refl _)]
  simp only [List.length_append, seg_len]
  rw [hidx]
  omega

end StreamingMirror.Sched
