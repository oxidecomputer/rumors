# Appendix: implementation plan

This plan is for the agent implementing the design, and for its
reviewer. The [exposition](exposition.md) holds the argument; this
document holds the route. Each step states its goal, the change, and the
evidence it commits. If a step's mechanism turns out to contradict its
goal, follow the goal and report the discrepancy. The inventory of names
below was taken at `98de1ce4`: confirm each with a search rather than
trusting the list. The goal is that nothing afterward refers to deleted
code.

## Conventions

- *The walk* is `streaming/materialized/`; *the proxy* is
  `streaming/remote/`. `FAN = 256`.
- **Heights.** Code counts typed height from the leaves (0) to the root
  (32). A level-ℓ question lists children at typed height `h = 32 − ℓ`.
  The walk creates the question queue for those questions with
  `window.capacity(h)`, and `proxy::Work::respond::<H>` delivers the
  replies they pair with, where `H::HEIGHT = h`. Every capacity below uses
  that same label, so one height names one logical queue everywhere.
- **Tasks.** A *task* is a future registered with the session's `Work`
  (`Work::spawn`) and driven by its terminal `complete`, never a runtime
  spawn. The crate is runtime-independent.
- **Verification discipline.**
  - Iterate with `just check` and `just test <filter>`.
  - Run `just gate` once, clean, before each commit.
  - Run long builds and the gate in the background, redirected to a file,
    and poll that file.
  - Do not iterate on wall-time measurements.
  - Commit every proptest seed file that appears.

## Invariants every step preserves

- **The walk.** Its exchange rules, its publication orders
  (`yield_resolve_query!`), its queues and their capacities, and its
  assembler chain are unchanged throughout. So are the frame grammar
  (`codec/signal.rs`), supply runs and `RunBudget`, the causal filter,
  and the in-process tests that connect two walks through
  `mirror_connected`.
- **Local question records** keep their publication order: the encoder
  publishes a reply's records after handing on its last frame (today,
  after the flush). `ProxyLocalQuestions` keeps capacity `K`, and the
  decoder keeps its blocking receive. Exposition §5.3 explains why this
  wait is local. Do not move the publication earlier.
- **The receive path waits only** where the exposition's §5.3 table says.
  Any new wait on the receive path needs the same kind of argument, and
  it needs it before it lands.

## Step 0: meter the count

**Goal.** Commit an enforced check of the in-flight count before any
change depends on it.

- In the walk's test trace (`materialized/progress.rs`,
  `progress/trace.rs`), record two new events:
  - with each wire event, the number of questions the outgoing reply
    carries (`yield_resolve_query!` already has them in hand);
  - a take event wherever a stage takes a reply (`requests.next()` in
    every stage loop, the terminal included), labeled with its height.
- Add `Trace::assert_outstanding_within(&Window)`. At every event it
  asserts `asked(h) − taken(h) ≤ window.capacity(h) + FAN + 1`, and at
  most one outstanding question at the root level. Call it wherever
  `assert_valid` runs on a session trace (`streaming/tests.rs`), with that
  session's window. This covers the capacity stress matrix and the
  scheduled proptests.
- **The liveness floor.** Commit a fixture that reaches equality, at
  level 3 or deeper under `Window::FLOOR`. The shape:
  - a first reply fills the question queue and gives the consuming stage
    one question to hold;
  - the consuming stage is delayed through the channel schedule;
  - the asking stage then hands on a reply carrying 256 questions, and
    stays blocked recording them.

  Assert that the high-water mark is exactly `1 + FAN + 1`, then let the
  session complete. This proves that the meter counts and that the
  capacity cannot shrink.
- The premise "a stage records one reply's questions before handing on
  another" is already enforced by `assert_valid`'s wire-contiguity clause.
  Name that dependency in the new checker's doc comment.

Commit: "Meter the walk's outstanding questions".

## Step 1: remove the opening-supply shortcut

**Goal.** Delete a special case that only helps between small replicas,
and that would deadlock over one socket.

- **The walk:**
  - remove the early-supply merge in `initiator_level`;
  - remove the early-supply arm of `responder_level`;
  - remove `OpeningHandoff` (both variants) and the `opening` parameter of
    `internal_level` / `internal_walk`.

  The responder's root-level empty queries are then answered by the
  initiator's stage 1 with ordinary supplies.
- **The proxy:**
  - delete `proxy/work/opening_supplies.rs` and `adapter::early_supplies`;
  - delete the supply half of `encode::opening` and of
    `adapter::opening_parts`;
  - delete the `opening_supplies` parameters of `internal_replies` and
    `decode_pump`;
  - delete `StreamClass::OpeningSupplies`.

  The initiator's stream 0 then carries nothing.
- The greeting keeps its root listing, the level-1 question. Only the
  unasked supplies go.
- **Tests.**
  - Remove tests of the deleted paths, including `adapter/tests/opening.rs`
    as far as it covers them.
  - Re-run `tests/wedge.rs`: `session_realizes_the_wedge_shape` should
    still decode the wedge skeleton, now with the wall crossing as level-2
    supplies. If the skeleton decoder special-cases the opening batch,
    simplify it.
  - `tests/hop_trace.rs`'s ledger must not change for its representative
    sessions. If one of them routes its critical path through an opening
    batch, state the one-crossing change in the commit message.
- This is a deliberate pre-release wire change. Re-accept the wire
  snapshots (`tests/gossip_snapshot.rs`, `tests/protocol_overhead.rs`, the
  `insta` snapshots) in this commit, and name the change in the message.
  Bookmark pins do not move.
- Update the prose that describes the shortcut: `remote.rs`,
  `encode::opening`, `stages.rs`, the walk's opening docs, and the
  greeting docs in `message.rs`.

Commit: "Remove the opening-supply shortcut".

## Step 2: park decoded replies

**Goal.** Give each level room for every invited reply, and move the
answer-record publication to the take. Both changes land on today's
transport, so every existing suite is the regression net.

1. **Parking capacity.**
   - Move the decoded-reply channel constructor out of `Work::respond`
     (`proxy/work.rs`) into `proxy/work/queues.rs`.
   - Give it capacity `C(h) = min(window.capacity(h) + FAN + 1, 256^(31−h))`,
     computed without overflow. Level 1, `h = 31`, gets 1.
   - Write the count's argument (exposition §5.1) at the constructor.
2. **Overflow check.**
   - Before each park, in the relay, compare the queue's occupancy with
     `C(h)`, the derived value, not the channel's configured capacity.
   - If the queue is at `C(h)`, fail with a new proxy error, for example
     `ParkingOverflow { height, capacity }`, attributed to the remote
     speaker. Its message says the peer answered a question never asked,
     or a local premise of the bound failed.
   - Otherwise send as usual.
   - Production channels have capacity `C(h)`, so the send never waits.
     Tests that shrink the channel with `with_kind_capacity` get a
     blocking queue, which the step 4 negative control relies on.
   - Occupancy is `max_capacity − capacity`. Expose it on the production
     and instrumented senders.
3. **Park the reply with the record it answers.**
   - The decoder still validates each query's positional derivation while
     decoding. It no longer returns the derived records, so
     `Decoded::questions` goes, and `yield_reply_scopes!` leaves the
     decode pumps.
   - It parks `(Reply, Scope)`, where the `Scope` is the record of the
     question the reply answers.
4. **Publish answer records at the take.**
   - The stream `respond` returns now owns the `ProxyNextScopes` sender.
   - When the stage takes `(reply, scope)`, the stream replays
     `ReplyLevel::derive` over the reply's reactions against `scope` (a
     match or query advances the position; a supply does not). It
     publishes each derived record, then yields the reply.
   - A derivation failure here is a local invariant failure, since decode
     already accepted the same sequence. Surface it as an internal error,
     never a panic.
   - The responder's terminal level derives nothing.
   - `ProxyNextScopes` gets capacity `FAN + 1`. Write the argument from
     exposition §5.3 at its constructor.
   - Restate `yield_reply_scopes!`'s ordering role, or remove the macro
     if the take is its only remaining use.
5. **Tests.**
   - Capacity law for `ProxyNextScopes`, pinned from both sides:
     - at `FAN − 1`, a full-fan reply stalls (`run_to_quiescence` reports
       `Stalled`);
     - at `FAN + 1`, `blocked_send_polls` is zero across the proxy suites.
   - The malformed-peer suite gains a surplus reply, which must end in
     `ParkingOverflow`, not a hang.
   - `capacity_stress_covers_every_queue_role` covers the proxy edges at
     their new widths.
   - Update the proxy trace checks (`assert_question_causality` and its
     neighbors) for publication at the take.
   - Keep `fan_occupancy`'s claim scoped to one active decoder.
6. **Prose.** Update the comments that allow one decoded reply per stage:
   `Work::respond`, the `stages.rs` module doc, `queues.rs`, and
   `message.rs`.

Commit: "Park decoded replies per level".

## Step 3: price parking, sharpen the model, and size the default

**Goal.** Make the budget describe what parking can hold. Stop the model
pricing queued work at its per-node worst case, or as if the replicas
shared nothing, where the session knows better. And keep the default
budget from constraining the reference link for 10⁶-message replicas.

Land this step as two commits:
1. The set statistic, the re-priced queued scopes, and the root
   comparison. These are independent of the socket and widen every
   window.
2. The parking charge and the default, with the census and calibration.

- **Constants.** In `window.rs`, derive the parked-reaction and
  listing-entry byte constants with `size_of`, as the existing constants
  are derived.
- **The set statistic.**
  - Add `set_leaves_quantile(n, m, j)`: the bound on leaves under any m
    distinct depth-j nodes, at union tail bits
    `UNION_TAIL_BITS + m·(8j + 2 − ⌊log₂ m⌋)`. Use the existing
    `small_mean_quantile` / `bernstein` pair with `num = n·m`.
  - Special cases: `m = 1` must equal `leaves_quantile`, and
    `m ≥ 256ʲ` returns `n`.
  - Check its integer form numerically against exact binomial tails over
    the same parameter sweep the existing quantiles use.
- **The charge.** In `from_budget`'s `charge(K)`, add the parking term for
  every depth: the least of the per-reply, per-level, and per-set bounds
  (exposition §6.3), with `slots(d) = min(K(d) + FAN + 1, 256^(d−1))`.
- **Queued scopes, priced as a set** (exposition §6.4). The existing scope
  charge at depth d becomes
  `min(m·C(d−1), set_leaves_quantile(n, m, d−1), occupied(n, d))`
  references plus `m` fixed scope parts, where `m = min(K, S(d))`. The
  queued questions at one depth concern distinct nodes, which is the
  premise the set bound needs. State that premise at the charge.
- **Pricing from the root comparison** (exposition §6.4).
  - At window resolution, count the root slots whose listings differ
    between the two greetings: a radix present on one side only, or
    digests that differ. Both the walk and the proxy resolve the window
    where both listings are in hand; pass the count into
    `WindowConfig::resolve`.
  - Add `D_hi` as a constant table over `k` in 0..256, with `k = 256`
    meaning no bound. A test re-derives every entry exactly with integer
    arithmetic, checking `C(256, k)·k^D ≤ 256^D / 2⁴⁸`.
  - When `k < 256`, cap the pricing inputs:
    - `S(2) ≤ k`, and `S(d) ≤ D_hi` for deeper d;
    - parking slots at `k` for level 2 and `D_hi` deeper;
    - parked listing entries at `D_hi · C(d)` per level.
  - Queue capacities never use `D_hi`. They stay the deterministic count
    of exposition §5, so a session in the statistical tail spends more
    memory but never fails. Write this at the capacity constructor, since
    it is the easiest rule to erode.
- Update `UNION_TAIL_BITS`'s accounting comment for the added statistics:
  one set statistic per depth per replica, plus the root comparison,
  which stays under 2⁸.
- **The default.**
  - Compute the threshold budget at the reference point: A = B = 10⁶,
    `SPEC_BDP_BYTES`, 100-byte messages. It is the least budget for which
    `from_budget` grants `min(W, max_d S(d))` with
    `W = ⌈SPEC_BDP_BYTES / DISPUTE_WIRE_BYTES⌉`.
  - Set `DEFAULT_SYNC_MEMORY_BUDGET` to that threshold rounded up to a
    power of two. With the refinements above, the model puts the threshold
    near 467 MiB, so the default likely stays at 512 MiB. The exact prices
    decide.
  - Commit a test that the default grants the reference window, so a
    later pricing change that would make the default constrain there
    fails.
- **The reference set size.**
  - Move `REFERENCE_SESSION_MESSAGES` to 10⁶, and regenerate
    `window/tradeoff.md` with `just window-tradeoff`.
  - Re-derive `REFERENCE_SCOPE_BYTES`.
  - Update the prose figures in `sizing.rs` (the window, the crossover
    arithmetic, the slowdown quotes). Any figure that is a measurement,
    such as the 41.73-byte overhead, is either re-measured at the new size
    or stated with the size it was measured at.
- **The census.** Record the parent commit's sizing output, then
  re-baseline `tests/window/{census,corners,knee,operator,sweep,tradeoff_probe,pipelining}.rs`
  as they apply. These report the sizing model, not allocations.
- **Calibration.** Measure actual peak parked bytes for 10⁶- and
  10⁷-message replicas with scattered differences. Record the results
  separately from model output, with workload and accounting boundary.
- **User-facing prose.** Update `Peer::sync_memory_budget`'s memory
  accounting and the sizing guide using the draft text below. At this
  commit the transport is still the bundle, so describe it as it is.

Commits: "Price queued work as sets, and from the root comparison", then
"Price parked replies; size the default for 10⁶-message replicas".

### Draft user-facing text

For `Peer::sync_memory_budget`, replacing the "Memory accounting"
section:

> # Memory accounting
>
> The budget is a sizing target, not a limit enforced at run time.
> Rumors chooses how much work to keep in flight so that, under a
> conservative model of how messages spread across its hash tree, the
> session's working memory stays within the budget. The model prices
> every buffer as full at once and every buffer's contents at a size
> that uniformly spread message addresses exceed with probability below
> 2⁻⁴⁰ per session.
>
> Most sessions use far less. Buffers fill only while one part of the
> comparison waits on another. The first exchange between two replicas
> also shows roughly how much they differ, and the estimate shrinks
> accordingly: replicas that gossip regularly are sized for their
> differences, not their total size. Measure your workload if you need
> a typical figure.
>
> A session can exceed its estimate if message addresses cluster far
> more than hashing makes likely. Addresses are hashes of message
> versions, so this does not happen by chance in practice. Exceeding the
> estimate uses more memory; it never affects correctness or progress.
>
> Even a zero budget keeps the minimum buffering needed for progress,
> which may exceed the target. The budget does not cover the replica
> itself, observer backlogs such as [`CausalMessages`], or the
> transport's own buffers.

For the sizing guide, a new section after "Why the link and message size
matter":

> # What the budget estimate means
>
> The window is sized from a worst case that is extremely unlikely to
> occur, not from typical use. For each tree level, the estimate assumes
> the level has as much work in flight as it is allowed, and that each
> piece of that work is as large as the tree's shape plausibly permits.
> Here "plausibly" means that uniformly hashed addresses exceed the
> assumed size with probability below 2⁻⁴⁰ per session.
>
> In ordinary operation the in-flight work is far smaller. Replicas that
> gossip regularly differ in a small fraction of their messages, and the
> protocol spends memory only where they differ. Levels fill their
> buffers only while waiting on deeper levels. The budget therefore
> answers "how much could this session need?" rather than "how much will
> it use?", and a session that stays well under it is the expected case.
>
> Choose the budget as the memory you are willing to have a session use
> at worst. If memory is scarce, a smaller budget remains correct; it
> only limits how much work overlaps each round trip.

## Step 4: collapse the transport to one pipe

**Goal.** Sessions run over one duplex byte stream, the stream-bundle
machinery is gone, and the committed demonstration shows that the cure
works and that the harness would catch its regression. This is one
atomic commit: code, wire snapshots, public contract docs, examples, and
tests. Update and test Sush's compatibility branch alongside it.

### The public API

- `Rumors::gossip_once<R, W>(&self, read: R, write: W)` returns
  `Result<(Gossiped, R, W), Error>`, with `R: AsyncRead + Unpin + Send`
  and `W: AsyncWrite + Unpin + Send`.
- `Bootstrap::join` and `Peer::retire` take the halves the same way and
  return them in their success outcomes.
- `Rumors::gossip` takes the halves the same way and owns them for the
  driver's lifetime. To reuse the connection after the policy ends, a
  caller passes `&mut read, &mut write`.
- Tokio's blanket implementations admit `&mut` halves everywhere. Document
  that after an error, or after cancelling an active session, the
  connection's position is unknown and the connection must be discarded,
  and that a later session on it gets no typed warning.
- Remove `Error::LinkPoisoned`.
- State the transport requirement on these functions and in the crate
  docs: one reliable, ordered duplex byte stream; directions that
  progress independently (a read proceeds while the same side's write is
  blocked); receiver-paced backpressure at any positive capacity; and
  end-of-stream or an error when the peer departs.
- Move `link.rs`'s section on securing the transport (authentication is
  authorization, integrity, confidentiality, freshness) and its session
  promises into the crate docs.

### The pipe

- **Add `remote/pipe.rs`,** holding two tasks in the session's `Work`.
- **The multiplexer** owns the write half. It takes `(Stream, Frame)`
  items from one capacity-1 channel per logical stream. It serves every
  frame that is not a supply run before any supply run, round-robin within
  each class, and it writes and flushes whole frames. When every channel
  has closed, it returns the write half for the closing exchange.
- **Keep the multiplexer's priority classes general:** one lane per
  class, served in class order, each lane in its own order. A later bulk
  lane (exposition §8) then slots in as the lowest class, rather than
  forcing a rewrite of a hard-coded rule for supply runs.
- **The demultiplexer** owns the read half. It reads each frame with
  `FrameRead` and routes it by stream index to that stream's decoder over
  a capacity-1 channel.
  - It reads a head's first byte with a single, cancellation-safe read.
    A frame opens with a CBOR array head. The completion marker opens
    with a text head, and a donation with a tag head.
  - On the first non-array head, it stops and returns the read half with
    that byte as lookahead. The closing exchange reads from there.
  - End-of-stream during the data phase is a truncation error of the
    pipe. It replaces the departure watch.
- **Delete `remote/streams.rs`.** `StreamSender` and `StreamReceiver`
  become the channel ends above. `decode_reply` already accepts any
  `Stream<Item = Frame>`. A decoder that received no question skips its
  end-of-stream check, as today.
  - Remove the labels and their renderer, the claims, the accept driver,
    `Done`, `ClaimSlots`, `AcceptError`, `StreamError`, and the error
    route.
  - Their one surviving case, end-of-stream before a logical stream's
    `End::Stream`, becomes the pipe's truncation error.
- **`state.rs` and `work.rs`.**
  - `Session` holds the pipe's channel ends instead of the connector,
    claims, and routes.
  - `Physical` loses `accept`, `errors`, and the departure watch.
  - `execute` selects the protocol against the pipe's failure.
  - `ControlRead` either becomes the lookahead-carrying read half or
    dissolves; it must not outlive its purpose.

### What is deleted

- `link.rs` and everything under it: `Link`, `SessionState`,
  `map_transport`, `Connector`, `Acceptor`, `Done`, `link::STREAM_COUNT`,
  `link/erased.rs`, and `link/routed*`.
- `conformance/link.rs` and `conformance/link/`. The `conformance`
  feature keeps its backend and bookmark suites.
- The proxy errors `Error::Accept` and `Error::Stream`.
- `testing::memnet`, and `testing::transport`'s connect and accept fault
  kinds.
- `tests/routed_link.rs`, and `tests/latency_link.rs` with its contract
  suite.
- In the testkit, `routed_tcp`: merge it with `tcp` into one helper that
  splits a `TcpStream`.
- `window.rs` takes its decoder count from the codec's `Stream::COUNT`.
- `MemoryLink` in doc examples becomes a `tokio::io::duplex` pair, split.

### The wire

- Frames keep their grammar, and stream labels disappear. In each
  direction the order is: preamble, greeting, data frames and each
  logical stream's `End::Stream`, then the closing items.
- This is a deliberate pre-release wire change. Re-accept the wire
  snapshots in this commit, naming "single-pipe framing: stream labels
  removed; control items share the pipe". Bookmark pins do not move.
- `crates/rumors-testkit/src/common/gossip_snapshot.rs` groups items by
  logical stream index from the observer hooks rather than parsed labels,
  and renders by logical stream, so that multiplexer interleaving alone
  never churns a snapshot.
- `StreamObserver` keeps its logical-stream index and direction. Its
  prose says logical streams, not transport streams.

### Drivers and remaining tests

- Replace `SessionTransport` with two erased halves.
- Remove the session funnels' `SessionState` calls in `peer/gossip.rs`.
- Re-anchor `tests/hop_trace.rs` on observer hooks over a delayed pipe.
  Its existing `3 + L + 1` ledger must hold unchanged.
- Run `benches/window_wallclock.rs` over the pipe, and move the latency
  benchmark to a delayed duplex pair.

### The demonstration

These tests are committed in the same change:

1. **The negative control.** Run the §4 fixture (`tests/wedge.rs`'s tree
   pair) over one `tokio::io::duplex` pair with `ProxyResponses` forced to
   one slot by `with_kind_capacity`. Under `run_to_quiescence` it must
   report `Stalled`. At the derived capacity it must complete and match
   `Tree::join`. Commit these as two named tests.
2. **The positive regime.**
   - Run the scheduled session suites over the pipe at `Window::FLOOR`
     with a one-byte duplex buffer, under `run_to_quiescence`. Every
     session must complete. This is the regime where any new wait of the
     receive path on the walk surfaces as a stall.
   - Also run the capacity stress matrix over the pipe.
3. **Unexpected stalls.** If any stall appears, find the violated premise
   (the count's event boundaries, a new receive-path wait, backend
   independence, or multiplexer scheduling) and resolve it. Never widen a
   queue to make a stall go away.

### Prose

- The commit updates every public signature and failure rule, migrates
  `link.rs`'s surviving documentation, and fixes every import, link,
  example, and comment that would describe deleted behavior. That
  includes `lib.rs`, `reconciliation.rs`, `sizing.rs`, `tutorial.rs`,
  `observe.rs`, and the streaming module docs.
- The sizing prose drops the bundle's transport-buffer caveats that no
  longer apply.
- Update `AGENTS.md`'s transport bullet, and regenerate the READMEs with
  `just readme`.

Commit: "Collapse the transport to one pipe".

## Step 5: measure the single-socket residual

**Goal.** Put a number on the serialization cost from exposition §9, and
publish the deployment guidance that bounds it.

- On the delayed pipe, measure a thin deep reply ready just after a
  maximum-size supply frame. Take one run with the socket's unsent bytes
  unbounded and one bounded (`TCP_NOTSENT_LOWAT` at about one frame where
  the platform has it; `SO_SNDBUF` at about one bandwidth-delay product
  plus one frame otherwise). Record both.
- Measure the in-level bulk penalty on the delayed pipe, for a
  representative divergent session: completion time against
  `max(descent critical path, total bytes / bandwidth)`. Exposition §8.2
  bounds the descent's share by a few one-way delays and simulates the
  duplex share. Include a balanced workload of few, large messages, where
  the duplex share is largest, and record the measured excess beside both
  figures.
- The hop ledger must stay fixed; elapsed time may move.
- Write the deployment guidance into the crate docs, after confirming
  platform support against current documentation. For TCP:
  - bound the socket's unsent bytes by whichever means the platform
    offers;
  - choose `target_message_size` for the head-of-line delay the
    deployment accepts, since the default run budget is sized for memory
    symmetry with the largest query reply, not for latency.

Commit: "Measure and document the single-socket residual".

## Step 6: records

- Update this note's README to describe the implemented result.
- Add one line to `formal/README.md`'s mux-campaign conclusion: the
  counting bound and a link to this note. Formal work is optional and
  blocks nothing.

## Acceptance

- Step 0's outstanding-question check runs in every walk suite, and its
  fixture reaches `K + FAN + 1` exactly.
- The shortcut's removal changes the wire snapshots only as its commit
  names, and leaves the hop ledger fixed.
- After step 2:
  - no parking overflow occurs in any conforming suite;
  - the `ProxyNextScopes` capacity law holds from both sides;
  - a surplus reply fails with `ParkingOverflow`.
- After step 3:
  - the set statistic matches exact binomial tails, and the `D_hi` table
    matches its exact re-derivation;
  - no queue capacity depends on `D_hi`;
  - the default grants the reference window at 10⁶ messages, pinned by a
    test;
  - the census is re-baselined against recorded parent output;
  - the calibration measurement is recorded.
- After step 4:
  - `just gate` is clean;
  - the negative control stalls and the derived capacity completes;
  - the pipe regime completes throughout;
  - the hop ledger matches its value before the collapse;
  - snapshot differences are labels and in-band control items only;
  - Sush's compatibility branch builds and passes its tests.
- Step 5's measurements are recorded, and the deployment note is
  published.

## Risks

- **The count or its event boundaries are wrong.** An unexpected
  high-water mark means a wrong event, height label, or production
  premise. Resolve it; never add slack.
- **A receive-path wait on the walk slips in.** Capacity cannot repair
  that. The pipe regime in step 4 is the detector; its negative control
  shows the detector works.
- **The byte model underprices.** The count is exact, but its byte price
  is statistical. Clustered addresses can exceed it without affecting
  correctness; the calibration measurement anchors the model.
- **Absorbed content of a failed session.** It is uncommitted.
  In-memory backends release it; a persistent backend must reclaim it, as
  it already reclaims any uncommitted node.
- **Loss coupling on TCP** is accepted: one recovery delays every level.
