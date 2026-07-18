/-
The weave's counting layer (PROGRESS.md §7 3b, the first Lean
obligation of merge completeness): the structural invariant `WCount`
that the worklist interpreter preserves — per-trace remainder
structure, counter agreement, and event provenance — proven with NO
enabledness hypothesis anywhere.

# Shape

The manual traces' unemitted suffixes are not state: they are
RECOVERED from the worklist by ownership (`manFilters` filters the
future emissions by `evOwner`, the per-channel-side producer/consumer
index the numbering layer assigns). So the induction over `weaveGo`
carries no ghost remainder list — only `goEvents`, the fuel-indexed
ghost twin of the interpreter that names the events the worklist will
emit, kept in lockstep fuel for fuel. The pump traces keep their
remainders in `MState.rem` exactly as the merge does, and the pump
preservation step is `scan_step` re-consumed verbatim.

# Where the hard content is NOT

Everything here is generic structure: `wEmit` appends
unconditionally, so preservation never asks whether a guard is open.
The two protocol-content obligations live in later layers:

- the INITIAL ALIGNMENT (`weaveState_wcount`'s hypotheses): the
  opening worklist's per-owner filters are exactly the manual traces
  — the recursion emits each trace's events in trace order;
- ENABLEDNESS at the manual emission points (the E1/E2 windows the
  eventdag tool checks executably at every weave position), where
  `Skel.schedulable` enters via the pump-progress lemmas.

With the alignment in hand, the final `WCount` pins the weave's event
multiset to the traces' (`out_count` excludes duplicates and
inventions) and embeds every manual trace in order
(`wcount_done_man_sublist`).
-/
import StreamingMirror.Proofs.Sched.Weave
import StreamingMirror.Proofs.Sched.Numbering

namespace StreamingMirror.Sched

open Model

variable (sk : Skel)

-- ================================================= ownership plumbing

/-- Manual-trace count: iopen, ropen, and the `rootH` walks — the
prefix of `procs` the weave emits by hand; the traces past it are the
pumps. -/
def manCount : Nat := 2 + sk.rootH

/-- The trace an event belongs to, as an index into `procs`: its
channel-side's unique producer (sends) or consumer (receives), per
the numbering layer's ownership functions. -/
def evOwner (e : Ev) : Nat :=
  if e.2.1 then sndOwner sk e.1 else rcvOwner sk e.1

/-- The weave's pump traces are exactly the merge's non-manual
traces: `procs` past the openers and walks. -/
theorem weavePumps_eq : weavePumps sk = (procs sk).drop (manCount sk) := by
  have hsplit : procs sk
      = ([iopenEvents sk, ropenEvents sk]
          ++ ((List.range sk.rootH).map fun i =>
            ((if (sk.rootH - 1 - i) % 2 == 1 then Party.I else Party.R,
              sk.rootH - 1 - i) : Party × Nat)).map (walkEvents sk))
        ++ weavePumps sk := by
    simp [procs, weavePumps, List.append_assoc]
  have hlen : ([iopenEvents sk, ropenEvents sk]
      ++ ((List.range sk.rootH).map fun i =>
        ((if (sk.rootH - 1 - i) % 2 == 1 then Party.I else Party.R,
          sk.rootH - 1 - i) : Party × Nat)).map (walkEvents sk)).length
      = manCount sk := by
    simp [manCount]
    omega
  rw [hsplit, ← hlen, List.drop_left]

/-- The encoder-order family's pump half is ALSO `weavePumps`: only
walk traces differ between the corners, and walks are manual. -/
theorem procsE_drop_pumps :
    (procsE sk).drop (manCount sk) = weavePumps sk := by
  have hsplit : procsE sk
      = ([iopenEvents sk, ropenEvents sk]
          ++ ((List.range sk.rootH).map fun i =>
            ((if (sk.rootH - 1 - i) % 2 == 1 then Party.I else Party.R,
              sk.rootH - 1 - i) : Party × Nat)).map (walkEventsE sk))
        ++ weavePumps sk := by
    simp [procsE, weavePumps, List.append_assoc]
  have hlen : ([iopenEvents sk, ropenEvents sk]
      ++ ((List.range sk.rootH).map fun i =>
        ((if (sk.rootH - 1 - i) % 2 == 1 then Party.I else Party.R,
          sk.rootH - 1 - i) : Party × Nat)).map (walkEventsE sk)).length
      = manCount sk := by
    simp [manCount]
    omega
  rw [hsplit, ← hlen, List.drop_left]

/-- The events a worklist will emit by hand, in order: the ghost twin
of `weaveGo` — same fuel, same expansion, no state — so the counting
induction can walk the two in lockstep. -/
def goEvents : Nat → List WOp → List Ev
  | 0, _ => []
  | _ + 1, [] => []
  | fuel + 1, op :: rest =>
      match op with
      | .emit e => e :: goEvents fuel rest
      | .scope h k feed => goEvents fuel (wScopeOps sk h k feed ++ rest)
      | .kid h k s lastD kidBase i feed =>
          goEvents fuel (wKidOps sk h k s lastD kidBase i feed ++ rest)

/-- Each manual trace's future events, recovered from the worklist by
ownership: filtering the future emissions to owner `m` names manual
trace `m`'s unemitted suffix — that recovery is `WCount.man_struct`. -/
def manFilters (fut : List Ev) : List (List Ev) :=
  (List.range (manCount sk)).map fun m =>
    fut.filter fun e => evOwner sk e == m

/-- `range n` splits around any member: the prefix below it, the
member, and a suffix of strictly larger indices. -/
private theorem range_split (n m : Nat) (hm : m < n) :
    ∃ post : List Nat, List.range n = List.range m ++ m :: post
      ∧ ∀ x ∈ post, m < x := by
  induction n with
  | zero => omega
  | succ n ih =>
      by_cases h : m = n
      · subst h
        exact ⟨[], by rw [List.range_succ], fun x hx => by cases hx⟩
      · obtain ⟨post, hpost, hgt⟩ := ih (by omega)
        refine ⟨post ++ [n], ?_, ?_⟩
        · rw [List.range_succ, hpost]
          simp [List.append_assoc]
        · intro x hx
          rcases List.mem_append.1 hx with hx | hx
          · exact hgt x hx
          · rcases hx with _ | ⟨_, hx⟩
            · omega
            · cases hx


/-- Consuming the head future advances exactly its owner's filter:
the filter family splits as flanks that ignore `e` plus the owner's
cell, which sheds its leading `e`. -/
theorem manFilters_cons {e : Ev} (fut : List Ev)
    (hm : evOwner sk e < manCount sk) :
    ∃ A r B,
      manFilters sk (e :: fut) = A ++ (e :: r) :: B
        ∧ manFilters sk fut = A ++ r :: B := by
  obtain ⟨post, hsplit, hgt⟩ := range_split (manCount sk) (evOwner sk e) hm
  have hflank : ∀ m : Nat, evOwner sk e ≠ m →
      (e :: fut).filter (fun x => evOwner sk x == m)
        = fut.filter fun x => evOwner sk x == m := by
    intro m hne
    rw [List.filter_cons]
    have hb : (evOwner sk e == m) = false := by
      simp only [beq_eq_false_iff_ne, ne_eq]
      exact hne
    simp [hb]
  refine ⟨(List.range (evOwner sk e)).map
      (fun m => fut.filter fun x => evOwner sk x == m),
    fut.filter (fun x => evOwner sk x == evOwner sk e),
    post.map (fun m => fut.filter fun x => evOwner sk x == m),
    ?_, ?_⟩
  · unfold manFilters
    rw [hsplit, List.map_append, List.map_cons]
    congr 1
    · refine List.map_congr_left fun m hmem => ?_
      have hlt := List.mem_range.1 hmem
      exact hflank m (by omega)
    · congr 1
      · rw [List.filter_cons]
        simp
      · refine List.map_congr_left fun m hmem => ?_
        have hlt := hgt m hmem
        exact hflank m (by omega)
  · unfold manFilters
    rw [hsplit, List.map_append, List.map_cons]

/-- Emptied futures filter to nothing: every cell of
`manFilters sk []` is `[]`. -/
theorem manFilters_nil_mem {r : List Ev} (hr : r ∈ manFilters sk []) :
    r = [] := by
  obtain ⟨m, -, rfl⟩ := List.mem_map.1 hr
  rfl

-- ============================================== emittedCount algebra

/-- `emittedCount` distributes over length-synced appends. -/
theorem emittedCount_append (p : Ev → Bool) :
    ∀ {ts₁ rs₁ : List (List Ev)} (ts₂ rs₂ : List (List Ev)),
      ts₁.length = rs₁.length →
      emittedCount p (ts₁ ++ ts₂) (rs₁ ++ rs₂)
        = emittedCount p ts₁ rs₁ + emittedCount p ts₂ rs₂
  | [], [], _, _, _ => by simp [emittedCount]
  | [], _ :: _, _, _, h => by simp at h
  | _ :: _, [], _, _, h => by simp at h
  | t :: ts₁, r :: rs₁, ts₂, rs₂, h => by
      simp only [List.cons_append, emittedCount]
      rw [emittedCount_append p ts₂ rs₂ (by simpa using h)]
      omega

-- ======================================================= the invariant

/-- The weave counting invariant: `MInv` for weave states, with the
manual remainders recovered from the worklist futures by ownership.

`man_struct`/`pump_struct` are trace monotonicity split at
`manCount`: each trace is its emitted prefix (an in-order subsequence
of `out`) plus its remaining suffix — the owner filter of `fut` for
manuals, the racked `MState.rem` for pumps. `out_count` is
provenance under every predicate, exactly as in `MInv`; without it a
padded `out` satisfies every other field. `owners_lt` keeps every
future manual-owned, so consuming one always advances exactly one
remainder and the counts stay balanced. Deliberately ABSENT: the
`e1_hist`/`e2_hist` edge-respect fields — those need enabledness at
the manual emission points, which is the next layer's content. -/
structure WCountP (P : List (List Ev)) (fut : List Ev) (st : MState) :
    Prop where
  owners_lt : ∀ e ∈ fut, evOwner sk e < manCount sk
  man_struct : Forall2
    (fun t r => ∃ pre, t = pre ++ r ∧ pre.Sublist st.out)
    (P.take (manCount sk)) (manFilters sk fut)
  pump_struct : Forall2
    (fun t r => ∃ pre, t = pre ++ r ∧ pre.Sublist st.out)
    (P.drop (manCount sk)) st.rem
  sent_eq : ∀ c, st.sent c = sndCount c st.out
  rcvd_eq : ∀ c, st.rcvd c = rcvCount c st.out
  out_count : ∀ p : Ev → Bool,
    (st.out.filter p).length
      = emittedCount p (P.take (manCount sk)) (manFilters sk fut)
        + emittedCount p (P.drop (manCount sk)) st.rem

/-- The d5 corner's instance: the invariant at the merge's own trace
family. Every preservation lemma below is generic over the family
(`WCountP`); this abbreviation keeps the d5 spelling and lets the
`.impl` campaign instantiate the same layer at `procsE`. -/
abbrev WCount (fut : List Ev) (st : MState) : Prop :=
  WCountP sk (procs sk) fut st

/-- The weave's starting state satisfies the counting invariant,
given the initial alignment: the worklist's per-owner filters are
exactly the manual traces (so nothing is emitted and every remainder
is whole). -/
theorem wcount_init {P : List (List Ev)} {fut : List Ev}
    (halign : manFilters sk fut = P.take (manCount sk))
    (hpumps : P.drop (manCount sk) = weavePumps sk)
    (howners : ∀ e ∈ fut, evOwner sk e < manCount sk) :
    WCountP sk P fut (weaveInit sk) := by
  refine ⟨howners, ?_, ?_, fun c => rfl, fun c => rfl, ?_⟩
  · rw [halign]
    exact Forall2.self fun t _ => ⟨[], rfl, List.nil_sublist _⟩
  · show Forall2 _ _ (weavePumps sk)
    rw [← hpumps]
    exact Forall2.self fun t _ => ⟨[], rfl, List.nil_sublist _⟩
  · intro p
    show (0 : Nat) = _ + emittedCount p _ (weavePumps sk)
    rw [halign, ← hpumps, emittedCount_refl, emittedCount_refl]

-- ================================================ manual-emit shape

/-- `wEmit` appends the event to the output, whatever its side. -/
theorem wEmit_out (st : MState) (e : Ev) :
    (wEmit st e).out = st.out ++ [e] := by
  obtain ⟨c, b, n⟩ := e
  cases b <;> rfl

/-- `wEmit` leaves the pump remainders alone. -/
theorem wEmit_rem (st : MState) (e : Ev) :
    (wEmit st e).rem = st.rem := by
  obtain ⟨c, b, n⟩ := e
  cases b <;> rfl

-- =============================================== preservation lemmas

/-- Emitting the worklist's next future preserves the counting
invariant: its owner's remainder advances by exactly that event;
every other manual remainder, and every pump trace, is untouched. -/
theorem wEmit_preserves {P : List (List Ev)} {fut : List Ev}
    {st : MState} {e : Ev}
    (hinv : WCountP sk P (e :: fut) st) :
    WCountP sk P fut (wEmit st e) := by
  have hm : evOwner sk e < manCount sk :=
    hinv.owners_lt e (List.mem_cons_self ..)
  obtain ⟨A, r, B, hAe, hA⟩ := manFilters_cons sk fut hm
  have hsplit := hinv.man_struct
  rw [hAe] at hsplit
  obtain ⟨ts₁, t, ts₂, hts, hlen₁, h₁, ⟨pre, hpre, hpresub⟩, h₂⟩ :=
    Forall2.append_cons_right hsplit
  have hext : ∀ {a b : List Ev},
      (∃ pre, a = pre ++ b ∧ pre.Sublist st.out) →
      ∃ pre, a = pre ++ b ∧ pre.Sublist (st.out ++ [e]) :=
    fun ⟨pre', hp, hs⟩ =>
      ⟨pre', hp, hs.trans (List.sublist_append_left ..)⟩
  refine ⟨fun e' he' => hinv.owners_lt e' (List.mem_cons_of_mem _ he'),
    ?_, ?_, ?_, ?_, ?_⟩
  · -- man_struct: reassemble around the advanced owner cell
    rw [hA, hts, wEmit_out]
    refine Forall2.append (h₁.imp fun _ _ h => hext h)
      (.cons ⟨pre ++ [e], by rw [hpre]; simp, ?_⟩
        (h₂.imp fun _ _ h => hext h))
    exact hpresub.append (List.Sublist.refl [e])
  · rw [wEmit_out, wEmit_rem]
    exact hinv.pump_struct.imp fun _ _ h => hext h
  · -- sent_eq, by the emitted side
    obtain ⟨c, b, n⟩ := e
    cases b with
    | false =>
        intro c'
        show st.sent c' = sndCount c' (st.out ++ [(c, false, n)])
        rw [sndCount_append_rcv]
        exact hinv.sent_eq c'
    | true =>
        intro c'
        show (if c' = c then st.sent c + 1 else st.sent c')
          = sndCount c' (st.out ++ [(c, true, n)])
        rw [sndCount_append_snd]
        by_cases h : c' = c <;> simp [h, hinv.sent_eq]
  · -- rcvd_eq, by the emitted side
    obtain ⟨c, b, n⟩ := e
    cases b with
    | false =>
        intro c'
        show (if c' = c then st.rcvd c + 1 else st.rcvd c')
          = rcvCount c' (st.out ++ [(c, false, n)])
        rw [rcvCount_append_rcv]
        by_cases h : c' = c <;> simp [h, hinv.rcvd_eq]
    | true =>
        intro c'
        show st.rcvd c' = rcvCount c' (st.out ++ [(c, true, n)])
        rw [rcvCount_append_snd]
        exact hinv.rcvd_eq c'
  · -- out_count: the owner's emitted prefix grows by exactly `e`
    intro p
    have hold := hinv.out_count p
    rw [hAe, hts, emittedCount_append p _ _ hlen₁] at hold
    simp only [emittedCount] at hold
    have hpre_old : t.take (t.length - (e :: r).length) = pre := by
      rw [hpre]
      have hl : (pre ++ e :: r).length - (e :: r).length
          = pre.length := by simp
      rw [hl, take_len_append]
    rw [hpre_old] at hold
    rw [wEmit_out, wEmit_rem, hA, hts, emittedCount_append p _ _ hlen₁]
    simp only [emittedCount]
    have hpre_new : t.take (t.length - r.length) = pre ++ [e] := by
      rw [hpre]
      have hl : (pre ++ e :: r).length - r.length = pre.length + 1 := by
        simp only [List.length_append, List.length_cons]
        omega
      rw [hl, take_append_succ]
    rw [hpre_new]
    have hLHS : ((st.out ++ [e]).filter p).length
        = (st.out.filter p).length + (if p e then 1 else 0) := by
      rw [List.filter_append, List.length_append]
      cases hpe : p e <;> simp [hpe]
    have hmid : ((pre ++ [e]).filter p).length
        = (pre.filter p).length + (if p e then 1 else 0) := by
      rw [List.filter_append, List.length_append]
      cases hpe : p e <;> simp [hpe]
    omega

/-- One pump step preserves the counting invariant: the merge only
touches the pump remainders, and `scan_step` accounts the emitted
event against them; the manual side just extends its prefix
sublists. -/
theorem wStep_preserves {P : List (List Ev)} {fut : List Ev}
    {st st' : MState}
    (hinv : WCountP sk P fut st) (hstep : step sk st = some st') :
    WCountP sk P fut st' := by
  unfold step at hstep
  cases hscan : scan sk st.sent st.rcvd st.rem with
  | none => rw [hscan] at hstep; simp at hstep
  | some pr =>
    obtain ⟨e, rem'⟩ := pr
    rw [hscan] at hstep
    simp only [Option.map] at hstep
    obtain ⟨-, hrs', hcnt⟩ := scan_step sk st.out st.sent st.rcvd
      hinv.pump_struct hscan
    have hman : Forall2
        (fun t r => ∃ pre, t = pre ++ r ∧ pre.Sublist (st.out ++ [e]))
        (P.take (manCount sk)) (manFilters sk fut) :=
      hinv.man_struct.imp fun _ _ ⟨pre, hp, hs⟩ =>
        ⟨pre, hp, hs.trans (List.sublist_append_left ..)⟩
    obtain ⟨c, sd, n⟩ := e
    have hone : ∀ p : Ev → Bool,
        (List.filter p [((c, sd, n) : Ev)]).length
          = if p (c, sd, n) then 1 else 0 := by
      intro p
      cases hpe : p (c, sd, n) <;> simp [hpe]
    cases sd with
    | true =>
        cases hstep
        refine ⟨hinv.owners_lt, hman, hrs', ?_, ?_, ?_⟩
        · intro c'
          rw [sndCount_append_snd]
          by_cases h : c' = c <;> simp [h, hinv.sent_eq]
        · intro c'
          rw [rcvCount_append_snd]
          exact hinv.rcvd_eq c'
        · intro p
          have hc := hcnt p
          have hold := hinv.out_count p
          have h1 := hone p
          show ((st.out ++ [((c, true, n) : Ev)]).filter p).length
              = emittedCount p (P.take (manCount sk))
                  (manFilters sk fut)
                + emittedCount p (P.drop (manCount sk)) rem'
          rw [List.filter_append, List.length_append]
          omega
    | false =>
        cases hstep
        refine ⟨hinv.owners_lt, hman, hrs', ?_, ?_, ?_⟩
        · intro c'
          rw [sndCount_append_rcv]
          exact hinv.sent_eq c'
        · intro c'
          rw [rcvCount_append_rcv]
          by_cases h : c' = c <;> simp [h, hinv.rcvd_eq]
        · intro p
          have hc := hcnt p
          have hold := hinv.out_count p
          have h1 := hone p
          show ((st.out ++ [((c, false, n) : Ev)]).filter p).length
              = emittedCount p (P.take (manCount sk))
                  (manFilters sk fut)
                + emittedCount p (P.drop (manCount sk)) rem'
          rw [List.filter_append, List.length_append]
          omega

/-- The counting invariant survives any amount of pump fuel. -/
theorem wMergeN_preserves {P : List (List Ev)} {fut : List Ev}
    (fuel : Nat) {st : MState}
    (hinv : WCountP sk P fut st) :
    WCountP sk P fut (mergeN sk fuel st) := by
  induction fuel generalizing st with
  | zero => exact hinv
  | succ f ih =>
      unfold mergeN
      cases hstep : step sk st with
      | some st' => exact ih (wStep_preserves sk hinv hstep)
      | none => exact hinv

/-- The counting invariant survives the greedy pump. -/
theorem wPump_preserves {P : List (List Ev)} {fut : List Ev}
    {st : MState}
    (hinv : WCountP sk P fut st) : WCountP sk P fut (wPump sk st) :=
  wMergeN_preserves sk _ hinv

/-- Emit-then-pump consumes exactly the worklist's next future. -/
theorem wEmitP_preserves {P : List (List Ev)} {fut : List Ev}
    {st : MState} {e : Ev}
    (hinv : WCountP sk P (e :: fut) st) :
    WCountP sk P fut (wEmitP sk st e) :=
  wPump_preserves sk (wEmit_preserves sk hinv)

-- ================================================ the master induction

/-- The counting invariant rides the interpreter: the ghost futures
shrink in lockstep with the worklist — an emit consumes its head, an
expansion rewrites both sides identically — so the fuel's end leaves
no futures at all. -/
theorem weaveGo_preserves {P : List (List Ev)} (fuel : Nat) :
    ∀ (ops : List WOp) (st : MState),
      WCountP sk P (goEvents sk fuel ops) st →
      WCountP sk P [] (weaveGo sk fuel ops st) := by
  induction fuel with
  | zero => intro ops st h; exact h
  | succ f ih =>
      intro ops st h
      match ops with
      | [] => exact h
      | .emit e :: rest =>
          exact ih rest _ (wEmitP_preserves sk h)
      | .scope h' k feed :: rest =>
          exact ih _ st h
      | .kid h' k s lastD kidBase i feed :: rest =>
          exact ih _ st h

/-- The weave's final state carries the counting invariant with no
futures left, GIVEN the initial alignment — the per-owner filters of
the opening worklist's future emissions are exactly the manual
traces, every future manual-owned. Those two hypotheses are the next
obligation (PROGRESS.md §7 3b); everything behind them is closed. -/
theorem weaveState_wcount
    (halign : manFilters sk (goEvents sk (weaveFuel sk) (weaveOps sk))
      = (procs sk).take (manCount sk))
    (howners : ∀ e ∈ goEvents sk (weaveFuel sk) (weaveOps sk),
      evOwner sk e < manCount sk) :
    WCount sk [] (weaveState sk) :=
  wPump_preserves sk
    (weaveGo_preserves sk _ _ _
      (wcount_init sk halign (weavePumps_eq sk).symm howners))

-- ======================================== corollaries of a drained run

/-- With no futures left, every manual trace embeds in the output in
order: its remainder filter is empty, so the trace IS its emitted
prefix. -/
theorem wcount_done_man_sublist {P : List (List Ev)} {st : MState}
    (h : WCountP sk P [] st) :
    ∀ t ∈ P.take (manCount sk), t.Sublist st.out := by
  intro t ht
  obtain ⟨r, hr, pre, hpre, hsub⟩ :=
    h.man_struct.exists_of_mem_left ht
  rw [manFilters_nil_mem sk hr] at hpre
  rw [hpre, List.append_nil]
  exact hsub

end StreamingMirror.Sched
