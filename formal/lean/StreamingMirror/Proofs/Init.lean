/-
The base case of the induction: `Inv` holds at `init`, for EVERY
skeleton — well-formedness is not needed, because `init` is
self-consistent by construction (each process's phase conditional
matches its own emptiness test, and every channel and every derived
count is zero).

Structure: projection lemmas keep `init sk` folded so the pointwise
count lemmas (`*_init`) match syntactically; `inv_iff` glues the four
`InvP` fields into the theorem.

Chain (shared foundation): the induction's base case, consumed by
Preserve.lean's assembly of `inv_reachable`. Map: Proofs/Map.lean.
-/
import StreamingMirror.Proofs.Lemmas

namespace StreamingMirror.Model

variable (sk : Skel) (ax : AxMode)

-- ==================================================== init projections

@[simp] theorem walk_init (pk : Party × Nat) :
    (init sk).walk pk = freshWalk sk pk.2 0 := rfl

@[simp] theorem asm_init (pk : Party × Nat) :
    (init sk).asm pk =
      ⟨0, if (sk.asmResList pk.1 pk.2).length > 0 then 0 else 3, 0⟩ := rfl

@[simp] theorem chan_init (c : Chan) : (init sk).chan c = 0 := rfl

@[simp] theorem iopenWire_init : (init sk).iopenWire = false := rfl
@[simp] theorem iopenQuery_init : (init sk).iopenQuery = false := rfl
@[simp] theorem iopenCh_init : (init sk).iopenCh = none := rfl
@[simp] theorem ropenGotWire_init : (init sk).ropenGotWire = false := rfl
@[simp] theorem ropenWire_init : (init sk).ropenWire = false := rfl
@[simp] theorem ropenRes_init : (init sk).ropenRes = false := rfl
@[simp] theorem ropenQ_init : (init sk).ropenQ = 0 := rfl
@[simp] theorem ropenCh_init : (init sk).ropenCh = none := rfl
@[simp] theorem absorbIdx_init : (init sk).absorbIdx = 0 := rfl
@[simp] theorem absorbPhase_init :
    (init sk).absorbPhase = if sk.totalLeafReqs > 0 then 0 else 3 := rfl
@[simp] theorem ifin_init : (init sk).ifin = false := rfl
@[simp] theorem rfinGotRes_init : (init sk).rfinGotRes = false := rfl
@[simp] theorem rfinGot_init : (init sk).rfinGot = 0 := rfl

-- ================================================ derived counts at init

theorem wkWireRecvd_init (pk : Party × Nat) :
    wkWireRecvd sk (init sk) pk = 0 := by
  by_cases hl : 0 < sk.stageLen pk.2
  · simp [wkWireRecvd, freshWalk, hl]
  · have hz : sk.stageLen pk.2 = 0 := by omega
    simp [wkWireRecvd, freshWalk, hz]

theorem wkAskedRecvd_init (pk : Party × Nat) :
    wkAskedRecvd sk (init sk) pk = 0 := by
  by_cases hl : 0 < sk.stageLen pk.2
  · simp [wkAskedRecvd, freshWalk, hl]
  · have hz : sk.stageLen pk.2 = 0 := by omega
    simp [wkAskedRecvd, freshWalk, hz]

theorem wkWireCount_init (pk : Party × Nat) :
    wkWireCount sk (init sk) pk = 0 := by
  simp [wkWireCount, freshWalk]

theorem wkResCount_init (pk : Party × Nat) :
    wkResCount sk (init sk) pk = 0 := by
  simp [wkResCount, freshWalk]

theorem wkQSum_init (pk : Party × Nat) :
    wkQSum sk (init sk) pk = 0 := by
  simp [wkQSum, freshWalk, foldl_const]

theorem wkWireSent_init (pk : Party × Nat) :
    wkWireSent sk (init sk) pk = 0 := by
  simp [wkWireSent, Skel.wiresBefore, wkWireCount_init, freshWalk]

theorem wkResSent_init (pk : Party × Nat) :
    wkResSent sk (init sk) pk = 0 := by
  simp [wkResSent, Skel.dsBefore, wkResCount_init, freshWalk]

theorem wkQSentTot_init (pk : Party × Nat) :
    wkQSentTot sk (init sk) pk = 0 := by
  simp [wkQSentTot, Skel.qsBefore, wkQSum_init, freshWalk]

theorem wkParentSent_init (pk : Party × Nat) :
    wkParentSent (init sk) pk = 0 := by
  simp [wkParentSent, freshWalk]

theorem asmOutSent_init (pk : Party × Nat) :
    asmOutSent (init sk) pk = 0 := rfl

theorem asmResRecvd_init (pk : Party × Nat) :
    asmResRecvd (init sk) pk = 0 := by
  simp [asmResRecvd]
  split <;> simp

theorem asmLevelRecvd_init (pk : Party × Nat) :
    asmLevelRecvd sk (init sk) pk = 0 := by
  simp [asmLevelRecvd, Skel.pendsBefore]

theorem absorbWireRecvd_init : absorbWireRecvd sk (init sk) = 0 := by
  by_cases h : sk.totalLeafReqs > 0
  · simp [absorbWireRecvd, h]
  · simp [absorbWireRecvd, h]; omega

theorem absorbAskedRecvd_init : absorbAskedRecvd sk (init sk) = 0 := by
  by_cases h : sk.totalLeafReqs > 0
  · simp [absorbAskedRecvd, h]
  · simp [absorbAskedRecvd, h]; omega

/-- Every producer count is zero at init. -/
theorem sentOf_init (c : Chan) : sentOf sk (init sk) c = 0 := by
  cases c <;>
    simp [sentOf, b2n, wkWireSent_init, wkResSent_init, wkQSentTot_init,
      wkParentSent_init, asmOutSent_init]

/-- Every consumer count is zero at init. -/
theorem recvdOf_init (c : Chan) : recvdOf sk (init sk) c = 0 := by
  cases c <;>
    simp [recvdOf, b2n, wkWireRecvd_init, wkAskedRecvd_init,
      asmResRecvd_init, asmLevelRecvd_init, absorbWireRecvd_init,
      absorbAskedRecvd_init]

-- ================================================== local invariants

theorem wkLocalOk_init (pk : Party × Nat) :
    wkLocalOk sk ax (init sk) pk = true := by
  by_cases hl : 0 < sk.stageLen pk.2
  · simp [wkLocalOk, freshWalk, hl]
  · have hz : sk.stageLen pk.2 = 0 := by omega
    simp [wkLocalOk, freshWalk, hz]

theorem asmLocalOk_init (pk : Party × Nat) :
    asmLocalOk sk (init sk) pk = true := by
  by_cases h : (sk.asmResList pk.1 pk.2).length > 0
  · simp [asmLocalOk, h]
  · simp [asmLocalOk, h]; omega

theorem topLocalOk_init : topLocalOk sk ax (init sk) = true := by
  by_cases h : sk.totalLeafReqs > 0
  · simp [topLocalOk, h]
  · simp [topLocalOk, h]; omega

-- ========================================================== the theorem

/-- `Inv` holds initially, for every skeleton and axiom mode. -/
theorem inv_init : Inv sk ax (init sk) = true := by
  rw [inv_iff]
  refine ⟨fun pk _ => wkLocalOk_init sk ax pk,
          fun pk _ => asmLocalOk_init sk pk,
          topLocalOk_init sk ax,
          fun c _ => ?_⟩
  simp [sentOf_init, recvdOf_init]

end StreamingMirror.Model
