# The latency price of σ*: round trips vs independent links

An analytic derivation with a step-counted simulation check — not a benchmark
of real code — answering: what would the σ* scheduler cost in expected round
trips, compared with the fully independent link construction? The note locates
*where* σ* waits rather than asserting a global penalty, gives closed forms for
standard shapes, and is honest about the limits of its harness tier.

- [`mux-latency.md`](mux-latency.md) — verdict up front (§0), the latency model
  (§1), completion-time laws (§2), the penalty located (§3), closed forms (§4),
  the harness and its limits (§5), and the σ*ₖ parking-dial addendum (§7).

The simulation that produced the `[checked]` tier:

- [`model.py`](model.py) — the analytic model.
- [`mux.py`](mux.py) — the scheduler under test.
- [`instances.py`](instances.py), [`gen.py`](gen.py) — instance shapes and generation.
- [`run_latency.py`](run_latency.py), [`timed.py`](timed.py) — the drivers.
- [`latency_results.json`](latency_results.json) — the recorded output.

The note adopts the hop vocabulary of
[streaming latency serialization](../2026-07-17-streaming-latency-serialization/README.md)
so the two compose.

---

Resurrected from `design/mux-latency.md and design/mux-latency/`, written 2026-07-21, retired in `e13854de` (Remove outdated design docs). The body below is verbatim: its `design/…` cross-references resolve through the [migration map](../2026-08-19-design-directory-migration/README.md).
