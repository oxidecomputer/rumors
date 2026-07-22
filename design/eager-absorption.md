# Eager absorption: feasibility of K-deep reply parking on one socket

Assessment of Finch's proposal (MUX-PROGRESS.md §5, the T8 log entry,
2026-07-21): convert incoming wire frames into *logical* protocol
replies at arrival — supplied (provision) runs absorbed at line rate
through the existing `Backend` streaming construction into unlinked
node handles — and park up to K logical replies per stream at the
demux boundary, so a K-reply-denominated window becomes sound on a
single socket; pair it with the sender-side inferred-credit scheme
σ\*ₖ. The claim under test: this is a **custody** change (where bytes
live pre-cursor), not a tree-semantics change — the walk's positional
consumption, merge decisions, and linking stay put.

This document is an assessment, not a design of record. It answers
from the code on `mux-elastic` (based on main: the 17-stream
single-socket mux, `src/tree/mirror/streaming/remote/`), with the
`link-transport` branch's `Window` read for comparison. Epistemic key
as in `design/streaming-wire-deadlock.md`: **[checked]** = verified by
reading the cited code; **[derived]** = argued here from checked
premises; **[open]** = needs a spike.

## 1. Verdict, stated first

**The proposal's receiver half is already the code's architecture.**
Construction of supplied subtrees does not run walk-side today: it
runs in per-stream proxy pump coroutines fed by the demux, pre-cursor,
using exactly the `Backend::parent` bottom-up fold the proposal names
(§2–§3). What limits the wire window to ~1 reply per stream is not
custody but three capacity-1 gates on the pump's inputs and outputs.
The receiver refactor therefore reduces to **capacity plumbing plus a
knob** — grade **moderate** — and the decode-context ledger the
proposal requires (position → prefix, expected radices, height) exists
verbatim as the `Scope` FIFO, registered by prior local emissions
exactly as the causality argument predicts (§3.3).

**The sender half (σ\*ₖ inference) is the invasive part** — grade
**invasive** — because the mux and demux currently share no state at
all, and the consumption-inference engine has no existing home (§7.3).
Without it, K-parking alone converts the deterministic w = 4 wedge
into a w > K wedge: a real mitigation with bounded memory (unlike the
rejected §5B capacity bump, the parked unit is a *decoded* reply, so
the bound is genuine), but not a liveness proof — and
`wc_impossibility_K` (MUX-PROGRESS.md T8) says no work-conserving
sender survives any fixed K.

**Version bounds cannot be resurrected by eager absorption** — the
deletion-honoring filter is entirely sender-side against the
receiver's handshake version, which is fixed for the session; the
receive path performs no version filtering at any time, so moving
construction earlier moves no check (§4). **[checked]**

Combined grade: **moderate (receiver) / invasive (sender), cleanly
separable.** No blockers found; surprises and unknowns in §8.

## 2. How a provision run flows today [checked]

The trace requested, demux to link, for the remote composition (local
materialized walk ↔ remote proxy). "Provision run" = the reply to an
empty `Query` (the whole-node request, `message.rs:60-66`), carried on
the wire as a run of leaf-supply frames.

1. **Demux.** The sole reader (`remote/session/incoming.rs:60-92`,
   `Demux::run`) decodes each frame and routes it into the per-stream
   one-slot handoff (`HANDOFF_CAPACITY = 1`, `remote/session.rs:31`).
   A full handoff blocks the reader — head-of-line across all 17
   streams (`incoming.rs:86-90`). Stream ↔ height is static
   (`remote/proxy/state.rs:57-59`).
2. **Which channel.** The height-H handoff's `ReceiverStream` is taken
   once by the height-H pump (`state.rs:46-48`,
   `Incoming::take`, `incoming.rs:36-41`).
3. **Which coroutine.** The pump is a `try_stream` spawned as an
   independently runnable task by `Work::respond`
   (`remote/proxy/work.rs:78-100`); the per-height bodies are
   `initiator` / `opening_responder` / `internal_replies` /
   `leaf_replies` / `complete_responder`
   (`remote/proxy/work/pump.rs:44-242`). Each loop iteration pops one
   decode context from the flushed-question FIFO
   (`questions.recv()`, e.g. `pump.rs:133`) and calls
   `decode_reply` (`pump.rs:134-136`), which consumes that reply's
   frames from the handoff until its explicit end.
4. **Which Backend calls.** Inside `decode_reply`
   (`remote/adapter/decode.rs:99-128`), each `Supply(version, message)`
   frame is validated against the scope (path recomputed from content,
   `decode.rs:276-321`), constructed as a leaf
   (`Leaf::leaf`, `decode.rs:175`), and streamed through a one-slot
   channel (`decode.rs:113`) into `Convert::assemble`
   (`decode.rs:190-206`), whose `fold_parents` flushes each completed
   radix group through `Backend::parent`
   (`convert.rs:104-154`, the call at `convert.rs:128`) — the exact
   bottom-up unlinked-handle fold the proposal describes. Memory
   during the fold is one leaf in flight plus one open group per level
   (`remote/adapter.rs:42-48`).
5. **Where the handle lands.** `reify` (`decode.rs:209-238`) slots the
   assembled height-H handles into the reply skeleton, producing a
   fully materialized `Reply<B, T, H>` — `O(fan)` reactions, each
   supply an `O(1)` cloneable handle (`backend.rs:23-24`; nodes are
   persistent-structure pointers, `typed/node.rs:150-162`). The pump
   yields it into the one-slot `ProxyResponses` buffer
   (`remote/proxy/work/queues.rs:14-16` via `work.rs:78-100`), then
   publishes the reply's derived lower scopes into the one-slot
   `ProxyNextScopes` (`proxy.rs:25-38`, `yield_reply_scopes!`).
6. **Where linking happens.** The walk — the materialized levels
   (`materialized/work/levels.rs`) — consumes the `Reply` positionally
   against its own `Query` queue, runs the `Resolver`
   (`materialized/work/resolver.rs:39-73`; supplies absorbed as
   `Resolve::Ready(Some(node))`, `resolver.rs:61`), and `assemble`
   links resolved scopes upward through `Backend::parent` in query
   order (`materialized/work/assembly.rs:72-90`, the call at `:88`),
   the root landing last (`levels.rs:78-81`, `:142-149`). At the
   terminal leaf level the initiator races `absorb` against the
   remaining work (`materialized.rs:518-534`; the pairing loop
   `:541-580`).

So the split is: **frames → handles happens pump-side, pre-cursor,
today** (steps 3–5); **handles → tree happens walk-side, at cursor,
today** (step 6). The proposal's custody line already exists; what
does not exist is depth behind it. The wire window per stream is ~2
replies — one parked in `ProxyResponses`, one being decoded — plus one
frame in the handoff; the deadlock doc's §2 cycle is exactly this
window being 1 where 2 was needed transitively.

## 3. The plumbing question, answered

### 3.1 Can construction be driven from the demux side? It already is.

The question dissolves on inspection: `decode_reply` is never called
by the walk and never touches walk state. Its inputs are the backend
handle (cloned per pump, `pump.rs:131`), the incoming frame stream,
and a `Scope`. The refactor shape — (a) register a decode-context FIFO
at emission, (b) demux-side converter pops contexts positionally and
drives Backend construction, (c) walk consumes positional descriptors
at cursor time — **is a description of the existing code**:

- (a) is the `Scope` ledger. `Scope<H>` holds parent prefix + the
  listed child radices + a positional cursor
  (`remote/adapter/scope.rs:11-19`) — precisely
  "position → prefix, expected radices"; height typing is static per
  stream and monomorphized per pump, no dynamic evidence needed.
- (b) is the pump + `decode_reply`.
- (c) is the `Reply<B, T, H>` itself: positional by stream order,
  handles only. No handle-*futures* are needed — construction
  completes during decode (line rate), so the parked descriptor is
  strictly simpler than the proposed `(position, handle-or-future)`.

What the refactor actually changes: the three capacity-1 gates that
serialize the pump to lookahead ~1 —
`ProxyResponses` (the parked-reply slot, `queues.rs:14-16`),
`ProxyLocalQuestions` (the context-FIFO depth, `queues.rs:19-21`),
`ProxyNextScopes` (`queues.rs:24-26`) — widened to K. The demux
handoff stays at 1: frames drain at line rate whenever the pump is not
parked, and the pump now parks only after K decoded replies are
parked. **[checked structure; K-liveness derived §7]**

### 3.2 What walk-owned state does construction touch? None.

Enumerating what decode needs against where it lives **[checked]**:

| needed by construction | where it lives | walk-owned? |
|---|---|---|
| expected parent prefix | `Scope::parent` (`scope.rs:35-37`) | no — proxy ledger |
| expected radices (positional cursor) | `Scope::children`/`next` (`scope.rs:40-44`) | no — proxy ledger |
| height typing (`Convert` instance) | static stream ↔ height (`state.rs:57-59`), monomorphized pumps | no |
| backend handle | cloned into each pump (`pump.rs:131`) | no — `Backend: Clone` (`backend.rs:24`) |
| the session's version state | **not consumed by decode at all** (§4) | n/a |

The walk-owned `Query::ours` (our node handles,
`materialized.rs:162-170`) is consumed only by the cursor-time
`Resolver`. Note its radix set equals the `Scope`'s radix set — both
derive from the same listing (`answer.rs:59-64` builds the wire
listing from `ours`; `Scope::new` records the listing's radices,
`scope.rs:26-32`) — so even the cursor-time pairing checks are
positionally derivable demux-side; they simply have no reason to move.

### 3.3 The causality property: registration precedes need [checked, arm by arm]

The load-bearing fact — the receive-side mirror of the send-side
announcement completeness that makes σ\* local
(MUX-ADJUDICATION.md §1.4) — deserves a name:

> **Context-registration causality.** Every arriving wire message is
> causally downstream of the local emission that registered its decode
> context. Construction at arrival never needs state that does not yet
> exist; it needs only state that today happens to be threaded through
> capacity-1 queues.

Enumerating every arrival kind against its registration
**[checked]**:

1. **Handshake frames.** Context is the fixed preamble; registered by
   connection establishment. No session state consumed.
2. **The initiator's opening query** (the sole reply that answers no
   prior question, `remote/adapter.rs:24-27`). `decode_opening`
   requires no scope — it *mints* the root scope from the listing
   itself (`decode.rs:38-55`); causally downstream of the version
   exchange that elected roles.
3. **Every subsequent reply** on a remote-spoken stream at height H
   (all remaining protocol traffic — the remote's questions to us
   arrive only *inside* these replies as `Query` reactions). Context:
   the `Scope` popped from `ProxyLocalQuestions`, which the encode
   task publishes only after the *entire* local reply containing the
   corresponding question has flushed
   (`remote/proxy/work/encode.rs:73-78`, batch collected in
   `write_reply` `:111-122`, published after; opening at `:97-102`;
   the adapter releases each scope only on write success,
   `remote/adapter.rs:28-31`). The remote can emit its k-th reply on
   the stream only after consuming our k-th question, which it
   receives only as part of that fully-flushed reply — the remote's
   pump consumes replies atomically (`decode.rs:131-187` reads to the
   explicit end) and its walk answers only whole logical replies
   (`levels.rs:182`). Registration therefore precedes any arrival that
   needs it by at least one network traversal. **No counterexample
   exists in the message vocabulary**: `message.rs:34-67` and the
   frame grammar (`remote.rs:3-16`) contain no third arrival kind.
4. **Nested contexts.** A `Query` reaction inside a decoded reply
   mints the scope for *our* future answer (`decode.rs:163-167`,
   published reply-first via `yield_reply_scopes!`, `proxy.rs:25-38`);
   our answer's encode consumes it and mints the scopes of the
   questions *we* ask (`encode.rs:73-78`). The ledger is closed: every
   scope is minted by decoding or encoding an adjacent-height reply,
   never by consulting the walk. This is the BFS alignment fact in
   code — stage n's scope list is fixed by stage n−1's decoded
   reactions.
5. **Stream end.** Transport control, consumed structurally by the
   demux against its own lifecycle array (`incoming.rs:74-84`).

One caveat separates *causal* from *operational* availability: the
push into `ProxyLocalQuestions` can lag its causal availability when
the queue is full (capacity 1 today; the encoder blocks mid-batch at
`encode.rs:78`). At the floor this is covered by the existing ordering
argument; at K it becomes a sizing obligation (§7.2). The proptest
worth writing asserts the causal form: **at every reply-start arrival
on stream s, the context for that reply has already been *emitted*
(flushed) locally** — instrumentable in the existing named-channel
harness (`channel.rs:84-90`).

## 4. The version-bound question: request time and answer time, never receive time [checked]

Where supplied content meets version bounds, exhaustively:

- **Request time (receiver side).** The merge join decides what to
  request: `EitherOrBoth::Right` — a child we lack — emits the empty
  `Query` (`materialized/work/answer.rs:74-81`, leaf-parent form
  `:123-127`). No bounds are consulted; the receiver asks for
  everything it lacks, and correctly so — what it should *not*
  re-learn is precisely what the sender's filter removes.
- **Answer time (sender side), the only filtering site.** Every
  `Supply` is pre-cleared against `their_version` — the counterparty's
  handshake version — before it is ever emitted:
  - internal `Left` arm: `unknown(backend, their_version, …)`
    (`answer.rs:67-73`);
  - leaf-parent `Left` arm: the `known` filter (`answer.rs:116-122`,
    predicate at `unknown.rs:44-49`);
  - terminal leaf answer: same filter (`answer.rs:145-155`);
  - whole-subtree provisions (the empty-listing branch of the walk):
    `unknown_providing` prunes the subtree and reports the survivors
    that become the `Supply` run (`levels.rs:194-207`, `:284-297`;
    the recursion `unknown.rs:94-128`, classification by memoized
    floor/ceiling at `unknown.rs:58-70`).
- **Receive time: nothing.** The `Resolver` absorbs supplies with
  ordering checks only (`resolver.rs:50-62`); terminal `absorb`
  likewise (`materialized.rs:560-565`); `assemble_supplies` and
  `fold_parents` construct without consulting any version
  (`decode.rs:190-206`, `convert.rs:104-154`).

The check is therefore a **cursor-independent function of the message
and the session's stable snapshot** on the *sender's* side:
`their_version` is fixed at `Connected` (`materialized.rs:311-343`)
and the answering tree is the immutable session root. On the
receiving side there is no check to hoist, so eager absorption is
trivially safe against resurrection: it constructs exactly what the
cursor-time path constructs, from the same pre-cleared frames, into
unlinked handles that acquire reachability only through the unchanged
cursor-order linking (`assembly.rs:72-90`). A *byzantine* sender can
supply redacted content in either regime — parking changes nothing
about that trust boundary. **[checked]**

## 5. The violation question: what fires where [checked]

Checks that fire during supplied-run consumption, with input
availability:

**Arrival-time today (decode/codec path)** — all inputs positionally
or content-derivable at arrival, none walk-owned:

| check | site | input |
|---|---|---|
| signal placement / phase schedule | codec (`remote.rs:8-11`, `codec/signal.rs`) | static |
| `LeafOutsideScope` | `decode.rs:289-295` | scope parent + content-derived path |
| `LeafOrder` (strict leaf ascent) | `decode.rs:296-308` | run-local |
| `SupplyOrder` (strict run ascent) | `decode.rs:310-313` | run-local |
| `TruncatedReply` / `UnexpectedStreamEnd` / `BareEndAfterReaction` | `decode.rs:147-155` | frame-local |
| `UnpositionedQuery` / `NonemptyLeafQuery` | `decode.rs:72`, `:90-93` | scope cursor |
| run/assembly agreement (internal asserts) | `decode.rs:222-236` | decode-local |

**Cursor-time today (walk path)** — inputs are `Query::ours` and the
query queue, but as §3.2 notes the radix content is the same as the
scope's:

| check | site | arrival-derivable? |
|---|---|---|
| `InvalidSupply` (radix ≤ previous, or > next held) | `resolver.rs:51-53`, `:58-59`; leaf form `materialized.rs:562-563` | yes — radices ≡ scope radices |
| `UnexpectedSupply` (collides with held child) | `resolver.rs:54-57` | yes — same |
| `UnexpectedMatch` / `UnexpectedQuery` (cursor exhausted) | `resolver.rs:45-47`, `:64-67`; `answer.rs:145-147` | yes — cursor arithmetic |
| `UnfinishedReply` (cursor not exhausted) | `resolver.rs:83-88`; `materialized.rs:564` | yes |
| `UnaskedReply` / `UnansweredQuery` (pairing counts) | `levels.rs:183-185`, `:240-242`; `materialized.rs:552-553`, `:575-577`; proxy `reject_extra` (`pump.rs:246-254`) | yes — FIFO counts |

There are **no hash checks on supplies** — supplied content is by
definition content the receiver holds no expectation for; hashes are
recomputed by construction, and `Match` is a positional trust
decision, not a verification. Conclusion: every input the cursor-time
checks need is available at arrival, but nothing forces the move —
under parking the checks run unchanged when the walk consumes the
parked reply, preserving the walk's error vocabulary verbatim. The
only behavioral delta is *timing*: decode-side violations for reply
k+j can now surface while the walk is still at reply k, which shifts
which-error-wins races (`tests/faults.rs` pins some of these;
re-examination needed, §8 [open]).

## 6. Torn state and reclamation [checked for Local; doc obligation for persistent backends]

`Backend`'s stated contract is that node values are cheap cloneable
*handles* (`backend.rs:23-24`); nothing in `parent`'s contract
(`backend.rs:34-52`) registers constructed nodes anywhere — the caller
holds the only handle. For `Local`, `parent` builds a persistent-
structure branch node (`backend/local.rs:82-108`); `typed::Node` wraps
refcounted shared pointers (`typed/node.rs:141-162`), so dropping an
unlinked subtree's root handle reclaims it. On session abort the
parked replies live in channel slots and task-owned locals inside
`Work::tasks` / the drivers future (`remote/proxy/work.rs:103-114`,
`tasks.rs:20-37`); abort drops the future tree, drops the handles,
reclaims the subtrees. **The walk already creates the same exposure**:
assembly constructs bottom-up with the root linked last, so every
session that aborts mid-descent already orphans constructed
intermediate nodes. K-parking changes the *quantity* bound (≤ K+1
in-flight constructions per stream instead of ~2), not the kind.

**Backend doc obligation if the refactor proceeds** (and honestly,
already latent today): a persistent backend whose `parent`/`leaf`
durably allocate must treat unlinked constructed nodes as
reclaimable garbage — content-addressed idempotency, session-scoped
staging, or GC of unreachable nodes — and must tolerate abort at any
point between construction and linking. This belongs in
`backend.rs`'s `parent` docs as a named `# Durability` section, plus
the conformance suite once one exists for backends.

## 7. K-window bookkeeping, and the Window relation

### 7.1 Where the parked queue lives and its bound

The parked-descriptor queue is `ProxyResponses` widened to K — already
per-stream (one pump per height per direction) and already demux-side
in the relevant sense (fed by the coroutine that drains the demux
handoff). For the widening to be effective, `ProxyLocalQuestions` and
`ProxyNextScopes` widen with it (§3.1); the demux handoff stays at 1.

RAM bound per parked reply **[checked]**: `Vec<Reaction>` of ≤ fan
entries; `Supply` = one handle (the subtree is shared structure, not
copied); `Query(listing)` = ≤ fan hashes. Worst case is the maximally
disputed reply ≈ fan² hashes ≈ 2 MB (`message.rs:15-17`); the
provision-run case the proposal targets is O(fan) handles. Total:
K × 17 streams × ≤ 2 MB worst, K × O(fan) handles typical. This is
the load-bearing improvement over every frame-denominated scheme: the
parked unit's cost is bounded by the *logical* reply, not by the
subtree it carried (§7.4).

### 7.2 The context-FIFO depth [open]

`ProxyLocalQuestions` occupancy = questions flushed but unanswered on
one stream. That is *not* K-bounded from below by anything: deeper
streams aggregate questions across many parent replies
(deadlock doc §4). Entries are tiny (prefix + ≤ fan radices), so a
generous fixed depth or an unbounded ledger with global accounting are
both plausible; deriving the actual bound from the walk's channel
capacities (or tying it to the same K) is a spike this assessment did
not close.

### 7.3 What the sender inference needs that the session doesn't expose

σ\*ₖ gates the *start* of pushing reply r on stream s until ≤ K−1
prior replies on s are un-provably-consumed. The natural gating point
is the top of the encode loop (`encode.rs:73` — reply-atomic, no
mid-reply withholding, exactly the constraint §5D of the deadlock doc
identified). The sender currently has:

- **Own pushes**: countable — encode tasks see reply boundaries;
  `FrameSender` receipts are flush-paced (`outgoing.rs:53-63`),
  which is the right observation per the standing ruling
  (consumption receipts stay out; MUX-PROGRESS.md §1).
- **Arrivals**: the decoded incoming replies, but they live in
  per-height pump tasks with no channel to the outgoing side. The mux
  scheduler sees only per-stream readiness
  (`outgoing.rs:200-224`); mux and demux meet only as independent
  futures in `Drivers::run` (`coordinator.rs:45-84`). **There is no
  shared state to hang an occupancy ledger on today.**

The new component is a per-session occupancy estimator: per-stream
push counters updated by encode, consumption evidence updated from
decoded arrivals, and the **inevitability closure** deriving the
silent consumptions (provision absorptions produce no reverse traffic
ever — the all-M/provision blindness that defeats evidence-only
schemes, MUX-ADJUDICATION.md §1.4). Its correctness window is narrow
on both sides: under-inference idles into a starvation deadlock,
over-inference readmits the wedge past K. Its specification *is*
`sigmaStarK_deadlock_free` (T8); shipping it ahead of that theorem
means shipping an unproven liveness argument, which is against this
codebase's stated posture (deadlock doc §4's closing paragraph).
Estimated as the dominant cost: an event-derivation engine in the
spirit of the probe's causal σ\*, order 1–2k lines plus its own test
apparatus.

### 7.4 Is this the Window mechanism relocated? Yes — with the credit inferred and the unit dissolved

`link-transport:src/tree/mirror/streaming/window.rs` widens the same
edges (its module doc names "the proxy's flushed-question and
next-scope queues" explicitly, `window.rs:3-5`), denominates the knob
in node references, and **delegates wire pacing to the transport's
per-stream flow control** (the Link contract; deadlock doc §8). The
proposal reconstructs both halves inside the endpoints of one socket:

- receiver K-parking supplies the per-stream buffer that a credit
  window's grant W promises explicitly;
- σ\*ₖ supplies the sender pacing that the credit *message* carries
  explicitly — "W = 1 credits inferred instead of sent"
  (MUX-ADJUDICATION.md §1.3), generalized to K.

And it dissolves the deadlock doc's one discontinuity: §5A ruled
1 < W *in reply units* unsound because a granted reply's buffer cost
was unbounded (a provision run's frames). Eager absorption changes the
buffered unit from frames to decoded replies — O(fan) handles for
precisely the provision case — so the grant unit and the buffer unit
finally match, and reply-denominated K > 1 becomes sound. That is the
design insight of T8 in one sentence, and it checks out against the
code. What the relocation does *not* buy relative to link-transport:
loss-level stream independence (one TCP segment loss still stalls all
17 streams), and the pacing invariant moves back from "by contract"
to "by proof" — the exact trade §5A's comparison table recorded.

## 8. The change list, the invariants, the tests, the honest unknowns

### Receiver half (moderate)

| module | change | est. |
|---|---|---|
| `streaming/window.rs` (port/adapt from link-transport) | K knob, reply-denominated for proxy edges | ~100 |
| `remote/proxy/work/queues.rs` | capacity K on all three edges; thread K through `Work::new` / `Handshaking` | ~80 |
| `remote/proxy/work/pump.rs` | no structural change **[checked]**; doc updates | ~30 |
| `remote/session.rs`, `remote.rs`, `materialized.rs` docs | the parking invariant + the causality property added to the deadlock-freedom prose | ~120 |
| `backend.rs` | the `# Durability` obligation (§6) | ~30 |
| tests (below) | | ~500 |

### Sender half (invasive)

| module | change | est. |
|---|---|---|
| new `remote/session/occupancy.rs` (or sibling) | per-stream ledger + inevitability closure | ~800–2000 |
| `remote/proxy/work/encode.rs` | reply-start gate | ~60 |
| pump → ledger wiring | consumption evidence from decoded arrivals | ~100 |
| `remote/session/outgoing.rs` | unchanged policy among gated-ready streams; docs | ~30 |

### New invariants needing tests/proofs, stated precisely

1. **Context-registration causality** (§3.3): for every arriving
   reply-start on stream s, the decode context it will consume was
   flushed locally before the arrival. Proptest over the instrumented
   channels + fault schedules; this is the receive-side twin of the
   announcement-completeness pillar and is worth pinning
   independently of K.
2. **Parking invisibility**: for every tree pair and schedule, the
   sequence of (reply, violation) outcomes the walk observes is
   identical at K = 1 and every K > 1 — parking may change timing and
   error *racing* only. Extend the existing streaming-vs-alternating
   oracle proptests with a K sweep; keep test default at the floor as
   `window.rs` does (`window.rs:100-113`).
3. **Eager-construction equivalence**: per stream, the sequence of
   `Backend` calls (`leaf`, `parent` with arguments) is invariant in
   K. Structurally true (same code path, §3.1); pin it with the
   adversarial backend's schedule instrumentation
   (`backend/local/adversarial.rs`) so a future pump rewrite cannot
   silently break it.
4. **Occupancy-inference soundness and liveness** (sender half):
   inferred-unconsumed(s) never undercounts actual (soundness — never
   more than K parked at the receiver), and inferred-unconsumed
   eventually decreases whenever actual does (liveness — no
   starvation idle). The Lean twin is T8; the Rust proptest runs the
   ledger against the closed-world session's ground truth.
5. **Memory bound**: parked bytes per direction ≤ K × 17 × (fan² hash
   + fan handles) — assertable in the instrumented channel harness.

### Existing coverage that carries over [checked]

The adapter's decode/encode losslessness and rejection proptests
(`remote/adapter/tests/properties.rs` — supplied runs, duplicate and
foreign leaves) cover the construction path unchanged. The capacity
stress matrix and its role-coverage assertion
(`streaming/tests/capacity.rs:71-166`) extend naturally to the new
capacities. The wedge-shaped regression seeds
(`tests/pairwise.proptest-regressions`,
`tests/shadow_validity.proptest-regressions`) are the K-widened
impossibility witness family for free. `run_to_quiescence`'s Stalled
witness remains the operational deadlock oracle.

### Unknowns and surprises

- **[surprise]** The plumbing question dissolved: the code already has
  the proposed architecture, and the assessment reduces to window
  plumbing plus the sender engine. No Rust touches tree semantics,
  merge arms, or linking — confirming the custody framing exactly.
- **[open]** `ProxyLocalQuestions` depth bound (§7.2) — spike.
- **[open]** Wire snapshot stability: receiver-only parking sends the
  same bytes but perturbs mux readiness timing, so whole-wire
  interleaving pins (`tests/gossip_snapshot.rs` and friends) may churn
  even before the sender half lands; per-stream content is invariant.
  Determine whether the pins are per-stream (survive) or whole-wire
  (deliberate re-accept) before starting.
- **[open]** Error-racing deltas (§5): which violation wins when an
  arrival-time decode error for reply k+j races the walk's cursor at
  reply k; `tests/faults.rs` pins today's order.
- **[open]** Whether receiver-half-only is shippable as a mitigation:
  it bounds memory honestly (unlike §5B) but moves the wedge to
  w > K rather than closing it; the codebase's standard
  (deadlock doc §4) argues no, unless K-parking ships explicitly as a
  performance change on top of a transport that already guarantees
  pacing — at which point it converges with link-transport's Window
  and the single-socket question returns to the formal campaign.
