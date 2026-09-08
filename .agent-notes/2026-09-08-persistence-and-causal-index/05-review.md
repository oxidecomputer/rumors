# Appendix A. Review of the earlier proposal

[Contents](README.md)

The earlier proposal, *Persistent storage and a causal index for rumors*,
records a design conversation from September 6–7, 2026, with Finch as author
of record. It has the right architectural direction: keep the content trie,
prepare immutable results concurrently, publish roots atomically, and own
snapshot retention in the library. Its difficulties come mainly from
assigning some responsibilities to the wrong phase and then claiming bounds
that those assignments do not support.

This appendix restates the relevant choices before evaluating them; the
earlier document is not required reading. “Source” section numbers identify
that document for readers comparing the two. These are design findings, not
reports of bugs already present in the library.

## A.1 What this plan keeps

- The address-based Merkle trie and its existing reconciliation rules.
- A separate local causal index, with fixed-width keys and sender tags.
- Sequential object IDs, immutable published records, and inline child hashes.
- Preparation outside the ownership gate, using structural sharing.
- Library-managed snapshots, reference counts, and crash-resumable cleanup.
- A byte-storage seam with atomic batches, without engine snapshot requirements.
- Property tests, an independent in-memory merge oracle, crash injection,
  memory accounting, and explicit wire-format review.

Sequential allocation is a sensible initial locality choice. The claim does
not need to be that content addressing is always slower. Similarly, avoiding
rank computation is reasonable without claiming no other finite-width
ordering scheme can exist under an explicit overflow policy.

## A.2 Reference accounting belongs in construction, not publication

Source §§5.2, 5.5, and 8.1 say the last batch sets counts for every new node,
and publication reads/increments counts for existing children of rewritten
paths. The same sections claim a fixed-size batch and work independent of
the update.

Take an import creating a million nodes. Setting their counts in the final
batch already requires a million updates. Or change many scattered leaves:
the number of shared children referenced by the rewritten branches grows
with the changed region. Bounded fanout limits the work **per branch**, not
the number of branches in the commit.

The replacement counts every allocated object, including private ones.
Construction registers edges incrementally in bounded atomic batches.
Publication transfers a small number of root ownership claims. This keeps
the intended immutable architecture and removes the contradiction rather
than moving large transactions into the engine.

## A.3 An owned graph needs roots beyond public snapshots

Source §§4.5 and 8.4 say only snapshots pin storage, while sessions and
preparations retain root handles. A raw pointer does not protect storage
from a collector just because a future might use it later.

The essential test is simple: a session captures root `R`, another operation
publishes `R2` and reclaims the old head, and the session then reads a child
of `R`. Its lifetime must protect that child.

Treat session views, prepared roots, construction roots, and user snapshots
as explicit graph owners. Clones can share one in-process owner, as `Arc`
does. The persistent count records ownership of graph edges and roots, not
each cheap clone made by a traversal.

## A.4 The proposed cache can keep the whole tree resident

Source §4.1 puts a `OnceLock<Arc<Decoded>>` in persistent handles; decoded
branches contain child handles. After reading every child, the root can reach
every decoded descendant through those strong references. Evicting entries
from a separate least-recently-used cache does not free that graph.

An evictable cache also cannot generally return an ordinary borrowed
`&Decoded` for the handle's whole lifetime without some pinning arrangement.

Use raw child IDs in decoded records and explicit, bounded loaded-node
guards. Keep storage ownership separate from cache ownership. Charge objects
held outside the cache as well as cache entries.

## A.5 Body counts are not inherently a large-commit bottleneck

Source §8.2 rejects body counts because they would require one count update
per message under the publication mutex. That conclusion follows from its
choice to defer ownership accounting until publication.

With construction-time accounting, a message's body edge is registered when
the message object is built. A snapshot then keeps the body alive through
ordinary graph reachability. Freeing a body requires no point lookup in every
current or historical tree and no mutable registry scan over an unbounded
number of snapshots.

The source's membership-based alternative also needs a race protocol: a
“not held anywhere” result must remain valid until the deletion commits,
while other sessions can stage or publish the same body. The counted-edge
construction transaction states that protocol directly.

Body sharing is still worthwhile, but distinguish bodies of already-authored
incoming messages from speculative local sends. Private drafts can temporarily
use the same next version for different payloads. They cannot safely share a
global version-address directory.

## A.6 The staged-body cleanup order leaks bodies

Source §8.3 tests body liveness while the session's own `StagedBodies` marker
still exists. That test must retain the body. The subsequent marker deletion
does not include another liveness test.

For a session that aborts after receiving one otherwise-unheld message:

```text
delete its private leaf -> its marker says retain body
delete its marker       -> nothing schedules the remaining body for deletion
```

The crash-open sequence has the same issue. Additional bookkeeping can fix
it, but counted bodies remove the separate membership/marker mechanism.
Releasing the last candidate or message edge naturally schedules the body.

The allocator-chunk scan also checks every allocated record in those chunks
for a count, even on a successful session. It cannot have total cost only
proportional to rejected messages. Durable construction roots avoid this
classification scan entirely.

## A.7 A set of pending decrements loses multiplicity

Source §8 uses `Free[ptr]` as a queue while describing decrements of queued
pointers. If two different freed parents each owe a decrement to the same
child, putting that child into a set twice records only one obligation.

There are two sound representations: a counted queue of decrement
obligations, or immediate atomic decrements with a set containing only
objects already at zero. The replacement chooses the latter. It is smaller
to explain and gives each zero object one unambiguous deletion transition.

Also define counts during incomplete cleanup. Saying they are correct only
after the queue drains leaves the most important crash states unspecified.
Count outgoing edges of objects waiting for reclamation until the batch
that actually deletes those edges.

## A.8 A publication binding must outlive old readers

Source §6.5 deletes `Epochs[commit_id]` when the group's currently live entry
count reaches zero. This table maps a group to its publication number (the
source's “epoch”). But an older snapshot or an in-flight session can still
hold a leaf from that group and need the binding to produce its tag.

Reference the small binding object from immutable message records. It then
lives as long as any current, historical, or private graph needs it, and
disappears without retaining a list of redacted messages.

Assign within-group positions once and permit holes. The only value that
must wait for publication is the group's publication number. Keeping that
uncertainty in one small reservation is clearer than spreading mutable
placement through ordinary content records.

## A.9 The retry argument overlooks frontiers and candidate scans

Source §7.4 says a candidate's answer changes only below nodes replaced by
the intervening commit. Consider a frontier-only advance: the content tree
stays empty, but the newer frontier establishes that an incoming candidate
has been redacted. Every content pointer can be unchanged while the
candidate's admission answer changes.

Rebasing must account for both content changes and frontier changes. A
content-identity early stop alone is unsound.

The source also proposes checking each admitted candidate while claiming
work depends only on overlap. Even a cheap check for each of a million
candidates visits a million candidates. Structural sharing saves many tree
reads; it does not automatically skip the outer loop.

Start with a correct streaming rebase and honest cost. Add selective rebasing
only with an algorithm that can skip candidate regions, including the effects
of frontier-only changes. Do not use a whole-reprepare mutex as the fallback
for repeated conflicts. That negates the concurrency the immutable design
is intended to preserve.

## A.10 Durable identity changes cannot wait for session completion

Source §5.1 identifies the session's final root write as the place party
changes become durable, but a donation has already crossed the wire before
that final write in the existing lifecycle.

If a donor transmits a party and crashes while its durable head still owns
that party, reopening can duplicate the recipient's authority. A perfect
content transaction does not prevent that failure.

The current [gossip driver](../../src/peer/gossip.rs) already contains the
important ordering: snapshot/fork together, and remove donated authority
from the bookmark before sending it. Persistence needs an equivalent durable
reservation/relinquishment protocol, with the recipient's content and
authority published together before completion.

The head also needs the network identifier, store identifier, codec-relevant
configuration, and explicit lifecycle state. `party: bytes` alone cannot
express an unused store, recoverable unsent retirement, and completed or
uncertain retirement safely.

## A.11 Causal closure and exact resume need precise statements

Source L-1 says that for an absent cause its version is contained in the
frontier. Containment alone does not prove the replica actually incorporated
the cause: it also holds after an erroneous partial import advances the
frontier and loses the cause's body.

State the invariant over actual emitted messages and their incorporation,
and derive it from complete state transitions. The index proof then follows
from that invariant and the sender's within-group order. Do not make the
containment test itself carry the proof of completeness.

A saved local key gives exact continuation from that key. It does not
atomically couple delivery to an application's side effects or cursor store.
The supplied “durable position without replay” goal needs this qualification.

Finally, an observer that performs async reads cannot safely walk a root
without retaining it somehow. It can retain a bounded page's view and release
it while quiet. A portable checkpoint needs a captured frontier/boundary
pair; borrowing whichever frontier is newest when a scan finishes can skip
an intervening commit.

## A.12 Several performance claims should become experiments

The source contains reasonable hypotheses presented as universal facts:

- A large delta, batch action list, or root registry is not bounded merely
  because the final write buffer has a cap. Every collection needs a bound
  or a spill path; arbitrary `T` decoding needs a separate qualification.
- “Two passes” of external merging is insufficient once run count exceeds
  the square of fanout. All passes and run-directory memory must be covered.
- Comparison sorting takes `O(k log k)` comparisons; fixed-width integer
  keys also admit radix-based sorting. The comparison lower bound is not a
  lower bound on every possible implementation of these sort keys.
- A cache of encoded top-level records is not the same size as their decoded
  representation, version summaries, and externally held guards. Nor is
  arrival-order proximity a guarantee that every causal range prunes well.
- Per-node boxing and lazy decoding are not free on the memory path. The
  existing decoded payload plus exact serialized bytes is a deliberate cache
  in [Message](../../src/message.rs), not automatically duplicate waste to
  remove during backend refactoring.
- A plan cannot infer that a destructor-safety change is unnecessary from
  holding just the old root. Temporary and losing inputs can contain user
  values too. Retain tests and inspect the actual implementation.

The implementation packages turn these hypotheses into measurements and
acceptance tests. Section 4.11 discusses the unresolved concurrency risk.
