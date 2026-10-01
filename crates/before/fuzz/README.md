# `before` fuzzing

This detached workspace uses libFuzzer to search byte strings and operation
sequences that the ordinary generators do not choose. The target bodies live
in `src/targets/`; the files in `fuzz_targets/` only connect them to
libFuzzer.

## Targets

- **`fuzz_decode`** is the fast, transport-independent pass over every raw
  decoder. An accepted input must be the value's exact canonical encoding and
  must decode identically a second time.
- **`fuzz_decode_differential`** compares raw, fused, borsh, and postcard
  decoding. It checks values, consumed bytes, and rejection classes, including
  `Span` and `Ranked` validation. It also compares `Count` with its public
  limb sequence.
- **`fuzz_decode_ops`** decodes a clock before driving public operations or
  receiving a decoded message. This exposes the algorithms to valid stored
  shapes that API-generated traces may not reach.
- **`fuzz_laws`** evaluates the crate's shared algebraic laws over decoded
  versions, parties, and clocks. Invalid chunks use canonical starting values,
  so every input runs every law group while successful decodes earn new
  coverage.

Every target also rejects a survivable allocation spike above the fuzz
harness's absolute emergency cap. The resource board and fuzz-fit, not this
coarse guard, establish the library's proportional cost contracts.

## Cadence

Verification separates target correctness from input discovery:

1. The ordinary gate builds every target and replays every committed seed
   through the exact target body. This deterministic pass catches broken
   framing, assertions, and transport comparisons.
2. `just all` runs a short coverage-guided smoke over every target.
3. Longer exploratory sessions use the root justfile's `fuzz` recipe with a
   larger duration. `just --show fuzz` displays the command of record.

Fuzzing requires the pinned nightly toolchain and
[cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz). A crash is written under
`artifacts/<target>/`; reproduce it with
`cargo +nightly fuzz run <target> artifacts/<target>/<crash-file>`.

## Committed seeds

`seeds/<target>/` contains canonical values, rejection-boundary witnesses,
operation sequences, and law inputs that random mutation is unlikely to
discover. The shared framing in `tests/support/fuzz_input.rs` is used by both
the targets and seed derivation. `tests/support/fuzz_seed_set.rs` derives the
seed bytes from the live API, and `tests/fuzz_seeds.rs` checks their names,
bytes, and intended semantic cases.

After a deliberate wire or framing change, regenerate the corpus with:

```sh
cargo run -p before --example fuzz_seeds
```

The live `corpus/`, `artifacts/`, and `target/` directories are
git-ignored. Coverage-guided runs read committed seeds from a second corpus
directory and write discoveries only to the live corpus.
