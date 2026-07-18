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

end StreamingMirror.Sched
