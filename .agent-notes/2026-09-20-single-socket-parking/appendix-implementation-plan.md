# Appendix: implementation plan

For a reader who knows the code. The [exposition](exposition.md) is the
argument; this is the route from the tree at `772cce34` to a crate whose
sessions run over one `AsyncRead + AsyncWrite` pair. Steps are ordered so
that every intermediate tree is gate-clean and every claim lands with its
instrument. Public-API shapes below are proposals for the owner's ruling,
not decisions.

Vocabulary used throughout: *walk* is the materialized participant
(`streaming/materialized/`); *proxy* is the wire-bound participant
(`streaming/remote/`); *height* is the code's coordinate (leaves at 0, root
at 32), where the exposition said *level*.

## What does not change

The walk, in full: `yield_resolve_query!`, its two ordering rules, the
assembler chain, the `Window` derivation's search, the population bounds,
the version-size pricing, the greeting, the `Reply`/`Reaction` vocabulary,
the frame signal grammar (`codec/signal.rs`), supply runs and the
`RunBudget`, the causal sieve, the violation vocabulary, and every
in-process test that drives two walks through `mirror_connected`.

## Step 0: pin the count on today's code

No behavior change. Land the instrument first.

- The quantity: per party, per height `h`, `flushed(h) − paired(h)`, where
  `flushed` counts questions the proxy's encoder has written to the wire
  (the `batch.len()` it publishes into `local_questions` after
  `write_reply`, plus the batch in hand while blocked publishing) and
  `paired` counts questions the walk's stage at `h` has dequeued and matched
  with a reply. The proxy's `Progress` trace (`proxy/work/progress.rs`)
  already records `wire_reply(height, batch_len)` and
  `decoded_reply(height, count)`; add the pairing event or read the
  `InternalChildQueries` receive count from the channel stats.
- The assertion: high-water of that quantity `≤ window.capacity(h) + FAN + 1`
  at every height, for every run of `streaming/tests/capacity.rs`'s stress
  matrix, the proxy harness suites under `remote/proxy/tests/`, and the
  adversarial scheduler in `testing::schedule`, at the budget window and at
  `WindowConfig::FLOOR`.
- Also pin the two facts the count rests on, as named tests if they are not
  already: a stage yields no second reply before its current reply's
  questions are all recorded (the sequential-scope shape of the
  `levels.rs` loops); and the proxy encoder publishes a reply's scopes
  only after that reply's last frame flushed, and takes no further local
  reply while blocked publishing (`encode::replies`).

Commit: "Pin the in-flight question bound".

## Step 1: park decoded replies, on the existing transport

Receive side only. Capacity monotonicity means widening a receive-side
channel cannot cost liveness, so this lands on the multi-stream `Link` with
every existing suite as its regression net.

1. **`ProxyResponses` widens.** Today `Work::respond` (`proxy/work.rs`)
   builds it at capacity 1 with the argument "one buffered response is
   sufficient". Move the constructor into `proxy/work/queues.rs` beside its
   siblings and build it at `window.capacity(H::HEIGHT) + FAN + 1`, with the
   counting argument of exposition §6.1 written at the constructor. The
   relay task (`relay`) is unchanged; its send now never blocks for a
   conforming peer.
2. **`local_questions` widens** to the same capacity. Its doc already
   observes that occupancy "can undercount the wire slightly"; restate that
   the undercount is at most `FAN + 1` and that the queue is sized so the
   encoder's publish never waits. Overflow here would mean the local walk
   over-asked, a crate bug and not a peer violation: `debug_assert`, never a
   wait.
3. **Scopes release at take time, not decode time.** The decode pump
   currently yields the reply and then publishes its derived scopes into
   `next_scopes` (window-sized), blocking there if full. Once the pump feeds
   a parking queue it must never block on anything downstream, so it yields
   `(Reply, Vec<Scope>)` and touches `next_scopes` no longer. The scopes are
   published at the walk-facing exit of `Work::respond`, immediately before
   the reply is handed to the walk (an async map over the response stream).
   `next_scopes` capacity becomes `FAN + 1`, with this argument at the
   constructor: when the walk takes reply *r + 1*, every scope of reply *r*
   has been dequeued by the encoder except possibly one (the encoder
   dequeues scope-first and may still be writing the previous reply), and
   the walk cannot take *r + 2* until the encoder has taken all of
   *r + 1*'s replies, so occupancy never exceeds one straggler plus one
   reply's fan. The `yield_reply_scopes!` macro's ordering role ("reply
   before its scopes") moves to that exit and is restated there.
4. **The opening supplies park too.** The initiator's early supplies
   (`OpeningSupplies`, stream 0 in the initiator direction) are decoded
   incrementally and consumed by the under-under-root decode pump's
   `advance_to` calls as it meets each root-level request. Once the pump
   feeds a parking queue it may run ahead of the walk, but it may also lag
   behind the demux, and today an unpolled opening decoder would back its
   frames up into a capacity-1 handoff and stall the demux. Give the opening
   decoder its own task and a `FAN`-deep queue of decoded root children
   (one handle each; the batch is at most one fan by construction, and the
   responder computes its membership from the two listings), so the demux
   never waits on it.
5. **The window prices the parked half.** In `window::from_budget`, add to
   the per-depth scope price a term for the parked reply: at child depth
   `d`, `children_quantile(n, d − 1)` reactions, each costing a reaction
   slot plus `children_quantile(n, d) × LISTING_ENTRY_BYTES` for a query
   listing (supply reactions cost one handle, already priced). Add the fixed
   slack pre-charge, `min(FAN + 1, 256^(d−1)) × parked_reply_price(d)` at
   every depth, beside the existing `supply_fans` pre-charge it mirrors
   (both sides run the same calculation without knowing their role, so
   charging every depth rather than one parity's is the conservative choice
   the existing loop already makes). The cap is structural, not
   statistical: questions at child depth `d` are about nodes at depth
   `d − 1`, of which at most `256^(d−1)` exist, so the parking capacity
   itself may be `min(K(d) + FAN + 1, 256^(d−1))` without weakening the
   never-full argument. That cap is what keeps the floor cheap at the top of
   the tree. The exposition's "a few megabytes" for the reference case (two
   divergent sets of 100,000 messages, zero budget) is, under the uniform
   model: depth 1, one reply of ≈ 256 children × ≈ 200 grandchildren
   × 25 B ≈ 1.3 MB; depth 2, at most 256 replies of ≈ 200 reactions
   × ≈ 1.5 grandchildren × 25 B ≈ 8 KB each ≈ 2 MB; depth 3 and below,
   258 replies of a few reactions each, tens of kilobytes. About 3.4 MB in
   all. A derivation, not a measurement; `just window-tradeoff` will report
   the real figure once the term is in. Derive the new slot constants by
   `size_of` as the existing ones are. Regenerate `window/tradeoff.md` with
   `just window-tradeoff`; re-baseline `tests/window/{census,corners,knee,
   operator,tradeoff_probe}.rs` with the parent-commit measurement recorded
   first so the attribution is clean. Update `sizing.rs` and the
   `Peer::sync_memory_budget` accounting paragraph.
6. **Tests and pins.** The step-0 high-water test now also reads
   `ProxyResponses` occupancy and asserts it never reaches capacity. The
   `ProxyLocalQuestions` occupancy derivation (the harvested
   `min(capacity, S)` supremum) re-derives at the new capacity. The
   adapter's `fan_occupancy` suite, which pins the per-reply decode fan, is
   untouched: the decoder still holds one reply's skeleton and one fan of
   leaves at a time. The prose that says "at most one decoded reply in
   flight per stage" (`message.rs`'s memory-unit paragraph, `stages.rs`'s
   module doc, `Work::respond`'s comment) re-denominates to the parking
   bound; `capacity_stress_covers_every_queue_role` keeps covering the
   proxy edges at their new widths.

Commit: "Park decoded replies per level".

## Step 2: demonstrate one pipe, before touching the API

An adequacy demonstration that the parking is the cure, on the current wire
format, with the current `Link` API untouched.

1. **A multiplexed test transport.** Under `remote/proxy/tests/`, a harness
   that drives two proxies over one `tokio::io::duplex` pair per direction:
   a test-only `Connector`/`Acceptor` whose "streams" are all written to and
   read from the one pipe, each stream's label and frames written whole
   under a lock so frames never interleave mid-frame, with the receiver
   demultiplexing by label and then by the frame's own stream index into
   per-stream handoffs. This is precisely the instantiation the July
   deadlock analysis rejected, and it should still fail the link
   conformance suite's independence probe, which is correct: the pipe does
   couple streams. The claim is that sessions complete anyway. The archive
   branch `wave1/integration` holds the single-socket campaign's harness,
   which may be the cheapest starting point; if neither is cheap, fold this
   demonstration into step 3's first commit, where the wedge at parking one
   is the known-bad, and let step 0's measurement carry the confidence
   until then.
2. **The known-bad and the cure.** `streaming/tests/wedge.rs` already pins
   the deterministic tree pair. Run it over the multiplexed transport twice:
   with `ProxyResponses` forced back to capacity 1 (a test knob, since
   step 1 made the capacity a constructor argument), `run_to_quiescence`
   must report `Stalled`; at the step-1 capacity it must complete and match
   `Tree::join`. Run the capacity stress matrix over the same transport.
3. **The invariant as a property, and its tightness.** Over the same
   transport, a proptest across generated tree pairs and the adversarial
   scheduler asserting that no `ProxyResponses` queue ever reaches its
   capacity. And one constructed shape that parks exactly
   `window.capacity(h) + FAN + 1` replies at some height: a full-fan reply
   whose questions cannot all be recorded because the stage below holds
   its one question and awaits a reply the scheduler delays, while the
   peer answers everything. At one slot less it must stall. This is the
   demonstration that the `+ 1` is real; commit both as named tests.
4. Any stall at the step-1 capacity is a finding about the count, not about
   the transport; stop and re-derive before proceeding.

Commit: "Demonstrate single-pipe liveness with parked replies".

## Step 3: collapse the transport

One atomic commit for the code (nothing compiles halfway), then a prose
sweep. This is the public-API change; per the triage plan, Sush's
compatibility branch is updated and tested alongside it.

### The link

- `Link<R, W>` holds `read: R`, `write: W`, and `SessionState`. The
  "control" qualifier on the halves has no referent once there is one
  stream; propose plain `read`/`write`. `SessionState` keeps its poison
  latch; the epoch counter loses its label tripwire role and may stay as a
  session count or go.
- Delete `Connector`, `Acceptor`, `Done`, `link::STREAM_COUNT` (the public
  constant; `window.rs` takes the decoder count from the codec's
  `Stream::COUNT`), `link/erased.rs` (the `DynRead`/`DynWrite` erasure in
  `peer/gossip.rs` is what remains), `link/routed.rs` and `link/routed/*`,
  `conformance/link.rs` and `conformance/link/*`, and their tests. The
  `conformance` feature keeps `bookmark`; whether a minimal pipe check
  (duplex independence, EOF on departure) is worth keeping is the owner's
  call.
- `link::memory()` and `memory_with_capacity()` return a `tokio::io::duplex`
  pair per direction.
- The contract, restated in `link.rs`'s module doc: one reliable, ordered,
  duplex byte stream; the two directions independent (a side's read
  progresses while its write is blocked); receiver-paced backpressure at
  any positive capacity; EOF or error on peer departure. The security
  paragraph is unchanged.

### The proxy

- New module `remote/pipe.rs` (name open): a *mux* task owning the write
  half, fed by per-height frame channels of capacity 1, taking ready frames
  round-robin and writing and flushing each; a *demux* task owning the read
  half, decoding frame heads with `FrameRead`, routing by stream index into
  per-height capacity-1 handoffs, and stopping on a head that is not a
  frame (the epilogue marker or a party donation), handing the reader back
  with that lookahead. The demux thereby absorbs the departure watch and
  `ControlRead`'s replay role.
- `StreamSender` and `StreamReceiver` bind to the mux and demux handoffs:
  no label write, no claim, no `Done`. `AcceptDriver`, `Claims`,
  `ClaimSlots`, `AcceptError`, `StreamError::{Mislabeled, SupplyClosed}`
  and the `label` renderer go. `Truncated` stays (pipe EOF before a
  height's `End::Stream`).
- `state.rs`'s `Session` holds the mux and demux handles where it held
  connector, claims, and route; `Physical` loses `accept` and `errors`;
  `work.rs::execute` selects the protocol against the demux's single
  failure route.
- Parking overflow becomes a typed violation at the demux (a reply arrived
  for a height with no room, which a conforming peer cannot cause): fail
  fast, never wait.

### The wire

- Frames are unchanged. Stream labels are gone. Preamble, greeting, frames,
  each height's `End::Stream`, then the epilogue marker (or the party
  hand-off) follow in order on the one pipe. Every frame opens with a CBOR
  array head; the epilogue marker opens with a text head and a party
  donation with a tag head, so the demux's stop rule is "any head that is
  not an array head ends the data phase", and it hands the reader back
  with that head as lookahead.
- Pre-release, this is a deliberate wire-format change: `tests/gossip_snapshot.rs`,
  `tests/protocol_overhead.rs`, and the `insta` snapshots re-accept in this
  commit, named as "single-pipe framing: stream labels removed; control
  items in-band". Bookmark pins do not move.
- The capture harness (`crates/rumors-testkit/src/common/gossip_snapshot.rs`)
  groups items by logical stream today using the parsed label. It groups
  by the frame's own stream index instead, taken from the observer hooks,
  so renders stay independent of the mux's interleaving.

### The session drivers and tests

- `Rumors::gossip_once`, `Bootstrap::join`, `Peer::retire`, and the gossip
  driver take `&mut Link<R, W>`; `SessionTransport` becomes the two erased
  halves plus the session state.
- `testing::transport` loses the `Connect`/`Accept` fault kinds;
  `testing::memnet` goes; the testkit's `routed_tcp` and `tcp` helpers
  become one TCP-pipe helper; `tests/routed_link.rs` goes;
  `tests/latency_link.rs` and the latency bench move to a delayed duplex
  pair; `tests/hop_trace.rs` re-anchors on the pipe and must reproduce its
  ledger (`3 + L + 1`); `benches/window_wallclock.rs` runs over the pipe.
- The observation surface keeps `StreamObserver` keyed by logical stream
  index and direction; only its prose changes ("data stream" becomes
  "logical stream").

### Prose

`lib.rs` (the transport paragraphs and the example), `reconciliation.rs`
("The bytes on the wire" is rewritten around one stream and parking),
`link.rs`, `sizing.rs`, `tutorial.rs`, `observe.rs`, `AGENTS.md`'s transport
bullet, `just readme`. Every mention of stream supplies, labels, claims,
routers, and per-stream flow control is excised or re-denominated, per the
no-ghost-references rule.

Commits: "Collapse the transport to one pipe" (code and snapshots), then
"Re-denominate the transport prose".

## Step 4: the multiplexer's policy, measured

Round-robin at frame granularity is live under any policy, so it ships
first. Then measure the one inherent cost: a supply frame at the maximum
run budget ahead of a thin deep reply, on the delayed pipe, with the delay
recorded. If it matters at realistic budgets, prefer question-bearing
frames over supply frames and document a small socket send buffer for TCP
deployments. The hop ledger must not move under either policy.

## Step 5: records

- This note's README records the determination and the owner's rulings.
- `formal/README.md`'s summary of the mux campaign's product conclusion is
  revised to state the counting bound and to point here; the Lean elastic
  model's doc comment, which reads the K-parking result as "no fixed
  parking bound survives", is restated as "no bound independent of the fan
  and the window survives". A charter for the counting lemma may be added
  to `Mux/Charters.lean`; it is secondary and no step above waits on it.

## Acceptance

- Step 0's high-water test is green at both the budget window and the
  floor, on the multi-stream transport, before step 1 lands.
- Step 2's wedge stalls at parking 1 and completes at the derived capacity,
  committed as two named tests.
- After step 3, `just gate` is clean; the hop ledger over the pipe equals
  the pre-collapse ledger; the snapshot diff is labels and in-band control
  items only, and the commit message says so; the window census is
  re-baselined with its parent-commit measurement recorded.
- Sush's compatibility branch builds and its tests pass against the new
  `Link`.

## Risks, named

- **The count is off by a constant.** The `+ 1` terms were derived by
  reading, not measured. Step 0 measures them; a high-water above the bound
  is a finding to resolve before step 1, not a reason to widen.
- **Absorbed content on failure.** Eager absorption puts supplied subtrees
  into the backend before the session commits; on failure they are garbage.
  For `Local` they are dropped `Arc`s; a persistent backend must reclaim
  them, which is its existing obligation for the `Ready` slots the walk
  already fills before commit. Ruled acceptable.
- **Snapshot churn under scheduling changes.** Only if the capture harness
  renders in pipe order; grouping by stream index (step 3) prevents it.
- **Byte head-of-line.** Inherent to one socket; step 4 measures it and
  bounds it by the run budget and the mux policy.
