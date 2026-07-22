# T10 scoping audit: capacity monotonicity (MUX-PROGRESS §3.4b)

**Frozen record (2026-07-22 estate audit):** kept deliberately — the scoping audit warranting AUDIT-NOTES A7's resolution route. Claims herein carry the epistemic status they had when written; where later work superseded one, the supersession lives in MUX-PROGRESS §4 or an in-place marker, and this file is otherwise not updated.


Role: T10, the capacity-monotonicity track. This document is the
charter-mandated Phase A deliverable — the classification of every
capacity mention on the progress path, the route decision, and its cost
estimate — committed before any building. The debt being resolved is
AUDIT-NOTES A7 (the standing window.rs / latency-doc Kahn argument,
[derived]-tier, consumed by no theorem of record); the target is
`deadlock_free_wide`: `DeadlockFree` (and the termination transfer) at
every pointwise capacity vector κ ≥ κ₀.

Epistemic key as in PROGRESS.md. Everything below marked [verified] I
re-read in this worktree at the current commit; line numbers are from
`formal/lean/`.

## 0. Verdict up front

**Route (2), the InvPW route, is licensed — and it is cheaper than the
charter anticipated.** The enabledness core is monotone throughout, and
in the strongest possible sense: the progress engine
(`Sched.progress_of_inv`, EndgameE.lean:672) needs **no widening at
all**. Since track G re-typed it over `InvPW` (conservation without the
`chan ≤ cap` half), it applies verbatim to wide states, and its
conclusion — `canStep` at FLOOR guards — lifts to the wide system by
per-arm guard monotonicity. Every EXACT capacity fact in the proof
stack lives on the schedule-construction side (a pure function of the
skeleton, computed once at the floor and reused untouched) or inside
the floor-enabledness certificates the argmin argument produces; none
is evaluated at wide occupancies. The one genuinely new obligation is
the `InvPW`-preservation sweep for the widened transition function,
and the track-F Steps extraction plus a chan-blindness observation
reduce that sweep to assembly. Estimated cost ~1,200–1,500 lines, all
mechanical; well within budget. Route (1) is not needed.

## 1. The capacity surface of the model [verified]

`Skel.cap` (Model.lean:40): `capLevel` for `level` channels, 1
elsewhere. Capacity appears in `apply`'s guards at exactly eight push
sites and nowhere else:

| site | guard | channel family |
|---|---|---|
| `iopenFire` (wire arm), Model.lean:340 | `s.chan c < 1` | wire I rootH |
| `iopenFire` (query arm), :345 | `s.chan c < 1` | asked I (rootH−1) |
| `ropenFire` (wire arm), :362 | `s.chan c < 1` | wire R rootH |
| `ropenFire` (res arm), :366 | `s.chan c < 1` | rootres |
| `ropenFire` (query arm), :371 | `s.chan c < 1` | asked R (rootH−2) |
| `walkFire`, :405 | `s.chan c < 1` | c = `obligChan pk o` (wire/lower/asked/upper — all cap 1) |
| `asmSend`, :438 | `s.chan c < sk.cap c` | level / rootret / rootrets |
| `absorbSend`, :461 | `s.chan c < sk.cap c` | level I 0 |

Receive guards are `s.chan c > 0` and close guards `s.chan c == 0` —
neither mentions capacity. So the widened system `applyW κ` differs
from `apply` at these eight guard literals and nowhere else; every
successor STATE EXPRESSION is identical. Two immediate consequences:

- **Guard monotonicity** (enabled-at-floor ⇒ enabled-wide, same
  successor) holds per arm by `chan < cap c ≤ κ c`, and for the
  hardcoded `< 1` sites by `sk.cap c = 1` (rfl at each concrete
  constructor).
- **κ = κ₀ recovery**: `applyW sk.cap = apply` pointwise; each arm is
  rfl after reducing `sk.cap` at the arm's constructor (`walkFire`
  needs `cases o`; `askedOut` needs its `pk.2 < 2` split). This is the
  charter's negative control and it is definitional, as predicted.

Remark (audit bonus, not a build item): for the `level` family ALONE,
widening is already a corollary of the flagship by skeleton
instantiation — `capLevel` is a `Skel` field, `wellFormed`'s only
capLevel conjunct is `capLevel ≥ 1` (Skel.lean:96), and margin 0 is
monotone in `capLevel` — so `deadlock_free {sk with capLevel := W}`
needs nothing new. The deployed-gap statement, however, is the
pointwise vector including the cap-1 wire families, which no Skel field
reaches; `applyW` is genuinely required, and subsumes the corollary.

## 2. Classification of every capacity mention on the progress path

The progress path is `deadlock_free` → `progress` → `progress_of_inv`
(EndgameE.lean:672–903) → { the decode layer (PendingE), the schedule
edge lemmas (Sched, Weave), the close cascade (`close_cascadeE`) },
with `inv_reachable` (Preserve/*) supplying the invariant and
Termination.lean supplying the run-length bound. Classification:

### MONOTONE (or capacity-blind)

- **`progress_of_inv`'s send arm** (EndgameE.lean:832): case-splits
  `hroom : s.chan c < sk.cap c`.
  - Enabled branch: `hroom` + `PendOkE.fire` gives a FLOOR-enabled
    action. MONOTONE: floor-enabled lifts to wide by guard
    monotonicity. No wide twin of `.fire` is ever needed.
  - Blocked branch: consumes only `¬hroom`, i.e. the floor fact
    `chan ≥ sk.cap c`, feeding omega together with conservation and
    the E2 edge to produce an earlier unperformed receive and a
    τ-minimality contradiction. The contradiction is genuine at ANY
    `InvPW` state — wide states included (a wide state's overfull
    channel satisfies `chan ≥ cap` a fortiori; the disabled-wide ⇒
    floor-fact direction `chan ≥ κ c ≥ cap c` is not even needed,
    because the split is taken at the floor). MONOTONE.
  - Net effect: **`progress_of_inv` holds at wide states as stated**,
    concluding floor `canStep`, which lifts. The wide progress lemma is
    a two-line corollary, not a re-derivation.
- **`progress_of_inv`'s receive arm** (:866): `0 < s.chan c` — guard
  identical in both systems. Capacity-blind.
- **The receive/close guards throughout `close_cascadeE`**
  (EndgameE.lean:216–656): consumes `InvPW` conservation only
  (`hchan0 : sent = recvd → chan = 0`, :262) plus `producerDone` —
  no capacity mention at all. Capacity-blind. Already `InvPW`-typed
  (track G).
- **The decode layer** (PendingE `pend_or_done` lemmas): hypothesis is
  `InvL` — cursor-only, chan-blind (`wkLocalOk`/`asmLocalOk`/
  `topLocalOk` read no occupancy; Lemmas.lean's congr lemmas prove
  it). Capacity-blind at the hypothesis level; see EXACT below for
  their `.fire` field.
- **`InvPW` itself** (Lemmas.lean:271): conservation without the cap
  half — the exact hypothesis shape wide states can satisfy. Track G's
  weakening is the license for this whole route; the argmin argument
  "never consumed `chan ≤ cap`" claim is [verified] by the read above.
- **`rho` and the termination stack** (Termination.lean:689): `rho`
  reads cursors only — `walkRho`/`asmRho`/`iopenRho`/`ropenRho`/
  `absorbRho`/`finRho` never read `s.chan`, and the component lifters
  (`rho_walk_lt`, :705) are explicitly generic over an arbitrary
  channel-field rewrite `f : Chan → Nat`. **The case analysis is
  capacity-blind**, so the termination transfer is automatic once the
  wide steps are dispatched to the floor lemmas (companion trick, §3):
  `rho_decreases`, `asmLevelsOk_preserved`, `terminating`,
  `greedy_run_terminal` all have cheap wide twins. (The charter asked
  this be checked and said so: checked, and it holds.)
- **The Steps extraction** (Mux/Proofs/Steps/*): `InvL` + count deltas
  per arm, allChans-relativized, stated against successor SHAPES
  (`step_fire` takes `hs' : setWalk … = s'`, not the apply equation)
  or against `apply` for guard-identical arms. `SendStep.hcap` is a
  floor fact PRODUCED, never consumed by the delta fields. Reusable
  wholesale for the wide sweep.

### EXACT (equalities and windows — and why none blocks)

- **Sched E2 arithmetic** (`scheduleE_e2`, Sched.lean:737:
  `n < rcvCount (take k) + sk.cap c`; edge predicate Sched.lean:182):
  an exact window, but a property of the FLOOR SCHEDULE OBJECT — a
  pure function of `sk`, state-independent. The wide proof reuses the
  floor schedule untouched; E2 is consumed only in the blocked branch
  together with the floor fact `chan ≥ cap`, where wider κ only makes
  the omega slack larger. Does not transfer, and does not need to.
- **The weave's borrowed-slot / `capLevel + 1` / `capLevel + 2`
  arithmetic** (Weave/Window.lean, TeleE, Emit, MasterE, FinalE …):
  exact throughout — this is the schedule CONSTRUCTION, again
  floor-side and state-independent. Untouched.
- **`PendOkE.fire`** (PendingE.lean:108): `chan < sk.cap c →
  (apply …).isSome` — exact floor enabledness, with `sk.cap f.1 = 1`
  facts hardwired at five decode sites (PendingE.lean:1121–1557).
  Consumed only under the enabled branch's `hroom`, i.e. at floor
  room; the lift to wide happens AFTER, at the `canStep` level.
  No wide re-derivation of the decode layer.
- **Close guards `chan == 0`**: equalities, but identical in both
  systems (closes are not capacity-gated). Neutral.
- **`InvP.flow`'s `chan ≤ sk.cap c` half** (Invariant.lean:254,
  Lemmas.lean:236) and `BaseFacts.slot` (SigmaStarInv.lean:311):
  exact occupancy bounds — DROPPED on this route. Wide-reachable
  states genuinely violate them; `InvPW` is the honest invariant.
- **Sub-margin controls** (`jam.dCount = capLevel + 2`, `parentTrap`):
  exact by design, floor-side pins; out of the wide path entirely (the
  wide theorem keeps the margin-0 hypothesis denominated at the FLOOR
  `capLevel`, which is the strongest honest form — widening `level`
  beyond the floor never re-tightens it).

### The one genuinely new obligation

`InvPW` preservation under `applyW` (23 arms). The existing monoliths
(Preserve/*, ~3,700 lines) prove `InvP → InvP` under `apply` and
cannot be invoked (hypothesis too strong at wide states, guards
differ at push arms). But the sweep reduces to ASSEMBLY over the
track-F Steps extraction via one observation, [verified] against the
definitions:

**Everything in `InvPW` and in the Steps deltas is chan-blind.**
`InvL`, `sentOf`, `recvdOf`, `rho`, `asmLevelsOk` read cursors only;
record-update projection makes the corresponding facts about
`{s with chan := f}` DEFINITIONALLY equal to the facts about `s`.
So for the five push arms, instantiate the Steps lemma at the
companion state whose chan field is the wide successor's
(`step_fire` is shape-based, so this is direct; the opener fires use
`iopen_fire_facts`/`ropen_fire_facts`, currently `private` in
SigmaStarInv.lean — de-privatize, a one-word touch to track F's file,
cheaper than duplicating ~150 lines), and read the wide successor's
conservation off the deltas plus the explicit chan bump. For the
eighteen guard-identical arms, `applyW κ a s = apply a s` per arm
(same match arm, definitional), and the Steps lemmas apply as-is.

## 3. Route decision and plan

**Route (2).** Build items, dependency-ordered:

1. `applyW κ` beside `apply` (never touching it), `runW`, `canStepW`,
   `stuckW`, `ReachableW`, `DeadlockFreeW`; `applyW_cap` (the κ = κ₀
   definitional recovery, negative control); `apply_mono` (guard
   monotonicity, per arm). ~250 lines.
2. The `InvPW` sweep: three small assemblers (quiet/recv/send — the
   send assembler has NO cap obligation, so it covers wire pushes
   too, which `BaseFacts.of_send` deliberately could not), then the
   23-arm dispatcher through Steps + companions. ~500–700 lines.
3. Wide progress + capstone: `invPW_reachableW`, `progressW` (the
   two-line lift), `deadlock_free_wide` over κ : Chan → Nat with
   `hκ : ∀ c, sk.cap c ≤ κ c` — pointwise, per-channel (finer than
   per-family; "widen levels, keep wires at 1, any mix" is an
   instance). ~150 lines.
4. Termination transfer: `rho_decreasesW`, `asmLevelsOkW` twins by
   companion dispatch; `terminatingW` (run length ≤ ρ(init) — ρ is
   chan-blind so ρ_κ = ρ), `maximal_run_terminal_wide`. ~250 lines.
5. Kernel decide anchors: a pinned skeleton completing under a
   widened κ (greedy `drainW` at level×4, wire×2), plus a pinned
   short prefix that `runW` accepts and floor `run` rejects (the
   wide semantics is genuinely wider — two back-to-back wire fires),
   plus `applyW_cap`. ~100 lines.
6. AUDIT-NOTES A7 → RESOLVED-by-theorem in A1's register; MODEL.md
   cross-reference.

**Scope covered**: `.impl` (the shipping encoder), margin-0 — the
flagship's corner, which is the A7 gap. **The d5/schedulable corner is
NOT covered**: Endgame.lean's d5 chain (`pends_sound`,
`close_cascade`, `progress_d5`) still consumes full `InvP` — it was
never re-typed to `InvPW` (only the E-side was, by track G). Wide-d5
needs that re-typing first (mechanical, est. ~300 lines of hypothesis
surgery in Endgame.lean, but it touches the d5 flagship's file);
deferred with this note rather than half-done.

**Estimated cost**: ~1,200–1,500 lines, mechanical assembly
throughout, no new induction anywhere. Honest budget: comfortable.
Route (1) (commutation/deferral, diamond lemmas over 23 arms — est.
several thousand lines and a genuinely new reordering induction) is
the fallback and is not needed.

## 4. Finding along the way (deliverable-2 adjacent, recorded here)

`EMuxInv.flow_wire` (Mux/Elastic.lean:195) is stated UNGUARDED
(`∀ p hh`), and is therefore unsatisfiable past walk (R,0)'s first
wire receive: `recvdOf (wire I 0)` Nat-subtraction-aliases walk
(R,0)'s consumer count (Invariant.lean recvdOf, the `h - 1` arm at
h = 0) while `sentOf (wire I 0)` stays 0 — the exact shape of the
track-F `delivered_eq` bug (MUX-PROGRESS §4, "the landed MuxInv was
UNSATISFIABLE as stated"), reproduced in the elastic twin. The seam
hypothesis `hinv` of `elastic_deadlock_free` is thus undischargeable
on any nontrivial run. The deliverable-2 sweep must allChans-guard
`flow_wire` (the consumer `EMuxInv.invPW` only reads it at
`allChans` members, so the weakening is free) and add the pipe-content
field the deliver arm needs. Recorded here so the statement change is
understood as a REPAIR, not a convenience.
