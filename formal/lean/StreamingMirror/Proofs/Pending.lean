/-
The pending layer (PROGRESS.md §7 items 5–6): each determined process
of a reachable state decodes to a position in its own schedule trace —
everything before it performed, the pending event carrying the
channel's CURRENT count as its seq.

# Why no new induction

The §6 plan expected a "performed-set = trace prefix" invariant by
`Reachable` induction. Under the amended `.full` none is needed: the
committed-arm mirrors of `wkLocalOk` (including the `d5` conjunct,
finding #7) pin, at every committed state, exactly the facts that make
the trace prefix below the committed event performed — and states at
CHOICE points never consult a cursor, because the pillar
(`walk_uncommitted_canStep`) and the opener mirrors discharge them
outright. So every lemma here is a static consequence of `InvP`.

"Performed" is count-based: event `(c, b, n)` has happened iff `n` is
below the state's derived count on `(c, b)` (`sentOf`/`recvdOf`). The
per-process lemmas conclude an `Or`: either the whole trace is
performed (the process is past its channel work, only closes remain),
or the trace splits as `pre ++ pending :: rest` with `pre` performed
and the pending event `PendOk` — seq = current count, its action
enumerated, and enabled as soon as its channel guard opens.

The argmin assembly (`Endgame.lean`) consumes these through the
τ-comparison `tau_le_of_pend`: an unperformed event of a trace sits at
or after the trace's pending split, so the pending head is τ-least —
position-in-schedule order along a trace is genuine by
`merge_complete` (every trace is a sublist of the schedule) and τ
injectivity.

Chain (d5, stage D): consumes `merge_complete` and the numbering layer;
provides the per-family decodes to Endgame.lean. E mirror:
PendingE.lean. Map: Proofs/Map.lean.
-/
import StreamingMirror.Proofs.Progress
import StreamingMirror.Proofs.Preserve
import StreamingMirror.Proofs.Wiring
import StreamingMirror.Proofs.Sched.Weave.Final

namespace StreamingMirror.Sched

open Model

variable (sk : Skel)

-- ================================================= schedule-side glue

/-- Merge completeness, read back through trace monotonicity: every
trace embeds in the schedule in order. This is what makes
position-in-schedule a total order along each trace. -/
theorem trace_sublist (hwf : sk.wellFormed = true)
    (hsched : sk.schedulable = true) {T : List Ev}
    (hT : T ∈ procs sk) : T.Sublist (schedule sk) := by
  obtain ⟨r, hr, pre, hpre, hsub⟩ :=
    (trace_monotone sk).exists_of_mem_left hT
  have hempty : r = [] := by
    have := List.all_eq_true.1 (merge_complete sk hwf hsched) r hr
    cases r with
    | nil => rfl
    | cons a l => simp at this
  rw [hempty, List.append_nil] at hpre
  exact hpre ▸ hsub

/-- τ injectivity in counting form: the schedule holds each event at
most once (its per-channel projections are canonical). -/
theorem schedule_count_le_one (hwf : sk.wellFormed = true) (e : Ev) :
    (schedule sk).count e ≤ 1 := by
  obtain ⟨c, b, n⟩ := e
  obtain ⟨m, hm⟩ := schedule_proj_canon sk hwf c b
  have hfilter : (schedule sk).count (c, b, n)
      = (proj c b (schedule sk)).count (c, b, n) := by
    unfold proj
    exact (List.count_filter (by simp)).symm
  rw [hfilter, hm, count_canon]
  split <;> omega

/-- Provenance: every schedule event was emitted by some trace. -/
theorem sched_mem_trace {e : Ev} (he : e ∈ schedule sk) :
    ∃ T ∈ procs sk, e ∈ T := by
  have hpos : 1 ≤ emittedCount (fun x => x == e) (procs sk)
      (finalState sk).rem := by
    rw [← schedule_count sk (fun x => x == e)]
    have hm : e ∈ (schedule sk).filter (fun x => x == e) :=
      List.mem_filter.2 ⟨he, by simp⟩
    have := List.length_pos_of_mem hm
    omega
  obtain ⟨T, hT, e', he', hbeq⟩ := emittedCount_pos hpos
  have : e' = e := by simpa using hbeq
  exact ⟨T, hT, this ▸ he'⟩

-- ============================================ performedness and PendOk

/-- Event `e` has already happened at state `s`: its seq is below the
state's derived count on its channel-side. -/
def performed (s : State) (e : Ev) : Prop :=
  if e.2.1 = true then e.2.2 < sentOf sk s e.1
  else e.2.2 < recvdOf sk s e.1

/-- The pending event's global obligations: its channel is a real flow
channel, its seq is the channel's CURRENT count (so it is the first
unperformed event of its channel-side), its action is enumerated, and
the action is enabled as soon as the channel guard opens (room for a
send, data for a receive). -/
structure PendOk (s : State) (f : Ev) (a : Action) : Prop where
  chan_mem : f.1 ∈ allChans sk
  seq : f.2.2 = (if f.2.1 = true then sentOf sk s f.1
    else recvdOf sk s f.1)
  act : a ∈ allActions sk
  fire : (if f.2.1 = true then s.chan f.1 < sk.cap f.1
      else 0 < s.chan f.1)
    → (apply sk .full a s).isSome = true

/-- A pending event is never performed: its seq IS the count. -/
theorem pend_not_performed {s : State} {f : Ev} {a : Action}
    (h : PendOk sk s f a) : ¬ performed sk s f := by
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
the schedule. -/
theorem tau_le_of_pend (hwf : sk.wellFormed = true)
    (hsched : sk.schedulable = true) {s : State}
    {T pre suf : List Ev} {f : Ev}
    (hT : T ∈ procs sk) (hdec : T = pre ++ f :: suf)
    (hpre : ∀ e ∈ pre, performed sk s e)
    {g : Ev} (hg : g ∈ T) (hnp : ¬ performed sk s g) :
    evIdx f (schedule sk) ≤ evIdx g (schedule sk) := by
  rw [hdec] at hg
  rcases List.mem_append.1 hg with hgpre | hgcons
  · exact absurd (hpre g hgpre) hnp
  · rcases List.mem_cons.1 hgcons with rfl | hgsuf
    · exact Nat.le_refl _
    · have hpair : ([f, g] : List Ev).Sublist T := by
        rw [hdec]
        refine List.Sublist.trans ?_ (List.sublist_append_right pre _)
        exact List.cons_sublist_cons.2 (List.singleton_sublist.2 hgsuf)
      have hsub : ([f, g] : List Ev).Sublist (schedule sk) :=
        hpair.trans (trace_sublist sk hwf hsched hT)
      exact Nat.le_of_lt
        (pos_lt_of_pair (schedule_count_le_one sk hwf) hsub)

-- ============================================= small counting bricks

/-- `qSum` is monotone in the child cursor. -/
theorem qSum_mono (pk : Party × Nat) (k : Nat) {i i' : Nat}
    (h : i ≤ i') : qSum sk pk k i ≤ qSum sk pk k i' := by
  induction i' with
  | zero => have : i = 0 := by omega
            subst this; exact Nat.le_refl _
  | succ n ih =>
      by_cases hlast : i = n + 1
      · subst hlast; exact Nat.le_refl _
      · have := ih (by omega)
        rw [qSum_succ]
        omega

/-- `qSum` through the child count is bounded by the scope total. -/
theorem qSum_le_qOf (pk : Party × Nat) (k : Nat) {i : Nat}
    (h : i ≤ sk.nChildren pk.2 (sk.stageScope pk.2 k)) :
    qSum sk pk k i ≤ sk.qOf pk.2 (sk.stageScope pk.2 k) := by
  rw [← qSum_total sk pk k]
  exact qSum_mono sk pk k h

-- ==================================== enumeration membership witnesses
-- `canStep` is an existential over `allActions`; every pending action
-- below needs its membership certificate.

/-- The sixteen fixed actions are enumerated. -/
theorem fixed_action_mem {a : Action}
    (ha : a ∈ ([.iopenChoose .wire, .iopenChoose .query, .iopenFire,
      .ropenRecv, .ropenChoose .wire, .ropenChoose .res,
      .ropenChoose .query, .ropenFire,
      .absorbRecvWire, .absorbRecvAsked, .absorbSend, .absorbCloseWire,
      .absorbCloseAsked, .finRet, .finRes, .finRets] : List Action)) :
    a ∈ allActions sk := by
  rw [allActions]
  exact List.mem_append.mpr (.inl (List.mem_append.mpr (.inl ha)))

/-- The per-walk fixed actions (recvs, fire, closes) are enumerated. -/
theorem walk_action_mem {pk : Party × Nat} (hpk : pk ∈ sk.walkKeys)
    {a : Action}
    (ha : a ∈ ([.walkRecvWire pk, .walkRecvAsked pk, .walkFire pk,
      .walkCloseWire pk, .walkCloseAsked pk] : List Action)) :
    a ∈ allActions sk := by
  rw [allActions]
  refine List.mem_append.mpr (.inl (List.mem_append.mpr (.inr ?_)))
  refine List.mem_flatMap.mpr ⟨pk, hpk, ?_⟩
  refine List.mem_append.mpr (.inl ?_)
  simp only [List.mem_cons, List.not_mem_nil, or_false] at ha
  rcases ha with rfl | rfl | rfl | rfl | rfl <;> simp

/-- The per-assembler actions are enumerated. -/
theorem asm_action_mem {pk : Party × Nat} (hpk : pk ∈ sk.asmKeys)
    {a : Action}
    (ha : a ∈ ([.asmRecvRes pk, .asmRecvLevel pk, .asmSend pk,
      .asmClose pk] : List Action)) :
    a ∈ allActions sk := by
  rw [allActions]
  refine List.mem_append.mpr (.inr ?_)
  exact List.mem_flatMap.mpr ⟨pk, hpk, ha⟩

-- ============================================= ledger counting bricks

/-- Downward-closed ledgers with a known count are full below it: the
committed-`.wire` arm's `i == wkWireCount` plus the per-child
prefix-closure shadow recover the guard's explicit wire prefix. -/
theorem frontier_of_count {p : Nat → Bool} {fan i : Nat}
    (hclosed : ∀ j < fan, p j = true → j = 0 ∨ p (j - 1) = true)
    (hcount : ((List.range fan).filter p).length = i) :
    ∀ j, j < i → p j = true := by
  intro j hj
  by_contra hnp
  have hpj : p j = false := by
    cases h : p j with
    | false => rfl
    | true => exact absurd h hnp
  -- everything at or above `j` is unfired (closure, upward induction)
  have habove : ∀ j', j ≤ j' → j' < fan → p j' = false := by
    intro j'
    induction j' with
    | zero =>
        intro hle _
        have : j = 0 := by omega
        exact this ▸ hpj
    | succ m ih =>
        intro hle hlt
        by_cases hjm : j ≤ m
        · cases h : p (m + 1) with
          | false => rfl
          | true =>
              rcases hclosed (m + 1) hlt h with h0 | hprev
              · exact absurd h0 (by omega)
              · have hm : p m = false := ih hjm (by omega)
                simp only [Nat.add_sub_cancel] at hprev
                rw [hm] at hprev
                cases hprev
        · have : j = m + 1 := by omega
          exact this ▸ hpj
  -- so the fired count is at most `j < i`
  have hle : ((List.range fan).filter p).length ≤ j := by
    have hsub : ∀ x ∈ List.range fan, p x = true → decide (x < j) = true := by
      intro x hx hpx
      rw [List.mem_range] at hx
      by_cases hxj : x < j
      · simp [hxj]
      · rw [habove x (by omega) hx] at hpx
        cases hpx
    calc ((List.range fan).filter p).length
        ≤ ((List.range fan).filter (fun x => decide (x < j))).length := by
          have hmono : ∀ l : List Nat,
              (∀ x ∈ l, p x = true → decide (x < j) = true) →
              (l.filter p).length ≤
                (l.filter (fun x => decide (x < j))).length := by
            intro l
            induction l with
            | nil => intro _; simp
            | cons x xs ih =>
                intro h
                simp only [List.filter_cons]
                by_cases hpx : p x = true
                · rw [if_pos hpx, if_pos (h x (List.mem_cons_self ..) hpx)]
                  simp only [List.length_cons]
                  exact Nat.succ_le_succ (ih fun y hy =>
                    h y (List.mem_cons_of_mem _ hy))
                · rw [if_neg hpx]
                  by_cases hdx : decide (x < j) = true
                  · rw [if_pos hdx]
                    simp only [List.length_cons]
                    exact Nat.le_trans (ih fun y hy =>
                      h y (List.mem_cons_of_mem _ hy)) (Nat.le_succ _)
                  · rw [if_neg hdx]
                    exact ih fun y hy => h y (List.mem_cons_of_mem _ hy)
          exact hmono _ hsub
      _ = j := by
          by_cases hjf : j ≤ fan
          · exact Model.length_filter_range_lt hjf
          · have hall : ∀ x ∈ List.range fan, decide (x < j) = true := by
              intro x hx
              rw [List.mem_range] at hx
              simp
              omega
            rw [List.filter_eq_self.mpr hall, List.length_range]
            exfalso
            -- j > fan yet j < i ≤ count ≤ fan: impossible
            have : ((List.range fan).filter p).length ≤ fan := by
              calc ((List.range fan).filter p).length
                  ≤ (List.range fan).length :=
                    List.Sublist.length_le List.filter_sublist
                _ = fan := List.length_range ..
            omega
  omega

/-- A ledger full through `i` counts at least `i`. -/
theorem count_ge_of_prefix {p : Nat → Bool} {fan i : Nat}
    (hif : i ≤ fan) (hlow : ∀ j, j < i → p j = true) :
    i ≤ ((List.range fan).filter p).length := by
  have hmono : ∀ l : List Nat,
      (∀ x ∈ l, decide (x < i) = true → p x = true) →
      (l.filter (fun x => decide (x < i))).length ≤ (l.filter p).length := by
    intro l
    induction l with
    | nil => intro _; simp
    | cons x xs ih =>
        intro h
        simp only [List.filter_cons]
        by_cases hdx : decide (x < i) = true
        · rw [if_pos hdx, if_pos (h x (List.mem_cons_self ..) hdx)]
          simp only [List.length_cons]
          exact Nat.succ_le_succ (ih fun y hy =>
            h y (List.mem_cons_of_mem _ hy))
        · rw [if_neg hdx]
          by_cases hpx : p x = true
          · rw [if_pos hpx]
            simp only [List.length_cons]
            exact Nat.le_trans (ih fun y hy =>
              h y (List.mem_cons_of_mem _ hy)) (Nat.le_succ _)
          · rw [if_neg hpx]
            exact ih fun y hy => h y (List.mem_cons_of_mem _ hy)
  calc i = ((List.range fan).filter (fun x => decide (x < i))).length :=
        (Model.length_filter_range_lt hif).symm
    _ ≤ ((List.range fan).filter p).length := by
        refine hmono _ ?_
        intro x _ hdx
        rw [decide_eq_true_eq] at hdx
        exact hlow x hdx

-- ======================================================= walk decode

/-- A walk key's height determines its party: initiators sit at odd
consumed heights, responders at even (the `wpk` pairing). -/
theorem walkKeys_parity (hwf : sk.wellFormed = true)
    {p : Party} {k : Nat} (hpk : (p, k) ∈ sk.walkKeys) :
    k < sk.rootH ∧ ((p = Party.I ∧ k % 2 = 1) ∨
      (p = Party.R ∧ k % 2 = 0)) := by
  have hev : sk.rootH % 2 = 0 := (wf_rootH hwf).1
  simp only [Skel.walkKeys, List.mem_append, List.mem_map,
    List.mem_range] at hpk
  rcases hpk with ⟨t, ht, heq⟩ | ⟨t, ht, heq⟩
  · rw [Prod.mk.injEq] at heq
    obtain ⟨hp, hk⟩ := heq
    exact ⟨by omega, Or.inl ⟨hp.symm, by omega⟩⟩
  · rw [Prod.mk.injEq] at heq
    obtain ⟨hp, hk⟩ := heq
    exact ⟨by omega, Or.inr ⟨hp.symm, by omega⟩⟩

/-- Every walk key's trace is a merge input. -/
theorem walkEvents_mem_procs (hwf : sk.wellFormed = true)
    {pk : Party × Nat} (hpk : pk ∈ sk.walkKeys) :
    walkEvents sk pk ∈ procs sk := by
  obtain ⟨p, k⟩ := pk
  obtain ⟨hkr, hpar⟩ := walkKeys_parity sk hwf hpk
  simp only [procs]
  refine List.mem_append.mpr (Or.inl (List.mem_append.mpr (Or.inl
    (List.mem_append.mpr (Or.inl (List.mem_append.mpr (Or.inr ?_)))))))
  refine List.mem_map.mpr ⟨(p, k), ?_, rfl⟩
  refine List.mem_map.mpr ⟨sk.rootH - 1 - k, List.mem_range.mpr (by omega), ?_⟩
  have hh : sk.rootH - 1 - (sk.rootH - 1 - k) = k := by omega
  rw [hh]
  rcases hpar with ⟨rfl, hodd⟩ | ⟨rfl, heven⟩
  · rw [if_pos (by simp [hodd])]
  · rw [if_neg (by simp [heven])]

/-- Every event of a completed-scope block is performed: the state's
derived counts dominate the scope-prefix sums, and a completed scope's
events all sit below its own prefix boundary. Serves every walk phase
(for phases `≤ 2` the current scope is past `j`; past phase 2 every
scope is). -/
theorem scopeBlock_performed (hwf : sk.wellFormed = true) {s : State}
    {pk : Party × Nat} (hi : InvP sk .full s) (hpk : pk ∈ sk.walkKeys)
    {j : Nat} (hj : j < (s.walk pk).scope) (hjs : j < sk.stageLen pk.2) :
    ∀ e ∈ scopeBlock sk pk j, performed sk s e := by
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
  unfold scopeBlock at he
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
  rcases mem_scopeSends sk he with rfl | ⟨i, hin, hchunk⟩
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

-- ==================================== in-scope prefix helper bricks

/-- Pointwise implication is filter-length monotone. -/
theorem length_filter_le_of_imp {p q : Nat → Bool} {l : List Nat}
    (h : ∀ x ∈ l, p x = true → q x = true) :
    (l.filter p).length ≤ (l.filter q).length := by
  induction l with
  | nil => simp
  | cons x xs ih =>
      simp only [List.filter_cons]
      by_cases hpx : p x = true
      · rw [if_pos hpx, if_pos (h x (List.mem_cons_self ..) hpx)]
        simp only [List.length_cons]
        exact Nat.succ_le_succ (ih fun y hy =>
          h y (List.mem_cons_of_mem _ hy))
      · rw [if_neg hpx]
        by_cases hqx : q x = true
        · rw [if_pos hqx]
          simp only [List.length_cons]
          exact Nat.le_trans (ih fun y hy =>
            h y (List.mem_cons_of_mem _ hy)) (Nat.le_succ _)
        · rw [if_neg hqx]
          exact ih fun y hy => h y (List.mem_cons_of_mem _ hy)

/-- `range n` splits at any cut below `n`. -/
theorem range_split {k n : Nat} (h : k ≤ n) :
    List.range n = List.range k ++ List.range' k (n - k) := by
  rw [List.range_eq_range', List.range_eq_range']
  have hn : n = k + (n - k) := by omega
  conv => lhs; rw [hn]
  rw [← List.range'_append]
  simp

/-- A `< i`-guarded filter over the full fan is the plain filter over
the prefix. -/
theorem filter_range_and_lt {p : Nat → Bool} {i fan : Nat}
    (h : i ≤ fan) :
    ((List.range fan).filter fun j => decide (j < i) && p j).length
      = ((List.range i).filter p).length := by
  rw [range_split h, List.filter_append, List.length_append]
  have hleft : (List.range i).filter (fun j => decide (j < i) && p j)
      = (List.range i).filter p := by
    apply List.filter_congr
    intro x hx
    rw [List.mem_range] at hx
    simp [hx]
  have hright : (List.range' i (fan - i)).filter
      (fun j => decide (j < i) && p j) = [] := by
    rw [List.filter_eq_nil_iff]
    intro x hx
    have := List.mem_range'_1.1 hx
    simp
    omega
  rw [hleft, hright]
  simp

/-- A downward-closed ledger fired at `i` is fired everywhere below. -/
theorem fired_below {p : Nat → Bool} {fan : Nat}
    (hclosed : ∀ j < fan, p j = true → j = 0 ∨ p (j - 1) = true)
    {i : Nat} (hif : i < fan) (hpi : p i = true) :
    ∀ j, j ≤ i → p j = true := by
  induction i with
  | zero =>
      intro j hj
      have : j = 0 := by omega
      exact this ▸ hpi
  | succ m ih =>
      intro j hj
      by_cases hjm : j = m + 1
      · exact hjm ▸ hpi
      · rcases hclosed (m + 1) hif hpi with h0 | hprev
        · exact absurd h0 (by omega)
        · simp only [Nat.add_sub_cancel] at hprev
          exact ih (by omega) hprev j (by omega)

/-- `wkQSum` as a map-sum, for the split lemmas. -/
theorem wkQSum_eq_sum (s : State) (pk : Party × Nat) :
    wkQSum sk s pk = ((List.range sk.fan).map (s.walk pk).qSent).sum := by
  unfold wkQSum
  rw [Model.foldl_add_eq_sum]
  omega

/-- The query ledger's prefix sum is dominated by the whole. -/
theorem qsum_prefix_le (s : State) (pk : Party × Nat) {i : Nat}
    (hif : i ≤ sk.fan) :
    ((List.range i).map (s.walk pk).qSent).sum ≤ wkQSum sk s pk := by
  rw [wkQSum_eq_sum, range_split hif, List.map_append, List.sum_append]
  omega

/-- The per-child ledger facts of a phase-2 walk under `.full`: the
fired-fact shadows `wkLocalOk` carries, named for the committed-case
splits. `hDdis` is the `d4` shadow (a fired wire discharges every
earlier D child), `hqres` the `d1int` shadow, `hrw` the `w` shadow. -/
theorem phase2_child_facts {s : State} {pk : Party × Nat}
    (hi : InvP sk .full s) (hpk : pk ∈ sk.walkKeys)
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
  simp [AxMode.full] at hwk
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
theorem walk_scope_bound {s : State} {pk : Party × Nat}
    (hi : InvP sk .full s) (hpk : pk ∈ sk.walkKeys) :
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
theorem walk_ledgers_empty {s : State} {pk : Party × Nat}
    (hi : InvP sk .full s) (hpk : pk ∈ sk.walkKeys)
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
theorem counts_of_empty {s : State} {pk : Party × Nat}
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

/-- The `walkKeys` converse: height below the root plus matching parity
IS membership. -/
theorem mem_walkKeys_of (hwf : sk.wellFormed = true) {p : Party} {k : Nat}
    (hk : k < sk.rootH)
    (hpar : (p = Party.I ∧ k % 2 = 1) ∨ (p = Party.R ∧ k % 2 = 0)) :
    (p, k) ∈ sk.walkKeys := by
  have hev : sk.rootH % 2 = 0 := (wf_rootH hwf).1
  simp only [Skel.walkKeys, List.mem_append, List.mem_map,
    List.mem_range]
  rcases hpar with ⟨rfl, hodd⟩ | ⟨rfl, heven⟩
  · refine Or.inl ⟨(sk.rootH - 1 - k) / 2, by omega, ?_⟩
    rw [Prod.mk.injEq]
    exact ⟨rfl, by omega⟩
  · refine Or.inr ⟨(sk.rootH - 2 - k) / 2, by omega, ?_⟩
    rw [Prod.mk.injEq]
    exact ⟨rfl, by omega⟩

/-- A walk's own four output channels and two inputs are flow channels. -/
theorem walk_chans_mem {pk : Party × Nat} (hpk : pk ∈ sk.walkKeys) :
    wireOut pk ∈ allChans sk ∧ askedIn pk ∈ allChans sk
      ∧ upperOut pk ∈ allChans sk ∧ lowerOut pk ∈ allChans sk := by
  refine ⟨?_, ?_, ?_, ?_⟩ <;>
    · unfold allChans
      refine List.mem_append.mpr (Or.inl (List.mem_append.mpr (Or.inl ?_)))
      refine List.mem_flatMap.mpr ⟨pk, hpk, ?_⟩
      simp

/-- The prologue wire channel is a flow channel: the walk above's
output, or a root wire. -/
theorem wireIn_mem_allChans (hwf : sk.wellFormed = true)
    {pk : Party × Nat} (hpk : pk ∈ sk.walkKeys) :
    wireIn pk ∈ allChans sk := by
  obtain ⟨p, k⟩ := pk
  obtain ⟨hkr, hpar⟩ := walkKeys_parity sk hwf hpk
  by_cases htop : k + 1 = sk.rootH
  · unfold allChans
    refine List.mem_append.mpr (Or.inr ?_)
    show wireIn (p, k) ∈ _
    unfold wireIn
    simp only [htop]
    rcases hpar with ⟨rfl, -⟩ | ⟨rfl, -⟩ <;> simp [Party.other]
  · have hup : (p.other, k + 1) ∈ sk.walkKeys := by
      refine mem_walkKeys_of sk hwf (by omega) ?_
      rcases hpar with ⟨rfl, hodd⟩ | ⟨rfl, heven⟩
      · exact Or.inr ⟨rfl, by omega⟩
      · exact Or.inl ⟨rfl, by omega⟩
    have : wireIn (p, k) = wireOut (p.other, k + 1) := by
      unfold wireIn wireOut
      rfl
    rw [this]
    exact (walk_chans_mem sk hup).1

/-- The query-out channel is a flow channel: the leaf-request stream or
the asked-in of the walk two stages down. -/
theorem askedOut_mem_allChans (hwf : sk.wellFormed = true)
    {pk : Party × Nat} (hpk : pk ∈ sk.walkKeys) (h1 : 1 ≤ pk.2) :
    askedOut pk ∈ allChans sk := by
  obtain ⟨p, k⟩ := pk
  obtain ⟨hkr, hpar⟩ := walkKeys_parity sk hwf hpk
  by_cases hlt : k < 2
  · unfold askedOut allChans
    rw [if_pos (by simpa using hlt)]
    refine List.mem_append.mpr (Or.inr ?_)
    simp
  · have hdn : (p, k - 2) ∈ sk.walkKeys := by
      refine mem_walkKeys_of sk hwf (by omega) ?_
      rcases hpar with ⟨rfl, hodd⟩ | ⟨rfl, heven⟩
      · exact Or.inl ⟨rfl, by omega⟩
      · exact Or.inr ⟨rfl, by omega⟩
    have : askedOut (p, k) = askedIn (p, k - 2) := by
      unfold askedOut askedIn
      rw [if_neg (by simpa using hlt)]
    rw [this]
    exact (walk_chans_mem sk hdn).2.1

-- ======================== the in-scope prefix performedness (chunks)

/-- Everything in the first `i` child chunks of the CURRENT scope is
performed, given the committed-arm discharge facts: `i` counted wires,
every D child below `i` resolved and at quota. This is the shared core
of all four committed-case splits. -/
theorem chunks_prefix_performed (hwf : sk.wellFormed = true) {s : State}
    {pk : Party × Nat} (hi : InvP sk .full s) (hpk : pk ∈ sk.walkKeys)
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
    phase2_child_facts sk hi hpk hph2
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

/-- The resolution count dominates the D-rank of any discharged
prefix. -/
theorem dRank_le_resCount {s : State} {pk : Party × Nat} {i : Nat}
    (hif : i ≤ sk.fan)
    (hdis : ∀ j, j < i →
      sk.childIsD pk.2 (sk.stageScope pk.2 (s.walk pk).scope) j = true →
      (s.walk pk).resDone j = true) :
    dRank sk pk (s.walk pk).scope i ≤ wkResCount sk s pk := by
  have h1 : ((List.range sk.fan).filter fun j =>
      decide (j < i) && sk.childIsD pk.2
        (sk.stageScope pk.2 (s.walk pk).scope) j).length
      ≤ ((List.range sk.fan).filter fun j =>
          (s.walk pk).resDone j).length := by
    refine length_filter_le_of_imp fun x _ hx => ?_
    simp only [Bool.and_eq_true, decide_eq_true_eq] at hx
    exact hdis x hx.1 hx.2
  rw [filter_range_and_lt hif] at h1
  simp only [wkResCount]
  exact h1

/-- A fired wire at `i` puts the wire count past `i` (prefix closure). -/
theorem wireCount_ge_succ {s : State} {pk : Party × Nat}
    (hi : InvP sk .full s) (hpk : pk ∈ sk.walkKeys)
    (hph2 : (s.walk pk).phase = 2) {i : Nat} (hif : i < sk.fan)
    (hw : (s.walk pk).wireDone i = true) :
    i + 1 ≤ wkWireCount sk s pk := by
  obtain ⟨-, hc1, -, -, -, -, -, -, -⟩ := phase2_child_facts sk hi hpk hph2
  have hclosed : ∀ j, j < sk.fan → (s.walk pk).wireDone j = true →
      j = 0 ∨ (s.walk pk).wireDone (j - 1) = true :=
    fun j hj hwj => (hc1 j hj hwj).2
  have hlow : ∀ j, j < i + 1 → (s.walk pk).wireDone j = true := by
    intro j hj
    exact fired_below hclosed hif hw j (by omega)
  simp only [wkWireCount]
  exact count_ge_of_prefix (by omega) hlow

/-- Map-flatten is flatMap, the form the chunk lemmas speak. -/
theorem flatten_map {α β : Type _} (l : List α) (F : α → List β) :
    (l.map F).flatten = l.flatMap F :=
  (List.flatMap_def ..).symm

/-- Sum of a pointwise-zero map. -/
theorem sum_map_zero {α : Type _} {f : α → Nat} :
    ∀ {l : List α}, (∀ x ∈ l, f x = 0) → (l.map f).sum = 0
  | [], _ => rfl
  | x :: xs, h => by
      simp only [List.map_cons, List.sum_cons,
        h x (List.mem_cons_self ..),
        sum_map_zero fun y hy => h y (List.mem_cons_of_mem _ hy)]

/-- `range` drops to a `range'` tail. -/
theorem drop_range {m n : Nat} (h : m ≤ n) :
    (List.range n).drop m = List.range' m (n - m) := by
  rw [range_split h, List.drop_left' (by rw [List.length_range])]

-- ============================================== the walk pending decode

/-- The walk's pending event and action, per phase: the prologue
receive it awaits, or the committed obligation's fire. Empty exactly at
choice points (phase-2 uncommitted) and past the channel work
(phase ≥ 3). -/
def wkPend (s : State) (pk : Party × Nat) : List (Ev × Action) :=
  let ws := s.walk pk
  if ws.phase = 0 then
    [((wireIn pk, false, ws.scope), .walkRecvWire pk)]
  else if ws.phase = 1 then
    [((askedIn pk, false, ws.scope), .walkRecvAsked pk)]
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

set_option maxHeartbeats 1000000 in
/-- The committed-case split: the in-scope prefix below the committed
obligation's event is performed, and the event carries the channel's
current count. This is where the amended guards earn their keep — the
`d5` mirrors force the parent into the performed prefix at exactly the
positions the trace splices it. -/
private theorem walk_committed_split (hwf : sk.wellFormed = true)
    {s : State} {pk : Party × Nat} (hi : InvP sk .full s)
    (hpk : pk ∈ sk.walkKeys) (hph2 : (s.walk pk).phase = 2)
    {o : Oblig} (hcm : (s.walk pk).committed = some o) :
    ∃ f isp ss,
      wkPend sk s pk = [(f, .walkFire pk)]
      ∧ scopeSends sk pk (s.walk pk).scope = isp ++ f :: ss
      ∧ (∀ e ∈ isp, performed sk s e)
      ∧ f.1 = obligChan pk o ∧ f.2.1 = true
      ∧ f.2.2 = sentOf sk s f.1
      ∧ f.1 ∈ allChans sk := by
  obtain ⟨hscope, hwbc, hqle, hresD, hres5, hresw, hqres, hq4, hw10⟩ :=
    phase2_child_facts sk hi hpk hph2
  have hn_fan : sk.nChildren pk.2 (sk.stageScope pk.2 (s.walk pk).scope)
      ≤ sk.fan := nChildren_le_fan hwf hscope
  have hwk := hi.wk pk hpk
  simp only [wkLocalOk] at hwk
  rw [hph2, hcm] at hwk
  -- shared chunk-membership transfer
  have hsub_of_lt : ∀ {j i' : Nat}, j < i' →
      ∀ e ∈ childChunk sk pk (s.walk pk).scope j,
      e ∈ (List.range i').flatMap (childChunk sk pk (s.walk pk).scope) :=
    fun {j i'} hj e he => List.mem_flatMap.mpr ⟨j, List.mem_range.mpr hj, he⟩
  cases o with
  | wire i =>
      simp [AxMode.full] at hwk
      obtain ⟨-, -, ⟨⟨hieq, hin⟩, hd4⟩, hd5⟩ := hwk
      have hdis : ∀ j, j < i →
          sk.childIsD pk.2 (sk.stageScope pk.2 (s.walk pk).scope) j = true →
          (s.walk pk).resDone j = true ∧ (s.walk pk).qSent j
            = sk.qCount pk.2 (sk.stageScope pk.2 (s.walk pk).scope) j := by
        intro j hj hD
        rcases hd4 j hj with hf | h
        · rw [hD] at hf; cases hf
        · exact h
      have hperf := chunks_prefix_performed sk hwf hi hpk hph2
        (show i ≤ _ by omega) (by omega) hdis
      have hpend : wkPend sk s pk = [((wireOut pk, true,
          sk.wiresBefore pk.2 (s.walk pk).scope + i), .walkFire pk)] := by
        simp [wkPend, hph2, hcm]
      have hseqf : sk.wiresBefore pk.2 (s.walk pk).scope + i
          = sentOf sk s (wireOut pk) := by
        rw [sentOf_wireOut hpk]
        unfold wkWireSent
        omega
      have hpd_of : (∀ j, j < sk.nChildren pk.2
          (sk.stageScope pk.2 (s.walk pk).scope) →
          sk.childIsD pk.2 (sk.stageScope pk.2 (s.walk pk).scope) j = true →
          (s.walk pk).resDone j = true) →
          (s.walk pk).parentDone = true := by
        intro hall2
        rcases hd5 with hpd | ⟨x, hx, hDx, hrx⟩
        · exact hpd
        · rw [hall2 x hx hDx] at hrx
          cases hrx
      cases hlast : ((List.range (sk.nChildren pk.2
          (sk.stageScope pk.2 (s.walk pk).scope))).filter fun i' =>
          sk.childIsD pk.2 (sk.stageScope pk.2 (s.walk pk).scope) i').getLast?
          with
      | none =>
          have hnoD : ∀ j, j < sk.nChildren pk.2
              (sk.stageScope pk.2 (s.walk pk).scope) →
              sk.childIsD pk.2 (sk.stageScope pk.2 (s.walk pk).scope) j
                = false := by
            intro j hj
            by_contra hD
            rw [Bool.not_eq_false] at hD
            have hm : j ∈ (List.range (sk.nChildren pk.2
                (sk.stageScope pk.2 (s.walk pk).scope))).filter fun i' =>
                sk.childIsD pk.2 (sk.stageScope pk.2 (s.walk pk).scope) i' :=
              List.mem_filter.2 ⟨List.mem_range.2 hj, hD⟩
            rw [List.getLast?_eq_none_iff.1 hlast] at hm
            cases hm
          have hpd := hpd_of fun j hj hD => absurd hD (by simp [hnoD j hj])
          refine ⟨(wireOut pk, true,
              sk.wiresBefore pk.2 (s.walk pk).scope + i),
            (upperOut pk, true, (s.walk pk).scope)
              :: (List.range i).flatMap
                (childChunk sk pk (s.walk pk).scope),
            (List.range' (i + 1) (sk.nChildren pk.2
              (sk.stageScope pk.2 (s.walk pk).scope) - i - 1)).flatMap
              (childChunk sk pk (s.walk pk).scope),
            hpend, ?_, ?_, rfl, rfl, hseqf, (walk_chans_mem sk hpk).1⟩
          · simp only [scopeSends, hlast]
            rw [flatten_map, range_split (show i ≤ _ by omega),
              List.flatMap_append,
              show sk.nChildren pk.2 (sk.stageScope pk.2 (s.walk pk).scope)
                  - i = (sk.nChildren pk.2
                    (sk.stageScope pk.2 (s.walk pk).scope) - i - 1) + 1
                from by omega,
              List.range'_succ, List.flatMap_cons,
              chunkR sk pk (s.walk pk).scope i (hnoD i hin)]
            simp [List.cons_append]
          · intro e he
            rcases List.mem_cons.1 he with rfl | he
            · show (s.walk pk).scope < sentOf sk s (upperOut pk)
              rw [sentOf_upperOut]
              simp only [wkParentSent]
              rw [if_pos (by simp [hph2, hpd])]
              omega
            · exact hperf e he
      | some jL =>
          have hjmem := List.mem_of_getLast? hlast
          rw [List.mem_filter, List.mem_range] at hjmem
          obtain ⟨hjLn, hjLD⟩ := hjmem
          have hget : ((List.range (sk.nChildren pk.2
              (sk.stageScope pk.2 (s.walk pk).scope))).map
              (childChunk sk pk (s.walk pk).scope)).getD jL []
              = childChunk sk pk (s.walk pk).scope jL := by
            rw [List.getD_eq_getElem?_getD, List.getElem?_map,
              List.getElem?_range hjLn]
            rfl
          have htake : ((List.range (sk.nChildren pk.2
              (sk.stageScope pk.2 (s.walk pk).scope))).map
              (childChunk sk pk (s.walk pk).scope)).take jL
              = (List.range jL).map (childChunk sk pk (s.walk pk).scope) := by
            rw [← List.map_take, List.take_range,
              Nat.min_eq_left (by omega)]
          have hdropfl : ((((List.range (sk.nChildren pk.2
              (sk.stageScope pk.2 (s.walk pk).scope))).map
              (childChunk sk pk (s.walk pk).scope)).drop (jL + 1)).flatten)
              = (List.range' (jL + 1) (sk.nChildren pk.2
                  (sk.stageScope pk.2 (s.walk pk).scope) - (jL + 1))).flatMap
                  (childChunk sk pk (s.walk pk).scope) := by
            rw [← List.map_drop, drop_range (by omega), flatten_map]
          rcases Nat.lt_or_ge i (jL + 1) with hij | hij
          · -- i ≤ jL: the wire precedes the splice; prefix is chunks < i
            rcases Nat.lt_or_ge i jL with hilt | hieq2
            · -- i < jL: the wire heads its own chunk inside the take
              refine ⟨(wireOut pk, true,
                  sk.wiresBefore pk.2 (s.walk pk).scope + i),
                (List.range i).flatMap (childChunk sk pk (s.walk pk).scope),
                (childChunk sk pk (s.walk pk).scope i).tail
                  ++ ((List.range' (i + 1) (jL - i - 1)).flatMap
                      (childChunk sk pk (s.walk pk).scope)
                    ++ ((childChunk sk pk (s.walk pk).scope jL).take 2
                      ++ (upperOut pk, true, (s.walk pk).scope)
                        :: ((childChunk sk pk (s.walk pk).scope jL).drop 2
                          ++ (List.range' (jL + 1) (sk.nChildren pk.2
                              (sk.stageScope pk.2 (s.walk pk).scope)
                                - (jL + 1))).flatMap
                              (childChunk sk pk (s.walk pk).scope)))),
                hpend, ?_, hperf,
                rfl, rfl, hseqf, (walk_chans_mem sk hpk).1⟩
              simp only [scopeSends, hlast]
              rw [htake, hget, flatten_map, hdropfl,
                range_split (show i ≤ jL by omega),
                List.flatMap_append,
                show jL - i = (jL - i - 1) + 1 from by omega,
                List.range'_succ, List.flatMap_cons]
              have hhead : childChunk sk pk (s.walk pk).scope i
                  = (wireOut pk, true,
                      sk.wiresBefore pk.2 (s.walk pk).scope + i)
                    :: (childChunk sk pk (s.walk pk).scope i).tail := by
                cases hD : sk.childIsD pk.2
                    (sk.stageScope pk.2 (s.walk pk).scope) i with
                | true => rw [chunkD sk pk (s.walk pk).scope i hD]; rfl
                | false => rw [chunkR sk pk (s.walk pk).scope i hD]; rfl
              conv => lhs; rw [hhead]
              simp [List.cons_append, List.append_assoc]
            · -- i = jL: the wire heads the take-2 pair
              have hieq3 : i = jL := by omega
              subst hieq3
              refine ⟨(wireOut pk, true,
                  sk.wiresBefore pk.2 (s.walk pk).scope + i),
                (List.range i).flatMap (childChunk sk pk (s.walk pk).scope),
                (lowerOut pk, true, sk.dsBefore pk.2 (s.walk pk).scope
                    + dRank sk pk (s.walk pk).scope i)
                  :: (upperOut pk, true, (s.walk pk).scope)
                  :: (seg (askedOut pk) true
                      (sk.qsBefore pk.2 (s.walk pk).scope
                        + qSum sk pk (s.walk pk).scope i)
                      (sk.qCount pk.2 (sk.stageScope pk.2 (s.walk pk).scope) i)
                    ++ (List.range' (i + 1) (sk.nChildren pk.2
                        (sk.stageScope pk.2 (s.walk pk).scope)
                          - (i + 1))).flatMap
                        (childChunk sk pk (s.walk pk).scope)),
                hpend, ?_, hperf,
                rfl, rfl, hseqf, (walk_chans_mem sk hpk).1⟩
              simp only [scopeSends, hlast]
              rw [htake, hget, flatten_map, hdropfl,
                chunkD sk pk (s.walk pk).scope i hjLD]
              simp [List.cons_append, List.append_assoc, seg]
          · -- jL < i: the wire is past the splice; the parent is owed
            have hnotDi : sk.childIsD pk.2
                (sk.stageScope pk.2 (s.walk pk).scope) i = false :=
              lastDOf_max sk (show lastDOf sk pk.2 (s.walk pk).scope
                = some jL from hlast) (by omega)
            have hpd : (s.walk pk).parentDone = true := by
              refine hpd_of fun j hj hD => ?_
              by_cases hji : j < i
              · exact (hdis j hji hD).1
              · have : sk.childIsD pk.2
                    (sk.stageScope pk.2 (s.walk pk).scope) j = false :=
                  lastDOf_max sk (show lastDOf sk pk.2 (s.walk pk).scope
                    = some jL from hlast) (by omega)
                rw [this] at hD
                cases hD
            refine ⟨(wireOut pk, true,
                sk.wiresBefore pk.2 (s.walk pk).scope + i),
              (List.range jL).flatMap (childChunk sk pk (s.walk pk).scope)
                ++ (childChunk sk pk (s.walk pk).scope jL).take 2
                ++ (upperOut pk, true, (s.walk pk).scope)
                  :: ((childChunk sk pk (s.walk pk).scope jL).drop 2
                    ++ (List.range' (jL + 1) (i - (jL + 1))).flatMap
                        (childChunk sk pk (s.walk pk).scope)),
              (List.range' (i + 1) (sk.nChildren pk.2
                  (sk.stageScope pk.2 (s.walk pk).scope) - i - 1)).flatMap
                  (childChunk sk pk (s.walk pk).scope),
              hpend, ?_, ?_,
              rfl, rfl, hseqf, (walk_chans_mem sk hpk).1⟩
            · simp only [scopeSends, hlast]
              rw [htake, hget, flatten_map, hdropfl,
                show sk.nChildren pk.2 (sk.stageScope pk.2 (s.walk pk).scope)
                    - (jL + 1) = (i - (jL + 1))
                      + (sk.nChildren pk.2
                        (sk.stageScope pk.2 (s.walk pk).scope) - i)
                  from by omega,
                ← List.range'_append, List.flatMap_append,
                show jL + 1 + 1 * (i - (jL + 1)) = i from by omega,
                show sk.nChildren pk.2 (sk.stageScope pk.2 (s.walk pk).scope)
                    - i = (sk.nChildren pk.2
                      (sk.stageScope pk.2 (s.walk pk).scope) - i - 1) + 1
                  from by omega,
                List.range'_succ, List.flatMap_cons,
                chunkR sk pk (s.walk pk).scope i hnotDi]
              simp [List.cons_append, List.append_assoc]
            · intro e he
              rcases List.mem_append.1 he with hL | hR
              rcases List.mem_append.1 hL with hjls | htk2
              · obtain ⟨j, hjm, hje⟩ := List.mem_flatMap.1 hjls
                rw [List.mem_range] at hjm
                exact hperf e (hsub_of_lt (by omega) e hje)
              · exact hperf e (hsub_of_lt (by omega) e
                  (List.mem_of_mem_take htk2))
              rcases List.mem_cons.1 hR with rfl | hR2
              · show (s.walk pk).scope < sentOf sk s (upperOut pk)
                rw [sentOf_upperOut]
                simp only [wkParentSent]
                rw [if_pos (by simp [hph2, hpd])]
                omega
              rcases List.mem_append.1 hR2 with hdp | hmid
              · exact hperf e (hsub_of_lt (by omega) e
                  (List.mem_of_mem_drop hdp))
              · obtain ⟨j, hjm, hje⟩ := List.mem_flatMap.1 hmid
                have := List.mem_range'_1.1 hjm
                exact hperf e (hsub_of_lt (by omega) e hje)
  | res i =>
      simp [AxMode.full] at hwk
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
      have hwc := wireCount_ge_succ sk hi hpk hph2
        (show i < sk.fan by omega) hwi
      have hperf := chunks_prefix_performed sk hwf hi hpk hph2
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
      cases hlast : ((List.range (sk.nChildren pk.2
          (sk.stageScope pk.2 (s.walk pk).scope))).filter fun i' =>
          sk.childIsD pk.2 (sk.stageScope pk.2 (s.walk pk).scope) i').getLast?
          with
      | none =>
          exfalso
          have hm : i ∈ (List.range (sk.nChildren pk.2
              (sk.stageScope pk.2 (s.walk pk).scope))).filter fun i' =>
              sk.childIsD pk.2 (sk.stageScope pk.2 (s.walk pk).scope) i' :=
            List.mem_filter.2 ⟨List.mem_range.2 hin, hDi⟩
          rw [List.getLast?_eq_none_iff.1 hlast] at hm
          cases hm
      | some jL =>
          have hjmem := List.mem_of_getLast? hlast
          rw [List.mem_filter, List.mem_range] at hjmem
          obtain ⟨hjLn, hjLD⟩ := hjmem
          have hijL : i ≤ jL := by
            by_contra hgt
            have := lastDOf_max sk (show lastDOf sk pk.2 (s.walk pk).scope
              = some jL from hlast) (show jL < i by omega)
            rw [this] at hDi
            cases hDi
          have hget : ((List.range (sk.nChildren pk.2
              (sk.stageScope pk.2 (s.walk pk).scope))).map
              (childChunk sk pk (s.walk pk).scope)).getD jL []
              = childChunk sk pk (s.walk pk).scope jL := by
            rw [List.getD_eq_getElem?_getD, List.getElem?_map,
              List.getElem?_range hjLn]
            rfl
          have htake : ((List.range (sk.nChildren pk.2
              (sk.stageScope pk.2 (s.walk pk).scope))).map
              (childChunk sk pk (s.walk pk).scope)).take jL
              = (List.range jL).map (childChunk sk pk (s.walk pk).scope) := by
            rw [← List.map_take, List.take_range,
              Nat.min_eq_left (by omega)]
          have hdropfl : ((((List.range (sk.nChildren pk.2
              (sk.stageScope pk.2 (s.walk pk).scope))).map
              (childChunk sk pk (s.walk pk).scope)).drop (jL + 1)).flatten)
              = (List.range' (jL + 1) (sk.nChildren pk.2
                  (sk.stageScope pk.2 (s.walk pk).scope) - (jL + 1))).flatMap
                  (childChunk sk pk (s.walk pk).scope) := by
            rw [← List.map_drop, drop_range (by omega), flatten_map]
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
          rcases Nat.lt_or_ge i jL with hilt | hieq2
          · refine ⟨(lowerOut pk, true,
                sk.dsBefore pk.2 (s.walk pk).scope
                  + dRank sk pk (s.walk pk).scope i),
              (List.range i).flatMap (childChunk sk pk (s.walk pk).scope)
                ++ [(wireOut pk, true,
                    sk.wiresBefore pk.2 (s.walk pk).scope + i)],
              seg (askedOut pk) true
                  (sk.qsBefore pk.2 (s.walk pk).scope
                    + qSum sk pk (s.walk pk).scope i)
                  (sk.qCount pk.2 (sk.stageScope pk.2 (s.walk pk).scope) i)
                ++ ((List.range' (i + 1) (jL - i - 1)).flatMap
                    (childChunk sk pk (s.walk pk).scope)
                  ++ ((childChunk sk pk (s.walk pk).scope jL).take 2
                    ++ (upperOut pk, true, (s.walk pk).scope)
                      :: ((childChunk sk pk (s.walk pk).scope jL).drop 2
                        ++ (List.range' (jL + 1) (sk.nChildren pk.2
                            (sk.stageScope pk.2 (s.walk pk).scope)
                              - (jL + 1))).flatMap
                            (childChunk sk pk (s.walk pk).scope)))),
              hpend, ?_, hprefperf,
              rfl, rfl, hseqf, (walk_chans_mem sk hpk).2.2.2⟩
            simp only [scopeSends, hlast]
            rw [htake, hget, flatten_map, hdropfl,
              range_split (show i ≤ jL by omega),
              List.flatMap_append,
              show jL - i = (jL - i - 1) + 1 from by omega,
              List.range'_succ, List.flatMap_cons,
              chunkD sk pk (s.walk pk).scope i hDi]
            simp [List.cons_append, List.append_assoc]
          · have hieq3 : i = jL := by omega
            subst hieq3
            refine ⟨(lowerOut pk, true,
                sk.dsBefore pk.2 (s.walk pk).scope
                  + dRank sk pk (s.walk pk).scope i),
              (List.range i).flatMap (childChunk sk pk (s.walk pk).scope)
                ++ [(wireOut pk, true,
                    sk.wiresBefore pk.2 (s.walk pk).scope + i)],
              (upperOut pk, true, (s.walk pk).scope)
                :: (seg (askedOut pk) true
                    (sk.qsBefore pk.2 (s.walk pk).scope
                      + qSum sk pk (s.walk pk).scope i)
                    (sk.qCount pk.2 (sk.stageScope pk.2 (s.walk pk).scope) i)
                  ++ (List.range' (i + 1) (sk.nChildren pk.2
                      (sk.stageScope pk.2 (s.walk pk).scope) - (i + 1))).flatMap
                      (childChunk sk pk (s.walk pk).scope)),
              hpend, ?_, hprefperf,
              rfl, rfl, hseqf, (walk_chans_mem sk hpk).2.2.2⟩
            simp only [scopeSends, hlast]
            rw [htake, hget, flatten_map, hdropfl,
              chunkD sk pk (s.walk pk).scope i hjLD]
            simp [List.cons_append, List.append_assoc]
  | query i =>
      simp [AxMode.full] at hwk
      obtain ⟨-, -, ⟨⟨⟨⟨hin, hDi⟩, hqlt⟩, hqpre⟩, hres⟩, hd5⟩ := hwk
      have h1 : 1 ≤ pk.2 := by
        cases hp2 : pk.2 with
        | zero => rw [hp2] at hDi; simp [Skel.childIsD] at hDi
        | succ m => omega
      have hwi : (s.walk pk).wireDone i = true :=
        hresw i (by omega) hres
      have hwc := wireCount_ge_succ sk hi hpk hph2
        (show i < sk.fan by omega) hwi
      have hdis : ∀ j, j < i →
          sk.childIsD pk.2 (sk.stageScope pk.2 (s.walk pk).scope) j = true →
          (s.walk pk).resDone j = true ∧ (s.walk pk).qSent j
            = sk.qCount pk.2 (sk.stageScope pk.2 (s.walk pk).scope) j :=
        fun j hj hD => ⟨hres5 i (by omega) hres j hj hD, hqpre j hj⟩
      have hperf := chunks_prefix_performed sk hwf hi hpk hph2
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
      cases hlast : ((List.range (sk.nChildren pk.2
          (sk.stageScope pk.2 (s.walk pk).scope))).filter fun i' =>
          sk.childIsD pk.2 (sk.stageScope pk.2 (s.walk pk).scope) i').getLast?
          with
      | none =>
          exfalso
          have hm : i ∈ (List.range (sk.nChildren pk.2
              (sk.stageScope pk.2 (s.walk pk).scope))).filter fun i' =>
              sk.childIsD pk.2 (sk.stageScope pk.2 (s.walk pk).scope) i' :=
            List.mem_filter.2 ⟨List.mem_range.2 hin, hDi⟩
          rw [List.getLast?_eq_none_iff.1 hlast] at hm
          cases hm
      | some jL =>
          have hjmem := List.mem_of_getLast? hlast
          rw [List.mem_filter, List.mem_range] at hjmem
          obtain ⟨hjLn, hjLD⟩ := hjmem
          have hijL : i ≤ jL := by
            by_contra hgt
            have := lastDOf_max sk (show lastDOf sk pk.2 (s.walk pk).scope
              = some jL from hlast) (show jL < i by omega)
            rw [this] at hDi
            cases hDi
          have hget : ((List.range (sk.nChildren pk.2
              (sk.stageScope pk.2 (s.walk pk).scope))).map
              (childChunk sk pk (s.walk pk).scope)).getD jL []
              = childChunk sk pk (s.walk pk).scope jL := by
            rw [List.getD_eq_getElem?_getD, List.getElem?_map,
              List.getElem?_range hjLn]
            rfl
          have htake : ((List.range (sk.nChildren pk.2
              (sk.stageScope pk.2 (s.walk pk).scope))).map
              (childChunk sk pk (s.walk pk).scope)).take jL
              = (List.range jL).map (childChunk sk pk (s.walk pk).scope) := by
            rw [← List.map_take, List.take_range,
              Nat.min_eq_left (by omega)]
          have hdropfl : ((((List.range (sk.nChildren pk.2
              (sk.stageScope pk.2 (s.walk pk).scope))).map
              (childChunk sk pk (s.walk pk).scope)).drop (jL + 1)).flatten)
              = (List.range' (jL + 1) (sk.nChildren pk.2
                  (sk.stageScope pk.2 (s.walk pk).scope) - (jL + 1))).flatMap
                  (childChunk sk pk (s.walk pk).scope) := by
            rw [← List.map_drop, drop_range (by omega), flatten_map]
          rcases Nat.lt_or_ge i jL with hilt | hieq2
          · refine ⟨(askedOut pk, true,
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
                    + qSum sk pk (s.walk pk).scope i
                    + (s.walk pk).qSent i + 1)
                  (sk.qCount pk.2 (sk.stageScope pk.2 (s.walk pk).scope) i
                    - (s.walk pk).qSent i - 1)
                ++ ((List.range' (i + 1) (jL - i - 1)).flatMap
                    (childChunk sk pk (s.walk pk).scope)
                  ++ ((childChunk sk pk (s.walk pk).scope jL).take 2
                    ++ (upperOut pk, true, (s.walk pk).scope)
                      :: ((childChunk sk pk (s.walk pk).scope jL).drop 2
                        ++ (List.range' (jL + 1) (sk.nChildren pk.2
                            (sk.stageScope pk.2 (s.walk pk).scope)
                              - (jL + 1))).flatMap
                            (childChunk sk pk (s.walk pk).scope)))),
              hpend, ?_, hprefperf,
              rfl, rfl, hseqf, askedOut_mem_allChans sk hwf hpk h1⟩
            simp only [scopeSends, hlast]
            rw [htake, hget, flatten_map, hdropfl,
              range_split (show i ≤ jL by omega),
              List.flatMap_append,
              show jL - i = (jL - i - 1) + 1 from by omega,
              List.range'_succ, List.flatMap_cons,
              chunkD sk pk (s.walk pk).scope i hDi, hsegsplit]
            simp [List.cons_append, List.append_assoc]
          · have hieq3 : i = jL := by omega
            subst hieq3
            have hpd : (s.walk pk).parentDone = true := by
              rcases hd5 with hpd | ⟨x, hx, hDx, hrx⟩
              · exact hpd
              · exfalso
                have hxle : x ≤ i := by
                  by_contra hgt
                  have := lastDOf_max sk (i := x) (show lastDOf sk pk.2
                    (s.walk pk).scope = some i from hlast)
                    (show i < x by omega)
                  rw [this] at hDx
                  cases hDx
                rcases Nat.lt_or_ge x i with hlt | hge
                · rw [(hdis x hlt hDx).1] at hrx
                  cases hrx
                · have : x = i := by omega
                  subst this
                  rw [hres] at hrx
                  cases hrx
            refine ⟨(askedOut pk, true,
                sk.qsBefore pk.2 (s.walk pk).scope
                  + qSum sk pk (s.walk pk).scope i + (s.walk pk).qSent i),
              (List.range i).flatMap (childChunk sk pk (s.walk pk).scope)
                ++ ((wireOut pk, true,
                    sk.wiresBefore pk.2 (s.walk pk).scope + i)
                  :: (lowerOut pk, true, sk.dsBefore pk.2 (s.walk pk).scope
                      + dRank sk pk (s.walk pk).scope i)
                  :: (upperOut pk, true, (s.walk pk).scope)
                  :: seg (askedOut pk) true
                    (sk.qsBefore pk.2 (s.walk pk).scope
                      + qSum sk pk (s.walk pk).scope i) ((s.walk pk).qSent i)),
              seg (askedOut pk) true
                  (sk.qsBefore pk.2 (s.walk pk).scope
                    + qSum sk pk (s.walk pk).scope i
                    + (s.walk pk).qSent i + 1)
                  (sk.qCount pk.2 (sk.stageScope pk.2 (s.walk pk).scope) i
                    - (s.walk pk).qSent i - 1)
                ++ (List.range' (i + 1) (sk.nChildren pk.2
                    (sk.stageScope pk.2 (s.walk pk).scope) - (i + 1))).flatMap
                    (childChunk sk pk (s.walk pk).scope),
              hpend, ?_, ?_,
              rfl, rfl, hseqf, askedOut_mem_allChans sk hwf hpk h1⟩
            · simp only [scopeSends, hlast]
              rw [htake, hget, flatten_map, hdropfl,
                chunkD sk pk (s.walk pk).scope i hjLD, hsegsplit]
              simp [List.cons_append, List.append_assoc]
            · intro e he
              rcases List.mem_append.1 he with hfm | hcons
              · exact hperf e hfm
              rcases List.mem_cons.1 hcons with rfl | hcons
              · show sk.wiresBefore pk.2 (s.walk pk).scope + i
                    < sentOf sk s (wireOut pk)
                rw [sentOf_wireOut hpk]
                unfold wkWireSent
                omega
              rcases List.mem_cons.1 hcons with rfl | hcons
              · show sk.dsBefore pk.2 (s.walk pk).scope
                    + dRank sk pk (s.walk pk).scope i
                    < sentOf sk s (lowerOut pk)
                rw [sentOf_lowerOut]
                unfold wkResSent
                omega
              rcases List.mem_cons.1 hcons with rfl | hseg
              · show (s.walk pk).scope < sentOf sk s (upperOut pk)
                rw [sentOf_upperOut]
                simp only [wkParentSent]
                rw [if_pos (by simp [hph2, hpd])]
                omega
              · exact hsegperf e hseg
  | parent =>
      simp [AxMode.full] at hwk
      obtain ⟨-, -, hpd, hd2⟩ := hwk
      have hd2' : ∀ j, j < sk.nChildren pk.2
          (sk.stageScope pk.2 (s.walk pk).scope) →
          sk.childIsD pk.2 (sk.stageScope pk.2 (s.walk pk).scope) j = true →
          (s.walk pk).resDone j = true := by
        intro j hj hD
        rcases hd2 j hj with hf | h
        · rw [hD] at hf; cases hf
        · exact h
      have hpend : wkPend sk s pk = [((upperOut pk, true,
          (s.walk pk).scope), .walkFire pk)] := by
        simp [wkPend, hph2, hcm]
      have hseqf : (s.walk pk).scope = sentOf sk s (upperOut pk) := by
        rw [sentOf_upperOut]
        simp only [wkParentSent]
        rw [if_neg (by simp [hpd])]
        omega
      cases hlast : ((List.range (sk.nChildren pk.2
          (sk.stageScope pk.2 (s.walk pk).scope))).filter fun i' =>
          sk.childIsD pk.2 (sk.stageScope pk.2 (s.walk pk).scope) i').getLast?
          with
      | none =>
          refine ⟨(upperOut pk, true, (s.walk pk).scope), [],
            ((List.range (sk.nChildren pk.2
                (sk.stageScope pk.2 (s.walk pk).scope))).map
                (childChunk sk pk (s.walk pk).scope)).flatten,
            hpend, ?_, ?_,
            rfl, rfl, hseqf, (walk_chans_mem sk hpk).2.2.1⟩
          · simp only [scopeSends, hlast]
            rfl
          · intro e he
            cases he
      | some jL =>
          have hjmem := List.mem_of_getLast? hlast
          rw [List.mem_filter, List.mem_range] at hjmem
          obtain ⟨hjLn, hjLD⟩ := hjmem
          have hwjL : (s.walk pk).wireDone jL = true :=
            hresw jL (by omega) (hd2' jL hjLn hjLD)
          have hwc := wireCount_ge_succ sk hi hpk hph2
            (show jL < sk.fan by omega) hwjL
          have hdis : ∀ j, j < jL →
              sk.childIsD pk.2 (sk.stageScope pk.2 (s.walk pk).scope) j
                = true →
              (s.walk pk).resDone j = true ∧ (s.walk pk).qSent j
                = sk.qCount pk.2 (sk.stageScope pk.2 (s.walk pk).scope) j :=
            fun j hj hD => hw10 jL (by omega) hwjL j hj hD
          have hperf := chunks_prefix_performed sk hwf hi hpk hph2
            (show jL ≤ _ by omega) (by omega) hdis
          have hresperf : dRank sk pk (s.walk pk).scope jL
              < wkResCount sk s pk := by
            have hd' : ∀ j, j < jL + 1 →
                sk.childIsD pk.2 (sk.stageScope pk.2 (s.walk pk).scope) j
                  = true →
                (s.walk pk).resDone j = true :=
              fun j hj hD => hd2' j (by omega) hD
            have := dRank_le_resCount sk (show jL + 1 ≤ sk.fan by omega) hd'
            have hstep : dRank sk pk (s.walk pk).scope (jL + 1)
                = dRank sk pk (s.walk pk).scope jL + 1 := by
              rw [dRank_succ, if_pos hjLD]
            omega
          have hget : ((List.range (sk.nChildren pk.2
              (sk.stageScope pk.2 (s.walk pk).scope))).map
              (childChunk sk pk (s.walk pk).scope)).getD jL []
              = childChunk sk pk (s.walk pk).scope jL := by
            rw [List.getD_eq_getElem?_getD, List.getElem?_map,
              List.getElem?_range hjLn]
            rfl
          have htake : ((List.range (sk.nChildren pk.2
              (sk.stageScope pk.2 (s.walk pk).scope))).map
              (childChunk sk pk (s.walk pk).scope)).take jL
              = (List.range jL).map (childChunk sk pk (s.walk pk).scope) := by
            rw [← List.map_take, List.take_range,
              Nat.min_eq_left (by omega)]
          have hdropfl : ((((List.range (sk.nChildren pk.2
              (sk.stageScope pk.2 (s.walk pk).scope))).map
              (childChunk sk pk (s.walk pk).scope)).drop (jL + 1)).flatten)
              = (List.range' (jL + 1) (sk.nChildren pk.2
                  (sk.stageScope pk.2 (s.walk pk).scope) - (jL + 1))).flatMap
                  (childChunk sk pk (s.walk pk).scope) := by
            rw [← List.map_drop, drop_range (by omega), flatten_map]
          refine ⟨(upperOut pk, true, (s.walk pk).scope),
            (List.range jL).flatMap (childChunk sk pk (s.walk pk).scope)
              ++ [(wireOut pk, true,
                  sk.wiresBefore pk.2 (s.walk pk).scope + jL),
                (lowerOut pk, true, sk.dsBefore pk.2 (s.walk pk).scope
                  + dRank sk pk (s.walk pk).scope jL)],
            seg (askedOut pk) true
                (sk.qsBefore pk.2 (s.walk pk).scope
                  + qSum sk pk (s.walk pk).scope jL)
                (sk.qCount pk.2 (sk.stageScope pk.2 (s.walk pk).scope) jL)
              ++ (List.range' (jL + 1) (sk.nChildren pk.2
                  (sk.stageScope pk.2 (s.walk pk).scope) - (jL + 1))).flatMap
                  (childChunk sk pk (s.walk pk).scope),
            hpend, ?_, ?_,
            rfl, rfl, hseqf, (walk_chans_mem sk hpk).2.2.1⟩
          · simp only [scopeSends, hlast]
            rw [htake, hget, flatten_map, hdropfl,
              chunkD sk pk (s.walk pk).scope jL hjLD]
            simp [List.cons_append, List.append_assoc]
          · intro e he
            rcases List.mem_append.1 he with hfm | hcons
            · exact hperf e hfm
            rcases List.mem_cons.1 hcons with rfl | hcons
            · show sk.wiresBefore pk.2 (s.walk pk).scope + jL
                  < sentOf sk s (wireOut pk)
              rw [sentOf_wireOut hpk]
              unfold wkWireSent
              omega
            rcases List.mem_cons.1 hcons with rfl | hnil
            · show sk.dsBefore pk.2 (s.walk pk).scope
                  + dRank sk pk (s.walk pk).scope jL
                  < sentOf sk s (lowerOut pk)
              rw [sentOf_lowerOut]
              unfold wkResSent
              omega
            · cases hnil

/-- The walk decode: past its channel work with everything performed,
or holding one pending event with the trace prefix below it performed.
Choice points (phase-2 uncommitted) are excluded — the pillar owns
them. -/
theorem walk_pend_or_done (hwf : sk.wellFormed = true) {s : State}
    {pk : Party × Nat} (hi : InvP sk .full s) (hpk : pk ∈ sk.walkKeys)
    (hnc : ¬((s.walk pk).phase = 2 ∧ (s.walk pk).committed = none)) :
    ((∀ e ∈ walkEvents sk pk, performed sk s e) ∧ wkPend sk s pk = [])
    ∨ ∃ f a pre suf, wkPend sk s pk = [(f, a)]
        ∧ walkEvents sk pk = pre ++ f :: suf
        ∧ (∀ e ∈ pre, performed sk s e)
        ∧ PendOk sk s f a := by
  by_cases hph3 : 3 ≤ (s.walk pk).phase
  · -- past the channel work: every block is a completed scope
    left
    constructor
    · intro e he
      obtain ⟨j, hjr, hje⟩ := List.mem_flatMap.1 he
      rw [List.mem_range] at hjr
      have hsc := (walk_scope_bound sk hi hpk).2 hph3
      exact scopeBlock_performed sk hwf hi hpk (by omega) hjr e hje
    · unfold wkPend
      rw [if_neg (by omega), if_neg (by omega), if_neg (by omega)]
  · have hsc := (walk_scope_bound sk hi hpk).1 (by omega)
    -- the shared outer split at the current scope
    have houter : walkEvents sk pk
        = (List.range (s.walk pk).scope).flatMap (scopeBlock sk pk)
          ++ scopeBlock sk pk (s.walk pk).scope
          ++ (List.range' ((s.walk pk).scope + 1)
              (sk.stageLen pk.2 - (s.walk pk).scope - 1)).flatMap
              (scopeBlock sk pk) := by
      unfold walkEvents
      rw [range_split (show (s.walk pk).scope ≤ sk.stageLen pk.2
        by omega), List.flatMap_append]
      have hlen : sk.stageLen pk.2 - (s.walk pk).scope
          = (sk.stageLen pk.2 - (s.walk pk).scope - 1) + 1 := by omega
      rw [hlen, List.range'_succ, List.flatMap_cons]
      simp [List.append_assoc]
    have hprepre : ∀ e ∈ (List.range (s.walk pk).scope).flatMap
        (scopeBlock sk pk), performed sk s e := by
      intro e he
      obtain ⟨j, hjr, hje⟩ := List.mem_flatMap.1 he
      rw [List.mem_range] at hjr
      exact scopeBlock_performed sk hwf hi hpk hjr (by omega) e hje
    rcases Nat.lt_or_ge (s.walk pk).phase 2 with hph01 | hph2'
    · -- a prologue receive is pending
      right
      rcases Nat.lt_or_ge (s.walk pk).phase 1 with hph0 | hph1
      · have hph : (s.walk pk).phase = 0 := by omega
        refine ⟨(wireIn pk, false, (s.walk pk).scope), .walkRecvWire pk,
          (List.range (s.walk pk).scope).flatMap (scopeBlock sk pk),
          ((askedIn pk, false, (s.walk pk).scope) ::
            scopeSends sk pk (s.walk pk).scope)
            ++ (List.range' ((s.walk pk).scope + 1)
                (sk.stageLen pk.2 - (s.walk pk).scope - 1)).flatMap
                (scopeBlock sk pk),
          ?_, ?_, hprepre, ?_, ?_, ?_, ?_⟩
        · unfold wkPend
          rw [if_pos hph]
        · rw [houter]
          unfold scopeBlock
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
          have happ : (apply sk .full (.walkRecvWire pk) s).isSome
              = true := by
            simp [apply, hpk, hph]
            omega
          exact happ
      · have hph : (s.walk pk).phase = 1 := by omega
        refine ⟨(askedIn pk, false, (s.walk pk).scope), .walkRecvAsked pk,
          (List.range (s.walk pk).scope).flatMap (scopeBlock sk pk)
            ++ [(wireIn pk, false, (s.walk pk).scope)],
          scopeSends sk pk (s.walk pk).scope
            ++ (List.range' ((s.walk pk).scope + 1)
                (sk.stageLen pk.2 - (s.walk pk).scope - 1)).flatMap
                (scopeBlock sk pk),
          ?_, ?_, ?_, ?_, ?_, ?_, ?_⟩
        · unfold wkPend
          rw [if_neg (by omega), if_pos hph]
        · rw [houter]
          unfold scopeBlock
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
          have happ : (apply sk .full (.walkRecvAsked pk) s).isSome
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
            walk_committed_split sk hwf hi hpk hph2 hcm
          refine ⟨f, .walkFire pk,
            (List.range (s.walk pk).scope).flatMap (scopeBlock sk pk)
              ++ (wireIn pk, false, (s.walk pk).scope)
              :: (askedIn pk, false, (s.walk pk).scope) :: isp,
            ss ++ (List.range' ((s.walk pk).scope + 1)
                (sk.stageLen pk.2 - (s.walk pk).scope - 1)).flatMap
                (scopeBlock sk pk),
            hpend, ?_, ?_, hmem, ?_, ?_, ?_⟩
          · rw [houter]
            unfold scopeBlock
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
            have happ : (apply sk .full (.walkFire pk) s).isSome
                = true := by
              simp [apply, hcm, hpk, hph2, hlt]
            exact happ

-- =========================================== the small-family decodes

/-- The initiator opening's pending fire. -/
def ioPend (s : State) : List (Ev × Action) :=
  match s.iopenCh with
  | some .wire => [((Chan.wire Party.I sk.rootH, true, 0), .iopenFire)]
  | some .query =>
      [((Chan.asked Party.I (sk.rootH - 1), true, 0), .iopenFire)]
  | none => []

/-- The responder opening's pending receive or fire. -/
def roPend (s : State) : List (Ev × Action) :=
  if s.ropenGotWire = false then
    [((Chan.wire Party.I sk.rootH, false, 0), .ropenRecv)]
  else match s.ropenCh with
  | some .wire => [((Chan.wire Party.R sk.rootH, true, 0), .ropenFire)]
  | some .res => [((Chan.rootres, true, 0), .ropenFire)]
  | some .query =>
      [((Chan.asked Party.R (sk.rootH - 2), true, s.ropenQ), .ropenFire)]
  | none => []

/-- The absorber's pending operation, by phase. -/
def abPend (s : State) : List (Ev × Action) :=
  if s.absorbPhase = 0 then
    [((Chan.wire Party.R 0, false, s.absorbIdx), .absorbRecvWire)]
  else if s.absorbPhase = 1 then
    [((Chan.leafRequests, false, s.absorbIdx), .absorbRecvAsked)]
  else if s.absorbPhase = 2 then
    [((Chan.level Party.I 0, true, s.absorbIdx), .absorbSend)]
  else []

/-- An assembler's pending operation, by phase. -/
def asmPend (s : State) (pk : Party × Nat) : List (Ev × Action) :=
  let a := s.asm pk
  if a.phase = 0 then [((asmResChan pk, false, a.idx), .asmRecvRes pk)]
  else if a.phase = 1 then
    [((asmLevelChan pk, false,
        sk.pendsBefore pk.1 pk.2 a.idx + a.got), .asmRecvLevel pk)]
  else if a.phase = 2 then
    [((sk.asmOutChan pk, true, a.idx), .asmSend pk)]
  else []

/-- The floating root-return receive. -/
def rrPend (s : State) : List (Ev × Action) :=
  if s.ifin = false then [((Chan.rootret, false, 0), .finRet)] else []

/-- The responder finish's pending receive. -/
def finPend (s : State) : List (Ev × Action) :=
  if s.rfinGotRes = false then [((Chan.rootres, false, 0), .finRes)]
  else if s.rfinGot < sk.rootPending then
    [((Chan.rootrets, false, s.rfinGot), .finRets)]
  else []

/-- The seven root-level channels are flow channels. -/
theorem root_chans_mem : Chan.wire Party.I sk.rootH ∈ allChans sk
    ∧ Chan.wire Party.R sk.rootH ∈ allChans sk
    ∧ Chan.leafRequests ∈ allChans sk
    ∧ Chan.level Party.I 0 ∈ allChans sk
    ∧ Chan.rootret ∈ allChans sk ∧ Chan.rootrets ∈ allChans sk
    ∧ Chan.rootres ∈ allChans sk := by
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_⟩ <;>
    · unfold allChans
      refine List.mem_append.mpr (Or.inr ?_)
      simp

/-- The initiator opening decode. -/
theorem iopen_pend_or_done (hwf : sk.wellFormed = true) {s : State}
    (hi : InvP sk .full s)
    (hch : s.iopenCh = none → doneIOpen s = true) :
    ((∀ e ∈ iopenEvents sk, performed sk s e) ∧ ioPend sk s = [])
    ∨ ∃ f a pre suf, ioPend sk s = [(f, a)]
        ∧ iopenEvents sk = pre ++ f :: suf
        ∧ (∀ e ∈ pre, performed sk s e)
        ∧ PendOk sk s f a := by
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
            have : (apply sk .full .iopenFire s).isSome = true := by
              simp only [apply, hc]
              rw [if_pos (by simpa [Skel.cap] using hchan)]
              rfl
            exact this
      | query =>
          have hq2 : s.iopenQuery = false ∧ s.iopenWire = true := by
            rw [hc] at hcq
            have := by simpa [AxMode.full] using hcq
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
            have : (apply sk .full .iopenFire s).isSome = true := by
              simp only [apply, hc]
              rw [if_pos (by simpa [Skel.cap] using hchan)]
              rfl
            exact this

/-- The floating root-return decode. -/
theorem rootret_pend_or_done {s : State} :
    ((∀ e ∈ [((Chan.rootret, false, 0) : Ev)], performed sk s e)
      ∧ rrPend s = [])
    ∨ ∃ f a pre suf, rrPend s = [(f, a)]
        ∧ [((Chan.rootret, false, 0) : Ev)] = pre ++ f :: suf
        ∧ (∀ e ∈ pre, performed sk s e)
        ∧ PendOk sk s f a := by
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
        have : (apply sk .full .finRet s).isSome = true := by
          simp [apply, hf]
          omega
        exact this

/-- The responder finish decode. -/
theorem fin_pend_or_done {s : State} (hi : InvP sk .full s) :
    ((∀ e ∈ finEvents sk, performed sk s e) ∧ finPend sk s = [])
    ∨ ∃ f a pre suf, finPend sk s = [(f, a)]
        ∧ finEvents sk = pre ++ f :: suf
        ∧ (∀ e ∈ pre, performed sk s e)
        ∧ PendOk sk s f a := by
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
        have : (apply sk .full .finRes s).isSome = true := by
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
          have : (apply sk .full .finRets s).isSome = true := by
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
theorem ropen_pend_or_done (hwf : sk.wellFormed = true) {s : State}
    (hi : InvP sk .full s)
    (hch : s.ropenGotWire = true → s.ropenCh = none →
      doneROpen sk s = true) :
    ((∀ e ∈ ropenEvents sk, performed sk s e) ∧ roPend sk s = [])
    ∨ ∃ f a pre suf, roPend sk s = [(f, a)]
        ∧ ropenEvents sk = pre ++ f :: suf
        ∧ (∀ e ∈ pre, performed sk s e)
        ∧ PendOk sk s f a := by
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
        have : (apply sk .full .ropenRecv s).isSome = true := by
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
                have : (apply sk .full .ropenFire s).isSome = true := by
                  simp only [apply, hc]
                  rw [if_pos (by simpa [Skel.cap] using hchan)]
                  rfl
                exact this
          | res =>
              have hnr : s.ropenRes = false ∧ s.ropenWire = true := by
                rw [hc] at hcr
                have := by simpa [AxMode.full] using hcr
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
                have : (apply sk .full .ropenFire s).isSome = true := by
                  simp only [apply, hc]
                  rw [if_pos (by simpa [Skel.cap] using hchan)]
                  rfl
                exact this
          | query =>
              have hq3 : s.ropenQ < sk.rootPending ∧ s.ropenRes = true := by
                rw [hc] at hcq
                have := by simpa [AxMode.full] using hcq
                exact this
              have hwtrue : s.ropenWire = true := by
                -- the topLocalOk w-shadow: res fired forces the wire
                have htop2 := hi.top
                simp only [topLocalOk, Bool.and_eq_true] at htop2
                obtain ⟨⟨⟨⟨⟨⟨⟨⟨⟨⟨⟨-, -⟩, -⟩, -⟩, hsh⟩, -⟩, -⟩, -⟩, -⟩, -⟩,
                  -⟩, -⟩ := htop2
                rcases (by simpa [AxMode.full] using hsh :
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
                have : (apply sk .full .ropenFire s).isSome = true := by
                  simp only [apply, hc]
                  rw [if_pos (by simpa [Skel.cap] using hchan)]
                  rfl
                exact this

/-- The absorber decode. -/
theorem absorb_pend_or_done (hwf : sk.wellFormed = true) {s : State}
    (hi : InvP sk .full s) :
    ((∀ e ∈ absorbEvents sk, performed sk s e) ∧ abPend s = [])
    ∨ ∃ f a pre suf, abPend s = [(f, a)]
        ∧ absorbEvents sk = pre ++ f :: suf
        ∧ (∀ e ∈ pre, performed sk s e)
        ∧ PendOk sk s f a := by
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
        have : (apply sk .full .absorbRecvWire s).isSome = true := by
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
          have : (apply sk .full .absorbRecvAsked s).isSome = true := by
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
          have : (apply sk .full .absorbSend s).isSome = true := by
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

/-- Assembler key bounds. -/
theorem asmKeys_bounds {pk : Party × Nat} (hpk : pk ∈ sk.asmKeys) :
    1 ≤ pk.2 ∧ (pk.1 = Party.I → pk.2 ≤ sk.rootH)
      ∧ (pk.1 = Party.R → pk.2 ≤ sk.rootH - 1) := by
  obtain ⟨p, j⟩ := pk
  simp only [Skel.asmKeys, List.mem_append, List.mem_map,
    List.mem_range] at hpk
  rcases hpk with ⟨t, ht, heq⟩ | ⟨t, ht, heq⟩ <;>
    · rw [Prod.mk.injEq] at heq
      obtain ⟨hp, hj⟩ := heq
      subst hp
      refine ⟨by omega, ?_, ?_⟩ <;>
        · intro h
          first
          | omega
          | cases h
  
/-- The responder's height-1 assembler never awaits level returns
(height-1 scopes are childless). -/
theorem pendAt_R_one (hwf : sk.wellFormed = true) (i : Nat) :
    sk.pendAt Party.R 1 i = 0 := by
  by_cases hin : i < (sk.asmResList Party.R 1).length
  · have hsucc := pendsBefore_succ sk (p := Party.R) (j := 1) hin
    have h1 := pendsBefore_asker_one hwf (p := Party.R) (by decide) i
    have h2 := pendsBefore_asker_one hwf (p := Party.R) (by decide) (i + 1)
    omega
  · unfold Skel.pendAt
    rw [List.getD_eq_getElem?_getD, List.getElem?_eq_none (by omega)]
    rfl

/-- An assembler's output count, on its own channel. -/
theorem sentOf_asmOut {s : State} {pk : Party × Nat}
    (hpk : pk ∈ sk.asmKeys) :
    sentOf sk s (sk.asmOutChan pk) = asmOutSent s pk := by
  obtain ⟨p, j⟩ := pk
  obtain ⟨h1, hIb, hRb⟩ := asmKeys_bounds sk hpk
  unfold Skel.asmOutChan
  by_cases hIr : p = Party.I ∧ j = sk.rootH
  · obtain ⟨rfl, rfl⟩ := hIr
    rw [if_pos (by simp)]
    rfl
  · rw [if_neg (by
      cases p <;> simp_all)]
    by_cases hRr : p = Party.R ∧ j = sk.rootH - 1
    · obtain ⟨rfl, rfl⟩ := hRr
      rw [if_pos (by simp)]
      rfl
    · rw [if_neg (by
        cases p <;> simp_all)]
      have hnI0 : ¬(p == Party.I && j == 0) = true := by
        simp
        intro _
        omega
      have hct : sk.asmKeys.contains (p, j) = true := by
        simpa using hpk
      have hnroot : isRootOutKey sk (p, j) = false := by
        unfold isRootOutKey
        cases p <;> simp_all
      show sentOf sk s (Chan.level p j) = asmOutSent s (p, j)
      simp only [sentOf]
      rw [if_neg hnI0, if_pos (by simp [hpk, hnroot])]

/-- An assembler's resolution-intake count, on its own channel. -/
theorem recvdOf_asmRes {s : State} {pk : Party × Nat}
    (hpk : pk ∈ sk.asmKeys) :
    recvdOf sk s (asmResChan pk) = asmResRecvd s pk := by
  obtain ⟨p, j⟩ := pk
  obtain ⟨h1, -, -⟩ := asmKeys_bounds sk hpk
  unfold asmResChan
  by_cases ha : asks p j = true
  · rw [if_pos ha]
    show recvdOf sk s (Chan.upper p (j - 1)) = asmResRecvd s (p, j)
    simp only [recvdOf]
    rw [show j - 1 + 1 = j from by omega]
  · rw [if_neg ha]
    show recvdOf sk s (Chan.lower p j) = asmResRecvd s (p, j)
    simp only [recvdOf]
    have hct : sk.asmKeys.contains (p, j) = true := by simpa using hpk
    rw [if_pos hct]

/-- An assembler's level-intake count, on its own channel. -/
theorem recvdOf_asmLevel {s : State} {pk : Party × Nat}
    (hpk : pk ∈ sk.asmKeys) :
    recvdOf sk s (asmLevelChan pk) = asmLevelRecvd sk s pk := by
  obtain ⟨p, j⟩ := pk
  obtain ⟨h1, -, -⟩ := asmKeys_bounds sk hpk
  unfold asmLevelChan
  show recvdOf sk s (Chan.level p (j - 1)) = asmLevelRecvd sk s (p, j)
  simp only [recvdOf]
  have hct : sk.asmKeys.contains (p, j - 1 + 1) = true := by
    rw [show j - 1 + 1 = j from by omega]
    simpa using hpk
  rw [if_pos hct, show j - 1 + 1 = j from by omega]

/-- The resolution-intake channel is a flow channel. -/
theorem asmResChan_mem (hwf : sk.wellFormed = true) {pk : Party × Nat}
    (hpk : pk ∈ sk.asmKeys) : asmResChan pk ∈ allChans sk := by
  obtain ⟨p, j⟩ := pk
  obtain ⟨h1, hIb, hRb⟩ := asmKeys_bounds sk hpk
  have hev : sk.rootH % 2 = 0 := (wf_rootH hwf).1
  have hge : 2 ≤ sk.rootH := (wf_rootH hwf).2
  unfold asmResChan
  by_cases ha : asks p j = true
  · rw [if_pos ha]
    have hkey : (p, j - 1) ∈ sk.walkKeys := by
      refine mem_walkKeys_of sk hwf ?_ ?_
      · cases p
        · have := hIb rfl
          unfold asks at ha
          simp at ha
          omega
        · have := hRb rfl
          omega
      · cases p
        · unfold asks at ha
          simp at ha
          exact Or.inl ⟨rfl, by omega⟩
        · unfold asks at ha
          simp at ha
          exact Or.inr ⟨rfl, by omega⟩
    have : Chan.upper p (j - 1) = upperOut (p, j - 1) := rfl
    rw [this]
    exact (walk_chans_mem sk hkey).2.2.1
  · rw [if_neg ha]
    have hkey : (p, j) ∈ sk.walkKeys := by
      refine mem_walkKeys_of sk hwf ?_ ?_
      · cases p
        · have := hIb rfl
          unfold asks at ha
          simp at ha
          omega
        · have := hRb rfl
          omega
      · cases p
        · unfold asks at ha
          simp at ha
          exact Or.inl ⟨rfl, by omega⟩
        · unfold asks at ha
          simp at ha
          exact Or.inr ⟨rfl, by omega⟩
    have : Chan.lower p j = lowerOut (p, j) := rfl
    rw [this]
    exact (walk_chans_mem sk hkey).2.2.2

/-- The level-intake channel is a flow channel wherever a level return
is actually owed. -/
theorem asmLevelChan_mem (hwf : sk.wellFormed = true) {s : State}
    {pk : Party × Nat} (hpk : pk ∈ sk.asmKeys)
    (hnz : 0 < sk.pendAt pk.1 pk.2 (s.asm pk).idx) :
    asmLevelChan pk ∈ allChans sk := by
  obtain ⟨p, j⟩ := pk
  obtain ⟨h1, hIb, hRb⟩ := asmKeys_bounds sk hpk
  unfold asmLevelChan
  by_cases hj1 : j = 1
  · subst hj1
    cases p with
    | I => exact (root_chans_mem sk).2.2.2.1
    | R =>
        rw [pendAt_R_one sk hwf] at hnz
        omega
  · have hkey : (p, j - 1) ∈ sk.asmKeys := by
      simp only [Skel.asmKeys, List.mem_append, List.mem_map,
        List.mem_range]
      cases p with
      | I =>
          have := hIb rfl
          exact Or.inl ⟨j - 2, by omega, by
            rw [Prod.mk.injEq]
            exact ⟨rfl, by omega⟩⟩
      | R =>
          have := hRb rfl
          exact Or.inr ⟨j - 2, by omega, by
            rw [Prod.mk.injEq]
            exact ⟨rfl, by omega⟩⟩
    unfold allChans
    refine List.mem_append.mpr (Or.inl (List.mem_append.mpr (Or.inr ?_)))
    exact List.mem_map.mpr ⟨(p, j - 1), hkey, rfl⟩

/-- Every assembler key's trace is a merge input. -/
theorem asmEvents_mem_procs {pk : Party × Nat} (hpk : pk ∈ sk.asmKeys) :
    asmEvents sk pk ∈ procs sk := by
  simp only [procs]
  refine List.mem_append.mpr (Or.inl (List.mem_append.mpr (Or.inr ?_)))
  exact List.mem_map.mpr ⟨pk, hpk, rfl⟩

/-- The assembler decode. -/
theorem asm_pend_or_done (hwf : sk.wellFormed = true) {s : State}
    (hi : InvP sk .full s) {pk : Party × Nat} (hpk : pk ∈ sk.asmKeys) :
    ((∀ e ∈ asmEvents sk pk, performed sk s e) ∧ asmPend sk s pk = [])
    ∨ ∃ f a pre suf, asmPend sk s pk = [(f, a)]
        ∧ asmEvents sk pk = pre ++ f :: suf
        ∧ (∀ e ∈ pre, performed sk s e)
        ∧ PendOk sk s f a := by
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
        have : (apply sk .full (.asmRecvRes pk) s).isSome = true := by
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
          have : (apply sk .full (.asmRecvLevel pk) s).isSome = true := by
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
          have : (apply sk .full (.asmSend pk) s).isSome = true := by
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
