# Materialized backlogs: durable causal subscriptions

`CausalMessages` delivers in causal order by staging each ingest pass in an
in-memory map, so the causal buffer materializes the whole undelivered delta —
payloads included — and a fresh observer replaying a large persistent store
holds the entire set resident. No per-node monoidal cache fixes this, because
rank order is global. This note designs a durable backlog that does. It was
filed as a design deliverable for later implementation, not implemented.

- [`materialized-backlogs.md`](materialized-backlogs.md) — context, the owner
  rulings of 2026-07-29/30, the design, the deliverable's scope, verification,
  risks, and — as an appendix — the persistent-storage campaign's decision
  record, whose rulings the note says continue to govern.

**Provenance note:** unlike everything else here, this document was never on
`main`. It lives on the unmerged branch `persistent-storage`, and is resurrected
from that branch's tip.

---

Resurrected from `design/materialized-backlogs.md` on branch `persistent-storage` (written 2026-07-29, last touched at `ff950517`, "the re-execution ratchet: the crate's own closures run the retry schedule"). The body below is verbatim: its `design/…` cross-references resolve through the [migration map](../2026-08-19-design-directory-migration/README.md).
