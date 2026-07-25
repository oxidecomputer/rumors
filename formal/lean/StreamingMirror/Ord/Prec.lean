/-
Dep-closure of the O future: `DepOK` transfers from the E future to
the O future — per CHANNEL, not per the E campaign's wire/asked class.

# Why the class refines

`manDep` is within-channel throughout: a receive's dep is its OWN
channel's same-seq send, a cap-1 send's dep its OWN channel's previous
receive (`manDep_chan`) — the manual dependency of a prologue receive
is never the other prologue receive, so the query-first swap never
flips a dep pair. But PrecE's transfer class `isWA` (wire OR asked,
either side) contains BOTH prologue receives, and a query-first walk
genuinely swaps them — the two futures' `isWA` filters differ, so
`weave_filter_isWA`'s shape is false between O and E and the E
transfer cannot be consumed as-is. Refining the class to one channel
(`onChan`) restores the filter equality: the two receives of any
prologue live on DISTINCT channels (`wireIn` vs `askedIn`), so every
single-channel filter sees the same subsequence in either order — the
same design fact the projection bridge cashes, here at both sides of
one channel at once. `depOK_transfer_chan` is `depOK_transfer`'s
positional argument with the class chosen per position (the event's
own channel) instead of fixed up front.

Duplicate-freedom of the O future comes from the O alignment exactly
as the E future's came from the E alignment: the per-owner filters are
the `procsO` traces, and every `procsO` trace is canon-shaped per
channel-side (`procsO_canon`), unconditionally over the class.

Chain (ord, stage D): the witness and its alignment; consumed by the
O master induction. Base mirror: PrecE.lean (a transfer, its class
refined from wire/asked to per-channel). Map: PROGRESS.md §13.
-/
import StreamingMirror.Ord.Align
import StreamingMirror.Proofs.Sched.Weave.PrecE

namespace StreamingMirror.Ord

open Model
open Sched

variable (sk : Skel) (ord : OrdMap)

-- ============================================ the per-channel class

/-- The single-channel class: both sides of one channel — the class
every `manDep` pair lives in (`manDep_chan`). -/
def onChan (c : Chan) : Ev → Bool := fun x => x.1 == c

/-- Every dep pair lives on ONE channel: `manDep` chains receives to
their own-seq sends and cap-1 sends to their own channel's previous
receive — never across channels, in particular never between the two
prologue receives. -/
theorem manDep_chan (e d : Ev) (hd : manDep e = some d) : d.1 = e.1 := by
  obtain ⟨c, b, n⟩ := e
  cases c <;> cases b <;> simp only [manDep] at hd
  all_goals first
    | (injection hd with h; subst h; rfl)
    | (split at hd
       · cases hd
       · injection hd with h; subst h; rfl)
    | cases hd

-- ================================================ Nodup from canon

/-- The O future is duplicate-free: the O alignment partitions it into
the `procsO` traces by owner, and every `procsO` trace is canon-shaped
— every assignment of the class. -/
theorem weaveO_future_nodup (hwf : sk.wellFormed = true) :
    ((weaveOps sk).flatMap (opEventsO sk ord)).Nodup := by
  obtain ⟨hown, halign⟩ := weaveO_initial_alignment sk ord hwf
  refine nodup_of_class_filters
    (fun a e => evOwner sk e == evOwner sk a)
    (fun a => beq_self_eq_true _) _ (fun a ha => ?_)
  have h1 : ((weaveOps sk).flatMap (opEventsO sk ord)).filter
      (fun e => evOwner sk e == evOwner sk a)
      ∈ manFilters sk ((weaveOps sk).flatMap (opEventsO sk ord)) := by
    unfold manFilters
    exact List.mem_map.mpr
      ⟨evOwner sk a, List.mem_range.mpr (hown a ha), rfl⟩
  rw [halign] at h1
  exact trace_nodup fun c b =>
    procsO_canon sk ord c b _ (List.mem_of_mem_take h1)

-- ============================================ the positional transfer

/-- THE TRANSFER, per-channel class: dep-closure survives any reorder
that fixes every single channel's subsequence, provided the target is
duplicate-free — `depOK_transfer`'s positional argument with the class
instantiated at each event's own channel (`manDep_chan` puts the dep
in the same class). -/
theorem depOK_transfer_chan {done l l' : List Ev}
    (hfil : ∀ c : Chan, l'.filter (onChan c) = l.filter (onChan c))
    (hnd : l'.Nodup) (h : DepOK done l) : DepOK done l' := by
  intro i e d hi hd
  have hPe : onChan e.1 e = true := by simp [onChan]
  have hPd : onChan e.1 d = true := by
    simp [onChan, manDep_chan e d hd]
  obtain ⟨hi', hei⟩ := List.getElem?_eq_some_iff.mp hi
  have hl' : l' = l'.take i ++ e :: l'.drop (i + 1) := by
    conv => lhs; rw [← List.take_append_drop i l']
    rw [List.drop_eq_getElem_cons hi', hei]
  have hel : e ∈ l := by
    have hm : e ∈ l.filter (onChan e.1) := by
      rw [← hfil e.1]
      exact List.mem_filter.mpr ⟨List.mem_of_getElem? hi, hPe⟩
    exact (List.mem_filter.mp hm).1
  obtain ⟨j, hjlt, hje⟩ := List.getElem_of_mem hel
  have hj? : l[j]? = some e := by
    rw [List.getElem?_eq_getElem hjlt, hje]
  rcases List.mem_append.1 (h j e d hj? hd) with hind | hintk
  · exact List.mem_append_left _ hind
  · have hl : l = l.take j ++ e :: l.drop (j + 1) := by
      conv => lhs; rw [← List.take_append_drop j l]
      rw [List.drop_eq_getElem_cons hjlt, hje]
    have hfl : l.filter (onChan e.1)
        = (l.take j).filter (onChan e.1)
          ++ e :: (l.drop (j + 1)).filter (onChan e.1) := by
      conv => lhs; rw [hl]
      rw [List.filter_append, List.filter_cons_of_pos hPe]
    have hfl' : l'.filter (onChan e.1)
        = (l'.take i).filter (onChan e.1)
          ++ e :: (l'.drop (i + 1)).filter (onChan e.1) := by
      conv => lhs; rw [hl']
      rw [List.filter_append, List.filter_cons_of_pos hPe]
    have hndf : (l'.filter (onChan e.1)).Nodup :=
      (List.filter_sublist ..).nodup hnd
    have hXX : (l'.take i).filter (onChan e.1)
        = (l.take j).filter (onChan e.1) :=
      nodup_append_cons_left_inj (by rw [← hfl']; exact hndf)
        (by rw [← hfl', hfil e.1, hfl])
    have hd' : d ∈ (l'.take i).filter (onChan e.1) := by
      rw [hXX]
      exact List.mem_filter.mpr ⟨hintk, hPd⟩
    exact List.mem_append_right _ (List.mem_filter.mp hd').1

-- ==================================== the two futures' shared class

/-- The two prologue channels disagree at the constructor (private
twin of Ord/Numbering's `askedIn_ne_wireIn`). -/
private theorem askedIn_ne_wireIn (pk : Party × Nat) :
    askedIn pk ≠ wireIn pk := by simp [askedIn, wireIn]

/-- The ordered prologue's single-channel filters are the E
prologue's: the two receives live on distinct channels, so at most one
passes any one channel's filter. -/
private theorem prologueO_filter_chan (pk : Party × Nat) (k : Nat)
    (c : Chan) :
    (prologueO ord pk k).filter (onChan c)
      = ([(wireIn pk, false, k), (askedIn pk, false, k)] : List Ev).filter
          (onChan c) := by
  unfold prologueO
  cases ord.walk pk with
  | replyFirst => rfl
  | queryFirst =>
      by_cases hw : wireIn pk = c
      · have ha : askedIn pk ≠ c := by
          rw [← hw]
          exact askedIn_ne_wireIn pk
        simp [onChan, hw, ha]
      · by_cases ha : askedIn pk = c
        · simp [onChan, hw, ha]
        · simp [onChan, hw, ha]

/-- O and E scope expansions agree on every single-channel filter:
only the prologue receives move between the assignments, and they move
within a two-element block whose channels are distinct. -/
theorem opEventsO_filter_chan (hwf : sk.wellFormed = true) (c : Chan) :
    ∀ (h k : Nat) (feed : List Ev), h < sk.rootH → k < sk.stageLen h →
      (opEventsO sk ord (.scope h k feed)).filter (onChan c)
        = (opEventsE sk (.scope h k feed)).filter (onChan c) := by
  intro h
  induction h with
  | zero =>
      intro k feed hh hk
      have hD0 : ∀ i, sk.childIsD 0 (sk.stageScope 0 k) i = false :=
        fun _ => rfl
      have hEO := opEventsO_scope_eq sk ord (Nat.le_of_lt hk) feed
      have hEE := opEventsE_scope_eq sk (Nat.le_of_lt hk) feed
      have hkidEq : ∀ i ∈ List.range (sk.nChildren 0 (sk.stageScope 0 k)),
          opEventsO sk ord (.kid 0 k (sk.stageScope 0 k) none
              (sk.wiresBefore 0 k) i feed)
            = opEventsE sk (.kid 0 k (sk.stageScope 0 k) none
              (sk.wiresBefore 0 k) i feed) := by
        intro i _
        have hO : opEventsO sk ord (.kid 0 k (sk.stageScope 0 k) none
              (sk.wiresBefore 0 k) i feed)
            = (wireOut (wpk 0), true, sk.wiresBefore 0 k + i)
                :: feed[i]?.toList := by
          rw [opEventsO_kid_eq,
            if_neg (by rw [hD0 i]; exact Bool.false_ne_true),
            if_pos (show ((0 : Nat) == 0) = true by rfl), List.append_nil]
        have hE : opEventsE sk (.kid 0 k (sk.stageScope 0 k) none
              (sk.wiresBefore 0 k) i feed)
            = (wireOut (wpk 0), true, sk.wiresBefore 0 k + i)
                :: feed[i]?.toList := by
          rw [opEventsE_kid_eq,
            if_neg (by rw [hD0 i]; exact Bool.false_ne_true),
            if_pos (show ((0 : Nat) == 0) = true by rfl), List.append_nil]
        rw [hO, hE]
      rw [hEO, hEE, flatMap_congr hkidEq, List.filter_append,
        prologueO_filter_chan, ← List.filter_append]
      simp only [List.cons_append, List.nil_append]
  | succ h ih =>
      intro k feed hh hk
      have hh' : h < sk.rootH := by omega
      have h1 : (1 : Nat) ≤ h + 1 := by omega
      have hsub : ∀ i, i < sk.nChildren (h + 1) (sk.stageScope (h + 1) k) →
          sk.wiresBefore (h + 1) k + i < sk.stageLen h := by
        intro i hi
        have htot := wiresBefore_total sk hwf h1 hh
        simp only [Nat.add_sub_cancel] at htot
        have hmono := wiresBefore_mono sk (h + 1)
          (show k + 1 ≤ sk.stageLen (h + 1) from hk)
        have hstep := wiresBefore_succ sk hk
        omega
      have hEO := opEventsO_scope_eq sk ord (Nat.le_of_lt hk) feed
      have hEE := opEventsE_scope_eq sk (Nat.le_of_lt hk) feed
      have hkid : ∀ i ∈ List.range
            (sk.nChildren (h + 1) (sk.stageScope (h + 1) k)),
          (opEventsO sk ord (.kid (h + 1) k (sk.stageScope (h + 1) k) none
              (sk.wiresBefore (h + 1) k) i feed)).filter (onChan c)
            = (opEventsE sk (.kid (h + 1) k (sk.stageScope (h + 1) k) none
                (sk.wiresBefore (h + 1) k) i feed)).filter (onChan c) := by
        intro i hi
        rw [List.mem_range] at hi
        by_cases hD : sk.childIsD (h + 1) (sk.stageScope (h + 1) k) i
        · have hIH := ih (sk.wiresBefore (h + 1) k + i)
            (chunkQ sk (h + 1) k i) hh' (hsub i hi)
          have hO : opEventsO sk ord (.kid (h + 1) k
                (sk.stageScope (h + 1) k) none
                (sk.wiresBefore (h + 1) k) i feed)
              = (wireOut (wpk (h + 1)), true, sk.wiresBefore (h + 1) k + i)
                  :: ((lowerOut (wpk (h + 1)), true,
                      sk.dsBefore (h + 1) k + dRank sk (wpk (h + 1)) k i)
                    :: (feed[i]?.toList
                      ++ opEventsO sk ord (.scope h
                          (sk.wiresBefore (h + 1) k + i)
                          (chunkQ sk (h + 1) k i)))) := by
            rw [opEventsO_kid_eq, if_pos hD]
            simp only [Nat.add_sub_cancel]
          have hE : opEventsE sk (.kid (h + 1) k
                (sk.stageScope (h + 1) k) none
                (sk.wiresBefore (h + 1) k) i feed)
              = (wireOut (wpk (h + 1)), true, sk.wiresBefore (h + 1) k + i)
                  :: ((lowerOut (wpk (h + 1)), true,
                      sk.dsBefore (h + 1) k + dRank sk (wpk (h + 1)) k i)
                    :: (feed[i]?.toList
                      ++ opEventsE sk (.scope h
                          (sk.wiresBefore (h + 1) k + i)
                          (chunkQ sk (h + 1) k i)))) := by
            rw [opEventsE_kid_eq, if_pos hD]
            simp only [Nat.add_sub_cancel]
          rw [hO, hE]
          simp only [List.filter_cons, List.filter_append, hIH]
        · have hIH := ih (sk.wiresBefore (h + 1) k + i) [] hh' (hsub i hi)
          have hO : opEventsO sk ord (.kid (h + 1) k
                (sk.stageScope (h + 1) k) none
                (sk.wiresBefore (h + 1) k) i feed)
              = (wireOut (wpk (h + 1)), true, sk.wiresBefore (h + 1) k + i)
                  :: (feed[i]?.toList
                    ++ opEventsO sk ord (.scope h
                        (sk.wiresBefore (h + 1) k + i) [])) := by
            rw [opEventsO_kid_eq, if_neg hD]
            simp only [Nat.add_sub_cancel,
              show ((h + 1 : Nat) == 0) = false from rfl,
              Bool.false_eq_true, if_false]
          have hE : opEventsE sk (.kid (h + 1) k
                (sk.stageScope (h + 1) k) none
                (sk.wiresBefore (h + 1) k) i feed)
              = (wireOut (wpk (h + 1)), true, sk.wiresBefore (h + 1) k + i)
                  :: (feed[i]?.toList
                    ++ opEventsE sk (.scope h
                        (sk.wiresBefore (h + 1) k + i) [])) := by
            rw [opEventsE_kid_eq, if_neg hD]
            simp only [Nat.add_sub_cancel,
              show ((h + 1 : Nat) == 0) = false from rfl,
              Bool.false_eq_true, if_false]
          rw [hO, hE]
          simp only [List.filter_cons, List.filter_append, hIH]
      have hFM : ((List.range
            (sk.nChildren (h + 1) (sk.stageScope (h + 1) k))).flatMap
            (fun i => opEventsO sk ord (.kid (h + 1) k
              (sk.stageScope (h + 1) k) none
              (sk.wiresBefore (h + 1) k) i feed))).filter (onChan c)
          = ((List.range
              (sk.nChildren (h + 1) (sk.stageScope (h + 1) k))).flatMap
              (fun i => opEventsE sk (.kid (h + 1) k
                (sk.stageScope (h + 1) k) none
                (sk.wiresBefore (h + 1) k) i feed))).filter (onChan c) := by
        simp only [List.filter_flatMap]
        exact flatMap_congr hkid
      rw [hEO, hEE, List.filter_append, prologueO_filter_chan,
        ← List.filter_append]
      simp only [List.cons_append, List.nil_append, List.filter_cons,
        List.filter_append, hFM]

/-- The two opening futures agree on every single channel. -/
theorem weave_filterO_chan (hwf : sk.wellFormed = true) (c : Chan) :
    ((weaveOps sk).flatMap (opEventsO sk ord)).filter (onChan c)
      = ((weaveOps sk).flatMap (opEventsE sk)).filter (onChan c) := by
  have hge := (wf_rootH hwf).2
  have hlen1 := wf_stageLen_top sk hwf
  rw [weave_flatMapO, weave_flatMapE, List.filter_append,
    List.filter_append, List.filter_append, List.filter_append,
    opEventsO_filter_chan sk ord hwf c (sk.rootH - 1) 0 _ (by omega)
      (by omega)]

-- ==================================================== the payoff

/-- Dep-closure of the O opening future: transferred from the E future
along the shared per-channel subsequences — every assignment of the
class. -/
theorem weaveO_flatMap_depOK (hwf : sk.wellFormed = true) :
    DepOK [] ((weaveOps sk).flatMap (opEventsO sk ord)) :=
  depOK_transfer_chan (weave_filterO_chan sk ord hwf)
    (weaveO_future_nodup sk ord hwf) (weaveE_flatMap_depOK sk hwf)

/-- `weaveE_goEvents_depOK`'s O twin: dep-closure of the O weave's
ghost future — the O consumption frame's precedence input. -/
theorem weaveO_goEvents_depOK (hwf : sk.wellFormed = true) :
    DepOK [] (goEventsO sk ord (weaveFuel sk) (weaveOps sk)) := by
  rw [goEventsO_weave sk ord (weave_events_lengthO sk ord hwf)]
  exact weaveO_flatMap_depOK sk ord hwf

end StreamingMirror.Ord
