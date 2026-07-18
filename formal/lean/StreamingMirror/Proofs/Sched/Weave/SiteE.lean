/-
The E futLen layer (unit 2b, PROGRESS.md §9): the encoder-order
segment and site count forms the eweave master induction consumes.

Two structural facts make this layer a thin bridge over the d5 one:

- Per channel-side, an epilogue-order scope block projects identically
  to its d5 block (`proj_scopeBlockE_eq`), so every whole-block
  segment form (`futLen_walkSegE_*`) is the d5 form after one rewrite.
- An encoder-order kid chunk IS the d5 spliced chunk with the splice
  disabled (`childChunk_spliced`: `lastD := none`), so the mid-scope
  run forms reuse `chunks_proj_*` at the literal `none` — no σ
  discriminant, no covered/past trichotomy; the parent rides the scope
  tail as an explicit cons instead.

The ancestor-tail forms (`futLen_ancE_*`) mirror `futLen_anc_*` over
the E in-flight shape — chunk-query residue, later kid chunks, the
pending parent, the scope suffix — and are SIMPLER than d5's: the
parent is always pending, so the upper share carries no `if`.

Chain (.impl, stage B): mirrors Emit.lean + Site.lean through the
`childChunk_spliced`/projection bridges; provides the E futLen and site
forms to TeleE.lean and MasterE.lean. Map: Proofs/Map.lean.
-/
import StreamingMirror.Proofs.Sched.Weave.Master
import StreamingMirror.Proofs.Sched.Weave.AlignE

namespace StreamingMirror.Sched

open Model

variable (sk : Skel)

-- ================================================= chunk and segment

/-- An encoder-order kid chunk is the spliced chunk with no splice. -/
theorem childChunk_spliced (h k i : Nat) :
    childChunk sk (wpk h) k i = splicedChunk sk h k none i := by
  by_cases hD : sk.childIsD h (sk.stageScope h k) i <;>
    simp [childChunk, splicedChunk, chunkQ, dRank, qSum, wpk, hD]

/-- An encoder-order kid run is a no-splice spliced run. -/
theorem childChunk_run_spliced (h k : Nat) (m i : Nat) :
    (List.range' i m).flatMap (childChunk sk (wpk h) k)
      = (List.range' i m).flatMap (splicedChunk sk h k none) :=
  flatMap_congr fun i' _ => childChunk_spliced sk h k i'

/-- Per channel-side, an E segment projects as its d5 segment. -/
theorem walkSegE_proj (h' : Nat) (c : Chan) (b : Bool) :
    ∀ (n a : Nat),
      proj c b ((List.range' a n).flatMap (scopeBlockE sk (wpk h')))
        = proj c b ((List.range' a n).flatMap (scopeBlock sk (wpk h')))
  | 0, _ => rfl
  | n + 1, a => by
      rw [List.range'_succ, List.flatMap_cons, List.flatMap_cons,
        proj_append, proj_append, proj_scopeBlockE_eq,
        walkSegE_proj h' c b n (a + 1)]

/-- The segment bridge, stated on the named segments. -/
theorem walkSegE_proj_eq (h' a b' : Nat) (c : Chan) (b : Bool) :
    proj c b (walkSegE sk h' a b') = proj c b (walkSeg sk h' a b') :=
  walkSegE_proj sk h' c b (b' - a) a

-- ===================================== whole-block E segment futLens

/-- `futLen` of the summaries an E stage window still owes. -/
theorem futLen_walkSegE_upper {fut : List Ev} {h' a b : Nat}
    (hab : a ≤ b)
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk h')
      = walkSegE sk h' a b) :
    futLen sk fut (walkIdx sk h') (upperOut (wpk h')) true = b - a := by
  rw [futLen_of_filter sk hfil, walkSegE_proj_eq,
    walkSeg_proj_upper sk hab, seg_len]

/-- `futLen` of the resolutions an E stage window still owes. -/
theorem futLen_walkSegE_res {fut : List Ev} {h' a b : Nat}
    (hab : a ≤ b) (hb : b ≤ sk.stageLen h')
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk h')
      = walkSegE sk h' a b) :
    futLen sk fut (walkIdx sk h') (lowerOut (wpk h')) true
      = sk.dsBefore h' b - sk.dsBefore h' a := by
  rw [futLen_of_filter sk hfil, walkSegE_proj_eq,
    walkSeg_proj_res sk hab hb, seg_len]

/-- `futLen` of the wires an E stage window still owes. -/
theorem futLen_walkSegE_wire {fut : List Ev} {h' a b : Nat}
    (hab : a ≤ b) (hb : b ≤ sk.stageLen h')
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk h')
      = walkSegE sk h' a b) :
    futLen sk fut (walkIdx sk h') (wireOut (wpk h')) true
      = sk.wiresBefore h' b - sk.wiresBefore h' a := by
  rw [futLen_of_filter sk hfil, walkSegE_proj_eq,
    walkSeg_proj_wire sk hab hb, seg_len]

/-- `futLen` of the queries an E stage window still owes. -/
theorem futLen_walkSegE_q {fut : List Ev} {h' a b : Nat}
    (hab : a ≤ b) (hb : b ≤ sk.stageLen h')
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk h')
      = walkSegE sk h' a b) :
    futLen sk fut (walkIdx sk h') (askedOut (wpk h')) true
      = sk.qsBefore h' b - sk.qsBefore h' a := by
  rw [futLen_of_filter sk hfil, walkSegE_proj_eq,
    walkSeg_proj_q sk hab hb, seg_len]

/-- `futLen` of the wire receives an E stage window still owes. -/
theorem futLen_walkSegE_wireIn {fut : List Ev} {h' a b : Nat}
    (hab : a ≤ b)
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk h')
      = walkSegE sk h' a b) :
    futLen sk fut (walkIdx sk h') (wireIn (wpk h')) false = b - a := by
  rw [futLen_of_filter sk hfil, walkSegE_proj_eq,
    walkSeg_proj_wireIn sk hab, seg_len]

/-- `futLen` of the query receives an E stage window still owes. -/
theorem futLen_walkSegE_askedIn {fut : List Ev} {h' a b : Nat}
    (hab : a ≤ b)
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk h')
      = walkSegE sk h' a b) :
    futLen sk fut (walkIdx sk h') (askedIn (wpk h')) false = b - a := by
  rw [futLen_of_filter sk hfil, walkSegE_proj_eq,
    walkSeg_proj_askedIn sk hab, seg_len]

-- ================================================ deep E stage counts

/-- A deep E stage parked at its window start: resolutions emitted
through the cursor. -/
theorem deep_lower_countE {g c : Nat} {fut : List Ev}
    {st : MState} (hc : c ≤ sk.stageLen g)
    (hpin : (proj (lowerOut (wpk g)) true st.out).length
        + futLen sk fut (walkIdx sk g) (lowerOut (wpk g)) true
      = sk.dsBefore g (sk.stageLen g))
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk g)
      = walkSegE sk g c (sk.stageLen g)) :
    sndCount (lowerOut (wpk g)) st.out = sk.dsBefore g c := by
  have hfl := futLen_walkSegE_res sk hc (Nat.le_refl _) hfil
  have hmono := dsBefore_mono sk g hc
  rw [sndCount_eq_proj]
  omega

/-- A deep E stage parked at its window start: summaries emitted
through the cursor. -/
theorem deep_upper_countE {g c : Nat} {fut : List Ev}
    {st : MState} (hc : c ≤ sk.stageLen g)
    (hpin : (proj (upperOut (wpk g)) true st.out).length
        + futLen sk fut (walkIdx sk g) (upperOut (wpk g)) true
      = sk.stageLen g)
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk g)
      = walkSegE sk g c (sk.stageLen g)) :
    sndCount (upperOut (wpk g)) st.out = c := by
  have hfl := futLen_walkSegE_upper sk hc hfil
  rw [sndCount_eq_proj]
  omega

-- ============================================= the no-splice run

/-- A single no-splice chunk carries no summary. -/
theorem schunkNone_proj_upper (h k i : Nat) :
    proj (upperOut (wpk h)) true (splicedChunk sk h k none i) = [] := by
  unfold splicedChunk
  rw [proj_cons_ne_chan (by simp [wireOut, upperOut])]
  by_cases hD : sk.childIsD h (sk.stageScope h k) i
  · rw [if_pos hD, proj_cons_ne_chan (by simp [lowerOut, upperOut]),
      show ((none : Option Nat) == some i) = false from rfl, if_neg
        (by simp), List.nil_append]
    exact proj_eq_nil fun e he h1 _ => by
      unfold chunkQ at he
      obtain ⟨t, -, rfl⟩ := List.mem_map.1 he
      simp only [askedOut, upperOut] at h1
      split at h1 <;> exact Chan.noConfusion h1
  · rw [if_neg hD]
    exact proj_nil _ _

/-- A no-splice kid run carries no summary. -/
theorem chunksNone_proj_upper (h k : Nat) :
    ∀ (m i : Nat),
      proj (upperOut (wpk h)) true
        ((List.range' i m).flatMap (splicedChunk sk h k none)) = []
  | 0, _ => rfl
  | m + 1, i => by
      rw [List.range'_succ, List.flatMap_cons, proj_append,
        schunkNone_proj_upper sk h k i,
        chunksNone_proj_upper h k m (i + 1)]
      rfl

-- ================================================ ancestor E futLens

/-- An in-flight E ancestor's future summary share: the pending parent
plus every later scope's — no splice case split. -/
theorem futLen_ancE_upper {fut : List Ev} {g A jD t : Nat}
    (hA : A < sk.stageLen g)
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk g)
      = (chunkQ sk g A jD).drop t
        ++ (List.range' (jD + 1)
              (sk.nChildren g (sk.stageScope g A) - (jD + 1))).flatMap
             (childChunk sk (wpk g) A)
        ++ ((upperOut (wpk g), true, A) : Ev)
          :: walkSegE sk g (A + 1) (sk.stageLen g)) :
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
    walkSegE_proj_eq,
    walkSeg_proj_upper sk (show A + 1 ≤ sk.stageLen g by omega)]
  simp only [List.nil_append, List.length_cons, seg_len]
  omega

/-- An in-flight E ancestor's future resolution share: everything past
the in-flight slot's own resolution. -/
theorem futLen_ancE_lower {fut : List Ev} {g A jD t : Nat}
    (hA : A < sk.stageLen g)
    (hjD : jD < sk.nChildren g (sk.stageScope g A))
    (hD : sk.childIsD g (sk.stageScope g A) jD = true)
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk g)
      = (chunkQ sk g A jD).drop t
        ++ (List.range' (jD + 1)
              (sk.nChildren g (sk.stageScope g A) - (jD + 1))).flatMap
             (childChunk sk (wpk g) A)
        ++ ((upperOut (wpk g), true, A) : Ev)
          :: walkSegE sk g (A + 1) (sk.stageLen g)) :
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
    walkSegE_proj_eq,
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

-- ================================================ the E tail-upper site

/-- The E parent site: at a scope's tail the future summary share is
the pending parent plus every later scope's. -/
theorem futLen_siteE_upper {fut : List Ev} {h k : Nat}
    (hk : k < sk.stageLen h)
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk h)
      = ((upperOut (wpk h), true, k) : Ev)
          :: walkSegE sk h (k + 1) (sk.stageLen h)) :
    futLen sk fut (walkIdx sk h) (upperOut (wpk h)) true
      = sk.stageLen h - k := by
  rw [futLen_of_filter sk hfil, proj_cons_self, walkSegE_proj_eq,
    walkSeg_proj_upper sk (show k + 1 ≤ sk.stageLen h by omega)]
  simp only [List.length_cons, seg_len]
  omega

/-- The E parent site's resolution share: the scope's are all spent. -/
theorem futLen_siteE_upper_res {fut : List Ev} {h k : Nat}
    (hk : k < sk.stageLen h)
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk h)
      = ((upperOut (wpk h), true, k) : Ev)
          :: walkSegE sk h (k + 1) (sk.stageLen h)) :
    futLen sk fut (walkIdx sk h) (lowerOut (wpk h)) true
      = sk.dsBefore h (sk.stageLen h) - sk.dsBefore h (k + 1) := by
  rw [futLen_of_filter sk hfil,
    proj_cons_ne_chan (by simp [upperOut, lowerOut]), walkSegE_proj_eq,
    walkSeg_proj_res sk (show k + 1 ≤ sk.stageLen h by omega)
      (Nat.le_refl _), seg_len]

/-- The E parent site's query share: the scope's are all spent. -/
theorem futLen_siteE_upper_q {fut : List Ev} {h k : Nat}
    (hk : k < sk.stageLen h)
    (hfil : fut.filter (fun e => evOwner sk e == walkIdx sk h)
      = ((upperOut (wpk h), true, k) : Ev)
          :: walkSegE sk h (k + 1) (sk.stageLen h)) :
    futLen sk fut (walkIdx sk h) (askedOut (wpk h)) true
      = sk.qsBefore h (sk.stageLen h) - sk.qsBefore h (k + 1) := by
  rw [futLen_of_filter sk hfil,
    proj_cons_ne_chan (by
      simp only [askedOut, upperOut]
      split <;> simp), walkSegE_proj_eq,
    walkSeg_proj_q sk (show k + 1 ≤ sk.stageLen h by omega)
      (Nat.le_refl _), seg_len]

-- ======================================== the E count pins (§9 (i))

/-- The stage-`h` walk sits at slot `walkIdx h` of the encoder-order
family too: the family swaps each walk's trace in place. -/
theorem procsE_walk {h : Nat} (hh : h < sk.rootH) :
    (procsE sk)[walkIdx sk h]? = some (walkEventsE sk (wpk h)) := by
  unfold procsE
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

/-- The responder opener sits at slot 1 of the encoder-order family. -/
theorem procsE_ropen : (procsE sk)[1]? = some (ropenEvents sk) := rfl

/-- The walk-owned send channels' E pins, concluded against the d5
totals: per channel-side the two orders project identically, so the
right-hand sides never change. -/
theorem walk_snd_pinE (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCountP sk (procsE sk) fut st) {hh : Nat}
    (hhr : hh < sk.rootH) (c : Chan)
    (hM : sndOwner sk c = walkIdx sk hh) :
    sndCount c st.out + futLen sk fut (walkIdx sk hh) c true
      = (proj c true (walkEvents sk (wpk hh))).length := by
  have hMlt : walkIdx sk hh < manCount sk := by
    unfold walkIdx manCount
    omega
  rw [sndCount_eq_proj]
  have hp := count_pinP sk (famOK_procsE sk hwf) h c true
    (by simpa using hM) hMlt (procsE_walk sk hhr)
  rw [proj_walkEventsE_eq] at hp
  exact hp

/-- The E summary pin (cf. `upper_snd_pin`). -/
theorem upper_snd_pinE (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCountP sk (procsE sk) fut st) {hh : Nat}
    (hhr : hh < sk.rootH) :
    sndCount (upperOut (wpk hh)) st.out
        + futLen sk fut (walkIdx sk hh) (upperOut (wpk hh)) true
      = sk.stageLen hh := by
  have hp := walk_snd_pinE sk hwf h hhr (upperOut (wpk hh)) rfl
  have hlen : (proj (upperOut (wpk hh)) true
      (walkEvents sk (wpk hh))).length = sk.stageLen hh := by
    rw [walk_upper_total]
    simp [canon, wpk]
  omega

/-- The E resolution pin (cf. `lower_snd_pin`). -/
theorem lower_snd_pinE (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCountP sk (procsE sk) fut st) {hh : Nat}
    (hhr : hh < sk.rootH) :
    sndCount (lowerOut (wpk hh)) st.out
        + futLen sk fut (walkIdx sk hh) (lowerOut (wpk hh)) true
      = sk.dsBefore hh (sk.stageLen hh) := by
  have hp := walk_snd_pinE sk hwf h hhr (lowerOut (wpk hh)) rfl
  have hlen : (proj (lowerOut (wpk hh)) true
      (walkEvents sk (wpk hh))).length
      = sk.dsBefore hh (sk.stageLen hh) := by
    rw [walk_lower_total]
    simp [canon, wpk]
  omega

/-- The E wire pin (cf. `wire_snd_pin`). -/
theorem wire_snd_pinE (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCountP sk (procsE sk) fut st) {hh : Nat}
    (hhr : hh < sk.rootH) :
    sndCount (wireOut (wpk hh)) st.out
        + futLen sk fut (walkIdx sk hh) (wireOut (wpk hh)) true
      = sk.wiresBefore hh (sk.stageLen hh) := by
  have hM : sndOwner sk (wireOut (wpk hh)) = walkIdx sk hh := by
    have hwire : wireOut (wpk hh) = Chan.wire (wpk hh).1 hh := rfl
    rw [hwire]
    simp only [sndOwner]
    rw [if_neg (by omega)]
  have hp := walk_snd_pinE sk hwf h hhr (wireOut (wpk hh)) hM
  have hlen : (proj (wireOut (wpk hh)) true
      (walkEvents sk (wpk hh))).length
      = sk.wiresBefore hh (sk.stageLen hh) := by
    rw [walk_wire_total]
    simp [canon, wpk]
  omega

/-- The E query pin (cf. `asked_snd_pin`; `h1` for the same reason —
the leaf stage owns no queries). -/
theorem asked_snd_pinE (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCountP sk (procsE sk) fut st) {hh : Nat}
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
  have hp := walk_snd_pinE sk hwf h hhr (askedOut (wpk hh)) hM
  have hlen : (proj (askedOut (wpk hh)) true
      (walkEvents sk (wpk hh))).length
      = sk.qsBefore hh (sk.stageLen hh) := by
    rw [walk_asked_total]
    simp [canon, wpk]
  omega

/-- The E root-resolution bank (cf. `rootres_pin`): the opener trace
is placement-independent. -/
theorem rootres_pinE (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCountP sk (procsE sk) fut st)
    (hsilent : futLen sk fut 1 Chan.rootres true = 0) :
    1 ≤ sndCount Chan.rootres st.out := by
  have hMlt : (1 : Nat) < manCount sk := by
    unfold manCount
    omega
  have hp := count_pinP sk (famOK_procsE sk hwf) h Chan.rootres true
    (M := 1) rfl hMlt (procsE_ropen sk)
  rw [ropen_rootres_total] at hp
  rw [sndCount_eq_proj]
  simp only [List.length_cons, List.length_nil] at hp
  omega

/-- The E root bank at a feed suffix (cf. `root_banked`). -/
theorem root_bankedE (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (hW : WCountP sk (procsE sk) fut st)
    (hfeed : ∃ i₀, fut.filter (fun e => evOwner sk e == 1)
      = ((ropenEvents sk).drop 3).drop i₀) :
    1 ≤ sndCount Chan.rootres st.out := by
  obtain ⟨i₀, hf⟩ := hfeed
  exact rootres_pinE sk hwf hW (feed_rootres_silent sk hf)

-- ================================== margin 0, the capacity hypothesis

/-- Margin 0 implies `schedulable`: the flagship's capacity hypothesis
subsumes the boundary bound with two to spare. -/
theorem margin0_schedulable
    (hm0 : ∀ s, sk.dCount s ≤ sk.capLevel) :
    sk.schedulable = true := by
  unfold Skel.schedulable
  rw [List.all_eq_true]
  intro s _
  simp only [decide_eq_true_eq]
  have := hm0 s
  omega

/-- Margin 0 in per-scope `dOf` form (cf. `schedulable_dOf`): no stage
disputes more children than the level channel holds outright. -/
theorem margin0_dOf (hm0 : ∀ s, sk.dCount s ≤ sk.capLevel)
    (g s : Nat) : sk.dOf g s ≤ sk.capLevel := by
  unfold Skel.dOf
  split
  · omega
  · exact hm0 s

-- ==================================== the E hsnd wrappers (§9 (ii))

/-- The E parent site's `hsnd` (cf. `upper_site_hsnd`). -/
theorem upper_site_hsndE (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCountP sk (procsE sk) fut st) {p : Party}
    {hh k : Nat}
    (hna : asks p hh = false) (hhr : hh < sk.rootH)
    (hk : k < sk.stageLen hh)
    (hfu : futLen sk fut (walkIdx sk hh) (upperOut (wpk hh)) true
      = sk.stageLen hh - k) :
    sndCount (Chan.upper p hh) st.out = k := by
  have hch : upperOut (wpk hh) = Chan.upper p hh := by
    rw [show upperOut (wpk hh) = Chan.upper (wpk hh).1 hh from rfl,
      wpk_fst_of_answerer hna]
  have hpin := upper_snd_pinE sk hwf h hhr
  rw [hch] at hpin hfu
  omega

/-- The E resolution site's `hsnd` (cf. `lower_site_hsnd`). -/
theorem lower_site_hsndE (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCountP sk (procsE sk) fut st) {p : Party}
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
  have hpin := lower_snd_pinE sk hwf h hhr
  rw [hch] at hpin hfu
  omega

/-- The E leaf-wire site's `hsnd` (cf. `wire0_site_hsnd`). -/
theorem wire0_site_hsndE (hwf : sk.wellFormed = true) {fut : List Ev}
    {st : MState} (h : WCountP sk (procsE sk) fut st) {k i : Nat}
    (hr : 0 < sk.rootH)
    (hfu : futLen sk fut (walkIdx sk 0) (wireOut (wpk 0)) true
      = sk.wiresBefore 0 (sk.stageLen 0) - (sk.wiresBefore 0 k + i))
    (hbnd : sk.wiresBefore 0 k + i
      < sk.wiresBefore 0 (sk.stageLen 0)) :
    sndCount (Chan.wire Party.R 0) st.out = sk.wiresBefore 0 k + i := by
  have hch : wireOut (wpk 0) = Chan.wire Party.R 0 := rfl
  have hpin := wire_snd_pinE sk hwf h hr
  rw [hch] at hpin hfu
  omega

/-- The E leaf-request site's `hsnd` (cf. `leafreq_site_hsnd`). -/
theorem leafreq_site_hsndE (hwf : sk.wellFormed = true)
    {fut : List Ev}
    {st : MState} (h : WCountP sk (procsE sk) fut st) {K i t : Nat}
    (hr : 1 < sk.rootH)
    (hfu : futLen sk fut (walkIdx sk 1) (askedOut (wpk 1)) true
      = sk.qsBefore 1 (sk.stageLen 1)
        - (sk.qsBefore 1 K + qSum sk (wpk 1) K i + t))
    (hbnd : sk.qsBefore 1 K + qSum sk (wpk 1) K i + t
      < sk.qsBefore 1 (sk.stageLen 1)) :
    sndCount Chan.leafRequests st.out
      = sk.qsBefore 1 K + qSum sk (wpk 1) K i + t := by
  have hch : askedOut (wpk 1) = Chan.leafRequests := rfl
  have hpin := asked_snd_pinE sk hwf h (Nat.le_refl 1) hr
  rw [hch] at hpin hfu
  omega

-- ==================================== the E ancestor pins (§9 (iii))

/-- The E in-flight ancestor's count pins (cf. `anc_position_counts`):
the parent is always pending, so the summary count is `A` outright —
no splice discriminant. -/
theorem anc_position_countsE (hwf : sk.wellFormed = true)
    {fut : List Ev} {st : MState}
    (h : WCountP sk (procsE sk) fut st)
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
  have hupp := upper_snd_pinE sk hwf h hgr
  have hlop := lower_snd_pinE sk hwf h hgr
  have hdr : dRank sk (wpk g) A jD + 1
      ≤ sk.dOf g (sk.stageScope g A) :=
    dRank_succ_le_dOf sk (wpk g) hjD hD
  have hds := dsBefore_succ sk hA
  have hmono : sk.dsBefore g (A + 1)
      ≤ sk.dsBefore g (sk.stageLen g) :=
    dsBefore_mono sk g hA
  exact ⟨by omega, by omega⟩

/-- `P1` at an E-covered ancestor (cf. `p1_of_anc`): margin 0 alone
closes the allocation — the pending parent means the summary count is
`A`, and the slot's resolutions fit inside the level capacity with no
schedulable slack needed. -/
theorem p1_of_ancE (hwf : sk.wellFormed = true)
    (hm0 : ∀ s, sk.dCount s ≤ sk.capLevel) {fut : List Ev}
    {st : MState}
    (h : WCountP sk (procsE sk) fut st) {p : Party} {g A jD : Nat}
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
    anc_position_countsE sk hwf h hgr hA hjD hD hfu hfl
  rw [hchu] at hcu
  rw [hchl] at hcl
  have hdr : dRank sk (wpk g) A jD + 1
      ≤ sk.dOf g (sk.stageScope g A) :=
    dRank_succ_le_dOf sk (wpk g) hjD hD
  have hcap := margin0_dOf sk hm0 g (sk.stageScope g A)
  rw [hcu, hcl]
  omega

end StreamingMirror.Sched
