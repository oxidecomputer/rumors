// The exposition of record for the streaming-mirror deadlock-freedom
// artifact. Audience: technically competent, no familiarity with the
// codebase, the data structure, or the algorithm assumed. The faithful
// development history is the companion document (narrative.typ); this
// document explains what is true and why, and may compress history
// where compression aids understanding.
//
// Build: typst compile exposition.typ (the PDF is derived, not tracked).

#set page(paper: "us-letter", numbering: "1", margin: (x: 1.15in, y: 1in))
#set text(size: 10.5pt)
#set heading(numbering: "1.1")
#set par(justify: true)
#show raw.where(block: true): set text(size: 8.75pt)
#show raw.where(block: true): block.with(
  fill: rgb("#f6f5f2"),
  inset: 8pt,
  radius: 3pt,
  width: 100%,
)
#show link: underline

// ---- Epistemic tags ------------------------------------------------
// Every load-bearing claim in this document carries one of three tags.
#let tag(body, fillc, strokec) = box(
  fill: fillc, stroke: 0.5pt + strokec, inset: (x: 3.5pt, y: 1.5pt),
  radius: 2pt, baseline: 20%,
  text(size: 0.72em, weight: "bold", tracking: 0.4pt, body),
)
#let kernel = tag([KERNEL], rgb("#e7f2e7"), rgb("#5a8a5a"))
#let gate = tag([GATE], rgb("#e7ecf5"), rgb("#5a6f9a"))
#let assumed = tag([ASSUMED], rgb("#f7ecdd"), rgb("#a8814f"))

// --------------------------------------------------------------------

#align(center)[
  #text(size: 20pt, weight: "bold")[Deadlock Freedom in the Streaming Mirror]
  #v(2pt)
  #text(size: 12.5pt)[Two disciplines, two theorems — and one channel]
  #v(6pt)
  #text(size: 9.5pt, fill: rgb("#555555"))[
    An exposition of the `rumors` formal artifact · 2026-07-19, extended
    2026-07-22 \
    Statements of record: `formal/lean/StreamingMirror/Statement.lean`
    (act one) · `…/Mux/Statement.lean` (act two)
  ]
]

#v(10pt)

#block(
  fill: rgb("#f6f5f2"), inset: 10pt, radius: 3pt, width: 100%,
)[
  *The result, in one box.* A streaming tree-reconciliation pipeline
  can order its per-scope sends in two disciplines: publish each
  scope's summary _early_ (the moment its disputed children resolve) or
  _last_ (after all of the scope's other traffic). Both are
  deadlock-free, under different hypotheses, and both facts are
  machine-checked down to the Lean kernel:

  #v(4pt)
  #grid(columns: (1fr, 1fr), column-gutter: 10pt,
    [
      *Parent-late* — the shipping encoder. Deadlock-free given the
      capacity discipline the code already enforces (assembler buffers
      at least as deep as any scope's dispute count).
      #v(2pt)
      #raw(block: true, lang: "lean",
"theorem deadlock_free :
  sk.wellFormed = true →
  (∀ s, sk.dCount s ≤ sk.capLevel) →
  DeadlockFree sk AxMode.impl")
    ],
    [
      *Parent-early* — the priced alternative. Deadlock-free at _any_
      buffer capacity, at the cost of serializing descent against
      assembly. No shipping encoder follows it; the theorem prices the
      corner.
      #v(2pt)
      #raw(block: true, lang: "lean",
"theorem deadlock_free_d5 :
  sk.wellFormed = true →
  sk.schedulable = true →
  DeadlockFree sk AxMode.full")
    ],
  )
  #v(2pt)
  Each rests on Lean's three standard axioms only (`propext`,
  `Classical.choice`, `Quot.sound`) — no `sorry`, no `native_decide`.
  #kernel
]

#v(6pt)

#block(
  fill: rgb("#f6f5f2"), inset: 10pt, radius: 3pt, width: 100%,
)[
  *The second result, in one box.* Act two asks whether those
  independent channels are themselves necessary: can one bounded
  read/write channel carry the whole session, with deadlock freedom
  restored purely by _scheduling_ the protocol's existing messages?
  The answer is a kernel-checked trichotomy:

  #v(4pt)
  - *Eagerness is fatal.* One fixed, tree-realizable skeleton defeats
    _every_ scheduler that must send when the pipe has room — at every
    capacity, at every _parking depth_ (how many frames a receiver
    will hold per stream), locality not even assumed
    (`wc_impossibility`, `wc_impossibility_K`). #kernel
  - *Patience and inference suffice.* A deterministic strategy that is
    local by construction — its every decision a function of the
    party's causal past — is deadlock-free at every capacity
    (`sigmaStarCausal_deadlock_free` + `sigmaStarCausal_charterLocal`);
    the conjecture that no such strategy exists is refuted outright
    (`c1_charter`). #kernel
  - *Omniscience buys only an order — and the order already existed.*
    A _fixed_ send order computed from both trees is live at capacity
    one (`oracle_deadlock_free`): the send projection of the
    deadlock-freedom proof's own witness schedule. #kernel
  #v(2pt)
  One theorem generalizes all of this to the point an implementation
  would build on. At any advertised per-direction window depths, _any_
  window-obeying frame order is deadlock-free and completes
  (`sigmaStarK_deadlock_free`, `sigmaStarK_completes`). Its
  specification was fixed in English before the theorem was built, and
  the landed statement matches it clause for clause; the crosswalk
  lives with the theorem's entry on the audit surface
  (`Mux/Statement.lean`). #kernel
]

#v(6pt)
#outline(indent: auto, depth: 2)

= What this document is

This document explains a formal verification result for the _streaming
mirror_, the tree-reconciliation protocol at the heart of `rumors`, a
Rust library for gossip with redaction. It is written for a reader who
knows systems programming and is comfortable with an invariant and an
induction, but has never seen this codebase, this data structure, or
this protocol. Everything needed is introduced.

The story is worth telling as a story, because the result is not the
one the effort set out to prove. The campaign began with a single
theorem in mind: the protocol cannot deadlock. The attempt to prove it
produced four things, in order. First, three ordering invariants the
implementation had been relying on silently. Then a machine-checked
counterexample: the intended theorem was _false_ as stated. Then the
discovery that the falsity was not a bug but an unarticulated _design
trade_, with two defensible sides. And finally two theorems, one per
side. One describes the implementation that ships; the other prices
the alternative. The deadlock-freedom claims themselves are the headline,
but the shape of the trade — and the exact capacity arithmetic on which
it turns — is the part a designer of a similar system will want.

The document has two acts. Act one is the result above: the
protocol over its intended transport — independent bounded channels —
under two send disciplines. Act two removes the transport
assumption and asks the question the first act leaves standing: the
theorems assume the cross-party wire streams cannot block one
another; the deployed system once muxed them over a single pipe and
deadlocked; _is that deadlock fundamental?_ Under proof, the answer
reorganized itself into the trichotomy of the second box. It also
produced an engineering consequence with a twist. The single channel
is provably viable — and the transport contract that forbids it stands
anyway, as the better product decision. Theorem-backed, not
theorem-forced (@consequence). This document presents each result
where the argument wants it, not in the order it was discovered; the
companion narrative preserves the discovery order, which was
considerably more surprising.

Three epistemic tags appear throughout, because this artifact is
honest about its trust boundaries:

- #kernel — proved in Lean and checked by its kernel; the strongest
  claim we know how to make.
- #gate — established executably, on every gated commit, by an
  independent implementation cross-checking the Lean definitions on
  hundreds of randomized protocol instances. This is how we know the
  _definitions_ mean what the theorems need them to mean; a kernel
  cannot check that.
- #assumed — stated, named, and argued for, but not proven. There are
  exactly two such items (@trust).

#v(4pt)
#heading(numbering: none, level: 1)[Act one — two disciplines, independent channels]

= The problem

== Reconciliation with redaction

`rumors` maintains a replicated set of messages across peers that
gossip pairwise: two peers connect, discover what each is missing, and
exchange exactly that. Deletion is by _redaction_ and leaves no
tombstones. Whether an absent message is "never seen" or "deleted" is
decided by _version bounds_ carried in the tree — per-subtree version
ceilings that let a peer conclude "anything this old here was
deliberately removed" without a marker per message. So the
reconciliation structure must let two peers compare entire histories
cheaply and locate their differences precisely.

The structure is a Merkle radix trie: messages sit at content-addressed
leaves; each interior node summarizes up to 256 children by hash. Two
peers with equal root hashes hold equal sets and are done. Two peers
with unequal roots recurse: exchange child summaries, discard matching
subtrees, descend into differing ones. The recursion touches only the
subtrees that differ — reconciliation cost scales with the difference,
not the database.

== Why streaming

The straightforward implementation of that recursion is
request/response per level: send the children of every disputed node,
wait for the peer's reactions, recurse. `rumors` ships that protocol
too (the _alternating_ mirror) and uses it as the behavioral oracle for
the one this document is about. Its cost is latency: a 32-level trie
means 32 round-trip phases, each fully drained before the next begins.

The _streaming_ mirror instead runs the whole recursion as a pipeline.
Each tree level is a stage; stages for different levels run
concurrently; a subtree's descent begins the moment its dispute is
known, while shallower levels are still being compared. Messages for
all levels interleave on the wire. Memory stays fixed: no stage
materializes a subtree; everything is bounded queues between stages.

That design decision — deep pipelining over bounded queues — is where
the deadlock question comes from. A pipeline of processes connected by
bounded channels can wait in a cycle: A's send blocks on a full queue
that B will drain only after receiving something C will produce only
after A's send lands. The rest of this document is about why that
cannot happen here — under either of two disciplines, for two different
reasons.

= The machine

== The walk: scopes, disputes, requests

Fix two peers with unequal roots. The session recurses over the
_dispute skeleton_: the tree of nodes both sides actually descend into.
A node in that tree is a _scope_. Comparing one scope's children, each
child lands in one of three classes:

- *Disputed* (D): both sides hold it, hashes differ. It becomes a
  scope one level down; the recursion continues into it.
- *Requested* (R): one side lacks it entirely and asks for the whole
  subtree; the other side supplies it outright — the supplied subtree,
  a run of frames with no interaction, is a _provision_. No further
  recursion.
- *Matched* (M): hashes agree, or the difference is one-sided content
  the receiving side's version bounds let it absorb without asking. No
  traffic at all; dropped from the skeleton.

The two parties play alternating roles by level — for a given scope,
one party is the _asker_ (it sent the query that opened the scope) and
the other the _answerer_ — and the roles flip at each level of descent.
Because the roles alternate, each party's walk stages sit at every
other height; that is why the pipeline's stages step down two heights
at a time. Both parties materialize their own copy of every reconciled
subtree, so both run the full pipeline; the two sides are duals.

== The pipeline: processes and channels

Per party, the session is a fixed set of sequential processes wired by
bounded single-producer/single-consumer FIFO channels
(@fig-pipeline):

- One *walk stage* per tree height: a loop over the scopes at that
  height, in order. For each scope it receives the peer's child
  listing (a _wire_ message) and the query that opened the scope, then
  publishes its traffic for that scope (next section).
- The *assembler tower*: one assembler per height, reuniting each
  scope's resolution with the completed returns of its child subtrees
  — positionally, in scope order — and passing the assembled subtree
  up one level, toward the root return.
- Fixed-shape processes at the session's two ends: the *openers*
  launch the root exchange, the *finishers* run each stream's
  end-of-stream close, and the leaf *absorber* consumes leaf-level
  provisions.

#figure(
  kind: image,
  raw(block: true,
"                     the peer (dual pipeline)
                          ▲ ▲ ▲
        wire channels (the ONLY cross-party edges, cap 1)
                          │ │ │
 ┌────────────────────────┴─┴─┴──────────────────────────────┐
 │  Walk(h+2)  ──asked──▶  Walk(h)  ──asked──▶  Walk(h−2) …  │
 │  (one walk stage per height; each walks its scopes in     │
 │   order: recv wire, recv query, publish, next scope)      │
 └───────┬──────────────────┬─────────────────┬──────────────┘
         │ resolutions      │ parent          │
         │ (cap 1)          │ summaries       │
         ▼                  ▼ (cap 1)         ▼
 ┌────────────────────────────────────────────────────────────┐
 │  Asm(j) ──level(j)──▶ Asm(j+1) ──level(j+1)──▶ … ──▶ root  │
 │  (the assembler tower; level channels have capacity        │
 │   C = FAN = 256 — every other channel has capacity 1)      │
 └────────────────────────────────────────────────────────────┘"),
  caption: [
    One party's slice of the pipeline, schematic. Queries (_asked_)
    flow toward the leaves between walk stages; resolutions and parent
    summaries flow from the walks into the assembler tower; assembled
    subtrees flow up the _level_ channels toward the root. Wires are
    the only channels that cross between the parties.
  ],
) <fig-pipeline>

Two facts about this topology do most of the work in everything that
follows. First, *only wire messages cross between the parties*;
queries, resolutions, parent summaries, and assembled returns are all
intra-party plumbing between pipeline stages. Second, *every channel
has capacity 1 except the assembler tower's level channels*, whose
capacity C is the one tunable that matters (the shipping code sets
C = FAN = 256, the radix).

== A scope's traffic

When a walk stage processes one scope, it owes the following sends —
this vocabulary recurs throughout:

- per child, a *wire* message: its own summary of that child, sent to
  the peer;
- per _disputed_ child, a *resolution*: the local record that this
  child becomes a subtree to assemble, sent to the assembler at that
  height; and one *query* per grandchild under it, sent to the walk
  stage two levels down — this is what launches the deeper descent;
- exactly one *parent summary* for the scope itself, sent up to the
  assembler one height above: "this scope resolves into the following
  children; assemble me when their subtrees arrive."

Within one channel the sends are in child order, always (positional
pairing is the protocol's identity carrier: the $n$-th message on a
channel is _about_ the $n$-th scope of the consuming stage — there are
no per-message addresses). Across channels, the order is a real
degree of freedom, and it is the subject of this entire document.

== From code to model

The Lean model abstracts this machine faithfully but finitely; its
definitions of record are `Skel.lean` and `Model.lean`, whose module
docs carry the specification:

- Sessions are quantified by *dispute skeletons* — the finite labeled
  trees of scopes defined above. `Skel.wellFormed` (about 25 lines)
  delimits the shape class to real session shapes: each stage's scopes
  are exactly the previous stage's disputed children, in order, and
  the per-scope child tables agree between levels. Payloads are erased; the protocol's
  channel behavior depends only on each child's D/R/M class, which is
  why erasing them is sound.
- Each process is a finite partially ordered set of operations (sends
  and receives); a state is the set of fired operations per process;
  channel occupancies are derived. One step fires one enabled
  operation.
- Publication order within a scope is _not_ fixed to the Rust's order.
  The model quantifies over every order permitted by a set of declared
  ordering rules (next section), under *committed choice*: a publisher
  chooses its next send when idle, and a chosen send _parks_ until its
  channel has room — it cannot be retracted or reordered past. This
  models a sequential implementation awaiting a bounded send, and it
  is load-bearing: under retractable ("may-fire") semantics, no
  ordering rule could ever cause a deadlock, because the checker would
  simply fire some other send. Real programs cannot skip ahead;
  commitment is what captures that.
- Cross-process interleaving is fully adversarial: any enabled
  operation, any process, any step.

*Deadlock freedom* is then: from every reachable state, either the
session is finished (`terminal`) or some operation is enabled
(`canStep`). No fairness assumption is needed anywhere — every step
fires an operation from a finite budget, so runs are finite and
termination is a corollary of deadlock freedom rather than a separate
liveness argument.

== The ledgers

The ordering rules — the model calls them _ledgers_, after the
trace-validation facility in the Rust (`Trace::assert_valid`) that
checks each of them on real encoder traces — are the load-bearing
interface between implementation and theorem. In one sentence each
(`Statement.lean` is the audit surface; `AxMode` in `Skel.lean` is the
definition):

/ w: a child's wire message precedes that child's internal
  publications.
/ d1: a resolution precedes its dependent queries.
/ d2: a parent summary follows all its disputed children's
  resolutions.
/ d3: sibling contiguity — a disputed child's _dependent work_ (its
  resolution and all of its queries) is published before the next
  child's resolution.
/ d4: wire contiguity — a child's wire message is not sent while any
  earlier disputed sibling is still unresolved or still owes queries;
  wires may not outrun the dependent work of the children before them.
/ d5: *parent-early* — once every disputed child is resolved, the
  parent summary precedes any further wire or query of the scope.
/ d6: *parent-last* — the parent summary is its scope's final send.

Mode `.full` asserts the first five with `d5`; mode `.impl` asserts
the first five with `d6`. The two placement rules contradict each
other (at any scope with traffic left after the final disputed
resolution) and are never combined.

A historical note that doubles as a warranty: *d3, d4, and the d5/d6
pair were not in the original interface*. Each was surfaced by this
verification effort. In each case the model exhibited a publication
order that satisfied every _written_ rule and deadlocked. And in each
case the implementation was found to be enforcing the missing rule
silently, by code structure rather than by contract. (All three
survive as pinned counterexamples, in `Controls.lean` and the
executable gate's pinned matrix.)
The Rust trace validator was tightened in step each time, so the
checked interface and the assumed interface coincide. The third of
these findings is @discovery, and it is the reason this document
describes two theorems rather than one.

== Two tiers of checking <two-tier>

The artifact is verified at two tiers, and the distinction matters for
what a reader should trust:

- The *kernel tier*: every theorem in the Lean development — 48
  files and just under 40,000 lines at act one's close — is checked by
  the Lean kernel, down to the three standard axioms.
  #kernel
- The *executable tier*: `lake exe eventdag` re-implements the
  schedules and the model _independently_, and on every gated commit
  it cross-checks the two on 300 randomized skeletons plus a pinned
  regression suite. The checks: transcription equality of the
  proof-side schedule definitions against the imperative model; replay
  of the witness schedules to session completion under both modes;
  adversarial drain assertions (the margin-0 `.impl` drains must
  complete — margin 0 is the capacity discipline of @floor — and the
  sub-margin traps must still reproduce); and a boundary matrix around
  the capacity law of @floor. #gate

The kernel tier makes the theorems unimpeachable _given the
definitions_. The executable tier is the answer to the question every
formal-methods skeptic should ask — "how do you know your definitions
describe the machine?" — and it is a real answer: a typo'd guard, a
swapped index, or a mistranscribed schedule breaks gate assertions
loudly, because an independent implementation disagrees on concrete
instances. Where the Rust itself is the referent, the tie-off is the
trace validator and the capacity pins of @chain.

= The discovery <discovery>

== One freedom too many

The first campaign target was the natural one: deadlock freedom under
the ledgers as then written — wire discipline, dependency order,
sibling and wire contiguity — for every well-formed, schedulable
skeleton. The proof strategy (an argmin argument; @proof) requires
that at any blocked state there is a well-defined "earliest missing
event," and the attempt to establish that property kept sliding off
one specific freedom the written rules left open: *nothing forced the
parent summary out of the walk*. A walk could resolve every disputed
child of a scope and then keep publishing — the last child's queries,
trailing wires for matched children — with the parent summary still in
hand.

Following the campaign's validate-before-prove discipline, the freedom
was probed executably before any more proof effort was spent: drive
the model with an adversary that always defers parent summaries behind
every other legal send. On random skeletons, the adversary found
genuinely stuck states. Minimization produced `Control.pdelay`, an
eleven-scope skeleton whose stuck run is now a kernel-checked theorem
(`Control.parentTrap_not_deadlockFree`): a well-formed, _schedulable_
session, every rule of the then-current interface obeyed, deadlocked.
#kernel The intended theorem was false.

The cycle deserves one paragraph, because it is the heart of the whole
design question. The parent-delaying walk commits to a last-chunk
query and parks on a full capacity-1 query channel. The summary it
withholds is exactly what the assembler one level up needs to begin
assembling the scope. Starved, that assembler stops consuming its own
level channel. The backlog propagates down the tower. Eventually a
_deeper_ assembler stops draining a lower walk's parent summaries.
That lower walk is parked on its own committed parent-summary send.
And its progress is what would have drained the query channel the
first walk is parked on. Commitment closes the cycle: nobody can
retract, nobody can proceed.

== Not a bug: a trade

The obvious reading — "the encoder must be re-ordered" — is wrong, and
discovering _why_ it is wrong was the most valuable step of the
campaign. The shipping encoder publishes the parent summary in the
scope's _epilogue_, after all other traffic, and does so deliberately;
the code comments call the order progress-critical: launch every
pending subtree's work before publishing the enclosing summary. The
placement is a _criticality ordering_. A parent summary is the least
urgent message a walk ever sends: its consumer must wait for subtree
returns that arrive far later anyway. The sends it would preempt under
parent-early are the most urgent in the protocol — the queries that
launch deeper descents and the wires the peer is waiting on. Parent-early buys deadlock immunity by releasing the upward
obligation before entering any send that can jam; parent-late buys
maximal pipelining by deferring the deferrable. (Since parent
summaries never cross the wire, neither placement changes the peer's
logical dependencies or the session's round-trip structure; what
parent-early costs is _overlap_ — a rendezvous with the assembler
tower at every scope boundary, on the critical path of descent.)

So the freedom the rules left open was not an oversight in the
implementation; it was an oversight in the _interface_. There are two
coherent disciplines. Each got a ledger (`d5` parent-early, `d6`
parent-last), each got a mode, and each got its theorem. The rest of
this document treats them as the peers they are.

== The capacity floor <floor>

Parent-late deadlocks only when the assembler tower can actually jam,
which makes buffer capacity the other axis of the trade. The exact law
— checked from both sides in both the model and the Rust — is:

#align(center)[
  _a parent scope disputing_ $N$ _children completes under every
  schedule iff_ $N <= C + 2$,
]

where $C$ is the level-channel capacity. #gate The `+2` is real slack
with a mechanical explanation. Beyond the $C$ returns buffered in the
channel, one return sits in the blocked assembler's _hand_ — its
committed send, parked but still holding its item. One more child
resolution sits parked in the capacity-1 resolution slot. That child's
return is not materialized yet, so it needs no room until the parent
summary frees the drain. Two borrowed positions, one at each end of
the bounded channel. The law is pinned executably at the boundary from
both directions: the Rust pipeline on a 256-fan stress tree stalls with
the assembler channel at 253 and completes at 254, and the model's
scaled instances reproduce the same threshold shape (the executable
gate's boundary matrix pins it).

Two properties of this floor made the theorem-design decision for us:

+ *The tight floor is poll-schedule-specific.* At exactly
  $N = C + 2$, the _deterministic_ Rust pipeline completes — but the
  model, which quantifies over every cross-process interleaving,
  exhibits an adversarial schedule that stalls even with the encoder's
  own per-walk send order enforced. #kernel The empirical floor is a
  fact about the runtime's actual poll orders, not about the
  discipline.
+ *One more unit of slack ends the case analysis.* At margin 0 —
  capacity at least the dispute count itself, $C >= N$, which is the
  shipping configuration (`FAN = 256 >=` children per scope
  $>=$ disputes per scope) — level-channel sends _never park at all_.
  The inequality doing the work deserves its own paragraph. A
  resolution's _pending count_ is the number of its disputed children
  whose assembled subtrees have not yet come back. The consuming
  assembler handles one parent resolution at a time, in scope order,
  and takes that parent's returns positionally. So every return
  sitting in the level channel belongs to the one parent currently
  being assembled: at most pending-count many, the pending count is at
  most the scope's dispute count, and margin 0 makes the dispute count
  at most $C$. The channel cannot fill. (That returns never accumulate
  for a _later_ parent is precisely the counting fact the proof
  establishes at this site; the schedule's positional structure
  forbids it.) The borrowed slots, and their sensitivity to
  implementation details at the channel's two ends, exit the argument
  entirely.

The flagship theorem therefore takes margin 0 as its capacity
hypothesis: it is simultaneously the robust bound (interleaving-proof,
implementation-detail-proof) and the honest one (it is what the
shipping code enforces, and what its author had proven to themselves —
the interface-level argument that needs no borrowed-slot accounting).
The tight $C + 2$ boundary stays characterized where fragile knowledge
belongs: in kernel-checked counterexamples and executable pins, not in
the hypotheses of the headline theorem.

== The design space, priced

#figure(
  table(
    columns: (auto, 1fr, 1fr),
    align: (left, left, left),
    stroke: 0.5pt + rgb("#cccccc"),
    inset: 6pt,
    [], [*parent-early (`d5`, mode `.full`)*],
    [*parent-late (`d6`, mode `.impl`)*],
    [who ships it],
    [no one (the model's witness schedule; realizable as an encoder —
     the needed lookahead is one scope's child classification, which
     the protocol delivers before any of the scope's sends)],
    [the `rumors` streaming encoder, deliberately],
    [assembler floor],
    [none — any capacity $>= 1$],
    [capacity $>=$ max per-scope disputes (margin 0; the tight
     adversarial floor sits 2 below, and is poll-schedule-specific)],
    [pipelining],
    [descent rendezvouses with the assembler tower at every scope
     tail; worst case degrades toward level-by-level lockstep],
    [fully decoupled; assembler backpressure lands on the one send
     nothing downstream waits for],
    [round-trips],
    [unchanged], [unchanged],
    [theorem],
    [`deadlock_free_d5` #kernel],
    [`deadlock_free` (the flagship) #kernel],
  ),
  caption: [
    The parent-placement design space. Both corners are proven; the
    implementation occupies the right one. Revisit the left corner
    only if the capacity floor ever becomes untenable — e.g. a
    memory-constrained peer that cannot afford radix-deep assembler
    buffers.
  ],
) <fig-space>

= The two theorems, precisely <theorems>

Both theorems live behind a deliberately small audit surface: to check
_what is claimed_, a skeptical reader reads `Statement.lean` and the
handful of definitions it names (about 220 lines total), and nothing
else. In prose:

*The flagship* (`Sched.deadlock_free`, `Proofs/EndgameE.lean`, via
`Sched.progress`): for every well-formed dispute skeleton whose
per-scope dispute counts are all at most the assembler capacity, no
state reachable under mode `.impl` — the encoder's real send order,
parent summary last, under committed choice and fully adversarial
cross-process interleaving — is stuck: every reachable state can step
or is terminal. #kernel

*The counterpart* (`Sched.deadlock_free_d5`, `Proofs/Endgame.lean`,
via `Sched.progress_d5`): the same conclusion under mode `.full` —
parent-early — with the capacity hypothesis weakened to
`schedulable`: per-scope disputes at most capacity _plus two_. That
bound is exactly the frontier past which no session could ever finish.
One past it, `pyramid1_not_schedulable` exhibits a well-formed
skeleton — from the _pyramid_ family, a single scope disputing many
children at once — whose greedy run jams, kernel-checked as
`pyramid1_not_deadlockFree`. That no interleaving whatsoever completes
it is checked at the executable tier (@two-tier). The counterpart is
therefore as capacity-general as any theorem could be. #kernel

Three structural notes:

- *The hypotheses are ordered by strength.* Margin 0 strictly implies
  `schedulable`; the flagship's statement mentions only margin 0
  because the weaker bound is subsumed, and the gap between the two —
  exactly the two borrowed slots of @floor — is where the parent-late
  trap lives.
- *Every hypothesis is load-bearing, and each is a theorem, not a
  promise.* Drop parent placement in either direction and
  `Control.parentTrap_not_deadlockFree` refutes the statement on a
  schedulable skeleton; drop wire contiguity and
  `Control.jam_not_deadlockFree` refutes it; exceed `schedulable`
  under `.full` and `Control.pyramid1_not_deadlockFree` refutes it.
  The negative controls are kernel-checked stuck runs, so the
  interface cannot silently rot: a model change that dissolves a trap
  breaks a theorem. #kernel
- *Termination rides along free.* Every operation comes from a finite
  skeleton-derived budget and every step spends one, so there are no
  infinite runs; "every maximal run ends terminal" is a corollary of
  deadlock freedom, with no fairness hypothesis anywhere.

== The chain to the implementation <chain>

The theorems quantify over _any_ system whose traces obey the ledgers
and whose configuration meets the capacity hypothesis. The shipping
Rust is tied to those hypotheses at both ends, so the chain
"prop-tested local invariants $arrow.r.double$ proven global theorem"
closes:

- *Ledgers $arrow.l.double$ traces.* `Trace::assert_valid`
  (`src/tree/mirror/streaming/materialized/progress.rs`, branch
  `parent-first`) checks every ledger of `AxMode.impl` on every
  encoder trace the streaming property tests produce — including,
  since the discovery, `Trace::assert_parent_last`, the `d6` check
  mirrored verbatim from the model's guard, exercised positively by
  the full streaming suite and negatively arm-by-arm. The `d5` corner
  deliberately has _no_ wired Rust check: `assert_parent_early` exists
  unwired with a `should_panic` pin documenting that the real encoder
  violates parent-early — the design-space record, preserved so it
  cannot rot silently.
- *Margin 0 $arrow.l.double$ configuration.* The capacity hypothesis
  is discharged by the shipping constant `FAN = 256` (the assembler
  channel capacity) against the radix bound (children per scope
  $<= 256$), pinned from both sides: a stress test that stalls at 253
  and completes at 254 (the slack is genuinely consumed), and the
  parent-delay boundary probes added during the discovery.
- *Definitions $arrow.l.double$ the executable gate*, per @two-tier.

= The proof, humanly <proof>

This section presents the real argument for the flagship — the same
argument the Lean artifact makes, at prose altitude. Cross-references
name the actual modules and theorems; `Proofs/Map.lean` is the
file-by-file navigation companion, including a table mapping each
module of this proof to its counterpart in the `d5` proof. The
argument has five stages.

#block(inset: (left: 10pt), stroke: (left: 2pt + rgb("#cccccc")))[
  *Stage A — a witness schedule.* Construct one concrete completion
  order for the whole session. \
  *Stage B — edge-respect.* Prove the witness never sends into a full
  channel nor receives ahead of supply. \
  *Stage C — merge completeness.* Prove every process's trace embeds
  in the witness order, making "schedule position" a total potential
  τ. \
  *Stage D — decode.* Prove every process of every reachable state
  sits _at_ a position of its trace. \
  *Stage E — argmin.* At any non-terminal reachable state, the
  τ-least pending event is enabled. Hence progress; hence deadlock
  freedom.
]

== Stage A: the witness schedule

The proof does not chase wait-cycles through arbitrary states.
Instead it builds one distinguished global order of every operation in
the session — the _eweave_ (`Sched/WeaveE.lean`): scope by scope in
breadth-first order, each scope's traffic in the encoder's own
per-child order, parent summary last, interleaved with the assembler
and absorber operations that consume it. This is a ghost object: no
scheduler is claimed or required to follow it. But it is a concrete
one. Before anything is proven about it, the executable gate replays
it to session completion under `.impl` on every pinned and randomized
skeleton, at margin 0. The gate also checks the proof-side definition
event-for-event against an independent construction. #gate The
schedule assigns every operation a position; call the position of an
event its τ.

Two terms of art describe the schedule's internal anatomy, and the
comparison between the two proofs is written in them. A _chunk_ is one
child's slice of a scope's traffic, taken as a unit: the child's wire
message, then — if the child is disputed — its resolution and its
queries. A scope's segment of the schedule is its chunks in child
order. The only remaining freedom is where the scope's one parent
summary goes. The eweave answers _last_, after the final chunk; the
`e` is for the encoder, whose order it mirrors. The `d5` proof builds
a different witness, the plain _weave_, which _splices_ the parent
summary into the middle: immediately after the last disputed child's
chunk. Everything downstream is parameterized so the two witnesses
share machinery. "Spliced versus last" is exactly where they differ.

== Stage B: the witness is edge-respecting

`weaveE_wedge` (`Weave/MasterE.lean`): walking the eweave from the
start, every send lands in a channel with room and every receive finds
its datum present, given only well-formedness and margin 0. #kernel

This is the bulk of the artifact. Its shape is an induction with a
strengthened invariant. The invariant carries a counting state — per
channel-side, how many events have fired — plus one _window_ for each
_site class_, meaning each kind of send the schedule can be standing
at (a wire, a resolution, a query, a parent summary). A window is the
arithmetic fact about the counts above and below that makes the next
send admissible. (These proof-side windows are unrelated to act two's
flow-control windows; the collision is historical.) Receives are the
easy half. The schedule places every receive after its matching send
— the message edge. Positional pairing, plus a per-channel numbering
theorem (`Sched/Numbering.lean`: on every channel-side, the family's
events carry sequence numbers $0, 1, 2, dots$ in order), then turns
"the datum is present" into an inequality between two counts.
Sends into capacity-1 channels between walks are similar: the previous
occupant's receive sits between any two same-channel sends in the
schedule, by construction.

The interesting site — the one that distinguishes the two proofs — is
the parent summary's send into the assembler tower. Here the flagship
cashes its capacity hypothesis, once, in one inequality: _a level
channel's occupancy never exceeds the pending count (@floor) of the
one parent resolution its consumer is currently assembling_ — at most
the scope's dispute count, which is at most $C$ by margin 0. So the send
site's window closes by a counting lemma; the tower above the site
literally cannot be full. The occupancy bound itself is where the
schedule's structure earns its keep: assemblers consume positionally
in scope order, so return backlog never spans two parents — the model
twin of the queues-module doc comment "the bound does not multiply
with tree width or depth."

It is instructive to see what the `d5` proof must do at the same site,
because the comparison _is_ the design trade, restated as proof
effort. Under parent-early at capacity 1, the tower above a parent
send can be nearly full, legitimately. Admissibility of the send
becomes a global fact about how far the whole tower above has drained.
The `d5` proof establishes it with telescoping ancestor sums — chains
of window facts walked up the spine of the tree (its `AscCover` and
`DescSupply` telescopes: ascending coverage of drained capacity,
descending supply of completed returns) — threaded through every site
of the master induction as a rolling context. Margin 0 replaces all of
that with the one inequality above. Symmetrically, the eweave's
parent-last shape _simplifies_ every remaining site. A kid chunk of
the eweave is the `d5` chunk with the spliced parent set to "none"
(`childChunk_spliced`, `Weave/SiteE.lean`). The whole chunk algebra
therefore transfers, minus the "is the parent spliced here?" case
analysis. The ancestor counts lose their conditionals: an ancestor's
parent summary is always still pending at an interior site. And the
ascent ladders collapse from inductions to two-case analyses. The refunds recur so systematically that the
mirror table in `Proofs/Map.lean` reads as a one-line-per-file
summary of the design trade itself: _the two proofs differ exactly
where the two encoders do._

== Stage C: merge completeness

`merge_completeE` (`Weave/FinalE.lean`): when the witness schedule is
consumed to its end, every process's trace has been consumed
completely — nothing remains pending anywhere. #kernel Consequently
every event of every process occurs in the schedule, and τ restricted
to any one process's trace is monotone (`trace_monotoneE`): the
schedule is a _completion_ of the whole session, and τ is a total
potential over its events. The argument is an argmin in miniature,
run over the drained final state: if some process retained a pending
event, rank the offenders by schedule position and examine the least;
its blocking event belongs to some process strictly earlier in τ that
must itself be blocked — descending past the least offender,
contradiction. Notably, this stage is where the campaign discovered
that completeness is _placement-independent_: once Stage B supplies
edge-respect, the same argument text serves both corners, and the
`d5`/`.impl` versions differ only in which witness they mention.

== Stage D: the decode

`Proofs/PendingE.lean`: in every reachable state, every process is
either finished or sits _at_ a well-defined position of its trace —
its performed events are exactly a prefix, and its unique _pending_
event carries precisely the current count of its channel-side.
#kernel

What makes this stage remarkable is what it does _not_ contain: an
induction over reachability. The session's inductive invariant
(`Invariant.lean`, established once by preservation over every action)
records, for each walk, a _mirror_ of its committed choice: the fact,
implied by the commit guard, about which of the walk's other
operations must already have fired. Under `.impl` the `d6` mirror says
the committed parent summary's every predecessor within the scope is
already fired. That pins the
performed set to a prefix _statically_: the decode is a case analysis
over the invariant's fields, not a new induction. The `d5` decode has
the same shape with the mirrors reversed — and, once more, the
parent-last direction is the cheaper one: the `.impl` walk decode
replaces a 275-line analysis of "where in the chunk did the parent
splice land?" with a two-case split at the scope tail.

== Stage E: the argmin

`Sched.progress` (`Proofs/EndgameE.lean`): a reachable, non-terminal
state can step. #kernel The argument is four sentences. Collect every
process's pending event — non-terminality makes the pool non-empty —
and take $e^*$, the τ-least. Every event that must precede $e^*$ sits
strictly below it in τ. For a receive, that predecessor is the
matching send, by the message edge. For a send into a bounded channel,
it is the slot-freeing receive, by the back-pressure edge. Suppose
some predecessor were unperformed. It would sit at or above its own
process's pending event, which would then rank below $e^*$ —
contradicting minimality. So every predecessor of $e^*$ is performed; flow
conservation on its channel then puts a datum (respectively, room)
where $e^*$ needs it; and by the decode, $e^*$'s owner stands exactly
at $e^*$, so the enabled operation is the owner's very next. If
instead the pool is empty, every process has fired all its sends, and
the end-of-stream close operations cascade to `terminal`.
`Sched.deadlock_free` is then immediate: a reachable stuck state
would be non-terminal with no enabled operation, contradicting
`progress`.

== What was hard, and where it went

A fair summary of the proof's economy: Stages C–E are short and were
largely written once, family-parameterized, and instantiated twice.
Stage B is where the ~36,000 lines live — the counting algebra,
numbering, alignment between the schedule and the walks' worklists,
and the master induction's rolling context — and within Stage B, the
capacity hypothesis is spent at exactly one site class. A reader who
wants the full texture of that machinery should enter through
`Proofs/Map.lean`, which orders the modules for reading and tables
the `d5`/`.impl` correspondence file by file.

= What to trust, and why <trust>

The complete trust ledger of the artifact:

+ *Kernel-checked* #kernel — both flagship-and-counterpart theorem
  pairs, every lemma below them, and every negative control
  (`Control.parentTrap_not_deadlockFree`,
  `Control.jam_not_deadlockFree`,
  `Control.pyramid1_not_deadlockFree`), each on `propext`,
  `Classical.choice`, `Quot.sound` only — no `sorry`, no
  `native_decide`. The hypotheses are non-vacuous by kernel-checked
  witnesses (`wellFormed_satisfiable`, `reachable_init`), and the
  claim is conservatively shaped: an accidental omission from the
  step enumeration could only make deadlock-freedom _harder_ to
  prove, not easier (`Statement.lean`, "Conservativity notes").
+ *Executable, gate-pinned* #gate — that the Lean definitions
  transcribe the specified machine. The evidence: schedule-definition
  equality against an independent implementation; witness replay to
  completion under both modes; adversarial drain assertions at and
  below the margin; the capacity boundary matrix; and the conjecture
  that `schedulable` coincides with acyclicity of the event DAG (the
  dependency graph over all sends and receives; the narrative defines
  it), checked in both directions. All of it runs as a 300-seed sweep
  on every def-touching commit.
+ *Assumed, named* #assumed — exactly two items.
  _Capacity monotonicity_: the theorems fix walk channels at
  capacity 1 and the assembler at `capLevel`; production only widens
  channels, and that widening cannot introduce a deadlock is argued
  (fixed per-walk order makes each process I/O-deterministic; in such
  process networks added buffering only relaxes back-pressure) but
  not proven. _Modeled-world premises_: conforming error-free peers,
  single-producer/single-consumer channels, sequential scopes per
  walk, per-channel in-order delivery — each anchored to the Rust
  structure that realizes it (the model's module docs name each
  anchor), the last also checked by the trace validator's radix-order
  rule.

Nothing else is assumed. In particular, no fairness: the scheduler may
be fully adversarial forever, and the theorems still hold.

= A reader's map: act one

For the reader who wants to go deeper, in reading order:

- `formal/lean/StreamingMirror/Statement.lean` — the audit surface:
  what is claimed, in ~220 lines of definitions chosen to be read.
- `formal/lean/StreamingMirror/Skel.lean` and `Model.lean` — the model
  of record, with the specification in their module docs: the skeleton
  abstraction, the channel graph, the obligation machine, the ledgers.
- `formal/lean/StreamingMirror/Controls.lean` — the negative controls:
  every trap in this act as a kernel-checked stuck run.
- `formal/lean/StreamingMirror/Proofs/Map.lean` — the proof map: the
  five stages file by file, and the `d5`/`.impl` mirror table.
- `lake exe eventdag` — the executable gate; its output is the
  pinned-matrix and boundary evidence this act tags #gate.
- The companion narrative (`formal/doc/narrative.typ`) — the faithful
  history of how both campaigns actually unfolded, including the
  parts this exposition compresses.

#v(4pt)
#heading(numbering: none, level: 1)[Act two — one channel]

= The question <mux-question>

The theorems of act one carry a premise so structural it is easy to
read past: the wire streams between the two parties are
_independent_ — a full or slow stream never prevents another from
delivering. (One stream per comparison stage. The walk steps down two heights at
a time, so a depth-32 trie has sixteen interior stages; the leaf level
adds one more stream — seventeen.) The deployed system honors that premise with the `Link`
transport contract: a remote connection must supply genuinely
non-interfering streams (QUIC streams, HTTP/2 streams, separate TCP
connections). The contract exists because its absence had already been paid for.
An earlier remote transport muxed all seventeen streams over one
pipe, and wide trees deadlocked it. The shape, in one sentence: a walk
waited on a deep answer that sat buried behind bulk provisions in the
pipe, behind a demultiplexer blocked on a one-slot handoff whose
consumer was that same waiting walk. The cycle reproduced identically
at 64-byte and 16-megabyte transport buffers; it was never about
buffer capacity.

The owner of this codebase conjectured that the deadlock was
fundamental, and posed it precisely. Freeze the message set — no new
frames, no credits, no acknowledgments; the mux may only _reorder_ what
the protocol already sends. Call a scheduler _local_ if its every
decision is a function of information in its party's causal past: the
party's own tree, plus everything it has observed on the wire so far,
and nothing held by the remote party that has not yet reached it. The
conjectures: *(C1)* for every pipe capacity, every pair of
deterministic local schedulers deadlocks on some tree pair the
protocol itself can synchronize; *(C2)* a deadlock-free send order
computable from _both_ sides' dispute structure exists — an oracle —
but is necessarily dependent on information not locally available, and
so is unrealizable.

Both conjectures were settled in Lean, on top of act one's artifact,
and neither survived in the form posed — each resolved into something
sharper. This act presents the results in their logical order. First
the model (@mux-model). Then the three answers: the impossibility that
is true (@answer-wc), the possibility that refutes C1 as posed
(@answer-sigma), and the oracle that proves C2's existence half in a
stronger-than-conjectured form (@answer-oracle). Then the window
generalization an implementation would rest on (@window), and the
engineering consequence (@consequence). The
statement of record for everything in this act is
`formal/lean/StreamingMirror/Mux/Statement.lean` — like act one's, it
restates every claim inline and proves each by citation, so the audit
surface cannot drift from the theorems.

= The mux, modeled <mux-model>

The mux model wraps act one's machine without touching it. Per
direction, one bounded FIFO _pipe_ of capacity $C >= 1$ (denominated
in messages — a boundary discussed in @trust2) replaces the seventeen
independent wire channels; the base model's wire cells become the
receiver's per-stream demultiplexer slots. The sender's _strategy_
chooses which enabled wire send enters the pipe next — under the same
committed choice as act one: a pushed frame cannot be retracted. The
demultiplexer delivers the pipe head into its stream's slot, and
blocks — head of line — while that slot is full. Everything
intra-party is exactly act one's machine; cross-process interleaving
remains fully adversarial. Deadlock freedom is as before: every
reachable state of the composition can step or is terminal.

A strategy is a function from the party's _observation_ — its own
structure plus the trace of its pushes and the frames delivered to it
— to its next push (or to idling). Three classes of strategy organize
everything that follows:

- *Work-conserving*: whenever the pipe has room and some send is
  enabled, the strategy must push something. Every eager mux —
  including the one that deadlocked in production — is in this class.
- *Local* (the charter's sense): decisions computable from the causal
  past, as defined above. Formalized _by construction_: the strategy
  of record consults the skeleton only through an _announced view_,
  reconstructed from the party's own structure plus delivered frames
  (@answer-sigma).
- *Window-disciplined* (the implementation's class): push only frames
  within an advertised per-stream window of inferred consumption
  (@window).

= Answer one: eagerness is fatal <answer-wc>

The true impossibility needs no locality hypothesis at all, and that
is its content: knowing everything does not help a scheduler that may
not wait.

*The theorem* (`wc_impossibility` #kernel): there is one fixed dispute
skeleton — the _wedge_: a root with fan seven, its first child
disputed two levels deep, six whole-subtree provisions queued behind —
on which _every_ pair of work-conserving strategies deadlocks, at
every pipe capacity $C >= 1$. The skeleton is realizable by a concrete
tree pair, pinned by a Rust bridge test that runs the real session and
decodes the wedge shape from its trace.

The proof is notable for what it does not contain. There is no
counting argument about capacity, and no information-theoretic fooling
of the scheduler. On the wedge, the protocol's own ordering ledgers
funnel any work-conserving scheduler down a corridor in which _every
decision point offers exactly one legal push_. All strategies in the
class — omniscient ones included — therefore walk the same forced run,
and the theorem is a replay of that run to a stuck state,
kernel-checked. The corridor's end is the production stall in
miniature: the handoff slot holds a provision its consumer will not
take until a deeper dispute resolves, and the frame that would resolve
it sits behind further provisions in the pipe. Burial, under
commitment. The
jam mechanism is act two's echo of act one's borrowed slots: a
one-slot demultiplexer handoff occupied by a frame its consumer will
not take yet, and the frame it needs buried behind that occupant in
FIFO order. Capacity never enters. Kernel anchors at $C in {1,2,3}$ plus one
_capacity-blind_ certificate — a final stuck state that is stuck for
the same reason at every capacity, because the jam lives in the
one-slot handoff rather than the pipe — cover every $C >= 4$. That is
the formal restatement of the production observation that the stall
was identical at 64 bytes and 16 megabytes.

Two negative controls pin that each hypothesis earns its keep.
#kernel A hand-built _idling_ strategy completes the wedge: work
conservation — the right to refuse the pipe — is the entire frontier.
And an unbounded-slot demultiplexer lets even the jamming scheduler
complete: the bounded per-stream state is load-bearing. Elasticity is
a cure, and @consequence spends it deliberately.

The generalization (`wc_impossibility_K` #kernel) closes the obvious
escape: give the demultiplexer $K$-deep parking per stream and the
burial just needs a wider wedge — for every fixed parking depth (per
direction, kernel-anchored at depths one through three and derived
beyond), a scaled wedge defeats every work-conserving pair at every
capacity. Parking depth is mitigation, not cure. The cure is either
elasticity or the right to idle — which is answer two.

= Answer two: patience and local inference suffice <answer-sigma>

C1, as posed, is false — and the witness is the theorem pair at the
heart of this act #kernel:

- `sigmaStarCausal_deadlock_free`: the strategy σ\*-causal, composed
  with itself, is deadlock-free on every skeleton in the theorems'
  class, at _every_ pipe capacity $C >= 1$; and it completes
  (`mux_terminating` supplies the bounded-step half).
- `sigmaStarCausal_charterLocal`: σ\*-causal is local in exactly the
  charter's sense — proven definitionally, because the strategy's one
  skeleton input _is_ the announced view.
- `c1_charter`: the conjecture's formal statement, refuted by that
  witness, unconditionally.

The strategy is _demand-lockstep with inference_. It maintains, from
its causal past alone, a growing certificate set: which of the peer's
consumptions are _certified_ (announced by frames already delivered)
or _inevitable_ (derivable — everything the peer still must do before
that consumption needs nothing further from this side). It pushes a
frame only when the consumption of that stream's previous frame is in
the set; otherwise it idles and lets the reverse traffic grow the set.
The liveness proof's engine is a coverage theorem: at any drained
stuck candidate, every event that precedes the missing push in act
one's witness order τ enters the causal certificate set by its own
position in that order — so the strategy would have pushed,
contradiction.

== Where the announcements live

The deepest finding of the whole campaign is _what "announced" turned
out to mean_, and it deserves the slow build-up.

Act one's model erases payloads, soundly: the protocol machines'
behavior depends only on each child's dispute class, so the model
keeps the classes and drops the bytes — by moving the classes out of
the messages and into the ambient skeleton that every definition
quantifies over. For act one that is a pure economy. But a mux
scheduler is not a protocol machine: it must _reconstruct_ the
skeleton from what it observes, and the erasure had quietly deleted
the classes from the observation channel. Ask precisely where the
sentence "the peer's reply announces that child 3 is disputed" lives
on the wire: not in the arrival _pattern_ — a reply frame landing on
stream $h$ looks identical whichever children it disputes — but in the
frame's _contents_. And the pattern-only substitute genuinely fails:
the discriminating counts accrue only as descents unfold, too late,
permanently — the erased-trace surrogate strategy provably starves on
the very wedge σ\*-causal completes. The announcements were never in
the message pattern; they were in the messages.

So there are three candidate grains for "what a local scheduler may
see," ordered by _when the peer's classes arrive_. The two wrong ones
err in opposite directions:

+ *In the view* — the strategy is born knowing classes it was never
  sent: too early, a causality violation dressed as locality (this was
  a real defect found in the campaign's first locality encoding, and
  the reason the final one is built the way it is).
+ *Never* (pattern only): starvation, per the surrogate above.
+ *At arrival* — each class becomes visible when the frame carrying it
  is delivered and decoded: exactly the causal past, and exactly what
  a real implementation sees, since the receive path decodes every
  frame on arrival.

The two wrong grains being wrong in opposite directions has a
consequence worth stating: the middle grain and the first are
_incomparable_ classes (kernel-pinned in both directions), so claims
about one never transfer silently to the other — the audit surface
names which grain every locality statement binds. And the finding
promotes one Rust bridge from supporting to constitutive. The
announced-skeleton reconstruction test — B5, the fifth of the
campaign's numbered Rust bridge tests — decodes a session's frame
transcript (contents, not pattern) and checks it determines the
skeleton. That is precisely the fact σ\*-causal's locality stands on.

What σ\*-causal is _not_ is fast at capacity one: its pacing is the
subject of the window generalization (@window), which is where the
practical system lives.

= Answer three: the order already existed <answer-oracle>

C2 conjectured that an oracle — given both sides' full dispute
structure — could emit a deadlock-free send order, and that the order
would be unrealizable locally. The existence half is true in a form
stronger than conjectured, and the right way to present it is as a
_recognition_, not a construction.

Act one's proof already contains a distinguished object: τ, the
witness schedule — a concrete, kernel-validated linearization of every
operation of the session, built to prove the independent-channel
theorems, long before these conjectures were posed. The oracle is
_that object's send log_: filter τ to one direction's wire sends and
push frames in exactly that sequence. A fixed list, computed once from
the two trees, no feedback, no adaptation; live at capacity one, on
every skeleton in the class (`oracle_deadlock_free` #kernel).

Why the send projection works is the instructive half. The order is
_feasible by construction_: it is the order in which an actual
(modeled) execution produced the frames, so every production
constraint — every intra-party dependency on the sending side — is
already satisfied when the list demands a frame. The tempting
alternative fails precisely there: pushing in the order the _receiver_
will consume (the receive projection of the same τ) jams on an
eleven-scope counterexample, kernel-pinned (`static_oracle_jams`
#kernel), because a consumption-friendly order can demand a frame
whose producer is parked behind a frame the order postponed. The
receiver's slots absorb the sender-side skew; nothing absorbs
producer infeasibility. An earlier adjudication in the campaign had
those two projections exactly backwards — the executable tier caught
it before the kernel did, which is the campaign's methodological story
in one sentence.

The necessity half of C2 lands class-relatively (`necessity`
#kernel). The oracle order is not computable from any single party's
view: the projections differ across view-equal skeletons
(`oracle_not_local`). Under work-conservation, nonlocal information
cannot save you anyway (@answer-wc). But because of answer two,
nonlocal information is _not necessary for liveness at all_. What the
conjecture's intuition was tracking is now stated exactly: what
credits (or an oracle, or true channel independence) buy over local
inference is _computation and timing_, never information the protocol
withholds. The inference σ\*-causal runs is exactly the credit
stream, derived instead of transmitted.

= The window dial <window>

The theorem any single-channel implementation would actually rest on
generalizes answer two from lockstep to windows. Each receiver
advertises, per direction, a parking depth $K$; each sender may run
$K$ frames past _inferred_ consumption per stream. Two facts make this
the deployable point in the design space:

- *Liveness is order-free within the discipline*
  (`sigmaStarK_deadlock_free`, `sigmaStarK_completes` #kernel). The
  quantifiers: every skeleton in the class, every capacity, every pair
  of advertised depths $K_I, K_R >= 1$ — independent, so unequal peers
  interoperate — and _every_ strategy pair in the window-disciplined
  class, meaning any scheduler that pushes _some_ licensed frame when
  one exists. All of them are deadlock-free and complete in bounded
  steps. The class quantification is the point. The shipped
  scheduler's priority ladder is a proven _instance_
  (`sigmaLadderK_windowDisciplined`), and frame ordering is thereby
  demoted from a correctness concern to a latency heuristic. You
  cannot pick a wrong order, only a slow one.
- *The latency law is exact at the evidence tier* #gate. Call a
  _fresh-dispute frontier_ the scopes at one height whose dispute
  classes the peer's replies are announcing for the first time — the
  places where the sender's inference must wait for evidence rather
  than derive it. Pacing on such a frontier is $K + 1$ frames per
  round trip per stream, and completion matches independent channels
  _exactly in round trips_ once $K$ exceeds the widest frontier's
  width — and the shipped default window is $"fan"^2$ scopes
  ($256^2$, the deployed configuration's advertisement), above any
  frontier a realistic divergence produces. Below that, the residual
  is hyperbolic in $K$: no cliff. (Validated probe-exact on all 54
  cells of a nine-shape, six-depth sweep by the campaign's timed
  harness, with the two ends of the dial reproducing known ground
  truth — $K = 1$ recovers demand-lockstep's measured pacing, large
  $K$ the independent-channel baseline.
  Latency claims are deliberately not theorems — the model is untimed;
  a chartered follow-on campaign owns that.)

This theorem was also the campaign's methodological capstone: its
English statement was fixed _before_ construction, clause by clause,
each clause carrying the audit rule naming the weakenings that would
gut it — single-window, concrete-scheduler-only,
omniscient-inference, progress-without-termination. The landed
theorem's crosswalk grades every clause EXACT, and the discipline —
specification first, statement graded against it — is now house
method.

= The engineering consequence <consequence>

Begin with a framing this act has so far let the reader supply
incorrectly: the product here is the _library_. `rumormill`, the
demonstration gossip daemon built on it, is exactly that — a
demonstration. The surface a user of `rumors` actually holds is the
`Link` transport contract. The library _requires_ independent streams
while shipping none; the user discharges the requirement with whatever
their environment already tunes best (QUIC streams, HTTP/2 streams,
separate TCP connections). Every consequence
below is a statement about what that contract should require, not
about any deployment.

The trichotomy prices the transport design space completely, and the
price list reads differently than the original deadlock suggested:

- The production deadlock was never about capacity, muxing, or missing
  information. It was about _eagerness_ — a scheduler denied the right
  to idle (@answer-wc) — against bounded per-stream parking.
- Liveness over one channel needs either elasticity or
  inference-gated sending at any window (@window). Elasticity means
  unbounded parking, proven as `elastic_deadlock_free` #kernel — and
  parking is cheaper than it sounds. The receiver can decode each
  frame on arrival and stream a provision's payload straight into its
  tree store, which was that data's destination anyway; a parked reply
  then costs a small descriptor, not buffered bytes. Neither route
  needs a wire byte the protocol doesn't already send.
- Round-trip parity with independent channels is reached at the
  default window #gate. What independent channels still buy is
  physics: per-stream loss isolation and packet-granularity
  interleaving under bulk transfer — real, and now the _only_ items on
  their side of the ledger.

One further reading of the suite, weaker in tier but decisive in
consequence, deserves its own paragraph. The kernel results bracket a
_mechanism_ claim without yet pinning it. Any live local scheduler
must withhold (@answer-wc). It must withhold on information: the
wrong fixed order dies with full knowledge (`static_oracle_jams`).
The information must come from frame _contents_, because pattern-only
inference starves (@answer-sigma). It must be inferred from
announcements, since the omniscient license is not locally computable
(`oracle_not_local`). And the announcement-inferred window discipline
suffices (@window). Every proven constraint points
at one mechanism: a correct single-channel scheduler effectively
re-implements per-stream windowing from application-level signals —
tracks the trace, infers each receiver buffer's occupancy, and imposes
backpressure on itself. That this is _necessary_ and not merely
sufficient is at present a derived claim #assumed, chartered
spec-first as T11 — the campaign's last numbered theorem target — the
forced-window theorem:
every charter-local strategy that is deadlock-free on the class is
license-bounded at every reachable observation.

Read as a product decision, the bracket settles the question the
campaign was chartered to inform. The choice was never windowing
versus no windowing; it is _whose windowing implementation_: a bespoke
inference engine — correctness session-fatal at the window boundary,
tuned by nobody — or transports that have had their decades of tuning
and carry the two physical properties no inference can recover. The
one corner where the bespoke engine genuinely wins (silent provision
runs pipelining without credit round trips) lives in a bandwidth-bound
regime where the win is invisible. The conclusion of record: *the
`Link` contract stands as the product surface.* Not theorem-forced —
the campaign proved the alternative exists and priced it exactly — but
theorem-backed: we verified the alternative, identified what it must
implement, and chose the tuned implementation of that same mechanism.

The single-socket design — maintained, with its code-grounded
executable plan, on the `single-connection` branch — is thereby the
_contingency of record_, not a successor. It is finished, shelved,
and theorem-backed, serving exactly the library user whose environment
cannot supply multi-stream transports. Window depths ride the
_greeting_ (the session's opening handshake). The sender's engine is
the inference of @answer-sigma at the window of @window. Over-window
arrival is a conformance violation that attributes cleanly to the
sending side's bug. And any window-obeying frame order is valid. Its
final
stage — removing `Link` — sits behind a gate now expected never to
fire, and that is the design working as intended.

The campaign's last symmetry is worth stating plainly. The
impossibility instinct that opened it was right after all — not about
deadlock-freedom, where the local scheduler won, but about mechanism:
there was never any escape from flow control, only the choice of who
implements it.

= What to trust, and why: act two <trust2>

The trust ledger, act two:

+ *Kernel-checked* #kernel — the entire suite behind
  `Mux/Statement.lean`: `c1_charter`, `c1_omniscient`,
  `wc_impossibility`, `wc_impossibility_K`,
  `sigmaStarCausal_deadlock_free`, `sigmaStarCausal_charterLocal`,
  `oracle_deadlock_free`, `necessity`, `elastic_deadlock_free`,
  `mux_terminating`, `sigmaStarK_deadlock_free`,
  `sigmaStarK_completes`, and every control pinning a hypothesis
  (the idler, the unbounded-slot completion, the static-oracle jam,
  the evidence-only starvation, the locality-grain pins) — each on
  the three standard axioms; no `sorry`, no `native_decide`.
+ *Executable, gate-pinned* #gate — three instruments. `lake exe
  muxprobe`, the mux twin of act one's gate: a 300-plus-row golden
  matrix over pinned and randomized skeletons, strategies, capacities,
  and window depths, with scans asserting every commit decision along
  the matrix was forced rather than chosen. The stage-0 causal sweep —
  stage 0 was the build plan's one blocking gate — with 4,970 runs
  under a structurally-blinded strategy. And the timed harness behind
  the latency law.
+ *Assumed, named* #assumed — the model boundary. First,
  message-denominated capacity: a "reply" is byte-unbounded, and
  byte-level liveness additionally needs byte pacing. Impossibility
  results transfer to bytes a fortiori; positive results do not; every
  positive statement's docstring carries the caveat. Second, the
  bridge premises tying model to Rust: the trace validator's ledgers,
  the wedge realizability test, and B5 — which the payload finding of
  @answer-sigma promotes to constitutive.
  A chartered trace validator — Rust proptests replaying their
  traces through the compiled Lean definitions — exists as a plan to
  shrink this category mechanically.

= A reader's map: act two

- `formal/lean/StreamingMirror/Mux/Statement.lean` — the audit
  surface: every claim of this act, restated inline, proof by
  citation; each entry's docstring carries its provenance and its
  load-bearing controls.
- `formal/lean/StreamingMirror/Mux/Causal.lean` and `SigmaStarK.lean`
  — the strategies of record, with the announced view and the
  licensing discipline in their module docs.
- The controls — `Mux/Controls.lean`,
  `Mux/Proofs/Oracle/Controls.lean`, and the refutation ledger in
  `Mux/Proofs/C1.lean` — every hypothesis pinned load-bearing by a
  kernel-checked witness.
- `formal/lean/StreamingMirror/Mux/Proofs/Map.lean` — the proof map
  for this act: the invariant family, the transport dial, the
  discharge map.
- `lake exe muxprobe` and `lake exe eventdag` — the executable gates
  behind every #gate tag; `lake build` re-certifies the audit surface
  against the theorems.
- The companion narrative (`formal/doc/narrative.typ`, part two) —
  how all of this actually happened, including the reversals this
  exposition presents as if they were always known.
