/-
The O futLen layer: the assignment-ordered segment and site count
forms the O master induction consumes.

The two structural facts that carried the E layer carry this one too,
one bridge deeper:

- Per channel-side, an O scope block projects identically to its d5
  block (`proj_scopeBlockO_eq` then `proj_scopeBlockE_eq` — the
  prologue's two receives live on distinct channels, so each channel's
  filter sees the same subsequence in either order), so every
  whole-block segment form (`futLen_walkSegO_*`) is the d5 form after
  one rewrite.
- Kid chunks carry no prologue and no parent — the E chunk bridges
  (`childChunk_spliced`, `childChunk_run_spliced`,
  `chunksNone_proj_upper`, SiteE.lean) serve the O runs verbatim; only
  the scope-suffix tail of each filter shape is respelled `walkSegO`.

The count pins re-derive through `count_pinPO` — `count_pinP`'s twin
over `FamOKO`, since the query-first absorber assignments cannot
inhabit `FamOK` — concluded against the d5 totals through the two
projection bridges, so no right-hand side changes. The margin-0 bricks
(`margin0_schedulable`, `margin0_dOf`) are SiteE.lean's, reused as-is.

Chain (ord, stage D exit): edge respect; consumed by Ord/Final.
Base mirror: SiteE.lean. Map: PROGRESS.md §13.
-/
import StreamingMirror.Ord.Window
import StreamingMirror.Ord.Align
import StreamingMirror.Proofs.Sched.Weave.SiteE

namespace StreamingMirror.Ord

open Model
open Sched

variable (sk : Skel) (ord : OrdMap)

-- ================================================= the segment bridge

/-- Per channel-side, an O segment run projects as its d5 segment run:
the O→E block bridge composed with the E→d5 one, per scope. -/
theorem walkSegO_proj (h' : Nat) (c : Chan) (b : Bool) :
    ∀ (n a : Nat),
      proj c b ((List.range' a n).flatMap (scopeBlockO sk ord (wpk h')))
        = proj c b ((List.range' a n).flatMap (scopeBlock sk (wpk h')))
  | 0, _ => rfl
  | n + 1, a => by
      rw [List.range'_succ, List.flatMap_cons, List.flatMap_cons,
        proj_append, proj_append, proj_scopeBlockO_eq,
        proj_scopeBlockE_eq, walkSegO_proj h' c b n (a + 1)]

/-- The segment bridge, stated on the named segments. -/
theorem walkSegO_proj_eq (h' a b' : Nat) (c : Chan) (b : Bool) :
    proj c b (walkSegO sk ord h' a b') = proj c b (walkSeg sk h' a b') :=
  walkSegO_proj sk ord h' c b (b' - a) a

-- ===================================== whole-block O segment futLens

/-- `futLen` of the summaries an O stage window still owes. -/
theorem futLen_walkSegO_upper {fut : List Ev} {h' a b : Nat}
    (hab : a ≤ b)
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk h')
      = walkSegO sk ord h' a b) :
    futLen sk fut (walkIdx sk h') (upperOut (wpk h')) true = b - a := by
  rw [futLen_of_filter sk hfil, walkSegO_proj_eq,
    walkSeg_proj_upper sk hab, seg_len]

/-- `futLen` of the resolutions an O stage window still owes. -/
theorem futLen_walkSegO_res {fut : List Ev} {h' a b : Nat}
    (hab : a ≤ b) (hb : b ≤ sk.stageLen h')
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk h')
      = walkSegO sk ord h' a b) :
    futLen sk fut (walkIdx sk h') (lowerOut (wpk h')) true
      = sk.dsBefore h' b - sk.dsBefore h' a := by
  rw [futLen_of_filter sk hfil, walkSegO_proj_eq,
    walkSeg_proj_res sk hab hb, seg_len]

/-- `futLen` of the wires an O stage window still owes. -/
theorem futLen_walkSegO_wire {fut : List Ev} {h' a b : Nat}
    (hab : a ≤ b) (hb : b ≤ sk.stageLen h')
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk h')
      = walkSegO sk ord h' a b) :
    futLen sk fut (walkIdx sk h') (wireOut (wpk h')) true
      = sk.wiresBefore h' b - sk.wiresBefore h' a := by
  rw [futLen_of_filter sk hfil, walkSegO_proj_eq,
    walkSeg_proj_wire sk hab hb, seg_len]

/-- `futLen` of the queries an O stage window still owes. -/
theorem futLen_walkSegO_q {fut : List Ev} {h' a b : Nat}
    (hab : a ≤ b) (hb : b ≤ sk.stageLen h')
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk h')
      = walkSegO sk ord h' a b) :
    futLen sk fut (walkIdx sk h') (askedOut (wpk h')) true
      = sk.qsBefore h' b - sk.qsBefore h' a := by
  rw [futLen_of_filter sk hfil, walkSegO_proj_eq,
    walkSeg_proj_q sk hab hb, seg_len]

/-- `futLen` of the wire receives an O stage window still owes. -/
theorem futLen_walkSegO_wireIn {fut : List Ev} {h' a b : Nat}
    (hab : a ≤ b)
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk h')
      = walkSegO sk ord h' a b) :
    futLen sk fut (walkIdx sk h') (wireIn (wpk h')) false = b - a := by
  rw [futLen_of_filter sk hfil, walkSegO_proj_eq,
    walkSeg_proj_wireIn sk hab, seg_len]

/-- `futLen` of the query receives an O stage window still owes. -/
theorem futLen_walkSegO_askedIn {fut : List Ev} {h' a b : Nat}
    (hab : a ≤ b)
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk h')
      = walkSegO sk ord h' a b) :
    futLen sk fut (walkIdx sk h') (askedIn (wpk h')) false = b - a := by
  rw [futLen_of_filter sk hfil, walkSegO_proj_eq,
    walkSeg_proj_askedIn sk hab, seg_len]

-- ================================================ ancestor O futLens

/-- An in-flight O ancestor's future summary share: the pending parent
plus every later scope's — no splice case split (cf.
`futLen_ancE_upper`; the chunk run is the E one at `lastD = none`). -/
theorem futLen_ancO_upper {fut : List Ev} {g A jD t : Nat}
    (hA : A < sk.stageLen g)
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk g)
      = (chunkQ sk g A jD).drop t
        ++ (List.range' (jD + 1)
              (sk.nChildren g (sk.stageScope g A) - (jD + 1))).flatMap
             (childChunk sk (wpk g) A)
        ++ ((upperOut (wpk g), true, A) : Ev)
          :: walkSegO sk ord g (A + 1) (sk.stageLen g)) :
    futLen sk fut (walkIdx sk g) (upperOut (wpk g)) true
      = sk.stageLen g - A := by
  have hne : proj (upperOut (wpk g)) true
      ((chunkQ sk g A jD).drop t) = [] :=
    chunkQ_drop_proj_ne sk g A jD t (by
      rintro ⟨hc, -⟩
      simp only [askedOut, upperOut] at hc
      split at hc <;> exact Chan.noConfusion hc)
  rw [futLen_of_filter sk hfil, proj_append, proj_append, hne,
    childChunk_run_spliced, chunksNone_proj_upper, proj_cons_self,
    walkSegO_proj_eq,
    walkSeg_proj_upper sk (show A + 1 ≤ sk.stageLen g by omega)]
  simp only [List.nil_append, List.length_cons, seg_len]
  omega

/-- An in-flight O ancestor's future resolution share: everything past
the in-flight slot's own resolution (cf. `futLen_ancE_lower`). -/
theorem futLen_ancO_lower {fut : List Ev} {g A jD t : Nat}
    (hA : A < sk.stageLen g)
    (hjD : jD < sk.nChildren g (sk.stageScope g A))
    (hD : sk.childIsD g (sk.stageScope g A) jD = true)
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk g)
      = (chunkQ sk g A jD).drop t
        ++ (List.range' (jD + 1)
              (sk.nChildren g (sk.stageScope g A) - (jD + 1))).flatMap
             (childChunk sk (wpk g) A)
        ++ ((upperOut (wpk g), true, A) : Ev)
          :: walkSegO sk ord g (A + 1) (sk.stageLen g)) :
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
    childChunk_run_spliced, chunks_proj_res sk g A none _ (jD + 1),
    proj_cons_ne_chan (by simp [upperOut, lowerOut]),
    walkSegO_proj_eq,
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

-- ================================================ the O tail-upper site

/-- The O parent site: at a scope's tail the future summary share is
the pending parent plus every later scope's (cf. `futLen_siteE_upper`). -/
theorem futLen_siteO_upper {fut : List Ev} {h k : Nat}
    (hk : k < sk.stageLen h)
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk h)
      = ((upperOut (wpk h), true, k) : Ev)
          :: walkSegO sk ord h (k + 1) (sk.stageLen h)) :
    futLen sk fut (walkIdx sk h) (upperOut (wpk h)) true
      = sk.stageLen h - k := by
  rw [futLen_of_filter sk hfil, proj_cons_self, walkSegO_proj_eq,
    walkSeg_proj_upper sk (show k + 1 ≤ sk.stageLen h by omega)]
  simp only [List.length_cons, seg_len]
  omega

/-- The O parent site's resolution share: the scope's are all spent. -/
theorem futLen_siteO_upper_res {fut : List Ev} {h k : Nat}
    (hk : k < sk.stageLen h)
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk h)
      = ((upperOut (wpk h), true, k) : Ev)
          :: walkSegO sk ord h (k + 1) (sk.stageLen h)) :
    futLen sk fut (walkIdx sk h) (lowerOut (wpk h)) true
      = sk.dsBefore h (sk.stageLen h) - sk.dsBefore h (k + 1) := by
  rw [futLen_of_filter sk hfil,
    proj_cons_ne_chan (by simp [upperOut, lowerOut]), walkSegO_proj_eq,
    walkSeg_proj_res sk (show k + 1 ≤ sk.stageLen h by omega)
      (Nat.le_refl _), seg_len]

/-- The O parent site's query share: the scope's are all spent. -/
theorem futLen_siteO_upper_q {fut : List Ev} {h k : Nat}
    (hk : k < sk.stageLen h)
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk h)
      = ((upperOut (wpk h), true, k) : Ev)
          :: walkSegO sk ord h (k + 1) (sk.stageLen h)) :
    futLen sk fut (walkIdx sk h) (askedOut (wpk h)) true
      = sk.qsBefore h (sk.stageLen h) - sk.qsBefore h (k + 1) := by
  rw [futLen_of_filter sk hfil,
    proj_cons_ne_chan (by
      simp only [askedOut, upperOut]
      split <;> simp), walkSegO_proj_eq,
    walkSeg_proj_q sk (show k + 1 ≤ sk.stageLen h by omega)
      (Nat.le_refl _), seg_len]

-- ================================================== the O count pins

/-- THE COUNT PIN over the O bundle: `count_pinP`'s statement with the
family fact swapped to `FamOKO` — the query-first absorber assignments
cannot inhabit `FamOK`, so the owner-projection collapse is
`out_proj_ownerO`'s; everything else is the base proof verbatim. -/
theorem count_pinPO {P : List (List Ev)} (hfam : FamOKO sk ord P)
    {fut : List Ev}
    {st : MState} (h : WCountP sk P fut st) (c : Chan) (b : Bool)
    {M : Nat} (hM : (if b then sndOwner sk c else rcvOwner sk c) = M)
    (hMlt : M < manCount sk)
    {T : List Ev} (hT : P[M]? = some T) :
    (proj c b st.out).length + futLen sk fut M c b
      = (proj c b T).length := by
  obtain ⟨r, pre, hr, hpre, hsub⟩ := cell_of_owner sk h hT
  have hout := out_proj_ownerO sk ord hfam h c b hM hT hr hpre hsub
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

/-- The walk-owned send channels' O pins, concluded against the d5
totals: per channel-side the class's orders all project identically,
so the right-hand sides never change (cf. `walk_snd_pinE`). -/
theorem walk_snd_pinO (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCountP sk (procsO sk ord) fut st) {hh : Nat}
    (hhr : hh < sk.rootH) (c : Chan)
    (hM : sndOwner sk c = walkIdx sk hh) :
    sndCount c st.out + futLen sk fut (walkIdx sk hh) c true
      = (proj c true (walkEvents sk (wpk hh))).length := by
  have hMlt : walkIdx sk hh < manCount sk := by
    unfold walkIdx manCount
    omega
  rw [sndCount_eq_proj]
  have hp := count_pinPO sk ord (famOKO_procsO sk ord hwf) h c true
    (by simpa using hM) hMlt (procsO_walk sk ord hhr)
  rw [proj_walkEventsO_eq, proj_walkEventsE_eq] at hp
  exact hp

/-- The O summary pin (cf. `upper_snd_pinE`). -/
theorem upper_snd_pinO (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCountP sk (procsO sk ord) fut st) {hh : Nat}
    (hhr : hh < sk.rootH) :
    sndCount (upperOut (wpk hh)) st.out
        + futLen sk fut (walkIdx sk hh) (upperOut (wpk hh)) true
      = sk.stageLen hh := by
  have hp := walk_snd_pinO sk ord hwf h hhr (upperOut (wpk hh)) rfl
  have hlen : (proj (upperOut (wpk hh)) true
      (walkEvents sk (wpk hh))).length = sk.stageLen hh := by
    rw [walk_upper_total]
    simp [canon, wpk]
  omega

/-- The O resolution pin (cf. `lower_snd_pinE`). -/
theorem lower_snd_pinO (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCountP sk (procsO sk ord) fut st) {hh : Nat}
    (hhr : hh < sk.rootH) :
    sndCount (lowerOut (wpk hh)) st.out
        + futLen sk fut (walkIdx sk hh) (lowerOut (wpk hh)) true
      = sk.dsBefore hh (sk.stageLen hh) := by
  have hp := walk_snd_pinO sk ord hwf h hhr (lowerOut (wpk hh)) rfl
  have hlen : (proj (lowerOut (wpk hh)) true
      (walkEvents sk (wpk hh))).length
      = sk.dsBefore hh (sk.stageLen hh) := by
    rw [walk_lower_total]
    simp [canon, wpk]
  omega

/-- The O wire pin (cf. `wire_snd_pinE`). -/
theorem wire_snd_pinO (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCountP sk (procsO sk ord) fut st) {hh : Nat}
    (hhr : hh < sk.rootH) :
    sndCount (wireOut (wpk hh)) st.out
        + futLen sk fut (walkIdx sk hh) (wireOut (wpk hh)) true
      = sk.wiresBefore hh (sk.stageLen hh) := by
  have hM : sndOwner sk (wireOut (wpk hh)) = walkIdx sk hh := by
    have hwire : wireOut (wpk hh) = Chan.wire (wpk hh).1 hh := rfl
    rw [hwire]
    simp only [sndOwner]
    rw [if_neg (by omega)]
  have hp := walk_snd_pinO sk ord hwf h hhr (wireOut (wpk hh)) hM
  have hlen : (proj (wireOut (wpk hh)) true
      (walkEvents sk (wpk hh))).length
      = sk.wiresBefore hh (sk.stageLen hh) := by
    rw [walk_wire_total]
    simp [canon, wpk]
  omega

/-- The O query pin (cf. `asked_snd_pinE`; `h1` for the same reason —
the leaf stage owns no queries). -/
theorem asked_snd_pinO (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCountP sk (procsO sk ord) fut st) {hh : Nat}
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
  have hp := walk_snd_pinO sk ord hwf h hhr (askedOut (wpk hh)) hM
  have hlen : (proj (askedOut (wpk hh)) true
      (walkEvents sk (wpk hh))).length
      = sk.qsBefore hh (sk.stageLen hh) := by
    rw [walk_asked_total]
    simp [canon, wpk]
  omega

/-- The O root-resolution bank (cf. `rootres_pinE`): the opener trace
is placement- and order-independent. -/
theorem rootres_pinO (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCountP sk (procsO sk ord) fut st)
    (hsilent : futLen sk fut 1 Chan.rootres true = 0) :
    1 ≤ sndCount Chan.rootres st.out := by
  have hMlt : (1 : Nat) < manCount sk := by
    unfold manCount
    omega
  have hp := count_pinPO sk ord (famOKO_procsO sk ord hwf) h
    Chan.rootres true (M := 1) rfl hMlt (procsO_ropen sk ord)
  rw [ropen_rootres_total] at hp
  rw [sndCount_eq_proj]
  simp only [List.length_cons, List.length_nil] at hp
  omega

/-- The O root bank at a feed suffix (cf. `root_bankedE`). -/
theorem root_bankedO (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (hW : WCountP sk (procsO sk ord) fut st)
    (hfeed : ∃ i₀, fut.filter (fun e => evOwner sk e == 1)
      = ((ropenEvents sk).drop 3).drop i₀) :
    1 ≤ sndCount Chan.rootres st.out := by
  obtain ⟨i₀, hf⟩ := hfeed
  exact rootres_pinO sk ord hwf hW (feed_rootres_silent sk hf)

-- ==================================== the O hsnd wrappers

/-- The O parent site's `hsnd` (cf. `upper_site_hsndE`). -/
theorem upper_site_hsndO (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCountP sk (procsO sk ord) fut st) {p : Party}
    {hh k : Nat}
    (hna : asks p hh = false) (hhr : hh < sk.rootH)
    (hk : k < sk.stageLen hh)
    (hfu : futLen sk fut (walkIdx sk hh) (upperOut (wpk hh)) true
      = sk.stageLen hh - k) :
    sndCount (Chan.upper p hh) st.out = k := by
  have hch : upperOut (wpk hh) = Chan.upper p hh := by
    rw [show upperOut (wpk hh) = Chan.upper (wpk hh).1 hh from rfl,
      wpk_fst_of_answerer hna]
  have hpin := upper_snd_pinO sk ord hwf h hhr
  rw [hch] at hpin hfu
  omega

/-- The O resolution site's `hsnd` (cf. `lower_site_hsndE`). -/
theorem lower_site_hsndO (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCountP sk (procsO sk ord) fut st) {p : Party}
    {hh k i : Nat}
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
  have hpin := lower_snd_pinO sk ord hwf h hhr
  rw [hch] at hpin hfu
  omega

/-- The O leaf-wire site's `hsnd` (cf. `wire0_site_hsndE`). -/
theorem wire0_site_hsndO (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCountP sk (procsO sk ord) fut st) {k i : Nat}
    (hr : 0 < sk.rootH)
    (hfu : futLen sk fut (walkIdx sk 0) (wireOut (wpk 0)) true
      = sk.wiresBefore 0 (sk.stageLen 0) - (sk.wiresBefore 0 k + i))
    (hbnd : sk.wiresBefore 0 k + i
      < sk.wiresBefore 0 (sk.stageLen 0)) :
    sndCount (Chan.wire Party.R 0) st.out = sk.wiresBefore 0 k + i := by
  have hch : wireOut (wpk 0) = Chan.wire Party.R 0 := rfl
  have hpin := wire_snd_pinO sk ord hwf h hr
  rw [hch] at hpin hfu
  omega

/-- The O leaf-request site's `hsnd` (cf. `leafreq_site_hsndE`). -/
theorem leafreq_site_hsndO (hwf : sk.wellFormed = true)
    {fut : List Ev}
    {st : MState} (h : WCountP sk (procsO sk ord) fut st) {K i t : Nat}
    (hr : 1 < sk.rootH)
    (hfu : futLen sk fut (walkIdx sk 1) (askedOut (wpk 1)) true
      = sk.qsBefore 1 (sk.stageLen 1)
        - (sk.qsBefore 1 K + qSum sk (wpk 1) K i + t))
    (hbnd : sk.qsBefore 1 K + qSum sk (wpk 1) K i + t
      < sk.qsBefore 1 (sk.stageLen 1)) :
    sndCount Chan.leafRequests st.out
      = sk.qsBefore 1 K + qSum sk (wpk 1) K i + t := by
  have hch : askedOut (wpk 1) = Chan.leafRequests := rfl
  have hpin := asked_snd_pinO sk ord hwf h (Nat.le_refl 1) hr
  rw [hch] at hpin hfu
  omega

-- ==================================== the O ancestor pins

/-- The O in-flight ancestor's count pins (cf. `anc_position_countsE`):
the parent is always pending, so the summary count is `A` outright. -/
theorem anc_position_countsO (hwf : sk.wellFormed = true)
    {fut : List Ev} {st : MState}
    (h : WCountP sk (procsO sk ord) fut st)
    {g A jD : Nat} (hgr : g < sk.rootH) (hA : A < sk.stageLen g)
    (hjD : jD < sk.nChildren g (sk.stageScope g A))
    (hD : sk.childIsD g (sk.stageScope g A) jD = true)
    (hfu : futLen sk fut (walkIdx sk g) (upperOut (wpk g)) true
      = sk.stageLen g - A)
    (hfl : futLen sk fut (walkIdx sk g) (lowerOut (wpk g)) true
      = sk.dsBefore g (sk.stageLen g)
        - (sk.dsBefore g A + dRank sk (wpk g) A jD + 1)) :
    sndCount (upperOut (wpk g)) st.out = A
      ∧ sndCount (lowerOut (wpk g)) st.out
        = sk.dsBefore g A + dRank sk (wpk g) A jD + 1 := by
  have hupp := upper_snd_pinO sk ord hwf h hgr
  have hlop := lower_snd_pinO sk ord hwf h hgr
  have hdr : dRank sk (wpk g) A jD + 1
      ≤ sk.dOf g (sk.stageScope g A) :=
    dRank_succ_le_dOf sk (wpk g) hjD hD
  have hds := dsBefore_succ sk hA
  have hmono : sk.dsBefore g (A + 1)
      ≤ sk.dsBefore g (sk.stageLen g) :=
    dsBefore_mono sk g hA
  exact ⟨by omega, by omega⟩

/-- `P1` at an O-covered ancestor (cf. `p1_of_ancE`): margin 0 alone
closes the allocation — the pending parent means the summary count is
`A`, and the slot's resolutions fit inside the level capacity with no
schedulable slack needed. -/
theorem p1_of_ancO (hwf : sk.wellFormed = true)
    (hm0 : ∀ s, sk.dCount s ≤ sk.capLevel) {fut : List Ev}
    {st : MState}
    (h : WCountP sk (procsO sk ord) fut st) {p : Party} {g A jD : Nat}
    (hna : asks p g = false) (hgr : g < sk.rootH)
    (hA : A < sk.stageLen g)
    (hjD : jD < sk.nChildren g (sk.stageScope g A))
    (hD : sk.childIsD g (sk.stageScope g A) jD = true)
    (hfu : futLen sk fut (walkIdx sk g) (upperOut (wpk g)) true
      = sk.stageLen g - A)
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
    anc_position_countsO sk ord hwf h hgr hA hjD hD hfu hfl
  rw [hchu] at hcu
  rw [hchl] at hcl
  have hdr : dRank sk (wpk g) A jD + 1
      ≤ sk.dOf g (sk.stageScope g A) :=
    dRank_succ_le_dOf sk (wpk g) hjD hD
  have hcap := margin0_dOf sk hm0 g (sk.stageScope g A)
  rw [hcu, hcl]
  omega

end StreamingMirror.Ord
