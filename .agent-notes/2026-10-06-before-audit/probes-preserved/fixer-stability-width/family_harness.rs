
/// Scratch (uncommitted): over pseudo-random histories, a stability query at
/// every width, on both sides of the 32-bit index boundary, leaves the stored
/// form `cmp_zero` leaves.
#[test]
fn zz_scratch_stability_family() {
    const SEEDS: u64 = 600;
    let widths = [
        0,
        64,
        32 * 23,
        32 * 26,
        FIRST_UNINDEXABLE_STABILITY_WIDTH - 1,
        FIRST_UNINDEXABLE_STABILITY_WIDTH,
        FIRST_UNINDEXABLE_STABILITY_WIDTH + 31,
        u64::MAX,
    ];
    let mut compactable = 0;
    for seed in 0..SEEDS {
        if run(Check::ScratchStabilityCoverage, seed, 0) == Outcome::Passed {
            compactable += 1;
        }
    }
    println!("scratch family coverage: {compactable} of {SEEDS} histories have something to compact");
    let mut failures = Vec::new();
    for seed in 0..SEEDS {
        for width in widths {
            let outcome = run(Check::ScratchStabilityFamily, seed, width);
            if outcome != Outcome::Passed {
                failures.push((seed, width, outcome));
            }
        }
    }
    let unindexable = failures.iter().filter(|(_, width, _)| *width >= FIRST_UNINDEXABLE_STABILITY_WIDTH).count();
    println!(
        "scratch family: {} failures of {} cases ({unindexable} at unindexable widths); first: {:?}",
        failures.len(),
        SEEDS * widths.len() as u64,
        failures.first()
    );
    assert!(failures.is_empty());
}
