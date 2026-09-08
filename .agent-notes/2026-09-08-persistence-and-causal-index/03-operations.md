# 3. Index, commits, and identity

[Contents](README.md) · [Next: implementation](04-implementation.md)

## 3.1 Give each admitted message a stable local position

Give the index a two-part key:

```text
(publication number, position within an admission group)
```

Both fields are checked `u64`s, encoded in big-endian order when used as
index paths. The resulting trie has at most 16 byte levels. Publication
numbers strictly increase with every head change, including an identity-only
change. This number is also the head's **revision**, used to detect a changed
base during publication. Positions inside a group are unique. Neither
counter wraps. A redaction removes an entry but never makes its position
available for reuse.

An **admission group** contains the candidates prepared by one operation.
Assign their within-group positions in causal order before attempting to
publish. Candidates later rejected by reconciliation leave holes. Dense
numbering is useful for initial construction, but it is not an invariant
worth rebuilding surviving records to preserve.

The publication number is assigned only when the operation wins publication.
Two groups prepared concurrently therefore do not have a predetermined order.
This preserves the freedom to prepare and finish them independently.

A message record stores `(group ID, within-group position)`. A small,
immutable group object supplies the publication number after commitment.
Looking up the group is normally cacheable, but price it as a possible read.
The group object is retained through actual references from message objects,
including messages kept by older snapshots. It is not deleted merely because
its last *currently live* message was redacted.

### Binding a group once

Reserve a group ID before building its message records. The reservation is
durable construction metadata with a count and an owning hold; it is the one
permitted unresolved dependency in a private graph. It contains no message
list and has no outgoing object edges.

At successful publication, atomically install the immutable group object
`{ publication_number }` and remove its reservation. No existing object
record is overwritten. If no messages from that group survive, publish no
binding; releasing the private graph eventually releases the reservation.

Type the boundary: a prepared root can contain a reserved group; a published
root cannot. Preparation establishes that the only unresolved group it
reaches is its own. Current subtrees already satisfy the published invariant.
Publication resolves that single group in constant record-count work.

This small reservation is necessary because the publication order cannot be
known while immutable message records are being prepared. It is not a
transaction over the group's messages.

## 3.2 Where the within-group order comes from

For local sends, use the order in which the batch issues sends and ticks
their versions. Redactions do not create index entries.

For gossip, each supplied message carries the sender's existing local key
as a 16-byte origin tag. A session has one remote sender, and every outbound
tag must refer to that sender's fixed, committed session view. The receiver
sorts the tags in temporary storage and assigns its own within-group
positions. It does not retain the previous hop's key as part of its own key.

Example:

```text
sender's keys:       (8, 2)  (8, 9)  (11, 0)
receiver's group:        0       1        2
receiver publishes: (27, 0) (27, 1) (27, 2)
```

The sender's order puts every included cause before its effect. Removing
items from an ordered sequence preserves that property among the survivors.
The receiver can therefore filter candidates without sorting them again.

The wire still sends leaves in address order, which supports efficient
content-trie construction. The extra tag is metadata on those existing
supplies. It does not add a causal-order wire phase or additional round trips.

Audit all supply paths, including opening supplies and terminal leaves, to
ensure their origin is the starting committed view. Private received nodes
with unresolved group bindings are never an outbound source. The existing
session contract that supplies belong to the sender's declared set provides
the starting point for this audit.

Validate tag uniqueness across a session, not merely inside one run. A sort
implementation must detect duplicate keys rather than silently replace one
message with another. As with the existing conformance checks, this detects
bugs in authorized peers; it does not defend against dishonest senders.

## 3.3 Keep final records immutable; spend temporary I/O explicitly

The received payload and version are available before their local position.
Do not put a placeholder position into an ordinary content leaf and expose
that leaf to caches or other builders.

Use two representations:

- A private received candidate refers to a validated version and owned body.
  The session's temporary reconstruction uses these candidates alongside
  handles into its starting view.
- A final immutable message record has the version, body, group ID, and
  within-group position. Content and index leaves refer to this record.

After the wire reconstruction finishes:

1. Scan candidates sorted by origin tag, assigning unique positions. Store
   `(address, final message ID)` in a second temporary ordered stream.
2. Scan that stream in address order to build final received content subtrees
   bottom-up and substitute them into the session result. Store the list of
   reconstructed trie prefixes and the temporary nodes incrementally too,
   rather than accumulating them in RAM.
3. Release temporary candidate/reconstruction roots as they become redundant.

Every object ID in either temporary stream remains protected by its operation's
parked holds or by a known retained graph until that stream has been consumed.
The ownership layer distinguishes these from dropped transient handles.
Sorting object IDs does not itself keep their objects alive.

This is an additional metadata pass and some additional node construction.
It does not re-decode or recopy every body. Unchanged subtrees from the
starting view remain shared.

The implementation can alternatively represent the reconstruction as an
immutable edit description and materialize its final nodes once. That is an
optimization only if it stays simpler than building the temporary tree. Start
with the auditable two-representation path and measure its cost.

An **external sort** orders data larger than memory: sort one bounded buffer
at a time into temporary files, called runs, then merge those sorted runs.
Limit the total buffers, merge readers, output buffers, and open files across
both sorts. If there are too many runs to merge at once, use repeated passes;
two passes are not a general bound on arbitrarily large input. Keep the run
inventory on disk too. Enforce temporary-byte and concurrent-session limits
before accepting more work.

## 3.4 Prepare a complete result against one base

Let `B` be the current replica view captured by the operation. Let `S` be a
completed session result with final immutable messages and its frontier.

Outside the ownership gate:

1. Compute `P = join(B, S)` using the same deletion rules as `Tree::join`.
2. Preserve `B`'s message records wherever both inputs contain the same
   version. Equal hashes compare content; they do not make different local
   group IDs interchangeable. This preference preserves existing index keys.
3. A received candidate is new exactly when it exists in `P` and is absent
   from `B`. This is a membership test, not a test of pointer identity.
4. In candidate order, build an index subtree for the admitted messages
   bottom-up from the sorted entries. This **bulk build** writes each node
   once instead of rewriting a path for every insertion.
   Its within-group positions are already fixed. Place it under the prospective
   next publication number in a prepared index root.
5. Remove the index entry for every message removed from `B` by the join.
   Keep the removed subtree owned until its entries have been enumerated.
6. Prepare the resulting frontier and root summaries, and acquire explicit
   owning holds for the two result roots.

Every object and edge produced along the way is registered by the bounded
construction transactions from section 2. There is no accumulated final batch
of reference counts, body increments, or index deletions.

A whole-subtree content deletion can be decided without visiting its leaves.
Updating the other index generally cannot: those messages occupy unrelated
local-order positions. Enumerate the affected messages incrementally and
charge that work to the deletion. If useful, externally sort removals by index
key to reduce path rewriting. Do not hide an `O(number removed)` operation
inside a supposedly constant-cost prune callback.

## 3.5 Publish with a constant number of ownership changes

The finished operation has a base revision, two owned prepared roots, a
prepared frontier, and at most one unbound admission group.

Under the gate:

1. Compare the current revision with the preparation's base. A revision
   includes identity changes as well as content changes.
2. If they differ, release the gate and reprepare. Do not write the head.
3. Otherwise assign the next publication number and bind the admission group
   if any of its messages survived.
4. Atomically replace `Head`, transfer its old root ownership into a retained
   view record, and transfer the prepared root holds into the new head.
   Include the applicable identity-state change in this same batch.
5. Publish the new in-process view under the brief view lock. A guard handles
   cancellation or an unknown write result by marking recovery necessary;
   it never allows memory to continue as though a possibly committed write
   did not occur.

There are a constant number of root/hold/group records here, independent of
the delta. The frontier and party encodings are variable-sized values;
“constant number of records” does not mean a fixed byte count for all clocks.

After releasing the gate, release private roots, reclaim bounded eligible
work, and dispose of temporary storage. A cleanup failure after a successful
publication must not be reported as though the logical commit were rolled
back. Keep commit outcome and maintenance status separate.

Hold the old roots and all potentially discarded user values across the
in-process swap. Drop them outside the view lock. Keeping the old root alone
does not prove every temporary value or failed candidate is protected; retain
the existing destructor/panic tests and audit all exits.

## 3.6 Concurrent sessions and retries

Two sessions can read the same base, receive different deltas, construct
objects, and prepare indexes concurrently. Their ownership transactions
interleave in bounded batches. Only their final root replacements conflict.

On a conflict, a gossip operation retains its completed session result `S`
and joins it against the new current view. This correctly incorporates newer
redactions and preserves already admitted messages' positions.

For the initial implementation, rerun admission as a bounded-memory scan of
the candidate stream. Reuse unchanged content subtrees and index subtrees
where the algorithm establishes equality. Keep within-group positions fixed;
only holes and the prospective publication-number prefix can change.

An optimized implementation can use a structural difference walk between
the old and new bases to revisit fewer candidates. That optimization needs
a work bound and tests. A tree-identity check inside a loop over every
candidate still visits every candidate. Do not claim “overlap-only retry”
until the algorithm actually skips unaffected candidate regions.

The causal cases remain straightforward:

| Concurrent changes | Required result |
| --- | --- |
| Both sessions supply `m` | First publication assigns its position; the other retains that record and adds no index entry |
| One supplies a cause, another the cause and effect | The cause is earlier within one group, or belongs to an earlier publication |
| One supplies `m`, another honors its redaction | The final join removes `m`; its index entry disappears with it |
| Both remove the same messages | Only entries still present in the chosen base are removed |
| An old snapshot remains open | Its roots and group bindings remain reachable regardless of the current set |

Never respond to repeated conflicts by holding the ownership gate through
repreparation. That would violate the concurrency requirement. Use a bounded
retry-work policy, returning a distinct contention result when exhausted;
retain or release the private work according to the explicit retry API.
The peer remains usable. This promises progress when publication eventually
offers a successful attempt, not starvation freedom under perpetual change.

A fair queue for the short gate prevents one task monopolizing ownership
batches. It does not, by itself, guarantee that an optimistic preparation
will ever find its base current. If workload measurements show unacceptable
large-session starvation, improving rebasing is a required design follow-up;
a long exclusive fallback is not an acceptable hidden solution.

## 3.7 Local sends are not remote merge results

Incoming messages already have globally established versions. Local intents
do not. A local batch contains “send this value” and “redact this version,”
not a finished independent replica to merge after a conflict.

Admit and serialize values before preparing; keep the resulting intents
owned or in bounded temporary storage. Against a captured base, assign draft
versions using its current party and frontier, then build the content and
index results. Draft versions are private, are not returned to the caller,
do not enter shared version-address body deduplication, and never cross the
wire.

The captured identity description is inert draft input, not another usable
peer identity. Keep any decoding or read-only aliasing needed for the clock
calculation inside a private draft type that cannot fork, donate, or transmit
authority. A draft tree cannot enter the ordinary peer-to-peer merge surface.
Only the successful revision check turns its versions into emitted events.

If publication loses its base, discard the version-stamped draft and replay
the intents against the new base. Do **not** rerun the user's closure. In
particular, do not merge a losing local draft into the winning state: two
draft sends from the same base can have equal versions and different payloads.

The successful attempt assigns versions above the frontier current at its
publication. Identity changes also invalidate an attempt, so a send cannot
publish using a party portion that was donated while it prepared.

The memory mode can continue assigning versions synchronously in its existing
commit critical section. It does not need to adopt persistent draft/retry
machinery merely to share an API facade.

A synchronous callback cannot spill an unbounded batch through async I/O.
Keep a bounded callback-based persistent batch for convenience and provide an
async staged-batch builder for larger atomic batches. Enforce the bound while
queuing, before memory has already been consumed.

## 3.8 Why the index order is causal

The proof needs more than “a cause's version is below the frontier.” Any
frontier containing an effect also contains its causal past, even if an
incorrect implementation lost the associated bodies. Start from the actual
state invariant:

> When a message becomes live, every earlier emitted message in its causal
> past has already been incorporated by that replica, or is incorporated in
> the same atomic result. An incorporated message that is absent has been
> redacted and cannot later become live again under that version.

Establish this inductively from local sends, complete reconciliation,
frontier-based deletion, and atomic recovery. In the test model, retain an
independent history of emitted messages to state it; production needs no history
log or tombstones.

Now consider two live messages `a < b`:

1. If both were newly admitted in one publication, sender tag order or local
   send order puts `a` before `b` within the group.
2. If `a` was already present when `b` was admitted, its publication number
   is lower and its position is preserved.
3. `a` cannot first become live in a later publication than `b`: the state
   invariant says it was already incorporated, and if absent as a redaction,
   it cannot reappear under that version.

Thus the index extends causal order. The argument also explains why partial
wire results cannot be published just to reduce a large commit's work.

## 3.9 Observers and snapshots

A snapshot keeps one complete pair of roots owned and traverses its index lazily.
Its range iterator uses the index's version-span summaries to prune; its
point lookup uses the content tree. All cold reads are fallible and async.
Cached root summaries support synchronous `len`, `latest`, and hash access.

The live causal observer advances a local cursor through successive views.
Each bounded page or in-flight read owns its view. It releases that view
before waiting for a later change. A suspended observer therefore need not
pin an entire old pass indefinitely.

For a pass, capture a frontier and an upper publication boundary from the
same view. Visit entries after the cursor and at or below that boundary.
Moving to a newer view may skip newly redacted entries, which cannot return;
entries newly admitted by that newer view are above the fixed boundary.
The pass completes only when that interval is exhausted.

Advance the portable checkpoint to the captured frontier only after the
pass's messages have been delivered under the existing checkpoint convention.
Do not substitute the newest head's frontier after a racing end-of-scan:
that could cover undelivered messages in a new publication. Do not construct
the portable checkpoint by simply folding yielded versions.

A local `Position` contains store identity and a key, with explicit start
and end-of-publication boundaries where needed. Prefer opaque constructors
and validated serialization. Validate foreign stores, future publications,
overflow, and the maximum issued position at the current boundary. Preserve
enough high-water metadata to validate a legitimately issued cursor even
after its message is redacted. An older genuine cursor need not name a live
entry.

Return a delivery's after-position with that delivery. Document saving it
after successful handling. A `Position` is not an acknowledgment stored by
Rumors, and a `Version` checkpoint is not a substitute for an exact local key.

## 3.10 Identity transitions need their own durable ordering

Persist `Network`, store identity, codec-relevant configuration, and lifecycle
state with the head. An empty store, an active peer with an empty message set,
and a retired peer are different states. A corruption error is none of them.

Local sends are durable before they become available to snapshots or gossip.
Ordinary outgoing gossip uses a committed view. This prevents restart below
versions this incarnation has already exposed through the library.

Forks and retirements require more: the donor must durably give up authority
before the recipient can receive it. Use a small durable reservation state
machine, coordinated by the ownership gate.

### Donating a fork for bootstrap

1. Atomically capture the session's starting view, split the active party,
   and record the donated portion as a **reserved, unsent** identity. Persist
   the retained active party and reservation together before using the split.
   The reservation carries the frontier at the split. Capturing these together
   prevents intervening local sends from outrunning the donated clock.
2. Reconcile using that starting view. Normal sends use only the retained
   party. Other sessions can continue; their prepared attempts observe the
   changed revision.
3. Immediately before attempting to write any identity bytes, atomically
   remove the reservation from recoverable custody. The active party remains
   narrowed. Wait for that write to be durably known successful.
4. Send the party. From this point onward, no error, cancellation, or missing
   acknowledgment can return it to the donor.

If a normal failure occurs before step 3, an async recovery transaction can
join the reservation back into the active party. If the process crashes
while the reservation is still durably unsent, open can do the same. A
cancelled operation leaves its unsent reservation for the next recovery
point; synchronous drop cannot perform a durable join.

If step 3 succeeded but no recipient acquired the party, that identity space
is stranded. Preserving it through an ambiguous handoff would require a
separate transfer protocol. Accept this existing class of loss rather than
reactivate possibly duplicated authority.

### Retiring

Apply the same sequence to the whole party. A recoverable, unsent retirement
reservation permits recovery. Once authority has been relinquished, persist
a retired lifecycle state that `open` cannot turn back into an active peer.
Keep the existing recovered/declined/uncertain distinctions in the public
lifecycle result, extended with storage outcome where necessary.

### Receiving authority

The receiver first completes reconciliation and receives the donated party.
It prepares the merge against its current state and validates disjointness.
Publish the resulting content, index, frontier, and expanded party atomically
before sending the session's completion certificate.

If the recipient crashes before publication, the donated space may be lost.
If it crashes afterward, it reopens with both the authority and the state
that makes using it safe. The donor never recovers authority solely because
the completion certificate was lost.

Only the active identity manager can operate on party bytes. Old snapshots
and retained root descriptors do not expose reactivatable parties. Storage
copies, backups, and an earlier process incarnation must never operate as
independent live peers under the same stored identity.

## 3.11 Wire and compatibility

Recommend `Protocol::V3` for origin tags, with no compatibility shim for this
unreleased library. This is a proposed owner decision. Before implementing
it, record the approved vocabulary and deliberately reaccept the wire pins
with a commit that names that change.

Tags add bytes to every supplied leaf, including one-message and opening
runs. Update run sizing, allocation accounting, maximum fan occupancy, codec
fixtures, wire observation/rendering, and the streaming conformance tests.
Recheck the deadlock argument with slow storage and bounded ownership batches;
never hold the ownership gate while waiting on a network peer or a channel
whose consumer needs that gate.

Keep `Bookmark` for the memory mode. A persistent peer's head and identity
reservation records take responsibility for durable custody; attaching an
independent bookmark to that peer would create two authorities of record.
Opening a persistent store restores it directly, rather than rebootstrapping
as bookmark recovery does.
