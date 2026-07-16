/-
The induction, assembled: every action preserves the invariant, hence
the invariant holds at every reachable state. This is the parametric
counterpart of Apalache's per-family consecution — for EVERY well-formed
skeleton, not a fixed small one — and the platform for `deadlock_free`
(the progress lemma consumes `inv_reachable`).
-/
import StreamingMirror.Proofs.Init
import StreamingMirror.Proofs.Preserve.Top
import StreamingMirror.Proofs.Preserve.Walk
import StreamingMirror.Proofs.Preserve.WalkFire
import StreamingMirror.Proofs.Preserve.Asm
import StreamingMirror.Proofs.Preserve.AbsorbFin

namespace StreamingMirror.Model

variable {sk : Skel} {ax : AxMode} {s s' : State}

/-- Consecution: every action of every process preserves `InvP`. -/
theorem inv_preserved (hwf : sk.wellFormed = true) (a : Action)
    (hstep : apply sk ax a s = some s') (hi : InvP sk ax s) :
    InvP sk ax s' := by
  cases a with
  | iopenChoose o => exact preserve_iopenChoose hwf o hstep hi
  | iopenFire => exact preserve_iopenFire hwf hstep hi
  | ropenRecv => exact preserve_ropenRecv hwf hstep hi
  | ropenChoose o => exact preserve_ropenChoose hwf o hstep hi
  | ropenFire => exact preserve_ropenFire hwf hstep hi
  | walkRecvWire pk => exact preserve_walkRecvWire hwf pk hstep hi
  | walkRecvAsked pk => exact preserve_walkRecvAsked hwf pk hstep hi
  | walkCommit pk o => exact preserve_walkCommit hwf pk o hstep hi
  | walkFire pk => exact preserve_walkFire hwf pk hstep hi
  | walkCloseWire pk => exact preserve_walkCloseWire hwf pk hstep hi
  | walkCloseAsked pk => exact preserve_walkCloseAsked hwf pk hstep hi
  | asmRecvRes pk => exact preserve_asmRecvRes hwf pk hstep hi
  | asmRecvLevel pk => exact preserve_asmRecvLevel hwf pk hstep hi
  | asmSend pk => exact preserve_asmSend hwf pk hstep hi
  | asmClose pk => exact preserve_asmClose hwf pk hstep hi
  | absorbRecvWire => exact preserve_absorbRecvWire hwf hstep hi
  | absorbRecvAsked => exact preserve_absorbRecvAsked hwf hstep hi
  | absorbSend => exact preserve_absorbSend hwf hstep hi
  | absorbCloseWire => exact preserve_absorbCloseWire hwf hstep hi
  | absorbCloseAsked => exact preserve_absorbCloseAsked hwf hstep hi
  | finRet => exact preserve_finRet hwf hstep hi
  | finRes => exact preserve_finRes hwf hstep hi
  | finRets => exact preserve_finRets hwf hstep hi

/-- The inductive invariant holds at every reachable state of every
well-formed skeleton, in every axiom mode. -/
theorem inv_reachable (hwf : sk.wellFormed = true)
    (hr : Reachable sk ax s) : Inv sk ax s = true := by
  induction hr with
  | init => exact inv_init sk ax
  | step a _ hstep ih =>
      rw [inv_iff] at ih ⊢
      exact inv_preserved hwf a hstep ih

end StreamingMirror.Model
