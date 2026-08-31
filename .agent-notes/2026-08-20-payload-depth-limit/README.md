# The payload depth limit

[`payload-depth-limit.md`](./payload-depth-limit.md) is the design document
and decision record for the payload-depth package: the configurable,
fleet-symmetric nesting-depth limit exchanged in the greeting and held to
exact equality; the peer-minted payload codec; admission by the receiver's
exact decode, extended to full value faithfulness (`Eq` mandated, decoded
value must equal the value sent); the closure-scoped batch with commit-on-Ok;
the exact ciborium pin with its bump playbook; and the vendor evaluation that
retained ciborium.

Retired as implemented: every ruling in the record shipped on this branch,
and the load-bearing invariants live inline at the code (the knob's rustdoc
and the crate root's payload contract), per the repository's rules — this
document is provenance. The body is byte-identical to the document's last
revision in `design/`; its citation of `design/cbor-legible-wire.md` resolves
to
[`../2026-08-19-cbor-legible-wire/cbor-legible-wire.md`](../2026-08-19-cbor-legible-wire/cbor-legible-wire.md).
