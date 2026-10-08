# Branch coverage of before and suanpan: provenance and notable gaps

## Provenance

- Host: the Mac (aarch64-apple-darwin), by the owner's one-campaign exception.
  ox-east-1 cannot run it: none of its ten illumos toolchains ships
  `profiler_builtins` (E0463).
- Tree: explore/l8-adequacy at f27c324a. Production code is identical to the
  base 58285ca5; the tip adds only an `#[ignore]`d census module under
  `src/testing/` (excluded from the report) and untracked files under `l8/`
  (outside the workspace).
- Command (exit 0, 757 tests passed, 5 skipped, 102 s):
  `RUSTC_WRAPPER= PROPTEST_RNG_SEED=8008 nice -n 10 cargo +nightly-2026-06-30 llvm-cov nextest --locked -p before -p suanpan --all-features --branch --no-report --build-jobs 8 --test-threads 8 --no-fail-fast`
- The `cargo llvm-cov report` step produced an empty lcov file: the Mac's
  `build.build-dir` places the instrumented test binaries under
  `/Volumes/forge/build/97/460a50edc40b42/llvm-cov-target/debug/deps`, where
  cargo-llvm-cov 0.8.7 does not look. I did not rerun the campaign; I
  rebuilt the report from the same run's 807 `.profraw` files with the
  pinned nightly's `llvm-profdata merge -sparse` and `llvm-cov export
  -format=lcov`, passing the 14 test binaries as `-object`s and ignoring
  `.cargo/registry`, `rustc/`, `/testing/`, `tests.rs`, and `/tests/`.
- Directories this created on the Mac (not deleted; the coordinator retires
  them): `/Users/oxide/src/rumors-audit-l8-adequacy/target/llvm-cov-target`
  (profraw files and examples) and
  `/Volumes/forge/build/97/460a50edc40b42/llvm-cov-target` (1.2 GB of
  instrumented build artifacts). The merged profile is
  `cov-mac.profdata` in this scratch directory.
- What LLVM counts as a branch: `if`/`while`/`&&`/`||` conditions and
  `let ... else`, not `match` arms; unexecuted match arms show as
  never-executed lines.

## Totals

10,102 of 10,324 lines and 1,576 of 1,652 branch outcomes executed
(SUMMARY.txt, per module; one file per module in this directory).

## Notable gaps (excluding assertion-failure arms and `unreachable!` arms)

Routed by kind.

1. **Public decoder rejection paths never executed: `Rank::decode` over a
   reader** (`rank.rs:366-428`). The 64-byte read chunk
   (`DECODE_CHUNK_BYTES`) splits the decoder into a slice path and an
   incremental path, and these arms never run: a rank of exactly 64 bytes
   followed by trailing data (391), a rank longer than 64 bytes that is
   truncated (409), trailing data read after the incremental path's buffer
   (425), and every `ErrorKind::Interrupted` retry and `Decode::Io`
   propagation (378, 392-393, 414-415, 426-427). All are documented return
   arms ("Decode::Io when the reader itself fails"; rejects truncation and
   trailing bytes).
2. **Dead code: `Sealed::hole_subtracts`** (`causally/polarity.rs:41, 70-76,
   128-134, 182-184`). Declared on the sealed polarity trait and implemented
   for `Down`, `Up`, and `Neutral`, but `grep -rn hole_subtracts` finds no
   caller. Trait methods escape the dead-code lint.
3. **Arithmetic path reached only on 32-bit: `Rank::accumulate`**
   (`rank.rs:505-531`, reached from 198-208 and 957-962). On a 64-bit host
   `alignment_fits` is always true, so the shifted-accumulator path runs only
   in the wasm32 pins (`rank_arithmetic` cases 2 and 4). The pins are its
   only instrument; see the pin calibration.
4. **Public trait-impl spellings never executed**: `Count`'s `Debug` and
   `Add<&Count> for Count` (`count.rs:302-325`), `Ranked`'s `Debug`
   (`ranked.rs:327-331`), `OwnVersion`'s comparison impls for one of the
   macro's right-hand-side forms and all of the `&&rhs` forms
   (`version/own.rs:180-205`), `Span` operator impls with an owned or
   borrowed version receiver (`span/algebra.rs:653-662`), `After`/`Before`
   `Debug` (`causally/forms.rs:401-409`), `From<&Query>` for `Query`
   (`causally/convert.rs:44-46`), `BitsWriter`'s `Debug`
   (`bits/writer.rs:378-384`). Mutation survivors confirm the same family
   (`Hash for Version` survives replacement by `()`).
5. **Branches never taken that may be untested regimes or dead code**
   (to be settled against the mutation results):
   - `party/forks.rs:61-64`: the iterator's exit when
     `decision == depth && forks_again`.
   - `party/fork/remove.rs:220`: `RegionKind::Unowned => Sibling::Unowned`.
   - `party/join.rs:36-39`: the `b_present == false` arm.
   - `party/compare.rs:149`: one outcome of `first_left || second_left`.
   - `bits/reader.rs:174-175, 201-202`: the reader's own truncation
     errors (decode validation catches truncation first).
   - `version/io/writer.rs:255-257`: an early return when the cursor has
     passed `end`.
   - `suanpan/src/accumulator/digits/normalize.rs:81-84`: the loop that
     lowers `highest` past zero digits.
   - `suanpan/src/accumulator/digits/zero_ranges.rs:141, 170, 195`.
6. **Unreachable-by-design fold arms**: `version.rs:774-779` (span_all) and
   `span/algebra.rs:449-453`; both carry a comment saying the balanced
   counter never places a lone input below a merged group. The `&=`/`|=`
   mutant in the former survives, consistent with that.
