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

## 5. The chosen route: canonical schedule construction [open]

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

Known design risk [open]: the mutual recursion between the walk-side
and assembly-side wavefronts (walk res-progress at a stage is throttled
to ≈1 unconsumed resolution by the cap-1 lower channels, so descent
past a D scope interleaves with assembly of its predecessor's subtree;
level channels carry `capLevel` slack in the other direction). If the
recursion resists a clean statement, the fallback is stage-compositional
rely-guarantee induction on `rootH` (the Phase B `openStage` shape) —
heavier machinery, kept as reserve.

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

## 7. Remaining work, in order

1. Executable candidate schedule + eventdag validation (§5 workflow).
2. `Proofs/Sched.lean`: the construction + the three edge-respect
   lemma families.
3. Opener/asm enabledness mirrors of the pillar (small).
4. The blame lemmas (§6 table), consuming §2 + Sched.
5. Argmin assembly: `progress`, then `deadlock_free` in
   Statement.lean's terms; the planned corollary "terminal ⟹ all
   channels drained" falls out of `Inv` at terminal.
