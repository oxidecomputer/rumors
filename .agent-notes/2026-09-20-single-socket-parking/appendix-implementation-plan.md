# Appendix: implementation plan

This plan is for the agent implementing the design and for the reviewer
of its work. The [exposition](exposition.md) holds the argument, and
each step cites the sections it implements. Every step states its goal,
the change, and the evidence it commits. Where a step's mechanism
contradicts its goal, follow the goal and report the discrepancy.

The names below were inventoried at commit `98de1ce4`. Confirm each with
a search before relying on it. When the plan is complete, nothing in the
tree refers to deleted code.

## Conventions

- **Paths.** A path beginning `src/`, `tests/`, `benches/`, or `crates/`
  is relative to the repository root. Any other path is relative to
  `src/tree/mirror/streaming/`.
- **Names.** *The walk* is `materialized/`. *The proxy* is
  `remote/proxy/`, with its codec in `remote/codec/` and its reply
  adapter in `remote/adapter/`. `FAN = 256`.
- **Heights.** Code counts typed height from the leaves (0) to the root
  (32), so a level-ℓ question lists children at typed height
  `h = 32 − ℓ`. The walk creates the question queue for those questions
  with `window.capacity(h)`, and `proxy::Work::respond::<H>` delivers
  the replies they pair with, where `H::HEIGHT = h`. Every capacity
  below uses the same label, so one height names one logical queue
  everywhere.
- **Tasks.** A *task* is a future registered with the session's `Work`
  (`Work::spawn`) and driven by its terminal `complete`, never a runtime
  spawn, because the crate is runtime-independent.
- **Verification.** Iterate with `just check` and `just test <filter>`,
  and run `just gate` once, clean, before each commit. Run long builds
  and the gate in the background, redirected to a file, and poll that
  file. Do not iterate on wall-time measurements. Commit every proptest
  seed file that appears.

## Invariants every step preserves

- **The walk.** Apart from the opening-shortcut removal in step 1, the
  walk's exchange rules, its publication orders (`yield_resolve_query!`),
  its queues and their capacities, and its assembler chain stay
  unchanged. So do the frame grammar (`remote/codec/signal.rs`), supply
  runs and `RunBudget`, the causal filter, and the in-process tests that
  connect two walks through `mirror_connected`.
- **Local question records** keep their publication order: the encoder
  publishes a reply's records after handing on its last frame. Today
  that means after the flush; over the pipe (step 5) it means after the
  last frame enters the multiplexer's channel, without waiting for the
  write. `ProxyLocalQuestions` keeps capacity `K`, and the decoder keeps
  its blocking receive. Exposition §5.3 explains why this wait is local.
  Do not move the publication earlier.
- **The receive path waits only where exposition §5.3's table says.**
  Any new wait needs the same kind of argument, written before the wait
  lands.

## Step 0: meter the count

**Goal.** Commit an enforced check of the in-flight count (exposition
§5.1) before any change depends on it.

- In the walk's test trace (`materialized/progress.rs`,
  `materialized/progress/trace.rs`), record two new events. With each
  wire event, record the number of questions the outgoing reply carries;
  `yield_resolve_query!` already has them in hand. Wherever a stage
  takes a reply (`requests.next()` in every stage loop, the terminal's
  included), record a take event labeled with its height.
- Add `Trace::assert_outstanding_within(&Window)`. At every event it
  asserts `asked(h) − taken(h) ≤ window.capacity(h) + FAN + 1`, and at
  most one outstanding question at the root level. Call it, with the
  session's window, wherever `assert_valid` runs on a session trace
  (`tests.rs`). That covers the capacity stress matrix and the scheduled
  proptests.
- Commit a liveness-floor fixture that reaches equality at level 3 or
  deeper under `Window::FLOOR`. A first reply fills the question queue
  and gives the consuming stage one question to hold. The channel
  schedule then delays the consuming stage, and the asking stage hands
  on a reply carrying 256 questions and stays blocked recording them.
  Assert that the high-water mark is exactly `1 + FAN + 1`, then let the
  session complete. The fixture shows that the meter counts and that the
  bound is reached.
- `assert_valid`'s wire-contiguity clause already enforces the count's
  premise, that a stage records one reply's questions before handing on
  another. Name that dependency in the new checker's doc comment.

Commit: "Meter the walk's outstanding questions".

## Step 1: remove the opening-supply shortcut

**Goal.** Delete a special case that helps only between small replicas
and would deadlock over one socket (exposition §2 and §4).

- In the walk, remove the early-supply merge in `initiator_level`, the
  early-supply arm of `responder_level`, `OpeningHandoff` (both
  variants), and the `opening` parameter of `internal_level` and
  `internal_walk`. The initiator's stage 1 then answers the responder's
  root-level empty queries with ordinary supplies.
- In the proxy, delete `remote/proxy/work/opening_supplies.rs`,
  `adapter::early_supplies`, the supply half of `encode::opening` and of
  `adapter::opening_parts`, the `opening_supplies` parameters of
  `internal_replies` and `decode_pump`, and
  `StreamClass::OpeningSupplies`. The initiator's stream 0 then carries
  nothing.
- The greeting keeps its root listing, which is the level-1 question.
  Only the unasked supplies go.
- Tests:
  - Remove tests of the deleted paths, including
    `remote/adapter/tests/opening.rs` as far as it covers them.
  - Re-run `src/tree/mirror/streaming/tests/wedge.rs`. Its fixture is the
    exposition's §4 cycle: a disputed root child plus a *wall* of six
    root children only the initiator holds.
    `session_realizes_the_wedge_shape` should still decode the session's
    dispute *skeleton* (the tree of disputed and requested scopes) as the
    wedge, with the wall now crossing as level-2 supplies. If the
    skeleton decoder special-cases the opening batch, simplify it.
  - `tests/hop_trace.rs` pins exact hop counts per session shape.
    `trace_bulk_initiator_session` exists to pin the shortcut: it asserts
    that the initiator's first write on stream 0 lands at hop 2, and a
    total of 5 hops. Rewrite its expectations for the shortcut's absence,
    and state the change in the commit message. The other pinned counts
    (insertion 7, redaction 7, converged 3) must not move.
- This is a deliberate pre-release wire change. Re-accept the wire
  snapshots (`tests/gossip_snapshot.rs`, `tests/protocol_overhead.rs`,
  and the `insta` snapshots) in this commit, and name the change in the
  message. Bookmark pins do not move.
- Update the prose that describes the shortcut: `remote.rs`,
  `encode::opening`, `remote/proxy/work/stages.rs`, the walk's opening
  docs, and the greeting docs in `message.rs`.

Commit: "Remove the opening-supply shortcut".

## Step 2: park decoded replies

**Goal.** Give each level room for every invited reply (exposition
§5.2), and move answer-record publication to the take (§5.3). Both
changes land on today's transport, so every existing suite serves as the
regression suite.

1. **Parking capacity.** Move the decoded-reply channel constructor out
   of `Work::respond` (`remote/proxy/work.rs`) into
   `remote/proxy/work/queues.rs`. Give the channel capacity
   `cap(h) = min(window.capacity(h) + FAN + 1, 256^(31−h))`, computed
   without overflow; level 1 (`h = 31`) gets 1. Compute it from the same
   `window.capacity(h)` value that the walk's question queue at that
   height uses, because correctness rests on that link rather than on
   the value. Write the count's argument (exposition §5.1) at the
   constructor.
2. **Overflow check.** Before each park, the relay compares the queue's
   occupancy with the derived `cap(h)`, not with the channel's configured
   capacity. At `cap(h)` it fails with a new proxy error, for example
   `ParkingOverflow { height, capacity }`, attributed to the remote
   speaker, whose message says that the peer answered a question never
   asked or that a local premise of the bound failed. Otherwise it sends
   as usual. Production channels have capacity `cap(h)`, so the send
   never waits; tests that shrink the channel with `with_kind_capacity`
   get a blocking queue, which the step 5 negative control relies on.
   Occupancy is `max_capacity − capacity`; expose it on the production
   and instrumented senders.
3. **Park the reply with the record it answers.** The decoder still
   validates each query's positional derivation while decoding, but no
   longer returns the derived records, so `Decoded::questions` goes and
   `yield_reply_scopes!` leaves the decode pumps. The decoder parks
   `(Reply, Scope)`, where the `Scope` is the record of the question the
   reply answers.
4. **Publish answer records at the take.**
   - The stream that `respond` returns now owns the `ProxyNextScopes`
     sender. When the stage takes `(reply, scope)`, the stream replays
     `ReplyLevel::derive` over the reply's reactions against `scope` (a
     match or query advances the position; a supply does not), publishes
     each derived record, and then yields the reply.
   - A derivation failure here is a local invariant failure, since
     decoding already accepted the same sequence. Surface it as an
     internal error, never a panic.
   - The responder's terminal level derives nothing.
   - Give `ProxyNextScopes` capacity `FAN + 1`, and write the argument
     from exposition §5.3 at its constructor.
   - Restate `yield_reply_scopes!`'s ordering role, or remove the macro
     if the take is its only remaining use.
5. **Tests.**
   - Test the `ProxyNextScopes` capacity at three points. At `FAN − 1`,
     a full-fan reply stalls (`run_to_quiescence` reports `Stalled`). At
     `FAN`, it completes, possibly after waits. At `FAN + 1`,
     `blocked_send_polls` is zero across the proxy suites.
   - *Parking's tightness.* Using step 0's shape, hold a level's
     consuming stage through the channel schedule until exactly
     `K(ℓ) + FAN + 1` replies are parked. Assert the occupancy, then
     release the stage and complete the session. This test shows that
     the capacity cannot shrink; step 0's fixture shows only that the
     meter counts.
   - *Mismatched windows.* Run sessions between a peer at
     `Window::FLOOR` and a peer at a wide fixed window
     (`WindowConfig::Fixed`), with each in each role. Assert that they
     complete and match the in-memory merge, and that neither side's
     parking check fires. These sessions check that each side's parking
     depends only on its own window (exposition §5.2). Fixed windows
     keep them meaningful after step 4, which shares budget-sized
     windows.
   - The malformed-peer suite gains a surplus reply, which must end in
     `ParkingOverflow` and not hang.
   - `capacity_stress_covers_every_queue_role` covers the proxy edges at
     their new widths.
   - Update the proxy trace checks (`assert_question_causality` and its
     neighbors) for publication at the take.
   - Keep `fan_occupancy`'s claim scoped to one active decoder.
6. **The backend contract.** Add a clause to `Backend`'s documentation:
   operations complete without waiting on any other session work,
   because decoders and the walk share the backend (exposition §5.3).
   Note at `Local` that it meets the clause by construction, with
   immutable shared nodes and no locks.
7. **Prose.** Update the comments that allow one decoded reply per
   stage: `Work::respond`, the `remote/proxy/work/stages.rs` module doc,
   `remote/proxy/work/queues.rs`, and `message.rs`.

Commit: "Park decoded replies per level".

## Step 3: price parking and queued scopes

**Goal.** Make the budget account for what parking can hold (exposition
§6.3), and stop the model pricing queued work at its per-node worst case
(§6.5). This is the private price of §6.2: each side still prices alone.

Land this step as two commits. The first adds the set statistic and
re-prices queued scopes, which is independent of the socket and widens
every window. The second adds the parking charge.

- **Constants.** In `window.rs`, derive the parked-reaction and
  listing-entry byte constants with `size_of`, as the existing constants
  are derived.
- **The set statistic.** Add `set_leaves_quantile(n, q, j)`, the bound
  on leaves under any q distinct depth-j nodes, at union tail bits
  `UNION_TAIL_BITS + q·(8j + 2 − ⌊log₂ q⌋)`. Build it from the existing
  `small_mean_quantile` / `bernstein` pair with `num = n·q`. At `q = 1`
  it must equal `leaves_quantile`, and at `q ≥ 256ʲ` it returns `n`.
  Check its integer form numerically against exact binomial tails over
  the same parameter sweep the existing quantiles use.
- **Queued scopes, priced as a set** (exposition §6.5). The existing
  scope charge at depth d becomes
  `min(q·C(d−1), set_leaves_quantile(n, q, d−1), occupied(n, d))`
  references plus `q` fixed scope parts, where `q = min(K, S(d))`. The
  set bound needs its premise that the queued questions at one depth
  concern distinct nodes; state it at the charge.
- **The parking charge.** In `from_budget`'s `charge(K)`, add the
  parking term for every depth: the least of the per-reply, per-level,
  and per-set bounds (exposition §6.3), with
  `slots(d) = min(K(d) + FAN + 1, 256^(d−1))`.
- Update `UNION_TAIL_BITS`'s accounting comment for the added
  statistics, one per depth per replica. The total count of statistics
  must stay under 2⁸, so that their union keeps the session's tail below
  2⁻⁴⁰.
- **The default.** Compute the threshold budget at the reference point:
  A = B = 10⁶, `SPEC_BDP_BYTES`, and 100-byte messages. It is the least
  budget for which `from_budget` grants `min(W, max_d S(d))`, with
  `W = ⌈SPEC_BDP_BYTES / DISPUTE_WIRE_BYTES⌉`. The model puts it near
  467 MiB here, so `DEFAULT_SYNC_MEMORY_BUDGET` stays at 512 MiB; the
  exact prices decide, rounding up to a power of two. Commit a test that
  the default grants the reference window, so that any later pricing
  change that would make the default constrain the reference link fails
  it.
- **The reference set size.** Move `REFERENCE_SESSION_MESSAGES` to 10⁶,
  regenerate `window/tradeoff.md` with `just window-tradeoff`, and
  re-derive `REFERENCE_SCOPE_BYTES`. Update the prose figures in
  `src/sizing.rs`: the window, the crossover arithmetic, and the
  slowdown quotes. Re-measure any figure that is a measurement, such as
  the 41.73-byte overhead, at the new size, or state the size it was
  measured at.
- **The census.** Record the parent commit's sizing output, then
  re-baseline
  `tests/window/{census,corners,knee,operator,sweep,tradeoff_probe,pipelining}.rs`
  as they apply. These tests report the sizing model, not allocations.

Commits: "Price queued work as sets", then "Price parked replies".

## Step 4: exchange budgets, and share one window

**Goal.** Price parking against the peer's window, so that the level-2
term and the other fixed charges shrink with the budget, and give both
sides one window set by the smaller budget (exposition §6.6). This is a
deliberate pre-release wire change to the greeting, landed on today's
transport so that every existing suite is the regression net.

- **The greeting.** The greeting (`message.rs`, and its codec in
  `remote/codec/greeting.rs`) gains one field: its sender's budget. The
  greetings still cross concurrently, as `accept` in
  `remote/proxy/start.rs` sends and receives them today; nothing may make
  one greeting wait for the other. The decoder rejects a missing or
  malformed budget, and the greeting malformation suites gain a case for
  it.
- **The window.** `WindowConfig::resolve` takes the peer's budget as
  well as the set sizes.
  - Compute `K_max`: the widest window that fits
    `min(own budget, peer's budget)` at a node price of zero, pricing
    parking against a peer holding the same window. That is, the parking
    term's listing entries at level d are capped at
    `(K(d + 1) + FAN + 1) · min(FAN, C(d))`, where `K(d + 1)` is the
    candidate window itself.
  - Take the side's own window: the widest, no wider than `K_max`, that
    fits the smaller budget at the backend's real `node_bytes`, with the
    parking cap using `K_max(d + 1)` in place of `K(d + 1)`.
  - Map the window to per-level capacities as today,
    `K(ℓ) = max(1, min(K, S(ℓ)))`. Resolve the window once per side, and
    use it for both the walk and the proxy.
  - Write at `resolve` why the bound uses a zero node price: the two
    sides may use different backends, and no backend's node costs less
    than nothing, so no peer's window exceeds `K_max`.
- **Test-only fixed windows** (`WindowConfig::Fixed`) ignore the
  exchange and keep their capacities. The mismatched-window sessions of
  step 2 use fixed windows on both sides, so they keep exercising the
  safety argument of exposition §5.2 after this step.
- **Tests.**
  - A property test over budgets, set sizes, and backend node prices:
    every side's window is at most `K_max`, and the computation is
    symmetric in the two budgets.
  - Two peers that trigger a session at the same moment still collapse
    into one session, with each greeting carrying its budget.
  - In test builds, at every park, count the query reactions across
    that level's parked replies, and assert that they number at most
    `K_max(d + 1) + FAN + 1`. This is the deterministic fact the
    exchange prices, checked in every session test.
  - Sessions between peers on different budgets complete, and both
    sides' windows come from the smaller budget.
- **The default.** Re-run step 3's default computation with the design's
  price. The model puts the threshold near 443 MiB, so the default stays
  at 512 MiB, and step 3's test must still pass.
- **The census.** Re-baseline the census tests of step 3 against this
  step's parent commit, and regenerate `window/tradeoff.md`.
- **Calibration.** Measure actual peak parked bytes for 10⁶- and
  10⁷-message replicas with scattered differences, under the design's
  price. Record the results separately from model output, with the
  workload and the accounting boundary.
- **Wire snapshots.** Re-accept `tests/gossip_snapshot.rs`,
  `tests/protocol_overhead.rs`, and the `insta` snapshots in this
  commit, naming "the greeting carries the sync memory budget". Bookmark
  pins do not move.
- **User-facing prose.** Update `Peer::sync_memory_budget`'s memory
  accounting and the sizing guide from the draft text below. The budget
  now caps a session: a session runs at the window the smaller of the two
  peers' budgets affords. The transport at this commit is still the
  bundle, so describe it as it is. The public API does not change, but
  the behavior does, so build and test Sush's compatibility branch.

Commit: "Exchange budgets in the greeting, and share one window".

### Draft user-facing text

For `Peer::sync_memory_budget`, replacing the "Memory accounting"
section:

> # Memory accounting
>
> The budget is a sizing target, not a limit enforced at run time.
> Rumors chooses how much work to keep in flight so that, under a
> conservative model of how messages spread across its hash tree, the
> session's working memory stays within the budget. The model prices
> every buffer as full at once, and every buffer's contents at a size
> that uniformly spread message addresses exceed with probability below
> 2⁻⁴⁰ per session.
>
> Most sessions use far less. Buffers fill only while one part of the
> comparison waits on another, and replicas that gossip regularly differ
> in few messages. Measure your workload if you need a typical figure.
>
> Both peers of a session announce their budgets, and the session runs
> within the smaller of the two. A peer with a large budget therefore
> spends less when it synchronizes with a peer that has a small one.
>
> A session can exceed its estimate if message addresses cluster far
> more than hashing makes likely. Addresses are hashes of message
> versions, so in practice this does not happen by chance. Exceeding the
> estimate uses more memory and never affects correctness or progress.
>
> Even a zero budget keeps the minimum buffering needed for progress,
> which may exceed the target. The budget does not cover the replica
> itself, observer backlogs such as [`CausalMessages`], or the
> transport's own buffers.

For the sizing guide, a new section after "Why the link and message size
matter":

> # What the budget estimate means
>
> The window is sized from a worst case that is very unlikely to occur,
> rather than from typical use. For each tree level, the estimate
> assumes that the level has as much work in flight as it is allowed,
> and that each piece of that work is as large as the tree's shape
> plausibly permits. Here "plausibly" means that uniformly hashed
> addresses exceed the assumed size with probability below 2⁻⁴⁰ per
> session.
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

## Step 5: collapse the transport to one pipe

**Goal.** Sessions run over one duplex byte stream, the stream-bundle
machinery is gone, and the committed demonstration shows both that the
design prevents the deadlock and that the harness would catch its
return (exposition §4, §5.3, §5.4, §7.5). This is one atomic commit:
code, wire snapshots, public contract docs, examples, and tests. Update
and test Sush's compatibility branch alongside it.

### The public API

- `Rumors::gossip_once<R, W>(&self, read: R, write: W)` returns
  `Result<(Gossiped, R, W), Error>`, with `R: AsyncRead + Unpin + Send`
  and `W: AsyncWrite + Unpin + Send`.
- `Bootstrap::join` and `Peer::retire` take the halves the same way and
  return them in their success outcomes.
- `Rumors::gossip` takes the halves the same way and owns them for the
  driver's lifetime. To reuse the connection after the policy ends, a
  caller passes `&mut read, &mut write`.
- Tokio's blanket implementations admit `&mut` halves everywhere.
  Document that after an error, or after cancelling an active session,
  the connection's position is unknown and the connection must be
  discarded, and that a later session on it gets no typed warning.
- Remove `Error::LinkPoisoned`.
- State the transport requirement on these functions and in the crate
  docs: one reliable, ordered duplex byte stream; directions that
  progress independently, so that a read proceeds while the same side's
  write is blocked; receiver-paced backpressure at any positive
  capacity; and end-of-stream or an error when the peer departs.
- Move `src/link.rs`'s section on securing the transport (authentication
  is authorization, integrity, confidentiality, freshness) and its
  session promises into the crate docs.

### The pipe

- **Add `remote/pipe.rs`,** holding two tasks in the session's `Work`.
- **The multiplexer** owns the write half. It takes `(Stream, Frame)`
  items from one capacity-1 channel per logical stream. It serves every
  frame that is not a supply run before any supply run, round-robin
  within each class, and it writes and flushes whole frames. When every
  channel has closed, it returns the write half for the closing
  exchange.
- **Keep the multiplexer's priority classes general:** one lane per
  class, served in class order, each lane in its own order. A later bulk
  lane (exposition §10.2) can then join as the lowest class without a
  rewrite of a rule hard-coded for supply runs.
- **The demultiplexer** owns the read half. It reads each frame with
  `FrameRead` and routes it by stream index to that stream's decoder
  over a capacity-1 channel.
  - It reads a head's first byte with a single, cancellation-safe read.
    A frame opens with a CBOR array head. The session's closing items
    open differently: the *completion marker* each side sends when its
    reconciliation is done opens with a text head, and a *donation* (the
    identity a retiring or bootstrapping peer hands over) opens with a
    tag head.
  - On the first head that is not an array head, it stops and returns
    the read half with that byte as lookahead, and the closing exchange
    reads from there.
  - End-of-stream during the data phase is a truncation error of the
    pipe. It replaces the *departure watch*, the task that today reads
    the control stream during reconciliation to notice the peer leaving.
- **Delete `remote/streams.rs`.** `StreamSender` and `StreamReceiver`
  become the channel ends above. `decode_reply` already accepts any
  `Stream<Item = Frame>`. A decoder that received no question skips its
  end-of-stream check, as it does today. Remove the labels and their
  renderer, the claims, the accept driver, `Done`, `ClaimSlots`,
  `AcceptError`, `StreamError`, and the error route. Their one surviving
  case, end-of-stream before a logical stream's `End::Stream`, becomes
  the pipe's truncation error.
- **`remote/proxy/state.rs` and `remote/proxy/work.rs`.** `Session`
  holds the pipe's channel ends instead of the connector, claims, and
  routes. `Physical` loses `accept`, `errors`, and the departure watch.
  `execute` selects the protocol against the pipe's failure.
  `ControlRead` either becomes the lookahead-carrying read half or
  dissolves; it must not outlive its purpose.

### What is deleted

- `src/link.rs` and everything under it: `Link`, `SessionState`,
  `map_transport`, `Connector`, `Acceptor`, `Done`, `link::STREAM_COUNT`,
  `src/link/erased.rs`, and `link/routed*`.
- `src/conformance/link.rs` and `conformance/link/`. The `conformance`
  feature keeps its backend and bookmark suites.
- The proxy errors `Error::Accept` and `Error::Stream`.
- `testing::memnet`, and `testing::transport`'s connect and accept fault
  kinds.
- `tests/routed_link.rs`, and `tests/latency_link.rs` with its contract
  suite.
- In the testkit, `routed_tcp`, which merges with `tcp` into one helper
  that splits a `TcpStream`.

Two dependents change with the deletions: `window.rs` takes its decoder
count from the codec's `Stream::COUNT`, and `MemoryLink` in doc examples
becomes a split `tokio::io::duplex` pair.

### The wire

- Frames keep their grammar, and stream labels disappear. In each
  direction the order is: preamble, greeting, data frames and each
  logical stream's `End::Stream`, then the closing items.
- This is a deliberate pre-release wire change. Re-accept the wire
  snapshots in this commit, naming "single-pipe framing: stream labels
  removed; control items share the pipe". Bookmark pins do not move.
- `crates/rumors-testkit/src/common/gossip_snapshot.rs` groups items by
  logical stream index from the observer hooks rather than from parsed
  labels, and renders by logical stream, so that multiplexer
  interleaving alone never changes a snapshot.
- `StreamObserver` keeps its logical-stream index and direction, and its
  prose speaks of logical streams rather than transport streams.

### Drivers and remaining tests

- Replace `SessionTransport` with two erased halves.
- Remove the session funnels' `SessionState` calls in
  `src/peer/gossip.rs`.
- Re-anchor `tests/hop_trace.rs` on observer hooks over a delayed pipe.
  Every hop count it pins after step 1 must hold unchanged.
- Run `benches/window_wallclock.rs` over the pipe, and move the latency
  benchmark to a delayed duplex pair.

### The demonstration

Commit these tests in the same change.

1. **The negative control.** Run the exposition's §4 fixture
   (the tree pair in `src/tree/mirror/streaming/tests/wedge.rs`) over
   one `tokio::io::duplex` pair, with `ProxyResponses` forced to one
   slot by `with_kind_capacity`. Under `run_to_quiescence` it must
   report `Stalled`. At the derived capacity it must complete and match
   `Tree::join`. Commit these as two named tests.
2. **The positive regime.** Run the scheduled session suites over the
   pipe at `Window::FLOOR`, with a one-byte duplex buffer, under
   `run_to_quiescence`, and require every session to complete. In this
   regime any new wait of the receive path on the walk shows up as a
   stall. Also run the capacity stress matrix over the pipe, and the
   mismatched-window sessions from step 2 and the different-budget
   sessions from step 4, at a one-byte duplex buffer.
3. **Unexpected stalls.** If any stall appears, find the violated
   premise (the count's event boundaries, a new receive-path wait,
   backend independence, or multiplexer scheduling) and resolve it.
   Never widen a queue to make a stall go away.

### Prose

- Update every public signature and failure rule, migrate `src/link.rs`'s
  surviving documentation, and fix every import, link, example, and
  comment that would describe deleted behavior. That includes
  `src/lib.rs`, `src/reconciliation.rs`, `src/sizing.rs`,
  `src/tutorial.rs`, `src/observe.rs`, and the streaming module docs.
- Drop the sizing prose's caveats about the bundle's transport buffers
  that no longer apply.
- Update `AGENTS.md`'s transport bullet, and regenerate the READMEs with
  `just readme`.

Commit: "Collapse the transport to one pipe".

## Step 6: measure the single-socket residual

**Goal.** Put a number on the serialization cost of exposition §7.3, and
publish the deployment guidance that bounds it.

- On the delayed pipe, measure a thin deep reply that becomes ready just
  after a maximum-size supply frame. Take one run with the socket's
  unsent bytes unbounded and one with them bounded (`TCP_NOTSENT_LOWAT`
  at about one frame where the platform has it, `SO_SNDBUF` at about one
  bandwidth-delay product plus one frame otherwise), and record both.
- Measure the in-level bulk penalty on the delayed pipe for a
  representative divergent session: completion time against
  `max(descent critical path, total bytes / bandwidth)`. Exposition
  §10.2 bounds the descent's share by a few one-way delays and simulates
  the duplex share. Include a balanced workload of few, large messages,
  where the duplex share is largest, and record the measured excess
  beside both figures.
- The hop ledger must stay fixed; elapsed time may move.
- Write the deployment guidance into the crate docs. Which systems
  provide `TCP_NOTSENT_LOWAT` is verified from their TCP headers (Linux
  and macOS yes; illumos and FreeBSD no), as is illumos's rule that a
  send buffer above 64 KiB must be set before `listen` or `connect`;
  illumos's default buffer sizes and tunables are still unverified, so
  confirm them on the illumos box first. For TCP, the guidance is to
  bound the socket's unsent bytes by whichever means the platform offers
  (`TCP_NOTSENT_LOWAT` where it exists, `SO_SNDBUF` sized to the link
  elsewhere), and to choose
  `target_message_size` for the head-of-line delay the deployment
  accepts, since the default run budget is sized for memory symmetry
  with the largest query reply rather than for latency.

Commit: "Measure and document the single-socket residual".

## Step 7: records

- Update this note's README to describe the implemented result.
- Add one line to the conclusion of `formal/README.md`'s second
  campaign, which asked whether one bounded channel could replace the
  stream bundle: the counting bound, and a link to this note. Formal
  work is optional and blocks nothing.

## Acceptance

- Step 0's outstanding-question check runs in every walk suite, and its
  fixture reaches `K + FAN + 1` exactly.
- Removing the shortcut changes the wire snapshots and
  `trace_bulk_initiator_session` only as its commit names, and every
  other pinned hop count stays fixed.
- After step 2, no parking overflow occurs in any conforming suite, the
  `ProxyNextScopes` capacity law holds at its three points, and a surplus
  reply fails with `ParkingOverflow`.
- After step 3:
  - the set statistic matches exact binomial tails;
  - parking's capacity is derived from each level's actual
    question-queue capacity;
  - a test pins that the default grants the reference window at 10⁶
    messages;
  - the census is re-baselined against recorded parent output.
- After step 4:
  - every side's window is at most `K_max`, and simultaneous sessions
    still collapse into one;
  - parked queries never exceed `K_max(d + 1) + FAN + 1` in any session
    test;
  - the default test still passes, and the census is re-baselined;
  - the calibration measurement is recorded;
  - Sush's compatibility branch builds and passes its tests.
- After step 5:
  - `just gate` is clean;
  - the negative control stalls and the derived capacity completes;
  - every session in the pipe regime completes;
  - every pinned hop count matches its value before the collapse;
  - snapshot differences are confined to labels and in-band control
    items;
  - Sush's compatibility branch builds and passes its tests.
- Step 6's measurements are recorded, and the deployment guidance is
  published.

## Risks

- **The count or its event boundaries are wrong.** An unexpected
  high-water mark means a wrong event, height label, or production
  premise. Resolve it; never add slack.
- **A receive-path wait on the walk slips in.** No capacity can repair
  that. The pipe regime in step 5 detects it, and its negative control
  shows that the detector works.
- **The byte model underprices.** The count is exact, but its price in
  bytes is statistical. Clustered addresses can exceed the price without
  affecting correctness, and the calibration measurement anchors the
  model.
- **A failed session leaves absorbed content.** That content is
  uncommitted. In-memory backends release it, and a persistent backend
  must reclaim it, as it already reclaims any uncommitted node.
- **Loss coupling on TCP** is accepted: one recovery delays every level.
