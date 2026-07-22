/-
Drift guards: the executable tier runs the kernel objects.

Three definitions exist twice, once on the statement path and once in
the executable tier (Mux/Gen.lean serves `lake exe muxprobe`, and
nothing a theorem of record quantifies over may live there; the
unbounded-slot control predates the elastic semantics). Twins invite
silent drift — an edit to one side would quietly decouple the
executable evidence from the kernel objects it vouches for — so each
pair is pinned equal here, definitionally. A pin failing to compile is
the drift alarm.

The K-dial relation `deliverStepK_one` (depths (1, 1) ARE the record
deliver arm) lives with its definition in
Proofs/WcImpossibilityK.lean; the dial's map is
Mux/Proofs/Map.lean.
-/
import StreamingMirror.Mux.Gen
import StreamingMirror.Mux.Controls
import StreamingMirror.Mux.Elastic
import StreamingMirror.Mux.Proofs.WcImpossibilityK
import StreamingMirror.Mux.Proofs.Oracle.Controls

namespace StreamingMirror.Mux

/-- The executable demand-order projection IS the kernel `demandOrder`
(Oracle/Controls.lean, the refuted-candidate receive projection that
`static_oracle_jams` consumes): muxprobe's `demand` matrix entry runs
the object the kernel pin is about. -/
theorem piOrder_eq_demandOrder : Gen.piOrder = demandOrder := rfl

/-- The executable wedge family IS the statement-path `wedgeW` at its
default root height (6, the family's kernel anchors' height): the
muxprobe wedge rows and the capacity-flat sweep run the shapes
`wc_impossibility_K` quantifies over, and `wedgeW_six` ties width 6 to
the `wedge` literal itself. -/
theorem wedgeFam_eq_wedgeW (w : Nat) : Gen.wedgeFam w = wedgeW w := rfl

/-- The unbounded-slot control variant IS the elastic semantics: the
option-C escape control (`Control.applyU`, minted before the elastic
composition existed as a semantics of record) and `applyE` are one
definition, so `wedge_unboundedSlot_completes` and
`wedge_elastic_completes` pin the same system. -/
theorem applyU_eq_applyE : Control.applyU = applyE := rfl

end StreamingMirror.Mux
