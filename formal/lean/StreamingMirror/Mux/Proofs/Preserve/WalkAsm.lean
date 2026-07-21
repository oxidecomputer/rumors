/-
Per-arm extraction of the walk/assembler preservation monoliths for the
muxed induction (stage-3 track E). A muxed state does not satisfy the
unmuxed conservation law (`InvP.flow` — frames ride the pipe), so the
monoliths of Proofs/Preserve/{Walk,Asm}.lean cannot be applied along a
muxed run. This file splits each of their arms in two:

- `preserveL_<X>`: the guard-inversion prelude plus the wk/asm/top
  bullets, verbatim — `InvL → InvL`, no use of `hi.flow`.
- `delta_<X>`: the flow bullet's `hchan`/`hsent`/`hrecv` computations,
  reassembled into the five-conjunct counting-delta shape the `MuxInv`
  induction consumes (wire sends framed, wire sums conserved, wire
  occupancy non-increasing, internal flow preserved, slots bounded).

Arms covered: `walkCommit`, `walkRecvWire`, `walkRecvAsked`,
`walkCloseWire`, `walkCloseAsked` (from Preserve/Walk.lean) and
`asmRecvRes`, `asmRecvLevel`, `asmSend`, `asmClose` (from
Preserve/Asm.lean).
-/
import StreamingMirror.Mux.Basic
import StreamingMirror.Proofs.Preserve

namespace StreamingMirror.Mux

open Model

variable {sk : Skel} {ax : AxMode} {s s' : State}

-- ======================================================= channel shape

/-- An internal channel is never a walk's wire input. -/
private theorem ne_wireIn {c : Chan} (hnw : isWire c = false)
    (pk : Party × Nat) : c ≠ wireIn pk := by
  intro hce
  rw [hce] at hnw
  simp [wireIn, isWire] at hnw

/-- A wire channel is never a walk's query input. -/
private theorem wire_ne_askedIn (p : Party) (h : Nat)
    (pk : Party × Nat) : Chan.wire p h ≠ askedIn pk := by
  simp [askedIn]

/-- A wire channel is never an assembler's resolution input. -/
private theorem wire_ne_asmResChan (p : Party) (h : Nat)
    (pk : Party × Nat) : Chan.wire p h ≠ asmResChan pk := by
  unfold asmResChan
  split <;> simp

/-- A wire channel is never an assembler's level input. -/
private theorem wire_ne_asmLevelChan (p : Party) (h : Nat)
    (pk : Party × Nat) : Chan.wire p h ≠ asmLevelChan pk := by
  simp [asmLevelChan]

/-- A wire channel is never an assembler's output. -/
private theorem wire_ne_asmOutChan (sk : Skel) (p : Party) (h : Nat)
    (pk : Party × Nat) : Chan.wire p h ≠ sk.asmOutChan pk := by
  unfold Skel.asmOutChan
  split
  · simp
  · split <;> simp

-- ================================================ wire-arm congruences

/-- Producer congruence on the wire family: `sentOf` at a wire channel
reads only the openers and the per-walk wire counts. -/
private theorem sentOf_wire_congr (sk : Skel) {s s' : State}
    (h1 : s'.iopenWire = s.iopenWire) (h2 : s'.ropenWire = s.ropenWire)
    (hws : ∀ q, wkWireSent sk s' q = wkWireSent sk s q)
    (p : Party) (h : Nat) :
    sentOf sk s' (Chan.wire p h) = sentOf sk s (Chan.wire p h) := by
  simp only [sentOf, h1, h2, hws]

/-- Consumer congruence on the wire family: `recvdOf` at a wire channel
reads only the opener receipt, the absorber cursor, and the per-walk
wire counts. -/
private theorem recvdOf_wire_congr (sk : Skel) {s s' : State}
    (h1 : s'.ropenGotWire = s.ropenGotWire)
    (h2 : s'.absorbPhase = s.absorbPhase)
    (h3 : s'.absorbIdx = s.absorbIdx)
    (hwr : ∀ q, wkWireRecvd sk s' q = wkWireRecvd sk s q)
    (p : Party) (h : Nat) :
    recvdOf sk s' (Chan.wire p h) = recvdOf sk s (Chan.wire p h) := by
  simp only [recvdOf, absorbWireRecvd, h1, h2, h3, hwr]

-- ========================================================== walkCommit

/-- `walkCommit` preserves the local fragment: the committed arm of
`wkLocalOk` is the `wkChoosable` guard verbatim.

Extracted from `Model.preserve_walkCommit` (Proofs/Preserve/Walk.lean):
the guard-inversion prelude and the wk/asm/top bullets; the flow bullet
is dropped. -/
theorem preserveL_walkCommit (hwf : sk.wellFormed = true)
    (pk : Party × Nat) (o : Oblig)
    (hstep : Model.apply sk ax (.walkCommit pk o) s = some s')
    (hi : InvL sk ax s) : InvL sk ax s' := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true] at hg
    obtain ⟨_hmem, hch⟩ := hg
    injection hstep with hs'
    rw [wkChoosable] at hch
    split at hch
    case isTrue => cases hch
    case isFalse hpc =>
      have hph2 : (s.walk pk).phase = 2 := by
        by_contra hne
        exact hpc (by simp [hne])
      have hcm : (s.walk pk).committed = none := by
        cases hcmv : (s.walk pk).committed with
        | none => rfl
        | some x => exact absurd (by simp [hcmv]) hpc
      refine ⟨fun pk' hpk' => ?_, fun pk' hpk' => ?_, ?_⟩
      · by_cases hpkeq : pk' = pk
        · subst hpkeq
          have hwalk : s'.walk pk'
              = { s.walk pk' with committed := some o } := by
            rw [← hs']; simp
          have hcount : wkWireCount sk s' pk'
              = wkWireCount sk s pk' := by
            simp [wkWireCount, hwalk]
          have hsc : scopeComplete sk pk'.2
              { s.walk pk' with committed := some o }
              = scopeComplete sk pk'.2 (s.walk pk') := rfl
          have hwk := hi.wk pk' hpk'
          simp only [wkLocalOk, hwalk, hcount, hsc] at hwk ⊢
          rw [hph2] at hwk ⊢
          rw [hcm] at hwk
          simp at hwk ⊢
          obtain ⟨hA, hB, hC⟩ := hwk
          refine ⟨hA, ⟨hB, hC⟩, ?_⟩
          -- the committed arm: three of four are the guard verbatim
          cases o with
          | res i => exact hch
          | query i => exact hch
          | parent => exact hch
          | wire i =>
              simp only [Bool.and_eq_true, decide_eq_true_eq,
                Bool.not_eq_true', List.all_eq_true,
                List.mem_range] at hch
              obtain ⟨⟨⟨⟨hin, hfront⟩, hlow⟩, hd4⟩, hd5⟩ := hch
              simp only [Bool.and_eq_true, beq_iff_eq,
                decide_eq_true_eq]
              have hn : sk.nChildren pk'.snd
                  (sk.stageScope pk'.snd (s.walk pk').scope)
                    ≤ sk.fan :=
                nChildren_le_fan hwf hA
              have hclosed : ∀ j < sk.fan,
                  (s.walk pk').wireDone j = true →
                  j = 0 ∨ (s.walk pk').wireDone (j - 1) = true := by
                intro j hj hwd
                rcases (hC j hj).1.1.1.1.1.1.1.1.1 with hf | ⟨-, h0⟩
                · rw [hwd] at hf; cases hf
                · exact h0
              refine ⟨⟨⟨?_, hin⟩, hd4⟩, hd5⟩
              rw [wkWireCount]
              exact (length_filter_of_frontier (by omega) hlow hfront
                hclosed).symm
        · have hw : s'.walk pk' = s.walk pk' := by
            rw [← hs']; exact setWalk_walk_ne s _ hpkeq
          rw [wkLocalOk_congr sk ax pk' hw]
          exact hi.wk pk' hpk'
      · rw [← hs']; exact hi.asm pk' hpk'
      · rw [← hs']; exact hi.top

/-- Counting deltas of `walkCommit`: every count frames.

Extracted from the flow bullet of `Model.preserve_walkCommit`: a
committed-choice update is invisible to every producer and consumer
count (`sentOf_setWalk_committed` / `recvdOf_setWalk_committed`) and
`chan` is untouched. -/
theorem delta_walkCommit (_hwf : sk.wellFormed = true)
    (pk : Party × Nat) (o : Oblig)
    (hstep : Model.apply sk ax (.walkCommit pk o) s = some s')
    (_hi : InvL sk ax s) :
    (∀ p h, sentOf sk s' (Chan.wire p h) = sentOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) + recvdOf sk s' (Chan.wire p h)
        = s.chan (Chan.wire p h) + recvdOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) ≤ s.chan (Chan.wire p h))
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c + recvdOf sk s c = sentOf sk s c →
        s'.chan c + recvdOf sk s' c = sentOf sk s' c)
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c ≤ sk.cap c → s'.chan c ≤ sk.cap c) := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue =>
    injection hstep with hs'
    have hchan : s'.chan = s.chan := by rw [← hs']; rfl
    have hsent : ∀ c, sentOf sk s' c = sentOf sk s c := by
      intro c; rw [← hs']
      exact sentOf_setWalk_committed sk s pk (some o) c
    have hrecv : ∀ c, recvdOf sk s' c = recvdOf sk s c := by
      intro c; rw [← hs']
      exact recvdOf_setWalk_committed sk s pk (some o) c
    exact ⟨fun p h => hsent _,
      fun p h => by rw [hchan, hrecv],
      fun p h => Nat.le_of_eq (congrFun hchan _),
      fun c _ _ heq => by rw [hchan, hsent, hrecv]; exact heq,
      fun c _ _ hcap => by rw [hchan]; exact hcap⟩

-- ======================================================== walkRecvWire

/-- `walkRecvWire` preserves the local fragment: the prologue wire
receive moves phase 0 → 1 and touches nothing a cursor reads.

Extracted from `Model.preserve_walkRecvWire`
(Proofs/Preserve/Walk.lean): guard inversion plus the wk/asm/top
bullets; the flow bullet is dropped. -/
theorem preserveL_walkRecvWire (_hwf : sk.wellFormed = true)
    (pk : Party × Nat)
    (hstep : Model.apply sk ax (.walkRecvWire pk) s = some s')
    (hi : InvL sk ax s) : InvL sk ax s' := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
    obtain ⟨⟨_hmem, hph0⟩, _hpos⟩ := hg
    injection hstep with hs'
    have hwalk : s'.walk pk
        = { s.walk pk with phase := 1, committed := none } := by
      rw [← hs']; simp
    refine ⟨fun pk' hpk' => ?_, fun pk' hpk' => ?_, ?_⟩
    · by_cases hpkeq : pk' = pk
      · subst hpkeq
        have hwk := hi.wk pk' hpk'
        simp only [wkLocalOk, hwalk] at hwk ⊢
        rw [hph0] at hwk
        simp at hwk ⊢
        exact ⟨hwk.1, hwk.2.1⟩
      · have hw : s'.walk pk' = s.walk pk' := by
          rw [← hs']; exact setWalk_walk_ne _ _ hpkeq
        rw [wkLocalOk_congr sk ax pk' hw]
        exact hi.wk pk' hpk'
    · rw [← hs']; exact hi.asm pk' hpk'
    · rw [← hs']; exact hi.top

/-- Counting deltas of `walkRecvWire`: the consumed wire channel trades
one unit of occupancy for one unit of consumption; everything else
frames.

Extracted from the flow bullet of `Model.preserve_walkRecvWire`.
Statement deviation: the wire sum conjunct is restricted to wire
channels in `allChans` — the phantom channel `wire I 0` aliases the
consumer count of the leaf responder walk `(R, 0)` (the Nat-subtraction
collision documented in Proofs/Wiring.lean), so at `pk = (R, 0)` its
sum moves with no matching occupancy change. Every wire channel a
process touches is in `allChans`, where the conjunct holds. -/
theorem delta_walkRecvWire (hwf : sk.wellFormed = true)
    (pk : Party × Nat)
    (hstep : Model.apply sk ax (.walkRecvWire pk) s = some s')
    (_hi : InvL sk ax s) :
    (∀ p h, sentOf sk s' (Chan.wire p h) = sentOf sk s (Chan.wire p h))
    ∧ (∀ p h, Chan.wire p h ∈ allChans sk →
        s'.chan (Chan.wire p h) + recvdOf sk s' (Chan.wire p h)
          = s.chan (Chan.wire p h) + recvdOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) ≤ s.chan (Chan.wire p h))
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c + recvdOf sk s c = sentOf sk s c →
        s'.chan c + recvdOf sk s' c = sentOf sk s' c)
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c ≤ sk.cap c → s'.chan c ≤ sk.cap c) := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
    obtain ⟨⟨hmem, hph0⟩, hpos⟩ := hg
    have hmem' : pk ∈ sk.walkKeys := by simpa using hmem
    injection hstep with hs'
    have hchan : s'.chan = bump s.chan (wireIn pk) (-1) := by
      rw [← hs']; rfl
    have hio : s'.iopenWire = s.iopenWire := by rw [← hs']; rfl
    have hro : s'.ropenWire = s.ropenWire := by rw [← hs']; rfl
    have hwalk : s'.walk pk
        = { s.walk pk with phase := 1, committed := none } := by
      rw [← hs']; simp
    have hws : ∀ q, wkWireSent sk s' q = wkWireSent sk s q := by
      intro q
      by_cases hq : q = pk
      · subst hq
        simp [wkWireSent, wkWireCount, hwalk]
      · have hwq : s'.walk q = s.walk q := by
          rw [← hs']; exact setWalk_walk_ne _ _ hq
        simp [wkWireSent, wkWireCount, hwq]
    have hsent : ∀ c ∈ allChans sk, sentOf sk s' c = sentOf sk s c := by
      intro c hc
      rw [← hs']
      exact sentOf_setWalk_same hwf _ pk
        { s.walk pk with phase := 1, committed := none } hmem'
        (by simp [wkWireSent, wkWireCount])
        (by simp [wkResSent, wkResCount])
        (by simp [wkQSentTot, wkQSum])
        (by simp [wkParentSent, hph0])
        hc
    refine ⟨fun p h => sentOf_wire_congr sk hio hro hws p h,
      ?_, ?_, ?_, ?_⟩
    · intro p h hcm
      by_cases hcw : Chan.wire p h = wireIn pk
      · rw [hcw]
        have hr' : recvdOf sk s' (wireIn pk)
            = wkWireRecvd sk s pk + 1 := by
          rw [← hs', recvdOf_wireIn hmem']
          simp [wkWireRecvd, hph0]
        have hr0 : recvdOf sk s (wireIn pk) = wkWireRecvd sk s pk :=
          recvdOf_wireIn hmem'
        rw [hchan, bump_neg_one, hr', hr0]
        omega
      · have hrecv : recvdOf sk s' (Chan.wire p h)
            = recvdOf sk s (Chan.wire p h) := by
          rw [← hs']
          exact recvdOf_setWalk_frame hwf _ pk _ hcm hcw
            (wire_ne_askedIn p h pk)
        rw [hchan, bump_ne _ _ hcw, hrecv]
    · intro p h
      by_cases hcw : Chan.wire p h = wireIn pk
      · rw [hcw, hchan, bump_neg_one]
        omega
      · exact Nat.le_of_eq (by rw [hchan, bump_ne _ _ hcw])
    · intro c hc hnw heq
      have hcw : c ≠ wireIn pk := ne_wireIn hnw pk
      have hrecv : recvdOf sk s' c = recvdOf sk s c := by
        by_cases h6 : c = askedIn pk
        · subst h6
          rw [← hs', recvdOf_askedIn, recvdOf_askedIn]
          simp [wkAskedRecvd, hph0]
        · rw [← hs']
          exact recvdOf_setWalk_frame hwf _ pk _ hc hcw h6
      rw [hchan, bump_ne _ _ hcw, hsent c hc, hrecv]
      exact heq
    · intro c _ hnw hcap
      rw [hchan, bump_ne _ _ (ne_wireIn hnw pk)]
      exact hcap

-- ======================================================= walkRecvAsked

/-- `walkRecvAsked` preserves the local fragment: the prologue query
receive moves phase 1 → 2, and the embedded `normWalk` is provably the
identity (a freshly-phase-2 walk has empty machinery).

Extracted from `Model.preserve_walkRecvAsked`
(Proofs/Preserve/Walk.lean): guard inversion plus the wk/asm/top
bullets; the flow bullet is dropped. -/
theorem preserveL_walkRecvAsked (_hwf : sk.wellFormed = true)
    (pk : Party × Nat)
    (hstep : Model.apply sk ax (.walkRecvAsked pk) s = some s')
    (hi : InvL sk ax s) : InvL sk ax s' := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
    obtain ⟨⟨hmem, hph1⟩, _hpos⟩ := hg
    have hmem' : pk ∈ sk.walkKeys := by simpa using hmem
    injection hstep with hs'
    -- phase-1 facts from the invariant: cursor in range, machinery empty
    have hwk := hi.wk pk hmem'
    simp only [wkLocalOk] at hwk
    rw [hph1] at hwk
    simp at hwk
    obtain ⟨hslt, ⟨hledger, hpd⟩, _hcm⟩ := hwk
    -- normWalk is the identity on the freshly-phase-2 walk
    have hnlt : ¬ (s.walk pk).scope ≥ sk.stageLen pk.2 := by omega
    have hscF : scopeComplete sk pk.2
        { s.walk pk with phase := 2, committed := none } = false := by
      simp [scopeComplete, hnlt, hpd]
    have hnw : normWalk sk pk.2
        { s.walk pk with phase := 2, committed := none }
        = { s.walk pk with phase := 2, committed := none } := by
      simp [normWalk, hscF]
    have hwalk : s'.walk pk
        = { s.walk pk with phase := 2, committed := none } := by
      rw [← hs']; simp [hnw]
    refine ⟨fun pk' hpk' => ?_, fun pk' hpk' => ?_, ?_⟩
    · by_cases hpkeq : pk' = pk
      · subst hpkeq
        simp only [wkLocalOk, hwalk, hscF]
        simp [hslt]
        intro j hj
        simp [hledger j hj]
      · have hw : s'.walk pk' = s.walk pk' := by
          rw [← hs']; exact setWalk_walk_ne _ _ hpkeq
        rw [wkLocalOk_congr sk ax pk' hw]
        exact hi.wk pk' hpk'
    · rw [← hs']; exact hi.asm pk' hpk'
    · rw [← hs']; exact hi.top

/-- Counting deltas of `walkRecvAsked`: the consumed query channel
(internal) trades one unit of occupancy for one unit of consumption;
the wire family frames completely.

Extracted from the flow bullet of `Model.preserve_walkRecvAsked`; the
`InvL` hypothesis is load-bearing (it proves the embedded `normWalk`
the identity, exactly as in the monolith's prelude). -/
theorem delta_walkRecvAsked (hwf : sk.wellFormed = true)
    (pk : Party × Nat)
    (hstep : Model.apply sk ax (.walkRecvAsked pk) s = some s')
    (hi : InvL sk ax s) :
    (∀ p h, sentOf sk s' (Chan.wire p h) = sentOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) + recvdOf sk s' (Chan.wire p h)
        = s.chan (Chan.wire p h) + recvdOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) ≤ s.chan (Chan.wire p h))
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c + recvdOf sk s c = sentOf sk s c →
        s'.chan c + recvdOf sk s' c = sentOf sk s' c)
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c ≤ sk.cap c → s'.chan c ≤ sk.cap c) := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
    obtain ⟨⟨hmem, hph1⟩, hpos⟩ := hg
    have hmem' : pk ∈ sk.walkKeys := by simpa using hmem
    injection hstep with hs'
    have hwk := hi.wk pk hmem'
    simp only [wkLocalOk] at hwk
    rw [hph1] at hwk
    simp at hwk
    obtain ⟨hslt, ⟨_hledger, hpd⟩, _hcm⟩ := hwk
    have hnlt : ¬ (s.walk pk).scope ≥ sk.stageLen pk.2 := by omega
    have hscF : scopeComplete sk pk.2
        { s.walk pk with phase := 2, committed := none } = false := by
      simp [scopeComplete, hnlt, hpd]
    have hnw : normWalk sk pk.2
        { s.walk pk with phase := 2, committed := none }
        = { s.walk pk with phase := 2, committed := none } := by
      simp [normWalk, hscF]
    have hwalk : s'.walk pk
        = { s.walk pk with phase := 2, committed := none } := by
      rw [← hs']; simp [hnw]
    have hchan : s'.chan = bump s.chan (askedIn pk) (-1) := by
      rw [← hs']; rfl
    have hio : s'.iopenWire = s.iopenWire := by rw [← hs']; rfl
    have hro : s'.ropenWire = s.ropenWire := by rw [← hs']; rfl
    have hgw : s'.ropenGotWire = s.ropenGotWire := by rw [← hs']; rfl
    have hab1 : s'.absorbPhase = s.absorbPhase := by rw [← hs']; rfl
    have hab2 : s'.absorbIdx = s.absorbIdx := by rw [← hs']; rfl
    have hws : ∀ q, wkWireSent sk s' q = wkWireSent sk s q := by
      intro q
      by_cases hq : q = pk
      · subst hq
        simp [wkWireSent, wkWireCount, hwalk]
      · have hwq : s'.walk q = s.walk q := by
          rw [← hs']; exact setWalk_walk_ne _ _ hq
        simp [wkWireSent, wkWireCount, hwq]
    have hwr : ∀ q, wkWireRecvd sk s' q = wkWireRecvd sk s q := by
      intro q
      by_cases hq : q = pk
      · subst hq
        simp [wkWireRecvd, hwalk, hph1]
      · have hwq : s'.walk q = s.walk q := by
          rw [← hs']; exact setWalk_walk_ne _ _ hq
        simp [wkWireRecvd, hwq]
    have hsent : ∀ c ∈ allChans sk, sentOf sk s' c = sentOf sk s c := by
      intro c hc
      rw [← hs']
      rw [show setWalk { s with chan := bump s.chan (askedIn pk) (-1) }
          pk (normWalk sk pk.2
            { s.walk pk with phase := 2, committed := none })
          = setWalk { s with chan := bump s.chan (askedIn pk) (-1) } pk
            { s.walk pk with phase := 2, committed := none } from by
        rw [hnw]]
      exact sentOf_setWalk_same hwf _ pk
        { s.walk pk with phase := 2, committed := none } hmem'
        (by simp [wkWireSent, wkWireCount])
        (by simp [wkResSent, wkResCount])
        (by simp [wkQSentTot, wkQSum])
        (by simp [wkParentSent, hph1, hpd])
        hc
    refine ⟨fun p h => sentOf_wire_congr sk hio hro hws p h,
      ?_, ?_, ?_, ?_⟩
    · intro p h
      rw [hchan, bump_ne _ _ (wire_ne_askedIn p h pk),
        recvdOf_wire_congr sk hgw hab1 hab2 hwr p h]
    · intro p h
      exact Nat.le_of_eq
        (by rw [hchan, bump_ne _ _ (wire_ne_askedIn p h pk)])
    · intro c hc hnwire heq
      by_cases h6 : c = askedIn pk
      · subst h6
        have hr' : recvdOf sk s' (askedIn pk)
            = wkAskedRecvd sk s pk + 1 := by
          rw [← hs', recvdOf_askedIn]
          simp [wkAskedRecvd, hnw, hph1]
        have hr0 : recvdOf sk s (askedIn pk) = wkAskedRecvd sk s pk :=
          recvdOf_askedIn
        rw [hchan, bump_neg_one, hsent _ hc, hr']
        rw [hr0] at heq
        omega
      · have hrecv : recvdOf sk s' c = recvdOf sk s c := by
          rw [← hs']
          exact recvdOf_setWalk_frame hwf _ pk _ hc
            (ne_wireIn hnwire pk) h6
        rw [hchan, bump_ne _ _ h6, hsent c hc, hrecv]
        exact heq
    · intro c _ _ hcap
      rw [hchan]
      by_cases h6 : c = askedIn pk
      · subst h6; rw [bump_neg_one]; omega
      · rw [bump_ne _ _ h6]; exact hcap

-- ======================================================= walkCloseWire

/-- `walkCloseWire` preserves the local fragment: phase 3 → 4 and
nothing else.

Extracted from `Model.preserve_walkCloseWire`
(Proofs/Preserve/Walk.lean): guard inversion plus the wk/asm/top
bullets; the flow bullet is dropped. -/
theorem preserveL_walkCloseWire (_hwf : sk.wellFormed = true)
    (pk : Party × Nat)
    (hstep : Model.apply sk ax (.walkCloseWire pk) s = some s')
    (hi : InvL sk ax s) : InvL sk ax s' := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq] at hg
    obtain ⟨⟨⟨_hmem, hph3⟩, _hpd⟩, _hz⟩ := hg
    injection hstep with hs'
    have hwalk : s'.walk pk = { s.walk pk with phase := 4 } := by
      rw [← hs']; simp
    refine ⟨fun pk' hpk' => ?_, fun pk' hpk' => ?_, ?_⟩
    · by_cases hpkeq : pk' = pk
      · subst hpkeq
        have hwk := hi.wk pk' hpk'
        simp only [wkLocalOk, hwalk] at hwk ⊢
        rw [hph3] at hwk
        simp at hwk ⊢
        exact hwk
      · have hw : s'.walk pk' = s.walk pk' := by
          rw [← hs']; exact setWalk_walk_ne s _ hpkeq
        rw [wkLocalOk_congr sk ax pk' hw]
        exact hi.wk pk' hpk'
    · rw [← hs']; exact hi.asm pk' hpk'
    · rw [← hs']; exact hi.top

/-- Counting deltas of `walkCloseWire`: a pure phase move, every count
frames.

Extracted from the flow bullet of `Model.preserve_walkCloseWire`. The
receive counts saturated at the stage length when phase reached 3, so
3 → 4 is count-neutral on every channel, including the closed wire
channel — no flow hypothesis is needed. -/
theorem delta_walkCloseWire (hwf : sk.wellFormed = true)
    (pk : Party × Nat)
    (hstep : Model.apply sk ax (.walkCloseWire pk) s = some s')
    (_hi : InvL sk ax s) :
    (∀ p h, sentOf sk s' (Chan.wire p h) = sentOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) + recvdOf sk s' (Chan.wire p h)
        = s.chan (Chan.wire p h) + recvdOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) ≤ s.chan (Chan.wire p h))
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c + recvdOf sk s c = sentOf sk s c →
        s'.chan c + recvdOf sk s' c = sentOf sk s' c)
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c ≤ sk.cap c → s'.chan c ≤ sk.cap c) := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq] at hg
    obtain ⟨⟨⟨hmem, hph3⟩, _hpd⟩, _hz⟩ := hg
    have hmem' : pk ∈ sk.walkKeys := by simpa using hmem
    injection hstep with hs'
    have hchan : s'.chan = s.chan := by rw [← hs']; rfl
    have hio : s'.iopenWire = s.iopenWire := by rw [← hs']; rfl
    have hro : s'.ropenWire = s.ropenWire := by rw [← hs']; rfl
    have hgw : s'.ropenGotWire = s.ropenGotWire := by rw [← hs']; rfl
    have hab1 : s'.absorbPhase = s.absorbPhase := by rw [← hs']; rfl
    have hab2 : s'.absorbIdx = s.absorbIdx := by rw [← hs']; rfl
    have hwalk : s'.walk pk = { s.walk pk with phase := 4 } := by
      rw [← hs']; simp
    have hws : ∀ q, wkWireSent sk s' q = wkWireSent sk s q := by
      intro q
      by_cases hq : q = pk
      · subst hq
        simp [wkWireSent, wkWireCount, hwalk]
      · have hwq : s'.walk q = s.walk q := by
          rw [← hs']; exact setWalk_walk_ne _ _ hq
        simp [wkWireSent, wkWireCount, hwq]
    have hwr : ∀ q, wkWireRecvd sk s' q = wkWireRecvd sk s q := by
      intro q
      by_cases hq : q = pk
      · subst hq
        simp [wkWireRecvd, hwalk, hph3]
      · have hwq : s'.walk q = s.walk q := by
          rw [← hs']; exact setWalk_walk_ne _ _ hq
        simp [wkWireRecvd, hwq]
    have hsent : ∀ c ∈ allChans sk, sentOf sk s' c = sentOf sk s c := by
      intro c hc
      rw [← hs']
      exact sentOf_setWalk_same hwf s pk
        { s.walk pk with phase := 4 } hmem'
        (by simp [wkWireSent, wkWireCount])
        (by simp [wkResSent, wkResCount])
        (by simp [wkQSentTot, wkQSum])
        (by simp [wkParentSent, hph3])
        hc
    have hrecv : ∀ c ∈ allChans sk,
        recvdOf sk s' c = recvdOf sk s c := by
      intro c hc
      rw [← hs']
      exact recvdOf_setWalk_same hwf s pk
        { s.walk pk with phase := 4 } hmem'
        (by simp [wkWireRecvd, hph3])
        (by simp [wkAskedRecvd, hph3])
        hc
    exact ⟨fun p h => sentOf_wire_congr sk hio hro hws p h,
      fun p h => by
        rw [hchan, recvdOf_wire_congr sk hgw hab1 hab2 hwr p h],
      fun p h => Nat.le_of_eq (congrFun hchan _),
      fun c hc _ heq => by
        rw [hchan, hsent c hc, hrecv c hc]; exact heq,
      fun c _ _ hcap => by rw [hchan]; exact hcap⟩

-- ====================================================== walkCloseAsked

/-- `walkCloseAsked` preserves the local fragment: phase 4 → 5, same
shape as `walkCloseWire`.

Extracted from `Model.preserve_walkCloseAsked`
(Proofs/Preserve/Walk.lean): guard inversion plus the wk/asm/top
bullets; the flow bullet is dropped. -/
theorem preserveL_walkCloseAsked (_hwf : sk.wellFormed = true)
    (pk : Party × Nat)
    (hstep : Model.apply sk ax (.walkCloseAsked pk) s = some s')
    (hi : InvL sk ax s) : InvL sk ax s' := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq] at hg
    obtain ⟨⟨⟨_hmem, hph4⟩, _hpd⟩, _hz⟩ := hg
    injection hstep with hs'
    have hwalk : s'.walk pk = { s.walk pk with phase := 5 } := by
      rw [← hs']; simp
    refine ⟨fun pk' hpk' => ?_, fun pk' hpk' => ?_, ?_⟩
    · by_cases hpkeq : pk' = pk
      · subst hpkeq
        have hwk := hi.wk pk' hpk'
        simp only [wkLocalOk, hwalk] at hwk ⊢
        rw [hph4] at hwk
        simp at hwk ⊢
        exact hwk
      · have hw : s'.walk pk' = s.walk pk' := by
          rw [← hs']; exact setWalk_walk_ne s _ hpkeq
        rw [wkLocalOk_congr sk ax pk' hw]
        exact hi.wk pk' hpk'
    · rw [← hs']; exact hi.asm pk' hpk'
    · rw [← hs']; exact hi.top

/-- Counting deltas of `walkCloseAsked`: a pure phase move, every count
frames.

Extracted from the flow bullet of `Model.preserve_walkCloseAsked`;
count-neutral across 4 → 5 exactly as `delta_walkCloseWire` is across
3 → 4. -/
theorem delta_walkCloseAsked (hwf : sk.wellFormed = true)
    (pk : Party × Nat)
    (hstep : Model.apply sk ax (.walkCloseAsked pk) s = some s')
    (_hi : InvL sk ax s) :
    (∀ p h, sentOf sk s' (Chan.wire p h) = sentOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) + recvdOf sk s' (Chan.wire p h)
        = s.chan (Chan.wire p h) + recvdOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) ≤ s.chan (Chan.wire p h))
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c + recvdOf sk s c = sentOf sk s c →
        s'.chan c + recvdOf sk s' c = sentOf sk s' c)
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c ≤ sk.cap c → s'.chan c ≤ sk.cap c) := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq] at hg
    obtain ⟨⟨⟨hmem, hph4⟩, _hpd⟩, _hz⟩ := hg
    have hmem' : pk ∈ sk.walkKeys := by simpa using hmem
    injection hstep with hs'
    have hchan : s'.chan = s.chan := by rw [← hs']; rfl
    have hio : s'.iopenWire = s.iopenWire := by rw [← hs']; rfl
    have hro : s'.ropenWire = s.ropenWire := by rw [← hs']; rfl
    have hgw : s'.ropenGotWire = s.ropenGotWire := by rw [← hs']; rfl
    have hab1 : s'.absorbPhase = s.absorbPhase := by rw [← hs']; rfl
    have hab2 : s'.absorbIdx = s.absorbIdx := by rw [← hs']; rfl
    have hwalk : s'.walk pk = { s.walk pk with phase := 5 } := by
      rw [← hs']; simp
    have hws : ∀ q, wkWireSent sk s' q = wkWireSent sk s q := by
      intro q
      by_cases hq : q = pk
      · subst hq
        simp [wkWireSent, wkWireCount, hwalk]
      · have hwq : s'.walk q = s.walk q := by
          rw [← hs']; exact setWalk_walk_ne _ _ hq
        simp [wkWireSent, wkWireCount, hwq]
    have hwr : ∀ q, wkWireRecvd sk s' q = wkWireRecvd sk s q := by
      intro q
      by_cases hq : q = pk
      · subst hq
        simp [wkWireRecvd, hwalk, hph4]
      · have hwq : s'.walk q = s.walk q := by
          rw [← hs']; exact setWalk_walk_ne _ _ hq
        simp [wkWireRecvd, hwq]
    have hsent : ∀ c ∈ allChans sk, sentOf sk s' c = sentOf sk s c := by
      intro c hc
      rw [← hs']
      exact sentOf_setWalk_same hwf s pk
        { s.walk pk with phase := 5 } hmem'
        (by simp [wkWireSent, wkWireCount])
        (by simp [wkResSent, wkResCount])
        (by simp [wkQSentTot, wkQSum])
        (by simp [wkParentSent, hph4])
        hc
    have hrecv : ∀ c ∈ allChans sk,
        recvdOf sk s' c = recvdOf sk s c := by
      intro c hc
      rw [← hs']
      exact recvdOf_setWalk_same hwf s pk
        { s.walk pk with phase := 5 } hmem'
        (by simp [wkWireRecvd, hph4])
        (by simp [wkAskedRecvd, hph4])
        hc
    exact ⟨fun p h => sentOf_wire_congr sk hio hro hws p h,
      fun p h => by
        rw [hchan, recvdOf_wire_congr sk hgw hab1 hab2 hwr p h],
      fun p h => Nat.le_of_eq (congrFun hchan _),
      fun c hc _ heq => by
        rw [hchan, hsent c hc, hrecv c hc]; exact heq,
      fun c _ _ hcap => by rw [hchan]; exact hcap⟩

-- ========================================================== asmRecvRes

/-- `asmRecvRes` preserves the local fragment: `asmLocalOk` at `pk`
re-establishes from the `pendAt` branch condition.

Extracted from `Model.preserve_asmRecvRes` (Proofs/Preserve/Asm.lean):
guard inversion plus the wk/asm/top bullets; the flow bullet is
dropped. -/
theorem preserveL_asmRecvRes (_hwf : sk.wellFormed = true)
    (pk : Party × Nat)
    (hstep : Model.apply sk ax (.asmRecvRes pk) s = some s')
    (hi : InvL sk ax s) : InvL sk ax s' := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
    obtain ⟨⟨hmem, hph⟩, _hpos⟩ := hg
    injection hstep with hs'
    have hpkmem : pk ∈ sk.asmKeys := List.contains_iff_mem.mp hmem
    have hold := hi.asm pk hpkmem
    simp only [asmLocalOk, hph] at hold
    simp at hold
    refine ⟨fun pk' hpk' => ?_, fun pk' hpk' => ?_, ?_⟩
    · rw [← hs']; exact hi.wk pk' hpk'
    · by_cases hpkeq : pk' = pk
      · subst hpkeq
        have hasm : s'.asm pk' = { s.asm pk' with
            phase := if sk.pendAt pk'.1 pk'.2 (s.asm pk').idx > 0
              then 1 else 2,
            got := 0 } := by rw [← hs']; simp
        by_cases hpend : sk.pendAt pk'.1 pk'.2 (s.asm pk').idx > 0
        · simp [asmLocalOk, hasm, hpend]
          omega
        · simp [asmLocalOk, hasm, hpend]
          omega
      · have ha : s'.asm pk' = s.asm pk' := by
          rw [← hs']; exact setAsm_asm_ne _ _ hpkeq
        rw [asmLocalOk_congr sk pk' ha]; exact hi.asm pk' hpk'
    · rw [← hs']; exact hi.top

/-- Counting deltas of `asmRecvRes`: the consumed resolution channel
(internal) trades one unit of occupancy for one unit of consumption;
the wire family frames completely.

Extracted from the flow bullet of `Model.preserve_asmRecvRes`; the
`InvL` hypothesis supplies the phase-0 empty ledger (`got = 0`) that
keeps the level count framed. -/
theorem delta_asmRecvRes (hwf : sk.wellFormed = true)
    (pk : Party × Nat)
    (hstep : Model.apply sk ax (.asmRecvRes pk) s = some s')
    (hi : InvL sk ax s) :
    (∀ p h, sentOf sk s' (Chan.wire p h) = sentOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) + recvdOf sk s' (Chan.wire p h)
        = s.chan (Chan.wire p h) + recvdOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) ≤ s.chan (Chan.wire p h))
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c + recvdOf sk s c = sentOf sk s c →
        s'.chan c + recvdOf sk s' c = sentOf sk s' c)
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c ≤ sk.cap c → s'.chan c ≤ sk.cap c) := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
    obtain ⟨⟨hmem, hph⟩, hpos⟩ := hg
    injection hstep with hs'
    have hpk2 : 1 ≤ pk.2 := asmKeys_snd_pos hmem
    have hpkmem : pk ∈ sk.asmKeys := List.contains_iff_mem.mp hmem
    have hold := hi.asm pk hpkmem
    simp only [asmLocalOk, hph] at hold
    simp at hold
    have hchan : s'.chan = bump s.chan (asmResChan pk) (-1) := by
      rw [← hs']; rfl
    have hsent : ∀ c, sentOf sk s' c = sentOf sk s c := by
      intro c
      rw [← hs']
      apply sentOf_ext_idx
      · intro pk''
        by_cases hq : pk'' = pk
        · subst hq; simp
        · simp [setAsm_asm_ne _ _ hq]
      all_goals rfl
    have hrecvw : ∀ (p : Party) (h : Nat),
        recvdOf sk s' (Chan.wire p h)
          = recvdOf sk s (Chan.wire p h) := by
      intro p h
      rw [← hs']
      rfl
    refine ⟨fun p h => hsent _, ?_, ?_, ?_, ?_⟩
    · intro p h
      rw [hchan, bump_ne _ _ (wire_ne_asmResChan p h pk), hrecvw]
    · intro p h
      exact Nat.le_of_eq
        (by rw [hchan, bump_ne _ _ (wire_ne_asmResChan p h pk)])
    · intro c hc _ heq
      by_cases hcc : c = asmResChan pk
      · subst hcc
        have h21 : pk.2 - 1 + 1 = pk.2 := by omega
        have hkey : ((pk.1, pk.2 - 1 + 1) : Party × Nat) = pk := by
          rw [h21]
        by_cases hask : asks pk.1 pk.2 = true
        · have hch : asmResChan pk = Chan.upper pk.1 (pk.2 - 1) := by
            simp [asmResChan, hask]
          have hrecvS : recvdOf sk s (asmResChan pk)
              = (s.asm pk).idx := by
            rw [hch]
            show asmResRecvd s (pk.1, pk.2 - 1 + 1) = _
            rw [hkey]
            simp [asmResRecvd, hph]
          have hrecvS' : recvdOf sk s' (asmResChan pk)
              = (s.asm pk).idx + 1 := by
            rw [← hs', hch]
            show asmResRecvd (setAsm _ pk _) (pk.1, pk.2 - 1 + 1) = _
            rw [hkey]
            simp only [asmResRecvd, setAsm_asm_self]
            split <;> simp
          rw [hchan, bump_neg_one, hsent, hrecvS']
          rw [hrecvS] at heq
          omega
        · have hch : asmResChan pk = Chan.lower pk.1 pk.2 := by
            simp [asmResChan, hask]
          have hrecvS : recvdOf sk s (asmResChan pk)
              = (s.asm pk).idx := by
            rw [hch]
            simp [recvdOf, hpkmem, asmResRecvd, hph]
          have hrecvS' : recvdOf sk s' (asmResChan pk)
              = (s.asm pk).idx + 1 := by
            rw [← hs', hch]
            simp only [recvdOf]
            rw [if_pos (by exact hmem)]
            simp only [asmResRecvd, setAsm_asm_self]
            split <;> simp
          rw [hchan, bump_neg_one, hsent, hrecvS']
          rw [hrecvS] at heq
          omega
      · have hrecv : recvdOf sk s' c = recvdOf sk s c := by
          rw [← hs']
          refine Eq.trans
            (recvdOf_setAsm_frame_res hwf _ pk _ hc hcc ?_) ?_
          · simp [asmLevelRecvd, hold.2]
          · exact recvdOf_ext sk (fun _ => rfl) (fun _ => rfl)
              (fun _ => rfl) rfl rfl rfl rfl rfl rfl c
        rw [hchan, bump_ne _ _ hcc, hsent, hrecv]
        exact heq
    · intro c _ _ hcap
      rw [hchan]
      by_cases hcc : c = asmResChan pk
      · subst hcc; rw [bump_neg_one]; omega
      · rw [bump_ne _ _ hcc]; exact hcap

-- ======================================================== asmRecvLevel

/-- `asmRecvLevel` preserves the local fragment: the phase-2 fullness
conjunct is the branch condition verbatim.

Extracted from `Model.preserve_asmRecvLevel`
(Proofs/Preserve/Asm.lean): guard inversion plus the wk/asm/top
bullets; the flow bullet is dropped. -/
theorem preserveL_asmRecvLevel (_hwf : sk.wellFormed = true)
    (pk : Party × Nat)
    (hstep : Model.apply sk ax (.asmRecvLevel pk) s = some s')
    (hi : InvL sk ax s) : InvL sk ax s' := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
    obtain ⟨⟨hmem, hph⟩, _hpos⟩ := hg
    injection hstep with hs'
    have hpkmem : pk ∈ sk.asmKeys := List.contains_iff_mem.mp hmem
    have hold := hi.asm pk hpkmem
    simp only [asmLocalOk, hph] at hold
    simp at hold
    refine ⟨fun pk' hpk' => ?_, fun pk' hpk' => ?_, ?_⟩
    · rw [← hs']; exact hi.wk pk' hpk'
    · by_cases hpkeq : pk' = pk
      · subst hpkeq
        have hasm : s'.asm pk' = { s.asm pk' with
            phase := if (s.asm pk').got + 1
                == sk.pendAt pk'.1 pk'.2 (s.asm pk').idx then 2 else 1,
            got := (s.asm pk').got + 1 } := by rw [← hs']; simp
        by_cases hfull : (s.asm pk').got + 1
            = sk.pendAt pk'.1 pk'.2 (s.asm pk').idx
        · simp [asmLocalOk, hasm, hfull]
          omega
        · simp [asmLocalOk, hasm, hfull]
          omega
      · have ha : s'.asm pk' = s.asm pk' := by
          rw [← hs']; exact setAsm_asm_ne _ _ hpkeq
        rw [asmLocalOk_congr sk pk' ha]; exact hi.asm pk' hpk'
    · rw [← hs']; exact hi.top

/-- Counting deltas of `asmRecvLevel`: the consumed level channel
(internal) trades one unit of occupancy for one unit of consumption
(`got + 1`); the wire family frames completely.

Extracted from the flow bullet of `Model.preserve_asmRecvLevel`. -/
theorem delta_asmRecvLevel (_hwf : sk.wellFormed = true)
    (pk : Party × Nat)
    (hstep : Model.apply sk ax (.asmRecvLevel pk) s = some s')
    (_hi : InvL sk ax s) :
    (∀ p h, sentOf sk s' (Chan.wire p h) = sentOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) + recvdOf sk s' (Chan.wire p h)
        = s.chan (Chan.wire p h) + recvdOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) ≤ s.chan (Chan.wire p h))
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c + recvdOf sk s c = sentOf sk s c →
        s'.chan c + recvdOf sk s' c = sentOf sk s' c)
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c ≤ sk.cap c → s'.chan c ≤ sk.cap c) := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
    obtain ⟨⟨hmem, hph⟩, hpos⟩ := hg
    injection hstep with hs'
    have hpk2 : 1 ≤ pk.2 := asmKeys_snd_pos hmem
    have hchan : s'.chan = bump s.chan (asmLevelChan pk) (-1) := by
      rw [← hs']; rfl
    have hsent : ∀ c, sentOf sk s' c = sentOf sk s c := by
      intro c
      rw [← hs']
      apply sentOf_ext_idx
      · intro pk''
        by_cases hq : pk'' = pk
        · subst hq; simp
        · simp [setAsm_asm_ne _ _ hq]
      all_goals rfl
    have hrecvw : ∀ (p : Party) (h : Nat),
        recvdOf sk s' (Chan.wire p h)
          = recvdOf sk s (Chan.wire p h) := by
      intro p h
      rw [← hs']
      rfl
    refine ⟨fun p h => hsent _, ?_, ?_, ?_, ?_⟩
    · intro p h
      rw [hchan, bump_ne _ _ (wire_ne_asmLevelChan p h pk), hrecvw]
    · intro p h
      exact Nat.le_of_eq
        (by rw [hchan, bump_ne _ _ (wire_ne_asmLevelChan p h pk)])
    · intro c hc _ heq
      by_cases hcc : c = asmLevelChan pk
      · subst hcc
        have h21 : pk.2 - 1 + 1 = pk.2 := by omega
        have hkey : ((pk.1, pk.2 - 1 + 1) : Party × Nat) = pk := by
          rw [h21]
        have hrecvS : recvdOf sk s (asmLevelChan pk)
            = sk.pendsBefore pk.1 pk.2 (s.asm pk).idx
              + (s.asm pk).got := by
          show recvdOf sk s (Chan.level pk.1 (pk.2 - 1)) = _
          simp only [recvdOf]
          rw [hkey, if_pos (by exact hmem)]
          rfl
        have hrecvS' : recvdOf sk s' (asmLevelChan pk)
            = sk.pendsBefore pk.1 pk.2 (s.asm pk).idx
              + (s.asm pk).got + 1 := by
          rw [← hs']
          show recvdOf sk (setAsm _ pk _) (Chan.level pk.1 (pk.2 - 1))
            = _
          simp only [recvdOf]
          rw [hkey, if_pos (by exact hmem)]
          simp only [asmLevelRecvd, setAsm_asm_self]
          omega
        rw [hchan, bump_neg_one, hsent, hrecvS']
        rw [hrecvS] at heq
        omega
      · have hrecv : recvdOf sk s' c = recvdOf sk s c := by
          rw [← hs']
          refine Eq.trans
            (recvdOf_setAsm_frame_level _ pk _ hcc ?_) ?_
          · simp only [asmResRecvd, setAsm_asm_self]
            split <;> simp [hph]
          · exact recvdOf_ext sk (fun _ => rfl) (fun _ => rfl)
              (fun _ => rfl) rfl rfl rfl rfl rfl rfl c
        rw [hchan, bump_ne _ _ hcc, hsent, hrecv]
        exact heq
    · intro c _ _ hcap
      rw [hchan]
      by_cases hcc : c = asmLevelChan pk
      · subst hcc; rw [bump_neg_one]; omega
      · rw [bump_ne _ _ hcc]; exact hcap

-- ============================================================= asmSend

/-- `asmSend` preserves the local fragment: the cursor advances and the
next phase re-establishes from the length branch condition.

Extracted from `Model.preserve_asmSend` (Proofs/Preserve/Asm.lean):
guard inversion plus the wk/asm/top bullets; the flow bullet is
dropped. -/
theorem preserveL_asmSend (_hwf : sk.wellFormed = true)
    (pk : Party × Nat)
    (hstep : Model.apply sk ax (.asmSend pk) s = some s')
    (hi : InvL sk ax s) : InvL sk ax s' := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
    obtain ⟨⟨hmem, hph⟩, _hcaplt⟩ := hg
    injection hstep with hs'
    have hpkmem : pk ∈ sk.asmKeys := List.contains_iff_mem.mp hmem
    have hold := hi.asm pk hpkmem
    simp only [asmLocalOk, hph] at hold
    simp at hold
    refine ⟨fun pk' hpk' => ?_, fun pk' hpk' => ?_, ?_⟩
    · rw [← hs']; exact hi.wk pk' hpk'
    · by_cases hpkeq : pk' = pk
      · subst hpkeq
        have hasm : s'.asm pk' =
            { idx := (s.asm pk').idx + 1,
              phase := if (s.asm pk').idx + 1
                  < (sk.asmResList pk'.1 pk'.2).length then 0 else 3,
              got := 0 } := by rw [← hs']; simp
        by_cases hlt : (s.asm pk').idx + 1
            < (sk.asmResList pk'.1 pk'.2).length
        · simp [asmLocalOk, hasm, hlt]
        · simp [asmLocalOk, hasm, hlt]
          omega
      · have ha : s'.asm pk' = s.asm pk' := by
          rw [← hs']; exact setAsm_asm_ne _ _ hpkeq
        rw [asmLocalOk_congr sk pk' ha]; exact hi.asm pk' hpk'
    · rw [← hs']; exact hi.top

/-- Counting deltas of `asmSend`: one send on the (internal) output
channel, matched by the cursor advance; both consumer counts at `pk`
telescope, and the wire family frames completely.

Extracted from the flow bullet of `Model.preserve_asmSend`; the `InvL`
hypothesis supplies the phase-2 fullness (`got = pendAt`) behind the
`pendsBefore_succ` telescope, and the slot conjunct on the touched
channel is the `chan < cap` guard. -/
theorem delta_asmSend (_hwf : sk.wellFormed = true)
    (pk : Party × Nat)
    (hstep : Model.apply sk ax (.asmSend pk) s = some s')
    (hi : InvL sk ax s) :
    (∀ p h, sentOf sk s' (Chan.wire p h) = sentOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) + recvdOf sk s' (Chan.wire p h)
        = s.chan (Chan.wire p h) + recvdOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) ≤ s.chan (Chan.wire p h))
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c + recvdOf sk s c = sentOf sk s c →
        s'.chan c + recvdOf sk s' c = sentOf sk s' c)
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c ≤ sk.cap c → s'.chan c ≤ sk.cap c) := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
    obtain ⟨⟨hmem, hph⟩, hcaplt⟩ := hg
    injection hstep with hs'
    have hpkmem : pk ∈ sk.asmKeys := List.contains_iff_mem.mp hmem
    have hold := hi.asm pk hpkmem
    simp only [asmLocalOk, hph] at hold
    simp at hold
    have hchan : s'.chan = bump s.chan (sk.asmOutChan pk) 1 := by
      rw [← hs']; rfl
    have hrecv : ∀ c, recvdOf sk s' c = recvdOf sk s c := by
      intro c
      rw [← hs']
      refine Eq.trans (recvdOf_setAsm_of_counts sk _ pk _ ?_ ?_ c) ?_
      · simp only [asmResRecvd, setAsm_asm_self]
        by_cases hlt2 : (s.asm pk).idx + 1
            < (sk.asmResList pk.1 pk.2).length
        · simp [hlt2, hph]
        · simp [hlt2, hph]
      · simp only [asmLevelRecvd, setAsm_asm_self]
        rw [pendsBefore_succ sk pk.1 pk.2 (s.asm pk).idx hold.1]
        omega
      · exact recvdOf_ext sk (fun _ => rfl) (fun _ => rfl)
          (fun _ => rfl) rfl rfl rfl rfl rfl rfl c
    have hsentw : ∀ (p : Party) (h : Nat),
        sentOf sk s' (Chan.wire p h)
          = sentOf sk s (Chan.wire p h) := by
      intro p h
      rw [← hs']
      rfl
    refine ⟨fun p h => hsentw p h, ?_, ?_, ?_, ?_⟩
    · intro p h
      rw [hchan, bump_ne _ _ (wire_ne_asmOutChan sk p h pk), hrecv]
    · intro p h
      exact Nat.le_of_eq
        (by rw [hchan, bump_ne _ _ (wire_ne_asmOutChan sk p h pk)])
    · intro c hc _ heq
      by_cases hcc : c = sk.asmOutChan pk
      · subst hcc
        by_cases h1 : (pk.1 == Party.I && pk.2 == sk.rootH) = true
        · have hpkI : pk = (Party.I, sk.rootH) := by
            simp only [Bool.and_eq_true, beq_iff_eq] at h1
            exact Prod.ext h1.1 h1.2
          have hch : sk.asmOutChan pk = Chan.rootret := by
            unfold Skel.asmOutChan
            rw [if_pos h1]
          have hsentS : sentOf sk s (sk.asmOutChan pk)
              = (s.asm pk).idx := by
            rw [hch]
            show asmOutSent s (Party.I, sk.rootH) = _
            rw [← hpkI]
            rfl
          have hsentS' : sentOf sk s' (sk.asmOutChan pk)
              = (s.asm pk).idx + 1 := by
            rw [← hs', hch]
            show asmOutSent (setAsm _ pk _) (Party.I, sk.rootH) = _
            rw [← hpkI]
            simp [asmOutSent]
          rw [hchan, bump_one, hrecv, hsentS']
          rw [hsentS] at heq
          omega
        · by_cases h2 : (pk.1 == Party.R && pk.2 == sk.rootH - 1)
              = true
          · have hpkR : pk = (Party.R, sk.rootH - 1) := by
              simp only [Bool.and_eq_true, beq_iff_eq] at h2
              exact Prod.ext h2.1 h2.2
            have hch : sk.asmOutChan pk = Chan.rootrets := by
              unfold Skel.asmOutChan
              rw [if_neg h1, if_pos h2]
            have hsentS : sentOf sk s (sk.asmOutChan pk)
                = (s.asm pk).idx := by
              rw [hch]
              show asmOutSent s (Party.R, sk.rootH - 1) = _
              rw [← hpkR]
              rfl
            have hsentS' : sentOf sk s' (sk.asmOutChan pk)
                = (s.asm pk).idx + 1 := by
              rw [← hs', hch]
              show asmOutSent (setAsm _ pk _) (Party.R, sk.rootH - 1)
                = _
              rw [← hpkR]
              simp [asmOutSent]
            rw [hchan, bump_one, hrecv, hsentS']
            rw [hsentS] at heq
            omega
          · have hch : sk.asmOutChan pk = Chan.level pk.1 pk.2 := by
              unfold Skel.asmOutChan
              rw [if_neg h1, if_neg h2]
            have hroot : isRootOutKey sk pk = false := by
              rw [Bool.eq_false_iff]
              intro hr
              simp only [isRootOutKey, Bool.or_eq_true] at hr
              rcases hr with hr | hr
              · exact h1 hr
              · exact h2 hr
            have hnot0 : ¬((pk.1 == Party.I && pk.2 == (0 : Nat))
                = true) := by
              simp only [Bool.and_eq_true, beq_iff_eq]
              rintro ⟨-, h0⟩
              have hpk2 : 1 ≤ pk.2 := asmKeys_snd_pos hmem
              omega
            have hcond : (sk.asmKeys.contains (pk.1, pk.2)
                && !isRootOutKey sk (pk.1, pk.2)) = true := by
              rw [show ((pk.1, pk.2) : Party × Nat) = pk from rfl,
                hmem, hroot]
              rfl
            have hsentS : sentOf sk s (sk.asmOutChan pk)
                = (s.asm pk).idx := by
              rw [hch]
              show sentOf sk s (Chan.level pk.1 pk.2) = _
              simp only [sentOf]
              rw [if_neg hnot0, if_pos hcond]
              rfl
            have hsentS' : sentOf sk s' (sk.asmOutChan pk)
                = (s.asm pk).idx + 1 := by
              rw [← hs', hch]
              show sentOf sk (setAsm _ pk _) (Chan.level pk.1 pk.2)
                = _
              simp only [sentOf]
              rw [if_neg hnot0, if_pos hcond]
              show asmOutSent (setAsm _ pk _) (pk.1, pk.2) = _
              rw [show ((pk.1, pk.2) : Party × Nat) = pk from rfl]
              simp [asmOutSent]
            rw [hchan, bump_one, hrecv, hsentS']
            rw [hsentS] at heq
            omega
      · have hsent : sentOf sk s' c = sentOf sk s c := by
          rw [← hs']
          refine Eq.trans (sentOf_setAsm_frame _ pk _ hcc) ?_
          exact sentOf_ext sk (fun _ => rfl) (fun _ => rfl)
            (fun _ => rfl) (fun _ => rfl) (fun _ => rfl)
            (fun _ => rfl) (fun _ => rfl) rfl rfl rfl rfl rfl rfl c
        rw [hchan, bump_ne _ _ hcc, hsent, hrecv]
        exact heq
    · intro c _ _ hcap
      rw [hchan]
      by_cases hcc : c = sk.asmOutChan pk
      · subst hcc; rw [bump_one]; omega
      · rw [bump_ne _ _ hcc]; exact hcap

-- ============================================================ asmClose

/-- `asmClose` preserves the local fragment: phase 3 → 4 and nothing
else.

Extracted from `Model.preserve_asmClose` (Proofs/Preserve/Asm.lean):
guard inversion plus the wk/asm/top bullets; the flow bullet is
dropped. -/
theorem preserveL_asmClose (_hwf : sk.wellFormed = true)
    (pk : Party × Nat)
    (hstep : Model.apply sk ax (.asmClose pk) s = some s')
    (hi : InvL sk ax s) : InvL sk ax s' := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq] at hg
    obtain ⟨⟨⟨_hmem, hph⟩, _hpd⟩, _hch0⟩ := hg
    injection hstep with hs'
    refine ⟨fun pk' hpk' => ?_, fun pk' hpk' => ?_, ?_⟩
    · rw [← hs']; exact hi.wk pk' hpk'
    · by_cases hpkeq : pk' = pk
      · subst hpkeq
        have hasm : s'.asm pk' = { s.asm pk' with phase := 4 } := by
          rw [← hs']; simp
        have hold := hi.asm pk' hpk'
        simp only [asmLocalOk, hasm, hph] at hold ⊢
        simp at hold ⊢
        omega
      · have ha : s'.asm pk' = s.asm pk' := by
          rw [← hs']; exact setAsm_asm_ne s _ hpkeq
        rw [asmLocalOk_congr sk pk' ha]; exact hi.asm pk' hpk'
    · rw [← hs']; exact hi.top

/-- Counting deltas of `asmClose`: a pure phase move, every count
frames.

Extracted from the flow bullet of `Model.preserve_asmClose`: both
consumer counts read `phase == 1 || phase == 2` (false on both sides
of 3 → 4) or only the cursor, and `chan` is untouched. -/
theorem delta_asmClose (_hwf : sk.wellFormed = true)
    (pk : Party × Nat)
    (hstep : Model.apply sk ax (.asmClose pk) s = some s')
    (_hi : InvL sk ax s) :
    (∀ p h, sentOf sk s' (Chan.wire p h) = sentOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) + recvdOf sk s' (Chan.wire p h)
        = s.chan (Chan.wire p h) + recvdOf sk s (Chan.wire p h))
    ∧ (∀ p h, s'.chan (Chan.wire p h) ≤ s.chan (Chan.wire p h))
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c + recvdOf sk s c = sentOf sk s c →
        s'.chan c + recvdOf sk s' c = sentOf sk s' c)
    ∧ (∀ c ∈ allChans sk, isWire c = false →
        s.chan c ≤ sk.cap c → s'.chan c ≤ sk.cap c) := by
  simp only [Model.apply] at hstep
  split at hstep
  case isFalse => simp at hstep
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq] at hg
    obtain ⟨⟨⟨_hmem, hph⟩, _hpd⟩, _hch0⟩ := hg
    injection hstep with hs'
    have hchan : s'.chan = s.chan := by rw [← hs']; rfl
    have hsent : ∀ c, sentOf sk s' c = sentOf sk s c := by
      intro c
      rw [← hs']
      apply sentOf_ext_idx
      · intro pk'
        by_cases hq : pk' = pk
        · subst hq; simp
        · simp [setAsm_asm_ne s _ hq]
      all_goals rfl
    have hrecv : ∀ c, recvdOf sk s' c = recvdOf sk s c := by
      intro c
      rw [← hs']
      apply recvdOf_setAsm_of_counts
      · simp [asmResRecvd, hph]
      · simp [asmLevelRecvd]
    exact ⟨fun p h => hsent _,
      fun p h => by rw [hchan, hrecv],
      fun p h => Nat.le_of_eq (congrFun hchan _),
      fun c _ _ heq => by rw [hchan, hsent, hrecv]; exact heq,
      fun c _ _ hcap => by rw [hchan]; exact hcap⟩

end StreamingMirror.Mux
