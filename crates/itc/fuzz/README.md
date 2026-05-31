# `itc` fuzz targets (PROG-5 / COV-7)

Coverage-guided fuzzing of the byte codec and the decode-then-operate path, via
[`cargo-fuzz`](https://github.com/rust-fuzz/cargo-fuzz) / libFuzzer.

This is a **standalone workspace** (note the empty `[workspace]` table in `Cargo.toml`):
it is detached from the parent `rumors` workspace on purpose, so the itc `clippy`/`nextest`
gate never tries to build it. Fuzzing needs a nightly toolchain and libFuzzer; the gate
does not.

## Prerequisites

```sh
rustup toolchain install nightly
cargo install cargo-fuzz
```

## Targets

- **`fuzz_decode`** — feeds arbitrary bytes to `Party::decode`, `Version::decode`, and
  `Clock::decode`. Asserts the keystone invariant inline: an accepted value re-encodes
  stably and decodes back to itself (so a non-canonical accept is a crash, not a silent
  pass). The structural `is_normal`-on-accept form of the same invariant is checked by the
  in-tree proptest `clock::tests::h34_decode_never_panics`.
- **`fuzz_decode_ops`** — decodes a value from the front of the input, then uses the
  trailing bytes as an op script (tick / fork / join / sync / send / receive + observers).
  Pushes adversarially-shaped but canonical trees through the working-form arithmetic and
  the repack-on-drop boundary.

## Run

From this directory:

```sh
cargo +nightly fuzz build                                       # build all targets
cargo +nightly fuzz run fuzz_decode      -- -max_total_time=20  # short smoke run
cargo +nightly fuzz run fuzz_decode_ops  -- -max_total_time=20
```

Drop `-max_total_time` to fuzz indefinitely. Crashes land in `artifacts/<target>/`;
reproduce with `cargo +nightly fuzz run <target> artifacts/<target>/<crash-file>`.

## Seeds

`seeds/<target>/` holds a small committed seed corpus (canonical encodings of known
clocks/parties/versions, and a couple of decode-then-ops scripts). cargo-fuzz seeds the
live corpus from here; the live `corpus/`, `artifacts/`, and `target/` directories are
git-ignored.
