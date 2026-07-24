/-
The O↔E bridges: per channel-side, every O trace projects IDENTICALLY
to its `procsE` counterpart — a prologue's two receives live on
distinct channels, so one channel's filter sees the same subsequence
in either order — and the O traces are pointwise permutations of the E
traces. Riding those two bridges, the numbering/ownership layer and
the generic-merge corollaries instantiate at `procsO` for every
assignment.

# The one structural asymmetry

`FamOK.pumps` demands the family's pump suffix be LITERALLY
`weavePumps sk` — a list equality, not a proj fact — and a query-first
absorber genuinely reorders the absorb trace, so that field is false
at `ord.absorb = .queryFirst` on any skeleton with a leaf request.
`famOK_procsO` therefore carries an `ord.absorb = .replyFirst`
hypothesis; walk assignments stay fully free (walks are manual — the
pump suffix never contains them), and the other three fields
(`procsO_canon`, `procsO_snd_owned`, `procsO_rcv_owned`) are proved
unconditionally. Discharging the query-first absorber needs a
pump-suffix generalization of `FamOK` (proj-shaped, or an
ord-parameterized pump list) — an existing-file change, the lead's
call. `ManRows` is proj-shaped throughout, so `manRows_procsO` is
unconditional.

Chain (ord, stage C): the projection/permutation bridges, the family
instances, and the merge-layer wrappers at `procsO`; consumed by the
O completeness stage. Base mirror: Proofs/Sched/Numbering.lean's E
bridge + Weave/Count.lean's `procsE` instances + Weave/FinalE.lean's
`manRows_procsE`. Map: PROGRESS.md §13.
-/
import StreamingMirror.Proofs.Sched.Weave.Final
import StreamingMirror.Ord.Sched

namespace StreamingMirror.Ord

open Model
open Sched

variable (sk : Skel) (ord : OrdMap)

-- ============================================ channel discrimination

/-- The two walk-prologue channels disagree at the constructor. -/
private theorem askedIn_ne_wireIn (pk : Party × Nat) :
    askedIn pk ≠ wireIn pk := by simp [askedIn, wireIn]

/-- The two absorber-prologue channels disagree at the constructor. -/
private theorem leaf_ne_wireR0 :
    (Chan.leafRequests : Chan) ≠ Chan.wire Party.R 0 := by simp

-- ============================================== the projection bridge

/-- Swapping two adjacent events on distinct channels is invisible to
every single-channel projection: at most one of the two passes the
filter. -/
private theorem proj_pair_swap {cx cy : Chan} (hne : cy ≠ cx)
    (bx by' : Bool) (nx ny : Nat) (c : Chan) (b : Bool) :
    proj c b [(cy, by', ny), (cx, bx, nx)]
      = proj c b [(cx, bx, nx), (cy, by', ny)] := by
  have h1 : proj c b [((cy, by', ny) : Ev), (cx, bx, nx)]
      = proj c b [((cy, by', ny) : Ev)] ++ proj c b [((cx, bx, nx) : Ev)] :=
    proj_append c b [(cy, by', ny)] [(cx, bx, nx)]
  have h2 : proj c b [((cx, bx, nx) : Ev), (cy, by', ny)]
      = proj c b [((cx, bx, nx) : Ev)] ++ proj c b [((cy, by', ny) : Ev)] :=
    proj_append c b [(cx, bx, nx)] [(cy, by', ny)]
  rw [h1, h2]
  by_cases hcx : c = cx
  · have hy : proj c b [((cy, by', ny) : Ev)] = [] := by
      subst hcx
      rw [proj_cons_ne_chan hne, proj_nil]
    rw [hy, List.nil_append, List.append_nil]
  · have hx : proj c b [((cx, bx, nx) : Ev)] = [] := by
      rw [proj_cons_ne_chan (fun h => hcx h.symm), proj_nil]
    rw [hx, List.nil_append, List.append_nil]

/-- The ordered prologue projects as the E prologue: its two receives
sit on distinct channels, so each channel-side sees the same
subsequence in either order. -/
theorem proj_prologueO_eq (pk : Party × Nat) (k : Nat) (c : Chan)
    (b : Bool) :
    proj c b (prologueO ord pk k)
      = proj c b [(wireIn pk, false, k), (askedIn pk, false, k)] := by
  unfold prologueO
  cases ord.walk pk with
  | replyFirst => rfl
  | queryFirst => exact proj_pair_swap (askedIn_ne_wireIn pk) false false k k c b

/-- O scope blocks project as E scope blocks: shared sends, bridged
prologue. -/
theorem proj_scopeBlockO_eq (pk : Party × Nat) (k : Nat) (c : Chan)
    (b : Bool) :
    proj c b (scopeBlockO sk ord pk k) = proj c b (scopeBlockE sk pk k) := by
  have hE : scopeBlockE sk pk k
      = [((wireIn pk, false, k) : Ev), (askedIn pk, false, k)]
        ++ scopeSendsE sk pk k := rfl
  unfold scopeBlockO
  rw [hE, proj_append, proj_append, proj_prologueO_eq]

/-- O walk traces project as E walk traces, per channel-side: the
per-scope bridge lifted over the stage's scopes. -/
theorem proj_walkEventsO_eq (pk : Party × Nat) (c : Chan) (b : Bool) :
    proj c b (walkEventsO sk ord pk) = proj c b (walkEventsE sk pk) := by
  simp only [walkEventsO, walkEventsE]
  induction (List.range (sk.stageLen pk.2)) with
  | nil => rfl
  | cons k ks ih =>
      rw [List.flatMap_cons, List.flatMap_cons, proj_append, proj_append,
        proj_scopeBlockO_eq, ih]

/-- The absorber's O block projects as its E block: the two receives
sit on distinct channels. -/
theorem proj_absorbBlockO_eq (j : Nat) (c : Chan) (b : Bool) :
    proj c b (absorbBlockO ord j)
      = proj c b [(Chan.wire Party.R 0, false, j),
          (Chan.leafRequests, false, j), (Chan.level Party.I 0, true, j)] := by
  unfold absorbBlockO
  cases ord.absorb with
  | replyFirst => rfl
  | queryFirst =>
      have hswap := proj_pair_swap leaf_ne_wireR0 false false j j c b
      calc proj c b ([((Chan.leafRequests : Chan), false, j),
              (Chan.wire Party.R 0, false, j)]
            ++ [((Chan.level Party.I 0 : Chan), true, j)])
          = proj c b [((Chan.leafRequests : Chan), false, j),
              (Chan.wire Party.R 0, false, j)]
            ++ proj c b [((Chan.level Party.I 0 : Chan), true, j)] :=
            proj_append ..
        _ = proj c b [((Chan.wire Party.R 0 : Chan), false, j),
              (Chan.leafRequests, false, j)]
            ++ proj c b [((Chan.level Party.I 0 : Chan), true, j)] := by
            rw [hswap]
        _ = proj c b ([((Chan.wire Party.R 0 : Chan), false, j),
              (Chan.leafRequests, false, j)]
            ++ [((Chan.level Party.I 0 : Chan), true, j)]) :=
            (proj_append ..).symm

/-- The absorber's O trace projects as `Sched.absorbEvents`, per
channel-side. -/
theorem proj_absorbEventsO_eq (c : Chan) (b : Bool) :
    proj c b (absorbEventsO sk ord) = proj c b (absorbEvents sk) := by
  simp only [absorbEventsO, absorbEvents]
  induction (List.range sk.totalLeafReqs) with
  | nil => rfl
  | cons j js ih =>
      simp only [List.flatMap_cons, proj_append, proj_absorbBlockO_eq, ih]

-- ============================================= the permutation bridge
-- For length/total facts not phrased through `proj`, the O traces are
-- pointwise permutations of the E traces.

/-- The ordered prologue is a permutation of the E prologue. -/
theorem prologueO_perm (pk : Party × Nat) (k : Nat) :
    (prologueO ord pk k).Perm
      [(wireIn pk, false, k), (askedIn pk, false, k)] := by
  unfold prologueO
  cases ord.walk pk with
  | replyFirst => exact List.Perm.refl _
  | queryFirst => exact List.Perm.swap _ _ _

/-- O scope blocks are permutations of E scope blocks. -/
theorem scopeBlockO_perm (pk : Party × Nat) (k : Nat) :
    (scopeBlockO sk ord pk k).Perm (scopeBlockE sk pk k) := by
  have hE : scopeBlockE sk pk k
      = [((wireIn pk, false, k) : Ev), (askedIn pk, false, k)]
        ++ scopeSendsE sk pk k := rfl
  unfold scopeBlockO
  rw [hE]
  exact (prologueO_perm ord pk k).append (List.Perm.refl _)

/-- O walk traces are pointwise permutations of E walk traces. -/
theorem walkEventsO_perm (pk : Party × Nat) :
    (walkEventsO sk ord pk).Perm (walkEventsE sk pk) := by
  simp only [walkEventsO, walkEventsE]
  induction (List.range (sk.stageLen pk.2)) with
  | nil => exact List.Perm.refl _
  | cons k ks ih =>
      rw [List.flatMap_cons, List.flatMap_cons]
      exact (scopeBlockO_perm sk ord pk k).append ih

/-- The absorber's O block is a permutation of its E block. -/
theorem absorbBlockO_perm (j : Nat) :
    (absorbBlockO ord j).Perm
      [(Chan.wire Party.R 0, false, j), (Chan.leafRequests, false, j),
        (Chan.level Party.I 0, true, j)] := by
  unfold absorbBlockO
  cases ord.absorb with
  | replyFirst => exact List.Perm.refl _
  | queryFirst => exact List.Perm.swap _ _ _

/-- The absorber's O trace is a permutation of `Sched.absorbEvents`. -/
theorem absorbEventsO_perm :
    (absorbEventsO sk ord).Perm (absorbEvents sk) := by
  simp only [absorbEventsO, absorbEvents]
  induction (List.range sk.totalLeafReqs) with
  | nil => exact List.Perm.refl _
  | cons j js ih =>
      simp only [List.flatMap_cons]
      exact (absorbBlockO_perm ord j).append ih

/-- The O event set has the E event set's size: the merge fuel and
every totals argument carry over unchanged. -/
theorem totalEventsO_eq : totalEventsO sk ord = totalEventsE sk := by
  have hW : ∀ pks : List (Party × Nat),
      ((pks.map (walkEventsO sk ord)).map List.length).sum
        = ((pks.map (walkEventsE sk)).map List.length).sum := by
    intro pks
    induction pks with
    | nil => rfl
    | cons p ps ih =>
        simp only [List.map_cons, List.sum_cons, ih,
          (walkEventsO_perm sk ord p).length_eq]
  simp only [totalEventsO, totalEventsE, procsO, procsE,
    List.map_append, List.sum_append, List.map_cons, List.sum_cons,
    (absorbEventsO_perm sk ord).length_eq]
  rw [hW]

-- =============================== canon + ownership at the O family

/-- The O family is memberwise contained in the d5 family: walk and
absorb entries are permutations, every other entry is shared. -/
theorem procs_mem_procsO :
    Forall2 (fun t t' => ∀ e ∈ t', e ∈ t) (procs sk) (procsO sk ord) := by
  simp only [procs, procsO]
  refine Forall2.append (Forall2.append (Forall2.append (Forall2.append
    (Forall2.self fun t _ e he => he)
    (forall2_map_self fun pk _ e he =>
      ((walkEventsO_perm sk ord pk).trans (walkEventsE_perm sk pk)).subset he))
    (Forall2.cons (fun e he => (absorbEventsO_perm sk ord).subset he) .nil))
    (Forall2.self fun t _ e he => he))
    (Forall2.self fun t _ e he => he)

/-- Sends own their trace index at `procsO`, every assignment:
`procs_snd_owned` through the membership transfer. -/
theorem procsO_snd_owned (hwf : sk.wellFormed = true) :
    Owned (sndOwner sk) true 0 (procsO sk ord) :=
  owned_of_forall2_mem (procs_snd_owned sk hwf) (procs_mem_procsO sk ord)

/-- Receives own their trace index at `procsO`, every assignment. -/
theorem procsO_rcv_owned (hwf : sk.wellFormed = true) :
    Owned (rcvOwner sk) false 0 (procsO sk ord) :=
  owned_of_forall2_mem (procs_rcv_owned sk hwf) (procs_mem_procsO sk ord)

/-- Every `procsO` trace's every channel-side projection is
canon-shaped: the walk and absorb arms ride the projection bridges,
every other arm is `procs_canon`'s. -/
theorem procsO_canon (c : Chan) (b : Bool) :
    ∀ t ∈ procsO sk ord, ∃ m, proj c b t = canon c b m := by
  intro t ht
  simp only [procsO, List.mem_append, List.mem_cons,
    List.not_mem_nil, or_false, List.mem_map] at ht
  rcases ht with ((((rfl | rfl) | ⟨pk, -, rfl⟩) | rfl) | ⟨pk, -, rfl⟩)
    | rfl | rfl
  · exact iopen_canon sk c b
  · exact ropen_canon sk c b
  · obtain ⟨m, hm⟩ := walk_canon sk pk c b
    exact ⟨m, by rw [proj_walkEventsO_eq, proj_walkEventsE_eq]; exact hm⟩
  · obtain ⟨m, hm⟩ := absorb_canon sk c b
    exact ⟨m, by rw [proj_absorbEventsO_eq]; exact hm⟩
  · exact asm_canon sk pk c b
  · by_cases h1 : c = Chan.rootret ∧ b = false
    · obtain ⟨rfl, rfl⟩ := h1
      exact ⟨1, by rw [proj_cons_self, proj_nil, canon_one]⟩
    · refine ⟨0, proj_eq_nil fun e he hc hcb => ?_⟩
      rcases he with _ | ⟨_, he⟩
      · exact h1 ⟨hc.symm, hcb.symm⟩
      · cases he
  · exact fin_canon sk c b

-- =================================== the family instances (FamOK)

/-- At a reply-first ABSORBER the O absorb trace is the base absorb
trace, literally: the block dispatch collapses to the E order. -/
theorem absorbEventsO_rf_absorb (h : ord.absorb = .replyFirst) :
    absorbEventsO sk ord = absorbEvents sk := by
  simp only [absorbEventsO, absorbBlockO, h]
  rfl

/-- The O family's pump suffix at a reply-first absorber is
`weavePumps`: walks are manual, and the absorb trace collapses. -/
theorem procsO_drop_pumps (h : ord.absorb = .replyFirst) :
    (procsO sk ord).drop (manCount sk) = weavePumps sk := by
  have hsplit : procsO sk ord
      = ([iopenEvents sk, ropenEvents sk]
          ++ ((List.range sk.rootH).map fun i =>
            ((if (sk.rootH - 1 - i) % 2 == 1 then Party.I else Party.R,
              sk.rootH - 1 - i) : Party × Nat)).map (walkEventsO sk ord))
        ++ weavePumps sk := by
    simp [procsO, weavePumps, List.append_assoc,
      absorbEventsO_rf_absorb sk ord h]
  have hlen : ([iopenEvents sk, ropenEvents sk]
      ++ ((List.range sk.rootH).map fun i =>
        ((if (sk.rootH - 1 - i) % 2 == 1 then Party.I else Party.R,
          sk.rootH - 1 - i) : Party × Nat)).map (walkEventsO sk ord)).length
      = manCount sk := by
    simp [manCount]
    omega
  rw [hsplit, ← hlen, List.drop_left]

/-- `FamOK` at `procsO`, for assignments whose ABSORBER is reply-first
(walk assignments fully free).

The restriction is structural, not a proof gap: `FamOK.pumps` is a
LITERAL list equality — the family's pump suffix must BE
`weavePumps sk` — and a query-first absorber genuinely reorders the
absorb trace's receives, so the field is false at
`ord.absorb = .queryFirst` on any skeleton with a leaf request. The
canon and ownership fields hold for every assignment (`procsO_canon`,
`procsO_snd_owned`, `procsO_rcv_owned`); only the pump pin needs the
hypothesis. Serving the full class needs a proj-shaped (or
ord-parameterized) pump field in `FamOK` — an existing-file change,
recorded for the lead. -/
theorem famOK_procsO (hwf : sk.wellFormed = true)
    (habs : ord.absorb = .replyFirst) : FamOK sk (procsO sk ord) :=
  ⟨procsO_canon sk ord, procsO_snd_owned sk ord hwf,
    procsO_rcv_owned sk ord hwf, procsO_drop_pumps sk ord habs⟩

-- ================================== the manual rows (ManRows)

/-- The stage-`h` walk sits at slot `walkIdx h` of the O family: the
family swaps each walk's trace in place. -/
theorem procsO_walk {h : Nat} (hh : h < sk.rootH) :
    (procsO sk ord)[walkIdx sk h]? = some (walkEventsO sk ord (wpk h)) := by
  unfold procsO
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

/-- The responder opener sits at slot 1 of the O family. -/
theorem procsO_ropen : (procsO sk ord)[1]? = some (ropenEvents sk) := rfl

/-- The O family reads the d5 manual rows through the two projection
bridges (O→E, then E→d5); unconditional — `ManRows` is proj-shaped
throughout. -/
theorem manRows_procsO : ManRows sk (procsO sk ord) :=
  ⟨fun hhr => ⟨_, procsO_walk sk ord hhr, fun c b =>
      (proj_walkEventsO_eq sk ord _ c b).trans
        (proj_walkEventsE_eq sk _ c b)⟩,
    ⟨_, procsO_ropen sk ord, fun _ _ => rfl⟩⟩

-- ============================ the generic-merge instances at procsO
-- Thin instantiations: the merge layer is generic over the trace list.

/-- The merge invariant at the O merge's final state. -/
theorem scheduleO_inv : MInv sk (procsO sk ord) (finalStateO sk ord) :=
  mergeN_preserves sk _ (minv_init sk (procsO sk ord))

/-- `trace_monotone` for the O merge. -/
theorem trace_monotoneO :
    Forall2 (fun t r => ∃ pre, t = pre ++ r ∧ pre.Sublist (scheduleO sk ord))
      (procsO sk ord) (finalStateO sk ord).rem :=
  (scheduleO_inv sk ord).rem_struct

/-- E1-respect of the O schedule, counted. -/
theorem scheduleO_e1 (k : Nat) (c : Chan) (n : Nat)
    (h : (scheduleO sk ord)[k]? = some (c, false, n)) :
    n < sndCount c ((scheduleO sk ord).take k) :=
  (scheduleO_inv sk ord).e1_hist k c n h

/-- E2-respect of the O schedule, counted. -/
theorem scheduleO_e2 (k : Nat) (c : Chan) (n : Nat)
    (h : (scheduleO sk ord)[k]? = some (c, true, n)) :
    n < rcvCount c ((scheduleO sk ord).take k) + sk.cap c :=
  (scheduleO_inv sk ord).e2_hist k c n h

/-- `schedule_count` for the O merge. -/
theorem scheduleO_count (p : Ev → Bool) :
    ((scheduleO sk ord).filter p).length
      = emittedCount p (procsO sk ord) (finalStateO sk ord).rem :=
  (scheduleO_inv sk ord).out_count p

-- ======================= the O schedule's numbering corollaries

/-- Prefixes project to prefixes: the emitted part of a trace never
projects past the trace's own canon stream. -/
private theorem proj_prefixO {c : Chan} {b : Bool} {pre r : List Ev} :
    proj c b pre <+: proj c b (pre ++ r) := by
  rw [proj_append]
  exact List.prefix_append ..

/-- Canon shape of the O schedule's projections, every assignment: on
every channel and side the O merge emits seqs `0, 1, 2, …` in
order. -/
theorem scheduleO_proj_canon (hwf : sk.wellFormed = true) (c : Chan)
    (b : Bool) : ∃ m, proj c b (scheduleO sk ord) = canon c b m := by
  have howned : Owned (if b then sndOwner sk else rcvOwner sk) b 0
      (procsO sk ord) := by
    cases b
    · exact procsO_rcv_owned sk ord hwf
    · exact procsO_snd_owned sk ord hwf
  obtain ⟨pre, hsub, hpre⟩ :=
    emitted_canon (trace_monotoneO sk ord) howned (procsO_canon sk ord c b)
  refine ⟨emittedCount (fun e => decide (e.1 = c) && (e.2.1 == b))
    (procsO sk ord) (finalStateO sk ord).rem, ?_⟩
  have hcount : (proj c b (scheduleO sk ord)).length
      = emittedCount (fun e => decide (e.1 = c) && (e.2.1 == b))
        (procsO sk ord) (finalStateO sk ord).rem := scheduleO_count sk ord _
  have hlenpre : (proj c b pre).length
      = emittedCount (fun e => decide (e.1 = c) && (e.2.1 == b))
        (procsO sk ord) (finalStateO sk ord).rem := by
    rw [hpre]
    simp [canon]
  have hsubp : (proj c b pre).Sublist (proj c b (scheduleO sk ord)) :=
    hsub.filter _
  have heq : proj c b pre = proj c b (scheduleO sk ord) :=
    hsubp.eq_of_length (by rw [hlenpre, hcount])
  rw [← heq, hpre]

/-- Positional E1 for the O schedule: every receive is preceded by the
send with its own seq. -/
theorem scheduleO_e1_pos (hwf : sk.wellFormed = true) (k : Nat) (c : Chan)
    (n : Nat) (h : (scheduleO sk ord)[k]? = some (c, false, n)) :
    ∃ j, j < k ∧ (scheduleO sk ord)[j]? = some (c, true, n) := by
  have hcount := scheduleO_e1 sk ord k c n h
  rw [sndCount_eq_proj] at hcount
  obtain ⟨m, hm⟩ := scheduleO_proj_canon sk ord hwf c true
  have hpref : proj c true ((scheduleO sk ord).take k)
      <+: canon c true m := by
    rw [← hm]
    conv => rhs; rw [← List.take_append_drop k (scheduleO sk ord)]
    exact proj_prefixO
  have htake := prefix_canon hpref
  have hmem : ((c, true, n) : Ev)
      ∈ proj c true ((scheduleO sk ord).take k) := by
    rw [htake]
    exact List.mem_map.2 ⟨n, List.mem_range.2 hcount, rfl⟩
  have hmem' : ((c, true, n) : Ev) ∈ (scheduleO sk ord).take k :=
    (List.mem_filter.1 hmem).1
  obtain ⟨j, hj⟩ := List.mem_iff_getElem?.1 hmem'
  rw [List.getElem?_take] at hj
  by_cases hjk : j < k
  · rw [if_pos hjk] at hj
    exact ⟨j, hjk, hj⟩
  · rw [if_neg hjk] at hj
    cases hj

/-- Two positions holding one event force a duplicate. -/
private theorem two_at_ltO {l : List Ev} {i j : Nat} {e : Ev} (hij : i < j)
    (hi : l[i]? = some e) (hj : l[j]? = some e) : 2 ≤ l.count e := by
  have h1 : e ∈ l.take j :=
    List.mem_iff_getElem?.2
      ⟨i, by rw [List.getElem?_take, if_pos hij]; exact hi⟩
  have h2 : e ∈ l.drop j :=
    List.mem_iff_getElem?.2 ⟨0, by rw [List.getElem?_drop]; simpa using hj⟩
  have hc1 : 0 < (l.take j).count e := List.count_pos_iff.2 h1
  have hc2 : 0 < (l.drop j).count e := List.count_pos_iff.2 h2
  have hsplit : l.count e = (l.take j).count e + (l.drop j).count e := by
    conv => lhs; rw [← List.take_append_drop j l]
    exact List.count_append
  omega

/-- τ injectivity for the O schedule: the O merge holds each event at
most once, so position-in-schedule is a well-defined timestamp. -/
theorem scheduleO_inj (hwf : sk.wellFormed = true) {i j : Nat} {e : Ev}
    (hi : (scheduleO sk ord)[i]? = some e)
    (hj : (scheduleO sk ord)[j]? = some e) : i = j := by
  obtain ⟨c, b, n⟩ := e
  obtain ⟨m, hm⟩ := scheduleO_proj_canon sk ord hwf c b
  have hpred : (fun e : Ev => decide (e.1 = c) && (e.2.1 == b))
      (c, b, n) = true := by simp
  have hcle : (scheduleO sk ord).count (c, b, n) ≤ 1 := by
    rw [← List.count_filter
      (p := fun e : Ev => decide (e.1 = c) && (e.2.1 == b))
      (l := scheduleO sk ord) hpred]
    rw [show (scheduleO sk ord).filter _ = proj c b (scheduleO sk ord)
        from rfl, hm, count_canon]
    split <;> omega
  by_contra hne
  rcases Nat.lt_or_ge i j with hij | hij
  · have := two_at_ltO hij hi hj
    omega
  · have := two_at_ltO (by omega : j < i) hj hi
    omega

end StreamingMirror.Ord
