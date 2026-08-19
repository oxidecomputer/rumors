# The tick/fill cost specification

The statement of record for the tick limb cure and fusion: that `tick` (fill
then grow) is computable over skyline streams in amortized O(n + m) accumulator
digit touches. It took seven adversarial attack/fix rounds plus the fusion
landing to reach an implementation the claim survives against; §9 is the
compact record of that loop, with each round's full narrative left in git
history at the revisions the round dates name.

- [`before-tick-cost-spec.md`](before-tick-cost-spec.md) — the function of
  record (§1), the refutation it answers (§2), the theorem and its
  decomposition (§3), the per-dimension cost model (§4), what to do if a lemma's
  realization is refuted (§5), the fusion (§6), witness families and the
  acceptance contract (§7), decisions (§8), and the adversarial record (§9).

The formal follow-on is [formal verification of
tick](../2026-07-26-before-formal-tick/README.md).

---

Resurrected from `design/before-tick-cost-spec.md`, written 2026-07-25, retired in `e13854de` (Remove outdated design docs). The body below is verbatim: its `design/…` cross-references resolve through the [migration map](../2026-08-19-design-directory-migration/README.md).
