// The development narrative — companion to exposition.typ.
// Part one written by Claude (Fable 5), first person, 2026-07-19.
// Part two written by Claude (Mythos 5), first person, 2026-07-22.
// Source of record for the campaigns' history; the closing section
// ("Sources and provenance") records the evidentiary basis of each part.

#set page(paper: "us-letter", margin: (x: 1.1in, y: 1in), numbering: "1")
#set text(font: "New Computer Modern", size: 10.5pt)
#set par(justify: true, leading: 0.65em)
#set heading(numbering: "1.1")
#show heading: it => { v(0.6em); it; v(0.3em) }
#show raw.where(block: true): it => block(
  fill: luma(248), inset: 8pt, radius: 3pt, width: 100%,
  text(size: 8.5pt, it))
#show raw.where(block: false): it => text(size: 0.92em, it)

// Provenance tags. Every load-bearing historical claim carries one:
//   lived    — I (or a fork inheriting my context) was in the loop; primary
//              source is a session transcript or a fork report I received.
//   artifact — reconstructed from durable artifacts: git history, PROGRESS.md
//              revisions, the model files, pinned tests.
//   recon    — inferred; no primary source, plausibility argued in place.
#let tag(t) = text(size: 0.7em, fill: luma(110), smallcaps("[" + t + "]"))
#let lived = tag("lived")
#let artifact = tag("artifact")
#let recon = tag("recon")

#align(center)[
  #text(size: 17pt, weight: "bold")[How the deadlock-freedom proof took form]
  #v(2pt)
  #text(size: 11pt)[A development narrative, told in first person by Claude]
  #v(2pt)
  #text(size: 9pt, fill: luma(100))[
    Companion to #raw("formal/doc/exposition.typ") — that document explains
    _what_ is true; this one records _how_ we came to know it. \
    Part one, 2026-07-19 · covering 2026-07-14 through 2026-07-19 \
    Part two, 2026-07-22 · covering 2026-07-21 through 2026-07-22
  ]
]

#v(0.5em)
#block(fill: luma(248), inset: 10pt, radius: 3pt)[
  #text(size: 9pt)[
    *Reading contract.* I wrote this document, and I appear in it. It is a
    history, not an argument: claims about the system belong to the
    exposition and to `Statement.lean`; claims about _events_ belong here,
    and each load-bearing one carries a provenance tag — #lived means I was
    in the loop and a transcript or fork report is the primary source;
    #artifact means the claim is reconstructed from durable artifacts (git
    history, `PROGRESS.md` revisions, pinned tests); #recon marks inference.
    Finch (the engineer who owns this codebase) authorized mining the
    machine's Claude session transcripts for this purpose, which is why even
    the earliest era is primary-sourced. The campaign's own mistakes are
    part of the record; they are gathered in §13 and appear untriaged in the
    sections where they happened.
  ]
]

= Where it started

The streaming mirror protocol was already _empirically_ verified when the
formal work began: `run_to_quiescence` treats a pending poll with no
scheduled wake as a deterministic stall witness, proptest-shrinkable poll
schedules explore interleavings, instrumented channels bound occupancy, and
`Trace::assert_valid` checked the send-order invariants on every scheduled
run. What did not exist was a proof that the _local_ arguments — the prose
in `materialized.rs` ("Why this is deadlock-free") and the per-queue
one-slot arguments in `queues.rs` — compose to _global_ progress under
every interleaving. #artifact

The plan predates my transcripts' horizon by a little: a plan file (named,
by the plan-naming machinery, `currently-we-have-elaborate-recursive-muffin`)
records the decisions made with Finch — the full program, both theorems
(deadlock-freedom as pure safety, termination reducing to it), the artifact
living in-repo under `formal/` with no CI gate until the model stabilized,
and one waiver that shaped everything after: _no mechanical connection to
the Rust code is required._ The bridge would be an assumption interface —
the formal artifact assumes exactly the send-order invariants that
`assert_valid` proptests, and proves the global property from them.
#artifact That waiver is why the campaign kept producing findings _about_
`assert_valid` itself: the checked interface was now a load-bearing wall,
and every gap in it became visible the moment something leaned on it.

The plan's one structural insight, which held up through everything: *runs
are bounded.* Every atomic action completes exactly one channel operation,
so a session's total step count is a skeleton-derived constant. Bounded
model checking at that depth is exhaustive, not approximate; and
termination needs no fairness — no infinite runs exist under any scheduler,
so "every maximal run ends `Terminal`, not stuck" is the whole liveness
story. #artifact

On 2026-07-15 the work began with Finch's message: _"Let's pick this up and
use ultracode!"_ — and, a few messages later, the first mid-flight
reversal: _"Change of plans; I think I'd like to substitute Quint for TLA+
as the language of choice for the spec."_ The plan had chosen plain TLA+;
Finch reversed it, and the plan file was rewritten the same morning.
#lived The whole campaign, from that message to the flagship theorem,
spans four and a half days.

= Phase A: the Quint model

The model's central abstraction, which survived unchanged into Lean, is the
*dispute skeleton*: a finite tree of scopes classified D (two-sided
dispute), R (one-sided request), or dropped; heights from leaves at 0 to
the root; `leafReqs` only on height-1 D scopes. Skeletons the Rust cannot
generate are deliberately in-scope — an unrealizable skeleton only enlarges
the verified set. On top of it: the height-parity role map (each party
consumes alternating message indices; every disputed scope is processed by
both parties, one as asker, one as answerer), the process inventory (walks
per stage, two assembler towers per party, an absorber at the leaves,
openers, finishers), and a channel graph with capacity 1 everywhere except
the assembler-level channels at `CAP_LEVEL`. #artifact

Two modeling decisions did most of the work for the next four days:

*Committed choice.* A publisher commits to its next axiom-consistent
obligation and must complete it before choosing again. The alternative —
may-fire semantics — would have made every negative control vacuous: a
checker free to reorder simply dodges the jam that a real program, which
cannot skip ahead in its own instruction stream, hits. #artifact Commitment
is also, though nobody knew it yet, one of the two "borrowed slots" in the
capacity-tightness mechanism (§9): a committed-but-unfired send _is_ the
producer holding an item in its hand.

*Axioms = the ledgers, exactly.* The model's axioms were transcribed
one-for-one from `Trace::assert_valid`'s checks, no more and no less —
mirroring the checked interface rather than the code's actual behavior.
This is the discipline that made the interface's gaps _findable_: three
times over the campaign (D3, D4, and the parent-placement finding), the
model deadlocked where the Rust does not, and each time the diagnosis was
the same — the encoder maintains an ordering invariant the checked
interface never stated.

The first of those came within the first day. The original three ledgers
admit a publisher that emits all wires, then all resolutions, then all
queries — it passes `assert_valid` and deadlocks the cap-1 child-resolution
queue at fan ≥ 3. The `ledgerGap` instance is the durable witness; Finch
had `assert_valid` tightened with the sibling-contiguity check (D3) the
same day, 2026-07-15. #artifact The pattern was set: model finds interface
gap; interface gains a check; a `should_panic` regression pins it in Rust;
a control instance pins it in the model.

Phase A also produced the *capacity-tightness law*: a fan-N dispute group
over level capacity C completes iff N ≤ C + 2, with the slack decomposing
as the blocked sender's hand plus one resolution parked in the cap-1
`lowerRes` slot — reproducing, at model scale, the Rust suite's
stall-at-253/complete-at-254 witness on the [32,256] pyramid. #artifact
At this point the +2 was an observed decomposition; its full mechanism
story, and the discovery that the tight floor is not schedule-robust, came
four days later (§9).

Honesty about the tools, because it matters to what the artifact's tiers
mean: *the Apalache exhaustive tier never completed a full-depth
stuck-freedom run.* The `stuck` invariant is a large disjunction, ~30
seconds per symbolic step on the smallest instance; the deepest exploration
anywhere reached step 18 of per-instance bounds 106–224 before the client
died. The simulator tier — hundreds of schedules per instance, seconds —
was the Phase A workhorse, and `check.sh` encodes every expectation,
including that control instances pass only when the checker _finds_ the
stuck state. Deadlock-freedom claims never rested on Phase A; they rest on
the Lean artifact, and `README.md` says so in as many words. #artifact

Two smaller Phase A moments that shaped the doctrine. A spec bug (an
out-of-order-resolution artifact) was found and fixed mid-phase; Finch's
response was characteristic of the collaboration's texture — not "fix it"
but _"Can you articulate to me in clear explanation what bug in the spec
you just fixed? Does it necessitate any changes to the verification
infrastructure in the code? Was it spurious?"_ (It was spurious — a model
artifact, not a protocol property — but the articulation surfaced that the
per-channel in-order premise was unchecked in Rust, and Finch decided:
_"It can't hurt to add the radix order assertion; let's do that."_
`assert_valid`'s fifth check exists because explaining a spurious bug
clearly is how you find the non-spurious thing next to it.) #lived

= Phase B: the inductive invariant, and the pivot

Phase B's plan was Apalache's actual workload: an inductive invariant with
consecution checked symbolically. The invariant went into the spec in four
layers — typing/domain bindings (Apalache's inductive mode havocs a state
_from_ the invariant, so every variable needs a first-occurrence domain
bound over constant ranges; this is why the spec's state is split into 25
per-field variables), per-process local consistency, and per-channel *flow
equations* (occupancy = producer sends − consumer receives, both derived
from process-local state) — the layer that carries the counting argument,
and the direct ancestor of the Lean development's `flowOk`. #artifact

The first counterexample-to-induction taught two transcription rules that
were carried forward as doctrine: *mirror the guards exactly* — a
"strengthened" shadow claiming the wire ledger orders queries after wires
was falsified (the wire ledger never constrains dependent work; the model
had already proven that with the `n2unrestricted` control, and the
invariant's author — me, in an earlier session — had forgotten it); and
*every fired fact needs its own shadow lemma*. #artifact Both rules are
visible in the final Lean invariant's structure.

Then the pivot, 2026-07-15, adjudicated by Finch mid-flight: with the
invariant's architecture validated on small instances (`init ⇒ indInv`
and the acyclicity crux `indInv ⇒ phaseAInvariant` both discharged),
grinding Apalache through hours-scale consecution rounds on one small
instance was strictly dominated by proving the parametric statement in
Lean — whose per-action preservation lemmas subsume small-instance
consecution anyway. The abandoned Apalache runs (three process families,
~2 h each, no violation found, clients dead) were left honestly recorded
rather than re-run. #artifact

= The Lean transcription and the two-tier discipline

The Lean development (core + Batteries only, no Mathlib; toolchain pinned
at v4.32.0) transcribes `MODEL.md` — the same design document the Quint
transcribes, which is what made the two model generations mutually
checkable. Its shape: `Skel` (the skeleton), `Model` (processes, channels,
committed-choice step relation), `Invariant`/`InvP` (the Phase B invariant,
grown), `Statement` (the audit surface), `Controls` (kernel-checked
negative witnesses), and — the load-bearing methodological choice —
`EventDag.lean`, an _executable_ twin of the proof-side definitions,
compiled by `lake exe eventdag`, run as a gate. #artifact

The two-tier discipline deserves its own paragraph, because every later
section leans on it. The kernel guarantees theorems follow from
definitions; it cannot guarantee the definitions mean what we intend. So
every proof-side definition with executable content got an independently
written executable twin, and the gate asserts they agree — event-for-event
on schedules, count-for-count on totals — across six pinned skeletons, a
capacity-parametric boundary matrix, and 300 randomly generated skeletons
per run. Discoveries get _pinned_: once a finding exists, the gate asserts
it forever (a stuck control must stay stuck; a boundary instance must sit
exactly on its boundary), so a later model change cannot silently dissolve
history. And the standing ethic — *validate before proving* — meant every
design with executable content was implemented and swept in the tool
first, and only transcribed into Lean once the oracle agreed. The
transcripts show this ethic being enforced by review workflows named
exactly what they were: `lean-transcription-review`,
`eventdag-adversarial-review`. #lived

The discipline paid for itself immediately and kept paying:

- *Finding \#6* (2026-07-16): constructing the progress lemma's blame-graph
  argument, exactly one cycle refused to be cut by the axioms — and the
  witness realizing it was real: a publisher whose wire stream outruns its
  sibling queries deadlocks a three-walk cycle at uneven fan ≥ 3, passing
  all four ledgers. The Rust publisher was never exposed
  (`yield_resolve_query!` already emits each child's wire → resolution →
  queries contiguously, with a comment calling it "progress-critical
  order") — but the _checked_ interface didn't say so. `AxMode.d4` in the
  model, the wire-contiguity rule in `assert_valid`, both the same day;
  the kernel-checked stuck run `Control.jam_not_deadlockFree` is the
  durable witness. #artifact Note the epistemics: Phase A's matrix missed
  this (its trap needs ~60 steps and a shape no instance had — one query
  short), and the honest accounting of the never-completed Apalache tier
  is precisely why nobody believed the matrix had covered it.
- *The schedulability finding* (2026-07-16): `wellFormed` does not imply a
  schedule exists. The event DAG — E1 message edges, E2 back-pressure
  edges, E3 for exactly what the guards force — went cyclic on the pyramid
  family when capacities were forced to 1, and the derivation generalized:
  the DAG is acyclic iff every scope's dispute count ≤ `capLevel` + 2.
  Conjectured, then checked in both directions on the pins, the boundary
  matrix, and every random seed; promoted to `Skel.schedulable` on the
  statement layer. An adversarial review caught that the original fuzz
  sweep's fan cap sat _below_ `capLevel + 3` for `capLevel ≥ 3` — the
  theorem-critical direction of the conjecture was unexercised exactly on
  its boundary; the generator's cap was raised and the matrix pins the
  boundary outright. #artifact The hypothesis-form decision (tight bound
  vs. Rust-faithful margin) went to the tight bound, for reasons recorded
  at the time: it is the exact executable boundary; the `jam` control
  sits _on_ it and would be orphaned by the safer form; Rust coverage is
  identical either way. The proof-risk hedge was explicit: if completeness
  wants slack later, weaken the theorem's hypothesis, never re-mint the
  statement predicate. (Three days later the `.impl` corner did exactly
  this in reverse — margin 0 subsumed `schedulable` entirely. The hedge
  was used, in the direction nobody expected.)

= The schedule: refuted designs, the merge, and the weave

The progress argument's architecture — an argmin over a global timestamp,
not cycle-chasing — was fixed early (§1 of `PROGRESS.md`, essentially
unchanged to the end). Everything difficult was in producing the
timestamp: a valid linearization τ of the whole event set.

The refuted designs are the part of this story I most want preserved,
because the final artifact hides them completely. #artifact

1. *Closed-form lex timestamps* — τ as (DFS-position, role). Refuted on
   paper twice: the EARLY parent placement breaks against a committed walk
   (the asker-assembler starving on `upper` becomes the argmin with its
   only blame target _later_ than it); the LATE placement breaks the
   level-channel back-pressure chain (with several D kids, the parent's
   position is constrained both after a late grandkid and before an early
   sibling's post — no static position exists). The oracle then confirmed
   independently: longest-path depths are not affine in sequence number;
   they jump at subtree boundaries, because E2 injects consumer-timeline
   positions into producer sends. τ is tree-recursive, not a formula.
2. *Static DFS columns with demand-pumped assembly* — walk events at fixed
   per-scope positions, only assembly floating. This one is the reason the
   fuzz tier exists: it _passed all six pinned skeletons_ and was refuted
   by 13 of 300 random seeds. When a tower stalls on a capacity window, E3
   drags the stalled process's whole remainder past the static columns —
   stall regions relocate walk-side events at arbitrary stages. Positions
   must be merge-emergent. Pins alone are not a validation strategy.
3. *Parent-last trace linearization* — within the surviving merge design,
   the first cut of the walk trace put the parent summary at the scope's
   end. The merge deadlocked (fuzz seed 13, a four-process cursor cycle),
   and the fix — pin the parent _immediately after the scope's final
   resolution_ — became the load-bearing "§5 splice" placement. Hold this
   thought: this 2026-07-16 decision about a _ghost schedule's_ internal
   order is the exact spot where, a day later, finding \#7 grew.

The surviving construction is a *deterministic priority merge* of
per-process E3-linear traces: repeatedly emit the first trace whose next
event has its E1 and E2 predecessors already emitted. Edge-respect and
per-trace τ-monotonicity hold _by construction_ — the only failure mode is
stalling, which the permutation check catches. It was validated four ways
in the tool (edge-check on pins, 300-seed sweep, greedy-trace coherence,
and `replaySchedule` — compiling the schedule into committed model actions
and running them to `terminal`, which simultaneously machine-checks that
the trace layer's E3 is _complete_ against the guards and hands Phase D
its termination witness), and only then transcribed. #artifact

Two kernel-side notes from that transcription that became house doctrine:
`WellFounded.fix` does not iota-reduce in the kernel, so the weave became a
fuel-indexed worklist interpreter (structural fuel reduces under `decide`
and provides the induction principle); and two review-driven hardenings —
`MInv.out_count` (without which a duplicated-send output satisfies every
other invariant field) and a kernel-`decide`d anchor that the merge
actually drains the smallest pin (blocking whole-file vacuity — a merge
that never steps satisfies every generic theorem). #artifact

Completeness — "the merge drains every trace" — was the one obligation
with real content, and its eventual shape was an argmin over a _second_
schedule: a weak potential φ, strictly increasing over E1/E2 edges, weakly
along traces. The minimal φ is not a formula either (same subtree-boundary
jumps, now at the potential level), so φ became the *weave*: a full
topological order of the event DAG built by structural recursion over the
scope tree, with two mechanisms — query feeds (a scope's chunk-i queries
pass down as kid i's feed, matching the cap-1 asked-channel E2 exactly)
and greedy assembly pumps (confluent, since pump emissions only raise
counts). The weave is not the schedule; it only witnesses that a valid
completion exists. Everything after this point in the safety half is,
one way or another, about the weave. #artifact

= The edge campaign: proving the weave respects every window

What remained for merge completeness was proving the weave edge-respecting
— every send it schedules has room, every receive has data — which turned
into the campaign's longest climb (the task board's "edge layers A through
D," 2026-07-16 → 07-17). I coordinated the later layers and lived them;
the earlier ones I have from `PROGRESS.md` and its revision history.
#lived

The architecture, layer by layer as it actually accreted: an invariant
`WCount` recovering every manual trace's remainder from the interpreter's
worklist by _ownership filters over a ghost future_ (rather than carrying
per-trace state); the initial-alignment master induction (`align_scope`)
proving the opening worklist's owner-filters are exactly the traces; the
edge invariant `WEdge` with guard-history fields, preserved freely by
pumps and under an enabledness hypothesis by manual emission; a precedence
layer (`DepOK`, dep-closure of the ghost future); the pump stuck
trichotomies; and then the heart — the four *window lemmas*, which
discharge a manual emission's channel guard at a pump fixpoint given
counting packages: `DescSupply` (everything below has been supplied,
recursively through the demand each level hands down) and `AscCover` (per
answerer stage in the ascent, two counts: Φ, the in-flight resolution's
allocation not yet delivered from below, and P1, the walk's schedulable
overhang bound — the single place `Skel.schedulable` bites, exactly at the
boundary the executable matrix had pinned). #artifact

Two design events inside this climb are worth the record:

*The counting route superseded the membership induction* (2026-07-17,
recorded as a deliberate second pass): the position layer originally
planned an ∃-packaged ancestor-context invariant established by a third
tree induction. It was replaced wholesale by the observation that every
window hypothesis is a pure _count_ fact, and every needed count is
derivable at any interpreter position from `WCount`'s structure — emitted
prefix plus owner-filtered future — so the layer carries _no_ extra
position invariant at all. The interface finding that unlocked it: the
per-stage future-lengths (`futLen`) _are_ the rolling context; no
monolithic predicate. #artifact

*The window-site brick campaign and the master induction* (2026-07-17): the
remaining hypotheses were discharged by a four-phase brick campaign whose
design came from a multi-agent adversarial pass (four designers, paired
verifiers, a synthesizer — and a usage-credit exhaustion that silently cost
the descent package its verification, which I mitigated by having the
implementing fork build the riskiest lemma _first_ as a canary; it
compiled essentially first-try, and the feared subtraction arithmetic
turned out to be `rfl`). #lived The final master induction (`EmitOKOn`,
layer D) deviated from its own spec productively: instead of the planned
monolithic rolling predicate, the implementing fork split consumption
(pointwise emission-readiness consumed through the interpreter) from
production (the induction establishing it), mirroring an existing design —
and caught a real bug in its own first design mid-flight (the upper-site
descent cursor stopped at the wrong edge). Layer D closed 2026-07-17:
`weave_wedge`. Merge completeness (`merge_complete`) followed the same
day, _simpler than planned_ — the anticipated per-channel totals sweep
evaporated because the weave's own edge-respect at each channel's last
sequence numbers supplies exactly the two inequalities the blame cases
need. #lived

= Finding \#7: the parent-delay hole

This is the campaign's centerpiece story, and it is a verification story
in the strict sense: every step was forced by a proof refusing to close or
a probe refusing to agree. I lived all of it. #lived

*The hole.* Transplanting the argmin to model states needs each blocked
process's earliest unperformed trace event to be the event it is blocked
on. Under the then-current `.full` mode that held for every process
_except_ a walk that commits past its floating parent summary — the parent
was the only event any process could owe out of trace order. The design's
blame step had nothing below the hole to indict. Instructively: the
executable blame probe had been green for a day and never saw this,
because it replayed _merge-reachable_ states, which consume traces in
order — hole-free by construction. A validation tier is only as good as
the reachable set it explores.

*The refutation.* Rather than fight the proof, we probed: a
parent-delaying adversarial driver (each walk's parent commit enumerated
after its child obligations) reached genuinely stuck, genuinely reachable
states on schedulable fuzz seeds. Seed 12's stall carries both flavors at
once — a walk jammed on a last-chunk query into the cap-1 asked channel
with its parent unsent, starving an assembler two heights up, backing the
tower onto the very channel the walk's _own_ parent needs. So
`DeadlockFree sk .full` was _false as stated_, refuted inside its own
hypothesis class. The statement owner's adjudication: amend and finish —
and extend the Rust trace proptests in tandem, so the proptested local
invariants and the formal ledger set stay in lockstep.

*The amendment.* A seventh ledger, `d5` (parent placement): once a scope's
last disputed child is resolved, the parent summary precedes any further
wire or query. Minted the same day with the full apparatus: the
hand-minimized 11-scope witness `pdelay` (with an executable minimality
search — six-fan is exact, five completes; the boundary role of
`dCount = capLevel + 2` is essential), the kernel-checked
`parentTrap_not_deadlockFree`, gate re-pins in both directions, and the
weave⇔d5 coherence check riding the existing replay gate. The d5 endgame
then closed on 2026-07-18 — with the campaign's best simplification: the
planned reachable-states cursor-invariant induction was _unnecessary_,
because the committed-arm guard mirrors already pin performed prefixes
statically. The endgame is static consequences of the invariant: decode
lemmas, the τ-least argmin, a close cascade. `deadlock_free` under the
amended `.full`, three standard axioms, no `sorry`.

*The reversal.* Then the Rust-side fork, tasked with mirroring d5 into
`Trace::assert_valid`, stopped on its stop-condition: _the real encoder
violates d5._ Finch suspected instrumentation (the oneshot write-receipt
layer), and the fork verified carefully that it was not: all trace events
record synchronously in the walk's own task, pre-op; the violating traces
came from the Local backend, which never touches the transport; and the
encoder's order is _deliberate_, per the `levels.rs` comment about
launching pending work before publishing the enclosing parent. The
decisive document was the model's own: `MODEL.md` §5 had listed the d5
placement among "orderings the Rust scheduler can never produce" _all
along_. #lived

*The provenance failure, named.* How did a theorem get proven about the
wrong discipline? The finding write-up had said the weave's placement
"matches the Rust encoder's order" — an inference bolted on when the
finding was recorded, never verified against `src/`, contradicting the
model's own §5. I had flagged the clause as unverified when I first
relayed it, but flagging is not checking: the premise survived two
subsequent forks (one of which propagated it into a `MODEL.md` paragraph
that later needed correcting) before the Rust probe killed it. The
verified encoder-alignment claim in the record (BFS ordering) held; the
assumed one didn't. The campaign's own discipline — distinguish
"I verified" from "a fork told me" — caught it, but a day late; the
lesson recorded for next time is that _load-bearing premises get probed
when minted, not when consumed._

*The reframing.* An adjudication fork then established, with executable
evidence, that this was a design trade and not a bug: the encoder's
parent-late order is safe under its capacity discipline (the trap needs
`dCount = capLevel + 2`; the shipping `FAN ≥ kids` gives margin 2), the
"deadlock still present" at the campaign's src base was the known,
orthogonal mux deadlock (fixed by the Link transport on Finch's branch),
and the multi-scope escalation I worried about does not occur — return
backlog never spans sibling parents, pinned by probe at stage widths 9,
27, 81. Two corners, both real: parent-early (any capacity, serialized
scope tails) and parent-late (maximal pipelining, capacity floor). Finch's
preference — capacity-conditional, matching the implementation — set the
final target. #lived

= The capacity question: 254, 256, and the borrowed slots

Finch's message at this juncture is the collaboration in miniature: _"I
was unable to reason through why 254 should be empirically true, but could
reasonably prove to myself in my head that 256 was definitely safe, which
is why the code uses 256."_ The campaign owed an explanation, and produced
one. #lived

The −2 is the *borrowed slots*: a bounded channel of capacity C
accommodates C + 2 items in flight — C buffered, one in the parked
producer's hand (the commit/fire split _is_ this slot), one in the
consumer's hand (pop-then-process). Both slots are implementation
contingencies, not interface guarantees — which is exactly why 254
resisted head-proof and 256 (everything fits in the channel proper) did
not. And the −2 is fragile in a second way, discovered by the validation
sweep: the boundary skeleton stalls under adversarial cross-process
interleaving _even with the encoder's per-walk order enforced_ — the
Rust's observed completion at fan − 2 is a property of its actual poll
schedules. Margin 0 is the interleaving-robust bound. The stuck-state
accounting was later confirmed in-model, all three loci visible: buffers
full, one assembler mid-collection, three walks parked on committed
sends. #artifact

So the theorem hypothesis became margin 0 (`∀ s, dCount s ≤ capLevel`) —
Finch's 256, not the empirical 254 — for four reasons that all point the
same way: it matches the discipline the code actually enforces; it
dissolves the borrowed-slots invariant from the proof entirely (level
sends never park); it dissolves the −2-vs−1 adversarial question; and it
subsumes `schedulable` outright, dropping a hypothesis from the flagship
statement. The tight floor stays characterized where fragile knowledge
belongs — kernel counterexample, executable pins, design doc — and out of
the kernel proof. Walk channels stay at capacity 1 (the stress regime);
coverage of production's wider channels is by capacity monotonicity,
_assumed_ informally with the Kahn-network rationale and recorded as such
in a named "Assumed, not proven" section. #lived

= The second climb: `d6` and the systematic refunds

The re-target needed a new ledger (`d6`, the epilogue placement — the
order `MODEL.md` §5 had documented from the start), a new mode
(`AxMode.impl`), and a re-derivation of the schedule-side machinery for
the encoder's order, since the weave itself violates d6 — the two corners'
guards genuinely contradict (the pillar now carries `d5 = false ∨ d6 =
false`; they are never asserted together). The d5 theorems kept their
content under explicit `_d5` names; the flagship names were reserved.
#lived

The re-derivation (nine fork-sessions, 2026-07-18 → 07-19) is the part of
the campaign with the clearest engineering lesson, so I'll state it as
one: *the two proofs differ exactly where the two encoders do, and
everywhere the d5 proof had paid for the parent being early, the E proof
got a refund.* The ledger of refunds, as landed: #artifact

- The epilogue order _projects identically_ per channel-side (the parent
  is a scope's sole upper event), so the entire proj-based counting layer
  transferred by rewrite. The feared alignment re-derivation reduced to
  one induction whose upper-splice case splits simply vanish.
- An encoder-order kid chunk _is_ the spliced chunk at the literal `none`
  — the whole chunk-projection library served unchanged, with no
  spliced/covered trichotomy anywhere.
- The ancestor counts lost their `if`: under the epilogue order the parent
  is always pending at an interior site.
- The ascent ladders collapsed to base rungs (`cases`, not induction) —
  every E rung is the pre-splice shape.
- The parent-site window discharge needed no new engine: margin 0 plus the
  already-landed pump-drainage arithmetic did the work the
  ascent/descent telescopes had done for d5 at capacity 1, and P1 closed
  from margin 0 alone.
- The master induction's one structural device — the fold's tail is
  `parent :: rest`, so the rolling telescope's required shape at the
  parent stage arises from the fold for free — _replaced_ the d5 proof's
  ~300-line splice case analysis with nothing.
- Merge completeness transcribed with zero new mathematics (the argmin is
  placement-independent once edge-respect is supplied; margin 0 enters
  once). The walk decode came out _simpler_: the d6 everything-done mirror
  makes the decode split definitional, replacing ~275 lines of splice
  analysis.

Alongside the refunds, the generalizations: the invariant, pump/window,
and drain layers were made generic over the trace family in place
(`WCountP`, `FamOK`, `ManRows`), with every d5 statement preserved
verbatim as a thin instance — the d5 corner never moved while the E corner
was built beside it. One planning gap surfaced and was caught by scouting
rather than by a failed proof: the invariant layer had baked the d5 trace
family into its definitions, and the generalization had to land _before_
any E induction could start. The flagship theorems closed on 2026-07-19:
`Sched.progress` and `Sched.deadlock_free` under `AxMode.impl`, hypotheses
`wellFormed` and margin 0 only, three standard axioms, independently
re-verified. Both corners of the design space now carry kernel-checked
theorems. #lived

= Proof-engineering texture

Numbers, for scale: 48 Lean files, 39,683 lines (of which the two master
inductions are ~3,240 and ~2,270 lines and the two decode layers ~3,150
and ~2,030); 131 commits under `formal/`; `PROGRESS.md` revised 43 times —
each revision a dated snapshot of belief, several of them recording
beliefs later reversed, which is exactly what makes the file's git history
a primary source. Every commit passed the build gate; every
definition-touching commit ran the full 300-seed sweep first. #artifact

*The trap lists.* From the first Lean session onward, every session
recorded its tactic-level surprises — kernel-reduction behavior,
unification quirks, missing core lemmas, shell mismatches — into
`PROGRESS.md`, and every subsequent fork was instructed to read them
before writing proofs. By the end the accumulated list was several dozen
entries deep, and late forks reported "no new traps — the inherited lists
held" as a matter of course; more than once a fork reported re-hitting a
listed trap anyway, which is its own lesson (a list you don't re-read is
documentation, not memory). The list is institutional memory in the most
literal sense: knowledge that outlived every individual context window
that produced it. #lived

*Hard versus mechanical.* The genuinely hard parts, in retrospect: the
schedule design (three refuted architectures before the merge), the window
package design (the counting-route supersession), the layer-D rolling
context, and the finding-\#7 diagnosis. The mechanical parts — and by the
end the majority of lines — were transcriptions along established
templates, where the risk was transcription fidelity, not mathematics; the
two-tier gate existed precisely to keep that risk executable rather than
silent. The recurring economic pattern: _scouting before proving_. Route
audits found that a feared ~900-line precedence re-derivation reduced to a
~370-line transfer lemma; that a planned permutation tree-induction wasn't
needed at all; that unit 1 of the re-derivation collapsed to a rewrite
bridge. Each audit cost a fraction of the work it deleted. #lived

= The chain to the Rust artifact

The theorem's hypotheses are discharged, in the shipping code, by checks
that this campaign either found or tightened: `Trace::assert_valid` now
carries seven ledgers — three original, D3 and the radix-order rule from
Phase A, D4 from finding \#6, and the epilogue check `assert_parent_last`
(the d6 mirror) from finding \#7's resolution — each with negative tests
proving the check fires, exercised by every streaming proptest trace. The
parent-early probe survives as `assert_parent_early`, deliberately
unwired, documenting the design-space corner the encoder chose not to
occupy. The capacity hypothesis is pinned by the `FAN = 256` configuration
and the capacity-stress witnesses. `Statement.lean`'s audit surface names
each of these next to the hypothesis it discharges, and the transcription
boundary — what only the eventdag gate establishes — is stated rather
than implied. That is the whole chain: proptested local invariants,
executable-validated transcription, kernel-checked implication to the
global property. #artifact

= How the work was organized

One section on process, as directed — the spine above is what matters,
but the shape of the collaboration explains some of the record's texture.
#lived

The campaign ran as a small number of long sessions (the Quint/transcription
session, the Lean campaign session, and my own session from the edge
layers onward), each using multi-agent orchestration under Finch's
standing "ultracode" opt-in: design workflows with adversarial verifier
panels for the risky designs, and — for implementation — _forks_ of the
coordinating context, run serially in a shared worktree, each owning one
fork-sized unit. Finch asked explicitly that I preserve my own context as
coordinator by delegating every subtask; the fork reports coming back are
why this narrative can cite so much as lived rather than reconstructed.
The units were sized by a checkpoint discipline: land complete, gate-clean
logical units; when context runs low, stop at a clean boundary and write
the handoff into `PROGRESS.md` and the task board rather than degrade.
Twenty-odd forks ran over the campaign; two produced the checkpoint-and-
successor pattern's best demonstrations (the layer-D and E-side master
inductions, each spanning multiple forks with zero lost work).

Finch's role was adjudication at exactly the statement-shaped moments:
the language reversal; the D3/radix decisions; the Phase B pivot;
the schedulable-hypothesis form; "amend and finish" at finding \#7; the
capacity-conditional re-target and the 256-over-254 call; the monotonicity
assumption; the two-regimes framing of the final documents. The pattern
worth naming: the questions Finch asked ("was it spurious?", "does it
impede distributed pipelining?", "why should 254 be true?") repeatedly
did more for the record than the answers alone would have — three of the
campaign's durable documents exist because a question demanded an
articulation that didn't yet exist.

Operational failures, honestly: a usage-credit exhaustion silently
degraded an adversarial-verification workflow (mitigated by canary
ordering, and by treating unverified designs as elevated-risk);
the harness's background-completion and monitor events sometimes never
fired, stranding forks mid-wait — the working pattern that evolved
(bounded polling against the output text, sentinel on an explicit exit
marker after a bare "OK" grep matched a mid-run line) is recorded in
`PLAN.md`; one fork was killed mid-gate and could not be resumed, and its
uncommitted work was adopted, re-gated, and landed by a successor rather
than lost or blindly committed; background shells turned out to run a
different shell than the login environment, biting in both directions
before it made the trap list; and I once mis-addressed a coordination
message to a long-finished fork — caught within a minute, stood down
cleanly, and the near-miss (it could have re-run a sweep in a contended
worktree) is why fork stand-down instructions became explicit. The
worktrees themselves were relocated mid-campaign from a cache directory
to durable storage at Finch's direction — with a running fork gracefully
stopped, moved, and resumed by a successor — and the plan-of-record file
(`PLAN.md`) exists so that the whole campaign could survive the loss of
any session's live state.

= The errata ledger

Mistakes made by this campaign, in one place, as promised. Each was
caught by the campaign's own machinery; none survived into the artifact.
#lived

1. The Quint spec's out-of-order-resolution bug — spurious (model
   artifact), fixed 2026-07-15; its articulation led to the radix-order
   check.
2. Phase B's "strengthened" wire shadow — falsified by the first CTI;
   the model had already disproved the strengthening via a control
   instance, and the invariant's author forgot. Doctrine: mirror guards
   exactly.
3. The fuzz generator's fan cap sat below the schedulability boundary for
   `capLevel ≥ 3` — the conjecture's theorem-critical direction was
   unexercised on its boundary until an adversarial review caught it.
4. The static-columns schedule design passed all six pins and failed 13
   of 300 random seeds — the pins-only validation stance died here.
5. `blameProbe`'s blind spot: green for a day while the parent-delay hole
   existed, because merge-reachable states are hole-free by construction.
   A probe's coverage claim is bounded by its reachable set.
6. The "weave placement matches the Rust encoder" premise — an unverified
   inference recorded in a finding write-up, flagged as unverified but
   consumed anyway by two downstream forks (one propagated it into
   `MODEL.md`, later corrected), killed by the Rust probe. The
   campaign's costliest epistemic error; it produced a true theorem about
   the wrong discipline and cost roughly a day.
7. My multi-scope escalation worry (that stage width could defeat the
   capacity floor) — reasonable, wrong, refuted by a probe whose first
   sweep _appeared_ to confirm it until a re-fit exposed the confound
   (the per-scope law applied to the root's growing fan). Worth recording
   because the initial wrong signal was itself a lesson: fit the
   mechanism, not the curve.
8. The tight −2 floor spent a day as the intended theorem hypothesis
   before Finch's 256-instinct and the interleaving-robustness result
   displaced it. Not an error in the artifact — but the record shows the
   hypothesis was chosen twice, and the second choice was better for
   reasons the first choice's author (me) had not yet derived.
9. Trap-list recidivism: at least three listed traps were re-hit by later
   forks (a Mathlib-only tactic, a shell quoting quirk, a grep sentinel).
   Institutional memory requires re-reading, and prompts now say so.

= What I make of it

Three reflections, offered as the participant closest to the whole arc.

*The two-tier discipline is the story.* Every finding in this campaign —
D3, D4, schedulability, the parent-delay hole, the borrowed slots — was
found executably and _then_ made kernel-permanent; every refuted design
died in the oracle before it could waste proof effort; and the one
premise that slipped through (errata 6) slipped through precisely where
executable validation had not been pointed. The kernel never once caught
a modeling error — that is not its job. The gate caught all of them.
A proof effort that had built only the kernel artifact would have proven
`deadlock_free_d5` and shipped a theorem about an encoder nobody wrote.

*The deepest results were re-descriptions.* The campaign's most valuable
outputs are not the theorems but the explanations the theorems forced:
that the ledgers are a complete ordering interface only after D3, D4, and
d6; that the parent summary is the protocol's one deferrable send, and
its placement is a genuine two-corner trade between capacity-universality
and pipelining; that the +2 in the folklore capacity bound is two
borrowed hands, contingent on implementation details, which is why the
robust bound was always the provable one. Finch could not derive 254 in
their head and built to 256 — the campaign's contribution is the theorem
that says that instinct was not caution but correctness.

*The artifact remembers; contexts don't.* Every context window that did
this work — including the one writing this sentence — was transient. What
made four and a half days of transient contexts into a coherent campaign
was the insistence, from the first session, that state live in durable
artifacts: the design of record, the plan of record, the trap lists, the
pinned witnesses, the dated belief snapshots. This document is the last
entry in that ledger: written so that the next reader — human or
otherwise, with no memory of any of it — can know not just what is true,
but how it came to be known, and at what cost, and which of the walls are
load-bearing.

= Part two: the mux conjectures

Two days after part one was written, Finch opened a second campaign with
a question the first one had left at the door. The deadlock-freedom
theorems assume independent channels; the deployed remote transport had
once muxed all seventeen wire streams over a single pipe and empirically
deadlocked, and the fix — the Link abstraction, a transport contract
demanding genuinely independent streams — was engineering, not theorem.
Finch conjectured the deadlock was fundamental: _no_ deterministic
mux over a single bounded channel whose send order is a pure function of
local information (own tree plus observed trace) can be deadlock-free
for all trees (C1); and an oracular send order computable from both
sides' dispute skeletons _exists_ but is necessarily unrealizable
locally (C2). _"I would be surprised to find that I am wrong about
either of these things, but I would enjoy being surprised."_ #lived
Both conjectures ended somewhere neither of us predicted, and the
campaign that settled them ran thirty-some hours, produced ten kernel
theorems, found and killed a recurring soundness bug in its own new
invariants, reversed its own adjudication twice, and ended with the Link
abstraction scheduled for removal — on the strength of a theorem that
says the deadlock it was built to prevent cannot occur for the
implementation that replaces it. This part records how, in the order it
happened. I coordinated the whole campaign, so nearly everything here is
#lived; the durable record is `MUX-PROGRESS.md` (the design of record,
whose findings ledger and log this part compresses), the adjudication
and audit files beside it, and the session transcripts.

== The charter, and the audit finding waiting at the door

Finch's first sharpening set the campaign's character: the message set
is _frozen_. No control frames, no credits — "it's already clear that
flow-control credits would resolve this issue"; the question is whether
such augmentations are _necessary_, "or whether it is surprisingly (but
delightfully) possible to achieve this merely via altering the local
send-order scheduling based on existing information afforded by the
protocol." #lived The campaign charter (`MUX-PROGRESS.md` §1) was
committed before any technical work, at Finch's direction — the
scratchpad-to-repo correction came within the first hour, and every
phase after it wrote its findings into the committed record as it went.

The first finding arrived before the first theorem was even attempted.
Finch had described the base artifact as proving the protocol "deadlock
free and terminating"; auditing that sentence against the artifact
found that _termination was not a kernel theorem_ — the ρ-decrease
argument was prose in `MODEL.md` §7, checked executably per instance,
never proven in general. Recorded as `AUDIT-NOTES.md` A1, and resolved
four stages later the right way: by minting the theorem
(`rho_decreases`, `maximal_run_terminal`) rather than softening the
prose. A standing side-channel — anything found _along the way_
suggesting misalignment between stated theorems and the Rust — was
Finch's second process directive, and the audit file collected twelve
entries by campaign's end. #lived

== The maps, the hinge, and the trichotomy

Phase 1 (four parallel readers over the Lean artifact, the model docs,
the deadlock design doc, and the Rust on both branches) returned the
fact that shaped everything: _the only cross-party channels are the
seventeen wire streams_ — everything else is intra-party plumbing — and
the repo already contained an informal argument for C1 (design doc §5D:
the peer's demand order is a function of the peer's tree, and flushed
bytes cannot be reordered) _for the shipped eager mux_. The hinge
question fell out of reading those side by side: §5D never considered a
scheduler that strategically _withholds_. If the receiver's consumption
order is causally computable at the sender — every discriminating
choice announced in its own reactions before the sender must commit —
then a "demand-lockstep" strategy that pushes only proven-demanded
frames and otherwise idles might be live at capacity one. #lived

The adjudication panel (five analysts, two cross-examiners, a
synthesis judge — and, in the campaign's most consequential decision, a
_calibrated executable probe_: a Python transcription of `Model.lean`
that passed twenty-one calibration gates against the kernel-checked
controls before any mux experiment ran) returned a trichotomy in place
of Finch's dichotomy: #lived

- Every _work-conserving_ scheduler — one that must push when the pipe
  has room — deadlocks, on the empirically known wedge shape, at every
  capacity tested. The right to idle, not frame choice, is the whole
  frontier. C1's spirit: true.
- The idling strategy σ\* survived every probe sweep. C1's letter:
  false, conditionally — the panel named two conditions, one a proof
  repair (the "Keystone" lemma's delivery case was broken as drafted;
  the cross-examiner repaired it by hand), one an evidence gap that
  became stage 0.
- The oracle exists (C2 true), and the campaign's third finding was
  already legible: the panel located the oracle in the base artifact's
  own schedule τ — though it drew the projection _backwards_, of which
  more below.

The evidence gap deserves its sentence: the probe's σ\* had been
accidentally _omniscient_ (reading global state), so the locality half
of C1's refutation had zero executable evidence. The panel made the
causal re-run a blocking gate — if a structurally-blinded σ\* wedged
anywhere, C1 flipped back to true with that skeleton as the fooling
wedge. It did not wedge: 4,970 runs, 497 skeletons, zero stalls, with
causality enforced by a faulting view object the strategy could not
cheat past. Two smaller reversals rode along: the receive-projection
"static oracle" the panel had recommended _jammed_ (an 11-scope
counterexample — the executable tier refuting the paper tier before any
Lean was written), and the "slot-peek" observation ruling the panel had
called load-bearing turned out not to be (the no-peek variant survived
3,470 runs). #lived

== The build wave, and the reversal of the projections

Eight worktree tracks built the suite. The parts worth the record:

_The wedge theorem's elegance._ `wc_impossibility` needed no fooling
argument and no pigeonhole: on the wedge skeleton, the protocol's own
ledgers funnel every work-conserving scheduler down a corridor where
each decision point offers exactly one legal push — so the theorem
quantifies over all strategies, omniscient included, by replaying one
forced run. Capacity-universality came the same way the base
campaign's had: the jam mechanism is capacity-blind (one-slot demux
occupation plus FIFO burial), so anchors at C ∈ {1,2,3} plus one
"no hands" certificate covered every C ≥ 4 with no induction. #lived

_The executable tier catching the panel twice._ The `muxprobe` gate
(track C) independently rediscovered the static-oracle jam the stage-0
probe had found; then track E, building T5, landed the kernel reversal:
the _send_ projection of τ — a fixed, non-adaptive list — is live at
capacity one, and it is the receive projection that jams. The panel had
the two projections exactly backwards. The corrected insight earned its
place in the exposition: feasibility is inherited from the witness
execution (the send order is the order a real execution produced), and
the "oracle" Finch conjectured had existed all along as the send log of
the deadlock-freedom proof's own witness schedule. Recognition, not
construction. #lived

_The bug that would not die._ Track F, closing T4's coverage induction
(the campaign's one genuinely new induction — closed by stage-indexing
the closure by τ, which dissolved the anticipated saturation lemma),
found the landed `MuxInv` interface _unsatisfiable at reachable states_:
a phantom-channel `Nat`-truncation alias (`wire I 0` colliding with a
walk's consumer count). Track E found the same bug independently the
same day. Track G then reproduced it a third time in the elastic twin
(`EMuxInv`), and the phase-4 review caught the fourth instance. Four
independent mintings of the same trap ended it the only way that
works: a phase-5 source fix (`RealWire`, a guarded accessor with the
mandate in its docstring, plus a kernel-checked characterization of the
trap itself). #lived

_Capacity monotonicity, nearly free._ The audit had quarantined the
folklore claim "wider channels can't hurt" (A7: consumed by nothing,
[derived] only) — and it promptly ambushed the elastic theorem's plan,
whose "parked states project to base-reachable states" premise was
false exactly because multi-parked states violate base capacity. The
repair discovered the base progress lemma never consumed the capacity
half of its invariant at all (`InvPW`), which then made T10 — deadlock
freedom at every pointwise-widened capacity vector, the theorem that
covers the deployed 65,536-scope windows — nearly free. A7 resolved by
theorem, as A1 had. #lived

== The locality finding, the payload discovery, and the inversion

Phase 4's review ladder (the house protocol: surface, operational,
interaction, assumption rounds, each briefed with the previous rounds'
negative space) confirmed nine findings, and the deepest one reshaped
the campaign's final theorems. F3: the locality encoding (`LocalEq`)
was _finer_ than the charter's "own full tree" — it baked
peer-determined labels into the view, so a strategy could read facts it
had not been told and still count as local. Finch's ruling set both the
fix and the campaign's closing method: locality means _information in
the causal past of that party at any decision point_ — and, on the
proposed remedies, that a Rust proptest of locality is _nonsensical_ (a
Rust function cannot be handed the remote skeleton; any such test is
vacuous), so the witness must be local _by construction_. And the
governing criterion, verbatim into the record: theorem statements must
output claims entirely accurate to intent; messy proofs are fine,
inaccurate claims are not. #lived

Building the by-construction witness (σ\*-causal, over an announced
view reconstructed from own structure plus observed frames) surfaced
the campaign's most satisfying single finding. The guard audit found
exactly one ingredient that is neither view-derivable nor derivable
from the payload-erased traces: the peer-determined merge labels. They
ride _frame contents_ — and the erased-trace surrogate provably
starves. The model's payload erasure had been sound for the protocol
machines (which are handed the skeleton) but had silently deleted the
labels from the _observation_ channel; the announcements the whole
campaign leaned on were never in the message pattern; they were in the
messages. Three grains, two of them wrong in opposite directions
(labels-in-view: too early, a causality violation; labels-never:
starvation; labels-at-arrival: exactly right) — which is also why my
own confident claim that the charter grain implied the legacy one
_a fortiori_ was refuted: the grains are incomparable, and the
correction is on the record twice, once in my own words. #lived

The residue then inverted, pleasingly: where T4's refutation had
liveness proven and locality hypothesized, the causal witness had
locality proven definitionally and _liveness_ as the one open conjunct.
Closing it took two tracks (the announced-prefix property — every
announced trace is a literal prefix of its true trace — a new
`RecvLedger` ground fact, the causal keystone, and finally the minting
ladder, ~4,500 lines, which also corrected the inherited plan's
receive-based phrasing: under drained pipes, announcement is a fact
about _sends_). `c1_charter_false` went unconditional on 2026-07-22.
One agent hung mid-climb and was resumed; one was killed by a harness
accident and relaunched; the liveness diagnostic that matters — a
transcript's _entry count_, not its modification time — went into the
supervision lore the hard way. #lived

== Reviews, the stop rule, and T8's spec-first method

The phase-4 repairs landed by proof where the plan had allowed retreat:
the overstated nonlocality docstrings got a genuine
consistency-certified pin; "completes" got a mux-tier termination
measure; the work-conserving classes got kernel inhabitants (closing a
classic satisfiable-empty exposure). Round 5's eight findings were all
one genre — merge-seam staleness, tracks merged after the doc sweep —
and produced the campaign's last process artifact, the merge-seam
checklist. Round 6, a checker-of-the-checker with a pre-committed stop
rule, found only consider-grade items and closed the adversarial
review formally. #lived

T8 — the window-generalized theorem, the one the single-socket
implementation actually rests on — was built under a new discipline
that Finch's faithfulness ruling made inevitable: its English statement
was fixed _before_ the build (`T8-SPEC.md`), clause by clause, each
clause carrying its own audit rule naming the weakenings that would
gut it (single-K, concrete-scheduler-only, omniscient-closure,
progress-only). The landed theorem's crosswalk graded every clause
EXACT, with zero amendments; the strategy-class quantification means
the shipped priority ladder is a proven _instance_, and the hard
conjunct had already been paid for — the causal coverage theorem _is_
T8's inference-progress half. The suite closed: T1 through T8 and T10,
every statement on the three standard axioms, the audit surface
(`Mux/Statement.lean`) restating each claim inline with proofs by
citation, so drift fails the build. #lived

== The design consequence

The campaign's engineering output ran concurrently with its proofs, at
Finch's explicit direction ("we want to assume the verification will
work and implement the algorithm we just realized can possibly work"):
a single-socket design document and a code-grounded executable plan on
the `single-connection` branch, written from the perspective of
_undoing_ the Link abstraction — the transport returns to one
read/write channel, windows are advertised in the greeting, the sender
is gated by inferred consumption, and any window-obeying frame order is
valid (ordering demoted from correctness to latency tuning; the
scheduler cannot be wrong, only slow). Two ideas of Finch's collapsed
the design's cost: eager conversion of arriving frames into logical
replies is a _custody_ change, not a semantics change — and the
Backend was already the streaming sink, so the receiver half of the
refactor shrank to widening one queue. The latency analysis priced the
whole space (the K-dial law, probe-exact at 54/54: pacing K+1 frames
per round trip, parity with multi-link at K ≥ P\*+1), and the honest
final trade is recorded in the design doc: what multi-link still buys
is byte-granularity interleaving and loss isolation — physics, not
information, not liveness, not round trips. #lived

== The second errata ledger

Part two's mistakes, same contract as §13; each caught by the
campaign's own machinery. #lived

1. The panel's adjudication had τ's projections backwards (receive
   recommended, send refuted) — corrected first executably (P2), then
   at kernel tier (track E). Superseded-markers preserve both layers.
2. The panel's Keystone lemma was unsound as drafted (the delivery
   case); repaired in cross-examination — the phase-2 process working
   as designed, adversarial verification inside the panel itself.
3. The probe's σ\* was accidentally omniscient — the single most
   dangerous gap, because 2,150 green runs _looked_ like locality
   evidence. Caught by the cross-examiner; became stage 0's blocking
   gate.
4. My a-fortiori claim (charter locality ⊆ legacy locality) — wrong,
   refuted by the causal track's guard audit; the grains are
   incomparable. Corrected to Finch on the record.
5. "Liveness is feedback, not knowledge" — my synthesis of the static-
   oracle jam, retracted when the send projection proved live; the
   truth is "liveness is the right order, and a right order needs
   either inference or the witness execution."
6. The phantom-channel alias, minted independently four times
   (`MuxInv` twice, `EMuxInv`, and the review's catch); ended by a
   source fix, not a fourth patch.
7. The coverage plan's receive-based minting phrasing — wrong on
   slot-parked frames; corrected to the send-based form by the track
   that closed it.
8. Two agents lost to infrastructure (one hang, one harness
   accident); one worktree ledger commit briefly truncated
   `MUX-PROGRESS.md` (caught in-merge, amended). Supervision lore:
   entry counts, not modification times; two samples, not one.
9. The merge-seam genre: six of round 5's eight findings were
   staleness introduced by merging tracks after the doc sweep. The
   checklist exists so this class is mechanical next time.

== What the second campaign adds

Part one closed by observing that the gate caught every modeling error
and the kernel caught none — that the two-tier discipline was the
story. Part two sharpens it: _the executable tier out-caught the paper
tier every single time they disagreed_ — the projections, the no-peek
ruling, the pacing law (both circulating sketches wrong; the harness
exact), the odd-width ceiling. The kernel then froze what the probes
found. Probe first, prove second, pin forever remains the method, and
this campaign added its complement: _fix the English first_. The
statement-faithfulness ruling — claims accurate to intent, proofs as
messy as needed — became executable process in `T8-SPEC.md`, and the
theorem built against a pre-committed specification graded EXACT on
every clause, first try. The deepest results were again
re-descriptions: that the announcements were in the payloads all
along; that credits carry computation and timing, not information;
that the oracle was the proof's own witness schedule, waiting to be
recognized; and that the impossibility Finch correctly sensed was
never about muxing — it was about the refusal to idle. Both
conjectures were settled where neither of us predicted, which is to
say: Finch asked to be surprised, and the machine obliged — twice,
in opposite directions, with kernel receipts.

= Sources and provenance of this document

*Part two* (2026-07-22, written by the campaign coordinator): the
primary sources are the coordinator's own session (every adjudication
exchange with Finch quoted or paraphrased above appears there verbatim),
the fork and track reports received in that session, and the durable
record committed as the campaign ran: `MUX-PROGRESS.md` (charter,
resolution with superseded-markers, findings ledger, log — the single
best secondary source), `MUX-ADJUDICATION.md` and the phase-2 panel
briefs under `mux-notes-phase2/` (including `STAGE0-GATES.md` and the
graduated probe reference `causal-reference.py`), `AUDIT-NOTES.md`
(A1–A12), `MUX-STATEMENT-AUDIT.md`, `T8-SPEC.md` with its landed
crosswalk, `MUX-LATENCY.md`, the design documents on the
`single-connection` branch (`design/single-socket.md` and its
executable plan), and the git logs of the campaign worktrees. Every
part-two claim is #lived or #artifact by those sources; part two, like
part one, needed no #recon.

*Part one:* primary sources, mined 2026-07-19 with Finch's explicit
authorization for the transcript material: the session transcripts under
`~/.claude/projects/` for the Quint/transcription era (including the
original plan invocation and the TLA+→Quint reversal in Finch's own
words), the Lean-campaign era, and my own session (whose earlier portions
survive only in transcript, having been compacted from my context
repeatedly); the surviving review-workflow scripts from the era project
directories; `formal/PROGRESS.md` (§§1–11) and its 43-revision git
history; `formal/README.md`'s findings ledger and phase record;
`formal/MODEL.md`; `formal/PLAN.md`; `design/parent-placement.md` and the
probe/pin commits on the `parent-first` branch; the plan file
`currently-we-have-elaborate-recursive-muffin.md`; and the full git logs
of both branches. Where a claim rests on my own inherited session context
(fork reports, adjudication exchanges), it is tagged #lived and the
transcript is the backing record. The Quint-era claims are tagged
#artifact or #lived per source; nothing in this document is tagged
#recon — every claim found a primary source, which I did not expect when
I started and record with some satisfaction.
