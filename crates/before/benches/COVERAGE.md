# Criterion-cell coverage index: `before`'s public API × the board benches

The reconciliation of record between the crate's public surface and the
wall-clock (criterion) bench surface: which cells time each public
operation, and where the coverage claim rests on a mechanism-based
not-applicable reason instead. The deterministic legs (counters, floors,
envelopes) are indexed elsewhere: the board module doc
(`src/meter/board.rs`) owns the criterion and the canonical
not-applicable list, and `tests/meter.rs` owns the enforced envelopes.
This file indexes the *time leg* only.

## Method, re-runnable

1. Enumerate the public API: rustdoc JSON (nightly, `--all-features`,
   `--no-deps`), walked from the crate root along public reachability.
   At this tree the walk yields 520 impl-level entries: 18 modules,
   27 types, 115 methods, 50 free functions, 12 associated/module
   constants, 298 trait impls (169 name-level, deduplicating borrow
   shapes).
2. Enumerate the bench surface: `cargo bench -p before --bench board --
   --list` (the cell IDs are `op/family`, generated from
   `meter::board::bench_cells`, so they mirror the amplification board
   cell for cell by construction — ID drift between board and bench is
   structurally impossible).
3. Reconcile: every public operation must be timed by a cell below or
   carried by the board module doc's not-applicable list with a
   mechanism-based reason. Anything in neither place is a coverage gap.

## The judged cell inventory (223 cells)

46 board operation rows × their applicable families (221 cells, the
smoke pin's derivation in `tests/amp_board_smoke.rs`), plus the
judge-only wide-display pair (2). Families: `dense`, `bigroot`,
`hugeleaf`, `cliff` (the four event shapes), `id-pair`, `comb-scatter`,
`harmonic`, `scatter`, the eight tick-cross shapes (`nested-full`,
`nested-wide`, `mirror-wide`, `mirror-narrow`, `staircase`,
`reveal-comb`, `reveal-hifloor`, `pure-comb` — reached only by the two
tick rows), and `benign` (the organic control).

| Row (bench group) | Families | Public operations timed |
|---|---|---|
| `version_decode` | dense, bigroot, hugeleaf, cliff, benign | `Version::decode` (and the serde/borsh deserialize wrappers) |
| `version_encode` | dense, bigroot, hugeleaf, cliff, benign | `Version::encode` (and `encode_to`, `as_bytes` materialization, serialize wrappers) |
| `version_cmp` | dense, bigroot, hugeleaf, cliff, benign | `PartialOrd` on `Version`/`batch::Version`, every borrow shape |
| `version_eq` | dense, bigroot, hugeleaf, cliff, benign | `PartialEq` on `Version` (a wholesale byte compare of the canonical streams; the time leg backstops that it stays linear) |
| `version_concurrent` | dense, bigroot, hugeleaf, cliff, benign | `Version::concurrent`, `batch::Version::concurrent` |
| `version_join` | dense, bigroot, hugeleaf, cliff, benign | `BitOr` on `Version`/`batch::Version` |
| `version_join_assign` | dense, bigroot, hugeleaf, cliff, benign | `BitOrAssign` |
| `version_meet` | dense, bigroot, hugeleaf, cliff, benign | `BitAnd` |
| `version_meet_assign` | dense, bigroot, hugeleaf, cliff, benign | `BitAndAssign` |
| `version_tick` | dense, bigroot, hugeleaf, cliff, the eight tick-cross shapes, benign | `Version::tick`, `batch::Version::tick` (adversarial version × small party; the tick crosses carry their own (event, id) pair) |
| `version_tick_adv_party` | id-pair, benign | the same tick, adversarial party × small version; `Party::tick` is its mirror |
| `version_batch_snapshot` | dense, bigroot, hugeleaf, cliff, benign | `batch::Version::snapshot` |
| `version_rank` | dense, bigroot, hugeleaf, cliff, harmonic, benign | `Version::rank` (and `Ranked::from`) |
| `rank_pair_ops` | dense, harmonic, benign | `Rank::cmp`, `Rank::checked_sub`, `Add` (value-content-denominated) |
| `rank_sum` | dense, harmonic, benign | `Sum<Rank>`/`Sum<&Rank>` (the mixed high-first fold; `AddAssign` is `Add` in place) |
| `version_distance` | dense, bigroot, hugeleaf, cliff, harmonic, benign | `Version::distance` |
| `version_lag` | dense, bigroot, hugeleaf, cliff, harmonic, benign | `Version::lag` |
| `version_min_ticks` | dense, bigroot, hugeleaf, cliff, harmonic, benign | `Version::min_ticks` |
| `version_join_all` | scatter, benign | `Version::join_all` (and `Sum`/`FromIterator`, the same fold) |
| `version_project` | dense, bigroot, hugeleaf, cliff, id-pair, comb-scatter, benign | `Div`/`DivAssign` (`version / party`), both small×adversarial crosses plus the I/O-denominated adversarial×adversarial cross |
| `version_display` | dense, bigroot, hugeleaf, cliff, benign | `Display` (and `Debug`) on `Version` |
| `version_from_str` | dense, bigroot, hugeleaf, cliff, benign | `FromStr` on `Version` |
| `version_hash` | dense, bigroot, hugeleaf, cliff, benign | `Hash` on `Version` |
| `causally_contains` | dense, bigroot, hugeleaf, cliff, benign | `causally::Range::contains` (every `causally` constructor and refinement performs the same comparisons) |
| `party_decode` | id-pair, benign | `Party::decode` |
| `party_encode` | id-pair, benign | `Party::encode` |
| `party_fork` | id-pair, benign | `Party::fork` (`forks`/the consuming array splits iterate it) |
| `party_join` | id-pair, benign | `Party::join` |
| `party_join_all` | scatter, benign | `Party::join_all` |
| `party_covers` | id-pair, benign | `Party::covers` |
| `party_disjoint` | id-pair, benign | `Party::is_disjoint` |
| `party_without` | id-pair, benign | `Party::without` |
| `party_display` | id-pair, benign | `Display`/`Debug` on `Party` |
| `party_from_str` | id-pair, benign | `FromStr` on `Party` |
| `party_hash` | id-pair, benign | `Hash` on `Party` |
| `clock_decode` | dense, bigroot, hugeleaf, cliff, id-pair, benign | `Clock::decode` |
| `clock_encode` | dense, bigroot, hugeleaf, cliff, id-pair, benign | `Clock::encode` |
| `clock_tick` | dense, bigroot, hugeleaf, cliff, id-pair, the eight tick-cross shapes, benign | `Clock::tick` (`send` by definition; `batch::Clock::tick`) |
| `clock_fork` | dense, bigroot, hugeleaf, cliff, id-pair, benign | `Clock::fork`, `batch::Clock::fork` |
| `clock_join` | dense, bigroot, hugeleaf, cliff, id-pair, benign | `Clock::join`, `batch::Clock::join` |
| `clock_sync` | dense, bigroot, hugeleaf, cliff, id-pair, benign | `Clock::sync`, `batch::Clock::sync` |
| `clock_recv` | dense, bigroot, hugeleaf, cliff, id-pair, benign | `Clock::recv` (`clock \| version` folds through the same path) |
| `clock_own_version` | dense, bigroot, hugeleaf, cliff, id-pair, comb-scatter, benign | `Clock::own_version` |
| `clock_display` | dense, bigroot, hugeleaf, cliff, id-pair, benign | `Display`/`Debug` on `Clock` |
| `clock_from_str` | dense, bigroot, hugeleaf, cliff, id-pair, benign | `FromStr` on `Clock` |
| `clock_hash` | dense, bigroot, hugeleaf, cliff, id-pair, benign | `Hash` on `Clock` |
| `version_display_wide` | hugeleaf | judge-only: `Display` at conversion-dominated widths (text ceiling) |
| `display_schoolbook` | hugeleaf | judge-only tripwire: the known-quadratic conversion class, required red |

The `tripwire` bench target adds `tripwire_unmetered_quadratic/quadratic`,
the judge's live red demonstration; `amplify.rs`, `party.rs`,
`version.rs`, and `clock.rs` are a separate genre (oracle-comparison and
worst-case-shape rows, not judged cells) and deliberately do not mirror
board IDs.

## Operations covered by a not-applicable reason, not a cell

The canonical list, with per-item mechanism-based reasons, is the board
module doc's "Coverage: the not-applicable list" (`src/meter/board.rs`).
Its categories, cross-checked against the rustdoc walk above: delegations
and aliases (batch operator matrix, `send`, `Debug`); folds priced by
their measured cells (`Clock::join_all`, `meet_all`'s bounded
accumulator, `forks`); bounded or trivial inputs (constructors, seed
predicates, `TryFrom` literals); moves, borrows, and byte copies
(`is_empty`, `as_bytes`, accessors, `Clone`, the byte-compare `Eq`s);
derived
pairings (`Ranked`, `Rank::Display`); `causally`'s constructors and
refinements; serde/borsh wrappers over the codec rows; and test support.

Deliberately not benched, with the reason stated here so the skip is a
decision and not an oversight:

- **`meter`'s own surface** (the generators, the counters, the board
  and tier2 instruments, and its `skyline`/`accum` re-exports): the
  instrument surface is the measurement apparatus itself, feature-gated
  out of production builds; the re-exported kernels are the
  implementation under every public operation, public only so the
  envelope suite can pin their internals. The kernels' resources are
  pinned by their envelope scenarios in `tests/meter.rs`, their
  agreement with the recursive oracle by their differential suites, and
  every public operation routes through them, so every board cell above
  times them at the public boundary. Direct criterion cells on the
  re-exported entry points would time a surface no consumer calls.
- **`oracle`** and the `error`/`iter` data types: reference and plumbing
  types with no packed-input computation of their own.

## Sub-floor cells

Cells whose bodies sit under the bench judge's 10 µs judgment floor at
the high scale are SKIPped by the judge, not judged green; the coverage
they provide is the documented fact that the operation is cheap at board
sizes. Known members: the hash rows and `version_eq` on `benign` (the
board doc's stated exposure), plus `rank_pair_ops/benign` and
`rank_sum/benign` (word-scale rank arithmetic over a few hundred
content bytes).
