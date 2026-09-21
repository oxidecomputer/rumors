# Appendix: implementation plan

For a reader who knows the code. The [exposition](exposition.md) gives the
argument; this is the route from `772cce34` to sessions over one pair of
read and write halves. The default sequence instruments the count, changes
reception on the existing transport, demonstrates one pipe, then changes
the API. Each commit includes the tests and documentation needed to keep
that intermediate tree gate-clean.

The removal of `Link`, by-value ownership of the halves, their return on
success, and support for callers passing mutable references are settled
by the owner's 2026-09-20 ruling. The pipe module's name and the measured
choice of mux policy remain implementation choices.

Here, *walk* means the materialized participant
(`streaming/materialized/`) and *proxy* the wire-bound participant
(`streaming/remote/`). Code height runs from leaves at 0 to the root at
32. A node at address depth `d` has height `32 − d`. In the interior
descent, an exposition question at level `ℓ` lists or reacts to children
of height `h = 32 − ℓ`; its subject node has height `h + 1 = 33 − ℓ`.
A terminal request concerns a leaf at height 0, with no children to list.
Below, `K(ℓ)` always means
the capacity of the local question-record queue for that logical level.
When joining code counters and constructors, distinguish child height,
subject height, and codec stream index; the opening and terminal streams
also have explicit roles. The convention is uniform: the walk's question-queue constructors, `Window::capacity`, and both `Progress` hooks (`wire_reply` and `decoded_reply`) take the *children's* typed height, `h = 32 − ℓ`, so one label names the same logical queue everywhere.

## What does not change

The walk's exchange rules: `yield_resolve_query!`, its publication orders,
the assembler chain, the `Window` search for an affordable width, population
bounds, version-size pricing, greeting, `Reply`/`Reaction` vocabulary,
frame signal grammar (`codec/signal.rs`), supply runs and `RunBudget`,
and causal sieve. In-process tests connecting two walks through
`mirror_connected` remain. The window's byte prices and granted widths do
change; the receive proxy changes scope custody. Existing protocol
violations remain applicable, with parking overflow handled explicitly.
Transport-specific errors disappear with the machinery that produced them.

## The rule for full queues: violations, not waits

The never-full argument turns on one discipline, applied everywhere on the
receive path: **the demultiplexer, and the decoders it feeds, never wait on
a condition the peer controls.** Every such condition is either impossible
for a conforming peer, in which case its occurrence is a typed violation
that fails the session, or a bounded local computation, in which case
waiting is fine. Today's per-stream transport tolerates a decoder that
waits on the peer (only that stream stalls, and the application's deadline
ends it); on one socket that same wait blocks every level, so each site
gets a rule. The sites, with the rule at each:

- **Parking at capacity on arrival** (`ProxyResponses`, widened in step
  1). A reply arriving for a level whose queue already holds
  `K(h) + FAN + 1` is a reply nobody asked for. Violation, the ingress twin
  of `Violation::UnaskedReply`; never a wait. Apply it in step 1 already,
  even though on the multi-stream link the wait would be harmless, so the
  invariant is tested there first.
- **A reply with no question to pair it with.** The decoder dequeues a
  scope from `local_questions` before decoding a reply at that height. If
  frames for height `h` arrive when no question at `h` has been flushed,
  waiting for the scope would block the demultiplexer forever on a
  non-conforming peer. Keep a per-height count of questions flushed (the
  encoder bumps it after `write_reply`) and of replies begun (the decoder
  bumps it at each reply's first frame); a reply begun beyond the flushed
  count is a violation at the frame that begins it. A scope that is
  merely late (flushed, not yet published because the encoder is between
  the flush and the publish) is a bounded local wait and stays a wait.
- **The opening batch.** Its decoder holds at most one fan of decoded root
  children, and the responder computes the batch's membership from the two
  listings. A supplied root child outside that set, or a second supply for
  one already received, is a violation at the decoder, not a full queue to
  wait on.
- **Supply overdraw and frame bounds.** Existing violations, unchanged: a
  supply beyond the greeting's declared set length (`SupplyLedger`), a run
  beyond the negotiated frame bound, a malformed frame, an unknown stream
  index.
- **Local queues that only this side fills.** `local_questions` and
  `next_scopes` are filled by this side's own encoder and typed exit;
  overflow there means the walk over-asked or the release argument is
  wrong, a crate bug, so a full queue is a `debug_assert`, never a wait
  and never a peer violation.
- **Bounded local waits that stay waits.** The capacity-1 handoff from the
  demultiplexer to a height's decoder (full only while that decoder is
  mid-frame, and the decoder depends only on the backend); the decoder's
  fan-bounded leaf channel into `Backend::assemble`; and, on the sending
  side, the capacity-1 frame channels into the multiplexer and the walk's
  own question and resolution queues, all of whose waits point at this
  side's own progress and are covered by the walk's argument.

Step 0's instrument should count arrivals at a full parking queue and
replies begun beyond the flushed count as zero on every conforming run;
the malformed-input suites gain one case per violation above.

## Step 0: pin the count on today's code

No behavior change. Land the instrument first.

- Measure, per party and logical level, `sent(ℓ) − consumed(ℓ)`. Increment
  `sent` by the number of questions in an outgoing reply when its last
  frame has flushed, before publication into `local_questions` can block.
  Increment `consumed` when the consuming stage takes the matching reply
  from the decoded-reply queue. Dequeuing an `InternalChildQueries` record
  is earlier and is not a substitute for this event. Completion of reply
  processing is later and is not the event either.
- Use `Progress` (`proxy/work/progress.rs`) and its
  `wire_reply(height, batch_len)` and `decoded_reply(height, count)` hooks
  where they match those boundaries. Add an explicit take event;
  decoding alone does not mean consumption. Register the greeting's one
  level-1 question separately and track the unasked opening batch outside
  this counter.
- Assert a high-water mark at most `K(ℓ) + FAN + 1`, with at most one
  outstanding greeting reply. Exercise `streaming/tests/capacity.rs`,
  `remote/proxy/tests/`, and `testing::schedule`, at both the budget window
  and `WindowConfig::FLOOR`.
- Pin the sequential-production facts in named tests: a stage sends no
  further outgoing reply until all questions in its current one are
  recorded; `encode::replies` publishes that outgoing reply's local
  bookkeeping only after its final frame flushes and takes no further
  local reply while blocked publishing.

Commit: "Pin the in-flight question bound".

## Step 1: park decoded replies, on the existing transport

Two existing channels widen, one publication moves later on its own edge,
and the opening decoder gains its own task and buffer. No new dependency
edge enters the dataflow. Widening alone does not introduce a capacity
wait, but that observation does not justify the publication move or the
new `next_scopes` capacity. Those need their own ordering and occupancy
checks below. Land all of this on the multi-stream `Link` first.

### Queues and publication

1. **Widen `ProxyResponses`.** Move its capacity-1 constructor from
   `Work::respond` (`proxy/work.rs`) into `proxy/work/queues.rs`. Allocate
   `C(ℓ) = K(ℓ) + FAN + 1`, or the structural cap derived below. Put the
   count and the exact take event at the constructor. A queue may fill;
   no next arriving reply may find it already full. Preserve the relay's
   role, but make overflow explicit rather than awaiting a slot that a
   conforming execution cannot need.
2. **Widen `local_questions`.** Use the corresponding derived capacity.
   Account for bookkeeping that has flushed but has not yet been
   published, including the batch in hand. Add an occupancy check tied to
   its actual dequeue event. Unexpected full-at-publication here is a
   local invariant failure, not evidence of peer misbehavior; fail without
   waiting and retain a debug assertion identifying the broken premise.
3. **Move received scopes to take time.** The decode pump currently yields
   an incoming reply and then publishes its derived scopes into the
   window-sized `next_scopes` queue. It will instead park
   `(Reply, Vec<Scope>)` and never publish to `next_scopes` itself.

   Name the events explicitly: the walk-facing exit of `Work::respond`
   takes incoming reply `r` out of parking (the consumption event),
   publishes the scopes for the peer's queries inside `r`, then hands
   `r` to the walk. The encoder uses these scopes when encoding the
   walk's outgoing answers to those queries. These received scopes are
   distinct from the local question records created by queries inside an
   outgoing reply. For the latter, the unchanged order is: flush the
   outgoing reply, then publish its local bookkeeping.

   Set `next_scopes` to `FAN + 1`. At the take of incoming reply `r + 1`,
   the encoder has dequeued the scopes of `r` except possibly one: it
   takes a scope before encoding its answer and may still be writing the
   preceding answer. The walk cannot take `r + 2` until the encoder has
   taken the answers generated from `r + 1`. Thus one possible straggler
   plus one incoming reply's fan fits. Write this argument at the
   constructor and test the boundaries under adversarial scheduling,
   including when the new capacity is smaller than the old window.
   Restate `yield_reply_scopes!`'s ordering role using these incoming and
   outgoing event names.
4. **Drive the opening independently.** `OpeningSupplies` (initiator
   stream 0) is currently decoded as the under-under-root pump calls
   `advance_to`. Give it its own task and a `FAN`-deep queue of decoded
   root children, one handle each. The batch has at most a fan of
   children, with membership known from both greetings. The task must
   drain it even when the consuming pump is unpolled; backend operations
   may delay it but must not wait for walk progress. Pin the opening
   cycle separately from the ordinary reply wedge.

### Derive the working-state price

Let `n` be the population input to the existing fan-size model and `d` the
child depth, equal to exposition level `ℓ`. Document how the existing
model obtains `n` from the greeting counts before evaluating the new
price; do not silently substitute a per-replica count for a combined one.
For reply payload at depth `d`, use
`children_quantile(n, d − 1)` reactions, each priced for its representation
and up to `children_quantile(n, d) × LISTING_ENTRY_BYTES` of query listing.
Supply handles must remain covered by that representation charge.

Add the parked reply price beside the local scope price in
`window::from_budget`. Include the scopes retained with each parked reply,
widened `local_questions`, fan-plus-one `next_scopes`, opening handle
queue, and active reply/stage/decoder state. Audit simultaneous ownership:
charge separate allocations separately and shared storage once. The
existing `supply_fans` charge may cover some of this; verify that coverage
before relying on it. The byte price must describe all retained working
state, not just digest payload. Replica content in the backend, including
uncommitted supplied content, remains outside this budget.

For the fixed parking slack, pre-charge at each depth

```text
min(FAN + 1, 256^(d−1)) × parked_reply_price(d).
```

This cap is structural: a level-`d` question concerns a node at depth
`d − 1`, and there are at most `256^(d−1)` such nodes. The actual parking
capacity may therefore be
`min(K(d) + FAN + 1, 256^(d−1))`. Keep the uncapped arithmetic out of any
integer overflow path. Both parties run the sizing calculation without
knowing their role; charging all depths rather than one parity is the
existing conservative convention. Distinguish this charge from actual
per-party occupancy.

### Numerical illustration

As an illustration of scale, consider fan estimates for a trie population
of `n = 100,000`: roughly 256 children near the root, roughly 200 children
beneath each of those, then roughly 1.5 one level further down. Using
about 25 bytes per listing entry gives:

| Child depth | Capped parking at minimum window | Approximate listing payload |
| --- | --- | --- |
| 1 | 1 reply | `256 × 200 × 25 B ≈ 1.3 MB` |
| 2 | 256 replies | `256 × 200 × 1.5 × 25 B ≈ 1.9 MB` |
| 3 and deeper | Up to 258 replies per depth | Small fans, adding tens of kilobytes in this illustration |

This is roughly 3–4 MB of payload in a conservative **per-party** charge
across all depths, not the sum of two replicas' actual allocations. Here
`n` is explicitly the illustrative trie population; the table does not
establish which population the sizing routine will use for two given
greeting counts. Nor does it evaluate the stated tail quantiles or price
all metadata. Compute that model result in the implementation before
making a budget claim for a particular pair of replicas.

### Implement and verify the accounting

Derive slot constants by `size_of`, as for existing constants, and include
heap-backed vectors and scopes separately. Update `sizing.rs` and
`Peer::sync_memory_budget`'s accounting paragraph with the boundary and
the statistical qualification: uniform hashing, tail probability `2⁻⁴⁸`
per estimate, union below `2⁻⁴⁰` per session. Clustered addresses can exceed
the byte estimate without violating the absolute slot count.

Record the parent commit's sizing output, then regenerate
`window/tradeoff.md` with `just window-tradeoff` and re-baseline
`tests/window/{census,corners,knee,operator,tradeoff_probe}.rs`. These report
the sizing model; they are not measurements of actual allocated memory.
Keep any allocation measurements separately labeled with workload and
accounting boundary.

### Tests and documentation

Carry forward step 0's sent-minus-consumed invariant. At every parking
enqueue, check occupancy **before** insertion is less than capacity;
allow occupancy to equal capacity afterward. Re-derive
`ProxyLocalQuestions`' harvested `min(capacity, S)` supremum using its
new capacity and actual lifetime. Add the scope-queue and independent
opening-decoder checks above; neither follows from widening alone.

The adapter's `fan_occupancy` tests still describe one reply skeleton and
one fan of leaves held by an active decoder, not all parked state. Keep
that distinction explicit. `capacity_stress_covers_every_queue_role` must
cover the proxy edges at their new widths. Update `message.rs`,
`stages.rs`, and `Work::respond` comments that currently allow only one
decoded reply per stage; state the derived capacity and arrival invariant.

Commit: "Park decoded replies per level".

## Step 2: demonstrate one pipe, before touching the API

The default milestone demonstrates adequacy on the current frame format
and `Link` API.

1. **Build a multiplexed test transport.** Use one
   `tokio::io::duplex(capacity)` call, giving endpoints A and B. Split
   each endpoint into read and write halves: A writes what B reads, and
   B writes what A reads. This is one duplex connection with two byte
   directions, not a pair of connections. A test-only
   `Connector`/`Acceptor` maps logical streams onto those directions.
   Serialize complete frames (and required labels) under the direction's
   writer lock; never interleave bytes within a frame. The receiving
   demux routes by the current-format labels and frame stream index into
   per-stream handoffs. This harness should still fail the `Link`
   conformance independence probe; sessions are expected to complete
   because the new reception removes their blocking dependency. The
   archived `wave1/integration` harness is a possible starting point.
2. **Pin the failure and cure.** Run `streaming/tests/wedge.rs`'s tree pair
   through that transport. With `ProxyResponses` forced to capacity 1,
   `run_to_quiescence` must report `Stalled`. At the derived capacity it
   must complete and match `Tree::join`. Commit two named tests and run
   the capacity stress matrix on the same transport. Also exercise the
   opening case with its independent decoder and full batch buffer.
3. **Pin arrivals and peak occupancy separately.** Generate tree pairs
   and adversarial schedules, checking that no arriving reply finds its
   parking queue already at capacity. Construct a run that reaches
   exactly `K(ℓ) + FAN + 1` parked replies at a level whose structural
   population cap permits it. The construction fills the record queue,
   leaves the consumer holding one question without taking its reply,
   and sends one further fan before recording can proceed. Temporarily
   delaying that consumer while replies park demonstrates a reachable
   occupancy; resume it to completion. This does not claim deadlock at
   one slot less. The depth-one wedge is the separate negative control.
4. **Investigate any unexpected stall.** Check the arithmetic and event
   boundaries, decode dependencies (including the opening), scope
   publication, and mux/demux scheduling. Resolve the violated premise
   before proceeding; do not merely widen the queue.

Commit: "Demonstrate single-pipe liveness with parked replies".

If the temporary `Link` harness is disproportionately costly, use this
explicit alternative order: steps 0 and 1, then a combined step 2/3 atomic
commit using the final pipe implementation. That commit must include the
same failing/passing wedge, arrival invariant, peak-occupancy fixture,
opening case, and stress matrix before it is accepted. The API change then
lands with the demonstration, not ahead of its acceptance conditions.

## Step 3: collapse the transport

One atomic commit covers the code, snapshots, public contract docs, and
working examples. A later prose pass may polish explanations, but may not
leave broken references or a false transport contract in the intermediate
tree. Update and test Sush's compatibility branch alongside the API change.

### The link, dissolved

`SessionState`'s epoch identified stream labels; those labels disappear.
Its poison latch prevented reuse of a stream left mid-session. Owned halves
express that restriction for callers who transfer ownership:

- `Rumors::gossip_once<R, W>(&self, read: R, write: W)` returns
  `Result<(Gossiped, R, W), Error>`, with
  `R: AsyncRead + Unpin + Send` and `W: AsyncWrite + Unpin + Send`.
  `Bootstrap::join` and `Peer::retire` return the halves in their success
  outcomes. An error does not return owned halves; cancellation drops
  values owned by the future. The continuous `gossip` driver owns its
  arguments for its lifetime. Remove `Error::LinkPoisoned`.
- The same signatures admit `&mut read` and `&mut write` through tokio's
  blanket implementations. Such callers retain the underlying halves and
  regain access when the borrow ends, including after error or
  cancellation; an error result does not return them. Document that the
  stream position is then unknown and the connection must be discarded.
  A later session is not guaranteed a typed poison error. Dropping a
  driver that owns only references likewise does not itself close the
  caller's connection.
- Delete `link.rs` and its machinery: `Link`, `SessionState`,
  `map_transport`, `Connector`, `Acceptor`, `Done`, `link::STREAM_COUNT`,
  `link/erased.rs`, `link/routed.rs`, `link/routed/*`,
  `conformance/link.rs`, `conformance/link/*`, and their tests.
  `window.rs` takes decoder count from codec `Stream::COUNT`; internal
  `DynRead`/`DynWrite` erasure remains in `peer/gossip.rs`. Callers wrap
  halves themselves instead of using `map_transport`. The `conformance`
  feature retains `bookmark` only. Tests replace `link::memory()` with
  duplex endpoints, using a testing helper if needed.
- Put the transport requirement on the session functions: one reliable,
  ordered duplex byte stream with independent directions, receiver-paced
  backpressure at any positive capacity, and EOF or error on peer
  departure. A read must progress while that side's write is blocked,
  as the greeting exchange requires. Migrate `link.rs`'s security
  paragraph (authentication is authorization, integrity, confidentiality,
  freshness) to the crate docs before deleting the source.

### The proxy

- Add `remote/pipe.rs` (name open). The mux owns the write half and takes
  frames from capacity-1 per-height channels, initially serving ready
  levels round-robin and writing/flushing whole frames. The demux owns
  the read half, reads heads with `FrameRead`, and routes by stream index
  into capacity-1 handoffs. It stops at the non-frame control head and
  returns the reader with that lookahead, absorbing the departure watch
  and `ControlRead`'s replay role. Decoder progress must be independent
  of the walk even when a handoff temporarily fills.
- Bind `StreamSender` and `StreamReceiver` to those handoffs. Remove label
  writes, claims, and `Done`; remove `AcceptDriver`, `Claims`,
  `ClaimSlots`, `AcceptError`, `StreamError::{Mislabeled, SupplyClosed}`,
  and the label renderer. Keep `Truncated` for pipe EOF before a logical
  height's `End::Stream`.
- In `state.rs`, replace `Session`'s connector, claims, and route with
  mux/demux handles. `Physical` loses `accept` and `errors`;
  `work.rs::execute` selects the protocol against the pipe's failure
  route.
- Detect parking overflow at the decoded-reply enqueue, where the unit
  is a reply rather than a frame, and propagate a typed failure through
  that route. Never block on overflow. Preserve enough diagnostics to
  distinguish excess peer replies from a broken local invariant.

### The wire

- Keep frames and remove stream labels. In each direction the preamble
  and greeting precede the data phase; data frames and each logical
  height's `End::Stream` precede the epilogue marker or party hand-off.
  A frame starts with a CBOR array head, the epilogue with a text head,
  and a donation with a tag head. On a non-array head, the demux returns
  control and its lookahead for the control parser to validate.
- This is a deliberate pre-release wire-format change. Re-accept
  `tests/gossip_snapshot.rs`, `tests/protocol_overhead.rs`, and the
  `insta` snapshots in the atomic commit, identifying "single-pipe
  framing: stream labels removed; control items in-band". Bookmark pins
  stay fixed.
- Change `crates/rumors-testkit/src/common/gossip_snapshot.rs` to group
  items by the frame's logical stream index from observer hooks rather
  than parsed labels. Render by logical stream so mux interleaving alone
  does not churn snapshots.

### The session drivers and tests

- Replace `SessionTransport` with two erased halves. Remove the session
  funnels' `SessionState::begin`/`finish` calls in `peer/gossip.rs` and
  return halves on success.
- Remove `testing::transport`'s `Connect`/`Accept` fault kinds,
  `testing::memnet`, and `tests/routed_link.rs`. Merge the testkit's
  `routed_tcp` and `tcp` helpers into one TCP-pipe helper. Remove
  `tests/latency_link.rs` with its contract suite and move the latency
  benchmark to a delayed duplex pair. Re-anchor `tests/hop_trace.rs` on
  the pipe, preserving its existing `3 + L + 1` ledger and its definition
  of `L`; run `benches/window_wallclock.rs` over the pipe.
- Retain `StreamObserver` by logical stream index and direction; update
  its wording from physical data streams to logical streams.

### Prose

The atomic commit updates public signatures and failure rules, migrates
`link.rs`'s surviving documentation, and fixes all imports, links,
examples, and comments that would otherwise describe deleted behavior.
This includes the affected parts of `lib.rs`, `reconciliation.rs`,
`sizing.rs`, `tutorial.rs`, `observe.rs`, the transport bullet in
`AGENTS.md`, and generated README material via `just readme`.
A following prose-only commit may improve organization and explanation;
it starts from accurate, gate-clean documentation.

Commits: "Collapse the transport to one pipe", then, if useful,
"Polish the single-pipe explanation".

## Step 4: the multiplexer's policy, measured

Frame round-robin supplies the required non-starvation and ships first.
Measure a maximum-size supply frame ahead of a thin deep reply on the
delayed pipe, including bytes already committed to transport buffers.
If measurements justify it, prefer question-bearing frames while retaining
non-starvation, and evaluate a smaller TCP send buffer. Priority can avoid
waiting behind the unsent supply backlog; it cannot bypass an in-progress
frame or bytes already buffered. The frame bound limits one frame, not
aggregate queuing delay. The dependency-hop ledger must stay fixed under
either policy; elapsed latency may change.

## Step 5: records

- Keep this note's README aligned with the implemented result and the
  recorded owner rulings.
- Update `formal/README.md`'s mux-campaign conclusion with the counting
  bound and a link here. Update the elastic model's comment to distinguish
  a bound chosen independently of the window and fan from the sufficient
  derived capacity. An optional counting charter in `Mux/Charters.lean`
  should state the arrival invariant as well as the occupancy upper
  bound. Formal work is secondary and does not block these steps.

## Acceptance

- Step 0's sent-minus-consumed bound passes at both the budget window and
  floor on the multi-stream transport before step 1 lands; the greeting
  is covered separately.
- Step 1's scope-publication, queue-custody, and opening-decoder checks
  pass, including a `next_scopes` reduction from a larger old window.
- The one-pipe wedge stalls at parking 1 and completes at the derived
  capacity in two named tests. A separate fixture reaches the occupancy
  bound and then completes; no minimum-capacity claim follows from it.
- No arriving reply finds its parking queue already at capacity across
  the stress matrix and generated schedules. Occupancy equal to capacity
  after insertion is allowed. Unexpected overflow fails without waiting.
- After step 3 (or the combined step 2/3), `just gate` is clean, the hop
  ledger matches its pre-collapse value, and snapshot differences are
  stream labels and in-band control items only, identified in the commit.
- The window census is re-baselined against recorded parent-commit output.
  Records identify the uniform-hashing assumptions, confidence bounds,
  excluded backend content, fixed floor, and distinction between sizing
  model output and actual allocation measurements.
- Sush's compatibility branch builds and its tests pass with the new
  session signatures.

## Risks, named

- **Count or lifetime mismatch.** An unexpected high-water mark may expose
  a wrong event boundary, height mapping, or sequential-production
  assumption. Resolve it rather than adding arbitrary slack.
- **A decoder still waits on the walk.** Correct capacity alone cannot
  repair this. Check scopes, opening scheduling, and backend independence
  when diagnosing a one-pipe stall.
- **Byte estimates underprice retained state.** The absolute count does
  not validate the statistical price. Audit scopes and auxiliary queues;
  clustered addresses can exceed the model even with correct accounting.
- **Absorbed content on failure.** Uncommitted supplies become garbage.
  `Local` releases its `Arc`s; a persistent backend must reclaim nodes,
  as it already must for `Ready` slots filled before commit. This custody
  is accepted by the owner.
- **Snapshot churn under scheduling.** Grouping by logical stream index
  prevents interleaving alone from changing the rendered capture.
- **Serialization and loss.** Step 4 measures queuing effects. One TCP
  loss recovery delays every level, including work that could have
  overlapped recovery on independent QUIC streams.
