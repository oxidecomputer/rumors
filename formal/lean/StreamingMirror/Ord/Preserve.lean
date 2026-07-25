/-
The O induction, assembled: every action of every process preserves
`InvPO` at EVERY per-loop dequeue-order assignment, hence the O
invariant holds at every `ReachableO` state. The fifteen shared arms
dispatch to the base-arm twins (each definitionally a base step); the
eight re-phased arms dispatch to the order-split twins. This is the
ord counterpart of the base `inv_preserved`/`inv_reachable` pair, and
the platform for the quantified flagship (the O progress layer
consumes `invO_reachable`).

Chain (ord, stage B): assembles `invO_reachable` from Ord/Init.lean
and the per-family O preservation files (Ord/Preserve/); consumed by
the O progress/flagship layers. Base mirror: Proofs/Preserve.lean.
Map: PROGRESS.md §13.
-/
import StreamingMirror.Ord.Init
import StreamingMirror.Ord.Preserve.Top
import StreamingMirror.Ord.Preserve.Walk
import StreamingMirror.Ord.Preserve.WalkFire
import StreamingMirror.Ord.Preserve.Asm
import StreamingMirror.Ord.Preserve.AbsorbFin

namespace StreamingMirror.Ord

open Model

variable {sk : Skel} {ax : AxMode} {ord : OrdMap} {s s' : State}

/-- O consecution: every action of every process preserves `InvPO`, at
every assignment of the two-point per-loop dequeue-order class. -/
theorem invO_preserved (hwf : sk.wellFormed = true) (a : Action)
    (hstep : applyO sk ax ord a s = some s') (hi : InvPO sk ax ord s) :
    InvPO sk ax ord s' := by
  cases a with
  | iopenChoose o => exact preserve_iopenChooseO hwf o hstep hi
  | iopenFire => exact preserve_iopenFireO hwf hstep hi
  | ropenRecv => exact preserve_ropenRecvO hwf hstep hi
  | ropenChoose o => exact preserve_ropenChooseO hwf o hstep hi
  | ropenFire => exact preserve_ropenFireO hwf hstep hi
  | walkRecvWire pk => exact preserve_walkRecvWireO hwf pk hstep hi
  | walkRecvAsked pk => exact preserve_walkRecvAskedO hwf pk hstep hi
  | walkCommit pk o => exact preserve_walkCommitO hwf pk o hstep hi
  | walkFire pk => exact preserve_walkFireO hwf pk hstep hi
  | walkCloseWire pk => exact preserve_walkCloseWireO hwf pk hstep hi
  | walkCloseAsked pk => exact preserve_walkCloseAskedO hwf pk hstep hi
  | asmRecvRes pk => exact preserve_asmRecvResO hwf pk hstep hi
  | asmRecvLevel pk => exact preserve_asmRecvLevelO hwf pk hstep hi
  | asmSend pk => exact preserve_asmSendO hwf pk hstep hi
  | asmClose pk => exact preserve_asmCloseO hwf pk hstep hi
  | absorbRecvWire => exact preserve_absorbRecvWireO hwf hstep hi
  | absorbRecvAsked => exact preserve_absorbRecvAskedO hwf hstep hi
  | absorbSend => exact preserve_absorbSendO hwf hstep hi
  | absorbCloseWire => exact preserve_absorbCloseWireO hwf hstep hi
  | absorbCloseAsked => exact preserve_absorbCloseAskedO hwf hstep hi
  | finRet => exact preserve_finRetO hwf hstep hi
  | finRes => exact preserve_finResO hwf hstep hi
  | finRets => exact preserve_finRetsO hwf hstep hi

/-- The O inductive invariant holds at every `ReachableO` state of
every well-formed skeleton, in every axiom mode, at every assignment
of the two-point per-loop dequeue-order class. -/
theorem invO_reachable (hwf : sk.wellFormed = true)
    (hr : ReachableO sk ax ord s) : InvPO sk ax ord s := by
  induction hr with
  | init => exact invO_init sk ax ord
  | step a _ hstep ih => exact invO_preserved hwf a hstep ih

end StreamingMirror.Ord
