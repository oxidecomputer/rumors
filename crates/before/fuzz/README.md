# `before` fuzz targets (PROG-5 / COV-7)

Coverage-guided fuzzing of the byte codec and the decode-then-operate path, via
[`cargo-fuzz`](https://github.com/rust-fuzz/cargo-fuzz) / libFuzzer.

This is a **standalone workspace** (note the empty `[workspace]` table in `Cargo.toml`):
it is detached from the parent `rumors` workspace on purpose, so the before `clippy`/`nextest`
gate never tries to build it. Fuzzing needs a nightly toolchain and libFuzzer; the gate
does not.

## Prerequisites

```sh
rustup toolchain install nightly
cargo install cargo-fuzz
```

## Targets

- **`fuzz_decode`** feeds arbitrary bytes to every top-level `decode` —
  `Party`, `Version`, `Clock`, `Rank`, `Ranked`, and `Span`. Asserts the key
  invariant inline: an accepted value re-encodes stably and decodes back to
  itself (so a non-canonical accept is a crash, not a silent pass). The
  structural `is_normal`-on-accept form of the same invariant is checked by
  the in-tree proptest `clock::tests::h34_decode_never_panics`.
- **`fuzz_decode_differential`** feeds the same arbitrary bytes to every
  decode that has a composed public-API counterpart and asserts agreement on
  accept, value + re-encode, and *rejection genre*: the fused `Span` and
  `Ranked` decodes against their carve-decode-validate spellings, borsh's
  self-delimiting prefix reads against the whole-slice raw decodes, and
  postcard (the byte-carrying serde format of record) against its `Vec<u8>`
  framing plus raw decode. The genre axis is what round-trip fuzzing cannot
  see: two paths both rejecting an input while disagreeing on *which* error
  breaks the documented precedence (structural genres outrank the pair
  verdict).
- **`fuzz_decode_ops`** decodes a value from the front of the input, then uses
  the trailing bytes as an op script (tick / fork / join / sync / send / receive
  + observers). Pushes adversarially-shaped but canonical trees through the
  skyline kernels every operation runs on.
- **`fuzz_laws`** decodes versions, parties, and a clock from length-prefixed
  chunks, then asserts every named law in `before::laws` on them — the same
  collection the in-tree law proptests drive, here fed hostile-but-canonical
  values. A violated law panics with the law's name, so the fuzzer minimizes
  straight to the algebraic defect.
- **`fuzz_parse`** feeds arbitrary UTF-8 to every public `FromStr` (the
  paper-notation parsers for `Party`, `Version`, and `Clock`, and the decimal
  `Ticks` parser). An accepted value's display must re-parse to the same
  value.

## Run

From this directory:

```sh
cargo +nightly fuzz build   # build all targets
cargo +nightly fuzz run fuzz_decode              corpus/fuzz_decode              seeds/fuzz_decode              -- -max_total_time=20
cargo +nightly fuzz run fuzz_decode_differential corpus/fuzz_decode_differential seeds/fuzz_decode_differential -- -max_total_time=20
cargo +nightly fuzz run fuzz_decode_ops          corpus/fuzz_decode_ops          seeds/fuzz_decode_ops          -- -max_total_time=20
cargo +nightly fuzz run fuzz_laws                corpus/fuzz_laws                seeds/fuzz_laws                -- -max_total_time=20
cargo +nightly fuzz run fuzz_parse               corpus/fuzz_parse               seeds/fuzz_parse               -- -max_total_time=20
```

Drop `-max_total_time` to fuzz indefinitely. Crashes land in `artifacts/<target>/`;
reproduce with `cargo +nightly fuzz run <target> artifacts/<target>/<crash-file>`.

## Seeds

`seeds/<target>/` holds a small committed seed corpus (canonical encodings of every
wire type, decode-then-ops scripts, law-target chunk inputs, display notation for the
parse target, and the differential target's per-genre rejection witnesses — including
wide-gamma bases, whose 64+-zero unary prefixes random bytes essentially never
produce). Nothing consumes it
implicitly: a run reads it only when the seed directory is named as an extra corpus
argument, as the invocations above (and the `just fuzz` recipe) do — libFuzzer reads
every named directory and writes new discoveries to the first, so the committed seeds
stay pristine. The live `corpus/`, `artifacts/`, and `target/` directories are
git-ignored. The seeds derive from the live public API: `tests/fuzz_seeds.rs` holds the
directory byte-identical to the derivation, and
`cargo run -p before --example fuzz_seeds` regenerates it.
