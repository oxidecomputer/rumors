# Reconciliation over one socket: parked replies

A design, not yet implemented. The streaming reconciliation protocol runs
today over a bundle of independently flow-controlled streams because its
deadlock-freedom argument needs them. This note shows that a single ordered
duplex byte stream suffices, at the same round-trip count and under a fixed
memory bound, with no flow-control machinery added to the wire and no change
to the walk or its messages. The change is confined to the receiving side,
and it is a resizing, not a re-plumbing: the per-level queue of decoded
replies that holds one reply today is sized from that side's own window
plus one fan, and a counting fact about the protocol guarantees no reply
ever arrives to find it full. With the streams gone, the `Link`
abstraction goes too: a session takes the two halves of a byte stream.

- [`exposition.md`](exposition.md): the argument, built up from the tree
  and the level-at-a-time protocol through the streaming protocol to the
  single-socket form. Written for a distributed-systems engineer who has
  not seen the code. Rewritten whole after each of two fresh-eyes reading
  rounds.
- [`appendix-implementation-plan.md`](appendix-implementation-plan.md): the
  route from the current tree to the single-pipe end state, step by step,
  for someone who knows the code. Instruments land before cures; the
  public-API shape is proposed for the owner's ruling.

Rulings from Finch, 2026-09-20, recorded so the plan need not re-ask them:
the `+ FAN` slack in the parking bound is acceptable (no reserve-first
tightening); eager absorption of supplied subtrees into the backend before
commit is desired, with the backend owning reclamation as it does for
in-memory handles; the `Link`'s stream bundle collapses outright rather than
surviving as an optional layer, since QUIC loss isolation buys nothing for
logically coupled streams; the formal counting lemma is secondary to the code
and its tests.

A second version of all three documents, reviewed and then revised by
GPT-6-Astra through the Codex CLI with no access to the repository,
sits in [`astra/`](astra/README.md) beside its [review](astra/review.md),
the [author's factual answers](astra/author-answers.md) it revised
against, and its [notes](astra/revision-notes.md). The two versions are
kept side by side for comparison; neither supersedes the other yet.

Prior notes this one answers: the [streaming wire
deadlock](../2026-07-17-streaming-wire-deadlock/README.md) (the cycle, and
the determination to demand independent streams), [eager
absorption](../2026-07-21-eager-absorption/README.md) (the custody
assessment this design relies on, whose §7.2 left the count open), and the
[single-socket campaign](../2026-07-21-single-socket/README.md) (declined
because it rebuilt sender-side pacing; this design needs none).

Written by Claude for Finch at `772cce34`; the reasoning was checked against
the code by reading, and no number in it has been measured yet.
