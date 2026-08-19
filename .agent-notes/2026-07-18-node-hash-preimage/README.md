# Single-preimage node hashing (lever E)

The Merkle convention iterated the branch rule once per compressed prefix
level, so a path-compressed node cost one BLAKE3 compression per level rather
than one per node. Two designs were drafted for collapsing that; the second
replaced the first, and the note pair is kept together because each is only
fully legible beside the other.

- [`node-hash-preimage.md`](node-hash-preimage.md) — the design that landed
  (2026-07-18): one compression per node. §5 explains why it beat the draft.
- [`spine-wrap-hash.md`](spine-wrap-hash.md) — the superseded draft: a
  two-rule spine-wrap convention, one compression per compressed prefix.
  Never implemented.

Both answer the lever E call in
[streaming latency serialization](../2026-07-17-streaming-latency-serialization/README.md) §10.

---

Resurrected from `design/node-hash-preimage.md` (written 2026-07-18, retired in `e13854de`, "Remove outdated design docs") and `design/spine-wrap-hash.md` (written 2026-07-18, retired in `9b2dd5ac`, "design: revise lever E to a single-preimage node hash"). Both bodies are verbatim: their `design/…` cross-references resolve through the [migration map](../2026-08-19-design-directory-migration/README.md).
