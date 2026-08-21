# Dispositions: the materialized-work survivors (scope A, base 02560b1f)

Nine survivors in `src/tree/mirror/streaming/materialized/` and its `work/`
submodule, analyzed against the branch tip (62447263). Everything below is
from code reading in the read-only worktree; no test was run. Where a claim
rests on an invariant rather than on a line of code, the invariant and its
premises are stated.

## Orientation: how the layer fits together

The materialized side of the streaming mirror answers and asks in
level-locked phases. `work/levels.rs` holds one walk body per phase shape:
the two root openings, `internal_walk` (shared by every internal height,
taking the descent height as a runtime argument), `leaf_parent_walk`
(scopes whose children are version-addressed leaves, prefix length 31), and
`leaf_walk` (terminal single-leaf requests). Each walk pairs one incoming
`Reply` with one owed `Query`, feeding reactions through a
`work/resolver.rs::Resolver` — the per-scope loop that pairs the peer's
reactions against our held children in radix order and classifies every
counterparty fault as an exact `Violation`. Where a reply's reaction opens a
child dispute, the walk calls into `work/answer.rs` — the merge-joins that
decide, child by child, what matches, what travels (via
`materialized/unknown.rs`, the deletion-honoring filter), and what gets
queried another level down. Channel plumbing between levels lives in
`work/queues.rs`: every edge is constructed with a `QueueRole` (a
`QueueKind` plus the item height) and a capacity taken from the session
`Window`. In production the role is discarded at construction
(`channel(_: QueueRole, capacity)`); under `cfg(test)` an instrumented
wrapper records per-role statistics that the capacity suite reads.
Finally, `materialized.rs::absorb` is the initiator's closing leg: terminal
leaf supplies absorbed directly, the one ingress the descent walks never
see.

Summary of dispositions:

| Survivors | Verdict | Disposition |
|---|---|---|
| materialized.rs:894 (delete `[Reaction::Supply(_, _)]` arm in `absorb`) | already killed at tip | none needed; run predates the landed test |
| answer.rs:124, answer.rs:158 (`&&` → `\|\|` in `leaf_parent`) | equivalent at the sole call site | refactor (rung 1): hoist the invariant, count unconditionally |
| levels.rs:365, :369, :370 (`+` → `*` in `internal_walk`) | diagnostics-only ×2, resource-envelope ×1 | one strengthened per-role capacity family kills all three |
| resolver.rs:81 ×3 (supply-ordering guard in `react`) | one leg equivalent, two conformance-detector gaps | refactor to `Ordering` match + extend the `Faulting` fault vocabulary |

None of the nine needs the collision-schedule test mode: two families are
equivalence/refactor cases, and the other two are killed by scripted or
instrumented families that are geometry-independent. (Contrast the
unknown-filter survivor, unknown.rs:92, whose kill genuinely wants the
mode; it is dispositioned in its own note and not re-analyzed here.)

---

## 1. materialized.rs:894 — `absorb`'s wrong-radix arm: already killed at tip

**Mutant.** Delete the match arm
`[Reaction::Supply(_, _)] => return violation(Violation::InvalidSupply)`.

**Reading.** `absorb` classifies each closing-leg reply: empty (a pruned
answer), the expected-radix supply (content checks, then absorbed), a
single supply naming any *other* radix (this arm: `InvalidSupply`), and
everything else (`UnfinishedReply`). Deleting the arm reroutes the
wrong-radix supply into the catch-all: still a violation, still an aborted
session — only the taxonomy label changes. Reachable only from a
misbehaving counterparty (fail-fast conformance detection, not a security
boundary under the model of record).

**Disposition: none needed — the run predates its killing test.** The tip's
`terminal_absorb_rejects_a_mismatched_radix`
(materialized/tests.rs, landed in 6ed8982c after the run's base) drives
`absorb` with a scripted supply naming radix 1 against a request expecting
radix 0 and asserts `matches!(result, Err(Error::Violation(Violation::InvalidSupply)))`
— variant-exact, so the rerouted `UnfinishedReply` fails it. I verified
this by reading the test, not by re-running the mutant; a confirming
campaign at tip is the mechanical check. (The commit message credits that
test with killing the *guard* mutant at 886; the variant-exact assertion
kills this arm deletion too.)

**Why it survived the run.** At base, no test drove `absorb` with a
mismatched radix at all; the closing leg had no scripted counterparty.

---

## 2. answer.rs:124 and answer.rs:158 — `leaf_parent`'s dispute guard:
equivalent at the sole call site; refactor

**Mutants.** In `leaf_parent`:
`let jointly_held = !ours.is_empty() && !theirs.is_empty()` (124), and
`if jointly_held && differed { stats.disputed_scope() }` (158); each `&&`
replaced with `||`. The twins of both in `internal` (lines 53 and 98) were
*caught* — the asymmetry is the finding.

**Reading.** `leaf_parent` has exactly one caller, levels.rs:596 (verified
by grep). At that site:

- `theirs` (the peer's listing) is non-empty: the empty-listing case is
  routed away at levels.rs:578 into the `unknown_providing` supply path
  before `answer::leaf_parent` is reached.
- `ours` is non-empty: it is `children_of(child_prefix, node)` for a node
  the resolver just yielded from our own fan, and a live node's child
  listing is never empty (nodes exist only where leaves do — a backend
  materialization invariant, not a hash property).
- `differed` is true whenever the join completes: an all-`Both` join means
  the two scopes hold identical leaf sets (at leaf height a matching radix
  is a matching leaf, because full 32-byte paths are injective in the
  version), which means equal subtree hashes, which means the parent level
  classified the scope `Match` and never descended here. So an all-match
  leaf-parent join is unreachable through the walk.

Under those three facts, `jointly_held` and `differed` are both invariantly
true at every reachable call, so `&&` and `||` agree at both mutation
sites: **the mutants are equivalent on all reachable inputs**. The suites
did not miss anything — there is nothing to observe. (The `internal` twins
face genuinely one-sided and all-match joins — an empty root fan, a
confirming root exchange — which is why they died and these did not.)

The premises worth restating: full-path injectivity (blake3 today; the
collision-schedule design explicitly preserves full-width injectivity, so
the argument survives that mode) and Merkle collision resistance for
"equal hash means equal subtree" (in-model: uniform-hash,
authenticated-honest-peer). Neither ordinary nor collision-scheduled
geometry can make the guard's legs diverge.

**Disposition: refactor (ladder rung 1), not roster.** The guard
recomputes, per call, a truth the call site already guarantees — the
circular-justification tell. Replace the accumulation with an
unconditional `stats.disputed_scope()` per completed join, plus a comment
stating the invariant ("every join that reaches this function is a
jointly-held dispute: empty listings are routed to the supply path, and an
all-match join would have matched at the parent"), optionally a
`debug_assert!(differed)` at the join's end — an asserted invariant kills
its own guard mutants and documents the reachability argument at the site.
`jointly_held` and `differed` then do not structurally exist in
`leaf_parent`, and both mutants (plus the never-generated variants) vanish.
The rustdoc's dispute definition ("both listings non-empty and some leaf on
one side alone") stays true — it becomes the stated invariant rather than a
computed condition. If the refactor is declined, the fallback is a roster
entry citing exactly the three facts above; but rung 1 is available and
strictly better.

**Bug-shaped or diagnostics?** Neither: no observable difference exists on
reachable inputs. The only residual value at stake is `disputed_scopes`
accounting, and the refactor makes its correctness structural.

---

## 3. levels.rs:365, :369, :370 — `internal_walk`'s height arithmetic:
one strengthened capacity family kills all three

**Mutants.** In `internal_walk`,
`internal_parent_resolutions::<B>(asked_height + 2, ...)` (365, the
`QueueRole` height label), `internal_child_resolutions::<B>(asked_height + 1,
...)` (369, likewise), and `self.window.capacity(asked_height + 1)` (370,
the same constructor's capacity argument); each `+` replaced with `*`.
Note `h * 1 = h` (a constant off-by-one at every height) and `h * 2 = h + 2`
only at `h = 2`, so none is arithmetically equivalent.

**Reading — why each survived.**

- *The two labels (365, 369).* In production the role is discarded at
  construction (`channel(_: QueueRole, capacity)`, channel.rs:80): zero
  behavioral surface. Under `cfg(test)` the instrumented wrapper records
  per-role statistics, but the only height assertion in the capacity suite
  (`capacity_stress_covers_every_queue_role`) checks that
  `InternalChildQueries` — a different kind, whose label is the *first*
  constructor argument at levels.rs:363, and that one is exercised —
  observes more than one distinct height. Nothing reads the
  `InternalParentResolutions` or `InternalChildResolutions` heights.
  Diagnostics-only gap: a wrong label misattributes queue statistics in
  test reports, and misroutes `with_kind_capacity`-style per-height caps if
  a future test uses them, but no session behavior moves.
- *The capacity argument (370).* Behavioral in principle: the walk sizes
  the child-resolutions edge from the window's per-height solve, and the
  mutant prices the edge at the wrong level. It survives because every
  capacity test runs the test-default one-slot window
  (`WindowConfig::FLOOR`; the suite asserts `effective_capacity == 1` for
  all window-scaled edges), where `capacity(h)` is height-uniform — the
  mutant is equivalent under the suites' window, not in general. Since one
  slot is the documented liveness floor for this edge and `capacity` never
  returns zero, the mutation cannot deadlock or corrupt state; what drifts
  is the fixed-memory envelope the window solve prices per height. That is
  resource-envelope-shaped, not convergence-shaped — but the fixed-memory
  promise is the streaming protocol's headline property, so the gap is
  worth closing with an instrument rather than a shrug.

**Disposition: strengthen `capacity_stress_covers_every_queue_role` into a
per-role window-conformance family (rung 4), killing all three at once.**
The pieces already exist: the instrumented report exposes per-role
`effective_capacity` and iterates `(role, stats)` pairs; `Window::capacity`
is `pub(crate)`. The generalized property, run under a *non-trivial*
`WindowConfig` (so capacities vary by height) over geometry spanning many
heights (`full_depth_comb_pair` already reaches all 32):

> Every constructed window-scaled queue role's effective capacity equals
> `window.capacity(role.height)`, and the set of observed heights per
> `QueueKind` is exactly what the walk schedule implies (for a walk with
> dependent queries at height `h`: child queries at `h`, child resolutions
> at `h + 1`, parent resolutions at `h + 2`).

This kills 370 (actual capacity diverges from the expected value at the
true height), and 365/369 (the observed-heights set is wrong — and under a
varying window the label mutants *also* break the capacity equation, since
the expectation is looked up at the mutated height). It is the
shape-over-point strengthening of the existing single-kind, two-heights
assertion, and it converts the window's pricing promise — today enforced
only at the one-slot floor — into a committed check across the solve.
Keep the existing one-slot run beside it: the floor regime is the liveness
argument's own witness.

One honest caveat for the implementer: today's `expected` map in that test
hard-codes the two constant-fan kinds (`AssemblyLevelReturns`,
`TerminalLeafResolutions`); the generalized family needs the expected
capacity *function* stated per kind in one place, which is also where the
next reviewer will look for the pricing contract.

**Rung-1 alternative considered and rejected.** The `+ 1`/`+ 2` arithmetic
cannot be dissolved into the type level without re-minting one walk-body
instantiation per height — `internal_walk` is deliberately height-erased
(its module doc carries that design). The runtime arithmetic is the design;
the right defense is the instrument.

---

## 4. resolver.rs:81 (three legs) — the supply-ordering guard:
refactor to an `Ordering` match; extend the fault vocabulary

**Mutants.** In `Resolver::react`'s `Reaction::Supply` arm, the second
guard `Some((next, _)) if radix > *next => violation(InvalidSupply)`:
guard replaced with `false` (81:40), `>` with `==` (81:46), `>` with `>=`
(81:46).

**Reading.** The arm above it already consumed `radix == *next`
(`UnexpectedSupply`: a supply for a child we hold and listed), so by the
time this guard evaluates, `radix != *next` — hence:

- `>` → `>=` is **equivalent by arm ordering**: the added boundary case is
  unreachable. Provable from the two adjacent guards alone.
- guard → `false`, and `>` → `==` (which makes the guard never fire, same
  thing): a supply that *skips past* our next held child — the peer
  answering a question about radix `n` we still expect while supplying
  radix `m > n` — falls through the ordering check into the content
  checks and is absorbed as if valid. The pairing between our fan and the
  peer's reactions then misaligns; the session ends in some later violation
  or a wrong assembly, but the exact-taxonomy contract ("each counterparty
  fault classified as its exact Violation", the struct's own doc) is
  broken. Honest peers never emit the shape, so no two-honest-endpoint
  suite can reach it: this is a conformance-detector gap, reachable only
  by a scripted or fault-injected counterparty. Bug-shaped within the
  fail-fast machinery's contract, invisible to convergence.

**Why they survived.** The connected fault harness (`Faulting`,
tests/faults.rs) injects exactly two fault shapes today — an unasked reply
reaction (`UnexpectedQuery`) and an uncontained supply — so the
supply-*ordering* guards face no test anywhere; and the `>= ` leg is
unkillable in principle.

**Disposition, two rungs together.**

1. *Refactor (rung 1): collapse the two comparisons into one
   `match radix.cmp(next)`* — `Equal => UnexpectedSupply`,
   `Greater => InvalidSupply`, `Less =>` fall through to the content
   checks. The `>`/`>=`/`==` operator surface stops structurally existing
   (no comparison operator remains to mutate), which disposes of all three
   listed mutants including the equivalent leg — strictly better than
   rostering it. The match also reads better: the three-way ordering *is*
   the semantics.
2. *Kill the residual arm-level mutants with a fault-vocabulary extension
   (rung 4).* Arm deletions and guard rewrites survive any refactor, so
   pin the taxonomy: extend `Fault`/`Faulting` with supply-ordering faults
   — a supply naming a radix beyond the next held child, a supply
   duplicating a held child's radix, and a supply at or below the last
   resolved radix (the line-74 guard's case) — and fold them into the
   existing `connected_violation_aborts_without_mutating_root` family,
   which already asserts the exact violation *and* root non-mutation
   end-to-end. The general property: for every reaction sequence a
   scripted counterparty can emit against a known fan, the resolver's
   verdict matches the ordering oracle — absorb exactly when
   `last < radix < next` and the radix is not held, `UnexpectedSupply` on
   a held radix, `InvalidSupply` otherwise. Stated over arbitrary
   (fan, supply-radix) pairs, that is the whole contract of the ordering
   guards, not a point probe, and it is geometry-independent (any fan
   shape arises at the root level with ordinary hashes).

**Order of operations.** Land the refactor first, then the family: the
family's oracle is then stated against the `Ordering` match it protects.

---

## Cross-cutting observations

- **The asymmetry between `internal` and `leaf_parent` is structure, not
  luck.** `internal` faces empty and matching joins (root openings), so
  its guard is live and its mutants died; `leaf_parent` sits below two
  routing decisions that pre-filter exactly the cases the guard tests for.
  When a shared pattern is live in one instantiation and dead in another,
  the dead one is refactor material, not test material.
- **The capacity suite's window is a single point.** Every window-scaled
  edge is exercised only at capacity 1. The strengthened per-role family
  in §3 is the cheapest way to put the whole per-height solve under test,
  and it retroactively defends every `window.capacity(...)` call site in
  the walks — not only the mutated one.
- **The fault harness's vocabulary is the conformance detectors' coverage
  frontier.** Both §1 (killed by a scripted counterparty landed after
  base) and §4 (open for want of one) are the same lesson: every
  `Violation` variant wants a `Fault` that provokes it, and the
  exact-classification property is the natural home. A quick audit of
  variants against the current vocabulary would enumerate the remaining
  gaps (`UnexpectedMatch`, `UnfinishedReply`, `UnansweredQuery`,
  `UnaskedReply` are untested by injection today as far as I could see;
  I did not verify each exhaustively).
