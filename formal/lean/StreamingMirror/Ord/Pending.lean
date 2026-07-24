/-
The O decode layer: `PendingE.lean`'s per-process decodes re-targeted
at the ord-parameterized traces (`procsO`/`scheduleO`) under
`AxMode.impl`, for every assignment of the two-point dequeue-order
class.

# What moves and what does not

Sends never move in the class, so the committed-arm ledger content and
every send-side seq fact are the E layer's, consumed verbatim
(`mem_scopeSendsE`, `phase2_child_factsE`, `chunks_prefix_performedE`,
`wireCount_ge_succE`, `walk_scope_boundE`, `walk_ledgers_emptyE`,
`counts_of_emptyE` — none of their statements mention a trace or a
prologue position, so they need no twins). What moves is WHICH
prologue receive a phase pends: a phase-0 walk awaits its assignment's
FIRST receive (`wireIn` reply-first, `askedIn` query-first), phase 1
the second, and the absorber dispatches likewise — `wkPendO`/`abPendO`
carry the dispatch, and the walk/absorb decodes place the pend in
`walkEventsO`/`absorbEventsO`.

# The O performedness denomination

"Performed" must count what the O dynamics actually consumed:
mid-prologue, a query-first loop holds its query received and its wire
still pending, which is exactly `recvdOfO`'s per-assignment selection
between the two base formulas — the base `recvdOf` mis-counts BOTH
prologue channels of a query-first loop at phase 1. So every decode
here concludes `performedO`/`PendOkO` (`recvdOfO`-denominated receive
seqs, `applyO`-enabled fires). On every channel without a pairing loop
the O count IS the base count, which is how the placement-independent
E decodes (openers, finishes, assemblers, the floating return)
transfer by bridge (`performedO_of_performed`/`pendOkO_of_E`) rather
than by re-proof.

# The completeness seam

τ-comparison rides `scheduleO` through an EXPLICIT merge-completeness
hypothesis `hrem : ((finalStateO sk ord).rem.all List.isEmpty) = true`
— the O merge completeness lands in a later unit, and the O endgame
discharges `hrem` there. It is threaded exactly where the E layer
consumed `merge_completeE`: `trace_sublistO` and `tau_le_of_pendO`.
Nothing else needs it — the remaining glue consumes only the generic
merge facts (`scheduleO_count`, `scheduleO_proj_canon`).

Chain (ord, stage E): the decode; consumed by the O endgame. Base
mirror: Proofs/PendingE.lean. Map: PROGRESS.md §13.
-/
import StreamingMirror.Proofs.PendingE
import StreamingMirror.Ord.Numbering
import StreamingMirror.Ord.Wiring

namespace StreamingMirror.Ord

open Model
open Sched

variable (sk : Skel) (ord : OrdMap)

-- ======================================= O performedness and PendOkO

/-- Event `e` has already happened at state `s` under the assignment:
its seq is below the state's O-derived count on its channel-side (cf.
`Sched.performed` — the producer side is order-blind, the consumer
side reads `recvdOfO`, which is what the O dynamics actually
consume). -/
def performedO (s : State) (e : Ev) : Prop :=
  if e.2.1 = true then e.2.2 < sentOf sk s e.1
  else e.2.2 < recvdOfO sk ord s e.1

/-- The pending event's global obligations under the assignment (cf.
`Sched.PendOkE`): its channel is a real flow channel, its seq is the
channel's CURRENT O count (so it is the first O-unperformed event of
its channel-side), its action is enumerated, and the action is
`applyO`-enabled as soon as the channel guard opens (room for a send,
data for a receive). -/
structure PendOkO (s : State) (f : Ev) (a : Action) : Prop where
  chan_mem : f.1 ∈ allChans sk
  seq : f.2.2 = (if f.2.1 = true then sentOf sk s f.1
    else recvdOfO sk ord s f.1)
  act : a ∈ allActions sk
  fire : (if f.2.1 = true then s.chan f.1 < sk.cap f.1
      else 0 < s.chan f.1)
    → (applyO sk .impl ord a s).isSome = true

-- ============================== the order-blind channel bridges
-- `recvdOfO` differs from `recvdOf` only on the four loop-consumed
-- channel families (walk wires, walk queries, the leaf wire, the leaf
-- requests). Everywhere else the O count IS the base count, and the E
-- decodes transfer by rewrite.

/-- The floating root return's O count is the base count. -/
theorem recvdOfO_rootret (s : State) :
    recvdOfO sk ord s Chan.rootret = recvdOf sk s Chan.rootret := rfl

/-- The root-returns channel's O count is the base count. -/
theorem recvdOfO_rootrets (s : State) :
    recvdOfO sk ord s Chan.rootrets = recvdOf sk s Chan.rootrets := rfl

/-- The root-resolution channel's O count is the base count. -/
theorem recvdOfO_rootres (s : State) :
    recvdOfO sk ord s Chan.rootres = recvdOf sk s Chan.rootres := rfl

/-- The opening wire's O count is the base count: the responder
opener has no pairing loop. -/
theorem recvdOfO_wire_I_root (s : State) :
    recvdOfO sk ord s (Chan.wire Party.I sk.rootH)
      = recvdOf sk s (Chan.wire Party.I sk.rootH) := by
  simp [recvdOfO, recvdOf]

/-- An assembler's resolution channel reads the base count: assemblers
have no pairing loop, and `upper`/`lower` are delegate arms of
`recvdOfO`. -/
theorem recvdOfO_asmResChan (s : State) (pk : Party × Nat) :
    recvdOfO sk ord s (asmResChan pk) = recvdOf sk s (asmResChan pk) := by
  unfold asmResChan
  split <;> rfl

/-- An assembler's level channel reads the base count. -/
theorem recvdOfO_asmLevelChan (s : State) (pk : Party × Nat) :
    recvdOfO sk ord s (asmLevelChan pk)
      = recvdOf sk s (asmLevelChan pk) := rfl

/-- Transfer a base-`performed` fact to `performedO`: the producer
side is order-blind, and a receive transfers whenever its channel's O
count is the base count. -/
theorem performedO_of_performed {s : State} {e : Ev}
    (hc : e.2.1 = true ∨ recvdOfO sk ord s e.1 = recvdOf sk s e.1)
    (h : performed sk s e) : performedO sk ord s e := by
  unfold performedO
  unfold performed at h
  rcases hc with hb | hcr
  · rwa [if_pos hb] at h ⊢
  · by_cases hb : e.2.1 = true
    · rwa [if_pos hb] at h ⊢
    · rw [if_neg hb] at h ⊢
      rwa [hcr]

/-- Transfer `PendOkE` to `PendOkO`: same channel and enumeration
obligations, the seq re-read through the O counts, the fire through
the action's O arm. Callers supply the two agreements — definitional
for every action without a pairing loop. -/
theorem pendOkO_of_E {s : State} {f : Ev} {a : Action}
    (h : PendOkE sk s f a)
    (happ : applyO sk .impl ord a s = Model.apply sk .impl a s)
    (hc : f.2.1 = true ∨ recvdOfO sk ord s f.1 = recvdOf sk s f.1) :
    PendOkO sk ord s f a := by
  refine ⟨h.chan_mem, ?_, h.act, fun hg => by rw [happ]; exact h.fire hg⟩
  have hs := h.seq
  rcases hc with hb | hcr
  · rwa [if_pos hb] at hs ⊢
  · by_cases hb : f.2.1 = true
    · rwa [if_pos hb] at hs ⊢
    · rw [if_neg hb] at hs ⊢
      rwa [hcr]

-- ================================================= scheduleO-side glue

/-- O merge completeness (the EXPLICIT hypothesis `hrem` — discharged
downstream by the O merge-completeness unit, at the endgame), read
back through trace monotonicity: every O trace embeds in `scheduleO`
in order. This is what makes position-in-`scheduleO` a total order
along each trace. -/
theorem trace_sublistO
    (hrem : ((finalStateO sk ord).rem.all List.isEmpty) = true)
    {T : List Ev} (hT : T ∈ procsO sk ord) :
    T.Sublist (scheduleO sk ord) := by
  obtain ⟨r, hr, pre, hpre, hsub⟩ :=
    (trace_monotoneO sk ord).exists_of_mem_left hT
  have hempty : r = [] := by
    have := List.all_eq_true.1 hrem r hr
    cases r with
    | nil => rfl
    | cons a l => simp at this
  rw [hempty, List.append_nil] at hpre
  exact hpre ▸ hsub

/-- τ injectivity in counting form: `scheduleO` holds each event at
most once (its per-channel projections are canonical at every
assignment). No completeness needed. -/
theorem scheduleO_count_le_oneO (hwf : sk.wellFormed = true) (e : Ev) :
    (scheduleO sk ord).count e ≤ 1 := by
  obtain ⟨c, b, n⟩ := e
  obtain ⟨m, hm⟩ := scheduleO_proj_canon sk ord hwf c b
  have hfilter : (scheduleO sk ord).count (c, b, n)
      = (proj c b (scheduleO sk ord)).count (c, b, n) := by
    unfold proj
    exact (List.count_filter (by simp)).symm
  rw [hfilter, hm, count_canon]
  split <;> omega

/-- Provenance: every `scheduleO` event was emitted by some O trace.
No completeness needed. -/
theorem sched_mem_traceO {e : Ev} (he : e ∈ scheduleO sk ord) :
    ∃ T ∈ procsO sk ord, e ∈ T := by
  have hpos : 1 ≤ emittedCount (fun x => x == e) (procsO sk ord)
      (finalStateO sk ord).rem := by
    rw [← scheduleO_count sk ord (fun x => x == e)]
    have hm : e ∈ (scheduleO sk ord).filter (fun x => x == e) :=
      List.mem_filter.2 ⟨he, by simp⟩
    have := List.length_pos_of_mem hm
    omega
  obtain ⟨T, hT, e', he', hbeq⟩ := emittedCount_pos hpos
  have : e' = e := by simpa using hbeq
  exact ⟨T, hT, this ▸ he'⟩

/-- A pending event is never O-performed: its seq IS the O count. -/
theorem pend_not_performedO {s : State} {f : Ev} {a : Action}
    (h : PendOkO sk ord s f a) : ¬ performedO sk ord s f := by
  have hseq := h.seq
  unfold performedO
  cases hb : f.2.1 with
  | true =>
      rw [hb] at hseq
      rw [if_pos rfl] at hseq ⊢
      omega
  | false =>
      rw [hb] at hseq
      rw [if_neg (by simp)] at hseq ⊢
      omega

/-- The τ-comparison: an O-unperformed event of a trace sits at or
after the trace's pending split, so the pending head is at or before
it in `scheduleO`. `hrem` is the O merge completeness, discharged
downstream by the endgame. -/
theorem tau_le_of_pendO (hwf : sk.wellFormed = true)
    (hrem : ((finalStateO sk ord).rem.all List.isEmpty) = true)
    {s : State} {T pre suf : List Ev} {f : Ev}
    (hT : T ∈ procsO sk ord) (hdec : T = pre ++ f :: suf)
    (hpre : ∀ e ∈ pre, performedO sk ord s e)
    {g : Ev} (hg : g ∈ T) (hnp : ¬ performedO sk ord s g) :
    evIdx f (scheduleO sk ord) ≤ evIdx g (scheduleO sk ord) := by
  rw [hdec] at hg
  rcases List.mem_append.1 hg with hgpre | hgcons
  · exact absurd (hpre g hgpre) hnp
  · rcases List.mem_cons.1 hgcons with rfl | hgsuf
    · exact Nat.le_refl _
    · have hpair : ([f, g] : List Ev).Sublist T := by
        rw [hdec]
        refine List.Sublist.trans ?_ (List.sublist_append_right pre _)
        exact List.cons_sublist_cons.2 (List.singleton_sublist.2 hgsuf)
      have hsub : ([f, g] : List Ev).Sublist (scheduleO sk ord) :=
        hpair.trans (trace_sublistO sk ord hrem hT)
      exact Nat.le_of_lt
        (pos_lt_of_pair (scheduleO_count_le_oneO sk ord hwf) hsub)

-- ==================================== walk-trace micro-bricks

/-- Membership shape of an ordered prologue: one of the two receives,
whichever the assignment dequeues first. -/
theorem mem_prologueO {pk : Party × Nat} {k : Nat} {e : Ev}
    (he : e ∈ prologueO ord pk k) :
    e = (wireIn pk, false, k) ∨ e = (askedIn pk, false, k) := by
  unfold prologueO at he
  cases hord : ord.walk pk with
  | replyFirst =>
      simp only [hord] at he
      rcases List.mem_cons.1 he with rfl | he
      · exact Or.inl rfl
      rcases List.mem_cons.1 he with rfl | he
      · exact Or.inr rfl
      · cases he
  | queryFirst =>
      simp only [hord] at he
      rcases List.mem_cons.1 he with rfl | he
      · exact Or.inr rfl
      rcases List.mem_cons.1 he with rfl | he
      · exact Or.inl rfl
      · cases he

/-- Every encoder-order scope send is a send: the committed-arm
prefixes transfer to `performedO` through the producer-side bridge. -/
theorem scopeSendsE_snd {pk : Party × Nat} {k : Nat} {e : Ev}
    (he : e ∈ scopeSendsE sk pk k) : e.2.1 = true := by
  rcases mem_scopeSendsE sk he with rfl | ⟨i, hin, hchunk⟩
  · rfl
  · cases hD : sk.childIsD pk.2 (sk.stageScope pk.2 k) i with
    | true =>
        rw [chunkD sk pk k i hD] at hchunk
        rcases List.mem_cons.1 hchunk with rfl | hchunk
        · rfl
        rcases List.mem_cons.1 hchunk with rfl | hchunk
        · rfl
        · obtain ⟨cc, bb, nn⟩ := e
          obtain ⟨-, hb, -, -⟩ := mem_seg hchunk
          simpa using hb
    | false =>
        rw [chunkR sk pk k i hD] at hchunk
        rcases List.mem_cons.1 hchunk with rfl | hchunk
        · rfl
        · cases hchunk

/-- A reply-first walk's O scope block is the E block, literally. -/
theorem scopeBlockO_rf {pk : Party × Nat} (h : ord.walk pk = .replyFirst)
    (k : Nat) :
    scopeBlockO sk ord pk k
      = (wireIn pk, false, k) :: (askedIn pk, false, k)
        :: scopeSendsE sk pk k := by
  simp [scopeBlockO, prologueO, h]

/-- A query-first walk's O scope block: the two prologue receives
swap; the send suffix is shared. -/
theorem scopeBlockO_qf {pk : Party × Nat} (h : ord.walk pk = .queryFirst)
    (k : Nat) :
    scopeBlockO sk ord pk k
      = (askedIn pk, false, k) :: (wireIn pk, false, k)
        :: scopeSendsE sk pk k := by
  simp [scopeBlockO, prologueO, h]

-- ============================================== the walk pending decode

/-- The walk's pending event and action under the assignment, per
phase: the prologue receive it awaits — the assignment's FIRST receive
at phase 0, the second at phase 1 — or the committed obligation's fire
(fires are order-blind; sends do not move in the class). Empty exactly
at choice points (phase-2 uncommitted) and past the channel work
(phase ≥ 3). Cf. `Sched.wkPend`, whose phase-0/1 arms are the
reply-first instances. -/
def wkPendO (s : State) (pk : Party × Nat) : List (Ev × Action) :=
  let ws := s.walk pk
  if ws.phase = 0 then
    match ord.walk pk with
    | .replyFirst => [((wireIn pk, false, ws.scope), .walkRecvWire pk)]
    | .queryFirst => [((askedIn pk, false, ws.scope), .walkRecvAsked pk)]
  else if ws.phase = 1 then
    match ord.walk pk with
    | .replyFirst => [((askedIn pk, false, ws.scope), .walkRecvAsked pk)]
    | .queryFirst => [((wireIn pk, false, ws.scope), .walkRecvWire pk)]
  else if ws.phase = 2 then
    match ws.committed with
    | some (.wire i) =>
        [((wireOut pk, true, sk.wiresBefore pk.2 ws.scope + i),
          .walkFire pk)]
    | some (.res i) =>
        [((lowerOut pk, true,
            sk.dsBefore pk.2 ws.scope + dRank sk pk ws.scope i),
          .walkFire pk)]
    | some (.query i) =>
        [((askedOut pk, true,
            sk.qsBefore pk.2 ws.scope + qSum sk pk ws.scope i
              + ws.qSent i),
          .walkFire pk)]
    | some .parent => [((upperOut pk, true, ws.scope), .walkFire pk)]
    | none => []
  else []

/-- Every event of a completed-scope O block is O-performed: BOTH base
receive formulas dominate a completed scope's index, so whichever the
assignment selects does, and the send suffix rides the E lemma through
the producer-side bridge. -/
theorem scopeBlock_performedO (hwf : sk.wellFormed = true) {s : State}
    {pk : Party × Nat} (hi : InvL sk .impl s) (hpk : pk ∈ sk.walkKeys)
    {j : Nat} (hj : j < (s.walk pk).scope) (hjs : j < sk.stageLen pk.2) :
    ∀ e ∈ scopeBlockO sk ord pk j, performedO sk ord s e := by
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
  have hWRO : (s.walk pk).scope ≤ wkWireRecvdO sk ord s pk := by
    cases hord : ord.walk pk <;> simp only [wkWireRecvdO, hord] <;> omega
  have hARO : (s.walk pk).scope ≤ wkAskedRecvdO sk ord s pk := by
    cases hord : ord.walk pk <;> simp only [wkAskedRecvdO, hord] <;> omega
  intro e he
  unfold scopeBlockO at he
  rcases List.mem_append.1 he with hpro | hsend
  · rcases mem_prologueO ord hpro with rfl | rfl
    · show (j : Nat) < recvdOfO sk ord s (wireIn pk)
      rw [recvdOfO_wireIn hpk]
      omega
    · show (j : Nat) < recvdOfO sk ord s (askedIn pk)
      rw [recvdOfO_askedIn]
      omega
  · have heB : e ∈ scopeBlockE sk pk j := by
      unfold scopeBlockE
      exact List.mem_cons_of_mem _ (List.mem_cons_of_mem _ hsend)
    exact performedO_of_performed sk ord
      (Or.inl (scopeSendsE_snd sk hsend))
      (scopeBlock_performedE sk hwf hi hpk hj hjs e heB)

set_option maxHeartbeats 1000000 in
/-- The O committed-case split (a private twin of
`Sched.walk_committed_splitE`, which is not exported): the in-scope
prefix below the committed obligation's event is performed, and the
event carries the channel's current count. The ledger content is
order-blind — sends do not move — so the body is the E body verbatim;
only the pend list is the O one (its phase-2 arms coincide with
`wkPend`'s). The prefix stays `performed`-denominated: it is all
sends, and the consumer bridges. -/
private theorem walk_committed_splitO (hwf : sk.wellFormed = true)
    {s : State} {pk : Party × Nat} (hi : InvL sk .impl s)
    (hpk : pk ∈ sk.walkKeys) (hph2 : (s.walk pk).phase = 2)
    {o : Oblig} (hcm : (s.walk pk).committed = some o) :
    ∃ f isp ss,
      wkPendO sk ord s pk = [(f, .walkFire pk)]
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
      have hpend : wkPendO sk ord s pk = [((wireOut pk, true,
          sk.wiresBefore pk.2 (s.walk pk).scope + i), .walkFire pk)] := by
        simp [wkPendO, hph2, hcm]
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
      have hpend : wkPendO sk ord s pk = [((lowerOut pk, true,
          sk.dsBefore pk.2 (s.walk pk).scope
            + dRank sk pk (s.walk pk).scope i), .walkFire pk)] := by
        simp [wkPendO, hph2, hcm]
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
      have hpend : wkPendO sk ord s pk = [((askedOut pk, true,
          sk.qsBefore pk.2 (s.walk pk).scope
            + qSum sk pk (s.walk pk).scope i + (s.walk pk).qSent i),
          .walkFire pk)] := by
        simp [wkPendO, hph2, hcm]
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
      have hpend : wkPendO sk ord s pk
          = [((upperOut pk, true, (s.walk pk).scope), .walkFire pk)] := by
        simp [wkPendO, hph2, hcm]
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

/-- The O walk decode: past its channel work with everything
O-performed, or holding one pending event — the assignment's next
prologue receive, or the order-blind committed fire — with the
`walkEventsO` prefix below it O-performed. Choice points (phase-2
uncommitted) are excluded — the pillar owns them. -/
theorem walk_pend_or_doneO (hwf : sk.wellFormed = true) {s : State}
    {pk : Party × Nat} (hi : InvL sk .impl s) (hpk : pk ∈ sk.walkKeys)
    (hnc : ¬((s.walk pk).phase = 2 ∧ (s.walk pk).committed = none)) :
    ((∀ e ∈ walkEventsO sk ord pk, performedO sk ord s e)
        ∧ wkPendO sk ord s pk = [])
    ∨ ∃ f a pre suf, wkPendO sk ord s pk = [(f, a)]
        ∧ walkEventsO sk ord pk = pre ++ f :: suf
        ∧ (∀ e ∈ pre, performedO sk ord s e)
        ∧ PendOkO sk ord s f a := by
  by_cases hph3 : 3 ≤ (s.walk pk).phase
  · -- past the channel work: every block is a completed scope
    left
    constructor
    · intro e he
      obtain ⟨j, hjr, hje⟩ := List.mem_flatMap.1 he
      rw [List.mem_range] at hjr
      have hsc := (walk_scope_boundE sk hi hpk).2 hph3
      exact scopeBlock_performedO sk ord hwf hi hpk (by omega) hjr e hje
    · unfold wkPendO
      rw [if_neg (by omega), if_neg (by omega), if_neg (by omega)]
  · have hsc := (walk_scope_boundE sk hi hpk).1 (by omega)
    -- the shared outer split at the current scope
    have houter : walkEventsO sk ord pk
        = (List.range (s.walk pk).scope).flatMap (scopeBlockO sk ord pk)
          ++ scopeBlockO sk ord pk (s.walk pk).scope
          ++ (List.range' ((s.walk pk).scope + 1)
              (sk.stageLen pk.2 - (s.walk pk).scope - 1)).flatMap
              (scopeBlockO sk ord pk) := by
      unfold walkEventsO
      rw [range_split (show (s.walk pk).scope ≤ sk.stageLen pk.2
        by omega), List.flatMap_append]
      have hlen : sk.stageLen pk.2 - (s.walk pk).scope
          = (sk.stageLen pk.2 - (s.walk pk).scope - 1) + 1 := by omega
      rw [hlen, List.range'_succ, List.flatMap_cons]
      simp [List.append_assoc]
    have hprepre : ∀ e ∈ (List.range (s.walk pk).scope).flatMap
        (scopeBlockO sk ord pk), performedO sk ord s e := by
      intro e he
      obtain ⟨j, hjr, hje⟩ := List.mem_flatMap.1 he
      rw [List.mem_range] at hjr
      exact scopeBlock_performedO sk ord hwf hi hpk hjr (by omega) e hje
    rcases Nat.lt_or_ge (s.walk pk).phase 2 with hph01 | hph2'
    · -- a prologue receive is pending: the assignment picks which
      right
      rcases Nat.lt_or_ge (s.walk pk).phase 1 with hph0 | hph1
      · have hph : (s.walk pk).phase = 0 := by omega
        cases hord : ord.walk pk with
        | replyFirst =>
            refine ⟨(wireIn pk, false, (s.walk pk).scope), .walkRecvWire pk,
              (List.range (s.walk pk).scope).flatMap (scopeBlockO sk ord pk),
              ((askedIn pk, false, (s.walk pk).scope) ::
                scopeSendsE sk pk (s.walk pk).scope)
                ++ (List.range' ((s.walk pk).scope + 1)
                    (sk.stageLen pk.2 - (s.walk pk).scope - 1)).flatMap
                    (scopeBlockO sk ord pk),
              ?_, ?_, hprepre, ?_, ?_, ?_, ?_⟩
            · simp [wkPendO, hph, hord]
            · rw [houter, scopeBlockO_rf sk ord hord]
              simp [List.cons_append, List.append_assoc]
            · exact wireIn_mem_allChans sk hwf hpk
            · show (s.walk pk).scope = recvdOfO sk ord s (wireIn pk)
              rw [recvdOfO_wireIn hpk]
              simp only [wkWireRecvdO, hord]
              unfold wkWireRecvd
              rw [if_neg (by omega), hph]
              simp
            · exact walk_action_mem sk hpk (by simp)
            · intro hch
              simp only [Bool.false_eq_true, if_false] at hch
              have happ : (applyO sk .impl ord (.walkRecvWire pk) s).isSome
                  = true := by
                simp [applyO, hord, hpk, hph]
                omega
              exact happ
        | queryFirst =>
            refine ⟨(askedIn pk, false, (s.walk pk).scope),
              .walkRecvAsked pk,
              (List.range (s.walk pk).scope).flatMap (scopeBlockO sk ord pk),
              ((wireIn pk, false, (s.walk pk).scope) ::
                scopeSendsE sk pk (s.walk pk).scope)
                ++ (List.range' ((s.walk pk).scope + 1)
                    (sk.stageLen pk.2 - (s.walk pk).scope - 1)).flatMap
                    (scopeBlockO sk ord pk),
              ?_, ?_, hprepre, ?_, ?_, ?_, ?_⟩
            · simp [wkPendO, hph, hord]
            · rw [houter, scopeBlockO_qf sk ord hord]
              simp [List.cons_append, List.append_assoc]
            · exact (walk_chans_mem sk hpk).2.1
            · show (s.walk pk).scope = recvdOfO sk ord s (askedIn pk)
              rw [recvdOfO_askedIn]
              simp only [wkAskedRecvdO, hord]
              unfold wkWireRecvd
              rw [if_neg (by omega), hph]
              simp
            · exact walk_action_mem sk hpk (by simp)
            · intro hch
              simp only [Bool.false_eq_true, if_false] at hch
              have happ : (applyO sk .impl ord (.walkRecvAsked pk) s).isSome
                  = true := by
                simp [applyO, hord, hpk, hph]
                omega
              exact happ
      · have hph : (s.walk pk).phase = 1 := by omega
        cases hord : ord.walk pk with
        | replyFirst =>
            refine ⟨(askedIn pk, false, (s.walk pk).scope),
              .walkRecvAsked pk,
              (List.range (s.walk pk).scope).flatMap (scopeBlockO sk ord pk)
                ++ [(wireIn pk, false, (s.walk pk).scope)],
              scopeSendsE sk pk (s.walk pk).scope
                ++ (List.range' ((s.walk pk).scope + 1)
                    (sk.stageLen pk.2 - (s.walk pk).scope - 1)).flatMap
                    (scopeBlockO sk ord pk),
              ?_, ?_, ?_, ?_, ?_, ?_, ?_⟩
            · simp [wkPendO, hph, hord]
            · rw [houter, scopeBlockO_rf sk ord hord]
              simp [List.cons_append, List.append_assoc]
            · intro e he
              rcases List.mem_append.1 he with hp | hone
              · exact hprepre e hp
              · rw [List.mem_singleton] at hone
                subst hone
                show (s.walk pk).scope < recvdOfO sk ord s (wireIn pk)
                rw [recvdOfO_wireIn hpk]
                simp only [wkWireRecvdO, hord]
                unfold wkWireRecvd
                rw [if_neg (by omega), hph]
                simp
            · exact (walk_chans_mem sk hpk).2.1
            · show (s.walk pk).scope = recvdOfO sk ord s (askedIn pk)
              rw [recvdOfO_askedIn]
              simp only [wkAskedRecvdO, hord]
              unfold wkAskedRecvd
              rw [if_neg (by omega), hph]
              simp
            · exact walk_action_mem sk hpk (by simp)
            · intro hch
              simp only [Bool.false_eq_true, if_false] at hch
              have happ : (applyO sk .impl ord (.walkRecvAsked pk) s).isSome
                  = true := by
                simp [applyO, hord, hpk, hph]
                omega
              exact happ
        | queryFirst =>
            refine ⟨(wireIn pk, false, (s.walk pk).scope), .walkRecvWire pk,
              (List.range (s.walk pk).scope).flatMap (scopeBlockO sk ord pk)
                ++ [(askedIn pk, false, (s.walk pk).scope)],
              scopeSendsE sk pk (s.walk pk).scope
                ++ (List.range' ((s.walk pk).scope + 1)
                    (sk.stageLen pk.2 - (s.walk pk).scope - 1)).flatMap
                    (scopeBlockO sk ord pk),
              ?_, ?_, ?_, ?_, ?_, ?_, ?_⟩
            · simp [wkPendO, hph, hord]
            · rw [houter, scopeBlockO_qf sk ord hord]
              simp [List.cons_append, List.append_assoc]
            · intro e he
              rcases List.mem_append.1 he with hp | hone
              · exact hprepre e hp
              · rw [List.mem_singleton] at hone
                subst hone
                show (s.walk pk).scope < recvdOfO sk ord s (askedIn pk)
                rw [recvdOfO_askedIn]
                simp only [wkAskedRecvdO, hord]
                unfold wkWireRecvd
                rw [if_neg (by omega), hph]
                simp
            · exact wireIn_mem_allChans sk hwf hpk
            · show (s.walk pk).scope = recvdOfO sk ord s (wireIn pk)
              rw [recvdOfO_wireIn hpk]
              simp only [wkWireRecvdO, hord]
              unfold wkAskedRecvd
              rw [if_neg (by omega), hph]
              simp
            · exact walk_action_mem sk hpk (by simp)
            · intro hch
              simp only [Bool.false_eq_true, if_false] at hch
              have happ : (applyO sk .impl ord (.walkRecvWire pk) s).isSome
                  = true := by
                simp [applyO, hord, hpk, hph]
                omega
              exact happ
    · -- phase 2: committed (uncommitted is the pillar's case)
      have hph2 : (s.walk pk).phase = 2 := by omega
      cases hcm : (s.walk pk).committed with
      | none => exact absurd ⟨hph2, hcm⟩ hnc
      | some o =>
          right
          obtain ⟨f, isp, ss, hpend, hss, hisp, hchan, hside, hseq, hmem⟩ :=
            walk_committed_splitO sk ord hwf hi hpk hph2 hcm
          have hmid : scopeBlockO sk ord pk (s.walk pk).scope
              = prologueO ord pk (s.walk pk).scope ++ (isp ++ f :: ss) := by
            unfold scopeBlockO
            rw [hss]
          refine ⟨f, .walkFire pk,
            (List.range (s.walk pk).scope).flatMap (scopeBlockO sk ord pk)
              ++ (prologueO ord pk (s.walk pk).scope ++ isp),
            ss ++ (List.range' ((s.walk pk).scope + 1)
                (sk.stageLen pk.2 - (s.walk pk).scope - 1)).flatMap
                (scopeBlockO sk ord pk),
            hpend, ?_, ?_, hmem, ?_, ?_, ?_⟩
          · rw [houter, hmid]
            simp [List.cons_append, List.append_assoc]
          · intro e he
            rcases List.mem_append.1 he with hp | hcons
            · exact hprepre e hp
            rcases List.mem_append.1 hcons with hpro | hin
            · rcases mem_prologueO ord hpro with rfl | rfl
              · show (s.walk pk).scope < recvdOfO sk ord s (wireIn pk)
                rw [recvdOfO_wireIn hpk]
                cases hord : ord.walk pk with
                | replyFirst =>
                    simp only [wkWireRecvdO, hord]
                    unfold wkWireRecvd
                    rw [if_neg (by omega), hph2]
                    simp
                | queryFirst =>
                    simp only [wkWireRecvdO, hord]
                    unfold wkAskedRecvd
                    rw [if_neg (by omega), hph2]
                    simp
              · show (s.walk pk).scope < recvdOfO sk ord s (askedIn pk)
                rw [recvdOfO_askedIn]
                cases hord : ord.walk pk with
                | replyFirst =>
                    simp only [wkAskedRecvdO, hord]
                    unfold wkAskedRecvd
                    rw [if_neg (by omega), hph2]
                    simp
                | queryFirst =>
                    simp only [wkAskedRecvdO, hord]
                    unfold wkWireRecvd
                    rw [if_neg (by omega), hph2]
                    simp
            · have hsendm : e ∈ scopeSendsE sk pk (s.walk pk).scope := by
                rw [hss]
                exact List.mem_append_left _ hin
              exact performedO_of_performed sk ord
                (Or.inl (scopeSendsE_snd sk hsendm)) (hisp e hin)
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

-- ============================== the placement-independent decodes
-- iopen/ropen/rootret/fin/asm run no pairing loop: their traces are
-- rows of `procsO` shared verbatim with `procsE`, every receive sits
-- on an order-blind channel, and every pending action's O arm is the
-- base arm definitionally. The E decodes are consumed OUTRIGHT and
-- re-denominated through the two bridges.

/-- Every iopen event is a send. -/
private theorem iopen_ev_snd {e : Ev} (he : e ∈ iopenEvents sk) :
    e.2.1 = true := by
  unfold iopenEvents at he
  rcases List.mem_cons.1 he with rfl | he
  · rfl
  rcases List.mem_cons.1 he with rfl | he
  · rfl
  · cases he

/-- Every iopen pending action fires through the base arm. -/
private theorem ioPend_applyO {s : State} {f : Ev} {a : Action}
    (hp : ioPend sk s = [(f, a)]) :
    applyO sk .impl ord a s = Model.apply sk .impl a s := by
  unfold ioPend at hp
  split at hp
  · injection hp with h1 _
    injection h1 with _ ha
    subst ha
    rfl
  · injection hp with h1 _
    injection h1 with _ ha
    subst ha
    rfl
  · simp at hp

/-- The initiator opening decode, O-denominated: the E decode outright
— the trace is all sends. -/
theorem iopen_pend_or_doneO (hwf : sk.wellFormed = true) {s : State}
    (hi : InvL sk .impl s)
    (hch : s.iopenCh = none → doneIOpen s = true) :
    ((∀ e ∈ iopenEvents sk, performedO sk ord s e) ∧ ioPend sk s = [])
    ∨ ∃ f a pre suf, ioPend sk s = [(f, a)]
        ∧ iopenEvents sk = pre ++ f :: suf
        ∧ (∀ e ∈ pre, performedO sk ord s e)
        ∧ PendOkO sk ord s f a := by
  rcases iopen_pend_or_doneE sk hwf hi hch with ⟨hall, hp⟩
    | ⟨f, a, pre, suf, hp, hdec, hpre, hok⟩
  · exact Or.inl ⟨fun e he => performedO_of_performed sk ord
      (Or.inl (iopen_ev_snd sk he)) (hall e he), hp⟩
  · refine Or.inr ⟨f, a, pre, suf, hp, hdec, ?_, ?_⟩
    · intro e he
      refine performedO_of_performed sk ord
        (Or.inl (iopen_ev_snd sk ?_)) (hpre e he)
      rw [hdec]
      exact List.mem_append_left _ he
    · refine pendOkO_of_E sk ord hok (ioPend_applyO sk ord hp)
        (Or.inl (iopen_ev_snd sk ?_))
      rw [hdec]
      exact List.mem_append_right _ (List.mem_cons_self ..)

/-- The root-return trace's one receive reads the base count. -/
private theorem rootret_ev_blind {s : State} {e : Ev}
    (he : e ∈ [((Chan.rootret, false, 0) : Ev)]) :
    e.2.1 = true ∨ recvdOfO sk ord s e.1 = recvdOf sk s e.1 := by
  rw [List.mem_singleton] at he
  subst he
  exact Or.inr (recvdOfO_rootret sk ord _)

/-- The root-return pending action fires through the base arm. -/
private theorem rrPend_applyO {s : State} {f : Ev} {a : Action}
    (hp : rrPend s = [(f, a)]) :
    applyO sk .impl ord a s = Model.apply sk .impl a s := by
  unfold rrPend at hp
  split at hp
  · injection hp with h1 _
    injection h1 with _ ha
    subst ha
    rfl
  · simp at hp

/-- The floating root-return decode, O-denominated: the E decode
outright. -/
theorem rootret_pend_or_doneO {s : State} :
    ((∀ e ∈ [((Chan.rootret, false, 0) : Ev)], performedO sk ord s e)
      ∧ rrPend s = [])
    ∨ ∃ f a pre suf, rrPend s = [(f, a)]
        ∧ [((Chan.rootret, false, 0) : Ev)] = pre ++ f :: suf
        ∧ (∀ e ∈ pre, performedO sk ord s e)
        ∧ PendOkO sk ord s f a := by
  rcases rootret_pend_or_doneE sk (s := s) with ⟨hall, hp⟩
    | ⟨f, a, pre, suf, hp, hdec, hpre, hok⟩
  · exact Or.inl ⟨fun e he => performedO_of_performed sk ord
      (rootret_ev_blind sk ord he) (hall e he), hp⟩
  · refine Or.inr ⟨f, a, pre, suf, hp, hdec, ?_, ?_⟩
    · intro e he
      refine performedO_of_performed sk ord
        (rootret_ev_blind sk ord ?_) (hpre e he)
      rw [hdec]
      exact List.mem_append_left _ he
    · refine pendOkO_of_E sk ord hok (rrPend_applyO sk ord hp)
        (rootret_ev_blind sk ord ?_)
      rw [hdec]
      exact List.mem_append_right _ (List.mem_cons_self ..)

/-- Every fin-trace receive reads the base count. -/
private theorem fin_ev_blind {s : State} {e : Ev}
    (he : e ∈ finEvents sk) :
    e.2.1 = true ∨ recvdOfO sk ord s e.1 = recvdOf sk s e.1 := by
  unfold finEvents at he
  rcases List.mem_cons.1 he with rfl | he
  · exact Or.inr (recvdOfO_rootres sk ord _)
  · obtain ⟨j, -, rfl⟩ := List.mem_map.1 he
    exact Or.inr (recvdOfO_rootrets sk ord _)

/-- The fin pending action fires through the base arm. -/
private theorem finPend_applyO {s : State} {f : Ev} {a : Action}
    (hp : finPend sk s = [(f, a)]) :
    applyO sk .impl ord a s = Model.apply sk .impl a s := by
  unfold finPend at hp
  split at hp
  · injection hp with h1 _
    injection h1 with _ ha
    subst ha
    rfl
  · split at hp
    · injection hp with h1 _
      injection h1 with _ ha
      subst ha
      rfl
    · simp at hp

/-- The responder finish decode, O-denominated: the E decode
outright. -/
theorem fin_pend_or_doneO {s : State} (hi : InvL sk .impl s) :
    ((∀ e ∈ finEvents sk, performedO sk ord s e) ∧ finPend sk s = [])
    ∨ ∃ f a pre suf, finPend sk s = [(f, a)]
        ∧ finEvents sk = pre ++ f :: suf
        ∧ (∀ e ∈ pre, performedO sk ord s e)
        ∧ PendOkO sk ord s f a := by
  rcases fin_pend_or_doneE sk hi with ⟨hall, hp⟩
    | ⟨f, a, pre, suf, hp, hdec, hpre, hok⟩
  · exact Or.inl ⟨fun e he => performedO_of_performed sk ord
      (fin_ev_blind sk ord he) (hall e he), hp⟩
  · refine Or.inr ⟨f, a, pre, suf, hp, hdec, ?_, ?_⟩
    · intro e he
      refine performedO_of_performed sk ord
        (fin_ev_blind sk ord ?_) (hpre e he)
      rw [hdec]
      exact List.mem_append_left _ he
    · refine pendOkO_of_E sk ord hok (finPend_applyO sk ord hp)
        (fin_ev_blind sk ord ?_)
      rw [hdec]
      exact List.mem_append_right _ (List.mem_cons_self ..)

/-- Every ropen-trace receive reads the base count (the opener has no
pairing loop). -/
private theorem ropen_ev_blind {s : State} {e : Ev}
    (he : e ∈ ropenEvents sk) :
    e.2.1 = true ∨ recvdOfO sk ord s e.1 = recvdOf sk s e.1 := by
  unfold ropenEvents at he
  rcases List.mem_cons.1 he with rfl | he
  · exact Or.inr (recvdOfO_wire_I_root sk ord _)
  rcases List.mem_cons.1 he with rfl | he
  · exact Or.inl rfl
  rcases List.mem_cons.1 he with rfl | he
  · exact Or.inl rfl
  · obtain ⟨j, -, rfl⟩ := List.mem_map.1 he
    exact Or.inl rfl

/-- The ropen pending action fires through the base arm. -/
private theorem roPend_applyO {s : State} {f : Ev} {a : Action}
    (hp : roPend sk s = [(f, a)]) :
    applyO sk .impl ord a s = Model.apply sk .impl a s := by
  unfold roPend at hp
  split at hp
  · injection hp with h1 _
    injection h1 with _ ha
    subst ha
    rfl
  · split at hp
    · injection hp with h1 _
      injection h1 with _ ha
      subst ha
      rfl
    · injection hp with h1 _
      injection h1 with _ ha
      subst ha
      rfl
    · injection hp with h1 _
      injection h1 with _ ha
      subst ha
      rfl
    · simp at hp

/-- The responder opening decode, O-denominated: the E decode
outright. -/
theorem ropen_pend_or_doneO (hwf : sk.wellFormed = true) {s : State}
    (hi : InvL sk .impl s)
    (hch : s.ropenGotWire = true → s.ropenCh = none →
      doneROpen sk s = true) :
    ((∀ e ∈ ropenEvents sk, performedO sk ord s e) ∧ roPend sk s = [])
    ∨ ∃ f a pre suf, roPend sk s = [(f, a)]
        ∧ ropenEvents sk = pre ++ f :: suf
        ∧ (∀ e ∈ pre, performedO sk ord s e)
        ∧ PendOkO sk ord s f a := by
  rcases ropen_pend_or_doneE sk hwf hi hch with ⟨hall, hp⟩
    | ⟨f, a, pre, suf, hp, hdec, hpre, hok⟩
  · exact Or.inl ⟨fun e he => performedO_of_performed sk ord
      (ropen_ev_blind sk ord he) (hall e he), hp⟩
  · refine Or.inr ⟨f, a, pre, suf, hp, hdec, ?_, ?_⟩
    · intro e he
      refine performedO_of_performed sk ord
        (ropen_ev_blind sk ord ?_) (hpre e he)
      rw [hdec]
      exact List.mem_append_left _ he
    · refine pendOkO_of_E sk ord hok (roPend_applyO sk ord hp)
        (ropen_ev_blind sk ord ?_)
      rw [hdec]
      exact List.mem_append_right _ (List.mem_cons_self ..)

/-- Every asm-trace receive reads the base count (assemblers have no
pairing loop; their channels are delegate arms of `recvdOfO`). -/
private theorem asm_ev_blind {s : State} {pk : Party × Nat} {e : Ev}
    (he : e ∈ asmEvents sk pk) :
    e.2.1 = true ∨ recvdOfO sk ord s e.1 = recvdOf sk s e.1 := by
  unfold asmEvents at he
  obtain ⟨j, -, hje⟩ := List.mem_flatMap.1 he
  rw [asmBlock_eq] at hje
  rcases List.mem_cons.1 hje with rfl | hje
  · exact Or.inr (recvdOfO_asmResChan sk ord _ pk)
  rcases List.mem_append.1 hje with hseg | hone
  · obtain ⟨cc, bb, nn⟩ := e
    obtain ⟨hc, hb, -, -⟩ := mem_seg hseg
    subst hc hb
    exact Or.inr (recvdOfO_asmLevelChan sk ord _ pk)
  · rw [List.mem_singleton] at hone
    subst hone
    exact Or.inl rfl

/-- The asm pending action fires through the base arm. -/
private theorem asmPend_applyO {s : State} {pk : Party × Nat} {f : Ev}
    {a : Action} (hp : asmPend sk s pk = [(f, a)]) :
    applyO sk .impl ord a s = Model.apply sk .impl a s := by
  simp only [asmPend] at hp
  split at hp
  · injection hp with h1 _
    injection h1 with _ ha
    subst ha
    rfl
  · split at hp
    · injection hp with h1 _
      injection h1 with _ ha
      subst ha
      rfl
    · split at hp
      · injection hp with h1 _
        injection h1 with _ ha
        subst ha
        rfl
      · simp at hp

/-- The assembler decode, O-denominated: the E decode outright. -/
theorem asm_pend_or_doneO (hwf : sk.wellFormed = true) {s : State}
    (hi : InvL sk .impl s) {pk : Party × Nat} (hpk : pk ∈ sk.asmKeys) :
    ((∀ e ∈ asmEvents sk pk, performedO sk ord s e)
        ∧ asmPend sk s pk = [])
    ∨ ∃ f a pre suf, asmPend sk s pk = [(f, a)]
        ∧ asmEvents sk pk = pre ++ f :: suf
        ∧ (∀ e ∈ pre, performedO sk ord s e)
        ∧ PendOkO sk ord s f a := by
  rcases asm_pend_or_doneE sk hwf hi hpk with ⟨hall, hp⟩
    | ⟨f, a, pre, suf, hp, hdec, hpre, hok⟩
  · exact Or.inl ⟨fun e he => performedO_of_performed sk ord
      (asm_ev_blind sk ord he) (hall e he), hp⟩
  · refine Or.inr ⟨f, a, pre, suf, hp, hdec, ?_, ?_⟩
    · intro e he
      refine performedO_of_performed sk ord
        (asm_ev_blind sk ord (pk := pk) ?_) (hpre e he)
      rw [hdec]
      exact List.mem_append_left _ he
    · refine pendOkO_of_E sk ord hok (asmPend_applyO sk ord hp)
        (asm_ev_blind sk ord (pk := pk) ?_)
      rw [hdec]
      exact List.mem_append_right _ (List.mem_cons_self ..)

-- ================================================== the absorber decode

/-- The absorber's pending operation under the assignment, by phase:
the assignment's FIRST receive at phase 0 (the leaf wire reply-first,
the leaf request query-first), the second at phase 1, the order-blind
level-0 return at phase 2. Cf. `Sched.abPend`, the reply-first
instance. -/
def abPendO (s : State) : List (Ev × Action) :=
  if s.absorbPhase = 0 then
    match ord.absorb with
    | .replyFirst =>
        [((Chan.wire Party.R 0, false, s.absorbIdx), .absorbRecvWire)]
    | .queryFirst =>
        [((Chan.leafRequests, false, s.absorbIdx), .absorbRecvAsked)]
  else if s.absorbPhase = 1 then
    match ord.absorb with
    | .replyFirst =>
        [((Chan.leafRequests, false, s.absorbIdx), .absorbRecvAsked)]
    | .queryFirst =>
        [((Chan.wire Party.R 0, false, s.absorbIdx), .absorbRecvWire)]
  else if s.absorbPhase = 2 then
    [((Chan.level Party.I 0, true, s.absorbIdx), .absorbSend)]
  else []

/-- At a query-first ABSORBER the O absorb block reorders: leaf
request, wire, then the level-0 return (cf.
`absorbBlockO_rf_absorb`). -/
theorem absorbBlockO_qf_absorb (h : ord.absorb = .queryFirst) (j : Nat) :
    absorbBlockO ord j
      = [(Chan.leafRequests, false, j), (Chan.wire Party.R 0, false, j),
          (Chan.level Party.I 0, true, j)] := by
  simp only [absorbBlockO, h, List.cons_append, List.nil_append]

/-- The absorber decode: past its channel work with everything
O-performed, or holding one pending event — the assignment's next
receive, or the order-blind level-0 send — with the `absorbEventsO`
prefix below it O-performed. -/
theorem absorb_pend_or_doneO (hwf : sk.wellFormed = true) {s : State}
    (hi : InvL sk .impl s) :
    ((∀ e ∈ absorbEventsO sk ord, performedO sk ord s e)
        ∧ abPendO ord s = [])
    ∨ ∃ f a pre suf, abPendO ord s = [(f, a)]
        ∧ absorbEventsO sk ord = pre ++ f :: suf
        ∧ (∀ e ∈ pre, performedO sk ord s e)
        ∧ PendOkO sk ord s f a := by
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
  have hrwO : recvdOfO sk ord s (Chan.wire Party.R 0)
      = absorbWireRecvdO sk ord s := by
    have hne : (0 == sk.rootH) = false := by
      simp
      omega
    simp [recvdOfO, hne]
  have hrqO : recvdOfO sk ord s Chan.leafRequests
      = absorbAskedRecvdO sk ord s := rfl
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
  have hWaO : s.absorbIdx ≤ absorbWireRecvdO sk ord s := by
    cases hord : ord.absorb <;>
      simp only [absorbWireRecvdO, hord] <;> omega
  have hAaO : s.absorbIdx ≤ absorbAskedRecvdO sk ord s := by
    cases hord : ord.absorb <;>
      simp only [absorbAskedRecvdO, hord] <;> omega
  have hwr0mem : Chan.wire Party.R 0 ∈ allChans sk := by
    have hkey : (Party.R, 0) ∈ sk.walkKeys :=
      mem_walkKeys_of sk hwf (by omega) (Or.inr ⟨rfl, by omega⟩)
    have : Chan.wire Party.R 0 = wireOut (Party.R, 0) := rfl
    rw [this]
    exact (walk_chans_mem sk hkey).1
  cases hord : ord.absorb with
  | replyFirst =>
      have hblock : ∀ j, j < s.absorbIdx →
          ∀ e ∈ absorbBlockO ord j, performedO sk ord s e := by
        intro j hj e he
        rw [absorbBlockO_rf_absorb ord hord] at he
        rcases List.mem_cons.1 he with rfl | he
        · show j < recvdOfO sk ord s (Chan.wire Party.R 0)
          rw [hrwO]
          omega
        rcases List.mem_cons.1 he with rfl | he
        · show j < recvdOfO sk ord s Chan.leafRequests
          rw [hrqO]
          omega
        rcases List.mem_cons.1 he with rfl | he
        · show j < sentOf sk s (Chan.level Party.I 0)
          rw [hsl]
          omega
        · cases he
      have hpreperf : ∀ e ∈ (List.range s.absorbIdx).flatMap
          (absorbBlockO ord), performedO sk ord s e := by
        intro e he
        obtain ⟨j, hjm, hje⟩ := List.mem_flatMap.1 he
        rw [List.mem_range] at hjm
        exact hblock j hjm e hje
      have hsplit : ∀ _ : s.absorbIdx < sk.totalLeafReqs,
          absorbEventsO sk ord
          = (List.range s.absorbIdx).flatMap (absorbBlockO ord)
            ++ ((Chan.wire Party.R 0, false, s.absorbIdx)
              :: (Chan.leafRequests, false, s.absorbIdx)
              :: (Chan.level Party.I 0, true, s.absorbIdx)
              :: (List.range' (s.absorbIdx + 1)
                  (sk.totalLeafReqs - s.absorbIdx - 1)).flatMap
                  (absorbBlockO ord)) := by
        intro hlt
        unfold absorbEventsO
        rw [range_split (show s.absorbIdx ≤ sk.totalLeafReqs by omega),
          List.flatMap_append,
          show sk.totalLeafReqs - s.absorbIdx
            = (sk.totalLeafReqs - s.absorbIdx - 1) + 1 from by omega,
          List.range'_succ, List.flatMap_cons,
          absorbBlockO_rf_absorb ord hord]
        rfl
      rcases Nat.lt_or_ge s.absorbPhase 3 with hph | hph3
      · right
        have hlt := hidx1 (by omega)
        rcases Nat.lt_or_ge s.absorbPhase 1 with hph0 | hph1
        · have hph' : s.absorbPhase = 0 := by omega
          refine ⟨(Chan.wire Party.R 0, false, s.absorbIdx),
            .absorbRecvWire,
            _, _, by simp [abPendO, hph', hord], hsplit hlt, hpreperf,
            hwr0mem, ?_, fixed_action_mem sk (by simp), ?_⟩
          · show s.absorbIdx = recvdOfO sk ord s (Chan.wire Party.R 0)
            rw [hrwO]
            simp only [absorbWireRecvdO, hord]
            unfold absorbWireRecvd
            rw [if_neg (by omega), hph']
            simp
          · intro hch
            rw [if_neg (by simp)] at hch
            have : (applyO sk .impl ord .absorbRecvWire s).isSome
                = true := by
              simp [applyO, hord, hph']
              omega
            exact this
        · rcases Nat.lt_or_ge s.absorbPhase 2 with hph1' | hph2
          · have hph' : s.absorbPhase = 1 := by omega
            refine ⟨(Chan.leafRequests, false, s.absorbIdx),
              .absorbRecvAsked,
              (List.range s.absorbIdx).flatMap (absorbBlockO ord)
                ++ [(Chan.wire Party.R 0, false, s.absorbIdx)],
              (Chan.level Party.I 0, true, s.absorbIdx)
                :: (List.range' (s.absorbIdx + 1)
                    (sk.totalLeafReqs - s.absorbIdx - 1)).flatMap
                    (absorbBlockO ord),
              by simp [abPendO, hph', hord], ?_, ?_,
              (root_chans_mem sk).2.2.1, ?_,
              fixed_action_mem sk (by simp), ?_⟩
            · rw [hsplit hlt]
              simp [List.cons_append]
            · intro e he
              rcases List.mem_append.1 he with hp | hone
              · exact hpreperf e hp
              · rw [List.mem_singleton] at hone
                subst hone
                show s.absorbIdx < recvdOfO sk ord s (Chan.wire Party.R 0)
                rw [hrwO]
                simp only [absorbWireRecvdO, hord]
                unfold absorbWireRecvd
                rw [if_neg (by omega), hph']
                simp
            · show s.absorbIdx = recvdOfO sk ord s Chan.leafRequests
              rw [hrqO]
              simp only [absorbAskedRecvdO, hord]
              unfold absorbAskedRecvd
              rw [if_neg (by omega), hph']
              simp
            · intro hch
              rw [if_neg (by simp)] at hch
              have : (applyO sk .impl ord .absorbRecvAsked s).isSome
                  = true := by
                simp [applyO, hord, hph']
                omega
              exact this
          · have hph' : s.absorbPhase = 2 := by omega
            refine ⟨(Chan.level Party.I 0, true, s.absorbIdx), .absorbSend,
              (List.range s.absorbIdx).flatMap (absorbBlockO ord)
                ++ [(Chan.wire Party.R 0, false, s.absorbIdx),
                  (Chan.leafRequests, false, s.absorbIdx)],
              (List.range' (s.absorbIdx + 1)
                (sk.totalLeafReqs - s.absorbIdx - 1)).flatMap
                (absorbBlockO ord),
              by simp [abPendO, hph'], ?_, ?_,
              (root_chans_mem sk).2.2.2.1, ?_,
              fixed_action_mem sk (by simp), ?_⟩
            · rw [hsplit hlt]
              simp [List.cons_append]
            · intro e he
              rcases List.mem_append.1 he with hp | htwo
              · exact hpreperf e hp
              rcases List.mem_cons.1 htwo with rfl | htwo
              · show s.absorbIdx < recvdOfO sk ord s (Chan.wire Party.R 0)
                rw [hrwO]
                simp only [absorbWireRecvdO, hord]
                unfold absorbWireRecvd
                rw [if_neg (by omega), hph']
                simp
              rcases List.mem_cons.1 htwo with rfl | hnil
              · show s.absorbIdx < recvdOfO sk ord s Chan.leafRequests
                rw [hrqO]
                simp only [absorbAskedRecvdO, hord]
                unfold absorbAskedRecvd
                rw [if_neg (by omega), hph']
                simp
              · cases hnil
            · show s.absorbIdx = sentOf sk s (Chan.level Party.I 0)
              rw [hsl]
            · intro hch
              rw [if_pos rfl] at hch
              have : (apply sk .impl .absorbSend s).isSome = true := by
                simp [apply, hph']
                omega
              exact this
      · left
        have hidx := hidx2 hph3
        have hpend0 : abPendO ord s = [] := by
          unfold abPendO
          rw [if_neg (by omega), if_neg (by omega), if_neg (by omega)]
        refine ⟨?_, hpend0⟩
        intro e he
        unfold absorbEventsO at he
        obtain ⟨j, hjm, hje⟩ := List.mem_flatMap.1 he
        rw [List.mem_range] at hjm
        exact hblock j (by omega) e hje
  | queryFirst =>
      have hblock : ∀ j, j < s.absorbIdx →
          ∀ e ∈ absorbBlockO ord j, performedO sk ord s e := by
        intro j hj e he
        rw [absorbBlockO_qf_absorb ord hord] at he
        rcases List.mem_cons.1 he with rfl | he
        · show j < recvdOfO sk ord s Chan.leafRequests
          rw [hrqO]
          omega
        rcases List.mem_cons.1 he with rfl | he
        · show j < recvdOfO sk ord s (Chan.wire Party.R 0)
          rw [hrwO]
          omega
        rcases List.mem_cons.1 he with rfl | he
        · show j < sentOf sk s (Chan.level Party.I 0)
          rw [hsl]
          omega
        · cases he
      have hpreperf : ∀ e ∈ (List.range s.absorbIdx).flatMap
          (absorbBlockO ord), performedO sk ord s e := by
        intro e he
        obtain ⟨j, hjm, hje⟩ := List.mem_flatMap.1 he
        rw [List.mem_range] at hjm
        exact hblock j hjm e hje
      have hsplit : ∀ _ : s.absorbIdx < sk.totalLeafReqs,
          absorbEventsO sk ord
          = (List.range s.absorbIdx).flatMap (absorbBlockO ord)
            ++ ((Chan.leafRequests, false, s.absorbIdx)
              :: (Chan.wire Party.R 0, false, s.absorbIdx)
              :: (Chan.level Party.I 0, true, s.absorbIdx)
              :: (List.range' (s.absorbIdx + 1)
                  (sk.totalLeafReqs - s.absorbIdx - 1)).flatMap
                  (absorbBlockO ord)) := by
        intro hlt
        unfold absorbEventsO
        rw [range_split (show s.absorbIdx ≤ sk.totalLeafReqs by omega),
          List.flatMap_append,
          show sk.totalLeafReqs - s.absorbIdx
            = (sk.totalLeafReqs - s.absorbIdx - 1) + 1 from by omega,
          List.range'_succ, List.flatMap_cons,
          absorbBlockO_qf_absorb ord hord]
        rfl
      rcases Nat.lt_or_ge s.absorbPhase 3 with hph | hph3
      · right
        have hlt := hidx1 (by omega)
        rcases Nat.lt_or_ge s.absorbPhase 1 with hph0 | hph1
        · have hph' : s.absorbPhase = 0 := by omega
          refine ⟨(Chan.leafRequests, false, s.absorbIdx),
            .absorbRecvAsked,
            _, _, by simp [abPendO, hph', hord], hsplit hlt, hpreperf,
            (root_chans_mem sk).2.2.1, ?_,
            fixed_action_mem sk (by simp), ?_⟩
          · show s.absorbIdx = recvdOfO sk ord s Chan.leafRequests
            rw [hrqO]
            simp only [absorbAskedRecvdO, hord]
            unfold absorbWireRecvd
            rw [if_neg (by omega), hph']
            simp
          · intro hch
            rw [if_neg (by simp)] at hch
            have : (applyO sk .impl ord .absorbRecvAsked s).isSome
                = true := by
              simp [applyO, hord, hph']
              omega
            exact this
        · rcases Nat.lt_or_ge s.absorbPhase 2 with hph1' | hph2
          · have hph' : s.absorbPhase = 1 := by omega
            refine ⟨(Chan.wire Party.R 0, false, s.absorbIdx),
              .absorbRecvWire,
              (List.range s.absorbIdx).flatMap (absorbBlockO ord)
                ++ [(Chan.leafRequests, false, s.absorbIdx)],
              (Chan.level Party.I 0, true, s.absorbIdx)
                :: (List.range' (s.absorbIdx + 1)
                    (sk.totalLeafReqs - s.absorbIdx - 1)).flatMap
                    (absorbBlockO ord),
              by simp [abPendO, hph', hord], ?_, ?_,
              hwr0mem, ?_,
              fixed_action_mem sk (by simp), ?_⟩
            · rw [hsplit hlt]
              simp [List.cons_append]
            · intro e he
              rcases List.mem_append.1 he with hp | hone
              · exact hpreperf e hp
              · rw [List.mem_singleton] at hone
                subst hone
                show s.absorbIdx < recvdOfO sk ord s Chan.leafRequests
                rw [hrqO]
                simp only [absorbAskedRecvdO, hord]
                unfold absorbWireRecvd
                rw [if_neg (by omega), hph']
                simp
            · show s.absorbIdx = recvdOfO sk ord s (Chan.wire Party.R 0)
              rw [hrwO]
              simp only [absorbWireRecvdO, hord]
              unfold absorbAskedRecvd
              rw [if_neg (by omega), hph']
              simp
            · intro hch
              rw [if_neg (by simp)] at hch
              have : (applyO sk .impl ord .absorbRecvWire s).isSome
                  = true := by
                simp [applyO, hord, hph']
                omega
              exact this
          · have hph' : s.absorbPhase = 2 := by omega
            refine ⟨(Chan.level Party.I 0, true, s.absorbIdx), .absorbSend,
              (List.range s.absorbIdx).flatMap (absorbBlockO ord)
                ++ [(Chan.leafRequests, false, s.absorbIdx),
                  (Chan.wire Party.R 0, false, s.absorbIdx)],
              (List.range' (s.absorbIdx + 1)
                (sk.totalLeafReqs - s.absorbIdx - 1)).flatMap
                (absorbBlockO ord),
              by simp [abPendO, hph'], ?_, ?_,
              (root_chans_mem sk).2.2.2.1, ?_,
              fixed_action_mem sk (by simp), ?_⟩
            · rw [hsplit hlt]
              simp [List.cons_append]
            · intro e he
              rcases List.mem_append.1 he with hp | htwo
              · exact hpreperf e hp
              rcases List.mem_cons.1 htwo with rfl | htwo
              · show s.absorbIdx < recvdOfO sk ord s Chan.leafRequests
                rw [hrqO]
                simp only [absorbAskedRecvdO, hord]
                unfold absorbWireRecvd
                rw [if_neg (by omega), hph']
                simp
              rcases List.mem_cons.1 htwo with rfl | hnil
              · show s.absorbIdx < recvdOfO sk ord s (Chan.wire Party.R 0)
                rw [hrwO]
                simp only [absorbWireRecvdO, hord]
                unfold absorbAskedRecvd
                rw [if_neg (by omega), hph']
                simp
              · cases hnil
            · show s.absorbIdx = sentOf sk s (Chan.level Party.I 0)
              rw [hsl]
            · intro hch
              rw [if_pos rfl] at hch
              have : (apply sk .impl .absorbSend s).isSome = true := by
                simp [apply, hph']
                omega
              exact this
      · left
        have hidx := hidx2 hph3
        have hpend0 : abPendO ord s = [] := by
          unfold abPendO
          rw [if_neg (by omega), if_neg (by omega), if_neg (by omega)]
        refine ⟨?_, hpend0⟩
        intro e he
        unfold absorbEventsO at he
        obtain ⟨j, hjm, hje⟩ := List.mem_flatMap.1 he
        rw [List.mem_range] at hjm
        exact hblock j (by omega) e hje

end StreamingMirror.Ord
