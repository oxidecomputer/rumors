# The streaming wire deadlock: diagnosis and fix space

Status: diagnosis complete (2026-07-16, against `83edcd94`, "WIP: swap
over to streaming; DEADLOCK STILL PRESENT"). Fix determined
2026-07-17: require a stream-capable transport contract — §8 is the
design; §5 is retained as the analysis that led there and as the
design of record for the optional single-socket instantiation.

Symptom: after the swap from the alternating (V1) to the streaming (V2)
protocol on the peer path, two proptests stall and are killed by
`run_to_quiescence`'s deterministic deadlock witness:

    FAIL rumors::pairwise        gossip_order_independent
    FAIL rumors::shadow_validity shadow_predicts_live_state

Both regressions are committed as proptest seeds
(`tests/pairwise.proptest-regressions`,
`tests/shadow_validity.proptest-regressions`); replaying either
reproduces the stall deterministically in ~20 ms:

    cargo nextest run -E 'test(gossip_order_independent)'

Epistemic key, following `formal/PROGRESS.md`: **[checked]** =
observed in an instrumented run of the shrunk regression;
**[derived]** = argument from the code in this document;
**[open]** = known unknown.

## 1. The shape of the failing case

The shrunk `gossip_order_independent` seed is three peers: `a` empty,
`b` with ~8 live messages after two redactions, `c` with ~8 live
messages. Session one (`a·b`) completes; session two (`a·c`) stalls
**[checked]**. Both sides of session two hold nontrivial, divergent
content: the responder's opening reply disputes one root child (a
`Query` reaction that opens a descent) and requests six others outright
(empty queries, answered by whole-subtree provisions).

Two reductions that bound the trigger **[checked]**:

- The stall is independent of transport backpressure: it reproduces
  identically with an 8 KiB, 64-byte, and 16 MiB duplex buffer. It is a
  logical wait cycle, not a buffer-size problem.
- The stall is a pure function of the tree contents, not of session
  history. After session one, `a1` and `b1` are semantically identical
  (equal shape hash, equal frontier, equal full
  `(key, version, payload)` listing) and behave identically: `b1·c1`
  stalls exactly as `a1·c1` does — with or without session one ever
  running. (An earlier draft of this document claimed a content-equal
  `b·c` control converged; that control was confounded — see §6a.)

## 2. The wait cycle

All the evidence below is from frame-level tracing of the mux/demux
plus await-level tracing of the proxy pumps and materialized walk
stages, on the shrunk case **[checked]**. At the stall, the initiator
machine is idle and healthy; the whole cycle lives on the responder
machine:

    responder machine, final parked states:

    [mat h30]  internal: sending parent resolution   <- no "sent" ever follows
    [proxy h30] pump: decoded; yielding reply        <- no "yielded" ever follows
    [proxy h28] pump: got question; decoding remote reply
    [DEMUX]    stream 1 h30: Supply/End (received)
    [DEMUX]    stream 1: routing...                  <- blocked forever

    initiator machine, final actions before going idle:

    [MUX] stream 1 h30: Supply/End (writing) ... flushed   <- 7th h30 answer
    [MUX] stream 2 h28: Supply/End (writing) ... flushed   <- the h28 answer
                                                              the responder needs

Six links, each individually by-design:

1. The materialized h30→h29 stage is parked in `upper.send()`: the
   one-slot `InternalParentResolutions` channel still holds the
   previous resolution.
2. That resolution is not drained because `assemble` is filling an
   *earlier* resolution's `Pending` slot — the first root child was
   disputed, and its slot fills only from returns that transitively
   require **h28 replies**. Assembly is positional and ordered, so
   later all-`Ready` provision resolutions cannot pass it.
3. The h28 reply the responder needs **is already on the wire** — the
   initiator flushed it — but it sits *behind* two more h30 supplies in
   the single incoming byte stream.
4. The demux (sole reader) is blocked routing h30 supply #7 into
   stream 1's one-slot handoff, so it never reaches the h28 frame:
   head-of-line blocking across logical streams.
5. Stream 1's handoff is full because the h30 pump is blocked yielding
   its previously decoded reply into the one-slot `ProxyResponses`.
6. `ProxyResponses`' consumer is the stage parked in link 1. Cycle
   closed; no waker anywhere; `run_to_quiescence` reports `Stalled`.

## 3. Why every one-slot argument is individually right and jointly wrong

Every capacity argument in `materialized/work/queues.rs` has the form
"if this sender blocks, the buffered item's dependent work is already
launched and can complete independently." In the in-process topology
that premise holds: each height's reply channel is an independent
edge, so a blocked `upper.send` at h30 cannot impede h28 replies.

The wire breaks exactly that premise **[derived]**:

- Both directions are one FIFO. The demux is a single reader feeding 17
  one-slot handoffs, and a full handoff stops the *reader*, i.e. every
  stream at once. Per-stream backpressure does not exist on the receive
  side; only global backpressure does.
- On the send side, `FrameSender`'s receipt fires on *flush*, not on
  remote *consumption*. The in-process topology paces every producer by
  actual consumption (a one-slot channel releases its sender only when
  the consumer takes the item). Over the wire, the transport buffer is
  an anonymous shared queue: the initiator legally runs an unbounded
  number of replies ahead of what the responder can absorb.

Put differently: the local mirror is credit-paced with credit = 1 on
every edge, and the wire session silently replaced consumption-paced
credits with flush-paced acks plus a shared FIFO. The deadlock is the
difference between those two semantics.

This is also precisely the gap between the Rust and the formal model.
`formal/MODEL.md` states "the pump's capacity-1 channel **is** the
wire: nothing else sits between" — i.e. the model's wire is a set of
independent per-height one-slot channels, which is the *local*
topology. The model's `DeadlockFree` target can be (and, per the proof
progress, likely is) true while the deployed composition deadlocks,
because the session layer (demux wire-order coupling, flush-paced
receipts) is not in the model. The fix should be chosen so that the
model's wire abstraction becomes *true of the implementation* again,
rather than papering over the counterexample (§7).

## 4. Why "make the handoffs FAN wide" is not sufficient

Raising `HANDOFF_CAPACITY` from 1 to 1024 makes the shrunk case (and
the full failing suite, presumably) pass **[checked]** — it is a
useful diagnostic and would be an effective *mitigation*. It is not a
bound, for two reasons **[derived]**:

- **The handoff is denominated in frames, not replies.** A provision —
  the answer to an empty query — is one reply carried as a run of
  leaf-supply frames, one frame per leaf in the subtree (since
  superseded: supply runs now batch many leaves per frame, see
  `design/streaming-latency-serialization.md` §10 lever A). A single
  provision parked behind a blocked stream occupies as many slots as
  the subtree has leaves. No constant width covers it.
- **The reply-count backlog is only fan-bounded at the opening level.**
  The responder's opening asks at most one fan of questions in one
  reply, so stream h30 carries at most 256 replies. Deeper streams
  aggregate questions across many parent replies, so their in-flight
  reply count is not bounded by a single fan either.

A capacity bump converts a deterministic deadlock into a rare one and
replaces the deadlock-freedom argument with "we have not generated the
counterexample yet." Given this crate pins its liveness with
`run_to_quiescence` and is mid-way through a kernel-checked
deadlock-freedom proof, that trade seems out of character for the
codebase.

## 5. The fix space

### A. Per-stream credit flow control

> **Superseded as the primary fix by §8's determination.** This
> section remains authoritative for two things: the flow-control
> theory (the window dial, the unit-mismatch discontinuity, the
> sizing math), which §8 inherits as the *contract* every transport
> instantiation must satisfy; and the concrete design of the
> single-socket mux, which survives as an optional future
> instantiation of §8's trait, not as core machinery.

Restore consumption pacing over the wire: a sender may start a new
reply on logical stream S only when the receiver has granted a credit
for it. (The bullets below present the mechanism at its
reply-denominated floor, W = 1, where the soundness argument is purely
structural; the shipped configuration is the byte-window
generalization — see "The window dial" and "Sizing" below.)

- **Grammar.** The dense signal byte reserves values 170–255. A credit
  state at `state 10` occupies `10·17 + stream` = 170..=186: one byte
  per grant, granting one reply on the *peer-spoken* stream `stream`.
  Frames of an already-started reply flow freely (the receiver has
  committed to consuming that whole reply); bare `StreamEnd` is free.
  Reserve `state 11` (187..=203) now for a future byte-denominated
  window update (§ "The window dial") so widening the window later is
  an additive change, not a second wire break.
- **Grant point.** A pump grants stream S exactly when it becomes
  ready to consume S's next reply: for scope-gated pumps, immediately
  after `questions.recv()` returns the scope (it will then sit in
  `incoming.next()` and drain frames promptly — pumps never park
  mid-reply); the opening-question pump grants once at session start.
  This makes the grant the wire twin of "the consumer polled the empty
  one-slot channel."
- **Mechanics.** Grants travel as control frames through the local
  mux (a small control queue, drained with priority, not subject to
  credit). Incoming grants are decoded by the demux and delivered to
  the local mux's per-stream counters (shared atomics + task wake; mux
  and demux already live in one `Drivers::run` select). The mux skips
  streams whose head frame would start an uncredited reply and serves
  any other ready stream — in §2's stall, the h28 answer overtakes the
  withheld h30 supplies and the cycle never forms.
- **Why it is sound.** With credits, a frame arrives only when its
  stream's consumer is already waiting for it, so the demux never
  blocks persistently: the sole reader always progresses, deliveries
  on different streams are independent, and each wire stream behaves
  exactly like the in-process one-slot channel it replaces. The
  materialized one-slot arguments then hold verbatim, and the formal
  model's "the capacity-1 channel is the wire" abstraction becomes a
  true refinement statement about the session layer — extendable in
  Lean as such (§7).
- **Memory.** At the W = 1 floor: unchanged — one in-flight reply per
  stream per direction, plus one control byte per reply of overhead.
  At the shipped byte-window setting: 17·(W + one maximum frame) per
  direction per session (see "Sizing" below) — still O(1) in tree
  size.
- **Throughput.** Per-stream, this is the same one-in-flight pacing the
  in-process design already chose everywhere (capacity 1 on every
  edge); bulk data (provision runs) is a single reply and streams at
  line rate. The window is a performance dial with a hard soundness
  condition — see "The window dial" below.
- **Cost.** V2 wire format changes (V2 is unreleased — it ships with
  this very swap, and V1 remains the selectable oracle); mux/demux and
  pump changes; capture-renderer support for the credit state; the
  gossip/bootstrap/retire snapshots re-accepted as a deliberate
  protocol change; docs for the new invariant in `remote.rs`,
  `session.rs`, and the "why this is deadlock-free" section of
  `materialized.rs`.

#### The window dial

Let W be the per-stream window: how far the sender may run ahead of
the receiver's consumption. The dial's endpoints and its one
discontinuity:

- **W = 1 reply (the correctness floor — not the shipped default).**
  Soundness is structural and free: a pump never parks mid-reply, so
  the frames of the one granted reply drain through the existing
  one-frame handoff with no new buffer anywhere. The cost is that
  *consecutive* replies on one stream serialize at ~1 RTT each —
  grant k+1 leaves only after reply k is consumed — so a level that
  fans n answers down one stream pays ~n·RTT where the (unsound)
  status quo pays ~1. Bulk transfer is unaffected (a provision is a
  *single* reply whose frames stream at line rate regardless of W),
  but the width cost is a genuine regression against the V1 oracle:
  V1 batches an **entire disputed level** into each alternating
  message and completes the descent in ≈ ½·log₂₅₆(2·D·N) exchanges
  (~4 round trips for sets up to 2³², per `alternating.rs`'s cost
  model — a protocol whose documented design tilt is
  latency-dominated links). W = 1 replaces that with Σ_ℓ width_ℓ
  round trips: a thousand-leaf divergence pays ~10³ RTTs where V1
  pays ~4. An earlier draft claimed the opposite (that V1 serialized
  per reaction); that was wrong. W = 1 survives below only as the
  zero-buffer degenerate mode and the model's base case.

- **1 < W in reply units is unsound — the dial's discontinuity.**
  Granting reply k+1 while k's consumer may still park promises
  buffering that cannot be bounded: the granted reply may be a
  whole-subtree provision run (§4). The discontinuity is a **unit
  mismatch**, not a magic number: replies are what the protocol
  counts, but their per-unit buffer cost is unbounded, so a grant
  denominated in replies can never be covered by a buffer denominated
  in frames. In particular, widening the handoff to K frames does
  *not* move the discontinuity — one granted reply exceeds any fixed
  K, and §2's cycle replays with a bigger subtree behind the parked
  stream. W = 1 is the unique reply-denominated point that is sound,
  and for a structural reason rather than a size one: the *actively
  decoded* reply needs zero buffer regardless of length, because the
  pump consumes its frames as they arrive and never parks mid-reply.
  The second granted reply is the first whose frames can arrive while
  the consumer is parked.

  Matching the units — grant what the buffer measures — is exactly the
  wider-window design, and the honest unit is **bytes**, not frames: a
  supply frame's body is an arbitrary `Message<T>`, so a frame-count
  window bounds count, not memory. The receiver grants a per-stream
  byte window it genuinely owns buffer for, the demux deposits without
  ever blocking, and the sender pauses a stream at window exhaustion
  (frame boundaries make mid-reply pauses clean). That is
  HTTP/2/QUIC-style flow control, with window updates coalesced
  (re-grant at half-window consumed) to bound control overhead.
  Soundness shifts from structural ("nothing granted that cannot
  drain") to resource-bounded ("nothing sent that cannot be
  buffered") — still sound, but a strictly harder invariant to state,
  test, and model.

- **What dialing up buys and costs.** Per-stream throughput becomes
  ≈ min(line rate, W/RTT), and a width-n level of small replies costs
  ~⌈n·r̄/W⌉ RTTs (r̄ = mean reply size) instead of n. The price is
  17·W bytes of committed buffer per direction per session, window
  accounting on both ends, and the weaker soundness argument. W = ∞
  is exactly option C, deadlock-free and memory-unbounded — the dial
  interpolates between the fix and the failure mode we rejected.

Independent of W, the session's critical *path* is depth·RTT — each
level's questions causally require the previous level's answers — so
credits only add width-proportional cost, never depth-proportional.

#### Sizing: V1's round-trip class is the budget

The alternating oracle sets the bar the streaming protocol must not
regress from: ≈ ½·log₂₅₆(2·D·N) exchanges, latency-dominated links as
the declared target. V2 shares the two-height stride, so its causal
critical path is the same exchange count; the sizing requirement is
that flow control add **zero width-proportional rounds** in the
common case. Byte windows meet it parametrically:

    extra rounds ≈ Σ_ℓ max(0, ⌈frontier_bytes_ℓ / W⌉ − 1)

which is zero whenever the per-stream window covers a level's
disputed frontier. Concretely:

- **Ship byte-denominated windows as the default.** A disputed node
  ships at most ~5 KB (V1's cost model), so W = 256 KiB per stream
  covers frontiers of ~50 disputed nodes per level outright and
  degrades gracefully — an over-wide level costs ⌈bytes/W⌉ rounds,
  never width rounds. At V1 parity on rounds, V2 still wins on what
  it was built for: fixed memory (no whole-level materialization) and
  cross-level transfer overlap.
- **Memory commitment:** 17·(W + one maximum frame) per direction per
  session — the one-frame overdraft lets a sender finish the frame it
  has started at window exhaustion, since codec frames are atomic and
  a partially received frame is undecodable. This is O(1) in tree
  size, so the out-of-memory-backend goal is intact; W is the knob
  that trades that constant against round trips, per deployment.
- **Initial window:** a protocol constant granted implicitly at
  session open, so the first W bytes per stream flow ungated and no
  handshake round is added. Window updates are coalesced (re-grant at
  half-window consumed): control overhead ≤ 2 bytes per W/2 bytes of
  payload.
- **Environment test:** estimate the largest per-level disputed
  frontier in bytes (≈ disputed nodes × node record size); if it
  exceeds W, each such level costs (⌈bytes/W⌉ − 1)·RTT extra — raise
  W or accept the rounds. `benches/gossip_grid` extended with an
  RTT-shaped transport is where this stops being derived and becomes
  measured.
- **W = 1-reply mode** remains available as a zero-buffer
  configuration for memory-starved deployments, and as the
  structurally-trivial base case the Lean extension proves first —
  but it is a floor, not a default.

#### Credits in-crate vs. demanding a multi-stream transport

The alternative to building flow control is to require one from below:
change the session contract from "any ordered byte pipe" to "a
transport that can open independent ordered streams," and map the 17
logical streams 1:1 onto transport streams. QUIC is the natural
instantiation. The steelman is strong: it deletes the mux, the demux,
the stream field of the signal byte, and the credit machinery; it buys
per-stream backpressure from a hardened, widely deployed
implementation; and it eliminates *transport*-level head-of-line
blocking under packet loss, which credits over one TCP pipe can never
fix (a lost segment stalls all 34 logical streams until retransmit).

Two concessions an earlier draft got wrong, before the comparison:

- **The transport route eliminates this class by construction, not by
  proof.** With transport-guaranteed independent streams, §2's cycle
  is unconstructible no matter what our schedule does: a parked
  stream backpressures only itself. The session-liveness apparatus —
  the mux proofs, the §7.4 model extension, the no-HOL assertions —
  does not "die"; it becomes *unnecessary*, which is the strongest
  possible fix for a bug class. Deleting the mux/demux deletes its
  proof obligations with it.
- **Determinism does not require testing over QUIC.** If the seam is
  a `Transport` trait ("open N independent flow-controlled ordered
  streams"), tests instantiate it with in-memory streams: the
  closed-world `run_to_quiescence` witness, the shrinkable schedules,
  and the behavioral proptest seeds (which pin behavior, not bytes)
  all survive unchanged. Wire pinning becomes per-stream — and the
  capture renderer already treats cross-stream interleaving as
  non-semantic, so per-stream *is* the semantic unit we were pinning
  all along. Only whole-wire byte pins and the mux's own tests go.

The corrected comparison:

| dimension | byte-window credits in-crate | multi-stream transport (QUIC in production) |
|---|---|---|
| who owns the pacing invariant | this crate: windows we implement and prove (§7.4 extension) | the transport: QUIC stream flow control *is* the same byte-window credit system, inherited |
| §2's deadlock class | closed by our proof | unconstructible by contract |
| round-trip profile | V1 parity when W covers the level frontier (see sizing) | V1 parity; windows auto-tuned by the transport |
| public contract | unchanged: any `AsyncRead + AsyncWrite` | new `Transport` trait; plain pipes need an adapter |
| single-pipe support (TCP, TLS, unix, one duplex) | native — the mux is the product | only via an adapter that *is* this same credit mux, obligations included |
| deterministic tests / seeds / model | all survive | all survive over the trait's in-memory instantiation; the session-layer proof burden disappears |
| wire pinning | whole-wire per direction | per-stream (the already-semantic unit); QUIC packetization/encryption unpinned |
| loss behavior | TCP HOL across streams under loss (correctness unaffected) | genuinely independent streams under loss |
| dependencies | none | quinn + rustls + UDP reachability in production |

What remains decisive is the single-pipe row, and it is a **product
question, not an engineering one**: is "reconcile over any ordered
byte pipe" part of this crate's contract? If yes, the byte-window mux
gets built and proven regardless, and QUIC is a second instantiation
of the same seam — streams mapped 1:1, inner credit frames disabled,
the transport's windows supplying the identical invariant. If no —
if every deployment can be handed a multi-stream transport — then
deleting the mux/demux outright is the simpler and *more* certain
fix, and the verification investment moves entirely to the in-process
topology, where the existing Lean proof already lives.

Either way, encode the lesson of how we got here structurally. The
mux/demux was built after deriving — validly for the in-process
graph, falsely for the composition — that per-stream credits were
unnecessary. The seam **"17 independently flow-controlled reply
streams per direction"** turns that one-off derivation into an
interface obligation: every instantiation must supply it, whether by
proof (the credit mux) or by contract (QUIC), and the materialized
walk and proxy pumps never learn which they are running on.

**Verdict:** build the seam first; it is required by both futures.
Behind it, ship byte-window credits (default W = 256 KiB, floor mode
W = 1 reply) if the any-byte-pipe contract stays — which is the
recommendation absent a decision to drop it — and treat the QUIC
binding as the class-eliminating instantiation to adopt when the
dependency posture and deployment allow. The one decision that must
be made deliberately, not by default, is the contract question above.
*That decision has now been made: the contract changes. See §8.*

### B. Capacity bump (mitigation only)

`HANDOFF_CAPACITY = FAN` (or larger). Green today, unsound per §4.
Acceptable only as a stopgap with the hazard documented, and it
weakens `assert_proxy_channels_are_bounded`-style guarantees.

### C. Unbounded per-stream demux buffers

Eliminates head-of-line blocking entirely, but the incoming path then
buffers O(peer divergence) in the worst case, abandoning the
fixed-memory guarantee that is the streaming mirror's reason to exist.

### D. Sender-side scheduling alone

The mux's bottom-most-first policy already prioritizes deep streams,
but scheduling cannot reorder bytes already flushed, and the initiator
flushes h30 answers before the h29 questions that will demand h28
answers even exist. Withholding question-bearing frames mid-reply
instead would stall the peer's whole-reply decode — the materialized
side consumes `Reply` messages atomically — so this route needs a
receiver-driven signal anyway, which is option A.

## 6. Why the lower-level streaming tests never caught this

This surprised us, so it is worth being precise. There are three tiers
of pre-existing test, and each misses the bug for a different reason:

1. **`streaming/tests.rs` (materialized ↔ materialized, proptested
   against the alternating oracle).** No wire exists in this topology:
   each height's replies travel their own one-slot channel, which is
   exactly the independence the deadlock argument needs. This tier
   *cannot* express the bug — the demux that couples the streams is
   not in the composition. Same for the channel-schedule and
   capacity-stress suites: they perturb *timing* and *capacity* of the
   independent edges, never the cross-stream delivery order.

2. **`remote/proxy/tests.rs` (two full peers over a real
   `tokio::io::duplex`, including `symmetric_accepts_match_local` and
   `wire_reconciliation_matches_local`).** This tier has the right
   topology and even a 37-byte transport, and it still passes. The
   reason is purely a generator-distribution gap **[checked]** (§6a
   for how this was established):

   `arb_divergent_pair` builds trees of at most 8 messages. The cycle
   needs a specific conjunction: the responder's opening must ask
   enough questions that the initiator's answer stream outruns the
   ~3 frames of per-stream slack (demux slot + `ProxyResponses` slot
   + the in-flight decode), *and* an early-radix-order child must be
   disputed deeply enough to create a `Pending` that gates assembly,
   *and* enough provisions must queue behind it on the same stream.
   Eight random content-addressed keys rarely produce ≥6 root
   children with the *first* one disputed at depth; 256 cases per
   property never found it. The trigger is otherwise ordinary: the
   deadlocking pair is act-built, single-session, and reproducible at
   this tier directly.

3. **The Lean model.** Models the local topology only, by declared
   scope ("Local backend, no remote transport"); its wire is the
   per-height capacity-1 channel. The bug lives entirely in the
   unmodeled session layer. The proof effort is not wrong — it is
   proving the abstraction that option A would make true.

The general lesson: every tier below the integration tests validated
the protocol against a wire that preserves per-stream independence.
The one component that destroys that independence — the shared-FIFO
demux with blocking handoffs — was only ever exercised by unit tests
of its own mechanics, never inside a composition whose *liveness*
depends on cross-stream ordering.

### 6a. A ruled-out hypothesis: fingerprint-equal replicas diverging

An earlier draft claimed that a direct `b·c` session with "the same
content" converged while `a·c` stalled, and hypothesized that
gossip-built replicas carry internal state the `(hash, latest)`
fingerprint does not pin. That would have been a much bigger bug than
the deadlock — equal fingerprints must be behaviorally
indistinguishable — so it was investigated to ground truth. The claim
was wrong; the control was confounded.

The confound: in the failing case `a`, `b`, `c` are the first, second,
and third forks of one seed. The "control" never created `a`, making
`b` the *first* fork — a different party region, therefore different
leaf versions, therefore different content-addressed keys, therefore a
different trie shape. It was never the same content.

The corrected experiment matrix, each cell deterministic under
`run_to_quiescence` **[checked]**:

| experiment | result |
|---|---|
| `a1` vs `b1` after session one: shape hash, frontier, full `(key, version, payload)` listing | all equal |
| `a1·c1` (the original failure) | stalls |
| `b1·c1`, same universe, after session one | stalls identically |
| `b·c`, same universe layout, session one never run | stalls identically |

Conclusions: the fingerprint invariant is intact — semantically equal
replicas stall or converge in lockstep; session history is irrelevant
(the deadlock is a pure function of the two trees and the elected
roles); and there is no hidden gossip-reachable state, consistent with
the node representation, where `ceiling`/`floor` are memoized pure
functions of the leaf versions and the shape hash plus leaves plus
root ceiling exhaust the semantic state. The integration tests found
the bug before the proxy proptests did only because chaining sessions
over forked universes happens to explore a richer distribution of
tree shapes — not because sessions mint novel state.

Cautionary note for future controls in this codebase: two replicas are
comparable only if built in identical universes — fork order allocates
party regions, and party regions determine keys, so *any* difference
in fork order changes every tree shape downstream.

## 7. Preventing regression once the fix lands

The items below predate §8's determination; §8.7 maps each onto the
transport-contract world (most survive with their target renamed; the
HOL-specific ones become conformance obligations on instantiations).
Ordered by how directly each pins the property:

1. **Commit the two proptest seeds** (already done) — they replay the
   original stall via the ordinary suite forever.

2. **A deterministic session-layer liveness fixture.** A proxy-tier
   test with a hand-built pair: one root child disputed two levels
   deep and listed first in radix order, six-plus provision children
   behind it. Drive `reconcile_symmetric_accepts` under
   `run_to_quiescence` with a small duplex. Today this deadlocks; with
   the fix it must converge. This is the counterexample from §2 made
   permanent, at the tier that should have owned it.

3. **Close the generator gap at the proxy tier.** Raise the wire
   proptests' size budget (more messages per side), and add a strategy
   biased toward the trigger geometry: wide roots with an
   early-radix-order dispute descending several levels, provisions
   behind it. Per §6a no session-chaining is needed — act-built pairs
   reach every relevant shape; the budget and bias are the whole gap.

4. **Extend the Lean model with the session layer** — the demux as a
   process performing wire-order sends into per-stream one-slot
   channels, the mux as the merge, credits as modeled messages. The
   deliverable is the refinement the model currently assumes: *under
   the credit discipline, each wire stream is observationally a
   capacity-1 channel*. That turns §3's gap into a theorem instead of
   a caveat. (Without credits, the extended model should *refute*
   progress on §2's skeleton — a good validation that the extension is
   faithful.)

5. **An instrumented no-HOL assertion.** The test-only channel layer
   already reports blocked-send polls per queue. Add the demux handoffs
   to that reporting and assert, in the wire proptests, that no demux
   handoff records a blocked send while any *other* stream had an
   undelivered frame pending — i.e., assert the independence invariant
   itself, not just the absence of its worst consequence. Cheap, and it
   catches near-misses that happen not to complete the cycle.

## 8. The determination: a stream-capable transport contract

Decided 2026-07-17: the library requires a transport that can
**accept and connect streams, abstractly**, and the caller is
responsible for providing it. Instantiations include in-memory
channels (tests), QUIC (native sub-streams), and multiple TCP
connections behind a caller-side router. The in-crate mux/demux stops
being core machinery; §5A's byte-window design survives only as the
optional single-socket instantiation, buildable later if a
one-socket deployment ever demands it.

The elaboration below is organized around the one genuinely tricky
part the determination names: the **double-layered bundling** — a
process holds many `Rumors` handles, each handle holds sessions with
many remote peers, and each session needs ~35 streams. QUIC natively
bundles streams-within-connection; nothing natively bundles
sessions-within-listener for plain TCP, so that layer needs explicit
machinery. The design splits the two layers so the protocol only ever
sees the inner one.

### 8.1 What the protocol requires of a stream

Per session, the protocol consumes:

- **One bidirectional control stream**, carrying (in order) the
  preamble, the causal-version handshake, and any trailing party
  frames (bootstrap/retire hand-off). The control stream must exist
  *before* speaker election — the election is computed from the
  version exchange — which is why it cannot be one of the labeled
  data streams. Two corollaries fall out free: the equal-version
  short circuit opens **zero** data streams, and the V1 alternating
  oracle runs entirely *on* the control stream, unchanged, so
  protocol negotiation and the oracle survive the migration intact.
- **Up to 17 outgoing and 17 incoming unidirectional data streams**,
  established lazily as the descent requires them (§8.3 — typical
  sessions materialize a handful), reliable and ordered individually,
  with **no ordering guaranteed across streams** — the protocol must
  not assume any (QUIC does not provide it; this is the formal
  restatement of what §2 taught).
- **Independent, receiver-paced, bounded-buffer flow control per
  stream**: writing to stream S may block only on S's receiver;
  reading S never depends on any other stream's progress. This is
  §5A's invariant verbatim, relocated from "thing we implement" to
  **contract clause every instantiation must satisfy** — natively
  true for QUIC (stream windows), TCP-per-connection (per-socket
  windows), and bounded in-memory channels; the obligation §7.4's
  model extension discharges for the mux instantiation only.
- Half-close semantics: a data stream ends (writer finishes; reader
  observes end-of-stream); the control stream outlives all data
  streams of its session.

### 8.2 The two layers, and who owns each

**Layer B — the trait the protocol consumes (library-owned).** Scoped
to one *link*: one `Rumors` handle × one remote peer, the unit
`gossip_when` already manages. Sketch:

```rust
/// One link's stream supply. Contract clauses are load-bearing;
/// see the conformance suite (§8.7).
pub trait Link: Send {
    type Control: AsyncRead + AsyncWrite + Unpin + Send;
    type Connector: Connector + Send + Sync;
    type Acceptor: Acceptor + Send;

    /// Split into the three concurrency roles. The halves live as
    /// long as the link: `Control` is the persistent control
    /// stream; the connector and acceptor serve every session on it.
    fn split(self) -> (Self::Control, Self::Connector, Self::Acceptor);
}

pub trait Connector {
    type Tx: AsyncWrite + Unpin + Send;
    /// Open one outgoing unidirectional stream.
    async fn connect(&self) -> io::Result<Self::Tx>;
}

pub trait Acceptor {
    type Rx: AsyncRead + Unpin + Send;
    /// Accept one incoming unidirectional stream, in arrival order.
    async fn accept(&mut self) -> io::Result<Self::Rx>;
}
```

Deliberate choices, each carrying a rationale:

- **Split halves, one per concurrency role** (determined 2026-07-17;
  an earlier sketch put `control`/`connect`/`accept` on one `&mut self`
  receiver, which cannot work). The session drives everything under
  one future with no spawn, and up to 17 encoders open streams
  concurrently while the accept loop runs: three `&mut self` methods
  on one object cannot be awaited from concurrent branches of that
  future. The shape mirrors the actual concurrency profile.
  `connect(&self)`: encoders share one connector by reference and
  connect concurrently — QUIC's `open_uni(&self)`, a channel sender,
  and a
  TCP dialer all provide this natively, and `&self` needs no `Clone`
  bound, which keeps the half boxable if the §8.9 llvm-lines gate
  demands `dyn` halves. `accept(&mut self)`: owned by the single
  claim loop that feeds the accept table — serial by design.
  `Control` owned outright: sequential by protocol (handshake before
  election, party frames after the data streams conclude). The
  rejected alternative — a coordinator task owning the link and
  serving connect/accept requests over channels — would rebuild a
  single serving loop funneling to many destinations *inside* the
  crate, the very §8.5 shape the contract exists to delete, and
  would pay channel plumbing for concurrency every known
  instantiation already provides.
- **The control stream is persistent and the link is long-lived.**
  Successive sessions on a link are serialized (as `gossip_when`
  already serializes them), and each new session's preamble rides the
  same control stream — so the existing `Staged` race ("remote
  preamble arriving vs. local tick") ports directly: remote
  initiation is bytes appearing on `control`, exactly as today.
  Data streams are opened fresh per session.
- **Streams are anonymous at the trait; the protocol labels them.**
  The first bytes the protocol writes on every opened data stream are
  a label: `(session epoch, stream index)`. Rationale: it keeps the
  trait minimal and instantiation-agnostic (a QUIC binding needs zero
  label logic — the label is just payload), it puts validation in one
  place, and it means `accept()` may yield streams in any order — the
  session keeps a bounded (≤ 17 entries) claim table pairing accepted
  streams with the pumps awaiting them. Set-up-time only, no cycles:
  pumps produce nothing that accepting depends on.
- **The epoch byte is cheap insurance, not load-bearing.** Protocol
  completion requires every data stream's end to be observed, so a
  finished session cannot leak stragglers into the next; the epoch
  exists to convert any violation of that reasoning (or a buggy
  instantiation reordering across sessions) into a loud validation
  error instead of a misrouted stream.
- **Unidirectional, not paired.** 17+17 uni streams match the
  protocol's asymmetric stream sets (the speakers' height schedules
  do not pair up). An instantiation may internally carry two logical
  streams on one bidirectional carrier as an optimization, but the
  trait does not know.

**Layer A — links out of connectivity (caller-owned, helpers
provided).** How a process turns its transport into `Link`s for its
handles. This is where the double bundling lives, and it is
*deliberately outside the core crate*: the core stays
zero-network-dependency; bindings ship as feature-gated modules or
sibling crates with the router helper among them.

### 8.3 Lazy stream establishment

Eagerly opening 17+17 streams per session would be mostly waste:
in the session-one trace, each speaker used streams 0 and 1 and closed
the other fifteen with a bare `End(Stream)` — thirty connections
dialed, for TCP-per-stream, to say "nothing here." The schedule
should create streams as reconciliation requires them, and the
protocol's structure makes that a *local* decision at both ends — no
new wire traffic, no trait change.

**The load-bearing fact:** a stream's replies answer questions its
*receiver* asked, and both ends already sequence on that question
flow before touching the stream. The receiving pump awaits
`questions.recv()` (an acked local question) before its first read of
`incoming`; the sending encoder awaits `requests.next()` before
producing its first frame. So nonemptiness is known on each side
before the stream is needed:

- **Sender:** open on the first request (before the reply is even
  computed, so a TCP dial overlaps the compute and the causal chain);
  a level with zero requests never opens its stream, and its encoder's
  `finish` skips the close it has nothing to close.
- **Receiver:** claim from the accept table on the first scope; a
  level whose scope flow closes with zero scopes never claims — its
  pump's loop already exits without reading, and the vacuous
  `reject_extra` is skipped rather than awaited.

Laziness is therefore a policy change in *when* `connect()` is called
and *when* a pump claims — the dataflow graph, the pump loops, and
the per-stream frame grammar are untouched. Typical sessions
materialize ~2–6 data streams instead of 34 (one speaker-pair per
*active* two-height stride); the worst case remains 34 and the
contract keeps that as the concurrency requirement.

**What it changes on the wire:** empty streams cease to exist, rather
than opening to carry one `End(Stream)`. Nonempty streams are
byte-identical to today per stream, preserving §8.6's snapshot
re-derivation property; empty-stream capture groups are dropped
deliberately. One validation softens: today an empty stream's
explicit end is checked eagerly (`reject_extra`); lazily, "the peer
opened a stream we expected vacuous" is mostly not detected at all.
A valid-label unasked stream delivered into a live-but-never-polled
claim slot sits there undetected until that level's claim receiver
drops — nearly the whole session; the `Unexpected` violation fires
only when delivery finds the receiver already gone, late in the
termination cascade. This is detection latitude, not safety — unasked
replies were never absorbable, and the parked stream's memory is
bounded by its own link stream's buffers — but it is a real loosening
of the violation surface and worth stating.

**Teardown:** the accept loop runs until every claimed stream has
concluded and every level's scope flow has closed; at that point any
pending `accept()` future is dropped and any unclaimed arrival is a
violation. The trait contract gains the corresponding clauses rather
than any new method: (i) `connect()` calls are sparse, mid-session, and
bounded by 17 per side — an instantiation must not require or await
a full complement; (ii) pending `accept()` futures are dropped at
session teardown and instantiations must tolerate cancellation
there; (iii) stream limits must admit 34 concurrent streams even
though typical sessions use a handful.

**Reuse across sessions: transport-level yes, protocol-level no.**
Data streams are semantically session-scoped; only the control stream
persists (it is the link's identity). Protocol-level reuse would buy
only the dial cost — a cost one instantiation pays — while charging
everywhere: session k+1's cleanliness would depend on session k's
shutdown (and an aborted session poisons every warm stream, so full
teardown logic survives anyway); the epoch byte would be promoted
from defense-in-depth to the sole session delimiter, and a violator's
bytes could sit unread in a warm stream until a later session claims
it; QUIC's idiom (fresh, near-free streams whose new IDs provide the
epoch property structurally) would be pessimized to favor the
degenerate instantiation; and the verification unit — one session,
one skeleton — would acquire cross-session obligations. The dial
cost is instead recovered where it lives: the TCP binding may pool
*connections*, handing `connect()` a warm socket under a per-lease
header (layer A's private framing), returning sockets to the pool
only at clean frame boundaries and killing them on abort — keep-alive
for the transport identity, fresh semantics above. A pooled lease
costs single-digit bytes more than a semantically reused stream and
none of its coupling.

A refinement considered and deferred: speculatively pre-opening
level h's stream when level h+2 shows disputes would hide even the
overlapped dial latency. It is deferred because the bet is on remote
state and loses in the common case. Your h stream is used iff the
peer's h+1 reply contains `Query`/`QueryEmpty` reactions, and their
reply may instead resolve every queried child by `Match` or inline
`Supply` — facts about *their* tree, unknowable until the reply
arrives, which is exactly plain laziness's trigger. One-directional
catch-up (the most ordinary gossip shape) resolves by inline supplies
one stride after every dispute, so the heuristic would dial mostly
for sessions that never use the stream. A wrong open is not free,
either: an opened, unused stream must still be labeled and explicitly
closed to stay distinguishable from a peer stalled mid-label — the
empty-stream traffic §8.3 removed, re-paid per miss. QUIC and
in-memory opens are near-free, so the heuristic could only ever
matter for TCP-per-stream; revisit only if that instantiation's
measured level latency demands it.

### 8.4 Instantiations, layer A shape for each

**In-memory (tests).** A `Link` from bounded channels / duplex pairs.
Fully deterministic under `run_to_quiescence`'s closed world; this is
what the entire existing verification apparatus runs on, unchanged.

**QUIC.** Two viable mappings:

- *Connection per link* (recommended, and the determination's
  framing): the caller dials/accepts one QUIC connection per
  (handle, peer) link and hands it off; `control` is the first bidi
  stream, `connect`/`accept` map 1:1 onto QUIC unidirectional streams.
  No router, no session tokens — the connection *is* the bundle, and
  QUIC's stream flow control supplies the §8.1 contract natively.
  Cost: one connection per link; with H handles × P peers that is
  H·P connections, which QUIC is designed for (cheap keep-alive,
  0-RTT resumption), but worth stating.
- *Shared connection per process pair*: one connection multiplexing
  many links; every stream then carries a link token ahead of the
  protocol label and a thin router in the binding demultiplexes.
  This reintroduces router machinery (below) for a connection-count
  saving; take it only if H·P connections measurably hurt.

**TCP, one connection per stream.** The instantiation the
determination sketches, and the one that genuinely needs the central
router:

- Each `connect()` dials the peer's listener and writes a **connect
  header**: `(link token, epoch, stream index)` — note this header is
  layer A's (routing) concern and is distinct from the protocol's
  label; with TCP carrying both would be redundant, so the binding
  may fold them, but the *ownership* is different and QUIC's binding
  has only the label.
- One **router task** owns the listener: accept, read the header,
  push the socket down a per-link mpsc to that link's `accept()`
  side. The control stream is the link's first connection.
- **Cost accounting, stated plainly:** with §8.3's lazy establishment,
  typically 3–7 sockets per session (control + one pair per active
  two-height stride), worst case 35, re-dialed per session (data
  streams are per-session; the control connection persists with the
  link). Dials overlap the descent's causal chain, but each active
  level's first reply still eats a TCP+TLS setup unless resumed.
  This instantiation is the *compatibility* option, not the
  performance option; deployments that can't take QUIC and can't
  afford per-stream sockets are exactly the future customer of the
  §5A mux instantiation.

**Single socket (future).** §5A's byte-window mux, packaged as a
`Link` implementation over one `AsyncRead + AsyncWrite`. All of §5A's
machinery and §7.4's proof obligation live here and only here.

### 8.5 The router discipline: the HOL lesson, one layer up

The central router is a single loop funneling streams to many
destinations — structurally the same shape as the demux that caused
§2. The same failure is available: if the router ever *awaits* a full
per-link queue, one stalled link head-of-line blocks every link on
the process. The discipline, which belongs in the router helper's
rustdoc as a hard rule and in the conformance suite as a test:

- **Never await a per-destination queue.** Bound each link's queue at
  the protocol maximum (17 data streams + 1 control per live session;
  serialized sessions make this a true bound), and treat overflow as
  a protocol violation by that link's peer — kill that link, never
  block the loop.
- The bound is sound for the same structural reason W = 1 was: a
  well-behaved peer cannot have more than a session's worth of
  streams in flight on one link, so a full queue *proves* misbehavior
  (or a local bug), and the correct response to misbehavior is
  eviction, not backpressure that punishes bystanders.
- The router hands off *sockets*, not bytes: after the header read,
  the connection never touches the router again, so per-stream flow
  control stays end-to-end between peer and pump. (A router that
  proxied bytes would silently become a mux and inherit §5A's entire
  problem statement.)
- Header reads are the router's only I/O on a connection and must be
  bounded (size and time): a peer that dials and stalls mid-header
  must not park the accept loop — header reads run as small spawned
  tasks feeding the routing step, with the same never-block law
  applied to their results.

### 8.6 What this changes in the crate

- **Deleted from core:** `session/` (mux, demux, coordinator's
  drive-both-directions loop), the handoff arrays, `DemuxError`'s
  HOL-adjacent variants. The per-stream pumps read/write their own
  streams directly: `FrameRead`/`FrameWrite` per stream, receipt =
  local flush (the publish-after-flush ordering is preserved as-is).
- **Codec:** keep the existing frame grammar per stream —
  permanently, not just for migration. The frame-kind state needs a
  byte regardless, so the dense `state × 17 + stream` packing makes
  the stream component *free* redundancy; per-stream decoding
  upgrades its validation from today's 17-way admission to **exact
  equality** with the claimed label (plus the existing per-class
  placement grid), rejected on mismatch, never advisory. This is
  defense-in-depth across caller-owned layer A: a router miswire, a
  pool lease crossing, or a claim-table bug surfaces at the first
  frame as a precise `labeled j / framed k` error instead of as
  garbled protocol. It also keeps per-stream frame bytes identical
  to today's captures, so the snapshot corpus re-derives by
  re-grouping. (The re-derivation audit confirmed populated streams
  byte-identical, with exactly two deliberate wire deltas: empty-
  stream capture groups are deleted outright — §8.3's lazy
  establishment — and each opened stream gains the 2-byte
  epoch/index label, which the snapshots do not hex-pin; the label
  bytes are pinned by their own unit test.) A denser state-only
  byte would save zero bytes and delete the tripwire; that idea is
  retired. The miswiring coverage landed at the streams tier rather
  than as a conformance-suite check: the mislabeled-frame unit tests
  (`src/tree/mirror/streaming/remote/streams/tests.rs`) pin that a
  wrong label surfaces as the precise mismatch error on frame one,
  which discharges the obligation the suite was once promised.
- **Public API:** `gossip(read, write)` → `gossip(link)`; likewise
  `bootstrap`, `retire`, `gossip_when`. The transport type erasure
  (`DynRead`/`DynWrite`) is rethought at the `Link` boundary — with
  §8.2's split shape, erasure is per-half (boxed `Control`/
  `Connector`/`Acceptor`, the `&self` connector needing no `Clone`
  bound to stay
  boxable) — to keep the monomorphization cap the current design
  bought.
- **`gossip_when`:** unchanged in shape — the select races
  `control`-stream bytes against the tick stream, `Staged` and the
  suppression token as today.
- **Party hand-off and bootstrap:** trailing party frames ride the
  control stream after the data streams conclude, preserving the
  current lifecycle ("raw reader positioned at the trailing party
  frame" becomes "control stream positioned after reconciliation").

### 8.7 Verification under the contract

- **The §2 class:** unconstructible for QUIC, TCP-per-stream, and
  in-memory instantiations — independence is supplied by contract,
  not proof. The class survives only inside the future mux
  instantiation, which inherits §5A + §7.4 wholesale, quarantined.
- **§7's items, mapped:** (1) the proptest seeds are behavioral and
  survive unchanged over the in-memory `Link`; (2) the deterministic
  HOL fixture becomes *two* artifacts — a conformance test asserting
  liveness under an adversarial-but-legal `Link` (tiny flow-control
  windows, worst-case accept reordering, per §8.1 the protocol must
  tolerate both), and a mux-instantiation regression test if/when
  that ships; (3) the generator-budget work is unchanged; (4) the
  Lean extension's target renames: model the `Link` contract as the
  wire (which is exactly the model's *current* abstraction — the
  determination makes the model's premise true by construction), with
  the mux instantiation as the one component still owing a refinement
  proof; (5) the no-HOL channel assertion moves to the router helper
  and the conformance suite.
- **Export the conformance suite.** Layer A is caller-owned, which
  means callers can build routers that reintroduce coupling. Ship the
  contract tests (`Link` liveness under reordering, tiny windows,
  serialized-session discipline, router never-block law) as a public
  test harness so a caller's instantiation can be validated the same
  way ours are.

### 8.8 Open questions

Status 2026-07-17: all four dispatched by the §8.9 execution
decisions — the first two deferred (they attach to bindings that do
not ship in round one), the third scheduled, the fourth already
answered in place.

- **Connection budget for QUIC-per-link** at H handles × P peers —
  acceptable for expected deployments, or does the shared-connection
  binding (with its token router) need to ship in the first round?
  *Deferred: no QUIC binding in round one (§8.9); decide when
  `rumors-quic` is built.*
- **TLS/authn placement for the TCP router**: the header is
  peer-controlled pre-auth input; does the router sit inside an
  authenticated layer (TLS first, then header) or does the header
  bind to an authenticated identity? Caller's domain, but the helper
  must not make the wrong thing easy. *Deferred with the TCP binding
  (§8.9).*
- **Erasure at the `Link` boundary**: confirm the monomorphization
  budget (the `design/height-erasure.md` concern) with a `dyn`-safe
  or internally-erased `Link` before the API freezes. *Scheduled:
  §8.9's sequencing keeps the erasure project separate but gates the
  trait freeze on a `cargo llvm-lines` spot-measurement.*
- **Whether the mux instantiation is ever built** — it should exist
  as a design (§5A) and a reserved seam, not as speculative code.

### 8.9 Execution decisions (2026-07-17)

Round-one scope and discipline, decided with Finch before execution:

- **Round one ships two public artifacts and no network binding**:
  the in-memory `Link` (zero-dependency, in core — the doctests,
  examples, and the whole verification apparatus need it, and it
  serves in-process use directly) and the §8.7 conformance suite as
  public API, so a caller-built `Link` is validatable from day one.
  Until a binding ships, real-network deployments implement `Link`
  against that suite.
- **Network bindings live in sibling crates** when they ship
  (`crates/rumors-quic`, `crates/rumors-tcp`): core stays
  zero-network-dependency in fact rather than by feature flag, and
  quinn/rustls version posture never touches core.
- **The refactor runs on a branch.** Intermediate commits may fail
  only the two committed deadlock seeds (everything else gate-clean);
  merge to main only fully green. Main's history gains no new red
  states beyond the one `83edcd94` introduced.
- **Link first, erasure after.** The correctness fix does not wait on
  `design/height-erasure.md`; that project follows separately. The
  one coupling honored now: the `Link` trait is designed
  dyn-friendly, and the API freeze is gated on a `cargo llvm-lines`
  spot-measurement confirming the boundary doesn't re-open the
  monomorphization budget.
- The Lean-model retarget (§8.7 item 4) proceeds as its own track in
  `formal/`, not inside this Rust execution.

### 8.10 Implementation addendum (2026-07-17, the `link-transport` branch)

Deviations from §8's sketches, discovered and decided during execution;
each is deliberate and the code documents it in place:

- **`Link` is a concrete generic struct, not a trait.** The trait bought
  nothing over a plain bundle `Link<CR, CW, C, A>` of four halves — every
  instantiation constructs the struct from whatever it has — so only
  `Connector` and `Acceptor` remain traits, and `split()` is unnecessary.
  Wrappers (fault injection, capture, adversity harnesses) decompose and
  reassemble via `LinkParts`, whose fields are public for exactly that.
- **Control is two half type parameters, not one
  `AsyncRead + AsyncWrite` object.** The preamble and the causal-version
  handshake genuinely read and write concurrently — §8.2's own
  halves-per-concurrency-role rationale, applied one level further down.
- **`Connector: Clone + Send + Sync + 'static`.** The protocol layer's
  response streams are `Send + 'static` throughout, so encoder tasks must
  *own* their stream supply; each holds a clone. This supersedes §8.2's
  "no `Clone` bound" note; the boxability that note protected survives via
  `Arc<dyn>` erasure, which is `Clone` for free. The acceptor, single-
  consumer by design, stays borrowed inside the session's one
  non-`'static` driver seat — where the deleted mux/demux drivers sat.
- **The epoch is a wrapping `u8` on the link.** It is a tripwire, not an
  identity: correctness rests on serialized sessions plus every claimed
  stream's observed end. Aliasing 256 sessions later requires either a
  transport that held a stream undelivered across 256 serialized sessions
  (contract violation) or a peer already inside the trust boundary, where
  lying in-protocol is easier than epoch collision. The counter now rides
  inside the link's `SessionState`, alongside the poison latch the next
  bullet introduces.
- **Session-boundary integrity is enforced twice, by two mechanisms that
  answer different questions.** The V2 *epilogue* — one marker byte each
  way on the control stream after all local session work — is
  peer-completion certification: it upgrades `Ok` from "my replica
  committed" to "both replicas committed", leaving only the irreducible
  two-generals residue (`Error::Epilogue`, distinguished as post-commit).
  Link *poisoning* is local fail-fast: the link's `SessionState` latches
  `poisoned` when a session begins and clears it only on clean
  completion, so a session that failed or was cancelled mid-frame leaves
  a latch that fails the next session immediately (`Error::LinkPoisoned`)
  instead of misparsing leftover control bytes — turning the old
  "discard the link on `Err`" documentation into an enforced contract.
  Neither subsumes the other: the epilogue says nothing about a session
  that never ran to its own end (cancellation is invisible to the wire),
  and poisoning says nothing about whether the *peer* committed (absent
  the epilogue, the local latch would clear on local success alone).
  Cheapest possible forms of each: one epilogue byte each way per session
  (two on the wire), two bytes of state on the link.
- **Stream ends are double-checked.** After the explicit `End(Stream)`
  control, the receiver requires transport end-of-stream; a peer that
  keeps talking past its own end surfaces as `StreamError::AfterEnd`
  rather than going unnoticed. Costs nothing against an honest peer (the
  sender half-closes immediately after the control) and recovers the old
  demux's frame-after-end detection, which §8.3's latitude had ceded.
- **Stream-supply failures are deferred, not immediately fatal.** A peer
  that completed its session cleanly has already delivered every stream
  this side will claim, and may drop its link while this side finishes.
  The accept driver therefore parks on transport-class failures (dropping
  undelivered claim slots); a pump that provably needed one fails the
  session with `StreamError::SupplyClosed`, carrying the deposited I/O
  cause. Label/epoch/duplicate/unexpected violations remain immediate.
- **Lazy establishment shipped from the start**, per the execution
  decision: senders open on their first frame, receivers claim on their
  first read, vacuous levels never touch the transport, and wire captures
  are per-stream with empty-stream groups gone.
- **Conformance shipped as a documented `conformance` cargo feature** on
  the core crate (public module, panics-on-violation check functions plus
  focused per-clause probes), validated against the in-memory link at
  default and one-byte windows and under batched accept reordering. Two
  real instantiations already exercise the seam: a per-stream TCP link in
  the integration tests' inter-process disruption suite (one listener per
  session side, ports swapped on the control stream — §8.4's
  compatibility mapping minus the router), and rumormill's iroh/QUIC
  binding (connection per link, streams one to one), which needed no
  compatibility shims.
- **The router never-block conformance test is deferred with the router
  helper itself.** §8.7's suite ships without it because the helper it
  would validate does not exist yet; the obligation travels with whichever
  sibling crate ships the first shared-connection router
  (`rumors-tcp`/`rumors-quic`), so the promise has a recorded owner
  rather than a silent gap.
- **The §8.9 llvm-lines freeze gate is satisfied.** Measured on the
  branch: `cargo llvm-lines` on `--test pairwise` totals 2,040,751 lines,
  unchanged from `design/height-erasure.md`'s 2.04M baseline. The
  per-half erasure funnel (`Arc<dyn>` connector, borrowed `dyn` acceptor)
  holds the monomorphization cap the trait freeze was gated on.

## Appendix: the throwaway repros

Both probe files were deleted from the tree after diagnosis (the
committed seeds subsume them). For reference, the minimal shape that
reproduces at will, in `tests/`-style code using the shared helpers —
note per §6a that session one is unnecessary, but the empty first fork
is not (fork order determines party regions and therefore every key):

```rust
let seed = rumors::Peer::<u64>::seed().into_rumors();
let _a = build_local(bootstrap_fork(&seed), &[]);       // fork 1: layout only
let b = build_local(bootstrap_fork(&seed), &b_actions); // 9 inserts, 2 redacts
let c = build_local(bootstrap_fork(&seed), &c_actions); // 8 inserts, 1 redact
wire_gossip(&b, &c);          // stalls: run_to_quiescence -> Stalled
```

with `b_actions`/`c_actions` exactly as recorded in
`tests/pairwise.proptest-regressions` (seed
`b08c93d6c5a24c78269bda9043cd12070fd70ebf95248c6f9447f4c8a6629490`).
The §6a experiment matrix lived in a temporary `fingerprint_probe.rs`
built from the same action lists, asserting semantic equality of
`a1`/`b1` and the stall of every same-universe pairing.
