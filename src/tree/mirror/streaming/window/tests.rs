/// Relate sizing inputs to the work a session creates.
mod application;

/// Numerical checks of the production occupancy bounds.
mod envelope;

use proptest::prelude::*;

use super::{
    DEFAULT_SYNC_MEMORY_BUDGET, DISPUTE_OVERHEAD_BYTES, FAN, FAN_SLOT_BYTES, KEY_DEPTH,
    LEAF_REQUEST_BYTES, REFERENCE_RECORD_BYTES, REFERENCE_SCOPE_BYTES, REFERENCE_SESSION_MESSAGES,
    REFERENCE_SLOT_BYTES, ReplicaSize, SCOPE_FIXED_BYTES, SPEC_BDP_BYTES,
    SUPPLY_DECODE_ENVELOPE_BYTES, SUPPLY_RECORDS_PER_STREAM, Window, WindowConfig,
    children_quantile, disputed, occupied, stage_population,
};
use crate::link::STREAM_COUNT;

/// The symmetric set size the fixed-scale tests derive against: both
/// replicas at a terabyte-scale corpus.
const SYMMETRIC: u64 = 10_000_000_000;

/// The in-memory backend's pricing, for tests that recompute the charge
/// the solve stayed inside: one pointer per reference at every fan and
/// version bound.
fn local_node_bytes(_children: usize, _version_bound: usize) -> usize {
    std::mem::size_of::<*const ()>()
}

/// Build the two greeting measurements used by the window calculation.
fn replicas(messages: [u64; 2], version_bytes: [u64; 2]) -> [ReplicaSize; 2] {
    [
        ReplicaSize::new(messages[0], version_bytes[0]),
        ReplicaSize::new(messages[1], version_bytes[1]),
    ]
}

/// Recompute a window's modeled charge from its installed capacities.
///
/// Each level's population is clamped to its capacity and priced at its
/// occupancy-thinned fan through the backend's pricing function, plus
/// the leaf-request edge at the capacity the assignment grants it.
fn charge(
    window: &Window,
    sizes: [u64; 2],
    node_bytes: impl Fn(usize, usize) -> usize,
    version_bound: usize,
) -> u128 {
    let n = u128::from(sizes[0].max(sizes[1]));
    let pair = u128::from(sizes[0]) * u128::from(sizes[1]);
    let mut total = (STREAM_COUNT as u128)
        * (SUPPLY_RECORDS_PER_STREAM as u128)
        * (node_bytes(0, version_bound) as u128 + FAN_SLOT_BYTES as u128);
    for depth in 1..=KEY_DEPTH {
        let held = usize::try_from(children_quantile(n, depth))
            .expect("a child count cannot exceed the radix fan");
        let reference = (node_bytes(held, version_bound) + REFERENCE_SLOT_BYTES) as u128;
        let capacity = window.capacity(KEY_DEPTH - depth) as u128;
        let population = stage_population(n, pair, depth).min(capacity);
        total +=
            population * (children_quantile(n, depth - 1) * reference + SCOPE_FIXED_BYTES as u128);
    }
    total
        + stage_population(n, pair, KEY_DEPTH).min(window.capacity(0) as u128)
            * LEAF_REQUEST_BYTES as u128
}

/// `Default` is the budget unconditionally: cargo features are additive,
/// so no build shape may change what a default-configured session does.
#[test]
fn default_is_the_budget_unconditionally() {
    let WindowConfig::Budget(bytes) = WindowConfig::default() else {
        panic!("the default window choice must be the budget, not a fixed table");
    };
    assert_eq!(bytes, DEFAULT_SYNC_MEMORY_BUDGET);
}

/// The explicit test floor resolves to the one-slot liveness floor at any
/// exchanged sizes: pinning it never depends on the greeting.
#[test]
fn explicit_floor_pins_every_capacity_at_one() {
    assert_eq!(
        WindowConfig::FLOOR.resolve(replicas([SYMMETRIC; 2], [0; 2]), local_node_bytes),
        Window::FLOOR
    );
}

/// An asymmetric session disputes almost nothing: with one side empty,
/// joint occupancy is zero, so every capacity floors regardless of budget
/// — a bootstrap-shaped catch-up is all supply, and supply does not ride
/// the window.
#[test]
fn asymmetric_sessions_get_floor_dispute_windows() {
    assert_eq!(
        Window::from_budget(
            replicas([0, SYMMETRIC], [0; 2]),
            usize::MAX,
            local_node_bytes,
        ),
        Window::FLOOR
    );
}

/// A zero memory budget yields the one-slot liveness floor at every
/// height: capacity zero would be a channel that can never carry an item,
/// and liveness outranks the budget.
#[test]
fn zero_budget_is_the_floor() {
    assert_eq!(
        Window::from_budget(replicas([SYMMETRIC; 2], [0; 2]), 0, local_node_bytes),
        Window::FLOOR
    );
}

/// An empty or tiny expected set floors every capacity regardless of
/// budget: with no population, no width can ever be occupied.
#[test]
fn tiny_set_is_the_floor() {
    assert_eq!(
        Window::from_budget(replicas([0; 2], [0; 2]), usize::MAX, local_node_bytes),
        Window::FLOOR
    );
}

/// The budget solve is total under pathological pricing.
///
/// A backend charging `usize::MAX` per node at `u64::MAX`-message
/// corpora drives the population-times-price products past `u128`, and
/// the solve saturates instead of wrapping — overstating the charge,
/// which can only narrow the window, so the result is the floor rather
/// than a panic (or, in release, a wrapped charge granting an unpriced
/// width).
#[test]
fn pathological_pricing_saturates_to_the_floor() {
    let window = Window::from_budget(
        replicas([u64::MAX; 2], [u64::MAX; 2]),
        usize::MAX,
        |_, _| usize::MAX,
    );
    assert_eq!(window, Window::FLOOR);
}

/// The structural near-root caps hold at any set size and any budget.
///
/// The level under the root fans one scope into at most `FAN`, and the next
/// at most `FAN²`, so their capacities never exceed those populations
/// however much budget is offered.
#[test]
fn near_root_capacities_are_structural() {
    let window = Window::from_budget(
        replicas([u64::MAX; 2], [0; 2]),
        usize::MAX,
        local_node_bytes,
    );
    // Height KEY_DEPTH−2 discusses depth-2 children: at most one full fan
    // of queried entries under the single jointly-known root.
    assert!(window.capacity(KEY_DEPTH - 2) <= FAN);
    // Height KEY_DEPTH−3 discusses depth-3 children: at most FAN².
    assert!(window.capacity(KEY_DEPTH - 3) <= FAN * FAN);
}

/// The default pairing pipelines where population lives: every mid-depth
/// level whose population exceeds one gets capacity well past the
/// serialization floor.
#[test]
fn default_budget_pipelines_the_fat_stages() {
    let window = Window::from_budget(
        replicas([SYMMETRIC; 2], [0; 2]),
        DEFAULT_SYNC_MEMORY_BUDGET,
        local_node_bytes,
    );
    // Depth 4 is the first boundary whose population outgrows the
    // structural caps at the default declaration; its height must carry
    // real width.
    assert!(window.capacity(KEY_DEPTH - 4) > FAN);
}

/// Deep levels are population-capped to a sliver at realistic set sizes.
///
/// A depth-10 dispute needs jointly occupied depth-8 slots, whose
/// expected count N²/256⁸ is far below one at a million messages — the
/// increasing-sparsity-by-depth shape the derivation encodes. The caps
/// stay small but nonzero: the 2⁻⁴⁸ envelope grants a few slots rather
/// than claiming an impossibility it cannot certify.
#[test]
fn deep_levels_are_sparse() {
    let window = Window::from_budget(
        replicas([1_000_000; 2], [0; 2]),
        usize::MAX,
        local_node_bytes,
    );
    for height in 0..=(KEY_DEPTH - 10) {
        assert!(
            window.capacity(height) <= 16,
            "height {height} got capacity {}",
            window.capacity(height),
        );
    }
}

/// The reference scope cost matches its derivation, and the default admits that population.
#[test]
fn reference_scope_cost_matches_the_derivation() {
    let n = u128::from(REFERENCE_SESSION_MESSAGES);
    let mut total = 0u128;
    for depth in 1..=KEY_DEPTH {
        let held = usize::try_from(children_quantile(n, depth))
            .expect("a child count cannot exceed the radix fan");
        let reference = (local_node_bytes(held, 0) + REFERENCE_SLOT_BYTES) as u128;
        total += stage_population(n, n * n, depth).min(n)
            * (children_quantile(n, depth - 1) * reference + SCOPE_FIXED_BYTES as u128);
    }
    total += stage_population(n, n * n, KEY_DEPTH).min(n) * LEAF_REQUEST_BYTES as u128;
    assert_eq!(
        REFERENCE_SCOPE_BYTES as u128,
        total.div_ceil(n),
        "REFERENCE_SCOPE_BYTES must equal the reference session's per-scope charge",
    );

    let window = Window::from_budget(
        replicas([REFERENCE_SESSION_MESSAGES; 2], [0; 2]),
        DEFAULT_SYNC_MEMORY_BUDGET,
        local_node_bytes,
    );
    assert!(
        (0..=KEY_DEPTH).any(|height| window.capacity(height) as u64 >= REFERENCE_SESSION_MESSAGES),
        "the policy default must admit the reference session's whole population in flight",
    );
}

/// The supply-decode envelope is the solve's own flat decode-fan term.
///
/// `SUPPLY_DECODE_ENVELOPE_BYTES` must equal the flat term
/// `from_budget` charges under the in-memory backend's pricing — one
/// fan channel plus one in-hand record per reply stream, each occupant
/// a pointer-priced leaf in its slot — so the pre-charge the operator
/// docs quote cannot drift from the solve.
#[test]
fn supply_decode_envelope_matches_the_charge() {
    let flat = (STREAM_COUNT as u128)
        * (SUPPLY_RECORDS_PER_STREAM as u128)
        * (local_node_bytes(0, 0) as u128 + FAN_SLOT_BYTES as u128);
    assert_eq!(
        SUPPLY_DECODE_ENVELOPE_BYTES as u128, flat,
        "SUPPLY_DECODE_ENVELOPE_BYTES must equal the solve's flat decode-fan term",
    );
}

/// The committed trade-off table is byte-identical to the derivation's
/// rendering.
///
/// Generation is deterministic — each row's window from the solve at
/// the example's set size, each cell from the wave form — so any drift
/// between the derivation and the table the rustdoc includes fails
/// here instead of shipping stale numbers; regenerate with
/// `just window-tradeoff`.
#[test]
fn tradeoff_table_matches_the_derivation() {
    assert_eq!(
        include_str!("tradeoff.md"),
        crate::testing::window_tradeoff_table(),
        "src/tree/mirror/streaming/window/tradeoff.md is stale: run `just window-tradeoff`",
    );
}

/// Preserve the default budget's capacity at two bandwidth-delay-product boundaries.
///
/// Each payload size determines a corpus that fills one BDP on the wire.
/// Find the smallest payload whose corpus fits entirely in the default
/// window, then check the capacity for a corpus of random u64 messages.
#[test]
fn default_crossover_matches_the_solve() {
    let window_at = |corpus: u64| {
        let window = Window::from_budget(
            replicas([corpus; 2], [0; 2]),
            DEFAULT_SYNC_MEMORY_BUDGET,
            local_node_bytes,
        );
        window.widest()
    };
    let crossover = (1..=REFERENCE_RECORD_BYTES).find(|&m| {
        let corpus = (SPEC_BDP_BYTES / (DISPUTE_OVERHEAD_BYTES + m)) as u64;
        window_at(corpus) >= corpus
    });
    assert_eq!(
        crossover,
        Some(52),
        "the default window no longer reaches the expected crossover",
    );
    // A random u64 payload CBOR-encodes to 9 bytes (header + value).
    let u64_corpus = (SPEC_BDP_BYTES / (DISPUTE_OVERHEAD_BYTES + 9)) as u64;
    assert_eq!(
        window_at(u64_corpus),
        91_941,
        "the default window changed for the u64 BDP-scale corpus",
    );
}

/// A materializing backend's node price, shaped like a database
/// placeholder: a fixed header, a per-child entry, and the resident
/// version bounds.
fn materializing_node_bytes(children: usize, version_bound: usize) -> usize {
    64 + 24 * children + version_bound
}

/// Pricier nodes buy narrower windows from the same budget.
///
/// A backend that materializes child tables and version bounds derives
/// capacities pointwise at or below the pointer-priced window at every
/// height, and its recomputed modeled charge still fits the budget:
/// the derivation spends the budget through the supplied function, not
/// through any built-in rate.
#[test]
fn function_pricing_narrows_the_window() {
    let (len, version_bytes, budget) = (1_u64 << 24, 512_u64, 64 << 20);
    let measurements = replicas([len; 2], [version_bytes; 2]);
    let cheap = Window::from_budget(measurements, budget, |_, _| {
        std::mem::size_of::<*const ()>()
    });
    let pricey = Window::from_budget(measurements, budget, materializing_node_bytes);
    for height in 0..=KEY_DEPTH {
        assert!(
            pricey.capacity(height) <= cheap.capacity(height),
            "height {height}: {} > {}",
            pricey.capacity(height),
            cheap.capacity(height),
        );
    }
    // The solve evaluated the function at the doubled joined-pair bound.
    let bound = 2 * usize::try_from(2 * version_bytes).expect("small bound");
    assert!(charge(&pricey, [len, len], materializing_node_bytes, bound) <= budget as u128);
}

proptest! {
    /// Choose the widest affordable window, except when progress requires exceeding the budget.
    #[test]
    fn window_uses_the_available_budget(
        sizes in proptest::array::uniform2(any::<u64>()),
        budget in 0usize..=1 << 44,
    ) {
        let window = Window::from_budget(replicas(sizes, [0; 2]), budget, local_node_bytes);
        prop_assert!(
            window == Window::FLOOR
                || charge(&window, sizes, local_node_bytes, 0) <= budget as u128
        );

        // Increase the common width by one wherever population permits.
        // It must either buy no additional capacity or exceed the budget.
        // Merely staying under budget would also accept an all-ones bug.
        let mut wider = window;
        let next = u128::from(window.widest()) + 1;
        let n = u128::from(sizes[0].max(sizes[1]));
        let pair = u128::from(sizes[0]) * u128::from(sizes[1]);
        for depth in 1..=KEY_DEPTH {
            wider.capacities[KEY_DEPTH - depth] =
                stage_population(n, pair, depth).min(next).max(1) as usize;
        }
        prop_assert!(
            wider == window || charge(&wider, sizes, local_node_bytes, 0) > budget as u128
        );
    }

    /// Population envelopes are internally consistent: joint occupancy
    /// never exceeds single-corpus occupancy, per-parent fans never
    /// exceed the structural fan, and every stage population respects its
    /// occupied-slot cap.
    #[test]
    fn envelopes_are_consistent(messages in 0u64.., depth in 1usize..=KEY_DEPTH) {
        let n = u128::from(messages);
        prop_assert!(disputed(n, n * n, depth) <= occupied(n, depth));
        prop_assert!(children_quantile(n, depth) <= FAN as u128);
        prop_assert!(stage_population(n, n * n, depth) <= occupied(n, depth - 1));
    }

    /// Every sufficiently deep stage has zero modeled occupancy for all
    /// representable set lengths.
    ///
    /// At these depths the uniform-hash tail makes even one disputed scope
    /// rarer than the session-wide bound. The calculation still charges leaf
    /// requests so its general formula does not depend on this shortcut. The
    /// maximal pair runs in every case because it is the first boundary to
    /// become nonzero if the tail constants change.
    #[test]
    fn deep_stage_populations_are_zero(sizes in proptest::array::uniform2(any::<u64>())) {
        for sizes in [[u64::MAX; 2], sizes] {
            let n = u128::from(sizes[0].max(sizes[1]));
            let pair = u128::from(sizes[0]) * u128::from(sizes[1]);
            for depth in 25..=KEY_DEPTH {
                prop_assert_eq!(
                    stage_population(n, pair, depth),
                    0,
                    "depth {}, sizes {:?}",
                    depth,
                    sizes,
                );
            }
        }
    }

    /// Capacities move smoothly as the set estimate crosses a tree-height
    /// boundary (a power of 256).
    ///
    /// A small drift in the estimate moves each capacity by at most its
    /// own drift plus a bounded ripple from the integer quantiles'
    /// bit-length granularity — multiplicatively at most a quarter of the
    /// width, plus a small absolute corner where a sparse quantile is
    /// itself only tens of slots. That ripple is the price of keeping the
    /// quantiles in their dominance-certified integer form. The bound
    /// still has teeth against the failure it exists to exclude: a charge
    /// quantized in whole saturable levels would step every fat stage by
    /// a third to a half of its width — tens of thousands of slots —
    /// exactly at these boundaries.
    #[test]
    fn capacities_are_smooth_across_height_boundaries(
        level in 2u32..=4,
        offset in 1u64..=1024,
        budget in (1usize << 24)..=(1 << 40),
    ) {
        let boundary = 256u64.pow(level);
        let below = Window::from_budget(
            replicas([boundary - offset; 2], [0; 2]),
            budget,
            local_node_bytes,
        );
        let above = Window::from_budget(
            replicas([boundary + offset; 2], [0; 2]),
            budget,
            local_node_bytes,
        );
        for height in 0..=KEY_DEPTH {
            let (b, a) = (below.capacity(height), above.capacity(height));
            let step = b.abs_diff(a) as u64;
            prop_assert!(
                step <= 2 * offset + (b as u64) / 4 + 32,
                "height {height}: {b} vs {a} across {boundary}±{offset}",
            );
        }
    }
}
