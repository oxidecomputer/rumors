# The CBOR-legible wire

[`cbor-legible-wire.md`](./cbor-legible-wire.md) is the design document of
record for converting the rumors wire protocol to end-to-end
deterministic-encoding CBOR: every directed stream of a V2 session parses as
an RFC 8742 CBOR sequence, the on-disk bookmark became a fully CBOR-parseable
file (format v4), a three-level observation hook was added, and the opaque
hexdump wire snapshots were supplanted by CBOR reflection rendering. Its
decision record carries the owner rulings and implementer resolutions the
work was built under, including the post-review rulings on error taxonomy,
feature gating, and hook identity.

Retired as implemented: the design shipped on this branch, survived a
multi-round adversarial review, and its invariants live inline at the code
they govern, per the repository's rules. The body is byte-identical to the
document's last revision in `design/`; its citation of
`design/payload-depth-limit.md` resolves to
[`../2026-08-20-payload-depth-limit/payload-depth-limit.md`](../2026-08-20-payload-depth-limit/payload-depth-limit.md).
The wire-corpus figures quoted inside were measured at the commits the
document names; the code they describe has the current numbers.
