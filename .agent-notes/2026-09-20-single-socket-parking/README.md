# Reconciliation over one socket: parked replies

A design, not yet implemented. Its figures come from the sizing model and
have not been measured.

The streaming reconciliation protocol runs over a bundle of
independently flow-controlled streams, so that one tree level waiting on
its consumer never stops another level's traffic. This note shows that
one ordered duplex byte stream suffices, with no credits and no new
messages. A receiver invites every reply it gets by asking a
question, so the number of replies it can have in flight at a level is
bounded by its own configuration: `K(ℓ) + 257`. A receiver that reserves
that room, and whose decoding never waits on the walk, never lets one
level block another, so the walk's existing deadlock-freedom argument
carries over.

The price is memory that the bundle kept back at the sender, and the
note prices it. Two changes recover much of it: queued work is priced as
a set, and the greetings carry each side's memory budget, so that both
sides share one window set by the smaller budget and price parking
against it. The net effect depends on scale. For two entirely different
10⁶-message replicas on a 12.5 MB bandwidth-delay link, the budget that
imposes no extra latency falls from about 0.75 GiB to about 0.43 GiB,
so the 512 MiB default stands, and now keeps that link busy. At 10⁷
messages it rises from about 1.7 to about 2.0 GiB.

## Documents

- [`exposition.md`](exposition.md) builds the argument from the protocol
  up, for an engineer fluent in distributed systems who has not read the
  code. It covers the conversation and the streaming walk (§§2–3), the
  deadlock one socket naively creates (§4), the count and the receive
  path it permits (§5), memory (§6), latency and loss (§7), the evidence
  (§8), explicit credits for contrast (§9), and follow-ons (§10).
- [`appendix-implementation-plan.md`](appendix-implementation-plan.md)
  is the implementation plan, for the implementing agent and its
  reviewer. Its steps meter the count, remove the opening shortcut, park
  replies on today's transport, price parking, exchange budgets in the
  greeting, collapse the transport to one pipe with its demonstration,
  and measure the serialization delay that priority cannot remove. It includes draft
  user-facing text on what the budget estimate means.
- [`sizing-model.py`](sizing-model.py) reproduces every modeled figure in
  the note. It transcribes the window model with the note's additions,
  and runs the duplex simulation. Run it with `python3 sizing-model.py`.

## Design decisions

- `Link` and its stream bundle are removed. Sessions take the halves of
  one duplex byte stream by value and return them on success. Callers
  that keep ownership pass `&mut` halves, and discard the connection
  after an error.
- The opening-supply shortcut is removed. It helps only between small
  replicas, and over one socket it would deadlock.
- The multiplexer serves every frame that is not a supply run before any
  supply run.
- A parking overflow fails the session with a typed error instead of
  waiting.
- Supplied content is absorbed into the backend before commit, and the
  backend owns reclaiming it when a session fails.
- Queued scopes are priced as a set.
- The greetings carry each side's budget, and the server's also carries
  the window it computes from the smaller budget. Both sides hold at
  most that window and price parking against it (exposition §6.6). A
  well-provisioned peer can be narrowed by a constrained one, but a
  session only moves as fast as its narrower side, so the session gains.
- A bound on the number of differing messages, drawn from comparing the
  two root listings, was considered and dropped: the shared window
  already gives sessions with few differences all the width they can use
  (exposition §6.8).
- The default budget keeps the reference link fully busy for
  10⁶-message replicas, and a committed test checks that it does.
- Explicit per-level credits are rejected for their complexity and their
  dependence on the link, and not for memory. Sized to the levels where
  bulk flows, credits need less memory than parking from about 10⁶
  messages up, about 0.26 GiB against 0.43 GiB at 10⁶ and 1.6 GiB
  against 2.9 GiB at 10⁸ on the long-haul link. Their price is a
  flow-control protocol with its own liveness argument, and sizing that
  needs the link's bandwidth-delay product (exposition §9).
- Fixed charges, including parking's `+ 257` slack, count against the
  budget as the model's existing fixed charges do, so they narrow the
  window. When fixed charges alone exceed a budget, the window falls to
  one slot per level and the estimate exceeds the budget. That is the
  minimum for progress that the budget's documentation already states.
- Giving up the bundle gives up QUIC's per-stream loss isolation.

## Open questions

1. **The session tail.** Nothing records why the model targets a failure
   probability of 2⁻⁴⁰ per session.

## A follow-on, not recommended

Deferring bulk (exposition §10.2) is not recommended. A supply travels
inline, in radix order, so it delays everything below and to its right
in the tree. A follow-on could instead send a *promise* in the reply and
move the content to a bulk lane that the multiplexer sends only when no
descent frame is ready. Under uniform hashing the gain is small.
Supplies concentrate in a band one or two levels thick, so the descent
loses at most the crossings remaining below the band, about one round
trip. Queuing requests behind bulk can also delay the start of the
reverse direction's bulk, but hashing spreads requests evenly, so this
costs a few percent of the transfer time only when both directions'
bulk is balanced and made of few, large messages. The follow-on stays
unbuilt unless measurement shows larger penalties.

## Accepted costs

When every root slot differs, parking can hold a whole level near the
root: every occupied depth-3 prefix, at 25 bytes each. That is about
24 MB at 10⁶ messages, 188 MB at 10⁷, and at most about 420 MB
(exposition §6.4). The shared window bounds how much of it can arrive
at once, and so its price: 92 MiB at the default budget with 10⁷
messages.

## Related notes

- [The streaming wire deadlock](../2026-07-17-streaming-wire-deadlock/README.md),
  which motivated independent streams.
- [Eager absorption](../2026-07-21-eager-absorption/README.md), whose
  analysis of who owns supplied content before commit this design relies
  on.
- [The single-socket campaign](../2026-07-21-single-socket/README.md),
  which pursued inferred credits instead (exposition §4).
