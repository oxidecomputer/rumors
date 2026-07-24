/-
The O invariant holds initially, for every assignment: at `init` both
receive formulas of every pairing loop agree (nothing is received in
either order), so `recvdOfO` collapses to `recvdOf` and the base
initialization theorem carries the flow field.

Chain (ord, stage A): the induction base; consumed by Ord/Preserve's
`invO_reachable`. Base mirror: Proofs/Init.lean. Map: PROGRESS.md §13.
-/
import StreamingMirror.Proofs.Init
import StreamingMirror.Ord.Wiring

namespace StreamingMirror.Ord

open Model

variable (sk : Skel) (ax : AxMode) (ord : OrdMap)

/-- At `init` the O receive counts agree with the base counts on every
channel: both of a loop's formulas read `0` (live stage) or `stageLen`
(empty stage) before anything moves. -/
theorem recvdOfO_init (c : Chan) : recvdOfO sk ord (init sk) c = 0 := by
  have hwk : ∀ pk : Party × Nat, wkWireRecvdO sk ord (init sk) pk = 0 := by
    intro pk
    cases hord : ord.walk pk <;>
      simp [wkWireRecvdO, hord, wkWireRecvd_init, wkAskedRecvd_init]
  have hak : ∀ pk : Party × Nat, wkAskedRecvdO sk ord (init sk) pk = 0 := by
    intro pk
    cases hord : ord.walk pk <;>
      simp [wkAskedRecvdO, hord, wkWireRecvd_init, wkAskedRecvd_init]
  have habw : absorbWireRecvdO sk ord (init sk) = 0 := by
    cases hord : ord.absorb <;>
      simp [absorbWireRecvdO, hord, absorbWireRecvd_init, absorbAskedRecvd_init]
  have haba : absorbAskedRecvdO sk ord (init sk) = 0 := by
    cases hord : ord.absorb <;>
      simp [absorbAskedRecvdO, hord, absorbWireRecvd_init, absorbAskedRecvd_init]
  cases c with
  | wire p h =>
      by_cases hr : h = sk.rootH
      · subst hr
        cases p with
        | I => simp [recvdOfO, b2n]
        | R => simp [recvdOfO, hwk]
      · by_cases hh : p = Party.R ∧ h = 0
        · obtain ⟨rfl, rfl⟩ := hh
          simpa [recvdOfO, hr] using habw
        · rcases Decidable.not_and_iff_not_or_not.mp hh with hp | h0
          · cases p
            · simp [recvdOfO, hr, hwk]
            · exact absurd rfl hp
          · cases p <;> simp [recvdOfO, hr, h0, hwk]
  | asked p h => simpa [recvdOfO] using hak (p, h)
  | leafRequests => simpa [recvdOfO] using haba
  | upper p h => simp [recvdOfO, recvdOf_init]
  | lower p h => simp [recvdOfO, recvdOf_init]
  | level p j => simp [recvdOfO, recvdOf_init]
  | rootret => simp [recvdOfO, recvdOf_init]
  | rootrets => simp [recvdOfO, recvdOf_init]
  | rootres => simp [recvdOfO, recvdOf_init]

/-- The O invariant at the initial state, every assignment. -/
theorem invO_init : InvPO sk ax ord (init sk) := by
  have hbase := (inv_iff sk ax (init sk)).mp (inv_init sk ax)
  exact ⟨hbase.wk, hbase.asm, hbase.top, fun c hc => by
    have := hbase.flow c hc
    rw [recvdOfO_init]
    rwa [recvdOf_init sk c] at this⟩

end StreamingMirror.Ord
