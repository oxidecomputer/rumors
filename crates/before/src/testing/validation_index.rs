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
//! **The pointwise differential table** ([`super::diff_ops`]). Every
//! deterministic operation in the three models' common vocabulary is spelled
//! once for production, the recursive oracle, and the function-space oracle.
//! Shared drivers apply every descriptor to arbitrary normal forms and values
//! produced by realistic operation traces.
//! What it alone catches: **population holes** — an operation checked on
//! arbitrary shapes but not on causally related values, or the reverse.
//!
//! **The trace differential** ([`super::optrace`]). One operation vocabulary
//! drives production and both oracles through the same stateful schedule. The
//! model trait is mandatory: adding a generated trace operation cannot compile
//! until all three implementations define it. What it alone catches:
//! **sequence errors** — a locally correct operation that leaves later
//! operations observing the wrong state.
//!
//! **Focused behavioral properties** (the public types' sibling test modules).
//! Public behavior outside that common vocabulary is tested at its natural
//! boundary: exact production policy against the recursive oracle, codecs and
//! streaming writers against canonical bytes, borrowing views against their
//! materialized values, and adapters against the operation they delegate to.
//! What they alone catch: **surface composition errors** that no independent
//! model represents, such as omitting one component from a streamed encoding.
//!
//! **The algebraic laws** (`crate::testing::laws`, driven by [`super::algebraic_laws`]
//! and shared with the fuzz targets). Law predicates over production alone:
//! lattice identities, monotonicity, the distance metric's axioms, order/rank
//! consistency. What they alone catch: contract violations **where no reference
//! implementation exists** — properties the paper never states (rank tiebreaks,
//! fork hand-back shapes) or that both oracles would share a blind spot on. A
//! law needs no oracle to disagree with; production either satisfies the
//! predicate or fails it.
//!
//! **Exhaustive small-scope enumeration** ([`super::exhaustive`]). Total
//! enumeration of every reachable state and operation pairing inside small
//! bounds, checked against the oracle. What it alone catches: **boundary
//! semantics proptests sample past** — the measure-zero corners (exact
//! equalities, empty regions, degenerate splits) that random generation hits
//! with vanishing probability. Within its scope the verdict is total, not
//! sampled; outside its scope it says nothing, which is exactly why the
//! proptest legs exist.
//!
//! **The 32-bit boundary pins** (the `wasm32-pins` workspace under this
//! crate). Public operations of this crate and of `suanpan` run in a wasm32
//! guest under wasmtime, on operands synthesized inside the guest, at the
//! exact coordinates where an index, a bit position, or a count crosses a
//! 32-bit `usize`. What they alone catch: **pointer-width boundary errors** —
//! a `usize` narrowing, wrap, or capacity limit that is the identity on a
//! 64-bit host, where every instrument above runs. The guest records the
//! message of every panic that reaches its panic hook, so a pin that expects
//! a documented panic fails when the operation instead aborts on allocation
//! failure, which ends in the same wasm trap. The pins do not exercise random
//! access into large streams. A stream bit position at or past `2^32` arises
//! in the pins only as a stream's live length or as the progress of a
//! sequential pass from bit zero: a reader consuming its input, or a writer
//! appending its output. No pin starts a read or a copy at such a position,
//! and the guest builds without the `borsh` feature, so the gamma decoder's
//! one-word window is absent from it. A starting position narrowed before it
//! becomes a byte index therefore passes every pin.
//!
//! # The resource instruments
//!
//! Cost claims are guarded by three complementary instruments: a global board
//! over chosen adversarial families, deterministic fuel over generated
//! programs, and a population atlas for inspecting the broader distribution.
//! A small focused suite remains for independent axes the board cannot vary.
//!
//! **The amplification board** (`crate::testing::meter::board`; rendered by
//! `just amp-board`).
//! The whole-surface dashboard: every operation × every committed
//! worst-case family, a four-size measurement ladder per cell, judged on
//! deterministic counters only (heap, scan bits, and digit touches) against
//! one fitted exponent trend per currency, per-size
//! constants, liveness floors, and owner-declared models. What
//! it alone catches: **structural blindness** — a resource regression on
//! a shape × operation pairing nobody thought to pin, and meter vacuity
//! (a counter that stopped watching reads a floor trip, not a green).
//! Red means untriaged, nothing else: every persistent contradiction
//! resolves to a cure or a declared model, and any red cell fails the
//! gate's board leg outright. It is the enforced per-operation record: every
//! compatible family reaches every compatible operation, and its shared
//! limits ratchet both growth and constant factors.
//!
//! **The focused resource checks** (`tests/meter.rs`). These vary an axis the
//! board deliberately holds fixed: an operation argument at fixed operands, a
//! paired control's marginal cost, the retained densification counter, or an
//! exact early-exit relationship. What they alone catch: **orthogonal cost
//! growth** that cannot be represented by scaling a board family's operands.
//! They also hold exact heap parities between entry points that compute the
//! same value, which catch **constant-size allocations** too small for the
//! board's peak-heap column to resolve, such as an operand clone on a path that
//! discards it.
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
//! input space, with the committed worst-case families overlaid as
//! marked points. What it alone provides: **the distribution** — an
//! audit view of where the bulk of the input space sends each
//! operation, so early-exit strata and log-factor banding are visible
//! to the eye. It enforces nothing (its committed checks are sampler
//! correctness and coverage parity against the board's operation inventory);
//! enforcement stays in the board and fuel bands.
//!
//! # The documentation instruments
//!
//! **The asymptotics liveness pins** ([`super::asymptotics`]). One pin
//! per documented non-linear mechanism — the fold entry points' log factor,
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
//! **The surface and board tiling** (`surfacecheck` and
//! [`crate::testing::meter::board`]'s coverage tables).
//! Every public function is priced by named board rows or excused with a
//! mechanism, never both and never neither. Public trait implementations are
//! pinned separately, with their resource decisions grouped into board
//! families. A new public function therefore cannot land unmeasured and
//! unexcused, and the board carries no orphan measurement.
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
//! - a 32-bit boundary pin reports `UnreachableCodeReached` with no panic
//!   message → the guest aborted without completing a panic, usually because
//!   memory ran out, either before the behavior under test or while
//!   formatting its panic message; find the allocation before reading the
//!   failure as a value defect. Other trap kinds name their own cause, such
//!   as `StackOverflow`.
//! - a board cell or focused resource check fails → the cost moved; measure at
//!   the parent commit before attributing, then either cure or bring
//!   the owner a declared-model case with the derivation.
//! - a liveness floor or band floor fails → a meter stopped watching,
//!   or a valid input legitimately did less work than the floor's
//!   premise — the latter is a floor-premise finding, not a meter bug.
//! - an asymptotics liveness pin fails → a documented mechanism is gone
//!   or moved; update the `# Complexity` sections and the pin in one
//!   change, whichever direction is correct.
