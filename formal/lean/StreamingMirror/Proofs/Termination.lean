/-
Termination: the ρ measure and its strict decrease (AUDIT-NOTES.md A1,
remedy (i); MODEL.md §7's paper argument, minted as a kernel theorem).

# The measure

`rho` is the total count of unfired program operations, read off the
state: per walk, the remaining receives, commits, and fires of the
current scope plus the full cost of every scope not yet entered plus
the two closes; per assembler, the remaining receive/send operations of
the current and future resolutions; the openers', absorber's, and
finales' remaining operations likewise. Every commit counts as one
operation and every fire as another, so the committed-choice split
(MODEL.md §5) decreases ρ at both of its steps.

The one non-obvious device is `cAdj`, the committed-slot adjustment: a
walk contributes 1 when uncommitted (the pending commit), 0 when
committed in phase 2 to a still-pending obligation (that commit already
fired), and 2 when committed to an obligation its flags show fired —
the last is unreachable, but pricing it at 2 makes the fire of such a
phantom obligation strictly decreasing too, so `rho_decreases` needs no
committed-consistency invariant. The openers carry the same device
inside `iopenRho`/`ropenRho`.

# The one hypothesis: `asmLevelsOk`

`rho_decreases` carries a single Boolean hypothesis: no assembler in
its level-receiving phase has already received its full pending count
(`got < pendAt`). At states violating it, `asmRecvLevel` genuinely
regresses — the overshooting receive consumes a message and moves no
cursor the measure can see, and no Nat measure of the assembler state
alone can price an unboundedly repeatable step. The hypothesis is an
inductive invariant from `init` (`asmLevelsOk_init`,
`asmLevelsOk_preserved`), so every run-level statement below is
hypothesis-free. This is the honest form of MODEL.md §7's "every step
fires 1 op": true on the reachable states, and the reachable-state
side condition is exactly this check.

# The corollaries

- `terminating`: every run from `init` has length ≤ ρ(init) — no
  infinite runs exist, and bounded model checking at depth ρ(init) + 1
  is exhaustive for reachability (MODEL.md §7's claim (ii), first
  half).
- `maximal_run_terminal` / `maximal_run_terminal_d5`: a run from
  `init` that cannot be extended ends `Terminal` — the second half,
  by composing with the progress flagships (`Sched.deadlock_free`,
  `Sched.deadlock_free_d5`); the hypotheses are each flagship's.
- `greedy_run_terminal`: the constructive package — the greedy drain
  with fuel ρ(init) reaches `Terminal` on every well-formed margin-0
  skeleton.

Chain (stage D/E consumer): consumes `Sched.deadlock_free` (EndgameE)
and `Sched.deadlock_free_d5` (Endgame) plus `Control.drain`.
Map: Proofs/Map.lean.
-/
import StreamingMirror.Proofs.EndgameE
import StreamingMirror.Proofs.Endgame
import StreamingMirror.Controls

namespace StreamingMirror.Model

open StreamingMirror

-- ==================================================== list-sum helpers

/-- Pointwise-dominated maps have dominated sums. -/
private theorem map_sum_le {α : Type _} {f g : α → Nat} :
    ∀ {l : List α}, (∀ x ∈ l, g x ≤ f x) → (l.map g).sum ≤ (l.map f).sum
  | [], _ => Nat.le_refl _
  | x :: xs, h => by
      simp only [List.map_cons, List.sum_cons]
      have h1 := h x (List.mem_cons_self ..)
      have h2 := map_sum_le fun y hy => h y (List.mem_cons_of_mem x hy)
      omega

/-- A pointwise-dominated map with one strict member has a strictly
smaller sum. -/
private theorem map_sum_lt {α : Type _} {f g : α → Nat} {y : α} :
    ∀ {l : List α}, (∀ x ∈ l, g x ≤ f x) → y ∈ l → g y < f y →
      (l.map g).sum < (l.map f).sum
  | [], _, hy, _ => nomatch hy
  | x :: xs, h, hy, hlt => by
      simp only [List.map_cons, List.sum_cons]
      rcases List.mem_cons.mp hy with heq | hy'
      · subst heq
        have h2 := map_sum_le fun z hz => h z (List.mem_cons_of_mem _ hz)
        omega
      · have h1 := h x (List.mem_cons_self ..)
        have h2 := map_sum_lt (fun z hz => h z (List.mem_cons_of_mem x hz))
          hy' hlt
        omega

/-- Pointwise-equal maps have equal sums. -/
private theorem map_sum_congr {α : Type _} {f g : α → Nat} :
    ∀ {l : List α}, (∀ x ∈ l, g x = f x) → (l.map g).sum = (l.map f).sum
  | [], _ => rfl
  | x :: xs, h => by
      simp only [List.map_cons, List.sum_cons]
      rw [h x (List.mem_cons_self ..),
        map_sum_congr fun y hy => h y (List.mem_cons_of_mem x hy)]

-- ================================================== per-walk components

/-- Remaining fire operations child `i` of scope `sid` (stage `h`) owes,
read off the walk's flags: the wire, and for a D child the resolution
plus the unsent queries. -/
def kidRem (sk : Skel) (h sid : Nat) (ws : WalkSt) (i : Nat) : Nat :=
  (if ws.wireDone i then 0 else 1) +
  (if sk.childIsD h sid i then
    (if ws.resDone i then 0 else 1) + (sk.qCount h sid i - ws.qSent i)
   else 0)

/-- Remaining fire operations of the walk's current scope: per-child
remainders plus the parent summary. -/
def obligRem (sk : Skel) (h sid : Nat) (ws : WalkSt) : Nat :=
  ((List.range (sk.nChildren h sid)).map (kidRem sk h sid ws)).sum +
  (if ws.parentDone then 0 else 1)

/-- The full fire-operation cost of scope `sid` at stage `h`: what a
fresh walk owes it. -/
def obligFull (sk : Skel) (h sid : Nat) : Nat :=
  ((List.range (sk.nChildren h sid)).map fun i =>
    1 + (if sk.childIsD h sid i then 1 + sk.qCount h sid i else 0)).sum + 1

/-- A fresh walk's scope remainder is the full scope cost. -/
theorem obligRem_fresh (sk : Skel) (h sid k : Nat) :
    obligRem sk h sid (freshWalk sk h k) = obligFull sk h sid := by
  unfold obligRem obligFull
  have h1 : ((List.range (sk.nChildren h sid)).map
        (kidRem sk h sid (freshWalk sk h k))).sum
      = ((List.range (sk.nChildren h sid)).map fun i =>
          1 + (if sk.childIsD h sid i then 1 + sk.qCount h sid i else 0)).sum :=
    map_sum_congr fun i _ => by
      unfold kidRem freshWalk
      simp
  rw [h1]
  simp [freshWalk]

/-- Is obligation `o` still pending by the walk's flags? The fires of
pending obligations strictly decrease `obligRem`; fires of non-pending
obligations leave it unchanged. -/
def wsPending (sk : Skel) (h sid : Nat) (ws : WalkSt) : Oblig → Bool
  | .wire i => decide (i < sk.nChildren h sid) && !ws.wireDone i
  | .res i => sk.childIsD h sid i && !ws.resDone i
  | .query i => sk.childIsD h sid i && decide (ws.qSent i < sk.qCount h sid i)
  | .parent => !ws.parentDone

/-- The committed-slot adjustment (module doc): 1 for the pending
commit, 0 once committed in phase 2 to a pending obligation, 2 for the
phantom committed-but-fired corner (unreachable; priced so its fire
still decreases). -/
def cAdj (sk : Skel) (h : Nat) (ws : WalkSt) : Nat :=
  match ws.committed with
  | none => 1
  | some o =>
      if ws.phase == 2 &&
          wsPending sk h (sk.stageScope h ws.scope) ws o then 0
      else 2

/-- `cAdj` of an uncommitted walk. -/
theorem cAdj_none (sk : Skel) (h : Nat) {ws : WalkSt}
    (hcm : ws.committed = none) : cAdj sk h ws = 1 := by
  unfold cAdj
  rw [hcm]

/-- Receives the walk still owes its current scope, by phase. -/
def recvRem (phase : Nat) : Nat :=
  if phase == 0 then 2 else if phase == 1 then 1 else 0

/-- Full cost of the stage's scopes from index `k` on: two receives,
the commit baseline, and two operations (commit + fire) per obligation,
per scope. -/
def walkTail (sk : Skel) (h k : Nat) : Nat :=
  (((sk.stageScopes h).drop k).map fun sid =>
    3 + 2 * obligFull sk h sid).sum

/-- One scope peels off the tail. -/
theorem walkTail_cons (sk : Skel) {h k : Nat} (hk : k < sk.stageLen h) :
    walkTail sk h k
      = 3 + 2 * obligFull sk h (sk.stageScope h k) + walkTail sk h (k + 1) := by
  have hk' : k < (sk.stageScopes h).length := hk
  unfold walkTail
  rw [List.drop_eq_getElem_cons hk', List.map_cons, List.sum_cons]
  have hsid : sk.stageScope h k = (sk.stageScopes h)[k] := by
    unfold Skel.stageScope
    rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hk',
      Option.getD_some]
  rw [hsid]

/-- The tail past the stage is empty. -/
theorem walkTail_past (sk : Skel) {h k : Nat} (hk : sk.stageLen h ≤ k) :
    walkTail sk h k = 0 := by
  unfold walkTail
  rw [List.drop_eq_nil_of_le hk]
  rfl

/-- Remaining operations of one walk: 0/1/2 through the closes, and in
the receive/publish phases the remaining receives, twice the remaining
obligations, the committed-slot adjustment, the untouched scopes'
full cost, and the two closes. -/
def walkRho (sk : Skel) (h : Nat) (ws : WalkSt) : Nat :=
  if ws.phase == 5 then 0
  else if ws.phase == 4 then 1
  else if ws.phase == 3 then 2
  else
    recvRem ws.phase + 2 * obligRem sk h (sk.stageScope h ws.scope) ws +
    cAdj sk h ws + walkTail sk h (ws.scope + 1) + 2

-- ============================================= per-walk step lemmas

/-- The walk measure of a fresh cursor: the scope's full cost plus the
tail, or the bare closes past the stage's end. -/
theorem walkRho_fresh (sk : Skel) (h k : Nat) :
    walkRho sk h (freshWalk sk h k)
      = if k < sk.stageLen h then
          2 + 2 * obligFull sk h (sk.stageScope h k) + 1
            + walkTail sk h (k + 1) + 2
        else 2 := by
  have hsc : (freshWalk sk h k).scope = k := rfl
  have hcm : (freshWalk sk h k).committed = none := rfl
  by_cases hk : k < sk.stageLen h
  · have hph : (freshWalk sk h k).phase = 0 := by
      unfold freshWalk
      simp [hk]
    rw [if_pos hk]
    unfold walkRho
    rw [hph, hsc, cAdj_none sk h hcm, obligRem_fresh]
    simp only [recvRem]
    simp
  · have hph : (freshWalk sk h k).phase = 3 := by
      unfold freshWalk
      simp [hk]
    rw [if_neg hk]
    unfold walkRho
    rw [hph]
    simp

/-- The walk measure in the publishing phase, spelled out. -/
theorem walkRho_phase2 (sk : Skel) (h : Nat) {ws : WalkSt}
    (hph : ws.phase = 2) :
    walkRho sk h ws
      = 2 * obligRem sk h (sk.stageScope h ws.scope) ws + cAdj sk h ws
        + walkTail sk h (ws.scope + 1) + 2 := by
  unfold walkRho
  rw [hph]
  simp [recvRem]

/-- `normWalk` never increases the walk measure: a scope advance
retires the completed scope's bookkeeping and unpacks exactly one tail
entry (or parks the walk at its closes past the last scope). -/
theorem walkRho_normWalk_le (sk : Skel) (h : Nat) (ws : WalkSt)
    (hcm : ws.committed = none) :
    walkRho sk h (normWalk sk h ws) ≤ walkRho sk h ws := by
  unfold normWalk
  split
  case isFalse => exact Nat.le_refl _
  case isTrue hg =>
    simp only [Bool.and_eq_true, beq_iff_eq] at hg
    rw [walkRho_phase2 sk h hg.1, cAdj_none sk h hcm, walkRho_fresh]
    by_cases hlt : ws.scope + 1 < sk.stageLen h
    · rw [if_pos hlt, walkTail_cons sk hlt]
      omega
    · rw [if_neg hlt]
      omega

/-- `cAdj` never exceeds 2, and outside the publishing phase it is at
least 1 (the pending-commit reading is phase-2-only). -/
theorem cAdj_cases (sk : Skel) (h : Nat) (ws : WalkSt)
    (hph : ws.phase ≠ 2) :
    cAdj sk h ws = 1 ∨ cAdj sk h ws = 2 := by
  unfold cAdj
  cases hcm : ws.committed with
  | none => exact Or.inl rfl
  | some o =>
      have hb : (ws.phase == 2) = false := by
        simpa using hph
      simp [hb]

/-- A wire receive strictly decreases the walk measure. -/
theorem walkRho_recvWire_lt (sk : Skel) (h : Nat) (ws : WalkSt)
    (hph : ws.phase = 0) :
    walkRho sk h { ws with phase := 1, committed := none }
      < walkRho sk h ws := by
  have hRem : obligRem sk h (sk.stageScope h ws.scope)
      { ws with phase := 1, committed := none }
      = obligRem sk h (sk.stageScope h ws.scope) ws := rfl
  have hlhs : walkRho sk h { ws with phase := 1, committed := none }
      = 1 + 2 * obligRem sk h (sk.stageScope h ws.scope) ws + 1
        + walkTail sk h (ws.scope + 1) + 2 := by
    unfold walkRho
    rw [cAdj_none sk h
      (ws := { ws with phase := 1, committed := none }) rfl]
    simp only [recvRem]
    simp [hRem]
  have hrhs : walkRho sk h ws
      = 2 + 2 * obligRem sk h (sk.stageScope h ws.scope) ws + cAdj sk h ws
        + walkTail sk h (ws.scope + 1) + 2 := by
    unfold walkRho
    rw [hph]
    simp only [recvRem]
    simp
  rw [hlhs, hrhs]
  rcases cAdj_cases sk h ws (by omega) with hc | hc <;> rw [hc] <;> omega

/-- An asked receive (with the trailing `normWalk`) strictly decreases
the walk measure. -/
theorem walkRho_recvAsked_lt (sk : Skel) (h : Nat) (ws : WalkSt)
    (hph : ws.phase = 1) :
    walkRho sk h (normWalk sk h { ws with phase := 2, committed := none })
      < walkRho sk h ws := by
  refine Nat.lt_of_le_of_lt
    (walkRho_normWalk_le sk h { ws with phase := 2, committed := none } rfl)
    ?_
  have hRem : obligRem sk h (sk.stageScope h ws.scope)
      { ws with phase := 2, committed := none }
      = obligRem sk h (sk.stageScope h ws.scope) ws := rfl
  have hlhs : walkRho sk h { ws with phase := 2, committed := none }
      = 2 * obligRem sk h (sk.stageScope h ws.scope) ws + 1
        + walkTail sk h (ws.scope + 1) + 2 := by
    rw [walkRho_phase2 sk h
      (ws := { ws with phase := 2, committed := none }) rfl,
      cAdj_none sk h (ws := { ws with phase := 2, committed := none }) rfl]
    simp [hRem]
  have hrhs : walkRho sk h ws
      = 1 + 2 * obligRem sk h (sk.stageScope h ws.scope) ws + cAdj sk h ws
        + walkTail sk h (ws.scope + 1) + 2 := by
    unfold walkRho
    rw [hph]
    simp only [recvRem]
    simp
  rw [hlhs, hrhs]
  rcases cAdj_cases sk h ws (by omega) with hc | hc <;> rw [hc] <;> omega

/-- A ledger-legal commit strictly decreases the walk measure: the
choosable obligation is pending, so the committed-slot adjustment drops
from 1 to 0. -/
theorem walkRho_commit_lt (sk : Skel) (ax : AxMode) (pk : Party × Nat)
    (ws : WalkSt) (o : Oblig)
    (hch : wkChoosable sk ax pk ws o = true) :
    walkRho sk pk.2 { ws with committed := some o }
      < walkRho sk pk.2 ws := by
  unfold wkChoosable at hch
  split at hch
  case isTrue => cases hch
  case isFalse hcond =>
    rw [Bool.or_eq_true] at hcond
    have hph : ws.phase = 2 := by
      by_cases hb : (ws.phase != 2) = true
      · exact absurd (Or.inl hb) hcond
      · simpa using hb
    have hcm : ws.committed = none := by
      cases hcmm : ws.committed with
      | none => rfl
      | some o' => exact absurd (Or.inr (by rw [hcmm]; rfl)) hcond
    have hpend : wsPending sk pk.2 (sk.stageScope pk.2 ws.scope) ws o
        = true := by
      cases o with
      | wire i =>
          simp only [Bool.and_eq_true, decide_eq_true_eq] at hch
          obtain ⟨⟨⟨⟨h1, h2⟩, -⟩, -⟩, -⟩ := hch
          simp [wsPending, h1, h2]
      | res i =>
          simp only [Bool.and_eq_true, decide_eq_true_eq] at hch
          obtain ⟨⟨⟨⟨⟨-, h2⟩, h3⟩, -⟩, -⟩, -⟩ := hch
          simp [wsPending, h2, h3]
      | query i =>
          simp only [Bool.and_eq_true, decide_eq_true_eq] at hch
          obtain ⟨⟨⟨⟨⟨⟨-, h2⟩, h3⟩, -⟩, -⟩, -⟩, -⟩ := hch
          simp [wsPending, h2, h3]
      | parent =>
          simp only [Bool.and_eq_true] at hch
          obtain ⟨⟨h1, -⟩, -⟩ := hch
          simp [wsPending, h1]
    have hRem : obligRem sk pk.2 (sk.stageScope pk.2 ws.scope)
        { ws with committed := some o }
        = obligRem sk pk.2 (sk.stageScope pk.2 ws.scope) ws := rfl
    have hpend2 : wsPending sk pk.2 (sk.stageScope pk.2 ws.scope)
        { ws with phase := 2, committed := some o } o = true := by
      cases o <;> simpa [wsPending] using hpend
    have hc' : cAdj sk pk.2 { ws with committed := some o } = 0 := by
      unfold cAdj
      simp [hph, hpend2]
    rw [walkRho_phase2 sk pk.2 (ws := { ws with committed := some o }) hph,
      walkRho_phase2 sk pk.2 hph, cAdj_none sk pk.2 hcm, hc']
    simp only [hRem]
    omega

/-- Firing a pending obligation strictly decreases the scope
remainder. -/
theorem obligRem_fire_lt (sk : Skel) (h sid : Nat) (ws : WalkSt)
    (o : Oblig)
    (hpend : wsPending sk h sid ws o = true) :
    obligRem sk h sid (fireOblig ws o) < obligRem sk h sid ws := by
  have hDin : ∀ i, sk.childIsD h sid i = true → i < sk.nChildren h sid := by
    intro i hd
    unfold Skel.childIsD at hd
    split at hd
    next => cases hd
    next hne =>
      cases hik : (sk.scope sid).kids[i]? with
      | none =>
          rw [hik] at hd
          simp at hd
      | some kk =>
          have hlen : i < (sk.scope sid).kids.length := by
            by_contra hout
            rw [List.getElem?_eq_none (by omega)] at hik
            cases hik
          unfold Skel.nChildren
          rw [if_neg hne]
          exact hlen
  cases o with
  | wire i =>
      simp only [wsPending, Bool.and_eq_true, decide_eq_true_eq,
        Bool.not_eq_true'] at hpend
      obtain ⟨hin, hwd⟩ := hpend
      unfold obligRem
      have hle : ∀ j ∈ List.range (sk.nChildren h sid),
          kidRem sk h sid (fireOblig ws (.wire i)) j
            ≤ kidRem sk h sid ws j := by
        intro j _
        unfold kidRem fireOblig
        by_cases hj : j = i
        · subst hj; simp [hwd]
        · have hji : (j == i) = false := by simpa using hj
          simp [hji]
      have hstrict : kidRem sk h sid (fireOblig ws (.wire i)) i
          < kidRem sk h sid ws i := by
        unfold kidRem fireOblig
        simp [hwd]
      have hsum := map_sum_lt hle (List.mem_range.mpr hin) hstrict
      have hpar : (fireOblig ws (Oblig.wire i)).parentDone = ws.parentDone :=
        rfl
      rw [hpar]
      omega
  | res i =>
      simp only [wsPending, Bool.and_eq_true, Bool.not_eq_true'] at hpend
      obtain ⟨hd, hrd⟩ := hpend
      unfold obligRem
      have hle : ∀ j ∈ List.range (sk.nChildren h sid),
          kidRem sk h sid (fireOblig ws (.res i)) j
            ≤ kidRem sk h sid ws j := by
        intro j _
        unfold kidRem fireOblig
        by_cases hj : j = i
        · subst hj; simp [hd, hrd]
        · have hji : (j == i) = false := by simpa using hj
          simp [hji]
      have hstrict : kidRem sk h sid (fireOblig ws (.res i)) i
          < kidRem sk h sid ws i := by
        unfold kidRem fireOblig
        simp [hd, hrd]
      have hsum := map_sum_lt hle (List.mem_range.mpr (hDin i hd)) hstrict
      have hpar : (fireOblig ws (Oblig.res i)).parentDone = ws.parentDone :=
        rfl
      rw [hpar]
      omega
  | query i =>
      simp only [wsPending, Bool.and_eq_true, decide_eq_true_eq] at hpend
      obtain ⟨hd, hq⟩ := hpend
      unfold obligRem
      have hle : ∀ j ∈ List.range (sk.nChildren h sid),
          kidRem sk h sid (fireOblig ws (.query i)) j
            ≤ kidRem sk h sid ws j := by
        intro j _
        unfold kidRem fireOblig
        by_cases hj : j = i
        · subst hj
          simp only [BEq.rfl, if_pos]
          simp [hd]
          omega
        · have hji : (j == i) = false := by simpa using hj
          simp [hji]
      have hstrict : kidRem sk h sid (fireOblig ws (.query i)) i
          < kidRem sk h sid ws i := by
        unfold kidRem fireOblig
        simp [hd]
        omega
      have hsum := map_sum_lt hle (List.mem_range.mpr (hDin i hd)) hstrict
      have hpar : (fireOblig ws (Oblig.query i)).parentDone = ws.parentDone :=
        rfl
      rw [hpar]
      omega
  | parent =>
      simp only [wsPending, Bool.not_eq_true'] at hpend
      unfold obligRem
      have hmap : ((List.range (sk.nChildren h sid)).map
            (kidRem sk h sid (fireOblig ws .parent))).sum
          = ((List.range (sk.nChildren h sid)).map
            (kidRem sk h sid ws)).sum := rfl
      rw [hmap]
      unfold fireOblig
      simp [hpend]

/-- Firing a non-pending obligation leaves the scope remainder
unchanged (the phantom corner: the flags already show it fired). -/
theorem obligRem_fire_eq (sk : Skel) (h sid : Nat) (ws : WalkSt)
    (o : Oblig)
    (hpend : wsPending sk h sid ws o = false) :
    obligRem sk h sid (fireOblig ws o) = obligRem sk h sid ws := by
  cases o with
  | wire i =>
      simp only [wsPending, Bool.and_eq_false_iff, decide_eq_false_iff_not,
        Bool.not_eq_false', Nat.not_lt] at hpend
      unfold obligRem
      refine congrArg (· + _) (map_sum_congr fun j hj => ?_)
      unfold kidRem fireOblig
      by_cases hji : j = i
      · subst hji
        rcases hpend with hout | hwd
        · exact absurd (List.mem_range.mp hj) (by omega)
        · simp [hwd]
      · have hji' : (j == i) = false := by simpa using hji
        simp [hji']
  | res i =>
      simp only [wsPending, Bool.and_eq_false_iff,
        Bool.not_eq_false'] at hpend
      unfold obligRem
      refine congrArg (· + _) (map_sum_congr fun j hj => ?_)
      unfold kidRem fireOblig
      by_cases hji : j = i
      · subst hji
        rcases hpend with hd | hrd
        · simp [hd]
        · simp [hrd]
      · have hji' : (j == i) = false := by simpa using hji
        simp [hji']
  | query i =>
      simp only [wsPending, Bool.and_eq_false_iff,
        decide_eq_false_iff_not, Nat.not_lt] at hpend
      unfold obligRem
      refine congrArg (· + _) (map_sum_congr fun j hj => ?_)
      unfold kidRem fireOblig
      by_cases hji : j = i
      · subst hji
        rcases hpend with hd | hq
        · simp [hd]
        · simp only [BEq.rfl, if_pos]
          have : sk.qCount h sid j - (ws.qSent j + 1)
              = sk.qCount h sid j - ws.qSent j := by omega
          simp [this]
      · have hji' : (j == i) = false := by simpa using hji
        simp [hji']
  | parent =>
      simp only [wsPending, Bool.not_eq_false'] at hpend
      unfold obligRem
      have hmap : ((List.range (sk.nChildren h sid)).map
            (kidRem sk h sid (fireOblig ws .parent))).sum
          = ((List.range (sk.nChildren h sid)).map
            (kidRem sk h sid ws)).sum := rfl
      rw [hmap]
      unfold fireOblig
      simp [hpend]

/-- A committed fire (with the trailing `normWalk`) strictly decreases
the walk measure — with a pending obligation through the remainder,
with a phantom one through the committed-slot adjustment. -/
theorem walkRho_fire_lt (sk : Skel) (h : Nat) (ws : WalkSt) (o : Oblig)
    (hph : ws.phase = 2) (hcm : ws.committed = some o) :
    walkRho sk h (normWalk sk h (fireOblig ws o)) < walkRho sk h ws := by
  have hcm' : (fireOblig ws o).committed = none := by
    cases o <;> rfl
  refine Nat.lt_of_le_of_lt
    (walkRho_normWalk_le sk h (fireOblig ws o) hcm') ?_
  have hphase : (fireOblig ws o).phase = 2 := by
    have hh : (fireOblig ws o).phase = ws.phase := by cases o <;> rfl
    rw [hh, hph]
  have hscope : (fireOblig ws o).scope = ws.scope := by cases o <;> rfl
  rw [walkRho_phase2 sk h hphase, walkRho_phase2 sk h hph,
    cAdj_none sk h hcm', hscope]
  cases hpend : wsPending sk h (sk.stageScope h ws.scope) ws o with
  | true =>
      have h1 := obligRem_fire_lt sk h (sk.stageScope h ws.scope) ws o hpend
      omega
  | false =>
      rw [obligRem_fire_eq sk h (sk.stageScope h ws.scope) ws o hpend]
      have h2 : cAdj sk h ws = 2 := by
        unfold cAdj
        rw [hcm]
        simp [hph, hpend]
      omega

/-- A wire close strictly decreases the walk measure. -/
theorem walkRho_closeWire_lt (sk : Skel) (h : Nat) (ws : WalkSt)
    (hph : ws.phase = 3) :
    walkRho sk h { ws with phase := 4 } < walkRho sk h ws := by
  unfold walkRho
  rw [hph]
  simp

/-- An asked close strictly decreases the walk measure. -/
theorem walkRho_closeAsked_lt (sk : Skel) (h : Nat) (ws : WalkSt)
    (hph : ws.phase = 4) :
    walkRho sk h { ws with phase := 5 } < walkRho sk h ws := by
  unfold walkRho
  rw [hph]
  simp

-- =========================================== assembler components

/-- Remaining operations of the assembler's current resolution, by
phase: the resolution receive, the level receives, and the send. -/
def asmItemRem (sk : Skel) (pk : Party × Nat) (a : AsmSt) : Nat :=
  if a.phase == 0 then sk.pendAt pk.1 pk.2 a.idx + 2
  else if a.phase == 1 then (sk.pendAt pk.1 pk.2 a.idx - a.got) + 1
  else 1

/-- Full cost of the resolutions from index `k` on: one receive, the
pending level receives, one send, per resolution. -/
def asmTail (sk : Skel) (pk : Party × Nat) (k : Nat) : Nat :=
  (((sk.asmResList pk.1 pk.2).drop k).map fun pend => pend + 2).sum

/-- One resolution peels off the assembler tail. -/
theorem asmTail_cons (sk : Skel) (pk : Party × Nat) {k : Nat}
    (hk : k < (sk.asmResList pk.1 pk.2).length) :
    asmTail sk pk k
      = (sk.pendAt pk.1 pk.2 k + 2) + asmTail sk pk (k + 1) := by
  unfold asmTail
  rw [List.drop_eq_getElem_cons hk, List.map_cons, List.sum_cons]
  have hp : sk.pendAt pk.1 pk.2 k = (sk.asmResList pk.1 pk.2)[k] := by
    unfold Skel.pendAt
    rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hk,
      Option.getD_some]
  rw [hp]

/-- The assembler tail past the resolution list is empty. -/
theorem asmTail_past (sk : Skel) (pk : Party × Nat) {k : Nat}
    (hk : (sk.asmResList pk.1 pk.2).length ≤ k) : asmTail sk pk k = 0 := by
  unfold asmTail
  rw [List.drop_eq_nil_of_le hk]
  rfl

/-- Remaining operations of one assembler. -/
def asmRho (sk : Skel) (pk : Party × Nat) (a : AsmSt) : Nat :=
  if a.phase == 4 then 0
  else if a.phase == 3 then 1
  else asmItemRem sk pk a + asmTail sk pk (a.idx + 1) + 1

-- ============================================== top-level components

/-- Remaining operations of the initiator opener (with the opener's
committed-slot adjustment, the walk device transported). -/
def iopenRho (s : State) : Nat :=
  2 * ((if s.iopenWire then 0 else 1) + (if s.iopenQuery then 0 else 1)) +
  (match s.iopenCh with
   | none => 1
   | some .wire => if s.iopenWire then 2 else 0
   | some .query => if s.iopenQuery then 2 else 0)

/-- Remaining operations of the responder opener. -/
def ropenRho (sk : Skel) (s : State) : Nat :=
  (if s.ropenGotWire then 0 else 1) +
  2 * ((if s.ropenWire then 0 else 1) + (if s.ropenRes then 0 else 1) +
       ((sk.scope 0).kids.length - s.ropenQ)) +
  (match s.ropenCh with
   | none => 1
   | some .wire => if s.ropenWire then 2 else 0
   | some .res => if s.ropenRes then 2 else 0
   | some .query => if s.ropenQ < (sk.scope 0).kids.length then 0 else 2)

/-- Remaining operations of the absorber: three per leaf request
(wire receive, request receive, send) plus the two closes. -/
def absorbRho (sk : Skel) (s : State) : Nat :=
  if s.absorbPhase == 5 then 0
  else if s.absorbPhase == 4 then 1
  else if s.absorbPhase == 3 then 2
  else
    (if s.absorbPhase == 0 then 3
     else if s.absorbPhase == 1 then 2 else 1) +
    3 * (sk.totalLeafReqs - s.absorbIdx - 1) + 2

/-- Remaining operations of the two finale consumers. -/
def finRho (sk : Skel) (s : State) : Nat :=
  (if s.ifin then 0 else 1) + (if s.rfinGotRes then 0 else 1) +
  (sk.rootPending - s.rfinGot)

-- ======================================================== the measure

/-- The walks' total measure. -/
def walkSum (sk : Skel) (s : State) : Nat :=
  (sk.walkKeys.map fun pk => walkRho sk pk.2 (s.walk pk)).sum

/-- The assemblers' total measure. -/
def asmSum (sk : Skel) (s : State) : Nat :=
  (sk.asmKeys.map fun pk => asmRho sk pk (s.asm pk)).sum

/-- Total remaining operations of the whole system (MODEL.md §7's ρ). -/
def rho (sk : Skel) (s : State) : Nat :=
  walkSum sk s + asmSum sk s + iopenRho s + ropenRho sk s +
    absorbRho sk s + finRho sk s

/-- No assembler in its level-receiving phase has already consumed its
full pending count (module doc: the one reachability fact
`rho_decreases` needs). -/
def asmLevelsOk (sk : Skel) (s : State) : Bool :=
  sk.asmKeys.all fun pk =>
    (s.asm pk).phase != 1 ||
      decide ((s.asm pk).got < sk.pendAt pk.1 pk.2 (s.asm pk).idx)

-- ================================================ component lifters

/-- Lift a strict walk decrease (with an arbitrary channel-field
rewrite) to the whole measure. -/
private theorem rho_walk_lt (sk : Skel) {s : State} (f : Chan → Nat)
    {pk : Party × Nat} {ws' : WalkSt} (hmem : pk ∈ sk.walkKeys)
    (hlt : walkRho sk pk.2 ws' < walkRho sk pk.2 (s.walk pk)) :
    rho sk (setWalk { s with chan := f } pk ws') < rho sk s := by
  have h1 : walkSum sk (setWalk { s with chan := f } pk ws')
      < walkSum sk s := by
    unfold walkSum
    refine map_sum_lt (fun pk' _ => ?_) hmem ?_
    · by_cases he : pk' = pk
      · subst he
        rw [setWalk_walk_self]
        exact Nat.le_of_lt hlt
      · rw [setWalk_walk_ne _ _ he]
        exact Nat.le_refl _
    · rw [setWalk_walk_self]
      exact hlt
  have h2 : asmSum sk (setWalk { s with chan := f } pk ws')
      = asmSum sk s := rfl
  have h3 : iopenRho (setWalk { s with chan := f } pk ws')
      = iopenRho s := rfl
  have h4 : ropenRho sk (setWalk { s with chan := f } pk ws')
      = ropenRho sk s := rfl
  have h5 : absorbRho sk (setWalk { s with chan := f } pk ws')
      = absorbRho sk s := rfl
  have h6 : finRho sk (setWalk { s with chan := f } pk ws')
      = finRho sk s := rfl
  unfold rho
  omega

/-- Lift a strict assembler decrease (with an arbitrary channel-field
rewrite) to the whole measure. -/
private theorem rho_asm_lt (sk : Skel) {s : State} (f : Chan → Nat)
    {pk : Party × Nat} {a' : AsmSt} (hmem : pk ∈ sk.asmKeys)
    (hlt : asmRho sk pk a' < asmRho sk pk (s.asm pk)) :
    rho sk (setAsm { s with chan := f } pk a') < rho sk s := by
  have h1 : asmSum sk (setAsm { s with chan := f } pk a')
      < asmSum sk s := by
    unfold asmSum
    refine map_sum_lt (fun pk' _ => ?_) hmem ?_
    · by_cases he : pk' = pk
      · subst he
        rw [setAsm_asm_self]
        exact Nat.le_of_lt hlt
      · rw [setAsm_asm_ne _ _ he]
        exact Nat.le_refl _
    · rw [setAsm_asm_self]
      exact hlt
  have h2 : walkSum sk (setAsm { s with chan := f } pk a')
      = walkSum sk s := rfl
  have h3 : iopenRho (setAsm { s with chan := f } pk a')
      = iopenRho s := rfl
  have h4 : ropenRho sk (setAsm { s with chan := f } pk a')
      = ropenRho sk s := rfl
  have h5 : absorbRho sk (setAsm { s with chan := f } pk a')
      = absorbRho sk s := rfl
  have h6 : finRho sk (setAsm { s with chan := f } pk a')
      = finRho sk s := rfl
  unfold rho
  omega

/-- Lift a strict initiator-opener decrease to the whole measure. -/
private theorem rho_io_lt (sk : Skel) {s s' : State}
    (h1 : walkSum sk s' = walkSum sk s) (h2 : asmSum sk s' = asmSum sk s)
    (h4 : ropenRho sk s' = ropenRho sk s)
    (h5 : absorbRho sk s' = absorbRho sk s)
    (h6 : finRho sk s' = finRho sk s)
    (hio : iopenRho s' < iopenRho s) : rho sk s' < rho sk s := by
  unfold rho
  omega

/-- Lift a strict responder-opener decrease to the whole measure. -/
private theorem rho_ro_lt (sk : Skel) {s s' : State}
    (h1 : walkSum sk s' = walkSum sk s) (h2 : asmSum sk s' = asmSum sk s)
    (h3 : iopenRho s' = iopenRho s)
    (h5 : absorbRho sk s' = absorbRho sk s)
    (h6 : finRho sk s' = finRho sk s)
    (hro : ropenRho sk s' < ropenRho sk s) : rho sk s' < rho sk s := by
  unfold rho
  omega

/-- Lift a strict absorber decrease to the whole measure. -/
private theorem rho_ab_lt (sk : Skel) {s s' : State}
    (h1 : walkSum sk s' = walkSum sk s) (h2 : asmSum sk s' = asmSum sk s)
    (h3 : iopenRho s' = iopenRho s) (h4 : ropenRho sk s' = ropenRho sk s)
    (h6 : finRho sk s' = finRho sk s)
    (hab : absorbRho sk s' < absorbRho sk s) : rho sk s' < rho sk s := by
  unfold rho
  omega

/-- Lift a strict finale decrease to the whole measure. -/
private theorem rho_fin_lt (sk : Skel) {s s' : State}
    (h1 : walkSum sk s' = walkSum sk s) (h2 : asmSum sk s' = asmSum sk s)
    (h3 : iopenRho s' = iopenRho s) (h4 : ropenRho sk s' = ropenRho sk s)
    (h5 : absorbRho sk s' = absorbRho sk s)
    (hfin : finRho sk s' < finRho sk s) : rho sk s' < rho sk s := by
  unfold rho
  omega

-- ================================================== the level invariant

/-- The initial state satisfies the level invariant: every assembler
starts before (or past) its level-receiving phase. -/
theorem asmLevelsOk_init (sk : Skel) :
    asmLevelsOk sk (init sk) = true := by
  unfold asmLevelsOk
  rw [List.all_eq_true]
  intro pk _
  unfold init
  by_cases hp : (sk.asmResList pk.1 pk.2).length > 0 <;> simp [hp]

/-- Every action preserves the level invariant: only the assembler's
own receives move `got`, and both keep it strictly below the pending
count while in the level-receiving phase. -/
theorem asmLevelsOk_preserved (sk : Skel) (ax : AxMode) {s s' : State}
    (a : Action) (hstep : apply sk ax a s = some s')
    (hlv : asmLevelsOk sk s = true) : asmLevelsOk sk s' = true := by
  cases a with
  | iopenChoose o =>
      simp only [apply] at hstep
      split at hstep
      · injection hstep with hs'; subst hs'; exact hlv
      · simp at hstep
  | iopenFire =>
      simp only [apply] at hstep
      cases hio : s.iopenCh with
      | none => simp [hio] at hstep
      | some ob =>
          cases ob with
          | wire =>
              simp only [hio] at hstep
              split at hstep
              · injection hstep with hs'; subst hs'; exact hlv
              · simp at hstep
          | query =>
              simp only [hio] at hstep
              split at hstep
              · injection hstep with hs'; subst hs'; exact hlv
              · simp at hstep
  | ropenRecv =>
      simp only [apply] at hstep
      split at hstep
      · injection hstep with hs'; subst hs'; exact hlv
      · simp at hstep
  | ropenChoose o =>
      simp only [apply] at hstep
      split at hstep
      · injection hstep with hs'; subst hs'; exact hlv
      · simp at hstep
  | ropenFire =>
      simp only [apply] at hstep
      cases hro : s.ropenCh with
      | none => simp [hro] at hstep
      | some ob =>
          cases ob with
          | wire =>
              simp only [hro] at hstep
              split at hstep
              · injection hstep with hs'; subst hs'; exact hlv
              · simp at hstep
          | res =>
              simp only [hro] at hstep
              split at hstep
              · injection hstep with hs'; subst hs'; exact hlv
              · simp at hstep
          | query =>
              simp only [hro] at hstep
              split at hstep
              · injection hstep with hs'; subst hs'; exact hlv
              · simp at hstep
  | walkRecvWire pk =>
      simp only [apply] at hstep
      split at hstep
      · injection hstep with hs'; subst hs'; exact hlv
      · simp at hstep
  | walkRecvAsked pk =>
      simp only [apply] at hstep
      split at hstep
      · injection hstep with hs'; subst hs'; exact hlv
      · simp at hstep
  | walkCommit pk o =>
      simp only [apply] at hstep
      split at hstep
      · injection hstep with hs'; subst hs'; exact hlv
      · simp at hstep
  | walkFire pk =>
      simp only [apply] at hstep
      cases hcm : (s.walk pk).committed with
      | none => simp [hcm] at hstep
      | some o =>
          simp only [hcm] at hstep
          split at hstep
          · injection hstep with hs'; subst hs'; exact hlv
          · simp at hstep
  | walkCloseWire pk =>
      simp only [apply] at hstep
      split at hstep
      · injection hstep with hs'; subst hs'; exact hlv
      · simp at hstep
  | walkCloseAsked pk =>
      simp only [apply] at hstep
      split at hstep
      · injection hstep with hs'; subst hs'; exact hlv
      · simp at hstep
  | asmRecvRes pk =>
      simp only [apply] at hstep
      split at hstep
      case isFalse => simp at hstep
      case isTrue hg =>
        injection hstep with hs'; subst hs'
        unfold asmLevelsOk at hlv ⊢
        rw [List.all_eq_true] at hlv ⊢
        intro pk' hpk'
        by_cases he : pk' = pk
        · subst he
          rw [setAsm_asm_self]
          by_cases hp : sk.pendAt pk'.1 pk'.2 (s.asm pk').idx > 0 <;>
            simp [hp]
        · rw [setAsm_asm_ne _ _ he]
          exact hlv pk' hpk'
  | asmRecvLevel pk =>
      simp only [apply] at hstep
      split at hstep
      case isFalse => simp at hstep
      case isTrue hg =>
        simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
        obtain ⟨⟨hmem, hph⟩, hpos⟩ := hg
        injection hstep with hs'; subst hs'
        unfold asmLevelsOk at hlv ⊢
        rw [List.all_eq_true] at hlv ⊢
        intro pk' hpk'
        by_cases he : pk' = pk
        · subst he
          rw [setAsm_asm_self]
          have hgot : (s.asm pk').got
              < sk.pendAt pk'.1 pk'.2 (s.asm pk').idx := by
            have h0 := hlv pk' hpk'
            rw [hph] at h0
            simpa using h0
          cases hbe : ((s.asm pk').got + 1
              == sk.pendAt pk'.1 pk'.2 (s.asm pk').idx) with
          | true => simp
          | false =>
              have hne : (s.asm pk').got + 1
                  ≠ sk.pendAt pk'.1 pk'.2 (s.asm pk').idx := by
                simpa using hbe
              simp
              omega
        · rw [setAsm_asm_ne _ _ he]
          exact hlv pk' hpk'
  | asmSend pk =>
      simp only [apply] at hstep
      split at hstep
      case isFalse => simp at hstep
      case isTrue hg =>
        injection hstep with hs'; subst hs'
        unfold asmLevelsOk at hlv ⊢
        rw [List.all_eq_true] at hlv ⊢
        intro pk' hpk'
        by_cases he : pk' = pk
        · subst he
          rw [setAsm_asm_self]
          by_cases hl : (s.asm pk').idx + 1
              < (sk.asmResList pk'.1 pk'.2).length <;> simp [hl]
        · rw [setAsm_asm_ne _ _ he]
          exact hlv pk' hpk'
  | asmClose pk =>
      simp only [apply] at hstep
      split at hstep
      case isFalse => simp at hstep
      case isTrue hg =>
        injection hstep with hs'; subst hs'
        unfold asmLevelsOk at hlv ⊢
        rw [List.all_eq_true] at hlv ⊢
        intro pk' hpk'
        by_cases he : pk' = pk
        · subst he
          rw [setAsm_asm_self]
          simp
        · rw [setAsm_asm_ne _ _ he]
          exact hlv pk' hpk'
  | absorbRecvWire =>
      simp only [apply] at hstep
      split at hstep
      · injection hstep with hs'; subst hs'; exact hlv
      · simp at hstep
  | absorbRecvAsked =>
      simp only [apply] at hstep
      split at hstep
      · injection hstep with hs'; subst hs'; exact hlv
      · simp at hstep
  | absorbSend =>
      simp only [apply] at hstep
      split at hstep
      · injection hstep with hs'; subst hs'; exact hlv
      · simp at hstep
  | absorbCloseWire =>
      simp only [apply] at hstep
      split at hstep
      · injection hstep with hs'; subst hs'; exact hlv
      · simp at hstep
  | absorbCloseAsked =>
      simp only [apply] at hstep
      split at hstep
      · injection hstep with hs'; subst hs'; exact hlv
      · simp at hstep
  | finRet =>
      simp only [apply] at hstep
      split at hstep
      · injection hstep with hs'; subst hs'; exact hlv
      · simp at hstep
  | finRes =>
      simp only [apply] at hstep
      split at hstep
      · injection hstep with hs'; subst hs'; exact hlv
      · simp at hstep
  | finRets =>
      simp only [apply] at hstep
      split at hstep
      · injection hstep with hs'; subst hs'; exact hlv
      · simp at hstep

-- ======================================================= the decrease

/-- Every enabled action strictly decreases ρ (MODEL.md §7: "every step
fires 1 op"), at any state satisfying the level invariant.

The 23-case analysis. The `asmLevelsOk` hypothesis is consumed only by
the `asmRecvLevel` case (module doc: the overshooting level receive is
the one step no state-only measure can price); every other case
decreases unconditionally, phantom corners included, thanks to the
committed-slot adjustment. -/
theorem rho_decreases (sk : Skel) (ax : AxMode) {s s' : State}
    (a : Action) (hlv : asmLevelsOk sk s = true)
    (hstep : apply sk ax a s = some s') : rho sk s' < rho sk s := by
  cases a with
  | iopenChoose o =>
      simp only [apply] at hstep
      split at hstep
      case isFalse => simp at hstep
      case isTrue hg =>
        simp only [Bool.and_eq_true, beq_iff_eq] at hg
        obtain ⟨hnone, hch⟩ := hg
        injection hstep with hs'; subst hs'
        refine rho_io_lt sk rfl rfl rfl rfl rfl ?_
        cases o with
        | wire =>
            have hw : s.iopenWire = false := by
              simpa [iopenChoosable] using hch
            simp only [iopenRho, hnone, hw]
            simp
        | query =>
            have hq : s.iopenQuery = false := by
              simp only [iopenChoosable, Bool.and_eq_true,
                Bool.not_eq_true'] at hch
              exact hch.1
            simp only [iopenRho, hnone, hq]
            simp
  | iopenFire =>
      simp only [apply] at hstep
      cases hio : s.iopenCh with
      | none => simp [hio] at hstep
      | some ob =>
          cases ob with
          | wire =>
              simp only [hio] at hstep
              split at hstep
              case isFalse => simp at hstep
              case isTrue hg =>
                injection hstep with hs'; subst hs'
                refine rho_io_lt sk rfl rfl rfl rfl rfl ?_
                cases hw : s.iopenWire <;>
                  simp only [iopenRho, hio, hw] <;> (simp; try omega)
          | query =>
              simp only [hio] at hstep
              split at hstep
              case isFalse => simp at hstep
              case isTrue hg =>
                injection hstep with hs'; subst hs'
                refine rho_io_lt sk rfl rfl rfl rfl rfl ?_
                cases hq : s.iopenQuery <;>
                  simp only [iopenRho, hio, hq] <;> (simp; try omega)
  | ropenRecv =>
      simp only [apply] at hstep
      split at hstep
      case isFalse => simp at hstep
      case isTrue hg =>
        simp only [Bool.and_eq_true, Bool.not_eq_true',
          decide_eq_true_eq] at hg
        obtain ⟨hgw, hpos⟩ := hg
        injection hstep with hs'; subst hs'
        refine rho_ro_lt sk rfl rfl rfl rfl rfl ?_
        simp only [ropenRho, hgw]
        simp
  | ropenChoose o =>
      simp only [apply] at hstep
      split at hstep
      case isFalse => simp at hstep
      case isTrue hg =>
        simp only [Bool.and_eq_true, beq_iff_eq] at hg
        obtain ⟨hnone, hch⟩ := hg
        injection hstep with hs'; subst hs'
        refine rho_ro_lt sk rfl rfl rfl rfl rfl ?_
        cases o with
        | wire =>
            simp only [ropenChoosable, Bool.and_eq_true,
              Bool.not_eq_true'] at hch
            obtain ⟨-, hw⟩ := hch
            simp only [ropenRho, hnone, hw]
            simp
        | res =>
            simp only [ropenChoosable, Bool.and_eq_true,
              Bool.not_eq_true'] at hch
            obtain ⟨⟨-, hr⟩, -⟩ := hch
            simp only [ropenRho, hnone, hr]
            simp
        | query =>
            simp only [ropenChoosable, Bool.and_eq_true,
              decide_eq_true_eq] at hch
            obtain ⟨⟨⟨-, hq⟩, -⟩, -⟩ := hch
            simp only [ropenRho, hnone, hq]
            simp
  | ropenFire =>
      simp only [apply] at hstep
      cases hro : s.ropenCh with
      | none => simp [hro] at hstep
      | some ob =>
          cases ob with
          | wire =>
              simp only [hro] at hstep
              split at hstep
              case isFalse => simp at hstep
              case isTrue hg =>
                injection hstep with hs'; subst hs'
                refine rho_ro_lt sk rfl rfl rfl rfl rfl ?_
                cases hw : s.ropenWire <;>
                  simp only [ropenRho, hro, hw] <;> (simp; try omega)
          | res =>
              simp only [hro] at hstep
              split at hstep
              case isFalse => simp at hstep
              case isTrue hg =>
                injection hstep with hs'; subst hs'
                refine rho_ro_lt sk rfl rfl rfl rfl rfl ?_
                cases hr : s.ropenRes <;>
                  simp only [ropenRho, hro, hr] <;> (simp; try omega)
          | query =>
              simp only [hro] at hstep
              split at hstep
              case isFalse => simp at hstep
              case isTrue hg =>
                injection hstep with hs'; subst hs'
                refine rho_ro_lt sk rfl rfl rfl rfl rfl ?_
                by_cases hq : s.ropenQ < (sk.scope 0).kids.length <;>
                  simp only [ropenRho, hro, hq] <;> (simp; try omega)
  | walkRecvWire pk =>
      simp only [apply] at hstep
      split at hstep
      case isFalse => simp at hstep
      case isTrue hg =>
        simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
        obtain ⟨⟨hmem, hph⟩, hpos⟩ := hg
        have hmem' : pk ∈ sk.walkKeys := by simpa using hmem
        injection hstep with hs'; subst hs'
        exact rho_walk_lt sk _ hmem'
          (walkRho_recvWire_lt sk pk.2 (s.walk pk) hph)
  | walkRecvAsked pk =>
      simp only [apply] at hstep
      split at hstep
      case isFalse => simp at hstep
      case isTrue hg =>
        simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
        obtain ⟨⟨hmem, hph⟩, hpos⟩ := hg
        have hmem' : pk ∈ sk.walkKeys := by simpa using hmem
        injection hstep with hs'; subst hs'
        exact rho_walk_lt sk _ hmem'
          (walkRho_recvAsked_lt sk pk.2 (s.walk pk) hph)
  | walkCommit pk o =>
      simp only [apply] at hstep
      split at hstep
      case isFalse => simp at hstep
      case isTrue hg =>
        simp only [Bool.and_eq_true] at hg
        obtain ⟨hmem, hch⟩ := hg
        have hmem' : pk ∈ sk.walkKeys := by simpa using hmem
        injection hstep with hs'; subst hs'
        exact rho_walk_lt sk s.chan hmem'
          (walkRho_commit_lt sk ax pk (s.walk pk) o hch)
  | walkFire pk =>
      simp only [apply] at hstep
      cases hcm : (s.walk pk).committed with
      | none => simp [hcm] at hstep
      | some o =>
          simp only [hcm] at hstep
          split at hstep
          case isFalse => simp at hstep
          case isTrue hg =>
            simp only [Bool.and_eq_true, beq_iff_eq,
              decide_eq_true_eq] at hg
            obtain ⟨⟨hmem, hph⟩, hchan⟩ := hg
            have hmem' : pk ∈ sk.walkKeys := by simpa using hmem
            injection hstep with hs'; subst hs'
            exact rho_walk_lt sk _ hmem'
              (walkRho_fire_lt sk pk.2 (s.walk pk) o hph hcm)
  | walkCloseWire pk =>
      simp only [apply] at hstep
      split at hstep
      case isFalse => simp at hstep
      case isTrue hg =>
        simp only [Bool.and_eq_true, beq_iff_eq] at hg
        obtain ⟨⟨⟨hmem, hph⟩, hpd⟩, hch0⟩ := hg
        have hmem' : pk ∈ sk.walkKeys := by simpa using hmem
        injection hstep with hs'; subst hs'
        exact rho_walk_lt sk s.chan hmem'
          (walkRho_closeWire_lt sk pk.2 (s.walk pk) hph)
  | walkCloseAsked pk =>
      simp only [apply] at hstep
      split at hstep
      case isFalse => simp at hstep
      case isTrue hg =>
        simp only [Bool.and_eq_true, beq_iff_eq] at hg
        obtain ⟨⟨⟨hmem, hph⟩, hpd⟩, hch0⟩ := hg
        have hmem' : pk ∈ sk.walkKeys := by simpa using hmem
        injection hstep with hs'; subst hs'
        exact rho_walk_lt sk s.chan hmem'
          (walkRho_closeAsked_lt sk pk.2 (s.walk pk) hph)
  | asmRecvRes pk =>
      simp only [apply] at hstep
      split at hstep
      case isFalse => simp at hstep
      case isTrue hg =>
        simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
        obtain ⟨⟨hmem, hph⟩, hpos⟩ := hg
        have hmem' : pk ∈ sk.asmKeys := by simpa using hmem
        injection hstep with hs'; subst hs'
        refine rho_asm_lt sk _ hmem' ?_
        by_cases hp : sk.pendAt pk.1 pk.2 (s.asm pk).idx > 0 <;>
          simp only [asmRho, asmItemRem, hph, hp] <;> simp
  | asmRecvLevel pk =>
      simp only [apply] at hstep
      split at hstep
      case isFalse => simp at hstep
      case isTrue hg =>
        simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
        obtain ⟨⟨hmem, hph⟩, hpos⟩ := hg
        have hmem' : pk ∈ sk.asmKeys := by simpa using hmem
        have hgot : (s.asm pk).got
            < sk.pendAt pk.1 pk.2 (s.asm pk).idx := by
          have hlv' := hlv
          unfold asmLevelsOk at hlv'
          rw [List.all_eq_true] at hlv'
          have h0 := hlv' pk hmem'
          rw [hph] at h0
          simpa using h0
        injection hstep with hs'; subst hs'
        refine rho_asm_lt sk _ hmem' ?_
        cases hbe : ((s.asm pk).got + 1
            == sk.pendAt pk.1 pk.2 (s.asm pk).idx) with
        | true =>
            have he : (s.asm pk).got + 1
                = sk.pendAt pk.1 pk.2 (s.asm pk).idx := by
              simpa using hbe
            simp only [asmRho, asmItemRem, hph]
            simp
            omega
        | false =>
            have hne : (s.asm pk).got + 1
                ≠ sk.pendAt pk.1 pk.2 (s.asm pk).idx := by
              simpa using hbe
            simp only [asmRho, asmItemRem, hph]
            simp
            omega
  | asmSend pk =>
      simp only [apply] at hstep
      split at hstep
      case isFalse => simp at hstep
      case isTrue hg =>
        simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
        obtain ⟨⟨hmem, hph⟩, hcap⟩ := hg
        have hmem' : pk ∈ sk.asmKeys := by simpa using hmem
        injection hstep with hs'; subst hs'
        refine rho_asm_lt sk _ hmem' ?_
        by_cases hl : (s.asm pk).idx + 1
            < (sk.asmResList pk.1 pk.2).length
        · have hc := asmTail_cons sk pk (k := (s.asm pk).idx + 1) hl
          simp only [asmRho, asmItemRem, hph, hl]
          simp
          omega
        · have hpast : asmTail sk pk ((s.asm pk).idx + 1 + 1) = 0 :=
            asmTail_past sk pk (by omega)
          simp only [asmRho, asmItemRem, hph, hl]
          simp
          omega
  | asmClose pk =>
      simp only [apply] at hstep
      split at hstep
      case isFalse => simp at hstep
      case isTrue hg =>
        simp only [Bool.and_eq_true, beq_iff_eq] at hg
        obtain ⟨⟨⟨hmem, hph⟩, hpd⟩, hch0⟩ := hg
        have hmem' : pk ∈ sk.asmKeys := by simpa using hmem
        injection hstep with hs'; subst hs'
        refine rho_asm_lt sk s.chan hmem' ?_
        simp only [asmRho, hph]
        simp
  | absorbRecvWire =>
      simp only [apply] at hstep
      split at hstep
      case isFalse => simp at hstep
      case isTrue hg =>
        simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
        obtain ⟨hph, hpos⟩ := hg
        injection hstep with hs'; subst hs'
        refine rho_ab_lt sk rfl rfl rfl rfl rfl ?_
        simp only [absorbRho, hph]
        simp
  | absorbRecvAsked =>
      simp only [apply] at hstep
      split at hstep
      case isFalse => simp at hstep
      case isTrue hg =>
        simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
        obtain ⟨hph, hpos⟩ := hg
        injection hstep with hs'; subst hs'
        refine rho_ab_lt sk rfl rfl rfl rfl rfl ?_
        simp only [absorbRho, hph]
        simp
  | absorbSend =>
      simp only [apply] at hstep
      split at hstep
      case isFalse => simp at hstep
      case isTrue hg =>
        simp only [Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at hg
        obtain ⟨hph, hcap⟩ := hg
        injection hstep with hs'; subst hs'
        refine rho_ab_lt sk rfl rfl rfl rfl rfl ?_
        by_cases hl : s.absorbIdx + 1 < sk.totalLeafReqs <;>
          simp only [absorbRho, hph, hl] <;> (simp; try omega)
  | absorbCloseWire =>
      simp only [apply] at hstep
      split at hstep
      case isFalse => simp at hstep
      case isTrue hg =>
        simp only [Bool.and_eq_true, beq_iff_eq] at hg
        obtain ⟨⟨hph, hpd⟩, hch0⟩ := hg
        injection hstep with hs'; subst hs'
        refine rho_ab_lt sk rfl rfl rfl rfl rfl ?_
        simp only [absorbRho, hph]
        simp
  | absorbCloseAsked =>
      simp only [apply] at hstep
      split at hstep
      case isFalse => simp at hstep
      case isTrue hg =>
        simp only [Bool.and_eq_true, beq_iff_eq] at hg
        obtain ⟨⟨hph, hpd⟩, hch0⟩ := hg
        injection hstep with hs'; subst hs'
        refine rho_ab_lt sk rfl rfl rfl rfl rfl ?_
        simp only [absorbRho, hph]
        simp
  | finRet =>
      simp only [apply] at hstep
      split at hstep
      case isFalse => simp at hstep
      case isTrue hg =>
        simp only [Bool.and_eq_true, Bool.not_eq_true',
          decide_eq_true_eq] at hg
        obtain ⟨hif, hpos⟩ := hg
        injection hstep with hs'; subst hs'
        refine rho_fin_lt sk rfl rfl rfl rfl rfl ?_
        simp only [finRho, hif]
        simp
  | finRes =>
      simp only [apply] at hstep
      split at hstep
      case isFalse => simp at hstep
      case isTrue hg =>
        simp only [Bool.and_eq_true, Bool.not_eq_true',
          decide_eq_true_eq] at hg
        obtain ⟨hgr, hpos⟩ := hg
        injection hstep with hs'; subst hs'
        refine rho_fin_lt sk rfl rfl rfl rfl rfl ?_
        simp only [finRho, hgr]
        simp
  | finRets =>
      simp only [apply] at hstep
      split at hstep
      case isFalse => simp at hstep
      case isTrue hg =>
        simp only [Bool.and_eq_true, decide_eq_true_eq] at hg
        obtain ⟨⟨hgr, hlt2⟩, hpos⟩ := hg
        injection hstep with hs'; subst hs'
        refine rho_fin_lt sk rfl rfl rfl rfl rfl ?_
        simp only [finRho]
        omega

-- ======================================================= run bounds

/-- Along any successful run, the measure pays for every step. -/
theorem run_length_le (sk : Skel) (ax : AxMode) :
    ∀ {acts : List Action} {s s' : State}, asmLevelsOk sk s = true →
      run sk ax s acts = some s' →
      acts.length + rho sk s' ≤ rho sk s := by
  intro acts
  induction acts with
  | nil =>
      intro s s' _ hrun
      simp only [run, Option.some.injEq] at hrun
      subst hrun
      simp
  | cons a rest ih =>
      intro s s' hlv hrun
      unfold run at hrun
      cases happ : apply sk ax a s with
      | none => simp [happ] at hrun
      | some s₁ =>
          have hrun' : run sk ax s₁ rest = some s' := by
            simpa [happ] using hrun
          have hd := rho_decreases sk ax a hlv happ
          have hlv' := asmLevelsOk_preserved sk ax a happ hlv
          have := ih hlv' hrun'
          simp only [List.length_cons]
          omega

/-- MODEL.md §7's claim (ii), first half, as a kernel theorem: every
run from `init` has length at most ρ(init) — no infinite runs exist,
and bounded checking at depth ρ(init) + 1 is exhaustive. -/
theorem terminating (sk : Skel) (ax : AxMode) {acts : List Action}
    {s' : State} (hrun : run sk ax (init sk) acts = some s') :
    acts.length ≤ rho sk (init sk) := by
  have := run_length_le sk ax (asmLevelsOk_init sk) hrun
  omega

-- ================================================ maximal-run closure

/-- `firstM` over `Option` fails only if every element fails. -/
private theorem firstM_eq_none {α β : Type _} {f : α → Option β} :
    ∀ {l : List α}, l.firstM f = none → ∀ a ∈ l, f a = none := by
  intro l
  induction l with
  | nil => intro _ a ha; cases ha
  | cons x xs ih =>
      intro h a ha
      cases hfx : f x with
      | some b => simp [List.firstM, hfx] at h
      | none =>
          rcases List.mem_cons.mp ha with rfl | ha'
          · exact hfx
          · exact ih (by simpa [List.firstM, hfx] using h) a ha'

/-- `firstM` over `Option` succeeds only through one of its elements. -/
private theorem firstM_eq_some {α β : Type _} {f : α → Option β} {b : β} :
    ∀ {l : List α}, l.firstM f = some b → ∃ a ∈ l, f a = some b := by
  intro l
  induction l with
  | nil => intro h; simp [List.firstM] at h
  | cons x xs ih =>
      intro h
      cases hfx : f x with
      | some b' =>
          simp [List.firstM, hfx] at h
          exact ⟨x, List.mem_cons_self .., by rw [hfx, h]⟩
      | none =>
          simp [List.firstM, hfx] at h
          obtain ⟨a, ha, hfa⟩ := ih h
          exact ⟨a, List.mem_cons_of_mem x ha, hfa⟩

/-- The greedy drain with fuel at least ρ reaches quiescence: each step
strictly decreases ρ, so the fixpoint arrives before the fuel runs
out. -/
theorem drain_quiescent (sk : Skel) (ax : AxMode) :
    ∀ (fuel : Nat) (s : State), asmLevelsOk sk s = true →
      rho sk s ≤ fuel →
      canStep sk ax (Control.drain sk ax fuel s) = false := by
  intro fuel
  induction fuel with
  | zero =>
      intro s hlv hle
      unfold Control.drain
      rw [canStep, List.any_eq_false]
      intro a _
      cases happ : apply sk ax a s with
      | none => simp
      | some s₁ =>
          have := rho_decreases sk ax a hlv happ
          omega
  | succ n ih =>
      intro s hlv hle
      unfold Control.drain
      cases hf : (allActions sk).firstM (fun a => apply sk ax a s) with
      | none =>
          rw [canStep, List.any_eq_false]
          intro a ha
          rw [firstM_eq_none hf a ha]
          simp
      | some s₁ =>
          obtain ⟨a, -, ha⟩ := firstM_eq_some hf
          have hd := rho_decreases sk ax a hlv ha
          exact ih s₁ (asmLevelsOk_preserved sk ax a ha hlv) (by omega)

/-- MODEL.md §7's claim (ii), second half, under the flagship's
hypotheses: a maximal run from `init` — one whose final state admits no
step — ends `Terminal`. Composes `rho_decreases` (maximal runs exist
and are short) with the progress flagship `Sched.deadlock_free`. -/
theorem maximal_run_terminal (sk : Skel) (hwf : sk.wellFormed = true)
    (hm0 : ∀ sc, sk.dCount sc ≤ sk.capLevel) {acts : List Action}
    {s' : State} (hrun : run sk .impl (init sk) acts = some s')
    (hmax : canStep sk .impl s' = false) :
    terminal sk s' = true := by
  have hr := run_reachable sk .impl hrun
  have hdf := Sched.deadlock_free sk hwf hm0 s' hr
  unfold stuck at hdf
  rw [hmax] at hdf
  simpa using hdf

/-- `maximal_run_terminal` at the design space's other corner: the
parent-early discipline, any capacity (`Sched.deadlock_free_d5`). -/
theorem maximal_run_terminal_d5 (sk : Skel) (hwf : sk.wellFormed = true)
    (hsched : sk.schedulable = true) {acts : List Action}
    {s' : State} (hrun : run sk .full (init sk) acts = some s')
    (hmax : canStep sk .full s' = false) :
    terminal sk s' = true := by
  have hr := run_reachable sk .full hrun
  have hdf := Sched.deadlock_free_d5 sk hwf hsched s' hr
  unfold stuck at hdf
  rw [hmax] at hdf
  simpa using hdf

/-- The constructive package: on every well-formed margin-0 skeleton
the greedy drain reaches `Terminal` within ρ(init) steps — termination
with an explicit fuel bound, no fairness hypothesis anywhere. -/
theorem greedy_run_terminal (sk : Skel) (hwf : sk.wellFormed = true)
    (hm0 : ∀ sc, sk.dCount sc ≤ sk.capLevel) :
    terminal sk
      (Control.drain sk .impl (rho sk (init sk)) (init sk)) = true := by
  have hq := drain_quiescent sk .impl (rho sk (init sk)) (init sk)
    (asmLevelsOk_init sk) (Nat.le_refl _)
  have hr := Control.drain_reachable sk .impl (rho sk (init sk))
    (Reachable.init)
  have hdf := Sched.deadlock_free sk hwf hm0 _ hr
  unfold stuck at hdf
  rw [hq] at hdf
  simpa using hdf

end StreamingMirror.Model
