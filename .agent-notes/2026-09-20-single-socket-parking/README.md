# Reconciliation over one socket: parked replies

A design, not yet implemented. Its numbers come from the sizing model and
have not been measured.

The streaming reconciliation protocol needs a bundle of independently
flow-controlled streams, so that one tree level waiting on its consumer
never stops another level's traffic. This note shows that one ordered
duplex byte stream suffices, with no credits or window advertisements.
The number of replies a receiver can have in flight at a level is bounded
by its own configuration, `K(ℓ) + 257`. A receiver that reserves that
room, and whose decoding never waits on the walk, never lets one level
block another, so the walk's existing deadlock-freedom argument carries
over.

The price is memory that the bundle kept back at the sender, and the
note prices it. It also sharpens the pricing in two places where the
session knows more than the model assumes. Two changes do the sharpening:
queued work is priced as a set, and the root listings the greetings
already exchange bound how many messages differ. The net effect depends
on scale:

- For two entirely different 10⁶-message replicas on a 12.5 MB
  bandwidth-delay link, the budget that imposes no extra latency falls
  from about 0.75 GiB to about 0.46 GiB, so the 512 MiB default likely
  stands.
- At 10⁷ messages it rises from about 1.7 to about 2.1 GiB.
- Replicas that differ in a few hundred messages are priced at a few
  MiB.

- [`exposition.md`](exposition.md) builds the argument from the protocol
  up, for an engineer fluent in distributed systems who has not read the
  code. It covers the conversation, the streaming walk and its progress
  property, the deadlock one socket creates naively, the count and the
  receive path it permits, memory, a contrast with explicit credits,
  costs, and evidence.
- [`appendix-implementation-plan.md`](appendix-implementation-plan.md) is
  the implementation plan, for the implementing agent and its reviewer.
  It meters the count, removes the opening shortcut, parks replies on
  today's transport, sharpens and extends the pricing and sizes the
  default, collapses
  the transport to one pipe with its demonstration, and measures the
  serialization residual. It includes draft user-facing text on what the
  budget estimate means.

## Design decisions

- `Link` and its stream bundle are removed. Sessions take the halves of
  one duplex byte stream by value and return them on success. Callers
  that keep ownership pass `&mut` halves and discard the connection after
  an error.
- The opening-supply shortcut is removed; it helps only between small
  replicas and would deadlock over one socket.
- The multiplexer serves every frame that is not a supply run before any
  supply run.
- A parking overflow fails the session with a typed error; it never
  waits.
- Supplied content is absorbed into the backend before commit. The
  backend owns reclaiming it when a session fails.
- Queued scopes are priced as a set, and pricing uses the difference bound
  from the root comparison. Queue capacities stay deterministic.
- The default budget keeps the reference link fully busy for 10⁶-message
  replicas, as a committed test checks.
- Explicit per-level credits are rejected. The exposition compares them:
  credits give a deterministic, link-sized bound, tighter than parking's
  for large divergent replicas, at the cost of a flow-control protocol
  and knowledge of the link.
- Giving up the bundle gives up QUIC's per-stream loss isolation.

## Open questions

1. **The progress floor and the budget.** Fixed charges, including
   parking's `+ 257` slack and level 2, are charged against the budget,
   which is today's convention, so they narrow the window. Treating them
   as a floor that may exceed the budget helps small budgets. The
   setter's documentation already allows such a floor.
2. **The session tail.** Nothing records why the model targets 2⁻⁴⁰ per
   session.
3. **Level 2.** When every root slot differs, parking can hold a whole
   level: about 25 bytes per message of the larger replica, up to about
   420 MB. Exposition §6.8 outlines the protocol change that would bound
   it: deferred questions.
4. **What else a session could know.** Exact peer tree profiles, or a
   difference estimator for the case where every root slot differs,
   would need new greeting fields (exposition §6.7).

## Related notes

- [The streaming wire deadlock](../2026-07-17-streaming-wire-deadlock/README.md),
  which motivated independent streams.
- [Eager absorption](../2026-07-21-eager-absorption/README.md), whose
  custody analysis this design relies on for supplied content.
- [The single-socket campaign](../2026-07-21-single-socket/README.md),
  which pursued sender-side pacing instead.
