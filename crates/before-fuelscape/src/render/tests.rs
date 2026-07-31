use crate::ops::ROSTER;
use crate::plan::{run_op, Plan, Samplers};

use super::{render_gallery, render_op, RenderMeta};

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
        let path = render_op(&atlas, &meta, &out).expect("render must succeed");
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
