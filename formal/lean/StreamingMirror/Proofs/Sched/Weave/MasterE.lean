/-
The E consumption induction (unit 2a, PROGRESS.md §9): `weaveGo_wedge`'s
twin over the encoder-order interpreter — the edge invariant at the
`procsE` family rides `weaveGoE`, each manual guard discharged from the
pointwise readiness property (`EmitOKOnP` at `procsE`), the precedence
layer, and the pump fixpoint the previous emission left behind.

The readiness property itself — discharging `EmitOKOnP` at every
position of the eweave's future, where the U-sites consume the margin-0
capacity hypothesis — is unit 2b, the E master induction. This file
only carries the generic consumption frame that turns that readiness
into `WEdgeP sk (procsE sk) [] (weaveStateE sk)`.
-/
import StreamingMirror.Proofs.Sched.Weave.Master
import StreamingMirror.Proofs.Sched.Weave.ExpandE

namespace StreamingMirror.Sched

open Model

variable (sk : Skel)

/-- THE E CONSUMPTION INDUCTION: `weaveGo_wedge` over the encoder-order
expanders, at the encoder-order family. -/
theorem weaveGoE_wedge (fuel : Nat) :
    ∀ (ops : List WOp) (st : MState) (done : List Ev),
      WEdgeP sk (procsE sk) (goEventsE sk fuel ops) st →
      DepOK done (goEventsE sk fuel ops) →
      (∀ x ∈ done, x ∈ st.out) →
      EmitOKOnP sk (procsE sk) (goEventsE sk fuel ops) [] →
      step sk st = none →
      WEdgeP sk (procsE sk) [] (weaveGoE sk fuel ops st) := by
  induction fuel with
  | zero => intro ops st done hW _ _ _ _; exact hW
  | succ f ih =>
      intro ops st done hW hdep hdone hemit hfix
      match ops with
      | [] => exact hW
      | .emit e :: rest =>
          have hgo : goEventsE sk (f + 1) (.emit e :: rest)
              = e :: goEventsE sk f rest := rfl
          rw [hgo] at hW hdep hemit
          have hen : enabled sk st.sent st.rcvd e = true := by
            refine hemit 0 e rfl st (by simpa using hW) hfix ?_
            intro d hd
            exact hdone d (depOK_head hdep d hd)
          show WEdgeP sk (procsE sk) []
            (weaveGoE sk f rest (wEmitP sk st e))
          refine ih rest (wEmitP sk st e) (done ++ [e])
            (wEdge_emitP sk hen hW) (depOK_tail hdep) ?_
            (emitOKOn_tail sk hemit) (wPump_fixpoint sk _)
          intro x hx
          rcases List.mem_append.1 hx with hx | hx
          · exact mem_out_wEmitP sk
              (List.mem_append_left _ (hdone x hx))
          · have hxe : x = e := List.mem_singleton.1 hx
            subst hxe
            exact mem_out_wEmitP sk
              (List.mem_append_right _ (List.mem_cons_self ..))
      | .scope h' k feed :: rest =>
          exact ih _ st done hW hdep hdone hemit hfix
      | .kid h' k s lastD kidBase i feed :: rest =>
          exact ih _ st done hW hdep hdone hemit hfix

end StreamingMirror.Sched
