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

Receive side only, and a resizing rather than a re-plumbing: two existing
channels widen, one publish moves to a later point on its existing edge,
and the opening decoder gains a buffer. No new edge enters the dataflow.
Capacity monotonicity means widening a receive-side channel cannot cost
liveness, so this lands on the multi-stream `Link` with every existing
suite as its regression net.

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

### The link, dissolved

Nothing remains for a `Link` to carry. Its `SessionState` held an epoch
(the stream-label tripwire, gone with the labels) and a poison latch (a
failed session leaves the byte stream mid-frame, so the next session on it
must fail fast). Ownership expresses the latch better than a flag:

- The session functions take the halves directly, by value, and return
  them on success:
  `Rumors::gossip_once<R, W>(&self, read: R, write: W) -> Result<(Gossiped, R, W), Error>`,
  with `R: AsyncRead + Unpin + Send` and `W: AsyncWrite + Unpin + Send`;
  the continuous `gossip` driver takes them by value and owns them for
  its life (dropping the driver drops the connection, which is what
  "cancellation poisons the link" meant); `Bootstrap::join` and
  `Peer::retire` return the halves inside their success outcomes. A
  failed session keeps the halves, so a caller who passed them by value
  cannot reuse a pipe whose position is unknown, and `Error::LinkPoisoned`
  goes.
- The by-value signature already admits references: `&mut T` satisfies
  the same `AsyncRead`/`AsyncWrite` bounds through tokio's blanket impls,
  so a caller who wants to keep the halves passes `&mut read, &mut write`
  and gets them back regardless of outcome. Ownership then guards reuse
  only for callers who opt into it, so the session functions' docs must
  carry the rule for the other case: after an error, the byte stream's
  position is unknown, and running another session on it yields a
  garbled preamble or a read that waits on a peer that has given up,
  rather than a typed error. Discard the connection. (Ruled 2026-09-20.)
- Delete `link.rs` and everything under it: `Link`, `SessionState`,
  `map_transport` (callers wrap the halves themselves), `Connector`,
  `Acceptor`, `Done`, `link::STREAM_COUNT` (the public constant;
  `window.rs` takes the decoder count from the codec's `Stream::COUNT`),
  `link/erased.rs` (the `DynRead`/`DynWrite` erasure in `peer/gossip.rs`
  is what remains, internal), `link/routed.rs` and `link/routed/*`,
  `conformance/link.rs` and `conformance/link/*`, and their tests. The
  `conformance` feature keeps `bookmark` only. `link::memory()` becomes a
  `tokio::io::duplex` pair the tests build where they need it (a helper in
  `testing` if the repetition warrants one), not a public constructor.
- The requirement on the halves, stated on the session functions' docs
  rather than as a contract module: one reliable, ordered, duplex byte
  stream; the two directions independent (a side's read progresses while
  its write is blocked, which the greeting exchange needs); receiver-paced
  backpressure at any positive capacity; EOF or error on peer departure.
  The security paragraph (authentication is authorization, integrity,
  confidentiality, freshness) moves to the crate docs unchanged.

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
- Every peer-controlled wait on the receive path becomes a typed
  violation, per the rule for full queues above: parking at capacity on
  arrival, a reply begun beyond the flushed-question count, an opening
  supply outside the computed batch. The demultiplexer waits only on
  bounded local work.

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

- `SessionTransport` becomes the two erased halves; the session funnels
  in `peer/gossip.rs` lose their `SessionState::begin`/`finish` calls and
  return the halves on success instead.
- `testing::transport` loses the `Connect`/`Accept` fault kinds;
  `testing::memnet` goes; the testkit's `routed_tcp` and `tcp` helpers
  become one TCP-pipe helper; `tests/routed_link.rs` goes;
  `tests/latency_link.rs` goes with the suite it ran, and the latency bench
  moves to a delayed duplex pair; `tests/hop_trace.rs` re-anchors on the pipe and must reproduce its
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
  session signatures.

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
