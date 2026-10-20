//! PROBE (audit-rank-fuel, uncommitted scratch): allocation volume of the
//! rank family across a doubling, the reading a committed band would assert.

use before::testing::meter::registry::Shape;
use before::{Party, Version};

/// Bytes the measured call requested from the allocator, with its input bytes.
fn allocated(v: &Version, w: Option<&Version>, op: &str) -> (u64, u64) {
    let bytes = (v.encode().len() + w.map_or(0, |w| w.encode().len())) as u64;
    let (stats, ()) = alloc_meter::measure(|| match op {
        "rank" => drop(std::hint::black_box(v.rank())),
        "distance" => drop(std::hint::black_box(v.distance(w.unwrap()))),
        "lag" => drop(std::hint::black_box(v.lag(w.unwrap()))),
        _ => unreachable!(),
    });
    (stats.bytes_allocated as u64, bytes)
}

fn ticked(v: &Version) -> Version {
    let mut w = v.clone();
    w.tick(&Party::seed());
    w
}

/// Prints each case's allocation per input byte at both scales and the ratio.
#[test]
fn rank_family_allocation_volume_across_a_doubling() {
    let cases: Vec<(&str, Version, Version)> = vec![
        (
            "FZ rank 512->1024",
            Shape::FreezeParade.build1(512).version(),
            Shape::FreezeParade.build1(1024).version(),
        ),
        (
            "DS rank 2048->4096",
            Shape::DenseSuffix.build2(2048, 2048).version(),
            Shape::DenseSuffix.build2(4096, 4096).version(),
        ),
        (
            "DS rank 512->1024",
            Shape::DenseSuffix.build2(512, 512).version(),
            Shape::DenseSuffix.build2(1024, 1024).version(),
        ),
    ];
    for (name, small, large) in &cases {
        let (m1, n1) = allocated(small, None, "rank");
        let (m2, n2) = allocated(large, None, "rank");
        println!(
            "PROBE {name}: {m1}/{n1} -> {m2}/{n2}  per-byte {:.3} -> {:.3}  ratio {:.3}",
            m1 as f64 / n1 as f64,
            m2 as f64 / n2 as f64,
            (m2 as f64 / n2 as f64) / (m1 as f64 / n1 as f64)
        );
    }
    for op in ["distance", "lag"] {
        let small = Shape::FreezeParade.build1(512).version();
        let large = Shape::FreezeParade.build1(1024).version();
        let (m1, n1) = allocated(&small, Some(&ticked(&small)), op);
        let (m2, n2) = allocated(&large, Some(&ticked(&large)), op);
        println!(
            "PROBE FZ {op} 512->1024: {m1}/{n1} -> {m2}/{n2}  per-byte {:.3} -> {:.3}  ratio {:.3}",
            m1 as f64 / n1 as f64,
            m2 as f64 / n2 as f64,
            (m2 as f64 / n2 as f64) / (m1 as f64 / n1 as f64)
        );
    }
}
