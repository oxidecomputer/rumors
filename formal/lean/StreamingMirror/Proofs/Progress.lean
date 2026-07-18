/-
The progress lemma's per-process enabledness layer, first pillar: the
committed-choice publisher can never be stuck at the CHOICE point. A
phase-2 uncommitted walk always has a choosable obligation — the least
unmet obligation of its current scope, taken in (res|query of the least
undischarged D child, when its wire is done) ≺ wire ≺ parent order,
passes every axiom guard in EVERY axiom mode — so blocking only ever
happens at channel operations, never at obligation choice.
-/
import StreamingMirror.Proofs.Lemmas

namespace StreamingMirror.Model

variable {sk : Skel} {ax : AxMode} {s : State} {pk : Party × Nat}

/-- One enabled enumerated action is enough to step: `canStep` is the
existential over `allActions`. -/
theorem canStep_of_action {a : Action} (ha : a ∈ allActions sk)
    (happ : (apply sk ax a s).isSome = true) : canStep sk ax s = true := by
  rw [canStep, List.any_eq_true]
  exact ⟨a, ha, happ⟩

/-- The enumeration covers the parent commit of every walk key: choosing
`.parent` is always an action the system is allowed to consider. -/
theorem walkCommit_parent_mem (hpk : pk ∈ sk.walkKeys) :
    Action.walkCommit pk .parent ∈ allActions sk := by
  rw [allActions]
  refine List.mem_append.mpr (.inl (List.mem_append.mpr (.inr ?_)))
  refine List.mem_flatMap.mpr ⟨pk, hpk, ?_⟩
  exact List.mem_append.mpr (.inl (by simp))

/-- The enumeration covers every fan-bounded wire commit of every walk
key. Child indices of a real scope are fan-bounded (`nChildren_le_fan`),
so every wire obligation the choice logic can produce is enumerated. -/
theorem walkCommit_wire_mem (hpk : pk ∈ sk.walkKeys) {i : Nat}
    (hi : i < sk.fan) :
    Action.walkCommit pk (.wire i) ∈ allActions sk := by
  rw [allActions]
  refine List.mem_append.mpr (.inl (List.mem_append.mpr (.inr ?_)))
  refine List.mem_flatMap.mpr ⟨pk, hpk, ?_⟩
  refine List.mem_append.mpr (.inr ?_)
  refine List.mem_flatMap.mpr ⟨i, List.mem_range.mpr hi, ?_⟩
  simp

/-- The enumeration covers every fan-bounded resolution commit of every
walk key; the `.res` twin of `walkCommit_wire_mem`. -/
theorem walkCommit_res_mem (hpk : pk ∈ sk.walkKeys) {i : Nat}
    (hi : i < sk.fan) :
    Action.walkCommit pk (.res i) ∈ allActions sk := by
  rw [allActions]
  refine List.mem_append.mpr (.inl (List.mem_append.mpr (.inr ?_)))
  refine List.mem_flatMap.mpr ⟨pk, hpk, ?_⟩
  refine List.mem_append.mpr (.inr ?_)
  refine List.mem_flatMap.mpr ⟨i, List.mem_range.mpr hi, ?_⟩
  simp

/-- The enumeration covers every fan-bounded query commit of every walk
key; the `.query` twin of `walkCommit_wire_mem`. -/
theorem walkCommit_query_mem (hpk : pk ∈ sk.walkKeys) {i : Nat}
    (hi : i < sk.fan) :
    Action.walkCommit pk (.query i) ∈ allActions sk := by
  rw [allActions]
  refine List.mem_append.mpr (.inl (List.mem_append.mpr (.inr ?_)))
  refine List.mem_flatMap.mpr ⟨pk, hpk, ?_⟩
  refine List.mem_append.mpr (.inr ?_)
  refine List.mem_flatMap.mpr ⟨i, List.mem_range.mpr hi, ?_⟩
  simp

/-- Least-witness extraction for a bounded Boolean search: any witness
below `n` yields the least one, with the predicate refuted strictly
below it. The choice logic uses this to name the least undone wire and
the least undischarged D child. -/
theorem exists_least_of_exists_lt {p : Nat → Bool} :
    ∀ {n : Nat}, (∃ j, j < n ∧ p j = true) →
      ∃ j, j < n ∧ p j = true ∧ ∀ k, k < j → p k = false := by
  intro n
  induction n with
  | zero => rintro ⟨j, hj, -⟩; omega
  | succ n ih =>
      rintro ⟨j, hjn, hpj⟩
      by_cases hn : (List.range n).any p = true
      · rw [List.any_eq_true] at hn
        obtain ⟨m, hm, hpm⟩ := hn
        rw [List.mem_range] at hm
        obtain ⟨m', hm', hpm', hleast⟩ := ih ⟨m, hm, hpm⟩
        exact ⟨m', by omega, hpm', hleast⟩
      · have hnone : ∀ k, k < n → p k = false := by
          intro k hk
          cases hpk : p k with
          | false => rfl
          | true =>
              exact absurd (List.any_eq_true.mpr
                ⟨k, List.mem_range.mpr hk, hpk⟩) hn
        have hjeq : j = n := by
          by_contra hne
          have hjlt : j < n := by omega
          rw [hnone j hjlt] at hpj
          cases hpj
        exact ⟨n, by omega, hjeq ▸ hpj, hnone⟩

/-- The `d5` discharge shape shared by the wire/query introduction
rules: the parent is already sent, or some D child of the scope is
still unresolved (so the parent-placement ledger does not yet bind). -/
def D5Free (ws : WalkSt) (n : Nat) (isD : Nat → Bool) : Prop :=
  ws.parentDone = true ∨ ∃ j, j < n ∧ isD j = true ∧ ws.resDone j = false

/-- Reduce the guard's `d5` conjunct to `D5Free`. -/
private theorem d5_conjunct {ws : WalkSt} {n : Nat} {isD : Nat → Bool}
    (hd5 : D5Free ws n isD) :
    (ax.d5 = false ∨ ws.parentDone = true) ∨
      ((List.range n).all fun j => !isD j || ws.resDone j) = false := by
  rcases hd5 with hpd | ⟨j, hj, hD, hr⟩
  · exact Or.inl (Or.inr hpd)
  · refine Or.inr ?_
    cases hall : (List.range n).all fun j => !isD j || ws.resDone j with
    | false => rfl
    | true =>
        have := List.all_eq_true.mp hall j (List.mem_range.mpr hj)
        rw [hD, hr] at this
        cases this

/-- Introduction rule for committing `.wire i` in phase 2: an undone
wire whose earlier wires are all done and whose earlier D children are
all discharged passes the wire guard outright — every `(!ax.flag || _)`
conjunct is settled on the right, so the commit is choosable in every
axiom mode (`hd5` settles the parent-placement conjunct). -/
theorem wkChoosable_wire_intro {ws : WalkSt} {i : Nat}
    (hph : ws.phase = 2) (hco : ws.committed = none)
    (hin : i < sk.nChildren pk.2 (sk.stageScope pk.2 ws.scope))
    (hund : ws.wireDone i = false)
    (hpre : ∀ j, j < i → ws.wireDone j = true)
    (hdis : ∀ j, j < i →
      sk.childIsD pk.2 (sk.stageScope pk.2 ws.scope) j = true →
      ws.resDone j = true ∧
        ws.qSent j = sk.qCount pk.2 (sk.stageScope pk.2 ws.scope) j)
    (hd5 : D5Free ws (sk.nChildren pk.2 (sk.stageScope pk.2 ws.scope))
      (sk.childIsD pk.2 (sk.stageScope pk.2 ws.scope))) :
    wkChoosable sk ax pk ws (.wire i) = true := by
  simp only [wkChoosable, hph, hco, bne_self_eq_false, Option.isSome_none,
    Bool.or_self, Bool.false_eq_true, if_false, Bool.and_eq_true,
    decide_eq_true_eq, Bool.not_eq_true', List.all_eq_true, List.mem_range,
    Bool.or_eq_true, beq_iff_eq]
  refine ⟨⟨⟨⟨hin, hund⟩, hpre⟩, Or.inr ?_⟩, ?_⟩
  · intro j hj
    cases hD : sk.childIsD pk.2 (sk.stageScope pk.2 ws.scope) j with
    | false => exact Or.inl rfl
    | true => exact Or.inr (hdis j hj hD)
  · simpa using d5_conjunct (ax := ax) hd5

/-- Introduction rule for committing `.res i` in phase 2: an unresolved
D child whose wire is done, whose earlier D children are all resolved,
and whose scope has no resolved child owing queries passes the res guard
outright, in every axiom mode. -/
theorem wkChoosable_res_intro {ws : WalkSt} {i : Nat}
    (hph : ws.phase = 2) (hco : ws.committed = none)
    (hin : i < sk.nChildren pk.2 (sk.stageScope pk.2 ws.scope))
    (hD : sk.childIsD pk.2 (sk.stageScope pk.2 ws.scope) i = true)
    (hnr : ws.resDone i = false)
    (hwire : ws.wireDone i = true)
    (hDpre : ∀ j, j < i →
      sk.childIsD pk.2 (sk.stageScope pk.2 ws.scope) j = true →
      ws.resDone j = true)
    (hd3 : ∀ j, j < sk.nChildren pk.2 (sk.stageScope pk.2 ws.scope) →
      ws.resDone j = true →
      ws.qSent j = sk.qCount pk.2 (sk.stageScope pk.2 ws.scope) j) :
    wkChoosable sk ax pk ws (.res i) = true := by
  simp only [wkChoosable, hph, hco, bne_self_eq_false, Option.isSome_none,
    Bool.or_self, Bool.false_eq_true, if_false, Bool.and_eq_true,
    decide_eq_true_eq, Bool.not_eq_true', List.all_eq_true, List.mem_range,
    Bool.or_eq_true, beq_iff_eq]
  refine ⟨⟨⟨⟨⟨hin, hD⟩, hnr⟩, ?_⟩, Or.inr hwire⟩, Or.inr ?_⟩
  · intro j hj
    cases hDj : sk.childIsD pk.2 (sk.stageScope pk.2 ws.scope) j with
    | false => exact Or.inl rfl
    | true => exact Or.inr (hDpre j hj hDj)
  · intro j hj
    cases hr : ws.resDone j with
    | false => exact Or.inl rfl
    | true => exact Or.inr (hd3 j hj hr)

/-- Introduction rule for committing `.query i` in phase 2: a resolved,
wire-done D child still owing queries, with every earlier child's query
quota met, passes the query guard outright, in every axiom mode. -/
theorem wkChoosable_query_intro {ws : WalkSt} {i : Nat}
    (hph : ws.phase = 2) (hco : ws.committed = none)
    (hin : i < sk.nChildren pk.2 (sk.stageScope pk.2 ws.scope))
    (hD : sk.childIsD pk.2 (sk.stageScope pk.2 ws.scope) i = true)
    (hq : ws.qSent i < sk.qCount pk.2 (sk.stageScope pk.2 ws.scope) i)
    (hqpre : ∀ j, j < i →
      ws.qSent j = sk.qCount pk.2 (sk.stageScope pk.2 ws.scope) j)
    (hres : ws.resDone i = true)
    (hwire : ws.wireDone i = true)
    (hd5 : D5Free ws (sk.nChildren pk.2 (sk.stageScope pk.2 ws.scope))
      (sk.childIsD pk.2 (sk.stageScope pk.2 ws.scope))) :
    wkChoosable sk ax pk ws (.query i) = true := by
  simp only [wkChoosable, hph, hco, bne_self_eq_false, Option.isSome_none,
    Bool.or_self, Bool.false_eq_true, if_false, Bool.and_eq_true,
    decide_eq_true_eq, Bool.not_eq_true', List.all_eq_true, List.mem_range,
    Bool.or_eq_true, beq_iff_eq]
  refine ⟨⟨⟨⟨⟨⟨hin, hD⟩, hq⟩, hqpre⟩, Or.inr hres⟩, Or.inr hwire⟩, ?_⟩
  simpa using d5_conjunct (ax := ax) hd5

/-- Introduction rule for committing `.parent` in phase 2: when the
parent is unsent and every D child of the scope is resolved, the parent
guard passes outright, in every axiom mode. -/
theorem wkChoosable_parent_intro {ws : WalkSt}
    (hph : ws.phase = 2) (hco : ws.committed = none)
    (hnp : ws.parentDone = false)
    (hd2 : ∀ j, j < sk.nChildren pk.2 (sk.stageScope pk.2 ws.scope) →
      sk.childIsD pk.2 (sk.stageScope pk.2 ws.scope) j = true →
      ws.resDone j = true) :
    wkChoosable sk ax pk ws .parent = true := by
  simp only [wkChoosable, hph, hco, bne_self_eq_false, Option.isSome_none,
    Bool.or_self, Bool.false_eq_true, if_false, Bool.and_eq_true,
    Bool.not_eq_true', List.all_eq_true, List.mem_range, Bool.or_eq_true]
  refine ⟨hnp, Or.inr ?_⟩
  intro j hj
  cases hDj : sk.childIsD pk.2 (sk.stageScope pk.2 ws.scope) j with
  | false => exact Or.inl rfl
  | true => exact Or.inr (hd2 j hj hDj)

/-- Case B of the obligation choice: if some wire of the current scope
is undone and every D child is either discharged or sits at-or-above an
undone wire, then the least undone wire is choosable — its prefix of
wires is done by minimality, and no undischarged D child fits below it.
-/
theorem wkChoosable_wire_of_undone {ws : WalkSt}
    (hph : ws.phase = 2) (hco : ws.committed = none)
    (hWex : ∃ j, j < sk.nChildren pk.2 (sk.stageScope pk.2 ws.scope) ∧
      ws.wireDone j = false)
    (hdis_or : ∀ j, j < sk.nChildren pk.2 (sk.stageScope pk.2 ws.scope) →
      sk.childIsD pk.2 (sk.stageScope pk.2 ws.scope) j = true →
      (ws.resDone j = true ∧
        ws.qSent j = sk.qCount pk.2 (sk.stageScope pk.2 ws.scope) j) ∨
        ∃ k, k ≤ j ∧ ws.wireDone k = false)
    (hd5 : D5Free ws (sk.nChildren pk.2 (sk.stageScope pk.2 ws.scope))
      (sk.childIsD pk.2 (sk.stageScope pk.2 ws.scope))) :
    ∃ i, i < sk.nChildren pk.2 (sk.stageScope pk.2 ws.scope) ∧
      wkChoosable sk ax pk ws (.wire i) = true := by
  have hWex' : ∃ j, j < sk.nChildren pk.2 (sk.stageScope pk.2 ws.scope) ∧
      (!ws.wireDone j) = true := by
    obtain ⟨j, hj, hw⟩ := hWex
    exact ⟨j, hj, by simp [hw]⟩
  obtain ⟨w, hwn, hpw, hmin⟩ := exists_least_of_exists_lt hWex'
  have hwund : ws.wireDone w = false := by simpa using hpw
  have hwpre : ∀ k, k < w → ws.wireDone k = true := by
    intro k hk
    have hk' : (!ws.wireDone k) = false := hmin k hk
    simpa using hk'
  refine ⟨w, hwn, wkChoosable_wire_intro hph hco hwn hwund hwpre ?_ hd5⟩
  intro j hj hD
  rcases hdis_or j (by omega) hD with hd | ⟨k, hkj, hkw⟩
  · exact hd
  · have hkdone : ws.wireDone k = true := hwpre k (by omega)
    rw [hkw] at hkdone
    cases hkdone

/-- A phase-2 walk that has not committed always has a choosable
obligation, together with its enumeration witness: the parent when
every D child is resolved and the parent is unsent (the `d5` ledger's
mandated placement), otherwise the least unmet obligation of the
current scope, taken in (res|query of the least undischarged D child,
when its wire is done) ≺ wire ≺ parent order — and each choice passes
every axiom guard in EVERY axiom mode. -/
theorem walk_uncommitted_choosable (hwf : sk.wellFormed = true)
    (hi : InvP sk ax s) (hpk : pk ∈ sk.walkKeys)
    (hph : (s.walk pk).phase = 2) (hco : (s.walk pk).committed = none) :
    ∃ o : Oblig, wkChoosable sk ax pk (s.walk pk) o = true ∧
      Action.walkCommit pk o ∈ allActions sk := by
  -- Extract the phase-2 branch of the walk's local invariant.
  have hwk := hi.wk pk hpk
  simp only [wkLocalOk] at hwk
  rw [hph, hco] at hwk
  simp at hwk
  obtain ⟨hscope, hnsc, hall⟩ := hwk
  have hn_fan : sk.nChildren pk.2 (sk.stageScope pk.2 (s.walk pk).scope)
      ≤ sk.fan := nChildren_le_fan hwf hscope
  -- Clean per-child facts (the ax-independent ledger conjuncts).
  have hres_D : ∀ j, j < sk.fan → (s.walk pk).resDone j = true →
      sk.childIsD pk.2 (sk.stageScope pk.2 (s.walk pk).scope) j = true := by
    intro j hj hr
    obtain ⟨⟨⟨⟨⟨⟨⟨⟨⟨-, c2⟩, -⟩, -⟩, -⟩, -⟩, -⟩, -⟩, -⟩, -⟩ := hall j hj
    rcases c2 with hf | ⟨-, hD⟩
    · rw [hr] at hf; cases hf
    · exact hD
  have hq_le : ∀ j, j < sk.fan → (s.walk pk).qSent j ≤
      sk.qCount pk.2 (sk.stageScope pk.2 (s.walk pk).scope) j := by
    intro j hj
    obtain ⟨⟨⟨⟨⟨⟨⟨⟨⟨-, -⟩, c3⟩, -⟩, -⟩, -⟩, -⟩, -⟩, -⟩, -⟩ := hall j hj
    exact c3
  have hres_pre : ∀ j, j < sk.fan → (s.walk pk).resDone j = true →
      ∀ k, k < j →
      sk.childIsD pk.2 (sk.stageScope pk.2 (s.walk pk).scope) k = true →
      (s.walk pk).resDone k = true := by
    intro j hj hr k hk hDk
    obtain ⟨⟨⟨⟨⟨⟨⟨⟨⟨-, -⟩, -⟩, -⟩, c5⟩, -⟩, -⟩, -⟩, -⟩, -⟩ := hall j hj
    rcases c5 with hf | hpre
    · rw [hr] at hf; cases hf
    · rcases hpre k hk with hDf | hres
      · rw [hDk] at hDf; cases hDf
      · exact hres
  -- Parent placement first: when every D child is resolved and the
  -- parent is unsent, the parent itself is choosable in every mode
  -- (`d2` is settled by the resolutions) — and it is the only choice
  -- the `d5` ledger leaves open there, so the enumeration must pick it.
  -- Otherwise `D5Free` holds and the pre-`d5` enumeration below goes
  -- through unchanged.
  have hd5exit : (∃ o : Oblig, wkChoosable sk ax pk (s.walk pk) o = true ∧
      Action.walkCommit pk o ∈ allActions sk) ∨
      D5Free (s.walk pk)
        (sk.nChildren pk.2 (sk.stageScope pk.2 (s.walk pk).scope))
        (sk.childIsD pk.2 (sk.stageScope pk.2 (s.walk pk).scope)) := by
    by_cases hAllD : (∀ j, j < sk.nChildren pk.2
        (sk.stageScope pk.2 (s.walk pk).scope) →
        sk.childIsD pk.2 (sk.stageScope pk.2 (s.walk pk).scope) j = true →
        (s.walk pk).resDone j = true)
    · cases hpdv : (s.walk pk).parentDone with
      | true => exact Or.inr (Or.inl hpdv)
      | false =>
          exact Or.inl ⟨.parent, wkChoosable_parent_intro hph hco hpdv hAllD,
            walkCommit_parent_mem hpk⟩
    · refine Or.inr (Or.inr ?_)
      by_contra hno
      refine hAllD (fun j hj hD => ?_)
      cases hr : (s.walk pk).resDone j with
      | true => rfl
      | false => exact absurd ⟨j, hj, hD, hr⟩ hno
  rcases hd5exit with hdone | hd5
  · exact hdone
  -- Split on whether the scope has an undischarged D child.
  by_cases hDex : ((List.range (sk.nChildren pk.2
      (sk.stageScope pk.2 (s.walk pk).scope))).any fun j =>
      sk.childIsD pk.2 (sk.stageScope pk.2 (s.walk pk).scope) j &&
        !((s.walk pk).resDone j && (s.walk pk).qSent j ==
          sk.qCount pk.2 (sk.stageScope pk.2 (s.walk pk).scope) j)) = true
  · -- Some D child is undischarged: name the least one, `js`.
    rw [List.any_eq_true] at hDex
    obtain ⟨j0, hj0, hbp0⟩ := hDex
    rw [List.mem_range] at hj0
    obtain ⟨js, hjs_n, hjs_bp, hjs_min⟩ :=
      exists_least_of_exists_lt (p := fun j =>
        sk.childIsD pk.2 (sk.stageScope pk.2 (s.walk pk).scope) j &&
          !((s.walk pk).resDone j && (s.walk pk).qSent j ==
            sk.qCount pk.2 (sk.stageScope pk.2 (s.walk pk).scope) j))
        ⟨j0, hj0, hbp0⟩
    simp only [Bool.and_eq_true, Bool.not_eq_true'] at hjs_bp
    obtain ⟨hjsD, hjs_und⟩ := hjs_bp
    have hjs_dis : ∀ k, k < js →
        sk.childIsD pk.2 (sk.stageScope pk.2 (s.walk pk).scope) k = true →
        (s.walk pk).resDone k = true ∧ (s.walk pk).qSent k =
          sk.qCount pk.2 (sk.stageScope pk.2 (s.walk pk).scope) k := by
      intro k hk hDk
      have hbpk := hjs_min k hk
      simp [hDk] at hbpk
      exact hbpk
    by_cases hwd : (s.walk pk).wireDone js = true
    · by_cases hrd : (s.walk pk).resDone js = true
      · -- Case A2: `js` is resolved, so it owes queries. Choose `.query js`.
        have hq_ne : ¬ ((s.walk pk).qSent js =
            sk.qCount pk.2 (sk.stageScope pk.2 (s.walk pk).scope) js) := by
          intro heq
          rw [hrd, heq] at hjs_und
          simp at hjs_und
        have hq_lt : (s.walk pk).qSent js <
            sk.qCount pk.2 (sk.stageScope pk.2 (s.walk pk).scope) js := by
          have hle := hq_le js (by omega)
          omega
        have hqpre : ∀ k, k < js → (s.walk pk).qSent k =
            sk.qCount pk.2 (sk.stageScope pk.2 (s.walk pk).scope) k := by
          intro k hk
          cases hDk : sk.childIsD pk.2
              (sk.stageScope pk.2 (s.walk pk).scope) k with
          | true => exact (hjs_dis k hk hDk).2
          | false =>
              have hq0 : sk.qCount pk.2
                  (sk.stageScope pk.2 (s.walk pk).scope) k = 0 := by
                simp [Skel.qCount, hDk]
              have hle := hq_le k (by omega)
              omega
        exact ⟨.query js,
          wkChoosable_query_intro hph hco hjs_n hjsD hq_lt hqpre hrd hwd hd5,
          walkCommit_query_mem hpk (by omega)⟩
      · -- Case A1: `js` is unresolved. Choose `.res js`.
        have hnr : (s.walk pk).resDone js = false := by
          cases h : (s.walk pk).resDone js with
          | false => rfl
          | true => exact absurd h hrd
        have hDpre : ∀ k, k < js →
            sk.childIsD pk.2 (sk.stageScope pk.2 (s.walk pk).scope) k = true →
            (s.walk pk).resDone k = true :=
          fun k hk hDk => (hjs_dis k hk hDk).1
        have hd3 : ∀ k,
            k < sk.nChildren pk.2 (sk.stageScope pk.2 (s.walk pk).scope) →
            (s.walk pk).resDone k = true → (s.walk pk).qSent k =
              sk.qCount pk.2 (sk.stageScope pk.2 (s.walk pk).scope) k := by
          intro k hkn hrk
          rcases Nat.lt_trichotomy k js with hlt | heq | hgt
          · exact (hjs_dis k hlt (hres_D k (by omega) hrk)).2
          · exact absurd (heq ▸ hrk) hrd
          · have hres_js := hres_pre k (by omega) hrk js hgt hjsD
            rw [hnr] at hres_js
            cases hres_js
        exact ⟨.res js,
          wkChoosable_res_intro hph hco hjs_n hjsD hnr hwd hDpre hd3,
          walkCommit_res_mem hpk (by omega)⟩
    · -- `js`'s wire is undone: choose the least undone wire (Case B).
      have hwdf : (s.walk pk).wireDone js = false := by
        cases h : (s.walk pk).wireDone js with
        | false => rfl
        | true => exact absurd h hwd
      have hdis_or : ∀ j,
          j < sk.nChildren pk.2 (sk.stageScope pk.2 (s.walk pk).scope) →
          sk.childIsD pk.2 (sk.stageScope pk.2 (s.walk pk).scope) j = true →
          ((s.walk pk).resDone j = true ∧ (s.walk pk).qSent j =
            sk.qCount pk.2 (sk.stageScope pk.2 (s.walk pk).scope) j) ∨
          ∃ k, k ≤ j ∧ (s.walk pk).wireDone k = false := by
        intro j hj hDj
        by_cases hjlt : j < js
        · exact Or.inl (hjs_dis j hjlt hDj)
        · exact Or.inr ⟨js, by omega, hwdf⟩
      obtain ⟨w, hwn, hch⟩ := wkChoosable_wire_of_undone hph hco
        ⟨js, hjs_n, hwdf⟩ hdis_or hd5
      exact ⟨.wire w, hch, walkCommit_wire_mem hpk (by omega)⟩
  · -- Every D child is discharged.
    have hDall : ∀ j,
        j < sk.nChildren pk.2 (sk.stageScope pk.2 (s.walk pk).scope) →
        sk.childIsD pk.2 (sk.stageScope pk.2 (s.walk pk).scope) j = true →
        (s.walk pk).resDone j = true ∧ (s.walk pk).qSent j =
          sk.qCount pk.2 (sk.stageScope pk.2 (s.walk pk).scope) j := by
      intro j hj hDj
      cases hbpj : (sk.childIsD pk.2 (sk.stageScope pk.2 (s.walk pk).scope) j &&
          !((s.walk pk).resDone j && (s.walk pk).qSent j ==
            sk.qCount pk.2 (sk.stageScope pk.2 (s.walk pk).scope) j)) with
      | true =>
          exact absurd (List.any_eq_true.mpr
            ⟨j, List.mem_range.mpr hj, hbpj⟩) hDex
      | false =>
          simp [hDj] at hbpj
          exact hbpj
    by_cases hW : ((List.range (sk.nChildren pk.2
        (sk.stageScope pk.2 (s.walk pk).scope))).any fun j =>
        !(s.walk pk).wireDone j) = true
    · -- Some wire is undone: choose the least undone wire (Case B).
      rw [List.any_eq_true] at hW
      obtain ⟨j0, hj0, hw0⟩ := hW
      rw [List.mem_range] at hj0
      have hw0' : (s.walk pk).wireDone j0 = false := by simpa using hw0
      obtain ⟨w, hwn, hch⟩ := wkChoosable_wire_of_undone hph hco
        ⟨j0, hj0, hw0'⟩ (fun j hj hDj => Or.inl (hDall j hj hDj)) hd5
      exact ⟨.wire w, hch, walkCommit_wire_mem hpk (by omega)⟩
    · -- Case C: wires done, D children discharged; only the parent can
      -- be unmet, and `scopeComplete = false` says it is. Choose `.parent`.
      have hWall : ∀ j,
          j < sk.nChildren pk.2 (sk.stageScope pk.2 (s.walk pk).scope) →
          (s.walk pk).wireDone j = true := by
        intro j hj
        cases hwj : (s.walk pk).wireDone j with
        | true => rfl
        | false =>
            exact absurd (List.any_eq_true.mpr
              ⟨j, List.mem_range.mpr hj, by simp [hwj]⟩) hW
      have hnge : ¬ ((s.walk pk).scope ≥ sk.stageLen pk.2) := by omega
      have hpd : (s.walk pk).parentDone = false := by
        cases hpdv : (s.walk pk).parentDone with
        | false => rfl
        | true =>
            exfalso
            have hsc_true : scopeComplete sk pk.2 (s.walk pk) = true := by
              simp only [scopeComplete]
              rw [if_neg hnge, hpdv, Bool.true_and, List.all_eq_true]
              intro i hi
              rw [List.mem_range] at hi
              rw [hWall i hi]
              cases hDi : sk.childIsD pk.2
                  (sk.stageScope pk.2 (s.walk pk).scope) i with
              | false => simp
              | true =>
                  obtain ⟨hr, hq⟩ := hDall i hi hDi
                  simp [hr, hq]
            rw [hsc_true] at hnsc
            cases hnsc
      exact ⟨.parent,
        wkChoosable_parent_intro hph hco hpd
          (fun j hj hDj => (hDall j hj hDj).1),
        walkCommit_parent_mem hpk⟩

/-- A phase-2 walk that has not committed always has a choosable
obligation — the least unmet obligation of its current scope, taken in
(res|query of least undischarged D child if its wire is done) ≺ wire ≺
parent order, passes every axiom guard in EVERY axiom mode. Hence the
committed-choice split can never deadlock at the choice point. -/
theorem walk_uncommitted_canStep (hwf : sk.wellFormed = true)
    (hi : InvP sk ax s) (hpk : pk ∈ sk.walkKeys)
    (hph : (s.walk pk).phase = 2) (hco : (s.walk pk).committed = none) :
    canStep sk ax s = true := by
  obtain ⟨o, hch, hmem⟩ := walk_uncommitted_choosable hwf hi hpk hph hco
  have happ : (apply sk ax (.walkCommit pk o) s).isSome = true := by
    simp [apply, hpk, hch]
  exact canStep_of_action hmem happ

-- ================================== opener mirrors of the pillar
-- The openers are the only other committed-choice processes (asms,
-- absorb, and the finishes are linear: every action is a channel
-- operation or close determined by phase alone, so the pillar's
-- content is vacuous for them). Their mirrors are direct: the guard
-- order under any mode is wire ≺ query (initiator) and wire ≺ res ≺
-- query (responder), and choosing the first unfired obligation settles
-- every `(!ax.flag || _)` conjunct on the right.

/-- The enumeration covers both initiator opening commits. -/
theorem iopenChoose_mem {o : IOblig} :
    Action.iopenChoose o ∈ allActions sk := by
  rw [allActions]
  refine List.mem_append.mpr (.inl (List.mem_append.mpr (.inl ?_)))
  cases o <;> simp

/-- The enumeration covers all three responder opening commits. -/
theorem ropenChoose_mem {o : ROblig} :
    Action.ropenChoose o ∈ allActions sk := by
  rw [allActions]
  refine List.mem_append.mpr (.inl (List.mem_append.mpr (.inl ?_)))
  cases o <;> simp

/-- An unfinished initiator opening at its choice point can always
commit: the unfired wire first, else the query (whose `w` guard the
fired wire settles) — in every axiom mode. The openers' mirror of
`walk_uncommitted_canStep`. -/
theorem iopen_unchosen_canStep (hnd : doneIOpen s = false)
    (hch : s.iopenCh = none) : canStep sk ax s = true := by
  rw [doneIOpen, Bool.and_eq_false_iff] at hnd
  cases hw : s.iopenWire with
  | false =>
      have happ : (apply sk ax (.iopenChoose .wire) s).isSome = true := by
        simp [apply, hch, iopenChoosable, hw]
      exact canStep_of_action iopenChoose_mem happ
  | true =>
      have hq : s.iopenQuery = false := by
        rcases hnd with h | h
        · rw [hw] at h; cases h
        · exact h
      have happ : (apply sk ax (.iopenChoose .query) s).isSome = true := by
        simp [apply, hch, iopenChoosable, hq, hw]
      exact canStep_of_action iopenChoose_mem happ

/-- An unfinished responder opening past its wire receive can always
commit at its choice point: the first unfired obligation in wire ≺ res ≺
query order passes every axiom guard in every mode (`w` and `wireFirst`
settled by the fired wire, `d1root` by the fired res; the query count
stays choosable because the invariant caps it at `rootPending`). The
responder mirror of `walk_uncommitted_canStep`. -/
theorem ropen_unchosen_canStep (hi : InvP sk ax s)
    (hgw : s.ropenGotWire = true) (hnd : doneROpen sk s = false)
    (hch : s.ropenCh = none) : canStep sk ax s = true := by
  cases hw : s.ropenWire with
  | false =>
      have happ : (apply sk ax (.ropenChoose .wire) s).isSome = true := by
        simp [apply, hch, ropenChoosable, hgw, hw]
      exact canStep_of_action ropenChoose_mem happ
  | true =>
      cases hr : s.ropenRes with
      | false =>
          have happ : (apply sk ax (.ropenChoose .res) s).isSome = true := by
            simp [apply, hch, ropenChoosable, hgw, hr, hw]
          exact canStep_of_action ropenChoose_mem happ
      | true =>
          -- Wire and res fired: `doneROpen = false` leaves only the
          -- query count short, and the invariant bounds it strictly.
          have htop := hi.top
          simp only [topLocalOk, Bool.and_eq_true, decide_eq_true_eq]
            at htop
          obtain ⟨⟨⟨⟨⟨⟨⟨⟨⟨-, hqle⟩, -⟩, -⟩, -⟩, -⟩, -⟩, -⟩, -⟩, -⟩ := htop
          rw [doneROpen, hgw, hw, hr] at hnd
          simp only [Bool.true_and] at hnd
          have hqlt : s.ropenQ < (sk.scope 0).kids.length := by
            have : ¬ (s.ropenQ = (sk.scope 0).kids.length) := by
              intro heq
              rw [heq] at hnd
              simp at hnd
            have hle : s.ropenQ ≤ (sk.scope 0).kids.length := hqle
            omega
          have happ : (apply sk ax (.ropenChoose .query) s).isSome = true := by
            simp [apply, hch, ropenChoosable, hgw, hr, hw, hqlt]
          exact canStep_of_action ropenChoose_mem happ

end StreamingMirror.Model
