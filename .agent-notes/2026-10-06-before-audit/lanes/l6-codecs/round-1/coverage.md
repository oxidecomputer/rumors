# L6 coverage record: codecs and serialization adapters

Revision: `main` at `58285ca5`; explore branch `explore/l6-codecs`. All runs
on ox-east-1 through `on-illumos.sh` with `unset CARGO_TARGET_DIR`. Logs and
mutation scripts are in this directory.

## Instrument inventory (explore branch)

Every instrument below is explore-only and unbriefed, apart from the wasm32
probe, which the test brief adapts. Mutation counts are "caught by the
instrument / caught by the committed `before` suite".

### 1. Spec-codec differential, `crates/before/src/testing/l6_spec.rs`

Commits `ab622a92`, `ef5bc48b`, `2036c94c`, `e2e4d8dd`.

- **What it checks.** An independent codec for all six wire types, written
  from the documented formats with no production reader, validator, writer,
  or rank fold. It comprises bit packing, gamma and zigzag coding, lenient
  parsers that record each canonicity violation and its detection position, a
  rank codec, and step-function order, area, join, and meet.
- **Oracle.** Every production entry is compared with it:
  - slice `decode`;
  - text `FromStr`;
  - postcard (accept set);
  - CBOR (accept set, and an error message naming an applicable class);
  - borsh `deserialize_reader` on input plus junk (verdict and bytes consumed);
  - chunked and `Interrupted` readers;
  - JSON span records.

  The accept set must be exact, a rejection's class must be applicable, and
  span's documented precedence is enforced. A first-detected-event model is
  checked informationally. An encoder leg requires production `encode`,
  `join`, `meet`, `span`, `rank`, `Ranked::encode`, and `Clock::encode` to
  emit the spec's bytes.
- **Reach (measured, `reach-hist.log`, 20,000 samples each).** Version
  inputs:
  - depth above 32 in 11.8% (deep spines to 400 levels), against a maximum
    depth of 4 for the committed `arb_oracle_version`;
  - a planted negative running height in 22.3%;
  - a collapsible pair in 15.5%;
  - an incomplete tree (truncation) in 32.2%;
  - heights up to about 300 bits, with wide negatives.

  Party inputs: depth above 8 in 15.6%, against a committed maximum of 4; an
  owned-pair violation in 29.3%. Ranks: `rho` 64..69 headers, zero final
  groups, dirty padding, and fractions up to 3000 digits. The committed
  generators produce only canonical values, and canonicity violations come
  from single-bit flips, planted pairs, and point tests. Committed deep coverage
  lives in shape-specific tests, not in the arbitrary generators.
- **Calibration.** Mutations M1 to M17 (`mutate.py`, `mutate2.py`,
  `mutate3.py`), described in "Calibration" below. Caught 17/17 by the
  harness and 17/17 by the committed suite. The weakest committed coverage is
  M6 (1 test) and M13, M14, M15 (2 tests each).
- **Results.** 300,000 cases per type, twice (`spec-long1.log`, 423 s;
  `spec-long2.log`, 1940 s, deep shapes included): all pass, with zero
  first-event mismatches.

### 2. Adapter, reader, and writer probes, `crates/before/src/testing/l6_probes.rs`

Commit `7cb120ff`.

- **What it checks.** Serde data-model leniency (prints); every reader-taking
  decoder and borsh under short reads, `Interrupted`, and failure at call
  *k*, against the slice decode; every streaming encoder under writers that
  fail, stop at a limit, or interrupt (a prefix is written and the error is
  reported).
- **Reach.** No committed test feeds `Interrupted` or short reads to any
  decoder, or a failing writer to any encoder.
- **Calibration.** `Interrupted` read as end-of-input in `Rank::decode`'s
  first loop is caught by the probe (`calib-reader.log`). Not run against
  the committed suite.

### 3. Hostile resource families, `crates/before/tests/l6_resource.rs`

Commit `34e6cf2e`.

- **What it checks.** Peak heap per input byte, accumulator touches, and scan
  bits at four sizes (16 KiB to 1 MiB) for families the board cannot form:
  rank prefixes far wider than their version (`Ranked`, slice and borsh),
  truncated wide ranks, and a tiny prefix before a wide version. A wide-leaf
  `Version::decode` serves as the reference row.
- **Reach.** The board's canonical families never produce a rank prefix wider
  than its version.
- **Calibration.** None; it is a measurement, not an assertion. All ratios
  were constant across sizes.

### 4. Writer copy-path harness, `crates/before/src/testing/l6_writer.rs`

Commit `2036c94c`.

- **What it checks.** It builds a whole tree around a hole filled by a
  canonical multi-leaf subtree shifted by an offset. It feeds `VersionWriter`
  the leaves before the hole, the subtree's first leaf, `copy_remainder`, and
  the leaves after, and requires the spec's canonical bytes. Heights come from
  a tiny pool including `2^90`, so equal wide siblings collapse and force
  split output.
- **Reach (measured by entry-panic probes, `reach5.log`).** It reaches
  split-mode splicing (R31), depth-1 copies (R32), and wide-last copies (R33).
  So does the committed suite: 16, 57, and 26 tests respectively, through
  `tick.rs:794` and `tick/raise.rs:360`.
- **Calibration** (`mutate4.py`, `adequacy-writer.log`). W1 (copied right
  edge marked leaf-sibling) and W3 (narrow splice range one bit long) are
  caught by the harness and by 60 and 65 committed tests. W2 (stray flag in
  the early-return branch) survived both; the branch is unreachable
  (simplification brief).
- **Results.** 20,000 cases pass.

### 5. wasm32 probes, `crates/before/wasm32-pins/{protocol,guest,harness}`

Commits `679cc2e5`, `3854b4c1`, `431011b3`.

- **What it checks.** `RankDecodeGroups(g)` streams a lazily generated
  canonical `2^-(8g)` through `Rank::decode`; for `g <= 4096` it is checked
  against the in-memory `synthesis::unit_fraction`. `VecGrowthBoundary(n)`
  reports how a full `Vec<u8>` of length `n` refuses `try_reserve(1)`.
  `VersionBorshWideLeaf(k)` and `VersionReaderWideLeaf(k)` decode a lazily
  streamed one-leaf version of height `2^k` through borsh and through
  `Version::decode`.
- **Reach.** The committed `RankDecode` pin stops at `2^29` groups; this
  reaches the `2^30` doubling boundary on both sides.
- **Results.** It demonstrates the defect at two sites. The rank decode traps
  at `2^30 + 1` groups and passes at `2^30` (`wasm1.log`). The borsh
  version decode traps at `2^30 + 5` bytes and passes at `2^30`, while the
  reader decode of the same bytes returns `Io(OutOfMemory)` (`wasm2.log`).

### 6. Mutation and reach scripts

`mutate.py` (per-site environment variables), `mutate2.py`, `mutate3.py`,
`mutate4.py`, `reach5.py` (one cached `L6M=k`). Each was reverted after use,
with `git diff` empty afterwards.

## Calibration (mutations)

The spec harness was run against each of these mutations:

- M1: never reject a negative running height.
- M2: over-reject zero deltas beside internal siblings.
- M3: the party validator treats one-child branches as owned.
- M4: reject flush padding.
- M5: the admission walk reports Equal for dominating pairs.
- M6: accept `rho == 64`.
- M7: accept a zero final group.
- M8: skip the `Ranked` rank check.
- M9: off-by-one in `Party::decode_prefix`'s byte count.
- M10: skip borsh's marker check.
- M11: the gamma window proves one bit too many.
- M12: drop the writer's collapse cascade.
- M13: report the pair verdict before the padding check (now caught by the
  precedence check itself, `calib-m13b.log`).
- M14: the admission walk never rejects a collapsible pair at an ordinary close.
- M15: `finish` skips the closing ancestors.
- M16: the standalone validator never rejects a collapsible pair.
- M17: the party validator never rejects an owned pair.

Committed-suite baseline: 693/693 and 700/700, so no mutation-free failures.

## Leads, evaluated

1. **Error precedence across entry points.** No in-tree property compared
   rejection classes across serde entries (observation 10); borsh is compared
   against a reference sharing its validators. The spec harness compared all
   entries against independent verdicts: no disagreement in 3.6 million inputs.
2. **Misbehaving readers.** Only `Rank::decode` hand-writes read loops; the
   others use `read_to_end` or `read_exact`. No divergence; calibrated.
3. **Writers failing midway.** A prefix is written and the error is returned;
   the docs never say partial output may remain (observation 5).
4. **One-constraint violations.** Planted at every kind of position, in every
   type, wide and deep included. All rejected with applicable classes.
5. **Byte-count arithmetic and `usize`.** Every `usize` in the lane indexes or
   counts memory, except three bit counts of at most 64
   (`simplification-bit-count-types.md`). The width-dependent behavior lies in
   *growth*: infallible `Vec` growth panics past `2^30` entries on 32-bit
   (**defect**, demonstrated for `Rank::decode` and borsh decoding; other sites in the fix note).
6. **Text versus serde.** They agree by construction; the framing leniency is
   recorded as questions Q1 and Q2.

## Closed fixes, re-attacked

- Span collapsible pair mid-stream: holds (M14 and M15 caught; thin, at two
  committed tests).
- Padding, truncation, trailing, and precedence reconciliation: holds (exact
  first-event agreement).
- Serde data model: incomplete for framing (Q1, Q2).
- Serde and borsh on wide and deep inputs: holds semantically. On 32-bit
  targets, borsh's stream buffer panics past `2^30` bytes (demonstrated; part
  of the defect).
- `Rank::decode` without retaining its input: holds (1.50 heap bytes per
  input byte on truncated wide ranks). The fraction buffer is the 32-bit
  defect.

## Blind spots that remain

- The 32-bit growth panic is demonstrated for `Rank::decode`'s group buffer
  and borsh's stream buffer. The `BitSink`, `Ranked` borsh `rank_bytes`,
  `BitStack`, and `BitsWriter` sites are inferred.
- No dynamic 32-bit run of the spec harness (the box has no native 32-bit
  target); 32-bit evidence is the wasm probes plus the conversion inventory.
- Format-crate leniency (non-minimal CBOR or postcard headers) was not probed.
- Postcard drops custom error messages, so through postcard only accept
  versus reject is compared.

## How to resume

Spec harness:
`on-illumos.sh <wt> 'unset CARGO_TARGET_DIR; export NEXTEST_TEST_THREADS=24; L6_CASES=300000 nice -n 10 cargo test -p before --all-features --locked --lib -- l6_spec --test-threads 8 --nocapture'`.

wasm probes: the command in the defect record.

Mutations: `python3 mutateN.py apply <wt>/crates/before/src`, run with `L6M=k`,
then `revert`, then check `git diff` is empty.

Next ideas, in order of expected yield:
- Demonstrate the `BitStack` instance with a deep party spine of about 1 GiB.
