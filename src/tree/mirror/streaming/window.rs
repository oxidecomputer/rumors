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
//! is one byte budget; each session turns it into **static per-height
//! capacities** using what the two replicas exchange in their greetings —
//! exact set sizes and version-size bounds — priced through the storage
//! backend's own cost function
//! ([`Backend::node_bytes`](super::Backend::node_bytes)). Channels stay
//! plain bounded queues; the [link](crate::link) remains the only
//! backpressure boundary with runtime semantics.
//!
//! # The occupancy model
//!
//! Content addresses are uniform 32-byte strings (the model of record:
//! uniform-hash, authenticated-honest-peer), so the trie's occupancy thins
//! geometrically with depth, and the population of scopes a level can
//! *ever* hold in flight is bounded by closed-form statistics of the two
//! corpora, sizes `A` and `B`:
//!
//! - **Occupied slots.** At most `min(256ʲ, max(A, B))` depth-`j` slots
//!   are occupied at all — deterministically, one slot per leaf per
//!   level.
//! - **Joint occupancy.** A scope is disputed only where **both** replicas
//!   occupy the slot — a *shared prefix*, deterministically capped by the
//!   smaller corpus — and two honest corpora are independent draws, so
//!   the expected jointly occupied depth-`j` slots are `≤ A·B/256ʲ`: the
//!   birthday scale that shuts dispute populations off past the joint
//!   frontier, falling ~256× per further depth. An asymmetric session (a
//!   bootstrap catch-up) disputes almost nothing and derives floor-width
//!   dispute capacities; its supplies stream outside the window.
//! - **Per-parent fan.** A disputed parent's queried children are the
//!   *replier's* — bounded by the larger corpus's occupied sub-slots,
//!   concentrating near `max(A, B)/256ʲ` per depth-`j` parent, far below
//!   the structural fan of 256 at every depth past the first few.
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
//!   No configuration, however memory-starved, may shrink them. Their
//!   residency is nevertheless in the budget: the decode fans hold
//!   backend-priced leaf nodes, charged flat as the supply-decode
//!   envelope ([`SUPPLY_DECODE_ENVELOPE_BYTES`]) since a floor-width
//!   window fills them exactly as a wide one does.
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

use super::{Local, materialized::Resolve};
use crate::link::STREAM_COUNT;
use crate::tree::typed::{self, Prefix, height::Z};

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

/// In-memory bytes of one child's slots in a level's in-flight
/// containers: a query slot, a resolution slot, and a listing entry.
///
/// Derived from `size_of` of the real slot types under the in-memory
/// backend, so a layout change moves the price with it instead of
/// leaving a hand-counted byte total stale: the pointer-aligned
/// `(u8, node-handle)` query slot (16 B); the `(u8, Resolve)` resolution
/// slot (24 B — `Option<Node>` consumes the handle's only null niche, so
/// the `Ready`/`Pending` tag sits out of line and the pair outgrows the
/// query slot by a word); and the byte-packed `(u8, Hash)` listing entry
/// (17 B). `Resolve`'s layout does not depend on the height or payload
/// parameters, so the leaf instantiation prices every level. Exact for
/// pointer-class node handles; a backend whose `Node` demands a wider
/// layout pads the real slots beyond this constant and owes that padding
/// to its own `node_bytes` price.
const REFERENCE_SLOT_BYTES: usize = std::mem::size_of::<(u8, typed::Node<(), Z>)>()
    + std::mem::size_of::<(u8, Resolve<Local, (), Z>)>()
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
const FAN_SLOT_BYTES: usize = std::mem::size_of::<(Prefix<Z>, typed::Node<(), Z>)>()
    - std::mem::size_of::<typed::Node<(), Z>>();

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
pub(crate) const SUPPLY_DECODE_ENVELOPE_BYTES: usize =
    STREAM_COUNT * (FAN + 1) * (std::mem::size_of::<typed::Node<(), Z>>() + FAN_SLOT_BYTES);

/// The specification link's bandwidth-delay product, in bytes: 12.5 MB,
/// where the two spec links coincide.
///
/// 100 Gbps × 1 ms round trip and 1 Gbps × 100 ms are the same product;
/// the trade-off table and the crossover figures in the budget docs are
/// stated at it.
pub(crate) const SPEC_BDP_BYTES: usize = 12_500_000;

/// End-to-end wire bytes of one disputed message beyond its record's
/// encoded payload: its question share, reply share, and record framing.
///
/// Calibrated: `tests/dispute_wire.rs` counts every byte of
/// deterministic in-memory sessions and pins the per-message cost as an
/// affine law — this intercept plus the record's borsh-encoded
/// payload — at three payload sizes. The closed form documented at
/// [`DEFAULT_SYNC_MEMORY_BUDGET`] is denominated in it.
pub(crate) const DISPUTE_OVERHEAD_BYTES: usize = 28;

/// The design record's borsh-encoded payload size: the `m = 172` column
/// of the trade-off table, and the record size the wire-cost anchor
/// below is stated at.
pub(crate) const DESIGN_RECORD_BYTES: usize = 172;

/// Wire bytes one disputed message costs end to end at the design
/// record size — its question share, reply share, and leaf record.
///
/// An anchor, not an input: nothing derives from it. The closed form
/// takes the record size `m` directly ([`DISPUTE_OVERHEAD_BYTES`]
/// `+ m` per message), and the default budget is a stated policy
/// choice. Calibrated: `tests/dispute_wire.rs`'s design-record cell
/// measures exactly this figure.
pub(crate) const DISPUTE_WIRE_BYTES: usize = DISPUTE_OVERHEAD_BYTES + DESIGN_RECORD_BYTES;

/// Session-envelope bytes one in-flight disputed scope is charged.
///
/// Derived, not fitted: the per-scope charge of the *design session* —
/// two corpora of 62,500 messages each (the spec BDP in design-size
/// records), in full divergence, every stage population held in
/// flight — under the in-memory backend's pricing, exactly as
/// [`from_budget`](Window::from_budget) charges it. The recomputation is
/// pinned by `scope_envelope_matches_the_derivation`, so this constant
/// fails loudly instead of drifting when the pricing or the occupancy
/// envelopes change.
pub(crate) const SCOPE_ENVELOPE_BYTES: usize = 4_865;

/// Worst-case memory one synchronization may spend by default: 512 MiB.
///
/// Chosen, not derived: a round policy default. What any budget buys
/// is the window the session derivation (`Window::from_budget`) solves
/// for it; the committed trade-off table renders that solve at the
/// spec BDP, and the closed form
///
/// > `slowdown(budget, m) ≈ max(1, BDP × 4865 / (budget × (28 + m)))`
///
/// approximates it for mental arithmetic — 4865 B is the pinned
/// per-scope envelope, 28 B the calibrated per-message wire intercept,
/// `m` the session's mean encoded record size, `BDP` the link's
/// bandwidth-delay product in bytes. Its accuracy band and the
/// decomposition behind it are worked at
/// [`Peer::sync_memory_budget`](crate::Peer::sync_memory_budget).
///
/// The headline consequence, derived from the solve itself and pinned
/// by `default_crossover_matches_the_solve`: at the spec BDP (12.5 MB,
/// where 1 Gbps × 100 ms and 100 Gbps × 1 ms coincide), the default
/// imposes **no window-induced serialization for any corpus whose mean
/// encoded record size is at least 51 B**, each record size evaluated
/// at its own BDP-scale corpus (the closed form's estimate, `m* =
/// BDP × 4865 / budget − 28 ≈ 85.3 B`, is the safe-side reading).
/// Above the crossover the in-flight disputes' own transfer time
/// covers the round trip; slowdown 1 is wire-time-optimal:
/// bandwidth-bound stays bandwidth-bound. The factor prices the
/// interleaved dispute walk only — supply runs stream outside the
/// window — and below the crossover degradation is smooth latency
/// (minimal 8-byte records at a BDP-scale corpus: ~4.2×, the factor
/// growing slowly with set size as the derived window narrows), never
/// memory: a session serializes into capacity-sized waves,
/// deadlock-free at any budget.
///
/// The budget is an envelope, not an allocation, and **per session**: a
/// session approaches it only against wide mutual divergence, typical
/// sessions hold kilobytes, and concurrent sessions on separate links
/// each carry their own. The decode fans' flat residency (~0.21 MB
/// under the in-memory backend's pricing) comes off the budget before
/// the solve because the fan channels exist at their correctness-floor
/// capacity regardless of window width — negligible at this scale, and
/// the smallest term of the corpus-fixed component `F` that sets the
/// closed form's accuracy band. See
/// [`Peer::sync_memory_budget`](crate::Peer::sync_memory_budget) for
/// the operator questions worked through, the band's decomposition,
/// and the tabulated trade-off.
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

impl Window {
    /// The liveness floor: one scope per edge, the deadlock-proof minimum.
    pub(crate) const FLOOR: Self = Self {
        capacities: [1; KEY_DEPTH + 1],
    };

    /// Derive per-height capacities from the two replicas' set sizes and
    /// version-size bounds, a worst-case memory budget, and the backend's
    /// node pricing function.
    ///
    /// Each height's capacity is `min(K, S(depth))`, floored at one slot:
    /// `S(d)` is the depth's integer population envelope ([module
    /// docs](self)) — deep, sparse stages get small static bounds no
    /// budget can widen, because their populations cannot exist — and `K`
    /// is the widest global width whose worst case fits the budget,
    /// charging each level's population once at its own occupancy-thinned
    /// fan, after the decode fans' flat residency
    /// ([`SUPPLY_DECODE_ENVELOPE_BYTES`]'s shape, priced through this
    /// session's own `node_bytes`) comes off the top. Disputes require joint occupancy, so the joint terms take the
    /// *pair product* of the two sizes — an asymmetric session (bootstrap
    /// catch-up) disputes almost nothing and gets narrow dispute windows,
    /// while its supplies stream outside the window — where the
    /// occupied-slot and per-parent fan terms bound the replier's listed
    /// children and take the larger side. Any budget, including zero,
    /// keeps every capacity at least one: liveness outranks the budget.
    ///
    /// A held reference at depth `d` is priced by
    /// `node_bytes(c_q(d), version_bound)`: its own children quantile,
    /// and the exchanged version-size bounds combined and doubled. Each
    /// exchanged bound covers every ceiling, floor, and leaf version its
    /// replica materializes (the greeting reads the per-node aggregate),
    /// and a bound a session assembles across the two joins a ceiling —
    /// or meets a floor — drawn from each side, encoding within the
    /// pair's sum (`before`'s pinned join- and meet-subadditivity
    /// lemmas); a node holds two bounds, hence the double. One priced
    /// residual: deletion-honoring can prune a side's contribution to a
    /// survivor subset whose recomputed bound is not one the input tree
    /// materialized, so the pair sum there is a priced envelope, pinned
    /// against reality by the census suite's reconciled-bound
    /// measurements. `node_bytes` must be an upper bound and monotone in
    /// both arguments ([`Backend::node_bytes`](super::Backend::node_bytes)),
    /// so evaluating it at quantiles keeps the whole charge an upper
    /// bound; monotonicity is spot-checked here in debug builds.
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

        // The decode fans' residency, charged flat: one fan channel per
        // reply stream at its correctness-floor capacity plus the record
        // in the reader's hand, each slot one backend-priced leaf node
        // (custody of the payload passed to the backend at construction,
        // so `node_bytes(0, ·)` is its whole resident price). Width
        // cannot shrink this term — the fan capacity is load-bearing for
        // liveness — so it comes off the budget before the solve.
        let supply_fans = (STREAM_COUNT as u128)
            * (FAN as u128 + 1)
            * (node_bytes(0, version_bound) as u128 + FAN_SLOT_BYTES as u128);

        // The worst case a width-k window admits: each level's population
        // charged once (the level's queues hold overlapping views of the
        // same in-flight scopes, and their node references are shared
        // handles, so per-queue multiplication would double-charge), plus
        // the leaf-request edge, plus the flat decode-fan term.
        //
        // The leaf-request edge is charged at the width the capacity
        // assignment below grants it — `population[KEY_DEPTH]`, the same
        // bound on both sides of the same function — because a bounded
        // channel's residency never exceeds its own capacity, so
        // capacity times item size covers that edge's worst case. The
        // one divergence between the halves is the liveness floor: the
        // assignment clamps the granted capacity to one slot even where
        // the population (and so the charge) is zero, a 40-byte
        // never-charged slot per session, priced like every other
        // stage's uncharged floor slot. No wider charge buys anything:
        // the granted statistic is `jointly_occupied(n, pair, 30)`
        // times the per-parent fan, whose quantile is zero for every
        // representable corpus (`small_mean_quantile` at j = 30 has 241
        // denominator bits against at most 128 for a product of two u64
        // corpora; nonzero needs pair ≥ 2¹⁹⁰). The entries that could
        // ever occupy the edge are bounded the same way one stage
        // deeper: a leaf request is a listing entry of a disputed
        // depth-31 leaf parent (`answer::leaf_parent`'s ask arm), and
        // the j = 31 quantile floors identically (249 denominator bits;
        // nonzero needs pair ≥ 2¹⁹⁸). Capacity beyond a population is
        // physically idle, so the edge floors at one slot and the
        // budget is spent where populations exist.
        //
        // Saturating arithmetic keeps the solve total: a population near
        // 2⁶⁴ times a near-`usize::MAX` scope price passes u128, and a
        // saturated charge only overstates, failing `charge(mid) <= budget`
        // and narrowing the window — the safe direction.
        let charge = |k: u128| -> u128 {
            let mut total = supply_fans;
            for depth in 1..=KEY_DEPTH {
                total = total
                    .saturating_add(population[depth].min(k).saturating_mul(scope_price[depth]));
            }
            total.saturating_add(population[KEY_DEPTH].min(k) * LEAF_REQUEST_BYTES as u128)
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

/// How a session chooses its window: fixed capacities, or a byte budget
/// resolved against the set sizes the greeting exchanges.
// A fixed table is ~260 B against `Budget`'s word; the config lives one
// per peer and per in-flight session, so indirection would spend an
// allocation to save nothing that matters.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Copy, Debug)]
pub(crate) enum WindowConfig {
    /// Predetermined capacities: the test floor, or a harness pinning
    /// exact widths.
    Fixed(Window),
    /// Derive per-height capacities at session start, once both replicas'
    /// sizes are known.
    Budget(usize),
}

impl WindowConfig {
    /// The one-slot serialization floor, pinned: every session edge at
    /// the capacity where a bad ordering would deadlock.
    ///
    /// Tests opt in explicitly so the capacity-one orderings the
    /// deadlock-freedom argument certifies stay exercised; the
    /// [`Default`] is the budget and never depends on how the crate is
    /// built (features are additive and must not change behavior).
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
        // Unconditional: cargo features are additive and unify across a
        // build graph, so no feature may change what `Default` means —
        // a harness crate enabling this crate's test feature must not
        // put production sessions at the serialization floor. Tests pin
        // [`FLOOR`](Self::FLOOR) explicitly instead.
        Self::Budget(DEFAULT_SYNC_MEMORY_BUDGET)
    }
}

// ─── The integer occupancy envelopes ─────────────────────────────────────
//
// Uniform 32-byte content addresses put Binomial(N, 256⁻ʲ) leaves under
// each depth-j prefix, with iid-uniform continuations — exact, no
// Poissonization. Chernoff–Hoeffding tails apply verbatim to every
// count below even though slot occupancies are dependent: occupancy
// indicators are multinomial-occupancy indicators, which are negatively
// associated (Dubhashi–Ranjan 1998), and joint-occupancy indicators —
// each the product of one indicator from each of two independent
// corpora, an increasing function of disjoint coordinate blocks of the
// concatenated family — are negatively associated too.
//
// The functions below bound the resulting occupancy statistics from
// above with pure integer arithmetic. Each integer quantile is
// constructed to dominate its exact-Chernoff counterpart, and the
// dominance is verified by `examples/envelope_sim.rs` over a dense
// sampled sweep of (N, depth) — sampled, not exhaustive — so on that
// certificate the integer envelopes inherit the exact tails' joint
// ≥ 1 − 2⁻⁴⁰ per-session bound (UNION_TAIL_BITS counts the union). All
// arithmetic is u128; `256^j` never materializes beyond need (its bit
// length is exactly 8j + 1).

/// `256^j`, saturating at `u128::MAX` (only ever compared against values
/// bounded by `u64` inputs, so saturation is always on the safe side).
fn pow256(j: usize) -> u128 {
    if j >= 16 { u128::MAX } else { 1u128 << (8 * j) }
}

/// The per-statistic tail level, in bits: every (stage, statistic)
/// quantile is taken at probability 2⁻⁴⁸.
///
/// The union accounting that turns it into the per-session bound: a
/// session consumes at most three quantiles per depth — the
/// joint-occupancy quantile and the two per-parent routes — across 32
/// depths, under 2⁸ statistics in all, so the union costs at most
/// 2⁸ × 2⁻⁴⁸ = 2⁻⁴⁰ per session. A per-parent quantile must hold for
/// every candidate parent at its depth simultaneously; the 8·j-bit
/// sharpening in [`tail_exponent`] pays that union, taken over all
/// 256ʲ candidate prefixes *without* conditioning on the parent being
/// queried (`P(queried ∧ X ≥ a) ≤ P(X ≥ a)`), which sidesteps the
/// conditioning inflation entirely.
const UNION_TAIL_BITS: usize = 48;

/// The Bernstein exponent delivering the union tail: `e⁻ᵗ ≤ 2⁻⁴⁸` needs
/// `t ≥ UNION_TAIL_BITS × ln 2 ≈ 33.3`, rounded up.
///
/// Derived from [`UNION_TAIL_BITS`]; a changed tail level must move
/// this with it.
const BERNSTEIN_TAIL: u128 = 34;

/// Depth cap on the per-depth tail sharpening: past it, `2^−(48+8×40)`
/// is already beyond any population a `u64` corpus can raise, so
/// sharpening further buys nothing and risks exponent overflow.
const TAIL_DEPTH_CAP: usize = 40;

/// Integer upper bound on `ln 2 ×` the union tail bits at parent depth
/// `j`: `⌈0.7 × (UNION_TAIL_BITS + 8 min(j, TAIL_DEPTH_CAP))⌉`.
///
/// The `8j` term pays the union over the `256ʲ` candidate parents at
/// depth `j`: a per-parent quantile at tail `2^−(48+8j)` holds for all
/// of them simultaneously at the flat 2⁻⁴⁸ statistic level
/// ([`UNION_TAIL_BITS`]). The coefficient is sound because
/// `0.7 > ln 2 ≈ 0.693`, so `e⁻ᵗ ≤ 2^−bits` whenever
/// `t ≥ ⌈0.7 × bits⌉`.
fn tail_exponent(j: usize) -> u128 {
    (7 * (UNION_TAIL_BITS + 8 * j.min(TAIL_DEPTH_CAP)) as u128).div_ceil(10)
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

/// Jointly occupied depth-`j` slots.
///
/// A slot is jointly occupied only if the smaller corpus occupies it, so
/// the deterministic caps are the slot count and the corpora. Two honest
/// corpora are independent draws, so a slot's joint-occupancy
/// probability is the product of its two occupancy marginals and the
/// expected jointly occupied slots sit at most at the pair mean
/// `A·B/256ʲ` — the birthday scale. The quantile is taken there:
/// Bernstein in the bulk (flat 2⁻⁴⁸ level, `t = 34 ≥ 48 ln 2`),
/// Poisson-type past the joint frontier where the mean is sub-unit.
/// `n` is the larger corpus and `pair = A·B`, so `pair / n` recovers
/// the smaller.
fn jointly_occupied(n: u128, pair: u128, j: usize) -> u128 {
    if j == 0 {
        // The root is jointly occupied only when both corpora are
        // non-empty; an empty side disputes nothing and only receives.
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
        && let Some(q) =
            small_mean_quantile(n, j, (UNION_TAIL_BITS + 8 * j.min(TAIL_DEPTH_CAP)) as u128)
    {
        return q;
    }
    let mean_hi = if j > 0 { n / pow256(j) } else { n } + 1;
    bernstein(mean_hi, tail_exponent(j))
}

/// Per-parent quantile, slots route: mean occupied child slots of a
/// depth-`j` parent are `256 × (1 − (1 − p)^N)` at `p = 256^−(j+1)`.
///
/// The upper envelope needs the exponent correction: `(1 − p)^N ≥
/// e^(−x′)` at `x′ = Np/(1 − p)`, so the mean is at most `256 × (1 −
/// e^(−x′)) ≤ 256 × 2x′/(2 + x′)`. The code evaluates that form at
/// `x = Np` instead of `x′` — an understatement of at most half a slot
/// for `p ≤ 1/256`, absorbed (with the floor division's sub-unit loss)
/// by the `+ 1` on `mean_hi`; the Bernstein slack at `t ≥ 34` rides on
/// top.
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
/// depth-(d−2) parents times the per-parent children quantile. `n` is the
/// larger corpus (whose children a reply lists); `pair` is the product of
/// the two corpus sizes, the scale of joint occupancy.
fn stage_population(n: u128, pair: u128, d: usize) -> u128 {
    if d == 0 || n == 0 {
        return 0;
    }
    if d == 1 {
        // The opening question: exactly one root scope.
        return 1;
    }
    let listed = jointly_occupied(n, pair, d - 2).saturating_mul(children_quantile(n, d - 2));
    occupied(n, d - 1).min(listed)
}

#[cfg(test)]
mod tests;
