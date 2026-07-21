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

end StreamingMirror.Mux
