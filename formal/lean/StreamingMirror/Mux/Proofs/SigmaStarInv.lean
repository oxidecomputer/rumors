/-
`MuxInv` preservation — the stage-3 flowOk-template induction
(MUX-ADJUDICATION.md §4 stage F) — plus the history-side invariants σ*'s
liveness proof consumes.

# The three layers

- `MuxInv` (Chase/Ground.lean): the transport ground facts. Preserved
  by EVERY muxed step under EVERY strategy pair — the strategy only
  selects which pushes happen, never what a step does — so the
  preservation theorem here (`sinv_reachable`) is generic and serves
  T5's oracle as well as T4's σ*.
- `HistInv` (this file): the observation histories decode — every
  recorded action belongs to its machine (`hist_party`, what makes
  `partyOf` correct) and the commit/flush ledger tracks the committed
  hand exactly (`hand_count`, what makes `committedInHist` agree with
  `holdsWire`). Also strategy-generic.
- `PushProven` (this file): σ*-specific — every push in the history
  carried a demand certificate at its own push-time observation prefix.
  This is INV-A of refute-c1 §2.1, the input to T4's Step 1: at a stuck
  state the pipe head's predecessor-consumption is closure-derivable at
  push time, and the keystone then contradicts its unconsumed slot.

# Method

Each base arm's facts come from the Steps files (Top/WalkAsm/Fire);
this file only assembles: the `QuietStep`/`RecvStep`/`SendStep` deltas
meet the count fields, the hands clauses meet `hand_count`, and the
push/deliver arms are pure counting over the FIFO split
(`hist_del`/`hist_pipe`).
-/
import StreamingMirror.Mux.Proofs.Steps.Top
import StreamingMirror.Mux.Proofs.Steps.WalkAsm
import StreamingMirror.Mux.Proofs.Steps.Fire
import StreamingMirror.Mux.Proofs.Chase
import StreamingMirror.Mux.SigmaStar

namespace StreamingMirror.Mux

open Model

variable {sk : Skel}

-- ===================================================== history algebra

/-- Heights appended by one observation, as `pushHeights` sees it. -/
theorem pushHeights_append (tr : List MObs) (o : MObs) :
    pushHeights (tr ++ [o])
      = pushHeights tr ++ (match o with
          | .pushed h => [h]
          | _ => []) := by
  rw [pushHeights, List.filterMap_append]
  cases o <;> rfl

/-- Heights appended by one observation, as `delHeights` sees it. -/
theorem delHeights_append (tr : List MObs) (o : MObs) :
    delHeights (tr ++ [o])
      = delHeights tr ++ (match o with
          | .delivered h => [h]
          | _ => []) := by
  rw [delHeights, List.filterMap_append]
  cases o <;> rfl

/-- `pushedCount` under an `.act` or `.delivered` append. -/
theorem pushedCount_append_other (tr : List MObs) {o : MObs}
    (hno : ∀ h, o ≠ .pushed h) (h : Nat) :
    pushedCount (tr ++ [o]) h = pushedCount tr h := by
  rw [pushedCount, pushHeights_append]
  cases o with
  | pushed h' => exact absurd rfl (hno h')
  | act a => simp [pushedCount]
  | delivered h' => simp [pushedCount]

/-- `pushedCount` under a `.pushed` append. -/
theorem pushedCount_append_pushed (tr : List MObs) (h h' : Nat) :
    pushedCount (tr ++ [.pushed h']) h
      = pushedCount tr h + (if h' = h then 1 else 0) := by
  rw [pushedCount, pushHeights_append]
  simp only [List.count_append, pushedCount]
  congr 1
  by_cases he : h' = h
  · subst he
    simp
  · simp [he]

/-- `deliveredCount` under a non-`.delivered` append. -/
theorem deliveredCount_append_other (tr : List MObs) {o : MObs}
    (hno : ∀ h, o ≠ .delivered h) (h : Nat) :
    deliveredCount (tr ++ [o]) h = deliveredCount tr h := by
  rw [deliveredCount, delHeights_append]
  cases o with
  | delivered h' => exact absurd rfl (hno h')
  | act a => simp [deliveredCount]
  | pushed h' => simp [deliveredCount]

/-- `deliveredCount` under a `.delivered` append. -/
theorem deliveredCount_append_delivered (tr : List MObs) (h h' : Nat) :
    deliveredCount (tr ++ [.delivered h']) h
      = deliveredCount tr h + (if h' = h then 1 else 0) := by
  rw [deliveredCount, delHeights_append]
  simp only [List.count_append, deliveredCount]
  congr 1
  by_cases he : h' = h
  · subst he
    simp
  · simp [he]

/-- `delTotal` under a non-`.delivered` append. -/
theorem delTotal_append_other (tr : List MObs) {o : MObs}
    (hno : ∀ h, o ≠ .delivered h) :
    delTotal (tr ++ [o]) = delTotal tr := by
  rw [delTotal, delHeights_append]
  cases o with
  | delivered h' => exact absurd rfl (hno h')
  | act a => simp [delTotal]
  | pushed h' => simp [delTotal]

/-- `delTotal` under a `.delivered` append. -/
theorem delTotal_append_delivered (tr : List MObs) (h' : Nat) :
    delTotal (tr ++ [.delivered h']) = delTotal tr + 1 := by
  rw [delTotal, delHeights_append]
  simp [delTotal]

-- =============================================== the commit/flush ledger

/-- Wire commits recorded on stream `h`, the `committedInHist`
numerator restated standalone. -/
def commitsOf (rootH : Nat) (tr : List MObs) (h : Nat) : Nat :=
  tr.countP fun o =>
    match o with
    | .act (.walkCommit pk (.wire _)) => pk.2 == h
    | .act (.iopenChoose .wire) => h == rootH
    | .act (.ropenChoose .wire) => h == rootH
    | _ => false

/-- Flush receipts recorded on stream `h`, the `committedInHist`
denominator restated standalone. -/
def pushesOf (tr : List MObs) (h : Nat) : Nat :=
  tr.countP fun o =>
    match o with
    | .pushed h' => h' == h
    | _ => false

/-- `committedInHist` is the ledger comparison. -/
theorem committedInHist_eq (rootH : Nat) (tr : List MObs) (h : Nat) :
    committedInHist rootH tr h
      = decide (pushesOf tr h < commitsOf rootH tr h) := rfl

/-- The two flush counts agree: `countP` over the tags is the
filterMap count. -/
theorem pushesOf_eq_pushedCount (tr : List MObs) (h : Nat) :
    pushesOf tr h = pushedCount tr h := by
  induction tr with
  | nil => rfl
  | cons o tr ih =>
      rw [pushesOf, List.countP_cons]
      rw [pushesOf] at ih
      cases o with
      | pushed h' =>
          rw [show pushedCount (MObs.pushed h' :: tr) h
              = (h' :: pushHeights tr).count h from rfl]
          rw [List.count_cons, ih]
          by_cases he : h' = h <;> simp [pushedCount, he, beq_iff_eq]
      | act a =>
          rw [show pushedCount (MObs.act a :: tr) h
              = pushedCount tr h from rfl, ih]
          simp [pushedCount]
      | delivered h' =>
          rw [show pushedCount (MObs.delivered h' :: tr) h
              = pushedCount tr h from rfl, ih]
          simp [pushedCount]

/-- Is this action a wire commit on stream `h`? The `commitsOf`
pattern, exposed for the per-arm neutrality side conditions. -/
def wireCommitOn (rootH : Nat) (a : Action) (h : Nat) : Bool :=
  match a with
  | .walkCommit pk (.wire _) => pk.2 == h
  | .iopenChoose .wire => h == rootH
  | .ropenChoose .wire => h == rootH
  | _ => false

/-- `commitsOf` under an `.act` append. -/
theorem commitsOf_append_act (rootH : Nat) (tr : List MObs) (a : Action)
    (h : Nat) :
    commitsOf rootH (tr ++ [.act a]) h
      = commitsOf rootH tr h
        + (if wireCommitOn rootH a h then 1 else 0) := by
  rw [commitsOf, List.countP_append, commitsOf]
  congr 1
  rw [List.countP_cons]
  simp only [List.countP_nil, Nat.zero_add]
  cases a <;> first
    | rfl
    | (rename_i x; cases x <;> rfl)

/-- `commitsOf` under a non-`.act` append. -/
theorem commitsOf_append_other (rootH : Nat) (tr : List MObs) {o : MObs}
    (hno : ∀ a, o ≠ .act a) (h : Nat) :
    commitsOf rootH (tr ++ [o]) h = commitsOf rootH tr h := by
  rw [commitsOf, List.countP_append, commitsOf]
  cases o with
  | act a => exact absurd rfl (hno a)
  | pushed h' => simp
  | delivered h' => simp

/-- `pushesOf` under an append. -/
theorem pushesOf_append (tr : List MObs) (o : MObs) (h : Nat) :
    pushesOf (tr ++ [o]) h
      = pushesOf tr h
        + (match o with
           | .pushed h' => if h' = h then 1 else 0
           | _ => 0) := by
  rw [pushesOf, List.countP_append, pushesOf]
  congr 1
  rw [List.countP_cons]
  simp only [List.countP_nil, Nat.zero_add]
  cases o with
  | pushed h' => by_cases he : h' = h <;> simp [he]
  | act a => rfl
  | delivered h' => rfl

-- ================================================ the extended invariant

/-- The history-side ground facts, strategy-generic: recorded actions
belong to their machine, and the commit/flush ledger is exactly the
committed-hand occupancy. -/
structure HistInv (sk : Skel) (s : MState) : Prop where
  hist_party : ∀ p a, MObs.act a ∈ s.hist p → actionParty a = p
  hand_count : ∀ p h, commitsOf sk.rootH (s.hist p) h
    = pushesOf (s.hist p) h
      + (if holdsWire sk p h s.base then 1 else 0)

/-- The full strategy-generic muxed invariant: transport ground facts
plus history decode. -/
structure SInv (sk : Skel) (s : MState) : Prop where
  mux : MuxInv sk s
  hist : HistInv sk s

/-- σ*'s push certificates (INV-A, refute-c1 §2.1): every recorded
push was proven-demanded against its own push-time observation
prefix. -/
def PushProven (sk : Skel) (s : MState) : Prop :=
  ∀ p i h, (s.hist p)[i]? = some (.pushed h) →
    pushedCount ((s.hist p).take i) h ≠ 0 →
    (Chan.wire p h, false, pushedCount ((s.hist p).take i) h - 1)
      ∈ inevitable sk p ((s.hist p).take i)

-- ====================================================== `partyOf` decode

/-- `partyOf` is correct on any history satisfying `hist_party`: a hit
names the machine itself. -/
theorem partyOf_eq {s : MState} (hh : HistInv sk s) {p q : Party}
    (hq : partyOf (s.hist p) = some q) : q = p := by
  rw [partyOf] at hq
  obtain ⟨o, ho, hsome⟩ := List.exists_of_findSome?_eq_some hq
  cases o with
  | act a =>
      have := hh.hist_party p a ho
      simp only [Option.some.injEq] at hsome
      rw [← hsome, this]
  | pushed h => cases hsome
  | delivered h => cases hsome

/-- A history holding a wire commit has acted, so `partyOf` hits — and
by `partyOf_eq` it hits the machine itself. -/
theorem partyOf_isSome_of_commits {s : MState} {p : Party} {h : Nat}
    (hc : commitsOf sk.rootH (s.hist p) h ≠ 0) :
    (partyOf (s.hist p)).isSome = true := by
  rw [commitsOf] at hc
  obtain ⟨o, ho, hpo⟩ := List.countP_pos_iff.mp (Nat.pos_of_ne_zero hc)
  cases o with
  | act a =>
      rw [partyOf]
      cases hfs : (s.hist p).findSome? fun o =>
          match o with
          | .act a => some (actionParty a)
          | _ => none with
      | some q => rfl
      | none =>
          have := List.findSome?_eq_none_iff.mp hfs (.act a) ho
          simp at this
  | pushed h' => simp at hpo
  | delivered h' => simp at hpo

/-- The ledger reads the hand exactly (`hand_count` cashed in):
`committedInHist` and `holdsWire` agree at every reachable state. -/
theorem committedInHist_iff_holdsWire {s : MState} (hh : HistInv sk s)
    (p : Party) (h : Nat) :
    committedInHist sk.rootH (s.hist p) h = holdsWire sk p h s.base := by
  rw [committedInHist_eq]
  have := hh.hand_count p h
  cases hw : holdsWire sk p h s.base with
  | true =>
      rw [hw] at this
      simp only [if_true] at this
      simp only [decide_eq_true_eq]
      omega
  | false =>
      rw [hw] at this
      rw [if_neg (by simp)] at this
      simp only [decide_eq_false_iff_not]
      omega

-- ============================================ shape facts, semantically

/-- What the count fields need from one base cursor step, shape-erased:
occupancy stays capacity-bounded, internal conservation holds, wire
producer counts are untouched, and the wire receive/slot sum is
conserved. -/
structure BaseFacts (sk : Skel) (s₀ b : State) : Prop where
  slot : ∀ c ∈ allChans sk, b.chan c ≤ sk.cap c
  flow_int : ∀ c ∈ allChans sk, isWire c = false →
    b.chan c + recvdOf sk b c = sentOf sk b c
  sent_wire : ∀ q g, Chan.wire q g ∈ allChans sk →
    sentOf sk b (Chan.wire q g) = sentOf sk s₀ (Chan.wire q g)
  del_sum : ∀ q g, Chan.wire q g ∈ allChans sk →
    recvdOf sk b (Chan.wire q g) + b.chan (Chan.wire q g)
      = recvdOf sk s₀ (Chan.wire q g) + s₀.chan (Chan.wire q g)

/-- A quiet step delivers the base facts. -/
theorem BaseFacts.of_quiet {s₀ b : State}
    (hslot : ∀ c ∈ allChans sk, s₀.chan c ≤ sk.cap c)
    (hflow : ∀ c ∈ allChans sk, isWire c = false →
      s₀.chan c + recvdOf sk s₀ c = sentOf sk s₀ c)
    (hq : QuietStep sk s₀ b) : BaseFacts sk s₀ b := by
  refine ⟨?_, ?_, ?_, ?_⟩
  · intro c hc
    rw [hq.chan]
    exact hslot c hc
  · intro c hc hw
    rw [hq.chan, hq.sent c hc, hq.recvd c hc]
    exact hflow c hc hw
  · intro q g hc
    exact hq.sent _ hc
  · intro q g hc
    rw [hq.chan, hq.recvd _ hc]

/-- A receive step delivers the base facts: the received channel's
occupancy drop balances its consumer-count rise. -/
theorem BaseFacts.of_recv {s₀ b : State} {c₀ : Chan}
    (hslot : ∀ c ∈ allChans sk, s₀.chan c ≤ sk.cap c)
    (hflow : ∀ c ∈ allChans sk, isWire c = false →
      s₀.chan c + recvdOf sk s₀ c = sentOf sk s₀ c)
    (hr : RecvStep sk s₀ b c₀) :
    BaseFacts sk s₀ b := by
  refine ⟨?_, ?_, ?_, ?_⟩
  · intro c hc
    rw [hr.chan]
    by_cases he : c = c₀
    · subst he
      rw [bump_neg_one]
      have := hslot c hc
      omega
    · rw [bump_ne _ _ he]
      exact hslot c hc
  · intro c hc hw
    have hflow := hflow c hc hw
    have hpos := hr.hpos
    rw [hr.chan, hr.sent c hc, hr.recvd c hc]
    by_cases he : c = c₀
    · subst he
      rw [bump_neg_one, if_pos rfl]
      omega
    · rw [bump_ne _ _ he, if_neg he]
      omega
  · intro q g hc
    exact hr.sent _ hc
  · intro q g hc
    have hpos := hr.hpos
    rw [hr.chan, hr.recvd _ hc]
    by_cases he : Chan.wire q g = c₀
    · rw [he, bump_neg_one, if_pos rfl]
      omega
    · rw [bump_ne _ _ he, if_neg he]
      omega

/-- A send step into an INTERNAL channel delivers the base facts: the
mux never sends on a wire channel through a base arm. -/
theorem BaseFacts.of_send {s₀ b : State} {c₀ : Chan}
    (hslot : ∀ c ∈ allChans sk, s₀.chan c ≤ sk.cap c)
    (hflow : ∀ c ∈ allChans sk, isWire c = false →
      s₀.chan c + recvdOf sk s₀ c = sentOf sk s₀ c)
    (hs : SendStep sk s₀ b c₀)
    (hw₀ : isWire c₀ = false) : BaseFacts sk s₀ b := by
  refine ⟨?_, ?_, ?_, ?_⟩
  · intro c hc
    rw [hs.chan]
    by_cases he : c = c₀
    · subst he
      rw [bump_one]
      have := hs.hcap
      omega
    · rw [bump_ne _ _ he]
      exact hslot c hc
  · intro c hc hw
    have hflow := hflow c hc hw
    rw [hs.chan, hs.sent c hc, hs.recvd c hc]
    by_cases he : c = c₀
    · subst he
      rw [bump_one, if_pos rfl]
      omega
    · rw [bump_ne _ _ he, if_neg he]
      omega
  · intro q g hc
    have hne : Chan.wire q g ≠ c₀ := by
      intro he
      rw [← he] at hw₀
      simp [isWire] at hw₀
    rw [hs.sent _ hc, if_neg hne]
    omega
  · intro q g hc
    have hne : Chan.wire q g ≠ c₀ := by
      intro he
      rw [← he] at hw₀
      simp [isWire] at hw₀
    rw [hs.chan, hs.recvd _ hc, bump_ne _ _ hne]


-- =============================================== the base-arm assembly

/-- The transport and action-attribution fields under one `.act`
append: none of them reads the hands, so both hand variants (neutral
and commit-flip) share this core. -/
private theorem mux_act_append {s : MState} {b : State} {a : Action}
    (hm : SInv sk s) (hL' : InvL sk .impl b)
    (hbf : BaseFacts sk s.base b) :
    MuxInv sk { s with base := b
                       hist := recordObs s.hist (actionParty a) (.act a) }
    ∧ ∀ p a', MObs.act a'
        ∈ recordObs s.hist (actionParty a) (.act a) p →
        actionParty a' = p := by
  have hno_p : ∀ (h : Nat), (MObs.act a) ≠ .pushed h := fun _ h => by cases h
  have hno_d : ∀ (h : Nat), (MObs.act a) ≠ .delivered h := fun _ h => by
    cases h
  have hhist : ∀ q, recordObs s.hist (actionParty a) (.act a) q
      = if q == actionParty a then s.hist q ++ [.act a] else s.hist q := by
    intro q
    rfl
  have hdh : ∀ q, delHeights (recordObs s.hist (actionParty a)
      (.act a) q) = delHeights (s.hist q) := by
    intro q
    rw [hhist]
    by_cases hq : (q == actionParty a) = true
    · rw [if_pos hq, delHeights_append]
      simp
    · rw [if_neg hq]
  have hph : ∀ q, pushHeights (recordObs s.hist (actionParty a)
      (.act a) q) = pushHeights (s.hist q) := by
    intro q
    rw [hhist]
    by_cases hq : (q == actionParty a) = true
    · rw [if_pos hq, pushHeights_append]
      simp
    · rw [if_neg hq]
  have hdt : ∀ q, delTotal (recordObs s.hist (actionParty a)
      (.act a) q) = delTotal (s.hist q) := by
    intro q
    rw [delTotal, delTotal, hdh]
  constructor
  · refine ⟨hL', hbf.slot, hbf.flow_int, ?_, ?_, ?_, ?_, ?_⟩
    · intro p h hc
      rw [hbf.sent_wire p h hc, ← hm.mux.pushed_eq p h hc]
      show pushedCount (recordObs s.hist (actionParty a) (.act a) p) h
        = _
      rw [pushedCount, hph, ← pushedCount]
    · intro p
      show delHeights (recordObs s.hist (actionParty a) (.act a) p.other)
        = (pushHeights (recordObs s.hist (actionParty a)
            (.act a) p)).take
            (delTotal (recordObs s.hist (actionParty a) (.act a) p.other))
      rw [hdh, hph, hdt]
      exact hm.mux.hist_del p
    · intro p
      show s.pipe p = _
      rw [hph, hdt]
      exact hm.mux.hist_pipe p
    · intro p h hc
      rw [hbf.del_sum p h hc, ← hm.mux.delivered_eq p h hc]
      show deliveredCount (recordObs s.hist (actionParty a)
        (.act a) p.other) h = _
      rw [deliveredCount, hdh, ← deliveredCount]
    · intro p h hne
      refine hm.mux.pushed_mem p h ?_
      revert hne
      show pushedCount (recordObs s.hist (actionParty a) (.act a) p) h
          ≠ 0 → _
      rw [pushedCount, hph, ← pushedCount]
      exact id
  · intro p a' hmem
    have hmem' : MObs.act a'
        ∈ recordObs s.hist (actionParty a) (.act a) p := hmem
    rw [hhist] at hmem'
    by_cases hq : (p == actionParty a) = true
    · rw [if_pos hq] at hmem'
      rcases List.mem_append.mp hmem' with hold | hnew
      · exact hm.hist.hist_party p a' hold
      · have := List.mem_singleton.mp hnew
        injection this with hh
        rw [hh]
        exact (beq_iff_eq.mp hq).symm
    · rw [if_neg hq] at hmem'
      exact hm.hist.hist_party p a' hmem'

/-- One hand-neutral base action's effect on the muxed state,
reassembled: the transport fields from `BaseFacts`, the histories from
the `.act`-append algebra, the untouched hand ledger from `HandsEq`. -/
theorem SInv.base_assemble {s : MState} {b : State} {a : Action}
    (hm : SInv sk s) (hL' : InvL sk .impl b)
    (hbf : BaseFacts sk s.base b)
    (hhand : ∀ p h, holdsWire sk p h b = holdsWire sk p h s.base)
    (hnc : ∀ h, wireCommitOn sk.rootH a h = false) :
    SInv sk { s with base := b
                     hist := recordObs s.hist (actionParty a) (.act a) } := by
  obtain ⟨hmux, hparty⟩ := mux_act_append (a := a) hm hL' hbf
  refine ⟨hmux, hparty, ?_⟩
  intro p h
  show commitsOf sk.rootH (recordObs s.hist (actionParty a)
    (.act a) p) h = pushesOf (recordObs s.hist (actionParty a)
    (.act a) p) h + _
  rw [show recordObs s.hist (actionParty a) (.act a) p
      = if p == actionParty a then s.hist p ++ [.act a] else s.hist p
      from rfl, hhand]
  by_cases hq : (p == actionParty a) = true
  · rw [if_pos hq, commitsOf_append_act, hnc h, if_neg (by simp),
      pushesOf_append]
    have := hm.hist.hand_count p h
    omega
  · rw [if_neg hq]
    exact hm.hist.hand_count p h

/-- The commit-arm assembly: one wire commit flips exactly one hand on
while its machine's ledger gains exactly one commit — the `hand_count`
books balance on both sides of the flip. -/
theorem SInv.base_assemble_commit {s : MState} {b : State} {a : Action}
    {p₀ : Party} {h₀ : Nat}
    (hm : SInv sk s) (hL' : InvL sk .impl b)
    (hbf : BaseFacts sk s.base b)
    (hap : actionParty a = p₀)
    (hcn : ∀ h, wireCommitOn sk.rootH a h = decide (h = h₀))
    (hoff : holdsWire sk p₀ h₀ s.base = false)
    (hon : holdsWire sk p₀ h₀ b = true)
    (hother : ∀ p h, ¬(p = p₀ ∧ h = h₀) →
      holdsWire sk p h b = holdsWire sk p h s.base) :
    SInv sk { s with base := b
                     hist := recordObs s.hist (actionParty a) (.act a) } := by
  obtain ⟨hmux, hparty⟩ := mux_act_append (a := a) hm hL' hbf
  refine ⟨hmux, hparty, ?_⟩
  intro p h
  show commitsOf sk.rootH (recordObs s.hist (actionParty a)
    (.act a) p) h = pushesOf (recordObs s.hist (actionParty a)
    (.act a) p) h + _
  rw [show recordObs s.hist (actionParty a) (.act a) p
      = if p == actionParty a then s.hist p ++ [.act a] else s.hist p
      from rfl]
  have hold := hm.hist.hand_count p h
  by_cases hq : (p == actionParty a) = true
  · have hp₀ : p = p₀ := by
      rw [beq_iff_eq.mp hq, hap]
    rw [if_pos hq, commitsOf_append_act, hcn h, pushesOf_append]
    by_cases hh : h = h₀
    · subst hh
      subst hp₀
      rw [hoff] at hold
      rw [hon]
      rw [if_neg (by simp)] at hold
      simp only [decide_true, if_true]
      omega
    · rw [hother p h (fun hcon => hh hcon.2)]
      simp only [decide_eq_true_eq]
      rw [if_neg hh]
      omega
  · rw [if_neg hq]
    by_cases hpe : p = p₀ ∧ h = h₀
    · exfalso
      rw [hpe.1, ← hap] at hq
      simp at hq
    · rw [hother p h hpe]
      exact hold

-- ================================================= the base dispatcher

/-- Decompose a successful muxed base step: no wire fire, the base
model stepped, the observation was recorded. -/
theorem applyBase_inv {ax : AxMode} {a : Action} {s s' : MState}
    (hstep : applyBase sk ax a s = some s') :
    isWireFire s.base a = false
    ∧ ∃ b, Model.apply sk ax a s.base = some b
      ∧ s' = { s with base := b
                      hist := recordObs s.hist (actionParty a) (.act a) } := by
  have hstep' : (if isWireFire s.base a then none
      else if (match a with
        | .walkCloseWire pk => !pipeClear s (wireIn pk)
        | .absorbCloseWire => !pipeClear s (Chan.wire .R 0)
        | _ => false) then none
      else (Model.apply sk ax a s.base).map fun b =>
        { s with base := b
                 hist := recordObs s.hist (actionParty a) (.act a) })
      = some s' := hstep
  clear hstep
  split at hstep'
  · cases hstep'
  next hnf =>
    cases hbl : (match a with
        | Action.walkCloseWire pk => !pipeClear s (wireIn pk)
        | Action.absorbCloseWire => !pipeClear s (Chan.wire Party.R 0)
        | _ => false) with
    | true =>
        rw [hbl] at hstep'
        simp at hstep'
    | false =>
        rw [hbl] at hstep'
        rw [if_neg (by simp)] at hstep'
        cases hb : Model.apply sk ax a s.base with
        | none =>
            rw [hb] at hstep'
            cases hstep'
        | some b =>
            rw [hb] at hstep'
            simp only [Option.map] at hstep'
            injection hstep' with hs'
            exact ⟨by simpa using hnf, b, rfl, hs'.symm⟩

/-- Capacity of a non-wire fired channel is one: every internal cell a
walk publishes into is a cap-1 handoff. -/
theorem cap_obligChan_nonwire (pk : Party × Nat) (o : Oblig) :
    sk.cap (obligChan pk o) = 1 ∨ obligChan pk o = wireOut pk := by
  cases o with
  | wire i => exact Or.inr rfl
  | res i => exact Or.inl rfl
  | query i =>
      refine Or.inl ?_
      rw [obligChan, askedOut]
      split <;> rfl
  | parent => exact Or.inl rfl

/-- Non-wire fired channels are internal. -/
theorem isWire_obligChan_nonwire (pk : Party × Nat) {o : Oblig}
    (hnw : ∀ i, o ≠ .wire i) : isWire (obligChan pk o) = false := by
  cases o with
  | wire i => exact absurd rfl (hnw i)
  | res i => rfl
  | query i =>
      rw [obligChan, askedOut]
      split <;> rfl
  | parent => rfl

/-- Every enabled base action preserves the strategy-generic muxed
invariant: the 23-arm dispatch through the Steps files. -/
theorem sinv_base (hwf : sk.wellFormed = true) {a : Action} {s s' : MState}
    (hstep : applyBase sk .impl a s = some s') (hm : SInv sk s) :
    SInv sk s' := by
  obtain ⟨hnf, b, hb, hs'⟩ := applyBase_inv hstep
  have hL := hm.mux.invl
  have hslot := hm.mux.slot
  have hflow := hm.mux.flow_int
  subst hs'
  cases a with
  | iopenChoose o =>
      cases o with
      | wire =>
          obtain ⟨hL', hq, hoff, hon, hother⟩ :=
            step_iopenChoose_wire hb hL
          exact SInv.base_assemble_commit hm hL'
            (BaseFacts.of_quiet hslot hflow hq) rfl
            (fun h => by
              show (h == sk.rootH) = decide (h = sk.rootH)
              by_cases hh : h = sk.rootH <;> simp [hh])
            hoff hon hother
      | query =>
          obtain ⟨hL', hq, hh⟩ := step_iopenChoose_query hb hL
          exact SInv.base_assemble hm hL'
            (BaseFacts.of_quiet hslot hflow hq) hh (fun h => rfl)
  | iopenFire =>
      have hch : s.base.iopenCh = some .query := by
        rw [Model.apply] at hb
        cases hio : s.base.iopenCh with
        | none => rw [hio] at hb; cases hb
        | some o =>
            cases o with
            | wire =>
                exfalso
                rw [isWireFire, hio] at hnf
                simp at hnf
            | query => rfl
      obtain ⟨hL', hsend, hh⟩ := step_iopenFire_query hch hb hL
      exact SInv.base_assemble hm hL'
        (BaseFacts.of_send hslot hflow hsend rfl) hh (fun h => rfl)
  | ropenRecv =>
      obtain ⟨hL', hr, hh⟩ := step_ropenRecv hb hL
      exact SInv.base_assemble hm hL'
        (BaseFacts.of_recv hslot hflow hr) hh (fun h => rfl)
  | ropenChoose o =>
      cases o with
      | wire =>
          obtain ⟨hL', hq, hoff, hon, hother⟩ :=
            step_ropenChoose_wire hb hL
          exact SInv.base_assemble_commit hm hL'
            (BaseFacts.of_quiet hslot hflow hq) rfl
            (fun h => by
              show (h == sk.rootH) = decide (h = sk.rootH)
              by_cases hh : h = sk.rootH <;> simp [hh])
            hoff hon hother
      | res =>
          obtain ⟨hL', hq, hh⟩ := step_ropenChoose_res hb hL
          exact SInv.base_assemble hm hL'
            (BaseFacts.of_quiet hslot hflow hq) hh (fun h => rfl)
      | query =>
          obtain ⟨hL', hq, hh⟩ := step_ropenChoose_query hb hL
          exact SInv.base_assemble hm hL'
            (BaseFacts.of_quiet hslot hflow hq) hh (fun h => rfl)
  | ropenFire =>
      have hch : s.base.ropenCh = some .res ∨ s.base.ropenCh = some .query := by
        rw [Model.apply] at hb
        cases hro : s.base.ropenCh with
        | none => rw [hro] at hb; cases hb
        | some o =>
            cases o with
            | wire =>
                exfalso
                rw [isWireFire, hro] at hnf
                simp at hnf
            | res => exact Or.inl rfl
            | query => exact Or.inr rfl
      rcases hch with hch | hch
      · obtain ⟨hL', hsend, hh⟩ := step_ropenFire_res hch hb hL
        exact SInv.base_assemble hm hL'
          (BaseFacts.of_send hslot hflow hsend rfl) hh (fun h => rfl)
      · obtain ⟨hL', hsend, hh⟩ := step_ropenFire_query hch hb hL
        exact SInv.base_assemble hm hL'
          (BaseFacts.of_send hslot hflow hsend rfl) hh (fun h => rfl)
  | walkRecvWire pk =>
      obtain ⟨hL', hr, hh⟩ := step_walkRecvWire hwf pk hb hL
      exact SInv.base_assemble hm hL'
        (BaseFacts.of_recv hslot hflow hr) hh (fun h => rfl)
  | walkRecvAsked pk =>
      obtain ⟨hL', hr, hh⟩ := step_walkRecvAsked hwf pk hb hL
      exact SInv.base_assemble hm hL'
        (BaseFacts.of_recv hslot hflow hr) hh (fun h => rfl)
  | walkCommit pk o =>
      cases o with
      | wire i =>
          obtain ⟨hL', hq, hmem, hoff, hon, hother⟩ :=
            step_walkCommit_wire hwf pk i hb hL
          refine SInv.base_assemble_commit (h₀ := pk.2) hm hL'
            (BaseFacts.of_quiet hslot hflow hq) rfl
            (fun h => by
              show (pk.2 == h) = decide (h = pk.2)
              by_cases hh : h = pk.2
              · subst hh
                simp
              · have hne : ¬pk.2 = h := fun hc => hh hc.symm
                simp [hne, hh])
            hoff hon ?_
          intro p h hne
          refine hother p h ?_
          intro hcon
          apply hne
          exact ⟨congrArg Prod.fst hcon, congrArg Prod.snd hcon⟩
      | res i =>
          obtain ⟨hL', hq, hh⟩ := step_walkCommit_res pk i hb hL
          exact SInv.base_assemble hm hL'
            (BaseFacts.of_quiet hslot hflow hq) hh (fun h => rfl)
      | query i =>
          obtain ⟨hL', hq, hh⟩ := step_walkCommit_query pk i hb hL
          exact SInv.base_assemble hm hL'
            (BaseFacts.of_quiet hslot hflow hq) hh (fun h => rfl)
      | parent =>
          obtain ⟨hL', hq, hh⟩ := step_walkCommit_parent pk hb hL
          exact SInv.base_assemble hm hL'
            (BaseFacts.of_quiet hslot hflow hq) hh (fun h => rfl)
  | walkFire pk =>
      -- decompose the fire; the wire obligation is barred by `hnf`
      simp only [Model.apply] at hb
      split at hb
      next o hcm =>
        split at hb
        case isFalse => cases hb
        case isTrue hg =>
          simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq]
            at hg
          obtain ⟨⟨hmem, hph2⟩, hlt1⟩ := hg
          have hmem' : pk ∈ sk.walkKeys := by simpa using hmem
          have hnw : ∀ i, o ≠ Oblig.wire i := by
            intro i hcon
            subst hcon
            rw [isWireFire, hcm] at hnf
            cases hnf
          injection hb with hbeq
          -- the chan-free core state and its facts
          obtain ⟨hL', hsent, hrecv, hchan, hoff, hhand'⟩ :=
            step_fire (s' := setWalk s.base pk
              (normWalk sk pk.2 (fireOblig (s.base.walk pk) o)))
              hwf pk o hmem' hph2 hcm rfl hL
          have hbt : b = { setWalk s.base pk
              (normWalk sk pk.2 (fireOblig (s.base.walk pk) o)) with
              chan := bump s.base.chan (obligChan pk o) 1 } := by
            rw [← hbeq]
            rfl
          have hL'' : InvL sk .impl b := by
            rw [hbt]
            exact InvL.chan_blind hL'
          have hcap1 : sk.cap (obligChan pk o) = 1 := by
            cases o with
            | wire i => exact absurd rfl (hnw i)
            | res i => rfl
            | query i =>
                show sk.cap (askedOut pk) = 1
                rw [askedOut]
                split <;> rfl
            | parent => rfl
          have hsend : SendStep sk s.base b (obligChan pk o) := by
            refine ⟨by omega, ?_, ?_, ?_⟩
            · rw [hbt]
            · intro c hc
              rw [hbt, sentOf_chan_blind]
              exact hsent c hc
            · intro c hc
              rw [hbt, recvdOf_chan_blind]
              exact hrecv c hc
          have hhands : ∀ p h, holdsWire sk p h b
              = holdsWire sk p h s.base := by
            intro p h
            rw [hbt, holdsWire_chan_blind]
            by_cases hpe : (p, h) = pk
            · have hroot : h ≠ sk.rootH := by
                have := walkKeys_height_lt hmem'
                intro hcon
                rw [← hpe] at this
                simp at this
                omega
              rw [holdsWire_eq_wireHand hroot,
                holdsWire_eq_wireHand hroot, hpe, hhand']
              have hwh : wireHand (s.base.walk pk) = false := by
                rw [wireHand, hcm]
                cases o with
                | wire i => exact absurd rfl (hnw i)
                | res i => simp
                | query i => simp
                | parent => simp
              rw [hwh]
            · exact hoff p h hpe
          exact SInv.base_assemble hm hL''
            (BaseFacts.of_send hslot hflow hsend
              (isWire_obligChan_nonwire pk hnw))
            hhands (fun h => rfl)
      next hcm => cases hb
  | walkCloseWire pk =>
      obtain ⟨hL', hq, hh⟩ := step_walkCloseWire hwf pk hb hL
      exact SInv.base_assemble hm hL'
        (BaseFacts.of_quiet hslot hflow hq) hh (fun h => rfl)
  | walkCloseAsked pk =>
      obtain ⟨hL', hq, hh⟩ := step_walkCloseAsked hwf pk hb hL
      exact SInv.base_assemble hm hL'
        (BaseFacts.of_quiet hslot hflow hq) hh (fun h => rfl)
  | asmRecvRes pk =>
      obtain ⟨hL', hr, hh⟩ := step_asmRecvRes hwf pk hb hL
      exact SInv.base_assemble hm hL'
        (BaseFacts.of_recv hslot hflow hr) hh (fun h => rfl)
  | asmRecvLevel pk =>
      obtain ⟨hL', hr, hh⟩ := step_asmRecvLevel hwf pk hb hL
      exact SInv.base_assemble hm hL'
        (BaseFacts.of_recv hslot hflow hr) hh (fun h => rfl)
  | asmSend pk =>
      obtain ⟨hL', hsend, hh⟩ := step_asmSend hwf pk hb hL
      refine SInv.base_assemble hm hL'
        (BaseFacts.of_send hslot hflow hsend ?_) hh (fun h => rfl)
      rw [Skel.asmOutChan]
      split
      · rfl
      · split <;> rfl
  | asmClose pk =>
      obtain ⟨hL', hq, hh⟩ := step_asmClose hwf pk hb hL
      exact SInv.base_assemble hm hL'
        (BaseFacts.of_quiet hslot hflow hq) hh (fun h => rfl)
  | absorbRecvWire =>
      obtain ⟨hL', hr, hh⟩ := step_absorbRecvWire hwf hb hL
      exact SInv.base_assemble hm hL'
        (BaseFacts.of_recv hslot hflow hr) hh (fun h => rfl)
  | absorbRecvAsked =>
      obtain ⟨hL', hr, hh⟩ := step_absorbRecvAsked hb hL
      exact SInv.base_assemble hm hL'
        (BaseFacts.of_recv hslot hflow hr) hh (fun h => rfl)
  | absorbSend =>
      obtain ⟨hL', hsend, hh⟩ := step_absorbSend hb hL
      exact SInv.base_assemble hm hL'
        (BaseFacts.of_send hslot hflow hsend rfl) hh (fun h => rfl)
  | absorbCloseWire =>
      obtain ⟨hL', hq, hh⟩ := step_absorbCloseWire hb hL
      exact SInv.base_assemble hm hL'
        (BaseFacts.of_quiet hslot hflow hq) hh (fun h => rfl)
  | absorbCloseAsked =>
      obtain ⟨hL', hq, hh⟩ := step_absorbCloseAsked hb hL
      exact SInv.base_assemble hm hL'
        (BaseFacts.of_quiet hslot hflow hq) hh (fun h => rfl)
  | finRet =>
      obtain ⟨hL', hr, hh⟩ := step_finRet hb hL
      exact SInv.base_assemble hm hL'
        (BaseFacts.of_recv hslot hflow hr) hh (fun h => rfl)
  | finRes =>
      obtain ⟨hL', hr, hh⟩ := step_finRes hb hL
      exact SInv.base_assemble hm hL'
        (BaseFacts.of_recv hslot hflow hr) hh (fun h => rfl)
  | finRets =>
      obtain ⟨hL', hr, hh⟩ := step_finRets hb hL
      exact SInv.base_assemble hm hL'
        (BaseFacts.of_recv hslot hflow hr) hh (fun h => rfl)

-- ==================================================== the push arm

private theorem party_other_ne (q : Party) : q.other ≠ q := by
  cases q <;> simp [Party.other]

/-- The push-side assembly: the flush receipt, the pipe append, and the
sender's cursor advance rebuild every field. `hoffd` is the hand flip
DOWN — the fired hand is cleared — and the guard's hand-was-held fact
balances the ledger. -/
theorem SInv.push_assemble {s : MState} {b : State} {p : Party} {h : Nat}
    (hm : SInv sk s) (hL' : InvL sk .impl b)
    (hmem_ch : Chan.wire p h ∈ allChans sk)
    (hchan : b.chan = s.base.chan)
    (hsw : ∀ q g, Chan.wire q g ∈ allChans sk →
      sentOf sk b (Chan.wire q g) = sentOf sk s.base (Chan.wire q g)
        + (if q = p ∧ g = h then 1 else 0))
    (hsint : ∀ c ∈ allChans sk, isWire c = false →
      sentOf sk b c = sentOf sk s.base c)
    (hrecv : ∀ c ∈ allChans sk, recvdOf sk b c = recvdOf sk s.base c)
    (hon : holdsWire sk p h s.base = true)
    (hoffd : holdsWire sk p h b = false)
    (hother : ∀ q g, ¬(q = p ∧ g = h) →
      holdsWire sk q g b = holdsWire sk q g s.base) :
    SInv sk { base := b
              pipe := fun q => if q == p
                then s.pipe q ++ [Chan.wire p h] else s.pipe q
              hist := recordObs s.hist p (.pushed h) } := by
  have hhist : ∀ q, recordObs s.hist p (.pushed h) q
      = if q == p then s.hist q ++ [.pushed h] else s.hist q := by
    intro q
    rfl
  have hnp : ∀ (g : Nat), (MObs.pushed h) ≠ .delivered g := by
    intro g hcon
    cases hcon
  have hlen : delTotal (s.hist p.other) ≤ (pushHeights (s.hist p)).length := by
    have := congrArg List.length (hm.mux.hist_del p)
    rw [List.length_take] at this
    have hdt : (delHeights (s.hist p.other)).length
        = delTotal (s.hist p.other) := rfl
    omega
  have hne_other : ∀ q : Party, q ≠ p → ¬((q == p) = true) := by
    intro q hq
    simp [hq]
  refine ⟨⟨hL', ?_, ?_, ?_, ?_, ?_, ?_, ?_⟩, ?_, ?_⟩
  · intro c hc
    rw [hchan]
    exact hm.mux.slot c hc
  · intro c hc hw
    rw [hchan, hsint c hc hw, hrecv c hc]
    exact hm.mux.flow_int c hc hw
  · -- pushed_eq
    intro q g hc
    rw [hsw q g hc]
    show pushedCount (recordObs s.hist p (.pushed h) q) g = _
    rw [hhist]
    by_cases hq : q = p
    · subst hq
      rw [if_pos (by simp), pushedCount_append_pushed,
        hm.mux.pushed_eq q g hc]
      by_cases hg : g = h
      · subst hg
        rw [if_pos rfl, if_pos ⟨rfl, rfl⟩]
      · rw [if_neg (fun hcon => hg hcon.symm),
          if_neg (fun hcon => hg hcon.2)]
    · rw [if_neg (hne_other q hq), hm.mux.pushed_eq q g hc,
        if_neg (fun hcon => hq hcon.1)]
      omega
  · -- hist_del
    intro q
    show delHeights (recordObs s.hist p (.pushed h) q.other)
      = (pushHeights (recordObs s.hist p (.pushed h) q)).take
          (delTotal (recordObs s.hist p (.pushed h) q.other))
    rw [hhist, hhist]
    by_cases hq : q = p
    · subst hq
      rw [if_neg (hne_other q.other (party_other_ne q)),
        if_pos (by simp)]
      rw [pushHeights_append]
      rw [List.take_append_of_le_length (by simpa using hlen)]
      exact hm.mux.hist_del q
    · by_cases hqo : q.other = p
      · rw [if_neg (hne_other q hq), if_pos (by simp [hqo])]
        rw [delHeights_append]
        simp only [List.append_nil]
        rw [show delTotal (s.hist q.other ++ [MObs.pushed h])
            = delTotal (s.hist q.other) from
          delTotal_append_other _ (fun g hcon => by cases hcon)]
        exact hm.mux.hist_del q
      · rw [if_neg (hne_other q hq), if_neg (hne_other q.other hqo)]
        exact hm.mux.hist_del q
  · -- hist_pipe
    intro q
    show (if q == p then s.pipe q ++ [Chan.wire p h] else s.pipe q)
      = ((pushHeights (recordObs s.hist p (.pushed h) q)).drop
          (delTotal (recordObs s.hist p (.pushed h) q.other))).map
          (Chan.wire q)
    rw [hhist, hhist]
    by_cases hq : q = p
    · subst hq
      rw [if_pos (by simp),
        if_neg (hne_other q.other (party_other_ne q)),
        if_pos (by simp)]
      rw [pushHeights_append]
      rw [List.drop_append_of_le_length (by simpa using hlen)]
      rw [List.map_append, hm.mux.hist_pipe q]
      rfl
    · by_cases hqo : q.other = p
      · rw [if_neg (hne_other q hq), if_pos (by simp [hqo]),
          if_neg (hne_other q hq)]
        rw [show delTotal (s.hist q.other ++ [MObs.pushed h])
            = delTotal (s.hist q.other) from
          delTotal_append_other _ (fun g hcon => by cases hcon)]
        exact hm.mux.hist_pipe q
      · rw [if_neg (hne_other q hq), if_neg (hne_other q.other hqo),
          if_neg (hne_other q hq)]
        exact hm.mux.hist_pipe q
  · -- delivered_eq
    intro q g hc
    rw [hchan, hrecv _ hc]
    show deliveredCount (recordObs s.hist p (.pushed h) q.other) g = _
    rw [hhist]
    by_cases hqo : q.other = p
    · rw [if_pos (by simp [hqo]),
        deliveredCount_append_other _ hnp]
      exact hm.mux.delivered_eq q g hc
    · rw [if_neg (hne_other q.other hqo)]
      exact hm.mux.delivered_eq q g hc
  · -- pushed_mem
    intro q g hne
    revert hne
    show pushedCount (recordObs s.hist p (.pushed h) q) g ≠ 0 → _
    rw [hhist]
    by_cases hq : q = p
    · subst hq
      rw [if_pos (by simp), pushedCount_append_pushed]
      intro hne
      by_cases hg : h = g
      · subst hg
        exact hmem_ch
      · rw [if_neg hg] at hne
        exact hm.mux.pushed_mem q g (by omega)
    · rw [if_neg (hne_other q hq)]
      exact hm.mux.pushed_mem q g
  · -- hist_party
    intro q a' hmem
    have hmem' : MObs.act a' ∈ recordObs s.hist p (.pushed h) q := hmem
    rw [hhist] at hmem'
    by_cases hq : q = p
    · subst hq
      rw [if_pos (by simp)] at hmem'
      rcases List.mem_append.mp hmem' with hold | hnew
      · exact hm.hist.hist_party q a' hold
      · have := List.mem_singleton.mp hnew
        cases this
    · rw [if_neg (hne_other q hq)] at hmem'
      exact hm.hist.hist_party q a' hmem'
  · -- hand_count
    intro q g
    show commitsOf sk.rootH (recordObs s.hist p (.pushed h) q) g
      = pushesOf (recordObs s.hist p (.pushed h) q) g + _
    rw [hhist]
    have hold := hm.hist.hand_count q g
    by_cases hq : q = p
    · subst hq
      rw [if_pos (by simp),
        commitsOf_append_other _ _ (fun a hcon => by cases hcon),
        pushesOf_append]
      by_cases hg : g = h
      · subst hg
        rw [hon, if_pos rfl] at hold
        rw [hoffd, if_neg Bool.false_ne_true]
        have hms : (match (MObs.pushed g : MObs) with
            | .pushed h' => if h' = g then 1 else 0
            | _ => 0) = 1 := by
          simp
        omega
      · rw [hother q g (fun hcon => hg hcon.2)]
        have : (match (MObs.pushed h : MObs) with
            | .pushed h' => if h' = g then 1 else 0
            | _ => 0) = 0 := by
          simp only
          rw [if_neg (fun hcon => hg hcon.symm)]
        omega
    · rw [if_neg (hne_other q hq),
        hother q g (fun hcon => hq hcon.1)]
      exact hold

/-- The initiator's opening fire, as base facts: only `iopenWire` and
the choice slot move, so the root wire's producer count rises by one
and everything else frames.

Public (not `private`): the elastic preservation sweep
(Mux/Elastic.lean, the stage-F obligation's elastic twin) consumes the
same opener-fire facts for its push arm. -/
theorem iopen_fire_facts {s₀ : State}
    (hch : s₀.iopenCh = some .wire) (hL : InvL sk ax s₀) :
    InvL sk ax { s₀ with iopenWire := true, iopenCh := none }
    ∧ (∀ q g, sentOf sk { s₀ with iopenWire := true, iopenCh := none }
        (Chan.wire q g) = sentOf sk s₀ (Chan.wire q g)
          + (if q = Party.I ∧ g = sk.rootH then 1 else 0))
    ∧ (∀ c, isWire c = false →
        sentOf sk { s₀ with iopenWire := true, iopenCh := none } c
          = sentOf sk s₀ c)
    ∧ (∀ c, recvdOf sk { s₀ with iopenWire := true, iopenCh := none } c
        = recvdOf sk s₀ c)
    ∧ holdsWire sk Party.I sk.rootH s₀ = true
    ∧ holdsWire sk Party.I sk.rootH
        { s₀ with iopenWire := true, iopenCh := none } = false
    ∧ (∀ q g, ¬(q = Party.I ∧ g = sk.rootH) →
        holdsWire sk q g { s₀ with iopenWire := true, iopenCh := none }
          = holdsWire sk q g s₀) := by
  have htop := hL.top
  rw [topLocalOk] at htop
  simp only [Bool.and_eq_true] at htop
  have hiw : s₀.iopenWire = false := by
    have h1 := htop.1.1.1.1.1.1.1.1.1.1.1
    rw [hch] at h1
    simpa using h1
  refine ⟨⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_⟩, ?_, ?_, ?_, ?_, ?_, ?_⟩
  · rw [wkLocalOk_congr sk ax pk rfl]
    exact hL.wk pk hpk
  · rw [asmLocalOk_congr sk pk rfl]
    exact hL.asm pk hpk
  · have htop' := hL.top
    rw [topLocalOk] at htop'
    simp only [Bool.and_eq_true] at htop'
    obtain ⟨⟨⟨⟨⟨⟨⟨⟨⟨⟨⟨-, -⟩, h3⟩, h4⟩, h5⟩, h6⟩, h7⟩, h8⟩, h9⟩, h10⟩,
      h11⟩, h12⟩ := htop'
    rw [topLocalOk]
    simp only [Bool.and_eq_true]
    exact ⟨⟨⟨⟨⟨⟨⟨⟨⟨⟨⟨by simp, by simp⟩, h3⟩, h4⟩, h5⟩, h6⟩, h7⟩, h8⟩,
      h9⟩, h10⟩, h11⟩, h12⟩
  · intro q g
    by_cases hqg : q = Party.I ∧ g = sk.rootH
    · obtain ⟨rfl, rfl⟩ := hqg
      rw [if_pos ⟨rfl, rfl⟩]
      simp [sentOf, b2n, hiw]
    · rw [if_neg hqg]
      by_cases hg : g = sk.rootH
      · have hq : q = Party.R := by
          cases q
          · exact absurd ⟨rfl, hg⟩ hqg
          · rfl
        subst hq
        subst hg
        simp [sentOf, b2n]
      · simp [sentOf, hg, wkWireSent, wkWireCount]
  · intro c hw
    cases c with
    | wire p h => simp [isWire] at hw
    | _ => rfl
  · intro c
    cases c <;> rfl
  · rw [holdsWire.eq_def, if_pos (by simp)]
    simp [hch]
  · rw [holdsWire.eq_def, if_pos (by simp)]
    simp
  · intro q g hqg
    by_cases hg : g = sk.rootH
    · have hq : q = Party.R := by
        cases q
        · exact absurd ⟨rfl, hg⟩ hqg
        · rfl
      subst hq
      subst hg
      rw [holdsWire.eq_def, holdsWire.eq_def]
    · rw [holdsWire_eq_wireHand hg, holdsWire_eq_wireHand hg]

/-- The responder's opening fire, as base facts.

Public for the same consumer as `iopen_fire_facts`. -/
theorem ropen_fire_facts {s₀ : State}
    (hch : s₀.ropenCh = some .wire) (hL : InvL sk ax s₀) :
    InvL sk ax { s₀ with ropenWire := true, ropenCh := none }
    ∧ (∀ q g, sentOf sk { s₀ with ropenWire := true, ropenCh := none }
        (Chan.wire q g) = sentOf sk s₀ (Chan.wire q g)
          + (if q = Party.R ∧ g = sk.rootH then 1 else 0))
    ∧ (∀ c, isWire c = false →
        sentOf sk { s₀ with ropenWire := true, ropenCh := none } c
          = sentOf sk s₀ c)
    ∧ (∀ c, recvdOf sk { s₀ with ropenWire := true, ropenCh := none } c
        = recvdOf sk s₀ c)
    ∧ holdsWire sk Party.R sk.rootH s₀ = true
    ∧ holdsWire sk Party.R sk.rootH
        { s₀ with ropenWire := true, ropenCh := none } = false
    ∧ (∀ q g, ¬(q = Party.R ∧ g = sk.rootH) →
        holdsWire sk q g { s₀ with ropenWire := true, ropenCh := none }
          = holdsWire sk q g s₀) := by
  have htop := hL.top
  rw [topLocalOk] at htop
  simp only [Bool.and_eq_true] at htop
  have hrw : s₀.ropenWire = false := by
    have h1 := htop.1.1.1.1.1.1.2
    rw [hch] at h1
    simpa using h1
  have hgw : s₀.ropenGotWire = true := by
    have h3 := htop.1.1.1.1.1.1.1.1.1.2
    rw [hch] at h3
    simpa using h3
  refine ⟨⟨fun pk hpk => ?_, fun pk hpk => ?_, ?_⟩, ?_, ?_, ?_, ?_, ?_, ?_⟩
  · rw [wkLocalOk_congr sk ax pk rfl]
    exact hL.wk pk hpk
  · rw [asmLocalOk_congr sk pk rfl]
    exact hL.asm pk hpk
  · have htop' := hL.top
    rw [topLocalOk] at htop'
    simp only [Bool.and_eq_true] at htop'
    obtain ⟨⟨⟨⟨⟨⟨⟨⟨⟨⟨⟨h1, h2⟩, -⟩, h4⟩, -⟩, -⟩, -⟩, -⟩, h9⟩, h10⟩,
      h11⟩, h12⟩ := htop'
    rw [topLocalOk]
    simp only [Bool.and_eq_true]
    exact ⟨⟨⟨⟨⟨⟨⟨⟨⟨⟨⟨h1, h2⟩, by simp [hgw]⟩, h4⟩, by simp⟩, by simp⟩,
      by simp⟩, by simp⟩, h9⟩, h10⟩, h11⟩, h12⟩
  · intro q g
    by_cases hqg : q = Party.R ∧ g = sk.rootH
    · obtain ⟨rfl, rfl⟩ := hqg
      rw [if_pos ⟨rfl, rfl⟩]
      simp [sentOf, b2n, hrw]
    · rw [if_neg hqg]
      by_cases hg : g = sk.rootH
      · have hq : q = Party.I := by
          cases q
          · rfl
          · exact absurd ⟨rfl, hg⟩ hqg
        subst hq
        subst hg
        simp [sentOf, b2n]
      · simp [sentOf, hg, wkWireSent, wkWireCount]
  · intro c hw
    cases c with
    | wire p h => simp [isWire] at hw
    | _ => rfl
  · intro c
    cases c <;> rfl
  · rw [holdsWire.eq_def, if_pos (by simp)]
    simp [hch]
  · rw [holdsWire.eq_def, if_pos (by simp)]
    simp
  · intro q g hqg
    by_cases hg : g = sk.rootH
    · have hq : q = Party.I := by
        cases q
        · rfl
        · exact absurd ⟨rfl, hg⟩ hqg
      subst hq
      subst hg
      rw [holdsWire.eq_def, holdsWire.eq_def]
    · rw [holdsWire_eq_wireHand hg, holdsWire_eq_wireHand hg]

/-- A successful push preserves the invariant, and its only history
effect is the flush receipt. -/
theorem sinv_firePush (hwf : sk.wellFormed = true) {C : Nat} {p : Party}
    {h : Nat} {s s' : MState}
    (hfp : firePush sk C p h s = some s') (hm : SInv sk s) :
    SInv sk s' ∧ s'.hist = recordObs s.hist p (.pushed h) := by
  simp only [firePush] at hfp
  split at hfp
  case isFalse => cases hfp
  case isTrue hroom =>
    split at hfp
    · -- the opening stream
      next hr =>
      have hr' : h = sk.rootH := by simpa using hr
      subst hr'
      cases p with
      | I =>
          cases hch : s.base.iopenCh with
          | none => rw [hch] at hfp; cases hfp
          | some o =>
              cases o with
              | query => rw [hch] at hfp; cases hfp
              | wire =>
                  rw [hch] at hfp
                  injection hfp with hs'
                  obtain ⟨hL', hsw, hsint, hrecv, hon, hoffd, hother⟩ :=
                    iopen_fire_facts hch hm.mux.invl
                  subst hs'
                  refine ⟨SInv.push_assemble hm hL'
                    (mem_allChans_wire_root _) rfl
                    (fun q g _ => hsw q g)
                    (fun c _ hw => hsint c hw)
                    (fun c _ => hrecv c) hon hoffd hother, rfl⟩
      | R =>
          cases hch : s.base.ropenCh with
          | none => rw [hch] at hfp; cases hfp
          | some o =>
              cases o with
              | query => rw [hch] at hfp; cases hfp
              | res => rw [hch] at hfp; cases hfp
              | wire =>
                  rw [hch] at hfp
                  injection hfp with hs'
                  obtain ⟨hL', hsw, hsint, hrecv, hon, hoffd, hother⟩ :=
                    ropen_fire_facts hch hm.mux.invl
                  subst hs'
                  refine ⟨SInv.push_assemble hm hL'
                    (mem_allChans_wire_root _) rfl
                    (fun q g _ => hsw q g)
                    (fun c _ hw => hsint c hw)
                    (fun c _ => hrecv c) hon hoffd hother, rfl⟩
    · -- a walk stream
      next hr =>
      have hr' : h ≠ sk.rootH := by simpa using hr
      split at hfp
      next i hcm =>
        split at hfp
        case isFalse => cases hfp
        case isTrue hg =>
          simp only [Bool.and_eq_true] at hg
          obtain ⟨hcon, hph⟩ := hg
          have hmem' : (p, h) ∈ sk.walkKeys :=
            (List.contains_iff_mem ..).mp hcon
          have hph2 : (s.base.walk (p, h)).phase = 2 := by
            simpa using hph
          injection hfp with hs'
          obtain ⟨hL', hsent, hrecv, hchan, hoff, hhand'⟩ :=
            step_fire (s' := setWalk s.base (p, h)
              (normWalk sk h (fireOblig (s.base.walk (p, h))
                (.wire i))))
            hwf (p, h) (.wire i) hmem' hph2 hcm rfl hm.mux.invl
          subst hs'
          refine ⟨SInv.push_assemble hm hL'
            (mem_allChans_wireOut hmem') hchan ?_ ?_
            (fun c hc => hrecv c hc) ?_ ?_ ?_, rfl⟩
          · intro q g hc
            rw [hsent _ hc]
            congr 1
            rw [show obligChan (p, h) (Oblig.wire i) = Chan.wire p h
              from rfl]
            by_cases hqg : q = p ∧ g = h
            · obtain ⟨rfl, rfl⟩ := hqg
              rw [if_pos rfl, if_pos ⟨rfl, rfl⟩]
            · have hne : Chan.wire q g ≠ Chan.wire p h := by
                intro hcon2
                apply hqg
                have h1 := congrArg wireParty hcon2
                have h2 := congrArg wireHeight hcon2
                exact ⟨h1, h2⟩
              rw [if_neg hne, if_neg hqg]
          · intro c hc hw
            have hne : c ≠ obligChan (p, h) (Oblig.wire i) := by
              intro hcon2
              rw [hcon2] at hw
              simp [isWire, obligChan, wireOut] at hw
            rw [hsent _ hc, if_neg hne]
            omega
          · rw [holdsWire_eq_wireHand hr']
            rw [hcon]
            rw [wireHand]
            rw [hph, hcm]
            rfl
          · rw [holdsWire_eq_wireHand hr']
            have hwk : (setWalk s.base (p, h)
                (normWalk sk h (fireOblig (s.base.walk (p, h))
                  (.wire i)))).walk (p, h)
                = normWalk sk h (fireOblig (s.base.walk (p, h))
                  (.wire i)) := setWalk_walk_self _ _ _
            have := hhand'
            rw [this]
            simp
          · intro q g hqg
            refine hoff q g ?_
            intro hcon2
            apply hqg
            exact ⟨congrArg Prod.fst hcon2, congrArg Prod.snd hcon2⟩
      next hcm => cases hfp

/-- Either the same party or the other: the two-point case split. -/
private theorem party_cases (q p : Party) : q = p ∨ q = p.other := by
  cases q <;> cases p <;> simp [Party.other]

/-- A delivery preserves the invariant: the FIFO head moves to its
slot, and the receipt extends the delivered prefix by exactly that
frame. -/
theorem sinv_deliver {C : Nat} {σI σR : Strategy} {p : Party}
    {s s' : MState}
    (hstep : apply sk .impl C σI σR (.deliver p) s = some s')
    (hm : SInv sk s) : SInv sk s' := by
  simp only [apply] at hstep
  split at hstep
  case h_2 => cases hstep
  case h_1 c rest hp =>
      split at hstep
      next h0 =>
        have hz : s.base.chan c = 0 := by simpa using h0
        obtain ⟨g, rfl⟩ := hm.mux.pipe_mem_wire (p := p)
          (c := c) (by rw [hp]; exact List.mem_cons_self ..)
        injection hstep with hs'
        subst hs'
        -- the head frame's position in the push order
        have hpipe := hm.mux.hist_pipe p
        rw [hp] at hpipe
        have hdrop : (pushHeights (s.hist p)).drop
            (delTotal (s.hist p.other))
            = g :: (rest.map wireHeight) := by
          have h1 := congrArg (List.map wireHeight) hpipe
          rw [List.map_map] at h1
          rw [show wireHeight ∘ Chan.wire p = id from rfl,
            List.map_id] at h1
          simpa [wireHeight] using h1.symm
        have hget : (pushHeights (s.hist p))[delTotal (s.hist p.other)]?
            = some g := by
          have h1 : ((pushHeights (s.hist p)).drop
              (delTotal (s.hist p.other)))[0]? = some g := by
            rw [hdrop]
            rfl
          rw [List.getElem?_drop] at h1
          simpa using h1
        have hpo : p ≠ p.other := fun hcon => party_other_ne p hcon.symm
        have hoo : p.other.other = p := Party.other_other p
        have hno_a : ∀ (a : Action),
            (MObs.delivered g : MObs) ≠ .act a := fun a hcon => by
          cases hcon
        have hno_p2 : ∀ (h' : Nat),
            (MObs.delivered g : MObs) ≠ .pushed h' := fun h' hcon => by
          cases hcon
        have hhist : ∀ q, recordObs s.hist p.other (.delivered g) q
            = if q == p.other then s.hist q ++ [.delivered g]
              else s.hist q := fun q => rfl
        have hself : (p.other == p.other) = true := by simp
        have hnp : (p == p.other) = false := by
          cases hb : (p == p.other)
          · rfl
          · exact absurd (beq_iff_eq.mp hb) hpo
        refine ⟨⟨InvL.chan_blind hm.mux.invl, ?_, ?_, ?_, ?_, ?_, ?_, ?_⟩,
          ?_, ?_⟩
        · -- slot
          intro c' hc'
          show bump s.base.chan (Chan.wire p g) 1 c' ≤ _
          by_cases he : c' = Chan.wire p g
          · subst he
            rw [bump_one, hz]
            show 1 ≤ sk.cap (Chan.wire p g)
            exact Nat.le_refl _
          · rw [bump_ne _ _ he]
            exact hm.mux.slot c' hc'
        · -- flow_int
          intro c' hc' hw
          show bump s.base.chan (Chan.wire p g) 1 c'
              + recvdOf sk _ c' = sentOf sk _ c'
          have hne : c' ≠ Chan.wire p g := by
            intro he
            rw [he] at hw
            simp [isWire] at hw
          rw [bump_ne _ _ hne, recvdOf_chan_blind, sentOf_chan_blind]
          exact hm.mux.flow_int c' hc' hw
        · -- pushed_eq
          intro q g' hc'
          show pushedCount (recordObs s.hist p.other (.delivered g) q) g'
            = _
          rw [hhist, sentOf_chan_blind]
          by_cases hq : q = p.other
          · rw [if_pos (by simp [hq]),
              pushedCount_append_other _ hno_p2]
            exact hm.mux.pushed_eq q g' hc'
          · rw [if_neg (by simp [hq])]
            exact hm.mux.pushed_eq q g' hc'
        · -- hist_del
          intro q
          show delHeights (recordObs s.hist p.other (.delivered g) q.other)
            = (pushHeights (recordObs s.hist p.other (.delivered g) q)).take
                (delTotal (recordObs s.hist p.other (.delivered g) q.other))
          rw [hhist, hhist]
          rcases party_cases q p with rfl | rfl
          · rw [if_pos hself, if_neg (by rw [hnp]; simp)]
            rw [delHeights_append]
            show delHeights (s.hist q.other) ++ [g] = _
            rw [show delTotal (s.hist q.other ++ [MObs.delivered g])
                = delTotal (s.hist q.other) + 1 from
              delTotal_append_delivered _ g]
            rw [List.take_add_one, hget, hm.mux.hist_del q]
            rfl
          · rw [hoo, if_neg (by rw [hnp]; simp), if_pos hself]
            have hpe : pushHeights (s.hist p.other ++ [MObs.delivered g])
                = pushHeights (s.hist p.other) := by
              rw [pushHeights_append]
              simp
            rw [hpe]
            have := hm.mux.hist_del p.other
            rw [hoo] at this
            exact this
        · -- hist_pipe
          intro q
          show (if q == p then rest else s.pipe q)
            = ((pushHeights (recordObs s.hist p.other (.delivered g) q)).drop
                (delTotal (recordObs s.hist p.other (.delivered g) q.other))).map
                (Chan.wire q)
          rw [hhist, hhist]
          rcases party_cases q p with rfl | rfl
          · rw [if_pos (by simp), if_pos hself,
              if_neg (by rw [hnp]; simp)]
            rw [show delTotal (s.hist q.other ++ [MObs.delivered g])
                = delTotal (s.hist q.other) + 1 from
              delTotal_append_delivered _ g]
            have hdd : (pushHeights (s.hist q)).drop
                (delTotal (s.hist q.other) + 1)
                = rest.map wireHeight := by
              have h2 : (pushHeights (s.hist q)).drop
                  (delTotal (s.hist q.other) + 1)
                  = ((pushHeights (s.hist q)).drop
                      (delTotal (s.hist q.other))).drop 1 := by
                rw [List.drop_drop]
              rw [h2, hdrop]
              rfl
            rw [hdd]
            have hrest : rest.map (Chan.wire q ∘ wireHeight)
                = rest := by
              have hmem : ∀ c' ∈ rest, Chan.wire q (wireHeight c') = c' := by
                intro c' hc'
                obtain ⟨gg, rfl⟩ := hm.mux.pipe_mem_wire (p := q)
                  (c := c') (by rw [hp]; exact List.mem_cons_of_mem _ hc')
                rfl
              calc rest.map (Chan.wire q ∘ wireHeight)
                  = rest.map id := List.map_congr_left
                    (fun c' hc' => hmem c' hc')
                _ = rest := List.map_id rest
            rw [List.map_map, hrest]
          · rw [hoo]
            rw [if_neg (by
              intro hcon
              exact hpo (beq_iff_eq.mp hcon).symm), if_pos hself,
              if_neg (by rw [hnp]; simp)]
            have hpe : pushHeights (s.hist p.other ++ [MObs.delivered g])
                = pushHeights (s.hist p.other) := by
              rw [pushHeights_append]
              simp
            rw [hpe]
            have := hm.mux.hist_pipe p.other
            rw [hoo] at this
            exact this
        · -- delivered_eq
          intro q g' hc'
          show deliveredCount (recordObs s.hist p.other
            (.delivered g) q.other) g'
            = recvdOf sk _ (Chan.wire q g')
              + bump s.base.chan (Chan.wire p g) 1 (Chan.wire q g')
          rw [hhist, recvdOf_chan_blind]
          rcases party_cases q p with rfl | rfl
          · rw [if_pos hself, deliveredCount_append_delivered]
            by_cases hg' : g = g'
            · subst hg'
              rw [if_pos rfl, bump_one]
              have := hm.mux.delivered_eq q g hc'
              omega
            · rw [if_neg hg', bump_ne _ _ (by
                intro hcon
                injection hcon with h1 h2
                exact hg' h2.symm)]
              have := hm.mux.delivered_eq q g' hc'
              omega
          · rw [hoo, if_neg (by rw [hnp]; simp), bump_ne _ _ (by
              intro hcon
              injection hcon with h1 h2
              exact party_other_ne p (by rw [h1]))]
            have := hm.mux.delivered_eq p.other g' hc'
            rw [hoo] at this
            exact this
        · -- pushed_mem
          intro q g' hne
          revert hne
          show pushedCount (recordObs s.hist p.other
            (.delivered g) q) g' ≠ 0 → _
          rw [hhist]
          by_cases hq : q = p.other
          · rw [if_pos (by simp [hq]),
              pushedCount_append_other _ hno_p2]
            exact hm.mux.pushed_mem q g'
          · rw [if_neg (by simp [hq])]
            exact hm.mux.pushed_mem q g'
        · -- hist_party
          intro q a' hmem
          have hmem' : MObs.act a'
              ∈ recordObs s.hist p.other (.delivered g) q := hmem
          rw [hhist] at hmem'
          by_cases hq : q = p.other
          · rw [if_pos (by simp [hq])] at hmem'
            rcases List.mem_append.mp hmem' with hold | hnew
            · exact hm.hist.hist_party q a' hold
            · cases List.mem_singleton.mp hnew
          · rw [if_neg (by simp [hq])] at hmem'
            exact hm.hist.hist_party q a' hmem'
        · -- hand_count
          intro q g'
          show commitsOf sk.rootH (recordObs s.hist p.other
            (.delivered g) q) g'
            = pushesOf (recordObs s.hist p.other (.delivered g) q) g' + _
          rw [hhist, show holdsWire sk q g'
              { s.base with chan := bump s.base.chan (Chan.wire p g) 1 }
              = holdsWire sk q g' s.base from holdsWire_chan_blind _ q g']
          by_cases hq : q = p.other
          · rw [if_pos (by simp [hq]),
              commitsOf_append_other _ _ hno_a, pushesOf_append]
            have := hm.hist.hand_count q g'
            omega
          · rw [if_neg (by simp [hq])]
            exact hm.hist.hand_count q g'
      next => cases hstep

/-- The push arm, decomposed to the strategy verdict. -/
theorem sinv_push (hwf : sk.wellFormed = true) {C : Nat}
    {σI σR : Strategy} {p : Party} {s s' : MState}
    (hstep : apply sk .impl C σI σR (.push p) s = some s')
    (hm : SInv sk s) :
    SInv sk s'
    ∧ ∃ h, (match p with | .I => σI | .R => σR) sk (s.hist p) = some h
        ∧ s'.hist = recordObs s.hist p (.pushed h) := by
  cases p with
  | I =>
      simp only [apply] at hstep
      cases hσ : σI sk (s.hist .I) with
      | none => rw [hσ] at hstep; cases hstep
      | some h =>
          rw [hσ] at hstep
          obtain ⟨hs, hh⟩ := sinv_firePush hwf hstep hm
          exact ⟨hs, h, hσ, hh⟩
  | R =>
      simp only [apply] at hstep
      cases hσ : σR sk (s.hist .R) with
      | none => rw [hσ] at hstep; cases hstep
      | some h =>
          rw [hσ] at hstep
          obtain ⟨hs, hh⟩ := sinv_firePush hwf hstep hm
          exact ⟨hs, h, hσ, hh⟩

/-- Every muxed step preserves the strategy-generic invariant. -/
theorem sinv_step (hwf : sk.wellFormed = true) {C : Nat}
    {σI σR : Strategy} {ma : MAction} {s s' : MState}
    (hstep : apply sk .impl C σI σR ma s = some s')
    (hm : SInv sk s) : SInv sk s' := by
  cases ma with
  | base a => exact sinv_base hwf hstep hm
  | push p => exact (sinv_push hwf hstep hm).1
  | deliver p => exact sinv_deliver hstep hm

/-- No committed wire hand exists at the initial state. -/
private theorem holdsWire_init (p : Party) (h : Nat) :
    holdsWire sk p h (init sk).base = false := by
  rw [holdsWire.eq_def]
  split
  · cases p <;> rfl
  · show (sk.walkKeys.contains (p, h)
      && ((Model.init sk).walk (p, h)).phase == 2
      && _) = false
    have hc : ((init sk).base.walk (p, h)).committed = none := by
      show (freshWalk sk h 0).committed = none
      rw [freshWalk]
    rw [hc]
    simp

/-- The strategy-generic invariant holds at every reachable muxed
state — the stage-3 preservation induction, discharged. -/
theorem sinv_reachable (hwf : sk.wellFormed = true) {C : Nat}
    {σI σR : Strategy} {s : MState}
    (hr : MReachable sk .impl C σI σR s) : SInv sk s := by
  induction hr with
  | init =>
      refine ⟨muxInv_init sk, ?_, ?_⟩
      · intro p a hmem
        cases hmem
      · intro p h
        rw [show (init sk).hist p = [] from rfl, holdsWire_init]
        rfl
  | step a hr' hstep ih => exact sinv_step hwf hstep ih

-- ================================================ σ*'s push certificates

/-- Extending a history by one observation keeps every push
certificate, provided the new observation carries its own. -/
private theorem certs_snoc {p : Party} {tr : List MObs} {o : MObs}
    (hcert : ∀ i h, tr[i]? = some (.pushed h) →
      pushedCount (tr.take i) h ≠ 0 →
      (Chan.wire p h, false, pushedCount (tr.take i) h - 1)
        ∈ inevitable sk p (tr.take i))
    (hnew : ∀ h, o = .pushed h → pushedCount tr h ≠ 0 →
      (Chan.wire p h, false, pushedCount tr h - 1)
        ∈ inevitable sk p tr) :
    ∀ i h, (tr ++ [o])[i]? = some (.pushed h) →
      pushedCount ((tr ++ [o]).take i) h ≠ 0 →
      (Chan.wire p h, false, pushedCount ((tr ++ [o]).take i) h - 1)
        ∈ inevitable sk p ((tr ++ [o]).take i) := by
  intro i h hget hcnt
  rcases Nat.lt_trichotomy i tr.length with hlt | heq | hgt
  · rw [List.getElem?_append_left hlt] at hget
    rw [List.take_append_of_le_length (Nat.le_of_lt hlt)] at hcnt ⊢
    exact hcert i h hget hcnt
  · subst heq
    rw [List.getElem?_concat_length] at hget
    injection hget with hget
    rw [List.take_append_of_le_length (Nat.le_refl _),
      List.take_length] at hcnt ⊢
    exact hnew h hget hcnt
  · rw [List.getElem?_eq_none (by
      rw [List.length_append, List.length_cons, List.length_nil]
      omega)] at hget
    cases hget

/-- A history-only view of the certificates, per party. -/
private theorem pushProven_iff {s : MState} :
    PushProven sk s ↔ ∀ p, ∀ i h, (s.hist p)[i]? = some (.pushed h) →
      pushedCount ((s.hist p).take i) h ≠ 0 →
      (Chan.wire p h, false, pushedCount ((s.hist p).take i) h - 1)
        ∈ inevitable sk p ((s.hist p).take i) := by
  constructor
  · intro hp p i h
    exact hp p i h
  · intro hp p i h
    exact hp p i h

/-- Every σ*-run step preserves the push certificates: base actions and
deliveries append non-push observations, and a push observation carries
the demand proof σ* itself computed. -/
theorem pushProven_step (hwf : sk.wellFormed = true) {C : Nat}
    {ma : MAction} {s s' : MState}
    (hstep : apply sk .impl C sigmaStar sigmaStar ma s = some s')
    (hm : SInv sk s) (hp : PushProven sk s) : PushProven sk s' := by
  have hgen : ∀ (q₀ : Party) (o : MObs),
      (∀ h, o = .pushed h → pushedCount (s.hist q₀) h ≠ 0 →
        (Chan.wire q₀ h, false, pushedCount (s.hist q₀) h - 1)
          ∈ inevitable sk q₀ (s.hist q₀)) →
      s'.hist = recordObs s.hist q₀ o →
      PushProven sk s' := by
    intro q₀ o hnew hh
    have hrec : ∀ q, s'.hist q
        = if q == q₀ then s.hist q ++ [o] else s.hist q := by
      intro q
      rw [hh]
      rfl
    intro q i h
    rw [hrec]
    by_cases hq : q = q₀
    · subst hq
      rw [if_pos (by simp)]
      exact certs_snoc (hp q) hnew i h
    · rw [if_neg (by simp [hq])]
      exact hp q i h
  cases ma with
  | base a =>
      obtain ⟨-, b, -, hs'⟩ := applyBase_inv hstep
      refine hgen (actionParty a) (.act a) ?_ (by rw [hs'])
      intro h hcon
      cases hcon
  | deliver p =>
      -- reuse the deliver decomposition only for the history shape
      simp only [apply] at hstep
      split at hstep
      case h_2 => cases hstep
      case h_1 c rest hpp =>
          split at hstep
          case isFalse => cases hstep
          case isTrue h0 =>
            injection hstep with hs'
            refine hgen p.other (.delivered (wireHeight c)) ?_
              (by rw [← hs'])
            intro h hcon
            cases hcon
  | push p =>
      obtain ⟨-, h, hσ, hh⟩ := sinv_push hwf hstep hm
      have hσ' : sigmaStar sk (s.hist p) = some h := by
        cases p <;> exact hσ
      refine hgen p (.pushed h) ?_ hh
      intro h' heq hcnt
      have heq' : h = h' := by
        injection heq
      subst heq'
      obtain ⟨p₀, hpo, -, -, hdem⟩ := sigmaStar_some_inv hσ'
      have hpe : p₀ = p := partyOf_eq hm.hist hpo
      subst hpe
      rw [demanded, Bool.or_eq_true] at hdem
      rcases hdem with hz | hmem
      · exfalso
        rw [Nat.beq_eq_true_eq] at hz
        exact hcnt hz
      · exact (List.contains_iff_mem ..).mp hmem

/-- σ*'s push certificates hold along every σ*×σ* run: INV-A of
refute-c1 §2.1, kernel form. -/
theorem pushProven_reachable (hwf : sk.wellFormed = true) {C : Nat}
    {s : MState}
    (hr : MReachable sk .impl C sigmaStar sigmaStar s) :
    PushProven sk s := by
  induction hr with
  | init =>
      intro p i h hget
      rw [show (init sk).hist p = [] from rfl] at hget
      cases i <;> cases hget
  | step a hr' hstep ih =>
      exact pushProven_step hwf hstep (sinv_reachable hwf hr') ih

end StreamingMirror.Mux
