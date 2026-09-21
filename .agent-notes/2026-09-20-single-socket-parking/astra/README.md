# Reconciliation over one socket: parked replies

A design, not yet implemented. The streaming reconciliation protocol runs
on a bundle of independently flow-controlled streams so that one tree
level can progress while another waits. This note derives how much room a
receiver needs to accept every reply it has invited, independently of the
walk's consumption order. With that room, and decoding freed from waits on
the walk, one ordered duplex byte stream suffices under the existing
walk's progress assumptions. No credits or window advertisements are added,
and the logical exchange and its dependent network crossings stay the same.

The reply count is absolute: a level needs at most its local question
window plus 256 replies from one branching step and one reply for the
question already held by its consuming stage. The economical byte sizing
remains statistical under uniform hashing, as in the current window model;
it covers session working state and excludes replica content in the
backend, including content awaiting commit. Reception changes include wider
queues, later publication of reply bookkeeping, and a separately driven
opening decoder with room for its bounded batch.

- [`exposition.md`](exposition.md) builds the argument from the tree and
  V1's level-at-a-time exchange through V2's streaming walk to one socket,
  for a distributed-systems specialist who has not seen the code.
- [`appendix-implementation-plan.md`](appendix-implementation-plan.md)
  gives the implementation sequence for a reader who knows the code:
  instrument the count, change reception, demonstrate one pipe, then
  collapse the transport API.

Finch's rulings of 2026-09-20 settle the design choices: retain the
`K + 256 + 1` capacity rather than change publication order to tighten it;
absorb supplied subtrees into the backend before commit, with reclamation
owned by the backend; remove `Link` and its stream bundle outright; and
take the byte stream's halves by value, returning them on success.
Callers may pass mutable references, but must discard the connection after
an error. Removing the bundle gives up QUIC's per-stream loss isolation:
a lost segment on a single TCP connection delays all levels for recovery,
including work that could otherwise have overlapped it. Formalization is
secondary to the code and its tests.

Prior notes this one answers: the [streaming wire
deadlock](../2026-07-17-streaming-wire-deadlock/README.md), which motivated
independent streams; [eager
absorption](../2026-07-21-eager-absorption/README.md), whose custody analysis
left the reply count open; and the [single-socket
campaign](../2026-07-21-single-socket/README.md), which pursued sender-side
pacing.

The design was developed by Claude for Finch against `772cce34`, with the
code read for its premises; no number in this note has been measured yet.
