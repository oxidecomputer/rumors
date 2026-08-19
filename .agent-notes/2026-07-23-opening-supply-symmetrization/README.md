# Opening-supply symmetrization

Implemented. The initiator's exclusive content was sent node by node during the
opening exchange where a whole-subtree batch would do; this note specifies the
supply-only root answer that fixes it. It is pedagogical by intent: it teaches
the opening exchange from scratch, demonstrates the cascade hazard that
disqualifies the obvious fix, and only then specifies the change precisely
enough to implement red-green cold.

- [`opening-supply-symmetrization.md`](opening-supply-symmetrization.md) —
  motivation (§1), how the opening worked (§2), the cascade hazard (§3), the
  design (§4), proof obligations (§5), implementation instructions (§6),
  rejected alternatives (§7), non-goals (§8), and the post-implementation
  amendments.

Its `file:line` anchors were verified at commit `22b61c02` and have since
drifted; §9 tells a fresh reader how to re-anchor.

---

Resurrected from `design/opening-supply-symmetrization.md`, written 2026-07-23, retired in `e13854de` (Remove outdated design docs). The body below is verbatim: its `design/…` cross-references resolve through the [migration map](../2026-08-19-design-directory-migration/README.md).
