# Appendix: implementation plan

For a reader who knows the code. The [exposition](exposition.md) gives the
argument; this is the route from `772cce34` to sessions over one pair of
read and write halves. The sequence instruments the count, changes
reception on the existing transport, then collapses the transport to one
pipe and demonstrates the cure in that same commit. Each commit includes
the tests and documentation needed to keep that intermediate tree
gate-clean.

The removal of `Link`, by-value ownership of the halves, their return on
success, and support for callers passing mutable references are settled
by the owner's ruling of 2026-09-20. The ruling of 2026-09-21 simplifies
the plan in five ways, each applied below: sent-question records are
published before the reply is flushed, so an arriving reply without a
record is a violation with no counter and no transient wait; the
opening-supply shortcut is removed; no throwaway demonstrator transport is
built; `StreamSender` and `StreamReceiver` dissolve into channels; and the
multiplexer prefers question-bearing frames from the start. The pipe
module's name remains an implementation choice.

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
also have explicit roles. The convention is uniform: the walk's
question-queue constructors, `Window::capacity`, and both `Progress` hooks
(`wire_reply` and `decoded_reply`) take the *children's* typed height,
`h = 32 − ℓ`, so one label names the same logical queue everywhere.

A *task* below is a future registered with the session's `Work`
(`Work::spawn`) and driven by its terminal `complete`, never a runtime
spawn: the crate is runtime-independent, and the session's own driver is
what schedules its concurrent pieces.

## What does not change

The walk's exchange rules: `yield_resolve_query!`, its publication orders,
the assembler chain, the `Window` search for an affordable width, population
bounds, version-size pricing, greeting, `Reply`/`Reaction` vocabulary,
frame signal grammar (`codec/signal.rs`), supply runs and `RunBudget`,
and causal sieve. In-process tests connecting two walks through
`mirror_connected` remain. The window's byte prices and granted widths do
change; the receive proxy changes scope custody; the opening-supply
shortcut is removed (step 1). Existing protocol violations remain
applicable, with parking overflow handled explicitly. Transport-specific
errors disappear with the machinery that produced them.

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
- **A reply with no record to pair it with.** The decoder takes a record
  from `local_questions` before decoding a reply at that height. Because
  the encoder publishes a reply's records *before* writing the reply
  (step 1), a conforming peer's reply can never arrive before its record
  exists. The decoder therefore takes the record with a non-blocking
  receive, and an empty queue at a reply's first frame is a violation:
  the reply to a question never sent. No counter, and no transient-wait
  case.
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
replies begun with no record present as zero on every conforming run; the
malformed-input suites gain one case per violation above.

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
  level-1 question separately; the opening batch, still present at step
  0, stays outside this counter.
- Assert a high-water mark at most `K(ℓ) + FAN + 1`, with at most one
  outstanding greeting reply. Exercise `streaming/tests/capacity.rs`,
  `remote/proxy/tests/`, and `testing::schedule`, at both the budget window
  and `WindowConfig::FLOOR`.
- Pin the sequential-production facts in named tests: a stage sends no
  further outgoing reply until all questions in its current one are
  recorded; `encode::replies` takes no further local reply until the
  current one's records are published (today after the final frame
  flushes; step 1 moves the publish before the write, and the pin moves
  with it).

Commit: "Pin the in-flight question bound".

## Step 1: remove the shortcut, then park decoded replies

Two commits, both on the multi-stream `Link`, with every existing suite
as the regression net. The first removes the opening-supply shortcut; the
second widens two existing channels and moves two publications, one
later and one earlier on its own edge. No new dependency edge enters the
dataflow. Widening alone introduces no capacity wait, but that
observation does not justify the publication moves; those need their own
ordering and occupancy checks below.

### Remove the opening-supply shortcut

Delete the early-supply path on both participants. In the walk,
`initiator_level`'s early-supply merge and the `OpeningHandoff` variants it
feeds (`Survivors` on the initiator, `Supplies` on the responder) go, so
the responder's root-level requests are answered by the initiator's stage
1 with ordinary supply replies at level 2, as any request is answered at
any level. In the proxy, `opening_supplies.rs`, `adapter::early_supplies`,
the supply half of `encode::opening` and of `adapter::opening_parts`, the
`opening_supplies` argument of `internal_replies` and `decode_pump`, and
`StreamClass::OpeningSupplies` go; the initiator's stream 0 then carries
nothing. The greeting still carries the listing, so the responder still
answers at once. This changes the wire (the initiator's stream 0 no
longer opens, and its replies to the responder's empty root queries carry
supplies instead of nothing), so the snapshots re-accept in this commit,
named "remove the opening-supply shortcut". The rationale is recorded in
the exposition (§2): the batch is nonempty only between tiny replicas.

Commit: "Remove the opening-supply shortcut".

### Queues and publication

1. **Widen `ProxyResponses`.** Move its capacity-1 constructor from
   `Work::respond` (`proxy/work.rs`) into `proxy/work/queues.rs`. Allocate
   `C(ℓ) = K(ℓ) + FAN + 1`, or the structural cap derived below. Put the
   count and the exact take event at the constructor. A queue may fill;
   no next arriving reply may find it already full. Preserve the relay's
   role, but make overflow explicit rather than awaiting a slot that a
   conforming execution cannot need.
2. **Widen `local_questions` and publish before the write.** Capacity
   `K(ℓ) + 2·FAN + 1`: the derived bound plus the records of the one
   outgoing reply the encoder is writing, since `encode::replies` now
   publishes a reply's records before `write_reply` rather than after.
   With that capacity the publish never blocks, which removes the reason
   the publish followed the flush (an encoder blocked with a reply
   half-written). The decoder takes records with a non-blocking receive;
   an absent record at a reply's first frame is a violation (the rule for
   full queues above). Add an occupancy check tied to the decoder's
   dequeue. Unexpected full-at-publication is a local invariant failure,
   not evidence of peer misbehavior; `debug_assert`, never a wait.
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
   outgoing reply. For the latter, the new order is: publish its local
   records, then write and flush the outgoing reply.

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
widened `local_questions`, fan-plus-one `next_scopes`, and active
reply/stage/decoder state. Audit simultaneous ownership:
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
new capacity and actual lifetime. Add the scope-queue and
early-publication checks above; neither follows from widening alone.

The adapter's `fan_occupancy` tests still describe one reply skeleton and
one fan of leaves held by an active decoder, not all parked state. Keep
that distinction explicit. `capacity_stress_covers_every_queue_role` must
cover the proxy edges at their new widths. Update `message.rs`,
`stages.rs`, and `Work::respond` comments that currently allow only one
decoded reply per stage; state the derived capacity and arrival invariant.

Commit: "Park decoded replies per level".

## Step 2: collapse the transport, and demonstrate the cure

One atomic commit covers the code, snapshots, public contract docs,
working examples, and the demonstration: the wedge tests, the arrival
invariant, the occupancy fixture, and the stress matrix, all over the
real pipe. There is no throwaway demonstrator transport (ruled
2026-09-21); step 0's instrument and step 1's parking on the existing
bundle carry the confidence until this commit, and the demonstration is
part of its acceptance. A later prose pass may polish explanations, but
may not leave broken references or a false transport contract in the
intermediate tree. Update and test Sush's compatibility branch alongside
the API change.

### The demonstration

1. **Pin the failure and cure.** Run `streaming/tests/wedge.rs`'s tree pair
   over one in-memory duplex pipe (one `tokio::io::duplex(capacity)`
   call, giving endpoints A and B; A writes what B reads and B writes what
   A reads: one duplex connection with two byte directions, not a pair of
   connections). With `ProxyResponses` forced to capacity 1,
   `run_to_quiescence` must report `Stalled`. At the derived capacity it
   must complete and match `Tree::join`. Commit two named tests and run
   the capacity stress matrix over the same pipe.
2. **Pin arrivals and peak occupancy separately.** Generate tree pairs
   and adversarial schedules, checking that no arriving reply finds its
   parking queue already at capacity. Construct a run that reaches
   exactly `K(ℓ) + FAN + 1` parked replies at a level whose structural
   population cap permits it: fill the record queue, leave the consumer
   holding one question without taking its reply, and send one further
   fan before recording can proceed. Temporarily delaying that consumer
   while replies park demonstrates a reachable occupancy; resume it to
   completion. This does not claim deadlock at one slot less. The
   depth-one wedge is the separate negative control.
3. **Investigate any unexpected stall.** Check the arithmetic and event
   boundaries, both bookkeeping moves, backend independence, and mux/demux
   scheduling. Resolve the violated premise before proceeding; do not
   merely widen the queue.

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

- Add `remote/pipe.rs` (name open), holding two tasks in the session's
  `Work`. The mux owns the write half and takes `(Stream, Frame)` items
  from capacity-1 per-height channels, serving question-bearing frames
  (every frame that is not a supply run) before supply runs, round-robin
  within each class, and writing and flushing whole frames. The demux
  owns the read half, reads heads with `FrameRead`, and routes by stream
  index into capacity-1 handoffs. It stops at the non-frame control head
  and returns the reader with that lookahead, absorbing the departure
  watch and `ControlRead`'s replay role. Decoder progress must be
  independent of the walk even when a handoff temporarily fills.
- Dissolve `StreamSender` and `StreamReceiver` (`remote/streams.rs`): an
  encoder sends its frames into its per-height mux channel, and a decoder
  reads frames from its handoff. Remove label writes, claims, `Done`,
  `AcceptDriver`, `Claims`, `ClaimSlots`, `AcceptError`, `StreamError`,
  and the label renderer; the one surviving case, pipe EOF before a
  logical height's `End::Stream`, becomes a `Truncated` error of the
  pipe module.
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

## The socket's send buffer: where one socket's latency residual lives

The whole thing is about *where* a queue lives relative to *who* gets to
reorder it.

### A TCP socket is a reservoir with a FIFO outlet

When the multiplexer writes a frame to the socket, the bytes do not go
onto the wire. They go into the kernel's send buffer, and TCP drains that
buffer onto the wire at whatever rate the congestion window and the
peer's receive window allow. The buffer holds bytes in two states:

```text
  application ──write──▶ [ unsent │ sent, awaiting ACK ] ──▶ wire
                            ▲                ▲
                     queued behind      already transmitted;
                     everything sent    kept only for retransmission
```

The right part must be large. To keep a link busy, TCP needs a
bandwidth-delay product of bytes in flight: on a 1 Gbps link with a
100 ms round trip that is 12.5 MB, all of it "sent, awaiting ACK", all of
it sitting in the send buffer until acknowledged. So a well-tuned buffer
is at least a BDP, and Linux autotunes it upward toward that.

The left part is the problem. A `write` only blocks when the *whole*
buffer is full. A bulk sender therefore fills it: at steady state it holds
roughly a BDP of in-flight bytes *plus* however much room is left as
unsent bytes. If the buffer is two BDPs, there is a BDP of unsent supply
bytes queued in the kernel at all times while supplies are flowing.

### Priority is decided at the inlet, but delay is paid at the outlet

The multiplexer's job is to choose what to write *next*. "Next" means:
after everything already in the buffer. The kernel drains the buffer in
order and offers userspace no way to reorder it. So when a thin,
critical-path reply becomes ready and the mux correctly puts it ahead of
every unsent supply frame *in its own queues*, the reply still lands
behind whatever supplies are already in the kernel's unsent portion.

How long is that? Unsent bytes ahead of it, divided by bandwidth. The
bytes in flight cost nothing extra, since they are already transmitting
and ACKs return at wire rate, so new bytes go out as fast as the unsent
queue ahead of them drains. With a BDP of unsent bytes ahead, the reply
waits one BDP's transmission time, which by definition is one round-trip
time: 100 ms on that link. The priority decision was right; it was made
too far upstream of the wire to matter.

This is exactly the failure HTTP/2 prioritization hit in practice a few
years ago: browsers asked servers to prioritize HTML over images, servers
did, and it made no difference because the reordering happened above a
kernel buffer holding megabytes of already-queued image bytes.

### How much this costs, and when

Less than the mechanism suggests, because priority settles the order
completely. A supply frame is written only in a gap when no level has a
thin frame ready, so the unsent portion fills with supplies only in the
bulk-heavy phase of a session, when supplies are being discovered faster
than the wire drains them. In that phase the wire is busy with bulk
regardless. A thin reply delayed behind the unsent queue delays only the
work that depends on it; the bulk transfer itself is not extended. The
delay becomes visible only where that dependent work would otherwise
have finished *after* the bulk: at the tail, where the last thin hops
discover the last supplies. Each such hop waits behind the unsent
portion once, so the exposed cost is a few unsent-drain times at the end
of a bulk-heavy session, and nothing in a session with little bulk,
because then the unsent portion is never full. With the unsent portion
bounded to about one frame, the tail cost falls to a few frames'
transmission. Liveness, hop count, and ordering are unaffected either
way; this is a tuning note for deployments, and step 3 measures it.

### The fix is to bound the unsent portion, not the buffer

You cannot shrink the buffer below a BDP without losing throughput,
because the in-flight bytes have to live somewhere. What you want is a
buffer that is deep enough to hold a BDP in flight but shallow in unsent
bytes. That is precisely what the `TCP_NOTSENT_LOWAT` socket option does:
the socket reports itself unwritable whenever its *unsent* bytes exceed
the threshold, regardless of how much is in flight. The application then
writes only when the unsent queue has nearly drained, so each of the
mux's decisions takes effect within about a threshold's worth of
transmission.

The option is not universal. Linux has had it since 3.12 (2013), and
macOS and iOS have it (Apple introduced it for exactly this purpose).
FreeBSD does not define it, and neither does illumos: its `netinet/tcp.h`
carries no such option, its `tcp(4P)` manual documents none, and it
exposes no ioctl that reports queued send bytes either (only `FIONREAD`,
for the receive side), so a multiplexer on illumos cannot even poll the
unsent depth itself. Windows approaches the same problem from the other
side: `SIO_IDEAL_SEND_BACKLOG_QUERY` reports the amount of outstanding
data that would keep the connection full, so an application can size its
send buffer to that and no larger.

Where the option is absent, bound the unsent portion by bounding the
whole buffer: set `SO_SNDBUF` explicitly to about the bandwidth-delay
product plus one frame. In-flight bytes then fill most of the buffer and
unsent bytes cannot exceed the margin. This requires knowing the BDP,
which a rack-internal deployment does and a WAN deployment may not, and
it costs throughput if the BDP is underestimated. On illumos the send
buffer is a fixed per-connection size, 48 KB by default (`send_buf`),
unless `SO_SNDBUF` sets it, and `SO_SNDBUF` is capped by the `max_buf`
tunable (1 MB by default); a deployment there that needs more than 1 MB
in flight sets both deliberately, `max_buf` above the link's BDP and
`SO_SNDBUF` to BDP plus a frame, and one that does not is already
bounded by the buffer it has. The deployment docs present both cases on
equal footing, the option where it exists and the buffer bound where it
does not, since the crate is general-purpose and neither case is the
primary one.

With the threshold at one frame, the worst case for a thin reply is: the
mux has just written a supply frame (up to the frame bound, about 1.6 MB
by default), the reply becomes ready, the socket stays unwritable until
that frame has mostly drained, then the reply is written behind at most a
threshold more. About two frames' transmission in all: 26 ms at 1 Gbps,
2.6 ms at 10 Gbps. Smaller supply frames shrink it further, and that knob
already exists (`target_message_size`, negotiated to the smaller of the
two peers' settings), because the default frame bound was chosen for
memory symmetry with the largest *query* reply, not because supplies need
to be that large.

Two things worth being clear about. This is a latency effect only;
parking guarantees liveness whatever the buffer holds. And the crate
cannot set the option, because it sees only the read and write halves; it
belongs in the deployment's socket setup, which is why it is recorded as a
deployment note rather than a code change.

### Why the stream bundle never had this problem

On the bundle, each level had its own connection or QUIC stream, and the
kernel or QUIC stack interleaves *across* streams at packet granularity,
about 1.4 KB at a time. A level's unsent bytes delay only that level. A
thin reply on its own stream reaches the wire within a packet or two of
being written, microseconds rather than milliseconds. That is the
bundle's one real latency advantage on a loss-free link, and it is the
thing `TCP_NOTSENT_LOWAT` plus supply deprioritization recovers to within
a couple of frames.

There is a receive-side twin, smaller and harmless: a thin frame behind
bulk in the kernel's *receive* buffer waits for the demultiplexer to read
and hand off the bulk ahead of it, which is decode time rather than
transmission time, and it is bounded by one frame because the handoffs
are capacity one and the decoders run concurrently.

## Step 3: the multiplexer's residual, measured

The multiplexer prefers question-bearing frames from the start (ruled
2026-09-21); every frame is still served in finite time because a
session's traffic is finite. What remains is the residual the previous
section locates: a thin reply waits for the frame in progress plus the
socket's *unsent* bytes, never for the unsent supply backlog above the
socket. Measure that on the delayed pipe with a maximum-size supply frame
committed ahead of a thin deep reply, once with the socket's unsent bytes
unbounded and once bounded (`TCP_NOTSENT_LOWAT` at about one frame where
the option exists; `SO_SNDBUF` at about BDP plus one frame where it does
not, illumos included), and record both. The deployment docs then say two
things for TCP: bound the socket's unsent bytes by whichever of those two
means the platform offers, and choose `target_message_size` for the
head-of-line delay a deployment will accept, since the default is sized
for memory symmetry rather than for latency. The dependency-hop
ledger must stay fixed; elapsed latency may change.

## Step 4: records

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
- The opening-shortcut removal re-accepts its snapshots in a commit that
  names the change, and nothing else moves in them.
- Step 1's scope-publication, early-publication, and queue-custody checks
  pass, including a `next_scopes` reduction from a larger old window.
- The one-pipe wedge stalls at parking 1 and completes at the derived
  capacity in two named tests. A separate fixture reaches the occupancy
  bound and then completes; no minimum-capacity claim follows from it.
- No arriving reply finds its parking queue already at capacity across
  the stress matrix and generated schedules. Occupancy equal to capacity
  after insertion is allowed. Unexpected overflow fails without waiting.
- After step 2, `just gate` is clean, the hop ledger matches its
  pre-collapse value, and snapshot differences are stream labels and
  in-band control items only, identified in the commit.
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
  repair this. Check both bookkeeping moves and backend independence when
  diagnosing a one-pipe stall.
- **Byte estimates underprice retained state.** The absolute count does
  not validate the statistical price. Audit scopes and auxiliary queues;
  clustered addresses can exceed the model even with correct accounting.
- **Absorbed content on failure.** Uncommitted supplies become garbage.
  `Local` releases its `Arc`s; a persistent backend must reclaim nodes,
  as it already must for `Ready` slots filled before commit. This custody
  is accepted by the owner.
- **Snapshot churn under scheduling.** Grouping by logical stream index
  prevents interleaving alone from changing the rendered capture.
- **Serialization and loss.** Step 3 measures queuing effects. One TCP
  loss recovery delays every level, including work that could have
  overlapped recovery on independent QUIC streams.
