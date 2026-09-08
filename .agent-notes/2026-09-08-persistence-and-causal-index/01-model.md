# 1. Purpose, model, and guarantees

[Contents](README.md) · [Next: storage and ownership](02-storage.md)

## 1.1 The library today and the proposed revision

Rumors replicates a set of messages. Each peer holds the whole live set,
changes it locally, and reconciles with other peers through gossip sessions.
Reconciliation combines new messages and honors redactions, so communicating
peers eventually agree. The peers are authenticated and authorized by the
application and are assumed to follow the protocol.

The API separates identity from everyday use. `Peer` is the unique handle
for joining or leaving a universe. It can be exchanged for freely cloneable
`Rumors` handles that send, redact, observe, and gossip concurrently. A
`Snapshot` is a fixed view of the set. Today the set lives in memory;
`Bookmark` stores identity checkpoints but no message bodies. Bookmark
recovery rejoins the network to recover content and reclaim identity.

A persistent Rumors replica should keep a live set larger than RAM, restart
with its identity and content intact, and iterate messages in causal order
without first collecting the whole set in memory. It should still reconcile
with other replicas while an unrelated session prepares a large update.

The existing Merkle trie remains the reconciliation structure. A **trie**
is a tree whose branches follow successive parts of a key; here, each step
follows one byte. A **Merkle** trie also summarizes each subtree with a hash,
called its digest, so peers can skip regions where they agree. Its keys are
hashes of message versions; that makes them useful for comparing sets but
unsuitable for causal iteration.

Add a second trie whose keys represent the order in which this replica
admitted messages. Store both tries as immutable objects. A new version of a
trie reuses unchanged objects and allocates replacements along changed paths.
The library preserves old objects while any snapshot or unfinished operation
needs them, using reference counts.

The **root** is the top node from which the rest of a tree is reachable.
Replacing its reference can select an entirely new tree while readers retain
the old one. **Publication** is the atomic change that selects the new roots
and associated peer state. Work prepared before publication remains private.

The engine remains a byte store. Rumors owns the object graph, consistency of
its two indexes, snapshots, identity recovery, and reclamation. No engine
snapshot, multi-statement transaction, compare-and-set, or merge operator is
required.

## 1.2 Four distinctions to establish first

### A message's version is its identity

An interval tree clock assigns a version to each send. It records which
earlier events the sender knows, rather than a wall-clock time. Write `a < b`
when `b` includes `a` and contains additional history. These versions have a
partial order: two independent sends can be incomparable. Causal iteration
respects this order; it does not need to agree on the order of independent
messages.

The content trie uses `H(version)`, the hash of the version, as a message's
address. Payload bytes do not enter that address or the Merkle digest. Two
different payloads must therefore never be published under the same version.

A storage checksum has a different job from a Merkle digest: it detects
damaged record bytes, including payload bytes and stored summaries.

### A live set is more than its remaining messages

Represent the replica's logical state as `(network, frontier, live messages)`.
The **frontier** is the version describing all history the replica has
incorporated, including redactions. It survives even when no messages remain.

Suppose Alice once held `m`, then redacted it. Her empty content tree alone
cannot distinguish that state from never having heard of `m`. Her frontier
does: when Bob offers `m`, Alice already knows its version and lacks the
message, so reconciliation honors its deletion.

Persisting a newer frontier with an incomplete older tree can consequently
turn missing data into apparent redactions. Publishing a frontier, content
root, and index root is one atomic operation.

### Peer identity grants authority to create versions

A `Party` owns part of the identity space: the clock's shared space of
authority to create events. A **universe** is the set of peers descended
from one seed. Its live parties must be disjoint. Forking gives part of that
authority to another participant; retiring transfers it away.

A saved tree is harmless to copy as a read-only value. A saved party is not
harmless to reactivate: another process may already be using it or one of
its descendants. Durable identity transitions therefore need a separate
argument from content atomicity.

Exactly one seed creates a universe. Opening an existing store never calls
`seed`, and a retired store is distinguishable from an unused store.

### Storage reachability is not message liveness

A redacted message is absent from the current set. Its bytes may still be
needed by an older snapshot, a session's starting view, or a prepared result.

**Reachable** means an object can be followed from an owned storage root.
**Live** means a message belongs to the current committed set. Reclamation
uses reachability, not a search through current message membership.

No per-message redaction record is added to the replicated state. Temporary
reclamation records describe unfinished storage work and disappear when that
work completes. They are not information peers need to honor a deletion.

## 1.3 The guarantees

### Atomic, durable local state

After a successful persistent mutation, the published state is durable.
After a crash during publication, opening yields either the previous complete
state or the new complete state. An acknowledged durable state must not be
rolled back by the storage implementation.

Publication includes the content root, causal index root, frontier, applicable
identity transition, and local publication number. No reader observes only
some of them.

An interrupted call can have committed without returning its result. Atomic
storage does not remove that uncertainty. In particular, retrying a send
after an unknown outcome can produce two messages; application-level request
identifiers are needed if the application wants to recognize that situation.

### Concurrent preparation, bounded publication work

Tree walks, sorting, index preparation, and retries run without a shared
write lock. One mutex, the **ownership gate**, serializes bounded batches of
object and reference-count writes and the final root replacement. It never
covers a whole walk or import. A separate brief **view lock** lets a reader
capture the current in-process roots while a publisher replaces them.

This is a bound on work, not a deadline: one storage write may itself be slow.
The engine must eventually complete an accepted operation for the system to
make progress. A large update still consumes disk bandwidth and can contend
with other work, but it cannot hold the gate for its entire preparation.

### Causal iteration

If both `a` and `b` are yielded, and `a < b`, yield `a` first. Concurrent
messages may appear in different orders at different replicas. Redacted
messages need not be yielded.

Default snapshot iteration, causal ranges, and the causal observer use this
order. Keep an explicitly named unordered traversal for compatibility and
specialized uses.

### Local cursor recovery

A cursor identifies a position in one replica's admission order. When an
application resumes from a position it has saved, messages at or before that
position are not yielded again; redacted entries can be skipped. Reopening
the same, current store preserves this meaning.

That is not exactly-once application processing. If an application performs
a side effect and crashes before saving the resulting cursor, it can repeat
the side effect. Saving the cursor first can skip the side effect. Rumors
cannot atomically coordinate arbitrary application storage through this API.

Return the position associated with each delivery, so an application can
save it with its own transaction when that is possible. Keep the existing
portable `Version` checkpoint as a separate, conservative resume mechanism.
It can be used on another replica in the same universe and can replay.

### Memory and storage limits

Persistent operations must not accumulate a collection proportional to the
set or delta in RAM. Traversal stacks, sort buffers, write buffers, cache
residency, and active work are bounded explicitly.

Some memory costs depend on individual values: a payload, a version,
and the decoded application value. Versions can grow with identity
fragmentation. A user-defined deserializer can allocate much more than its
input size. A fixed tree depth does not bound those allocations.

State memory as configured working space plus these value-sized costs; provide
typed resource-limit errors when configured byte ceilings would be exceeded.
The process's total resident memory also includes application values of type
`T` and the storage engine's own allocations. The implementation plan
distinguishes these costs and specifies what to measure.

Likewise, old snapshots and unfinished sessions can retain substantial disk
space. Bound their number and scratch usage, expose retained-work statistics,
and document that retaining old snapshots delays reclamation.

## 1.4 What is preserved, and what is allowed to cost more

Preserve the content trie's canonical shape: equal sets produce the same
tree, regardless of insertion order. Its 32-byte addresses bound it to 32
levels with at most 256 children per branch. This number of children is the
branch's **fanout**. Single-child paths are compressed into one stored prefix.
Peers compare corresponding address prefixes, using
the existing digest and frontier rules and the existing stream schedule.

The causal index adds ordering metadata to supplied messages, explained in
chapter 3. That changes byte accounting but adds no network ordering pass.

The causal index necessarily adds storage and mutation work. Removing many
messages may require visiting their index entries and eventually reclaiming
their bytes even when the wire can communicate that deletion compactly.
Do not confuse divergence-proportional communication with constant local
deletion work.

Preserve the synchronous memory implementation and its payload cache unless
measurements justify a specific change. Its extra causal-index cost must be
measured separately from any abstraction overhead.

The difficult limits remain explicit: storage faults can stop progress,
perpetual publication conflicts can starve optimistic work, and a lost
identity handoff can strand identity space. None should be disguised by a
whole-update lock, a silently weakened storage contract, or a recovery guess.
