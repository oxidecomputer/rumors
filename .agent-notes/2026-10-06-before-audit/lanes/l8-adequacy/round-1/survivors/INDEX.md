# Mutation survivors, classified

Campaign settings: cargo-mutants 27.1.0, full `before` or `suanpan` nextest
suite (`--all-features`), `PROPTEST_RNG_SEED=8008`, excluding `src/testing/`
and test files (`l8/mutants-common.sh`). Raw lists per module in this
directory; `raw-<group>/` holds cargo-mutants' own files.

Classes:

- **G**: a reachable value change no test detects (a test gap).
- **T**: an unverified public trait-impl behavior (`Debug`, `Hash`,
  `size_hint`, operator spellings).
- **C**: value-neutral, changes work only; the gate's board leg may catch it
  (cargo-mutants runs nextest only).
- **E**: equivalent: no observable change on any input.
- **U**: unreachable by design (code a comment or invariant says never runs).
- **I**: instrument or test-support code compiled into the crate.
- **?**: needs the owning lane's judgment.

| group | done | caught | missed | unviable |
|---|---:|---:|---:|---:|
| count.rs (pilot) | 57 | 41 | 3 | 13 |
| suanpan | 411 | 355 | 30 | 26 |
| version (partial, 255 of 1,596) | 255 | 181 | 7 | 67 |
| rest (partial, 42 of 113) | 42 | 7 | 21 | 14 |

## count.rs

| mutant | class | note |
|---|---|---|
| 303 `Debug for Count` -> `Ok(Default)` | T | "The same format as `Display`" unchecked |
| 324 `Add<&Count> for Count` -> `Default` | T | spelling never executed (coverage) |
| 360 `Sum<&Count>` -> `Default` | T | only `Sum<Count>` is tested |

## suanpan

| mutant(s) | class | note |
|---|---|---|
| normalize.rs:81-83 (5 mutants) | G | width-shrinking loop never runs; reachable, witness in briefs/machinery-suanpan-normalize-bound.md |
| normalize.rs:74 `*` -> `/` | G | one-digit growth only tested with top carry 1; witness gives a wrong value under the mutant |
| read.rs:90, 99 `>>=` -> `<<=` | C | drain loops push up to three extra zero digits; touch counts are documented as not an API promise |
| operators.rs:382 `\|\|` -> `&&` in `shift_left` | C | `shl(0)` takes the general path; equal value |
| zero_ranges.rs:141, 150 `<` -> `<=` in `take_below`; 170 guard -> false; 195-196 guards | C, ? | skip-structure only; may drop ranges and break "scans proportional to prior writes" (zero_ranges.rs:15-19); L7 to measure |
| zero_ranges.rs:185 `last` (3 mutants) | E | read only inside a `debug_assert` |
| zero_ranges.rs:194 `compact_storage` -> `()` | E | an empty or single-entry map behaves as `Empty`/`One` |
| conversions.rs:46, read.rs:115 `\|` -> `^` | E | halves never overlap |
| digits.rs:247, 260 debug assertions -> `()` | E | assertions only |
| accumulator.rs:197, digits.rs:106 reservations -> `()` | E | capacity only |
| accumulator.rs:432, zero_ranges.rs:51, digits.rs:123 `Debug` | T | debug output unverified |

## version (partial)

| mutant(s) | class | note |
|---|---|---|
| version.rs:162 `Hash for Version` -> `()` | T | the Eq/Hash law holds for a constant hash; nothing checks content dependence |
| version.rs:776, 777 in `span_all` | U | fold arm the counter never reaches (comment at 770-773) |
| version/instrument.rs:33, 57, 63 | I | meter-feature entry points used by the board |
| version/overlay.rs:142 `>` -> `>=` in `advance_set` | ? | reorders tied crossings; equivalent iff all four `CursorSet`s fold ties order-insensitively (place.rs:410, projection.rs:420, filter.rs:418, 716); L2/L5 |

## rest (partial)

| mutant(s) | class | note |
|---|---|---|
| fold.rs:27, 32 in `balanced_try_fold` | C | fold degrades to quadratic; measured 14x scan bits; the floor-only asymptotics pins pass; the board should catch it (findings/instrument-calibrations.md) |
| recurse.rs:45, 49 | I | test-only stack guard constants |
| shape.rs:176, 228, 286 `size_hint` for `Plateaus`, `Regions`, `Overlay` (every constant) | T | public iterators' hints unchecked |
