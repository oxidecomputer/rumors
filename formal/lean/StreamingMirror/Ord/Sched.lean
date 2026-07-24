/-
The ord-parameterized trace layer: the per-process E3-linear traces
with each pairing loop's two-receive prologue in ITS assigned order,
and the same fuel-indexed priority merge run over them.

Only the prologue receives move. Sends never move in the dequeue-order
class, so every scope's send suffix is `Sched.scopeSendsE` — the
`.impl` encoder order, parent at the scope tail — verbatim, and the
absorber's level-0 return stays after its block's two receives. The
two receives of any prologue therefore live on DISTINCT channels
(`wireIn` vs `askedIn` for a walk; the leaf wire vs `leafRequests` for
the absorber), which is the design fact Ord/Numbering.lean cashes in:
per channel-side the O traces project identically to the `procsE`
family, so the proj-based counting layer transfers by rewrite.

The reply-first assignment collapses the family definitionally
(`procsO_rf`, `scheduleO_rf`, both `rfl`): anything proved over
`procsO` reads at `ord = .rf` as the `procsE` statement. The
executable twin is `EventDag.schedCandidateO`/`walkTraceEO`/
`absorbTraceO`, kept in agreement by the tool's gate.

Chain (ord, stage C): the O trace family and its merge; consumed by
Ord/Numbering.lean. Base mirror: Proofs/Sched.lean's `procsE`/
`scheduleE` tier. Map: PROGRESS.md §13.
-/
import StreamingMirror.Proofs.Sched
import StreamingMirror.Ord.Basic

namespace StreamingMirror.Ord

open Model
open Sched

variable (sk : Skel) (ord : OrdMap)

-- ==================================================== the O trace family

/-- The two prologue receives of scope `k` at walk `pk`, in the loop's
assigned order: reply-first receives the wire then the paired query,
query-first the swap. -/
def prologueO (ord : OrdMap) (pk : Party × Nat) (k : Nat) : List Ev :=
  match ord.walk pk with
  | .replyFirst => [(wireIn pk, false, k), (askedIn pk, false, k)]
  | .queryFirst => [(askedIn pk, false, k), (wireIn pk, false, k)]

/-- One scope of a walk's O trace: the ordered prologue, then the
encoder-order sends (parent at the scope tail — the `.impl` chain's
send order; the d5 splice never enters this family). -/
def scopeBlockO (pk : Party × Nat) (k : Nat) : List Ev :=
  prologueO ord pk k ++ scopeSendsE sk pk k

/-- Walk `pk`'s full O trace: its stage's scopes in order. -/
def walkEventsO (pk : Party × Nat) : List Ev :=
  (List.range (sk.stageLen pk.2)).flatMap (scopeBlockO sk ord pk)

/-- The absorber's block for leaf request `j`: the two receives in the
absorber's assigned order, then the level-0 return (reply-first is
`Sched.absorbEvents`'s order: wire, leaf request, send). -/
def absorbBlockO (ord : OrdMap) (j : Nat) : List Ev :=
  (match ord.absorb with
    | .replyFirst =>
        [(Chan.wire Party.R 0, false, j), (Chan.leafRequests, false, j)]
    | .queryFirst =>
        [(Chan.leafRequests, false, j), (Chan.wire Party.R 0, false, j)])
    ++ [(Chan.level Party.I 0, true, j)]

/-- The absorber's full O trace: its per-leaf-request blocks in
order. -/
def absorbEventsO : List Ev :=
  (List.range sk.totalLeafReqs).flatMap (absorbBlockO ord)

/-- The O process traces: `Sched.procsE` with each walk trace in its
assigned prologue order and the absorb trace in the absorber's.

Same fixed priority order — openers, walks by descending stage,
absorb, the asm towers bottom-up, the floating `rootret` receive,
fins — and every family other than the walks and the absorber is
order-independent (no other process runs a pairing loop). -/
def procsO : List (List Ev) :=
  let walkOrder : List (Party × Nat) :=
    (List.range sk.rootH).map fun i =>
      let h := sk.rootH - 1 - i
      (if h % 2 == 1 then Party.I else Party.R, h)
  [iopenEvents sk, ropenEvents sk]
    ++ walkOrder.map (walkEventsO sk ord)
    ++ [absorbEventsO sk ord]
    ++ sk.asmKeys.map (asmEvents sk)
    ++ [[(Chan.rootret, false, 0)], finEvents sk]

/-- The O merge's fuel: the whole event set's size. -/
def totalEventsO : Nat := ((procsO sk ord).map List.length).sum

/-- The O merge's final state (cf. `Sched.finalStateE`). -/
def finalStateO : MState :=
  mergeN sk (totalEventsO sk ord) ⟨[], fun _ => 0, fun _ => 0, procsO sk ord⟩

/-- The O canonical schedule: the merge's output over the O traces. -/
def scheduleO : List Ev := (finalStateO sk ord).out

-- ============================================ the reply-first collapse

/-- The all-reply-first O family IS the encoder-order family,
definitionally: every walk prologue and the absorber dispatch to the E
order, and the send suffixes are shared. -/
theorem procsO_rf : procsO sk .rf = procsE sk := rfl

/-- The all-reply-first O schedule IS the encoder-order schedule:
same traces, same fuel, same merge. -/
theorem scheduleO_rf : scheduleO sk .rf = scheduleE sk := rfl

end StreamingMirror.Ord
