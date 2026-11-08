use proptest::prelude::*;

use super::*;
use crate::point::Replicas;

/// A point of the quick grid with the replica counts and mean message size
/// given.
fn point(replicas: Replicas, mean: f64) -> Point {
    let mut point = crate::grid::Grid::parse(include_str!("../../grids/quick.toml"))
        .unwrap()
        .points()
        .unwrap()[0]
        .1;
    point.replicas = replicas;
    point.sizes.mean = mean;
    point
}

/// Replica counts with some messages on each side, so every rate a fit reads is
/// determined.
fn replicas() -> impl Strategy<Value = Replicas> {
    (
        0u64..1_000_000,
        1u64..1_000_000,
        1u64..1_000_000,
        0u64..10_000,
        0u64..10_000,
    )
        .prop_map(
            |(shared, insert_a, insert_b, redact_a, redact_b)| Replicas {
                shared,
                insert_a,
                insert_b,
                redact_a,
                redact_b,
            },
        )
}

/// Replica counts of at least 100,000 messages on each side, the scale
/// calibration fits its memory line to.
fn large_replicas() -> impl Strategy<Value = Replicas> {
    (100_000u64..1_000_000, 100_000u64..1_000_000).prop_map(|(insert_a, insert_b)| Replicas {
        insert_a,
        insert_b,
        ..Replicas::default()
    })
}

proptest! {
    /// Fitting a cost model to the durations another model predicts for a
    /// point and an empty pair recovers that model.
    ///
    /// The fit and the prediction are inverse, so calibration's rates
    /// reproduce the durations they were measured from.
    #[test]
    fn cost_fit_inverts_prediction(
        replicas in replicas(),
        build in 1e-6..1e-4f64,
        fetch in 1e-6..1e-4f64,
        session in 1e-6..1e-4f64,
        overhead in 0.0..1.0f64,
    ) {
        let truth = CostModel {
            build_seconds_per_message: build,
            fetch_seconds_per_message: fetch,
            session_seconds_per_message: session,
            overhead_seconds: overhead,
        };
        let noise = point(replicas, 100.0);
        let empty = point(Replicas::default(), 100.0);
        let fitted = CostModel::fit(
            &noise,
            build * build_messages(&noise),
            fetch * replicas.live_b() as f64,
            session * (replicas.only_a() + replicas.only_b()) as f64,
            truth.prepare_seconds(&empty),
        );
        let close = |x: f64, y: f64| (x - y).abs() <= 1e-9 * (1.0 + x.abs());
        prop_assert!(close(fitted.build_seconds_per_message, build));
        prop_assert!(close(fitted.fetch_seconds_per_message, fetch));
        prop_assert!(close(fitted.session_seconds_per_message, session));
        prop_assert!(close(fitted.overhead_seconds, overhead));
    }

    /// Fitting a memory model to the bytes another model predicts for an
    /// empty pair and a large one recovers that model, so it predicts the
    /// same bytes for every other point, whatever its message size.
    #[test]
    fn memory_fit_inverts_prediction(
        large in large_replicas(),
        other in replicas(),
        fixed in 0.0..1e8f64,
        overhead in 0.0..2048.0f64,
        mean in 1.0..1000.0f64,
        other_mean in 1.0..10_000.0f64,
    ) {
        let truth = MemoryModel { fixed_bytes: fixed, overhead_bytes_per_message: overhead };
        let empty = point(Replicas::default(), mean);
        let large = point(large, mean);
        let fitted = MemoryModel::fit(
            (&empty, truth.bytes(&empty)),
            (&large, truth.bytes(&large)),
        );
        let other = point(other, other_mean);
        let (predicted, expected) = (fitted.bytes(&other) as f64, truth.bytes(&other) as f64);
        // Each measurement rounds to whole bytes, and the slope carries
        // that error, divided by the messages between the two points, to
        // every message of the other point: at most a few dozen bytes.
        prop_assert!((predicted - expected).abs() <= 1e-6 * expected + 64.0, "{predicted} against {expected}");
    }

    /// A warm-up costs nothing on a fresh connection, whose session time
    /// covers its setup, and on a warm one it takes at least the handshake
    /// round trip and grows with the bytes it carries.
    #[test]
    fn warm_up_grows_with_its_bytes(
        bytes in 1u64..(256 << 20),
        more in 1u64..(256 << 20),
        window in (64u64 << 10)..(1 << 30),
    ) {
        let costs = CostModel::default();
        let mut warm = point(Replicas::default(), 100.0);
        warm.connection = Connection::Warm;
        let fresh = Point { connection: Connection::Fresh, ..warm };
        prop_assert_eq!(costs.warm_seconds(&fresh, bytes, window), 0.0);
        let Link::Emulated { rtt, .. } = warm.link else { unreachable!("the quick grid emulates its links") };
        let (fewer, larger) = (costs.warm_seconds(&warm, bytes, window), costs.warm_seconds(&warm, bytes + more, window));
        prop_assert!(fewer >= rtt.seconds());
        prop_assert!(larger > fewer);
    }
}
