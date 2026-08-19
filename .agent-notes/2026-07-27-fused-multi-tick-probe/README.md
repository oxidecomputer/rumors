# The fused multi-tick: `ticks(n)`

A measure-and-design probe on a branch that was never meant to merge. The
question: can a run of `n` ticks be fused into one computation, priced better
than iterating? The verdict is yes, with one structural refinement, and the note
carries deterministic prices and a design sketch for a public `ticks(n)`
operation mirroring the whole `tick` surface.

- [`probe-ticks.md`](probe-ticks.md) — the verdict and its refinement, pricing,
  the design sketch, and the probe artifacts.

Owner ruling recorded mid-probe: the operation is named `ticks`, not `tick_by`.

---

Resurrected from `design/probe-ticks-68.md`, written 2026-07-27, retired in `e13854de` (Remove outdated design docs). The body below is verbatim: its `design/…` cross-references resolve through the [migration map](../2026-08-19-design-directory-migration/README.md).
