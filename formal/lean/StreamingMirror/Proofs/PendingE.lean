/-
The `.impl` pending layer: `Pending.lean`'s decode lemmas re-targeted
at the encoder-order traces (`procsE`/`scheduleE`) under `AxMode.impl`.

# The three-way split (PROGRESS.md §9 item 4)

Mode-free helpers stay in `Pending.lean` and are consumed as-is. The
placement-independent decodes (openers, finishes, absorber, assemblers
— identical traces in both orders) are textual twins: the mode enters
only as the `.impl` literal, and `simp [AxMode.impl]` normalizes the
shared ledger fields exactly as `.full` does. The walk decode is the
one genuinely restructured piece: under `.impl` the `.wire`/`.query`
committed arms lose their `d5` conjunct and the `.parent` arm gains
the `d6` everything-done conjunct, pinning the parent's pend position
at the scope tail of `walkEventsE` — the d5 decode's parent-mid-scope
case analysis is replaced by the simpler tail case, mirroring how the
whole E re-derivation refunds the splice machinery.

τ-comparison rides `scheduleE` through `merge_completeE`, so the
schedule-glue lemmas here take the margin-0 capacity hypothesis where
their d5 counterparts took `schedulable`.

Chain (.impl, stage D): mirrors Pending.lean (walk decode restructured
for d6 — the parent pends at the scope tail); provides the decodes to
EndgameE.lean. Map: Proofs/Map.lean.
-/
import StreamingMirror.Proofs.Pending
import StreamingMirror.Proofs.Sched.Weave.FinalE

namespace StreamingMirror.Sched

open Model

variable (sk : Skel)

/-- Membership shape of a scope's encoder-order sends: the parent
summary or a child-chunk event (same disjuncts as `mem_scopeSends`;
only the order in the list differs). -/
theorem mem_scopeSendsE {pk : Party × Nat} {k : Nat} {e : Ev}
    (he : e ∈ scopeSendsE sk pk k) :
    e = (upperOut pk, true, k)
      ∨ ∃ i, i < sk.nChildren pk.2 (sk.stageScope pk.2 k)
          ∧ e ∈ childChunk sk pk k i := by
  simp only [scopeSendsE] at he
  rcases List.mem_append.1 he with hm | hone
  · obtain ⟨l, hl, hel⟩ := List.mem_flatten.1 hm
    obtain ⟨i, hir, rfl⟩ := List.mem_map.1 hl
    exact Or.inr ⟨i, List.mem_range.1 hir, hel⟩
  · exact Or.inl (List.mem_singleton.1 hone)

-- ================================================= scheduleE-side glue

/-- Merge completeness, read back through trace monotonicity: every
trace embeds in the scheduleE in order. This is what makes
position-in-scheduleE a total order along each trace. -/
theorem trace_sublistE (hwf : sk.wellFormed = true)
    (hm0 : ∀ sc, sk.dCount sc ≤ sk.capLevel) {T : List Ev}
    (hT : T ∈ procsE sk) : T.Sublist (scheduleE sk) := by
  obtain ⟨r, hr, pre, hpre, hsub⟩ :=
    (trace_monotoneE sk).exists_of_mem_left hT
  have hempty : r = [] := by
    have := List.all_eq_true.1 (merge_completeE sk hwf hm0) r hr
    cases r with
    | nil => rfl
    | cons a l => simp at this
  rw [hempty, List.append_nil] at hpre
  exact hpre ▸ hsub

/-- τ injectivity in counting form: the scheduleE holds each event at
most once (its per-channel projections are canonical). -/
theorem scheduleE_count_le_oneE (hwf : sk.wellFormed = true) (e : Ev) :
    (scheduleE sk).count e ≤ 1 := by
  obtain ⟨c, b, n⟩ := e
  obtain ⟨m, hm⟩ := scheduleE_proj_canon sk hwf c b
  have hfilter : (scheduleE sk).count (c, b, n)
      = (proj c b (scheduleE sk)).count (c, b, n) := by
    unfold proj
    exact (List.count_filter (by simp)).symm
  rw [hfilter, hm, count_canon]
  split <;> omega

/-- Provenance: every scheduleE event was emitted by some trace. -/
theorem sched_mem_traceE {e : Ev} (he : e ∈ scheduleE sk) :
    ∃ T ∈ procsE sk, e ∈ T := by
  have hpos : 1 ≤ emittedCount (fun x => x == e) (procsE sk)
      (finalStateE sk).rem := by
    rw [← scheduleE_count sk (fun x => x == e)]
    have hm : e ∈ (scheduleE sk).filter (fun x => x == e) :=
      List.mem_filter.2 ⟨he, by simp⟩
    have := List.length_pos_of_mem hm
    omega
  obtain ⟨T, hT, e', he', hbeq⟩ := emittedCount_pos hpos
  have : e' = e := by simpa using hbeq
  exact ⟨T, hT, this ▸ he'⟩

-- ============================================ performedness and PendOkE

/-- The pending event's global obligations: its channel is a real flow
channel, its seq is the channel's CURRENT count (so it is the first
unperformed event of its channel-side), its action is enumerated, and
the action is enabled as soon as the channel guard opens (room for a
send, data for a receive). -/
structure PendOkE (s : State) (f : Ev) (a : Action) : Prop where
  chan_mem : f.1 ∈ allChans sk
  seq : f.2.2 = (if f.2.1 = true then sentOf sk s f.1
    else recvdOf sk s f.1)
  act : a ∈ allActions sk
  fire : (if f.2.1 = true then s.chan f.1 < sk.cap f.1
      else 0 < s.chan f.1)
    → (apply sk .impl a s).isSome = true

/-- A pending event is never performed: its seq IS the count. -/
theorem pend_not_performedE {s : State} {f : Ev} {a : Action}
    (h : PendOkE sk s f a) : ¬ performed sk s f := by
  have hseq := h.seq
  unfold performed
  cases hb : f.2.1 with
  | true =>
      rw [hb] at hseq
      rw [if_pos rfl] at hseq ⊢
      omega
  | false =>
      rw [hb] at hseq
      rw [if_neg (by simp)] at hseq ⊢
      omega

/-- The τ-comparison: an unperformed event of a trace sits at or after
the trace's pending split, so the pending head is at or before it in
the scheduleE. -/
theorem tau_le_of_pendE (hwf : sk.wellFormed = true)
    (hm0 : ∀ sc, sk.dCount sc ≤ sk.capLevel) {s : State}
    {T pre suf : List Ev} {f : Ev}
    (hT : T ∈ procsE sk) (hdec : T = pre ++ f :: suf)
    (hpre : ∀ e ∈ pre, performed sk s e)
    {g : Ev} (hg : g ∈ T) (hnp : ¬ performed sk s g) :
    evIdx f (scheduleE sk) ≤ evIdx g (scheduleE sk) := by
  rw [hdec] at hg
  rcases List.mem_append.1 hg with hgpre | hgcons
  · exact absurd (hpre g hgpre) hnp
  · rcases List.mem_cons.1 hgcons with rfl | hgsuf
    · exact Nat.le_refl _
    · have hpair : ([f, g] : List Ev).Sublist T := by
        rw [hdec]
        refine List.Sublist.trans ?_ (List.sublist_append_right pre _)
        exact List.cons_sublist_cons.2 (List.singleton_sublist.2 hgsuf)
      have hsub : ([f, g] : List Ev).Sublist (scheduleE sk) :=
        hpair.trans (trace_sublistE sk hwf hm0 hT)
      exact Nat.le_of_lt
        (pos_lt_of_pair (scheduleE_count_le_oneE sk hwf) hsub)
/-- Every event of a completed-scope block is performed: the state's
derived counts dominate the scope-prefix sums, and a completed scope's
events all sit below its own prefix boundary. Serves every walk phase
(for phases `≤ 2` the current scope is past `j`; past phase 2 every
scope is). -/
theorem scopeBlock_performedE (hwf : sk.wellFormed = true) {s : State}
    {pk : Party × Nat} (hi : InvP sk .impl s) (hpk : pk ∈ sk.walkKeys)
    {j : Nat} (hj : j < (s.walk pk).scope) (hjs : j < sk.stageLen pk.2) :
    ∀ e ∈ scopeBlockE sk pk j, performed sk s e := by
  -- the six count dominations
  have hWR : (s.walk pk).scope ≤ wkWireRecvd sk s pk := by
    unfold wkWireRecvd
    by_cases hph : (s.walk pk).phase ≥ 3
    · rw [if_pos (by omega)]
      have hwk := hi.wk pk hpk
      simp only [wkLocalOk] at hwk
      rcases Bool.and_eq_true .. ▸ hwk with ⟨hcur, -⟩
      rw [if_neg (by omega)] at hcur
      simp only [Bool.and_eq_true] at hcur
      obtain ⟨⟨hsl, -⟩, -⟩ := hcur
      have : (s.walk pk).scope = sk.stageLen pk.2 := by simpa using hsl
      omega
    · rw [if_neg (by omega)]
      omega
  have hAR : (s.walk pk).scope ≤ wkAskedRecvd sk s pk := by
    unfold wkAskedRecvd
    by_cases hph : (s.walk pk).phase ≥ 3
    · rw [if_pos (by omega)]
      have hwk := hi.wk pk hpk
      simp only [wkLocalOk] at hwk
      rcases Bool.and_eq_true .. ▸ hwk with ⟨hcur, -⟩
      rw [if_neg (by omega)] at hcur
      simp only [Bool.and_eq_true] at hcur
      obtain ⟨⟨hsl, -⟩, -⟩ := hcur
      have : (s.walk pk).scope = sk.stageLen pk.2 := by simpa using hsl
      omega
    · rw [if_neg (by omega)]
      omega
  have hWS : sk.wiresBefore pk.2 (s.walk pk).scope ≤ wkWireSent sk s pk :=
    Nat.le_add_right ..
  have hRS : sk.dsBefore pk.2 (s.walk pk).scope ≤ wkResSent sk s pk :=
    Nat.le_add_right ..
  have hQS : sk.qsBefore pk.2 (s.walk pk).scope ≤ wkQSentTot sk s pk :=
    Nat.le_add_right ..
  have hPS : (s.walk pk).scope ≤ wkParentSent s pk :=
    Nat.le_add_right ..
  -- the events, one shape at a time
  intro e he
  unfold scopeBlockE at he
  rcases List.mem_cons.1 he with rfl | he
  · -- prologue wire receive
    show (j : Nat) < recvdOf sk s (wireIn pk)
    rw [recvdOf_wireIn hpk]
    omega
  rcases List.mem_cons.1 he with rfl | he
  · -- prologue asked receive
    show (j : Nat) < recvdOf sk s (askedIn pk)
    rw [recvdOf_askedIn]
    omega
  rcases mem_scopeSendsE sk he with rfl | ⟨i, hin, hchunk⟩
  · -- parent summary
    show (j : Nat) < sentOf sk s (upperOut pk)
    rw [sentOf_upperOut]
    omega
  · -- child chunk events
    have hwlt : sk.wiresBefore pk.2 j + i
        < sk.wiresBefore pk.2 (s.walk pk).scope := by
      have hsucc := wiresBefore_succ sk hjs
      have hmono := wiresBefore_mono sk pk.2
        (show j + 1 ≤ (s.walk pk).scope by omega)
      omega
    cases hD : sk.childIsD pk.2 (sk.stageScope pk.2 j) i with
    | true =>
        rw [chunkD sk pk j i hD] at hchunk
        rcases List.mem_cons.1 hchunk with rfl | hchunk
        · show sk.wiresBefore pk.2 j + i < sentOf sk s (wireOut pk)
          rw [sentOf_wireOut hpk]
          unfold wkWireSent at *
          omega
        rcases List.mem_cons.1 hchunk with rfl | hchunk
        · show sk.dsBefore pk.2 j + dRank sk pk j i
              < sentOf sk s (lowerOut pk)
          rw [sentOf_lowerOut]
          have hdlt := dRank_succ_le_dOf sk pk (k := j) hin hD
          have hsucc := dsBefore_succ sk hjs
          have hmono := dsBefore_mono sk pk.2
            (show j + 1 ≤ (s.walk pk).scope by omega)
          unfold wkResSent at *
          omega
        · obtain ⟨cc, bb, nn⟩ := e
          obtain ⟨hc, hb, hlo, hhi⟩ := mem_seg hchunk
          subst hc hb
          have h1 : 1 ≤ pk.2 := by
            cases hp2 : pk.2 with
            | zero =>
                rw [hp2] at hD
                simp [Skel.childIsD] at hD
            | succ m => omega
          show nn < sentOf sk s (askedOut pk)
          rw [sentOf_askedOut hwf hpk h1]
          have hqlt : sk.qsBefore pk.2 j + qSum sk pk j i
                + sk.qCount pk.2 (sk.stageScope pk.2 j) i
              ≤ sk.qsBefore pk.2 (s.walk pk).scope := by
            have hq1 : qSum sk pk j i
                  + sk.qCount pk.2 (sk.stageScope pk.2 j) i
                = qSum sk pk j (i + 1) := (qSum_succ sk pk j i).symm
            have hq2 := qSum_le_qOf sk pk j
              (show i + 1 ≤ sk.nChildren pk.2 (sk.stageScope pk.2 j)
                by omega)
            have hsucc := qsBefore_succ sk hjs
            have hmono := qsBefore_mono sk pk.2
              (show j + 1 ≤ (s.walk pk).scope by omega)
            omega
          unfold wkQSentTot at *
          omega
    | false =>
        rw [chunkR sk pk j i hD] at hchunk
        rcases List.mem_cons.1 hchunk with rfl | hchunk
        · show sk.wiresBefore pk.2 j + i < sentOf sk s (wireOut pk)
          rw [sentOf_wireOut hpk]
          unfold wkWireSent at *
          omega
        · cases hchunk

/-- The per-child ledger facts of a phase-2 walk under `.impl`: the
fired-fact shadows `wkLocalOk` carries, named for the committed-case
splits. `hDdis` is the `d4` shadow (a fired wire discharges every
earlier D child), `hqres` the `d1int` shadow, `hrw` the `w` shadow. -/
theorem phase2_child_factsE {s : State} {pk : Party × Nat}
    (hi : InvP sk .impl s) (hpk : pk ∈ sk.walkKeys)
    (hph2 : (s.walk pk).phase = 2) :
    (s.walk pk).scope < sk.stageLen pk.2 ∧
    (∀ j, j < sk.fan → (s.walk pk).wireDone j = true →
      j < sk.nChildren pk.2 (sk.stageScope pk.2 (s.walk pk).scope)
        ∧ (j = 0 ∨ (s.walk pk).wireDone (j - 1) = true)) ∧
    (∀ j, j < sk.fan → (s.walk pk).qSent j
      ≤ sk.qCount pk.2 (sk.stageScope pk.2 (s.walk pk).scope) j) ∧
    (∀ j, j < sk.fan → (s.walk pk).resDone j = true →
      sk.childIsD pk.2 (sk.stageScope pk.2 (s.walk pk).scope) j = true) ∧
    (∀ j, j < sk.fan → (s.walk pk).resDone j = true →
      ∀ j2, j2 < j →
        sk.childIsD pk.2 (sk.stageScope pk.2 (s.walk pk).scope) j2 = true →
        (s.walk pk).resDone j2 = true) ∧
    (∀ j, j < sk.fan → (s.walk pk).resDone j = true →
      (s.walk pk).wireDone j = true) ∧
    (∀ j, j < sk.fan → 0 < (s.walk pk).qSent j →
      (s.walk pk).resDone j = true) ∧
    (∀ j, j < sk.fan → 0 < (s.walk pk).qSent j →
      ∀ j2, j2 < j → (s.walk pk).qSent j2
        = sk.qCount pk.2 (sk.stageScope pk.2 (s.walk pk).scope) j2) ∧
    (∀ j, j < sk.fan → (s.walk pk).wireDone j = true →
      ∀ j2, j2 < j →
        sk.childIsD pk.2 (sk.stageScope pk.2 (s.walk pk).scope) j2 = true →
        (s.walk pk).resDone j2 = true ∧ (s.walk pk).qSent j2
          = sk.qCount pk.2 (sk.stageScope pk.2 (s.walk pk).scope) j2) := by
  have hwk := hi.wk pk hpk
  simp only [wkLocalOk] at hwk
  rw [hph2] at hwk
  simp [AxMode.impl] at hwk
  obtain ⟨hscope, ⟨-, hall⟩, -⟩ := hwk
  refine ⟨hscope, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_⟩
  · intro j hj hw
    obtain ⟨⟨⟨⟨⟨⟨⟨⟨c1, -⟩, -⟩, -⟩, -⟩, -⟩, -⟩, -⟩, -⟩ := hall j hj
    rcases c1 with hf | h
    · rw [hw] at hf; cases hf
    · exact h
  · intro j hj
    obtain ⟨⟨⟨⟨⟨⟨⟨⟨-, -⟩, c3⟩, -⟩, -⟩, -⟩, -⟩, -⟩, -⟩ := hall j hj
    exact c3
  · intro j hj hr
    obtain ⟨⟨⟨⟨⟨⟨⟨⟨-, c2⟩, -⟩, -⟩, -⟩, -⟩, -⟩, -⟩, -⟩ := hall j hj
    rcases c2 with hf | ⟨-, hD⟩
    · rw [hr] at hf; cases hf
    · exact hD
  · intro j hj hr j2 hj2 hD2
    obtain ⟨⟨⟨⟨⟨⟨⟨⟨-, -⟩, -⟩, -⟩, c5⟩, -⟩, -⟩, -⟩, -⟩ := hall j hj
    rcases c5 with hf | hpre
    · rw [hr] at hf; cases hf
    · rcases hpre j2 hj2 with hDf | hres
      · rw [hD2] at hDf; cases hDf
      · exact hres
  · intro j hj hr
    obtain ⟨⟨⟨⟨⟨⟨⟨⟨-, -⟩, -⟩, -⟩, -⟩, c6⟩, -⟩, -⟩, -⟩ := hall j hj
    rcases c6 with hf | hw
    · rw [hr] at hf; cases hf
    · exact hw
  · intro j hj hq
    obtain ⟨⟨⟨⟨⟨⟨⟨⟨-, -⟩, -⟩, -⟩, -⟩, -⟩, c7⟩, -⟩, -⟩ := hall j hj
    rcases c7 with hz | hres
    · omega
    · exact hres
  · intro j hj hq j2 hj2
    obtain ⟨⟨⟨⟨⟨⟨⟨⟨-, -⟩, -⟩, c4⟩, -⟩, -⟩, -⟩, -⟩, -⟩ := hall j hj
    rcases c4 with hz | hpre
    · omega
    · exact hpre j2 hj2
  · intro j hj hw j2 hj2 hD2
    obtain ⟨⟨⟨⟨⟨⟨⟨⟨-, -⟩, -⟩, -⟩, -⟩, -⟩, -⟩, -⟩, c10⟩ := hall j hj
    rcases c10 with hf | hpre
    · rw [hw] at hf; cases hf
    · rcases hpre j2 hj2 with hDf | hdis
      · rw [hD2] at hDf; cases hDf
      · exact hdis

-- =============================== per-phase walk-state extraction

/-- The scope cursor against the stage length, per phase band. -/
theorem walk_scope_boundE {s : State} {pk : Party × Nat}
    (hi : InvP sk .impl s) (hpk : pk ∈ sk.walkKeys) :
    ((s.walk pk).phase ≤ 2 → (s.walk pk).scope < sk.stageLen pk.2) ∧
    (3 ≤ (s.walk pk).phase → (s.walk pk).scope = sk.stageLen pk.2) := by
  have hwk := hi.wk pk hpk
  simp only [wkLocalOk] at hwk
  rcases Bool.and_eq_true .. ▸ hwk with ⟨hcur, -⟩
  simp only [Bool.and_eq_true] at hcur
  obtain ⟨⟨hsl, -⟩, -⟩ := hcur
  constructor
  · intro hph
    rw [if_pos hph] at hsl
    simpa using hsl
  · intro hph
    rw [if_neg (by omega)] at hsl
    simpa using hsl

/-- Outside phase 2 the publishing ledgers are all clear. -/
theorem walk_ledgers_emptyE {s : State} {pk : Party × Nat}
    (hi : InvP sk .impl s) (hpk : pk ∈ sk.walkKeys)
    (hph : (s.walk pk).phase ≠ 2) :
    (∀ j, j < sk.fan → (s.walk pk).wireDone j = false
      ∧ (s.walk pk).resDone j = false ∧ (s.walk pk).qSent j = 0)
    ∧ (s.walk pk).parentDone = false
    ∧ (s.walk pk).committed = none := by
  have hwk := hi.wk pk hpk
  simp only [wkLocalOk] at hwk
  rcases Bool.and_eq_true .. ▸ hwk with ⟨hleft, -⟩
  rcases Bool.and_eq_true .. ▸ hleft with ⟨-, hor⟩
  simp only [Bool.or_eq_true, beq_iff_eq] at hor
  rcases hor with hph2 | hemp
  · exact absurd hph2 hph
  · simp only [Bool.and_eq_true, List.all_eq_true, List.mem_range,
      Bool.not_eq_true', beq_iff_eq] at hemp
    obtain ⟨⟨hled, hpd⟩, hcm⟩ := hemp
    refine ⟨fun j hj => ?_, hpd, hcm⟩
    obtain ⟨⟨hw, hr⟩, hq⟩ := hled j hj
    exact ⟨hw, hr, by simpa using hq⟩

/-- Clear ledgers count to zero. -/
theorem counts_of_emptyE {s : State} {pk : Party × Nat}
    (hled : ∀ j, j < sk.fan → (s.walk pk).wireDone j = false
      ∧ (s.walk pk).resDone j = false ∧ (s.walk pk).qSent j = 0) :
    wkWireCount sk s pk = 0 ∧ wkResCount sk s pk = 0
      ∧ wkQSum sk s pk = 0 := by
  refine ⟨?_, ?_, ?_⟩
  · simp only [wkWireCount]
    rw [List.filter_eq_nil_iff.mpr fun j hj =>
      by simp [(hled j (List.mem_range.1 hj)).1]]
    rfl
  · simp only [wkResCount]
    rw [List.filter_eq_nil_iff.mpr fun j hj =>
      by simp [(hled j (List.mem_range.1 hj)).2.1]]
    rfl
  · rw [wkQSum_eq_sum,
      show (List.range sk.fan).map (s.walk pk).qSent
        = (List.range sk.fan).map (fun _ => 0) from
        List.map_congr_left fun j hj =>
          (hled j (List.mem_range.1 hj)).2.2]
    induction List.range sk.fan with
    | nil => rfl
    | cons x xs ih => simpa using ih

-- ================================== derived-key channel memberships

-- ======================== the in-scope prefix performedness (chunks)

/-- Everything in the first `i` child chunks of the CURRENT scope is
performed, given the committed-arm discharge facts: `i` counted wires,
every D child below `i` resolved and at quota. This is the shared core
of all four committed-case splits. -/
theorem chunks_prefix_performedE (hwf : sk.wellFormed = true) {s : State}
    {pk : Party × Nat} (hi : InvP sk .impl s) (hpk : pk ∈ sk.walkKeys)
    (hph2 : (s.walk pk).phase = 2) {i : Nat}
    (hin : i ≤ sk.nChildren pk.2 (sk.stageScope pk.2 (s.walk pk).scope))
    (hwc : i ≤ wkWireCount sk s pk)
    (hdis : ∀ j, j < i →
      sk.childIsD pk.2 (sk.stageScope pk.2 (s.walk pk).scope) j = true →
      (s.walk pk).resDone j = true ∧ (s.walk pk).qSent j
        = sk.qCount pk.2 (sk.stageScope pk.2 (s.walk pk).scope) j) :
    ∀ e ∈ (List.range i).flatMap (childChunk sk pk (s.walk pk).scope),
      performed sk s e := by
  obtain ⟨hscope, -, hqle, -, -, -, -, -, -⟩ :=
    phase2_child_factsE sk hi hpk hph2
  have hn_fan : sk.nChildren pk.2 (sk.stageScope pk.2 (s.walk pk).scope)
      ≤ sk.fan := nChildren_le_fan hwf hscope
  -- every child below `i` is at quota (W children are quota-0)
  have heq : ∀ j, j < i → (s.walk pk).qSent j
      = sk.qCount pk.2 (sk.stageScope pk.2 (s.walk pk).scope) j := by
    intro j hj
    cases hD : sk.childIsD pk.2 (sk.stageScope pk.2 (s.walk pk).scope) j with
    | true => exact (hdis j hj hD).2
    | false =>
        have hq0 : sk.qCount pk.2 (sk.stageScope pk.2 (s.walk pk).scope) j
            = 0 := by
          simp [Skel.qCount, hD]
        have := hqle j (by omega)
        omega
  -- the resolution count dominates the D prefix
  have hrc : dRank sk pk (s.walk pk).scope i ≤ wkResCount sk s pk := by
    have h1 : ((List.range sk.fan).filter fun j =>
        decide (j < i) && sk.childIsD pk.2
          (sk.stageScope pk.2 (s.walk pk).scope) j).length
        ≤ ((List.range sk.fan).filter fun j =>
            (s.walk pk).resDone j).length := by
      refine length_filter_le_of_imp fun x _ hx => ?_
      simp only [Bool.and_eq_true, decide_eq_true_eq] at hx
      exact (hdis x hx.1 hx.2).1
    rw [filter_range_and_lt (by omega)] at h1
    simp only [wkResCount]
    exact h1
  intro e he
  obtain ⟨j, hjr, hje⟩ := List.mem_flatMap.1 he
  rw [List.mem_range] at hjr
  cases hD : sk.childIsD pk.2 (sk.stageScope pk.2 (s.walk pk).scope) j with
  | true =>
      rw [chunkD sk pk (s.walk pk).scope j hD] at hje
      rcases List.mem_cons.1 hje with rfl | hje
      · show sk.wiresBefore pk.2 (s.walk pk).scope + j
            < sentOf sk s (wireOut pk)
        rw [sentOf_wireOut hpk]
        unfold wkWireSent
        omega
      rcases List.mem_cons.1 hje with rfl | hje
      · show sk.dsBefore pk.2 (s.walk pk).scope + dRank sk pk (s.walk pk).scope j
            < sentOf sk s (lowerOut pk)
        rw [sentOf_lowerOut]
        have hstep : dRank sk pk (s.walk pk).scope (j + 1)
            = dRank sk pk (s.walk pk).scope j + 1 := by
          rw [dRank_succ, if_pos hD]
        have hmono := dRank_mono sk pk (s.walk pk).scope
          (show j + 1 ≤ i by omega)
        unfold wkResSent
        omega
      · obtain ⟨cc, bb, nn⟩ := e
        obtain ⟨hc, hb, hlo, hhi⟩ := mem_seg hje
        subst hc hb
        have h1 : 1 ≤ pk.2 := by
          cases hp2 : pk.2 with
          | zero =>
              rw [hp2] at hD
              simp [Skel.childIsD] at hD
          | succ m => omega
        show nn < sentOf sk s (askedOut pk)
        rw [sentOf_askedOut hwf hpk h1]
        have hsum : qSum sk pk (s.walk pk).scope i ≤ wkQSum sk s pk := by
          have hcongr : (List.range i).map
              (fun i' => sk.qCount pk.2
                (sk.stageScope pk.2 (s.walk pk).scope) i')
              = (List.range i).map (s.walk pk).qSent := by
            refine List.map_congr_left fun x hx => ?_
            rw [List.mem_range] at hx
            exact (heq x hx).symm
          calc qSum sk pk (s.walk pk).scope i
              = ((List.range i).map (s.walk pk).qSent).sum := by
                unfold qSum
                rw [hcongr]
            _ ≤ wkQSum sk s pk := qsum_prefix_le sk s pk (by omega)
        have hq1 : qSum sk pk (s.walk pk).scope j
              + sk.qCount pk.2 (sk.stageScope pk.2 (s.walk pk).scope) j
            = qSum sk pk (s.walk pk).scope (j + 1) :=
          (qSum_succ sk pk (s.walk pk).scope j).symm
        have hq2 := qSum_mono sk pk (s.walk pk).scope
          (show j + 1 ≤ i by omega)
        unfold wkQSentTot
        omega
  | false =>
      rw [chunkR sk pk (s.walk pk).scope j hD] at hje
      rcases List.mem_cons.1 hje with rfl | hje
      · show sk.wiresBefore pk.2 (s.walk pk).scope + j
            < sentOf sk s (wireOut pk)
        rw [sentOf_wireOut hpk]
        unfold wkWireSent
        omega
      · cases hje

/-- A fired wire at `i` puts the wire count past `i` (prefix closure). -/
theorem wireCount_ge_succE {s : State} {pk : Party × Nat}
    (hi : InvP sk .impl s) (hpk : pk ∈ sk.walkKeys)
    (hph2 : (s.walk pk).phase = 2) {i : Nat} (hif : i < sk.fan)
    (hw : (s.walk pk).wireDone i = true) :
    i + 1 ≤ wkWireCount sk s pk := by
  obtain ⟨-, hc1, -, -, -, -, -, -, -⟩ := phase2_child_factsE sk hi hpk hph2
  have hclosed : ∀ j, j < sk.fan → (s.walk pk).wireDone j = true →
      j = 0 ∨ (s.walk pk).wireDone (j - 1) = true :=
    fun j hj hwj => (hc1 j hj hwj).2
  have hlow : ∀ j, j < i + 1 → (s.walk pk).wireDone j = true := by
    intro j hj
    exact fired_below hclosed hif hw j (by omega)
  simp only [wkWireCount]
  exact count_ge_of_prefix (by omega) hlow

set_option maxHeartbeats 1000000 in
/-- The `.impl` committed-case split: the in-scope prefix below the
committed obligation's event is performed, and the event carries the
channel's current count.

Where the `d5` split cased on the last disputed child to locate the
spliced parent, here the parent is the scope's tail send — it never
enters a committed prefix, and the `.parent` arm's `d6` mirror instead
pins the entire chunk run performed. -/
private theorem walk_committed_splitE (hwf : sk.wellFormed = true)
    {s : State} {pk : Party × Nat} (hi : InvP sk .impl s)
    (hpk : pk ∈ sk.walkKeys) (hph2 : (s.walk pk).phase = 2)
    {o : Oblig} (hcm : (s.walk pk).committed = some o) :
    ∃ f isp ss,
      wkPend sk s pk = [(f, .walkFire pk)]
      ∧ scopeSendsE sk pk (s.walk pk).scope = isp ++ f :: ss
      ∧ (∀ e ∈ isp, performed sk s e)
      ∧ f.1 = obligChan pk o ∧ f.2.1 = true
      ∧ f.2.2 = sentOf sk s f.1
      ∧ f.1 ∈ allChans sk := by
  obtain ⟨hscope, hwbc, hqle, hresD, hres5, hresw, hqres, hq4, hw10⟩ :=
    phase2_child_factsE sk hi hpk hph2
  have hn_fan : sk.nChildren pk.2 (sk.stageScope pk.2 (s.walk pk).scope)
      ≤ sk.fan := nChildren_le_fan hwf hscope
  have hwk := hi.wk pk hpk
  simp only [wkLocalOk] at hwk
  rw [hph2, hcm] at hwk
  cases o with
  | wire i =>
      simp [AxMode.impl] at hwk
      obtain ⟨-, -, ⟨hieq, hin⟩, hd4⟩ := hwk
      have hdis : ∀ j, j < i →
          sk.childIsD pk.2 (sk.stageScope pk.2 (s.walk pk).scope) j = true →
          (s.walk pk).resDone j = true ∧ (s.walk pk).qSent j
            = sk.qCount pk.2 (sk.stageScope pk.2 (s.walk pk).scope) j := by
        intro j hj hD
        rcases hd4 j hj with hf | h
        · rw [hD] at hf; cases hf
        · exact h
      have hperf := chunks_prefix_performedE sk hwf hi hpk hph2
        (show i ≤ _ by omega) (by omega) hdis
      have hpend : wkPend sk s pk = [((wireOut pk, true,
          sk.wiresBefore pk.2 (s.walk pk).scope + i), .walkFire pk)] := by
        simp [wkPend, hph2, hcm]
      have hseqf : sk.wiresBefore pk.2 (s.walk pk).scope + i
          = sentOf sk s (wireOut pk) := by
        rw [sentOf_wireOut hpk]
        unfold wkWireSent
        omega
      cases hD : sk.childIsD pk.2 (sk.stageScope pk.2 (s.walk pk).scope) i
          with
      | false =>
          refine ⟨(wireOut pk, true,
              sk.wiresBefore pk.2 (s.walk pk).scope + i),
            (List.range i).flatMap (childChunk sk pk (s.walk pk).scope),
            (List.range' (i + 1) (sk.nChildren pk.2
                (sk.stageScope pk.2 (s.walk pk).scope) - (i + 1))).flatMap
                (childChunk sk pk (s.walk pk).scope)
              ++ [(upperOut pk, true, (s.walk pk).scope)],
            hpend, ?_, hperf, rfl, rfl, hseqf, (walk_chans_mem sk hpk).1⟩
          simp only [scopeSendsE]
          rw [flatten_map,
            range_split (show i ≤ sk.nChildren pk.2
              (sk.stageScope pk.2 (s.walk pk).scope) by omega),
            List.flatMap_append,
            show sk.nChildren pk.2 (sk.stageScope pk.2 (s.walk pk).scope) - i
              = (sk.nChildren pk.2 (sk.stageScope pk.2 (s.walk pk).scope)
                - (i + 1)) + 1 from by omega,
            List.range'_succ, List.flatMap_cons,
            chunkR sk pk (s.walk pk).scope i hD]
          simp [List.cons_append, List.append_assoc]
      | true =>
          refine ⟨(wireOut pk, true,
              sk.wiresBefore pk.2 (s.walk pk).scope + i),
            (List.range i).flatMap (childChunk sk pk (s.walk pk).scope),
            (lowerOut pk, true, sk.dsBefore pk.2 (s.walk pk).scope
                + dRank sk pk (s.walk pk).scope i)
              :: (seg (askedOut pk) true
                  (sk.qsBefore pk.2 (s.walk pk).scope
                    + qSum sk pk (s.walk pk).scope i)
                  (sk.qCount pk.2 (sk.stageScope pk.2 (s.walk pk).scope) i)
                ++ ((List.range' (i + 1) (sk.nChildren pk.2
                    (sk.stageScope pk.2 (s.walk pk).scope) - (i + 1))).flatMap
                    (childChunk sk pk (s.walk pk).scope)
                  ++ [(upperOut pk, true, (s.walk pk).scope)])),
            hpend, ?_, hperf, rfl, rfl, hseqf, (walk_chans_mem sk hpk).1⟩
          simp only [scopeSendsE]
          rw [flatten_map,
            range_split (show i ≤ sk.nChildren pk.2
              (sk.stageScope pk.2 (s.walk pk).scope) by omega),
            List.flatMap_append,
            show sk.nChildren pk.2 (sk.stageScope pk.2 (s.walk pk).scope) - i
              = (sk.nChildren pk.2 (sk.stageScope pk.2 (s.walk pk).scope)
                - (i + 1)) + 1 from by omega,
            List.range'_succ, List.flatMap_cons,
            chunkD sk pk (s.walk pk).scope i hD]
          simp [List.cons_append, List.append_assoc]
  | res i =>
      simp [AxMode.impl] at hwk
      obtain ⟨-, -, ⟨⟨⟨⟨hin, hDi⟩, hnr⟩, hpre⟩, hwi⟩, hd3⟩ := hwk
      have h1 : 1 ≤ pk.2 := by
        cases hp2 : pk.2 with
        | zero => rw [hp2] at hDi; simp [Skel.childIsD] at hDi
        | succ m => omega
      have hpre' : ∀ j, j < i →
          sk.childIsD pk.2 (sk.stageScope pk.2 (s.walk pk).scope) j = true →
          (s.walk pk).resDone j = true := by
        intro j hj hD
        rcases hpre j hj with hf | h
        · rw [hD] at hf; cases hf
        · exact h
      have hd3' : ∀ j, j < sk.nChildren pk.2
          (sk.stageScope pk.2 (s.walk pk).scope) →
          (s.walk pk).resDone j = true → (s.walk pk).qSent j
            = sk.qCount pk.2 (sk.stageScope pk.2 (s.walk pk).scope) j := by
        intro j hj hr
        rcases hd3 j hj with hf | h
        · rw [hr] at hf; cases hf
        · exact h
      have hdis : ∀ j, j < i →
          sk.childIsD pk.2 (sk.stageScope pk.2 (s.walk pk).scope) j = true →
          (s.walk pk).resDone j = true ∧ (s.walk pk).qSent j
            = sk.qCount pk.2 (sk.stageScope pk.2 (s.walk pk).scope) j :=
        fun j hj hD => ⟨hpre' j hj hD,
          hd3' j (by omega) (hpre' j hj hD)⟩
      have hwc := wireCount_ge_succE sk hi hpk hph2
        (show i < sk.fan by omega) hwi
      have hperf := chunks_prefix_performedE sk hwf hi hpk hph2
        (show i ≤ _ by omega) (by omega) hdis
      have hpend : wkPend sk s pk = [((lowerOut pk, true,
          sk.dsBefore pk.2 (s.walk pk).scope
            + dRank sk pk (s.walk pk).scope i), .walkFire pk)] := by
        simp [wkPend, hph2, hcm]
      -- the resolution ledger is EXACTLY the D prefix below `i`
      have hseteq : ∀ j, j < sk.fan → (s.walk pk).resDone j
          = (decide (j < i) && sk.childIsD pk.2
              (sk.stageScope pk.2 (s.walk pk).scope) j) := by
        intro j hj
        by_cases hji : j < i
        · cases hD : sk.childIsD pk.2
              (sk.stageScope pk.2 (s.walk pk).scope) j with
          | true =>
              rw [hpre' j hji hD]
              simp [hji]
          | false =>
              cases hr : (s.walk pk).resDone j with
              | false => simp [hji]
              | true =>
                  have := hresD j hj hr
                  rw [hD] at this
                  cases this
        · cases hr : (s.walk pk).resDone j with
          | false => simp [hji]
          | true =>
              exfalso
              rcases Nat.lt_or_ge i j with hij2 | hij2
              · have := hres5 j hj hr i hij2 hDi
                rw [hnr] at this
                cases this
              · have hje : j = i := by omega
                subst hje
                rw [hnr] at hr
                cases hr
      have hcnt : wkResCount sk s pk = dRank sk pk (s.walk pk).scope i := by
        simp only [wkResCount]
        rw [List.filter_congr fun j hj => hseteq j (List.mem_range.1 hj),
          filter_range_and_lt (show i ≤ sk.fan by omega)]
        rfl
      have hseqf : sk.dsBefore pk.2 (s.walk pk).scope
          + dRank sk pk (s.walk pk).scope i = sentOf sk s (lowerOut pk) := by
        rw [sentOf_lowerOut]
        unfold wkResSent
        omega
      have hprefperf : ∀ e ∈ (List.range i).flatMap
            (childChunk sk pk (s.walk pk).scope)
          ++ [(wireOut pk, true,
              sk.wiresBefore pk.2 (s.walk pk).scope + i)],
          performed sk s e := by
        intro e he
        rcases List.mem_append.1 he with hfm | hone
        · exact hperf e hfm
        · rw [List.mem_singleton] at hone
          subst hone
          show sk.wiresBefore pk.2 (s.walk pk).scope + i
              < sentOf sk s (wireOut pk)
          rw [sentOf_wireOut hpk]
          unfold wkWireSent
          omega
      refine ⟨(lowerOut pk, true,
          sk.dsBefore pk.2 (s.walk pk).scope
            + dRank sk pk (s.walk pk).scope i),
        (List.range i).flatMap (childChunk sk pk (s.walk pk).scope)
          ++ [(wireOut pk, true,
              sk.wiresBefore pk.2 (s.walk pk).scope + i)],
        seg (askedOut pk) true
            (sk.qsBefore pk.2 (s.walk pk).scope
              + qSum sk pk (s.walk pk).scope i)
            (sk.qCount pk.2 (sk.stageScope pk.2 (s.walk pk).scope) i)
          ++ ((List.range' (i + 1) (sk.nChildren pk.2
              (sk.stageScope pk.2 (s.walk pk).scope) - (i + 1))).flatMap
              (childChunk sk pk (s.walk pk).scope)
            ++ [(upperOut pk, true, (s.walk pk).scope)]),
        hpend, ?_, hprefperf, rfl, rfl, hseqf,
        (walk_chans_mem sk hpk).2.2.2⟩
      simp only [scopeSendsE]
      rw [flatten_map,
        range_split (show i ≤ sk.nChildren pk.2
          (sk.stageScope pk.2 (s.walk pk).scope) by omega),
        List.flatMap_append,
        show sk.nChildren pk.2 (sk.stageScope pk.2 (s.walk pk).scope) - i
          = (sk.nChildren pk.2 (sk.stageScope pk.2 (s.walk pk).scope)
            - (i + 1)) + 1 from by omega,
        List.range'_succ, List.flatMap_cons,
        chunkD sk pk (s.walk pk).scope i hDi]
      simp [List.cons_append, List.append_assoc]
  | query i =>
      simp [AxMode.impl] at hwk
      obtain ⟨-, -, ⟨⟨⟨hin, hDi⟩, hqlt⟩, hqpre⟩, hres⟩ := hwk
      have h1 : 1 ≤ pk.2 := by
        cases hp2 : pk.2 with
        | zero => rw [hp2] at hDi; simp [Skel.childIsD] at hDi
        | succ m => omega
      have hwi : (s.walk pk).wireDone i = true :=
        hresw i (by omega) hres
      have hwc := wireCount_ge_succE sk hi hpk hph2
        (show i < sk.fan by omega) hwi
      have hdis : ∀ j, j < i →
          sk.childIsD pk.2 (sk.stageScope pk.2 (s.walk pk).scope) j = true →
          (s.walk pk).resDone j = true ∧ (s.walk pk).qSent j
            = sk.qCount pk.2 (sk.stageScope pk.2 (s.walk pk).scope) j :=
        fun j hj hD => ⟨hres5 i (by omega) hres j hj hD, hqpre j hj⟩
      have hperf := chunks_prefix_performedE sk hwf hi hpk hph2
        (show i ≤ _ by omega) (by omega) hdis
      have hpend : wkPend sk s pk = [((askedOut pk, true,
          sk.qsBefore pk.2 (s.walk pk).scope
            + qSum sk pk (s.walk pk).scope i + (s.walk pk).qSent i),
          .walkFire pk)] := by
        simp [wkPend, hph2, hcm]
      -- the query ledger cuts exactly at `i`
      have hqsum_exact : wkQSum sk s pk
          = qSum sk pk (s.walk pk).scope i + (s.walk pk).qSent i := by
        rw [wkQSum_eq_sum,
          range_split (show i + 1 ≤ sk.fan by omega),
          List.map_append, List.sum_append, List.range_succ,
          List.map_append, List.sum_append]
        have hz : ((List.range' (i + 1) (sk.fan - (i + 1))).map
            (s.walk pk).qSent).sum = 0 := by
          refine sum_map_zero fun j hj => ?_
          have hjb := List.mem_range'_1.1 hj
          by_contra hnz
          have hq := hq4 j (by omega) (by omega) i (by omega)
          omega
        have hleft : ((List.range i).map (s.walk pk).qSent).sum
            = qSum sk pk (s.walk pk).scope i := by
          unfold qSum
          congr 1
          refine List.map_congr_left fun j hj => ?_
          exact hqpre j (List.mem_range.1 hj)
        rw [hz, hleft]
        simp
      have hseqf : sk.qsBefore pk.2 (s.walk pk).scope
          + qSum sk pk (s.walk pk).scope i + (s.walk pk).qSent i
          = sentOf sk s (askedOut pk) := by
        rw [sentOf_askedOut hwf hpk h1]
        unfold wkQSentTot
        omega
      -- the mid-chunk seg split at the query cursor
      have hsegsplit : seg (askedOut pk) true
          (sk.qsBefore pk.2 (s.walk pk).scope
            + qSum sk pk (s.walk pk).scope i)
          (sk.qCount pk.2 (sk.stageScope pk.2 (s.walk pk).scope) i)
          = seg (askedOut pk) true
              (sk.qsBefore pk.2 (s.walk pk).scope
                + qSum sk pk (s.walk pk).scope i) ((s.walk pk).qSent i)
            ++ (askedOut pk, true,
                sk.qsBefore pk.2 (s.walk pk).scope
                  + qSum sk pk (s.walk pk).scope i + (s.walk pk).qSent i)
              :: seg (askedOut pk) true
                (sk.qsBefore pk.2 (s.walk pk).scope
                  + qSum sk pk (s.walk pk).scope i + (s.walk pk).qSent i + 1)
                (sk.qCount pk.2 (sk.stageScope pk.2 (s.walk pk).scope) i
                  - (s.walk pk).qSent i - 1) := by
        conv => lhs; rw [show sk.qCount pk.2
            (sk.stageScope pk.2 (s.walk pk).scope) i
            = (s.walk pk).qSent i
              + ((sk.qCount pk.2 (sk.stageScope pk.2 (s.walk pk).scope) i
                - (s.walk pk).qSent i - 1) + 1) from by omega]
        rw [← seg_append, seg_cons]
      have hsegperf : ∀ e ∈ seg (askedOut pk) true
          (sk.qsBefore pk.2 (s.walk pk).scope
            + qSum sk pk (s.walk pk).scope i) ((s.walk pk).qSent i),
          performed sk s e := by
        intro e he
        obtain ⟨cc, bb, nn⟩ := e
        obtain ⟨hc, hb, hlo, hhi⟩ := mem_seg he
        subst hc hb
        show nn < sentOf sk s (askedOut pk)
        rw [sentOf_askedOut hwf hpk h1]
        unfold wkQSentTot
        omega
      -- prefix through the fired wire and resolution of chunk `i`
      have hresperf : dRank sk pk (s.walk pk).scope i < wkResCount sk s pk := by
        have hd' : ∀ j, j < i + 1 →
            sk.childIsD pk.2 (sk.stageScope pk.2 (s.walk pk).scope) j = true →
            (s.walk pk).resDone j = true := by
          intro j hj hD
          rcases Nat.lt_or_ge j i with hlt | hge
          · exact (hdis j hlt hD).1
          · have : j = i := by omega
            subst this
            exact hres
        have := dRank_le_resCount sk (show i + 1 ≤ sk.fan by omega) hd'
        have hstep : dRank sk pk (s.walk pk).scope (i + 1)
            = dRank sk pk (s.walk pk).scope i + 1 := by
          rw [dRank_succ, if_pos hDi]
        omega
      have hprefperf : ∀ e ∈ (List.range i).flatMap
            (childChunk sk pk (s.walk pk).scope)
          ++ ((wireOut pk, true, sk.wiresBefore pk.2 (s.walk pk).scope + i)
            :: (lowerOut pk, true, sk.dsBefore pk.2 (s.walk pk).scope
                + dRank sk pk (s.walk pk).scope i)
            :: seg (askedOut pk) true
              (sk.qsBefore pk.2 (s.walk pk).scope
                + qSum sk pk (s.walk pk).scope i) ((s.walk pk).qSent i)),
          performed sk s e := by
        intro e he
        rcases List.mem_append.1 he with hfm | hcons
        · exact hperf e hfm
        rcases List.mem_cons.1 hcons with rfl | hcons
        · show sk.wiresBefore pk.2 (s.walk pk).scope + i
              < sentOf sk s (wireOut pk)
          rw [sentOf_wireOut hpk]
          unfold wkWireSent
          omega
        rcases List.mem_cons.1 hcons with rfl | hseg
        · show sk.dsBefore pk.2 (s.walk pk).scope
              + dRank sk pk (s.walk pk).scope i < sentOf sk s (lowerOut pk)
          rw [sentOf_lowerOut]
          unfold wkResSent
          omega
        · exact hsegperf e hseg
      refine ⟨(askedOut pk, true,
          sk.qsBefore pk.2 (s.walk pk).scope
            + qSum sk pk (s.walk pk).scope i + (s.walk pk).qSent i),
        (List.range i).flatMap (childChunk sk pk (s.walk pk).scope)
          ++ ((wireOut pk, true,
              sk.wiresBefore pk.2 (s.walk pk).scope + i)
            :: (lowerOut pk, true, sk.dsBefore pk.2 (s.walk pk).scope
                + dRank sk pk (s.walk pk).scope i)
            :: seg (askedOut pk) true
              (sk.qsBefore pk.2 (s.walk pk).scope
                + qSum sk pk (s.walk pk).scope i) ((s.walk pk).qSent i)),
        seg (askedOut pk) true
            (sk.qsBefore pk.2 (s.walk pk).scope
              + qSum sk pk (s.walk pk).scope i + (s.walk pk).qSent i + 1)
            (sk.qCount pk.2 (sk.stageScope pk.2 (s.walk pk).scope) i
              - (s.walk pk).qSent i - 1)
          ++ ((List.range' (i + 1) (sk.nChildren pk.2
              (sk.stageScope pk.2 (s.walk pk).scope) - (i + 1))).flatMap
              (childChunk sk pk (s.walk pk).scope)
            ++ [(upperOut pk, true, (s.walk pk).scope)]),
        hpend, ?_, hprefperf, rfl, rfl, hseqf,
        askedOut_mem_allChans sk hwf hpk h1⟩
      simp only [scopeSendsE]
      rw [flatten_map,
        range_split (show i ≤ sk.nChildren pk.2
          (sk.stageScope pk.2 (s.walk pk).scope) by omega),
        List.flatMap_append,
        show sk.nChildren pk.2 (sk.stageScope pk.2 (s.walk pk).scope) - i
          = (sk.nChildren pk.2 (sk.stageScope pk.2 (s.walk pk).scope)
            - (i + 1)) + 1 from by omega,
        List.range'_succ, List.flatMap_cons,
        chunkD sk pk (s.walk pk).scope i hDi, hsegsplit]
      simp [List.cons_append, List.append_assoc]
  | parent =>
      simp [AxMode.impl] at hwk
      obtain ⟨-, -, ⟨hnp, hd2⟩, hd6⟩ := hwk
      have hdis : ∀ j, j < sk.nChildren pk.2
          (sk.stageScope pk.2 (s.walk pk).scope) →
          sk.childIsD pk.2 (sk.stageScope pk.2 (s.walk pk).scope) j = true →
          (s.walk pk).resDone j = true ∧ (s.walk pk).qSent j
            = sk.qCount pk.2 (sk.stageScope pk.2 (s.walk pk).scope) j := by
        intro j hj hD
        rcases (hd6 j hj).2 with hf | h
        · rw [hD] at hf; cases hf
        · exact h
      have hwcn : sk.nChildren pk.2 (sk.stageScope pk.2 (s.walk pk).scope)
          ≤ wkWireCount sk s pk := by
        cases hn : sk.nChildren pk.2 (sk.stageScope pk.2 (s.walk pk).scope)
            with
        | zero => omega
        | succ m =>
            have hwm : (s.walk pk).wireDone m = true :=
              (hd6 m (by rw [hn]; omega)).1
            have := wireCount_ge_succE sk hi hpk hph2
              (show m < sk.fan by omega) hwm
            omega
      have hperf := chunks_prefix_performedE sk hwf hi hpk hph2
        (Nat.le_refl _) hwcn hdis
      have hpend : wkPend sk s pk
          = [((upperOut pk, true, (s.walk pk).scope), .walkFire pk)] := by
        simp [wkPend, hph2, hcm]
      have hseqf : (s.walk pk).scope = sentOf sk s (upperOut pk) := by
        rw [sentOf_upperOut]
        simp only [wkParentSent]
        rw [if_neg (by simp [hnp])]
        omega
      refine ⟨(upperOut pk, true, (s.walk pk).scope),
        (List.range (sk.nChildren pk.2
          (sk.stageScope pk.2 (s.walk pk).scope))).flatMap
          (childChunk sk pk (s.walk pk).scope),
        [],
        hpend, ?_, hperf, rfl, rfl, hseqf,
        (walk_chans_mem sk hpk).2.2.1⟩
      simp only [scopeSendsE]
      rw [flatten_map]
/-- The walk decode: past its channel work with everything performed,
or holding one pending event with the trace prefix below it performed.
Choice points (phase-2 uncommitted) are excluded — the pillar owns
them. -/
theorem walk_pend_or_doneE (hwf : sk.wellFormed = true) {s : State}
    {pk : Party × Nat} (hi : InvP sk .impl s) (hpk : pk ∈ sk.walkKeys)
    (hnc : ¬((s.walk pk).phase = 2 ∧ (s.walk pk).committed = none)) :
    ((∀ e ∈ walkEventsE sk pk, performed sk s e) ∧ wkPend sk s pk = [])
    ∨ ∃ f a pre suf, wkPend sk s pk = [(f, a)]
        ∧ walkEventsE sk pk = pre ++ f :: suf
        ∧ (∀ e ∈ pre, performed sk s e)
        ∧ PendOkE sk s f a := by
  by_cases hph3 : 3 ≤ (s.walk pk).phase
  · -- past the channel work: every block is a completed scope
    left
    constructor
    · intro e he
      obtain ⟨j, hjr, hje⟩ := List.mem_flatMap.1 he
      rw [List.mem_range] at hjr
      have hsc := (walk_scope_boundE sk hi hpk).2 hph3
      exact scopeBlock_performedE sk hwf hi hpk (by omega) hjr e hje
    · unfold wkPend
      rw [if_neg (by omega), if_neg (by omega), if_neg (by omega)]
  · have hsc := (walk_scope_boundE sk hi hpk).1 (by omega)
    -- the shared outer split at the current scope
    have houter : walkEventsE sk pk
        = (List.range (s.walk pk).scope).flatMap (scopeBlockE sk pk)
          ++ scopeBlockE sk pk (s.walk pk).scope
          ++ (List.range' ((s.walk pk).scope + 1)
              (sk.stageLen pk.2 - (s.walk pk).scope - 1)).flatMap
              (scopeBlockE sk pk) := by
      unfold walkEventsE
      rw [range_split (show (s.walk pk).scope ≤ sk.stageLen pk.2
        by omega), List.flatMap_append]
      have hlen : sk.stageLen pk.2 - (s.walk pk).scope
          = (sk.stageLen pk.2 - (s.walk pk).scope - 1) + 1 := by omega
      rw [hlen, List.range'_succ, List.flatMap_cons]
      simp [List.append_assoc]
    have hprepre : ∀ e ∈ (List.range (s.walk pk).scope).flatMap
        (scopeBlockE sk pk), performed sk s e := by
      intro e he
      obtain ⟨j, hjr, hje⟩ := List.mem_flatMap.1 he
      rw [List.mem_range] at hjr
      exact scopeBlock_performedE sk hwf hi hpk hjr (by omega) e hje
    rcases Nat.lt_or_ge (s.walk pk).phase 2 with hph01 | hph2'
    · -- a prologue receive is pending
      right
      rcases Nat.lt_or_ge (s.walk pk).phase 1 with hph0 | hph1
      · have hph : (s.walk pk).phase = 0 := by omega
        refine ⟨(wireIn pk, false, (s.walk pk).scope), .walkRecvWire pk,
          (List.range (s.walk pk).scope).flatMap (scopeBlockE sk pk),
          ((askedIn pk, false, (s.walk pk).scope) ::
            scopeSendsE sk pk (s.walk pk).scope)
            ++ (List.range' ((s.walk pk).scope + 1)
                (sk.stageLen pk.2 - (s.walk pk).scope - 1)).flatMap
                (scopeBlockE sk pk),
          ?_, ?_, hprepre, ?_, ?_, ?_, ?_⟩
        · unfold wkPend
          rw [if_pos hph]
        · rw [houter]
          unfold scopeBlockE
          simp [List.cons_append, List.append_assoc]
        · exact wireIn_mem_allChans sk hwf hpk
        · show (s.walk pk).scope = recvdOf sk s (wireIn pk)
          rw [recvdOf_wireIn hpk]
          unfold wkWireRecvd
          rw [if_neg (by omega)]
          rw [hph]
          simp
        · exact walk_action_mem sk hpk (by simp)
        · intro hch
          simp only [Bool.false_eq_true, if_false] at hch
          have happ : (apply sk .impl (.walkRecvWire pk) s).isSome
              = true := by
            simp [apply, hpk, hph]
            omega
          exact happ
      · have hph : (s.walk pk).phase = 1 := by omega
        refine ⟨(askedIn pk, false, (s.walk pk).scope), .walkRecvAsked pk,
          (List.range (s.walk pk).scope).flatMap (scopeBlockE sk pk)
            ++ [(wireIn pk, false, (s.walk pk).scope)],
          scopeSendsE sk pk (s.walk pk).scope
            ++ (List.range' ((s.walk pk).scope + 1)
                (sk.stageLen pk.2 - (s.walk pk).scope - 1)).flatMap
                (scopeBlockE sk pk),
          ?_, ?_, ?_, ?_, ?_, ?_, ?_⟩
        · unfold wkPend
          rw [if_neg (by omega), if_pos hph]
        · rw [houter]
          unfold scopeBlockE
          simp [List.cons_append, List.append_assoc]
        · intro e he
          rcases List.mem_append.1 he with hp | hone
          · exact hprepre e hp
          · rw [List.mem_singleton] at hone
            subst hone
            show (s.walk pk).scope < recvdOf sk s (wireIn pk)
            rw [recvdOf_wireIn hpk]
            unfold wkWireRecvd
            rw [if_neg (by omega), hph]
            simp
        · exact (walk_chans_mem sk hpk).2.1
        · show (s.walk pk).scope = recvdOf sk s (askedIn pk)
          rw [recvdOf_askedIn]
          unfold wkAskedRecvd
          rw [if_neg (by omega), hph]
          simp
        · exact walk_action_mem sk hpk (by simp)
        · intro hch
          simp only [Bool.false_eq_true, if_false] at hch
          have happ : (apply sk .impl (.walkRecvAsked pk) s).isSome
              = true := by
            simp [apply, hpk, hph]
            omega
          exact happ
    · -- phase 2: committed (uncommitted is the pillar's case)
      have hph2 : (s.walk pk).phase = 2 := by omega
      cases hcm : (s.walk pk).committed with
      | none => exact absurd ⟨hph2, hcm⟩ hnc
      | some o =>
          right
          obtain ⟨f, isp, ss, hpend, hss, hisp, hchan, hside, hseq, hmem⟩ :=
            walk_committed_splitE sk hwf hi hpk hph2 hcm
          refine ⟨f, .walkFire pk,
            (List.range (s.walk pk).scope).flatMap (scopeBlockE sk pk)
              ++ (wireIn pk, false, (s.walk pk).scope)
              :: (askedIn pk, false, (s.walk pk).scope) :: isp,
            ss ++ (List.range' ((s.walk pk).scope + 1)
                (sk.stageLen pk.2 - (s.walk pk).scope - 1)).flatMap
                (scopeBlockE sk pk),
            hpend, ?_, ?_, hmem, ?_, ?_, ?_⟩
          · rw [houter]
            unfold scopeBlockE
            rw [hss]
            simp [List.cons_append, List.append_assoc]
          · intro e he
            rcases List.mem_append.1 he with hp | hcons
            · exact hprepre e hp
            rcases List.mem_cons.1 hcons with rfl | hcons
            · show (s.walk pk).scope < recvdOf sk s (wireIn pk)
              rw [recvdOf_wireIn hpk]
              unfold wkWireRecvd
              rw [if_neg (by omega), hph2]
              simp
            rcases List.mem_cons.1 hcons with rfl | hin
            · show (s.walk pk).scope < recvdOf sk s (askedIn pk)
              rw [recvdOf_askedIn]
              unfold wkAskedRecvd
              rw [if_neg (by omega), hph2]
              simp
            · exact hisp e hin
          · rw [hside]
            exact hseq
          · exact walk_action_mem sk hpk (by simp)
          · intro hch
            rw [hside, if_pos rfl] at hch
            have hcap : sk.cap f.1 = 1 := by
              rw [hchan]
              cases o with
              | wire i => rfl
              | res i => rfl
              | query i =>
                  show sk.cap (askedOut pk) = 1
                  unfold askedOut
                  split
                  · rfl
                  · rfl
              | parent => rfl
            have hlt : s.chan (obligChan pk o) < 1 := by
              rw [← hchan, ← hcap]
              exact hch
            have happ : (apply sk .impl (.walkFire pk) s).isSome
                = true := by
              simp [apply, hcm, hpk, hph2, hlt]
            exact happ
/-- The initiator opening decode. -/
theorem iopen_pend_or_doneE (hwf : sk.wellFormed = true) {s : State}
    (hi : InvP sk .impl s)
    (hch : s.iopenCh = none → doneIOpen s = true) :
    ((∀ e ∈ iopenEvents sk, performed sk s e) ∧ ioPend sk s = [])
    ∨ ∃ f a pre suf, ioPend sk s = [(f, a)]
        ∧ iopenEvents sk = pre ++ f :: suf
        ∧ (∀ e ∈ pre, performed sk s e)
        ∧ PendOkE sk s f a := by
  have hge : 2 ≤ sk.rootH := (wf_rootH hwf).2
  have htop := hi.top
  simp only [topLocalOk, Bool.and_eq_true] at htop
  have hsw : sentOf sk s (Chan.wire Party.I sk.rootH)
      = b2n s.iopenWire := by simp [sentOf]
  have hsq : sentOf sk s (Chan.asked Party.I (sk.rootH - 1))
      = b2n s.iopenQuery := by simp [sentOf]
  cases hc : s.iopenCh with
  | none =>
      left
      have hdone := hch hc
      simp only [doneIOpen, Bool.and_eq_true] at hdone
      obtain ⟨hw, hq⟩ := hdone
      refine ⟨?_, by simp [ioPend, hc]⟩
      intro e he
      unfold iopenEvents at he
      rcases List.mem_cons.1 he with rfl | he
      · show (0 : Nat) < sentOf sk s (Chan.wire Party.I sk.rootH)
        rw [hsw, hw]
        simp [b2n]
      rcases List.mem_cons.1 he with rfl | he
      · show (0 : Nat) < sentOf sk s (Chan.asked Party.I (sk.rootH - 1))
        rw [hsq, hq]
        simp [b2n]
      · cases he
  | some o =>
      right
      -- the committed-arm mirrors of `topLocalOk`
      obtain ⟨⟨⟨⟨⟨⟨⟨⟨⟨⟨⟨hcw, hcq⟩, -⟩, -⟩, -⟩, -⟩, -⟩, -⟩, -⟩, -⟩, -⟩,
        -⟩ := htop
      cases o with
      | wire =>
          have hnw : s.iopenWire = false := by
            rw [hc] at hcw
            simpa using hcw
          refine ⟨(Chan.wire Party.I sk.rootH, true, 0), .iopenFire,
            [], [(Chan.asked Party.I (sk.rootH - 1), true, 0)],
            by simp [ioPend, hc], rfl, ?_,
            (root_chans_mem sk).1, ?_, fixed_action_mem sk (by simp), ?_⟩
          · intro e he
            cases he
          · show (0 : Nat) = sentOf sk s (Chan.wire Party.I sk.rootH)
            rw [hsw, hnw]
            rfl
          · intro hchan
            rw [if_pos rfl] at hchan
            have : (apply sk .impl .iopenFire s).isSome = true := by
              simp only [apply, hc]
              rw [if_pos (by simpa [Skel.cap] using hchan)]
              rfl
            exact this
      | query =>
          have hq2 : s.iopenQuery = false ∧ s.iopenWire = true := by
            rw [hc] at hcq
            have := by simpa [AxMode.impl] using hcq
            exact this
          refine ⟨(Chan.asked Party.I (sk.rootH - 1), true, 0), .iopenFire,
            [(Chan.wire Party.I sk.rootH, true, 0)], [],
            by simp [ioPend, hc], rfl, ?_,
            ?_, ?_, fixed_action_mem sk (by simp), ?_⟩
          · intro e he
            rw [List.mem_singleton] at he
            subst he
            show (0 : Nat) < sentOf sk s (Chan.wire Party.I sk.rootH)
            rw [hsw, hq2.2]
            simp [b2n]
          · have hkey : (Party.I, sk.rootH - 1) ∈ sk.walkKeys :=
              mem_walkKeys_of sk hwf (by omega)
                (Or.inl ⟨rfl, by
                  have := (wf_rootH hwf).1
                  omega⟩)
            have : Chan.asked Party.I (sk.rootH - 1)
                = askedIn (Party.I, sk.rootH - 1) := rfl
            rw [this]
            exact (walk_chans_mem sk hkey).2.1
          · show (0 : Nat) = sentOf sk s (Chan.asked Party.I (sk.rootH - 1))
            rw [hsq, hq2.1]
            rfl
          · intro hchan
            rw [if_pos rfl] at hchan
            have : (apply sk .impl .iopenFire s).isSome = true := by
              simp only [apply, hc]
              rw [if_pos (by simpa [Skel.cap] using hchan)]
              rfl
            exact this

/-- The floating root-return decode. -/
theorem rootret_pend_or_doneE {s : State} :
    ((∀ e ∈ [((Chan.rootret, false, 0) : Ev)], performed sk s e)
      ∧ rrPend s = [])
    ∨ ∃ f a pre suf, rrPend s = [(f, a)]
        ∧ [((Chan.rootret, false, 0) : Ev)] = pre ++ f :: suf
        ∧ (∀ e ∈ pre, performed sk s e)
        ∧ PendOkE sk s f a := by
  cases hf : s.ifin with
  | true =>
      left
      refine ⟨?_, by simp [rrPend, hf]⟩
      intro e he
      rw [List.mem_singleton] at he
      subst he
      show (0 : Nat) < recvdOf sk s Chan.rootret
      show (0 : Nat) < b2n s.ifin
      rw [hf]
      simp [b2n]
  | false =>
      right
      refine ⟨(Chan.rootret, false, 0), .finRet, [], [],
        by simp [rrPend, hf], rfl, ?_,
        (root_chans_mem sk).2.2.2.2.1, ?_, fixed_action_mem sk (by simp), ?_⟩
      · intro e he
        cases he
      · show (0 : Nat) = recvdOf sk s Chan.rootret
        show (0 : Nat) = b2n s.ifin
        rw [hf]
        rfl
      · intro hchan
        rw [if_neg (by simp)] at hchan
        have : (apply sk .impl .finRet s).isSome = true := by
          simp [apply, hf]
          omega
        exact this

/-- The responder finish decode. -/
theorem fin_pend_or_doneE {s : State} (hi : InvP sk .impl s) :
    ((∀ e ∈ finEvents sk, performed sk s e) ∧ finPend sk s = [])
    ∨ ∃ f a pre suf, finPend sk s = [(f, a)]
        ∧ finEvents sk = pre ++ f :: suf
        ∧ (∀ e ∈ pre, performed sk s e)
        ∧ PendOkE sk s f a := by
  have htop := hi.top
  simp only [topLocalOk, Bool.and_eq_true, decide_eq_true_eq] at htop
  obtain ⟨-, hgle⟩ := htop
  cases hgr : s.rfinGotRes with
  | false =>
      right
      refine ⟨(Chan.rootres, false, 0), .finRes, [],
        (List.range sk.rootPending).map fun j =>
          (Chan.rootrets, false, j),
        by simp [finPend, hgr], rfl, ?_,
        (root_chans_mem sk).2.2.2.2.2.2, ?_,
        fixed_action_mem sk (by simp), ?_⟩
      · intro e he
        cases he
      · show (0 : Nat) = recvdOf sk s Chan.rootres
        show (0 : Nat) = b2n s.rfinGotRes
        rw [hgr]
        rfl
      · intro hchan
        rw [if_neg (by simp)] at hchan
        have : (apply sk .impl .finRes s).isSome = true := by
          simp [apply, hgr]
          omega
        exact this
  | true =>
      have hperf0 : performed sk s (Chan.rootres, false, 0) := by
        show (0 : Nat) < b2n s.rfinGotRes
        rw [hgr]
        simp [b2n]
      rcases Nat.lt_or_ge s.rfinGot sk.rootPending with hlt | hge
      · right
        refine ⟨(Chan.rootrets, false, s.rfinGot), .finRets,
          (Chan.rootres, false, 0)
            :: ((List.range s.rfinGot).map fun j =>
              (Chan.rootrets, false, j)),
          (List.range' (s.rfinGot + 1)
            (sk.rootPending - s.rfinGot - 1)).map fun j =>
            (Chan.rootrets, false, j),
          by simp [finPend, hgr, hlt], ?_, ?_,
          (root_chans_mem sk).2.2.2.2.2.1, ?_,
          fixed_action_mem sk (by simp), ?_⟩
        · unfold finEvents
          rw [range_split (show s.rfinGot ≤ sk.rootPending by omega),
            List.map_append,
            show sk.rootPending - s.rfinGot
              = (sk.rootPending - s.rfinGot - 1) + 1 from by omega,
            List.range'_succ, List.map_cons]
          simp [List.cons_append]
        · intro e he
          rcases List.mem_cons.1 he with rfl | he
          · exact hperf0
          · obtain ⟨j, hjm, rfl⟩ := List.mem_map.1 he
            rw [List.mem_range] at hjm
            show j < recvdOf sk s Chan.rootrets
            show j < s.rfinGot
            omega
        · show s.rfinGot = recvdOf sk s Chan.rootrets
          rfl
        · intro hchan
          rw [if_neg (by simp)] at hchan
          have : (apply sk .impl .finRets s).isSome = true := by
            simp [apply, hgr, hlt]
            omega
          exact this
      · left
        have hgeq : s.rfinGot = sk.rootPending := by omega
        refine ⟨?_, by simp [finPend, hgr, hgeq]⟩
        intro e he
        unfold finEvents at he
        rcases List.mem_cons.1 he with rfl | he
        · exact hperf0
        · obtain ⟨j, hjm, rfl⟩ := List.mem_map.1 he
          rw [List.mem_range] at hjm
          show j < recvdOf sk s Chan.rootrets
          show j < s.rfinGot
          omega

/-- The responder opening decode. -/
theorem ropen_pend_or_doneE (hwf : sk.wellFormed = true) {s : State}
    (hi : InvP sk .impl s)
    (hch : s.ropenGotWire = true → s.ropenCh = none →
      doneROpen sk s = true) :
    ((∀ e ∈ ropenEvents sk, performed sk s e) ∧ roPend sk s = [])
    ∨ ∃ f a pre suf, roPend sk s = [(f, a)]
        ∧ ropenEvents sk = pre ++ f :: suf
        ∧ (∀ e ∈ pre, performed sk s e)
        ∧ PendOkE sk s f a := by
  have hge : 2 ≤ sk.rootH := (wf_rootH hwf).2
  have hev : sk.rootH % 2 = 0 := (wf_rootH hwf).1
  have htop := hi.top
  simp only [topLocalOk, Bool.and_eq_true, decide_eq_true_eq] at htop
  obtain ⟨⟨⟨⟨⟨⟨⟨⟨⟨⟨⟨-, -⟩, -⟩, hqle⟩, -⟩, hcw⟩, hcr⟩, hcq⟩, -⟩, -⟩, -⟩,
    -⟩ := htop
  have hrw : recvdOf sk s (Chan.wire Party.I sk.rootH)
      = b2n s.ropenGotWire := by simp [recvdOf]
  have hsw : sentOf sk s (Chan.wire Party.R sk.rootH)
      = b2n s.ropenWire := by simp [sentOf]
  have hsq : sentOf sk s (Chan.asked Party.R (sk.rootH - 2))
      = s.ropenQ := by
    simp [sentOf]
  have hqmem : Chan.asked Party.R (sk.rootH - 2) ∈ allChans sk := by
    have hkey : (Party.R, sk.rootH - 2) ∈ sk.walkKeys :=
      mem_walkKeys_of sk hwf (by omega) (Or.inr ⟨rfl, by omega⟩)
    have : Chan.asked Party.R (sk.rootH - 2)
        = askedIn (Party.R, sk.rootH - 2) := rfl
    rw [this]
    exact (walk_chans_mem sk hkey).2.1
  cases hgw : s.ropenGotWire with
  | false =>
      right
      refine ⟨(Chan.wire Party.I sk.rootH, false, 0), .ropenRecv, [],
        (Chan.wire Party.R sk.rootH, true, 0)
          :: (Chan.rootres, true, 0)
          :: ((List.range sk.rootPending).map fun j =>
              (Chan.asked Party.R (sk.rootH - 2), true, j)),
        by simp [roPend, hgw], rfl, ?_,
        (root_chans_mem sk).1, ?_, fixed_action_mem sk (by simp), ?_⟩
      · intro e he
        cases he
      · show (0 : Nat) = recvdOf sk s (Chan.wire Party.I sk.rootH)
        rw [hrw, hgw]
        rfl
      · intro hchan
        rw [if_neg (by simp)] at hchan
        have : (apply sk .impl .ropenRecv s).isSome = true := by
          simp [apply, hgw]
          omega
        exact this
  | true =>
      have hperf0 : performed sk s (Chan.wire Party.I sk.rootH, false, 0) := by
        show (0 : Nat) < recvdOf sk s (Chan.wire Party.I sk.rootH)
        rw [hrw, hgw]
        simp [b2n]
      cases hc : s.ropenCh with
      | none =>
          left
          have hdone := hch hgw hc
          simp only [doneROpen, Bool.and_eq_true, beq_iff_eq] at hdone
          obtain ⟨⟨⟨-, hw⟩, hr⟩, hq⟩ := hdone
          refine ⟨?_, by simp [roPend, hgw, hc]⟩
          intro e he
          unfold ropenEvents at he
          rcases List.mem_cons.1 he with rfl | he
          · exact hperf0
          rcases List.mem_cons.1 he with rfl | he
          · show (0 : Nat) < sentOf sk s (Chan.wire Party.R sk.rootH)
            rw [hsw, hw]
            simp [b2n]
          rcases List.mem_cons.1 he with rfl | he
          · show (0 : Nat) < sentOf sk s Chan.rootres
            show (0 : Nat) < b2n s.ropenRes
            rw [hr]
            simp [b2n]
          · obtain ⟨j, hjm, rfl⟩ := List.mem_map.1 he
            rw [List.mem_range] at hjm
            show j < sentOf sk s (Chan.asked Party.R (sk.rootH - 2))
            rw [hsq]
            unfold Skel.rootPending at hjm
            omega
      | some o =>
          right
          cases o with
          | wire =>
              have hnw : s.ropenWire = false := by
                rw [hc] at hcw
                simpa using hcw
              refine ⟨(Chan.wire Party.R sk.rootH, true, 0), .ropenFire,
                [(Chan.wire Party.I sk.rootH, false, 0)],
                (Chan.rootres, true, 0)
                  :: ((List.range sk.rootPending).map fun j =>
                    (Chan.asked Party.R (sk.rootH - 2), true, j)),
                by simp [roPend, hgw, hc], rfl, ?_,
                (root_chans_mem sk).2.1, ?_,
                fixed_action_mem sk (by simp), ?_⟩
              · intro e he
                rw [List.mem_singleton] at he
                subst he
                exact hperf0
              · show (0 : Nat) = sentOf sk s (Chan.wire Party.R sk.rootH)
                rw [hsw, hnw]
                rfl
              · intro hchan
                rw [if_pos rfl] at hchan
                have : (apply sk .impl .ropenFire s).isSome = true := by
                  simp only [apply, hc]
                  rw [if_pos (by simpa [Skel.cap] using hchan)]
                  rfl
                exact this
          | res =>
              have hnr : s.ropenRes = false ∧ s.ropenWire = true := by
                rw [hc] at hcr
                have := by simpa [AxMode.impl] using hcr
                exact this
              refine ⟨(Chan.rootres, true, 0), .ropenFire,
                [(Chan.wire Party.I sk.rootH, false, 0),
                  (Chan.wire Party.R sk.rootH, true, 0)],
                (List.range sk.rootPending).map fun j =>
                  (Chan.asked Party.R (sk.rootH - 2), true, j),
                by simp [roPend, hgw, hc], rfl, ?_,
                (root_chans_mem sk).2.2.2.2.2.2, ?_,
                fixed_action_mem sk (by simp), ?_⟩
              · intro e he
                rcases List.mem_cons.1 he with rfl | he
                · exact hperf0
                rcases List.mem_cons.1 he with rfl | he
                · show (0 : Nat) < sentOf sk s (Chan.wire Party.R sk.rootH)
                  rw [hsw, hnr.2]
                  simp [b2n]
                · cases he
              · show (0 : Nat) = sentOf sk s Chan.rootres
                show (0 : Nat) = b2n s.ropenRes
                rw [hnr.1]
                rfl
              · intro hchan
                rw [if_pos rfl] at hchan
                have : (apply sk .impl .ropenFire s).isSome = true := by
                  simp only [apply, hc]
                  rw [if_pos (by simpa [Skel.cap] using hchan)]
                  rfl
                exact this
          | query =>
              have hq3 : s.ropenQ < sk.rootPending ∧ s.ropenRes = true := by
                rw [hc] at hcq
                have := by simpa [AxMode.impl] using hcq
                exact this
              have hwtrue : s.ropenWire = true := by
                -- the topLocalOk w-shadow: res fired forces the wire
                have htop2 := hi.top
                simp only [topLocalOk, Bool.and_eq_true] at htop2
                obtain ⟨⟨⟨⟨⟨⟨⟨⟨⟨⟨⟨-, -⟩, -⟩, -⟩, hsh⟩, -⟩, -⟩, -⟩, -⟩, -⟩,
                  -⟩, -⟩ := htop2
                rcases (by simpa [AxMode.impl] using hsh :
                    s.ropenRes = false ∨ s.ropenWire = true) with hf | h
                · rw [hq3.2] at hf; cases hf
                · exact h
              refine ⟨(Chan.asked Party.R (sk.rootH - 2), true, s.ropenQ),
                .ropenFire,
                (Chan.wire Party.I sk.rootH, false, 0)
                  :: (Chan.wire Party.R sk.rootH, true, 0)
                  :: (Chan.rootres, true, 0)
                  :: ((List.range s.ropenQ).map fun j =>
                    (Chan.asked Party.R (sk.rootH - 2), true, j)),
                (List.range' (s.ropenQ + 1)
                  (sk.rootPending - s.ropenQ - 1)).map fun j =>
                  (Chan.asked Party.R (sk.rootH - 2), true, j),
                by simp [roPend, hgw, hc], ?_, ?_,
                hqmem, ?_, fixed_action_mem sk (by simp), ?_⟩
              · unfold ropenEvents
                rw [range_split (show s.ropenQ ≤ sk.rootPending
                    by omega),
                  List.map_append,
                  show sk.rootPending - s.ropenQ
                    = (sk.rootPending - s.ropenQ - 1) + 1 from by omega,
                  List.range'_succ, List.map_cons]
                simp [List.cons_append]
              · intro e he
                rcases List.mem_cons.1 he with rfl | he
                · exact hperf0
                rcases List.mem_cons.1 he with rfl | he
                · show (0 : Nat) < sentOf sk s (Chan.wire Party.R sk.rootH)
                  rw [hsw, hwtrue]
                  simp [b2n]
                rcases List.mem_cons.1 he with rfl | he
                · show (0 : Nat) < sentOf sk s Chan.rootres
                  show (0 : Nat) < b2n s.ropenRes
                  rw [hq3.2]
                  simp [b2n]
                · obtain ⟨j, hjm, rfl⟩ := List.mem_map.1 he
                  rw [List.mem_range] at hjm
                  show j < sentOf sk s (Chan.asked Party.R (sk.rootH - 2))
                  rw [hsq]
                  omega
              · show s.ropenQ = sentOf sk s (Chan.asked Party.R (sk.rootH - 2))
                rw [hsq]
              · intro hchan
                rw [if_pos rfl] at hchan
                have : (apply sk .impl .ropenFire s).isSome = true := by
                  simp only [apply, hc]
                  rw [if_pos (by simpa [Skel.cap] using hchan)]
                  rfl
                exact this

/-- The absorber decode. -/
theorem absorb_pend_or_doneE (hwf : sk.wellFormed = true) {s : State}
    (hi : InvP sk .impl s) :
    ((∀ e ∈ absorbEvents sk, performed sk s e) ∧ abPend s = [])
    ∨ ∃ f a pre suf, abPend s = [(f, a)]
        ∧ absorbEvents sk = pre ++ f :: suf
        ∧ (∀ e ∈ pre, performed sk s e)
        ∧ PendOkE sk s f a := by
  have hge : 2 ≤ sk.rootH := (wf_rootH hwf).2
  have htop := hi.top
  simp only [topLocalOk, Bool.and_eq_true, decide_eq_true_eq] at htop
  obtain ⟨⟨⟨⟨-, hcur⟩, -⟩, -⟩, -⟩ := htop
  have hidx1 : s.absorbPhase ≤ 2 → s.absorbIdx < sk.totalLeafReqs := by
    intro h
    rw [if_pos h] at hcur
    simpa using hcur
  have hidx2 : 3 ≤ s.absorbPhase → s.absorbIdx = sk.totalLeafReqs := by
    intro h
    rw [if_neg (by omega)] at hcur
    simpa using hcur
  have hrw : recvdOf sk s (Chan.wire Party.R 0)
      = absorbWireRecvd sk s := by
    have hne : (0 == sk.rootH) = false := by
      simp
      omega
    simp [recvdOf, hne]
  have hrq : recvdOf sk s Chan.leafRequests = absorbAskedRecvd sk s := rfl
  have hsl : sentOf sk s (Chan.level Party.I 0) = s.absorbIdx := by
    simp [sentOf]
  have hWa : s.absorbIdx ≤ absorbWireRecvd sk s := by
    unfold absorbWireRecvd
    by_cases h3 : 3 ≤ s.absorbPhase
    · rw [if_pos (by omega)]
      have := hidx2 h3
      omega
    · rw [if_neg (by omega)]
      omega
  have hAa : s.absorbIdx ≤ absorbAskedRecvd sk s := by
    unfold absorbAskedRecvd
    by_cases h3 : 3 ≤ s.absorbPhase
    · rw [if_pos (by omega)]
      have := hidx2 h3
      omega
    · rw [if_neg (by omega)]
      omega
  have hblock : ∀ j, j < s.absorbIdx →
      ∀ e ∈ [((Chan.wire Party.R 0, false, j) : Ev),
        (Chan.leafRequests, false, j), (Chan.level Party.I 0, true, j)],
      performed sk s e := by
    intro j hj e he
    rcases List.mem_cons.1 he with rfl | he
    · show j < recvdOf sk s (Chan.wire Party.R 0)
      rw [hrw]
      omega
    rcases List.mem_cons.1 he with rfl | he
    · show j < recvdOf sk s Chan.leafRequests
      rw [hrq]
      omega
    rcases List.mem_cons.1 he with rfl | he
    · show j < sentOf sk s (Chan.level Party.I 0)
      rw [hsl]
      omega
    · cases he
  have hwr0mem : Chan.wire Party.R 0 ∈ allChans sk := by
    have hkey : (Party.R, 0) ∈ sk.walkKeys :=
      mem_walkKeys_of sk hwf (by omega) (Or.inr ⟨rfl, by omega⟩)
    have : Chan.wire Party.R 0 = wireOut (Party.R, 0) := rfl
    rw [this]
    exact (walk_chans_mem sk hkey).1
  have hsplit : ∀ hlt : s.absorbIdx < sk.totalLeafReqs,
      absorbEvents sk
      = (List.range s.absorbIdx).flatMap (fun j =>
          [((Chan.wire Party.R 0, false, j) : Ev),
            (Chan.leafRequests, false, j), (Chan.level Party.I 0, true, j)])
        ++ ((Chan.wire Party.R 0, false, s.absorbIdx)
          :: (Chan.leafRequests, false, s.absorbIdx)
          :: (Chan.level Party.I 0, true, s.absorbIdx)
          :: (List.range' (s.absorbIdx + 1)
              (sk.totalLeafReqs - s.absorbIdx - 1)).flatMap (fun j =>
              [((Chan.wire Party.R 0, false, j) : Ev),
                (Chan.leafRequests, false, j),
                (Chan.level Party.I 0, true, j)])) := by
    intro hlt
    unfold absorbEvents
    rw [range_split (show s.absorbIdx ≤ sk.totalLeafReqs by omega),
      List.flatMap_append,
      show sk.totalLeafReqs - s.absorbIdx
        = (sk.totalLeafReqs - s.absorbIdx - 1) + 1 from by omega,
      List.range'_succ, List.flatMap_cons]
    rfl
  have hpreperf : ∀ e ∈ (List.range s.absorbIdx).flatMap (fun j =>
      [((Chan.wire Party.R 0, false, j) : Ev),
        (Chan.leafRequests, false, j), (Chan.level Party.I 0, true, j)]),
      performed sk s e := by
    intro e he
    obtain ⟨j, hjm, hje⟩ := List.mem_flatMap.1 he
    rw [List.mem_range] at hjm
    exact hblock j hjm e hje
  rcases Nat.lt_or_ge s.absorbPhase 3 with hph | hph3
  · right
    have hlt := hidx1 (by omega)
    rcases Nat.lt_or_ge s.absorbPhase 1 with hph0 | hph1
    · have hph' : s.absorbPhase = 0 := by omega
      refine ⟨(Chan.wire Party.R 0, false, s.absorbIdx), .absorbRecvWire,
        _, _, by simp [abPend, hph'], hsplit hlt, hpreperf,
        hwr0mem, ?_, fixed_action_mem sk (by simp), ?_⟩
      · show s.absorbIdx = recvdOf sk s (Chan.wire Party.R 0)
        rw [hrw]
        unfold absorbWireRecvd
        rw [if_neg (by omega), hph']
        simp
      · intro hchan
        rw [if_neg (by simp)] at hchan
        have : (apply sk .impl .absorbRecvWire s).isSome = true := by
          simp [apply, hph']
          omega
        exact this
    · rcases Nat.lt_or_ge s.absorbPhase 2 with hph1' | hph2
      · have hph' : s.absorbPhase = 1 := by omega
        refine ⟨(Chan.leafRequests, false, s.absorbIdx), .absorbRecvAsked,
          (List.range s.absorbIdx).flatMap (fun j =>
            [((Chan.wire Party.R 0, false, j) : Ev),
              (Chan.leafRequests, false, j),
              (Chan.level Party.I 0, true, j)])
            ++ [(Chan.wire Party.R 0, false, s.absorbIdx)],
          (Chan.level Party.I 0, true, s.absorbIdx)
            :: (List.range' (s.absorbIdx + 1)
              (sk.totalLeafReqs - s.absorbIdx - 1)).flatMap (fun j =>
              [((Chan.wire Party.R 0, false, j) : Ev),
                (Chan.leafRequests, false, j),
                (Chan.level Party.I 0, true, j)]),
          by simp [abPend, hph'], ?_, ?_,
          (root_chans_mem sk).2.2.1, ?_,
          fixed_action_mem sk (by simp), ?_⟩
        · rw [hsplit hlt]
          simp [List.cons_append]
        · intro e he
          rcases List.mem_append.1 he with hp | hone
          · exact hpreperf e hp
          · rw [List.mem_singleton] at hone
            subst hone
            show s.absorbIdx < recvdOf sk s (Chan.wire Party.R 0)
            rw [hrw]
            unfold absorbWireRecvd
            rw [if_neg (by omega), hph']
            simp
        · show s.absorbIdx = recvdOf sk s Chan.leafRequests
          rw [hrq]
          unfold absorbAskedRecvd
          rw [if_neg (by omega), hph']
          simp
        · intro hchan
          rw [if_neg (by simp)] at hchan
          have : (apply sk .impl .absorbRecvAsked s).isSome = true := by
            simp [apply, hph']
            omega
          exact this
      · have hph' : s.absorbPhase = 2 := by omega
        refine ⟨(Chan.level Party.I 0, true, s.absorbIdx), .absorbSend,
          (List.range s.absorbIdx).flatMap (fun j =>
            [((Chan.wire Party.R 0, false, j) : Ev),
              (Chan.leafRequests, false, j),
              (Chan.level Party.I 0, true, j)])
            ++ [(Chan.wire Party.R 0, false, s.absorbIdx),
              (Chan.leafRequests, false, s.absorbIdx)],
          (List.range' (s.absorbIdx + 1)
            (sk.totalLeafReqs - s.absorbIdx - 1)).flatMap (fun j =>
            [((Chan.wire Party.R 0, false, j) : Ev),
              (Chan.leafRequests, false, j),
              (Chan.level Party.I 0, true, j)]),
          by simp [abPend, hph'], ?_, ?_,
          (root_chans_mem sk).2.2.2.1, ?_,
          fixed_action_mem sk (by simp), ?_⟩
        · rw [hsplit hlt]
          simp [List.cons_append]
        · intro e he
          rcases List.mem_append.1 he with hp | htwo
          · exact hpreperf e hp
          rcases List.mem_cons.1 htwo with rfl | htwo
          · show s.absorbIdx < recvdOf sk s (Chan.wire Party.R 0)
            rw [hrw]
            unfold absorbWireRecvd
            rw [if_neg (by omega), hph']
            simp
          rcases List.mem_cons.1 htwo with rfl | hnil
          · show s.absorbIdx < recvdOf sk s Chan.leafRequests
            rw [hrq]
            unfold absorbAskedRecvd
            rw [if_neg (by omega), hph']
            simp
          · cases hnil
        · show s.absorbIdx = sentOf sk s (Chan.level Party.I 0)
          rw [hsl]
        · intro hchan
          rw [if_pos rfl] at hchan
          have : (apply sk .impl .absorbSend s).isSome = true := by
            simp [apply, hph']
            omega
          exact this
  · left
    have hidx := hidx2 hph3
    have hpend0 : abPend s = [] := by
      unfold abPend
      rw [if_neg (by omega), if_neg (by omega), if_neg (by omega)]
    refine ⟨?_, hpend0⟩
    intro e he
    unfold absorbEvents at he
    obtain ⟨j, hjm, hje⟩ := List.mem_flatMap.1 he
    rw [List.mem_range] at hjm
    exact hblock j (by omega) e hje

-- ============================================== the assembler decode

/-- The assembler decode. -/
theorem asm_pend_or_doneE (hwf : sk.wellFormed = true) {s : State}
    (hi : InvP sk .impl s) {pk : Party × Nat} (hpk : pk ∈ sk.asmKeys) :
    ((∀ e ∈ asmEvents sk pk, performed sk s e) ∧ asmPend sk s pk = [])
    ∨ ∃ f a pre suf, asmPend sk s pk = [(f, a)]
        ∧ asmEvents sk pk = pre ++ f :: suf
        ∧ (∀ e ∈ pre, performed sk s e)
        ∧ PendOkE sk s f a := by
  have hasm := hi.asm pk hpk
  simp only [asmLocalOk, Bool.and_eq_true, decide_eq_true_eq,
    beq_iff_eq, Bool.or_eq_true, bne_iff_ne, ne_eq,
    Bool.not_eq_true'] at hasm
  obtain ⟨⟨⟨⟨hcur, -⟩, hg1⟩, hg2⟩, hg0⟩ := hasm
  have hidx1 : (s.asm pk).phase ≤ 2 →
      (s.asm pk).idx < (sk.asmResList pk.1 pk.2).length := by
    intro h
    rw [if_pos h] at hcur
    simpa using hcur
  have hidx2 : 3 ≤ (s.asm pk).phase →
      (s.asm pk).idx = (sk.asmResList pk.1 pk.2).length := by
    intro h
    rw [if_neg (by omega)] at hcur
    simpa using hcur
  have hRR := recvdOf_asmRes sk (s := s) hpk
  have hRL := recvdOf_asmLevel sk (s := s) hpk
  have hSO := sentOf_asmOut sk (s := s) hpk
  -- a completed block's events are all performed
  have hblock : ∀ j, j < (s.asm pk).idx →
      ∀ e ∈ asmBlock sk pk j, performed sk s e := by
    intro j hj e he
    rw [asmBlock_eq] at he
    rcases List.mem_cons.1 he with rfl | he
    · show j < recvdOf sk s (asmResChan pk)
      rw [hRR]
      simp only [asmResRecvd]
      omega
    rcases List.mem_append.1 he with hseg | hone
    · obtain ⟨cc, bb, nn⟩ := e
      obtain ⟨hc, hb, hlo, hhi⟩ := mem_seg hseg
      subst hc hb
      show nn < recvdOf sk s (asmLevelChan pk)
      rw [hRL]
      simp only [asmLevelRecvd]
      have hp1 : sk.pendsBefore pk.1 pk.2 j + sk.pendAt pk.1 pk.2 j
          = sk.pendsBefore pk.1 pk.2 (j + 1) :=
        (pendsBefore_succ sk (by omega)).symm
      have hp2 := pendsBefore_mono sk pk.1 pk.2
        (show j + 1 ≤ (s.asm pk).idx by omega)
      omega
    · rw [List.mem_singleton] at hone
      subst hone
      show j < sentOf sk s (sk.asmOutChan pk)
      rw [hSO]
      simp only [asmOutSent]
      omega
  have hpreperf : ∀ e ∈ (List.range (s.asm pk).idx).flatMap
      (asmBlock sk pk), performed sk s e := by
    intro e he
    obtain ⟨j, hjm, hje⟩ := List.mem_flatMap.1 he
    rw [List.mem_range] at hjm
    exact hblock j hjm e hje
  have hsplit : ∀ hlt : (s.asm pk).idx < (sk.asmResList pk.1 pk.2).length,
      asmEvents sk pk
      = (List.range (s.asm pk).idx).flatMap (asmBlock sk pk)
        ++ ((asmResChan pk, false, (s.asm pk).idx)
          :: (seg (asmLevelChan pk) false
              (sk.pendsBefore pk.1 pk.2 (s.asm pk).idx)
              (sk.pendAt pk.1 pk.2 (s.asm pk).idx)
            ++ (sk.asmOutChan pk, true, (s.asm pk).idx)
              :: (List.range' ((s.asm pk).idx + 1)
                ((sk.asmResList pk.1 pk.2).length
                  - (s.asm pk).idx - 1)).flatMap (asmBlock sk pk))) := by
    intro hlt
    unfold asmEvents
    rw [range_split (show (s.asm pk).idx
        ≤ (sk.asmResList pk.1 pk.2).length by omega),
      List.flatMap_append,
      show (sk.asmResList pk.1 pk.2).length - (s.asm pk).idx
        = ((sk.asmResList pk.1 pk.2).length - (s.asm pk).idx - 1) + 1
        from by omega,
      List.range'_succ, List.flatMap_cons]
    rw [asmBlock_eq]
    simp [List.cons_append, List.append_assoc]
  rcases Nat.lt_or_ge (s.asm pk).phase 3 with hph | hph3
  · right
    have hlt := hidx1 (by omega)
    rcases Nat.lt_or_ge (s.asm pk).phase 1 with hph0 | hph1
    · have hph' : (s.asm pk).phase = 0 := by omega
      refine ⟨(asmResChan pk, false, (s.asm pk).idx), .asmRecvRes pk,
        _, _, by simp [asmPend, hph'], hsplit hlt, hpreperf,
        asmResChan_mem sk hwf hpk, ?_, asm_action_mem sk hpk (by simp), ?_⟩
      · show (s.asm pk).idx = recvdOf sk s (asmResChan pk)
        rw [hRR]
        simp only [asmResRecvd]
        rw [hph']
        simp
      · intro hchan
        rw [if_neg (by simp)] at hchan
        have : (apply sk .impl (.asmRecvRes pk) s).isSome = true := by
          simp [apply, hpk, hph']
          omega
        exact this
    · rcases Nat.lt_or_ge (s.asm pk).phase 2 with hph1' | hph2
      · have hph' : (s.asm pk).phase = 1 := by omega
        have hgot : (s.asm pk).got < sk.pendAt pk.1 pk.2 (s.asm pk).idx := by
          rcases hg1 with hne | h
          · exact absurd hph' (by simpa using hne)
          · exact h
        have hsegsplit : seg (asmLevelChan pk) false
            (sk.pendsBefore pk.1 pk.2 (s.asm pk).idx)
            (sk.pendAt pk.1 pk.2 (s.asm pk).idx)
            = seg (asmLevelChan pk) false
                (sk.pendsBefore pk.1 pk.2 (s.asm pk).idx) (s.asm pk).got
              ++ (asmLevelChan pk, false,
                  sk.pendsBefore pk.1 pk.2 (s.asm pk).idx + (s.asm pk).got)
                :: seg (asmLevelChan pk) false
                  (sk.pendsBefore pk.1 pk.2 (s.asm pk).idx
                    + (s.asm pk).got + 1)
                  (sk.pendAt pk.1 pk.2 (s.asm pk).idx
                    - (s.asm pk).got - 1) := by
          conv => lhs; rw [show sk.pendAt pk.1 pk.2 (s.asm pk).idx
              = (s.asm pk).got
                + ((sk.pendAt pk.1 pk.2 (s.asm pk).idx
                  - (s.asm pk).got - 1) + 1) from by omega]
          rw [← seg_append, seg_cons]
        refine ⟨(asmLevelChan pk, false,
            sk.pendsBefore pk.1 pk.2 (s.asm pk).idx + (s.asm pk).got),
          .asmRecvLevel pk,
          (List.range (s.asm pk).idx).flatMap (asmBlock sk pk)
            ++ ((asmResChan pk, false, (s.asm pk).idx)
              :: seg (asmLevelChan pk) false
                (sk.pendsBefore pk.1 pk.2 (s.asm pk).idx) (s.asm pk).got),
          seg (asmLevelChan pk) false
              (sk.pendsBefore pk.1 pk.2 (s.asm pk).idx
                + (s.asm pk).got + 1)
              (sk.pendAt pk.1 pk.2 (s.asm pk).idx - (s.asm pk).got - 1)
            ++ (sk.asmOutChan pk, true, (s.asm pk).idx)
              :: (List.range' ((s.asm pk).idx + 1)
                ((sk.asmResList pk.1 pk.2).length
                  - (s.asm pk).idx - 1)).flatMap (asmBlock sk pk),
          by simp [asmPend, hph'], ?_, ?_,
          asmLevelChan_mem sk hwf (s := s) hpk (by omega), ?_,
          asm_action_mem sk hpk (by simp), ?_⟩
        · rw [hsplit hlt, hsegsplit]
          simp [List.cons_append, List.append_assoc]
        · intro e he
          rcases List.mem_append.1 he with hp | hcons
          · exact hpreperf e hp
          rcases List.mem_cons.1 hcons with rfl | hseg
          · show (s.asm pk).idx < recvdOf sk s (asmResChan pk)
            rw [hRR]
            simp only [asmResRecvd]
            rw [hph']
            simp
          · obtain ⟨cc, bb, nn⟩ := e
            obtain ⟨hc, hb, hlo, hhi⟩ := mem_seg hseg
            subst hc hb
            show nn < recvdOf sk s (asmLevelChan pk)
            rw [hRL]
            simp only [asmLevelRecvd]
            omega
        · show sk.pendsBefore pk.1 pk.2 (s.asm pk).idx + (s.asm pk).got
              = recvdOf sk s (asmLevelChan pk)
          rw [hRL]
          rfl
        · intro hchan
          rw [if_neg (by simp)] at hchan
          have : (apply sk .impl (.asmRecvLevel pk) s).isSome = true := by
            simp [apply, hpk, hph']
            omega
          exact this
      · have hph' : (s.asm pk).phase = 2 := by omega
        have hgot : (s.asm pk).got = sk.pendAt pk.1 pk.2 (s.asm pk).idx := by
          rcases hg2 with hne | h
          · exact absurd hph' (by simpa using hne)
          · exact h
        refine ⟨(sk.asmOutChan pk, true, (s.asm pk).idx), .asmSend pk,
          (List.range (s.asm pk).idx).flatMap (asmBlock sk pk)
            ++ ((asmResChan pk, false, (s.asm pk).idx)
              :: seg (asmLevelChan pk) false
                (sk.pendsBefore pk.1 pk.2 (s.asm pk).idx)
                (sk.pendAt pk.1 pk.2 (s.asm pk).idx)),
          (List.range' ((s.asm pk).idx + 1)
            ((sk.asmResList pk.1 pk.2).length
              - (s.asm pk).idx - 1)).flatMap (asmBlock sk pk),
          by simp [asmPend, hph'], ?_, ?_, ?_, ?_,
          asm_action_mem sk hpk (by simp), ?_⟩
        · rw [hsplit hlt]
          simp [List.cons_append, List.append_assoc]
        · intro e he
          rcases List.mem_append.1 he with hp | hcons
          · exact hpreperf e hp
          rcases List.mem_cons.1 hcons with rfl | hseg
          · show (s.asm pk).idx < recvdOf sk s (asmResChan pk)
            rw [hRR]
            simp only [asmResRecvd]
            rw [hph']
            simp
          · obtain ⟨cc, bb, nn⟩ := e
            obtain ⟨hc, hb, hlo, hhi⟩ := mem_seg hseg
            subst hc hb
            show nn < recvdOf sk s (asmLevelChan pk)
            rw [hRL]
            simp only [asmLevelRecvd]
            omega
        · -- asmOutChan is a flow channel
          obtain ⟨p, j⟩ := pk
          obtain ⟨h1, hIb, hRb⟩ := asmKeys_bounds sk hpk
          unfold Skel.asmOutChan
          by_cases hIr : p = Party.I ∧ j = sk.rootH
          · obtain ⟨rfl, rfl⟩ := hIr
            rw [if_pos (by simp)]
            exact (root_chans_mem sk).2.2.2.2.1
          · rw [if_neg (by cases p <;> simp_all)]
            by_cases hRr : p = Party.R ∧ j = sk.rootH - 1
            · obtain ⟨rfl, rfl⟩ := hRr
              rw [if_pos (by simp)]
              exact (root_chans_mem sk).2.2.2.2.2.1
            · rw [if_neg (by cases p <;> simp_all)]
              unfold allChans
              refine List.mem_append.mpr (Or.inl
                (List.mem_append.mpr (Or.inr ?_)))
              exact List.mem_map.mpr ⟨(p, j), hpk, rfl⟩
        · show (s.asm pk).idx = sentOf sk s (sk.asmOutChan pk)
          rw [hSO]
          rfl
        · intro hchan
          rw [if_pos rfl] at hchan
          have : (apply sk .impl (.asmSend pk) s).isSome = true := by
            simp [apply, hpk, hph']
            omega
          exact this
  · left
    have hidx := hidx2 hph3
    have hg0' : (s.asm pk).got = 0 := by
      rcases hg0 with hne | h
      · rw [Bool.or_eq_false_iff] at hne
        obtain ⟨-, h3⟩ := hne
        simp only [decide_eq_false_iff_not] at h3
        omega
      · exact h
    refine ⟨?_, by
      unfold asmPend
      rw [if_neg (by omega), if_neg (by omega), if_neg (by omega)]⟩
    intro e he
    unfold asmEvents at he
    obtain ⟨j, hjm, hje⟩ := List.mem_flatMap.1 he
    rw [List.mem_range] at hjm
    exact hblock j (by omega) e hje


end StreamingMirror.Sched
