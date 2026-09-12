//! Queue sizing for pipelined reconciliation.
//!
//! A session can work on several subtrees while earlier replies are in flight.
//! Wider queues overlap those waits but retain more state. The greeting supplies
//! both set sizes and version-size bounds; [`Window::from_budget`] combines
//! them with [`Backend::node_bytes`] to choose fixed capacities for the session.
//!
//! # From tree shape to queue width
//!
//! A *scope* is the prefix named by a query. A query for a depth-`d − 1`
//! scope carries child references at depth `d`; the queue is labelled with
//! those children's typed height, `KEY_DEPTH − d`. Depth counts from the root,
//! while typed height counts from the leaves.
//!
//! The model assumes uniform hashes of distinct leaf versions. With set sizes
//! `A` and `B`, three bounds describe the work at each depth:
//!
//! - [`occupied`] bounds occupied prefixes by the number of available prefixes
//!   and leaves. This bound is deterministic.
//! - [`disputed`] bounds prefixes both replicas hold with different contents.
//!   Matching subtrees stop descent; one-sided subtrees are supplied whole.
//!   Its mean bound is `A·B / 256ʲ` at depth `j`. Shared messages do not make
//!   replicas independent; the function's comment explains why this bound
//!   still applies to their differences.
//! - [`children_quantile`] bounds a parent's occupied children, using both
//!   its leaf count and its occupied child slots, capped by the radix.
//!
//! A disputed prefix at depth `d − 2` can list children at depth `d − 1`;
//! these become the next queried scopes, retaining references at depth `d`.
//! [`stage_population`] combines the parent and child bounds into `S(d)`,
//! the population used to size that queue. The opening root query is a
//! separate, single-scope case.
//!
//! Each queue receives `max(1, min(K, S(d)))` slots. The byte budget chooses
//! `K`; the population bound stops it from buying width where little work is
//! expected. [`Window::from_budget`] prices the retained child references and
//! fixed scope state at each depth, adds the decode buffers, and searches for
//! the largest affordable `K`.
//!
//! # What the bounds promise
//!
//! The statistical bounds use a tail probability of 2⁻⁴⁸ per statistic.
//! Per-parent bounds cover every possible parent, including ones selected by
//! the protocol's descent. Their combined failure probability is below 2⁻⁴⁰
//! under the uniform-hash model; [`UNION_TAIL_BITS`] explains the accounting.
//! The numerical tests check the integer approximations against a separate
//! Chernoff calculation, and session tests check the scope/depth mapping.
//!
//! This probability concerns the statistical estimates, **not** whether a
//! queue fills. A small budget deliberately chooses `K < S(d)`, so ordinary
//! traffic can encounter backpressure. Clustered paths can exceed `S(d)` even
//! with a large budget. Either way, bounded queues wait for consumers rather
//! than growing to hold the whole frontier.
//!
//! Every window queue keeps at least one slot, preserving progress under the
//! [materialized walk's ordering rules](super::materialized) and the
//! [link contract](crate::link). This floor may cost more than a tiny budget.
//! The byte calculation also relies on its fan-size estimates and the backend's
//! pricing contract; a queue-count limit alone is not a hard resident-memory cap.
//! Replica storage, transport buffers, and other sessions are separate costs.
//!
//! # Queues outside the window
//!
//! Assembly return queues keep a full fan so completed children can wait while
//! the walk constructs their parent resolution. Decode buffers also keep a
//! fixed fan of leaves plus the reader's current record; their cost is charged
//! before selecting a window width. These are different queues with different
//! purposes, although both use the radix as their capacity.
//!
//! The proxy's flushed-question queue *does* use the window. It tracks questions
//! already sent but not yet answered. An upstream queue can recycle its slots
//! while answers at this depth remain pending, so its capacity does not limit
//! this queue's total population. The flushed-question queue needs its own
//! depth-specific bound, just like the walk's query and resolution queues.

use super::{Backend, Local, materialized::Resolve};
use crate::link::STREAM_COUNT;
use crate::tree::typed::{self, Prefix, height::Z};

/// The tree's maximum branching factor: one child per radix byte.
///
/// Also the hard capacity floor of the assembly fan queues (see the
/// [module docs](self)): those channels must admit one *full* fan
/// regardless of any window tuning.
pub(crate) const FAN: usize = 256;

/// Radix levels in the trie: one byte of a 32-byte leaf path per
/// level. Typed heights run from `Z = 0` (leaves) to `Root = KEY_DEPTH`;
/// the *depth* of the children discussed at height `h` is `KEY_DEPTH − h`.
const KEY_DEPTH: usize = typed::hash::PATH_LEN;

/// In-memory bytes of one child's slots in a level's in-flight
/// containers: a query slot, a resolution slot, and a listing entry.
///
/// Derived from `size_of` of the real slot types under the in-memory
/// backend, so a layout change moves the price with it instead of
/// leaving a hand-counted byte total stale: the pointer-aligned
/// `(u8, node-handle)` query slot; the `(u8, Resolve)` resolution
/// slot (`Option<Node>` consumes the handle's only null niche, so
/// the `Ready`/`Pending` tag sits out of line and the pair outgrows the
/// query slot by a word); and the byte-packed `(u8, Hash)` listing
/// entry. `Resolve`'s layout does not depend on the height or payload
/// parameters, so the leaf instantiation prices every level. Exact for
/// pointer-class node handles; a backend whose `Node` demands a wider
/// layout pads the real slots beyond this constant and owes that padding
/// to its own `node_bytes` price.
pub(crate) const REFERENCE_SLOT_BYTES: usize = std::mem::size_of::<(u8, typed::Node<Z>)>()
    + std::mem::size_of::<(u8, Resolve<<Local as Backend>::Erased>)>()
    + std::mem::size_of::<(u8, typed::Hash)>();

/// Fixed in-memory bytes per buffered scope beyond its per-child slots:
/// two inline prefixes (40 B each) and the container `Vec` headers.
const SCOPE_FIXED_BYTES: usize = 2 * 40 + 2 * 24;

/// In-memory bytes of one buffered leaf request: an inline leaf prefix.
const LEAF_REQUEST_BYTES: usize = 40;

/// In-memory bytes one decode-fan slot spends beyond the leaf node value
/// it carries: the inline leaf prefix and the pair's padding.
///
/// Derived from the queue's real item layout — `size_of` of the
/// prefix-and-node pair minus the node value — so a layout change moves
/// the price with it instead of leaving a hand-counted byte total stale.
/// The derivation is exact for pointer-class node handles; a backend
/// whose `Node<Z>` demands wider alignment pads the real slot beyond
/// `node_bytes + FAN_SLOT_BYTES` and owes that padding to its own
/// `node_bytes` price.
pub(crate) const FAN_SLOT_BYTES: usize =
    std::mem::size_of::<(Prefix<Z>, typed::Node<Z>)>() - std::mem::size_of::<typed::Node<Z>>();

/// Worst-case bytes the decode fans of one session keep resident, under
/// the in-memory backend's pricing.
///
/// Derived, term by term: each of the [`STREAM_COUNT`] reply streams a
/// session decodes owns one fan channel of [`FAN`] slots plus the record
/// in the reader's hand, and each occupant is one backend-priced leaf
/// node in its slot — for the in-memory backend, a pointer-sized handle
/// (pinned by the `Local` handle assertion) beside [`FAN_SLOT_BYTES`] of
/// slot. [`from_budget`](Window::from_budget) charges the same shape
/// through the live backend's own `node_bytes`; this constant is that
/// charge under the in-memory backend, the flat pre-charge the operator
/// docs quote.
#[cfg(any(test, feature = "test-internals"))]
pub(crate) const SUPPLY_DECODE_ENVELOPE_BYTES: usize =
    STREAM_COUNT * (FAN + 1) * (std::mem::size_of::<typed::Node<Z>>() + FAN_SLOT_BYTES);

/// The specification link's bandwidth-delay product, in bytes: 12.5 MB,
/// where the two spec links coincide.
///
/// 100 Gbps × 1 ms round trip and 1 Gbps × 100 ms are the same product;
/// the trade-off table and the crossover figures in the budget docs are
/// stated at it.
#[cfg(any(test, feature = "test-internals"))]
pub(crate) const SPEC_BDP_BYTES: usize = 12_500_000;

/// Approximate wire overhead per differing message, excluding its encoded payload.
///
/// `tests/dispute_wire.rs` counts all protocol writes in both directions,
/// divides by the differing messages, and subtracts their encoded payloads.
/// The reference fixture has 2,048 shared messages and 8,192 new ones per
/// side. Its whole-byte mean gives 43; the unrounded mean is about 43.9.
/// Two fully divergent 100,000-message sets instead average about 41.7.
/// These include hashes, versions, framing, and amortized session setup;
/// none is a fixed header size or a bound on other workloads.
#[cfg(any(test, feature = "test-internals"))]
pub(crate) const DISPUTE_OVERHEAD_BYTES: usize = 43;

/// A round encoded payload size above the sizing guide's measured BDP crossover.
#[cfg(any(test, feature = "test-internals"))]
pub(crate) const DESIGN_RECORD_BYTES: usize = 100;

/// Wire cost of one reference-size message, checked by the wire calibration suite.
#[cfg(any(test, feature = "test-internals"))]
pub(crate) const DISPUTE_WIRE_BYTES: usize = DISPUTE_OVERHEAD_BYTES + DESIGN_RECORD_BYTES;

/// Average modeled bytes per scope with every stage at its population limit.
///
/// The calibration case has two fully divergent sets of 62,500 messages.
/// `scope_envelope_matches_the_derivation` recomputes the average using
/// `Local` pricing. This is a calibration value, not an input to the window.
/// It is unsuitable as a fixed price at small budgets: decode buffers and
/// near-root stages contribute costs that do not scale with window width.
#[cfg(any(test, feature = "test-internals"))]
pub(crate) const SCOPE_ENVELOPE_BYTES: usize = 5_431;

/// Default target for one synchronization's pipeline memory: 512 MiB.
///
/// See [`Peer::sync_memory_budget`](crate::Peer::sync_memory_budget) for
/// the contract and [the sizing guide](crate::sizing) for tuning guidance.
pub const DEFAULT_SYNC_MEMORY_BUDGET: usize = 512 * 1024 * 1024;

/// Per-height channel capacities for one session, in disputed scopes.
///
/// Constructed from the two replicas' exchanged set sizes, a memory
/// budget, and the backend's per-node price by
/// [`from_budget`](Self::from_budget) — usually through a
/// [`WindowConfig`] once the greeting supplies the sizes — and consumed
/// by the channel constructors of the materialized walk and the remote
/// proxy, each at the typed height its items carry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Window {
    /// Channel capacity per typed item height, `0..=KEY_DEPTH`.
    capacities: [usize; KEY_DEPTH + 1],
}

/// Derive and inspect capacities by typed item height.
impl Window {
    /// The liveness floor: one scope per edge, the deadlock-proof minimum.
    /// Reached only through [`WindowConfig::FLOOR`], the test suites'
    /// opt-in.
    #[cfg(any(test, feature = "test-internals"))]
    pub(crate) const FLOOR: Self = Self {
        capacities: [1; KEY_DEPTH + 1],
    };

    /// Give every non-root level the same capacity in protocol tests.
    ///
    /// The budget calculation assumes hash-distributed paths and keeps the
    /// deepest queues at one slot. This lets fixtures exercise pipelining
    /// there. The root keeps its structural one-slot channels.
    #[cfg(test)]
    pub(crate) const fn uniform(capacity: usize) -> Self {
        assert!(capacity > 0, "a protocol queue needs at least one slot");
        let mut capacities = [capacity; KEY_DEPTH + 1];
        capacities[KEY_DEPTH] = 1;
        Self { capacities }
    }

    /// Choose fixed queue widths from the greeting and this session's byte budget.
    ///
    /// At each child depth `d`, a queued scope retains up to `C(d − 1)` child
    /// references. Each reference is priced at its own fan bound `C(d)` by
    /// the backend, plus its query, resolution, and listing slots. `S(d)` bounds
    /// how many such scopes the population can offer. See the module docs for
    /// the depth/height mapping and the statistical assumptions.
    ///
    /// After charging fixed decode buffers, find the largest common width `K`
    /// fitting the sum of `min(K, S(d))` scope charges. Each queue then gets
    /// at least one slot, even if that floor exceeds the budget. This is a
    /// sizing calculation, not an allocation or a measurement of live memory.
    ///
    /// Version prices use twice the sum of the replicas' encoded-version
    /// bounds: a result combines both histories and carries a ceiling and a
    /// floor. Deletion-pruned subsets are checked separately by the conformance
    /// and retained-root census tests. The backend's price must be monotone in
    /// both fan size and version size for these upper estimates to be useful.
    pub(crate) fn from_budget(
        local_messages: u64,
        remote_messages: u64,
        local_version_bytes: u64,
        remote_version_bytes: u64,
        budget_bytes: usize,
        node_bytes: impl Fn(usize, usize) -> usize,
    ) -> Self {
        let n = u128::from(local_messages.max(remote_messages));
        let pair = u128::from(local_messages) * u128::from(remote_messages);
        let budget = budget_bytes as u128;
        // Ceiling and floor each encode within the exchanged aggregates'
        // sum `local + remote` (each side's aggregate covers the bounds
        // it materializes; a cross-side assembly joins or meets one from
        // each, within the pinned pairwise lemmas; deletion-pruned
        // survivor bounds are *priced* within the same sum, guarded by
        // the census pin); the pair together within its double.
        let version_bound = usize::try_from(
            2 * (u128::from(local_version_bytes) + u128::from(remote_version_bytes)),
        )
        .unwrap_or(usize::MAX);

        #[cfg(debug_assertions)]
        for window in [0usize, 1, 16, FAN].windows(2) {
            debug_assert!(
                node_bytes(window[0], version_bound) <= node_bytes(window[1], version_bound),
                "node_bytes must be monotone in the child count",
            );
            debug_assert!(
                node_bytes(window[1], version_bound / 2) <= node_bytes(window[1], version_bound),
                "node_bytes must be monotone in the version bound",
            );
        }

        // Populations and per-scope fans per depth, computed once. A
        // buffered scope whose children sit at depth d is one queried
        // entry of the depth-d stage; its held references are the
        // children of one depth-(d−1) parent, each priced at its own
        // depth's fan quantile.
        let mut population = [0u128; KEY_DEPTH + 1];
        let mut scope_price = [0u128; KEY_DEPTH + 1];
        for depth in 1..=KEY_DEPTH {
            let held = children_quantile(n, depth).try_into().unwrap_or(usize::MAX);
            // Widened before the add: a backend pricing nodes near
            // `usize::MAX` must not wrap the slot term away.
            let reference_bytes =
                node_bytes(held, version_bound) as u128 + REFERENCE_SLOT_BYTES as u128;
            population[depth] = stage_population(n, pair, depth);
            scope_price[depth] =
                children_quantile(n, depth - 1) * reference_bytes + SCOPE_FIXED_BYTES as u128;
        }

        // Decode buffering is independent of K: one fixed leaf channel and
        // an in-hand record per reply stream. Price it before widening queues.
        let supply_fans = (STREAM_COUNT as u128)
            * (FAN as u128 + 1)
            * (node_bytes(0, version_bound) as u128 + FAN_SLOT_BYTES as u128);

        // Sum the scope charges across depths. Each price includes the
        // query, resolution, and listing views; do not multiply it again
        // by the number of queues carrying those views.
        //
        // Leaf requests retain only a prefix and need their own slot charge.
        // Zero population estimates still receive one progress slot below,
        // just like an unaffordable one-slot window.
        //
        // Saturation makes an unrepresentable charge unaffordable, so large
        // populations or backend prices can narrow the window but never wrap
        // into a falsely cheap width.
        let charge = |k: u128| -> u128 {
            let mut total = supply_fans;
            for depth in 1..=KEY_DEPTH {
                total = total
                    .saturating_add(population[depth].min(k).saturating_mul(scope_price[depth]));
            }
            total.saturating_add(population[KEY_DEPTH].min(k) * LEAF_REQUEST_BYTES as u128)
        };

        // No modeled population benefits from K above the largest S(d).
        // Charge is monotone in K. Keep the affordable lower half, rounding
        // the midpoint upward so adjacent bounds still make progress.
        let ceiling = population.iter().copied().max().unwrap_or(1).max(1);
        let (mut lo, mut hi) = (1u128, ceiling);
        while lo < hi {
            let mid = lo + (hi - lo).div_ceil(2);
            if charge(mid) <= budget {
                lo = mid;
            } else {
                hi = mid - 1;
            }
        }

        // Convert child depths back to typed heights. The one-slot floor
        // takes precedence over both the budget and a zero population estimate.
        let mut capacities = [1usize; KEY_DEPTH + 1];
        for (height, capacity) in capacities.iter_mut().enumerate() {
            let depth = KEY_DEPTH - height;
            if depth == 0 {
                // Height KEY_DEPTH is the root itself: no queried
                // population exists above it, and its edges are the
                // structural one-slot root channels.
                continue;
            }
            *capacity = population[depth]
                .min(lo)
                .clamp(1, usize::MAX as u128)
                .try_into()
                .expect("clamped to usize range");
        }
        Self { capacities }
    }

    /// The channel capacity for a window edge whose items carry typed
    /// height `height`.
    pub(crate) fn capacity(&self, height: usize) -> usize {
        self.capacities[height.min(KEY_DEPTH)]
    }

    /// The widest per-height capacity this window grants, in disputed
    /// scopes: the summary a session reports as
    /// [`SessionStats::window_granted`](super::stats::SessionStats::window_granted).
    pub(crate) fn widest(&self) -> u64 {
        self.capacities
            .iter()
            .copied()
            .max()
            .expect("a window always carries every height's capacity") as u64
    }
}

/// How a session chooses its window: fixed capacities, or a byte budget
/// resolved against the set sizes the greeting exchanges.
// A fixed table is ~260 B against `Budget`'s word; the config lives one
// per peer and per in-flight session, so indirection would spend an
// allocation to save nothing that matters.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Copy, Debug)]
pub(crate) enum WindowConfig {
    /// Predetermined capacities for tests, including [`Self::FLOOR`].
    #[cfg(any(test, feature = "test-internals"))]
    Fixed(Window),
    /// Derive per-height capacities at session start, once both replicas'
    /// sizes are known.
    Budget(usize),
}

/// Resolve configured policy once the session's sizes are known.
impl WindowConfig {
    /// The one-slot serialization floor, pinned: every session edge at
    /// the capacity where a bad ordering would deadlock.
    ///
    /// Tests opt in explicitly so the capacity-one orderings the
    /// deadlock-freedom argument certifies stay exercised; the
    /// [`Default`] is the budget and never depends on how the crate is
    /// built (features are additive and must not change behavior).
    #[cfg(any(test, feature = "test-internals"))]
    pub(crate) const FLOOR: Self = Self::Fixed(Window::FLOOR);

    /// Resolve the session's window against the exchanged set sizes and
    /// version-size bounds.
    pub(crate) fn resolve(
        self,
        local_len: u64,
        remote_len: u64,
        local_version_bytes: u64,
        remote_version_bytes: u64,
        node_bytes: impl Fn(usize, usize) -> usize,
    ) -> Window {
        match self {
            #[cfg(any(test, feature = "test-internals"))]
            Self::Fixed(window) => window,
            Self::Budget(bytes) => Window::from_budget(
                local_len,
                remote_len,
                local_version_bytes,
                remote_version_bytes,
                bytes,
                node_bytes,
            ),
        }
    }
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self::Budget(DEFAULT_SYNC_MEMORY_BUDGET)
    }
}

// Integer approximations to the occupancy tails. Uniform leaf placement makes
// prefix occupancies negatively associated: learning that one prefix is crowded
// cannot make disjoint prefixes collectively more crowded. Chernoff bounds
// therefore apply despite dependence between prefixes. This property survives
// combining independent leaf families and applying increasing functions to
// disjoint groups (Dubhashi–Ranjan, Proposition 7).
//
// The arguments below explain each rounding step. The tests in `envelope`
// compare their results with a separate numerical calculation; those sampled
// checks protect the arithmetic, while `application` checks actual session
// work.

/// `256^j`, saturating above every corpus size and product of two corpus sizes.
fn pow256(j: usize) -> u128 {
    if j >= 16 { u128::MAX } else { 1u128 << (8 * j) }
}

/// The per-statistic tail level, in bits: every (stage, statistic)
/// quantile is taken at probability 2⁻⁴⁸.
///
/// The union accounting that turns it into the per-session bound: a session
/// uses one disputed-prefix bound and two per-parent routes for each replica,
/// at each depth. Across 32 depths that is fewer than 2⁸ bounds, so the union
/// costs at most 2⁸ × 2⁻⁴⁸ = 2⁻⁴⁰ per session. A per-parent quantile must hold
/// for every candidate parent at its depth simultaneously; the 8·j-bit
/// sharpening in [`tail_exponent`] pays that union, taken over all 256ʲ
/// candidate prefixes *without* conditioning on the parent being queried
/// (`P(queried ∧ X ≥ a) ≤ P(X ≥ a)`), which sidesteps the conditioning
/// inflation entirely.
const UNION_TAIL_BITS: usize = 48;

/// Flat-tail exponent, using the same upward rounding as the per-parent bound.
const BERNSTEIN_TAIL: u128 = tail_exponent(0);

/// Integer upper bound on `ln 2 ×` the union tail bits at parent depth
/// `j`: `⌈0.7 × (UNION_TAIL_BITS + 8j)⌉`, for `j ≤ KEY_DEPTH`.
///
/// The `8j` term pays the union over the `256ʲ` candidate parents at depth `j`:
/// a per-parent quantile at tail `2^−(48+8j)` holds for all of them
/// simultaneously at the flat 2⁻⁴⁸ statistic level ([`UNION_TAIL_BITS`]). The
/// coefficient is sound because `0.7 > ln 2 ≈ 0.693`, so `e⁻ᵗ ≤ 2^−bits`
/// whenever `t ≥ ⌈0.7 × bits⌉`.
const fn tail_exponent(j: usize) -> u128 {
    (7 * (UNION_TAIL_BITS + 8 * j) as u128).div_ceil(10)
}

/// Integer quantile from the multiplicative Chernoff tail
/// `P(X ≥ μ + x) ≤ exp(−x²/(2μ + x))`: `x = ⌊√(2μt)⌋ + t` suffices for
/// tail `≤ e⁻ᵗ`.
///
/// Sufficiency, with `s = ⌊√(2μt)⌋`: the requirement `x² ≥ t(2μ + x)`
/// reduces to `s² + st ≥ 2μt`, and `(s + 1)² > 2μt` leaves a deficit
/// of at most `2s`, covered by `st` at any `t ≥ 2` (every caller's `t`
/// is ≥ 34). Callers pass `mean_hi` as an upper bound on the true
/// mean; the quantile is monotone in `μ`, so an overestimate only
/// widens the envelope.
fn bernstein(mean_hi: u128, t: u128) -> u128 {
    mean_hi + (2 * mean_hi * t).isqrt() + t
}

/// Integer quantile for a sub-unit mean `num/256^j`, or `None` when the
/// mean is not clearly sub-unit.
///
/// Uses the Poisson-type tail `P(X ≥ a) ≤ (eμ)^a` — the binomial union
/// bound `(N choose a) pᵃ ≤ (eμ/a)ᵃ ≤ (eμ)ᵃ` at `a ≥ 1`, `μ = Np`.
/// With `b = bitlen(256^j) − bitlen(num) ≥ 5` (so `μ ≤ 2^−(b−1)` and
/// `eμ ≤ 2^(2.45−b)`, since `e < 2^1.45`), `a = t/(b−3) + 2` exceeds
/// `t/(b−2.45)` and so gives tail `≤ 2⁻ᵗ`; and if `eμ ≤ 2⁻ᵗ` even one
/// occurrence exceeds the level, so the quantile is zero.
fn small_mean_quantile(num: u128, j: usize, t: u128) -> Option<u128> {
    if num == 0 {
        return Some(0);
    }
    let den_bits = (8 * j + 1) as u32;
    let num_bits = 128 - num.leading_zeros();
    // num × 2^(t+2) < 256^j, compared in exponents since 256^j is a
    // power of two: strict inequality holds whenever num's bit length
    // stays under the remaining headroom.
    if u128::from(num_bits) + t + 2 < u128::from(den_bits) {
        return Some(0);
    }
    let b = u128::from(den_bits.saturating_sub(num_bits));
    if b >= 5 {
        return Some(t / (b - 3) + 2);
    }
    None
}

/// Occupied depth-`j` slots: both caps are deterministic — one slot per
/// level per leaf — so no concentration term is needed.
fn occupied(n: u128, j: usize) -> u128 {
    if j == 0 {
        return u128::from(n >= 1);
    }
    pow256(j).min(n)
}

/// Bound prefixes both replicas occupy with different contents at depth `j`.
///
/// Shared leaves alone produce a matching subtree and stop descent. Split the
/// leaves into three independent families: shared, left-only, and right-only.
/// A prefix is disputed when it contains at least two families. If their
/// occupancy probabilities are `s`, `a`, and `b`, this event has probability
/// `ab + as + bs − 2abs`.
///
/// The product of the replicas' occupancy marginals is an upper bound: its
/// excess over that expression is `s²(1 − a)(1 − b) ≥ 0`. Each marginal is at
/// most its set size divided by `256ʲ`, giving mean at most `A·B / 256ʲ`.
/// The disputed-prefix indicators remain negatively associated because the
/// two-families condition increases with each family's occupancy.
///
/// Apply a tail bound to that mean, capped by the prefix count and smaller
/// corpus. `n = max(A, B)` and `pair = A·B`, so `pair / n = min(A, B)`.
fn disputed(n: u128, pair: u128, j: usize) -> u128 {
    if j == 0 {
        // An empty side disputes nothing and only receives. Otherwise
        // the root may be disputed, so reserve room for it.
        return u128::from(pair >= 1);
    }
    let smaller = pair.checked_div(n).unwrap_or(0);
    let quantile = match small_mean_quantile(pair, j, UNION_TAIL_BITS as u128) {
        Some(q) => q,
        None => bernstein(pair / pow256(j) + 1, BERNSTEIN_TAIL),
    };
    pow256(j).min(smaller).min(quantile)
}

/// Per-parent quantile, leaves route: occupied sub-slots under one
/// depth-`j` parent are at most its leaves, `Binomial(N, 256⁻ʲ)` —
/// Bernstein at the union level in the bulk, Poisson-type below unit
/// mean.
fn leaves_quantile(n: u128, j: usize) -> u128 {
    if j > 0
        && let Some(q) = small_mean_quantile(n, j, (UNION_TAIL_BITS + 8 * j) as u128)
    {
        return q;
    }
    let mean_hi = if j > 0 { n / pow256(j) } else { n } + 1;
    bernstein(mean_hi, tail_exponent(j))
}

/// Bound occupied child slots under one depth-`j` parent.
///
/// The exact mean is `256 × (1 − (1 − p)^N)`, with `p = 256^−(j+1)`.
/// The rational approximation `256 × 2Np / (2 + Np)` avoids floating point;
/// adding one covers integer division's rounding loss.
///
/// To check the approximation, put `x = Np`. The exact mean is at most
/// `256 × (1 − exp(−x/(1−p)))`. Since `1 − exp(−x) ≤ 2x/(2+x)`, using `x`
/// instead of `x/(1−p)` loses at most `256p/(2(1−p)) ≤ 128/255 < 1` slot.
/// Bernstein's slack covers this: with `s = floor(sqrt(2·mean_hi·t))`,
/// raising the true mean by one leaves the tail inequality slack at least
/// `s(t−4) − 3t + 1 > 0`, since `mean_hi ≥ 1`, `t ≥ 34`, and `s ≥ 8`.
/// The numerical reference checks the final quantile against the exact mean.
fn child_slots_quantile(n: u128, j: usize) -> u128 {
    let fan = FAN as u128;
    let child_slots = pow256(j).saturating_mul(fan);
    let mean_hi = fan.min(2 * n * fan / (2u128.saturating_mul(child_slots).saturating_add(n)) + 1);
    fan.min(bernstein(mean_hi, tail_exponent(j)))
}

/// Per-parent children quantile at parent depth `j`: the structural fan,
/// the leaves route, and the slots route, at their minimum.
fn children_quantile(n: u128, j: usize) -> u128 {
    (FAN as u128)
        .min(leaves_quantile(n, j))
        .min(child_slots_quantile(n, j))
}

/// Bound queried scopes that retain child references at depth `d`.
///
/// A scope at depth `d − 1` is one child listed by a disputed parent at
/// depth `d − 2`. Bound their count by disputed parents times children per
/// parent, and by the number of occupied prefixes at the scope's depth.
/// `n` is the larger set size; `pair` is the product of the two sizes.
fn stage_population(n: u128, pair: u128, d: usize) -> u128 {
    if d == 0 || n == 0 {
        return 0;
    }
    if d == 1 {
        // The opening question: exactly one root scope.
        return 1;
    }
    let listed = disputed(n, pair, d - 2).saturating_mul(children_quantile(n, d - 2));
    occupied(n, d - 1).min(listed)
}

#[cfg(test)]
mod tests;
