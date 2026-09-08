# 2. Storage and ownership

[Contents](README.md) · [Next: operations](03-operations.md)

## 2.1 What the engine supplies

Keep the contract small. These are capabilities, not final Rust signatures:

```text
get(key) -> optional bytes
scan(lower, upper) -> ascending, incremental key/value stream
write(batch) -> atomically replace/delete the named keys, durably on success
```

The library uses tagged key spaces. An adapter may map tags to column
families or prefix them onto keys. Neither representation changes semantics.
The store is exclusively opened by one Rumors instance; the application must
not edit its key spaces behind the library.

Required details:

1. `get` returns one complete record. It agrees with completed writes.
2. A batch is atomic across all key spaces. Successful writes are durable.
   Recovery after an interrupted write exposes either its complete before
   state or its complete after state.
3. A batch is a bounded in-memory value. Larger preparations consist of many
   such batches; only the final publication changes the current replica.
4. Scans are bounded-memory, ordered, and support resuming after an exclusive
   key. They need no snapshot isolation. An immutable, owned range is stable;
   a changing maintenance range is visited with resumable passes, not treated
   as an atomic view of the store.
5. The adapter documents error and cancellation outcomes. It must settle or
   fence outstanding writes before handing storage to a new exclusive opener.
   A write from an abandoned instance must never arrive after recovery.
6. Ordinary reads of immutable records can proceed independently of the
   library's ownership gate. No adapter may retain an unbounded request queue
   or require a session to finish before an accepted storage request finishes.

Do not require “error means nothing committed.” A backend may lose the
completion result after committing. Represent a definitely aborted write
separately from an unknown outcome when the adapter can distinguish them.
An unknown ownership/publication outcome makes the current instance unusable
until the backend has settled its writes and the library has reopened it.

Temporary storage is separate: bounded buffers, ordered external sorting,
incremental scans, and explicit cleanup. It needs no crash durability because
no published object depends on temporary bytes. The caller can supply files
or an appropriate engine key space. This facility is not an engine snapshot.

## 2.2 Immutable objects

Use sequential, never-reused object IDs. IDs are storage identities, not
message identities and not Merkle hashes. Reserve ranges durably with a
small allocator update; unused IDs after a crash are harmless holes.

An object is immutable from the moment its complete record is installed.
Objects contain IDs of their dependencies, not recursively cached decoded
children. A content branch and a causal-index branch each have at most 256
children. An **owning edge** is a stored reference that keeps its target
allocated. Together, the objects and these references form an ownership graph.

The ordering metadata has two parts: which operation admitted a message
(its **admission group**), and its position among that operation's messages.
A received candidate also carries a **sender tag**, its position in the
sender's order. Chapter 3 explains how the receiver uses these to assign a
new local order. Start with these object kinds:

| Object | Immutable contents | Owning edges |
| --- | --- | --- |
| Body | Exact admitted payload bytes and their length | None; a chunked representation can add bounded-fanout body objects later |
| Received candidate | Version, body ID, sender tag; private to reconstruction | Body |
| Message | Version, body ID, admission-group ID, position within that group | Body and admission group |
| Content leaf | Compressed path and message ID | Message |
| Content branch | Compressed path, children, child digests, summaries | Children |
| Index leaf | Compressed local-order path and message ID | Message |
| Index branch | Compressed path, children, version span, count | Children |
| Admission group | Publication number assigned to the group | None |

The shared message object is useful: changing a leaf's compressed prefix
does not duplicate its payload or its placement metadata. The index points
to the same message object rather than a particular content-leaf wrapper.
Index correctness is equality of message membership, not equality of wrapper
pointers.

The graph must have no cycles: branches reference lower tree structure,
leaves reference message records, and messages reference bodies and groups.
Nothing points back from a body or group into either tree. Reference counting
alone would not reclaim a cycle. Temporary reconstruction nodes obey the same
rule, with received candidates in place of final messages.

A branch stores the summaries needed without descending: hash where
applicable, leaf count, version floor and ceiling, and maximum version-bound
size. The floor describes history common to every leaf; the ceiling contains
every leaf's history. Together they are the branch's **version span**, used
to accept or reject a whole subtree during a causal query or merge. This
summary is distinct from the replica's frontier, which survives deletion.
Summaries are computed at construction. Persist child digests inline in
content branches so supplying a child listing does not read every child.

An index leaf initially reads the version from its message object. It does
not copy potentially large version bytes merely to avoid that metadata read.
If measurement favors an inline version, that is a record-layout decision;
either layout keeps bodies out of index-only queries.

Admission groups need a special construction reservation because their
publication number is not known during preparation. Section 3 explains the
one-time binding. No published graph reaches an unresolved reservation.

## 2.3 What a reference count means

For each allocated object `x`, maintain:

```text
refs(x) = number of owning object edges to x
        + number of durable root entries owning x
```

Count **all** allocated objects, including private preparations and objects
waiting to be reclaimed. The equation does not depend on whether a node has
ever been published. Each occurrence of an owning edge counts, even if two
edges happen to target the same object.

A durable root entry is a small record saying that some operation or replica
view owns an object. Root entries include:

- the current replica's content and index roots;
- prepared roots and temporary construction roots;
- roots retained for older snapshots and session views.

A private root entry is called a **hold**. It protects work that is not yet
reachable from the current replica, such as a branch still being built.

The key spaces can initially be:

```text
Meta                 format, store ID, allocator state
Head                 roots, frontier, identity state, publication number
Objects              immutable records, by object ID
Refs                 reference counts, by object ID
Holds                private roots, by operation ID and hold ID
Retained             older views still represented by durable roots
Zero                 zero-count objects pending deletion
GroupReservations    groups awaiting their publication number
IdentityReservations recoverable, unsent party portions
BodyDirectory        optional address -> shareable body ID
```

Use checked arithmetic for counts and IDs. Overflow is a typed refusal before
mutation. The count is not inferred from live messages or recomputed by
walking the set during normal operation.

## 2.4 Construction pays for its own edges

Register an object's owning edges when constructing it, not when publishing
the completed replica. Construction and its count updates form one atomic
ownership transaction.

For a new branch with children `a`, `b`, and `c`:

1. Hold valid read ownership of every child throughout the operation.
2. Outside the ownership gate, compute the complete immutable branch record
   and its summaries.
3. Under the gate, read the required counts, install the branch, register its
   three child edges, initialize its count to one, and create its construction
   hold. All of these changes are in the same atomic batch.
4. Only after success, make the new owned handle available to other work.
5. Release construction holds that the builder no longer needs. Their child
   references now survive through the new parent.

A whole bootstrap repeats this operation incrementally. Every intermediate
graph is owned and recoverable, but no intermediate graph is current.

Batch adjacent constructions up to fixed byte and operation limits. A batch
can combine increments and decrements to the same child; it must preserve
their multiplicity. Provisional handles whose objects are still inside the
batch cannot escape to another task. Flush before yielding such handles.

The gate can cover several prepared records and a bounded number of count
reads, never an entire received region. Its maximum work is a configuration
parameter plus the cost of the largest allowed record. Do not make it depend
on a session's number of messages.

This also solves the “existing children” problem. A new parent sharing an old
subtree increments that subtree's count now, while the source view still
protects it. The final publication owes no deferred increment for that edge.

Durable writes at this frequency have a cost. Begin with bounded coalescing,
measure storage barriers per admitted message, and tune batching. Do not add
a weaker durability mode until its ordering and recovery rules have their
own specification. The initial contract is intentionally sufficient by itself.

## 2.5 Cheap handles without an unbounded decoded graph

Separate three things:

```text
ObjectId       identifies immutable storage
OwnedRoot      keeps a storage graph alive
LoadedNode     temporarily holds decoded bytes while a graph owner protects them
```

Cloning `OwnedRoot` shares an in-process `Arc` around one ownership claim; it
does not increment a persistent count. Traversing a child keeps the source
root owner alive. It need not create a new durable root for every read.
Creating a new parent is different: its child edges are real persistent
ownership and are registered by the construction transaction.

Decoded branches contain raw child IDs. They do not each contain a
`OnceLock<Arc<DecodedChild>>`. Otherwise a root touched by a full scan retains
the entire decoded tree even after the cache evicts its least-recently-used
entries.

Cache values may be `Arc`s. Count the bytes retained by cache entries and by
loaded nodes still held outside the cache. Eviction only releases the cache's
share. An operation's live loaded-node set is bounded by the traversal/window
budget. A cache hit does not substitute for a storage ownership claim.

`read` returns an owned decoded guard, not a reference borrowed indefinitely
from an evictable cache. A user-visible message owns its decoded payload after
the body has been read; merely keeping that `Arc<T>` need not pin storage.

## 2.6 Snapshots are owned by the library

The current in-process replica view contains an `Arc` that names the two
roots and their frontier. Taking a snapshot clones this view under the same
brief synchronization used to publish a replacement. No storage I/O is needed.

When replacing the head, the publication batch transfers the old head's two
root references into a `Retained` record. It transfers the prepared roots'
holds into the new head's ownership. These are ownership transfers, so their
net count changes can be zero; they must still be represented atomically.

The old `Retained` record remains while any in-process view needs it. This
includes explicit snapshots, a session's starting view, a prepared merge's
base, and a reader with I/O in flight. An internal view uses exactly the same
lifetime mechanism as a public snapshot.

Once the final in-process view disappears, its retained root record can be
deleted and the roots decremented. The library needs only a bounded registry
of active views and construction holds, not an entry per reachable node.

The publication transition and view capture must be coordinated: a reader
either acquires the current view before the swap, in which case retention
already protects it, or acquires the new view. It cannot obtain a bare old
pointer in the interval between publication and reclamation.

Registry limits apply to distinct views and operations. Clones of one view
share its registration. If retaining another published generation would
exceed the configured limit, publication can wait or return a resource-limit
result; it cannot invalidate a snapshot already handed to the application.

## 2.7 Drop, cancellation, and root cleanup

Rust `Drop` cannot await. Give storage-owning handles an explicit asynchronous
release, and keep synchronous drop safe without allocating an unbounded
queue of work.

Every private construction hold already has a durable `Holds` row. The
in-process registry records whether that hold is active. Synchronous drop
removes its active registration. The row remains until bounded maintenance
deletes it and decrements its object. Maintenance resumes scans by key and
revisits active rows on later passes.

Temporary files can also contain object IDs awaiting a later pass. Those IDs
need ownership even though no in-memory handle exists. Support a **parked
hold**: a durable root row retained by the operation as a whole, rather than
by an individual in-process handle. One active-operation registration protects
all of its parked holds. Their inventory is on storage, so parking a million
candidates does not create a million registry entries in RAM. Reading one
borrows that operation's protection; explicitly consuming the hold releases
it only after the receiving parent or another owner is registered.

Use parked holds for candidate IDs in sort files and final message IDs waiting
for the address pass. They cost storage proportional to unfinished work and
bounded ownership updates to release. If an operation ends or crashes, its
remaining parked holds become eligible for a resumable cleanup scan. Never
treat a bare ID written to nondurable scratch as an owning edge.

Normal asynchronous builders release their exhausted holds promptly, keeping
the number of active roots bounded by their working frontier. Before accepting
more construction, apply backpressure if root registrations or cleanup debt
reach their configured limits. Do not let a per-session vector of abandoned
handles grow with the set.

Cancellation during a store write is an ownership outcome, not just a dropped
future. If its result is unknown, a synchronous guard marks the instance as
needing recovery before the gate can be reused. The adapter must settle its
outstanding I/O before reopen; recovery then reads which complete batch exists.
Cancellation between settled batches merely abandons durable private roots.

## 2.8 Zero counts drive reclamation

`Zero` is a set of objects whose count is **already zero**. It is not a set of
pending decrements. This distinction prevents losing two decrements to the
same object by storing them under one key.

Removing a root or an object edge decrements its target in the same atomic
batch that removes the edge. If the count becomes zero, add the target to
`Zero`. Zero-count objects cannot acquire new owners: construction must use
an existing valid graph owner, and a directory lookup must acquire ownership
under the gate before returning a shareable object.

For each zero-count object, reclamation:

1. Reads its immutable record to learn its outgoing edges.
2. Under the gate, rechecks its zero-count work entry.
3. Deletes the object, its count, and its work entry; decrements every outgoing
   edge, enqueuing any newly zero targets, all in one bounded batch.

Another collector may consume the work entry during step 1. If the record
has disappeared, recheck under the gate: a missing work entry means the work
is finished, not corruption. A group reservation has no outgoing edges and
can be reclaimed without an ordinary object record.

The count equation remains true even while the cascade is unfinished: a
zero-count parent retains its child edges until the parent is deleted.
Atomic queue consumption makes replay after a crash safe.

The batch limit must accommodate at least one maximum-fanout object's edge
updates. If a representation cannot meet that limit, give its reclamation an
explicit persisted progress record; do not perform half of a decrement set
and hope to infer the rest after a crash.

Offer bounded maintenance after mutations and an explicit `reclaim().await`
that drains currently eligible work. A successful send need not await an
arbitrarily large unrelated cascade. A caller may drive reclamation more
aggressively, and storage-pressure backpressure must eventually require it.
The library remains runtime-independent; no hidden task or timer is required.

Deletion makes space reusable; it need not shrink the engine's file or
securely erase historical media bytes. Those are separate engine operations.

## 2.9 Bodies can be counted too

Count a message object's edge to its body when constructing that message.
Reclamation then releases the body through the same graph
algorithm, without checking membership through every snapshot.

Body sharing is an optimization layered on this correct lifetime model.
For incoming, already-authored messages, a directory maps the version address
to a body object. Acquire a temporary body hold under the ownership gate,
then build a message pointing to it. Competing sessions select the same
directory entry. Any privately written losing copy is ordinary reclaimable
work. This guarantees safe sharing; avoiding every duplicate transient write
is a separate performance goal.

When a directory body's count reaches zero, remove its matching directory
entry in the same ownership transaction. The directory is not itself an
owning edge, or it would keep every body alive forever. IDs are never reused,
so a replacement body cannot be confused with an old queued deletion.

Local sends require special care. Two speculative local batches can compute
the same next version for different values before one wins publication.
Their private bodies must use independent object IDs; they cannot enter a
version-address deduplication directory. A conflict requires replaying the
losing local intent with new versions, not joining these two drafts. This
distinction is specified in section 3.

One immutable message can be referenced by both content and index leaves.
Consequently, a redaction removes both references, while old views retain
their own paths to the message and body. There is no per-body snapshot scan
and no special body-liveness marker sweep.

## 2.10 Recovery and record validation

On exclusive open:

1. Ensure previous writes have settled; validate format and head metadata.
2. Establish ownership of the current roots before serving readers.
3. Reconcile the identity reservation state as specified in section 3.
4. Treat private holds and retained views from the previous process as
   abandoned. Remove them in bounded ownership transactions.
5. Resume zero-count reclamation. It can run incrementally; expose remaining
   recovery work instead of requiring a full live-set scan before service.

Creation installed each object (or group reservation), its edges, its count,
and a root hold atomically. Therefore recovery need not enumerate allocator
chunks looking for uncounted objects. Every abandoned allocated graph has a
durable owner record, or is already on the zero-count cascade. Unused allocated IDs contain
no records and need no cleanup.

Records carry a kind, format version, and checksum over an unambiguous encoding
of key space, key, and contents. Lengths and arithmetic are checked before
allocation. Decode errors identify the key and failure. Never turn a missing
required object into an empty subtree.

Validate path compression, child ordering, bounds ordering, and parent/child
digest agreement at the appropriate read boundary. A checksum and an ordered
span do not prove a stored memo equals the fold of the children. Constructors,
property tests, and an explicit full audit provide that stronger assurance.
The audit is useful maintenance, not a mandatory `open` traversal.

CRC-32C is a reasonable initial fault-detection choice if its accepted residual
risk is documented; no checksum detects every corruption. Do not base the
choice on an unmeasured universal throughput number or claim Merkle hashes
protect payload bytes.
