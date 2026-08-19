# The skyline encoding: an exposition

The intuition-first companion to the `Version` representation redesign — what a
`Version` *is* when you look at it as a skyline, why the previous format fought
back, and the two observations that dissolve the problem. Written while the
adoption decision was still open, so nothing in it is a decision record; it
optimizes for understanding rather than for specifying.

- [`before-skyline-encoding.md`](before-skyline-encoding.md) — what a Version
  is (§1), what the old format stored (§2), the two observations (§3), the
  format (§4), reading and writing (§5–§6), validation without arithmetic (§7),
  ticks as splices (§8), worked examples (§9), honest costs (§10), and the
  side-by-side correspondence (§11).

The normative specification is §10 of the [amplification
campaign](../2026-07-22-before-adversarial-resource-amplification/README.md);
where the two disagree, that one is the specification.

---

Resurrected from `design/before-skyline-encoding.md`, written 2026-07-23, retired in `e13854de` (Remove outdated design docs). The body below is verbatim: its `design/…` cross-references resolve through the [migration map](../2026-08-19-design-directory-migration/README.md).
