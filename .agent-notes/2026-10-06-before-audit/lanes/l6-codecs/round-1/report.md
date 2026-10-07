<!-- CAVEAT LECTOR: the round-1 report of the L6 (codecs) auditor, Claude Opus 5.5, condensed by the coordinator, who verified the explore branch's signatures; results are the auditor's own claims. -->

# Lane L6, round 1: report

The explore branch is `explore/l6-codecs`, and all nine of its commits are
signed. The sibling files in this directory are the auditor's deliverables.

## Verdict

- One confirmed defect, of medium severity, and four questions. The lane is
  at diminishing returns for value-level codec correctness.
- An independent specification codec for all six wire types was compared
  against every decode entry point:
  - slice decode
  - text
  - postcard
  - CBOR
  - borsh in a stream
  - chunked and `Interrupted` readers
  - JSON spans
- It ran 300,000 cases per type, twice. Production returned exactly the
  model's error class on every rejected input.
- Its 17 calibration mutations were all caught. The committed suite caught
  all 17 too, so no new semantic instrument is proposed.

## Defect

**On 32-bit targets, decoders panic once an input-sized buffer passes 2^30
bytes.** The records are
[`defect-rank-decode-wasm32-growth.md`](defect-rank-decode-wasm32-growth.md),
[`test-brief-rank-decode-wasm32-growth.md`](test-brief-rank-decode-wasm32-growth.md),
and [`fix-note-rank-decode-wasm32-growth.md`](fix-note-rank-decode-wasm32-growth.md).

- **Mechanism:** `Vec::push` doubles a full 2^30-byte buffer to 2^31 bytes,
  which exceeds `isize::MAX` on 32-bit, so it panics with "capacity
  overflow". Memory is not exhausted.
- **Demonstrated sites, on wasm32:**
  - `Rank::decode`'s fraction buffer (`rank.rs:660`, `:669`)
  - borsh's stream buffer (`borsh_impls.rs:104`)
- **Divergence:** the reader-based `Version::decode` returns
  `Decode::Io(OutOfMemory)` for the same bytes, so one input yields three
  outcomes across targets and entry points.
- **Inferred sites, not demonstrated:**
  - the rank integral's bit buffer
  - `Ranked`'s borsh prefix buffer
  - the validators' bit stacks
  - `BitsWriter` growth
- **What the fixed decoder should return** is question 17.

## Questions

These are in [`questions.md`](questions.md).

- **Q4:** the fixed decoder's outcome on 32-bit. This is question 17 in
  `QUESTIONS.md`.
- **Q1:** human-readable serde accepts JSON arrays in place of the records it
  emits. This is question 18.
- **Q2:** CBOR byte strings decode as `Count` limbs. This is question 19.
- **Q3:** `Decode::TrailingBits` is also returned for an all-zero final
  fraction group, which lies inside the stream. Widening the variant's doc is
  a docs-branch correction toward the code.

## Simplification briefs

- [`simplification-padded-prefix.md`](simplification-padded-prefix.md): one
  shared step for "decode a tree, then its padding".
- [`simplification-bit-count-types.md`](simplification-bit-count-types.md):
  three bit counts typed as `usize`, each at most 64.
- [`simplification-dead-splice-branch.md`](simplification-dead-splice-branch.md):
  a provably unreachable early return in `splice_continuation`. A mutation
  inside it survives all 695 committed tests.

## Observations and coverage

- Observations: [`observations.md`](observations.md). They cover borsh docs,
  error-chain duplication, a 2x peak heap in `Ranked` decoding, generator
  rejection rates, and checks that rest on one or two tests.
- Coverage and the instrument inventory: [`coverage.md`](coverage.md).
- Resumption notes: [`NOTES.md`](NOTES.md).
