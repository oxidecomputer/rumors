# Orientation map: `before` and `suanpan` at 9e5784fb

Paths are relative to `crates/before/` unless prefixed. "Verified" means I ran or mechanically checked it; everything else is assessed by reading the cited lines.

## 1. Layering of the shipped code (top to bottom)

- **Public types** (`src/lib.rs:425-433`): `Clock` (`src/clock.rs`, a `Party` paired with a `Version`, plus `clock/forks.rs`), `Party` (`src/party.rs`, the id tree as packed bits; `party/forks.rs`; `party/ops/` = build, compare, diff, index, split, sum, sum_split), `Version` (`src/version.rs`, the event tree as a skyline stream; `version/own.rs` lazy projection `OwnVersion`; `version/rank.rs` + `rank/num.rs` the `Rank` with its two-arm numerator; `version/ranked.rs`; `version/ticks.rs` the unbounded `Ticks` count), `Span` (`src/span.rs`; `span/algebra.rs` the four operators, `span/own.rs`, `span/verdict.rs`, `span/wire.rs`), `causally` (`src/causally.rs`; conjunction, convert, forms, polarity, query), `shape` (`src/shape.rs`, public step-function iterators), `iter` (`src/iter.rs`, fork iterators), `error` (`src/error.rs`). Cross-cutting: `src/fold.rs` (the balanced binary-counter reduction every n-ary fold runs on), `src/recurse.rs` (test-only stack guard plus the always-compiled segment counter), `src/auto_traits.rs`, `serde_impls.rs`, `borsh_impls.rs`.
- **The event coding and its kernels** (`src/version/skyline.rs`, `pub mod` under `meter`/test, else `pub(crate)`, `src/version.rs:26-29`): `admit` (fused parse-and-dominate for the span wire form), `build` (`SkylineBuilder`, the collapsing output builder), `decode`/`encode`/`validate` (`validate_prefix` is the production decode pass, `validate.rs:57`), `emit` (join/meet/hull), `fill` + `fill/{fuse,memo,prescan}` (the fused tick), `grow` (the splice), `literal`, `masked`, `overlay` (cursors and the advance law), `place` + `place/filter` (span and query walks), `query` + `query/{integral,web}` (rank, distance, lag, min_ticks, project), `shape`, `signed`, `sweep`, `text`, `walk`, `watermark` (`MinWeb`), and the counters `pool_traffic`, `web_traffic`; `version/hull_traffic.rs` beside them.
- **The id coding** (`src/idbits.rs`: 2-bit child-presence tags, `IdReader`, `skip_subtree`) with its operations in `party/ops/` and its parser/validator in `codec/tree.rs`.
- **The codec substrate** (`src/codec.rs:1-18`): `bits` (`Bits` frozen refcounted at-rest form, `BitsView`), `buf` (`BitsBuf` build form), `build` (`PackedBuilder`), `code`/`int` (word-scale code and value forms), `cursor`/`dsi` (per-bit and word-parallel readers over `dsi-bitstream`), `gamma`, `stack` (`BitStack`, `PopStack`), `scan` (scan-meter), `base` + `base/limb_meter.rs` (`Base` = `dashu_int::UBig` newtype implementing `suanpan::Magnitude`), `text`, `tree`, `literal`, `display`.
- **`suanpan` underneath** (`crates/suanpan/src/lib.rs:352-362`): `Accumulator` (`accumulator.rs`: lazy-zone signed digits, collapsing sign fold, zero-run ledger, quick register), `Limbs` (`limbs.rs`), `Magnitude` (`magnitude.rs`), `touch_meter` (feature), `UBig` re-export. Every skyline walk's running height or difference is an `Accumulator` (`skyline.rs:102-110`).

## 2. Data flow of the four core operations

**tick.** `Clock::tick` (`clock.rs:102`) calls `Version::tick(&Party)` (`version.rs:180`), which is `Version::from_bits(skyline::fill::tick(self.0.live(), party))`. `fill::tick` (`fill.rs:209`) is `ticks(event, id, &Base::from(1))` (`fill.rs:247`): `fused_fill` (`fill.rs:293`) builds a `FillWalk` over a `DsiCursor` on the event bits and an `IdReader::root` on the id bits, with `Accumulator`s for height and gap, a `MinWeb` watermark, a `Memo` frame ledger, the `Out` changed-flag output, and a `RouteProbe`; it returns `FillOutcome::Changed(BitsBuf)` (the `SkylineBuilder` output) or `Unchanged(Route)`, in which case `grow::emit` (`grow.rs:493`) splices `+n` along the route. The result passes the one storage gate, `Version::from_bits` → `codec::Bits::freeze` (`version.rs:1199`, `bits.rs:120`), which seals the marker padding. `Party::tick` (`party.rs:180`) is the mirror spelling.

**fork.** `Clock::fork` (`clock.rs:153`) forks the party and clones the version (`O(1)` refcount, `version.rs:91-93`). `Party::fork` (`party.rs:233`) is `self.view().split()`: `IdReader::split` (`party/ops/split.rs:20`, `build_split` walks the unary spine by bit position and splices) yields two `BitsBuf`s, each through `Party::from_bits` (`party.rs:720`) → `Bits::freeze`. Balanced `forks(k)` is `party/forks.rs`' lazy `Split` and `clock/forks.rs`.

**join of versions.** `Version::join` (`version.rs:459`) → `join_refs` (`version.rs:889`): `codec::canonical_eq` (clone-identity then `memcmp`, `bits.rs:8-31`), then the empty-operand identities via `is_empty_stream`, then `skyline::emit::join` (`emit.rs:129`) = `emit(a, b, follow_max)` (`emit.rs:279`): `OpenedPair::open` (`overlay.rs:700`) opens a `LeafCursor` (a `DsiCursor`) per stream and seeds `diff = a_first − b_first` in an `Accumulator`; the loop calls `advance_diff` (`overlay.rs:656`), picks the side from `diff.sign()`, codes the boundary delta, and feeds `SkylineBuilder::leaf`; `finish()` returns the `BitsBuf`. `|=` takes `join_view` (`version.rs:864`); `meet` is the same sweep with `follow_min`; `span` uses `emit::hull` (`emit.rs:190`), one sweep emitting both. `Clock::join` adds `Party::join` (`party.rs:298`) → `IdReader::sum` (`party/ops/sum.rs:28`).

**comparison.** `PartialEq`/`PartialOrd` come from the `causal_cmp_impls!` macro (`version.rs:1721-1760`): `eq` is `codec::canonical_eq` (byte equality), `partial_cmp` is `skyline::sweep::causal_cmp` (`sweep.rs:94`): a `ptr_eq` rung, then `sweep(a, b, order_exit, Directions::relation)` (`sweep.rs:263`), which opens an `OpenedPair`, folds `diff.sign()` into two surviving `Directions` once per elementary interval, applies the exit predicate, and advances. Projected comparison (`OwnVersion`) is `masked::causal_cmp`; span placement is `place::span`; `Ranked`'s order is `query::rank_cmp`. **Decode** is `Version::decode` (`version.rs:1110`): `validate_prefix` + `require_marker_padding`, then `Bits::from_canonical` adopts the read buffer (`bits.rs:139`).

## 3. Invariants, hard rules, quantitative promises

- **Canonicality and byte equality.** "A skyline stream is canonical iff" minimal topology, natural heights, exact stream (`skyline.rs:67-77`); "Byte-equality is therefore semantic equality on this coding" (`skyline.rs:87`). At rest a value "holds its canonical packed preorder bit stream, marker-padded to a byte boundary" and decode "*strictly validates* normal form (iteratively — nothing here recurses), then adopts the (canonical) input bytes" (`codec.rs:13-18`). Hard rule: "`decode` strictly rejects non-canonical input; byte-equality is what `Eq`/`Hash` rest on" (`AGENTS.md:38-39`); `Eq` is `canonical_eq`, `Hash` is `canonical_hash` (`version.rs:1724-1727`, `102-105`).
- **Linearity.** Safety rules "Causal Singularity" and "Identity Linearity" (`lib.rs:224-242`); pins `assert_not_impl_any!(Party: Clone, Copy)` (`party.rs:74`), `assert_not_impl_any!(Clock: Clone, Copy)` (`clock.rs:62`), and no borrowing `BitOr` for `Clock` (`clock.rs:1076-1077`); `AGENTS.md:40-42`. Escape hatches: bytes (`lib.rs:236`), `dangerously_alias`, text/literal doors (`party.rs:13-17`).
- **No depth recursion.** "No library traversal recurses on tree depth" (`AGENTS.md:25-37`); "Every library traversal is iterative: depth lives on explicit heap stacks" (`recurse.rs:9-14`); the proof is `clock::tests::deep_tree_stack_safety` at depth 100 000 (`clock/tests.rs:551-566`). `descend!` is used only by `testing/bridge.rs` and test files (verified by grep). The oracle is deliberately not hardened (`oracle.rs:14-32`).
- **Asymptotic and space promises.** "Any asymptotic claim is a hard guarantee ... for all input sizes" and "the auxiliary space required to compute any operation is at most a small constant multiple of the input size ... Any operation which requires more than a small amount of temporary memory ... is a bug" (`lib.rs:333-358`); size figures ~3 B/party, ~100 B/version at 100 parties and 10^6 events, ~50 B and ~2 000 B under churn (`lib.rs:312-319`). The board judges exponents at `MAX_SCALING_EXPONENT = 1.15` (`board/ceilings.rs:69`) with declared models for the `O(D log k)` folds (`fold.rs:5-6`, `ceilings.rs:9-18`), and the rank family states `O(M(|v|))`/`Ω(M(|v|))` (`query/integral.rs:213-241`). Flag: `tests/meter.rs:7-9` still says "Today's implementation is far from that — several operations amplify their input by large constants or worse", which contradicts the crate docs' present-tense guarantee; one of the two is stale.
- **suanpan.** "machine-word deltas and sign reads amortized O(1), wide deltas amortized O(operand limbs), on every input sequence" (`suanpan/src/lib.rs:1-3`), the cost table (`208-223`), touch counts "are **exact** ... a change to any operation's count is a breaking change of this crate" (`283-286`), `UBig` is `dashu_int` 0.5 and "bumping that dependency is a breaking change" (`296-297`); sign queries take `&mut self` (`123-126`).
- **Other hard rules.** `#![forbid(unsafe_code)]` (`lib.rs:412`, `suanpan/src/lib.rs:350`); the public API is stable (`AGENTS.md:43-45`); the oracle is never fixed to match production (`validation_index.rs:172-173`).

## 4. The instrument story

| Instrument | Failure class it catches | Lives in | Recipe | Tier |
|---|---|---|---|---|
| Oracle differential (recursive paper transcription) | wrong answers vs the paper | `src/oracle/`, `testing/bridge.rs`, per-module `tests.rs`, `testing/optrace.rs`, `generators.rs`, `grow_brute_force.rs` | `test-all` | gate (workspace stream) |
| Semantic (function-space) oracle | a bug both tree recursions share | `testing/semantic_oracle.rs` | `test-all` | gate |
| Exhaustive enumeration | measure-zero corners | `testing/exhaustive.rs` (`exhaustive_small`; `exhaustive_deep` is `#[ignore]`) | `test-all` / manual | gate / manual |
| Algebraic and representational laws | contract violations with no reference | `src/laws.rs`, `testing/algebraic_laws.rs`; also `fuzz_laws` | `test-all`; `fuzz` | gate; all |
| Pointwise differential table | population holes; known-bad descriptors convicted | `testing/diff_ops.rs` | `test-all` | gate |
| Surface roster and coverage suite | coverage holes, dead citations | `src/surface.rs`, `testing/surface_coverage.rs`, `tools/citecheck` | `test-all`, `citecheck` | gate |
| Surface totality (rustdoc JSON) + censuses | items in files the roster scan never names; new trait impls | `surfacecheck/`, `crates/surface-scan` | `surface-totality` | gate (surface stream); CI `instruments` |
| Resource envelopes | constant-factor regressions, cure backslides (×1.25 ceilings, ×0.75 tripwires) | `tests/meter.rs` | `test-all` (all features) | gate |
| Amplification board | shape × op pairings nobody pinned; meter vacuity (floors) | `src/meter/board/`, `examples/amp_board.rs`, `tests/amp_board_smoke.rs` | `amp-board-acceptance`, `worst-cases-pin`; `amp-board`/`worst-cases` | gate (board stream); CI `instruments`; manual views |
| Bench judge | work no deterministic counter sees | `benches/board.rs`, `benches/tripwire.rs`, `tools/benchjudge`, `tools/benchjudge-expected.json`, `tests/bench_judge_roster.rs` | `bench-judge`, `bench-judge-tripwire` | all only (quiet machine) |
| Fuzz targets | non-canonical accept, decode-path divergence, law violations on hostile bytes, heap amplifiers (`fuzz/src/lib.rs`) | `fuzz/`, seeds via `tests/fuzz_seeds.rs`, `examples/fuzz_seeds.rs` | `fuzz-build`; `fuzz` | gate (fuzz stream); all |
| Fuzz-fit bands | shapes nobody chose; total-cost drift; dead meter via `ENFORCE_MARGIN_BELOW` (`bands.rs:194`) | `fuzzfit/` | `fuzzfit`; `fuzzfit-calibrate` | gate (wasm stream); manual re-pin |
| Fuelscape islands / population atlas | nothing (audit view); sampler correctness, coverage parity, island attachment | `crates/before-fuelscape`, `fuelscape/`, `build.rs`, `docs/`, `testing/fuelscape_islands.rs` | `fuelscape-test`; `fuelscape-verify`, `fuelscape-claims`; `fuelscape` | gate (wasm stream); ci; manual |
| wasm32 pins | 32-bit position/width seams, red-first | `wasm32-pins/` | `wasm32-pins` | gate (wasm stream) |
| Coverage pins | uncovered kernel lines/branches by name | `tools/covcheck`, `tools/covcheck-expected.json` | `coverage-kernel`, `coverage-kernel-branch` | CI `coverage` job only |
| Mutants roster | exclusion patterns drifting from the live inventory | `.cargo/mutants.toml`, `tools/mutantcheck` | `mutants-list` (campaign itself manual) | gate-lints; ci |
| Superlinear tripwires | a known-bad kernel silently deleted | `tests/superlinear_tripwires.rs` naming `_reads_superlinear` fns | `test-all` | gate |
| Asymptotics pins | a documented non-linear mechanism disappearing | `testing/asymptotics.rs` | `test-all` | gate |
| Snapshots | doc goldens drift | `testing/snapshots.rs` (insta inline) | `test-all` | gate |
| Verdict matrix | a verdict kernel diverging from its siblings on adversarial operands | `tests/verdict_matrix.rs` | `test-all` | gate |
| Other tamper pins | hidden/foreign surface, stale-state model, fold `D` axis, answer embedding, clone rungs, forks saturation | `tests/{doc_hidden,foreign_reexport,stale_state,fold_skeleton,answer_embedded,coincident_span,forks_max}.rs` | `test-all` | gate |
| suanpan claims roster and touch pins | cost table vs roster vs witnesses; exact touch counts | `suanpan/src/claims.rs`, `accumulator/tests/metered.rs` | `test-all` | gate |

Gate composition verified from `justfile:403` and `:464-471`; CI jobs from `.github/workflows/ci.yml:49-230` (`ci`, `instruments`, `coverage`).

## 5. Module dependency graph

Generated by `map/depgraph.py` (edges = `use crate::`/`use super::` targets plus inline `crate::` paths; output `map/before_depgraph.txt`, production-only projection `map/production_edges.txt`). Layered, bottom up:

1. `error`, `fold`, `recurse`, `codec::{code,int,stack,scan,base}` — import nothing above.
2. `codec::{bits,buf,cursor,dsi,gamma,literal,text,tree}` → `error`; `idbits` → `codec`; `codec::display` → `idbits`.
3. `party::ops::*` → `codec`, `idbits`; `version::skyline::*` → `codec`, `idbits`, `fold`; `version::{rank,ticks,hull_traffic}` → `codec`, `error`.
4. `party` → `codec`, `error`, `fold`, `idbits`, `shape`; `version` → `codec`, `error`, `fold`, `shape`, `span`; `span` → `causally`, `codec`, `error`, `version`; `causally` → `codec`, `span`, `version`; `shape` → `version::skyline::shape`.
5. `clock` → `fold`, `party`, `shape`, `version`; `iter` re-exports the fork iterators; `serde_impls`/`borsh_impls` sit on top.

The one top-level cycle is `causally ↔ span ↔ version` (with `shape`): `span` imports `place` from `version::skyline`, `causally::query` imports `place::filter`, `version.rs` imports `span::Span` for `span()`; `party::ops::diff` also imports `version::skyline::overlay`. `codec::build` mentions `version` only in docs. Upward imports into a public type's module are therefore concentrated at the skyline seams (`span.rs:10`, `causally/query.rs:18`, `party/ops/diff.rs:49`).

Instrument touching production: no production (non-comment) code names `crate::meter`, `crate::laws`, `crate::oracle`, `crate::testing`, or `crate::surface` (verified by grep; the script's "production → instrument" edges are all rustdoc links). Instrument *hooks* compiled into production are the cfg-gated counters: `codec::scan::record_bits` (dsi, build, cursor, gamma, idbits, party::ops::index/diff), `codec::base::limb_meter` (`base/limb_metered.rs`, `rank/num.rs`, `query/integral.rs`), `version::hull_traffic` (`version.rs`), `skyline::{web_traffic,pool_traffic}` (`watermark.rs`, `grow.rs`), `recurse::SEGMENTS_GROWN`. Reverse direction: `meter.rs:73-81` re-exports `version::skyline`, `hull_traffic::SpanTraffic`, `web_traffic::EmitTraffic`; `meter/board/family.rs:1199` calls `idbits::skip_subtree`; `oracle/version.rs:8` imports `version::skyline::grow::Cost`; test files reach `codec::built_view`, `Base`, `skyline::{encode,validate,overlay,signed,build}`. `suanpan` has no cycles (`accumulator` → `Limbs`, `Magnitude`, `touch_meter`).

## 6. Glossary (defining file)

- **skyline** — a `Version` as a step function over the unit id interval (`skyline.rs:4`).
- **plateau** — one maximal constant run of it, a leaf spanning a dyadic interval; the `overlay` module mints the cursor vocabulary (`skyline.rs:5-7`); public `shape::Plateau`.
- **elementary interval** — a maximal span crossing no leaf boundary of either operand (`sweep.rs:9-11`).
- **overlay / advance law** — the tiling cursors and the rule for which cursor advances (`overlay.rs:1-67`).
- **watermark / web** — the anchored range-minimum `MinWeb` shared by fill and min_ticks (`watermark.rs:1-12`); `query/web.rs` drives it as `ReignWeb` with `Reign` payloads.
- **anchor, latent (Λ), follower** — `gap = h − A`, the parked boundary `A − m`, and `m − X` trackers (`watermark.rs:16-25`, `89-103`).
- **arming** — pushing a pending range's boundary onto the web (`watermark.rs:105-124`); in `query/integral.rs:100-114` an `Arming` is a promotion-ledger entry (the "wide arming" family).
- **undercut** — an emission below the tracked minimum, propagated outward (`watermark.rs:71-81`); `web_traffic.rs` classifies dominated undercuts.
- **freeze** — two meanings: `Bits::freeze`, the build-to-storage seam (`bits.rs:107-125`), and the integral's freeze trigger evicting live drift (`query/integral.rs:81-98`).
- **priced by** — the cost convention: work charged against bits already read or written at that width (`overlay.rs:76-84`).
- **hull** — `(meet, join, relation)` from one sweep (`emit.rs:146-160`); the `Hull` fold enum (`version.rs:1321`, driving `span_all` at `652-692`); `hull_traffic.rs` counts rungs.
- **seam** — used generically for a boundary between two forms or layers, never defined: `bits.rs:6`, `suanpan/src/magnitude.rs:1`.
- **envelope** — a pinned counter ceiling with ×1.25 slack (`tests/meter.rs:1-11`).
- **liveness floor** vs **improvement tripwire** — a derived minimum of irreducible work vs a measured ×0.75 band (`tests/meter.rs:35-53`; `board.rs:82-113`).
- **tripwire** (adequacy) — a committed known-bad artifact that must read red (`surface_coverage.rs:71-92`; `benches/tripwire.rs`; `tests/superlinear_tripwires.rs`).
- **family / Shape / FamilyId / FamilySpec** — a registered adversarial input family (`meter/registry.rs:1-59`); `Packed` is a generator's output (`meter.rs:92`).
- **board / cell / currency / shard** — the op × family matrix (`meter/board.rs:1`); one prepared measurement (`board/cell.rs:1`; distinct from public `shape::Cell`); one deterministic meter (`board/currency.rs:5-8`); one child process slice (`board/shard.rs:1`).
- **denomination** — the bytes a cost is charged against (`board/cell.rs:4-95`).
- **declared model** — an owner-ratified per-cell cost law replacing a global ceiling (`board.rs:168-187`).
- **band** — a pinned fuel line with widths (`fuzzfit/harness/src/bands.rs:1-22`); also envelope flatness bands and the registry's `Bands` (`registry.rs:1085`).
- **island** — one rendered `<details>` chart per operation (`build.rs:9-15`, `testing/fuelscape_islands.rs:1-11`).
- **atlas** — the population heatmap set (`before-fuelscape/src/lib.rs:1-14`; `dump.rs:14` `atlas.json`).
- **fuel** — wasmtime instruction count, the fuzz-fit and atlas currency (`fuzzfit/harness/src/lib.rs:8-13`).
- **luck-proof** — appears once, `meter.rs:26` ("the luck-proof touch list sit on the registry's `FamilyId`"), with no definition or occurrence in `registry.rs` (verified by grep); a dangling coinage.
- **Tier 2** — the sizer's name for the skyline coding (`meter/tier2.rs:1-2`); `testing/compactness.rs:4-5` still speaks of "the claim its adoption turns on".
- **leg** — a differential leg (`surface.rs:27` `Leg`: Bound/Law/Trans/Excluded) and, separately, a gate leg.
- **organic** — op-trace populations, as opposed to arbitrary normal forms (`testing/algebraic_laws.rs:16-28`).
- **suanpan terms** — lazy zone, recenter, quick register, zero-run ledger/certificate, collapsing sign fold, domination certificate, touch, limb (`suanpan/src/lib.rs:45-196`, `270-292`).

## 7. Public API surface

From `lib.rs:425-445`: `Clock`; `Party`; `Version`, `OwnVersion`, `Rank`, `Ranked`, `Ticks`, `Limbs`; `Span`, `OwnSpan`, `Dominance`, `Endpoint`, `Placement`, `Precedence`; `pub mod causally` (`after`, `before`, `strictly_after`, `strictly_before`, `since`, `until`, `delta`, `toward`, `all`, `Floor`, `Ceiling`, `Query`, `Coverage`, `Polarity`, `Down`, `Up`, `Neutral`); `pub mod error` (`Overlap`, `Crossed`, `TooWide`, `Decode`, `Parse`); `pub mod iter` (`Party`, `Clock` fork iterators); `pub mod shape` (`Plateau`, `Rise`, `Region`, `Cell`, `Plateaus`, `Regions`, `Overlay`, `Cells`, `combine`). Feature-gated instrument surface: `oracle`, `meter` (with `board`, `registry`, `tier2`, the `skyline` re-export, and the counter readers `meter.rs:3552-3722`), `surface`, `laws`; `serde`/`borsh` impls. The roster of record is `surface::METHOD_SURFACE` (96 `op:` rows) and `FAMILY_SURFACE` (36) — counts by grep, `surface.rs:322`, `1016`. The one `#[doc(hidden)]` item is the sealed `PartyLiteral` (`tests/doc_hidden.rs:21`); no foreign re-exports (`tests/foreign_reexport.rs:25-28`).

`suanpan` (`suanpan/src/lib.rs:352-362`): `Accumulator`, `Limbs`, `Magnitude`, `UBig` (re-export), and `touch_meter::{touches, reset}` under `touch-meter`.

## 8. Partitions and their neighbors

- **party** — reads `idbits` for tags, writes through `codec::PackedBuilder`; `diff` borrows `skyline::overlay::PlateauCursor`; `index` is the fold-only random-access table the `scan-meter` liveness floor in `party/tests.rs` guards.
- **version-core** — the doors are thin: every kernel is `skyline::*`; `join_refs`/`join_view` short-circuits must stay in lockstep (`version.rs:884-888`); `hull_traffic` counters live here.
- **rank** — `rank.rs` owns the wire form, `num.rs` the two-arm numerator whose ceiling the wasm32 pins hold to the real backend; the fold is `query::rank`.
- **span-causally** — both borrow `skyline::place`/`place::filter`; this is the upward-import cycle of section 5; `span/wire.rs:156-169` consumes `skyline::Admission` from the admission walk.
- **skyline-coding** — canonical form (`skyline.rs:65-90`) is what every other partition's byte-equality rests on; `text` is the production `Display`/`FromStr`; `build` is shared by emit, grow, fill, query::project.
- **skyline-fill-grow** — consumers of `walk`, `watermark`, `signed`, `idbits`; `grow::Cost` is imported by the oracle; the `luck-proof` and `Tier 2` wording issues are neighbors' docs.
- **skyline-sweep-place-masked** — all ride `overlay::{OpenedPair, advance_diff, advance_set}`; `sweep::Directions`/`eq_exit` are reused by masked, place, filter, emit.
- **skyline-query** — `integral` is the shared integrator for rank/distance/lag/rank_cmp; `web` drives `watermark::MinWeb`; the settle taps feed `limb_meter::record_densified`.
- **skyline-watermark** — two clients (fill, query::web); `pool_traffic`/`web_traffic` counters read via `meter`.
- **clock** — pure composition of party and version doors plus the folds in `fold.rs`; the depth-100k proof lives in its tests.
- **codec-bits** — `Bits::freeze` and `from_canonical` are the only storage gates; `scan::record_bits` hooks sit here.
- **codec-base-text-tree** — `Base` implements `suanpan::Magnitude`; `limb_meter` is the limb currency's source; `text.rs`/`tree.rs` are the id-side parsers (the event side is `skyline::text`).
- **crate-root** — `AGENTS.md:5-6` names a "Law of Disjointness" and a "public `implementation` module" that do not exist in `lib.rs` (verified); `examples/code_study.rs:6` links `before::implementation` too; `borsh_impls/tests.rs:225` links `crate::bookmark` (a `rumors` module).
- **suanpan** — `before` depends on the exact touch-count contract (`suanpan/src/lib.rs:283-286`); `Limbs`/`Magnitude` are the seams `codec::base` and `rank::num` use.
- **suanpan-tests** — `claims.rs` cites `../before/tests/meter.rs` as a witness file (`claims.rs:99`), so envelope test renames break suanpan's roster.
- **oracle-laws** — the oracle is deliberately recursive and unhardened; `laws` is consumed by `testing/algebraic_laws`, the fuzz target, and `verdict_matrix` transcriptions.
- **meter-core** — generators are private; only `registry::Shape` mints them; `tests/meter.rs` and the board are the two enforcement homes.
- **meter-registry-tier2** — `FamilyId` (52 variants by grep) is the axis for the board, the bands, the verdict matrix, and the fuelscape overlay; band-name parity is pinned in `tests/amp_board_smoke.rs`.
- **board-frame** — `ByCurrency` totality is the compile-time axis; `coverage.rs` tiling is tested against `surface_coverage`.
- **board-families-floors-judge** — floors are derived from operands (`operand.rs`), never measurements; `judge::trend` is the acceptance fit.
- **board-ops-render** — `ops.rs` rows consume bundle slots only; `worst.rs`'s `WORST_RANKINGS` is the gate pin; `shard.rs` is the process protocol.
- **surface-roster** — three jaws: `surface.rs` vs source scan (`surface_coverage`), vs rustdoc JSON (`surfacecheck`), vs `doc_hidden`/`foreign_reexport` pins; `surface-scan` is shared with suanpan's claims.
- **testing-oracles** — `bridge.rs` is the only depth-recursive code left, guarded by `recurse::descend!`; `validation_index.rs` is the crate's own map.
- **testing-diff-gen** — `diff_ops` tiling against `surface` `Bound` citations; `generators` (proptest) vs `meter` generators (closed-form) are different instruments (`generators.rs:21-24`).
- **envelopes-a / envelopes-b** — dev-profile pins (`tests/meter.rs:60-65`); `_reads_superlinear` kernels are rostered by `tests/superlinear_tripwires.rs`; band names are rostered by the registry.
- **tests-other** — `verdict_matrix` derives its pool from `FamilyId`; `bench_judge_roster` pins `tools/benchjudge-expected.json`; `fuzz_seeds` shares `support/fuzz_seed_set.rs` with the example.
- **benches-examples** — `benches/board.rs` IDs equal board cell names via `board::bench_cells`; `presize.rs` uses the `before_alloc_ab` cfg registered in `Cargo.toml:106-108`; `amp_board.rs` installs the allocator the board cannot own.
- **fuzz-guests-pins** — fuzz targets run under `fuzz/src/lib.rs`' heap cap; the wasm32 guest synthesizes inputs in-guest; pins are red-first.
- **fuzzfit-strategies** — families translate board shapes into programs built through public ops only; `Independent` regime crosses universes (cost binds, values are meaningless).
- **fuzzfit-bands** — `bands.rs` is generated by `bin/calibrate`; `ENFORCE_MARGIN_BELOW` is the liveness margin; `fuelscape` reuses `wasm.rs`'s driver.
- **fuelscape-pipeline** — `count`/`enumerate` are the sampler's adequacy pins; `ops.rs` coverage is bound to `surface` rows; enforces no fuel number.
- **fuelscape-render** — `compact.rs` output is committed under `crates/before/fuelscape/` and byte-verified by `fuelscape-verify`; `build.rs` refuses a stale `docs/fuelscape-header.html`.
