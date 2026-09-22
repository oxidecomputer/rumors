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
the count and the receive path it permits, and §6 accounts for memory,
including what reducing the root level's share would take. Section 7
contrasts the design with explicit credits, §8 lists the remaining
costs, and §9 describes the evidence. The
[appendix](appendix-implementation-plan.md) is the implementation plan.

## 2. The conversation

Every message has an address: the hash of the causal version stamped on
it when it was sent. The 32-byte addresses determine a 256-ary radix
trie, one address byte per level. A node's children, at most 256, are its
*fan*. Storage compresses single-child chains, but the conversation still
descends one byte per level. Each interior node memoizes a 24-byte
*digest* of its subtree; the protocol assumes that distinct subtrees have
distinct digests, so equal digests mean equal subtrees.

Each side opens with a *greeting*: its causal version, its live-message
count, and a *listing* of the root's children as (radix byte, digest)
pairs. A version summarizes a replica's entire history of sends and
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
crossing early. Under uniform hashing a replica of N messages leaves a
given root slot empty with probability about e^(−N/256), so the batch is
nonempty essentially only between replicas below a few thousand
messages. It needs its own stream, decoder, and pairing rule, and over
one socket it creates a second instance of the deadlock in §4. Without
it, those children travel one crossing later, as ordinary supplies
answering the responder's empty queries. The greeting's root listing is a
different thing and stays: it is the level-1 question, it lets the
responder answer at once, and it needs no stream of its own.

Matching digests prune shared subtrees. The walk follows disputed
prefixes until it reaches the subtrees held by one side only. For
replicas differing in `D` of `N` messages, the expected depth of that
frontier is about `log₂₅₆(2·D·N)`: about five when `D = N = 10⁶`.

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

The first version of the protocol sends one complete level per message,
alternating sides. Exactly one message is in flight, and its receiver is
waiting for it, so one ordered byte stream per direction suffices. The
cost is aggregation: a message holds a whole level of disputes, can
approach the size of the set, and cannot depart until all of it is
ready. Each message still advances the descent by one level per network
crossing.

### 3.2 The streaming walk

The shipped protocol keeps the same exchange but sends one reply per
question instead of one message per level. Each side runs a *stage* at
each level where it asks questions. Stage ℓ takes one local question
record, then takes the matching reply, and processes its reactions in
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
default, the size of a maximally disputed reply); every other frame
carries at most one 256-entry listing, about 7 KB.

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
construction. On the wire, today's transport provides it: seventeen
independently flow-controlled streams per direction, one per level each
side replies at, plus the terminal leaf stream and the opening batch's
stream. Order within a stream is guaranteed; order across streams is
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
Both replicas hold root child 0, with digests that differ all the way
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

The same shape can occur at any depth, and the opening batch creates it
directly, since its bulk is written before the thin reply the responder
needs first.

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

A conforming peer answers only questions it has received, once each. So
every reply in transit, being decoded, or waiting for its stage answers a
distinct outstanding question, and the same bound applies to replies.
This is a count of replies, not bytes: a supply can carry a large
subtree, and §6 deals with size.

The count is a property of the asking side's walk alone. It needs neither
the peer's window nor any statistical assumption about either tree, and
it depends on nothing in the transport.

### 5.2 Parking

Call the queue of decoded replies between a level's decoder and its stage
the level's *parking*. Give it capacity

  `C(ℓ) = min(K(ℓ) + 257, 256^(ℓ−1))`.

Parking can fill, but no conforming arrival can find it already full:
that arrival would be one outstanding reply more than the count allows.
So the decoder checks occupancy before it parks a reply. A full parking
queue means the peer answered a question never asked, or a local premise
of the count has failed. Either way the session fails with an error that
says so; it never waits. The capacity is also as small as it can be: a
committed test drives one level to exactly `K(ℓ) + 257` parked replies,
so any smaller capacity would fail a conforming session.

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
| Decoder | the storage backend, absorbing supplied leaves | The backend progresses independently of the walk. |
| Decoder | room in parking | Never under conformance (§5.2); the decoder checks and fails instead of waiting. |

The local record deserves a word. The encoder publishes a reply's
question records just after handing the reply's last frame to the
multiplexer, and before that frame reaches the wire. A decoder waits for a
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
The queue needs `256 + 1` slots. When the stage takes a reply, the
encoder has consumed the records for every earlier reply's answers except
possibly the last, whose answer may still sit in the one-slot channel
between the walk and the encoder. The walk takes its next reply only
after handing on all of the current one's answers.

The other bookkeeping edges keep their existing arguments. On this path,
nothing the walk controls can stop the socket reader.

### 5.4 The sending side

Per-level encoders feed a *multiplexer* that owns the socket's write
half. It keeps each level's frames in order, and it serves every frame
that is not a supply run before any supply run, round-robin within each
class. Every frame is still served eventually, because a session's
traffic is finite. The preference exists for latency (§8): the thin
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
  leaves under any single depth-j node, and `S(d)` scopes in dispute at
  depth d. Each is a quantile at tail 2⁻⁴⁸ that holds for every node at
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

The charge for level d is the least of three bounds:

1. **Per reply:** `slots(d) · (2·C(d−1)·ρ + min(C(d−1)·C(d), L(d−1))·ε)`.
2. **Per level (deterministic):** parked replies at one level concern
   distinct nodes, whose subtrees are disjoint. Across the level their
   reactions number at most `2·occupied(n, d)` and their entries at most
   `occupied(n, d + 1)`.
3. **Per set (statistical):** the same disjointness, sharpened. The peer's
   leaves under any fixed set of m distinct depth-j nodes are
   Binomial(n, m/256ʲ). Take the union over every such set, all
   C(256ʲ, m) of them, where log₂ C(256ʲ, m) ≤ m·(8j + 2 − ⌊log₂ m⌋). The
   binomial quantile at tail 2^−(48 + that) then bounds the leaves under
   whichever m nodes the protocol happens to hold. With m = 1 it is
   exactly `L(j)`. With m = `slots(d)` and j = d − 1, it bounds the
   level's entries, and twice it bounds the reactions.

### 6.4 Two refinements the design adopts

Pricing parking honestly makes the model's pessimism expensive, so the
design also sharpens two places where the model knows less than it
could.

**Queued scopes, priced as a set.** Today every queued question is
charged for `C(d−1)` child references, the most any single node plausibly
has. The queued questions at one depth concern distinct nodes, so the
per-set bound applies to them exactly as it does to parking: `m` queued
questions retain at most `min(m·C(d−1), set bound, occupied(n, d))`
references. This change is independent of the socket; it widens every
window the model grants.

**Pricing from the root comparison.** Each greeting carries its sender's
root listing, so both sides know, before the window is sized, exactly
which of the 256 root slots differ. Let `D` be the number of messages
held by one side and not the other, deletions included. A root slot
differs exactly when at least one of those `D` messages falls under it.
Under uniform hashing that is `D` balls in 256 bins, and all of them land
within some `k` bins with probability at most `C(256, k) · (k/256)^D`.
So, at the model's tail of 2⁻⁴⁸:

```text
D ≤ D_hi(k) = ⌈(48 + log₂ C(256, k)) / log₂(256 / k)⌉,   for k < 256.
```

| Differing root slots `k` | 1 | 8 | 32 | 83 | 128 | 177 | 224 | 251 | 255 | 256 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `D_hi(k)` | 8 | 20 | 62 | 171 | 300 | 511 | 953 | 2,848 | 9,918 | — |

The bound is tight where it matters: 10 real differences give about 10
differing slots and `D_hi ≈ 23`; 100 give about 83 and `D_hi = 171`.

It then applies at every depth. A question concerns a node whose
contents differ, and every such node contains at least one of the `D`
messages. Nodes at one depth are disjoint, so:

- no level has more than `D` questions: `S(d) ≤ D_hi`;
- no level parks more than `D` replies;
- no level carries listings for more than `D` children, so at most
  `D_hi · C(d)` entries.

At level 2 the count is exact: `k` questions, of which only the disputed
slots can return listings.

Two limits apply:

- **Saturation.** When all 256 slots differ, which is likely once `D`
  exceeds about 1,400 (probability about one in three at 1,400, nine in
  ten at 2,000), the comparison says only that `D` is large, and the
  size-based model prices the session.
- **Pricing only.** The bound enters pricing, never a queue's capacity.
  Capacities stay the deterministic count of §5, so a session in the
  2⁻⁴⁸ tail uses more memory than estimated but never fails.

### 6.5 The budget that imposes no latency

The sizing guide models slowdown relative to a fully used link as
`max(1, W / K)`, where `W = BDP / (43 + m)` is the link's bandwidth-delay
product in messages, m the mean encoded message size, and 43 bytes the
measured per-message protocol overhead. A window of at least
`K* = min(W, max_d S(d))` therefore imposes no additional latency. The
second term covers the case where the population, not the link, caps
useful width. The *threshold budget* is the charge at `K*`: today's scope
charge before, and the refined scope charge plus parking after.

The depths fall into three bands:

- **Near the root (d = 1, 2),** populations are tiny and fans full. Level 2
  is the one term that does not shrink with the link: when every root
  slot differs, its replies can together list the whole depth-3
  frontier, about 25 bytes per message of the larger replica up to about
  10⁷ messages and approaching 420 MB beyond (§6.8).
- **The frontier band** runs from depth `log₂₅₆ n + 1` to
  `log₂₅₆ n² + 1`. There `S(d) ≥ K*`, so every level is saturated.
- **The tail** is negligible: `S(d)` falls about 256-fold per level past
  the frontier.

For two entirely different replicas of n messages each, 100-byte
messages, and the in-memory backend, the modeled threshold budgets,
before → after, are:

| n | In-rack (100 Gb/s, 50 µs; W ≈ 4,400) | Metro (10 Gb/s, 2 ms; W ≈ 17,500) | Long haul (1 Gb/s, 100 ms; W ≈ 87,400) |
| --- | --- | --- | --- |
| 10⁵ | 32 → 33 MiB | 111 → 64 MiB | 404 → 78 MiB |
| 10⁶ | 53 → 75 MiB | 189 → 172 MiB | 767 → 467 MiB |
| 10⁷ | 112 → 394 MiB | 429 → 797 MiB | 1.7 → 2.1 GiB |
| 10⁸ | 124 → 750 MiB | 481 → 1,713 MiB | 1.9 → 5.1 GiB |

These figures come from a transcription of the model with estimated slot
sizes (ρ ≈ 32 B, ε ≈ 25 B); the implementation computes them exactly.

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
(about 210 MB on the long-haul link); QUIC needs its connection-level pool
sized to eighteen per-stream windows. One socket needs one. So in resident
terms, parking partly moves memory from kernel buffers the budget never
saw into process memory it now prices.

### 6.7 What else a session could know

The ITC versions in the greetings add four things:

- **Equality:** equal versions end the session.
- **Containment:** if one version is below the other, one side has seen
  every send the other has, so the only difference in that direction is
  deletions it has made.
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

Two further gaps would need new greeting fields:

- **The peer's tree shape.** The model prices the peer's fans
  statistically. A peer could send an exact profile of its upper levels,
  for example the number of depth-3 prefixes under each root child, which
  would make the level-2 term exact.
- **The size of the difference once all 256 root slots differ.** A
  difference estimator, such as the strata estimator of Eppstein,
  Goodrich, Uyeda and Varghese (SIGCOMM 2011), bounds it where the root
  comparison cannot.

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
7 million entries: about 190 MB, almost all of it listing shared content.
The stream bundle kept most of those replies unproduced at the sender.

Nothing in parking can shrink this, because every one of those replies
was invited by a single opening reply. Shrinking it takes a protocol
change that lets the asker invite less at once. This design does not
make that change, but it would look like this:

- **A deferral reaction.** Alongside match, supply, and query, a reply
  can react to a differing child with *defer*: "we differ here, and I
  will ask later". It holds the child's position, so positional pairing
  is unchanged.
- **Standalone questions.** Later, the asker sends a question about each
  deferred child. It carries the child's prefix, since it has no position
  to pair with, and the asker's listing. Standalone questions at a level
  queue behind that level's other questions and are answered in order,
  so pairing stays positional within the level.
- **The answerer keeps a handle per deferred child,** captured when it
  compared the parent's children, so a standalone question costs no more
  backend work than the query it replaces.
- **The asker invites only what it can hold.** The "+256" term of the
  count, a whole reply's questions invited at once, becomes a per-level
  invitation cap `B` that the asker chooses. Level 2's parking falls from
  "the whole level" to `B` replies.

The costs:

- Each deferred subtree starts at least one crossing later.
- The wire gains two message kinds.
- The walk gains a deferred-question queue per level.
- The progress argument needs re-deriving. Today a parent resolution is
  published only after the work that fills its slots has been launched;
  a deferred child's work launches later. So deferred questions must be
  issued by work that never waits behind the resolution they fill, or
  the stage could block on its resolution queue while the assembler
  waits for a deferred child that the stage has yet to ask about.

In effect, deferral turns a question into an explicit, receiver-issued
credit expressed in the protocol's own vocabulary. That is the subject of
the next section.

## 7. Explicit credits, for contrast

The alternative this design rejects is per-level flow control over the
one socket, as HTTP/2 and QUIC implement it. This section describes that
design concretely enough to compare.

**How it would work.**
- Each side grants its peer a *credit* per level: a number of bytes the
  peer may send on that level before waiting.
- The sender's multiplexer writes a level's frame only when that level
  has credit for it. An encoder whose level is out of credit blocks, and
  so does the walk stage feeding it, which leaves unsent replies
  unproduced at the sender, as the bundle does.
- The receiver returns credit in a new control frame as its stage
  consumes. It must return credit at the *take*, not at decoding, or
  decoded replies accumulate exactly as they do in parking.
- Credit frames must never wait behind data. They travel outside flow
  control and ahead of every data frame.
- Every level's credit must admit at least one whole frame, or frames
  must be splittable across credit grants.

**Memory.** The receiver's buffering is exactly the sum of the credits
it grants: deterministic, independent of set size, hashing, and the
model's quantiles. The level-2 problem disappears, since the initiator
cannot send more level-2 bytes than the responder has granted.

**Latency.** Credit is consumed as data arrives and returned a crossing
after the consuming stage takes it. A level whose credit is smaller than
the bandwidth-delay product stalls once per round trip while it carries
bulk. So to impose no latency, every level that may carry bulk alone
needs credit of at least one bandwidth-delay product. Sharing a
connection-wide pool does not reduce this. A pool smaller than the sum of
the per-level credits couples the levels again, which is the deadlock
this design exists to avoid. The transport contract's pooled-flow-control
rule already says as much for QUIC.

So credits that impose no latency cost up to seventeen bandwidth-delay
products per direction: about 212 MB on the long-haul link. That is
fixed, whatever the set size. Parking needs no knowledge of the link:
the receiver accepts everything it invited, and the budget alone sets
how much it invites.

On the long-haul link, for entirely different replicas, the latency-free
budgets compare as follows. Both use the refined scope charge. The
credit column adds 17 bandwidth-delay products; smaller credits would
trade memory for per-round-trip stalls.

| n | Parking | Explicit credits |
| --- | --- | --- |
| 10⁵ | 78 MiB | 249 MiB |
| 10⁶ | 467 MiB | 418 MiB |
| 10⁷ | 2.1 GiB | 1.1 GiB |
| 10⁸ | 5.1 GiB | 1.7 GiB |

For ordinary sessions, with few differences, parking is priced at a few
MiB (§6.5). A credit design's buffers could be sized down in the same
way, from the same root comparison.

**Complexity.** Credits are a flow-control protocol inside the crate:
- per-level accounting on both sides;
- a new control frame, with its own priority rule;
- a rule tying credit to frame sizes;
- a new violation (a peer exceeding its credit);
- a sizing policy, which in practice means estimating the bandwidth-delay
  product at run time, as HTTP/2 implementations do.

The deadlock argument inherits the walk's, plus a proof that the credit
loop is live: credit frames always flow, and every level's credit admits
a frame. Parking adds one queue capacity, one occupancy check, and one
publication move, and its argument is the count in §5.1.

**What neither changes.** Both keep the dependent-crossing count. Both
suffer the send-buffer residual of §8, since the kernel drains in order
beneath any user-level scheduling, and both couple levels under TCP loss.

**Summary.** Credits buy a deterministic, link-sized memory bound. That
bound is tighter than parking's for large, heavily divergent replicas,
and looser for small ones. The price is a flow-control protocol with its
own liveness argument, and a need to know the link. Parking buys
simplicity and link-independence, at the price of a statistical,
workload-sized bound whose level-2 term grows with the set. Deferred
questions (§6.8) sit between the two: they bound level 2 using the
protocol's own questions as credit, with no separate control channel.

## 8. Costs

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
already in the kernel's send buffer, which drains in order. To keep a link
busy, a socket holds about one bandwidth-delay product of sent,
unacknowledged bytes. It can also hold unsent bytes, up to whatever room
its buffer has beyond that. A socket tuned to twice the bandwidth-delay
product therefore delays each thin reply on the critical path by up to
one round trip while bulk is flowing. The session loses time only when
the delayed descent outlasts the bulk; the worst case is about as many
round trips as descent levels remain once bulk begins, usually one or
two. An untuned socket whose buffer is below the bandwidth-delay product
never holds unsent bytes; its throughput is capped instead.

Bounding the unsent bytes removes the effect. `TCP_NOTSENT_LOWAT` set to
about one frame does this where the platform provides it (Linux and macOS
do). Elsewhere, setting `SO_SNDBUF` to about one bandwidth-delay product
plus one frame bounds the unsent bytes with the whole buffer. The residual
is then about two frames' transmission time, about 29 ms at 1 Gb/s with
the default run budget and less with a smaller one. The crate sees only
the two halves of the connection, so this is deployment guidance, not
code. Platform support here is taken from documentation and is confirmed
when that guidance is written.

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

## 9. Evidence

The argument has four load-bearing claims, and each gets a committed
check that fails if the claim ever stops holding:

- **The count.** Test traces of the walk record each outgoing reply's
  question count and each take. A checker asserts, at every event and in
  every walk test, that outstanding questions stay within `K(ℓ) + 257`.
  One fixture drives a level to exactly that number, which shows both
  that the meter counts and that the capacity is not slack. The premise
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
  law pinned from both sides: one slot short of a full fan stalls on a
  full-fan reply, and `256 + 1` never blocks. Parking's overflow check
  fires, as an error rather than a hang, when a malformed peer sends an
  unasked reply.
- **The prices.**
  - The set statistic and the `D_hi` table are checked against exact
    binomial and occupancy tails, as the existing quantiles are.
  - A test holds the default budget to its tuning goal.
  - The window census is re-baselined from recorded output at the parent
    commit.
  - One measurement of actual peak parked memory, for 10⁶- and
    10⁷-message replicas with scattered differences, is recorded
    separately from model output.

These tests show that the implementation corresponds to the argument;
they do not prove it over all executions.
