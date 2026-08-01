use crate::ops::ROSTER;
use crate::plan::{run_op, Plan, Samplers};

use super::{render_gallery, render_op, AtlasData, OverlayData, RenderMeta, SampleData};

/// The whole pipeline runs end to end at tiny scale.
///
/// Sample uniform inputs, measure real fuel in the fuzz-fit guest,
/// render an SVG per operation with the provenance stamp drawn in, and
/// emit the gallery. This is the gate's liveness check for the
/// instrument — seconds, not a real survey (full renders go through the
/// `just fuelscape` recipe) — and it needs the guest wasm, which the
/// recipe builds first (`just fuzzfit-build`).
#[test]
fn pipeline_smoke_samples_measures_and_renders() {
    // The grid top must reach every row's minimum total size, so every
    // row has at least one column to sample.
    let plan = Plan {
        base_seed: 0x5eed,
        samples_per_column: 3,
        max_bytes: 8,
    };
    let meta = RenderMeta {
        commit: "smoke".into(),
        base_seed: plan.base_seed,
        samples_per_column: plan.samples_per_column,
    };
    let out = std::env::temp_dir().join(format!("before-fuelscape-smoke-{}", std::process::id()));
    std::fs::create_dir_all(&out).expect("temp output dir");
    let samplers = Samplers::build(&plan);

    // Every roster row at tiny sizes: every input space's draw, every
    // measured kernel name, every overlay mapping, and every render run
    // once, so a row that cannot sample, measure, or draw fails the gate
    // here rather than in a full survey.
    let mut rendered = Vec::new();
    for op in ROSTER {
        let atlas = run_op(&plan, &samplers, op);
        assert!(
            atlas.samples.iter().all(|s| s.fuel > 0),
            "{}: a measured kernel call cannot consume zero fuel",
            op.name
        );
        let data = AtlasData::from_atlas(&atlas);
        let path = render_op(&data, &meta, &out, 1.0).expect("render must succeed");
        let svg = std::fs::read_to_string(&path).expect("rendered SVG exists");
        assert!(
            svg.contains("commit smoke") && svg.contains("seed 0x5eed"),
            "{}: the provenance stamp must be drawn into the image",
            op.name
        );
        rendered.push((op.name.to_string(), path));
    }
    let gallery = render_gallery(&rendered, &meta, &out).expect("gallery must render");
    let html = std::fs::read_to_string(&gallery).expect("gallery exists");
    for op in ROSTER {
        assert!(
            html.contains(&format!("{}.svg", op.name)),
            "the gallery must link every rendered operation ({} missing)",
            op.name
        );
    }
    std::fs::remove_dir_all(&out).expect("smoke output cleans up");
}

/// The font-scale knob is alive and rendering is deterministic.
///
/// At any fixed scale two renders of the same atlas are byte-identical,
/// and a non-unit scale changes the output — a dead parameter would
/// silently ship print figures with unreadable text while every gate
/// stays green.
#[test]
fn font_scale_changes_the_svg_and_rendering_is_deterministic() {
    let meta = RenderMeta {
        commit: "scale".into(),
        base_seed: 1,
        samples_per_column: 2,
    };
    let data = AtlasData {
        op_name: "synthetic".into(),
        unary: false,
        size_measure: "synthetic measure".into(),
        samples: vec![
            SampleData {
                size: 2,
                arity: 2,
                fuel: 100,
                rejected: 0,
            },
            SampleData {
                size: 4,
                arity: 2,
                fuel: 350,
                rejected: 1,
            },
        ],
        overlay: vec![OverlayData {
            family: "synthetic family".into(),
            size: 4,
            fuel: 500,
        }],
    };
    let out = std::env::temp_dir().join(format!("before-fuelscape-scale-{}", std::process::id()));
    let (a, b, c) = (out.join("a"), out.join("b"), out.join("c"));
    for dir in [&a, &b, &c] {
        std::fs::create_dir_all(dir).expect("temp output dir");
    }

    let unit = render_op(&data, &meta, &a, 1.0).expect("render must succeed");
    let unit_again = render_op(&data, &meta, &b, 1.0).expect("render must succeed");
    assert_eq!(
        std::fs::read(&unit).expect("rendered SVG exists"),
        std::fs::read(&unit_again).expect("rendered SVG exists"),
        "two renders of the same atlas at the same scale must be byte-identical"
    );

    let scaled = render_op(&data, &meta, &c, 2.0).expect("render must succeed");
    assert_ne!(
        std::fs::read(&unit).expect("rendered SVG exists"),
        std::fs::read(&scaled).expect("rendered SVG exists"),
        "a non-unit font scale must change the rendered text geometry"
    );
    std::fs::remove_dir_all(&out).expect("scale output cleans up");
}

/// The binned grid is a pure, platform-independent function of the
/// samples: `aggregate` over a fixed synthetic atlas hashes to the same
/// committed value on every host and architecture.
///
/// A dump commits its `HeatGrid`, and the loader re-derives that grid
/// bit-for-bit on whatever machine opens the dump — so the bin geometry
/// must not lean on the platform math library, whose `log2` differs by
/// an ulp across libms exactly at bin boundaries. The sample set below
/// spreads fuel values across ~48 octaves so a platform divergence
/// anywhere in the log2 range flips at least one bin count and the
/// hash. The committed constant was produced by this test's own first
/// run; its value carries no meaning beyond cross-host agreement.
#[test]
fn aggregate_bins_identically_on_every_platform() {
    // SplitMix64: a fixed, portable stream — no dependency on rand's
    // version-to-version value stability.
    let mut state = 0x9e3779b97f4a7c15u64;
    let mut next = move || {
        state = state.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    };

    let mut samples = Vec::new();
    for col in 3..=13u32 {
        let size = 1usize << col;
        for _ in 0..500 {
            let r = next();
            let octave = next() % 48;
            let fuel = 1 + (r % (1u64 << octave.max(1)));
            samples.push(SampleData {
                size,
                arity: 2,
                fuel,
                rejected: 0,
            });
        }
    }
    let overlay = (0..24)
        .map(|i| OverlayData {
            family: format!("family_{i}"),
            size: 1usize << (3 + (i % 11)),
            fuel: 1 + (next() % (1u64 << 40)),
        })
        .collect();
    let data = AtlasData {
        op_name: "platform_pin".into(),
        unary: true,
        size_measure: "packed bytes".into(),
        samples,
        overlay,
    };

    let grid = super::aggregate(&data);
    let bytes = serde_json::to_vec(&grid).expect("grid serializes");
    // FNV-1a, inline: a stable checksum with no new dependency.
    let mut hash = 0xcbf29ce484222325u64;
    for &b in &bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    assert_eq!(
        hash, 0xa11d493893d27963,
        "aggregate produced a grid whose serialization hashes differently \
         from the committed cross-platform value"
    );
}
