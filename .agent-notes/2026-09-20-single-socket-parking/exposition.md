# Reconciliation over one socket

## 1. Summary

`rumors` reconciles two replicas of a message set by walking a hash tree
from the root downward and exchanging only what differs. The protocol
streams that walk: several tree levels are in flight at once, each
progressing as its own inputs arrive. Today it needs seventeen
independently flow-controlled streams in each direction, so that one
level waiting on its consumer never stops another level's traffic. QUIC
provides such streams natively; over TCP they cost a connection each.

This design runs the same protocol over one ordered duplex byte stream.
The logical messages, their order within each level, and the number of
dependent network crossings are unchanged. Nothing is added to the wire:
no credits, no window advertisements, no sender-side inference about the
receiver. The design rests on one count:

> At each level, the number of replies travelling toward a receiver that
> its consuming stage has not yet taken is bounded by the receiver's own
> configuration: its question-queue width plus 257.

A receiver that reserves that many slots per level can accept every reply
it has invited without waiting for the stage that will consume it. With that
room, one piece of bookkeeping moved, and one shortcut removed, decoding
never waits on the walk. The socket reader therefore never blocks on the
walk, and the walk's existing progress argument carries over unchanged.

The costs are these:

- **Memory.** Replies that the stream bundle held back at the sender now
  wait, decoded, at the receiver, and the session budget must price them.
  The design also sharpens the pricing in two places, and the net effect
  depends on scale:
  - For two entirely different 10⁶-message replicas, the budget that keeps
    a 12.5 MB bandwidth-delay link fully busy falls from about 0.75 GiB to
    about 0.46 GiB.
  - At 10⁷ messages it rises from about 1.7 to about 2.1 GiB.
  - Replicas that differ in a few hundred messages are priced at a few
    MiB.
- **Serialization.** A small, urgent frame can wait behind a large frame
  already committed to the socket. This costs elapsed time, never a
  round trip, and a socket option bounds it.
- **Loss.** On TCP, one lost segment delays every level.

Sections 2 and 3 describe the protocol and the progress property it
already has. Section 4 shows why one socket naively deadlocks, §5 derives
the count and the receive path it permits, §6 accounts for memory, and
§7 contrasts the design with explicit credits. Section 8 examines a
follow-on that separates bulk from the descent and finds it not worth
building. Section 9 lists the remaining costs, and §10 describes the
evidence. The
[appendix](appendix-implementation-plan.md) is the implementation plan.

## 2. The conversation

Every message has an address: the hash of the causal version stamped on
it when it was sent. The 32-byte addresses determine a 256-ary radix
trie, one address byte per level. A node's children, at most 256, are its
*fan*. Storage compresses single-child chains, but the conversation still
descends one byte per level. Each interior node memoizes a 24-byte
*digest* of its subtree; the protocol assumes that distinct subtrees have
distinct digests, so equal digests mean equal subtrees.

Each side opens with a *greeting*: its causal version (an interval tree
clock, ITC), its live-message count, and a *listing* of the root's
children as (radix byte, digest) pairs. A version summarizes a replica's entire history of sends and
redactions, so equal versions mean equal sets, and the session ends
there.

Otherwise the sides exchange questions and replies. A *question* about a
node lists that node's children. A *reply* reacts to each listed child in
order, and adds any children the replier holds that the listing omits:

- **match**: my digest for this child equals yours;
- **supply**: you lack this child; here is its subtree, filtered against
  your version so that messages you have seen and deleted stay deleted;
- **query**: our digests differ; here is my listing of this child's
  children. If I lack the child entirely, the listing is empty: send me
  all of it.

A query is itself the next question, one level down, so every reply both
answers one side and asks the other to continue. A question or reply is
*at level ℓ* when the children it lists or reacts to sit at depth ℓ.
The initiator's greeting listing is the question at level 1. The sides
alternate: the responder replies at odd levels, the initiator at even
levels, and the final exchange supplies individual leaves.

For example, suppose A asks B about a node whose children, in A's
listing, are `10` and `20`. B holds the same `10`, holds a different
`20`, and also holds `30`, which A lacks. B's reply is `Match`,
`Query(B's children of 20)`, `Supply(30, subtree)`. The first two
reactions refer to A's listing by position; the supply names its radix.
B's query is now a question to A, one level down.

Today's protocol adds one shortcut, which this design removes. After the
greetings cross, the initiator already knows which of its root children
the responder lacks, and ships them unasked as an *opening batch*, one
crossing early. Under uniform hashing a replica of n messages leaves a
given root slot empty with probability about e^(−n/256), so the batch is
nonempty essentially only between replicas below a few thousand
messages. It needs its own stream, decoder, and pairing rule, and over
one socket it creates a second instance of the deadlock in §4. Without
it, those children travel one crossing later, as ordinary supplies
answering the responder's empty queries. The greeting's root listing is a
different thing and stays: it is the level-1 question, it lets the
responder answer at once, and it needs no stream of its own.

Matching digests prune shared subtrees. The walk follows disputed
prefixes until it reaches subtrees held by one side only. For replicas
of about n messages that differ in `D`, the deepest disputes sit near
depth `log₂₅₆(D·n)`: about five when `D = n = 10⁶`.

Two properties of the conversation carry the rest of this note:

- **Each reply answers exactly one question.** Matches and queries refer
  to the question's listing by position, and replies carry no path, so
  the *k*-th reply at a level answers the *k*-th question there. The
  asker therefore keeps a local *record* of each question it asks until
  the reply arrives.
- **Only supplies carry content of unbounded size.** A query lists at most
  256 children. A supply can carry an arbitrarily large subtree.

## 3. From a level at a time to a streaming walk

### 3.1 A level at a time

A protocol could send one complete level per message, alternating
sides. Exactly one message is in flight, and its receiver is
waiting for it, so one ordered byte stream per direction suffices. The
cost is aggregation: a message holds a whole level of disputes, can
approach the size of the set, and cannot depart until all of it is
ready. Each message still advances the descent by one level per network
crossing. The streaming walk below keeps that crossing count and removes
the aggregation.

### 3.2 The streaming walk

The protocol keeps that exchange but sends one reply per question
instead of one message per level. Each side runs a *stage* at each level
where it asks questions. The node a question is about is its *scope*.
Stage ℓ takes one local question record, then takes the matching reply
(an event this note calls the *take*), and processes its reactions in
order: matches and supplies settle children; each query asks the stage
to reconcile one child, which produces an outgoing reply at level ℓ + 1
carrying questions at level ℓ + 2.

For each outgoing reply, a stage follows a fixed publication order:

1. hand the reply toward the wire;
2. publish a *resolution* saying which of the child's children are
   settled and which await deeper work;
3. record the reply's questions, one by one, in the queue feeding stage
   ℓ + 2.

It records all of one reply's questions before handing on another reply.
An *assembler* per level rebuilds subtrees from resolutions, filling each
pending slot with the result a deeper assembler returns.

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
frame, each tagged with its level. A supply's leaves travel in runs sized
by a byte budget both sides agree on in the greeting (about 1.8 MB by
default, the size of a maximally disputed reply). Every other frame, a
match, a query, or a reply's end, carries at most one 256-entry listing,
about 7 KB. This note calls those frames *thin*, and a reply made only of
them a thin reply.

### 3.3 The window

The question queue at level ℓ holds the records needed to interpret
replies there. Its capacity, the *window* `K(ℓ)`, limits how many
disputes a level can have in flight: wider keeps a long, fast link busy;
narrower retains less state. The session derives the window from a byte
budget (512 MiB by default today), the set sizes in the greetings, and a
uniform-hashing model of how many disputes each level can hold, `S(ℓ)`.
It chooses the widest affordable `K` and gives each level
`K(ℓ) = max(1, min(K, S(ℓ)))`. Section 6 describes that pricing.

### 3.4 The progress property

The streaming walk is already deadlock-free, under one premise about its
channels. Every wait in the walk is for the next item of one specific
queue, and every queue's producer produces its items in order. The
publication order above makes one slot per question and resolution queue
enough: a stage blocked recording a question has already handed its
reply toward the wire, and a stage blocked publishing a resolution sits
behind an older resolution whose dependent work is already under way.
The assembler return queues hold one full fan. The argument is written at
the head of the walk's module (`streaming/materialized.rs`), and a Lean
development checks a model of it.

The premise is **independence**: a full queue stalls only its own
producer, never delivery on another queue. In one process that holds by
construction. On the wire, today's transport provides it with seventeen
independently flow-controlled streams per direction. The responder's
seventeen are its sixteen odd levels plus the leaves it supplies at the
end. The initiator's are its fifteen even levels, its final leaf
requests, and the opening batch. Order within a stream is guaranteed; order across streams is
not.

This design keeps the walk and its argument, and replaces that transport
with one socket. What must be shown is that sharing the socket cannot
make one level's blocked consumer stop another level's delivery.

## 4. One socket, and the deadlock

A socket delivers bytes in the order the sender wrote them, which need
not be the order in which the receiver's stages consume them. If the
reader delivering the head frame must wait until that frame's stage
accepts it, the reader cannot reach a later frame that would let the
stage accept it.

A committed test fixture produces the smallest such shape, at the root.
Its trees also carry root children that only the responder holds, which
play no part in the cycle and are left out here. Both replicas hold root
child 0, with digests that differ all the way
down to one leaf. The initiator also holds six root children, 1 through
6, that the responder lacks. Take every queue at one slot, including the
queue of decoded replies waiting for their stage:

1. The responder's opening reply at level 1 asks seven level-2 questions:
   a query listing its children of 0, and six empty queries for 1–6.
2. The initiator's stage 1 answers them in order at level 2: a thin reply
   that queries deeper under 0, then six supplies, S1–S6, which reach the
   socket in that order.
3. The responder's stage 2 takes the thin reply. It sends a level-3
   reply, publishes the resolution for 0 (pending deeper work), and
   records its level-4 question. The assembler takes that resolution and
   waits for the deeper result.
4. Stage 2 takes S1 and publishes its resolution, filling the one-slot
   resolution queue, since the assembler is still waiting on 0. It takes
   S2 and blocks publishing the next resolution.
5. S3 fills the decoded-reply queue; the decoder holds S4, waiting; the
   socket reader has handed S5 to the decoder's input and holds S6,
   waiting.
6. Behind S6 on the socket is the initiator's level-4 reply, the one the
   level-4 question from step 3 asked for. Stage 4 needs it to finish
   0's subtree; only then can the assembler accept S1's resolution, free
   stage 2, and let the supplies drain. The socket reader waits on
   exactly the progress it is preventing.

The same shape can occur at any depth. The opening batch creates it
directly: its bulk is written before any level-2 reply, including the
thin one the responder's descent needs first.

There are three ways out:

- **Explicit credits:** the receiver tells the sender how much each level
  can accept. This is a multiplexed flow-control protocol of its own, as
  in HTTP/2 or QUIC.
- **Inferred credits:** the sender infers consumption from the peer's
  later questions. This adds bookkeeping and learns of progress a
  crossing late.
- **Receive-side buffering:** the receiver drains every reply whether or
  not its stage is ready. This removes the cycle, and the next section
  shows that the buffer it needs is bounded.

## 5. Replies are invited

### 5.1 The count

A reply exists only because its receiver asked the question it answers.
So count questions from the asker's side. Say a question at level ℓ is
*outstanding* from the moment the stage that asks it hands its carrying
reply toward the wire until the stage that consumes it takes the matching
reply. Taking the reply is the event that counts; dequeuing the local
record comes one step earlier, and finishing the reply's processing
comes later.

For ℓ ≥ 3 the asking stage is stage ℓ − 2; for ℓ = 2 it is the
responder's opening. By §3.2, the asking stage hands on one reply,
carrying at most 256 questions, then records all of them before handing
on another. The consuming stage holds at most one dequeued record while
it waits for that record's reply. Every outstanding question is therefore
in exactly one of three places:

```text
  K(ℓ)   recorded in the question queue, not yet dequeued
+    1   dequeued by stage ℓ, its reply not yet taken
+  256   handed toward the wire in the asking stage's current reply,
         not yet recorded
```

At most `K(ℓ) + 257` questions at level ℓ are outstanding. A level-ℓ
question concerns a node at depth ℓ − 1, of which there are at most
`256^(ℓ−1)`, so level 1 has one outstanding question (the greeting) and
level 2 at most 256.

A *conforming* peer, one that follows the protocol, answers only
questions it has received, once each. So
every reply in transit, being decoded, or waiting for its stage answers a
distinct outstanding question, and the same bound applies to replies.
This is a count of replies, not bytes: a supply can carry a large
subtree, and §6 deals with size.

The count is a property of the asking side's walk alone. It needs neither
the peer's window nor any statistical assumption about either tree, and
it depends on nothing in the transport.

### 5.2 Parking

Call the queue of decoded replies between a level's decoder and its stage
the level's *parking*. Its capacity is computed from the level's actual
question-queue capacity `K(ℓ)`, whatever that is:

  `cap(ℓ) = min(K(ℓ) + 257, 256^(ℓ−1))`.

Correctness rests only on that link: however the window chooses `K(ℓ)`,
parking has room for everything the count allows.

**The two sides' windows may differ.** Each side sizes its window from
its own budget, and the greeting never exchanges it. That is safe because
a reply travels only in answer to its receiver's own question. The
replies in flight toward a side are bounded by that side's window, and
its parking is sized from the same number. The peer's window never
enters.

Suppose one peer has a far wider window. It asks more questions at once,
so more answers flow toward it, and they park in its own parking, sized
by its own window. On the narrow side:
- The wide peer's questions arrive only inside replies to the narrow
  side's questions, at most 256 per reply, so the narrow side's count and
  price already cover them.
- The narrow side's outgoing answers never park at the narrow side; they
  drain toward the peer's socket reader, which never waits.

A mismatch changes only how much each side asks at once: throughput,
never progress. In this protocol, asking a question grants credit for
exactly one reply, and each receiver issues its own.

Parking can fill, but no conforming arrival can find it already full:
that arrival would be one outstanding reply more than the count allows.
So the decoder checks occupancy before it parks a reply. A full parking
queue means the peer answered a question never asked, or a local premise
of the count has failed. Either way the session fails with an error that
says so; it never waits. The capacity is also as small as it can be: a
committed test holds a level's consuming stage until exactly
`K(ℓ) + 257` replies are parked, so any smaller capacity would fail a
conforming session.

### 5.3 A receive path that never waits on the walk

Room is not enough if decoding waits on the walk for anything else. With
the stream bundle replaced by one socket, the receive path is a *demux*
that reads frames in socket order and hands each to its level's decoder,
and one decoder per level that rebuilds replies and parks them. Here is
every wait on that path:

| Waiter | Waits for | Why the wait ends without the walk |
| --- | --- | --- |
| Demux | the socket | The peer writes independently. |
| Demux | its level's decoder to accept a frame (one-slot handoff) | The decoder is only finishing its current frame. |
| Decoder | the local record of the question its next reply answers | See below. |
| Decoder | the storage backend, absorbing supplied leaves | The backend contract requires it (below). |
| Decoder | room in parking | Never under conformance (§5.2); the decoder checks and fails instead of waiting. |

The local record deserves a word. The encoder publishes a reply's
question records just after handing the reply's last frame to the
multiplexer's channel, without waiting for the multiplexer to write it,
so before that frame reaches the wire. A decoder waits for a
record only when the record queue is empty. If an answer to one of those
questions has arrived, the encoder is already past the reply's last
frame, and with the queue empty its publication cannot wait. So the
decoder waits only for its own side's encoder to be scheduled. The order
is the walk's own rule, wire before internal publication, applied
unchanged.

One piece of bookkeeping does move. A reply the walk receives can carry
questions of the peer's, and the encoder that sends the walk's answers
needs, for each answer, the record of the question it answers. Today the
decoder publishes those records into a window-sized queue right after
handing the reply on; with replies parked, that queue could fill while
the decoder has replies still to park. So publication moves to the take:
when the stage takes a reply, the reply's answer records are derived
from it and published, and only then is the reply handed to the stage.
The queue needs `256 + 1` slots. The encoder takes an answer's record
from this queue *before* it takes the answer itself. The walk takes its
next reply only after handing on all of the current one's answers, and
the channel between the walk and the encoder holds one answer. So when
the stage takes a reply, every earlier answer's record has been consumed
except possibly one: the record for the answer still sitting in that
channel. One record left over plus up to 256 new ones fits in 257 slots.
At 256 slots the take could wait briefly for the encoder, and below 256
it can deadlock.

The storage backend is the one shared resource. The walk uses it too, so
decoding waits on the walk if a walk task can hold something a decoder
needs, such as a lock, a transaction, or a slot in a bounded pool,
while that task is blocked on a queue. The backend contract must
therefore require that backend operations complete without waiting on
any other session work. The in-memory backend meets this by
construction: its nodes are immutable and shared, and it takes no locks.
A persistent backend must meet it explicitly.

The other bookkeeping edges keep their existing arguments. On this path,
nothing the walk controls can stop the socket reader.



### 5.4 The sending side

Per-level encoders feed a *multiplexer* that owns the socket's write
half. It keeps each level's frames in order, and it serves every frame
that is not a supply run before any supply run, round-robin within each
class. Every frame is still served eventually, because a session's
traffic is finite. The preference exists for latency (§9): the thin
replies that advance the descent do not queue behind bulk that is not
yet written. The sender can wait for bandwidth, and encoders can wait for
the multiplexer, but no receiving stage's consumption order can stop the
socket from draining, so the cycle of §4 cannot form, and the walk's
progress property applies over one socket.

## 6. Memory

### 6.1 What a parked reply holds

Parked replies are decoded, but they do not hold content. Supplied leaves
are absorbed into the storage backend as they arrive, and a parked supply
holds only a handle to the absorbed subtree. The session budget covers
working state, not replica content. Content absorbed by a session that
later fails is uncommitted; the backend reclaims it as it reclaims any
uncommitted node.

A parked reply to a question about a node P at depth ℓ − 1 holds its
reactions, one per child of P either side holds, and the listings in its
queries: at most 256 entries per query, 25 bytes each in memory. Parking
also keeps the one record of the question the reply answers, and not the
records for the reply's own questions, which are derived when the stage
takes it (§5.3).

### 6.2 How the window prices memory

The window model knows the two set sizes, `A` and `B`, and uses
`n = max(A, B)`. For each depth it uses two kinds of bound:

- **Deterministic:** a node has at most 256 children, and at most
  `occupied(n, j) = min(256ʲ, n)` prefixes at depth j are occupied.
- **Statistical, under uniform hashing:** `C(j)` children and `L(j)`
  leaves under any single depth-j node, and `S(d)` scopes (questions)
  that can be in dispute at depth d. Each is a quantile at tail 2⁻⁴⁸ that holds for every node at
  its depth simultaneously, and a union bound over all of them keeps the
  session's failure probability below 2⁻⁴⁰. Knowing only set sizes,
  `S(d)` prices the two sets as if they were entirely different.

The charge assumes every window slot is full at once and every slot's
contents are at their quantile: a high-probability worst case, not an
expectation. The window is the largest `K` whose charge fits the budget.
Crossing the tail threatens neither correctness nor progress; a session
that crosses it uses more memory than estimated.

### 6.3 The price of parking

At level d, parking holds at most `slots(d) = min(K(d) + 257, 256^(d−1))`
replies. One reply to a question about P holds at most `2·C(d−1)`
reactions and at most `min(C(d−1)·C(d), L(d−1))` listing entries. The
second term holds because every listed grandchild prefix contains at
least one of the replier's leaves under P. Let ρ be the bytes per parked
reaction and ε the bytes per listing entry.

For each depth d, the charge is the least of three bounds:

1. **Per reply:** `slots(d) · (2·C(d−1)·ρ + min(C(d−1)·C(d), L(d−1))·ε)`.
2. **Per level (deterministic):** parked replies at one level concern
   distinct nodes, whose subtrees are disjoint. Across the level, their
   reactions number at most `2·occupied(n, d)` and their entries at most
   `occupied(n, d + 1)`.
3. **Per set (statistical):** the same disjointness, sharpened.
   - The peer's leaves under any fixed set of q distinct depth-j nodes are
     Binomial(n, q/256ʲ).
   - Taking the union over every such set costs `log₂ binom(256ʲ, q)`
     bits, and `log₂ binom(N, q) ≤ q·log₂(eN/q)` gives at most
     `q·(8j + 2 − ⌊log₂ q⌋)`.
   - The binomial quantile at tail 2^−(48 + that) then bounds the leaves
     under whichever q nodes the protocol happens to hold. With q = 1 it
     is exactly `L(j)`.
   - Applied with q = `slots(d)` and j = d − 1, it bounds the level's
     entries, and twice it bounds the reactions.

### 6.4 Two refinements the design adopts

Pricing parking honestly makes the model's pessimism expensive, so the
design also sharpens two places where the model knows less than it
could.

**Queued scopes, priced as a set.** Today every queued question is
charged for `C(d−1)` child references, the most any single node plausibly
has. The queued questions at one depth concern distinct nodes, so the
per-set bound applies to them exactly as it does to parking: q queued
questions retain at most `min(q·C(d−1), set bound, occupied(n, d))`
references. This change is independent of the socket; it widens every
window the model grants.

**Pricing from the root comparison.** Each greeting carries its sender's
root listing, so both sides know, before the window is sized, exactly
which of the 256 root slots differ. Let `D` be the number of messages
held by one side and not the other, deletions included. A root slot
differs exactly when at least one of those `D` messages falls under it.
Under uniform hashing that is `D` balls in 256 bins, and all of them land
within some `k` bins with probability at most `binom(256, k) · (k/256)^D`.
So, at the model's tail of 2⁻⁴⁸, `D` is at most

```text
D_hi(k) = the least D with binom(256, k) · (k/256)^D ≤ 2⁻⁴⁸,   for k < 256,
        ≈ ⌈(48 + log₂ binom(256, k)) / log₂(256 / k)⌉.
```

| Differing root slots `k` | 1 | 8 | 32 | 83 | 128 | 177 | 224 | 251 | 255 | 256 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `D_hi(k)` | 7 | 20 | 62 | 171 | 300 | 511 | 953 | 2,848 | 9,918 | — |

The bound is tight where it matters: 10 real differences give about 10
differing slots and `D_hi = 23`; 100 give about 83 and `D_hi = 171`.

It then applies at every depth. A question concerns a node whose
contents differ, and every such node contains at least one of the `D`
messages. Nodes at one depth are disjoint, so:

- no level has more than `D` questions: `S(d) ≤ D_hi`;
- no level parks more than `D` replies;
- no level carries listings for more than `D` children, so at most
  `D_hi · C(d)` entries.

At level 2 the count is exact: `k` questions, of which only the disputed
slots can return listings.

**Where it pays.** At the default budget, nothing changes: the
size-only window already exceeds any `D` below saturation. The bound
matters at budgets of tens of MiB with replicas of 10⁶ messages or more.
There the size-only model prices full divergence, and its fixed charges
alone drive the window to one slot. One slot sends each level one
parent's questions per round trip, so 100 scattered differences take on
the order of 100 round trips. Priced from the root comparison, the same
session gets a window covering every dispute:

| Replicas | Differences | Budget | Window, size-only → root comparison |
| --- | --- | --- | --- |
| 10⁶ | 100 | 16 MiB | 1 → 171 |
| 10⁶ | 1,000 | 16 MiB | 1 → 117 |
| 10⁷ | 100 | 64 MiB | 1 → 171 |
| 10⁷ | 1,000 | 64 MiB | 1 → 1,051 |

That is the steady state of a node that gossips with many peers under a
divided memory budget: large replicas, small per-session budgets, and few
differences between sessions. Above about 1,400 differences the bound
saturates, and such a session is priced as fully divergent again; only a
difference estimator in the greeting (§6.7) would cover it.

The capped populations feed the window's capacities as well as its
price, and that is safe. Parking's capacity is computed from each level's
actual question-queue capacity (§5.2), so a session whose `D` lands in
the 2⁻⁴⁸ tail gets narrower queues than it could use. That costs
pipelining and some memory beyond the estimate, but never correctness.
When all 256 slots differ, which is likely once `D` exceeds about 1,400
(probability about one in three at 1,400, nine in ten at 2,000), the
comparison says only that `D` is large, and the size-based model prices
the session.

### 6.5 The budget that imposes no latency

The sizing guide models slowdown relative to a fully used link as
`max(1, W / K)`, where `W = BDP / (43 + m)` is the link's bandwidth-delay
product in messages, m the mean encoded message size, and 43 bytes the
calibrated per-message protocol overhead (the constant the sizing guide
uses; a measurement between fully divergent 10⁵-message sets gives about
42). A window of at least
`K* = min(W, max_d S(d))` therefore imposes no additional latency. The
second term covers the case where the population, not the link, caps
useful width. The *threshold budget* is the charge at `K*`: today's scope
charge before, and the refined scope charge plus parking after. The
figures below, like every modeled figure in this note, come from
[`sizing-model.py`](sizing-model.py), a transcription of the window
model with this note's additions.

The depths fall into three bands:

- **Near the root (d = 1, 2),** populations are tiny and fans full. Level 2
  is the one term that shrinks with neither the link nor the budget: its
  parking holds up to 256 replies at any window. When every root slot
  differs, those replies can list every occupied depth-3 prefix, about
  25 bytes per message of the larger replica up to about 10⁷ messages,
  and approaching 420 MB beyond (§6.8).
- **The saturated band** runs from depth `log₂₅₆ n + 1` to
  `log₂₅₆ n² + 1`. There `S(d) ≥ K*`, so every level is saturated.
- **The tail** is negligible: `S(d)` falls about 256-fold per level past
  the saturated band.

For two entirely different replicas of n messages each, 100-byte
messages, and the in-memory backend, the modeled threshold budgets,
before → after, are:

| n | In-rack (100 Gb/s, 50 µs; W ≈ 4,400) | Metro (10 Gb/s, 2 ms; W ≈ 17,500) | Long haul (1 Gb/s, 100 ms; W ≈ 87,400) |
| --- | --- | --- | --- |
| 10⁵ | 32 → 33 MiB | 111 → 64 MiB | 404 → 78 MiB |
| 10⁶ | 53 → 75 MiB | 189 → 172 MiB | 767 → 467 MiB |
| 10⁷ | 112 → 394 MiB | 429 → 797 MiB | 1.7 → 2.1 GiB |
| 10⁸ | 124 → 750 MiB | 481 → 1,713 MiB | 1.9 → 5.1 GiB |

The slot sizes are estimates (ρ ≈ 32 B, ε ≈ 25 B); the implementation
computes the figures exactly. When fixed charges alone exceed a budget,
as level 2 does for large replicas at small budgets, the window falls to
one slot per level and the estimate exceeds the budget. That is the
setter's documented progress floor.

- **Up to about 10⁶ messages,** the set-priced scopes save more than
  parking costs on long links.
- **From 10⁷ on,** level 2 dominates and the threshold rises.

At a fixed budget of 512 MiB, the window for 10⁶-message replicas widens
from about 48,600 to about 107,000, while for 10⁷ it narrows from about
20,900 to about 8,100.

The default budget's tuning goal is to keep the reference link, 12.5 MB of
bandwidth-delay with 100-byte messages, fully busy for two replicas of
10⁶ messages. The default becomes the exact threshold there, rounded up
to a power of two, and a committed test holds it to that goal. The model
puts the threshold at about 467 MiB, so the default likely stays at
512 MiB.

Those are the fully divergent cases. Replicas that gossip regularly
differ in few messages, and the root comparison prices them accordingly.
For two 10⁶-message replicas differing in 10, 100, and 1,000 messages,
the long-haul threshold is about 1 MiB, 6 MiB, and 43 MiB.

### 6.6 What the budget does not count

Replica content, including content absorbed before commit, is outside
the budget, as are transport buffers. Removing the bundle shrinks the
latter. To impose no latency, any stream that may carry the bulk alone
needs about one bandwidth-delay product of receive window. With one TCP
connection per stream, that is up to seventeen such windows per direction
(about 212 MB on the long-haul link). QUIC needs its connection-level pool
sized to eighteen per-stream windows: the seventeen data streams and the
control stream. One socket needs one. So in resident
terms, parking partly moves memory from kernel buffers the budget never
saw into process memory it now prices.

### 6.7 What else a session could know

The ITC versions in the greetings add four things:

- **Equality:** equal versions end the session.
- **Containment:** a version records sends and redactions alike, so if
  A's version is below B's, B has seen every event A has. A then holds
  nothing B has not seen: anything A holds that B lacks, B has deleted,
  and the supply filter drops it. Bulk flows only from B to A.
- **The size of the difference as an area:** `Version::lag` and
  `distance` measure the history one side has and the other lacks, and
  `min_ticks` gives a lower bound on the events it contains.
- **Not an upper bound on events.** The `before` documentation is
  explicit that no version bounds its event count from above: an
  increment over an interval can always be refined into concurrent
  increments over its halves. Converting area into a count of messages
  would need the smallest id share that has ever ticked, which no peer
  knows and no greeting carries.

So versions can predict a lower bound on the work, but cannot bound
memory.

Two kinds of greeting field could add more, and they are worth very
different amounts.

**The peer's tree shape buys almost nothing.** An exchanged profile of
the peer's tree would replace the model's statistical fans with exact
ones. The cheapest such field is each tree's *leaf depth*: the depth by
which all of its leaves are separated, easily memoized in the tree.
- **It does not bound the descent.** A dispute at depth d needs a message
  on each side sharing a d-byte prefix, and one tree's own depth says
  nothing about prefixes shared *across* trees. Two replicas holding one
  message each, sharing 31 bytes, have minimal leaf depths but descend to
  depth 31. Under uniform hashing the deepest disputes sit near
  `log₂₅₆(n_A·n_B)`, near but not below each tree's own depth of about
  `log₂₅₆(n²/2)`.
- **What it does give is small.** Past the larger leaf depth, every
  node on both sides holds exactly one leaf, so fans there are exactly
  1 where the model prices about 4 or 5. But memory cost concentrates at
  levels 2 through 5, above any leaf depth, where fans are genuinely wide
  and the quantiles are already close to the truth. Modeled with
  `sizing-model.py`, exact deep fans change thresholds and windows by
  about 1% or less at 10⁵ to 10⁷ messages.

Richer profiles, such as the number of depth-3 prefixes under each root
child, fail for the same reason: they sharpen fans, and fans are not
where the pessimism is.

**The size of the difference is where the pessimism is.** The model
prices two replicas as if they shared nothing. The root comparison
corrects that below saturation (§6.4). Above it, a difference estimator
would, such as the strata estimator of Eppstein, Goodrich, Uyeda and
Varghese (SIGCOMM 2011), carried in the greeting.

### 6.8 Level 2, and what shrinking it would take

When every root slot differs, level 2 is the one place where parking can
hold a whole level. The responder's opening reply asks about every
differing root child at once. The initiator's stage 1 answers all of
those questions without waiting for anything further. Each answer, about
one root child, lists the children of every disputed node beneath it at
depth 2.

This is a realistic case, not only a worst case. Take two 10⁷-message
replicas that share nearly everything, where one side holds 200,000 extra
messages scattered by hash. About 95% of the 65,536 depth-2 nodes are
disputed, each with about 115 children, so the level-2 replies list about
7 million entries: about 180 MB, almost all of it listing shared content.
The stream bundle kept most of those replies unproduced at the sender.

Nothing in parking can shrink this, because every one of those replies
was invited by a single opening reply. Only a protocol change that lets
the asker invite fewer questions at once could shrink it, and that
change costs more than it saves: it would need standalone questions that
carry their own paths, and a re-derived progress argument. This design
accepts the level-2 term and prices it.

## 7. Explicit credits, for contrast

The alternative this design rejects is per-level flow control over the
one socket, as HTTP/2 and QUIC implement it.

### 7.1 How it would work

- Each side grants its peer a *credit* per level: a number of bytes the
  peer may send on that level before waiting.
- The sender's multiplexer writes a level's frame only when that level has
  credit for it. An encoder whose level is out of credit blocks, and so
  does the walk stage feeding it, which leaves unsent replies unproduced
  at the sender, as the bundle does.
- The receiver returns credit in a new control frame as its stage
  consumes. It must return credit at the take, not at decoding, or decoded
  replies accumulate exactly as they do in parking.
- Credit frames never wait behind data: they travel outside flow control
  and ahead of every data frame.
- Every level's credit admits at least one whole frame, or frames split
  across credit grants.
- The sum of the per-level credits is the connection's buffering. A
  shared pool smaller than that sum couples the levels again, which is the
  deadlock this design exists to avoid, as the transport contract's
  pooled-flow-control rule already says for QUIC.

### 7.2 What it costs and buys

**Memory.** The receiver's buffering is exactly the credit it grants:
deterministic, and independent of hashing and the model's quantiles. The
level-2 problem disappears, since the initiator cannot send more level-2
bytes than the responder has granted.

**Latency.** Credit returns a crossing after the take. A level whose
credit is smaller than the bandwidth-delay product stalls once per round
trip while it carries bulk. How much credit a session needs to impose no
latency depends on where bulk flows. §8.2 shows that under uniform
hashing it flows in a band about two levels thick, predictable from the
set sizes. That leaves two ways to size credit:

- **Every level a full bandwidth-delay product.** This is safe whatever
  the tree's shape. It costs 17 × 12.5 MB ≈ 212 MB per direction on the
  long-haul link.
- **The band's levels a full bandwidth-delay product, and every other
  level one frame.** This costs about 2 × 12.5 MB + 15 × 1.8 MB ≈ 52 MB.
  If bulk falls outside the predicted band, the affected level stalls a
  round trip per credit window: slower, never stuck.

On the long-haul link, for entirely different replicas, the latency-free
budgets compare as follows (all with the refined scope charge):

| n | Parking | Credits, every level | Credits, band-targeted |
| --- | --- | --- | --- |
| 10⁵ | 78 MiB | 249 MiB | 96 MiB |
| 10⁶ | 467 MiB | 418 MiB | 265 MiB |
| 10⁷ | 2.1 GiB | 1.1 GiB | 0.96 GiB |
| 10⁸ | 5.1 GiB | 1.7 GiB | 1.6 GiB |

So on memory, band-targeted credits beat parking from about 10⁶ messages
up, by roughly 2× at 10⁶ and 3× at 10⁸. Parking's level-2 term, which
grows with the set, is what they avoid. Below 10⁶ the two are close. For
ordinary sessions, with few differences, both can be sized small from the
same root comparison.

**Complexity.** Credits are a flow-control protocol inside the crate:
- per-level accounting on both sides;
- a new control frame with its own priority rule;
- a rule tying credit to frame sizes;
- a new violation, a peer exceeding its credit;
- a sizing policy that needs the bandwidth-delay product, estimated at
  run time as HTTP/2 implementations do, plus the band prediction for the
  cheaper variant.

The deadlock argument inherits the walk's, plus a proof that the credit
loop is live: credit frames always flow, and every level's credit admits
a frame. Parking adds one queue capacity, one occupancy check, and one
publication move. Its argument is the count in §5.1, and it needs to know
nothing about the link.

**What neither changes.** Both keep the dependent-crossing count. Both
suffer the send-buffer residual of §9, since the kernel sends bytes in
the order they were written, beneath any scheduling the crate does, and
both couple levels under TCP loss.

### 7.3 The trade

Credits buy a deterministic bound. Sized to the band, that bound is
several times tighter than parking's for large, heavily divergent
replicas. The price is a flow-control protocol with its own liveness
argument, and a need to know the link and predict the band. Parking buys
simplicity and link-independence. Its price is a statistical,
workload-sized bound whose level-2 term grows with the set: at the
default budget, about 0.46 GiB against 0.26 GiB for band-targeted credits
at 10⁶ messages. This design takes parking's side of that trade; the
memory figures above are the cost of doing so.

## 8. Deferring bulk: a follow-on, not recommended

### 8.1 Where bulk costs time

The *descent* finds out what differs: matches, queries, and replies, all
thin. The *bulk*, the supplied subtrees, transfers it. Only the descent
has dependencies, so the fastest possible session takes about
`max(critical path of the descent, all bytes / bandwidth)`. Any protocol
that sends bulk only when no descent frame is ready approaches that.

Today a supply is an ordinary reaction, carried inline in radix order.
It delays the rest of its reply, because stages take whole replies, and
every later reply at its level, because a level's frames are ordered. In
tree terms, it delays everything below it and to its right. This
design's multiplexer already sends supply runs only when no other frame
is ready, but it cannot reorder frames within a level. For arbitrary
trees, the worst case approaches the *sum* of the two terms.

A follow-on could remove that. Large supplies would become thin
*promises* in their replies, and their content would travel on a bulk
lane the multiplexer sends only when no descent frame is ready. This
section shows why that is not worth building.

### 8.2 How much delay inline bulk adds, under uniform hashing

**Bulk sits in one band.** A differing message is supplied at the first
depth where the other side holds nothing under its prefix. A depth-j
prefix is occupied with probability `1 − e^(−n/256ʲ)`. Each level divides
the exponent by 256, so that probability falls from nearly 1 to nearly 0
within one level: at n = 10⁶ it is about 1 at depth 2, 0.058 at depth 3,
and 0.0002 at depth 4. About 94% of supplies fall at depth 3 and nearly
all the rest at depth 4. Above the band, supplies are exponentially rare,
so the descent reaches the band undisturbed.

**Waiting behind bulk is not lost time.** Say the descent reaches the band
at time `t`, and the critical path's next frame waits behind `x` bytes of
band supplies. Many interleaved supplies can make that wait long. But the
wire is carrying bytes the session must carry anyway. If `r` crossings of
one-way delay `δ` remain after the wait, the critical path finishes at
about `t + x / bandwidth + r·δ`, with `x ≤ B`. The best possible finish
is `t + max(B / bandwidth, r·δ)`, so the excess is at most `r·δ`.

For example: the band holds 100 MB on a 1 Gb/s link (0.8 s), the critical
query waits behind 90 MB (0.72 s), and two 50 ms crossings remain. It
finishes at 0.82 s against a best of 0.8 s. Even with all 100 MB ahead of
it, 0.1 s is lost: the two crossings, not the 0.8 s.

Waits in one direction cannot add up to more than that direction's
transfer time, however many supplies there are. The same holds when
reading or absorbing bulk, rather than the wire, limits the rate. Since
the deepest disputes sit near `log₂₅₆(D·n)` and the band near `log₂₅₆ n`,
`r ≈ log₂₅₆ D + 1`: two crossings for a few hundred differences, about
three at a million.

**The two directions can serialize.** At the band, one side's replies
carry both its supplies and its *requests*: empty queries that the other
side answers with supplies of its own, one level down. A request queued
behind bulk delays the start of the reverse bulk, and the reverse wire
idles. Reactions go in the radix order of hashed prefixes, so requests
spread evenly through the forward bulk, and the reverse bulk starts about
one crossing late and keeps pace. It idles only where that interleaving
runs dry.

A simulation in `sizing-model.py` measures the idle time as a share of
the larger direction's transfer time, for `N_b` band supplies. Each cell
gives the mean, with the worst run in parentheses:

| `N_b` | Reverse bulk half the forward | Balanced | Reverse bulk twice the forward |
| --- | --- | --- | --- |
| 100 | 1.3% (15%) | 7.2% (25%) | 0.6% (5%) |
| 1,000 | 0.1% (1%) | 2.4% (8%) | 0.1% (0.6%) |
| 10,000 | 0.01% (0.1%) | 0.7% (2.2%) | 0.01% (0.04%) |

Balanced bulk behaves like a queue whose arrivals match its service rate,
so the reverse wire idles for about the largest deficit of a random walk:
`0.7 · √N_b · m / bandwidth` for supplies of about m bytes. Unbalanced
bulk hides the smaller direction almost entirely. These are typical-case
figures, not tail bounds.

**Message size decides the duplex term.** A band supply holds about
`1 + D_supplier / n_other` messages. When the replicas are of comparable
size and differ in fewer messages than they hold, that is about one
message. Large supplies arise only when one side holds far more differing
messages than the other holds at all, as in a bootstrap. Then the bulk is
unbalanced and the duplex term vanishes. So, under uniform hashing:

- **Small messages:** the duplex term vanishes. 200,000 differences of
  100 bytes between 10⁶-message replicas, 0.23 s of bulk at 1 Gb/s, lose
  well under a millisecond to it.
- **Large messages:** the duplex term appears only when the bulk is
  balanced. 100 balanced 1 MB messages lose about 56 ms of 0.8 s on
  average, and up to about 200 ms.

**The payoff.** Inline bulk costs at most the crossings left below the
band, about one round trip, plus the duplex term for balanced bulk of
large messages. That is the same order as the send-buffer residual (§9),
which a socket option bounds. It holds for any workload: addresses are
hashes of versions, so the tree's shape is uniform whatever the messages
are.

### 8.3 What the follow-on would take

- **A promise reaction.** A supply larger than about one run becomes a
  thin `Promise(radix)`. Small supplies stay inline; a node already knows
  its leaf count.
- **A bulk lane.** Each promised subtree travels as a group: a header
  naming its prefix, its filtered leaf runs, and an end marker. Every
  leaf's path follows from its version, so groups need no pairing and can
  arrive in any order.
- **A scheduling rule.** Send bulk exactly when no descent frame is ready,
  in promise order.
- **Tables on both sides.** The sender keeps a table of unsent promises,
  capped; at the cap it falls back to inline supplies, so the cap costs
  latency, never progress. The receiver absorbs each group with a decoder
  that never waits on the walk, and files the finished subtree by prefix.
  A promised child's resolution slot is pending until its group arrives.
- **A new progress argument.** Assemblers can now wait on bulk, and the
  stages feeding them on their resolution queues. Bulk depends only on
  the sender's backend and on the socket draining, and it flows whenever
  the descent pauses. The argument is short, but it is new.
- **A concurrency cost.** A resolution waiting on bulk holds its level
  back to one window's width.

It would build on this plan without undoing any of it. The pipe would
gain a lane and a route, and the level decoders would shed their heaviest
work. Parking's count and price would not change. Removing the opening
shortcut, which shipped bulk *early*, points the same way.

Against a payoff of about one round trip, a new reaction, lane, and
progress argument are not worth building. The plan's measurement step
(step 5 of the appendix) checks real sessions against the bound; only a
measured penalty well beyond a round trip would reopen the question. The
plan still asks for general multiplexer priority classes, which cost
nothing and keep the option open.

## 9. Costs

**Dependent crossings are unchanged, with one exception.** Count a
crossing whenever a message needed to advance the descent passes from
one party to the other. The streaming walk descends one level per
dependent crossing over the bundle and over one socket alike, with the
same opening and closing exchanges. The exception is the removed opening
batch: the initiator's exclusive root children arrive one crossing later,
which matters only between small replicas (§2).

**The logical payload is unchanged.** The frame grammar stays; stream
labels disappear, and the preamble, greeting, and closing items share the
socket with the frames.

**Serialization costs elapsed time, never crossings.** When a thin reply
becomes ready, the multiplexer puts it ahead of every unwritten supply
frame. It can still wait for two things: the frame being written (up to
one supply run, about 15 ms at 1 Gb/s by default) and unsent bytes
already in the kernel's send buffer, which drains in order. This note
calls that remaining delay, which no user-level priority can remove, the
*send-buffer residual*.

To keep a link busy, a socket holds about one bandwidth-delay product of
sent, unacknowledged bytes. It can also hold unsent bytes, up to whatever
room its buffer has beyond that. A socket tuned to twice the
bandwidth-delay product therefore delays each thin reply on the critical
path by up to one round trip while bulk is flowing. The session loses
time only when the delayed descent outlasts the bulk. At worst it loses
one round trip for each crossing that remains once bulk begins, and
§8.2 shows that two or three remain. An untuned socket whose buffer is
below the bandwidth-delay product never holds unsent bytes; its
throughput is capped instead.

Bounding the unsent bytes removes the effect. `TCP_NOTSENT_LOWAT` set to
about one frame does this where the platform provides it (Linux and macOS
do). Elsewhere, setting `SO_SNDBUF` to about one bandwidth-delay product
plus one frame bounds the unsent bytes with the whole buffer. The residual
is then about two frames' transmission time, about 29 ms at 1 Gb/s with
the default run budget and less with a smaller one. The crate sees only
the two halves of the connection, so this is deployment guidance, not
code.

On the receiving side, a thin frame behind bulk in the receive buffer
waits until the bulk ahead of it is decoded. That costs time only when
decoding is slower than the wire, and then the session is bound by
decoding anyway. The stream bundle had neither effect: the transport
interleaves streams packet by packet, so a thin reply on its own stream
waits a packet or two. That is its one latency advantage on a loss-free
link.

**Loss couples levels.** On TCP, a lost segment delays every level until
it is recovered, including work that an independent QUIC stream could
have advanced meanwhile. A QUIC deployment of this design uses one
bidirectional stream and accepts the same coupling.

**The transport contract shrinks.** A session takes the two halves of one
reliable, ordered duplex byte stream whose directions progress
independently, with receiver-paced backpressure at any positive capacity
and end-of-stream or an error when the peer departs. Successful sessions
return the halves. After an error the connection's position is unknown,
so it must be discarded.

## 10. Evidence

The argument has four load-bearing claims, and each gets a committed
check that fails if the claim ever stops holding. A fifth premise, backend
independence, is a documented contract clause rather than a test:

- **The count.** Test traces of the walk record each outgoing reply's
  question count and each take. A checker asserts, at every event and in
  every walk test, that outstanding questions stay within `K(ℓ) + 257`.
  One fixture drives a level to exactly that number, which shows that the
  meter counts. A second holds a level's consuming stage until exactly
  that many replies are parked, which shows that parking's capacity is
  not slack. The premise
  that a stage records one reply's questions before handing on another is
  already asserted by the walk's trace checker.
- **The receive path.** Sessions over one in-memory socket run under a
  scheduler that reports a stall whenever every task waits. They run at
  the one-slot window, a one-byte socket buffer, and adversarial poll
  orders, and must complete. Any new wait of the receive path on the walk
  shows up as a stall. The fixture of §4 with parking forced to one slot
  must stall, which shows the harness detects this class of deadlock, and
  at the derived capacity it must complete and match the in-memory merge.
- **The bookkeeping capacities.** The answer-record queue has a capacity
  law at three points: one slot short of a full fan stalls on a full-fan
  reply, a full fan completes, and `256 + 1` never blocks. Parking's overflow check
  fires, as an error rather than a hang, when a malformed peer sends an
  unasked reply.
- **The prices.**
  - The set statistic and the `D_hi` table are checked against exact
    binomial and occupancy tails, as the existing quantiles are.
  - `sizing-model.py` reproduces every modeled figure in this note, and
    the implementation's exact figures replace them.
  - A test holds the default budget to its tuning goal.
  - The window census is re-baselined from recorded output at the parent
    commit.
  - One measurement of actual peak parked memory, for 10⁶- and
    10⁷-message replicas with scattered differences, is recorded
    separately from model output.

These tests show that the implementation corresponds to the argument;
they do not prove it over all executions.
