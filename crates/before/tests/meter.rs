//! Deterministic resource envelopes for Before operations.
//!
//! Fixed input shapes bound peak transient heap, accumulator digit touches,
//! and encoded bits scanned. Scaling tests bound each cost per input unit
//! across a size doubling.
//!
//! # The columns
//!
//! [`metered`] reads three deterministic counters around each scenario body:
//!
//! - **Peak heap bytes**: the [`PeakAlloc`] delta over the scenario body.
//! - **Accumulator digit touches** (`suanpan::touch_meter`, under
//!   `touch-meter`): the cliff-free accumulator's own currency, where the
//!   folds and the tick walk do their arithmetic.
//! - **Scanned bits** ([`meter::scan_bits`], under `scan-meter`):
//!   encoded bits read and written through metered primitives.
//!
//! Stack depth is not a meter column. Dedicated deep-tree tests cover the
//! library's iterative traversals at depths that would overflow a recursive
//! implementation.
//!
//! The counters are process-global, so per-scenario readings are meaningful
//! only under nextest's process-per-test isolation. This test binary requires
//! the `touch-meter` and `scan-meter` features, so every counter is present.
//!
//! # The pin convention
//!
//! Each ceiling is 125% of a measured reading, rounded up. Each counter also
//! has one of two lower bounds:
//!
//! - An **improvement tripwire** ([`Floor::Tripwire`]) is 75% of the measured
//!   reading, rounded down. It distinguishes a genuine improvement from a
//!   counter that stopped observing work.
//! - A **liveness floor** ([`Floor::LiveBits`]) states a mechanism's
//!   irreducible work, never a measured basis: a walk that must read every
//!   live input bit, by contract (a strict validator, a decoder) or by
//!   construction (`ID_JOIN` and `ID_WITHOUT`, whose output depends on
//!   every tag; `ID_COVERS` and `ID_DISJOINT`, whose diverted pair is
//!   decided only at the last unary node), scans each at least once, less
//!   the tail the floor's definition allows past a decision. An improvement
//!   can approach but never cross it, so a trip means the work left the
//!   metered primitives; on a row whose walk also writes, the counter
//!   holds those writes too. This floor detects total bypass, not partial
//!   rerouting.
//!
//! Re-measure with this binary under `--no-capture --all-features`; it prints
//! each reading with a `MEASURED` prefix.
//!
//! Wall time is never asserted here: it is the one number that is not
//! deterministic. The pins are dev-profile, where assertions exercise the
//! same metered primitives as the operation. Every column's reading is
//! deterministic on a given target; CI checks the pins on a second target.

use before::testing::meter::registry::Shape;
use std::cmp::Ordering;
use std::fmt::{Debug, Write as _};

use before::testing::meter;
use before::{Party, Ticks, Version};
use num_bigint::BigUint;
use peak_alloc::PeakAlloc;

#[global_allocator]
static HEAP: PeakAlloc = PeakAlloc;

#[path = "meter/fixtures.rs"]
mod fixtures;
use fixtures::*;
#[path = "meter/pins.rs"]
mod pins;
use pins::*;
#[path = "meter/harness.rs"]
mod harness;
use harness::*;
/// Resource envelopes for core operations on the dense-spine family.
#[path = "meter/dense_operations.rs"]
mod dense_operations;
/// Resource envelopes for ranks and rank arithmetic.
#[path = "meter/rank_operations.rs"]
mod rank_operations;
/// Core version operations across wide-value and alternating-shape families.
#[path = "meter/shape_operations.rs"]
mod shape_operations;
/// Count-scaling checks for `Version::ticks`.
#[path = "meter/tick_counts.rs"]
mod tick_counts;
/// Validation envelopes for encoded version streams.
#[path = "meter/version_codec.rs"]
mod version_codec;
/// Flatness checks for adversarial version folds.
#[cfg(feature = "touch-meter")]
#[path = "meter/version_scaling.rs"]
mod version_scaling;

/// A cost band proving comparison stops after a decisive prefix.
#[cfg(feature = "touch-meter")]
#[path = "meter/eq_early_exit.rs"]
mod eq_early_exit;

/// A wide-by-dense rank family that prices promotion settlement.
#[cfg(feature = "touch-meter")]
#[path = "meter/ledger_wide_arming.rs"]
mod ledger_wide_arming;

/// A rank family that separates live spans from absolute positions.
#[cfg(feature = "touch-meter")]
#[path = "meter/hoisted_window.rs"]
mod hoisted_window;

/// A rank family whose exact answer requires wide multiplication.
#[cfg(feature = "touch-meter")]
#[path = "meter/answer_embedded_product.rs"]
mod answer_embedded_product;

/// Flatness checks for repeated rank settlement.
#[cfg(feature = "touch-meter")]
#[path = "meter/settle_flatness.rs"]
mod settle_flatness;

/// Resource envelopes for party operations on deep ownership trees.
#[path = "meter/party_operations.rs"]
mod party_operations;
use party_operations::id_pair_input_bytes;
/// Exact scan pins for deep party walks.
#[cfg(feature = "scan-meter")]
#[path = "meter/id_walk_scan_cost.rs"]
mod id_walk_scan_cost;

/// Direct cost checks for the accumulator streams used by version walks.
#[cfg(feature = "touch-meter")]
#[path = "meter/accum_streams.rs"]
mod accum_streams;

/// Resource envelopes for rank, minimum-tick, and projection kernels.
#[path = "meter/query_kernels.rs"]
mod query_kernels;
use query_kernels::query_env;
/// Resource envelopes for balanced folds and their identity fast path.
#[path = "meter/fold_operations.rs"]
mod fold_operations;
/// A nonshrinking population that checks balanced meet folds.
#[cfg(feature = "touch-meter")]
#[path = "meter/meet_fold.rs"]
mod meet_fold;
/// Resource envelopes for ticking versions across adversarial shapes.
#[path = "meter/tick_operations.rs"]
mod tick_operations;
/// Resource envelopes for pairwise version queries and masked comparisons.
#[path = "meter/version_queries.rs"]
mod version_queries;

/// Cost checks for repeated wide minimum reveals during ticking.
#[cfg(feature = "touch-meter")]
#[path = "meter/width_circulation_cost.rs"]
mod width_circulation_cost;

/// Liveness and cost checks for dominated minimum emissions.
#[cfg(feature = "touch-meter")]
#[path = "meter/dominated_undercut_cost.rs"]
mod dominated_undercut_cost;

/// Relational scan checks for causal filtering and placement.
#[cfg(feature = "scan-meter")]
#[path = "meter/placement.rs"]
mod placement;

/// Relational scan checks for fused span construction.
#[cfg(feature = "scan-meter")]
#[path = "meter/span.rs"]
mod span;

/// Relational cost checks for fused span decoding.
#[cfg(feature = "scan-meter")]
#[path = "meter/span_codec.rs"]
mod span_codec;

/// Scan-liveness checks for clone-identity fast paths.
#[cfg(feature = "scan-meter")]
#[path = "meter/identity_fast_paths.rs"]
mod identity_fast_paths;
