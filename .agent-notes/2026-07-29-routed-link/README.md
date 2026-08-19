# The routed link

The design of record for transports with no innate substreams — TCP and
everything shaped like it. A `Link` needs one persistent bidirectional control
stream plus up to `STREAM_COUNT` unidirectional data streams per direction; this
note decides how an accept/connect byte-stream transport supplies that, by
instantiating a per-process router as an in-crate generic adapter. The `Link`
contract itself is unchanged: this is purely how a transport comes to satisfy it.

- [`routed-link.md`](routed-link.md) — problem and shape (§1), the
  `Addr`/`Conn`/`Dial`/`Listen` seam (§2), the wire (§3), the router (§4), link
  establishment (§5), the contract mapping (§6), and the decision records (§7).

Instantiates the router sketched in §8.4–8.5 of [the streaming wire
deadlock](../2026-07-17-streaming-wire-deadlock/README.md).

---

Resurrected from `design/routed-link.md`, written 2026-07-29, retired in `e13854de` (Remove outdated design docs). The body below is verbatim: its `design/…` cross-references resolve through the [migration map](../2026-08-19-design-directory-migration/README.md).
