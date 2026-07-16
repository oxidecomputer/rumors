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

/-- The canonical schedule: the merge run to fixpoint from empty
counters. τ(e) = index in this list. Kept event-for-event equal to
`EventDag.schedCandidate` by the eventdag gate. -/
def schedule : List Ev :=
  (mergeN sk (totalEvents sk)
    ⟨[], fun _ => 0, fun _ => 0, procs sk⟩).out

end StreamingMirror.Sched
