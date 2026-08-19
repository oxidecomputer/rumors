# Formal verification of tick

The charter for proving, in Lean, what the [tick cost
specification](../2026-07-25-before-tick-cost-spec/README.md) argued: total
equivalence and amortized linearity for `tick`. Separated from the hardening
campaign by owner direction and written to be self-contained — the hardening
campaign's artifacts are inputs to it, never dependencies of its meaning.

- [`before-formal-tick.md`](before-formal-tick.md) — mission (§1), the claims
  (§2), inheritance from the hardening campaign (§3), method (§4), phases and
  gates (§5), risks and boundaries (§6), and the decision record (§7).

---

Resurrected from `design/before-formal-tick.md`, written 2026-07-26, retired in `e13854de` (Remove outdated design docs). The body below is verbatim: its `design/…` cross-references resolve through the [migration map](../2026-08-19-design-directory-migration/README.md).
