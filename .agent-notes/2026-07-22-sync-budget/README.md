# The sync budget: session memory bounded by occupancy

The campaign that gave a session a memory bound derived from occupancy rather
than a guessed constant, and then let the backend price it. Phases 1–3 landed
on 2026-07-22 and phase 4 the same day, spec-first, with each section carrying
its own landed status and transcription deviations recorded as dated amendments
in place.

- [`sync-budget.md`](sync-budget.md) — what landed (§1), backend-priced
  budgeting (§2), and what was deliberately left out of scope (§3).

The occupancy mathematics itself lives inline at the functions that implement
it (`src/tree/mirror/streaming/window.rs`). Grew out of
[streaming latency serialization](../2026-07-17-streaming-latency-serialization/README.md).

---

Resurrected from `design/sync-budget.md`, written 2026-07-22, retired in `e13854de` (Remove outdated design docs). The body below is verbatim: its `design/…` cross-references resolve through the [migration map](../2026-08-19-design-directory-migration/README.md).
