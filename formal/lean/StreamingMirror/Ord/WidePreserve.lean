/-
The wide-O induction, assembled: every action of every process
preserves the weak O invariant `InvPWO` along `applyWO`, at EVERY
pointwise capacity vector κ and EVERY per-loop dequeue-order
assignment of the two-point class — hence `InvPWO` holds at every
`ReachableWO` state. This is the one link Ord/Wide.lean left open
(`progressWO`'s explicit hypothesis): with it, the widened flagship
assembles in Ord/WideEndgame.lean.

The dispatch mirrors Ord/Preserve.lean's shape over the minus-cap arm
family (Ord/WidePreserve/): the eighteen non-push arms are `applyO`'s
verbatim, the five κ-guarded pushes never consume the guard value —
conservation is capacity-blind, which is why the weak invariant is
inductive at every κ at all. The induction base is `invO_init`'s
weakening: `InvPWO` carries no capacity clause, so the floor
initialization covers every κ.

Chain (ord, stage G): consumes the Ord/WidePreserve/ arm files and
Ord/Init.lean; concludes `invPWO_preservedWO`/`invPWO_reachableWO`,
consumed by Ord/WideEndgame.lean. Base mirror: Proofs/Wide.lean
(`invPW_preserved_W`/`invPW_reachableW`). Map: PROGRESS.md §13.
-/
import StreamingMirror.Ord.WidePreserve.Top
import StreamingMirror.Ord.WidePreserve.Walk
import StreamingMirror.Ord.WidePreserve.WalkFire
import StreamingMirror.Ord.WidePreserve.Asm
import StreamingMirror.Ord.WidePreserve.AbsorbFin

namespace StreamingMirror.Ord

open Model

variable {sk : Skel} {κ : Chan → Nat} {ax : AxMode} {ord : OrdMap} {s s' : State}

/-- Wide-O consecution: every action of every process preserves the
weak O invariant along `applyWO`, at every capacity vector κ and every
assignment of the two-point per-loop dequeue-order class. -/
theorem invPWO_preservedWO (hwf : sk.wellFormed = true) (a : Action)
    (hstep : applyWO sk κ ax ord a s = some s') (hi : InvPWO sk ax ord s) :
    InvPWO sk ax ord s' := by
  cases a with
  | iopenChoose o => exact preserve_iopenChooseWO hwf o hstep hi
  | iopenFire => exact preserve_iopenFireWO hwf hstep hi
  | ropenRecv => exact preserve_ropenRecvWO hwf hstep hi
  | ropenChoose o => exact preserve_ropenChooseWO hwf o hstep hi
  | ropenFire => exact preserve_ropenFireWO hwf hstep hi
  | walkRecvWire pk => exact preserve_walkRecvWireWO hwf pk hstep hi
  | walkRecvAsked pk => exact preserve_walkRecvAskedWO hwf pk hstep hi
  | walkCommit pk o => exact preserve_walkCommitWO hwf pk o hstep hi
  | walkFire pk => exact preserve_walkFireWO hwf pk hstep hi
  | walkCloseWire pk => exact preserve_walkCloseWireWO hwf pk hstep hi
  | walkCloseAsked pk => exact preserve_walkCloseAskedWO hwf pk hstep hi
  | asmRecvRes pk => exact preserve_asmRecvResWO hwf pk hstep hi
  | asmRecvLevel pk => exact preserve_asmRecvLevelWO hwf pk hstep hi
  | asmSend pk => exact preserve_asmSendWO hwf pk hstep hi
  | asmClose pk => exact preserve_asmCloseWO hwf pk hstep hi
  | absorbRecvWire => exact preserve_absorbRecvWireWO hwf hstep hi
  | absorbRecvAsked => exact preserve_absorbRecvAskedWO hwf hstep hi
  | absorbSend => exact preserve_absorbSendWO hwf hstep hi
  | absorbCloseWire => exact preserve_absorbCloseWireWO hwf hstep hi
  | absorbCloseAsked => exact preserve_absorbCloseAskedWO hwf hstep hi
  | finRet => exact preserve_finRetWO hwf hstep hi
  | finRes => exact preserve_finResWO hwf hstep hi
  | finRets => exact preserve_finRetsWO hwf hstep hi

/-- The weak O invariant holds at the initial state, every assignment:
the full O invariant does (`invO_init`), and weakens. `InvPWO` carries
no capacity clause, so this one base covers every κ. -/
theorem invPWO_init (sk : Skel) (ax : AxMode) (ord : OrdMap) :
    InvPWO sk ax ord (init sk) :=
  (invO_init sk ax ord).weak

/-- The weak O invariant holds at every wide-O-reachable state of
every well-formed skeleton, at every κ and every assignment of the
two-point class. -/
theorem invPWO_reachableWO (hwf : sk.wellFormed = true)
    (hr : ReachableWO sk κ ax ord s) : InvPWO sk ax ord s := by
  induction hr with
  | init => exact invPWO_init sk ax ord
  | step a _ hstep ih => exact invPWO_preservedWO hwf a hstep ih

end StreamingMirror.Ord
