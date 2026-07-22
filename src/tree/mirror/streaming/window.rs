//! The pipeline window: per-height static bounds on in-flight disputed
//! scopes, sized by the occupancy statistics of uniform content addresses.
//!
//! Every recursive edge in the streaming session — the walk's query and
//! resolution queues, the proxy's flushed-question and next-scope queues —
//! is a bounded channel. One slot per edge is the *liveness floor*: the
//! ordering invariants in [`materialized`](super::materialized)'s module
//! docs prove a session at capacity one never deadlocks. But one slot also
//! admits only ~2 disputed scopes in flight per level, which serializes the
//! descent into one wire round trip per disputed scope. The window widens
//! those edges so sibling scopes pipeline; capacity only relaxes the wait
//! graph, so every schedule live at the floor stays live at any width.
//!
//! The public knob ([`Peer::sync_memory_budget`](crate::Peer::sync_memory_budget))
//! is denominated in the two quantities a deployment can state — the
//! message count the set is expected to reach, and a worst-case memory
//! budget for one synchronization of a set that size — and each edge's
//! capacity is a **static, per-height bound** derived from them at
//! configuration time. Channels stay plain bounded queues; the
//! [link](crate::link) remains the only backpressure boundary with runtime
//! semantics.
//!
//! # The occupancy model
//!
//! Content addresses are uniform 32-byte strings (the model of record:
//! uniform-hash, authenticated-honest-peer), so the trie's occupancy thins
//! geometrically with depth, and the population of scopes a level can
//! *ever* hold in flight is bounded by closed-form statistics of `N`
//! uniform keys:
//!
//! - **Occupied slots.** A depth-`j` slot is occupied with probability
//!   `1 − (1 − 256⁻ʲ)ᴺ`; at most `min(256ʲ, N)` slots are occupied at all
//!   — deterministically, since each of `N` leaves occupies one slot per
//!   level.
//! - **Joint occupancy.** A scope is disputed only where **both** replicas
//!   occupy the slot; two honest corpora are independent draws, so the
//!   expected jointly occupied depth-`j` slots are `≤ N²/256ʲ` — the
//!   birthday scale that shuts dispute populations off past the joint
//!   frontier, falling ~256× per further depth.
//! - **Per-parent fan.** A disputed parent's queried children are bounded
//!   by its own occupied sub-slots, concentrating near `N/256ʲ` per
//!   depth-`j` parent — far below the structural fan of 256 at every
//!   depth past the first few.
//!
//! Each bound enters as an *integer envelope*: a quantile at tail
//! probability 2⁻⁴⁸ per (stage, statistic) from the multiplicative
//! Chernoff inequality (sums of negatively associated occupancy
//! indicators), or a Poisson-type tail where the mean is sub-unit. The
//! envelopes hold simultaneously with probability ≥ 1 − 2⁻⁴⁰ per session.
//!
//! # Why probabilistic bounds may back static capacities
//!
//! If a session's real population exceeds a capacity — probability
//! < 2⁻⁴⁰ under the model, and only an off-model key distribution can do
//! systematically worse — the channel fills and **that stage serializes**:
//! backpressure, exactly as at the liveness floor. Degradation is latency,
//! never memory growth and never deadlock, because every edge keeps its
//! one-slot floor and capacity only relaxes the wait graph.
//!
//! # Two boundaries the window deliberately does not reach
//!
//! - Capacity is a bound, not an allocation: the channels are
//!   semaphore-bounded and allocate per queued item, so an idle wide window
//!   costs nothing.
//! - The assembly fan queues are **not** window edges. Their capacity of
//!   one full fan is a *correctness* floor, not a tunable: below it, a
//!   maximally disputed reply's child completions cannot all enqueue while
//!   the walk finishes the reaction loop, and the session deadlocks —
//!   demonstrated by `underbuffered_mirror_stalls` in the capacity tests.
//!   No configuration, however memory-starved, may shrink them.
//!
//! # Sizing the flushed-question edge
//!
//! The proxy's flushed-question queue
//! ([`ProxyLocalQuestions`](super::channel::QueueKind::ProxyLocalQuestions))
//! holds questions that are on the wire but unanswered: the encoder
//! publishes a question only after the reply carrying it has completely
//! flushed, and the decoder retires one per decoded wire reply — a full
//! round trip later. Its capacity is window-wide, and that is not
//! defensive headroom. The claim — derived, not measured, from the
//! premises below — is:
//!
//! > At a level whose descent ultimately asks `S` questions, the queue's
//! > supremum occupancy over schedules is exactly `min(capacity, S)`. Its
//! > own capacity is the *only* structural bound short of the frontier.
//!
//! The `≤` half is immediate: the channel is bounded, and a question
//! enters the queue at most once. Reachability is the substantive half.
//! Questions aggregate *across* parent replies — every reply flushed at
//! the level above deposits up to a full fan of them — and a bounded
//! channel upstream limits how many items sit on that edge at once, never
//! how many pass through it: slots recycle. Per-edge independence (the
//! [link contract](crate::link)) therefore admits schedules in which
//! retirement stalls while production continues — a live counterparty
//! that serves every level above promptly but lags on this one, or a
//! local walk that defers consuming this level's responses so retirement
//! parks behind the proxy's one-slot response relay. Under such a
//! schedule, replies decoded above keep refilling the next-scope edge,
//! the encoder keeps pairing recycled scopes with the walk's replies, and
//! each flushed pair deposits up to a fan more questions with none
//! retired: occupancy climbs until the queue's own capacity clamps it or
//! the frontier runs out.
//!
//! Premises, each checked against the code it names:
//!
//! - the encoder flushes one complete wire reply, then publishes that
//!   reply's entire question batch, before dequeuing its next scope
//!   (`remote/proxy/work/encode.rs`; file paths, not intra-doc links,
//!   because `proxy` is private to `remote` and unresolvable from here);
//! - the decoder dequeues question-first and retires exactly one entry
//!   per decoded reply, in wire order (`remote/proxy/work/pump.rs`);
//! - one reply asks at most one fan of questions (its disputed children);
//! - edges are independently flow-controlled, so a full edge stalls only
//!   its own producer — the premise the session's whole liveness argument
//!   already rests on.

use crate::tree::MERKLE_HASH_LEN;

/// The tree's maximum branching factor: one child per radix byte.
///
/// Also the hard capacity floor of the assembly fan queues (see the
/// [module docs](self)): those channels must admit one *full* fan
/// regardless of any window tuning.
pub(crate) const FAN: usize = 256;

/// Radix levels in the trie: one byte of a 32-byte content address per
/// level. Typed heights run from `Z = 0` (leaves) to `Root = KEY_DEPTH`;
/// the *depth* of the children discussed at height `h` is `KEY_DEPTH − h`.
const KEY_DEPTH: usize = 32;

/// In-memory bytes of one child's slots in a level's in-flight containers.
///
/// One `(u8, node-handle)` query slot and one `(u8, resolve)` resolution
/// slot (pointer-aligned pairs, 16 B each), plus one `(u8, Hash)` listing
/// entry (byte-packed).
const REFERENCE_SLOT_BYTES: usize = 16 + 16 + (1 + MERKLE_HASH_LEN);

/// Fixed in-memory bytes per buffered scope beyond its per-child slots:
/// two inline prefixes (40 B each) and the container `Vec` headers.
const SCOPE_FIXED_BYTES: usize = 2 * 40 + 2 * 24;

/// In-memory bytes of one buffered leaf request: an inline leaf prefix.
const LEAF_REQUEST_BYTES: usize = 40;

/// Messages a peer sizes its window for by default: 10^10, a terabyte of
/// minimal ~100-byte messages — the count a terabyte-scale set cannot
/// exceed, since smaller messages do not encode.
pub const DEFAULT_EXPECTED_MESSAGES: u64 = 10_000_000_000;

/// Worst-case memory one synchronization may spend by default: 10 GiB.
///
/// An envelope, not an allocation — a session approaches it only against
/// multi-gigabyte divergence; typical sessions hold kilobytes. Tune down
/// via [`Peer::sync_memory_budget`](crate::Peer::sync_memory_budget) under
/// hard memory budgets.
pub const DEFAULT_SYNC_MEMORY_BUDGET: usize = 10 << 30;

/// Per-height channel capacities for one session, in disputed scopes.
///
/// Constructed from a set-size estimate, a memory budget, and the
/// backend's per-node price by [`from_budget`](Self::from_budget);
/// consumed by the channel constructors of the materialized walk and the
/// remote proxy, each at the typed height its items carry. `Default`
/// differs by build: production sessions get the window
/// [`DEFAULT_EXPECTED_MESSAGES`] and [`DEFAULT_SYNC_MEMORY_BUDGET`]
/// derive at the in-memory backend's node price, while test builds
/// (`cfg(test)` and the `test-internals` feature) get the one-slot
/// liveness floor so every schedule keeps being exercised at the capacity
/// where a bad ordering *would* deadlock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Window {
    /// Channel capacity per typed item height, `0..=KEY_DEPTH`.
    capacities: [usize; KEY_DEPTH + 1],
}

impl Window {
    /// The liveness floor: one scope per edge, the deadlock-proof minimum.
    pub(crate) const FLOOR: Self = Self {
        capacities: [1; KEY_DEPTH + 1],
    };

    /// Derive per-height capacities from an expected set size, a
    /// worst-case memory budget, and the backend's bytes per resident
    /// node reference.
    ///
    /// Each height's capacity is `min(K, S(depth))`, floored at one slot:
    /// `S(d)` is the depth's integer population envelope ([module
    /// docs](self)) — deep, sparse stages get small static bounds no
    /// budget can widen, because their populations cannot exist — and `K`
    /// is the widest global width whose worst case fits the budget,
    /// charging each level's population once at its own occupancy-thinned
    /// fan. Any budget, including zero, keeps every capacity at least
    /// one: liveness outranks the budget.
    pub(crate) fn from_budget(
        expected_messages: u64,
        budget_bytes: usize,
        node_bytes: usize,
    ) -> Self {
        let n = u128::from(expected_messages);
        let reference_bytes = (node_bytes + REFERENCE_SLOT_BYTES) as u128;
        let budget = budget_bytes as u128;

        // Populations and per-scope fans per depth, computed once. A
        // buffered scope whose children sit at depth d is one queried
        // entry of the depth-d stage; its held references are the
        // children of one depth-(d−1) parent.
        let mut population = [0u128; KEY_DEPTH + 1];
        let mut scope_price = [0u128; KEY_DEPTH + 1];
        for depth in 1..=KEY_DEPTH {
            population[depth] = stage_population(n, depth);
            scope_price[depth] =
                children_quantile(n, depth - 1) * reference_bytes + SCOPE_FIXED_BYTES as u128;
        }

        // The worst case a width-k window admits: each level's population
        // charged once (the level's queues hold overlapping views of the
        // same in-flight scopes, and their node references are shared
        // handles, so per-queue multiplication would double-charge), plus
        // the leaf-request edge, whose items are bare prefixes bounded by
        // the corpus rather than by dispute statistics.
        let charge = |k: u128| -> u128 {
            let mut total = 0u128;
            for depth in 1..=KEY_DEPTH {
                total += population[depth].min(k) * scope_price[depth];
            }
            total + n.min(k) * LEAF_REQUEST_BYTES as u128
        };

        // Capacity beyond the widest population is physically idle, so
        // the search stops there; within it, the charge is monotone in k.
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
}

impl Default for Window {
    fn default() -> Self {
        // Tests run at the floor so the capacity-one orderings the
        // deadlock-freedom argument certifies stay exercised; production
        // sessions pipeline by default at the in-memory backend's price.
        #[cfg(any(test, feature = "test-internals"))]
        {
            Self::FLOOR
        }
        #[cfg(not(any(test, feature = "test-internals")))]
        {
            Self::from_budget(
                DEFAULT_EXPECTED_MESSAGES,
                DEFAULT_SYNC_MEMORY_BUDGET,
                super::backend::Local::NODE_BYTES,
            )
        }
    }
}

// ─── The integer occupancy envelopes ─────────────────────────────────────
//
// Uniform 32-byte content addresses put Binomial(N, 256⁻ʲ) leaves under
// each depth-j prefix, with iid-uniform continuations. The functions
// below bound the resulting occupancy statistics from above with pure
// integer arithmetic — every quantile dominates its exact Chernoff
// counterpart (sums of negatively associated indicators), so the
// envelopes hold jointly with probability ≥ 1 − 2⁻⁴⁰ per session. All
// arithmetic is u128; `256^j` never materializes beyond need (its bit
// length is exactly 8j + 1).

/// `256^j`, saturating at `u128::MAX` (only ever compared against values
/// bounded by `u64` inputs, so saturation is always on the safe side).
fn pow256(j: usize) -> u128 {
    if j >= 16 { u128::MAX } else { 1u128 << (8 * j) }
}

/// Integer upper bound on `ln 2 ×` the union tail bits at parent depth
/// `j`: `⌈0.7 × (48 + 8 min(j, 40))⌉`.
fn tail_exponent(j: usize) -> u128 {
    (7 * (48 + 8 * j.min(40)) as u128).div_ceil(10)
}

/// Integer quantile from the multiplicative Chernoff tail
/// `P(X ≥ μ + x) ≤ exp(−x²/(2μ + x))`: `x = ⌊√(2μt)⌋ + t` suffices for
/// tail `≤ e⁻ᵗ`.
fn bernstein(mean_hi: u128, t: u128) -> u128 {
    mean_hi + (2 * mean_hi * t).isqrt() + t
}

/// Integer quantile for a sub-unit mean `num/256^j`, or `None` when the
/// mean is not clearly sub-unit.
///
/// Uses the Poisson-type tail `P(X ≥ a) ≤ (eμ)^a`: with
/// `b = bitlen(256^j) − bitlen(num) ≥ 5` (so `μ ≤ 2^−(b−1)` and
/// `eμ ≤ 2^(2.45−b)`), `a = t/(b−3) + 2` gives tail `≤ 2⁻ᵗ`; and if
/// `eμ ≤ 2⁻ᵗ` even one occurrence exceeds the level, so the quantile is
/// zero.
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

/// Jointly occupied depth-`j` slots.
///
/// Slot and corpus caps plus a quantile at the pair mean `N²/256ʲ`:
/// Bernstein in the bulk (flat 2⁻⁴⁸ level, `t = 34 ≥ 48 ln 2`),
/// Poisson-type past the joint frontier where the mean is sub-unit.
fn jointly_occupied(n: u128, j: usize) -> u128 {
    if j == 0 {
        return u128::from(n >= 1);
    }
    let pair_mean = n.saturating_mul(n);
    let quantile = match small_mean_quantile(pair_mean, j, 48) {
        Some(q) => q,
        None => bernstein(pair_mean / pow256(j) + 1, 34),
    };
    pow256(j).min(n).min(quantile)
}

/// Per-parent quantile, leaves route: occupied sub-slots under one
/// depth-`j` parent are at most its leaves, `Binomial(N, 256⁻ʲ)` —
/// Bernstein at the union level in the bulk, Poisson-type below unit
/// mean.
fn leaves_quantile(n: u128, j: usize) -> u128 {
    if j > 0
        && let Some(q) = small_mean_quantile(n, j, 48 + 8 * j.min(40) as u128)
    {
        return q;
    }
    let mean_hi = if j > 0 { n / pow256(j) } else { n } + 1;
    bernstein(mean_hi, tail_exponent(j))
}

/// Per-parent quantile, slots route: mean occupied child slots of a
/// depth-`j` parent are `256 × (1 − (1 − 256^−(j+1))^N)`, and
/// `1 − e⁻ˣ ≤ 2x/(2+x)` gives the integer mean envelope; Bernstein slack
/// at the union level on top.
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

/// The depth-`d` stage population: its queried listing entries.
///
/// Entries live at depth `d − 1`, bounded by the occupied-slot cap there
/// and by the listed-under-disputed-parents aggregate — jointly occupied
/// depth-(d−2) parents times the per-parent children quantile.
fn stage_population(n: u128, d: usize) -> u128 {
    if d == 0 || n == 0 {
        return 0;
    }
    if d == 1 {
        // The opening question: exactly one root scope.
        return 1;
    }
    let listed = jointly_occupied(n, d - 2).saturating_mul(children_quantile(n, d - 2));
    occupied(n, d - 1).min(listed)
}

#[cfg(test)]
mod tests;
