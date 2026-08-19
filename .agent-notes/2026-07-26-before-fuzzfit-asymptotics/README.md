# The fuzz-fit asymptotics harness

The instrument of record for `before`'s instruction-count asymptotics: a
harness that fits observed instruction counts against input size and judges the
fit against pinned bands, so a superlinear regression fails a gate rather than
going unnoticed. The note is the statement of the instrument — what it guards
and what it *adds* over the existing meters — and §8 argues its adequacy by
constructing known-bad implementations and showing the harness rejects them.

- [`before-fuzzfit-asymptotics.md`](before-fuzzfit-asymptotics.md) — what it
  guards (§1), the mechanism (§2), generators (§3), denomination (§4), band
  calibration and enforcement (§5), the pin of record (§6), the platform note
  (§7), adequacy and residual risk (§8), and the dated decision record (§9).

Cited as the instrument by the [amplification
campaign](../2026-07-22-before-adversarial-resource-amplification/README.md)'s
metering-gate section.

---

Resurrected from `design/before-fuzzfit-asymptotics.md`, written 2026-07-26, retired in `e13854de` (Remove outdated design docs). The body below is verbatim: its `design/…` cross-references resolve through the [migration map](../2026-08-19-design-directory-migration/README.md).
