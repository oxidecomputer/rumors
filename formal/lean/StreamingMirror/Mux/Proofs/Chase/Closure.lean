/-
The demand closures (MUX-ADJUDICATION.md §3 T2; refute-c1 §1.3 as
repaired by attack-refute F1/F6): what a machine can prove about the
session's events from its own observation history.

# The two tiers

*Certified* evidence is push arithmetic: a machine's own flush receipts
ground its own wire sends, and its demux's delivery receipts ground the
peer's (FIFO makes arrival order push order, so per-stream counts are
exact). *Inevitable* is the forward closure: a non-push event fires in
every continuation once its whole dependency past does, so an event
whose E1/E2 predecessors and full trace prefix are derivable is
derivable. Pushes are strategy-gated and are NEVER derived — they enter
only as grounded evidence. That exclusion is the self-containment
property the cross-examination singled out (attack-refute §4.5): no
demand proof ever cites traffic not already committed to the transport.

# Deviations from refute-c1 §1.3, recorded

- The delivery events of refute-c1's vocabulary do not exist here: the
  closure speaks `Sched.Ev` only, and the F1 repair's forward-delivery
  case is discharged where it belongs — in the keystone, by the
  FIFO-ancestry hypothesis (`MuxInv.pushtime_delivered`), not by
  closure members. This is the "ban forward-del citations from I-step"
  reading of the repair, which attack-refute F1 offers as the
  alternative restatement.
- The I-step guard is positional (attack-refute F6): a send's E2
  predecessor must be a MEMBER; no occupancy is ever computed, so
  monotonicity in the observation is by construction.
- The I-step's E3 guard requires the event's whole trace prefix, not
  the immediate predecessor: the most restrictive reading, and the one
  that makes the closure literally downward-closed (used by the
  keystone's argmin).
- I-step is party-uniform where refute-c1 §1.3 restricts to the peer's
  side: deriving one's own future non-push events is equally sound
  (they are scheduler-forced at stuck states, which is all soundness
  says) and strictly more useful to σ*.
- `certified` keeps only C-own/C-arr, the push-count grounding; the
  C-prog back-closure is dropped. It is subsumed: for every non-push
  event the forward closure re-derives it, and for peer pushes the
  cross-stream program-order evidence matters only to eagerness, never
  to the stuck-state coverage argument, where pipes-empty makes
  arrival grounding complete (MUX-ADJUDICATION §1.1's Step 4). If the
  stage-3 σ* wants the extra eagerness, mint the back-closure there.
-/
import StreamingMirror.Mux.Proofs.Chase.Ground
import StreamingMirror.Proofs.Sched.Numbering

namespace StreamingMirror.Mux

open Model
open Sched (Ev procsE proj canon count_canon)

/-- Every event of the session, as the flattened trace family. -/
def evUniv (sk : Skel) : List Ev := (procsE sk).flatten

/-- Universe membership is trace membership. -/
theorem mem_evUniv {sk : Skel} {e : Ev} :
    e ∈ evUniv sk ↔ ∃ T ∈ procsE sk, e ∈ T := by
  rw [evUniv, List.mem_flatten]

/-- A trace never repeats an event: its per-channel projections are
canonical, so equal events would need equal seqs twice. -/
theorem trace_count_le_one {sk : Skel} {T : List Ev}
    (hT : T ∈ procsE sk) (e : Ev) : T.count e ≤ 1 := by
  obtain ⟨c, b, n⟩ := e
  obtain ⟨m, hm⟩ := Sched.procsE_canon sk c b T hT
  have hfilter : T.count (c, b, n) = (proj c b T).count (c, b, n) := by
    unfold Sched.proj
    exact (List.count_filter (by simp)).symm
  rw [hfilter, hm, count_canon]
  split <;> omega

-- ========================================================== grounding

/-- Push evidence: the wire sends this machine's own history commits
to — its flush receipts on its own streams, its delivery receipts on
the peer's (C-own and C-arr of refute-c1 §1.3, as per-stream counts). -/
def groundedPush (p : Party) (tr : List MObs) (e : Ev) : Bool :=
  isWire e.1 && e.2.1 &&
    (if wireParty e.1 == p then
      decide (e.2.2 < pushedCount tr (wireHeight e.1))
    else decide (e.2.2 < deliveredCount tr (wireHeight e.1)))

/-- Grounded evidence names a wire send below an observed count. -/
theorem groundedPush_inv {p : Party} {tr : List MObs} {e : Ev}
    (hg : groundedPush p tr e = true) :
    ∃ q h n, e = (Chan.wire q h, true, n)
      ∧ ((q = p ∧ n < pushedCount tr h)
        ∨ (q = p.other ∧ n < deliveredCount tr h)) := by
  obtain ⟨c, b, n⟩ := e
  rw [groundedPush, Bool.and_eq_true, Bool.and_eq_true] at hg
  obtain ⟨⟨hw, hb⟩, hcond⟩ := hg
  obtain ⟨q, h, rfl⟩ := isWire_eq hw
  have hb' : b = true := hb
  subst hb'
  refine ⟨q, h, n, rfl, ?_⟩
  simp only [wireParty, wireHeight] at hcond
  by_cases hq : q = p
  · rw [if_pos (by simp [hq])] at hcond
    exact Or.inl ⟨hq, by simpa using hcond⟩
  · rw [if_neg (by simp [hq])] at hcond
    refine Or.inr ⟨?_, by simpa using hcond⟩
    cases p <;> cases q <;> first | rfl | exact absurd rfl hq

/-- Evidence only accumulates along an observation history. -/
theorem groundedPush_mono {p : Party} {tr tr' : List MObs}
    (hp : tr <+: tr') {e : Ev} (hg : groundedPush p tr e = true) :
    groundedPush p tr' e = true := by
  obtain ⟨q, h, n, rfl, hcase⟩ := groundedPush_inv hg
  have hpu := pushedCount_le_of_prefix hp h
  have hde := deliveredCount_le_of_prefix hp h
  rw [groundedPush]
  simp only [isWire, wireParty, wireHeight, Bool.true_and]
  rcases hcase with ⟨rfl, hlt⟩ | ⟨rfl, hlt⟩
  · have hpp := pushedCount_le_of_prefix hp h
    rw [if_pos (by simp)]
    exact decide_eq_true (by omega)
  · have hne : (p.other == p) = false := by cases p <;> rfl
    rw [if_neg (by rw [hne]; exact Bool.false_ne_true)]
    exact decide_eq_true (by omega)

-- ======================================================== the I-step

/-- One forward-derivation check against a candidate set `D`: `e` is a
non-push event whose E1 send, positional E2 predecessor, and whole
trace prefix are already in `D` (attack-refute F6's membership form).

`e`'s trace prefix is read off `takeWhile`: traces never repeat an
event (`trace_count_le_one`), so the segment before the first
occurrence IS the E3 past. -/
def istepOk (sk : Skel) (D : List Ev) (e : Ev) : Bool :=
  !(isWire e.1 && e.2.1) &&
  (e.2.1 || D.contains (e.1, true, e.2.2)) &&
  (!e.2.1 || decide (e.2.2 < sk.cap e.1)
    || D.contains (e.1, false, e.2.2 - sk.cap e.1)) &&
  ((procsE sk).all fun T =>
    !(T.contains e)
      || (T.takeWhile (fun x => !(x == e))).all (D.contains ·))

/-- An I-step member is never a push. -/
theorem istepOk_not_push {sk : Skel} {D : List Ev} {e : Ev}
    (h : istepOk sk D e = true) : (isWire e.1 && e.2.1) = false := by
  rw [istepOk] at h
  simp only [Bool.and_eq_true] at h
  have := h.1.1.1
  rwa [Bool.not_eq_true'] at this

/-- An I-step receive's send is a member. -/
theorem istepOk_e1 {sk : Skel} {D : List Ev} {e : Ev}
    (h : istepOk sk D e = true) (hb : e.2.1 = false) :
    (e.1, true, e.2.2) ∈ D := by
  rw [istepOk] at h
  simp only [Bool.and_eq_true] at h
  have := h.1.1.2
  rw [hb] at this
  simp only [Bool.false_or] at this
  exact (List.contains_iff_mem ..).mp this

/-- An I-step send's cap-window predecessor is a member, past the free
window. -/
theorem istepOk_e2 {sk : Skel} {D : List Ev} {e : Ev}
    (h : istepOk sk D e = true) (hb : e.2.1 = true)
    (hcap : ¬ e.2.2 < sk.cap e.1) :
    (e.1, false, e.2.2 - sk.cap e.1) ∈ D := by
  rw [istepOk] at h
  simp only [Bool.and_eq_true] at h
  have := h.1.2
  rw [hb] at this
  simp only [Bool.not_true, Bool.false_or, Bool.or_eq_true,
    decide_eq_true_eq] at this
  rcases this with hlt | hmem
  · exact absurd hlt hcap
  · exact (List.contains_iff_mem ..).mp hmem

/-- An I-step member's whole trace past is a member set. -/
theorem istepOk_prefix {sk : Skel} {D : List Ev} {e : Ev}
    (h : istepOk sk D e = true) {T : List Ev} (hT : T ∈ procsE sk)
    (heT : e ∈ T) :
    ∀ x ∈ T.takeWhile (fun x => !(x == e)), x ∈ D := by
  rw [istepOk] at h
  simp only [Bool.and_eq_true] at h
  have := List.all_eq_true.mp h.2 T hT
  rw [Bool.or_eq_true] at this
  rcases this with hnc | hall
  · rw [Bool.not_eq_true', ← Bool.not_eq_true] at hnc
    exact absurd ((List.contains_iff_mem ..).mpr heT) hnc
  · intro x hx
    exact (List.contains_iff_mem ..).mp (List.all_eq_true.mp hall x hx)

/-- The I-step check is monotone in the candidate set. -/
theorem istepOk_mono {sk : Skel} {D D' : List Ev}
    (hsub : ∀ x ∈ D, x ∈ D') {e : Ev} (h : istepOk sk D e = true) :
    istepOk sk D' e = true := by
  rw [istepOk] at h ⊢
  simp only [Bool.and_eq_true] at h ⊢
  obtain ⟨⟨⟨hnp, he1⟩, he2⟩, he3⟩ := h
  have hc : ∀ x : Ev, D.contains x = true → D'.contains x = true := by
    intro x hx
    exact (List.contains_iff_mem ..).mpr
      (hsub x ((List.contains_iff_mem ..).mp hx))
  refine ⟨⟨⟨hnp, ?_⟩, ?_⟩, ?_⟩
  · rw [Bool.or_eq_true] at he1 ⊢
    rcases he1 with hb | hm
    · exact Or.inl hb
    · exact Or.inr (hc _ hm)
  · rw [Bool.or_eq_true, Bool.or_eq_true] at he2 ⊢
    rcases he2 with (hb | hlt) | hm
    · exact Or.inl (Or.inl hb)
    · exact Or.inl (Or.inr hlt)
    · exact Or.inr (hc _ hm)
  · rw [List.all_eq_true] at he3 ⊢
    intro T hT
    have := he3 T hT
    rw [Bool.or_eq_true] at this ⊢
    rcases this with hnc | hall
    · exact Or.inl hnc
    · refine Or.inr ?_
      rw [List.all_eq_true] at hall ⊢
      intro x hx
      exact hc _ (hall x hx)

-- ======================================================= the closures

/-- One saturation pass: keep the members, adopt fresh evidence, admit
every event whose I-step check passes against the current set. -/
def closureStep (sk : Skel) (p : Party) (tr : List MObs)
    (D : List Ev) : List Ev :=
  (evUniv sk).filter fun e =>
    D.contains e || groundedPush p tr e || istepOk sk D e

/-- The saturation chain, from the grounded evidence. -/
def closureN (sk : Skel) (p : Party) (tr : List MObs) : Nat → List Ev
  | 0 => (evUniv sk).filter (groundedPush p tr)
  | n + 1 => closureStep sk p tr (closureN sk p tr n)

/-- The certified events: the push evidence itself (C-own/C-arr of
refute-c1 §1.3; the C-prog back-closure is deliberately dropped — see
the module doc). -/
def certified (sk : Skel) (p : Party) (tr : List MObs) : List Ev :=
  (evUniv sk).filter (groundedPush p tr)

/-- The inevitable events: the forward closure of the evidence, run to
the universe's depth (each productive pass adds an event, so
universe-many passes saturate). -/
def inevitable (sk : Skel) (p : Party) (tr : List MObs) : List Ev :=
  closureN sk p tr (evUniv sk).length

/-- Every closure stage stays inside the universe. -/
theorem closureN_subset_univ {sk : Skel} {p : Party} {tr : List MObs} :
    ∀ n, ∀ e ∈ closureN sk p tr n, e ∈ evUniv sk := by
  intro n e he
  cases n with
  | zero => exact (List.mem_filter.mp he).1
  | succ n => exact (List.mem_filter.mp he).1

/-- Each saturation pass keeps its members. -/
theorem closureN_le_succ {sk : Skel} {p : Party} {tr : List MObs}
    (n : Nat) : ∀ e ∈ closureN sk p tr n, e ∈ closureN sk p tr (n + 1) := by
  intro e he
  show e ∈ closureStep sk p tr (closureN sk p tr n)
  refine List.mem_filter.mpr ⟨closureN_subset_univ n e he, ?_⟩
  rw [Bool.or_eq_true, Bool.or_eq_true]
  exact Or.inl (Or.inl ((List.contains_iff_mem ..).mpr he))

/-- The saturation chain is increasing. -/
theorem closureN_le {sk : Skel} {p : Party} {tr : List MObs}
    {m n : Nat} (hmn : m ≤ n) :
    ∀ e ∈ closureN sk p tr m, e ∈ closureN sk p tr n := by
  induction n with
  | zero =>
      intro e he
      have : m = 0 := by omega
      exact this ▸ he
  | succ n ih =>
      intro e he
      by_cases hlast : m = n + 1
      · exact hlast ▸ he
      · exact closureN_le_succ n e (ih (by omega) e he)

/-- Evidence is inevitable. -/
theorem certified_subset_inevitable {sk : Skel} {p : Party}
    {tr : List MObs} :
    ∀ e ∈ certified sk p tr, e ∈ inevitable sk p tr :=
  closureN_le (Nat.zero_le _)

/-- Inevitable events live in the universe (hence in some trace). -/
theorem inevitable_subset_univ {sk : Skel} {p : Party} {tr : List MObs} :
    ∀ e ∈ inevitable sk p tr, e ∈ evUniv sk :=
  closureN_subset_univ _

/-- The closure inversion: an inevitable event is grounded evidence or
passes the I-step check against the closure itself.

This is the keystone's induction handle: monotonicity lifts the stage
at which an event entered to the saturated set, so no stage bookkeeping
survives into consumers. -/
theorem inevitable_inv {sk : Skel} {p : Party} {tr : List MObs} {e : Ev}
    (he : e ∈ inevitable sk p tr) :
    groundedPush p tr e = true
      ∨ istepOk sk (inevitable sk p tr) e = true := by
  suffices h : ∀ n, ∀ e ∈ closureN sk p tr n,
      groundedPush p tr e = true
        ∨ istepOk sk (closureN sk p tr n) e = true by
    rcases h _ e he with hg | hstep
    · exact Or.inl hg
    · exact Or.inr hstep
  intro n
  induction n with
  | zero =>
      intro e he
      exact Or.inl (List.mem_filter.mp he).2
  | succ n ih =>
      intro e he
      obtain ⟨-, hcond⟩ := List.mem_filter.mp he
      rw [Bool.or_eq_true, Bool.or_eq_true] at hcond
      rcases hcond with (hmem | hg) | hstep
      · rcases ih e ((List.contains_iff_mem ..).mp hmem) with hg | hstep
        · exact Or.inl hg
        · exact Or.inr (istepOk_mono (closureN_le_succ n) hstep)
      · exact Or.inl hg
      · exact Or.inr (istepOk_mono (closureN_le_succ n) hstep)

/-- The closures are monotone in the observation history: proofs never
retract (refute-c1 §1.1). -/
theorem inevitable_mono {sk : Skel} {p : Party} {tr tr' : List MObs}
    (hp : tr <+: tr') :
    ∀ e ∈ inevitable sk p tr, e ∈ inevitable sk p tr' := by
  suffices h : ∀ n, ∀ e ∈ closureN sk p tr n, e ∈ closureN sk p tr' n by
    exact h _
  intro n
  induction n with
  | zero =>
      intro e he
      obtain ⟨hu, hg⟩ := List.mem_filter.mp he
      exact List.mem_filter.mpr ⟨hu, groundedPush_mono hp hg⟩
  | succ n ih =>
      intro e he
      obtain ⟨hu, hcond⟩ := List.mem_filter.mp he
      refine List.mem_filter.mpr ⟨hu, ?_⟩
      rw [Bool.or_eq_true, Bool.or_eq_true] at hcond ⊢
      rcases hcond with (hmem | hg) | hstep
      · exact Or.inl (Or.inl ((List.contains_iff_mem ..).mpr
          (ih e ((List.contains_iff_mem ..).mp hmem))))
      · exact Or.inl (Or.inr (groundedPush_mono hp hg))
      · exact Or.inr (istepOk_mono ih hstep)

end StreamingMirror.Mux
