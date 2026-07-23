use proptest::prelude::*;

use super::{
    DEFAULT_SYNC_MEMORY_BUDGET, DESIGN_LINK_BYTES_PER_MS, DESIGN_LINK_RTT_MS, DISPUTE_WIRE_BYTES,
    FAN, FAN_SLOT_BYTES, KEY_DEPTH, LEAF_REQUEST_BYTES, REFERENCE_SLOT_BYTES, SCOPE_ENVELOPE_BYTES,
    SCOPE_FIXED_BYTES, SUPPLY_DECODE_ENVELOPE_BYTES, Window, WindowConfig, children_quantile,
    jointly_occupied, occupied, stage_population,
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

/// The worst case a derived window admits, recomputed exactly as the
/// solve charges it.
///
/// Each level's population is clamped to its capacity and priced at its
/// occupancy-thinned fan through the backend's pricing function, plus
/// the leaf-request edge at the capacity the assignment grants it.
fn charge(
    window: &Window,
    n: u128,
    node_bytes: impl Fn(usize, usize) -> usize,
    version_bound: usize,
) -> u128 {
    let mut total = (STREAM_COUNT as u128)
        * (FAN as u128 + 1)
        * (node_bytes(0, version_bound) as u128 + FAN_SLOT_BYTES as u128);
    for depth in 1..=KEY_DEPTH {
        let held = usize::try_from(children_quantile(n, depth)).unwrap_or(usize::MAX);
        let reference = (node_bytes(held, version_bound) + REFERENCE_SLOT_BYTES) as u128;
        let capacity = window.capacity(KEY_DEPTH - depth) as u128;
        let population = stage_population(n, n * n, depth).min(capacity);
        total +=
            population * (children_quantile(n, depth - 1) * reference + SCOPE_FIXED_BYTES as u128);
    }
    total
        + stage_population(n, n * n, KEY_DEPTH).min(window.capacity(0) as u128)
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
        WindowConfig::FLOOR.resolve(SYMMETRIC, SYMMETRIC, 0, 0, local_node_bytes),
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
        Window::from_budget(0, SYMMETRIC, 0, 0, usize::MAX, local_node_bytes),
        Window::FLOOR
    );
}

/// A zero memory budget yields the one-slot liveness floor at every
/// height: capacity zero would be a channel that can never carry an item,
/// and liveness outranks the budget.
#[test]
fn zero_budget_is_the_floor() {
    assert_eq!(
        Window::from_budget(SYMMETRIC, SYMMETRIC, 0, 0, 0, local_node_bytes),
        Window::FLOOR
    );
}

/// An empty or tiny expected set floors every capacity regardless of
/// budget: with no population, no width can ever be occupied.
#[test]
fn tiny_set_is_the_floor() {
    assert_eq!(
        Window::from_budget(0, 0, 0, 0, usize::MAX, local_node_bytes),
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
        u64::MAX,
        u64::MAX,
        u64::MAX,
        u64::MAX,
        usize::MAX,
        |_, _| usize::MAX,
    );
    assert_eq!(window, Window::FLOOR);
}

/// The structural near-root caps hold at any set size and any budget.
///
/// The level under the root fans one scope into at most 256, and the next
/// at most 256², so their capacities never exceed those populations
/// however much budget is offered.
#[test]
fn near_root_capacities_are_structural() {
    let window = Window::from_budget(u64::MAX, u64::MAX, 0, 0, usize::MAX, local_node_bytes);
    // Height KEY_DEPTH−2 discusses depth-2 children: at most one full fan
    // of queried entries under the single jointly-known root.
    assert!(window.capacity(KEY_DEPTH - 2) <= FAN);
    // Height KEY_DEPTH−3 discusses depth-3 children: at most 256².
    assert!(window.capacity(KEY_DEPTH - 3) <= FAN * FAN);
}

/// The default pairing pipelines where population lives: every mid-depth
/// level whose population exceeds one gets capacity well past the
/// serialization floor.
#[test]
fn default_budget_pipelines_the_fat_stages() {
    let window = Window::from_budget(
        SYMMETRIC,
        SYMMETRIC,
        0,
        0,
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
    let window = Window::from_budget(1_000_000, 1_000_000, 0, 0, usize::MAX, local_node_bytes);
    for height in 0..=(KEY_DEPTH - 10) {
        assert!(
            window.capacity(height) <= 16,
            "height {height} got capacity {}",
            window.capacity(height),
        );
    }
}

/// The scope envelope is the derivation's own number, not a hand-fitted
/// one.
///
/// `SCOPE_ENVELOPE_BYTES` converts the design link's bandwidth-delay
/// product in scopes into the default budget. Its value must equal the
/// per-scope charge of the design session — BDP-scale corpora in full
/// divergence, every stage population held in flight, priced through the
/// in-memory backend's function — recomputed here exactly as the solve
/// charges it, so the constant fails loudly instead of drifting when the
/// pricing or the envelopes change. The end-to-end statement is asserted
/// too: the default budget admits the whole BDP in flight at the design
/// session.
#[test]
fn scope_envelope_matches_the_derivation() {
    let bdp = (DESIGN_LINK_BYTES_PER_MS * DESIGN_LINK_RTT_MS / DISPUTE_WIRE_BYTES) as u64;
    let n = u128::from(bdp);
    let mut total = 0u128;
    for depth in 1..=KEY_DEPTH {
        let held = usize::try_from(children_quantile(n, depth)).unwrap_or(usize::MAX);
        let reference = (local_node_bytes(held, 0) + REFERENCE_SLOT_BYTES) as u128;
        total += stage_population(n, n * n, depth).min(n)
            * (children_quantile(n, depth - 1) * reference + SCOPE_FIXED_BYTES as u128);
    }
    total += stage_population(n, n * n, KEY_DEPTH).min(n) * LEAF_REQUEST_BYTES as u128;
    assert_eq!(
        SCOPE_ENVELOPE_BYTES as u128,
        total.div_ceil(n),
        "SCOPE_ENVELOPE_BYTES must equal the design session's per-scope charge",
    );

    let window = Window::from_budget(bdp, bdp, 0, 0, DEFAULT_SYNC_MEMORY_BUDGET, local_node_bytes);
    assert!(
        (0..=KEY_DEPTH).any(|height| window.capacity(height) as u64 >= bdp),
        "the default budget must admit the design link's BDP in flight",
    );
}

/// The supply-decode envelope is the charge's own shape at the design
/// session, and the default budget is the scope term plus exactly it.
///
/// `SUPPLY_DECODE_ENVELOPE_BYTES` must equal the flat decode-fan term
/// `from_budget` charges under the in-memory backend's pricing — one
/// fan channel plus one in-hand record per reply stream, each occupant
/// a pointer-priced leaf in its slot — and the default budget must
/// decompose into the dispute-scope product plus this envelope, so
/// neither constant can drift from the solve it feeds.
#[test]
fn supply_decode_envelope_matches_the_charge() {
    let flat = (STREAM_COUNT as u128)
        * (FAN as u128 + 1)
        * (local_node_bytes(0, 0) as u128 + FAN_SLOT_BYTES as u128);
    assert_eq!(
        SUPPLY_DECODE_ENVELOPE_BYTES as u128, flat,
        "SUPPLY_DECODE_ENVELOPE_BYTES must equal the solve's flat decode-fan term",
    );
    assert_eq!(
        DEFAULT_SYNC_MEMORY_BUDGET,
        DESIGN_LINK_BYTES_PER_MS * DESIGN_LINK_RTT_MS / DISPUTE_WIRE_BYTES * SCOPE_ENVELOPE_BYTES
            + SUPPLY_DECODE_ENVELOPE_BYTES,
        "the default budget is the scope term plus the supply-decode envelope",
    );
}

/// The envelope-to-wire ratio the operator equations quote is exactly
/// this quotient.
///
/// `sync_memory_budget`'s docs state `slowdown ≈ max(1, 25 × BDP /
/// budget)` and `budget ≈ 25 × BDP / slowdown`; the 25 is
/// `SCOPE_ENVELOPE_BYTES / DISPUTE_WIRE_BYTES` rounded up. Pinning the
/// quotient keeps the quoted figure and the constants from drifting
/// apart.
#[test]
fn envelope_to_wire_ratio_matches_the_operator_docs() {
    assert_eq!(SCOPE_ENVELOPE_BYTES.div_ceil(DISPUTE_WIRE_BYTES), 25);
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
/// height, and its recomputed worst-case charge still fits the budget:
/// the derivation spends the budget through the supplied function, not
/// through any built-in rate.
#[test]
fn function_pricing_narrows_the_window() {
    let (len, version_bytes, budget) = (1 << 24, 512, 64 << 20);
    let cheap = Window::from_budget(len, len, version_bytes, version_bytes, budget, |_, _| {
        std::mem::size_of::<*const ()>()
    });
    let pricey = Window::from_budget(
        len,
        len,
        version_bytes,
        version_bytes,
        budget,
        materializing_node_bytes,
    );
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
    assert!(charge(&pricey, u128::from(len), materializing_node_bytes, bound) <= budget as u128);
}

proptest! {
    /// The derived window's worst-case charge stays inside the stated
    /// budget, except where the one-slot floor alone exceeds it
    /// (liveness outranks the budget).
    #[test]
    fn window_stays_inside_the_budget(
        messages in 1u64..,
        budget in 0usize..=1 << 44,
    ) {
        let window = Window::from_budget(messages, messages, 0, 0, budget, local_node_bytes);
        prop_assert!(
            window == Window::FLOOR
                || charge(&window, u128::from(messages), local_node_bytes, 0)
                    <= budget as u128
        );
    }

    /// Population envelopes are internally consistent: joint occupancy
    /// never exceeds single-corpus occupancy, per-parent fans never
    /// exceed the structural fan, and every stage population respects its
    /// occupied-slot cap.
    #[test]
    fn envelopes_are_consistent(messages in 0u64.., depth in 1usize..=KEY_DEPTH) {
        let n = u128::from(messages);
        prop_assert!(jointly_occupied(n, n * n, depth) <= occupied(n, depth));
        prop_assert!(children_quantile(n, depth) <= FAN as u128);
        prop_assert!(stage_population(n, n * n, depth) <= occupied(n, depth - 1));
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
        let below = Window::from_budget(boundary - offset, boundary - offset, 0, 0, budget, local_node_bytes);
        let above = Window::from_budget(boundary + offset, boundary + offset, 0, 0, budget, local_node_bytes);
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
