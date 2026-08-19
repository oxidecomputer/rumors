# The streaming wire deadlock

The diagnosis that produced the `Link` contract. After the peer path swapped
from the alternating (V1) protocol to the streaming (V2) one, two proptests
stalled on a genuine wait cycle; this note locates the cycle, shows why every
"just widen the handoff" fix is individually right and jointly insufficient,
and determines the fix — a stream-capable transport contract (§8), the
argument that `src/link.rs` still rests on.

- [`streaming-wire-deadlock.md`](streaming-wire-deadlock.md) — diagnosis
  (§1–§4), the fix space (§5, including the single-socket instantiation later
  declined), regression prevention (§7), and the determination (§8).

---

Resurrected from `design/streaming-wire-deadlock.md`, written 2026-07-17, retired in `e13854de` (Remove outdated design docs). The body below is verbatim: its `design/…` cross-references resolve through the [migration map](../2026-08-19-design-directory-migration/README.md).
