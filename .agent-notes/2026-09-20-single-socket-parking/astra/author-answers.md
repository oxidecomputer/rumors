# Author's answers to the review's factual questions

These are facts from the codebase and its history that the reviewer could not check. Use them to resolve the review items they bear on; do not contradict them. Where an item is a judgment call, the author's ruling is stated.

## V1's real shape (review item 1, faith point 9)

V1 was a *bidirectional alternating* protocol: one message per level, the two sides alternating, each message carrying a whole level's reactions and the next level's listings, built in full before it shipped. Its own documentation counted "≈ ½·log₂₅₆(2·D·N) exchanges after the opening", each exchange being two messages that descend two levels. So V1 and V2 have the same message shape and the same number of dependent crossings per level. The document's "each level costs a full round trip", "2(L + 1)", and "roughly half of V1's" are wrong and must go. V2's actual gains over V1: no per-level barrier (a reply at level ℓ + 1 departs the moment its own question's reply arrives, while the rest of level ℓ is still in flight, so transmission and computation overlap across levels), and bounded memory (no whole-level message). The dependent-crossing count is the same.

## Levels, compression, leaves (items 7, 8, 32, faith point 1)

- A hop is one address byte. The descent pops one prefix byte per level, and path compression does not shorten it: a compressed run is traversed byte by byte like any other. Compression is storage, not conversation.
- Held interior nodes have at least two children (that is what compression guarantees), so a genuine listing of an interior node is never empty. Leaves have no children. At the leaf level the empty query has one more meaning: it requests that individual leaf. An empty tree's greeting listing is empty and means "send everything".
- Streams per direction, seventeen: the responder speaks at levels 1, 3, …, 31 (sixteen) plus the leaf exchange; the initiator speaks the opening batch (supplies of root children, at level 1), levels 2, 4, …, 30 (fifteen), plus the leaf exchange. The leaf exchange is the descent's end: the initiator's last replies ask for individual leaves it lacks (empty queries at the leaf level), and the responder supplies them; each side has one stream for it.

## The greeting's base case (item 19)

The greeting is the one question at level 1. The responder's opening answers it with one reply at level 1. At most one reply at level 1 is ever outstanding toward the initiator.

## Decoder independence is a change, not a fact (items 2, 9; faith point 5)

Two things in today's receiver couple decoding to the walk, and both move: (a) the decoder publishes the next-level scopes (bookkeeping for the peer's questions inside a decoded reply) into a bounded queue, which can block it; that publish moves to the moment the stage takes the reply, and the queue is sized one fan plus one, argued at the constructor; (b) the opening batch's decoder is today driven by the stage that consumes it; it gets its own task and a fan-deep queue of decoded root children. With both moves the decoder depends only on the storage backend. The exposition must state these two moves rather than assert independence, and §1's "no queue is added and no edge is re-plumbed" must become: the same queues, two of them widened; one publish moved later on its own edge; one decoder given its own task and buffer; no new edge in the dataflow.

## Tightness (items 5, 7; faith point 7)

Withdraw the claim that one slot fewer deadlocks. Keep two separate facts: the bound is tight as an occupancy bound (a constructed run reaches exactly K(ℓ) + 257 parked replies), and the wedge at parking depth one is the negative control that deadlocks. Do not claim necessity of the capacity.

## Memory accounting (items 6, 17, 18, 21, 37, 38; faith point 8)

- The count of parked replies is absolute. Their byte size is a statistical estimate under uniform hashing, exactly as the window's sizing already is (tail probability per estimate 2⁻⁴⁸, union under 2⁻⁴⁰ per session); the structural worst case is one fan of reactions each with one fan of listing entries.
- Replica content, including supplied content absorbed into the backend before commit, is outside the session's working-memory budget, as it is today; a failed session discards it (a persistent backend reclaims it like any uncommitted node).
- The fixed floor per level is min(256 + 1, 256^(ℓ−1)) parked replies (questions at level ℓ are about nodes at level ℓ − 1, of which at most 256^(ℓ−1) exist), priced at that level's estimated reply size.
- Digests are 24 bytes; a listing entry is about 25 bytes; the structural worst-case reply is 256 × 256 × 25 B ≈ 1.6 MB.
- "plus one fan" in the README is an informal summary of K + 256 + 1; say so or write it out.

## Versions (item 42)

The version is a causal clock that summarizes the replica's entire history of sends and redactions, its own and those it has absorbed. Equal versions therefore mean equal histories, hence equal sets.

## The five-levels figure (item 45)

Under uniform hashing the disputed paths of two replicas differing in D of N messages have separated, in expectation, after about log₂₅₆(2·D·N) levels: about five for D = N = 10⁶. It is the expected depth of the frontier, not the deepest path.

## Loss isolation (item 11; README)

The owner has ruled that the `Link` abstraction goes and QUIC's per-stream loss isolation is not retained. The reviewer is right that the README's rationale ("buys nothing for logically coupled streams") is not sound as stated: other levels' work can overlap one stream's loss recovery. State the ruling, state the cost honestly (a lost segment stalls the whole connection for one recovery, and work that would have overlapped it waits), and do not claim the isolation was valueless.

## Frame priority (item 10)

A multiplexer that prefers question-bearing frames cannot preempt a frame in transmission or bytes already in the transport's buffers. State the actual benefit: a thin reply waits for at most the frames already committed ahead of it, each bounded by the frame limit, not for the whole supply backlog.

## Ownership of the byte stream (items 54, 58)

Ruled 2026-09-20: session functions take the read and write halves by value and return them on success. `&mut` halves satisfy the same bounds through tokio's blanket impls, so a caller may pass references and keeps the halves regardless of outcome; for that case the docs carry the rule that after an error the stream's position is unknown and the connection must be discarded. Nothing about this is open.

## Other rulings

- Item 49: say the third remedy was rejected before the count was done; "miscount" and "left open" describe the same history, and "left open" is the accurate one.
- Items 4, 14, 15: the invariant everywhere is "no arriving reply finds its level's queue already at capacity". The step-0 measurement: a question counts as sent when the reply carrying it has been flushed to the wire; it counts as consumed when its stage has taken the matching reply out of the parking queue.
- Item 12: one slot suffices for the question queues and the resolution queues; the assemblers' return queues are one fan wide by construction.
- Item 47: "exactly one message in flight" is a statement about the descent, after the greeting.
- Item 64: keep one sentence on the Lean development: it explored the deadlock-freedom argument and the unbounded-parking case, its statements are trusted less than the code, and nothing in the note rests on it.
- Item 65: remove the revision-history sentence from the README.
