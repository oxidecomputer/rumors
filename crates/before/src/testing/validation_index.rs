//! The validation index: every instrument that guards this crate, what
//! failure class each one catches that the others cannot, and where it
//! lives.
//!
//! This page is a map for a maintainer orienting cold, in the spirit of
//! a documentation-only module: it holds no code. Two questions organize
//! the whole architecture — *is the answer right?* (semantic
//! instruments) and *is the cost right?* (resource instruments) — and
//! within each, the instruments are complementary: every one exists
//! because some failure class slips past all the others. When a change
//! trips one instrument, this page says which neighbors to check; when a
//! new instrument is proposed, the bar is a failure class no row below
//! already catches, named as a constructible input.
//!
//! # The semantic instruments
//!
//! **The public-surface coverage roster** (`crate::surface`, enforced by
//! [`super::surface_coverage`]). The differential architecture compares
//! three implementations — production, the recursive paper-transcription
//! oracle (`crate::oracle`), and the function-space semantic oracle
//! ([`super::semantic_oracle`]) — along three legs, and the roster
//! commits one row per public operation stating each leg's disposition:
//! bound by a named test, law-pinned, transitively bound, or excluded
//! with the reason. What it alone catches: **coverage holes** — a new
//! public operation cannot land unbound (the roster is held equal, name
//! for name, to the `pub fn` surface extracted from source), and a
//! renamed or deleted differential fails the row that cites it. The
//! differentials themselves catch wrong answers; the roster catches the
//! silent absence of a differential. The rows live in `crate::surface`
//! (public under the `meter` feature) exactly so external instrument
//! crates bind to the same enumeration instead of hand-maintaining a
//! second one.
//!
//! **The algebraic laws** (`crate::laws`, driven by
//! [`super::algebraic_laws`] and shared with the fuzz targets). Law
//! predicates over production alone: lattice identities, monotonicity,
//! the distance metric's axioms, order/rank consistency. What they alone
//! catch: contract violations **where no reference implementation
//! exists** — properties the paper never states (rank tiebreaks, fork
//! hand-back shapes) or that both oracles would share a blind spot on.
//! A law needs no oracle to disagree with; it convicts production by
//! itself.
//!
//! **Exhaustive small-scope enumeration** ([`super::exhaustive`]). Total
//! enumeration of every reachable state and operation pairing inside
//! small bounds, checked against the oracle. What it alone catches:
//! **boundary semantics proptests sample past** — the measure-zero
//! corners (exact equalities, empty regions, degenerate splits) that
//! random generation hits with vanishing probability. Within its scope
//! the verdict is total, not sampled; outside its scope it says nothing,
//! which is exactly why the proptest legs exist.
//!
//! # The resource instruments
//!
//! Cost claims are guarded by four instruments in a deliberate layering:
//! deterministic counters for enforcement breadth, process-isolated
//! envelopes for the enforced per-operation record, wall time only where
//! no counter can see the class, and fuel where no family was chosen at
//! all.
//!
//! **The amplification board** (`crate::meter::board`; rendered by
//! `just amp-board`).
//! The whole-surface dashboard: every operation × every committed
//! adversarial family, two probe sizes per cell, judged on deterministic
//! counters only (heap, stack segments, limb ops, scan bits, digit
//! touches) against fitted exponents, per-byte constants, liveness
//! floors, and owner-declared models — at both acceptance scales. What
//! it alone catches: **structural blindness** — a resource regression on
//! a shape × operation pairing nobody thought to pin, and meter vacuity
//! (a counter that stopped watching reads a floor trip, not a green).
//! Red means untriaged, nothing else: every persistent contradiction
//! resolves to a cure or a declared model, and the red-triage buffer
//! (`BOARD_EXPECTED_REDS`) is asserted empty at acceptance. The board
//! asserts breadth, not records: its readings are indicative, the
//! enforcement lives in the envelope suite.
//!
//! **The resource-envelope suite** (`tests/meter.rs`). The enforced
//! per-operation record: process-isolated scenarios (nextest, one
//! process per test) pinning exact counter envelopes with ×1.25 slack,
//! flatness bands over doubling schedules, liveness floors, and the
//! committed known-bad kernels (schoolbook converters, sequential-reduce
//! folds, retired quadratic walks) held red beside the green pins. What
//! it alone catches: **constant-factor regressions and cure
//! backslides** — the board's ceilings are class-scale and would forgive
//! a doubled constant; the envelope pins move only through a reviewed
//! diff. Its adequacy kernels are also the tripwires proving the
//! criteria can fail at all.
//!
//! **The bench judge** (`tools/benchjudge` over `benches/board.rs`;
//! `just bench-judge`). The wall-time exponent leg: criterion medians at
//! the two board scales, fitted per cell, judged through the committed
//! expected-verdict roster (`tools/benchjudge-expected.json`, membership
//! pinned by `tests/bench_judge_roster.rs`). What it alone catches:
//! **work invisible to every deterministic counter** — cost in layers
//! the meters do not instrument (backend multiplication below the limb
//! shim, container bookkeeping between metered primitives). It is the
//! one nondeterministic instrument, so it is judged as an exponent class
//! over medians (quick sampling for iteration, full sampling for any
//! quoted number, on a quiet machine) and never byte-pinned.
//!
//! **The fuzz-fit bands** (the `fuzzfit` workspace under this crate).
//! Public operations compiled to wasm and metered in wasmtime *fuel*
//! (deterministic, host-independent), with log-log fuel-vs-size bands
//! calibrated from a deterministic corpus and enforced over random
//! programs plus a deterministic prefix. What it alone catches:
//! **shapes nobody chose** — the structural blind spot of every
//! chosen-family instrument above — and total-cost drift that escapes
//! the metered currencies while still costing instructions. Its bands
//! carry liveness margins (`ENFORCE_MARGIN_BELOW`) so a dead
//! measurement reads red, not green.
//!
//! **The population atlas** (the `before-fuelscape` crate, an external
//! instrument). Per-operation heatmaps of deterministic fuel against
//! exact input size, sampled uniformly from each size's whole canonical
//! input space, with the committed adversarial families overlaid as
//! marked points. What it alone provides: **the distribution** — an
//! audit view of where the bulk of the input space sends each
//! operation, so early-exit strata and log-factor banding are visible
//! to the eye. It enforces nothing (its committed checks are sampler
//! correctness and coverage parity against `crate::surface`);
//! enforcement stays in the envelopes and bands, which is why it may
//! read the roster but never mint a threshold.
//!
//! # The documentation instruments
//!
//! **The asymptotics liveness pins** ([`super::asymptotics`]). One pin
//! per documented non-linear mechanism — the fold doors' log factor,
//! the render merge's superlinear growth, the settle's answer-embedded
//! product — each reading a deterministic counter or exact value
//! identity on a committed family. What they alone catch: **a
//! documented mechanism silently disappearing** — a cure or rewiring
//! that removes the behavior the rustdoc's `# Complexity` section still
//! claims flips the pin red, so the documentation moves in the same
//! change. The `# Complexity` prose itself is review-maintained: each
//! section states its own bound inline, denominated in its operation's
//! actual arguments.
//!
//! **The board tiling** ([`crate::meter::board`]'s coverage tables).
//! Every public-surface row priced by named board rows or excused with
//! a mechanism, never both, never neither — so a new public operation
//! cannot land unmeasured and unexcused, and the board carries no
//! orphan row.
//!
//! # Reading the map
//!
//! The scaffolding these instruments share — generators, the
//! oracle⇄impl bridge, the deterministic RNG, the op-trace driver — is
//! indexed in [`super`]'s module doc; wire-format pins (snapshots,
//! strict-decode rejection, fuzz seeds) ride the codec test suites and
//! the `tests/` binaries. A rough triage guide for a red instrument:
//!
//! - a differential or law fails → the answer is wrong; shrink it,
//!   commit the seed, fix production (never the oracle to match).
//! - a board cell or envelope pin fails → the cost moved; measure at
//!   the parent commit before attributing, then either cure or bring
//!   the owner a declared-model case with the derivation.
//! - a liveness floor or band floor fails → a meter stopped watching,
//!   or an honest input legitimately did less work than the floor's
//!   premise — the latter is a floor-premise finding, not a meter bug.
//! - an asymptotics liveness pin fails → a documented mechanism is gone
//!   or moved; update the `# Complexity` sections and the pin in one
//!   change, whichever direction is honest.
