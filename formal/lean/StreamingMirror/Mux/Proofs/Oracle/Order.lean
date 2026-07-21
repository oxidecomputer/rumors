/-
The oracle's order layer (MUX-ADJUDICATION.md §3 T5, stage-3 track E):
the send projection of the canonical schedule, the oracle strategy of
record, and the run invariant that ties a party's push history to the
projection's τ-prefix.

# The oracle of record, after the P2/muxprobe reversal

The adjudication's primary form — `ofSchedule (demandOrder sk d)`, the
RECEIVE projection of τ pushed as a precomputed list — was executably
refuted (MUX-PROGRESS.md log 2026-07-21, the π-eligibility failure;
STAGE0-GATES.md P2), so T5 takes the recorded fallback. The fallback's
"state-feedback / exit-certificate" formulation is here realized in its
sharpest form: the strategy consults its observation history ONLY
through the count of its own flush receipts, and pushes its wire sends
in the order the canonical schedule `scheduleE` fires them — the SEND
projection `sendProj`, indexed by own-push count (`oracle`). The state
feedback the probe's oracle read off the simulator state is not needed:
per-stream demux slots absorb exactly the cross-stream skew that broke
the receive projection, and the τ-argmin proof (Oracle.lean) shows a
send-order pipe can never bury a frame its consumer needs first.

# The run invariant

`OracleInv` is the whole engine: along an oracle-driven run, the events
a machine has pushed are EXACTLY the first `K` entries of its send
projection, where `K` is its flush-receipt count. Its preservation
needs nothing from the base state — pushes are the only observations
that move it, and the oracle names precisely the projection's next
entry — so the induction here is over histories alone, disjoint from
the `MuxInv` ground-fact preservation. At a stuck state the invariant
turns pipe positions into `scheduleE` positions
(`pipe_first_frame_pos`, Oracle.lean): FIFO order becomes τ-order,
which is what the chase-style contradiction consumes.
-/
import StreamingMirror.Mux.Proofs.Chase

namespace StreamingMirror.Mux

open Model
open Sched (Ev scheduleE performed evIdx proj canon sndCount)

variable {sk : Skel}

-- ====================================================== the projection

/-- Party `p`'s wire send events, in canonical `.impl` schedule order:
the τ-prefix source the oracle pushes from. -/
def sendProj (sk : Skel) (p : Party) : List Ev :=
  (scheduleE sk).filter fun e => isWire e.1 && (wireParty e.1 == p) && e.2.1

/-- T5's oracle of record: push own wire sends in `sendProj` order, one
entry per flush receipt; idle while the next entry is not yet in hand.

A pure function of the full bidirectional skeleton and the machine's
own push count — full-skeleton knowledge plus current observation,
exactly C2's chartered input (MUX-ADJUDICATION §1.3) — and NON-adaptive
in the same sense as the refuted receive-projection pusher: entry `k`
of a fixed list names the `k`-th push. What the receive projection got
wrong was the ORDER, not the information; see `static_oracle_jams`
(Oracle.lean) and the module doc. -/
def oracle (p : Party) : Strategy := fun sk tr =>
  ((sendProj sk p)[(pushHeights tr).length]?).map fun e => wireHeight e.1

/-- The projection embeds in the schedule. -/
theorem sendProj_sublist (sk : Skel) (p : Party) :
    (sendProj sk p).Sublist (scheduleE sk) :=
  List.filter_sublist

/-- Projection membership, unpacked: a scheduled wire send of `p`. -/
theorem sendProj_mem {p : Party} {e : Ev} (he : e ∈ sendProj sk p) :
    e ∈ scheduleE sk ∧ ∃ h n, e = (Chan.wire p h, true, n) := by
  obtain ⟨hsched, hpred⟩ := List.mem_filter.mp he
  obtain ⟨c, b, n⟩ := e
  simp only [Bool.and_eq_true, beq_iff_eq] at hpred
  obtain ⟨⟨hw, hparty⟩, hb⟩ := hpred
  obtain ⟨q, h, hc⟩ := isWire_eq hw
  subst hc
  refine ⟨hsched, h, n, ?_⟩
  have hq : q = p := hparty
  have hb' : b = true := hb
  rw [hq, hb']

/-- Projection membership, packed: every scheduled wire send of `p` is
a projection entry. -/
theorem mem_sendProj {p : Party} {h n : Nat}
    (hmem : ((Chan.wire p h, true, n) : Ev) ∈ scheduleE sk) :
    ((Chan.wire p h, true, n) : Ev) ∈ sendProj sk p :=
  List.mem_filter.mpr ⟨hmem, by simp [isWire, wireParty]⟩

/-- An ordered pair of positions embeds as an ordered pair sublist. -/
private theorem pair_sublist_getElem {α : Type _} :
    ∀ {l : List α} {i j : Nat} (hij : i < j) (hj : j < l.length),
      [l[i]'(by omega), l[j]'hj].Sublist l := by
  intro l
  induction l with
  | nil => intro i j hij hj; simp at hj
  | cons a l ih =>
      intro i j hij hj
      match i, j with
      | 0, j + 1 =>
          refine List.Sublist.cons_cons a (List.singleton_sublist.mpr ?_)
          exact List.getElem_mem _
      | i + 1, j + 1 =>
          exact List.Sublist.cons a (ih (by omega) (by simpa using hj))

/-- Projection positions are τ-positions: entries at ordered projection
indices sit at ordered `scheduleE` indices. -/
theorem sendProj_evIdx_lt (hwf : sk.wellFormed = true) {p : Party}
    {i j : Nat} {x y : Ev} (hij : i < j)
    (hx : (sendProj sk p)[i]? = some x)
    (hy : (sendProj sk p)[j]? = some y) :
    evIdx x (scheduleE sk) < evIdx y (scheduleE sk) := by
  obtain ⟨hjl, hyj⟩ := List.getElem?_eq_some_iff.mp hy
  obtain ⟨hil, hxi⟩ := List.getElem?_eq_some_iff.mp hx
  have hpair := pair_sublist_getElem (l := sendProj sk p) hij hjl
  rw [hxi, hyj] at hpair
  exact Sched.pos_lt_of_pair (Sched.scheduleE_count_le_oneE sk hwf)
    (hpair.trans (sendProj_sublist sk p))

-- =================================================== canonical seqs

/-- The projection keeps every send of its own channels, so its
per-channel send projections agree with the schedule's. -/
theorem proj_sendProj (sk : Skel) (p : Party) (h : Nat) :
    proj (Chan.wire p h) true (sendProj sk p)
      = proj (Chan.wire p h) true (scheduleE sk) := by
  unfold Sched.proj sendProj
  rw [List.filter_filter]
  refine List.filter_congr fun e _ => ?_
  cases hpred : (decide (e.1 = Chan.wire p h) && (e.2.1 == true)) with
  | false => simp
  | true =>
      simp only [Bool.and_eq_true, decide_eq_true_eq, beq_iff_eq] at hpred
      obtain ⟨hc, hb⟩ := hpred
      simp [hc, hb, isWire, wireParty]

/-- A canon prefix is the canon of its own length. -/
private theorem canon_take (c : Chan) (b : Bool) {k M : Nat} (hk : k ≤ M) :
    (canon c b M).take k = canon c b k := by
  unfold Sched.canon
  rw [← List.map_take, List.take_range, Nat.min_eq_left hk]

/-- In a canonically-numbered stream, the entry at position `K` carries
seq = the count of its channel-side among the first `K` entries: the
"next frame" is literally numbered by what precedes it. -/
theorem seq_eq_sndCount_take {W : List Ev} {c : Chan} {n K : Nat}
    (hcanon : proj c true W = canon c true (proj c true W).length)
    (hK : W[K]? = some (c, true, n)) :
    n = sndCount c (W.take K) := by
  have htake : W.take (K + 1) = W.take K ++ [(c, true, n)] := by
    rw [List.take_add_one, hK]
    rfl
  -- the (K+1)-prefix's projection is a canon prefix ending in (c,true,n)
  have hpre : proj c true (W.take (K + 1)) <+: proj c true W :=
    (List.take_prefix _ _).filter _
  obtain ⟨suf, hsuf⟩ := hpre
  have hlen : (proj c true (W.take (K + 1))).length
      ≤ (proj c true W).length := by
    rw [← hsuf]
    simp
  have hcanon' : proj c true (W.take (K + 1))
      = canon c true (proj c true (W.take (K + 1))).length := by
    have h1 := congrArg
      (List.take (proj c true (W.take (K + 1))).length) hcanon
    rw [canon_take _ _ hlen] at h1
    rw [← h1, ← hsuf]
    exact (List.take_left ..).symm
  -- split the (K+1)-projection as K-projection plus the new entry
  have hsplit : proj c true (W.take (K + 1))
      = proj c true (W.take K) ++ [(c, true, n)] := by
    rw [htake]
    unfold Sched.proj
    rw [List.filter_append]
    simp
  have hlen2 : (proj c true (W.take (K + 1))).length
      = (proj c true (W.take K)).length + 1 := by
    rw [hsplit]
    simp
  -- canon (m+1) ends in seq m; the split ends in seq n; lengths match
  have hcanon2 : proj c true (W.take K) ++ [((c, true, n) : Ev)]
      = canon c true ((proj c true (W.take K)).length + 1) := by
    rw [← hsplit, hcanon', hlen2]
  have hcsucc : canon c true ((proj c true (W.take K)).length + 1)
      = canon c true (proj c true (W.take K)).length
        ++ [(c, true, (proj c true (W.take K)).length)] := by
    unfold Sched.canon
    rw [List.range_succ, List.map_append]
    simp
  rw [hcsucc] at hcanon2
  have hlencanon : (canon c true (proj c true (W.take K)).length).length
      = (proj c true (W.take K)).length := by
    unfold Sched.canon
    simp
  have hlast := List.append_inj_right hcanon2 (by rw [hlencanon])
  have hn : n = (proj c true (W.take K)).length := by
    have := congrArg (fun l : List Ev => (l.getD 0 (c, true, 0)).2.2) hlast
    simpa using this
  rw [hn, Sched.sndCount_eq_proj]

-- ================================================ numbered push events

/-- The wire send events of a height list, numbered by occurrence: the
`i`-th entry of height `h` carries seq = the count of `h` before it. -/
def evsOf (p : Party) (l : List Nat) : List Ev :=
  (List.range l.length).map fun i =>
    (Chan.wire p (l.getD i 0), true, (l.take i).count (l.getD i 0))

/-- A machine's pushes as numbered send events: positional identity is
seq identity (the canonical-numbering reading of the flush history). -/
def pushEvs (p : Party) (tr : List MObs) : List Ev :=
  evsOf p (pushHeights tr)

theorem evsOf_length (p : Party) (l : List Nat) :
    (evsOf p l).length = l.length := by
  unfold evsOf
  simp

/-- Numbered events, read positionally. -/
theorem evsOf_getElem? (p : Party) {l : List Nat} {i : Nat}
    (hi : i < l.length) :
    (evsOf p l)[i]?
      = some (Chan.wire p (l.getD i 0), true,
          (l.take i).count (l.getD i 0)) := by
  unfold evsOf
  rw [List.getElem?_map, List.getElem?_range hi]
  rfl

/-- Numbering commutes with appending one push. -/
theorem evsOf_snoc (p : Party) (l : List Nat) (h : Nat) :
    evsOf p (l ++ [h])
      = evsOf p l ++ [(Chan.wire p h, true, l.count h)] := by
  unfold evsOf
  rw [List.length_append, List.length_singleton, List.range_succ,
    List.map_append]
  congr 1
  · refine List.map_congr_left fun i hi => ?_
    rw [List.mem_range] at hi
    have h1 : (l ++ [h]).getD i 0 = l.getD i 0 := by
      rw [List.getD_eq_getElem?_getD, List.getD_eq_getElem?_getD,
        List.getElem?_append_left hi]
    rw [h1, List.take_append_of_le_length (Nat.le_of_lt hi)]
  · have hget : (l ++ [h]).getD l.length 0 = h := by
      rw [List.getD_eq_getElem?_getD,
        List.getElem?_append_right (Nat.le_refl _)]
      simp
    rw [List.map_singleton, hget, List.take_left]

/-- Per-channel counts of the numbered events are height counts. -/
theorem sndCount_evsOf (p : Party) (h : Nat) (l : List Nat) :
    sndCount (Chan.wire p h) (evsOf p l) = l.count h := by
  suffices key : ∀ r : List Nat,
      sndCount (Chan.wire p h) (evsOf p r.reverse) = r.reverse.count h by
    have := key l.reverse
    simpa using this
  intro r
  induction r with
  | nil => rfl
  | cons a r ih =>
      rw [List.reverse_cons, evsOf_snoc]
      unfold Sched.sndCount at ih ⊢
      rw [List.filter_append, List.length_append, ih, List.count_append]
      congr 1
      by_cases hh : a = h
      · subst hh
        simp
      · have hne : (Chan.wire p a = Chan.wire p h) = False := by
          simp [hh]
        simp [hne, hh]

/-- Every numbered event's seq is below its height's total count. -/
theorem evsOf_mem_inv {p : Party} {l : List Nat} {e : Ev}
    (he : e ∈ evsOf p l) :
    ∃ h n, e = (Chan.wire p h, true, n) ∧ n < l.count h := by
  suffices key : ∀ r : List Nat, e ∈ evsOf p r.reverse →
      ∃ h n, e = (Chan.wire p h, true, n) ∧ n < r.reverse.count h by
    have := key l.reverse (by simpa using he)
    simpa using this
  intro r
  induction r with
  | nil => intro he'; cases he'
  | cons a r ih =>
      intro he'
      rw [List.reverse_cons, evsOf_snoc] at he'
      rcases List.mem_append.mp he' with hin | hnew
      · obtain ⟨h, n, rfl, hlt⟩ := ih hin
        refine ⟨h, n, rfl, ?_⟩
        rw [List.reverse_cons, List.count_append]
        omega
      · rw [List.mem_singleton] at hnew
        refine ⟨a, r.reverse.count a, hnew, ?_⟩
        rw [List.reverse_cons, List.count_append]
        simp

-- ===================================================== the run invariant

/-- The oracle's run invariant: the machine's numbered push history is
exactly the τ-prefix of its send projection at its own push count.

Preservation (below) is over oracle-driven runs only — the oracle is a
specific strategy, and this is precisely the fact its naming discipline
maintains. `MuxInv` (Chase/Ground) carries the ground facts shared by
every strategy; this invariant carries the one thing only the oracle
guarantees: pushes happen in τ order. -/
def OracleInv (sk : Skel) (p : Party) (s : MState) : Prop :=
  pushEvs p (s.hist p)
    = (sendProj sk p).take (pushHeights (s.hist p)).length

/-- The invariant holds initially: no pushes, empty prefix. -/
theorem oracleInv_init (sk : Skel) (p : Party) :
    OracleInv sk p (init sk) := rfl

/-- Push histories see only `.pushed` observations. -/
theorem pushHeights_append_act (tr : List MObs) (a : Action) :
    pushHeights (tr ++ [.act a]) = pushHeights tr := by
  unfold pushHeights
  rw [List.filterMap_append]
  simp

/-- Push histories see only `.pushed` observations (delivery twin). -/
theorem pushHeights_append_delivered (tr : List MObs) (h : Nat) :
    pushHeights (tr ++ [.delivered h]) = pushHeights tr := by
  unfold pushHeights
  rw [List.filterMap_append]
  simp

/-- A flush receipt appends its height. -/
theorem pushHeights_append_pushed (tr : List MObs) (h : Nat) :
    pushHeights (tr ++ [.pushed h]) = pushHeights tr ++ [h] := by
  unfold pushHeights
  rw [List.filterMap_append]
  rfl

/-- The invariant is untouched by any observation that is not a push. -/
theorem oracleInv_of_hist {s s' : MState} {p : Party}
    (ho : OracleInv sk p s)
    (hh : s'.hist p = s.hist p
      ∨ (∃ a, s'.hist p = s.hist p ++ [.act a])
      ∨ ∃ h, s'.hist p = s.hist p ++ [.delivered h]) :
    OracleInv sk p s' := by
  unfold OracleInv pushEvs
  rcases hh with hh | ⟨a, hh⟩ | ⟨h, hh⟩
  · rw [hh]; exact ho
  · rw [hh, pushHeights_append_act]; exact ho
  · rw [hh, pushHeights_append_delivered]; exact ho

/-- `applyBase` records one `.act` observation and leaves the pipes. -/
theorem applyBase_parts {ax : AxMode} {a : Action} {s s' : MState}
    (hstep : applyBase sk ax a s = some s') :
    s'.pipe = s.pipe
      ∧ s'.hist = recordObs s.hist (actionParty a) (.act a) := by
  have hshape : applyBase sk ax a s = none
      ∨ applyBase sk ax a s = (Model.apply sk ax a s.base).map fun b =>
          { s with base := b
                   hist := recordObs s.hist (actionParty a) (.act a) } := by
    unfold applyBase
    dsimp only
    repeat' split
    all_goals first | exact Or.inl rfl | exact Or.inr rfl
  rcases hshape with hnone | hmap
  · rw [hnone] at hstep; cases hstep
  · rw [hmap] at hstep
    cases hb : Model.apply sk ax a s.base with
    | none => rw [hb] at hstep; cases hstep
    | some b =>
        rw [hb] at hstep
        injection hstep with hs'
        rw [← hs']
        exact ⟨rfl, rfl⟩

/-- `firePush` records exactly one flush receipt and one pipe entry. -/
theorem firePush_parts {C : Nat} {p : Party} {h : Nat} {s s' : MState}
    (hstep : firePush sk C p h s = some s') :
    s'.hist = recordObs s.hist p (.pushed h)
      ∧ s'.pipe = fun q => if q == p then s.pipe q ++ [Chan.wire p h]
          else s.pipe q := by
  unfold firePush at hstep
  dsimp only at hstep
  split at hstep
  case isFalse => cases hstep
  case isTrue =>
    repeat' (split at hstep)
    all_goals first
      | (injection hstep with hs'; rw [← hs']; exact ⟨rfl, rfl⟩)
      | cases hstep

/-- One oracle-named push extends the τ-prefix by exactly its next
entry: the preservation engine. -/
theorem oracleInv_push (hwf : sk.wellFormed = true) {C : Nat}
    {p' : Party} {h : Nat} {s s' : MState}
    (horacle : oracle p' sk (s.hist p') = some h)
    (hfire : firePush sk C p' h s = some s')
    (ho : OracleInv sk p' s) : OracleInv sk p' s' := by
  obtain ⟨hhist, -⟩ := firePush_parts hfire
  have htr : s'.hist p' = s.hist p' ++ [.pushed h] := by
    rw [hhist]
    unfold recordObs
    simp
  -- the oracle named the projection's next entry
  unfold oracle at horacle
  cases hget : (sendProj sk p')[(pushHeights (s.hist p')).length]? with
  | none => rw [hget] at horacle; cases horacle
  | some e =>
      rw [hget] at horacle
      have hmem : e ∈ sendProj sk p' := List.mem_of_getElem? hget
      obtain ⟨-, he, ne, hedec⟩ := sendProj_mem hmem
      have hh : h = he := by
        have hor : some (wireHeight e.1) = some h := by
          simpa using horacle
        rw [hedec] at hor
        simpa [wireHeight] using hor.symm
      subst hh
      -- its seq is the current push count of its stream
      have hcanon : proj (Chan.wire p' h) true (sendProj sk p')
          = canon (Chan.wire p' h) true
              (proj (Chan.wire p' h) true (sendProj sk p')).length := by
        rw [proj_sendProj]
        exact scheduleE_canon_self hwf _ true
      have hseq : ne = sndCount (Chan.wire p' h)
          ((sendProj sk p').take (pushHeights (s.hist p')).length) :=
        seq_eq_sndCount_take hcanon (hedec ▸ hget)
      have hcnt : ne = (pushHeights (s.hist p')).count h := by
        rw [hseq, ← ho, pushEvs, sndCount_evsOf]
      -- extend both sides by one
      have ho' : evsOf p' (pushHeights (s.hist p'))
          = (sendProj sk p').take (pushHeights (s.hist p')).length := ho
      unfold OracleInv pushEvs
      rw [htr, pushHeights_append_pushed, evsOf_snoc,
        List.length_append, List.length_singleton, List.take_add_one,
        hget]
      simp only [Option.toList_some]
      rw [← ho', hedec, hcnt]

/-- The invariant is preserved by every muxed step of an oracle-driven
run, for both parties at once. -/
theorem oracleInv_step (hwf : sk.wellFormed = true) {C : Nat}
    {a : MAction} {s s' : MState}
    (hstep : apply sk .impl C (oracle .I) (oracle .R) a s = some s')
    (hoI : OracleInv sk .I s) (hoR : OracleInv sk .R s) :
    OracleInv sk .I s' ∧ OracleInv sk .R s' := by
  cases a with
  | base a =>
      obtain ⟨-, hhist⟩ := applyBase_parts (a := a) hstep
      have hcase : ∀ q : Party, s'.hist q = s.hist q
          ∨ s'.hist q = s.hist q ++ [.act a] := by
        intro q
        rw [hhist]
        unfold recordObs
        by_cases hq : (q == actionParty a) = true
        · rw [if_pos hq]; exact Or.inr rfl
        · rw [if_neg (by simpa using hq)]; exact Or.inl rfl
      constructor <;>
        · refine oracleInv_of_hist (by assumption) ?_
          rcases hcase _ with hc | hc
          · exact Or.inl hc
          · exact Or.inr (Or.inl ⟨a, hc⟩)
  | push p' =>
      have hσ : apply sk .impl C (oracle .I) (oracle .R) (.push p') s
          = (match oracle p' sk (s.hist p') with
             | some h => firePush sk C p' h s
             | none => none) := by
        cases p' <;> rfl
      rw [hσ] at hstep
      cases horc : oracle p' sk (s.hist p') with
      | none => rw [horc] at hstep; cases hstep
      | some h =>
          rw [horc] at hstep
          obtain ⟨hhist, -⟩ := firePush_parts hstep
          have hother : ∀ q, (q == p') = false → s'.hist q = s.hist q := by
            intro q hq
            rw [hhist]
            unfold recordObs
            rw [if_neg (by simp [hq])]
          cases p' with
          | I =>
              refine ⟨oracleInv_push hwf horc hstep hoI, ?_⟩
              exact oracleInv_of_hist hoR (Or.inl (hother .R rfl))
          | R =>
              refine ⟨?_, oracleInv_push hwf horc hstep hoR⟩
              exact oracleInv_of_hist hoI (Or.inl (hother .I rfl))
  | deliver p' =>
      simp only [apply] at hstep
      cases hp : s.pipe p' with
      | nil =>
          rw [hp] at hstep
          dsimp only at hstep
          cases hstep
      | cons c rest =>
          rw [hp] at hstep
          dsimp only at hstep
          split at hstep
          · injection hstep with hs'
            have hhist : s'.hist = recordObs s.hist p'.other
                (.delivered (wireHeight c)) := by rw [← hs']
            have hcase : ∀ q : Party, s'.hist q = s.hist q
                ∨ s'.hist q = s.hist q ++ [.delivered (wireHeight c)] := by
              intro q
              rw [hhist]
              unfold recordObs
              by_cases hq : (q == p'.other) = true
              · rw [if_pos hq]; exact Or.inr rfl
              · rw [if_neg (by simpa using hq)]; exact Or.inl rfl
            constructor <;>
              · refine oracleInv_of_hist (by assumption) ?_
                rcases hcase _ with hc | hc
                · exact Or.inl hc
                · exact Or.inr (Or.inr ⟨wireHeight c, hc⟩)
          · cases hstep

/-- The invariant holds at every reachable state of an oracle-driven
run: the induction T5's stuck-state argument stands on. -/
theorem oracleInv_reachable (hwf : sk.wellFormed = true) {C : Nat}
    {s : MState}
    (hr : MReachable sk .impl C (oracle .I) (oracle .R) s) :
    OracleInv sk .I s ∧ OracleInv sk .R s := by
  induction hr with
  | init => exact ⟨oracleInv_init sk .I, oracleInv_init sk .R⟩
  | step a _ hstep ih => exact oracleInv_step hwf hstep ih.1 ih.2

end StreamingMirror.Mux
