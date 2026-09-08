# 4. Implementation and verification

[Contents](README.md) · [Next: comparison appendix](05-review.md)

## 4.1 How to use this implementation plan

Complete the work packages in order. A package's acceptance criteria are
conditions for advancing, not a list to defer until the end. Keep incomplete
persistence machinery private or experimental until crash ownership and
identity recovery are tested. Do not ship a persistent mode that intentionally
has no reclamation and promise to add its ownership model later.

Before each implementation commit, run `just gate` as required by the
repository. Use the justfile for narrower iteration. Regenerate READMEs with
`just readme` after crate-level rustdoc changes. Every new test has an English
doc comment; families of schedules and failure boundaries are proptests.
Commit every generated regression seed at its resolved persistence path.

## 4.2 Package 0: pin the obligations and baseline

Read the crate docs and the modules listed in the source map below at the
implementation revision. Confirm how local commits, session snapshots,
identity donation, payload decoding, and observers interact.

Write a small state model before broad Rust refactoring. It should include
immutable objects and edges, root holds, zero-count work, a current head,
private preparations, two indexes, and identity reservations. A test-only
history records actual emitted messages for the causal invariant. This model
is an **oracle**: an independent calculation of the expected result, used to
check the implementation.

Model the following atomic transitions: bounded object construction, hold
release, one zero-object reclamation, root publication, group binding, party
reservation, and relinquishment. Model a crash between every transition and
both outcomes of an interrupted atomic write.

Measure the existing memory mode on representative small and large payloads,
different version widths, sparse deltas, bulk bootstrap, redaction, snapshots,
and observers. Separate CPU, allocated bytes, peak live nodes, and compile
cost. Record workload sizes and measurement uncertainty before selecting an
acceptable overhead threshold.

Acceptance:

- The model checks the count equation continuously, not only after cleanup.
- A bounded publication touches no per-message ownership records.
- The model exposes unsafe donation-before-durable-relinquishment and
  unordered-group publication as failing negative controls.
- The baseline distinguishes content-tree performance from the proposed
  causal index's necessary extra work.

## 4.3 Package 1: byte storage and the ownership layer

Implement private `Store` capabilities, structured error outcomes, allocator,
record framing, counts, construction holds, retained roots, and zero-count
reclamation. Include the admission-group reservation as an explicit object
construction state from the beginning.

Build a test store with separate visible and durable state, atomic batches,
fault injection, and a pending-write model. A plain `BTreeMap` clone alone
does not exercise uncertain completion, cancellation, or write settlement.
Scans should be able to interleave with unrelated writes without promising
snapshots.

Use a synthetic immutable graph with no cycles to exercise ownership before
tree integration.
Cover repeated edges to the same child, shared roots, cancellation during
construction, two concurrent collectors, and dropping root handles while a
publication transfers their durable ownership.

Acceptance:

- Each crash recovers a valid graph, and abandoning all private owners leaves
  exactly the current graph once reclamation drains.
- An incoming edge is never installed without a protecting owner of its
  target; zero-count objects cannot be revived.
- No count or cleanup update is lost when the same object is referenced more
  than once in a batch.
- Active-root registration and dropped-hold cleanup remain bounded as object
  count grows. Parked sort-file references retain their objects through an
  operation-level registration. No root list in RAM grows with the prepared tree.
- One slow bounded ownership transaction can delay the gate; a whole
  multi-batch construction cannot retain it between batches.

## 4.4 Package 2: immutable content storage and reads

Add body, received-candidate, final-message, content-leaf, and branch records.
A received candidate is an immutable version/body pair without final local
placement; it is private to reconstruction. Add loaded-node guards and a
bounded decode cache whose child references are raw IDs.

Adapt the existing session backend boundary to these handles. In particular,
the current `Leaf::leaf(version, message)` surface has neither an instance
argument nor ordering context. Introduce explicit ingress context for the
store, operation owner, codec, and origin tag; do not hide it in thread-local
state. The outgoing leaf surface must fetch body bytes asynchronously while
keeping source ownership alive.

Implement persistent `join`, `unknown`, bulk assembly, point lookup, and an
address walk. Reuse pure rules and representation-independent helpers where
that removes duplication. Keep the existing synchronous memory traversals;
do not turn every memory operation into a boxed future and poll it with a
no-op waker as an initial architectural requirement.

Use explicit bounded traversal state where needed for async reads. Retaining
the height-typed protocol does not require every storage walk to encode its
entire recursive future in the type system.

Acceptance:

- Generate one legal single-universe event history and replay its immutable
  versioned inputs through both storage representations. Persistent content
  matches `Tree::join`; the oracle is a shadow calculation, not another live
  author holding a duplicate party. Never mix independently seeded universes.
- Empty-after-redaction preserves its frontier.
- A held snapshot survives replacements and reclamation; closing it makes
  exclusive old objects reclaimable.
- A complete scan with a small cache does not retain the whole decoded graph.
- Backend memory pricing includes loaded records and guards, not Local's
  pointer-only charge.

## 4.5 Package 3: the causal index and immutable placement

Implement the 16-byte index trie, group reservation/binding, index-only
iteration, range pruning, and temporary origin/address sorts. In memory,
the corresponding group binding can be a one-time initialization shared by
`Arc`; it does not require an engine or a persistent count.

Implement the candidate-to-final-message conversion and the address-ordered
replacement build. Test sparse path-compressed scopes, whole-root imports,
and a candidate removed from the reconstructed result before finalization.
The temporary order map is replayable for admission retries.

Integrate exact index additions and removals. Keep existing message records
when joining equal content, and ensure prefix changes do not change a
message's local position.

Acceptance:

- Content and index name exactly the same message set at every publication.
- For generated emitted messages, every yielded cause precedes its effect.
- No published object reaches an unresolved group.
- A group stays readable while only an old snapshot references its messages,
  then disappears after their last references are released.
- Duplicate origin tags are errors, not overwrites; overflow cannot wrap a
  position or publication number.
- Sorting exceeds memory and merge-fanout limits in tests and still remains
  bounded, including its run inventory and output buffers.

## 4.6 Package 4: concurrent publication and local mutation

Implement prepared-root ownership transfer and head publication. Include
storage outcome guards around the durable-write/in-memory-view boundary.
Add gossip rebasing against a newer head and a bounded retry-work policy.
Start with a streaming candidate rescan; add selective retry only with an
actual region-skipping algorithm.

Implement persistent local mutations from replayable intents. An unsuccessful
attempt must discard draft versions and regenerate them; the user closure
runs once. Add a bounded callback batch and an async staged builder.

Add body sharing for imported messages using the optional directory and
counted body holds. Keep speculative local bodies out of that directory.
This can be a separate commit within the package: correctness never depends
on eliminating duplicate transient body writes.

Acceptance:

- Force an unrelated publication between preparation and commit for every
  overlap/redaction case in section 3. The result equals the corresponding
  serial oracle operation on the new base.
- Two draft local sends with equal versions and different payloads cannot
  both publish or be merged as equal content.
- A payload destructor that takes a snapshot cannot run under the view lock;
  panics and cancellation do not expose partial state.
- Publication performs a constant number of ownership-record operations as
  the update grows. Instrument count reads, count writes, and gate ownership,
  not just elapsed time.
- During a large preparation and reclamation, small independent sessions
  continue taking ownership and publication turns. No retry fallback holds
  the gate through a walk.
- A contention result is distinct from corruption and leaves the peer usable.

## 4.7 Package 5: identity, opening, and the wire

Implement empty/active/retired opening states and durable unsent reservations.
Exercise persistent seed, bootstrap provider and recipient, ordinary gossip,
retirement donor and recipient, and reopen. Keep the memory bookmark path.
Restore format and codec-relevant configuration before decoding stored data.
Application payload-schema compatibility remains an application obligation.
If a decoder or admission-setting change can reject previously accepted
bytes, require an explicit streaming validation/migration before enabling
gossip from that store. Lazy record checks and a root-format version do not
establish compatibility of every stored payload with a new `T`.

Implement the owner-reviewed wire change for origin tags. Update every leaf
encoder, decoder, observer/renderer path, byte-accounting formula, and fixture.
Do not claim unchanged memory or stream behavior simply because the new
field is fixed-width.

Acceptance:

- Crash before and after reserve, relinquish, first identity-byte write,
  recipient publication, and completion certificate. No schedule produces
  overlapping active parties or reuses an externally observed version.
- An ambiguous handoff can strand authority but cannot recover it twice.
- A retired store cannot reopen as active; corrupt storage cannot become
  `Empty`; changing universe or store identity is never an implicit repair.
- A crash of all persistent peers followed by reopen requires no new seed.
  Their state and identity remain usable under the established universe.
- Mixed memory/persistent sessions and all-memory sessions obey the same
  wire contract and causal-order rules.
- Slow storage, cancellation, one-byte transport buffers, and concurrent
  reclamation do not introduce a wait cycle.
- Reaccept deliberate wire snapshots through the repository's prescribed
  process and name the protocol change in the implementing commit.

## 4.8 Package 6: public observers and lifecycle API

Keep the public facade small. Preserve `Peer<T, Bookmark>` and its memory
behavior while developing persistence behind a separate module/facade; decide
whether to unify generic parameters only after the internal boundary works.
Avoid exposing one omnibus backend trait covering storage, ordering,
reclamation, and session scheduling.

Illustrative surface, with names to settle during API review:

```text
persistent::Peer::open(store) -> Active(peer) | Empty(store) | Retired(store)
persistent::Peer::seed_in(store)
persistent::Peer::bootstrap_in(store)

rumors.try_send(value).await
rumors.try_redact(version).await
rumors.try_batch(bounded_callback).await
rumors.begin_batch().await -> async staged builder -> commit().await

rumors.snapshot() -> library-owned snapshot
snapshot.try_get(version).await
snapshot.iter() / range(query) -> fallible causal streams
snapshot.unordered() -> explicitly unordered fallible stream

rumors.causal_messages_from(position) -> fallible delivery stream
delivery.after_position() -> serializable local position
observer.checkpoint() -> conservative portable Version

snapshot.release().await
rumors.reclaim().await
peer.close().await
```

Opening and seeding remain distinct operations. An active peer with no
messages is still an active peer. Storage errors carry backend context and
the stage of failure. Unknown outcomes, definite operation rejection, and
maintenance failures after commitment are different categories.

Snapshot release and close do not promise to free objects another live view
still holds. Closing prevents new operations and settles outstanding work;
the store's exclusive identity lease cannot be released while an old task
can still write to it. Specify how surviving snapshots retain read access or
delay final store closure; do not silently invalidate a valid snapshot.

Acceptance:

- Cursor resume at every delivery boundary, through redaction-only commits,
  concurrent publication, empty passes, final `None`, and reopen.
- A portable checkpoint never advances past an undelivered bounded pass.
- The observer releases storage views while quiet and has no unbounded
  per-pass backlog.
- Examples show application side effect/cursor ordering without an
  exactly-once claim, and distinguish local positions from portable versions.

## 4.9 Package 7: a real store, bounds, and operational documentation

Qualify at least one real adapter against the minimal byte-store contract and
run the crash/concurrency suites against it. Choose it by the ease of testing
atomic durability and bounded batches on supported targets, not by an assumed
engine snapshot facility. Keep additional adapters outside the core crate
until they add demonstrated coverage or application value.

Run larger-than-cache tests for bootstrap, sparse and overlapping gossip,
large redactions, held snapshots, interrupted sorting, restart cleanup, and
sustained small commits competing with a large prepared session.

Report both owned allocations and process-level observations. Include the
engine's queues/caches and filesystem cache effects. Process resident memory
is not an exact accounting of Rust objects.

Acceptance:

- Controlled memory stays bounded as set and delta sizes increase under
  fixed value-size and concurrency limits.
- Once external views and private jobs are released and maintenance drains,
  every remaining object is reachable from the head, every body directory
  entry is valid, and no zero-work/hold/reservation debris remains.
- Causal-index storage, duplicate transient writes, sort passes, barriers,
  gate transaction sizes, and conflict work are reported separately.
- Storage reuse is demonstrated under long churn; immediate file shrinkage
  is not substituted for the reachability test.
- The formal model and maintainer documentation describe the implemented
  transition system, and the public tutorial includes a durable lifecycle.

## 4.10 Resource accounting and knobs

Use a budget ledger that covers simultaneous allocations:

```text
controlled resident memory <=
    shared decode/body caches and active decoded guards
  + bounded root/operation registry
  + active_sessions * per_session_working_budget
  + bounded local-batch preparation
  + ownership-write and reclamation buffers

value-sized costs = encoded/decoded frontier and party,
                    largest admitted payload/version/record,
                    application T allocations,
                    adapter-owned memory
```

The per-session budget contains the wire window, ingress payloads, traversal
frames, all sort buffers, all merge readers and writers, and private batching.
Do not reserve a scratch fraction and then allocate that fraction once for
each sort space. Do not charge loaded persistent nodes as memory-mode pointers.

Start with explicit knobs for cache bytes, session working bytes, maximum
active sessions/views, bounded write bytes/operations, scratch RAM/disk,
value-size limits, maintenance work per call and backlog, and retry work.
Reject settings too small to perform one legal maximum-fanout ownership step.

The current session budget is useful as a baseline, not proof that it now
covers persistence. Select numeric defaults from the real-adapter workload
matrix. Avoid committing a 64 MiB cache formula or a one-millisecond reclaim
claim before measuring decoded records, version widths, and storage latency.

## 4.11 Decisions that remain measurements or review choices

The following choices are open to measurement or review. Immutable storage,
library-owned snapshots/counts, the minimal engine contract, and preparation
without a session-wide write lock are fixed requirements.

| Item | Initial recommendation | Evidence required to change it |
| --- | --- | --- |
| Final immutable records after origin sorting | Extra address-ordered metadata pass | A faster private-builder design with equally explicit cache/ownership boundaries |
| Index leaf version | Load the shared message metadata | I/O and version-size measurements favoring duplication |
| Imported body deduplication | Counted directory acquisition | Measured duplicate-write cost versus directory contention |
| Retry | Reuse trees; stream admission again; bounded retry work | Region-skipping algorithm plus work counters for selective rebasing |
| Gate batching | Fixed byte and operation caps | Small-session tail latency and durable barriers per message |
| Checksum | CRC-32C as an initial fault detector | Chosen fault model and measured alternative, not hostile-peer economics |
| Public facade | Separate persistence facade initially | Concrete examples showing a simpler unified generic API |
| Wire version | Deliberate V3 origin-tag vocabulary | Owner review before protocol implementation |

Large-session starvation under sustained writes is an unresolved performance
obligation, not permission to serialize a large commit. Record an acceptable
workload/latency target in package 0, measure it in packages 4 and 7, and
revisit the rebase algorithm if it fails.

## 4.12 Source map

| Existing source | Reason to inspect or extend |
| --- | --- |
| [src/lib.rs](../../src/lib.rs) | Universe, identity, runtime, payload, and observation contracts |
| [src/tree.rs](../../src/tree.rs) | Frontier outside the nodes; `act` and `join`; oracle |
| [src/tree/typed/untyped.rs](../../src/tree/typed/untyped.rs) | Compression, sharing, summaries, node representation |
| [src/tree/traverse/join.rs](../../src/tree/traverse/join.rs) | Base-preferred equality and deletion-honoring traversal |
| [src/tree/mirror/streaming/backend.rs](../../src/tree/mirror/streaming/backend.rs) | Materiality boundary and persistent node pricing |
| [src/tree/mirror/streaming/materialized.rs](../../src/tree/mirror/streaming/materialized.rs) | Schedule, source-set obligations, and wait-graph argument |
| [src/tree/mirror/streaming/remote/adapter/decode.rs](../../src/tree/mirror/streaming/remote/adapter/decode.rs) | Ingress custody and leaf construction context |
| [src/peer/gossip.rs](../../src/peer/gossip.rs) | Snapshot/fork atomicity, donation gates, absorption, completion |
| [src/batch.rs](../../src/batch.rs) | Closure runs once, queued actions, local version assignment |
| [src/message.rs](../../src/message.rs) | Exact cached bytes, codec and payload admission |
| [src/rumors/causal.rs](../../src/rumors/causal.rs) | Current causal ordering and checkpoint timing |
| [src/bookmark.rs](../../src/bookmark.rs) | Identity custody across restart and storage obligations |
| [src/link.rs](../../src/link.rs) | Session success, failure, cancellation, transport independence |
| [justfile](../../justfile) | Verification recipes and commit gate |
