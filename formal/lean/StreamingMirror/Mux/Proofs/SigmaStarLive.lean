/-
T4 — σ* is deadlock-free (MUX-ADJUDICATION.md §3 T4; the campaign's
centerpiece): the demand-lockstep strategy pair is deadlock-free on
every well-formed margin-0 session over the single-pipe transport, at
every capacity C ≥ 1 ("completes" in the grounded sense: stuck-freedom
here, termination via `mux_terminating`, Proofs/Termination.lean).
This is the liveness half of C1's refutation — the charter-grain
refutation of record is `c1_charter_false` via σ*-causal
(Proofs/C1.lean) — with no fixed capacity, no control messages, only
the right to idle.

# The four steps (refute-c1 §2, as formalized)

1. **Pipes drain at stuck states.** A buried pipe head would need its
   per-stream predecessor unconsumed in the slot; σ*'s push certificate
   (`PushProven`, INV-A) says that consumption was derivable at push
   time, closure monotonicity carries the derivation to the stuck
   state's history prefix, and the keystone turns derivable into
   performed — contradiction (`sigmaStar_pipes_empty`).
2. **Stuckness exhibits a withheld push** — the τ-least unperformed
   event, with every τ-earlier event performed (`chase`, stage 2).
3. **Its predecessor is consumed** — the E2 edge puts `rcv(c, k−1)`
   τ-below the withheld send, so the chase's cover marks it performed.
4. **Coverage** — the genuinely new induction (`closure_coverage`):
   with pipes drained, EVERY event τ-below the withheld push is in the
   sender's demand closure — performed wire sends are push-grounded
   (own flushes or delivered arrivals; pipes-empty makes arrival
   grounding complete), and every non-push event enters by I-step
   because its whole dependency past sits strictly τ-below it. So the
   predecessor's consumption is derivable, the frame is
   proven-demanded, σ* names a push, and `mstuck` is refuted.

The stage-0 probe validated the decidable Step-4 invariant on
4,970/4,970 causal σ*×σ* runs (STAGE0-GATES.md P1); this file is its
kernel transcription, with the closure stage-indexed by τ so no
saturation lemma is ever needed.
-/
import StreamingMirror.Mux.Proofs.SigmaStarInv

namespace StreamingMirror.Mux

open Model
open Sched (Ev procsE scheduleE performed pends PendOkE evIdx)

variable {sk : Skel}

-- ================================================== list positioning

/-- Locate the `n`-th hit of a `filterMap` inside its source: the
source index, the witness element, and the prefix correspondence that
makes counts at the cut agree. -/
private theorem filterMap_take_index {α β : Type _} (f : α → Option β) :
    ∀ (l : List α) (n : Nat) (b : β),
      (l.filterMap f)[n]? = some b →
      ∃ i a, l[i]? = some a ∧ f a = some b
        ∧ (l.take i).filterMap f = (l.filterMap f).take n := by
  intro l
  induction l with
  | nil =>
      intro n b hget
      simp at hget
  | cons x t ih =>
      intro n b hget
      cases hfx : f x with
      | none =>
          have hfm : (x :: t).filterMap f = t.filterMap f := by
            simp [hfx]
          rw [hfm] at hget
          obtain ⟨i, a, hia, hfa, htake⟩ := ih n b hget
          refine ⟨i + 1, a, by simpa using hia, hfa, ?_⟩
          rw [List.take_succ_cons, List.filterMap_cons, hfx, htake, hfm]
      | some c =>
          have hfm : (x :: t).filterMap f = c :: t.filterMap f := by
            simp [hfx]
          rw [hfm] at hget
          cases n with
          | zero =>
              simp only [List.getElem?_cons_zero, Option.some.injEq]
                at hget
              subst hget
              exact ⟨0, x, rfl, hfx, by simp⟩
          | succ m =>
              simp only [List.getElem?_cons_succ] at hget
              obtain ⟨i, a, hia, hfa, htake⟩ := ih m b hget
              refine ⟨i + 1, a, by simpa using hia, hfa, ?_⟩
              rw [List.take_succ_cons, List.filterMap_cons, hfx, htake,
                hfm, List.take_succ_cons]

/-- Either the same party or the other: the two-point case split. -/
private theorem party_cases' (q p : Party) : q = p ∨ q = p.other := by
  cases q <;> cases p <;> simp [Party.other]

/-- The push-tag extractor only hits `.pushed` observations. -/
private theorem pushed_of_extract {a : MObs} {g : Nat}
    (h : (match a with
          | MObs.pushed h => some h
          | _ => none) = some g) : a = .pushed g := by
  cases a with
  | pushed h' =>
      injection h with h
      rw [h]
  | act a' => cases h
  | delivered h' => cases h

-- ========================================== Step 1: the pipes drain

/-- At a σ*-stuck state both pipes are empty (refute-c1 §2.1): a pipe
head's per-stream predecessor is unconsumed in the slot, yet σ*'s push
certificate derived that consumption at push time, monotonicity keeps
it derived, and the keystone performs it. -/
theorem sigmaStar_pipes_empty (hwf : sk.wellFormed = true)
    (hm0 : ∀ sc, sk.dCount sc ≤ sk.capLevel) {C : Nat} {s : MState}
    (hm : SInv sk s) (hpp : PushProven sk s)
    (hstuck : mstuck sk .impl C sigmaStar sigmaStar s = true)
    (p : Party) : s.pipe p = [] := by
  cases hp : s.pipe p with
  | nil => rfl
  | cons c rest =>
      exfalso
      -- the head is a wire frame with an occupied slot
      obtain ⟨g, rfl⟩ := hm.mux.pipe_mem_wire (p := p) (c := c)
        (by rw [hp]; exact List.mem_cons_self ..)
      have hz : s.base.chan (Chan.wire p g) ≠ 0 :=
        mstuck_deliver_blocked hstuck hp
      -- the head sits at position `delTotal` of the flush order
      have hpipe := hm.mux.hist_pipe p
      rw [hp] at hpipe
      have hget : (pushHeights (s.hist p))[delTotal (s.hist p.other)]?
          = some g := by
        have h1 : ((pushHeights (s.hist p)).drop
            (delTotal (s.hist p.other)))[0]? = some g := by
          have h2 := congrArg (List.map wireHeight) hpipe
          rw [List.map_map,
            show wireHeight ∘ Chan.wire p = id from rfl,
            List.map_id] at h2
          rw [← h2]
          rfl
        rw [List.getElem?_drop] at h1
        simpa using h1
      -- the head's channel is real
      have hmem_ch : Chan.wire p g ∈ allChans sk := by
        refine hm.mux.pushed_mem p g ?_
        intro hcz
        have : g ∈ pushHeights (s.hist p) :=
          List.mem_of_getElem? hget
        rw [pushedCount] at hcz
        exact absurd (List.count_pos_iff.mpr this) (by omega)
      -- push-time prefix of the head's flush
      obtain ⟨i₀, a₀, hi₀, hfa₀, htake₀⟩ := filterMap_take_index _
        (s.hist p) (delTotal (s.hist p.other)) g hget
      have ha₀ : a₀ = .pushed g := pushed_of_extract hfa₀
      subst ha₀
      -- counting: the head's seq is the delivered count, off by one
      -- against the slot
      have hkeq : pushedCount ((s.hist p).take i₀) g
          = deliveredCount (s.hist p.other) g := by
        rw [pushedCount, deliveredCount, hm.mux.hist_del p]
        show ((s.hist p).take i₀ |>.filterMap _).count g = _
        rw [htake₀]
        rfl
      have hslot := hm.mux.delivered_eq p g hmem_ch
      have hcap := hm.mux.slot (Chan.wire p g) hmem_ch
      have hcap1 : sk.cap (Chan.wire p g) = 1 := rfl
      -- the certificate at push time, carried to the stuck state
      have hkpos : pushedCount ((s.hist p).take i₀) g ≠ 0 := by
        omega
      have hcert := hpp p i₀ g hi₀ hkpos
      -- the keystone performs it
      have hperf := keystone hwf hm0 hm.mux hstuck p ((s.hist p).take i₀)
        (hm.mux.pushtime_delivered p htake₀)
        (fun h' => deliveredCount_le_of_prefix
          (List.take_prefix i₀ (s.hist p)) h')
        _ hcert
      rw [performed_rcv_iff] at hperf
      omega

-- ================================ the schedule sits inside the universe

/-- A positive emitted count names a trace element satisfying the
predicate. -/
private theorem emittedCount_pos {P : Ev → Bool} :
    ∀ (ts rs : List (List Ev)), 0 < Sched.emittedCount P ts rs →
      ∃ t ∈ ts, ∃ x ∈ t, P x = true := by
  intro ts
  induction ts with
  | nil =>
      intro rs h
      cases rs <;> simp [Sched.emittedCount] at h
  | cons t ts ih =>
      intro rs h
      cases rs with
      | nil => simp [Sched.emittedCount] at h
      | cons r rs =>
          rw [Sched.emittedCount] at h
          by_cases h1 :
              0 < ((t.take (t.length - r.length)).filter P).length
          · obtain ⟨x, hx⟩ := List.exists_mem_of_length_pos h1
            obtain ⟨hxt, hPx⟩ := List.mem_filter.mp hx
            exact ⟨t, List.mem_cons_self ..,
              x, List.mem_of_mem_take hxt, hPx⟩
          · obtain ⟨t', ht', hx⟩ := ih rs (by omega)
            exact ⟨t', List.mem_cons_of_mem t ht', hx⟩

/-- Every scheduled event is a universe event: the merge's provenance
(`scheduleE_count`) says the output holds only what some trace
emitted. -/
theorem mem_evUniv_of_mem_scheduleE {e : Ev}
    (he : e ∈ scheduleE sk) : e ∈ evUniv sk := by
  have hcnt := Sched.scheduleE_count sk (fun x => x == e)
  have h1 : 0 < ((scheduleE sk).filter (fun x => x == e)).length := by
    have hmem : e ∈ (scheduleE sk).filter (fun x => x == e) :=
      List.mem_filter.mpr ⟨he, by simp⟩
    exact List.length_pos_of_mem hmem
  rw [hcnt] at h1
  obtain ⟨t, ht, x, hx, hPx⟩ := emittedCount_pos _ _ h1
  have hxe : x = e := by simpa using hPx
  subst hxe
  exact mem_evUniv.mpr ⟨t, ht, hx⟩

/-- Duplicate-free from unit counts, by hand (the schedule's τ
injectivity in `Nodup` form). -/
private theorem nodup_of_count_le_one :
    ∀ {l : List Ev}, (∀ e, l.count e ≤ 1) → l.Nodup := by
  intro l
  induction l with
  | nil => exact fun _ => List.nodup_nil
  | cons a t ih =>
      intro h
      rw [List.nodup_cons]
      constructor
      · intro hmem
        have h1 := h a
        rw [List.count_cons_self] at h1
        have h2 := List.one_le_count_iff.mpr hmem
        omega
      · refine ih ?_
        intro e
        have h1 := h e
        rw [List.count_cons] at h1
        omega

/-- A scheduled event's τ sits below the universe's size: the closure
run to universe depth reaches every τ-indexed stage. -/
theorem evIdx_lt_univ (hwf : sk.wellFormed = true) {e : Ev}
    (he : e ∈ scheduleE sk) :
    evIdx e (scheduleE sk) < (evUniv sk).length := by
  have h1 : evIdx e (scheduleE sk) < (scheduleE sk).length := by
    have hg := Sched.evIdx_getElem? he
    exact (List.getElem?_eq_some_iff.mp hg).1
  have h2 : (scheduleE sk).length ≤ (evUniv sk).length := by
    refine List.Subperm.length_le ?_
    refine List.subperm_of_subset ?_
      (fun x hx => mem_evUniv_of_mem_scheduleE hx)
    exact nodup_of_count_le_one (Sched.scheduleE_count_le_oneE sk hwf)
  omega

-- ============================= Step 4: the coverage induction (the core)

/-- The first occurrence splits off the `takeWhile` prefix. -/
private theorem dropWhile_first {l : List Ev} {e : Ev} (he : e ∈ l) :
    ∃ rest, l.dropWhile (fun x => !(x == e)) = e :: rest := by
  induction l with
  | nil => cases he
  | cons a t ih =>
      by_cases hae : a = e
      · subst hae
        exact ⟨t, by simp⟩
      · rw [List.dropWhile_cons, if_pos (by simp [hae])]
        rcases List.mem_cons.mp he with rfl | het
        · exact absurd rfl hae
        · exact ih het

/-- Coverage (refute-c1 §2.4, the load-bearing novelty): at a drained
state whose τ-prefix below `N` is entirely performed, every scheduled
event τ-below `N` is in EITHER party's demand closure by its own τ
stage — performed wire sends are push-grounded (pipes-empty makes
arrival grounding complete), and every other event I-steps in because
its whole dependency past is strictly τ-below it.

The bound `k` drives the strong induction; instantiate `k := N`. -/
theorem closure_coverage (hwf : sk.wellFormed = true)
    (hm0 : ∀ sc, sk.dCount sc ≤ sk.capLevel) {s : MState}
    (hm : MuxInv sk s) (hpI : s.pipe .I = []) (hpR : s.pipe .R = [])
    (p : Party) (N : Nat)
    (hperf : ∀ g ∈ scheduleE sk, evIdx g (scheduleE sk) < N →
      performed sk s.base g) :
    ∀ (k : Nat), ∀ e ∈ scheduleE sk, evIdx e (scheduleE sk) < N →
      evIdx e (scheduleE sk) < k →
      e ∈ closureN sk p (s.hist p) (evIdx e (scheduleE sk) + 1) := by
  intro k
  induction k with
  | zero =>
      intro e _ _ hk
      omega
  | succ k ih =>
      intro e he hN hk
      have hu : e ∈ evUniv sk := mem_evUniv_of_mem_scheduleE he
      obtain ⟨c, b, n⟩ := e
      by_cases hpw : (isWire c && b) = true
      · -- a wire send: performed, hence push-grounded
        rw [Bool.and_eq_true] at hpw
        obtain ⟨hw, rfl⟩ := hpw
        obtain ⟨q, g, rfl⟩ := isWire_eq hw
        have hch : Chan.wire q g ∈ allChans sk := evUniv_wire_mem hwf hu
        have hp := hperf _ he hN
        rw [performed_snd_iff] at hp
        have hg : groundedPush p (s.hist p) (Chan.wire q g, true, n)
            = true := by
          rw [groundedPush]
          simp only [isWire, wireParty, wireHeight, Bool.true_and]
          by_cases hqp : q = p
          · subst hqp
            rw [if_pos (by simp)]
            have hpe := hm.pushed_eq q g hch
            exact decide_eq_true (by omega)
          · have hqo : q = p.other := by
              rcases party_cases' q p with h | h
              · exact absurd h hqp
              · exact h
            subst hqo
            rw [if_neg (by
              intro hcon
              exact hqp (beq_iff_eq.mp hcon))]
            have hdel := hm.delivered_eq p.other g hch
            rw [Party.other_other] at hdel
            have hflow := hm.flow_wire p.other g hch
            have hpc : pipeCount s (Chan.wire p.other g) = 0 := by
              rw [pipeCount]
              have hempty : s.pipe (wireParty (Chan.wire p.other g))
                  = [] := by
                show s.pipe p.other = []
                cases p
                · exact hpR
                · exact hpI
              rw [hempty]
              rfl
            exact decide_eq_true (by omega)
        have h0 : (Chan.wire q g, true, n)
            ∈ closureN sk p (s.hist p) 0 :=
          List.mem_filter.mpr ⟨hu, hg⟩
        exact closureN_le (Nat.zero_le _) _ h0
      · -- a non-push event: its whole dependency past I-steps it in
        have hstep : istepOk sk
            (closureN sk p (s.hist p) (evIdx ((c, b, n) : Ev)
              (scheduleE sk))) (c, b, n) = true := by
          rw [istepOk]
          simp only [Bool.and_eq_true]
          refine ⟨⟨⟨by simp [hpw], ?_⟩, ?_⟩, ?_⟩
          · -- E1: the receive's send is τ-below
            cases b with
            | true => simp
            | false =>
                rw [Bool.false_or]
                obtain ⟨hsm, hτ⟩ := tau_e1 hwf he
                have hin := ih _ hsm (by omega) (by omega)
                refine (List.contains_iff_mem ..).mpr ?_
                exact closureN_le (by omega) _ hin
          · -- E2: the send's cap-window receive is τ-below
            cases b with
            | false => simp
            | true =>
                by_cases hcap : n < sk.cap c
                · simp [hcap]
                · obtain ⟨hrm, hτ⟩ := tau_e2 hwf he (by omega)
                  have hin := ih _ hrm (by omega) (by omega)
                  rw [Bool.or_eq_true]
                  refine Or.inr ?_
                  refine (List.contains_iff_mem ..).mpr ?_
                  exact closureN_le (by omega) _ hin
          · -- E3: the whole trace past is τ-below
            rw [List.all_eq_true]
            intro T hT
            rw [Bool.or_eq_true]
            by_cases heT : (c, b, n) ∈ T
            · refine Or.inr ?_
              rw [List.all_eq_true]
              intro x hx
              have hxT : x ∈ T :=
                (List.takeWhile_prefix _).sublist.mem hx
              have hxm : x ∈ scheduleE sk :=
                (Sched.trace_sublistE sk hwf hm0 hT).mem hxT
              -- x precedes e in T
              obtain ⟨tail, htail⟩ := dropWhile_first heT
              have hpair : ([x, (c, b, n)] : List Ev).Sublist T := by
                have hsplit := List.takeWhile_append_dropWhile
                  (p := fun y => !(y == (c, b, n))) (l := T)
                rw [htail] at hsplit
                rw [← hsplit]
                have h1 : ([x] : List Ev).Sublist
                    (T.takeWhile (fun y => !(y == (c, b, n)))) :=
                  List.singleton_sublist.mpr hx
                have h2 : ([(c, b, n)] : List Ev).Sublist
                    ((c, b, n) :: tail) :=
                  List.singleton_sublist.mpr (List.mem_cons_self ..)
                exact List.Sublist.append h1 h2
              have hτx : evIdx x (scheduleE sk)
                  < evIdx ((c, b, n) : Ev) (scheduleE sk) :=
                tau_lt_of_trace_pair hwf hm0 hT hpair
              have hin := ih _ hxm (by omega) (by omega)
              refine (List.contains_iff_mem ..).mpr ?_
              exact closureN_le (by omega) _ hin
            · refine Or.inl ?_
              rw [Bool.not_eq_true']
              cases hcont : T.contains (c, b, n) with
              | false => rfl
              | true =>
                  exact absurd ((List.contains_iff_mem ..).mp hcont) heT
        show (c, b, n) ∈ closureStep sk p (s.hist p) _
        refine List.mem_filter.mpr ⟨hu, ?_⟩
        rw [Bool.or_eq_true, Bool.or_eq_true]
        exact Or.inr hstep

-- ============================================================= T4

/-- T4, the campaign's centerpiece (MUX-ADJUDICATION §3; the liveness
half of C1's refutation — the charter-grain refutation of record is
`c1_charter_false` via σ*-causal, Proofs/C1.lean): the σ*×σ*
composition is deadlock-free on the shipping encoder's class at every
capacity C ≥ 1 per direction.

Steps: the push certificates drain the pipes at any stuck candidate
(`sigmaStar_pipes_empty`); the chase exhibits the τ-least withheld push
with everything τ-below it performed; coverage puts the predecessor's
consumption in the sender's closure, so the frame is proven-demanded,
σ* names a push, and the stuck state cannot exist.

Capacity is message-denominated; the W = 1 byte-soundness caveat of
record is Mux/Basic.lean's module doc (# The byte-denomination
caveat). -/
theorem sigmaStar_deadlock_free (hwf : sk.wellFormed = true)
    (hm0 : ∀ sc, sk.dCount sc ≤ sk.capLevel) (C : Nat) (hC : 1 ≤ C) :
    MuxDeadlockFree sk .impl C sigmaStar sigmaStar := by
  intro s hr
  cases hst : mstuck sk .impl C sigmaStar sigmaStar s with
  | false => rfl
  | true =>
      exfalso
      have hm := sinv_reachable hwf hr
      have hpp := pushProven_reachable hwf hr
      -- Step 1: the pipes drain
      have hpI : s.pipe .I = [] :=
        sigmaStar_pipes_empty hwf hm0 hm hpp hst .I
      have hpR : s.pipe .R = [] :=
        sigmaStar_pipes_empty hwf hm0 hm hpp hst .R
      have hnt : mterminal sk s = false := by
        rw [mstuck, Bool.and_eq_true, Bool.not_eq_true'] at hst
        exact hst.1
      -- Steps 2–3: the τ-least withheld push, everything below performed
      obtain ⟨f, a, p, hh, hfc, hfb, hfseq, hfsched, hfnp, hleast,
        hcover, hpend, hok, hhold⟩ :=
        chase hwf hm0 hm.mux hpI hpR hst hnt
      obtain ⟨c', b', n'⟩ := f
      simp only at hfc hfb hfseq
      subst hfc
      subst hfb
      subst hfseq
      -- the withheld stream is real and history-held
      have hch : Chan.wire p hh ∈ allChans sk := by
        rw [holdsWire.eq_def] at hhold
        split at hhold
        · next hr' =>
            rw [show hh = sk.rootH from by simpa using hr']
            exact mem_allChans_wire_root p
        · simp only [Bool.and_eq_true] at hhold
          exact mem_allChans_wireOut
            ((List.contains_iff_mem ..).mp hhold.1.1)
      have hhold' : holdsWire sk p hh s.base = true := by
        rw [holdsWire.eq_def] at hhold ⊢
        exact hhold
      have hcm : committedInHist sk.rootH (s.hist p) hh = true := by
        rw [committedInHist_iff_holdsWire hm.hist]
        exact hhold'
      have hne : pushedCount (s.hist p) hh
          = sentOf sk s.base (Chan.wire p hh) :=
        hm.mux.pushed_eq p hh hch
      -- Step 4: the frame is proven-demanded
      have hdem : demanded sk p (s.hist p) hh = true := by
        rw [demanded, Bool.or_eq_true]
        by_cases hn0 : pushedCount (s.hist p) hh = 0
        · exact Or.inl (by simpa using hn0)
        · refine Or.inr ((List.contains_iff_mem ..).mpr ?_)
          have hcap1 : sk.cap (Chan.wire p hh) = 1 := rfl
          obtain ⟨hrm, hτ⟩ := tau_e2 hwf hfsched (by omega)
          have hcov := closure_coverage hwf hm0 hm.mux hpI hpR p
            (evIdx ((Chan.wire p hh, true,
              sentOf sk s.base (Chan.wire p hh)) : Ev) (scheduleE sk))
            hcover
            (evIdx ((Chan.wire p hh, true,
              sentOf sk s.base (Chan.wire p hh)) : Ev) (scheduleE sk))
            _ hrm hτ hτ
          have hlt := evIdx_lt_univ hwf hrm
          have hmono := closureN_le (sk := sk) (p := p)
            (tr := s.hist p) (n := (evUniv sk).length)
            (by omega) _ hcov
          rw [hcap1] at hmono
          rw [hne]
          exact hmono
      -- σ* therefore names a push — against stuckness
      have hcz : commitsOf sk.rootH (s.hist p) hh ≠ 0 := by
        rw [committedInHist_eq, decide_eq_true_eq] at hcm
        omega
      obtain ⟨q, hq0⟩ := Option.isSome_iff_exists.mp
        (partyOf_isSome_of_commits hcz)
      have hqp : q = p := partyOf_eq hm.hist hq0
      have hq : partyOf (s.hist p) = some p := by
        rw [hqp] at hq0
        exact hq0
      have hsome := sigmaStar_isSome hq
        (holdsWire_mem_wireHeights hhold') hcm hdem
      obtain ⟨h', hσ⟩ := Option.isSome_iff_exists.mp hsome
      obtain ⟨q', -, -, hcm', -⟩ := sigmaStar_some_inv hσ
      have hwp : WithheldPush sk C p h' s := by
        refine ⟨?_, ?_⟩
        · rw [← committedInHist_iff_holdsWire hm.hist]
          exact hcm'
        · have hempty : s.pipe p = [] := by
            cases p
            · exact hpI
            · exact hpR
          rw [hempty]
          simp only [List.length_nil]
          omega
      have hout := mstuck_withheld hst hwp
      apply hout
      cases p <;> exact hσ

end StreamingMirror.Mux
