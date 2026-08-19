# Streaming latency serialization

Why the streaming protocol spent latency it did not need to: one-slot channels
serialized the descent, so a session paid a round trip per level rather than
pipelining. The note carries the instrument, the confirmed root cause, the
experiment that made capacity the knob, and the fix space — including the
"parity levers" ladder (§10) that later notes here implement one lever at a
time.

- [`streaming-latency-serialization.md`](streaming-latency-serialization.md) —
  instrument (§1), results (§2), root cause (§3), fix space (§5), the memory
  price of K (§6), the levers (§10), and the hop ledger (§11).

Commit hashes in the body resolve on the archive branch `wave1/integration`,
not on any branch that survives today; the body's own header says so.

---

Resurrected from `design/streaming-latency-serialization.md`, written 2026-07-17, retired in `e13854de` (Remove outdated design docs). The body below is verbatim: its `design/…` cross-references resolve through the [migration map](../2026-08-19-design-directory-migration/README.md).
