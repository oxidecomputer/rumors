import Batteries

/-! # Toolchain smoke test

Exercises the pieces the real development relies on: Batteries is
importable, `omega` closes linear-arithmetic goals, `Fin`-indexed
functions elaborate, and `decide` evaluates concrete `List.foldl`
computations in the kernel.
-/

namespace StreamingMirror

/-- `omega` closes a linear-arithmetic goal over `Nat`. -/
theorem succ_le_of_lt (n m : Nat) (h : n < m) : n + 1 ≤ m := by omega

/-- A `Fin`-indexed function: rotate an index forward by `k`, wrapping
modulo the bound. -/
def rotate {n : Nat} (i : Fin n) (k : Nat) : Fin n :=
  ⟨(i.val + k) % n, Nat.mod_lt _ i.pos⟩

/-- `decide` verifies a concrete `List.foldl` computation by kernel
reduction. -/
theorem foldl_sum : [1, 2, 3, 4].foldl (· + ·) 0 = 10 := by decide

#eval (rotate (n := 5) ⟨3, by omega⟩ 4).val  -- expect 2

end StreamingMirror
