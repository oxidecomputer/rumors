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

end StreamingMirror.Sched
