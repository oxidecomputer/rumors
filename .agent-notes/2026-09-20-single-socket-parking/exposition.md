# Reconciliation over one socket

## 1. The claim

`rumors` reconciles two replicas of a message set by walking a hash tree
from the root downward and exchanging what differs. The shipped protocol
streams the walk, allowing several levels to be in flight together. It
requires seventeen independently flow-controlled streams per direction so
that one level can keep flowing while another is blocked. QUIC supplies
that separation; a direct TCP implementation of this contract uses a
connection per stream, unless it adds its own multiplexed flow control.

This design replaces the bundle with one ordered duplex byte stream. It
preserves the logical messages, their order within each level, and the
number of dependent network crossings. It adds no credits, window
advertisements, or sender-side inference about what the receiver can
absorb. The central fact is a count:

> At each level, the replies sent toward a receiver but not yet taken by
> the stage that asked for them have an upper bound determined by that
> receiver's own configuration.

Reserving that many decoded-reply slots removes the wait on a blocked
stage that would otherwise stop the socket reader. The receive-side
changes retain the existing queues, widening two of them, and move two
pieces of bookkeeping: one later, to the moment its stage takes the
reply, and one earlier, to before the reply is written. No new dependency
edge enters the dataflow. Together, these changes free decoding from
waits on the walk.

The count is deterministic. The economical byte sizing is statistical
under uniform hashing, and covers protocol working state, excluding
replica content stored in the backend even before commit. The result also
inherits the existing walk's progress property and assumes that backend
operations, transport, and runnable tasks eventually progress. It removes
a circular wait caused by multiplexing; serialization and loss still
affect elapsed time.

Sections 2–4 build the protocol from V1 to V2 and state that inherited
progress property. Section 5 shows why naive multiplexing deadlocks; §6
derives the count and the receive-side changes; §7 accounts for memory;
and §8 describes costs and the evidence to seek. The
[appendix](appendix-implementation-plan.md) gives the implementation plan.

## 2. The conversation

Every message has an address: the hash of the causal version stamped on it
when sent. These 32-byte addresses determine a 256-ary radix trie, one
address byte per level. A node's children, at most 256, are its *fan*.
Storage compresses single-child runs, so a stored interior node has at
least two children. The conversation still traverses a compressed run one
byte at a time: compression changes storage, not the level count. Leaves
have no children. Each interior node memoizes a *digest* of its subtree,
a function of the version set alone. Subject to the protocol's assumption
that distinct sets do not collide in these hashes, equal digests identify
equal subtrees.

Each side starts with a *greeting*, containing its causal version,
live-message count, and a *listing* of the root's children as pairs of
radix byte and digest. The version summarizes the replica's entire history
of sends and redactions, including history it has absorbed. Equal versions
therefore identify equal histories and equal sets; in that case the
session ends without a descent.

Otherwise, a *question* about a node lists its children. A *reply* reacts
to each listed child and supplies children of its own missing from the
listing:

- **match**: my digest for this child equals yours;
- **supply**: you lack this child; here is its subtree, filtered against
  your version so that messages you have seen and deleted stay deleted;
- **query**: our digests differ; here is my listing of this child's
  children. If I lack the child, I send an empty listing to request it
  whole.

The filter propagates deletions without tombstones; the argument below
does not depend further on its operation. The distinction between a
nonempty interior listing and an empty request has explicit endpoints.
At the leaf level an empty query requests the individual leaf. An empty
tree's greeting listing requests everything. A singleton is a leaf, with
its address path; it does not require an empty listing to masquerade as a
held interior node.

A query is the next question, one level down. Thus a reply both answers
one side and asks the other to continue. In the interior descent, a
question or reply is *at* level ℓ when the children it lists or reacts to
are at address depth ℓ.
Its subject node is at depth ℓ − 1, and its queries ask at level ℓ + 1.
The initiator's greeting is the question at level 1. Interior replies
alternate: the responder sends odd levels and the initiator even levels.
At the end, the initiator's last replies request individual leaves it
lacks, and the responder supplies them in the terminal leaf exchange.

For example, let A ask B about a node at depth 2. A's level-3 listing has
children `10` and `20`. B has the same `10`, lacks `20`, and has an extra
`30`. B's level-3 reply is `Match`, `Query([])`, `Supply(30, subtree)`.
The first two reactions refer positionally to A's listed children; the
supply names its radix. B has now become the asker: its empty query about
`20` is a level-4 question. A's next level-4 reply answers it by supplying
that subtree. If B instead held a different `20`, its query would list
`20`'s children for A to compare at level 4.

The greeting carries the initiator's root listing so that the responder
can answer at once. The shipped protocol adds one more shortcut, which
this design removes: after both greetings the initiator knows which of its
root children the responder lacks, and ships them unasked, one hop early,
as an opening batch. Under uniform hashing a responder holding N messages
leaves a given root slot empty with probability about e^(−N/256), so the
batch is nonempty only between tiny replicas, and it costs a decoder,
a stream, and a pairing rule of its own. Without it, those children are
supplied one hop later, as the answers to the responder's empty queries,
like every other request.

Matching digests prune shared subtrees. The remaining work follows
disputed prefixes and inspects their fans until it reaches the *disjoint
frontier*: on those unresolved branches, the cut below which subtrees are
held by one side only. Under uniform hashing, addresses spread over the
256ʲ prefixes at depth *j*. For replicas differing in `D` of `N` messages,
the estimated expected frontier depth is about `log₂₅₆(2·D·N)`, about five
when `D = N = 10⁶`. This is an expectation, not a limit on the deepest
path.

Two properties will let us count and store the replies.

**Each reply answers exactly one question.** Matches and queries refer
positionally to the question's listing; supplies name their radix. Replies
carry no path. The *k*-th reply at a level answers the *k*-th question
there, so the asker retains a *local question record*: handles to the
children whose digests it listed. This record is distinct from the
transmitted listing. It can hold one fan of handles and remains available
until the reply can be interpreted.

**Only supplies carry subtree content of unbounded size.** A query holds
at most 256 listing entries, although a reply containing a fan of such
queries can itself be large. Every supply answers a question.

The logical reply shape is:

```rust
enum Reaction<Node> {
    Match,                    // next listed child: same digest
    Query(Vec<(u8, Digest)>),  // next listed child: my question about it
    Supply(u8, Node),         // unlisted child, named by radix
}

struct Reply<Node> {
    reactions: Vec<Reaction<Node>>,
}
```

## 3. V1: a level at a time

V1 sends one complete level per message, alternating sides. The responder
builds and sends all level-1 reactions together with their level-2
questions; the initiator reads the whole message, builds all level-2
reactions and level-3 questions, and sends them back. The next level waits
for the whole preceding level.

During this serialized descent after the greeting, exactly one message is
in flight and its receiver is waiting for it. One ordered byte stream per
direction suffices. The cost is aggregation: a message grows with the
number of disputes at its level, can transiently duplicate much of the
set, and cannot depart until all its work is ready.

Each alternating message advances the descent one level in one network
crossing. An exchange of two messages advances two levels. V1 therefore
already has one dependent crossing per level, plus the opening and closing
costs; its barrier affects elapsed time, not that crossing count.

## 4. V2: every level at once

V2 preserves the exchange rules and logical reactions, but sends one reply
per question instead of one message per level. Subject to local queue
capacity, a reply can proceed as soon as its own input is ready, while
other replies at the preceding level are still being computed or sent.
Transmission and computation overlap across levels. The dependent-crossing
count stays the same; the gains are removal of the whole-level barrier
and a bound on the work retained in flight.

### 4.1 Stages

Each side runs a *stage* at each level where it asks questions. Stage ℓ
dequeues one local question record and then takes its matching level-ℓ
reply. It processes that reply's reactions in order: matches settle
children, supplies provide children, and each query asks it to reconcile
one child by comparing the peer's listing with its own children there.

For each queried child, the stage performs a complete publication sequence:

1. Send the outgoing reply at level ℓ + 1, including any questions at
   level ℓ + 2.
2. Publish a *resolution* for that queried child: which of its children
   are settled, and which await deeper work.
3. Record the local question records for that outgoing reply, one at a
   time, in the queue for stage ℓ + 2.

It records all questions from this outgoing reply before sending another,
even when several queries came in the same incoming reply. This is the
sequential rule that bounds how far sent questions can outrun their local
records. An *assembler* rebuilds subtrees from the resolutions in order,
filling pending slots with results returned by deeper assemblers before
returning the assembled subtree upward.

The responder also has an *opening*: it answers the greeting's one level-1
question and records its level-2 questions. The initiator's first stage is
stage 1.

```text
initiator                                         responder
          <----------- both greetings ----------->

stage 1 <------------------- reply at 1 ----------- opening
   |                                                 records questions at 2
   | sends replies at 2; records questions at 3
   +------------------------- replies at 2 ------> stage 2
                                                    | sends replies at 3;
                                                    | records questions at 4
stage 3 <------------------- replies at 3 -----------+
   ...                                               ...
```

The alternating stages form the wire-facing chain. Local question queues
skip from stage ℓ to stage ℓ + 2 on the same side; resolutions feed local
assemblers, and assembled results return upward. These additional edges
matter to progress. On the wire, each reply occupies one or more frames,
each tagged with its level and bounded by the frame limit exchanged in
the greetings.

### 4.2 The window

The local question queue at level ℓ holds the records needed to interpret
replies there. Its capacity, the *window* `K(ℓ)`, limits concurrent
disputes. A wider window can keep a link with high bandwidth and long
latency busy; a narrower one retains less state.

The session derives its window from a byte budget (512 MiB by default),
the set sizes in the greetings, and a uniform-hashing model of how many
disputes can plausibly occupy each level. It chooses the widest affordable
width `K` and caps each level by its estimated population `S(ℓ)`, keeping
at least one slot: `K(ℓ) = max(1, min(K, S(ℓ)))`. If actual disputes exceed
that population estimate they wait for slots. This limits their number
without relying on the estimate for correctness. Estimating the bytes
held by each slot is a separate matter, revisited in §7.

### 4.3 The walk's progress contract

The single-socket argument uses the following property of the shipped
walk as an inherited prerequisite; it does not prove the complete
walk-and-assembler algorithm here:

> With independent, reliable, ordered reply queues and no external failure,
> the walk completes a finite reconciliation under the progress assumptions
> below. Local question and resolution queues need at least one slot;
> assembler return queues have one fan of slots.

The relevant local queues are:

| Queue | Producer → consumer | Progress capacity |
| --- | --- | --- |
| Question records at ℓ | Stage ℓ − 2 → stage ℓ; greeting and opening at levels 1 and 2 | At least 1 |
| Resolutions | Stage → its assembler | At least 1 |
| Assembled subtrees | Deeper assembler → parent assembler | One fan |

The publication order in §4.1 is part of this prerequisite. A question is
sent before its local record is made available to its consuming stage;
a resolution is published before the local questions whose deeper results
will fill it. Assemblers consume resolutions and return results in the
corresponding order. Terminal work supplies individual leaves and has no
dependency on a deeper stage. These are the ordering constraints used by
the existing progress argument. They do not imply that every empty-queue
read is already owed an item: an idle stage can simply be waiting for work
that has not yet been generated.

Throughout, liveness assumes a conforming peer, finite inputs, eventual
completion of backend operations independently of the walk, fair scheduling
of runnable tasks, and eventual delivery or an error from the transport.
Terminal stages can still wait for input, storage, or transmission under
these assumptions.

The wire requirement is precise: a blocked reply queue at one level must
not prevent a different level's ready reply from arriving. Today's `Link`
provides seventeen independently flow-controlled streams per direction:
the responder's levels 1, 3, …, 31 plus its terminal leaf stream, and the
initiator's levels 2, 4, …, 30 plus its terminal leaf stream and the
stream for the opening batch this design removes. Each stream is ordered
on its own; cross-level order is unrestricted. Section 6 will
show that the proposed receiver removes the additional circular wait
introduced by sharing a socket, so this inherited progress result still
applies.

## 5. One socket, and the deadlock

Why not put all those frames on one socket and sort them by level at the
receiver? The socket delivers the sender's write order, which can differ
from the stages' consumption order. If delivering the head reply waits for
its stage, the reader cannot reach a later reply that would unblock that
stage. The following shape, found by property testing, makes the cycle
concrete with one-slot question, resolution, and decoded-reply queues.

1. The responder's stage 2 sends a reply at level 3 containing seven
   level-4 questions: one nonempty query about the lowest-radix child it
   disputes and six empty queries about children it lacks.
2. The initiator's stage 3 answers in that order at level 4: a thin reply
   containing further queries, followed by six subtree supplies, S1–S6.
3. The responder's stage 4 takes the thin reply, sends replies at level 5,
   publishes the resolution that awaits deeper results, and records
   questions at level 6. Its assembler takes that resolution and waits.
   Stage 4 then processes S1 and fills the one-slot resolution queue.
   It takes S2 and blocks trying to publish S2's resolution.
4. S3 fills the one-slot decoded-reply queue. The decoder holds S4,
   waiting to enqueue it. The frame handoff can accept one further frame;
   the demultiplexer then blocks on another supply frame. S5 and S6 have
   not drained, and the level-6 replies needed to finish the disputed work
   follow those supplies, so the receiver cannot reach them.
5. Stage 6 must take those replies and process them; its assembler must
   then return the results that fill stage 4's assembler's pending slots.
   Only then can that assembler accept S1's resolution and free stage 4
   to drain the parked supplies. The socket reader is waiting on exactly
   the progress it is preventing.

The shipped protocol's opening batch would make a second such cycle, with
its bulk written before the thin reply the receiver needs first; removing
the batch (§2) removes the cycle.

Three remedies have been considered:

- **Explicit credits:** tell the sender how much each receiving level can
  accept. This is a multiplexed flow-control protocol of its own, as in
  HTTP/2 or QUIC; the current design demands it from the transport.
- **Inferred credits:** infer consumption from the peer's later questions.
  This adds sender-side bookkeeping and learns progress only after a
  network crossing.
- **Unbounded receive buffering:** drain every reply regardless of whether
  its stage is ready. This removes the circular wait but appeared to
  forfeit V2's memory advantage.

The third remedy was rejected before the count was done. The missing
quantity was how far the wire could run ahead of the local question queue.

## 6. Replies are invited

### 6.1 The count

Count one side's outstanding questions at level ℓ. A question counts as
*sent* when the reply carrying it has been flushed to the wire. It remains
outstanding until its stage takes the matching reply out of the decoded
reply queue; that is the *consumption* event, before processing finishes.
Dequeuing the local question record alone does not consume the reply.

For ℓ ≥ 3 the producer is stage ℓ − 2; for ℓ = 2 it is the responder's
opening. By §4.1, the producer sends at most one fan of questions in a
reply, then records all of them before sending another reply. The
consumer holds at most one dequeued question while awaiting its reply.
Every sent, unconsumed question is therefore in one of three places:

```text
  K(ℓ)   recorded in the local queue, not yet dequeued
+    1   dequeued by stage ℓ, matching reply not yet taken
+  256   sent in the producer's current reply, not yet recorded
```

Thus at most `K(ℓ) + 257` are outstanding. Once a reply is taken, any
record retained while processing it no longer contributes to this count;
it still contributes to working memory. At level 1 there is a simpler
base case: the greeting is one question, and the responder's opening
answers it with one reply, so at most one is outstanding toward the
initiator.

A conforming peer answers each question exactly once. Every reply still
in transit, being decoded, or parked corresponds to one of these
outstanding questions. The same bound therefore applies to reply objects.
It does not bound wire bytes: one supply can carry a large subtree, and
frames and control items are different units.

This count depends on the receiver's own window and the fixed fan limit.
It needs neither the peer's window nor a probabilistic assumption about
either tree.

### 6.2 Parking

Call the queue of decoded replies between a level's decoder and stage its
*parking queue*. Give it capacity `C(ℓ) = K(ℓ) + 257` (or the structural
cap in §7). A newly arriving reply must find room: if `C(ℓ)` replies were
already parked, that reply would be a further outstanding reply, exceeding
the count. The queue may reach capacity after an enqueue. The invariant
is that **no arriving reply finds its level's queue already at capacity**.
This is the sense in which parking is “never full” for the receiver.

Room alone is insufficient if decoding waits elsewhere on the walk. The
receiver already absorbs supplied leaves incrementally into the storage
backend, retaining subtree handles rather than content bytes. Two pieces
of bookkeeping still couple the decoder to the walk, and each moves:

- **Received scopes move later.** A decoder currently publishes *scopes*,
  the bookkeeping used to answer the peer's questions inside an incoming
  reply, into a bounded queue. Instead it parks those scopes with the
  reply, and publication moves to the moment the stage takes that reply.
  The scope queue holds one fan plus one; the sequential stage/encoder
  ordering bounds its occupancy, as detailed in the appendix.
- **Sent-question records move earlier.** The encoder currently publishes
  the record a decoder needs to interpret a reply only after that reply's
  last frame has flushed, because publishing earlier could block the
  encoder with a reply half-written. Sized from the count, plus one fan
  for the reply being written, that queue never blocks, so the record is
  published before the reply is written. A reply can then never arrive
  before its record exists, and a decoder that finds none has received a
  reply to a question never sent: a violation, detected without waiting.

The shipped protocol's opening batch, decoded by a pump the walk drives,
would be a third coupling; this design removes the batch (§2). These
changes preserve the walk's reply–resolution–question order; they change
where the proxy keeps its bookkeeping.

A demultiplexer can now read frames in socket order and feed each level's
decoder without waiting for that level's stage. Decoding may take time,
particularly in the backend, but under §4.3's assumptions it completes
independently of the walk. A full parking queue has no further legal
arrival to block.

On the sending side, per-level encoders feed a multiplexer that preserves
each level's frame order and serves question-bearing frames before supply
frames, round-robin within each class. Every frame is served in finite
time because a session's traffic is finite, which is all the argument
needs; the priority keeps the descent's thin replies from queuing behind
bulk (§8). The sender can wait for bandwidth, decoding, or backend
service, but no receiving stage's consumption order can stop drainage and
create the cycle of §5. The inherited progress property therefore
applies. The window mechanism remains; its byte price and hence its
granted capacity change in §7.

A fixed parking depth chosen independently of the window lacks this
general guarantee: a larger window can admit more replies than it can
hold, though that does not force every such run to deadlock. The derived
capacity is sufficient. It is also attainable as a peak occupancy on a
suitable tree and schedule, but that does not establish that one slot less
would deadlock. If an arrival does find the derived queue full, the
implementation must fail rather than wait: either the peer violated the
reply rules or a local premise of the bound is broken.

## 7. Memory: the window, revisited

“Decoded” matters. Parking supply bytes would retain arbitrarily large
subtrees. Incremental absorption instead leaves handles in the reply.
Replica content in the backend, including content absorbed before commit,
is outside the session's working-memory budget. It still occupies storage
(and RAM for an in-memory backend). A failed session discards its
uncommitted content; a persistent backend reclaims it as it does other
uncommitted nodes.

A parked reply has at most one reaction per child. A supply adds a handle;
a match adds no payload beyond its reaction slot; a query adds up to 256
listing entries. Digests are 24 bytes and a radix/digest entry is about
25 bytes. The structural maximum for query-listing payload is therefore
`256 × 256 × 25 B ≈ 1.6 MB` per reply, before reaction slots, vector
metadata, scopes, and allocation overhead. Under uniform hashing a full
fan of full fans is plausible near the top of a large tree; clustered
addresses can produce it deeper down too.

Parking retains the remote half of work the window already admitted.
For each outstanding question the asker holds its local record, while the
peer's reply is still remote, in transit, or parked. There is at most one
parked reply per outstanding question. No second admission mechanism is
needed, and the two parties' windows remain private and may differ.

The budget calculation must now price both halves. It retains the search
for an affordable window, the population caps, and the greeting's bound on
version size used in pricing. It adds the estimated reply size to each
window slot's local-record price and charges fixed slack for the
additional fan and the consuming stage's one question.
Questions at level ℓ concern nodes at depth ℓ − 1, of which there can be
at most `256^(ℓ−1)`. The fixed parking charge per level is thus

```text
min(257, 256^(ℓ−1)) × estimated_reply_bytes(ℓ).
```

The entire parking capacity can likewise be capped at
`min(K(ℓ) + 257, 256^(ℓ−1))`, making the level-1 capacity exactly one.
This structural cap is independent of hashing statistics. Even a zero
requested budget keeps the minimum question slots and fixed working
state; it does not mean zero allocation. The appendix gives an
illustrative parking calculation on the scale of a few megabytes.

The accounting also includes scopes retained with parked replies, the
widened local-question bookkeeping queue, the fan-plus-one scope queue,
and active stage and decoder state. These
must be charged where they are simultaneously live, counting shared
allocations once. The appendix makes that custody audit part of the
implementation; the reply-payload estimate alone is not the full charge.

Two bounds remain distinct. Reply counts and the finite structural maximum
of their metadata are deterministic. The economical byte estimates use
the existing uniform-hashing model, with tail probability `2⁻⁴⁸` per
estimate and a union bound below `2⁻⁴⁰` per session. These are model-based
qualifications, not protection against deliberately clustered addresses.
A session can exceed its estimated byte budget without exceeding a single
queue's slot capacity. Such an overrun does not invalidate the parking
argument, provided storage operations continue to make progress.

On the old bundle, per-stream receive windows enforced consumption pacing,
and charging those transport windows to the session budget was unfinished
work. With parking, transport buffers no longer have to enforce per-level
consumption pacing. They still hold protocol bytes while transmission,
decoding, and backend service proceed, and affect elapsed latency.

## 8. Costs, removals, and evidence

**Dependent crossings stay the same.** Count a hop when a message needed
to advance the descent crosses from one party to the other. V1, V2, and
the single-socket form all descend one level per dependent crossing, with
the same opening and closing exchanges. A stage submits its produced
reply to the encoder and multiplexer; transmission occurs when its frames
are served. No credit exchange or inference round trip is added. This is
a dependency count, not a promise of equal wall-clock latency.

**Logical payload is unchanged.** Frame grammar remains, stream labels
disappear, and preamble and closing control items share the pipe. Snapshot
changes should be confined to those framing changes.

**Serialization and loss have costs.** A supply frame already ahead of a
thin reply delays it, and bytes already buffered in the transport add
further delay. The greeting's frame limit bounds each frame, not the total
backlog. The multiplexer serves question-bearing frames first, so a ready
thin reply waits only for frames already committed ahead of it, never for
the unsent supply backlog. Priority cannot preempt those frames or the
transport's buffered bytes. On TCP, a lost segment delays all levels for
recovery. Work that could have advanced on another QUIC stream now waits;
the owner has accepted that loss of isolation.

**The transport contract shrinks.** A session takes the halves of one
reliable, ordered duplex byte stream whose directions can progress
independently. `Link` and its stream-supply machinery go. Successful
sessions return the halves; after an error the connection must be
discarded because its position is unknown. A QUIC deployment uses one
bidirectional stream. The appendix specifies ownership and migration.

**Tests check the implementation against the argument.** They are evidence
of correspondence, not proofs over all executions:

1. Before changing reception, instrument sent questions minus consumed
   replies on the current transport. Check `K(ℓ) + 257` under the capacity
   stress matrix and adversarial schedules, with the greeting's separate
   one-reply base case.
2. Run the same deterministic wedge over one in-memory duplex pipe. It
   must stall with parking depth one and complete at the derived capacity.
   The same fixture must fail without the cure.
3. Check the arrival invariant across generated trees and schedules, and
   pin a run that reaches exactly `K(ℓ) + 257` parked replies at a level
   where the structural cap permits it. Reaching capacity demonstrates
   occupancy tightness; the depth-one wedge separately demonstrates
   deadlock.
4. Preserve the dependency-hop ledger and account for every wire snapshot
   change. Record the revised sizing model separately from measurements
   of actual allocated memory.

A Lean development explored the walk's deadlock-freedom argument and the
unbounded-parking case; its statements are trusted less than the code,
and nothing in this note rests on it.
