# The abstract model of the streaming mirror protocol

This is the design document for the formal deadlock-freedom artifact. It is
the single specification that `quint/streamingMirror.qnt` (Phase A/B) and the
Lean package (Phase C/D) both transcribe; when the three disagree, this file
is wrong exactly until the disagreement is resolved, and the resolution lands
here first.

The modeled system is `src/tree/mirror/streaming/` as of 2026-07-15 — the
**in-memory driver** (`mirror_connected`): two conforming parties on the
`Local` backend, no remote transport (none exists yet; `remote/` is a framing
layer below this model's abstraction). Every structural claim below was
extracted from the Rust with file/line citations and adversarially
cross-checked; the extraction reports live with the working notes, and the
load-bearing citations are repeated here.

## 1. What is proved, what is assumed

**Assumed (the bridge to Rust, proptest-verified there):** the send-order
invariants, encoded *exactly* as the four checks of
`progress.rs::Trace::assert_valid` (§6) — the three original ledgers plus
the sibling-contiguity check that **this modeling effort surfaced as
missing**: the original three ledgers admitted a "publish all wires, then
all resolutions, then all queries" implementation that passes `assert_valid`
and deadlocks the cap-1 child-resolution queue at fan ≥ 3 (the `ledgerGap`
instance). `assert_valid` was tightened on 2026-07-15 so that the checked
invariant and the assumed axiom set coincide. The Rust's hard-wired
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
- (iii) *Validation property, not a theorem*: the capacity-tightness law
  N ≤ C + 2 for the inter-level return boundary (§8).

**Explicitly not modeled** (modeled-world premises, each with its Rust
anchor):

- **The error path.** All park-forever sites (`next_or_pending!`,
  `divert`'s park) and all `Violation` aborts are reachable only after a
  fault has claimed the driver's one-slot error channel
  (streaming.rs:84-91). Two conforming peers on `Local`
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
  session before the protocol begins ("two roots hash equal only when
  their versions are equal", protocol.rs:116-120) — the handshake, the
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
  answer.rs:112-115 — so height-0 leaves are never disputed; height-**1**
  scopes *are* disputable, via `answer::internal` instantiated at `H = Z`).
- Fan bound: ≤ F children per scope (`FAN = 256` in Rust, queues.rs:36).

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
(materialized.rs:183-186).

Per party, the session is a fixed pipeline of **walk stages** `S(p, h)`, one
per consumed message index `h`, plus an opening, a terminal, assemblers, and
pumps. The schedule (`mirror_connected`, streaming.rs:120-130) fixes:

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
("the counts must move together", peer.rs:114-117).

## 4. Processes and the channel graph

All concurrency is cooperative: one Tokio task, futures interleaved at await
points (`join_all` + `select!`; no `tokio::spawn` anywhere). The model's
interleaving semantics — any one enabled atomic action per step — is a strict
superset, sound for safety.

### Process inventory (per instance)

| Model process | Rust | Program shape |
|---|---|---|
| `IOpen` | `initiator_level` (work.rs:169-212) | send 1 wire reply (root listing) → send 1 `rootQuery` |
| `ROpen` | `responder_level` (work.rs:215-280) | recv 1 wire → publish root obligations (§5) |
| `Walk(p, h)` | `internal_level` / `leaf_parent_level` / `leaf_level` + its `Work::respond` pump | per scope: prologue recvs → obligation poset → epilogue; then 2 close-recvs |
| `Asm(p, j)` | `assemble` + `return_into` (work.rs:93-105, 518-548) | loop: recv resolution → recv 1 level item per `Pending` slot → send the assembled return upward; then 1 close-recv |
| `Absorb` | `absorb` (materialized.rs:564-604) | per leaf request: recv wire → recv `leafReq` → send return; then 2 close-recvs |
| `IFinish` | work.rs:206-209 | recv 1 from `rootReturn` |
| `RFinish` | inline root assembly (work.rs:268-277) | recv `rootRes` → recv 1 `rootReturns` item per root `Pending` |

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
"14" is the edge taxonomy, not the channel count):

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
queues.rs:296-297).

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

1. **Prologue** (fixed order, program structure): `recv wire` then
   `recv asked` (work.rs:310-311 — reply first, then query).
2. **Publication obligations** (the poset the axioms guard): per D child c —
   `send wire(c)`, `send lowerRes(c)`, `send asked(g)` for each child g of
   c; per R child c — `send wire(c)` only; plus one `send upperRes(σ)`
   (the scope epilogue).
3. Within a scope, **each channel's sends fire in child order** (program
   structure, never relaxed; the axioms govern only cross-channel
   interleavings). Positional pairing is the protocol's identity carrier —
   returns are prefix-less and replies pair with the query queue by index
   (work.rs:512-515) — so an implementation that reorders within a channel
   is functionally incorrect and outside the theorem's scope, and the
   counting abstraction is unsound for it (out-of-order arrivals misbind
   the consumers' positional schedules and manufacture spurious model
   deadlocks). Checked in Rust by `assert_valid`'s radix-order rule.
4. Scope σ+1's prologue requires **all** of σ's obligations fired (walks are
   sequential across scopes — a modeled-world premise slightly stronger than
   the ledgers, which would tolerate cross-scope pipelining; the Rust walk
   is a single sequential loop).
5. After the last scope: `recvClose wire`, then `recvClose asked`
   (work.rs:367-369: the trailing `queries.recv()` check).

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

## 6. The axioms: the ledgers of `Trace::assert_valid`

The axiom guards transcribe progress.rs:40-98 *exactly* — the ledgers, not
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
  `send lowerRes(c)` fired for every D child c of σ. (The Rust comment says
  "launches" = queries — work.rs:359-361 — but the *checked* property is
  child resolutions, which is weaker; the model assumes only the checked
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
  instance; the doc argument in queues.rs:225-229 — "by the time a later
  resolution can block behind it, all work needed by the buffered
  resolution has been launched" — implicitly relied on it). The Rust
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

`AxiomMode` switches: `W`, `D1root`, `D1internal`, `D2`, `D3`, `D4` — each
independently droppable, giving the negative controls N1 (drop W), N2 (drop
D1 at the root), N3 (drop D1 internally), N4 (drop D2), N5/`ledgerGap`
(drop D3), and the Lean control `Control.jam` (drop D4; the Quint spec
predates D4 and has no `AX_D4` const — the Lean model is the model of
record for it). Dropping a guard removes those poset edges and nothing else; the
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
  Rust: both terminal futures resolved, all 49 + 48 registered tasks
  returned — the end-of-stream cascade.)
- `Stuck` ≡ ¬Terminal ∧ no operation enabled — the exact model twin of the
  quiescence driver's `Pending`-with-no-wake (`Quiescence::Stalled`,
  tests.rs:62-91).
- `safe` ≡ ¬Stuck (equivalently Terminal ∨ ∃ enabled).
- ρ(s) ≡ total unfired operations. Every step fires 1 op (2 for a
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
without it.) Rust ground truth (capacity.rs:167-190): the `[32, 256]` pyramid at
default C = 256 reaches high-water ≥ 254, stalls at C = 253, completes at
C = 254. The Rust test pins the *deterministic* run; the model checks the
stronger all-schedules claim at the scaled instance (F = 4: stall reachable
at C = 1, safe at C = 2) — if the all-schedules claim fails where the
deterministic run passes, that is a finding about scheduling slack, not
automatically a model bug (queues.rs:71-73 explicitly refuses to rely on
such slack).

Production stance: C = F, under which `Asm` sends never block — occupancy on
`level(p, j)` is bounded by the pending count of the one in-flight parent
resolution ≤ F ("the bound does not multiply with tree width or depth",
queues.rs:73-74). That inequality is the one FAN counting lemma of the Lean
proof.

## 9. Known risks and premises (tracked)

1. **SPSC / no sender clones** — verified today; a future `Sender::clone`
   breaks close semantics and enabledness stability. Modeled-world premise.
2. **Rendezvous inlining of pumps/forwarders** — valid while `respond` /
   `return_into` pull only after the previous send completes
   (send-then-next loop shape, work.rs:83-88, 98-104). The +2 arithmetic
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
| `Trace::assert_valid` wire ledger (progress.rs:49-56, 94-97) | Axiom W guard |
| dependent ledger (progress.rs:58-78, 90-93) | Axiom D1 guard |
| lower ledger (progress.rs:60-62, 79-86) | Axiom D2 guard |
| sibling-contiguity check (progress.rs, added 2026-07-15) | Axiom D3 guard |
| wire-contiguity check (progress.rs, added 2026-07-16) | Axiom D4 guard |
| radix-order check (progress.rs, added 2026-07-15) | per-channel in-order program structure (§5.3) |
| `yield_resolve_query!` (materialized.rs:104-144) | the honest linearization (one refinement of the poset) |
| `outgoing_responses` doc (queues.rs:38-42) | `wire` cap 1 + pump hand |
| `assembly_level_returns` doc (queues.rs:60-74) | `level` cap C, FAN counting lemma |
| the twelve other constructor docs (queues.rs) | per-channel cap-1 sufficiency lemmas, one each |
| `run_to_quiescence` `Stalled` (tests.rs:62-91) | `Stuck` |
| `capacity_stress_witness_requires_inter_level_fan` (capacity.rs:167-190) | tightness instances (§8) |
| `capacity_stress_matrix` shapes (capacity.rs:69-109) | positive instance skeletons |
| session completion (`join!` resolution, streaming.rs:61) | `Terminal` |

Instance-to-witness mapping, expected outcomes, and the N1–N4 control
predictions live in `formal/README.md` next to the runner that checks them.
