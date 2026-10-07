# Machinery brief: reach `Rank::decode`'s rejection paths across its 64-byte read chunk

## The failure class it catches

`Rank::decode<R: Read>` (`crates/before/src/rank.rs:366-428`) reads a fixed
64-byte prefix (`DECODE_CHUNK_BYTES`, rank.rs:596), decodes it through the
slice path when the rank ends inside it, and otherwise continues through an
incremental path. Branch coverage of the whole `before` suite
(coverage/before__rank.txt) shows these documented return arms never run:

| line | arm | contract it implements |
|---|---|---|
| 391 | rank ends exactly at byte 64, then the reader yields more data: `TrailingBits` | rejects bytes after the closing marker |
| 409 | rank longer than 64 bytes, reader hits EOF before it ends: `Truncated` | rejects truncation |
| 425 | rank longer than 64 bytes ends exactly at a refill boundary, then more data: `TrailingBits` | rejects trailing bytes |
| 378, 392, 414, 426 | `ErrorKind::Interrupted`: retry | `std::io::Read` contract |
| 393, 415, 427 | reader error at those points: `Decode::Io` | "`Decode::Io` when the reader itself fails" (rank.rs:341) |

A line that never runs cannot fail a test, so every defect confined to those
arms is undetectable today. Constructible examples that the current suite
passes:

- line 391 `Ok(_) => return Err(Decode::TrailingBits)` replaced by
  `Ok(_) => return Ok(rank)`: `Rank::decode` accepts a 64-byte rank followed
  by garbage, violating strict decoding and the canonical-encoding claims
  `Ranked` and the order-preserving key rely on.
- line 409 returning `Err(Decode::TrailingBits)`: a truncated long rank
  reports the wrong error.
- dropping an `Interrupted` arm: an interrupted read turns into a spurious
  `Decode::Io`.

## Why the existing instrument misses it

`rank_decode_is_independent_of_reader_chunks` (`crates/before/src/version/tests.rs:1353-1376`)
compares a slice reader with a one-byte reader and claims to exercise "both
the fixed-prefix path and the incremental path, including acceptance and
every structural rejection". Two facts make the claim weaker than stated:

1. The prefix loop (rank.rs:368-381) keeps reading until it holds 64 bytes or
   sees EOF, so a slice reader and a one-byte reader take the same path
   through the decoder for every input. The comparison is mostly the
   function against itself.
2. Its inputs never place a rank boundary at or past byte 64 with data after
   it, and never truncate a long rank: random byte strings (weight 3)
   self-delimit within a few bytes, and seeded rank encodings (weight 1) are
   used whole, with no suffix and no truncation.

No reader in the suite returns `Interrupted`.
`rank_decode_preserves_late_reader_errors` covers one `Io` arm (379, a
failure after a short complete rank).

## Proposed machinery

Extend `rank_decode_is_independent_of_reader_chunks` in place (same file and
module), and correct its doc comment to what the strengthened test proves.

- **Oracle.** Compare every reader-based verdict with
  `Rank::decode_bytes(&whole_input)` (rank.rs:433), the in-memory decoder:
  it decodes the entire input in one pass and rejects any byte after the
  rank, sharing none of the chunk and refill logic. Equal values on success,
  and equal `Decode` variants on failure.
- **Generator: encodings of controlled length.** Build ranks directly with
  `Rank::from_raw(num, exp)` (rank.rs:479, crate-private; normalized when
  `num` is odd): integral part `0`, `exp = 8k - j` with `j in 0..8`, and an
  odd `num < 2^exp` drawn from random bytes so the fraction bits are dense.
  Such a rank encodes in `ceil((exp_groups * 9 + 2) / 8)` bytes, where
  `exp_groups = ceil(exp / 8)`: one header bit for the zero integral part,
  nine bits per fraction group, one closing bit. Choose `k` so lengths
  straddle each refill boundary: 63, 64, 65, 127, 128, 129 bytes. Also keep
  the existing random-bytes and seeded-rank arms.
- **Input shapes per rank**, chosen by the strategy, not filtered: the exact
  encoding; the encoding followed by a nonempty suffix of 1 to 70 bytes
  (including all-zero suffixes); and a strict prefix of every length class
  (cut inside the first 64 bytes, at exactly 64, and past it).
- **Readers**: the slice; `OneByteReader`; an `InterruptingReader` that
  returns `ErrorKind::Interrupted` before every successful read; and a
  `FailingAfter(k)` reader that yields the first `k` bytes and then an error
  of kind `Other`.
- **Assertions**:
  1. For the slice, one-byte, and interrupting readers:
     `Rank::decode(reader)` equals `Rank::decode_bytes(input)`, value and
     error variant.
  2. Explicitly, so the oracle cannot drift silently: the exact encoding
     decodes to the generated rank; any nonempty suffix gives
     `Decode::TrailingBits`; any strict prefix gives `Decode::Truncated`.
  3. For `FailingAfter(k)` with `k <= input.len()`: `Decode::Io` with kind
     `Other`. The decoder always reads past the rank to prove EOF, so it
     always meets the failure.

## Calibration evidence

Coverage is the evidence: lines 391, 409, 425 and the `Interrupted` and late
`Io` arms are never executed by the current suite (Mac llvm-cov run,
coverage/NOTES.md). A builder should confirm the extended property reaches
each listed line, for instance by temporarily replacing each arm with a
wrong verdict (the three constructions above) and seeing the property fail
for each, then restoring by reversible string swap.

## Budget

Each case decodes at most a few hundred bytes four ways; the default 256
cases stay far inside nextest's 180 s limit.
