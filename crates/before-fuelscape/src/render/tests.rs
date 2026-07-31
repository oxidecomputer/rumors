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
