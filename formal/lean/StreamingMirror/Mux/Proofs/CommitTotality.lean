/-
T1, `commit_totality` (MUX-ADJUDICATION.md §3): under the shipping
interface `.impl`, an uncommitted phase-2 walk at any reachable state
has exactly ONE choosable obligation — the ledgers W/D1/D4/D6 plus the
per-channel child order totally order each scope's publications.

Existence is the landed Progress pillar (`walk_uncommitted_choosable`);
this file adds the uniqueness half and packages both as unique
existence (spelled out — the artifact is Mathlib-free, so no `∃!`). The
statement discharges the commit-control boundary of the mux campaign
(attack-prove F5, adopted by MUX-ADJUDICATION §2.2): the harness keeps
`walkCommit` adversarial and lets σ gate pushes only, and this theorem
is why that costs nothing under `.impl` — the adversary's commit
"choices" are forced, so the probe's fused commit+fire is WLOG and
T3's forced run needs no commit case analysis.

The uniqueness argument, by unordered obligation pair (each bullet a
`.impl` guard doing the work):

- wire/wire — the later wire's prefix conjunct demands the earlier one
  done; the earlier demands itself undone;
- wire/res — below the wire, D4 demands the child discharged; at or
  above it, W's fired-wire demand plus wire-prefix closure contradict
  the wire's own undoneness;
- wire/query — below the wire, D4 demands the quota met; at or above
  it, D1's resolved-child demand plus the W shadow (resolved ⇒ wire
  done, the invariant's fired-fact lemma) contradict undoneness;
- wire/parent, res/parent, query/parent — D6's epilogue conjunct
  demands every wire done and every D child fully discharged;
- res/res — the later resolution's contiguity prefix (D-children
  resolved below it) contradicts the earlier's unresolvedness;
- res/query — D3 (sibling contiguity) demands every resolved child's
  quota met, while the query demands its own child resolved (D1) with
  quota unmet;
- query/query — the later query's prefix conjunct demands the earlier
  child's quota met.

Two reachability facts feed the bash, both read off `wkLocalOk`
(`inv_reachable`): wire-done prefix closure (fired wires are an
initial segment of the child list) and the W fired-fact shadow
(a resolved child's wire is done).
-/
import StreamingMirror.Proofs.Preserve
import StreamingMirror.Proofs.Progress

namespace StreamingMirror.Model

variable {sk : Skel} {s : State} {pk : Party × Nat}

-- ================================================= invariant extraction

/-- Fired wires form an initial segment of the child list: the walk's
wire ledger conjunct (`wireDone j` steps down to `wireDone (j-1)`),
closed downward. -/
theorem wireDone_prefix (hi : InvL sk .impl s) (hpk : pk ∈ sk.walkKeys)
    (hph : (s.walk pk).phase = 2) (hco : (s.walk pk).committed = none) :
    ∀ j, j < sk.fan → (s.walk pk).wireDone j = true →
      ∀ i, i < j → (s.walk pk).wireDone i = true := by
  have hwk := hi.wk pk hpk
  simp only [wkLocalOk] at hwk
  rw [hph, hco] at hwk
  simp at hwk
  obtain ⟨-, -, hall⟩ := hwk
  have hstep : ∀ j, j < sk.fan → (s.walk pk).wireDone j = true →
      j = 0 ∨ (s.walk pk).wireDone (j - 1) = true := by
    intro j hj hwd
    obtain ⟨⟨⟨⟨⟨⟨⟨⟨⟨c1, -⟩, -⟩, -⟩, -⟩, -⟩, -⟩, -⟩, -⟩, -⟩ := hall j hj
    rcases c1 with hf | ⟨-, hc⟩
    · rw [hwd] at hf; cases hf
    · exact hc
  intro j
  induction j with
  | zero => intro _ _ i hi; omega
  | succ j ih =>
      intro hj hwd i hij
      rcases hstep (j + 1) hj hwd with h0 | hprev
      · omega
      · have hprev' : (s.walk pk).wireDone j = true := by
          simpa using hprev
        rcases Nat.lt_or_ge i j with hlt | hge
        · exact ih (by omega) hprev' i hlt
        · have : i = j := by omega
          exact this ▸ hprev'

/-- The W ledger's fired-fact shadow at walks: a resolved child's wire
is done (the invariant conjunct the first Phase B CTI demanded). -/
theorem resDone_wireDone (hi : InvL sk .impl s) (hpk : pk ∈ sk.walkKeys)
    (hph : (s.walk pk).phase = 2) (hco : (s.walk pk).committed = none) :
    ∀ j, j < sk.fan → (s.walk pk).resDone j = true →
      (s.walk pk).wireDone j = true := by
  have hwk := hi.wk pk hpk
  simp only [wkLocalOk] at hwk
  rw [hph, hco] at hwk
  simp at hwk
  obtain ⟨-, -, hall⟩ := hwk
  intro j hj hr
  obtain ⟨⟨⟨⟨⟨-, c6⟩, -⟩, -⟩, -⟩, -⟩ := hall j hj
  rcases c6 with (hax | hf) | hwd
  · exact absurd hax (by decide)
  · rw [hr] at hf; cases hf
  · exact hwd

-- ============================================== guard elimination rules

section Elim

variable {ws : WalkSt}

/-- Decompose a passing `.impl` wire guard into its conjuncts. -/
theorem wkChoosable_wire_elim {i : Nat}
    (hph : ws.phase = 2) (hco : ws.committed = none)
    (h : wkChoosable sk .impl pk ws (.wire i) = true) :
    i < sk.nChildren pk.2 (sk.stageScope pk.2 ws.scope) ∧
    ws.wireDone i = false ∧
    (∀ j, j < i → ws.wireDone j = true) ∧
    (∀ j, j < i →
      sk.childIsD pk.2 (sk.stageScope pk.2 ws.scope) j = true →
      ws.resDone j = true ∧
        ws.qSent j = sk.qCount pk.2 (sk.stageScope pk.2 ws.scope) j) := by
  simp only [wkChoosable, hph, hco, bne_self_eq_false, Option.isSome_none,
    Bool.or_self, Bool.false_eq_true, if_false, AxMode.impl, Bool.not_true,
    Bool.not_false, Bool.false_or, Bool.true_or, Bool.and_eq_true,
    decide_eq_true_eq, Bool.not_eq_true', List.all_eq_true, List.mem_range,
    Bool.or_eq_true, beq_iff_eq, and_true] at h
  obtain ⟨⟨⟨hin, hund⟩, hpre⟩, hd4⟩ := h
  refine ⟨hin, hund, hpre, fun j hj hD => ?_⟩
  rcases hd4 j hj with hf | hdis
  · rw [hD] at hf; cases hf
  · exact hdis

/-- Decompose a passing `.impl` resolution guard into its conjuncts. -/
theorem wkChoosable_res_elim {i : Nat}
    (hph : ws.phase = 2) (hco : ws.committed = none)
    (h : wkChoosable sk .impl pk ws (.res i) = true) :
    i < sk.nChildren pk.2 (sk.stageScope pk.2 ws.scope) ∧
    sk.childIsD pk.2 (sk.stageScope pk.2 ws.scope) i = true ∧
    ws.resDone i = false ∧
    ws.wireDone i = true ∧
    (∀ j, j < i →
      sk.childIsD pk.2 (sk.stageScope pk.2 ws.scope) j = true →
      ws.resDone j = true) ∧
    (∀ j, j < sk.nChildren pk.2 (sk.stageScope pk.2 ws.scope) →
      ws.resDone j = true →
      ws.qSent j = sk.qCount pk.2 (sk.stageScope pk.2 ws.scope) j) := by
  simp only [wkChoosable, hph, hco, bne_self_eq_false, Option.isSome_none,
    Bool.or_self, Bool.false_eq_true, if_false, AxMode.impl, Bool.not_true,
    Bool.false_or, Bool.and_eq_true,
    decide_eq_true_eq, Bool.not_eq_true', List.all_eq_true, List.mem_range,
    Bool.or_eq_true, beq_iff_eq] at h
  obtain ⟨⟨⟨⟨⟨hin, hD⟩, hnr⟩, hDpre⟩, hw⟩, hd3⟩ := h
  refine ⟨hin, hD, hnr, hw, fun j hj hDj => ?_, fun j hj hr => ?_⟩
  · rcases hDpre j hj with hf | hres
    · rw [hDj] at hf; cases hf
    · exact hres
  · rcases hd3 j hj with hf | hq
    · rw [hr] at hf; cases hf
    · exact hq

/-- Decompose a passing `.impl` query guard into its conjuncts. -/
theorem wkChoosable_query_elim {i : Nat}
    (hph : ws.phase = 2) (hco : ws.committed = none)
    (h : wkChoosable sk .impl pk ws (.query i) = true) :
    i < sk.nChildren pk.2 (sk.stageScope pk.2 ws.scope) ∧
    sk.childIsD pk.2 (sk.stageScope pk.2 ws.scope) i = true ∧
    ws.qSent i < sk.qCount pk.2 (sk.stageScope pk.2 ws.scope) i ∧
    ws.resDone i = true ∧
    (∀ j, j < i →
      ws.qSent j = sk.qCount pk.2 (sk.stageScope pk.2 ws.scope) j) := by
  simp only [wkChoosable, hph, hco, bne_self_eq_false, Option.isSome_none,
    Bool.or_self, Bool.false_eq_true, if_false, AxMode.impl, Bool.not_true,
    Bool.not_false, Bool.false_or, Bool.true_or, Bool.and_eq_true,
    decide_eq_true_eq, List.all_eq_true, List.mem_range,
    beq_iff_eq, and_true] at h
  obtain ⟨⟨⟨⟨hin, hD⟩, hq⟩, hqpre⟩, hres⟩ := h
  exact ⟨hin, hD, hq, hres, hqpre⟩

/-- Decompose a passing `.impl` parent guard into its conjuncts: the
D6 epilogue demands the whole scope's other sends done. -/
theorem wkChoosable_parent_elim
    (hph : ws.phase = 2) (hco : ws.committed = none)
    (h : wkChoosable sk .impl pk ws .parent = true) :
    ws.parentDone = false ∧
    (∀ j, j < sk.nChildren pk.2 (sk.stageScope pk.2 ws.scope) →
      ws.wireDone j = true ∧
        (sk.childIsD pk.2 (sk.stageScope pk.2 ws.scope) j = true →
          ws.resDone j = true ∧
            ws.qSent j =
              sk.qCount pk.2 (sk.stageScope pk.2 ws.scope) j)) := by
  simp only [wkChoosable, hph, hco, bne_self_eq_false, Option.isSome_none,
    Bool.or_self, Bool.false_eq_true, if_false, AxMode.impl, Bool.not_true,
    Bool.false_or, Bool.and_eq_true,
    Bool.not_eq_true', List.all_eq_true, List.mem_range,
    Bool.or_eq_true, beq_iff_eq] at h
  obtain ⟨⟨hnp, -⟩, hd6⟩ := h
  refine ⟨hnp, fun j hj => ?_⟩
  obtain ⟨hwd, hrest⟩ := hd6 j hj
  refine ⟨hwd, fun hDj => ?_⟩
  rcases hrest with hf | hdis
  · rw [hDj] at hf; cases hf
  · exact hdis

end Elim

-- ======================================================= the uniqueness

/-- Uniqueness of the choosable obligation under `.impl`: the ledger
set W/D1/D3/D4/D6 plus child order admits at most one passing guard.

Stated against the LOCAL fragment (`InvL`) — the fired-fact closures
the cross-index cases need live in `wkLocalOk`, and nothing here reads
channel occupancy, so demanding `InvP` was a free over-ask (phase-4
sweep: the weakening matters at in-flight muxed states, where `InvP`
is false but `InvL` holds); `commit_totality` below instantiates it at
reachable states through `InvP.local`. -/
theorem commit_unique (hwf : sk.wellFormed = true)
    (hi : InvL sk .impl s) (hpk : pk ∈ sk.walkKeys)
    (hph : (s.walk pk).phase = 2) (hco : (s.walk pk).committed = none)
    {o o' : Oblig}
    (h : wkChoosable sk .impl pk (s.walk pk) o = true)
    (h' : wkChoosable sk .impl pk (s.walk pk) o' = true) :
    o = o' := by
  -- The scope is real (phase-2 cursor in range), so its child count is
  -- fan-bounded and every index in play sits inside the ledger window.
  have hwk := hi.wk pk hpk
  simp only [wkLocalOk] at hwk
  rw [hph, hco] at hwk
  simp at hwk
  obtain ⟨hscope, -, -⟩ := hwk
  have hn_fan : sk.nChildren pk.2 (sk.stageScope pk.2 (s.walk pk).scope)
      ≤ sk.fan := nChildren_le_fan hwf hscope
  have hwpre := wireDone_prefix hi hpk hph hco
  have hrw := resDone_wireDone hi hpk hph hco
  -- Cross-constructor contradictions, one per unordered pair.
  -- wire i ⊥ res j
  have no_wire_res : ∀ i j : Nat,
      wkChoosable sk .impl pk (s.walk pk) (.wire i) = true →
      wkChoosable sk .impl pk (s.walk pk) (.res j) = true → False := by
    intro i j hwi hrj
    obtain ⟨hin, hund, hpre, hd4⟩ := wkChoosable_wire_elim hph hco hwi
    obtain ⟨hjn, hD, hnr, hwd, -, -⟩ := wkChoosable_res_elim hph hco hrj
    rcases Nat.lt_trichotomy j i with hlt | heq | hgt
    · exact absurd (hd4 j hlt hD).1 (by rw [hnr]; exact Bool.false_ne_true)
    · rw [heq] at hwd; rw [hwd] at hund; cases hund
    · have := hwpre j (by omega) hwd i hgt
      rw [this] at hund; cases hund
  -- wire i ⊥ query j
  have no_wire_query : ∀ i j : Nat,
      wkChoosable sk .impl pk (s.walk pk) (.wire i) = true →
      wkChoosable sk .impl pk (s.walk pk) (.query j) = true → False := by
    intro i j hwi hqj
    obtain ⟨hin, hund, hpre, hd4⟩ := wkChoosable_wire_elim hph hco hwi
    obtain ⟨hjn, hD, hq, hres, -⟩ := wkChoosable_query_elim hph hco hqj
    rcases Nat.lt_trichotomy j i with hlt | heq | hgt
    · have := (hd4 j hlt hD).2; omega
    · have hwd := hrw j (by omega) hres
      rw [heq] at hwd; rw [hwd] at hund; cases hund
    · have hwd := hrw j (by omega) hres
      have := hwpre j (by omega) hwd i hgt
      rw [this] at hund; cases hund
  -- wire i ⊥ parent
  have no_wire_parent : ∀ i : Nat,
      wkChoosable sk .impl pk (s.walk pk) (.wire i) = true →
      wkChoosable sk .impl pk (s.walk pk) .parent = true → False := by
    intro i hwi hp
    obtain ⟨hin, hund, -, -⟩ := wkChoosable_wire_elim hph hco hwi
    obtain ⟨-, hd6⟩ := wkChoosable_parent_elim hph hco hp
    have := (hd6 i hin).1
    rw [this] at hund; cases hund
  -- res i ⊥ query j
  have no_res_query : ∀ i j : Nat,
      wkChoosable sk .impl pk (s.walk pk) (.res i) = true →
      wkChoosable sk .impl pk (s.walk pk) (.query j) = true → False := by
    intro i j hri hqj
    obtain ⟨-, -, -, -, -, hd3⟩ := wkChoosable_res_elim hph hco hri
    obtain ⟨hjn, -, hq, hres, -⟩ := wkChoosable_query_elim hph hco hqj
    have := hd3 j hjn hres; omega
  -- res i ⊥ parent
  have no_res_parent : ∀ i : Nat,
      wkChoosable sk .impl pk (s.walk pk) (.res i) = true →
      wkChoosable sk .impl pk (s.walk pk) .parent = true → False := by
    intro i hri hp
    obtain ⟨hin, hD, hnr, -, -, -⟩ := wkChoosable_res_elim hph hco hri
    obtain ⟨-, hd6⟩ := wkChoosable_parent_elim hph hco hp
    have := ((hd6 i hin).2 hD).1
    rw [this] at hnr; cases hnr
  -- query i ⊥ parent
  have no_query_parent : ∀ i : Nat,
      wkChoosable sk .impl pk (s.walk pk) (.query i) = true →
      wkChoosable sk .impl pk (s.walk pk) .parent = true → False := by
    intro i hqi hp
    obtain ⟨hin, hD, hq, -, -⟩ := wkChoosable_query_elim hph hco hqi
    obtain ⟨-, hd6⟩ := wkChoosable_parent_elim hph hco hp
    have := ((hd6 i hin).2 hD).2; omega
  -- Same-constructor index agreement.
  cases o with
  | wire i =>
      cases o' with
      | wire i' =>
          obtain ⟨-, hund, hpre, -⟩ := wkChoosable_wire_elim hph hco h
          obtain ⟨-, hund', hpre', -⟩ := wkChoosable_wire_elim hph hco h'
          rcases Nat.lt_trichotomy i i' with hlt | heq | hgt
          · rw [hpre' i hlt] at hund; cases hund
          · exact heq ▸ rfl
          · rw [hpre i' hgt] at hund'; cases hund'
      | res j => exact (no_wire_res i j h h').elim
      | query j => exact (no_wire_query i j h h').elim
      | parent => exact (no_wire_parent i h h').elim
  | res i =>
      cases o' with
      | wire j => exact (no_wire_res j i h' h).elim
      | res i' =>
          obtain ⟨-, hD, hnr, -, hDpre, -⟩ := wkChoosable_res_elim hph hco h
          obtain ⟨-, hD', hnr', -, hDpre', -⟩ :=
            wkChoosable_res_elim hph hco h'
          rcases Nat.lt_trichotomy i i' with hlt | heq | hgt
          · rw [hDpre' i hlt hD] at hnr; cases hnr
          · exact heq ▸ rfl
          · rw [hDpre i' hgt hD'] at hnr'; cases hnr'
      | query j => exact (no_res_query i j h h').elim
      | parent => exact (no_res_parent i h h').elim
  | query i =>
      cases o' with
      | wire j => exact (no_wire_query j i h' h).elim
      | res j => exact (no_res_query j i h' h).elim
      | query i' =>
          obtain ⟨-, -, hq, -, hqpre⟩ := wkChoosable_query_elim hph hco h
          obtain ⟨-, -, hq', -, hqpre'⟩ := wkChoosable_query_elim hph hco h'
          rcases Nat.lt_trichotomy i i' with hlt | heq | hgt
          · have := hqpre' i hlt; omega
          · exact heq ▸ rfl
          · have := hqpre i' hgt; omega
      | parent => exact (no_query_parent i h h').elim
  | parent =>
      cases o' with
      | wire j => exact (no_wire_parent j h' h).elim
      | res j => exact (no_res_parent j h' h).elim
      | query j => exact (no_query_parent j h' h).elim
      | parent => rfl

-- ======================================================== the theorem

/-- T1, `commit_totality`: under the shipping interface `.impl`, at any
reachable state, an uncommitted phase-2 walk has a UNIQUE choosable
obligation (MUX-ADJUDICATION §3, T1).

Existence is the Progress pillar (`walk_uncommitted_choosable`, the
`d5 = false` branch of `hmode`); uniqueness is `commit_unique`. The
consequence for the mux campaign: `walkCommit` may stay adversarial in
the harness while σ gates pushes only — the adversary has no commit
choice to abuse — and every strategy consultation on a forced run
happens after a forced commit (attack-refute §2's strengthening,
consumed by T3). -/
theorem commit_totality (hwf : sk.wellFormed = true) :
    ∀ s, Reachable sk .impl s → ∀ pk, pk ∈ sk.walkKeys →
      (s.walk pk).phase = 2 → (s.walk pk).committed = none →
      ∃ o : Oblig, wkChoosable sk .impl pk (s.walk pk) o = true ∧
        ∀ o' : Oblig,
          wkChoosable sk .impl pk (s.walk pk) o' = true → o' = o := by
  intro s hr pk hpk hph hco
  have hi : InvP sk .impl s := (inv_iff sk .impl s).mp (inv_reachable hwf hr)
  obtain ⟨o, hch, -⟩ :=
    walk_uncommitted_choosable hwf hi.local hpk hph hco (Or.inl rfl)
  exact ⟨o, hch, fun o' hch' =>
    commit_unique hwf hi.local hpk hph hco hch' hch⟩

end StreamingMirror.Model
