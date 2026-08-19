# How `rumors` compares: the literature review

A survey placing `rumors` among its neighbors in the reconciliation literature —
what the design takes from each line of prior art, and what each still does
better than it. The framing it lands on: `rumors` is an *augmented Merkle
repair*, Dynamo-style Merkle reconciliation made exact and tombstone-free by
fusing it with an ORSWOT-style causal-dominance rule carried per subtree rather
than per element, over ITC versions, tuned to spend bandwidth to buy round trips.

- [`literature-review.md`](literature-review.md) — epidemic anti-entropy,
  Merkle repair in production stores, canonical-shape search trees, range-based
  set reconciliation, sketch-based reconciliation, tombstone-free deletion in
  CRDTs, the version algebra, and what is new here.

Each section names the loss case honestly: where a neighbor is the better
choice, the review says so and says why. The founding entry is Demers et al.
(PODC 1987), which is also where the crate's central problem comes from — that
paper introduced death certificates because deletion cannot be represented by
absence, and the redaction design is an answer to the trade it left open.

---

Moved from `design/literature-review.md` (last revised 2026-07-24). Contents
unchanged. The body is written in rustdoc's voice and uses intra-doc link syntax
(`crate#…`), which reads as a draft of a crate-docs section rather than a
maintainer's note; it is not included into any rustdoc today.
