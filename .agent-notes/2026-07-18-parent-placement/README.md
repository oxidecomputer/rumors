# Parent placement: the deadlock-freedom / pipelining trade

The formal deadlock-freedom campaign found that the model's schedule and the
streaming encoder place a scope's parent resolution on opposite sides of that
scope's final sends — and that the two placements are the two corners of a real
design space. Parent-late buys maximal pipelining and pays a hard assembler
capacity floor; parent-early buys liveness at any capacity and pays pipelining.
This note characterizes both corners and records the resolution adopted.

- [`parent-placement.md`](parent-placement.md) — the two placements (§1), the
  capacity floor (§2), liveness at any capacity (§3), the design space (§5),
  and the resolution (§6).

Still cited by the Lean model's documentation (`formal/MODEL.md`,
`formal/PLAN.md`, `formal/PROGRESS.md`) as the record of that design space.

---

Resurrected from `design/parent-placement.md`, written 2026-07-18, retired in `e13854de` (Remove outdated design docs). The body below is verbatim: its `design/…` cross-references resolve through the [migration map](../2026-08-19-design-directory-migration/README.md).
