# Reconciliation over one socket

## 1. The claim

`rumors` reconciles two replicas of a message set by walking a hash tree
from the root downward and exchanging only what differs. The shipped
implementation streams that walk, with every level of the tree in flight at
once, and it asks the transport for a bundle of seventeen independently
flow-controlled streams per direction, because the argument that the walk
never deadlocks needs one level's stream to keep flowing while another's is
blocked. The bundle is a real obligation: QUIC provides it natively, TCP
needs a connection per stream, and the crate ships a conformance suite to
police it.

The bundle is unnecessary. The same protocol, with the same messages in the
same order at every level and the same round-trip count, runs over one
ordered duplex byte stream, under a memory bound of the same kind the
session already keeps. Nothing is added to the wire: no credits, no window
advertisements, no inference on the sending side about what the receiver
can absorb. The whole change is on the receiving side, and it rests on one
counting fact about the protocol as it already is:

> At every level, a receiver can never have more replies in flight toward
> it (sent, and not yet consumed by the stage that asked for them) than a
> number it computes from its own configuration.

A receiver that reserves room for that many decoded replies per level can
always drain the socket, and a socket that is always drained never couples
one level to another. The room already exists: each level has a queue of
decoded replies between its decoder and its stage, holding one reply
today. The change is to size that queue, per session, from the window the
session already computes, plus one fan. No queue is added and no edge is
re-plumbed, and with the streams gone the transport abstraction that
supplied them goes too: a session takes the two halves of a byte stream
and nothing else.

Sections 2 to 4 build the protocol up to the point where the bundle became
necessary; §5 shows the deadlock the bundle prevents; §6 does the count; §7
settles memory; §8 says what the change costs, what it removes, and how we
will know it is right. An [appendix](appendix-implementation-plan.md) gives
the implementation plan against the current code.

## 2. The conversation

Every message is stored at one address, the hash of the causal version
stamped on it when it was sent, so the set of messages a replica holds
determines one tree: a 256-ary radix trie, one byte of the address per
level, thirty-two levels deep, with single-child runs compressed away. A
node's children, at most 256, are its *fan*; a held interior node always
has at least two. Each interior node memoizes a *digest* of its subtree, a
function of the version set alone. Two replicas that hold the same set hold
the same tree, and two subtrees with equal digests hold the same messages.

A session opens with a *greeting* from each side: its causal version, its
live-message count, its bounds on version size and on the frames it will
send and accept, and a *listing* of its root's children as pairs of radix
byte and digest. A version advances on every send and every redaction, so
equal versions mean the same history and the same set, and the session ends
with nothing to exchange. Otherwise the two sides descend together.

The descent is a conversation of questions and replies. A *question* about
a node is a listing of that node's children. The *reply* reacts to each
listed child, and adds a supply for each child of its own that the listing
lacked:

- **match**: my digest for this child equals yours; nothing beneath it
  needs attention;
- **supply**: you lack this child; here is its whole subtree, filtered
  against your version so that anything you have already seen and deleted
  stays deleted (this filter is how deletions propagate without tombstones,
  and nothing below depends on it further);
- **query**: we both hold this child and our digests differ; here is *my*
  listing of its children. An empty listing means instead that I do not
  hold this child: send it whole. (A held node's listing is never empty,
  so the two readings cannot collide.)

A query is the next question, one level down, so each reply moves the
descent one level deeper along every disputed path, and the two sides
alternate. Say a question or reply is *at* level ℓ when the nodes it lists
or reacts to sit at level ℓ; a question at level ℓ is thus about a node at
level ℓ − 1, a reply at level ℓ answers a question at level ℓ, and the
queries a reply carries are questions at level ℓ + 1. The initiator's
greeting, listing the root's children, is its question at level 1; the
responder's replies are at the odd levels and the initiator's at the even
ones.

The greeting gives the opening one shortcut. After the exchange, both sides
hold both root listings, so the initiator already knows which of its root
children the responder lacks, and it ships them at once, unasked, as one
batch of supplies. The responder's reply at level 1 still asks for those
children with empty queries; the initiator answers each with nothing
further, and the responder pairs the empty answer with the child it already
received.

Where digests agree the walk stops, so work is proportional to the
difference between the replicas, and the walk ends at the *disjoint
frontier*, the cut below which every subtree is held by one side only.
Uniform hashing spreads addresses evenly over the 256ʲ prefixes at depth
*j*, so disputes thin geometrically with depth: for two fully divergent
sets of a million messages the frontier lies about five levels down.

Two properties of replies carry the argument; hold them lightly until they
are needed.

**A reply answers exactly one question.** Matches and queries are keyed
positionally against the question's listing, and a supply names its child
by radix; no reply carries a path. The asker knows which question it asked
*k*-th at a level, and the *k*-th reply at that level answers it. To
interpret the reply, the asker must have kept the question: its own handles
to the children it listed. One fan of handles per outstanding question is
the protocol's working memory (§4.2).

**Every large message is a supply, and every supply was asked for**, save
the opening batch, whose membership both sides compute from the two
greetings. The only bulk on the wire is subtree content, and content ships
only where a listing showed the receiver lacked it. Questions are small, at
most 256 digests (§6).

Illustratively, a reply's reactions are keyed to the question they answer:

```rust
enum Reaction<Node> {
    Match,                     // the next listed child: same digest
    Query(Vec<(u8, Digest)>),  // the next listed child: my listing of its children
    Supply(u8, Node),          // a child you did not list, named by radix
}

struct Reply<Node> {
    reactions: Vec<Reaction<Node>>,
}
```

## 3. V1: a level at a time

The straightforward wire shape sends the descent one level per message. The
responder computes every reaction at level one, builds one message holding
all of them, and sends it; the initiator reads it whole, computes every
reaction at level two, builds one message, sends it. Nothing at level ℓ + 1
is asked until everything at level ℓ has been answered.

This protocol is simple and hand-verifiably correct, and it needs one
ordered byte stream per direction and nothing more: at any moment exactly
one message is in flight, and its receiver is the party waiting for it. Its
costs are memory and latency. A level's message is built in full before it
ships, so it grows with the divergence, and a session can transiently hold
a second copy of much of the set. And each level costs a full round trip:
`L` levels of dispute take about `2(L + 1)` one-way crossings.

## 4. V2: every level at once

The streaming protocol keeps V1's messages and changes their granularity.
Each reply is sent as soon as its question has been read, one reply per
question rather than one message per level, and replies at different levels
are in flight at the same time. A dispute's path from root to frontier
becomes a chain of replies each departing one hop after the last arrived,
and the session's critical path is one crossing per level plus a fixed
handshake cost, roughly half of V1's.

### 4.1 Stages

Each side runs one *stage* per level it asks at, and stage ℓ is a loop with
three parts. It takes the next question this side asked at level ℓ and the
next reply from the peer at level ℓ; the two pair positionally. It reacts:
a match settles a child, a supply is absorbed, and a query is a dispute,
for which the stage computes its own reply at level ℓ + 1 by comparing the
peer's listing against its own children of that node, and sends it. And it
publishes: for the node it has just processed, a *resolution* naming which
children are settled and which are pending on deeper stages; and, into a
queue for stage ℓ + 2, the questions at level ℓ + 2 that its replies
carried. An assembler per level rebuilds the reconciled tree from
resolutions, in order, filling each pending slot as the deeper result
returns.

Each side also has an *opening*, a stage with nothing to pair: the
responder's answers the greeting's question at level 1 and records
questions at level 2, and the initiator's sends the unasked batch.

```
 initiator                                               responder

 opening: supplies for the root children ──────────────▶ (paired with the empty
          the responder's listing lacks                    questions at 2 below)
           question at 1 (the greeting) ───────────────▶ opening: sends replies at 1,
                                                                  records questions at 2
 stage 1 ◀──────── replies at 1 ──────────────────────────────────┘
   │ sends replies at 2, records questions at 3
   └────────────────── replies at 2 ───────────────────▶ stage 2: sends replies at 3,
                                                                  records questions at 4
 stage 3 ◀──────── replies at 3 ──────────────────────────────────┘
   ⋮                                                               ⋮
```

Each side is a pipeline of stages joined by bounded queues (questions down,
resolutions to the assemblers, assembled subtrees back up), and the wire
joins the two pipelines: the replies one side sends at level ℓ are consumed,
in order, by the other side's stage ℓ, so each level's replies form one more
queue, this one crossing between the parties. On the wire a reply travels
as one or more frames, each tagged with its level and none larger than the
frame bound the greeting exchanged. Seen whole, the session is one chain of
stages, alternating sides, with a wire queue on every link. Every stage is
a deterministic sequential process that blocks only on an empty queue it
reads or a full queue it writes: a Kahn network with bounded queues.

### 4.2 The window

The queue of level-ℓ questions, filled by stage ℓ − 2 and drained by stage
ℓ, is where the working memory lives: each entry holds this side's own
children of the node asked about, so that the reply can be interpreted. Its
capacity is the *window* at level ℓ. A wider window puts more disputes at
that level in flight at once, which is what keeps a long, fat link busy; a
narrower one holds less state. A session sets its window from a byte budget
(512 MiB by default), the two set sizes from the greeting, and the
statistics of a uniformly hashed trie, which bound how many disputes each
level can plausibly carry: level ℓ's queue gets `K(ℓ) = min(K, S(ℓ))`
slots, where `K` is the widest width the budget affords and `S(ℓ)` that
level's plausible population. A level whose disputes outrun the estimate
waits for slots, so the cap bounds memory, not correctness, and every queue
keeps at least one slot at any budget.

### 4.3 Why it cannot deadlock, and what that asks of the wire

A network of bounded queues can deadlock only through a cycle of waits, and
there are two kinds. A stage blocked writing a full queue waits on that
queue's reader, which is below it in the chain; a stage blocked reading an
empty queue waits on that queue's writer, which is above it. The walk
breaks every cycle by fixing the order in which a stage publishes what it
produces for each dispute: first the reply to the peer, then the
resolution, then the questions, recorded one by one into the queue for the
stage two levels below. Under that order, whenever a stage is blocked
reading, the item it needs is owed by a stage that already holds everything
required to produce it: the reply carrying the questions was sent whole
before any of them was recorded, and the resolution before the questions
that fill it. That stage is therefore not waiting above; it is working, or
blocked writing further down. Every chain of waits descends.

The bottom never waits. The deepest stages answer requests for individual
leaves with supplies alone and resolve nothing pending. And returns travel
upward without blocking: each assembler's return queue is one fan wide,
returns arrive in the order the questions were asked, and the questions of
the next resolution are not recorded until this one is published, so at
most one fan of returns ever waits for a resolution the assembler has not
yet received. One slot per queue therefore suffices for progress, and a
wider window only relaxes the wait graph. This is the shipped walk's own
argument, restated; what the rest of this note needs from it is the demand
it makes of the wire.

That demand is *independence*: a full cross-party queue at level ℓ may
block the stage that writes it and nothing else, and reading level ℓ's
replies must never wait on another level's progress. Inside one process
this is automatic. Across a network it is the `Link` contract: seventeen
streams per direction, one for each of the sixteen levels a side speaks at
from the opening down and one for the leaves, each reliable and ordered on
its own, each with its own receiver-paced flow control, with no ordering
promised across them.

## 5. One socket, and the deadlock

Why not write all seventeen conversations into one socket, each frame
tagged with its level, and sort them out at the receiver? Because the
receiver reads the socket in the order the sender wrote it, and that is not
the receiver's consumption order. When the frame at the head of the socket
belongs to a level whose stage cannot take it, everything behind it waits,
including frames for levels that could proceed and whose progress is what
would free the head. This is head-of-line blocking, and in this protocol it
is not an inefficiency but a deadlock. Property testing found the shape the
first time the streaming protocol ran over a single socket, at the one-slot
queues of §4.3:

1. The responder's stage 2, replying about some node, disputes one of its
   children and lacks six others: its questions at level 4 are one query
   with a listing and six empty queries, the dispute happening to sit at
   the lowest radix.
2. The initiator's stage 3 answers all seven at level 4, in radix order:
   first the thin reply about the disputed child, carrying the initiator's
   own questions at level 5, then six fat supplies.
3. The responder's stage 4 takes the thin reply, sends its replies at level
   5, publishes a resolution with pending slots, and records its questions
   at level 6. It takes the supplies in turn, absorbing each and publishing
   its resolution, and blocks on the second: the one-slot resolution queue
   is occupied, and the assembler, which assembles in order, is waiting on
   the first resolution's pending slots.
4. Those slots fill from the initiator's replies at level 6, which the
   initiator has already written. They sit in the socket behind the
   remaining supplies.
5. The receiver's demultiplexer holds a supply it cannot hand to the blocked
   stage, so it never reaches the level-6 replies. Nothing moves.

Each link in that chain is by design; what breaks is the independence
demand of §4.3. The opening batch makes the same shape at the top of the
tree, with the bulk written *before* the thin reply the receiver needs
first. Three remedies are known, and the crate's history tried each.

- **Explicit credits.** Give the sender the receiver's constraints as
  per-stream credits, in the manner of HTTP/2 and QUIC. This works, and it
  is a flow-control protocol of its own; the decision was to demand it of
  the transport rather than build it.
- **Inferred credits.** Let the sender infer what the receiver has consumed
  from the receiver's later questions. This can be made to work, but the
  inference engine is large and lags a round trip behind the truth.
- **Unbounded buffering at the receiver**, so the socket can always be
  drained. A queue that never fills couples nothing, so §4.3's demand is met
  outright; this was rejected because the buffer appeared to grow with the
  divergence, forfeiting the memory bound the streaming protocol exists to
  keep.

The third was rejected on a miscount.

## 6. Replies are invited

### 6.1 The count

Recall the first property of §2: a reply answers exactly one question. Now
count the questions one side can have outstanding at level ℓ.

They are asked by stage ℓ − 2, or at level 2 by the opening. That stage
sends each reply at level ℓ − 1 as one message carrying every question it
asks, at most one per child, so at most 256. It then records those
questions one at a time in the level-ℓ queue, whose capacity is `K(ℓ)`,
blocking when the queue is full, and it sends no further reply until every
question of the current one is recorded. Stage ℓ dequeues one question at a
time and only then waits for its reply. So the questions at level ℓ on the
wire whose replies are not yet consumed number at most

    K(ℓ)   recorded, not yet dequeued by stage ℓ
  +   1    dequeued by stage ℓ, its reply awaited or in hand
  + 256    sent in the current reply but not yet recorded, the queue being full

or `K(ℓ) + 257`. A conforming peer answers each question exactly once and
never answers a question that was not asked, so the same number bounds the
replies at level ℓ the peer can have in flight toward this side; and by the
second property of §2, replies are all there is, so it bounds everything
the peer can have on the wire at that level. It depends on nothing the peer
does and nothing about either tree's contents: only on this side's window,
which this side chose from the greeting, and on the fan, a constant of the
tree. The opening batch, the one message that answers no question, is at
most one fan of subtrees, and its membership is known from the greetings.

The analysis behind the third remedy left exactly this quantity open: how
far the wire can run ahead of the local queue. The answer is one reply's
worth, plus one.

### 6.2 Parking

On the receiving side, each level already has a queue of *decoded* replies
between its decoder and stage ℓ, one reply deep. Size it to hold
`K(ℓ) + 257`, and give the opening batch's decoder one fan of room for its
decoded root children. That is the whole change to the dataflow: the same
queue, on the same edge, with a capacity taken from the window the session
computes from the greeting. Call the widened queue the level's *parking*.
A demultiplexer reads a frame, hands it to that level's decoder, and reads
the next; the decoder assembles frames into a reply and parks it. The
decoder's only dependency is the storage backend, never a stage, so it
always finishes; and by the count no reply ever arrives to find its queue
full, so the demultiplexer never waits on any stage and the socket is
always drained. Stage ℓ takes replies from its parking queue in order,
exactly as it took them from its stream before.

On the sending side, the per-level encoders feed one multiplexer that
writes ready frames to the socket. Any order that starves no level is safe,
since nothing in the argument depends on the order across levels, and
within a level frames keep their order; the sender's writes wait on nothing
but bandwidth and the frames ahead of them. Nothing else changes: not the
walk, not its publication order, not the assemblers, not the window, not
the frames.

This is enough because of what §4.3 asked of the wire. A queue that is
never full blocks no writer and couples no levels, so independence holds,
and the walk's own argument goes through over one socket exactly as it does
inside one process. Unbounded buffering was always safe; the count says the
buffer never needs more than `K(ℓ) + 257`, and a bounded queue that no
arrival ever finds full is, in every reachable state, the unbounded one.

The depth must be derived, not chosen. A parking depth fixed on its own,
say a thousand replies, fails as soon as a session's window at that level
exceeds it: more questions outstanding than a thousand, and more answers
than room. Only a depth computed from the window and the fan survives, and
both are known before the descent begins.

If a reply does arrive to a full parking queue, the peer has sent more than
it was asked, which the protocol already treats as a violation when it
finds an unasked reply at the end of a level; the session fails with a
typed violation instead of blocking.

## 7. Memory: the window, revisited

"Decoded" does the work in §6.2. A supply reply can carry a subtree of any
size; parking its bytes would bring back unbounded memory. But the
receiver's decoder already hands each leaf to the storage backend as it is
read, retaining only the reply's skeleton, so a decoded supply is a handle
into the backend, and the content it carried is content the replica is
about to hold in any case. (On a failed session that content is discarded
with the rest of the session's state; a persistent backend reclaims it as
it reclaims any uncommitted node.) A parked reply therefore costs one
reaction per child: a handle for a supply, nothing for a match, and at most
256 digests for a query. The worst case, a reply about a node whose every
child is disputed with a full fan of grandchildren apiece, is about 1.5 MB
of digests, a shape that exists only near the root of a very large tree.

Parking is not a second window. Consider one dispute in flight, from the
moment its question is sent until its reply is consumed. Two pieces of
state exist on the asking side: the asker's own children of the node, held
to interpret the reply; and the peer's reactions, remote until they arrive
and then decoded until the stage reaches them. Every outstanding question
has exactly one local record (in the window queue, in the stage's hand, or
awaiting recording) and at most one parked reply. They are the local and
remote halves of the same set of in-flight disputes, and the count of §6.1
says how many such pairs can exist at once. Parking admits no work the
window did not admit first; the window remains the one knob.

For the sizing calculation this means one thing. The remote half of each
dispute in flight, which on a bundle waited in the transport's buffers and
was never priced by the session, is now priced beside the local half, from
the same fan statistics one level down, and the granted window narrows by
that term; the search for the widest affordable width, the population caps,
and the greeting's version-size bound are unchanged. There is also a fixed
floor: one fan plus one of slack per level, there because one reply can fan
out 256 ways before its questions can all be recorded (the walk already
pays for that shape in its fan-wide return queues), capped by the number of
nodes a level can hold at all, which keeps the top of the tree cheap. Even
at a zero budget, where every window queue keeps its single slot, the
floor for two divergent sets of a hundred thousand messages comes to a few
megabytes; the arithmetic is in the appendix.

Two bounds are in play and should be kept apart. The bound on how many
replies are parked is absolute, from the count. The bound on their bytes is
statistical, resting on the uniform spread of hashed addresses exactly as
the window's sizing already does; clustered addresses can exceed it, and
then the session holds more than it estimated, as it can today, but nothing
blocks.

What dissolves is the transport's part. On a bundle, the per-stream receive
window was load-bearing, the mechanism by which the receiver's consumption
paced the sender, and sizing those windows from the session budget was an
open task. On one socket the transport's buffer holds undecoded bytes only
until the demultiplexer reads them, which it always does: pure
bandwidth-delay storage, with no protocol state in it, its size the
transport's business. And nothing about the window crosses the wire. Each
side parks what it asked for, so the two sides' windows stay private and
may differ, as they may today.

## 8. Costs, removals, and evidence

**Round trips** are unchanged. The sender writes each reply the moment it
is produced, the receiver drains the socket continuously, and each reply
crosses in one hop and is consumed in the order it was before. Nothing is
inferred and nothing is granted, so there is no inference lag and no credit
round trip. The sender is if anything freer: its stages never wait on the
receiver's consumption order, where today a stage can block mid-reply on a
peer's full stream window.

**Bandwidth** is unchanged, less the per-stream labels; frames already
carry their level.

**One socket costs two things**, both properties of running any multiplexed
protocol over one TCP connection and neither introduced here. Head-of-line
delay at frame granularity: a large supply frame ahead of a small reply
delays it by the frame's transmission time, bounded by the greeting's frame
bound, and kept off the critical path by having the multiplexer of §6.2
prefer question-bearing frames. And loss coupling: a lost segment stalls
the whole connection for one recovery where a QUIC bundle would stall one
stream; since a session completes no earlier than its slowest level in any
case, this costs less than it first appears.

**What is removed.** The transport contract collapses to one ordered duplex
byte stream whose two directions are independent, and with it the `Link`
abstraction that bundled streams: a session takes the read and write halves
directly, by value, and hands them back when it completes. A session that
fails keeps them, which is ownership saying what a poison flag said before,
that the stream's position can no longer be trusted. The stream-supply
machinery that served the bundle (the appendix inventories it) is replaced
by the multiplexer and demultiplexer of §6.2, and the session's preamble
and closing marker share the one pipe with the frames, in order. A QUIC
deployment opens one bidirectional stream.

**How we will know.** Confidence rests on tests, in this order.

1. Pin the count on today's code before anything moves: across the
   capacity stress matrix and under adversarial scheduling, the questions
   sent at a level but not yet consumed never exceed `K(ℓ) + 257`.
2. Demonstrate the deadlock and its cure on one shape: the deadlock of §5 over
   a single in-memory pipe stalls at a parking depth of one and completes
   at `K(ℓ) + 257`. A test that passes only with the cure present is the
   criterion's proof that it can fail.
3. Show the bound holds, and that it is tight: across generated trees and
   adversarial schedules no arriving reply ever finds its parking queue
   full; and one constructed shape parks exactly `K(ℓ) + 257` replies and
   stalls at one less, the proof that the `+ 1` is real.
4. Hold the round-trip ledger: the hop-count instrument over one pipe reads
   the same per-level hop counts as over the bundle.
5. Keep the wire snapshots honest: the only bytes that move are the deleted
   labels and the in-band control items, and the commit that moves them
   says so.

A Lean development under `formal/` explored the deadlock-freedom argument
of §4.3 and the unbounded-parking case; it was produced with heavy model
assistance, its statements are trusted less than the code, and nothing
above rests on it. If the count is ever transcribed there, it is one safety
invariant, that parked replies at a level never exceed the window plus the
fan plus one, and the rest follows by the never-full argument.
