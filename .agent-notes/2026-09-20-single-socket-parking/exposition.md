# Reconciliation over one socket

## 1. Summary

`rumors` reconciles two replicas of a message set by walking a hash tree
from the root downward and exchanging only what differs. The protocol
streams that walk: several tree levels are in flight at once, and each
advances as its own inputs arrive. So that one level waiting on its
consumer never stops another level's traffic, the protocol today runs
over seventeen independently flow-controlled streams in each direction.
QUIC provides such streams natively; over TCP they cost a connection
each.

This design runs the same protocol over one ordered duplex byte stream.
The logical messages, their order within each level, and the number of
dependent network crossings stay the same. The wire gains no credits, no
window advertisements, and no inference by the sender about the
receiver. The design rests on one count:

> At each level, the number of replies travelling toward a receiver that
> its consuming stage has not yet taken is bounded by the receiver's own
> configuration: its question-queue width plus 257.

A receiver that reserves that many slots per level can accept every
reply it has invited without waiting for the stage that will consume it.
Given that room, and with one piece of bookkeeping moved and one
shortcut removed, decoding never waits on the walk. The socket reader
therefore never blocks on the walk, and the walk's existing progress
argument carries over unchanged.

The design has three costs. The first is memory. Replies that the
stream bundle held back at the sender now wait, decoded, at the
receiver, and the session's memory budget must price them. The design
also sharpens the pricing in two places, so the net effect depends on
scale. For two entirely different replicas of 10⁶ messages, the budget
that keeps a 12.5 MB bandwidth-delay link fully busy falls from about
0.75 GiB to about 0.46 GiB. At 10⁷ messages it rises from about 1.7 to
about 2.1 GiB. Replicas that differ in a few hundred messages are priced
at a few MiB.

The second cost is serialization. A small, urgent frame can wait behind
a large frame already committed to the socket. That wait adds elapsed
time but never a round trip, and a socket option bounds it. The third is
loss isolation: on TCP, one lost segment delays every level.

Sections 2 and 3 describe the protocol and the progress property it
already has. Section 4 shows how one socket naively deadlocks, and §5
derives the count and the receive path it permits. Sections 6 and 7
account for memory and time, and §8 lists the committed checks. Section
9 contrasts the design with explicit credits, and §10 examines three
follow-ons. The [appendix](appendix-implementation-plan.md) is the
implementation plan.

## 2. The conversation

Every message has an *address*: the hash of the causal version stamped
on it when it was sent. The 32-byte addresses determine a 256-ary radix
trie, one address byte per level. A node's children, at most 256, are
its *fan*. Storage compresses single-child chains, but the conversation
still descends one byte per level. Each interior node memoizes a 24-byte
*digest* of its subtree. The protocol assumes that distinct subtrees
have distinct digests, so equal digests mean equal subtrees.

Each side opens with a *greeting* carrying its causal version (an
interval tree clock, ITC), its live-message count, and a *listing* of
the root's children as (radix byte, digest) pairs. A version summarizes
a replica's entire history of sends and redactions, so equal versions
mean equal sets, and the session ends there.

Otherwise the sides exchange questions and replies. A *question* about a
node lists that node's children. A *reply* reacts to each listed child
in order, and adds any children the replier holds that the listing
omits. Each reaction is one of three kinds:

- a *match*: my digest for this child equals yours;
- a *supply*: you lack this child, so here is its subtree, filtered
  against your version so that messages you have seen and deleted stay
  deleted;
- a *query*: our digests differ, so here is my listing of this child's
  children. If I lack the child entirely, the listing is empty, which
  asks you to send all of it.

A query is itself the next question, one level down, so every reply both
answers one side and asks the other to continue. A question or reply is
*at level ℓ* when the children it lists or reacts to sit at depth ℓ. The
initiator's greeting listing is the question at level 1. The responder
therefore replies at odd levels and the initiator at even levels, and
the final exchange supplies individual leaves.

For example, suppose A asks B about a node whose children, in A's
listing, are `10` and `20`. B holds the same `10`, holds a different
`20`, and also holds `30`, which A lacks. B's reply is `Match`,
`Query(B's children of 20)`, `Supply(30, subtree)`. The first two
reactions refer to A's listing by position, and the supply names its
radix. B's query is now a question to A, one level down.

Today's protocol adds one shortcut, which this design removes. After the
greetings cross, the initiator already knows which of its root children
the responder lacks, and ships them unasked as an *opening batch*, one
crossing early. Under uniform hashing, a replica of n messages leaves a
given root slot empty with probability about e^(−n/256), so the batch is
nonempty essentially only between replicas below a few thousand
messages. It needs its own stream, decoder, and pairing rule, and over
one socket it creates a second instance of the deadlock in §4. Without
it, those children travel one crossing later, as ordinary supplies
answering the responder's empty queries. The greeting's root listing is
a separate thing and stays: it is the level-1 question, it lets the
responder answer at once, and it needs no stream of its own.

Matching digests prune shared subtrees, and the walk follows disputed
prefixes until it reaches subtrees held by one side only. For replicas
of about n messages that differ in `D`, the deepest disputes sit near
depth `log₂₅₆(D·n)`: about five when `D = n = 10⁶`.

Two properties of the conversation carry the rest of this note. First,
each reply answers exactly one question. Matches and queries refer to the
question's listing by position, and replies carry no path, so the *k*-th
reply at a level answers the *k*-th question there. The asker therefore
keeps a local *record* of each question it asks until the reply arrives.
Second, only supplies carry content of unbounded size. A query lists at
most 256 children, while a supply can carry an arbitrarily large
subtree.

## 3. The streaming walk

### 3.1 A level at a time

A protocol could send one complete level per message, alternating sides.
Exactly one message would then be in flight, and its receiver would be
waiting for it, so one ordered byte stream per direction would suffice.
The cost is aggregation. A message holds a whole level of disputes, can
approach the size of the set, and cannot depart until all of it is
ready. Each message advances the conversation one level per network
crossing. The streaming walk keeps that crossing count and removes the
aggregation.

### 3.2 The streaming walk

The streaming walk sends one reply per question instead of one message
per level. Each side runs a *stage* for each level at which it asks
questions and so receives replies. The node a question is about is its
*scope*. Stage ℓ repeatedly takes the record of its side's next level-ℓ
question, then takes the reply that answers it, an event this note calls
the *take*, and processes the reply's reactions in order. Matches and
supplies settle children. Each query asks the stage to reconcile one
child, which produces an outgoing reply at level ℓ + 1 carrying
questions at level ℓ + 2.

For each outgoing reply, a stage follows a fixed *publication order*:

1. hand the reply toward the wire;
2. publish a *resolution* saying which of the child's children are
   settled and which await deeper work;
3. record the reply's questions, one by one, in the queue feeding stage
   ℓ + 2.

It records all of one reply's questions before handing on another reply.

The walk's result is the merged tree, which an *assembler* per level
builds from resolutions. An assembler takes resolutions in order. For
each child that awaits deeper work, it waits until the next assembler
down returns that child's finished subtree. While an assembler waits on
a deep result it takes no further resolutions, so its resolution queue
can fill.

```text
initiator                                         responder
          <----------- both greetings ----------->
stage 1 <------------------- reply at 1 ----------- opening
   | sends replies at 2; records questions at 3     records questions at 2
   +------------------------- replies at 2 ------> stage 2
                                                    | sends replies at 3;
                                                    | records questions at 4
stage 3 <------------------- replies at 3 ----------+
   ...                                               ...
```

On the wire, each reply occupies one or more *frames*, one reaction per
frame, each tagged with its level. A supply's leaves travel in *supply
runs*: frames sized by a byte budget the two sides agree on in the
greeting, about 1.8 MB by default, which is the size of a maximally
disputed reply. Every other frame (a match, a query, or the end of a
reply) carries at most one 256-entry listing, about 7 KB. This note
calls those frames *thin*, and a reply made only of them a thin reply.
Thin frames carry the *descent*, the part of the conversation that finds
out what differs. Supply runs carry the *bulk*, the content that
transfers the difference.

### 3.3 The window

The question queue at level ℓ holds the records a side needs to
interpret replies there. Its capacity, the *window* `K(ℓ)`, limits how
many disputes a level can have in flight. A wider window keeps a long,
fast link busy, and a narrower one retains less state. The session
derives the window from a byte budget (512 MiB by default today), the
set sizes in the greetings, and a uniform-hashing model of how many
disputes each level can hold, `S(ℓ)`. It chooses the widest affordable
`K` and gives each level `K(ℓ) = max(1, min(K, S(ℓ)))`. Section 6
describes the pricing.

### 3.4 The progress property

The streaming walk is already deadlock-free, given one premise about its
channels. Every wait in the walk is for the next item of one specific
queue, and every queue's producer produces its items in order. The
publication order makes one slot per question queue and per resolution
queue enough. A stage blocked recording a question has already handed
its reply toward the wire, and a stage blocked publishing a resolution
sits behind an older resolution whose dependent work is already under
way. The assemblers' return queues hold one full fan. The argument is
written at the head of the walk's module (`streaming/materialized.rs`),
and a Lean development checks a model of it.

The premise is *independence*: a full queue stalls only its own
producer, never delivery on another queue. Within one process that
holds by construction. Across the wire, today's transport provides it
with seventeen independently flow-controlled streams per direction,
ordered within each stream but not across streams. The responder's
seventeen carry its sixteen odd levels and the leaves it supplies at the
end. The initiator's carry its fifteen even levels, its final leaf
requests, and the opening batch.

This design keeps the walk and its argument, and replaces that transport
with one socket. What must be shown is that sharing the socket cannot
let one level's blocked consumer stop another level's delivery.

## 4. One socket, and the deadlock

A socket delivers bytes in the order the sender wrote them, which need
not be the order in which the receiver's stages consume them. If the
reader delivering the head frame must wait until that frame's stage
accepts it, the reader cannot reach a later frame that would let the
stage accept it.

A committed test fixture produces the smallest such cycle, at the root.
Both replicas hold root child 0, with digests that differ all the way
down to one leaf. The initiator also holds six root children, 1 through
6, that the responder lacks. (The fixture's trees also hold root
children that only the responder has. They play no part in the cycle
and are left out here.) Give every queue one slot, including the queue
of decoded replies waiting for their stage:

1. The responder's opening reply at level 1 asks seven level-2
   questions: a query listing its children of 0, and six empty queries
   for 1 through 6.
2. The initiator's stage 1 answers them in order at level 2: a thin
   reply that queries deeper under 0, then six supplies, S1 through S6,
   which reach the socket in that order.
3. The responder's stage 2 takes the thin reply. It sends a level-3
   reply, publishes the resolution for 0 (pending deeper work), and
   records its level-4 question. The assembler takes that resolution and
   waits for the deeper result.
4. Stage 2 takes S1 and publishes its resolution, which fills the
   one-slot resolution queue, since the assembler is still waiting on 0.
   It takes S2 and blocks publishing the next resolution.
5. S3 fills the decoded-reply queue. The decoder holds S4, waiting to
   queue it. The socket reader has handed S5 to the decoder's input and
   holds S6, waiting.
6. Behind S6 on the socket is the initiator's level-4 reply, the one the
   level-4 question from step 3 asked for. Stage 4 needs it to finish
   0's subtree. Only then can the assembler accept S1's resolution, free
   stage 2, and let the supplies drain. The socket reader waits on
   exactly the progress it is preventing.

The same cycle can form at any depth. The opening batch creates it
directly, because its bulk is written before any level-2 reply,
including the thin one the responder's descent needs first.

There are three ways out. With *explicit credits*, the receiver tells
the sender how much each level can accept; this is a multiplexed
flow-control protocol of its own, as in HTTP/2 or QUIC. With *inferred
credits*, the sender infers consumption from the peer's later questions,
which adds bookkeeping and learns of progress a crossing late. With
*receive-side buffering*, the receiver drains every reply whether or not
its stage is ready. That removes the cycle, and the next section shows
that the buffer it needs is bounded.

## 5. Replies are invited

### 5.1 The count

A reply exists only because its receiver asked the question it answers,
so we count questions from the asker's side. A question at level ℓ is
*outstanding* from the moment the stage that asks it hands its carrying
reply toward the wire until the stage that consumes it takes the
matching reply. The take falls after the stage dequeues the question's
record and before it finishes processing the reply.

For ℓ ≥ 3 the asking stage is stage ℓ − 2, and for ℓ = 2 it is the
responder's opening. By the publication order (§3.2), the asking stage
hands on one reply carrying at most 256 questions, then records all of
them before handing on another. The consuming stage holds at most one
dequeued record while it waits for that record's reply. Every
outstanding question is therefore in exactly one of three places:

```text
  K(ℓ)   recorded in the question queue, not yet dequeued
+    1   dequeued by stage ℓ, its reply not yet taken
+  256   handed toward the wire in the asking stage's current reply,
         not yet recorded
```

At most `K(ℓ) + 257` questions at level ℓ are outstanding. A level-ℓ
question concerns a node at depth ℓ − 1, of which there are at most
`256^(ℓ−1)`, so level 1 has one outstanding question (the greeting's
listing) and level 2 at most 256.

A *conforming* peer, one that follows the protocol, answers only
questions it has received, and each only once. Every reply in transit,
being decoded, or waiting for its stage therefore answers a distinct
outstanding question, and the same bound applies to replies. The bound
counts replies. A supply can carry a large subtree, and §6 deals with
size.

The count is a property of the asking side's walk alone. It needs
neither the peer's window nor any statistical assumption about either
tree, and it depends on nothing in the transport.

### 5.2 Parking

Call the queue of decoded replies between a level's decoder and its
stage the level's *parking*. Its capacity is computed from the level's
actual question-queue capacity `K(ℓ)`, whatever the window chose:

  `cap(ℓ) = min(K(ℓ) + 257, 256^(ℓ−1))`.

Correctness rests only on that link between the two capacities: however
the window chooses `K(ℓ)`, parking has room for everything the count
allows.

The two sides' windows may differ. Each side sizes its window from its
own budget, and the greeting does not carry it. That is safe because a
reply travels only in answer to its receiver's own question. The replies
in flight toward a side are bounded by that side's window, and its
parking is sized from the same number. Suppose one peer has a far wider
window. It asks more questions at once, so more replies flow toward it,
and they park in its own parking. The narrow side receives the wide
peer's questions only inside replies to its own questions, at most 256
per reply, so its count and its price already cover them. Its outgoing
replies park at the wide peer, whose socket reader never waits. A
mismatch therefore changes only how much each side asks at once, which
affects throughput but never progress. Asking a question
grants credit for exactly one reply, and each receiver issues its own.

Parking can fill, but no conforming arrival can find it already full:
that arrival would be one outstanding reply more than the count allows.
So the decoder checks occupancy before it parks a reply. A full parking
queue means that the peer answered a question never asked, or that a
local premise of the count has failed. In either case the session fails
with an error that says so, instead of waiting.

The capacity is also as small as it can be. A committed test holds a
level's consuming stage until exactly `K(ℓ) + 257` replies are parked,
so any smaller capacity would fail a conforming session.

### 5.3 A receive path that never waits on the walk

Room is not enough if decoding waits on the walk for anything else.
Over one socket, the receive path has two parts: a *demultiplexer* that
reads frames in socket order and hands each to its level's decoder, and
one decoder per level that rebuilds replies and parks them.

The receive path handles two kinds of question record, and the argument
needs them kept apart. A decoder interprets each reply against the
record of the question it answers. That question is our own, asked in a
reply we sent, so this note calls its record a *local record*; our
encoder publishes it. Conversely, when our encoder sends an answer, it
needs the record of the peer's question it is answering. This note
calls that an *answer record*; it is derived from the peer's reply
that carried the question.

Here is every wait on the receive path:

| Waiter | Waits for | Why the wait ends without the walk |
| --- | --- | --- |
| Demultiplexer | the socket | The peer writes independently. |
| Demultiplexer | its level's decoder to accept a frame (a one-slot handoff) | The decoder is only finishing its current frame. |
| Decoder | the local record for its next reply | See below. |
| Decoder | the storage backend, absorbing supplied leaves | The backend contract requires it (below). |
| Decoder | room in parking | Never, under conformance (§5.2). The decoder checks and fails instead of waiting. |

Local records follow the walk's own rule, wire before internal
publication. The encoder publishes a reply's local records just after
handing the reply's last frame to the multiplexer's channel (§5.4). It
does not wait for the multiplexer to write that frame, so the records
are published before the frame reaches the wire. A decoder waits for a
local record only when the queue of them is empty. If a reply to one of
those questions has arrived, the encoder is already past the carrying
reply's last frame, and with the queue empty its publication cannot
block. The decoder therefore waits only for its own side's encoder to be
scheduled.

Answer records are the one piece of bookkeeping that moves. Today the
decoder publishes them into a window-sized queue right after handing a
reply on. With parking, the decoder can run as far ahead of the walk as
a full parking queue allows, so that queue could fill while the decoder
still has replies to park, and the decoder would then wait on the walk.
Publication therefore moves to the take: when the stage takes a reply,
the reply's answer records are derived from it and published, and only
then is the reply handed to the stage.

The answer-record queue then needs `256 + 1` slots. The encoder takes an
answer's record *before* it takes the answer itself. The walk takes its
next reply only after handing on all of the current reply's answers, and
the channel between the walk and the encoder holds one answer. So when
the stage takes a reply, the encoder has consumed every earlier answer's
record except possibly one: the record for the answer still sitting in
that channel. That leftover record plus up to 256 new ones fit in 257
slots. With 256 slots the take could wait briefly for the encoder, and
with fewer it can deadlock.

The storage backend is the one resource the receive path shares with the
walk. Decoding would wait on the walk if a walk task could hold
something a decoder needs, such as a lock, a transaction, or a slot in a
bounded pool, while that task is blocked on a queue. The backend
contract must therefore require that backend operations complete without
waiting on any other session work. The in-memory backend meets this by
construction: its nodes are immutable and shared, and it takes no
locks. A persistent backend must meet it explicitly.

The walk's other bookkeeping queues keep their existing arguments. On
the receive path, then, nothing the walk controls can stop the socket
reader.

### 5.4 The sending side

Per-level encoders feed a *multiplexer* that owns the socket's write
half. It keeps each level's frames in order, and it serves every frame
that is not a supply run before any supply run, round-robin within each
class. A session's traffic is finite, so every frame is still served
eventually. The preference exists for latency (§7.3): the thin replies
that advance the descent do not queue behind unwritten bulk.

The sender can wait for bandwidth, and encoders can wait for the
multiplexer, but no receiving stage's consumption order can stop the
socket from draining. The cycle of §4 therefore cannot form, and the
walk's progress property holds over one socket.

## 6. Memory

### 6.1 What a parked reply holds

Parked replies are decoded, but they hold no content. The decoder
absorbs supplied leaves into the storage backend as they arrive, and a
parked supply holds only a handle to the absorbed subtree. The session
budget covers working state, and replica content lies outside it.
Content absorbed by a session that later fails is uncommitted, and the
backend reclaims it as it reclaims any uncommitted node.

A parked reply to a question about a node P at depth ℓ − 1 holds its
reactions, one for each child of P that either side holds, and the
listings in its queries: at most 256 entries per query, at 25 bytes each
in memory. Parking also keeps the record of the question the reply
answers. It does not keep answer records for the reply's own questions,
which are derived at the take (§5.3).

### 6.2 How the window prices memory

The window model knows the two set sizes, `A` and `B`, and uses
`n = max(A, B)`. For each depth it uses two kinds of bound. The
deterministic bounds are that a node has at most 256 children, and that
at most `occupied(n, j) = min(256ʲ, n)` prefixes at depth j are
occupied. The statistical bounds assume uniform hashing: `C(j)` bounds
the children and `L(j)` the leaves under any single depth-j node, and
`S(d)` bounds the scopes (questions) that can be in dispute at depth d.
Each statistical bound is a quantile at tail 2⁻⁴⁸ that holds for every
node at its depth simultaneously, and a union bound over all of them
keeps the session's failure probability below 2⁻⁴⁰. Knowing only set
sizes, `S(d)` prices the two sets as if they were entirely different.

The charge assumes that every window slot is full at once and that every
slot's contents are at their quantile. It is a worst case that holds
with high probability, far above what a typical session uses. The window
is the largest `K` whose charge fits the budget. A session that crosses
the tail uses more memory than estimated, and its correctness and
progress are unaffected.

### 6.3 The price of parking

At level d, parking holds at most `slots(d) = min(K(d) + 257,
256^(d−1))` replies. One reply to a question about a node P at depth
d − 1 holds at most `2·C(d−1)` reactions and at most
`min(C(d−1)·C(d), L(d−1))` listing entries. The second term holds
because every listed grandchild prefix contains at least one of the
replier's leaves under P. Let ρ be the bytes per parked reaction and ε
the bytes per listing entry.

For each depth d, the charge is the least of three bounds.

1. The *per-reply* bound multiplies the slots by the largest reply:
   `slots(d) · (2·C(d−1)·ρ + min(C(d−1)·C(d), L(d−1))·ε)`.
2. The *per-level* bound is deterministic. Parked replies at one level
   concern distinct nodes, whose subtrees are disjoint, so across the
   level their reactions number at most `2·occupied(n, d)` and their
   entries at most `occupied(n, d + 1)`.
3. The *per-set* bound sharpens that disjointness statistically. The
   peer's leaves under any fixed set of q distinct depth-j nodes follow
   Binomial(n, q/256ʲ). A union over every such set costs
   `log₂ binom(256ʲ, q)` bits of tail, which
   `log₂ binom(N, q) ≤ q·log₂(eN/q)` bounds by
   `q·(8j + 2 − ⌊log₂ q⌋)`. The binomial quantile at tail
   2^−(48 + that) then bounds the leaves under whichever q nodes the
   protocol happens to hold, and with q = 1 it equals `L(j)`. Applied
   with q = `slots(d)` and j = d − 1, it bounds the level's entries, and
   twice that bounds the reactions.

### 6.4 Level 2, level by level

One term of this price behaves differently from the rest. At level 2,
`slots(2) = min(K(2) + 257, 256) = 256` at any window, so parking can
hold every reply the level has, and its price shrinks with neither the
budget nor the link. The first levels of one session show why level 2
alone behaves this way, and how large its term gets.

Take two replicas of n = 10⁷ messages that share nearly everything,
where one side also holds D = 200,000 messages scattered by hash. Two
quantities drive every level. The first is size. A node at depth j holds
about `n / 256ʲ` leaves: 39,000 at depth 1, 153 at depth 2, and 0.6 at
depth 3. Spread over 256 child slots, x leaves occupy about
`256 · (1 − e^(−x/256))` of them, so the root and each depth-1 node have
all 256 children, and a depth-2 node has about 115. The second is
difference. A node differs between the replicas when at least one of the
D extra messages falls under it, which happens with probability
`1 − e^(−D/256ʲ)`: 1 at depth 1, 0.95 at depth 2, and 0.012 at depth 3.

At level 1, the one question is the initiator's greeting listing and the
one reply is the responder's opening. All 256 root children differ, so
that reply holds 256 queries, each listing that child's 256 children:
65,536 entries, about 1.6 MB. At most one reply parks.

At level 2, the opening reply asks 256 questions, one per root child,
all at once. They are the count's 256-question term (§5.1): handed
toward the wire together, before any is recorded, so no window limits
them. They are also the whole level, because only 256 depth-1 nodes
exist. The initiator's stage 1 answers each one. A reply reacts to one
root child's 256 children, of which 95% differ, so it holds about 244
queries, each listing about 115 depth-3 children. That is about 28,000
entries per reply, about 700 KB, and all 256 replies together hold 7.2
million entries, about 180 MB.

At level 3, the initiator asks about 62,000 questions inside those
replies, and their replies park at the initiator. Here the receiver's
own window governs: at most `K(3) + 257` are outstanding. Each reply
reacts to one depth-2 node's 115 children, of which only 1.2% differ, so
it is mostly matches with one or two short queries, about 3.7 KB. A
receiver that chose a wide level-3 window pays for it slot by slot, as
its budget priced; one that chose a narrow window parks at most 258
replies. Deeper levels behave like level 3, and their replies shrink as
fewer nodes differ.

Level 2 is special, then, because its entire question population fits
in the count's 256-question term. Level 1 has one question. From level 3
on, a level can have 65,536 questions or more, and the receiver's window
caps how many are outstanding.

The initiator paces level 2 too, but the responder cannot count on it.
Stage 1 records each reply's 244 level-3 questions before handing on the
next reply, so the initiator's own level-3 window limits how far level 2
runs ahead. At the default budget with 10⁷ messages that window is about
8,100 (§6.7), so about 33 level-2 replies can run ahead, roughly 23 MB.
Only an initiator whose window holds all 62,000 level-3 questions, which
takes a budget of about 2 GiB here, sends the whole 180 MB at once.
Windows are not exchanged and may differ (§5.2), so the responder still
prices all 256 replies.

When nearly every depth-2 node differs, the level-2 replies list about
every occupied depth-3 prefix, `2²⁴ · (1 − e^(−n/2²⁴))` of them, at 25
bytes each:

| n | Occupied depth-3 prefixes | Level-2 volume |
| --- | --- | --- |
| 10⁶ | 0.97·n | 24 MB |
| 10⁷ | 0.75·n | 188 MB |
| 10⁸ | nearly all 2²⁴ | 418 MB |

At 10⁷, the example's 95% dispute rate gives the 180 MB above.

Parking cannot shrink this term, because a single opening reply invited
every one of those replies. Only a protocol change that lets the asker
invite fewer questions at once could shrink it. Such a change would need
standalone questions that carry their own paths, and a re-derived
progress argument, so it costs more than it saves. This design accepts
the level-2 term and prices it. Exchanging windows in the greeting
(§10.1) would bound it by the peer's window.

### 6.5 Queued scopes, priced as a set

Pricing parking adds to every window's charge, which makes the model's
pessimism elsewhere more expensive. The design therefore sharpens two
places where the model knows less than the session does. The first is
the price of queued questions.

Today every queued question is charged for `C(d−1)` child references,
the most any single node plausibly has. But the queued questions at one
depth concern distinct nodes, so the per-set bound of §6.3 applies to
them exactly as it does to parking: q queued questions retain at most
`min(q·C(d−1), set bound, occupied(n, d))` references. This change is
independent of the socket, and it widens every window the model grants.

### 6.6 Pricing from the root comparison

The second refinement bounds how much the replicas differ. Each greeting
carries its sender's root listing, so both sides know, before the window
is sized, exactly which of the 256 root slots differ. Let `D` be the
number of messages held by one side and not the other, deletions
included. A root slot differs exactly when at least one of those `D`
messages falls under it. Under uniform hashing that is `D` balls thrown
into 256 bins, and all of them land within some `k` bins with
probability at most `binom(256, k) · (k/256)^D`. So when `k` slots
differ, at the model's tail of 2⁻⁴⁸, `D` is at most

```text
D_hi(k) = the least D with binom(256, k) · (k/256)^D ≤ 2⁻⁴⁸,   for k < 256,
        ≈ ⌈(48 + log₂ binom(256, k)) / log₂(256 / k)⌉.
```

| Differing root slots `k` | 1 | 8 | 32 | 83 | 128 | 177 | 224 | 251 | 255 | 256 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `D_hi(k)` | 7 | 20 | 62 | 171 | 300 | 511 | 953 | 2,848 | 9,918 | — |

The bound is tight where it matters: 10 actual differences give about 10
differing slots and `D_hi = 23`, and 100 give about 83 and
`D_hi = 171`. (The causal versions in the greetings cannot supply such a
bound, because no version bounds its event count from above; see
§10.2.)

The bound then applies at every depth. A question concerns a node whose
contents differ, and every such node contains at least one of the `D`
messages. Nodes at one depth are disjoint, so no level has more than `D`
questions (`S(d) ≤ D_hi`), no level parks more than `D` replies, and no
level carries listings for more than `D` children, which caps its
listing entries at `D_hi · C(d)`. At level 2 the count is exact up to
`k`: the opening asks one question for each differing slot the initiator
holds, and only the disputed ones, held by both sides, can return
listings.

At the default budget this changes nothing, because the size-only window
already exceeds any `D` below saturation. The bound matters at budgets
of tens of MiB with replicas of 10⁶ messages or more. There the
size-only model prices full divergence, and its fixed charges (§6.7)
alone drive the window to one slot. With one slot, each level sends one
parent's questions per round trip, so 100 scattered differences take on
the order of 100 round trips. Priced from the root comparison, the same
session gets a window as wide as `D_hi`, or as much of it as the budget
allows:

| Replicas | Differences | Budget | Window, size-only → root comparison |
| --- | --- | --- | --- |
| 10⁶ | 100 | 16 MiB | 1 → 171 |
| 10⁶ | 1,000 | 16 MiB | 1 → 117 |
| 10⁷ | 100 | 64 MiB | 1 → 171 |
| 10⁷ | 1,000 | 64 MiB | 1 → 1,051 |

These sessions are the steady state of a node that gossips with many
peers under a divided memory budget: large replicas, small per-session
budgets, and few differences per session.

The bound saturates above about 1,400 differences. All 256 root slots
then likely differ (the probability is about one in three at 1,400
differences and nine in ten at 2,000), the comparison says only that `D`
is large, and the size-only model prices the session as fully divergent.
Only a difference estimator in the greeting (§10.2) would cover such
sessions.

The capped populations set the window's capacities as well as its
price, and that is safe. Parking's capacity is computed from each
level's actual question-queue capacity (§5.2), so a session whose `D`
lands in the 2⁻⁴⁸ tail gets narrower queues than it could use. That
costs pipelining and some memory beyond the estimate, but never
correctness.

### 6.7 The budget that imposes no latency

The sizing guide models a session's slowdown, relative to a fully used
link, as `max(1, W / K)`. Here `W = BDP / (43 + m)` is the link's
bandwidth-delay product in messages, m is the mean encoded message size,
and 43 bytes is the calibrated per-message protocol overhead: the
constant the sizing guide uses, where a measurement between fully
divergent 10⁵-message sets gives about 42. A window narrower than `W`
slows the session in proportion, so a window of at least
`K* = min(W, max_d S(d))` imposes no additional latency. The second term
covers the case where the population, not the link, caps useful width.
The *threshold budget* is the charge at `K*`. Before this design that
charge is the existing scope charge, and after it the refined scope
charge plus parking. The figures below, like every modeled figure in
this note, come from
[`sizing-model.py`](sizing-model.py), a transcription of the window
model with this note's additions.

By depth, the charge falls into three bands. Near the root (d = 1, 2),
populations are tiny and fans are full, and level 2 contributes its
window-independent term (§6.4). When every root slot differs, the
level-2 replies can list every occupied depth-3 prefix at 25 bytes each:
about 24 MB at 10⁶ messages, 188 MB at 10⁷, and at most about 420 MB.
The saturated band runs from depth `log₂₅₆ n + 1` to
`log₂₅₆ n² + 1`; there `S(d) ≥ K*`, so every level is saturated. Past
that band, `S(d)` falls about 256-fold per level, and the tail is
negligible.

For two entirely different replicas of n messages each, with 100-byte
messages and the in-memory backend, the modeled threshold budgets,
before → after, are:

| n | In-rack (100 Gb/s, 50 µs; W ≈ 4,400) | Metro (10 Gb/s, 2 ms; W ≈ 17,500) | Long haul (1 Gb/s, 100 ms; W ≈ 87,400) |
| --- | --- | --- | --- |
| 10⁵ | 32 → 33 MiB | 111 → 64 MiB | 404 → 78 MiB |
| 10⁶ | 53 → 75 MiB | 189 → 172 MiB | 767 → 467 MiB |
| 10⁷ | 112 → 394 MiB | 429 → 797 MiB | 1.7 → 2.1 GiB |
| 10⁸ | 124 → 750 MiB | 481 → 1,713 MiB | 1.9 → 5.1 GiB |

The slot sizes are estimates (ρ ≈ 32 B, ε ≈ 25 B), and the
implementation computes the figures exactly. Up to about 10⁶ messages,
the set-priced scopes save more than parking costs on long links. From
10⁷ on, level 2 dominates and the threshold rises. At a fixed budget of
512 MiB, the window for 10⁶-message replicas widens from about 48,600 to
about 107,000, while for 10⁷ it narrows from about 20,900 to about
8,100.

Some charges do not shrink with the window: the model's existing
*fixed charges*, and now parking's `+ 257` slack at every level and the
level-2 term. The *floor charge* is the charge at a one-slot window.
When fixed charges alone exceed a budget, as level 2 does for large
replicas at small budgets, the window falls to one slot per level and
the estimate exceeds the budget. The budget's documentation already
states this minimum: even a zero budget keeps the buffering that
progress needs.

The default budget's tuning goal is to keep the *reference link*, 12.5
MB of bandwidth-delay product with 100-byte messages, fully busy for two
replicas of 10⁶ messages. The default becomes the exact threshold there,
rounded up to a power of two, and a committed test holds it to that
goal. The model puts the threshold at about 467 MiB, so the default
likely stays at 512 MiB.

Those are the fully divergent cases. Replicas that gossip regularly
differ in few messages, and the root comparison prices them accordingly.
For two 10⁶-message replicas differing in 10, 100, and 1,000 messages,
the long-haul threshold is about 1 MiB, 6 MiB, and 43 MiB.

### 6.8 What the budget does not count

Replica content, including content absorbed before commit, lies outside
the budget, and so do transport buffers. Removing the stream bundle
shrinks the transport buffers. To impose no latency, any stream that may
carry the bulk alone needs about one bandwidth-delay product of receive
window. With one TCP connection per stream, that is up to seventeen such
windows per direction, about 212 MB on the long-haul link. QUIC needs
its connection-level pool sized to eighteen per-stream windows, for the
seventeen data streams and the control stream. One socket needs one
window. In resident terms, parking partly moves memory out of kernel
buffers the budget never saw and into process memory that it now
prices.

## 7. Latency, loss, and the transport contract

### 7.1 What stays the same

Count a *dependent crossing* whenever a message needed to advance the
descent passes from one party to the other. The streaming walk descends
one level per dependent crossing over the bundle and over one socket
alike, with the same opening and closing exchanges. The one exception is
the removed opening batch: the initiator's exclusive root children
arrive one crossing later, which matters only between small replicas
(§2).

The logical payload is also unchanged. The frame grammar stays, stream
labels disappear, and the preamble, greeting, and closing items share
the socket with the frames.

### 7.2 Where bulk flows

Under uniform hashing, bulk flows in a band about two levels thick, and
the descent has only a few crossings left once bulk begins. Both facts
bound what one socket costs in time, and credits (§9) and deferred bulk
(§10.3) depend on them too.

A differing message is supplied at the first depth where the other side
holds nothing under its prefix. A depth-j prefix is occupied with
probability `1 − e^(−n/256ʲ)`. Each level divides the exponent by 256,
so that probability falls from nearly 1 to nearly 0 within one level: at
n = 10⁶ it is about 1 at depth 2, 0.058 at depth 3, and 0.0002 at
depth 4. About 94% of supplies fall at depth 3, and nearly all the rest
at depth 4. This note calls those levels the *band*. It is predictable
from the set sizes, and above it supplies are exponentially rare, so the
descent reaches the band undisturbed.

The deepest disputes sit near depth `log₂₅₆(D·n)` (§2) and the band near
`log₂₅₆ n`, so about `r ≈ log₂₅₆ D + 1` crossings remain once bulk
begins: two for a few hundred differences, and about three at a million.
Addresses are hashes of versions, so the tree's shape is uniform
whatever the messages are, and these figures hold for any workload.

### 7.3 Serialization behind bulk

When a thin reply becomes ready, the multiplexer puts it ahead of every
unwritten supply frame (§5.4). The reply can still wait for two things:
the frame being written, which is up to one supply run (about 15 ms at
1 Gb/s by default), and unsent bytes already in the kernel's send
buffer, which drains in order. This note calls the delay that remains,
which no user-level priority can remove, the *send-buffer residual*. It
costs elapsed time and never a dependent crossing.

To keep a link busy, a socket holds about one bandwidth-delay product of
sent, unacknowledged bytes. It can also hold unsent bytes, up to
whatever room its buffer has beyond that. A socket tuned to twice the
bandwidth-delay product therefore delays each thin reply on the critical
path by up to one round trip while bulk flows. The session loses time
only when the delayed descent outlasts the bulk, and then at most one
round trip for each crossing that remains once bulk begins: two or three
(§7.2). An untuned socket whose buffer is smaller than the
bandwidth-delay product never holds unsent bytes; its throughput is
capped instead.

Bounding the unsent bytes removes the effect. Where the platform
provides `TCP_NOTSENT_LOWAT`, setting it to about one frame does this;
Linux and macOS provide it, according to their documentation, which this
note has not verified. Elsewhere, setting `SO_SNDBUF` to about one
bandwidth-delay product plus one frame bounds the unsent bytes with the
whole buffer. The residual is then about two frames' transmission time:
about 29 ms at 1 Gb/s with the default run budget, and less with a
smaller one. The crate sees only the two halves of the connection, so
these settings are deployment guidance for the application.

On the receiving side, a thin frame behind bulk in the receive buffer
waits until the bulk ahead of it is decoded. That costs time only when
decoding is slower than the wire, and then decoding bounds the session
anyway. The stream bundle has neither effect, because the transport
interleaves streams packet by packet and a thin reply on its own stream
waits a packet or two. That is the bundle's one latency advantage on a
loss-free link.

### 7.4 Loss

On TCP, a lost segment delays every level until it is recovered,
including work that an independent QUIC stream could have advanced
meanwhile. A QUIC deployment of this design uses one bidirectional
stream and accepts the same coupling.

### 7.5 The transport contract

The transport contract shrinks. A session takes the two halves of one
reliable, ordered duplex byte stream whose directions progress
independently, with receiver-paced backpressure at any positive
capacity, and with end-of-stream or an error when the peer departs. A
successful session returns the halves. After an error the connection's
position is unknown, so the connection must be discarded.

## 8. Evidence

Each claim the argument rests on has a committed check that fails if the
claim stops holding. Backend independence (§5.3) is the exception: it is
a clause of the documented backend contract.

The count is metered. Test traces of the walk record each outgoing
reply's question count and each take, and a checker asserts, at every
event in every walk test, that outstanding questions stay within
`K(ℓ) + 257`. One fixture drives a level to exactly that number, which
shows that the meter counts. A second holds a level's consuming stage
until exactly that many replies are parked, which shows that parking's
capacity has no slack. The walk's trace checker already asserts the
count's premise, that a stage records one reply's questions before
handing on another.

The receive path is tested for stalls. Sessions over one in-memory
socket run under a scheduler that reports a stall whenever every task
waits. They run at the one-slot window, with a one-byte socket buffer
and adversarial poll orders, and must complete, so any new wait of the
receive path on the walk shows up as a stall. The fixture of §4, with
parking forced to one slot, must stall, which shows that the harness
detects this class of deadlock. At the derived capacity the same fixture
must complete and match the in-memory merge.

The bookkeeping capacities are tested at their boundaries. The
answer-record queue is tested at three capacities: one slot short of a
full fan stalls on a full-fan reply, a full fan completes, and
`256 + 1` never blocks. Parking's overflow check fails the session with
an error, and does not hang, when a malformed peer sends an unasked
reply.

The prices are checked against exact computation. The set statistic and
the `D_hi` table are checked against exact binomial and occupancy tails,
as the existing quantiles are. `sizing-model.py` reproduces every
modeled figure in this note, and the implementation's exact figures
replace them. A test holds the default budget to its tuning goal. The
window census tests, which report the sizing model's outputs, are
re-baselined from recorded output at the parent commit. One measurement
of actual peak parked memory, for 10⁶- and 10⁷-message replicas with
scattered differences, is recorded separately from model output.

These tests show that the implementation corresponds to the argument
over the executions they run; they do not prove it over all executions.

## 9. Explicit credits, for contrast

### 9.1 How credits would work

The alternative this design rejects is per-level flow control over the
one socket, as HTTP/2 and QUIC implement it. It would work as follows.

- Each side grants its peer a *credit* per level: a number of bytes the
  peer may send on that level before waiting.
- The sender's multiplexer writes a level's frame only when that level
  has credit for it. An encoder whose level is out of credit blocks, and
  so does the walk stage feeding it, which leaves unsent replies
  unproduced at the sender, as the bundle does.
- The receiver returns credit in a new control frame as its stage
  consumes. It must return credit at the take; returning it at decoding
  would let decoded replies accumulate exactly as they do in parking.
- Credit frames travel outside flow control and ahead of every data
  frame, so they never wait behind data.
- Every level's credit admits at least one whole frame, or else frames
  split across credit grants.
- The per-level credits sum to the connection's buffering. A shared pool
  smaller than that sum couples the levels again and reintroduces the
  deadlock, as today's transport contract already warns for QUIC's
  pooled flow control.

### 9.2 Memory and latency

With credits, the receiver's buffering is exactly the credit it grants:
deterministic, and independent of hashing and the model's quantiles. The
level-2 problem disappears, since the initiator cannot send more level-2
bytes than the responder has granted.

Credit returns a crossing after the take, so a level whose credit is
smaller than the bandwidth-delay product stalls once per round trip
while it carries bulk. How much credit a session needs to impose no
latency therefore depends on where bulk flows, which under uniform
hashing is the band of §7.2. Credit can be sized in two ways. Giving
every level a full bandwidth-delay product is safe whatever the tree's
shape, and costs 17 × 12.5 MB ≈ 212 MB per direction on the long-haul
link. Giving the band's levels a full bandwidth-delay product and every
other level one frame costs about 2 × 12.5 MB + 15 × 1.8 MB ≈ 52 MB. If
bulk then falls outside the predicted band, the affected level stalls a
round trip per credit window, which slows it without stopping it.

On the long-haul link, for entirely different replicas, the latency-free
budgets compare as follows, all with the refined scope charge:

| n | Parking | Credits, every level | Credits, band-targeted |
| --- | --- | --- | --- |
| 10⁵ | 78 MiB | 249 MiB | 96 MiB |
| 10⁶ | 467 MiB | 418 MiB | 265 MiB |
| 10⁷ | 2.1 GiB | 1.1 GiB | 0.96 GiB |
| 10⁸ | 5.1 GiB | 1.7 GiB | 1.6 GiB |

On memory, band-targeted credits beat parking from about 10⁶ messages
up, by roughly 2× at 10⁶ and 3× at 10⁸, because they avoid parking's
level-2 term, which grows with the set. Below 10⁶ the two are close. For
ordinary sessions, with few differences, both can be sized small from
the same root comparison.

### 9.3 Complexity, and the trade

Credits are a flow-control protocol inside the crate. They need
per-level accounting on both sides, a new control frame with its own
priority rule, a rule tying credit to frame sizes, and a new violation
for a peer that exceeds its credit. They also need a sizing policy that
knows the bandwidth-delay product, estimated at run time as HTTP/2
implementations do, and for the cheaper variant a prediction of the
band. Their deadlock argument inherits the walk's and adds a proof that
the credit loop is live: credit frames always flow, and every level's
credit admits a frame. Parking adds one queue capacity, one occupancy
check, and one moved publication. Its argument is the count of §5.1,
and it needs to know nothing about the link.

Neither design changes the dependent-crossing count. Both suffer the
send-buffer residual (§7.3), because the kernel sends bytes in the order
they were written, beneath any scheduling the crate does, and both
couple levels under TCP loss.

Credits give a deterministic bound which, sized to the band, is several
times tighter than parking's for large, heavily divergent replicas. Their
price is a flow-control protocol with its own liveness argument, and the
need to know the link and predict the band. Parking gives simplicity and
independence from the link. Its price is a statistical bound sized by
the workload, whose level-2 term grows with the set: at the default's
reference point of 10⁶ messages on the long-haul link, about 0.46 GiB
against 0.26 GiB for band-targeted credits. This design chooses parking,
and the memory figures above are the cost of that choice.

## 10. Follow-ons

Three extensions build on this design. Exchanging windows in the
greeting is worth pursuing. Richer greeting fields add little beyond a
difference estimator, and deferring bulk is not worth building.

### 10.1 Exchanging windows

The greeting does not carry a side's window, and §5.2 shows that
correctness does not need it. Carrying it would sharpen the price, by
more than level 2 alone suggests.

Every query in a reply parked at a receiver is a question the peer is
still waiting on. The peer handed the query on inside that reply, and
the question stays outstanding until the receiver answers it, which the
receiver does only after taking the reply from parking. By the peer's
own count (§5.1), the queries in the receiver's parked level-d replies
therefore number at most `K_peer(d + 1) + 257`. Queries are the only
reactions that carry listings, and listings hold most parked bytes;
matches and supplied handles are small. A known peer window therefore
bounds the listing volume parked at every level:

```text
parked entries(d) ≤ (K_peer(d + 1) + 257) · min(256, C(d))
```

At level 2 this turns the initiator's pacing of §6.4 into a price.

For entirely different replicas, with both peers on one budget and each
pricing parking by the other's window, `sizing-model.py` gives the
following, today → with exchanged windows:

| n | Window at 16 MiB | Window at 64 MiB | Window at 512 MiB | Floor charge | Long-haul threshold |
| --- | --- | --- | --- | --- | --- |
| 10⁵ | 650 → 853 | 17,133 → 26,522 | unchanged | 7.7 → 5.6 MiB | 78 → 76 MiB |
| 10⁶ | 1 → 382 | 3,012 → 4,736 | 107,053 → 123,098 | 29.9 → 6.5 MiB | 467 → 443 MiB |
| 10⁷ | 1 → 119 | 1 → 1,382 | 8,066 → 14,514 | 247 → 9.4 MiB | 2.1 → 2.0 GiB |
| 10⁸ | 1 → 95 | 1 → 1,125 | 1,171 → 11,588 | 418 → 10 MiB | 5.1 → 2.9 GiB |

The floor charge collapses, because the level-2 term stops growing with
the set and becomes proportional to the peer's window. That is what
rescues small budgets from the one-slot window. The exchange also helps
at any divergence, as long as the peer's window is narrow, whereas the
root comparison helps only while fewer than about 1,400 messages differ
(§6.6). The latency-free threshold barely moves until 10⁸, because a
wide window on both sides already pays for its width; the gain there is
mostly the level-2 term.

The bound helps only when the peer's window is narrow. Facing a peer at
the 512 MiB default, a 16 MiB side is still priced at one slot, because
the wide peer can have that many more questions outstanding. A simple
rule closes the gap: both sides use the smaller of the two windows. The
windows then act as a *static credit*, declared once in the greeting and
never updated, with no control frames and no knowledge of the link. The
narrow side gets the table's shared window (for example 1 → 119 at 10⁷
messages and 16 MiB), and the wide side gives up width it likely could
not use anyway, since each level's concurrency is already gated by what
the peer's questions feed it. That last claim is unmeasured.

The exchange has costs. The greeting must carry each side's budget
rather than its window, because the client sends its greeting before it
knows the server's set size and so cannot yet compute its window. With
both budgets and both set sizes in hand, each side computes the same
shared window: the largest width that fits the smaller budget, priced
against a peer of the same width. The sizing function then becomes
shared between the two sides. Two versions that size differently would
disagree about each other's windows, which misprices memory but never
breaks a session, because parking's capacity still comes from each
side's own queue (§5.2). And a side's window comes to depend on its
peer's configuration, so a peer with a small budget narrows the session.
Under the model of record, where peers are authenticated and follow the
protocol, that narrowing affects only throughput.

The exchange does not move the default. The default is sized for a
window wide enough for the reference link, and at that width the charge
is the price of the width itself. At 10⁶ messages, queued scopes cost
215 MiB and per-slot parking 252 MiB. Priced against an equally wide
peer, the exchange removes only 24 MiB of the parking, so the threshold
falls from 467 to 443 MiB, and both round up to a 512 MiB default. It
would halve a default aimed at larger replicas: from 4 GiB to 2 GiB for
a 10⁷-message target, and from 8 GiB to 4 GiB for 10⁸. Its gains lie in
the fixed charges, and so at small budgets.

This follow-on is worth pursuing once this design lands. It complements
the root comparison: that bound shrinks the price when the replicas
differ little, and this one when the peers' windows are narrow, as they
are for a node splitting its memory across many peers. It changes the
greeting, so it is a wire change with its own snapshots, but it adds no
messages and nothing to the progress argument.

### 10.2 What else the greeting could carry

The causal versions already in the greetings say four things. Equal
versions end the session. A version records sends and redactions alike,
so if A's version is below B's, B has seen every event A has. A then
holds nothing B has not seen: anything A holds that B lacks, B has
deleted, and the supply filter drops it, so bulk flows only from B to A.
`Version::lag` and `distance` measure, as an area, the history one side
has and the other lacks, and `min_ticks` gives a lower bound on the
number of events it contains. But no version bounds its event count from
above, as the `before` documentation states: an increment over an
interval can always be refined into concurrent increments over its
halves. Converting area into a count of messages would need the smallest
id share that has ever ticked, which no peer knows and no greeting
carries. Versions can therefore bound the work from below, but they
cannot bound memory.

Two further kinds of greeting field could add information, and they are
worth very different amounts.

An exchanged profile of the peer's tree would replace the model's
statistical fans with exact ones, and gains almost nothing. The cheapest
such field is each tree's *leaf depth*, the depth by which all of its
leaves are separated, which a tree can easily memoize. It does not bound
the descent. A dispute at depth d needs a message on each side sharing a
d-byte prefix, and one tree's own depth says nothing about prefixes
shared *across* trees: two replicas holding one message each, sharing
31 bytes, have minimal leaf depths but descend to depth 31. Under uniform
hashing the deepest disputes sit near `log₂₅₆(n_A·n_B)`, near but not
below each tree's own depth of about `log₂₅₆(n²/2)`.

What an exact leaf depth does give is small. Past the larger leaf depth,
every node on both sides holds exactly one leaf, so fans there are
exactly 1 where the model prices about 4 or 5. But memory cost
concentrates at levels 2 through 5, above any leaf depth, where fans are
wide and the quantiles are already close to the truth. Modeled with
`sizing-model.py`, exact deep fans change thresholds by under 1% and
windows by at most about 5% at 10⁵ to 10⁷ messages. The gain also
shrinks with size. The deep tail it removes costs roughly the same few
MB at any n, because each level divides the chance of a shared prefix by
256, while every other charge grows with n. The removed share of the
floor charge falls from about 2% at 10⁵ messages to 0.1% at 10⁷. Richer
profiles, such as the number of depth-3 prefixes under each root child,
fail for the same reason: they sharpen fans, and the fans are not where
the model is pessimistic.

The model's pessimism lies in the size of the difference, since it
prices two replicas as if they shared nothing. The root comparison
corrects that below saturation (§6.6). Above saturation, a difference
estimator carried in the greeting would correct it, such as the strata
estimator of Eppstein, Goodrich, Uyeda and Varghese (SIGCOMM 2011).

### 10.3 Deferring bulk

A supply is an ordinary reaction, carried inline in radix order. It
delays the rest of its reply, because stages take whole replies, and
every later reply at its level, because a level's frames are ordered. In
tree terms, it delays everything below it and to its right. The
multiplexer already sends supply runs only when no other frame is ready
(§5.4), but it cannot reorder frames within a level.

Only the descent has dependencies, so the fastest possible session takes
about `max(critical path of the descent, all bytes / bandwidth)`, and
any protocol that sends bulk only when no descent frame is ready
approaches that. With inline bulk, the worst case for arbitrary trees
approaches the *sum* of the two terms. A follow-on could remove that:
large supplies would become thin *promises* in their replies, and their
content would travel on a *bulk lane* that the multiplexer sends only
when no descent frame is ready. Under uniform hashing, though, inline
bulk costs about one round trip, and the follow-on is not worth
building.

Waiting behind bulk loses little time, because the wire is carrying
bytes the session must carry anyway. Say the descent reaches the band
(§7.2) at time `t`, and the critical path's next frame waits behind `x`
bytes of band supplies, out of the band's `X` bytes in that direction.
Many interleaved supplies can make that wait long. If `r` crossings of
one-way delay `δ` remain after the wait, the critical path finishes at
about `t + x / bandwidth + r·δ`, with `x ≤ X`. The best possible finish
is `t + max(X / bandwidth, r·δ)`, so the excess is at most `r·δ`. For
example, suppose the band holds 100 MB on a 1 Gb/s link (0.8 s), the
critical query waits behind 90 MB (0.72 s), and two 50 ms crossings
remain. The session finishes at 0.82 s against a best of 0.8 s. Even
with all 100 MB ahead of the query, it loses 0.1 s, the time of the two
crossings, and none of the 0.8 s of transfer. Waits in one direction
cannot add up to more than that direction's transfer time, however many
supplies there are, and the same holds when reading or absorbing bulk,
rather than the wire, limits the rate. With `r` at two or three (§7.2),
the excess is about one round trip.

The two directions can serialize, though. At the band, one side's
replies carry both its supplies and its *requests*: empty queries that
the other side answers with supplies of its own, one level down. A
request queued behind bulk delays the start of the reverse bulk, and the
reverse wire idles. Reactions go in the radix order of hashed prefixes,
so requests spread evenly through the forward bulk. The reverse bulk
starts about one crossing late and keeps pace, idling only where that
interleaving runs dry. A simulation in `sizing-model.py` measures the
idle time as a share of the larger direction's transfer time, for `N_b`
band supplies. Each cell gives the mean, with the worst run in
parentheses:

| `N_b` | Reverse bulk half the forward | Balanced | Reverse bulk twice the forward |
| --- | --- | --- | --- |
| 100 | 1.3% (15%) | 7.2% (25%) | 0.6% (5%) |
| 1,000 | 0.1% (1%) | 2.4% (8%) | 0.1% (0.6%) |
| 10,000 | 0.01% (0.1%) | 0.7% (2.2%) | 0.01% (0.04%) |

Balanced bulk behaves like a queue whose arrivals match its service
rate, so the reverse wire idles for about the largest deficit of a
random walk: `0.7 · √N_b · m / bandwidth` for supplies of about m bytes.
Unbalanced bulk hides the smaller direction almost entirely. These are
typical-case figures; the simulation gives no tail bound.

Message size decides whether this duplex term matters. A band supply
holds about `1 + D_supplier / n_other` messages. When the replicas are
of comparable size and differ in fewer messages than they hold, that is
about one message. Large supplies arise only when one side holds far
more differing messages than the other holds at all, as in a bootstrap,
and then the bulk is unbalanced and the duplex term vanishes. So, under
uniform hashing, small messages make the duplex term vanish: 200,000
differences of 100 bytes between 10⁶-message replicas, 0.23 s of bulk at
1 Gb/s, lose well under a millisecond to it. Large messages incur it
only when the bulk is balanced: 100 balanced 1 MB messages lose about
56 ms of 0.8 s on average, and up to about 200 ms.

Inline bulk therefore costs at most the crossings left below the band,
about one round trip, plus the duplex term for balanced bulk of large
messages. That is the same order as the send-buffer residual (§7.3),
which a socket option bounds, and because the tree's shape does not
depend on the messages (§7.2), it holds for any workload.

The follow-on would add a `Promise(radix)` reaction. A supply larger
than about one run would become a thin promise, and small supplies would
stay inline, since a node already knows its leaf count. Each promised
subtree would travel on the bulk lane as a group: a header naming its
prefix, its filtered leaf runs, and an end marker. Every leaf's path
follows from its version, so groups need no pairing and can arrive in
any order. The multiplexer would send bulk exactly when no descent frame
is ready, in promise order. The sender would keep a capped table of
unsent promises and fall back to inline supplies at the cap, so the cap
costs latency and never progress. The receiver would absorb each group
with a decoder that never waits on the walk, and file the finished
subtree by prefix; a promised child's resolution slot would stay pending
until its group arrived.

The follow-on also needs a new progress argument. Assemblers could now
wait on bulk, and the stages feeding them could wait on their resolution
queues. Bulk depends only on the sender's backend and on the socket
draining, and it flows whenever the descent pauses, so the argument is
short, but it is new. A resolution waiting on bulk would also hold its
level back to one window's width, which costs concurrency.

It would build on this design without undoing any of it. The pipe would
gain a lane and a route, the level decoders would shed their heaviest
work, and parking's count and price would not change. Removing the
opening shortcut, which ships bulk early, points the same way.

Against a gain of about one round trip, a new reaction, lane, and
progress argument are not worth building. The implementation plan's
step 5 measures real sessions against the bound, and only a measured
penalty well beyond a round trip would reopen the question. The plan
still keeps the multiplexer's priority classes general, which costs
nothing and leaves room for a bulk lane.
