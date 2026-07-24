/-
The O pump family: `weavePumps` with the absorber row in ITS assigned
order. One definition and its two pins, split out so the pump-layer
bundle (Ord/Pump.lean) and the O weave (Ord/Weave.lean) can consume
them without owning each other.

Chain (ord, stage D): the pump family; consumed by Ord/Pump and
Ord/Weave. Base mirror: Proofs/Sched/Weave.lean's `weavePumps`. Map:
PROGRESS.md §13.
-/
import StreamingMirror.Ord.Numbering
import StreamingMirror.Proofs.Sched.Weave

namespace StreamingMirror.Ord

open Model Sched

variable (sk : Skel) (ord : OrdMap)

/-- The pump traces under the assignment: `weavePumps` with the
absorber row in `ord.absorb`'s order; every other row (asm towers,
the rootret floater, fins) is the base row. -/
def weavePumpsO : List (List Ev) :=
  [absorbEventsO sk ord]
    ++ sk.asmKeys.map (asmEvents sk)
    ++ [[(Chan.rootret, false, 0)], finEvents sk]

/-- The reply-first collapse: the O pump family IS `weavePumps`. -/
theorem weavePumpsO_rf : weavePumpsO sk .rf = weavePumps sk := by
  unfold weavePumpsO weavePumps
  rw [absorbEventsO_rf_absorb sk _ rfl]

/-- The O family's pump suffix, unconditionally: the manual prefix
(openers + walks) peels off and the pump rows are `weavePumpsO`'s.
The reply-first-conditional pin `procsO_drop_pumps` is this fact
composed with the absorber collapse. -/
theorem procsO_drop_pumpsO :
    (procsO sk ord).drop (manCount sk) = weavePumpsO sk ord := by
  have hsplit : procsO sk ord
      = ([iopenEvents sk, ropenEvents sk]
          ++ ((List.range sk.rootH).map fun i =>
            ((if (sk.rootH - 1 - i) % 2 == 1 then Party.I else Party.R,
              sk.rootH - 1 - i) : Party × Nat)).map (walkEventsO sk ord))
        ++ weavePumpsO sk ord := by
    simp [procsO, weavePumpsO, List.append_assoc]
  rw [hsplit, List.drop_append_of_le_length (by simp [manCount]; omega),
    List.drop_eq_nil_of_le (by simp [manCount]; omega), List.nil_append]

end StreamingMirror.Ord
