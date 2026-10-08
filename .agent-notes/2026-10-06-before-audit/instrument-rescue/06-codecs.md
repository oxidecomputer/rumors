<!-- CAVEAT LECTOR: written by Claude (Opus 5.5) as the instrument-rescue cataloguer for the codecs lane (L6), from the explore branch, the lane's records, and two runs on ox-east-1; for Finch's review. -->

# Instrument rescue: the codecs lane (L6)

## Lane summary

**Source.** `explore/l6-codecs`, tip `431011b3c`, base `58285ca51`: nine
commits, eleven files, 3,020 added lines (verified, `git diff --stat`). I
read every added file whole. The codec source those files exercise is
identical between the base and `main` (`434cbfc82`); only `wasm32-pins`
moved, by one suanpan landing case (verified, `git diff --stat 58285ca51
main`). So every reading below that the lane took at the base still
describes `main`'s codecs.

**What the lane built.** An independent specification of the six wire
formats, driven against every decode entry point; generators of near-valid
encodings; an encoder-agreement property; probes of misbehaving readers and
failing writers; serde framing probes; a harness for the version writer's
subtree copy; hostile resource families; four wasm32 probes with two lazy
stream fixtures; and, outside the branch, mutation and reach scripts.

**What already reached a ready branch.**

- The JSON positional-array probe (`probe_json_composites_accept_arrays`)
  is subsumed by #51's leniency property on
  `fix/before-serde-record-framing` (verified, #51's entry and diffstat).
- The lane's defect, simplification, and documentation briefs reached #53,
  #67, and #85. None of those branches carries an L6 instrument (verified,
  their entries and diffstats).

**Three facts to weigh before ranking.**

1. *A polished form of the wasm32 probes is slated for deletion.* The
   reviewed rewrite of the two wasm32 probes worth keeping (entries 2 and
   11) is commit `8208efaeb` on `fix/before-wasm32-buffer-growth`. #53's
   entry says the coordinator deletes that branch at retirement unless you
   keep it (verified).
2. *The mutation and reach scripts exist only in the session scratchpad.*
   `mutate.py` through `mutate4.py` and `reach5.py` live in
   `/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/auditor-l6/`,
   which does not survive a reboot. The lane's directory under
   `lanes/l6-codecs/round-1/` holds only the Markdown records (verified,
   `ls`). Entries 10 and 14 cover them.
3. *The survey's account of the spec harness and the two padding survivors
   needs correcting.* Section 2.1 of `instrument-survey.md` says the
   harness enforces only span's documented precedence and so does not catch
   the survivors at `span/wire.rs:136` and `party/io.rs:29` (`>` to `>=`).
   Its reasoning is wrong, and its conclusion holds only in part. The
   harness enforces more than span's precedence:
   several of its verdicts admit only the first-detected class, which is
   stricter than `Decode`'s documented contract. On the input that
   distinguishes each survivor, a lone first field that fills the buffer
   and carries a dirty padding bit, the harness rejects the survivor's
   `Truncated` report (verified, my census, by calling the harness's own
   `judge`). Its generators produce that input for 1 clock input in 703 and
   1 span input in 10,000, so a default run of 2,000 cases would catch the
   clock survivor about 94% of the time and the span survivor about 18%
   (inferred from the census rates). Entry 1 gives the mechanism, and why
   catching them pins a precedence you have not promised.

**Baseline.** `instrument-rescue/00-baseline.md` did not exist when I wrote
this. Reach comparisons use the adequacy lane's census of the committed
generators (`lanes/l8-adequacy/round-1/census-baseline.txt`) and my reading
of `main`. The census's headline numbers: `arb_oracle_version` reaches depth
4 at most, 21 nodes, and heights of up to 514 bits (median 128);
`arb_oracle_party_nonempty` reaches depth 4 and 26 encoded bits (verified,
that file).

**Runs.** Two of the three allowed, both on ox-east-1 in my scratch
worktree `/Users/oxide/src/rumors-rescue-l6` at `431011b3c`, with one
uncommitted census test appended to `l6_spec.rs`: a `--no-run` build, and
the census. Logs: `run1-build.log` and `run2-census.log` in the session
scratchpad's `rescue-l6/` directory.

**Provenance marks.** *Verified*: I checked it against the code, a log, or
my run. *Reported*: a lane record says so and I did not re-check it.
*Inferred*: my reasoning from the code, not checked by a run.

**Count.** Sixteen full entries and one one-line entry, ordered by my
judgment of value.

---

## 1. Specification-codec decode differential

- **What it is.** An independent model of the six wire formats with a
  property driver. `explore/l6-codecs:crates/before/src/testing/l6_spec.rs`
  at `431011b3c` (commits `ab622a925`, `ef5bc48b2`), 2,053 lines in all:
  about 1,010 lines of model (bit packing, sealing, Elias gamma, zigzag,
  tree and rank encoders, lenient parsers, slice and stream verdicts), 400
  of generators (entry 5), 390 of driver, and 120 of reach diagnostics
  (entry 12). Six tests, `l6_spec_version`, `_party`, `_rank`, `_clock`,
  `_span`, `_ranked`, at 2,000 cases each by default (`L6_CASES`)
  (verified, reading).
- **What it reaches or checks.**
  - *The oracle.* For each input, the model computes a verdict: the value
    it accepts, or the set of applicable error classes, the class a
    sequential parse detects first, and, for spans, the classes permitted
    by the documented rule "Structural encoding errors take precedence over
    endpoint ordering" (`span/wire.rs:94`).
    The verdict code calls no production reader, validator, writer, or rank
    fold (verified, the module's imports and every call in lines
    25–1037). Production enters as the subject, and through
    `bridge::to_oracle_*` only to read back an accepted value.
  - *The entry points.* Every input meets: the slice `decode`; the same
    decode through a chunked reader that injects `Interrupted` (entry 4's
    `Chaos`, without its failure injection); the hexadecimal `FromStr`
    (party and version);
    postcard (accept set); CBOR (accept set, and an error message that
    names an applicable class); borsh `deserialize_reader` on the input
    followed by up to five junk bytes (verdict and bytes consumed, against
    an independent stream model); and, for spans, the JSON record form,
    accepted exactly when the endpoints are ordered. An accepted value must
    equal the model's, and an accepted version must adopt the input bytes
    as `as_bytes` (verified, lines 1756–1934).
  - *Inputs.* From entry 5's generators. Of 20,000 version inputs, 11.8%
    parse to a tree deeper than 32 levels, 15.5% hold a collapsible pair,
    22.3% a negative running height, and 32.2% end mid-tree (verified,
    `reach-hist.log`). Of 20,000 party inputs, 14.6% parse deeper than 8
    and 29.3% hold an owned pair (verified, same log; the coverage record
    says 15.6% for depth, which the log does not support). Heights are no
    wider than the committed generator's (verified, same log). My census
    adds (verified, `run2-census.log`): a span's lower endpoint is deeper
    than 32 in 18.4% of 480,000 span inputs; span's documented precedence
    actually narrows the permitted classes in 6.1% of them; and 18.9% of
    40,000 rank inputs are longer than the decoder's 64-byte prefix, 10.2%
    accepted.
  - *Where the verdict is stricter than the contract.* `Decode`'s rustdoc
    says: "An input may have more than one defect. Unless a decoder
    documents a precedence rule, callers should handle any applicable
    variant" (verified, `error.rs` at `main`). The model honors that for a
    single tree read from a slice, and for a span's upper endpoint. It
    admits only the first-detected class in three places (verified,
    reading): the first field of a clock or span (`prefix_stage` returns
    that field's classes and stops), every borsh stream verdict
    (`stream_tree` and its callers return one class), and a rank header
    past the format bound (`NotCanonical` alone). Production agrees with
    those narrower verdicts today (reported: zero first-event mismatches
    in 3.6 million inputs). Folded in as is, these verdicts would pin
    today's detection order. No decoder documents that order, and #67's
    "What to weigh" leaves documenting one to you.
  - *Predicates no committed test states* (verified, by reading `main`'s
    serde, borsh, version, and span tests): that each decoder's accept set
    equals an independently written specification's, beyond single-bit
    flips of depth-4 encodings; that postcard, CBOR, text, and borsh accept
    exactly what the slice decoder accepts; that `Party`, `Version`,
    `Clock`, `Span`, and `Ranked` decode identically through chunked and
    interrupting readers (committed: `Rank` only, one byte per read); that
    borsh consumes exactly the specified byte count against a model sharing
    no validator (the committed borsh differential's references call
    `validate::from_reader` and `validate::prefix_from_reader`).
- **Coverage beyond the committed suite.**
  - The committed rejection tests check production against itself, or
    against re-encoding through the bridge: planted collapsible pairs,
    single-bit flips (exhaustive at depth 2, arbitrary at depth 4), and
    truncation sweeps (verified, `version/io/tests.rs`). None generates a
    non-canonical tree deeper than 4 at random. None checks, beyond those
    small depths, that a rejected input deserved rejection: an
    over-strict validator that refused only some deep canonical encodings
    would be caught only if a committed deep-shape test happened to build
    one (inferred). This harness checks both directions on deep inputs.
  - The two padding survivors (`>` to `>=` at `span/wire.rs:136` and
    `party/io.rs:29`) return `Truncated` instead of `TrailingBits` when a
    lone first field fills the buffer and carries a dirty padding bit
    (reported, the adequacy lane's brief
    `test-span-lone-endpoint-padding.md`; consistent with the code). On
    that input the model's `prefix_stage` admits `{TrailingBits}` alone.
    My census (verified, `run2-census.log`, 480,000 inputs per type):
    - clock: 683 such inputs (1 in 703); production returns
      `TrailingBits` on all 683, and `judge` rejects a `Truncated` report
      on all 683;
    - span: 48 such inputs (1 in 10,000), with the same two results.

    So a default run of 2,000 cases catches the clock survivor with
    probability about 0.94 and the span survivor about 0.18; a
    300,000-case run catches both (inferred from these rates; no mutant was
    run). Read this as the narrowed verdict above at work, not as a
    contract the survivors break: `Truncated` is also applicable there,
    since the second field is missing.
- **Evidence.**
  - Calibration (reported, with logs I checked): all 17 of the lane's
    mutants (entry 10) fail it. The committed suite also fails all 17; the
    weakest committed coverage is one test for M6 (accept a rank header of
    `rho == 64`) and two each for M13, M14, and M15 (verified,
    `adequacy.log`, `adequacy2.log`).
  - Long runs: 300,000 cases per type, twice, the second with deep shapes
    (reported; `spec-long2.log` shows all seven tests passing in 1,940 s,
    verified). It caught no defect.
- **Fold-in cost.**
  - *Model.* Port as a test-only `testing::oracles::codec` (keep it under
    `#[cfg(test)]`; `oracles::tree` is public under the `oracle` feature).
    Eight helpers recurse on tree depth (`SV::normalize`, `zip`, `le`,
    `SP::normalize`, `drop_empty`, `from_oracle`, `sv_to_oracle`,
    `sp_to_oracle`); `crates/before/AGENTS.md` requires them to be iterative
    or to descend through `recurse::descend!`. Drop the module-wide
    `#![allow(dead_code)]`.
  - *Driver.* Replace the hand-built `TestRunner`, which has no failure
    persistence and a fresh seed per run, with `proptest!` so failures
    leave committed seeds; drop `L6_CASES`; give the six tests their
    missing doc comments; decide the narrowed verdicts (widen them to every
    applicable class, or document a precedence first).
  - *Cheaper partial fold-ins.* The cross-entry legs need no model: one
    property that requires text, postcard, CBOR, borsh, and chunked readers
    to agree with the slice decode, over entry 5's generators, would keep
    the entry-point agreement for perhaps 150 lines (my estimate). The
    `diff_ops` table
    does not fit decoding, since its descriptors compare value operations
    across three implementations (verified, its module doc).
  - *Runtime.* At 2,000 cases, all eleven `l6_` library tests finished in
    10.31 s together (verified, `spec2.log`, libtest, debug), far inside
    nextest's 180-second limit.
  - *Dependencies.* None new: `hex` and `serde_bytes` are dependencies of
    `before`, and `postcard`, `ciborium`, `serde_json`, and `borsh` are
    dev-dependencies (verified, `Cargo.toml`).
  - It needs a validation-index entry; the index says only that codecs are
    tested "against canonical bytes" (verified).
- **Overlaps.** Entry 4's reader probe and #78's scripted reader (both
  cover reader chunking for `Rank`). The algebra lane's leaf-list model has
  its own canonical encoders, `encode_version` and `encode_party`
  (verified, `explore/l2-algebra:crates/before/tests/l2_probe/model.rs`),
  but no decoders that I found. It would stand beside the committed
  rejection tests and borsh differential as a second, independent check,
  replacing none: the borsh differential asks a different question (the
  windowed cursor against a per-bit cursor).
- **Dependencies.** None on unlanded branches. #53, #67, and #78 change
  code the harness exercises without changing any verdict it expects
  (inferred from their entries).
- **Value, in one sentence.** It is the only check of every decoder's
  accept set and error classes, through every entry point, against a
  specification written from the documentation, on deep and non-canonical
  inputs the committed generators never draw; folding it costs a port of
  about 1,800 lines and a decision about precedence.

## 2. wasm32 borsh probe over a wide one-leaf version

- **What it is.** A wasm32 boundary probe and a lazy stream fixture.
  `explore/l6-codecs:crates/before/wasm32-pins/`, commits `3854b4c16` and
  `431011b3c`: `Check::VersionBorshWideLeaf` (protocol), guest
  `version_borsh_wide_leaf`, the fixture `synthesis::WideLeafReader`
  (about 45 lines; a `Read` that yields one leaf of height `2^k`,
  `ceil((2k + 3) / 8)` bytes, with no backing buffer), three harness tests
  that print rather than assert, and the guest manifest change that turns
  on `before`'s `borsh` feature (lockfile gains `borsh` 1.6.1 and
  `cfg_aliases` 0.2.1, the main workspace's versions) (verified, reading).
- **What it reaches or checks.** A borsh `Version::deserialize_reader` on
  a 32-bit target, over streams of up to `2^30` bytes, so the stream
  cursor's bit positions reach about `2^33`. It checks the decoded live
  length, `2k + 2` bits. No independent oracle is needed: the input is
  constructed canonical. Readings: `k = 1000` passes; `k = 2^32 - 2`
  (exactly `2^30` bytes) passes in 148 s; `k = 2^32 + 16` traps in 142 s
  (verified, `wasm2.log`).
- **Coverage beyond the committed suite.** At `main` the guest builds
  `before` with `meter` only (verified, guest `Cargo.toml`), so no committed
  check compiles any borsh code for a 32-bit target. In this decode the
  gamma code is too long for the word window, so `StreamBitsReader::read_bit`
  turns every position up to about `2^33` into a byte index at
  `borsh_impls.rs:107`, `(self.position / 8) as usize`. A narrowing written
  as `(self.position as usize) / 8` would read bit `2^32` as bit 0, the
  leaf flag, and the marker as a zero, so the decode would fail with
  `TrailingBits` (inferred from the stream layout; not run). #92 records
  that the gamma window, the site of one narrowing no wasm32 check catches,
  compiles only with `borsh` or tests while the guest builds without
  `borsh`, and it lists a borsh check in the guest as one of two things a
  check on narrowed *starting* positions needs. This probe supplies the
  guest-side `borsh`. It does not close that gap: its reads are sequential,
  and the gamma window runs only once, at position 1, before the slow path
  takes over (inferred, `read_gamma`).
- **Evidence.** It demonstrated the second site of the 32-bit growth panic,
  which your final ruling (no size promises) dissolved. It has caught no
  narrowing; none was injected.
- **Fold-in cost.** The reviewed rewrite exists:
  `version_borsh_decode_streams_bytes_at_the_doubling_limit` on
  `8208efaeb` asserts `Passed`, the exact live length, and the exact bytes
  through a streaming comparison (verified, `git show 8208efaeb`). Its
  doc comments cite a "documented 32-bit buffer limit" that you withdrew,
  so they need restating as a bit-position check, and its past-limit
  companion should go. It needs a new protocol variant, which lands after
  #58 derives the protocol decoders from their numbering (notice 96).
  Runtime is about 150 s per case under
  load (verified, `wasm2.log`, release); the wasm32-pins workspace has no
  nextest time limit at `main`, and #57 gives it 20 minutes. A leaf just
  past `2^32` bits (`k` near `2^31`, about 512 MiB) would cross the same
  boundary at roughly half the cost (inferred). New dependencies: `borsh`
  in the guest only.
- **Overlaps.** Entry 11 (same fixture style, rank path) and entry 13 (the
  same stream through `Version::decode`). The committed
  `version_decode_crosses_the_usize_position_boundary` covers the slice
  decoder's positions, not borsh's.
- **Dependencies.** #58 for the protocol variant; the survival of
  `fix/before-wasm32-buffer-growth` if you want the reviewed form.
- **Value, in one sentence.** It is the only instrument that runs a borsh
  decode on a 32-bit target, past the `2^32` bit-position boundary in
  borsh's stream cursor, and a reviewed form of it exists on a branch
  slated for deletion.

## 3. Hostile resource families for `Rank` and `Ranked` rejection

- **What it is.** A resource measurement.
  `explore/l6-codecs:crates/before/tests/l6_resource.rs`, commit
  `34e6cf2ee`, 130 lines: an integration test with `PeakAlloc` as the
  global allocator that prints, for five hostile families and one reference
  row at four sizes (16 KiB to 1 MiB), the peak heap per input byte,
  accumulator touches, and scan bits.
  It asserts nothing (verified, reading).
- **What it reaches or checks.** Families: a `Ranked` key whose rank prefix
  is an integral `8n` bits wide before an empty version (slice and borsh);
  a long-fraction prefix; a truncated wide rank; a tiny prefix before a
  wide one-leaf version; and that version alone as the reference row.
  Readings were flat across sizes: 5.00, 6.00, 3.67, 1.50, 5.00, and 2.50
  heap bytes per input byte, with touches and scan bits growing linearly
  (verified, `resource1.log`).
- **Coverage beyond the committed suite.** The board has rejection rows for
  `Version`, `Span`, `Party`, and `Clock` decoding, and none for `Rank` or
  `Ranked` (verified, the operation names in `board/ops.rs`); its `Ranked`
  rows decode canonical keys only. So a regression that made a
  mismatched-prefix or truncated-rank rejection superlinear in heap or scan
  would pass every committed cost instrument.
- **Evidence.** No finding. Observation 6 (a `Ranked` decode builds the
  prefix rank and discards it) rests on these readings. #53's rank decoder
  changes these paths, so the readings are pre-#53 (inferred).
- **Fold-in cost.** As board rows beside "the rejection surface" in
  `board/ops.rs`: for example `rank_decode_truncated`,
  `ranked_decode_mismatch`, and `ranked_borsh_deserialize_mismatch`, with
  floors from `rejection_floors` and the existing
  `RANK_DESERIALIZE_*` and `RANKED_DESERIALIZE_*` heap ceilings. The
  mismatched prefixes need constructing per family; the board's `defect`
  module holds such "invalid inputs used to measure rejection costs"
  (verified, its module doc). Each new row also needs its entry in the
  worst-case ranking table, `WORST_RANKINGS`. The probe ran all 24
  measurements in 0.75 s (verified,
  `resource1.log`). No new dependencies.
- **Overlaps.** None in other lanes that I know of. It would sit beside
  the board's canonical `rank_*` and `ranked_*` rows.
- **Dependencies.** Land after #53, whose rank decoder moves these
  readings and lowers `RANK_DESERIALIZE_HEAP_BYTES_PER_INPUT_BYTE` to 5.0.
- **Value, in one sentence.** It closes the one decoder rejection path the
  board does not meter, with families already constructed, and would lock
  #53's heap improvement in on that path too.

## 4. Misbehaving-reader probe

- **What it is.** A metamorphic property and a reader double.
  `explore/l6-codecs:crates/before/src/testing/l6_probes.rs`, commit
  `7cb120ffe`: the `Chaos` reader (about 45 lines) and
  `probe_reader_entries_ignore_chunking`, 512 cases (verified, reading).
- **What it reaches or checks.** `Chaos` returns at most 1 to 79 bytes per
  read, returns `Interrupted` on every second to fourth call or never, and
  can fail once at call 1 to 11. For values from the committed generators,
  each encoding unmodified, cut by a byte, extended by a byte, or
  bit-flipped, it requires every reader-taking decoder (`Party`, `Version`,
  `Clock`, `Span`, `Rank`, `Ranked`) to return through `Chaos` what it
  returns from a slice, and a failing reader to give `Io(Other)` or the
  slice's verdict. Borsh must reach the same verdict and consume the same
  bytes through `Chaos` as through a whole reader. One draw in two makes a
  rank long enough to pass `Rank::decode`'s 64-byte prefix. The oracle is
  production's own slice decode.
- **Coverage beyond the committed suite.** At `main`, no test returns
  `Interrupted` from any reader, and only `Rank::decode` meets a chunked
  reader (one byte per read) or a failing one (verified, grep). #78 extends
  `Rank` with an exact oracle. This probe is the only check of the other
  five decoders and all six borsh impls under short reads, `Interrupted`,
  and failure. Those decoders delegate to `read_to_end` and `read_exact`
  today (reported), so the probe guards against a future hand-written read
  loop rather than a known gap.
- **Evidence.** Turning `Interrupted` into end-of-input in `Rank::decode`'s
  first read loop fails it (verified, `calib-reader.log`). It was not run
  against the committed suite or the adequacy lane's survivors. Its
  failing-reader oracle accepts either outcome, so it shares the weakness
  #78's reviewer found in round 1 (a decoder that swallows a reader error
  after its verdict is settled would pass) (inferred). It caught no
  defect.
- **Fold-in cost.** Generalize #78's scripted reader and exact oracle to
  the other five decoders and borsh, rather than adding `Chaos` beside it.
  About 100 lines. Runtime 0.83 s (verified, `probes2.log`). No new
  dependencies; the test has a doc comment.
- **Overlaps.** #78 (`Rank` only, stronger oracle); entry 1's chunked leg
  (same `Chaos` reader, no failure injection, spec-generated inputs).
- **Dependencies.** #78, which stacks on #53.
- **Value, in one sentence.** A cheap extension of #78's reader discipline
  to every decoder and to borsh, protecting reader entry points that are
  simple today against a future hand-written loop.

## 5. Near-valid encoding generators

- **What it is.** Strategies producing encoded byte strings.
  `explore/l6-codecs:crates/before/src/testing/l6_spec.rs` lines 1146–1546
  at `431011b3c`, about 400 lines: `arb_version_input`, `arb_party_input`,
  `arb_rank_input`, `arb_clock_input`, `arb_span_input`, and
  `arb_ranked_input`, built from model trees and from the `Edit`, `Seal`,
  `ByteEdit`, and `RankEdit` edit families (verified, reading).
- **What it reaches or checks.** Model trees can hold what production
  types cannot: negative running heights and collapsible pairs. Shapes are
  bushy four times in five (recursion depth up to 6 for versions and 7 for
  parties, against the committed generators' 4) and left- or right-leaning
  spines of 30 to 399 levels once in five. Heights mix small
  values, `u64` values, the `2^64` neighborhood, `2^30` to `2^300` plus or
  minus 2, and negatives. Edits flip, cut, insert, or delete a live bit;
  seals omit the marker, add junk after it, add or drop a byte, or pack
  raw; byte edits then flip a bit, cut the buffer, or append one to three
  bytes. Rank edits add header widths 64 to 69, an all-zero final group, or
  dirty padding; fractions reach 3,000 digits. Span pairs are ordered,
  equal, crossed, or arbitrary; `Ranked` keys pair a version with its own
  or another version's rank. Reach numbers are in entry 1, including my
  census's span depth (18.4% of lower endpoints deeper than 32) and rank
  length (18.9% longer than 64 bytes).
- **Coverage beyond the committed suite.** The committed generators produce
  canonical values only, at depth 4 or less (verified, the census and
  `generators.rs`). These are the only source of structurally whole but
  non-canonical encodings at random, and of deep spines of either kind.
- **Evidence.** Through entry 1, all 17 mutants. On their own they check
  nothing.
- **Fold-in cost.** Into `testing::generators`, with the model trees they
  build from (entry 1's model, or a reduced tree type). Recursion and
  `allow(dead_code)` as in entry 1. A census floor per regime would keep
  them from silently narrowing (entry 12). They also serve without the
  model: a totality property (no decoder panics on any of them) and the
  cross-entry property in entry 1's cheaper fold-in.
- **Overlaps.** The events lane's co-generated pairs (#74) and the spans
  lane's shaped generators reach depth too, but produce values, not
  non-canonical encodings (reported, `instruments.md`).
- **Dependencies.** None.
- **Value, in one sentence.** The only generators of deep and non-canonical
  encodings; worth folding with entry 1, and usable on their own for
  totality and entry-point agreement.

## 6. Failing-writer probe

- **What it is.** A property and a writer double.
  `explore/l6-codecs:crates/before/src/testing/l6_probes.rs`, commit
  `7cb120ffe`: the `Sink` writer (about 35 lines) and
  `probe_encoders_under_failing_writers`, 256 cases (verified, reading).
- **What it reaches or checks.** `Sink` accepts 1 to 39 bytes per write,
  returns `Interrupted` on every second to fourth call or never, and stops
  at a limit drawn from 0 to 1.2 times the encoding's length, either with
  an error or by returning `Ok(0)`. Over eleven encoders (`encode_to` for
  all six types, `encode_rank_to` for `Ranked` and `Version`, and borsh
  `serialize` for `Clock`, `Span`, and `Ranked`), it requires: the bytes
  written are a prefix of the canonical encoding; with room, the result is
  `Ok` and the bytes are exact; without room, the result is an error, of
  kind `Other` when the writer failed. The oracle is production's own
  `encode`.
- **Coverage beyond the committed suite.** At `main`, the only test writer
  is `OneByteWriter`, which accepts one byte per call and never fails or
  interrupts (verified, `version/tests.rs`). So an encoder that dropped a
  writer's error, or a hand-written write loop that gave up on
  `Interrupted`, would pass every committed test as long as its bytes were
  right (inferred).
- **Evidence.** It passes (verified, `probes2.log`). It was never
  calibrated, and caught nothing.
- **Fold-in cost.** About 80 lines beside
  `composite_streaming_encoders_match_their_buffered_forms`. The "a prefix
  was written" assertion states something no `# Errors` section promises
  (observation 5, recorded in `follow-ups.md` as an undocumented
  contract), so either document it first or assert only error propagation
  and exactness with room. Runtime 0.25 s (verified, `probes2.log`). No new
  dependencies. Borsh `serialize` for `Party`, `Version`, and `Rank` is
  absent; the first two are one `write_all` of the stored bytes (verified,
  `borsh_impls.rs`).
- **Overlaps.** The committed `OneByteWriter` tests (partial writes only).
- **Dependencies.** None; the partial-write wording waits on your call.
- **Value, in one sentence.** The only check that encoders report a
  writer's failure and survive `Interrupted`, at the cost of a small test
  and one documentation decision.

## 7. Encoder agreement with an independent pointwise model

- **What it is.** A property. `l6_spec_encoders_agree_with_production` in
  `explore/l6-codecs:crates/before/src/testing/l6_spec.rs` (lines
  1715–1754), using entry 1's encoders and the model's pointwise `zip`
  (verified, reading).
- **What it reaches or checks.** For two model versions, a model party, and
  a model rank, production must emit the model's exact bytes for: the
  version itself (built through the bridge), the join and the meet
  (against the pointwise maximum and minimum, normalized), the span, the
  rank, the `Ranked` key, the party, the clock, and `Rank::from_raw`. One
  operand in five is drawn as a deep spine of 30 to 399 levels. After
  normalization, 19.5% of 40,000 operands are deeper than 32 and 14.5%
  deeper than 128 (verified, `run2-census.log`), so about a third of cases
  join or meet at least one deep operand (inferred). Ranks reach 3,000
  fraction digits. The model shares no code with production or with the two
  committed oracles, and writes its bits without production's
  `BitsWriter`, which the bridge's own encoder uses (verified,
  `bridge.rs`).
- **Coverage beyond the committed suite.** `diff_ops` checks join and meet
  against the tree and function oracles on depth-4 values and traces, and
  the snapshot and rank-battery tests fix bytes for chosen values
  (verified, reading). This property is the only one that compares
  production's bytes with an encoder written from the format description,
  at random, including joins of deep spines.
- **Evidence.** Of the 13 mutants calibrated against the harness's
  separate legs, M12 (the writer skips its collapse cascade) fails this
  test and no other leg of the harness (verified, `calib-spec2.log`).
  The committed suite also kills M12, with 58 failing tests (verified,
  `adequacy.log`). No defect.
- **Fold-in cost.** Small once entry 1's encoders exist: one property.
  It could also become a `diff_ops` descriptor whose oracle side encodes
  the tree oracle's value with the model encoder, so every population meets
  it, though the table's populations lack the deep spines. Runtime: 2.2 s
  at its default count before the deep shapes were added (verified,
  `calib-spec2.log`, nextest); unmeasured since.
- **Overlaps.** The algebra lane's leaf-list model, with its own encoders,
  random topology to depth 300, and correlated pairs that force collapse
  cascades (reported, `instruments.md`). That model reaches bushier deep
  trees than this property's spines; one of the two suffices.
- **Dependencies.** Entry 1's model.
- **Value, in one sentence.** An independent byte-level check of every
  encoder and of the join and meet writers on deep spines, largely
  duplicated by the algebra lane's model.

## 8. Writer copy-path harness

- **What it is.** A property over internal writer APIs.
  `explore/l6-codecs:crates/before/src/testing/l6_writer.rs`, commits
  `679cc2e51` and `2036c94c7`, 166 lines; `l6_writer_copy_matches_spec`,
  4,000 cases by default (verified, reading).
- **What it reaches or checks.** It builds a tree with one hole, fills the
  hole with a canonical multi-leaf subtree shifted by an offset, and feeds
  `VersionWriter` the leaves before the hole, the subtree's first leaf,
  `VersionSubtree::copy_remainder`, and the leaves after. The result must
  equal the model's canonical bytes for the whole tree. Heights come from
  `{0, 1, 2, 2^90, 2^90 + 1}`, so equal wide siblings collapse and push the
  writer into split output. Entry-panic probes show it reaches split-mode
  splicing, depth-1 copies, and copies whose last code is wide (reported,
  coverage record).
- **Coverage beyond the committed suite.** Its oracle is independent of the
  writer. The committed copy tests compare a splice against per-leaf
  feeding through the same `VersionWriter`, on chosen shapes (verified,
  `copy_subtree_remainder_matches_per_leaf_feeding` in
  `version/io/writer/tests.rs`). So a defect that both feeding paths share
  would pass those tests, though the wider committed suite catches M12, a
  writer defect of that kind, with 58 failing tests (verified,
  `adequacy.log`). Its reach is not new: the committed suite reaches the
  same three paths in 16, 57, and 26 tests (verified, `reach5.log`
  summaries).
- **Evidence.** W1 and W3 fail it, and 60 and 65 committed tests also fail
  them. W2 survives both, because the branch it mutates cannot run; #67
  deletes that branch (verified, `adequacy-writer.log`, #67's entry).
- **Fold-in cost.** About 150 lines in `version/io/writer/tests.rs`, needing
  entry 1's encoder or the bridge's. Remove its `prop_assume!`, which never
  rejects because the inner tree is forced to be a branch (inferred), and
  make its two recursive helpers iterative. Runtime unmeasured at 4,000
  cases; it failed in 1.3 s under W1 (verified, `adequacy-writer.log`). No
  new dependencies.
- **Overlaps.** Committed `copy_subtree_remainder_matches_per_leaf_feeding`,
  `collapse_after_a_splice_matches_per_leaf_feeding`, and the `tick` suites.
- **Dependencies.** #67 rewrites `splice_continuation`; the harness still
  applies (inferred).
- **Value, in one sentence.** An independent oracle on a path the committed
  suite already exercises heavily; low marginal value.

## 9. First-detected-class model

- **What it is.** A predicate inside entry 1, reported but never enforced:
  the `first` field of each rejection verdict, and the statistics line
  `judge` prints when production reports another class (verified, lines
  134–148 and 1123–1127).
- **What it reaches or checks.** That production reports the first defect
  a sequential parse meets. Over 300,000 cases per type, twice, production
  matched it every time (reported; neither `spec2.log` nor
  `spec-long2.log` prints a mismatch line, verified).
- **Coverage beyond the committed suite.** If enforced, it would fix the
  precedence on every multiply-defective input, including those where both
  classes stay applicable. That covers #67's mutant B and the two padding
  survivors on every input that has them, not only where entry 1's
  narrowed sets happen to bite (inferred). No committed test states a
  precedence beyond span's.
- **Evidence.** None as a check: it never fails.
- **Fold-in cost.** A few lines once entry 1 exists: turn the statistics
  line into an assertion. It should follow, not precede, a documented
  precedence in `Decode` and each decoder.
- **Overlaps.** Entry 1's narrowed verdicts; the adequacy lane's unbuilt
  brief `test-span-lone-endpoint-padding.md`, which pins the same
  precedence at one point.
- **Dependencies.** Your decision on documenting a precedence (#67).
- **Value, in one sentence.** Ready-made enforcement for a precedence rule
  if you choose to document one; otherwise nothing to fold.

## 10. Decoder mutation schemas, M1 to M17

- **What it is.** Calibration scripts.
  `mutate.py`, `mutate2.py`, and `mutate3.py` in the session scratchpad's
  `auditor-l6/` directory (not on the branch): 17 reversible string swaps
  at decoder and validator sites, each live when an environment variable
  names it; `mutate2.py` appends a cached `crate::l6m(k)` switch to
  `lib.rs`. Each swap asserts its anchor occurs exactly once (verified,
  reading).
- **What it reaches or checks.** Under-rejection (M1 a negative running
  height, M14 to M16 a collapsible pair at three sites, M17 an owned pair),
  over-rejection (M2 zero deltas beside internal siblings, M4 flush
  padding), misclassified structure (M3 one-child branches as owned, M5
  dominating pairs as equal), boundary errors (M6 `rho == 64`, M9 the
  padded byte count, M11 the gamma window's proof), skipped checks (M7 a
  zero final rank group, M8 the `Ranked` rank match, M10 borsh's marker),
  the writer's collapse cascade (M12), and span's precedence (M13)
  (verified, the scripts and the coverage record).
- **Coverage beyond the committed suite.** As data: it found three decoder
  checks that rest on one or two committed tests (observation 11a,
  verified against `adequacy.log` and `adequacy2.log`): M6 by
  `rank_decoding_rejects_each_malformed_input_class` alone; M14 and M15 by
  `span_borsh_rejects_collapsible_join` and
  `span_decode_rejects_each_malformed_input_class` alone.
- **Evidence.** Every committed run before mutation passed (693 of 693 and
  700 of 700); every mutant failed at least one committed test and entry 1
  (verified).
- **Fold-in cost.** Not as scripts: the repository has no mutation-schema
  mechanism, and the anchors rot. #67 moves M9's target into
  `Bits::padded_len` (inferred to break its exactly-once assertion). The
  durable forms are entry 1, whose generators exercise header widths 64 to
  69 and planted pairs at every position, or point tests that thicken the
  three thin spots. Preserving the scripts and logs means copying them into
  `lanes/l6-codecs/round-1/` before a reboot.
- **Overlaps.** The adequacy lane's cargo-mutants campaign and survivor
  index, which covers the same files mechanically.
- **Dependencies.** None.
- **Value, in one sentence.** A record of which decoder checks are thin,
  worth keeping as data, not as machinery.

## 11. wasm32 rank group-stream probe

- **What it is.** A wasm32 boundary probe and a lazy stream fixture.
  Commit `679cc2e51`: `Check::RankDecodeGroups`, guest
  `rank_decode_groups`, the fixture `synthesis::UnitFractionReader` (about
  75 lines; a `Read` yielding the canonical `2^-(8g)` with no backing
  buffer), and three printing harness tests (verified, reading).
- **What it reaches or checks.** `Rank::decode` of `g` fraction groups from
  a lazy reader: 1,000 and 4,096 groups, each compared with the in-memory
  `synthesis::unit_fraction`; `2^30` groups (exponent `2^33`), which passes
  in 128 s; and `2^30 + 1`, which traps in 124 s (verified, `wasm1.log`).
  Past 4,096 groups it checks only that the decode succeeds.
- **Coverage beyond the committed suite.** Small. The committed
  `rank_decode_accepts_the_first_exponent_past_usize` decodes a
  rank of exponent `2^32` (`2^29` groups) from an in-memory slice and
  checks it exactly through an adjacent-value identity (verified,
  `pins.rs` and the guest's `rank_decode`). Past the 64-byte prefix both
  take the decoder's streaming path (inferred). This probe goes one
  doubling further, from a lazy source, where no further narrowing is
  known (inferred).
- **Evidence.** It demonstrated the first site of the 32-bit growth panic,
  which your final ruling dissolved.
- **Fold-in cost.** The reviewed rewrite on `8208efaeb` checks the exact
  value at `2^30` groups through a streaming comparison of its `Display`
  text, without allocating (verified). Same restatement and protocol work
  as entry 2; about 130 s per case.
- **Overlaps.** The committed `RankDecode` check; #29's `Sum` check.
- **Dependencies.** As entry 2.
- **Value, in one sentence.** Little beyond the committed check; its lazy
  reader and streaming text comparison are reusable fixtures.

## 12. Reach histograms

- **What it is.** A diagnostic. `l6_reach_histograms` in `l6_spec.rs`,
  commit `e2e4d8dd8`, about 120 lines; prints histograms, asserts nothing
  (verified, reading).
- **What it reaches or checks.** For 20,000 draws each, depth and height
  width of entry 5's version and party inputs, with their violation rates,
  beside the committed `arb_oracle_version` and
  `arb_oracle_party_nonempty`. It took 62.6 s (verified, `reach-hist.log`).
- **Coverage beyond the committed suite.** None as a check. The committed
  census floors in `testing/generators/tests.rs` hold the committed
  generators' regimes (reported, the survey's section 3).
- **Evidence.** It produced entry 1's reach numbers.
- **Fold-in cost.** Only as census floors for entry 5's generators, if
  folded, at far fewer samples; otherwise leave it.
- **Overlaps.** The adequacy lane's `l8_census.rs`; #74's census floors.
- **Dependencies.** Entry 5.
- **Value, in one sentence.** Useful only as the liveness floor for entry
  5's generators.

## 13. wasm32 reader probe over a wide one-leaf version

- **What it is.** A wasm32 probe. Commit `3854b4c16`:
  `Check::VersionReaderWideLeaf`, guest `version_reader_wide_leaf`, and
  `Failure::OutOfMemory`, reusing entry 2's fixture (verified, reading).
- **What it reaches or checks.** `Version::decode` of entry 2's stream
  through a non-slice reader: `k = 1000` passes; `2^30 + 5` bytes returns
  `Decode::Io` of kind `OutOfMemory` in 15.7 s (verified, `wasm2.log`).
- **Coverage beyond the committed suite.** It records `std`'s fallible
  `read_to_end` refusing growth on wasm32. #53's `Decode::Io` paragraph
  states that behavior generally; this is its evidence.
- **Evidence.** One observation; no defect under your final ruling.
- **Fold-in cost.** Not recommended as a check: it would fix `std`'s growth
  policy on one target, which neither crate promises.
- **Overlaps.** Entry 2.
- **Dependencies.** None.
- **Value, in one sentence.** Evidence for #53's documentation sentence,
  not an instrument to keep.

## 14. Writer mutants and reach probes

- **What it is.** Calibration scripts. `mutate4.py` (W1 to W3) and
  `reach5.py` (R31 to R33, entry panics), session scratchpad only
  (verified, reading).
- **What it reaches or checks.** W1 marks a copied right edge as a leaf's
  sibling; W2 pushes a stray flag in `splice_continuation`'s early return;
  W3 lengthens the narrow splice range by one bit. R31 to R33 panic on
  entry to split-mode splicing, a depth-1 copy, and a wide last copy, to
  count which tests reach them: 16, 57, and 26 failures (verified,
  `reach5.log`).
- **Coverage beyond the committed suite.** None now. W2's survival showed
  the early return cannot run, which #67 acts on.
- **Evidence.** Above, and entry 8.
- **Fold-in cost.** Not as machinery; #67 deletes W2's anchor and may move
  R31's (inferred). The reach counts are a one-time reading.
- **Overlaps.** Entry 10; the adequacy campaign's survivor
  `version/io/writer.rs:255:34`, the same branch.
- **Dependencies.** None.
- **Value, in one sentence.** Spent: its finding is #67.

## 15. CBOR framing probes

- **What it is.** A printing probe. `probe_cbor_text_and_array_as_bytes`
  in `l6_probes.rs`, commit `7cb120ffe` (verified, reading).
- **What it reaches or checks.** Binary serde through ciborium: a CBOR text
  string is rejected for byte-encoded types; a CBOR array of integers is
  accepted in place of a byte string; a CBOR byte string `h'0102'` decodes
  as the `Count` with limbs `[1, 2]` (verified, `spec2.log`).
- **Coverage beyond the committed suite.** You ruled on question Q2 to
  accept and document the bridging, and #85 adds the module-doc sentence.
  No committed or ready test states it (verified: #85 leaves the serde
  tests unchanged, #51's added tests never mention CBOR or `Count`, and
  `main`'s serde tests do not either).
- **Evidence.** It produced question Q2.
- **Fold-in cost.** One assertion-bearing test of a few lines in
  `serde_impls/tests.rs`, if you want the documented bridging held by a
  test as your Q1 ruling asked for the leniency.
- **Overlaps.** #51 (human-readable framing); #85 (the documentation).
- **Dependencies.** #85.
- **Value, in one sentence.** Low: a one-line guard for a documented
  bridging that comes from the format crate, not from `before`.

## 16. wasm32 `Vec` growth diagnostic

- **What it is.** A wasm32 probe of `std`. Commit `679cc2e51`:
  `Check::VecGrowthBoundary`, guest `vec_growth_boundary` (verified,
  reading).
- **What it reaches or checks.** That `try_reserve(1)` on a full `Vec<u8>`
  of `2^30` bytes fails as a capacity overflow, not an allocator refusal
  (verified, `wasm1.log`, 3.6 s).
- **Coverage beyond the committed suite.** None for either crate's
  contract.
- **Evidence.** It established the growth panic's mechanism.
- **Fold-in cost.** Not recommended.
- **Overlaps.** None.
- **Dependencies.** None.
- **Value, in one sentence.** A diagnostic of `std`, spent with the
  dissolved defect.

## Already carried: the JSON positional-array probe

`probe_json_composites_accept_arrays` (`l6_probes.rs`) observed that
human-readable `Clock`, `Span`, and `Ranked` accept their fields as a JSON
array. Your Q1 ruling made that intended, and #51
(`fix/before-serde-record-framing`) states and tests it across csv, rmp,
bencode, and the other formats.

## What I could not assess

The brief limits my runs to censuses and runtimes, so three claims rest on
inference rather than a mutant run:

- that the borsh probe (entry 2) fails under the narrowing it targets,
  argued from the stream layout;
- that the spec harness fails under the two padding survivors (entry 1).
  The census verifies that the harness's `judge` rejects a `Truncated`
  report on every distinguishing input; that the survivors return
  `Truncated` there is the adequacy lane's verified report and my reading
  of the mutated line;
- that the reader probe (entry 4) would pass a decoder that swallows a
  late reader error, argued from its accept-either oracle.

I also did not measure entry 7's or entry 8's runtime at their current
default case counts. The harness's eleven tests together took 10.31 s at
defaults (`spec2.log`), which bounds entry 7.
