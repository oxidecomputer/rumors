# The uniformity envelope: how loose is the window charge?

An analysis artifact from the [single-socket
campaign](../2026-07-21-single-socket/README.md), which was declined. Its
durable content is transport-independent: an exact uniform-occupancy population
and reply-size analysis, the sharpened envelope derived from it, and **L(N)**,
the count of simultaneously-heavy stages — the principled divisor for splitting
a session memory budget across per-stream receive windows.

- [`uniformity-envelope.md`](uniformity-envelope.md) — the question (§1),
  geometry anchors (§2), uniform occupancy exactly (§3), the sharpened envelope
  (§4), results and simulation (§5–§6), the adoptable integer charge (§7),
  residual looseness (§8), the verdict (§9), and L(N) (§10).
- [`envelope-sim.py`](envelope-sim.py) — the simulation behind §6.

Everything the body calls "landed" lives on the campaign branch, not on any
branch that survives; the body's own header block says so. The envelope
mathematics itself now lives inline at the code that implements it
(`src/tree/mirror/streaming/window.rs`), which is why this note was retired.

---

Resurrected from `design/b05-uniformity-envelope.md` and `design/b05-envelope-sim.py` (written 2026-07-22, retired in `ded24eb3`, "The envelope derivations move inline: window.rs carries its own mathematics"). Bodies verbatim: their `design/…` cross-references resolve through the [migration map](../2026-08-19-design-directory-migration/README.md).
