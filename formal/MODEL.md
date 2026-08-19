# The abstract model of the streaming mirror protocol

This is the design document for the formal deadlock-freedom artifact. It is
the single specification that `quint/streamingMirror.qnt` (Phase A/B) and the
Lean package (Phase C/D) both transcribe; when the three disagree, this file
is wrong exactly until the disagreement is resolved, and the resolution lands
here first.

The modeled system is the connected-session core of
`src/tree/mirror/streaming/` — the **in-memory driver** (`mirror_connected`,
`streaming/driver.rs`): two conforming parties on the `Local` backend. The
remote transport (`remote/`, running over a caller-supplied `Link`) sits
below this model's abstraction: a remote session drives the same walk,
queue, and assembler machinery modeled here, the model's wire edge is a
bounded in-order FIFO, and capacity monotonicity (§1.iv) covers a
conforming link's larger buffering — while the remote adaptation layer
itself (codec, proxy pumps, stream supply) is outside the model, validated
by the link conformance suite and the trace bridges instead. Structural
claims below were extracted from the Rust (2026-07-15, re-audited
2026-07-23) and adversarially cross-checked; the load-bearing citations
are repeated here as durable anchors (file and item — the 07-15 line
numbers rotted within days). The one divergence the 07-23 re-audit
flagged (the pairing loops' dequeue order) is resolved by the
dequeue-order class amendment in §5.

## 1. What is proved, what is assumed

**Assumed (the bridge to Rust, proptest-verified there):** the send-order
invariants, encoded *exactly* as the checks of
`progress.rs::Trace::assert_valid` (§6) — the three original ledgers plus
the three checks that **this modeling effort surfaced as missing**:
sibling contiguity (D3, 2026-07-15: the original three admitted a
"publish all wires, then all resolutions, then all queries" implementation
that passes `assert_valid` and deadlocks the cap-1 child-resolution queue
at fan ≥ 3 — the `ledgerGap` instance), wire contiguity (D4, finding #6,
2026-07-16), and parent placement (finding #7, 2026-07-17 — resolved
2026-07-18 as a two-corner design space: D5 is the weave's parent-early
discipline, D6 the shipping encoder's epilogue discipline; §6 and
`.agent-notes/2026-07-18-parent-placement/parent-placement.md`).
`assert_valid` is tightened alongside each finding so that the checked
invariant and the assumed axiom set coincide — for finding #7 the check is the D6 (epilogue) form, the
order the encoder actually has. The Rust's hard-wired
`yield_resolve_query!` order is one refinement of the guarded-
nondeterministic publication relation; the theorems cover every
implementation whose traces satisfy the tightened `assert_valid`.

**Proved about the model:**

- (i) **Deadlock-freedom** (safety): every reachable non-terminal state has
  an enabled action, over every scheduler interleaving and every
  ledger-consistent publication linearization.
- (ii) **Termination**: every maximal run reaches `Terminal`. No fairness
  hypothesis: every action strictly decreases the remaining-operation count
  ρ, so no infinite runs exist and (ii) is a corollary of (i) (§7).
  Kernel-proven since 2026-07-21 (`Proofs/Termination.lean`, closing
  an audit finding resolved by theorem): `rho_decreases` is the 23-case strict-decrease
  lemma (its one hypothesis, `asmLevelsOk`, is an inductive invariant
  from `init` — see the module doc for why some measure hypothesis is
  unavoidable at ill-formed states), `terminating` bounds every run
  from `init` by ρ(init), `maximal_run_terminal` /
  `maximal_run_terminal_d5` derive (ii) from each progress flagship,
  and `greedy_run_terminal` is the constructive form with the explicit
  fuel bound ρ(init).
- (iii) *Validation property, not a theorem*: the capacity-tightness law
  N ≤ C + 2 for the inter-level return boundary (§8).
- (iv) **Capacity monotonicity** (for the `.impl` flagship): (i) and
  (ii) hold not just at the model capacities (walk channels at 1, the
  assembler at `capLevel`) but at EVERY pointwise-widened capacity
  vector κ ≥ that floor — widen levels to the deployed window, keep
  wires at 1, or any mix. Kernel-proven since 2026-07-21
  (`Sched.deadlock_free_wide` and `terminatingW`,
  `lean/StreamingMirror/Proofs/Wide.lean`): the
  widened transition function `applyW κ` recovers `apply` at κ = floor
  definitionally, and ρ never reads occupancy, so the run bound is the
  floor's. The `d5` corner's wire-widening remains on the informal
  Kahn argument (Statement.lean, "Assumed, not proven").
- (v) **Dequeue-order indifference** (for the `.impl` flagship): (i),
  (ii), and (iv) hold at EVERY assignment of the two-point per-loop
  dequeue-order class (each walk stage and the absorber independently
  reply-first or query-first, closes tied to the prologue choice —
  the shipping Rust is the all-query-first instance). Kernel-proven
  since 2026-07-24 (`Sched.deadlock_free_anyOrder`,
  `Sched.deadlock_free_wide_anyOrder`; the class, its exclusions, and
  the reply-first residue: §5's dequeue-order subsection).

**Explicitly not modeled** (modeled-world premises, each with its Rust
anchor):

- **The error path.** All park-forever sites (`next_or_pending!`,
  `divert`'s park) and all `Violation` aborts are reachable only after a
  fault has claimed the driver's one-slot error channel
  (`driver.rs`, the error route's `try_send`-only `report`). Two conforming peers on `Local`
  (`Error = Infallible`) never reach them, so the model omits them and the
  theorem is genuine deadlock-freedom of violation-free sessions — not
  "deadlock-freedom modulo abort".
- **Backend awaits.** Backend calls precede the scope's wire yield and hold
  no channel endpoints: finite stutter, no wait-for edges. Adversarial test
  delays are self-waking and clamped to 2 polls — schedule permutation, not
  suspension.
- **Payloads.** Hashes, versions, node contents, radices. Sound because the
  count and order of channel operations depend only on each child's
  merge-join arm, never on payloads (verified against every arm of
  `answer.rs` and `resolver.rs`); the one payload-dependent control-flow
  divergence is the violation abort, excluded above.

## 2. The skeleton abstraction

Quantification over tree pairs is replaced by nondeterminism over **dispute
skeletons**: the finite tree of scopes the session actually recurses into.

- Heights: leaves at 0, the root at `ROOT_H` (Rust: 32; the model instances
  use 4 and 6). `ROOT_H` is even; this parity is structural, not stylistic
  (§3).
- A **scope** is a node the two parties recurse on. The root is a scope,
  and always a disputed one: `mirror_connected` is entered only after the
  handshake's version comparison, and equal versions short-circuit the
  session before the protocol begins ("equal only when their versions are
  equal", `streaming/protocol.rs`) — the handshake, the
  short-circuit, and the initiator-selection tiebreak all sit outside the
  modeled function and therefore outside the theorem's claim.
  Each scope at height `h ≥ 2` has an ordered list of children, each labeled:
  - `D` — two-sided dispute (`Reaction::Query(nonempty)`): a recursive scope
    at `h − 1`;
  - `R` — one-sided request (`Reaction::Query(empty)`): a degenerate scope at
    `h − 1` with no children (the answerer of the parent lacks it and asks
    for the whole subtree; the reply is pure supplies);
  - `M` — match or absorbed supply: **zero channel operations on both
    sides**, dropped from the skeleton entirely.
- Each scope at height 1 carries only a count `leafReqs ≥ 0` of leaf
  requests (`answer::leaf_parent` maps `Both` to `Match` unconditionally —
  `work/answer.rs` — so height-0 leaves are never disputed; height-**1**
  scopes *are* disputable, via `answer::internal` instantiated at `H = Z`).
- Fan bound: ≤ F children per scope (`FAN = 256` in Rust, `streaming/window.rs`,
  consumed by the assembler queue constructor in `work/queues.rs`).
- Ids are BFS order, and since 2026-07-16 `wellFormed` checks the
  cross-parent consequence, not just per-scope ascending kids: each
  stage's kid lists, flattened in scope order, ARE the next stage down
  (`wf_bfs_aligned`). The conjunct exists for the progress proof
  (`Sched.progress`), which keys each channel's n-th message to the n-th
  scope of the consuming stage; a crossed-but-otherwise-well-formed
  skeleton stays count-consistent and completes, so this narrows the
  theorem's domain to the documented (and Rust-realized) class rather
  than fixing a defect.

Soundness: honest walks are deterministic given trees, so every real
communication skeleton is a model path; unrealizable skeletons (e.g. shapes
radix-trie path compression could not produce) only enlarge the verified set.
During development, a counterexample must be realizability-checked before
being treated as a Rust bug.

**Directionality note.** `R` is directional by construction: an `R` child of
scope σ is lacked by σ's *answerer* (the merge-join Right arm). A child only
the *asker* lacks arrives as a `Supply` reaction and is absorbed — zero
channel ops — hence `M`. The skeleton does not need a second request label.

## 3. Parties, stages, and the height-parity theorem

Message index convention (from the Rust types): `Query<H>` pairs with
`Reply<H>` where `H` is the *children's* height; the scope sits at `H + 1`
(the `materialized.rs` message types).

Per party, the session is a fixed pipeline of **walk stages** `S(p, h)`, one
per consumed message index `h`, plus an opening, a terminal, assemblers, and
pumps. The schedule (`mirror_connected`'s phase-schedule expansion,
`streaming/driver.rs`) fixes:

- **Initiator** stages consume odd `h = ROOT_H−1, ROOT_H−3, …, 1`;
  it therefore *processes* (asks about) scopes at even heights
  `ROOT_H, …, 2` and *answers* scopes at odd heights.
- **Responder** stages consume even `h = ROOT_H−2, …, 2, 0`; it processes
  odd-height scopes `ROOT_H−1, …, 1` and answers even-height ones.

Every disputed scope is processed by **both** parties in different roles:
the **asker** (whose stage pairs the incoming reply with its own queued
query, in its loop prologue) and the **answerer** (whose stage one level up
expands the dispute inline in its reaction loop). Each party materializes
its own copy of every reconciled scope; that is why each reply stage feeds
*two* assemblers (§4).

"D = 31 is odd" is not quoted anywhere in the Rust; it is *derived* — the
schedule plus `ReplyHeight`'s two-height stride forces the parity map above,
and the parity map is what the model builds in. The Rust pins the counts
("must move together", `streaming/protocol/peer.rs`).

## 4. Processes and the channel graph

All concurrency is cooperative: one Tokio task, futures interleaved at await
points (`try_join` + `select!`; no `tokio::spawn` anywhere in
`src/tree/mirror/`). The model's
interleaving semantics — any one enabled atomic action per step — is a strict
superset, sound for safety.

### Process inventory (per instance)

| Model process | Rust | Program shape |
|---|---|---|
| `IOpen` | `initiator_level` (`work/levels.rs`) | send 1 wire reply (root listing) → send 1 `rootQuery` |
| `ROpen` | `responder_level` (`work/levels.rs`) | recv 1 wire → publish root obligations (§5) |
| `Walk(p, h)` | `internal_level` / `leaf_parent_level` / `leaf_level` (`work/levels.rs`) + its `Work::respond` pump | per scope: prologue recvs → obligation poset → epilogue; then 2 close-recvs |
| `Asm(p, j)` | `assemble` + `return_into` (`work.rs`) | loop: recv resolution → recv 1 level item per `Pending` slot → send the assembled return upward; then 1 close-recv |
| `Absorb` | `absorb` (`materialized.rs`) | per leaf request: recv wire → recv `leafReq` → send return; then 2 close-recvs |
| `IFinish` | `initiator_level`'s tail (`work/levels.rs`) | recv 1 from `rootReturn` |
| `RFinish` | inline root assembly (`responder_level`'s tail, `work/levels.rs`) | recv `rootRes` → recv 1 `rootReturns` item per root `Pending` |

### Stream/driver pairs collapse to one sequential process

Two edges in the Rust are `async_stream` yields polled by a driving loop:
walk → pump (`Work::respond`) and assemble → forwarder (`return_into`).
In both, the stream runs **only while polled**, and the driver polls only
between completed sends (`while let Some(x) = next().await { send(x).await }`).
So stream and driver are strictly sequential — the pair is one process, and
the yield is faithfully a plain **send into the driver's capacity-1 output
channel with blocked-sender-holds-item semantics**: when the channel is
full, the committed send holds one item in hand and the producer's
subsequent operations wait behind it. Jammed-state buffering per such edge
is therefore channel (1) + hand (1); the pump adds no independent third
buffer (an earlier draft of this document claimed it did — the poll
mechanics refute that).

The pump's capacity-1 channel **is** the wire: nothing else sits between
the parties in `mirror_connected` (`divert` is a combinator, not a
channel).

### Channel instances

Channels are bounded SPSC FIFOs; items are opaque, so a channel's state is
its occupancy (fired sends − fired recvs) plus static capacity. Verified
SPSC: no protocol sender is ever cloned; the only cloned sender in the
driver is the error slot's, which is `try_send`-only and adds no wait-for
edges (excluded with the error path).

Per party `p` and applicable height, with the Rust `QueueKind` each
transcribes (channel instances are keyed `(kind, height)` in Rust too —
"14" is the edge taxonomy, not the channel count). Capacities in this
table are the **model floor** the theorems are stated at. The shipping
Rust runs the recursive walk edges (`asked`, `upperRes`, `lowerRes`) at
session-window capacities ≥ 1 (`work/queues.rs`: recursive edges take
their capacity from the session's window), the wire at 1, and the
assembler at `FAN` (its hard floor, not a tunable); every such vector is
covered by capacity monotonicity (§1.iv, `Sched.deadlock_free_wide`).

| Model channel | Cap | Producer → Consumer | QueueKind |
|---|---|---|---|
| `wire(p, h)` | 1 | `Pump(p, h)` → counterparty `Walk(¬p, h−1)`-or-terminal | `OutgoingResponses` |
| `asked(p, h)` | 1 | `Walk(p, h+2)` → `Walk(p, h)` | `InternalChildQueries` (interior), `ResponderChildQueries` (R opening), `LeafRequests` (I leaf-parent → `Absorb`) |
| `upperRes(p, h)` | 1 | `Walk(p, h)` epilogue → `Asm(p, h+1)` | `InternalParentResolutions` / `LeafParentResolutions` / `TerminalLeafResolutions` |
| `lowerRes(p, h)` | 1 | `Walk(p, h)` D-arms → `Asm(p, h)` | `InternalChildResolutions` / `LeafChildResolutions` |
| `level(p, j)` | **C** (= F at Rust defaults) | `Asm(p, j)` forwarder (or `Absorb` at j = 0) → `Asm(p, j+1)` | `AssemblyLevelReturns` |
| `rootQuery` | 1 | `IOpen` → `Walk(I, ROOT_H−1)` | `InitiatorRootQuery` |
| `rootReturn` | 1 | `Asm(I, ROOT_H)` → `IFinish` | `InitiatorRootReturn` |
| `rootRes` | 1 | `ROpen` → `RFinish` | `ResponderRootResolution` |
| `rootReturns` | 1 | `Asm(R, ROOT_H−1)` → `RFinish` | `ResponderRootReturns` |

Assembler chains: on each side, scopes at height j all flow through one
resolution channel, by parity — the initiator's even-height (asked) scopes
through `upperRes`, its odd-height (answered) scopes through `lowerRes`, and
dually for the responder. `Asm(p, j)`'s level input is `level(p, j−1)`;
its output feeds `level(p, j)` — except the top of each chain:
`Asm(I, ROOT_H)` sends into `rootReturn` (cap 1) and `Asm(R, ROOT_H−1)` into
`rootReturns` (cap 1), the responder's root assembly being inlined in
`RFinish`. At the bottom, `Absorb` produces `level(I, 0)`; the responder's
`Asm(R, 1)` (`assemble_leaves`) consumes no level items at all — its
resolutions are `Pending`-free by construction (`TerminalLeafResolutions`,
`work/queues.rs`).

### Resolution `pending` counts (asymmetric by role)

For scope σ with `d` D-children and `r` R-children:

- **asker-side** resolution of σ (epilogue `upperRes`): `pending = d` — the
  asker already possesses R-children's subtree content only when *it* is
  the supplier; an R child of σ is lacked by σ's answerer, so on the asker's
  side it is `Ready`.
- **answerer-side** resolution of σ (D-arm `lowerRes`): `pending = d + r`.
- Height-1 scopes: answerer (always the initiator) has
  `pending = leafReqs`; asker-side (responder, via `leaf_level`) resolutions
  have `pending = 0`.

These counts drive exactly which `level` recvs each assembler performs, and
they are why the initiator alone has leaf `Pending`s (absorb returns) while
the responder's terminal resolutions assemble immediately.

## 5. The obligation machine

Every process is a finite partially-ordered set of **operations**, derived
from the skeleton by a pure function. The entire dynamic state is, per
process, the set of already-fired operation indices; channel occupancies are
derived. One step = firing one enabled operation (a rendezvous fires its two
halves as one step).

Operation kinds and enabling guards:

- `send(ch)` — occupancy(ch) < cap(ch).
- `recv(ch)` — occupancy(ch) > 0.
- `recvClose(ch)` — occupancy(ch) = 0 ∧ the producer process has fired
  **all** its operations (Rust: sender dropped at process end;
  recv-on-empty-closed is the loop-exit branch, a positive protocol
  signal).

**Committed choice (load-bearing).** Publication obligations do not fire
under may-fire semantics. A publisher with no operation in flight
nondeterministically **commits** to one unfired obligation whose active
axiom guards are satisfied at choice time; the committed operation must
then complete (blocking on its channel if full) before the next choice.
This models an arbitrary *sequential* implementation whose trace satisfies
the axioms — which is what the theorem quantifies over. Under may-fire
semantics dropping an axiom never produces a deadlock, because the checker
simply fires the withheld operation; a real program that wrote the sends
in a bad order cannot skip ahead, and commitment is what captures that.
Commitment steps are counted in the run-length bound (§7).

Per-scope structure of a walk stage, for each scope σ it processes in
order (scope order = query order = BFS/radix order of the skeleton):

1. **Prologue** (fixed order, program structure). The model's baseline
   transcription is `recv wire` then `recv asked` — reply first, then
   query, the order the Rust had at extraction. Since fd36bb65
   ("Dequeue query-first in the pairing loops") the shipping walk
   dequeues the query first and then awaits the wire reply
   (`work/levels.rs`), freeing the queue slot one reply earlier so the
   window accounting is exact; the end-of-stream checks flipped with
   it (loop exit on the closed query queue, leftover-reply check
   after). The pairing is unchanged — the k-th query still meets the
   k-th reply — but the prologue's I/O order is not the baseline
   transcription's, and capacity monotonicity does not cover an order
   change. **RESOLVED (2026-07-23, the order-indifference amendment):
   the divergence flagged here by the 07-23 re-audit is closed by
   quantifying the `.impl` theorems over the prologue order itself
   rather than re-pinning the model to one order — the dequeue-order
   class subsection below states the class, the claim of record, and
   the reply-first residue.**
2. **Publication obligations** (the poset the axioms guard): per D child c —
   `send wire(c)`, `send lowerRes(c)`, `send asked(g)` for each child g of
   c; per R child c — `send wire(c)` only; plus one `send upperRes(σ)`
   (the scope epilogue).
3. Within a scope, **each channel's sends fire in child order** (program
   structure, never relaxed; the axioms govern only cross-channel
   interleavings). Positional pairing is the protocol's identity carrier —
   returns are prefix-less and replies pair with the query queue by index
   (the pairing loops of `work/levels.rs`) — so an implementation that
   reorders within a channel
   is functionally incorrect and outside the theorem's scope, and the
   counting abstraction is unsound for it (out-of-order arrivals misbind
   the consumers' positional schedules and manufacture spurious model
   deadlocks). Checked in Rust by `assert_valid`'s radix-order rule.
4. Scope σ+1's prologue requires **all** of σ's obligations fired (walks are
   sequential across scopes — a modeled-world premise slightly stronger than
   the ledgers, which would tolerate cross-scope pipelining; the Rust walk
   is a single sequential loop).
5. After the last scope: `recvClose wire`, then `recvClose asked` in
   the baseline (reply-first) transcription. The query-first member of
   the dequeue-order class closes `asked` then `wire` — the shipping
   loop since fd36bb65 exits on the closed query queue and then checks
   the reply stream for a leftover. The close order is tied to the
   loop's prologue choice, never a separate choice point (the
   dequeue-order class subsection below).

Within step 2 there is **no fixed cross-channel program order** among the
obligations of one scope: beyond the per-channel child order of step 3, the
only intra-scope edges are the axiom guards of §6, switched by `AxiomMode`.
This is deliberately weaker than the Rust (whose reaction loop is totally
ordered): the positive theorem then covers every publication linearization
the trace validator accepts — e.g. wire(i+1) before res(i), or the parent
resolution before the last child's queries — orderings the Rust scheduler
can never produce.

`ROpen`'s obligations are the same shape for the root scope (wire reply,
`rootRes` with `pending = d + r`, one `asked` send per D/R root child), with
one prologue recv (the opening wire message). `IOpen` has two ops:
`wire-yield` then `send rootQuery` — the `InitialQuery` wire-ledger edge.

### The dequeue-order class (§5.1 resolution, 2026-07-23)

Each pairing loop of the session — each walk stage `S(p, h)`, and the
absorber — is a two-receive loop: per scope (per leaf request, for the
absorber) it dequeues one wire reply and one queued query, then (for
walks) publishes. The order-indifference metatheorem quantifies over
the **two-point per-loop dequeue choice**:

- **reply-first** (`PairOrder.replyFirst`): recv wire at phase 0, recv
  asked at phase 1; end-of-stream closes wire at phase 3, asked at
  phase 4 — the baseline transcription of §5.1/§5.5;
- **query-first** (`PairOrder.queryFirst`): recv asked at phase 0,
  recv wire at phase 1; closes asked at phase 3, wire at phase 4 — the
  shipping order since fd36bb65. The close order is **tied** to the
  loop's prologue choice (the Rust loop exits on whichever queue it
  dequeues first going closed); the model does not treat closes as a
  separate choice point.

An `OrdMap` assigns one `PairOrder` to every walk stage and one to the
absorber, each independently — 2^(#stages + 1) assignments per
skeleton. `applyO` is the ord-parameterized transition function;
`applyO_rf` pins its all-reply-first instance to `Model.apply`
definitionally, so the baseline theorems are the `ord = .rf` instances
of the quantified ones and the shipping Rust is the all-query-first
instance.

**Explicitly outside the class** (loop shapes that are not a two-point
dequeue choice; none is the shipping Rust, and none is covered):

- racing both inputs (a `select!` over reply queue and query queue,
  order resolved per message at runtime);
- cross-scope prefetching (dequeuing scope k+1's inputs before scope
  k's obligations complete — the §5.4 sequential-scope premise, still
  in force);
- a prologue interleaved with the scope's sends (both receives always
  precede every publication of the scope).

**Claim of record** (fixed 2026-07-23, before any Lean transcription;
kernel-proven 2026-07-24): under `wellFormed` and margin 0
(`∀ σ, dCount σ ≤ capLevel`), every assignment's `.impl` session is
deadlock-free and terminating, at the floor capacities and at every
pointwise-widened capacity vector κ ≥ floor —
`Sched.deadlock_free_anyOrder` (Ord/Endgame.lean),
`Sched.deadlock_free_wide_anyOrder` (Ord/WideEndgame.lean),
`Ord.rho_decreasesO` / `Ord.terminatingO` (Ord/Termination.lean),
each at kernel axioms `[propext, Classical.choice, Quot.sound]`; the
audit surface is Ord/Statement.lean. This is per-loop dequeue-order
indifference over the two-point class above, NOT arbitrary-order
indifference: nothing is claimed for the excluded loop shapes.

**Reply-first residue** (each theorem below stays certifying the
baseline reply-first order only; dated notes, deliberate scope
decisions of 2026-07-23, not gaps discovered later):

- `Sched.deadlock_free_d5` (2026-07-23): the parent-early corner is a
  design-space record, not the shipping encoder; its proof chain (the
  spliced weave, the AscCover/DescSupply telescopes) stays reply-first
  and no claim is made about a query-first `d5` corner.
- The mux layer (2026-07-23): `wc_impossibility`,
  `wc_impossibility_K`, `sigmaStar_deadlock_free`,
  `sigmaStarK_deadlock_free`, `sigmaStarCausal_deadlock_free`,
  `oracle_deadlock_free`, `elastic_deadlock_free`, `mux_terminating`,
  and their kin build on the baseline `Model.apply` and stay
  reply-first.

## 6. The axioms: the ledgers of `Trace::assert_valid`

The axiom guards transcribe the checks of `progress.rs::Trace::assert_valid`
*exactly* — the ledgers, not
the module doc's prose, are the assumed interface, because they are what the
Rust proptests enforce on every scheduled run. Model obligations correspond
to trace events as: `wire-yield` ↦ `Wire`, `send lowerRes(c)` /
`send rootRes` ↦ `Resolution`, `send asked(g)` ↦ `DependentWork` (recorded at
g, charged to parent(g) = c), `send upperRes(σ)` ↦ `ParentResolution`,
`send rootQuery` ↦ `InitialQuery`. (`Ready` events need no model op — an R
child's only operation is its wire yield, so its wire-ledger edge is
trivially ordered.)

- **Axiom W (wire ledger).** Per (party, scope): internal publications
  consume wire credits 1-for-1. Model guard: `send lowerRes(c)` requires
  `wire-yield(c)` fired; `send rootQuery` requires `IOpen`'s yield fired;
  `send rootRes` requires `ROpen`'s yield fired. The ledger's bijection
  (end-of-trace drain) holds by construction here — each scope has exactly
  one wire op and at most one publication op.
- **Axiom D1 (dependent ledger).** A child query fires only after its
  scope's resolution: `send asked(g)` requires `send lowerRes(c)` fired
  (g a child of c); root queries require `rootRes` fired. Cardinality
  (exactly `pending` dependent works per resolution) holds by construction:
  the skeleton mints exactly one query per D/R child.
- **Axiom D2 (lower ledger).** A parent resolution declaring N pending
  requires ≥ N prior child-scope resolutions: `send upperRes(σ)` requires
  `send lowerRes(c)` fired for every D child c of σ. (The *checked*
  property is child resolutions — weaker than the dependent-work
  launches prose might suggest; the model assumes only the checked
  property, making the theorem stronger. R children deposit their lower
  ledger credit on the *answerer's* side; on the asker's side σ's
  `pending` counts only D children, whose asker-side resolutions are the
  D-arm `lowerRes` sends of the same walk — the guard above.)

- **Axiom D3 (sibling contiguity).** A child's resolution may be chosen
  only once every already-resolved sibling of the same scope has sent all
  its dependent queries. This axiom was **not** in the original plan: the
  model surfaced it. The three ledgers above are per-scope and never order
  child i's queries before child i+1's resolution, so a "all wires, all
  resolutions, then all queries" implementation satisfied them and
  deadlocked the cap-1 `lowerRes` channel at fan ≥ 3 (`ledgerGap`
  instance — the one-slot floor of the child-resolution edge is
  sufficient only under this axiom, which the queue docs' sufficiency
  argument implicitly relied on). The Rust
  enforces it syntactically (`yield_resolve_query!` + the sequential
  reaction loop), and `Trace::assert_valid` now checks it (the
  sibling-contiguity rule, added 2026-07-15 as a result of this finding).

- **Axiom D4 (wire sibling contiguity).** A child's wire may be chosen
  only once every earlier D sibling of the same scope is resolved and has
  sent all its dependent queries. Like D3, the model surfaced it —
  finding #6 (Phase C, 2026-07-16): D3 polices the *resolution* stream
  but nothing ordered child i's queries before child i+1's **wire**, so a
  publisher whose wire stream runs ahead of its query stream satisfies
  {W, D1, D2, D3} and deadlocks a three-walk wait cycle at uneven fan
  (`lean/StreamingMirror/Controls.lean`: walk (R,2) sends wires B1, B2
  with B1's queries 1-of-4 done and commits wire B3, which jams behind an
  unconsumed wire; the walk two stages down starves for the second asked;
  the walk between them jams its fourth wire — kernel-checked stuck run,
  `Control.jam_not_deadlockFree`). The Rust enforces contiguity
  syntactically (the same `yield_resolve_query!` expansion whose doc
  calls it "progress-critical order"), and `Trace::assert_valid` now
  checks it (the wire-contiguity rule, added 2026-07-16 as a result of
  this finding). Note that with W and D1, D4 forces the per-scope
  publication order to be essentially the macro's own order — it subsumes
  D3, which is kept as defense-in-depth.

- **Axiom D5 (parent placement).** Once every D child of a scope is
  resolved, the parent resolution must be sent before any further wire or
  query of that scope — and first, when the scope has no D children at
  all. Like D3 and D4 the model surfaced it — finding #7 (Phase C,
  2026-07-17, the parent-delay finding): {W, D1, D2, D3, D4} never forced
  the floating parent out, so a publisher could commit a last-chunk query
  or trailing W wire with the parent unsent; the unsent parent starves
  the assembler two heights up, the level towers back up and stop
  draining the walk's own `upper` channel below, and the walk two stages
  down wedges on its parent fire — a commit/back-pressure cycle
  (`lean/StreamingMirror/Controls.lean`: `Control.pdelay`, a kernel-
  checked stuck run on a well-formed, **schedulable** skeleton —
  `Control.parentTrap_not_deadlockFree` refutes the pre-finding target
  statement itself). D5 is the *weave's* discipline — the placement the
  §5 candidate schedule pins, deadlock-free at any `capLevel`
  (`Sched.deadlock_free_d5`) — and NOT the shipping encoder's: the Rust
  emits the parent in the scope *epilogue*, after the last chunk's
  queries and trailing wires (§5 above lists parent-before-last-queries
  among the orderings "the Rust scheduler can never produce";
  `.agent-notes/2026-07-18-parent-placement/parent-placement.md`
  records the full design space and why the epilogue order wins on
  pipelining). An earlier revision of this
  paragraph claimed the encoder enforces the D5 placement; that was
  wrong, caught 2026-07-18 when the Rust-side probe found the real
  publication order violating the minted check.

- **Axiom D6 (epilogue placement).** The shipping encoder's corner of
  the same design space, the mirror-image of D5: the parent summary of
  a scope departs only after every other send of the scope — all
  wires, and every D child's resolution and full query quota. This is
  the order `materialized/levels.rs` produces ("Launch every `Pending`
  slot's work before publishing its enclosing parent resolution").
  Under D6 (mode `AxMode.impl`: D6 instead of D5, all else as `.full`)
  deadlock freedom additionally requires the capacity margin
  `capLevel ≥` max per-scope dispute count — the encoder's
  `FAN ≥ kids` discipline; at `dCount = capLevel + 2` the parent-delay
  trap is real (`Control.parentTrap`). D5 and D6 are never asserted
  together: at any scope with a send left after the final
  D-resolution, their guards contradict and the choice point wedges.

`AxiomMode` switches: `W`, `D1root`, `D1internal`, `D2`, `D3`, `D4`, `D5`,
`D6` — each independently droppable, giving the negative controls N1 (drop
W), N2 (drop D1 at the root), N3 (drop D1 internally), N4 (drop D2),
N5/`ledgerGap` (drop D3), and the Lean controls `Control.jam` (drop D4) and
`Control.pdelay` (drop D5 with D6 unasserted; the Quint spec predates all
three and has no `AX_D4`/`AX_D5`/`AX_D6` consts — the Lean model is the
model of record for them).
Dropping a guard removes those poset edges and nothing else; the
checker then searches the freed linearizations for a stuck state. One
scaffolding const, `WIRE_FIRST`, is **not an axiom**: because the wire
ledger never constrains `DependentWork`, a bare D1 drop frees queries
before the wire reply and deadlocks already at fan 2 (`n2unrestricted`) —
`WIRE_FIRST` pins queries after their child's wire send so the N2/N3
controls isolate the resolution-vs-query reorder their predictions are
about. Every mode change is a `const` of one parameterized spec — no
forked copies.

## 7. Predicates, theorems, and why bounded checking is complete

- `Terminal` ≡ every process has fired all its operations. (Operationally in
  Rust: both terminal futures resolved, every registered task returned —
  the end-of-stream cascade.)
- `Stuck` ≡ ¬Terminal ∧ no operation enabled — the exact model twin of the
  quiescence driver's `Pending`-with-no-wake (`Quiescence::Stalled`,
  `src/testing.rs`).
- `safe` ≡ ¬Stuck (equivalently Terminal ∨ ∃ enabled).
- ρ(s) ≡ total unfired operations (in Lean: `Model.rho`,
  Proofs/Termination.lean, where the strict decrease is the kernel
  theorem `rho_decreases`). Every step fires 1 op (2 for a
  rendezvous), so ρ strictly decreases; run length ≤ ρ(init). Hence:
  **no infinite runs exist**, bounded model checking at depth ρ(init) + 1 is
  **exhaustive** for reachability, and termination is a fairness-free
  corollary of safety — every maximal run is finite and, by (i), ends
  Terminal. A standing constraint follows: **the model must never grow an
  unbounded loop**, or both the BMC-completeness argument and the
  fairness-free termination argument silently die. Any future edit that adds
  an op not consumed from a finite skeleton-derived budget is wrong.

## 8. The capacity-tightness law (validation property)

With C the `level` capacity and a parent scope disputing N children whose
subtrees complete while the parent's reaction loop is still running, the
pairing completes under every schedule iff **N ≤ C + 2**. The two slack
units, in model terms: one return held in the blocked assembler's hand (its
committed `send level` unfired), and one child resolution parked in the
cap-1 `lowerRes` slot — that child's return has not been materialized yet,
so it never needs level-queue room before the parent resolution frees the
drain. (An earlier draft attributed the second unit to a stream yield
slot; the collapse of stream/driver pairs into sequential processes — §4 —
eliminates that mechanism, and the model reproduces the Rust thresholds
without it.) Rust ground truth (`streaming/tests/capacity.rs`,
`capacity_stress_witness_requires_inter_level_fan`): the `[32, 256]` pyramid at
default C = 256 reaches high-water ≥ 254, stalls at C = 253, completes at
C = 254. The Rust test pins the *deterministic* run; the model checks the
stronger all-schedules claim at the scaled instance (F = 4: stall reachable
at C = 1, safe at C = 2) — if the all-schedules claim fails where the
deterministic run passes, that is a finding about scheduling slack, not
automatically a model bug (the assembler queue's doc in `work/queues.rs`
explicitly refuses to rely on such slack).

Production stance: C = F, under which `Asm` sends never block — occupancy on
`level(p, j)` is bounded by the pending count of the one in-flight parent
resolution ≤ F ("the bound does not multiply with tree width or depth",
`work/queues.rs`). That inequality is the one FAN counting lemma of the Lean
proof.

## 9. Known risks and premises (tracked)

1. **SPSC / no sender clones** — verified today; a future `Sender::clone`
   breaks close semantics and enabledness stability. Modeled-world premise.
2. **Rendezvous inlining of pumps/forwarders** — valid while `respond` /
   `return_into` pull only after the previous send completes
   (send-then-next loop shape, `work.rs`). The +2 arithmetic
   shifts if that changes.
3. **Sequential-scope premise** (§5.4) — slightly stronger than the ledgers;
   a pipelined future implementation would need the poset loosened.
4. **Model-only deadlocks in relaxed/reduced configurations** are
   Rust-relevant only when schedule-independent (counting arguments, no
   races): true for the tightness stall and the expected N1/N2 cycles.
5. **Unrealizable skeletons** — sound for the theorems; check realizability
   before reporting a counterexample as a Rust bug.
6. **No unbounded loops** — the §7 standing constraint.

## 10. Cross-reference table (Rust ↔ model)

| Rust artifact | Model name |
|---|---|
| `Trace::assert_valid` wire ledger (progress.rs) | Axiom W guard |
| dependent ledger (progress.rs) | Axiom D1 guard |
| lower ledger (progress.rs) | Axiom D2 guard |
| sibling-contiguity check (progress.rs, added 2026-07-15) | Axiom D3 guard |
| wire-contiguity check (progress.rs, added 2026-07-16) | Axiom D4 guard |
| epilogue-placement check (`Trace::assert_parent_last`, progress.rs, added with finding #7's resolution, 2026-07-18) | Axiom D6 guard (mode `AxMode.impl`) |
| `Trace::assert_parent_early` (deliberately unwired, `should_panic`-pinned: the weave's parent-early corner, .agent-notes/2026-07-18-parent-placement/parent-placement.md) | Axiom D5 guard (mode `AxMode.full`) |
| radix-order check (progress.rs, added 2026-07-15) | per-channel in-order program structure (§5.3) |
| `yield_resolve_query!` (materialized.rs) | the honest linearization (one refinement of the poset) |
| `outgoing_responses` doc (`work/queues.rs`) | `wire` cap 1 + pump hand |
| `assembly_level_returns` doc (`work/queues.rs`) | `level` cap C, FAN counting lemma |
| the twelve other constructor docs (queues.rs) | per-channel cap-1 sufficiency lemmas, one each |
| `run_to_quiescence` `Stalled` (`src/testing.rs`) | `Stuck` |
| `capacity_stress_witness_requires_inter_level_fan` (`streaming/tests/capacity.rs`) | tightness instances (§8) |
| `capacity_stress_matrix` shapes (`streaming/tests/capacity.rs`) | positive instance skeletons |
| session completion (the terminal `try_join`, `streaming/driver.rs`) | `Terminal` |

Instance-to-witness mapping, expected outcomes, and the N1–N4 control
predictions live in `formal/quint/README.md` next to the runner that
checks them.
