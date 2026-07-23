# The single-socket exploration: a retrospective

Decision record, 2026-07-22 (Finch). The single-socket transport
campaign — replacing the Link's per-stream flow control with a single
byte stream and sender-inferred windows — was explored to a working
prototype tier and **declined**. The Link stays. This document is the
record of what was explored, what was learned, and why the trade
resolved the way it did, so the question is answered by reading rather
than re-derivation. The archive is branch `wave1/integration`
(which grew from `single-connection` and holds the full trail); the campaign's design documents there
(`single-socket.md`, `single-socket-plan.md`, `byte-window-plan.md`,
`pooled-budget-spike.md`, `b05-uniformity-envelope.md`) carry the full
decision trail, B0.1–B0.9.

## What was explored

The premise: the Link's contract (a control stream plus independently
flow-controlled data streams) is a nontrivial obligation on callers —
subtle enough to ship a conformance suite — and a session over one
plain `AsyncRead + AsyncWrite` would dissolve it. The cost: flow
control must then be rebuilt above the socket. The campaign built, in
sequence: a receive-window advertisement riding the greeting; a
sender-side occupancy ledger inferring the peer's consumption from
arrival causality plus an inevitability closure (no credit frames —
"zero extra metadata"); byte-denominated admission replacing reply
counts (a ~5,000× per-slot spread between sparse and dense replies
made count windows price thin replies at fat cost); a
uniform-occupancy envelope derivation with a smooth
element-declaration divisor; and finally a pooled budget across
streams with per-stream floors, probed across 3,340 runs with zero pool-attributable
wedges (every hard-stuck reproduced by the uncoupled control). A parallel Lean campaign proved the static count-window form
(T8, per-direction, asymmetric) and the floor base case
(`sigmaStarCausal_deadlock_free`, unconditional).

## What was learned (the durable yield)

1. **Byte denomination is correct and count denomination is not** —
   for this protocol's reply-size spread, a count window either
   over-admits memory or under-admits concurrency by orders of
   magnitude. Vindicated by convergence: QUIC-class transports chose
   byte windows decades ago. Under the Link, this conclusion is
   *delivered by the transport for free*.
2. **The fat/thin structural asymmetry**: every megabyte-scale arrival
   is self-invited (a reply to one's own question; the merge-join
   answers every query exactly once), and every uninvited arrival
   (questions) is thin and guaranteed an echo. Any future flow-control
   reasoning here should start from this shape.
3. **The across-stream division is the irreducibly predictive
   decision.** Metering adapts within a stream; dividing one memory
   budget across seventeen per-stream windows requires a prior on
   level *simultaneity*. The uniformity analysis (harvested:
   `b05-uniformity-envelope.md`) derives it — L(N) simultaneously
   heavy levels for a declared set size N, smooth and monotone — and
   this applies directly to sizing the Link's per-stream transport
   receive windows, where the same division problem exists today
   unprincipled.
4. **Uniform-occupancy pricing beats worst-case pricing ~2–2.5×**
   (corrected per-stage envelope vs the flat charge), with the
   population object being queried listing entries, not branch
   counts — the analysis survives for any future memory-budget work.
5. **Pooling with per-stream floors is empirically sound and its
   starvation objection dissolves for a principled reason**: a causal
   consumption meter charges dependency, not consumer speed, so a
   lagging consumer cannot squat a shared budget. Recorded with a
   3,340-run probe and an inventoried proof obligation
   (`pooled-budget-spike.md`).
6. **The model of record**: uniform-hash keys plus
   authenticated-honest peers. Hostile-peer regimes are off-model
   (authorization already grants set-destruction); no pricing argument
   may rest on adversary economics. Landed as an `AGENTS.md` hard
   rule alongside the no-ghost-references rule.

## Why it was declined

- **The zero-metadata property saves nothing material.** Explicit
  credit returns (QUIC `MAX_STREAM_DATA`, H2 `WINDOW_UPDATE`) cost
  ~5–15 bytes roughly once per RTT per active stream — well under
  0.1% of this protocol's traffic. The elegance was real as insight;
  as engineering it purchased a large machinery (ledger, closure,
  pool) and a proof burden to avoid a negligible cost.
- **Inference is slower than truth.** The inferred window is
  conservative by construction (over-estimation of unconsumed cost is
  the safe direction); an explicit credit is prompt. Battle-tested
  stacks additionally bring receive-window autotuning, pacing, and
  per-stream loss recovery; the single socket couples all seventeen
  streams under one lost segment — its own design doc carried that as
  an accepted price, and it is a real tail regression.
- **The novel piece is exactly the unproven piece.** The landed Lean
  suite covers static, message-denominated, fixed-gate windows; the
  pooled byte gate fails its quantifier twice (run-adaptive grants;
  byte denomination — "positives do not transfer to bytes"). Closing
  it is a T8-scale track with no owner (B0.9). Everything the campaign
  wanted to ship rested on either that unowned proof or a permanent
  evidence-tier posture.
- **The surviving argument didn't survive scrutiny.** "Any byte
  stream, no mux dependency" was the campaign's real motivation, and
  it was judged not a product requirement — the existing deployments
  bind through iroh, and the Link's conformance suite plus its
  deployment mileage is known complexity versus new complexity owned
  forever.

## What was harvested (landed on `link-transport`)

Transport-independent fixes and tests, each cited at its landing
commit on `link-transport`: the parked-reply memory-accounting
tests (`3a5ba643`); the context-registration-causality proptest
(`675e2f53`); the `ProxyLocalQuestions` occupancy derivation
(supremum exactly min(capacity, S)) with the eager-absorption
assessment imported (`b76a31f3`); the ghost-reference prose sweep
(`44724ad0`); the `AGENTS.md` hard rules (`a59dc786`). Analyses:
the corrected uniformity envelope and its simulation, imported as
a decision-record artifact (`5f86158f`) — principled per-stream
receive-window sizing from one budget via L(N). Everything else — greeting
advertisement, widened parking, the ledger, the pool, the socket
harness — remains on the archive branch as the priced record of the
road not taken.

## The follow-on task this opens

Size the Link's per-stream transport receive windows from a single
byte-denominated session budget using the harvested L(N) analysis —
replacing whatever ad-hoc figures the bindings use today. That task
inherits this campaign's best result without any of its machinery.
