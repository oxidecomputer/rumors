# The progress lemma: design of record

Target: `progress : InvP sk .full s → terminal sk s = false →
canStep sk .full s = true`, then
`deadlock_free = inv_reachable + progress` (Statement.lean's
`DeadlockFree sk AxMode.full`). This document is the design of record
for that proof: what is proven, what is designed, what was tried and
refuted, and the exact workflow for the part still open. Read it before
writing any progress-lemma Lean. Companion docs: MODEL.md (the model),
README.md Phase C (status and narrative).

Epistemic key used throughout: **[proven]** = in the repo, kernel-checked;
**[checked]** = validated executably by `lake exe eventdag` on the pinned
matrix, not yet a theorem; **[derived]** = paper argument in this
document; **[open]** = known unknown.

## 1. Architecture: argmin over a schedule, not cycle-chasing

Suppose no action is enabled at a reachable non-terminal state. Then:

1. Every non-done process is **blocked at a channel operation with a
   determined next event** — never at a choice point. This is the
   enabledness pillar **[proven]**, `Proofs/Progress.lean`:
   `walk_uncommitted_canStep` shows a phase-2 uncommitted walk always
   has a choosable obligation, in *every* axiom mode (each
   `(!ax.flag || P)` guard conjunct is discharged by proving `P`
   outright from the invariant). Openers admit the same argument
   [derived, small]. So blocked processes are exactly: starving recvs,
   committed-jam sends, and close waits — all with a determined next
   event `e(P)`.
2. Assign each blocked process the timestamp `μ(P) = τ(e(P))` of its
   next event under a fixed valid schedule `τ` of ALL events (§3). Take
   the argmin `P₀` over non-done processes (`¬terminal` ⟹ the set is
   inhabited).
3. Per blocked mode, a **blame lemma** exhibits a non-done process `Q`
   with `μ(Q) < μ(P₀)` — contradiction with minimality, so some process
   was enabled after all. The blamed side needs
   `τ(e(Q)) ≤ τ(e_awaited)`, which is per-shape reasoning from `Q`'s
   local invariant, NOT blanket program-order monotonicity of τ (that
   stronger property is false; see §4).

This argmin form avoids formalizing "follow blame edges to a cycle,
then refute the cycle" — no path induction, no pigeonhole.

## 2. The proven layers (route-independent)

Everything in this section is consumed as-is by any variant of the
proof; none of it depends on how τ is defined.

- **Safety**: `inv_reachable` — `InvP` at every reachable state of every
  well-formed skeleton, every axiom mode. [proven]
- **Enabledness pillar**: `Proofs/Progress.lean` (see §1). [proven]
- **Counting layer**: `Proofs/Counting.lean` — whole-sweep supply =
  demand per channel family, each reduced through `wf_bfs_aligned` to
  `List.length_flatMap`. [proven] Inventory:

  | channel family | lemma | equals |
  |---|---|---|
  | interior wires, stage h+1 | `wiresBefore_full` | `stageLen h` |
  | leaf wires, stage 0 | `wiresBefore_full_leaf` | `totalLeafReqs` |
  | interior queries, stage h+2 | `qsBefore_full` | `stageLen h` |
  | leaf queries, stage 1 | `qsBefore_full_leaf` | `totalLeafReqs` |
  | resolutions, stage h+1 | `dsBefore_full` | #D scopes at h+1 |
  | asker level demand, height j+1 | `pendsBefore_asker_full` | #D scopes at j |
  | answerer level demand, height j+2 | `pendsBefore_answerer_full` | `stageLen j` |
  | answerer level demand, height 1 | `pendsBefore_answerer_leaf` | `totalLeafReqs` |

  These refute "the producer is done" inside every starve-mode blame
  lemma: a done producer has sent its whole-sweep total; the starving
  consumer's flow equation (`Inv`'s `flowOk`) then contradicts its own
  received count.
- **BFS alignment**: `wellFormed`'s final conjunct
  (`(scopesAt (h+1)).flatMap kids = scopesAt h`, extracted as
  `wf_bfs_aligned`). Every positional argument below keys a channel's
  n-th message to the n-th scope of the consuming stage; this equation
  IS that correspondence. Status honesty: a wellFormed-but-crossed
  skeleton (kids `[4]` before kids `[3]`) stays count-consistent and
  completes greedily with `Inv` held — the conjunct is a
  **proof-method requirement** aligned with the docstring's BFS promise
  and the Rust encoder's actual output, not a protocol finding.
  [proven, with the crossed probe result checked executably]

## 3. The event vocabulary and the DAG [checked]

Events are `(channel, side, seq)` for every message a completed session
carries. Edge families:

- **E1 (message)**: `snd(c,n) ≺ rcv(c,n)` — positional pairing is the
  protocol's identity carrier.
- **E2 (back-pressure)**: `rcv(c,n) ≺ snd(c, n + cap c)` — a full
  channel's next send waits on the consumer.
- **E3 (forced program order)**: only what the `.full` guards force.
  Per walk scope: `rcvW ≺ rcvA ≺ every send ≺ next rcvW`; in-order
  wires; per D child `i`: `wire i ≺ res i ≺ queries of i` (W, d1int),
  in-order D-res prefix, d4 (`last event of D block i ≺ wire (i+1)`),
  d2 (`res i ≺ parent`). **The parent send otherwise floats** — this
  drives §4. Asm/absorb/openers/fins are linear (ropen: gotWire ≺
  wire ≺ res ≺ queries).

`lake exe eventdag` builds this DAG per pinned skeleton, cross-checks
every channel's analytic snd/rcv totals against `sentOf`/`recvdOf` at a
drained terminal state, runs Kahn, and dumps longest-path depths.
Verdict (2026-07-16): **acyclic on all six pins** (positive matrix +
`jam`), totals pass everywhere, per-channel-side depth strictly
monotone in seq. Acyclicity of this DAG is precisely "a valid τ
exists"; the depth tables are the oracle for §5.

**The cap-1 experiment [checked]** (the `capOne` knob in `analyze`):
rebuild every E2 edge with all capacities forced to 1 and re-run Kahn.
Result: still acyclic on smokeChain, rMix, comb6, and `jam` (which
already runs at `capLevel = 1`), but **cyclic on both pyramids** — the
cycle is `lower rcv 2 → lower snd 3 → upper snd 0 → upper rcv 0 →
level rcv 0 → level snd 1 → (wrap)` at `(R, 2)`: the walk's floating
parent-send closes a loop through the asker-asm's level intake and the
answerer-asm below when the level channel loses its slack. Two
consequences. (1) It upgrades the Phase A `pyramidC1` negative from
"the greedy scheduler jams" to "**no schedule whatsoever** completes
the session" — cyclicity of the event DAG refutes every schedule, not
one. (2) It pins that `capLevel ≥ 2` slack on the level channels is
**load-bearing** for fan-shaped skeletons: the §5 construction must
consume the real `capLevel`, and any cap-1 simplification of the
E2 lemma family is refuted in advance.

## 4. Refuted designs — do not retry these

The natural candidate was a closed-form lex timestamp
`τ(event) = (DFS-pre of the event's scope, role)`. Two placements of
the floating parent-send/upper-recv pair were worked out and refuted on
paper, each by a reachable configuration [derived]; the oracle then
independently confirmed the conclusion [checked].

- **EARLY placement** (parent at `(pre y, role)` just after the scope
  prologue): breaks when the walk at scope `y` is committed to `.wire c`
  for a kid `c` (so `τ(e(walk)) = (pre c, 0) > (pre y, ·)`) while the
  asker-asm starves on `upper(y)` with `μ = (pre y, ·)` — the asm is
  the argmin and its only blame target is *later* than it. Committed
  choice is what makes this real: the walk cannot fire parent until the
  committed wire drains.
- **LATE placement** (parent at `(pre lastSlot(y), role)` or
  post-block): breaks the level-channel back-pressure chain — an
  answerer-asm below, jammed sending the return for an EARLY D kid
  `k′`, has `μ = (post k′, ·)`, smaller than the consumer's
  `rcvUpper(y)`, and the consumer is its only blame target. With
  several D kids and a walk committed to a query under the LAST kid,
  the two constraints on the parent's position are contradictory:
  it must sit both after `(pre g)` for a late grandkid `g` and before
  `(post k′)` with `post k′ < pre g`. No single static position exists.
- **Oracle confirmation**: longest-path depths are NOT affine in seq —
  they jump at subtree boundaries (`level R 2` snds on pyramid2:
  11, 13, 21, 23, 25, 27, 34, 36). The composition law the depths obey
  is `τ(sndOut(k′)) = max(post-subtree(k′), τ(rcvLvl(k′ − cap)) + 1)`:
  E2 injects CONSUMER-timeline positions into producer sends, which no
  per-scope pre/post role table can express.

Conclusion: **τ is tree-recursive, not closed-form.** Any future
attempt at a lex formula must first survive the eventdag edge check on
all six pins; the two configurations above are the minimal adversaries.

- **STATIC DFS COLUMNS (even with demand-pumped assembly)**: place all
  walk/opener events at fixed per-scope pre/post positions along the
  DFS (positions = prefix sums), and let only the assembly side float
  behind a demand pump. Passed all six pins; refuted by the random
  sweep (13/300 seeds) [checked]. Mechanism: when absorb or an asm
  tower stalls on a capLevel window, E3 drags the stalled process's
  WHOLE remainder along — leaf slot columns, then stage-0 prologues
  and uppers, then interior uppers at arbitrary stages (`upper I 3`,
  `upper R 2` violations observed) — past `post(parent)`. Stall
  regions relocate walk-side events, so no static position assignment
  exists. Positions must be merge-emergent (§5); do not retry
  spine-with-fixed-columns.

## 5. The chosen route: canonical schedule construction [checked]

Define a canonical serial schedule of all events as a **structural
recursion over the skeleton** (descent and assembly interleaved along
the DFS wavefront, with the cap-window slack), and take τ = position in
that list. Then:

- "τ is valid" = one lemma per edge family: for each E1/E2/E3 edge
  `u ≺ v`, `idx u < idx v` in the construction. The acyclicity proof
  and the τ definition are the same object.
- The blame lemmas (§6) consume only these positional facts plus the
  counting layer and `Inv`.
- The complete schedule is simultaneously the **Phase D termination
  witness** (an explicit run to `terminal`; the ρ-by-1 ranking rides on
  it). Two theorems, one artifact.

**Workflow discipline (validate-then-prove):** implement the candidate
construction executably FIRST (in EventDag.lean or a sibling), check on
all six pins that (a) it is a permutation of the event set, (b) every
generated edge is respected. Only then write Lean proofs. The oracle
exists precisely so no proof effort is spent on a wrong construction.

**The candidate exists, validates, and replays [checked]**
(2026-07-16, `EventDag.schedCandidate` + `validateSchedule` +
`replaySchedule`). The construction is the **deterministic priority
merge of the per-process E3-linear traces**: every process contributes
its trace (the `procTraces` node arrays — walks, openers, absorb, both
asm towers, fins split around the floating `rootret` receive), ordered
descent-before-assembly, and the merge repeatedly emits the first
trace whose next event has its E1 (message sent) and E2 (cap window
open) predecessors emitted. Two properties hold by construction:

- **Edge-respect**: an emitted event has every DAG predecessor
  emitted — E1/E2 checked at emission, E3 because each trace
  linearizes its process's forced order. The merge cannot emit a
  violating order; its only failure mode is stalling, which the
  permutation check catches.
- **Trace monotonicity**: τ is monotone along every process trace —
  the bulk of what the §6 blame lemmas consume (the blamed process's
  unperformed successor bounds its μ).

Both by-construction properties are now kernel-checked theorems
(`Proofs/Sched.lean`, 2026-07-16, transcription pinned to the tool by
the exact-equality gate): monotonicity structurally, against the final
state's actual remainder; edge-respect in counted guard-history form
(seq < prefix count at every emission index). Both are generic over
the trace list — no distinctness or numbering assumptions — so the
per-channel canonical-numbering layer and completeness are the only
Sched obligations left (§7 item 3). [proven] Two review-driven
hardenings of the same date: `MInv.out_count` (provenance — under
every predicate the output counts exactly the traces' emitted
prefixes; without it a duplicated-send output satisfies every other
field, and the numbering layer could not key the schedule's n-th send
to its producer's n-th), and `smokeChain_merge_complete` — a
kernel-`decide`d anchor that the merge drains every smokeChain trace,
which both blocks whole-file vacuity (a never-stepping merge satisfies
every generic theorem) and is the first kernel-checked INSTANCE of
merge completeness. [proven]

One linearization choice is load-bearing: the walk trace pins the
floating parent send **immediately after the scope's final resolution**
(after rcvA when no D kids), NOT at the scope's end. Parent-last
deadlocks the merge — the last D block's trailing queries need descent
that needs assembly that needs that very parent (a four-process cursor
cycle, fuzz seed 13) — while parent-after-last-res is safe: the upper
window depends only on strictly earlier scopes' subtrees.

Verdict: permutation + every-edge-respected on all six pins and on
every acyclic skeleton of the 300-seed random sweep (`runFuzz`, now
part of the tool's gate; the pins alone missed the parent-last bug and
the static-column design — random shapes are load-bearing). Correctly
NOT a valid schedule on `pyramid 1` (cyclic DAG, §3). And the
strongest check: `replaySchedule` compiles the candidate into real
model actions (commit-then-fire per send) and runs them through
`apply` under `AxMode.full` — every pin and every acyclic fuzz seed
**replays from `init` to `terminal`**, so the trace layer's E3 is
complete against the model's guards (not merely sound), and each
schedule is an explicit termination witness — the Phase D artifact,
already in hand at the executable tier.

**Finding: `wellFormed` does not imply schedulability — the progress
theorem needs a capLevel hypothesis.** The cap-1 cycle (§3)
generalizes. Derivation [derived]: within one parent `s`, the walk's
`snd res` for D kid `q+1` needs (cap-1 lower) the answerer-asm's
`rcv res(q)`, which needs (asm E3) `snd out(q−1)`, which needs (level
E2 window) the asker-asm one height up to have received the level
return of D kid `q−1−capLevel`; that receive sits (asm E3) behind
`rcv upper(s)`, i.e. behind the parent summary, which (d2) waits on
`snd res(q+1)` — a cycle exactly when both endpoints are kids of the
same parent, i.e. when some scope has `dCount ≥ capLevel + 3`.
Conjecture: the event DAG is acyclic **iff `∀ s, dCount(s) ≤
capLevel + 2`** (now `Skel.schedulable`) [checked: both directions
hold on all pins, on the capLevel-parametric boundary matrix in
`runAll`'s self-test (`boundaryProbe`, both sides at capLevel
1/2/3/4/6 — deterministic coverage the fuzz envelope cannot always
reach; an adversarial review caught the original sweep's fan cap
sitting BELOW `capLevel + 3` for capLevel ≥ 3, leaving the
theorem-critical direction unexercised on the boundary — the fan cap
is now 7 and the matrix pins it outright), and on all 300 seeds of the
random sweep — zero mismatches; `leafReqs` is confirmed
unconstrained — a single height-1 D scope with `leafReqs ≫ capLevel`
stays acyclic]. `jam`
(`capLevel = 1`, a 3-D-kid parent) sits exactly ON the boundary and
passes; `pyramid 1` (4 D kids, capLevel 1) violates it and jams. The
Rust implementation has `capLevel = FAN ≥ kids ≥ dCount` — margin 2 —
so this is a model-tightness fact, not an implementation bug, but the
progress statement MUST carry the hypothesis; `wellFormed`'s
`capLevel ≥ 1` alone is refuted by `pyramid 1`.

**Hypothesis form: DECIDED (2026-07-16) — the tight bound, promoted to
`Skel.schedulable` on the statement layer's audit surface.** [proven —
the definition plus kernel-`decide`d anchors: `pyramid1_not_schedulable`
and `positives_schedulable` (Statement.lean), `jam_on_boundary` and
`pyramid1_not_deadlockFree` (Controls.lean — the latter, a greedy
stuck run under `.full` via `drain_reachable`, makes the hypothesis's
load-bearing-ness itself a theorem, not just its non-redundancy; the
⟺-acyclicity claim and the universal "no schedule completes a
violating session" remain checked, not proven.] Tight over
Rust-faithful (`dCount ≤ capLevel`) because: (a) it
is the exact executable boundary, so the predicate coincides with "some
schedule exists" rather than with one proof strategy's slack; (b) `jam`
sits ON the boundary and is a pinned positive — the Rust-faithful form
would exclude it from the theorem's coverage and orphan the finding-#6
narrative from `deadlock_free`; (c) Rust coverage is identical either
way (margin 2). The proof-risk hedge inverts cleanly: if merge
completeness (§7 item 3) wants slack, weaken the THEOREM's hypothesis
and leave the predicate — a strengthening TODO, not a statement-layer
re-mint. `EventDag.schedulable` was deleted in favor of the promoted
predicate; `runFuzz` pins the model definition directly.

The §5 design risk (mutual recursion of the wavefronts) resolved by
giving up static positions entirely (§4's third refuted design): all
deferral lives in the merge, whose completeness ("every trace drains")
is precisely where the capLevel hypothesis will enter the Lean proof.
The stage-compositional rely-guarantee fallback was not needed for the
construction; it remains the reserve shape for the completeness
induction itself.

## 6. Blame lemma inventory [derived]

Per blocked mode of the argmin process: whom to blame, and what refutes
the blame target being done. "Counting" = the §2 table + flow equation.

| blocked mode | awaited event | blame target | done-refutation |
|---|---|---|---|
| walk phase 0 (wire starve) | `snd wire` for its scope | producer stage above (or opener at rootH) | counting: wires |
| walk phase 1 (asked starve) | `snd query` for its scope | launcher two stages above (or opener) | counting: queries |
| walk committed `.wire i` jam | `rcv wire` of stage predecessor | consumer stage below (or absorb) | consumer done ⟹ recvd = total ⟹ chan empty, contradiction |
| walk committed `.res i` jam | `rcv res` of previous D scope | answerer-asm at its height | ditto |
| walk committed `.query i` jam | `rcv asked` of predecessor | walk two stages below (or absorb) | ditto |
| walk committed `.parent` jam | `rcv upper` of previous scope | asker-asm at its height | ditto |
| asm phase 0 (res starve) | `snd res` / `snd upper` | walk at its height / one below | counting: res, parents |
| asm phase 1 (level starve) | `snd out` below | asm one height below (or absorb) | counting: level totals |
| asm phase 2 (out jam) | `rcv level` above | asm one height above (or fins) | ditto |
| absorb starves/jams | leaf wire / leaf request / level 0 | stage-0 walk, stage-1 walk, asm (·,1) | counting: leaf totals |
| opener jams (ropen query multi-shot) | `rcv asked` at rootH−2 | walk (R, rootH−2) | counting |
| close waits (phase 3/4, asm 3) | producer not done | the producer, whose own next event is earlier or a close one tier up | closes form a final tier ordered by stage; chains terminate at openers |

Structural facts already in `Inv` that these consume: walk recvd counts
by phase (scope k, phase 0 ⟹ k wires/k askeds; phase 1 ⟹ k+1/k;
phase 2 ⟹ k+1/k+1; phase ≥3 ⟹ stage totals), canonical prefix ledgers,
committed-arm coherence for all four obligations, `asm(R,1)` never in
phase 1 at height-1 scopes without kids.

**How the merge τ (§5) discharges these [derived].** Define
`μ(P) = τ(P's earliest UNPERFORMED trace event)` — trace position, not
execution order, so a walk committed to a later-in-trace obligation
still gets its μ from the earliest event it owes (the §1 "next event"
should be read this way). Then every starve-mode blame is the same
three-step: (1) `Inv`'s counts show the awaited `snd(c,n)` is
unperformed by its owner `Q` (the §2 counting layer refutes "Q is
done"); (2) unperformed ⟹ `μ(Q) ≤ τ(snd(c,n))` by trace monotonicity —
by construction of the merge, no per-shape positional arithmetic;
(3) `τ(snd(c,n)) < τ(rcv(c,n)) ≤ μ(P₀)` by the E1 edge. Jam modes are
the mirror image through the E2 edge. What remains per-shape is only
step (1), which is exactly the `Inv` bookkeeping the table's
done-refutation column already names. This also dissolves the old
worry that a blocked launcher committed to `.parent` while owing a
query needs a parent-vs-query τ fact the DAG cannot supply: with μ
over unperformed trace events, whichever of the two is trace-earlier
bounds μ(Q), and both are bounded by the awaited send.

## 7. Remaining work, in order

1. ~~Executable candidate schedule + eventdag validation~~ — done
   (§5, `EventDag.schedCandidate`), validated four ways: edge-check on
   the pins, 300-seed random sweep (`runFuzz`, in the tool's gate with
   self-testing negative controls: pyramid-1 cyclicity, pyramid-1
   candidate rejection, E1-swap mutation), model replay to terminal
   (`replaySchedule`), and the greedy-trace coherence pin.
2. ~~Decide the capLevel hypothesis form and thread it into the
   statement layer~~ — done: the tight form as `Skel.schedulable`
   (§5, decision paragraph), with kernel-checked anchors pinning
   non-redundancy (`pyramid1_not_schedulable`), positive-matrix
   coverage (`positives_schedulable`), boundary exactness
   (`jam_on_boundary`), and — post-review — load-bearing-ness itself
   (`pyramid1_not_deadlockFree`: the greedy run under `.full` is
   kernel-checked stuck one D kid past the bound). The Phase C target
   statement is now
   `sk.wellFormed → sk.schedulable → DeadlockFree sk .full`
   (Statement.lean's `DeadlockFree` docstring).
3. `Proofs/Sched.lean`: ~~transcribe the merge + the by-construction
   lemmas~~ — done. Traces as prefix-sum folds (`wiresBefore` &c.
   replace the running counters, connecting the traces to the counting
   layer), merge as a fuel-indexed fixpoint over remaining-suffix
   lists, pinned event-for-event to the tool's `schedCandidate` by the
   eventdag gate (all pins + 300 seeds, exact equality). Kernel-checked
   and generic over ANY trace list: `trace_monotone` (structural form,
   pinned to `finalState.rem` — an existentially-quantified suffix is
   trivially satisfiable at `pre = []`; see the docstring),
   `schedule_e1`/`schedule_e2` (counted guard-history form, τ-indexed),
   plus the review-driven `MInv.out_count` (provenance: the output
   counts exactly the emitted trace prefixes under every predicate —
   added while the merge induction was open precisely so the numbering
   layer never has to reopen it) and the `smokeChain_merge_complete`
   kernel anchor (non-vacuity + the first completeness instance).
   ~~(a) the canonical per-channel numbering layer~~ — done
   (`Proofs/Sched/Numbering.lean`), and the eventdag gate now checks
   the layer's own claims (`numberingErrs`: canon per-trace
   projections, one producer/consumer per channel-side, canon schedule
   projections) on every pin and acyclic seed — validated before any
   Lean was written. The shape: `proj`/`seg`/`canon` name the
   projection algebra; every block projects to a segment whose offset
   is a Skel prefix sum, so each trace folds to canon
   (`procs_canon`) — the parent splice is proven projection-invisible
   (`proj_scopeSends`), and the in-scope rank totals (`dRank_total`,
   `qSum_total`) meet the outer telescopes (`wiresBefore_succ` &c.)
   exactly. Cross-trace uniqueness is OWNERSHIP, not pairwise
   disjointness: `sndOwner`/`rcvOwner : Chan → Nat` name each
   channel-side's unique trace index and every family proves its
   events point at itself (`procs_snd_owned`/`procs_rcv_owned`, the
   only lemmas needing `wellFormed` — parity and `rootH ≥ 2`); two
   producers would name two indices at once. The consumer
   (`emitted_canon` → `schedule_proj_canon`) squeezes `out_count`
   between `Sublist.filter` and the canon prefixes: the SCHEDULE's own
   projections are canon. Corollaries: `schedule_e1_pos`
   ("`snd(c,n)` precedes `rcv(c,n)`", positional E1) and
   `schedule_inj` (τ injectivity); kernel anchors
   `smokeChain_schedule_nodup`, `smokeChain_level_canon`.
   Still open in Sched: (b) merge COMPLETENESS (`finalState.rem` all
   empty) — the real content, where `Skel.schedulable` enters; the
   reserve shape is the Phase B stage-compositional induction.
   **Stall-refutation design: DECIDED and tool-validated
   (2026-07-16).** The shape is the §1 argmin transplanted to the
   Sched layer — no cycle-chasing, no path induction:
   - *Reduction 1 (fuel, generic).* Each `step` drains one event and
     fuel = `totalEvents`, so a non-empty `finalState.rem` forces a
     reachable STALLED state: some trace non-empty, every non-empty
     trace's head disabled.
   - *Reduction 2 (blame is a function, from 3a).* At a reachable
     state, a disabled head names its blocker: `rcv(c,n)` starved
     (`n ≥ sent c`) blames `snd(c, sent c)`; `snd(c,n)` jammed
     (`n ≥ rcvd c + cap c`) blames `rcv(c, rcvd c)`. By canon +
     ownership (3a) + per-channel totals (snd total = rcv total,
     already tool-checked in `analyze`), the blocker exists, is
     unemitted, and sits in the remaining suffix of its unique owner
     trace — at or after that trace's head.
   - *The invariant: a WEAK POTENTIAL φ : Ev → Nat*, strictly
     increasing across every E1 edge (`φ(snd(c,n)) < φ(rcv(c,n))`)
     and every E2 edge (`φ(rcv(c,n)) < φ(snd(c, n + cap c))` — where
     `schedulable` must enter, on the level channels), and *weakly*
     increasing along every trace of `procs`. Then at a stalled
     state, blamed-head φ < blocked-head φ (weak up the owner's
     suffix, strict across the blocking edge, weak along same-channel
     sends/rcvs), and the argmin head over non-empty traces is a
     contradiction. §4's refutations do not apply: φ is a coarse
     rank with massive ties (E3 only weak), not a position order.
   - *Tool validation (`EventDag.weakPotential` + `blameProbe`, in
     both gates).* `weakPotential` computes the pointwise-least φ
     (weighted Kahn: E1/E2 edges weight 1, trace-consecutive weight
     0; exists iff acyclic). `blameProbe` replays the merge and at
     EVERY reachable state checks, for every disabled head: blocker
     owner exists and is unique, φ strictly drops from blocked head
     to owner's head, and blame chains reach an enabled head with no
     trace revisited. Green on all six pins and all 300 acyclic fuzz
     seeds; negative controls: `pyramid 1`'s probe must find a blame
     cycle at its stall and its `weakPotential` must be `none`. The
     observed blame-edge alphabet (`.blame.tsv` per pin) matches the
     §6 table exactly.
   - *The φ witness: the tree-recursive WEAVE, validated
     (2026-07-16, `EventDag.weaveOrder`).* The minimal φ is NOT
     per-channel affine in seq (`.phi.tsv` + critical-edge
     provenance: jam's `asked I 1` snds sit at φ 2, 5, 12 — jumps at
     subtree boundaries, §4's mechanism recurring at the potential
     level), and per-height linear forms are refuted analytically
     (the level-window wrap forces per-block granularity). So φ is
     not a formula at all: `weaveOrder` constructs a FULL topological
     order of the event DAG by structural recursion over the scope
     tree, and φ = position in it (strict everywhere ⊇ the weak
     potential the argmin needs). Two mechanisms carry the whole
     design: (1) QUERY FEEDS — a scope's chunk-`i` queries (for kid
     `i`'s kids) pass down as kid `i`'s feed and are emitted one per
     kid-chunk, matching the cap-1 asked-channel E2 exactly while
     preserving the issuer's trace order (all of a chunk's queries
     precede the next chunk's wire because the recursion returns
     first); (2) GREEDY ASSEMBLY PUMPS — the linear traces (absorb,
     asm towers, float, fin) drain greedily after every descent
     emission; pump emissions only raise counts, so greedy pumping
     is confluent. The parent summary follows the last resolution
     (the §5 splice), before that kid's feed and descent. Validated:
     permutation + every-edge-respected (`validateSchedule`, the
     same checker as the merge candidate) on all six pins, all 300
     acyclic fuzz seeds, and the capLevel boundary matrix (completes
     ON `dCount = capLevel + 2` at every capLevel probed, is
     rejected one past); `pyramid 1`'s weave is rejected (negative
     control). The weave is NOT the schedule: τ and the blame
     lemmas stay with the merge; the weave only witnesses that a
     valid completion exists.
   - *~~Transcription~~ — done (`Proofs/Sched/Weave.lean`,
     2026-07-16): `Sched.weave` as a fuel-indexed WORKLIST
     interpreter (`WOp`/`weaveGo` — restructured from the first-cut
     mutual recursion because `WellFounded.fix` does not iota-reduce
     in the kernel, and structural fuel both reduces under `decide`
     and hands the validity proofs one induction principle), with the
     KEY reuse: the weave state IS `MState` and the pump IS `mergeN`
     restricted to the pump traces, so the whole `MInv` layer applies
     to weave states unchanged. Pinned event-for-event to the tool's
     `weaveOrder` by the eventdag gate (pins + seeds; the tool pump
     was aligned to `mergeN`'s scan order); kernel anchors pin
     length + Nodup on the smallest pin.*
   - *~~Weave counting layer~~ — done (`Proofs/Sched/Weave/
     Count.lean`, 2026-07-16): the `WCount` invariant — `MInv` for
     weave states, with the manual traces' remainders RECOVERED from
     the worklist by ownership (`manFilters` filters the ghost
     futures `goEvents`, the fuel-locked twin of `weaveGo`, by
     `evOwner` from 3a) rather than carried as state. Preservation is
     closed through `wEmit` (the owner's remainder advances by
     exactly the emitted event), the pump (`scan_step` re-consumed
     verbatim on the pump suffix of `procs`), and the `weaveGo`
     master induction; NO enabledness hypothesis anywhere, per the
     permutation/edge-respect split. `weaveState_wcount` reduces the
     layer to two open alignment hypotheses: the opening worklist's
     per-owner filters ARE the manual traces, and every future is
     manual-owned.*
   - *~~Initial alignment~~ — done (`Proofs/Sched/Weave/Align.lean`,
     2026-07-16): `weave_wcount` — the weave state satisfies `WCount`
     with NO remaining hypotheses. The master induction
     (`align_scope`, by stage): a subtree op's per-owner filters are
     (1) each covered walk's contiguous `descIdx` run — at the own
     stage the `scopeBlock` itself, the kid feeds resplicing the
     chunk queries into `scopeSends`' §5 splice via clause (2) one
     stage down — (2) the feeder's feed in order, (3) nothing else.
     The top assembly instantiates at the root scope op: root-stage
     uniqueness (`wf_root_stage`, from the kid accounting — dedup'd
     kids ARE the non-root ids by `Subperm` pigeonhole, the parent
     chain caps every non-root height) makes the telescope endpoints
     (`descIdx_zero_arg`/`descIdx_total`) cover whole stages, and the
     filter-partition length identity discharges `weaveFuel` through
     `goEvents_weave`. The weave's output is a permutation of the
     manual traces riding the pumps: the permutation half of weave
     validity, closed.*
   - *~~Edge-respect, generic + discharge + manual-manual layers~~ —
     done (2026-07-17, `Weave/Edge.lean` + `Weave/Prec.lean`):
     `WEdge` (= `WCount` + `MInv`'s guard-history fields), preserved
     freely by the pump (`scan` checks) and by manual emission under
     an `enabled` hypothesis; `wPump_fixpoint` (sum-length fuel runs
     the merge to a stuck state); the discharge toolkit —
     `wproj_canon` (EVERY weave state's projections are canonical, so
     each guard is a membership claim: predecessor ∈ `out`),
     `mem_out_of_elsewhere` (conservation with no counting),
     `pump_support` (pumps never touch wire-above-leaf or asked
     channels); and `weave_goEvents_depOK` — `DepOK`, the dep-closure
     of the initial ghost future (each manual-manual predecessor lies
     strictly earlier), by a second `align_scope`-style master
     induction (`dep_scope`) with the query-base identity
     `queries_base` (chunk-query seqs = kid-stage scope indices).*
   - *Edge-respect is COMPLETE [proven]*: the pump-window discharges
     landed as the `Weave/Window.lean` window lemmas, and the layer-D
     assembly as `Weave/Master.lean`'s `weave_wedge` (see the (f)
     record below). Design of record for the pump case-tree, as
     designed and landed (derived 2026-07-17, all cases closed): at each such emission,
     suppose the window shut; the consumer asm/absorb's remainder
     head (its seq = the current count, by canon-suffix) is disabled
     at the pump fixpoint, and the head trichotomy — res-starved /
     level-starved / out-blocked / exhausted — closes as follows.
     Starved-against-blocked and exhausted cases close purely or by
     accounting (`pendsBefore` totals = producer totals). Res-starved
     closes against POSITION FACTS: completed-subtree boundaries
     below (`∈ past` memberships of boundary sends), ancestor
     res/upper memberships above. The DESCENT (consumer's supplier
     chain, downward) costs one boundary membership per two stages
     and bottoms out at absorb (leaf wires/`leafRequests` of complete
     subtrees). The ASCENT (out-blocked chain, upward) alternates
     answerer/asker per stage; at each answerer the pends-coverage
     accounting (`pendsBefore` through the ancestor's res ≥ the
     descendant's stage index + 1) kills it; at each asker it needs
     the ancestor's CURRENT rank `r`: if `r` is the scope's last
     D-rank the §5 SPLICE has already emitted the ancestor's upper
     (the load-bearing placement, again), else `r + 2 ≤ dCount ≤
     capLevel + 2` — `Skel.schedulable`, biting exactly at the
     boundary as in the executable matrix. Position facts are
     supplied per position as an ∃-packaged ancestor context
     (`PumpObl`/`CtxOK`, a pointwise list property like `DepOK` but
     with existential ancestor coordinates — no closed-form ascending
     index needed), established by a third tree induction carrying
     the ancestor path. Bottom-up build order, with status
     (2026-07-17): (a)–(c) ~~state layer + cell shapes + stuck
     trichotomies~~ done (`Weave/Pump.lean`) — `out_proj_owner`,
     `cell_head_seq`, `cell_not_out`, `wedge_rcvd_le_sent`, the
     `procs` positional reads, `prefix_flatMap`, and the four stuck
     lemmas: `asm_stuck` (exhausted / res-starved / level-starved /
     out-blocked, all counts pinned, failed guard recorded),
     `absorb_stuck`, `fin_stuck`, `rootret_stuck`. (d) ~~cursor
     accounting~~ done (`Proofs/Counting.lean`) — `pendsBefore_asker`
     (= `dsBefore (j-1) k`, all cursors), `pendsBefore_asker_one`
     (height-1 askers pend nothing), `pendsBefore_answerer`
     (= `wiresBefore (j-1) K` at the D-filtered cursor), the
     `asmResList` length lemmas, `wf_scopesAt_zero`,
     `foldl_add_take_le`. (e) ~~the four window discharges~~ done
     (`Weave/Window.lean`; ascent package reworked 2026-07-17, see
     the boundary resolution under (f)) — `upper_window`,
     `lower_window`, `wire0_window`, `leafreq_window`, each
     concluding `seq ≤ rcvCount` at a pump fixpoint from: `hsnd`
     (the seq about to go out IS the send count — layer D reads it
     off `cell_head_seq`), a bound placing the seq inside the trace
     total, and the POSITION PACKAGES `DescSupply` (recursive: res
     present through the demand each level hands down via
     `pendsBefore`, bottoming at absorb's wire+request feeds) and
     `AscCover` (per ANSWERER stage in the ascent range, two count
     facts: `Φ` — `snd(level below) < pendsBefore(snd lower)`, the
     in-flight resolution's allocation not yet delivered from below —
     and `P1` — `snd lower ≤ dsBefore(snd upper) + capLevel + 1`,
     the walk's schedulable overhang bound) plus
     `1 ≤ sndCount rootres`; `lower_window` additionally takes
     `hp1`, the emitting walk's own `P1` at the unsent seq. All four
     windows and `tower_noblock` now take `WEdge` (the ascent needs
     `wedge_rcvd_le_sent`). The chains: `tower_deliver` (descent
     recursion; `absorb_deliver` at the base; height-1 asker killed
     by `pendsBefore_asker_one`), `tower_noblock` (ascent recursion
     carrying `hself` — the asker-entry fact `snd(level below) <
     dsBefore(snd upper) + capLevel` — with `top_blocked` killing
     the two tops via the singleton `rootret` total and `rootrets`
     total = `rootPending`), with `pends_total_prod` (consumer pends
     total = producer res count) and
     `level_snd_le`/`level0_snd_le`/`levelR0_snd_zero` (count ≤
     owner trace total; the phantom responder level 0 is silent)
     closing exhaustion, and `cap_pos`/`wf_capLevel` the pure
     starving-vs-blocked contradictions.
     NEXT IN ORDER: (f) the `CtxOK` layer establishing the window
     lemmas' hypotheses at each pump-facing manual emission. Started
     (`Weave/Ctx.lean`): `walk_prefix_lower` (the own-walk descent
     brick — a cell headed at the scope-`k` parent summary carries
     every earlier scope's resolution, via the de-privatized
     `proj_block_*` family and `proj_flatMap_seg`), plus the
     telescope counting steps in `Proofs/Counting.lean`
     (`take_flatMap_blocks`, `ds_wires`, `pendsBefore_answerer_ds`).
     The ascent BOUNDARY is resolved (the `AscCover`/`hself` rework
     below, landed in the (e) layer), and the position layer has a
     COUNTING ROUTE that supersedes the membership induction
     originally planned here (2026-07-17, second pass): every
     window-lemma hypothesis — `DescSupply`, `Φ`, `P1`, `hsnd`,
     `hroot`, the leaf locals — is a pure count fact, and every
     needed count is derivable at any interpreter position from
     `WCount.man_struct`: each manual trace is (emitted prefix) ++
     (its owner filter of `fut`), so for a walk-owned channel
     `sndCount c out = (proj c of the trace).length − (proj c of
     the fut filter).length`. The trace totals are the `walk_canon`
     segs; the fut side is computed from the worklist tail by the
     `align_scope` clause-3 partition (a subtree's stage-`h'` events
     are the `walkSeg` over `descIdx` windows) plus
     per-partial-scope chunk shapes (`scopeSends_eq`/`splicedChunk`,
     to de-privatize). Pump-owned counts (`level`) never need direct
     pins: the `Φ` telescope bounds them through walk counts via
     `asm_out_le_res` and the new `asm_pends_le_out` (landed).
     So layer D carries NO extra position invariant — the worklist
     shape it already inducts over determines every pin. Landed
     bricks (2026-07-17): the splice vocabulary de-privatized;
     `align_kids_suffix` (Align.lean — the tail partition: a
     mid-scope worklist suffix's filters are the remaining
     `splicedChunk` run at the own stage, `walkSeg` over `descIdx`
     windows below, and `F.drop i` on the feeder; no new induction —
     each unwoven kid subtree is whole, so `align_scope` covers it);
     `SpineLink`/`phi_of_spine` (Ctx.lean — `Φ` by downward
     induction over per-stage count links, base links capping the
     producer asker by an unsent summary, step links refuted through
     `asm_pends_le_out` twice); `Emit.lean` (NEW — the per-emit
     assembly layer, upper-emission prototype): `futLen` (an owner's
     share of the future on a channel-side) with **the interface
     finding that per-stage `futLen` values ARE the RestCtx** — no
     monolithic predicate; `count_pin` (emitted + future share =
     whole-trace total, through `man_struct` + `out_proj_owner`),
     the trace totals and assembled pins
     `upper/lower/wire_snd_pin` + `rootres_pin` (hsnd, hroot),
     `p1_of_position` + `schedulable_dOf` (P1 — where `schedulable`
     bites) and `splice_link` (`SpineLink.step`'s pends identity),
     `descSupply_step` (two descent stages per step, in cursor form)
     + `descSupply_base_I/R` (absorber feeds; pend-free `R` base).
     The window-site brick campaign (2026-07-17, the residue's
     items (1)–(3), spec'd by an adversarially-verified multi-agent
     design pass, landed complete in four phases, `b94a73e6` …
     `79c29dfd`): Phase 1 pure bricks — `asks_add_two`,
     `dsBefore_mono`/`dRank_mono`, `kid_index_lt`/`spine_nest` (the
     window-nesting inequalities' omega half),
     `childIsD_eq_kid_kind`, `descIdx_peel`/`descIdx_le_stageLen`,
     the `lastDOf` splice facts (`lastDOf_max`,
     `lastDOf_isSome_of_D`, `dRank_lastD`, `dRank_below_lastD`),
     `ds_wires_mid` → `pends_cut_mid` (THE mid-cursor pends
     conversion), `qs_wires`/`qs_wires_mid` (a stage's query
     numbering IS the kid stage's wire numbering),
     `answerer_resList_total`. Phase 2 the futLen residue —
     `asked_snd_pin`, `feed_rootres_silent`, the `chunkQ` mid-feed
     windows, `futLen_anc_upper/lower` (the in-flight ancestor's
     three-segment tail; the t-cursor cancels, so the pins are
     insensitive to feed progress), the five `futLen_site_*` forms
     carrying their strict in-range bounds as conjuncts; Site.lean
     (NEW) with the four `*_site_hsnd` wrappers. Phase 3 the ascent
     bottom — `absorb_out_le_req` (request-side: the wire-side
     count touches the cut at the last request slot and is
     uninhabitable there), `SpineLink.absorbBase` + a new
     `phi_of_spine` arm (the campaign's one edit to landed code; a
     constructor cannot specialize the uniform parameter `p`, hence
     the `hp : p = Party.I` equation), `spineLink_absorb_at`. Phase
     4 the assemblers — P1 (`p1_of_lower_site`,
     `anc_position_counts`, `p1_of_anc`; every covered ancestor is
     in the `+1` position shape, the only non-`+1` P1 is
     `lower_window`'s own-stage `hp1`), the spine ladder
     (`spineLink_base_at`/`spineLink_step_at` with ancestor
     coordinates spelled inline, `ascCover_of_spine`; per ancestor
     the σ discriminant is COMPUTED as `lastDOf g A == some jD`,
     never carried), and the descent packages (`descSupply_down` —
     the assembled telescope, whose two-peel cut re-basing is
     `rfl`, which is why the feared subtraction arithmetic never
     bit — `descSupply_step_asker`,
     `descSupply_upper_site`/`_zero`/`descSupply_lower_site`,
     concluding the windows' `hdesc` hypotheses verbatim).
     Phase 5 LANDED COMPLETE — LAYER D IS CLOSED (2026-07-17:
     eb71f7cc..52199fb9 the consumption half, the telescope, the
     ladders/coverage, the floor counts, the descent packages, the
     five site discharges, and the leaf case; 2b812c92
     `emitOK_kids`, the interior fold; 9a286b6a `emitOK_scope` +
     the top assembly). The theorem:
     `weave_wedge : wellFormed → schedulable →
     WEdge sk [] (weaveState sk)` =
     `weaveState_wedge_of_emitOK ∘ emitOK_weave`. The production
     half as landed, in `Weave/Master.lean`: `EmitOKOn` (pointwise
     emission-readiness of the ghost future) is established by
     `emitOK_scope`, a structural induction over stages whose
     entry context per scope `(h, k, rest)` is five clauses — the
     after-scope low windows (`walkSeg` from `descIdx (k+1)`
     cursors), the `AncTele` telescope over `rest` with the parent
     feed cursor SATURATED (the un-consumed feed lives in the
     scope's own expansion, rebuilt per site by `ancTele_rebase`),
     the coherence link `hcoh0`, and the openers'-share clause
     stated ABSTRACT over the consumed prefix (∀ pre c,
     pre.filter mF = feed.drop c → foreign-uniform → ∃ i₀, …) so
     it composes through the recursion — at the root the feed IS
     ropen's tail (`ropen_drop_eq_feed`) and the clause discharges
     itself, which is how one statement serves both the interior
     scopes (mF = walkIdx (h+1)) and the root (mF = 1, guarded
     `hmFeq` vacuous). `emitOK_kids` folds the slots: per D slot
     wire (manual) → resolution (`ready_lower`) → splice summary
     (`ready_upper_splice`, when `lastDOf == some i`) → feed query
     (manual, `askedOut = asked` for interior stages) → subtree by
     the stage-below IH with the pushed context (coordinates
     `(k, i)` at the scope stage via positional if-updates, parent
     cursor `i+1`, own chunk saturated by
     `chunkQ_length`+`drop_length`, `isD` at h+2 re-derived by
     `parent_slot_isD`, low windows by `deep_glue` at `i+1`, the
     owner-1 clause composed through `hfd` at `pre := laterflat`);
     W slots are the manual pair plus a childless subtree
     (`nChildren_kid_notD` + `scopeFeed_nil`, same IH). The top
     assembly `emitOK_weave` peels the five openers (seq-0
     `enabled_snd_low` ×4 + the wire receive from its `manDep`
     predecessor) and enters the root scope with the trivial
     context: empty tail (every low window sits at its
     `descIdx_total` endpoint = `stageLen`), vacuous telescope,
     vacuous guards. New traps from the interior fold, beyond the
     leaf case's list: the site's OWN event heads its `fut` — every
     per-site filter computation (hown/hdeep/tele-rebase/hfeed)
     must peel it too, not just the later heads; `head_snd_wire`'s
     stage unifies as the opaque projection `(wpk (hp+1)).2`, so
     its `1 ≤ hh` side goal needs `show 1 ≤ hp + 1` before omega
     (same for `askedOut`'s if-condition: `show ¬(hp+1+1 < 2)`,
     and close the `askedOut = asked` bridge with a trailing
     `rfl` — the projection arithmetic is defeq but not
     rfl-at-reducible); hown chains need TWO `filter_append`s
     (`(subEv ++ L) ++ rest` after one assoc); a cons-headed
     `pre ++ rest` is DEFEQ to the goal's fut (cons_append
     reduces), so `ancTele_rebase (pre := lowEv :: … :: (subEv ++
     L))` unifies against `lowEv :: … :: ((subEv ++ L) ++ rest)`
     with no propositional assoc — only opaque-left appends need
     `List.append_assoc` rewrites; `rw [h1]` with `h1 : (1:Nat) =
     …` rewrites EVERY literal 1 including inside `rootH - 1 -
     g'` — rewrite inside the lemma instance (`rwa … at h2`)
     instead; pass `descIdx_total`'s depth explicitly (a `_` there
     leaves a metavariable the `by omega` side goal cannot see).
   - *~~Closing (b)~~ — DONE (task #11, 2026-07-17,
     `Weave/Final.lean`, a6786e05 + 72f772a5): MERGE COMPLETENESS
     IS PROVEN — `merge_complete : wellFormed → schedulable →
     (finalState sk).rem.all isEmpty`.* Two halves:
     - *The drained weave* (`wFinal := wPump (weaveState)`, a merge
       fixpoint by `wPump_fixpoint` — no weaveGo analysis needed):
       every trace is a sublist of its output
       (`all_sublist_wfinal`). Manual traces are whole because the
       future is spent (`man_proj_full` via `out_proj_owner` at the
       empty cells); the pump towers drain by a stall refutation at
       the fixpoint — `chain_no_jam` (a level-feed jam forces, arm
       by arm through `asm_stuck`, a jam on the tower's own output,
       climbing into `top_blocked`), then the bottom-up
       `asm_counts_full` induction collapsing each trichotomy to
       its exhausted arm (absorber base from the drained stage-0/1
       walks via `wiresBefore_full_leaf`/`qsBefore_full_leaf`;
       level feeds bridged by `pends_total_prod`; responder base by
       `pendsBefore_asker_one`), fins and the floating root return
       last; cells are then literally empty (`cell_not_out` against
       the totals per head shape).
     - *The argmin* (`merge_complete`): heads of non-empty final
       remainders are disabled (`scan_none_heads` at
       `mergeN_fixpoint`); rank them by weave position (`evIdx` —
       total by `all_sublist_wfinal`, unique by `count_canon`
       through the canonical projections). The minimum head's
       blocker is the send at the current count (starved receive)
       or the receive its cap window awaits (jammed send); it
       EXISTS by the weave's own edge-respect at the last seqs
       (`hRS`/`hSR`: E1 on the last receive gives rcv-total ≤
       snd-total, E2 on the last send gives snd-total ≤ rcv-total +
       cap — exactly enough slack for both blame cases, so the
       anticipated per-channel totals sweep was UNNECESSARY), is
       unemitted by canonical freshness
       (`not_mem_schedule_of_count`), and sits in its owner's final
       remainder behind a head (`blame_head`) that E1/E2-in-the-
       weave (`mem_take_snd`/`mem_take_rcv` reading counts off
       canon prefixes) places strictly below the minimum —
       contradiction. Consequences now available downstream: with
       all remainders empty, `trace_monotone` specializes to "every
       trace is a sublist of the schedule", and `schedule_count`
       pins the schedule's per-channel totals to the whole-trace
       totals; τ is total on the event set.
     - *New traps:* a `rw [sndCount_eq_proj] at h` whose FIRST
       match is a different instance than the intended one leaves
       the target count unrewritten and later unification whnf-
       diverges trying to defeq `sndCount` against a proj-length
       inside `wFinal` (symbolically evaluating the weave) — pass
       counts through `▸`-wrappers (`mem_take_snd`/`_rcv`) so atom
       spellings stay consistent for `omega`; rewriting a
       projection-shaped hypothesis whose event seq itself spells
       `(proj …).length` duplicates the pattern inside the
       replacement (use `proj_mem_of_lt`, which rewrites `← hcanon`
       on a `canon_mem` fact instead); `List.Sublist`'s second
       constructor is `cons_cons` (two binders) in this toolchain;
       `∃ i j, i < j ∧ l[i]? = …` needs `: Nat` annotations or the
       `LT` instance sticks; `asmEvents`' `(p, j).fst`-spelled
       length atoms need type-ascribed `have`s before omega.
4. ~~Opener/asm enabledness mirrors of the pillar~~ — done
   (`Proofs/Progress.lean`, 2026-07-17): `iopen_unchosen_canStep` /
   `ropen_unchosen_canStep` (the first unfired obligation in wire ≺
   res ≺ query order passes every guard in every mode; the query
   count stays choosable by `topLocalOk`'s `rootPending` bound). Asms,
   absorb, and the finishes are linear — every action a channel op or
   close determined by phase — so the pillar's content is vacuous for
   them; nothing to state until the stuck analysis consumes it.
5. **THE PARENT-DELAY FINDING (2026-07-17): `DeadlockFree sk .full`
   was FALSE as stated — refuted executably, then RESOLVED the same
   day by the `d5` (parent placement) ledger. The adjudication was
   "amend and finish" (statement owner, 2026-07-17), with the Rust
   trace proptests to be extended in tandem so the proptested local
   invariants and the formal ledger set stay in lockstep; the
   amendment is landed (see the resolution record after the finding),
   and items 5–6 now target the amended `.full`.**
   - *How it was found.* Transplanting the §6 argmin to model states
     needs each blocked process's earliest unperformed trace event to
     be the event it is blocked on. Under `.full` that holds for every
     process EXCEPT a walk that commits past its floating parent: the
     guards let a walk whose D children are all resolved commit (and
     jam on) a last-chunk query or trailing W wire with the parent
     still unsent — the parent is the ONLY event any process can owe
     out of trace order (openers are forced linear by their guards;
     asm/absorb/fins have no choices; every other walk deviation is
     excluded by w/d1int/wireFirst/d4/in-order ledgers). At such a
     state the argmin's blame has nothing below the hole to indict:
     §6's step (3) fails, and `blameProbe` never saw the case because
     merge-reachable states consume traces in order — hole-free.
   - *The refutation.* `EventDag.advActions`/`drainAdv` (in both
     gates): the greedy driver with each walk's `.parent` commit
     moved after its child-obligation commits. On schedulable fuzz
     seeds it reaches genuinely stuck states (`terminal = false`,
     `canStep .full = false` — checked against the real `allActions`
     enumeration; every adversary state is `Reachable` since only the
     choice among enabled actions differs). First witness, seed 12,
     carries BOTH flavors at once: `walk(R,2)` committed `.query 4`
     with parent unsent jams the cap-1 asked channel; its unsent
     parent starves `asm(R,3)`, the level tower backs up two heights,
     `asm(R,1)` stops draining `upper(R,0)`, so `walk(R,0)`'s
     committed `.parent` cannot fire and never reaches the next
     scope's asked-receive — closing the cycle back at the asked
     channel. (`walk(I,1)` sits in the trailing-wire flavor of the
     same trap.) Pinned both ways: `runAll` asserts the six pins
     complete under the adversary; `runFuzz` asserts the stalls
     reproduce, so a model change cannot dissolve the finding without
     a deliberate re-audit. The commit/fire split is load-bearing
     here exactly as designed — the deadlock is real under committed
     choice, and un-committing would hide it.
   - *Why this is a model-tightness finding, finding-#6-shaped.* The
     weave pins the parent immediately after the final resolution
     (the §5 load-bearing placement), matching the Rust encoder's
     order; the model's `wkChoosable` never encoded that. The
     resolution is a seventh ordering ledger — `AxMode.d5`, parent
     placement: *a walk with every D child resolved must send the
     parent before committing any further wire or query*.
   - *The resolution, landed (2026-07-17).* The exact guard, as
     minted (the conjunct appended to `wkChoosable`'s `.wire i` AND
     `.query i` arms, `Model.lean`; mirrored verbatim in
     `wkLocalOk`'s committed `.wire`/`.query` matches):

     ```
     (!ax.d5 || ws.parentDone ||
       !(List.range n).all fun j => !sk.childIsD h s j || ws.resDone j)
     ```

     Plain-English local-invariant spelling for `Trace::assert_valid`
     (the Rust twin): *scanning a walk's publication stream, once the
     resolution of the scope's last disputed child has been emitted,
     the next wire or query of that same scope before the parent
     summary is a violation* — equivalently, the parent summary sits
     immediately after the final resolution up to already-owed
     queries of EARLIER children, and first in an undisputed scope
     (no disputed children ⇒ no wire/query may precede the parent).
     Note the guard binds from scope entry when the scope has no D
     children at all — the weave sends such parents first, and the
     `.res` arm is deliberately NOT guarded (resolutions are what
     turn the condition on, and d2 already orders the parent after
     them).
     - `AxMode` surgery: `d5 : Bool` inserted after `d4` (before the
       `wireFirst` scaffolding); `.full` gains it; the pre-finding
       set survives as `Control.fullNoD5` (the control mode), and
       `Control.fullNoD4` is now explicitly pre-BOTH findings.
     - Kernel controls (`Controls.lean`, all `decide`, no native
       trust): `Control.pdelay` — the hand-minimized 11-scope twin of
       seed 12 (root─B(D,3 D kids: two childless + one with SIX R
       kids)─six R leaves, capLevel 1; six is minimal, five
       completes; two D kids complete at ANY chunk size, pinning the
       `dCount = capLevel + 2` boundary role) — with
       `parentTrap_stuck` (the 103-action parent-delaying schedule
       runs to a stuck state under `fullNoD5`, BOTH stall flavors at
       once), `parentTrap_not_deadlockFree :
       ¬ DeadlockFree pdelay fullNoD5` (on a well-formed AND
       schedulable skeleton — inside the target's hypothesis class,
       unlike `pyramid 1`), `pdelay_on_boundary`,
       `d5_rejects_parentTrap` (today's `.full` refuses the schedule
       at its first parent-delaying commit), and
       `pdelay_completes_full` (non-vacuity: `d5` removes schedules,
       not sessions). `d4_rejects_trap` now pins the jam trap refused
       under both `fullNoD5` and `.full`.
     - Proof-side collateral of the guard change (all landed): the
       pillar `walk_uncommitted_choosable` gained the `d5`-mandated
       enumeration head — parent FIRST when every D child is resolved
       and the parent is unsent (choosable in every mode via `d2`),
       with a `D5Free` discharge (`parentDone ∨ some D unresolved`)
       threaded through `wkChoosable_wire_intro`/`_query_intro`/
       `_wire_of_undone` for the remaining cases; `preserve_walkCommit`
       and the two `WalkFire` committed-arm destructures track the
       extra conjunct. No per-child `d5` fired-fact shadow was added
       to `wkLocalOk` — the committed-match mirror is enough for
       preservation, and the cursor invariant item 6 needs is new
       structure anyway (mint it there if required).
     - Executable re-pins (`EventDag.lean`): `drainAdv` is now
       mode-indexed; `runFuzz` asserts the adversary's stalls
       REPRODUCE under `fullNoD5` (≥ 1 across the sweep) AND that
       every schedulable seed drains to terminal under `.full` (a
       stall there is a hard error again); `runAll` asserts the six
       pins complete under the adversary in both modes, and the
       boundary matrix gains a per-capLevel adversarial drain check
       under `.full` (fork #12's residual). `replaySchedule` (under
       `.full`) doubles as the weave⇔`d5` coherence check: the weave's
       compiled actions pass the new guard on every fuzz seed and pin.
     - Rust side (adjudicated to land with the proof, in the
       campaign's own worktree, NOT yet done): extend
       `Trace::assert_valid` with the seventh check per the
       plain-English spelling above, plus its proptest coverage, so
       the proptested local invariants again exactly match the
       ledger set the theorem assumes.
   - *Why the finding UNBLOCKS the endgame.* Under `d5` the
     hole vanishes: every process's performed set is exactly a trace
     prefix (provable by `Reachable` induction — the precise
     "reachable state ↔ schedule position" bridge, now a per-process
     cursor equation against `sentOf`/`recvdOf`). The §6 argmin then
     simplifies below its original design: take the τ-least
     unperformed event `e*`; its E1/E2 predecessors are performed
     (they sit τ-below), so flow conservation puts data (resp. room)
     on its channel — run-ahead receives only widen the window — and
     its owner's cursor sits AT `e*`, so the owning action is enabled
     outright: starving rcv, jammed snd, and choice points (the
     pillar + item-4 mirrors) all close without any blame table; only
     the all-events-performed endgame (closes and finishes, by the §2
     totals) remains. `trace_monotone`, `schedule_e1_pos`, the E2
     positional twin, `schedule_inj`, and `merge_complete`'s totality
     of τ are exactly the facts this consumes.
6. With the amendment landed: the performed-set/cursor invariant, the
   `e*`-argmin progress lemma, then `progress` and `deadlock_free` in
   Statement.lean's (amended) terms; the planned corollary "terminal ⟹
   all channels drained" falls out of `Inv` at terminal. The
   termination theorem's witness is the §5 schedule via
   `replaySchedule`'s compilation, already checked executably. The
   d5 fired-fact/cursor invariant is minted HERE (per-walk: performed
   set = trace prefix), not in `wkLocalOk` — see item 5's resolution
   record for what the committed-match already carries.
