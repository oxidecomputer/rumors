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
  today's transport, sharpens and extends the pricing, sizes the default,
  collapses the transport to one pipe with its demonstration, and
  measures the serialization delay that priority cannot remove. It
  includes draft user-facing text on what the budget estimate means.
- [`sizing-model.py`](sizing-model.py) reproduces every modeled figure in
  the note: a transcription of the window model with the note's
  additions, and the duplex simulation. Run it with
  `python3 sizing-model.py`.

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
- Explicit per-level credits are rejected, for simplicity and
  link-independence rather than memory. Sized to the levels where bulk
  flows, credits need less memory than parking from about 10⁶ messages
  up: about 0.26 GiB against 0.46 GiB at 10⁶, and 1.6 GiB against 5.1
  GiB at 10⁸, on the long-haul link. The price is a flow-control
  protocol with its own liveness argument, and sizing that needs the
  link's bandwidth-delay product (exposition §7).
- Fixed charges, including parking's `+ 257` slack and the level-2 term,
  are charged against the budget, as today's fixed charges are, so they
  narrow the window. The level-2 term does not shrink with the window.
  When fixed charges alone exceed a budget, the window falls to one slot
  per level and the estimate exceeds the budget: the setter's documented
  progress floor.
- Giving up the bundle gives up QUIC's per-stream loss isolation.

## Open questions

1. **The session tail.** Nothing records why the model targets 2⁻⁴⁰ per
   session.
2. **A difference estimator.** Above about 1,400 differences the root
   comparison saturates, and sessions are priced as fully divergent. A
   difference estimator in the greeting would cover them. Exact tree
   profiles, including an exchanged leaf depth, would not: they sharpen
   fans, which are already priced close to the truth (exposition §6.7).

## A possible follow-on

**Deferring bulk** (exposition §8), examined and not recommended. A supply travels inline, in radix
order, so it delays everything below and to its right in the tree. A
follow-on could send a *promise* in the reply instead, and move the
content to a bulk lane that the multiplexer sends only when no descent
frame is ready.

Under uniform hashing the payoff is small. Supplies concentrate in a
band one or two levels thick, so the descent loses at most the crossings
remaining below the band: about one round trip. Queuing requests behind
bulk can also delay the start of the reverse direction's bulk. Hashing
spreads requests evenly, so that costs a few percent of the transfer time
only when both directions' bulk is balanced and made of few, large
messages. The follow-on is recorded, not recommended, unless measurement
shows larger penalties.

## Accepted costs

- When every root slot differs, parking can hold a whole level near the
  root: every occupied depth-3 prefix, at 25 bytes each. That is about
  24 MB at 10⁶ messages and 188 MB at 10⁷, and at most about 420 MB. Shrinking it would take a protocol change that costs more than
  it saves.

## Related notes

- [The streaming wire deadlock](../2026-07-17-streaming-wire-deadlock/README.md),
  which motivated independent streams.
- [Eager absorption](../2026-07-21-eager-absorption/README.md), whose
  analysis of who owns supplied content before commit this design relies
  on.
- [The single-socket campaign](../2026-07-21-single-socket/README.md),
  which pursued inferred credits instead (exposition §4).
