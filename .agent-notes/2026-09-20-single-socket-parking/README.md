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
note prices it. For two 10⁶-message replicas on a 12.5 MB
bandwidth-delay link, the budget that imposes no extra latency rises from
about 0.75 GiB to about 1.0 GiB, and the default budget rises to match.

- [`exposition.md`](exposition.md) builds the argument from the protocol
  up, for an engineer fluent in distributed systems who has not read the
  code. It covers the conversation, the streaming walk and its progress
  property, the deadlock one socket creates naively, the count and the
  receive path it permits, memory, costs, and evidence.
- [`appendix-implementation-plan.md`](appendix-implementation-plan.md) is
  the implementation plan, for the implementing agent and its reviewer.
  It meters the count, removes the opening shortcut, parks replies on
  today's transport, prices parking and resizes the default, collapses
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
- The default budget keeps the reference link fully busy for 10⁶-message
  replicas.
- Giving up the bundle gives up QUIC's per-stream loss isolation.

## Open questions

1. **The progress floor and the budget.** Fixed charges, including
   parking's `+ 257` slack and level 2, are charged against the budget,
   which is today's convention, so they narrow the window. Treating them
   as a floor that may exceed the budget roughly halves the narrowing at
   small budgets. The setter's documentation already allows such a floor.
2. **Re-pricing queued scopes with the set bound.** The per-set statistic
   applies to the existing scope charge as well, and would widen every
   window independently of this design.
3. **Pricing from what the session knows.** The model prices replicas as
   if they shared nothing. The two root listings, already exchanged in
   the greetings, say exactly which of the 256 root slots differ.
   - Below saturation, the number of differing slots bounds the number of
     differing messages at the model's tail, and every dispute and
     parking term with it (exposition §6.6). This tells the ordinary case
     apart from the bulk case with no wire change.
   - Versions give equality, containment, and a lower bound on the work,
     but not an upper bound.
   - Exact peer tree profiles, or a difference estimator for the saturated
     case, would need new greeting fields.
4. **The session tail.** Nothing records why the model targets 2⁻⁴⁰ per
   session. Loosening it widens today's window somewhat, but barely moves
   parking's price.
5. **Level 2.** Near the root, parking can hold a whole level: about 25
   bytes per message of the larger replica, up to about 420 MB. Only a
   protocol change, such as the responder asking its opening questions in
   batches, would reduce that.

## Related notes

- [The streaming wire deadlock](../2026-07-17-streaming-wire-deadlock/README.md),
  which motivated independent streams.
- [Eager absorption](../2026-07-21-eager-absorption/README.md), whose
  custody analysis this design relies on for supplied content.
- [The single-socket campaign](../2026-07-21-single-socket/README.md),
  which pursued sender-side pacing instead.
