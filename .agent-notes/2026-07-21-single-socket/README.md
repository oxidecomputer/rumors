# The single-socket transport campaign (explored, then declined)

An exploration of replacing the `Link`'s per-stream flow control with a single
byte stream and sender-inferred receive windows. It reached a working prototype
tier and was **declined** on 2026-07-22: the `Link` stayed, and parts of the work
were harvested onto `link-transport`. Read the retrospective first — it is the
decision record, and it says what was learned and what was harvested.

- [`single-socket-retrospective.md`](single-socket-retrospective.md) — the
  decision record (2026-07-22, Finch): what was explored, the durable yield,
  why it was declined, what was harvested, and the follow-on task it opened.
- [`single-socket.md`](single-socket.md) — the design of record while the
  campaign was live: end state and `Link` as scaffolding (§1), the receive side
  (§2), the σ*ₖ send engine (§3), the theorem interface (§4), acceptance (§5),
  honest residuals (§6).
- [`single-socket-plan.md`](single-socket-plan.md) — the execution plan: stages
  as a dependency DAG, anchored tasks, tests named before code, gates per stage,
  and the design-doc/code discrepancies found while anchoring (§8).

Companions elsewhere in these notes:
[eager absorption](../2026-07-21-eager-absorption/README.md) (the custody
assessment the design leans on) and
[the uniformity envelope](../2026-07-22-uniformity-envelope/README.md) (the
window-charge analysis, whose transport-independent core outlived the campaign).
The campaign's other documents — `byte-window-plan.md`, `pooled-budget-spike.md` —
live only on the archive branch `wave1/integration`.

---

Resurrected from `design/single-socket.md, design/single-socket-plan.md, design/single-socket-retrospective.md`, written 2026-07-21 – 2026-07-22, retired in `e13854de` (Remove outdated design docs). The body below is verbatim: its `design/…` cross-references resolve through the [migration map](../2026-08-19-design-directory-migration/README.md).
