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

1. Enumerate the board's product: 65 operation rows over 21 shapes, 1090
   applicable cells (the per-shape derivation and the enforced pin:
   `tests/amp_board_smoke.rs`, `EXPECTED_CELLS`).
2. Enumerate the bench surface: `cargo bench -p before --bench board --
   --list` under each `BOARD_BENCH_MODE`. Cell IDs are `op/family`,
   generated from `meter::board::bench_cells` — the board's own cell
   table filtered by mode — so the bench mirrors the board cell for
   cell by construction and ID drift is structurally impossible.
3. Reconcile the public API: every public operation must be timed by a
   cell below or carried by the board module doc's not-applicable list
   with a mechanism-based reason. The inherent public-fn surface is
   extracted from source and pinned name-for-name by the triangle
   suite (`testing::triangle`, `METHOD_SURFACE`/`FAMILY_SURFACE`), so
   an operation cannot land unenumerated. Anything timed by no cell and
   excused by no reason is a coverage gap.

## The judged cell inventory

The bench mirror carries the board's 1090 cells (`BOARD_BENCH_MODE=full`,
the mode for final verdicts: `just bench-judge-full`) plus the two
judge-only wide-display cells — 1092 cells per scale. The judge recipes'
cadence (`just bench-judge`, both scales through the committed roster
`tools/benchjudge-expected.json`) times the `pinned` subset, 324 cells
per scale:

- **The designed diagonal (311)**: every operation on `dense`,
  `benign`, `id-pair`, and `scatter` where applicable; the magnitude
  shapes (`bigroot`, `hugeleaf`, `cliff`) on every group but the rank
  rows; `harmonic` on the measure and rank rows; `comb-scatter` on the
  projection rows; the ten tick crosses on the tick rows
  (`meter::board::designed`, declared per shape on the shape axis).
- **The board-red riders (13)** (`meter::board::BOARD_RED_BENCH_RIDERS`):
  standing board reds outside the diagonal keep a time leg — the
  display and `min_ticks` rows on the tick-cross and harmonic shapes.
- **The wide-display pair (2)**: `version_display_wide/hugeleaf` and
  `display_schoolbook/hugeleaf`, the text-ceiling conversion-class
  cells (pinned by `benches/common/sidecar.rs`, `TEXT_CEILING_CELLS`).

Families (19): `dense`, `bigroot`, `hugeleaf`, `cliff` (the four event
shapes), `harmonic`, `id-pair`, `scatter`, `comb-scatter`, the ten
tick-walk crosses (`nested-full`, `nested-wide`, `mirror-wide`,
`mirror-narrow`, `staircase`, `reveal-comb`, `reveal-hifloor`,
`pure-comb`, `ascend-cliff`, `ascend-plateau`), and `benign` (the
organic control).

Family-carrier classes, naming which shapes a row runs on in the full
product (the operand bundles decide applicability; family alone):

- **V17**: the 17 version-carrying shapes — all but `id-pair` and
  `scatter`.
- **P13**: the 13 party-pair-carrying shapes — `id-pair`,
  `comb-scatter`, the ten tick crosses, and `benign`.
- **C18**: V17 plus `id-pair` (every clock-carrying shape).
- **X18**: V17 plus `id-pair` (the projection rows' cross set).
- **F2**: the fold populations — `scatter` and `benign`.

| Row (bench group) | Full product | Pinned (judge cadence) | Public operations timed |
|---|---|---|---|
| `version_decode` | V17 | dense, bigroot, hugeleaf, cliff, benign | `Version::decode` (and the serde/borsh deserialize wrappers) |
| `version_encode` | V17 | dense, bigroot, hugeleaf, cliff, benign | `Version::encode` (and `encode_to`, `as_bytes` materialization, serialize wrappers) |
| `version_cmp` | V17 | dense, bigroot, hugeleaf, cliff, benign | `PartialOrd` on `Version`/`batch::Version`, every borrow shape |
| `version_eq` | V17 | dense, bigroot, hugeleaf, cliff, benign | `PartialEq` on `Version` (a wholesale byte compare of the canonical streams; the time leg backstops that it stays linear) |
| `version_concurrent` | V17 | dense, bigroot, hugeleaf, cliff, benign | `Version::concurrent`, `batch::Version::concurrent` |
| `version_join` | V17 | dense, bigroot, hugeleaf, cliff, benign | `BitOr` on `Version`/`batch::Version` |
| `version_join_assign` | V17 | dense, bigroot, hugeleaf, cliff, benign | `BitOrAssign` |
| `version_meet` | V17 | dense, bigroot, hugeleaf, cliff, benign | `BitAnd` |
| `version_meet_assign` | V17 | dense, bigroot, hugeleaf, cliff, benign | `BitAndAssign` |
| `version_tick` | V17 | dense, bigroot, hugeleaf, cliff, the ten tick crosses, benign | `Version::tick`, `batch::Version::tick` (adversarial version × small party; the tick crosses carry their own (event, id) pair) |
| `version_ticks` | V17 | dense, bigroot, hugeleaf, cliff, the ten tick crosses, benign | `Version::ticks` at the fixed count (`meter::board::TICKS_BOARD_COUNT`); `Party::ticks` and `Clock::ticks` are the same kernel through their own spellings, and the count axis is pinned point-to-point by `tests/meter.rs`'s flatness rows |
| `version_tick_adv_party` | P13 | id-pair, benign | the same tick, adversarial party × small version; `Party::tick` is its mirror |
| `version_batch_snapshot` | V17 | dense, bigroot, hugeleaf, cliff, benign | `batch::Version::snapshot` |
| `version_rank` | V17 | dense, bigroot, hugeleaf, cliff, harmonic, benign | `Version::rank` (and `Ranked::from`) |
| `rank_pair_ops` | V17 | dense, harmonic, benign | `Rank::cmp`, `Rank::checked_sub`, `Add` (value-content-denominated; `checked_sub → None` measured here, both directions attempted) |
| `rank_sum` | V17 | dense, harmonic, benign | `Sum<Rank>`/`Sum<&Rank>` (the mixed high-first fold; `AddAssign` is `Add` in place) |
| `version_distance` | V17 | dense, bigroot, hugeleaf, cliff, harmonic, benign | `Version::distance` |
| `version_lag` | V17 | dense, bigroot, hugeleaf, cliff, harmonic, benign | `Version::lag` |
| `version_min_ticks` | V17 | dense, bigroot, hugeleaf, cliff, harmonic, benign + riders: mirror-wide, mirror-narrow | `Version::min_ticks` |
| `version_join_all` | F2 | scatter, benign | `Version::join_all` (and `Sum`/`FromIterator`, the same fold) |
| `version_project` | X18 | dense, bigroot, hugeleaf, cliff, id-pair, comb-scatter, benign | `Div`/`DivAssign` (`version / party`), both small×adversarial crosses plus the I/O-denominated adversarial×adversarial cross |
| `version_display` | V17 | dense, bigroot, hugeleaf, cliff, benign + riders: harmonic, nested-full, nested-wide, mirror-wide, mirror-narrow, staircase | `Display` (and `Debug`) on `Version` |
| `version_from_str` | V17 | dense, bigroot, hugeleaf, cliff, benign | `FromStr` on `Version` (accepting direction) |
| `version_hash` | V17 | dense, bigroot, hugeleaf, cliff, benign | `Hash` on `Version` |
| `causally_contains` | V17 | dense, bigroot, hugeleaf, cliff, benign | `causally::Range::contains` (every `causally` constructor and refinement performs the same comparisons) |
| `party_decode` | P13 | id-pair, benign | `Party::decode` |
| `party_encode` | P13 | id-pair, benign | `Party::encode` |
| `party_fork` | P13 | id-pair, benign | `Party::fork` (`forks`/the consuming array splits iterate it) |
| `party_join` | P13 | id-pair, benign | `Party::join` (accepting arm) |
| `party_join_all` | F2 | scatter, benign | `Party::join_all` (accepting arm; `Clock::join_all` runs the identical indexed fold — delegation, the board doc's NA list) |
| `party_covers` | P13 | id-pair, benign | `Party::covers` |
| `party_disjoint` | P13 | id-pair, benign | `Party::is_disjoint` |
| `party_without` | P13 | id-pair, benign | `Party::without` (`Some` arm) |
| `party_display` | P13 | id-pair, benign | `Display`/`Debug` on `Party` |
| `party_from_str` | P13 | id-pair, benign | `FromStr` on `Party` (accepting direction) |
| `party_hash` | P13 | id-pair, benign | `Hash` on `Party` |
| `clock_decode` | C18 | dense, bigroot, hugeleaf, cliff, id-pair, benign | `Clock::decode` |
| `clock_encode` | C18 | dense, bigroot, hugeleaf, cliff, id-pair, benign | `Clock::encode` |
| `clock_tick` | C18 | dense, bigroot, hugeleaf, cliff, id-pair, the ten tick crosses, benign | `Clock::tick` (`send` by definition; `batch::Clock::tick`) |
| `clock_fork` | C18 | dense, bigroot, hugeleaf, cliff, id-pair, benign | `Clock::fork`, `batch::Clock::fork` |
| `clock_join` | C18 | dense, bigroot, hugeleaf, cliff, id-pair, benign | `Clock::join`, `batch::Clock::join` (accepting arm) |
| `clock_sync` | C18 | dense, bigroot, hugeleaf, cliff, id-pair, benign | `Clock::sync`, `batch::Clock::sync` (accepting arm) |
| `clock_recv` | C18 | dense, bigroot, hugeleaf, cliff, id-pair, benign | `Clock::recv` (`clock \| version` folds through the same path) |
| `clock_own_version` | X18 | dense, bigroot, hugeleaf, cliff, id-pair, comb-scatter, benign | `Clock::own_version` |
| `clock_display` | C18 | dense, bigroot, hugeleaf, cliff, id-pair, benign + riders: harmonic, nested-full, mirror-wide, mirror-narrow, staircase | `Display`/`Debug` on `Clock` |
| `clock_from_str` | C18 | dense, bigroot, hugeleaf, cliff, id-pair, benign | `FromStr` on `Clock` (accepting direction) |
| `clock_hash` | C18 | dense, bigroot, hugeleaf, cliff, id-pair, benign | `Hash` on `Clock` |
| `version_decode_truncated` | V17 | dense, bigroot, hugeleaf, cliff, benign | `Version::decode` rejection: last byte dropped (EOF discoverable only at the stream's end) |
| `version_decode_trailing` | V17 | dense, bigroot, hugeleaf, cliff, benign | `Version::decode` rejection: junk after the complete valid stream |
| `version_decode_noncanon` | V17 | dense, bigroot, hugeleaf, cliff, benign | `Version::decode` rejection: minimality violation at the preorder-last pair |
| `version_parse_trailing` | V17 | dense, bigroot, hugeleaf, cliff, benign | `FromStr` on `Version`, rejection: junk after the complete valid text |
| `version_parse_noncanon` | V17 | dense, bigroot, hugeleaf, cliff, benign | `FromStr` on `Version`, rejection: equal-sibling respelling at the text's end |
| `party_decode_truncated` | P13 | id-pair, benign | `Party::decode` rejection: truncation |
| `party_decode_trailing` | P13 | id-pair, benign | `Party::decode` rejection: trailing junk |
| `party_decode_noncanon` | P13 | id-pair, benign | `Party::decode` rejection: collapsible terminal pair at the preorder end |
| `party_parse_trailing` | P13 | id-pair, benign | `FromStr` on `Party`, rejection: trailing junk |
| `party_parse_noncanon` | P13 | id-pair, benign | `FromStr` on `Party`, rejection: collapsible respelling at the text's end |
| `clock_decode_truncated` | C18 | dense, bigroot, hugeleaf, cliff, id-pair, benign | `Clock::decode` rejection: truncation (component non-canonicality is the component validators — delegation) |
| `clock_decode_trailing` | C18 | dense, bigroot, hugeleaf, cliff, id-pair, benign | `Clock::decode` rejection: trailing junk |
| `clock_parse_trailing` | C18 | dense, bigroot, hugeleaf, cliff, id-pair, benign | `FromStr` on `Clock`, rejection: junk inside the outer parens, past the full component parse |
| `party_join_overlap` | P13 | id-pair, benign | `Party::join` rejection (Law of Disjointness), the overlap at the preorder-last position |
| `clock_join_overlap` | P13 | id-pair, benign | `Clock::join` rejection (the party join is the gate; no version work) |
| `clock_sync_overlap` | P13 | id-pair, benign | `Clock::sync` rejection, same gate |
| `party_join_all_overlap` | P13 | id-pair, benign | `Party::join_all` overlap hand-back (`Err(Vec)`), every overlapping input returned through the per-call id index |
| `party_without_none` | P13 | id-pair, benign | `Party::without` `None` arm: identical regions, the empty result known only after both streams walk in full |
| `version_display_wide` | judge-only: hugeleaf | hugeleaf | `Display` at conversion-dominated widths (text ceiling; the honest divide-and-conquer class) |
| `display_schoolbook` | judge-only: hugeleaf | hugeleaf | judge-only tripwire: the known-quadratic conversion class, required red by the roster |

The `tripwire` bench target adds `tripwire_unmetered_quadratic/quadratic`,
the judge's live red demonstration (`just bench-judge-tripwire`,
`--expect-red`); `amplify.rs`, `party.rs`, `version.rs`, and `clock.rs`
are a separate genre (oracle-comparison and worst-case-shape rows, not
judged cells) and deliberately do not mirror board IDs.

## Operations covered by a not-applicable reason, not a cell

The canonical list, with per-item mechanism-based reasons, is the board
module doc's "Coverage: the not-applicable list" (`src/meter/board.rs`).
Its categories: delegations and aliases (batch operator matrix, `send`,
`Debug`, `Clock::join_all`/clock component validation); folds priced by
their measured cells (`meet_all`'s bounded accumulator, `forks`);
bounded or trivial inputs (constructors, seed predicates, `TryFrom`
literals, the anonymous decodes/parses); moves, borrows, and byte
copies (`is_empty`, `as_bytes`, accessors, `Clone`, the byte-compare
`Eq`s); derived pairings (`Ranked`, `Rank::Display`); `causally`'s
constructors and refinements; serde/borsh wrappers over the codec rows;
and test support.

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
- **`oracle`** and the `error`/`iter` data types: reference and
  plumbing types with no packed-input computation of their own.

## Judged verdicts at this tip

The item-11 realization runs (2026-07-27, both scales through the
committed roster; quick sampling, then record sampling at 33 min 27 s
wall) each read **246 green / 3 red / 54 sub-floor over the 303 pinned
cells**, roster satisfied: the three reds are exactly the rostered
expectations (the schoolbook tripwire at e ≈ 2.0 over text 1.7, the
hugeleaf display pair at e ≈ 1.4 over general 1.3), the fifteen
bigroot sweeps read e 0.92–1.04, and the thirteen riders read
e 0.93–1.18. The two sampling regimes agreed cell for cell — identical
SKIP sets, red exponents within 0.03 — the committed evidence that the
roster's expectations are exponent classes independent of sampling.
The standing wall-leg verdict is judged in quick mode
(`just bench-judge`); record sampling belongs to the campaign's
acceptance sweep alone.

## Sub-floor cells

Cells whose bodies sit under the bench judge's 10 µs judgment floor at
the high scale are SKIPped by the judge, not judged green; the coverage
they provide is the documented fact that the operation is cheap at
board sizes. The set is enumerated per run in the judge's SKIP lines
and was identical under quick and record sampling (2026-07-27): 54
cells in four genres.

- **Byte movers** (materialize, compare, or hash stored canonical
  bytes): `version_encode`/`version_eq`/`version_hash` and
  `clock_encode`/`clock_hash` on their five pinned families each
  (`clock_encode`/`clock_hash` a sixth, `id-pair`), `clock_fork` on
  `dense`, `bigroot`, `hugeleaf`, `cliff`, and `benign`,
  `party_encode`/`party_hash` on both party families, `party_fork`/`party_covers`/`party_disjoint`/
  `party_decode` on `benign`.
- **Word-scale rank arithmetic**: `rank_pair_ops` on all three pinned
  families, `rank_sum` on `benign`.
- **Small-operand rejections** (the `benign` bundle's operands are
  organically small): the five party rejection rows, the three
  overlap rows, and `party_join_all_overlap`, each on `benign` only.
- **One query**: `version_min_ticks/cliff` (the comb's packed form is
  small at board sizes).

## Decision record

- 2026-07-27 (the ticks(n) landing): the `version_ticks` row joins the
  board and its bench mirror (+19 full cells, +15 pinned: the tick
  diagonal's shapes). Re-counting for this entry also trued the
  inventory against the code (`bench_cells` at both modes): the board
  this file described had already grown from the closeout census's
  989/303 to 1071 full / 309 pinned before this landing — cells landed
  by later tracks without a re-sweep here — so the numbers above are
  re-derived whole at 1090 full / 324 pinned (311 designed-diagonal
  cells + the 13 riders), and the method's step 1 now names the
  21-shape board the smoke pin enforces.
- 2026-07-27 (P5.5, the closeout census): re-derived whole against the
  989-cell board (64 rows × 19 shapes: the 18 rejection rows and the
  `reveal-comb`/`reveal-hifloor`/`pure-comb`/`ascend-cliff`/
  `ascend-plateau` crosses included) and the two-mode bench mirror;
  the pinned subset counted at 303 cells per scale with
  `BOARD_RED_BENCH_RIDERS` populated (13) and the judge roster
  realized to the schoolbook tripwire plus the hugeleaf display pair,
  judge-verified under both sampling regimes. Cross-checks: every
  board row × family pair has its bench mirror by construction
  (`bench_cells` is the board's own table); every public operation is
  timed by a row above or carried by the board doc's NA list; no gap
  found.
