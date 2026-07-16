/-
The canonical schedule, transcribed for proof (PROGRESS.md §5, §7
item 3): the per-process E3-linear traces as structural folds over the
skeleton, and the priority merge as a fuel-indexed step function. This
is proof scaffolding — the audit surface (Statement.lean) never
mentions it; τ = position in `schedule` is the potential the progress
lemma's argmin argument consumes.

# Relation to the executable oracle

`EventDag.lean` (the `eventdag` exe) carries the same construction in
imperative form, plus everything a definition cannot: the DAG edge
check, the model replay, the random sweep. The two are kept in exact
agreement by the tool's gate — `runAll` and `runFuzz` fail unless
`Sched.schedule` equals `EventDag.schedCandidate` event-for-event on
every pin and every acyclic fuzz seed. Change either side only with
that check in hand; it is the validate-then-prove discipline applied
to this transcription itself.

Two deltas from the imperative form, both simplifications the proofs
want and the gate certifies as behavior-preserving:

- Running counters become `Skel`'s prefix sums (`wiresBefore`,
  `dsBefore`, `qsBefore`, `pendsBefore`): each event's seq is a closed
  form of its scope position, which is exactly the correspondence the
  counting layer (`Proofs/Counting.lean`) speaks about.
- Per-process cursors become remaining-suffix lists (`MState.rem`):
  the merge step is a structural recursion (`scan`), and "trace
  monotonicity" becomes literal — a process's emitted events are the
  complement of its remaining suffix.

# The obligations this file carries (PROGRESS.md §5)

- Edge-respect and per-trace monotonicity: by construction of `step`
  (a receive is emitted only after its send's count, a send only into
  an open cap window, and each trace only ever emits its head).
- Merge COMPLETENESS — `schedule` drains every trace — is the real
  content, and is where the `Skel.schedulable` hypothesis must enter:
  `Pin.pyramid 1` (well-formed, not schedulable) stalls the merge with
  events unemitted. Open; see PROGRESS.md §7.
-/
import StreamingMirror.Model
import StreamingMirror.Instances

namespace StreamingMirror.Sched

open Model

/-- Event: channel, side (`true` = snd, `false` = rcv), 0-based seq —
the same triple the eventdag oracle uses. -/
abbrev Ev := Chan × Bool × Nat

variable (sk : Skel)

-- ================================================= per-process traces
-- Each trace linearizes one process's E3-forced order (the `.full`
-- guards); seqs come from the Skel prefix sums, so trace membership is
-- positional arithmetic, not counter simulation.

/-- Send chunk of child `i` at scope `k` of stage `pk.2`: the wire,
then — for a disputed child — its resolution and dependent queries.
The seqs are the prefix sums: wire `i` is the stage's
`wiresBefore + i`-th wire, the resolution's rank counts prior D
siblings, and the queries start after every earlier child's. -/
def childChunk (pk : Party × Nat) (k i : Nat) : List Ev :=
  let h := pk.2
  let s := sk.stageScope h k
  let wire : Ev := (wireOut pk, true, sk.wiresBefore h k + i)
  if sk.childIsD h s i then
    let dRank := ((List.range i).filter (fun i' => sk.childIsD h s i')).length
    let res : Ev := (lowerOut pk, true, sk.dsBefore h k + dRank)
    let qBase := sk.qsBefore h k
      + ((List.range i).map (fun i' => sk.qCount h s i')).sum
    wire :: res :: ((List.range (sk.qCount h s i)).map fun t =>
      (askedOut pk, true, qBase + t))
  else [wire]

/-- The sends of scope `k`, with the parent summary spliced immediately
after the scope's final resolution: after the last D child's res,
BEFORE that child's queries; first of all when the scope disputes
nothing. The placement is load-bearing (PROGRESS.md §5): parent-last
deadlocks the merge, parent-after-last-res is safe because the upper
window depends only on strictly earlier scopes' subtrees. A D child's
chunk is `wire :: res :: queries`, so `take 2` cuts exactly after the
res. -/
def scopeSends (pk : Party × Nat) (k : Nat) : List Ev :=
  let h := pk.2
  let s := sk.stageScope h k
  let n := sk.nChildren h s
  let parent : Ev := (upperOut pk, true, k)
  let chunks := (List.range n).map (childChunk sk pk k)
  match ((List.range n).filter (fun i => sk.childIsD h s i)).getLast? with
  | none => parent :: chunks.flatten
  | some j =>
      (chunks.take j).flatten ++ (chunks.getD j []).take 2
        ++ parent :: ((chunks.getD j []).drop 2
        ++ (chunks.drop (j + 1)).flatten)

/-- One scope of a walk's trace: the two-receive prologue, then the
sends. -/
def scopeBlock (pk : Party × Nat) (k : Nat) : List Ev :=
  (wireIn pk, false, k) :: (askedIn pk, false, k) :: scopeSends sk pk k

/-- Walk `pk`'s full trace: its stage's scopes in order. -/
def walkEvents (pk : Party × Nat) : List Ev :=
  (List.range (sk.stageLen pk.2)).flatMap (scopeBlock sk pk)

/-- iopen: the opening wire, then the root query. -/
def iopenEvents : List Ev :=
  [(Chan.wire Party.I sk.rootH, true, 0),
   (Chan.asked Party.I (sk.rootH - 1), true, 0)]

/-- ropen: receive the opening wire, answer with wire and root
resolution, then the root child queries. -/
def ropenEvents : List Ev :=
  (Chan.wire Party.I sk.rootH, false, 0)
    :: (Chan.wire Party.R sk.rootH, true, 0)
    :: (Chan.rootres, true, 0)
    :: ((List.range sk.rootPending).map fun j =>
        (Chan.asked Party.R (sk.rootH - 2), true, j))

/-- Absorb: wire, leaf request, level-0 return, looped per leaf. -/
def absorbEvents : List Ev :=
  (List.range sk.totalLeafReqs).flatMap fun j =>
    [(Chan.wire Party.R 0, false, j),
     (Chan.leafRequests, false, j),
     (Chan.level Party.I 0, true, j)]

/-- Asm `pk`, resolution `idx`: the resolution receive, its pending
level returns (seqs by `pendsBefore`), the output send. -/
def asmBlock (pk : Party × Nat) (idx : Nat) : List Ev :=
  (asmResChan pk, false, idx)
    :: ((List.range (sk.pendAt pk.1 pk.2 idx)).map fun t =>
        (asmLevelChan pk, false, sk.pendsBefore pk.1 pk.2 idx + t))
    ++ [(sk.asmOutChan pk, true, idx)]

/-- Asm `pk`'s full trace: its resolution list in order. -/
def asmEvents (pk : Party × Nat) : List Ev :=
  (List.range (sk.asmResList pk.1 pk.2).length).flatMap (asmBlock sk pk)

/-- fins, minus the floating `rootret` receive (its own trace in
`procs`): the root resolution, then the root returns in order. -/
def finEvents : List Ev :=
  (Chan.rootres, false, 0)
    :: ((List.range sk.rootPending).map fun j => (Chan.rootrets, false, j))

/-- Every process trace, in the merge's fixed priority order: openers,
walks by descending stage (descent before assembly), absorb, the asm
towers bottom-up (I then R, `asmKeys`' order), the floating `rootret`
receive, the rest of fins. -/
def procs : List (List Ev) :=
  let walkOrder : List (Party × Nat) :=
    (List.range sk.rootH).map fun i =>
      let h := sk.rootH - 1 - i
      (if h % 2 == 1 then Party.I else Party.R, h)
  [iopenEvents sk, ropenEvents sk]
    ++ walkOrder.map (walkEvents sk)
    ++ [absorbEvents sk]
    ++ sk.asmKeys.map (asmEvents sk)
    ++ [[(Chan.rootret, false, 0)], finEvents sk]

-- ========================================================== the merge

/-- Merge state: the emitted prefix, per-channel emitted send/receive
counts, and each trace's remaining suffix (a process's emitted events
are exactly its trace minus its suffix — trace monotonicity is
structural, not simulated). -/
structure MState where
  out : List Ev
  sent : Chan → Nat
  rcvd : Chan → Nat
  rem : List (List Ev)

/-- Is `e` emittable against the emitted prefix? A receive needs its
message sent (E1); a send needs its cap window open (E2). E3 needs no
check: only trace heads are offered. -/
def enabled (sent rcvd : Chan → Nat) : Ev → Bool
  | (c, true, n) => decide (n < rcvd c + sk.cap c)
  | (c, false, n) => decide (n < sent c)

/-- Find the first trace whose head is enabled; return the head and
the suffix list with that trace advanced. `none` means every trace is
drained or stalled — completeness (PROGRESS.md §5) is the claim that
under `Skel.schedulable` the drained case is the only one. -/
def scan (sent rcvd : Chan → Nat) : List (List Ev) → Option (Ev × List (List Ev))
  | [] => none
  | [] :: ts => (scan sent rcvd ts).map fun (e, ts') => (e, [] :: ts')
  | (e :: rest) :: ts =>
      if enabled sk sent rcvd e then some (e, rest :: ts)
      else (scan sent rcvd ts).map fun (e', ts') => (e', (e :: rest) :: ts')

/-- One merge step: emit the first enabled head. -/
def step (st : MState) : Option MState :=
  (scan sk st.sent st.rcvd st.rem).map fun (e, rem') =>
    match e with
    | (c, true, _) =>
        { out := st.out ++ [e]
          sent := fun c' => if c' = c then st.sent c + 1 else st.sent c'
          rcvd := st.rcvd, rem := rem' }
    | (c, false, _) =>
        { out := st.out ++ [e], sent := st.sent
          rcvd := fun c' => if c' = c then st.rcvd c + 1 else st.rcvd c'
          rem := rem' }

/-- Fuel-indexed merge: iterate `step` until it stalls or the fuel is
spent. Every step emits exactly one event, so total-event-count fuel
is never the binding constraint — `mergeN` stops at the fixpoint. -/
def mergeN : Nat → MState → MState
  | 0, st => st
  | fuel + 1, st =>
      match step sk st with
      | some st' => mergeN fuel st'
      | none => st

/-- The whole event set's size — the merge's sufficient fuel. -/
def totalEvents : Nat := ((procs sk).map List.length).sum

/-- The merge's final state: run to fixpoint from empty counters
(total-event fuel; each step emits one event, so the fixpoint is
reached). The lemmas below speak about this state — in particular its
`rem`, the traces' unemitted suffixes, which completeness must show
empty. -/
def finalState : MState :=
  mergeN sk (totalEvents sk) ⟨[], fun _ => 0, fun _ => 0, procs sk⟩

/-- The canonical schedule: the final state's output. τ(e) = index in
this list. Kept event-for-event equal to `EventDag.schedCandidate` by
the eventdag gate. -/
def schedule : List Ev := (finalState sk).out


-- ================================== the by-construction lemmas (§5)
-- Everything below is generic over the trace list `procs₀`: none of it
-- looks inside a trace, so it holds for ANY merge input. The
-- trace-structure layer (canonical per-channel seq numbering) and
-- merge completeness are the separate, later obligations.

/-- Pointwise relation between two lists (batteries ships no
`Forall₂`; this is the fragment the merge invariant needs). -/
inductive Forall2 {α β : Type _} (R : α → β → Prop) : List α → List β → Prop
  | nil : Forall2 R [] []
  | cons {a la b lb} : R a b → Forall2 R la lb → Forall2 R (a :: la) (b :: lb)

/-- A reflexive-shaped instance: relate every element to itself. -/
theorem Forall2.self {α : Type _} {R : α → α → Prop} :
    ∀ {l : List α}, (∀ a ∈ l, R a a) → Forall2 R l l
  | [], _ => .nil
  | a :: _, h =>
      .cons (h a (List.mem_cons_self ..))
        (Forall2.self fun x hx => h x (List.mem_cons_of_mem a hx))

/-- Weaken the relation pointwise. -/
theorem Forall2.imp {α β : Type _} {R S : α → β → Prop}
    (h : ∀ a b, R a b → S a b) :
    ∀ {la : List α} {lb : List β}, Forall2 R la lb → Forall2 S la lb
  | _, _, .nil => .nil
  | _, _, .cons hab t => .cons (h _ _ hab) (t.imp h)

/-- Every left element has a related right partner. -/
theorem Forall2.exists_of_mem_left {α β : Type _} {R : α → β → Prop} :
    ∀ {la : List α} {lb : List β}, Forall2 R la lb → ∀ {a}, a ∈ la →
      ∃ b ∈ lb, R a b
  | _, _, .cons hab t, a, ha => by
      rcases List.mem_cons.1 ha with rfl | ha'
      · exact ⟨_, List.mem_cons_self .., hab⟩
      · obtain ⟨b, hb, hr⟩ := t.exists_of_mem_left ha'
        exact ⟨b, List.mem_cons_of_mem _ hb, hr⟩

/-- Taking at most the left part of an append never sees the right. -/
private theorem take_append_le {α : Type _} :
    ∀ (n : Nat) (l₁ l₂ : List α), n ≤ l₁.length →
      (l₁ ++ l₂).take n = l₁.take n
  | 0, _, _, _ => by simp
  | n + 1, [], _, h => by simp at h
  | n + 1, a :: l₁, l₂, h => by
      simp only [List.cons_append, List.take_succ_cons]
      rw [take_append_le n l₁ l₂ (by simpa using h)]

/-- Taking exactly the left part of an append recovers it. -/
private theorem take_len_append {α : Type _} (l₁ l₂ : List α) :
    (l₁ ++ l₂).take l₁.length = l₁ := by
  rw [take_append_le _ _ _ (Nat.le_refl _), List.take_length]

/-- Taking one past the left part of an append captures its head. -/
private theorem take_append_succ {α : Type _} :
    ∀ (l₁ : List α) (a : α) (l₂ : List α),
      (l₁ ++ a :: l₂).take (l₁.length + 1) = l₁ ++ [a]
  | [], _, _ => by simp
  | x :: l₁, a, l₂ => by
      simp only [List.cons_append, List.length_cons, List.take_succ_cons,
        take_append_succ l₁ a l₂]

/-- Sends on `c` in a prefix — the count the E1 guard consults. -/
def sndCount (c : Chan) (l : List Ev) : Nat :=
  (l.filter fun e => decide (e.1 = c) && e.2.1).length

/-- Receives on `c` in a prefix — the count the E2 guard consults. -/
def rcvCount (c : Chan) (l : List Ev) : Nat :=
  (l.filter fun e => decide (e.1 = c) && !e.2.1).length

/-- Appending a send bumps its own channel's send count and nothing
else. -/
theorem sndCount_append_snd (c c' : Chan) (l : List Ev) (n : Nat) :
    sndCount c' (l ++ [(c, true, n)])
      = sndCount c' l + (if c' = c then 1 else 0) := by
  by_cases h : c' = c
  · subst h; simp [sndCount, List.filter_append]
  · have h' : ¬(c = c') := fun hh => h hh.symm
    simp [sndCount, List.filter_append, h, h']

theorem rcvCount_append_snd (c c' : Chan) (l : List Ev) (n : Nat) :
    rcvCount c' (l ++ [(c, true, n)]) = rcvCount c' l := by
  simp [rcvCount, List.filter_append]

theorem sndCount_append_rcv (c c' : Chan) (l : List Ev) (n : Nat) :
    sndCount c' (l ++ [(c, false, n)]) = sndCount c' l := by
  simp [sndCount, List.filter_append]

theorem rcvCount_append_rcv (c c' : Chan) (l : List Ev) (n : Nat) :
    rcvCount c' (l ++ [(c, false, n)])
      = rcvCount c' l + (if c' = c then 1 else 0) := by
  by_cases h : c' = c
  · subst h; simp [rcvCount, List.filter_append]
  · have h' : ¬(c = c') := fun hh => h hh.symm
    simp [rcvCount, List.filter_append, h, h']

/-- Filter-length of the traces' emitted prefixes: for each
`(trace, remainder)` pair, the prefix is `t.take (t.length - r.length)`
(under `rem_struct`'s decomposition that IS the emitted prefix), and
the counts sum across pairs. -/
def emittedCount (p : Ev → Bool) : List (List Ev) → List (List Ev) → Nat
  | t :: ts, r :: rs =>
      ((t.take (t.length - r.length)).filter p).length + emittedCount p ts rs
  | _, _ => 0

/-- Nothing is emitted while every remainder is its whole trace. -/
private theorem emittedCount_refl (p : Ev → Bool) :
    ∀ l : List (List Ev), emittedCount p l l = 0
  | [] => rfl
  | _ :: ts => by simp [emittedCount, emittedCount_refl p ts]

/-- The merge invariant, relative to the original trace list.

`rem_struct` is trace monotonicity in structural form: each trace is
its emitted prefix (an in-order subsequence of `out`) plus its
remaining suffix — no event-distinctness assumption anywhere.
`e1_hist`/`e2_hist` are edge-respect in positional form: at the index
where a receive (send) sits, its guard's count inequality held over
the strict prefix — exactly the τ-indexed facts the §6 blame lemmas
consume. `out_count` is provenance: under every predicate, `out` has
exactly as many events as the traces' emitted prefixes — without it,
an `out` padded with duplicated sends satisfies every other field
(`rem_struct` bounds `out` from below only), and the canonical
per-channel numbering layer could not key the schedule's n-th send on
a channel to its producing trace's n-th. -/
structure MInv (procs₀ : List (List Ev)) (st : MState) : Prop where
  rem_struct : Forall2
    (fun t r => ∃ pre, t = pre ++ r ∧ pre.Sublist st.out) procs₀ st.rem
  sent_eq : ∀ c, st.sent c = sndCount c st.out
  rcvd_eq : ∀ c, st.rcvd c = rcvCount c st.out
  e1_hist : ∀ k c n, st.out[k]? = some (c, false, n) →
    n < sndCount c (st.out.take k)
  e2_hist : ∀ k c n, st.out[k]? = some (c, true, n) →
    n < rcvCount c (st.out.take k) + sk.cap c
  out_count : ∀ p : Ev → Bool,
    (st.out.filter p).length = emittedCount p procs₀ st.rem

/-- The initial merge state satisfies the invariant. -/
theorem minv_init (procs₀ : List (List Ev)) :
    MInv sk procs₀ ⟨[], fun _ => 0, fun _ => 0, procs₀⟩ := by
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_⟩
  · exact Forall2.self fun t _ => ⟨[], rfl, List.nil_sublist _⟩
  · intro c; rfl
  · intro c; rfl
  · intro k c n h; simp at h
  · intro k c n h; simp at h
  · intro p; simp [emittedCount_refl]

/-- What one successful `scan` does to the suffix structure: the
emitted event is some trace's enabled head, that trace advances by
one, every emitted prefix stays a sublist of the grown output, and the
emitted-prefix counts grow by exactly the one event, under every
predicate. -/
theorem scan_step (out : List Ev) (sent rcvd : Chan → Nat)
    {procs₀ ts ts' : List (List Ev)} {e : Ev}
    (hrs : Forall2
      (fun t r => ∃ pre, t = pre ++ r ∧ pre.Sublist out) procs₀ ts)
    (hscan : scan sk sent rcvd ts = some (e, ts')) :
    enabled sk sent rcvd e = true ∧
    Forall2
      (fun t r => ∃ pre, t = pre ++ r ∧ pre.Sublist (out ++ [e]))
      procs₀ ts' ∧
    ∀ p : Ev → Bool, emittedCount p procs₀ ts'
      = (if p e then 1 else 0) + emittedCount p procs₀ ts := by
  induction ts generalizing procs₀ ts' with
  | nil => cases hrs; simp [scan] at hscan
  | cons t₀ ts₁ ih =>
      cases hrs with
      | @cons tr₀ ps₁ _ _ hpair htail =>
        obtain ⟨pre, hpre, hsub⟩ := hpair
        match t₀ with
        | [] =>
            cases hrec : scan sk sent rcvd ts₁ with
            | none => rw [scan, hrec] at hscan; simp at hscan
            | some pr =>
                obtain ⟨e', ts₁'⟩ := pr
                rw [scan, hrec] at hscan
                simp only [Option.map] at hscan
                cases hscan
                obtain ⟨hen, hrest, hcnt⟩ := ih htail hrec
                refine ⟨hen, .cons
                  ⟨pre, hpre, hsub.trans (List.sublist_append_left ..)⟩
                  hrest, ?_⟩
                intro p
                have hc := hcnt p
                simp only [emittedCount]
                omega
        | ev :: rest =>
            by_cases hen : enabled sk sent rcvd ev = true
            · rw [scan, if_pos hen] at hscan
              cases hscan
              subst hpre
              refine ⟨hen, .cons ⟨pre ++ [e], by simp, ?_⟩ ?_, ?_⟩
              · exact hsub.append (List.Sublist.refl [e])
              · exact htail.imp fun t r ⟨pre', h', hs'⟩ =>
                  ⟨pre', h', hs'.trans (List.sublist_append_left ..)⟩
              · intro p
                simp only [emittedCount]
                have hL1 : (pre ++ e :: rest).length - rest.length
                    = pre.length + 1 := by
                  simp only [List.length_append, List.length_cons]; omega
                have hL2 : (pre ++ e :: rest).length - (e :: rest).length
                    = pre.length := by
                  simp only [List.length_append, List.length_cons]; omega
                rw [hL1, hL2, take_append_succ,
                  take_append_le _ _ _ (Nat.le_refl _), List.take_length,
                  List.filter_append, List.length_append]
                cases hpe : p e <;> simp [hpe] <;> omega
            · cases hrec : scan sk sent rcvd ts₁ with
              | none =>
                  rw [scan, if_neg hen, hrec] at hscan; simp at hscan
              | some pr =>
                  obtain ⟨e', ts₁'⟩ := pr
                  rw [scan, if_neg hen, hrec] at hscan
                  simp only [Option.map] at hscan
                  cases hscan
                  obtain ⟨hen', hrest, hcnt⟩ := ih htail hrec
                  refine ⟨hen', .cons
                    ⟨pre, hpre, hsub.trans (List.sublist_append_left ..)⟩
                    hrest, ?_⟩
                  intro p
                  have hc := hcnt p
                  simp only [emittedCount]
                  omega

/-- One merge step preserves the invariant. -/
theorem step_preserves {procs₀ : List (List Ev)} {st st' : MState}
    (hinv : MInv sk procs₀ st) (hstep : step sk st = some st') :
    MInv sk procs₀ st' := by
  unfold step at hstep
  cases hscan : scan sk st.sent st.rcvd st.rem with
  | none => rw [hscan] at hstep; simp at hstep
  | some pr =>
    obtain ⟨e, rem'⟩ := pr
    rw [hscan] at hstep
    simp only [Option.map] at hstep
    obtain ⟨hen, hrs', hcnt⟩ := scan_step sk st.out st.sent st.rcvd
      hinv.rem_struct hscan
    obtain ⟨c, sd, n⟩ := e
    cases sd with
    | true =>
        cases hstep
        refine ⟨hrs', ?_, ?_, ?_, ?_, ?_⟩
        · intro c'
          rw [sndCount_append_snd]
          by_cases h : c' = c <;> simp [h, hinv.sent_eq]
        · intro c'
          rw [rcvCount_append_snd]
          exact hinv.rcvd_eq c'
        · intro k c' n' hk
          rcases Nat.lt_or_ge k st.out.length with hlt | hge
          · rw [List.getElem?_append_left hlt] at hk
            rw [take_append_le _ _ _ (Nat.le_of_lt hlt)]
            exact hinv.e1_hist k c' n' hk
          · rw [List.getElem?_append_right hge] at hk
            cases hm : k - st.out.length with
            | zero => rw [hm] at hk; simp at hk
            | succ m => rw [hm] at hk; simp at hk
        · intro k c' n' hk
          rcases Nat.lt_or_ge k st.out.length with hlt | hge
          · rw [List.getElem?_append_left hlt] at hk
            rw [take_append_le _ _ _ (Nat.le_of_lt hlt)]
            exact hinv.e2_hist k c' n' hk
          · rw [List.getElem?_append_right hge] at hk
            cases hm : k - st.out.length with
            | zero =>
                rw [hm] at hk
                simp only [List.getElem?_cons_zero, Option.some.injEq,
                  Prod.mk.injEq] at hk
                obtain ⟨hc, -, hn⟩ := hk
                subst hc hn
                have hkl : k = st.out.length := by omega
                subst hkl
                rw [take_len_append]
                have hrc := hinv.rcvd_eq c
                simp only [enabled, decide_eq_true_eq] at hen
                omega
            | succ m => rw [hm] at hk; simp at hk
        · intro p
          have hc := hcnt p
          rw [List.filter_append, List.length_append, hinv.out_count p, hc]
          cases hpe : p (c, true, n) <;> simp [hpe] <;> omega
    | false =>
        cases hstep
        refine ⟨hrs', ?_, ?_, ?_, ?_, ?_⟩
        · intro c'
          rw [sndCount_append_rcv]
          exact hinv.sent_eq c'
        · intro c'
          rw [rcvCount_append_rcv]
          by_cases h : c' = c <;> simp [h, hinv.rcvd_eq]
        · intro k c' n' hk
          rcases Nat.lt_or_ge k st.out.length with hlt | hge
          · rw [List.getElem?_append_left hlt] at hk
            rw [take_append_le _ _ _ (Nat.le_of_lt hlt)]
            exact hinv.e1_hist k c' n' hk
          · rw [List.getElem?_append_right hge] at hk
            cases hm : k - st.out.length with
            | zero =>
                rw [hm] at hk
                simp only [List.getElem?_cons_zero, Option.some.injEq,
                  Prod.mk.injEq] at hk
                obtain ⟨hc, -, hn⟩ := hk
                subst hc hn
                have hkl : k = st.out.length := by omega
                subst hkl
                rw [take_len_append]
                have hsc := hinv.sent_eq c
                simp only [enabled, decide_eq_true_eq] at hen
                omega
            | succ m => rw [hm] at hk; simp at hk
        · intro k c' n' hk
          rcases Nat.lt_or_ge k st.out.length with hlt | hge
          · rw [List.getElem?_append_left hlt] at hk
            rw [take_append_le _ _ _ (Nat.le_of_lt hlt)]
            exact hinv.e2_hist k c' n' hk
          · rw [List.getElem?_append_right hge] at hk
            cases hm : k - st.out.length with
            | zero => rw [hm] at hk; simp at hk
            | succ m => rw [hm] at hk; simp at hk
        · intro p
          have hc := hcnt p
          rw [List.filter_append, List.length_append, hinv.out_count p, hc]
          cases hpe : p (c, false, n) <;> simp [hpe] <;> omega

/-- The invariant survives any amount of fuel. -/
theorem mergeN_preserves {procs₀ : List (List Ev)} (fuel : Nat)
    {st : MState} (hinv : MInv sk procs₀ st) :
    MInv sk procs₀ (mergeN sk fuel st) := by
  induction fuel generalizing st with
  | zero => exact hinv
  | succ f ih =>
      unfold mergeN
      cases hstep : step sk st with
      | some st' => exact ih (step_preserves sk hinv hstep)
      | none => exact hinv

/-- The invariant at the merge's final state. -/
theorem schedule_inv : MInv sk (procs sk) (finalState sk) :=
  mergeN_preserves sk _ (minv_init sk (procs sk))

-- ============================== the corollaries the blame lemmas use

/-- Trace monotonicity, pinned to the final remainder: each trace is
its emitted prefix — an in-order subsequence of the schedule, so τ is
monotone along it — plus its ACTUAL unemitted suffix,
`(finalState sk).rem`. Do not weaken the remainder to an existential:
with the suffix unconstrained the split is trivially satisfiable at
`pre = []`. Completeness (open, needs `Skel.schedulable`) is the claim
that every remainder is empty, which specializes this to "every trace
is a sublist of the schedule". -/
theorem trace_monotone :
    Forall2 (fun t r => ∃ pre, t = pre ++ r ∧ pre.Sublist (schedule sk))
      (procs sk) (finalState sk).rem :=
  (schedule_inv sk).rem_struct

/-- E1-respect, counted: at every receive's position, strictly more
sends than its seq have already happened on its channel. (The
canonical per-channel numbering of the traces — the next layer — turns
this into "`snd(c,n)` precedes `rcv(c,n)`".) -/
theorem schedule_e1 (k : Nat) (c : Chan) (n : Nat)
    (h : (schedule sk)[k]? = some (c, false, n)) :
    n < sndCount c ((schedule sk).take k) :=
  (schedule_inv sk).e1_hist k c n h

/-- E2-respect, counted: every send fires into an open cap window. -/
theorem schedule_e2 (k : Nat) (c : Chan) (n : Nat)
    (h : (schedule sk)[k]? = some (c, true, n)) :
    n < rcvCount c ((schedule sk).take k) + sk.cap c :=
  (schedule_inv sk).e2_hist k c n h

/-- Provenance: under every predicate, the schedule counts exactly what
the traces' emitted prefixes count — the output cannot hold an event no
trace emitted, nor an extra copy of one. This is what lets the
canonical-numbering layer key the schedule's n-th send on a channel to
the producing trace's n-th (`rem_struct` alone bounds the output from
below only). -/
theorem schedule_count (p : Ev → Bool) :
    ((schedule sk).filter p).length
      = emittedCount p (procs sk) (finalState sk).rem :=
  (schedule_inv sk).out_count p

-- ===================================== kernel-tier non-vacuity anchor
-- Every theorem above is generic over the merge input and would hold
-- vacuously if the merge never stepped: `schedule = []` satisfies all
-- of them (`[][k]? = none`; monotone prefixes may be empty). The
-- executable gate pins non-vacuity continuously against the oracle;
-- the anchor below pins it in the KERNEL, so `lake build` alone
-- certifies the merge actually runs — and it is simultaneously the
-- first kernel-checked instance of merge completeness (PROGRESS.md
-- §7's open obligation), on the smallest pin.

set_option maxRecDepth 16000 in
/-- The merge drains every smokeChain trace: the final remainders are
all empty, so the schedule holds the pin's whole event set and the
theorems above range over a real, completed merge. -/
theorem smokeChain_merge_complete :
    ((finalState Pin.smokeChain).rem.all List.isEmpty) = true := by
  decide

end StreamingMirror.Sched
