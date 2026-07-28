use crate::ops::ROSTER;
use crate::plan::{run_op, Plan, Samplers};

use super::{render_gallery, render_op, RenderMeta};

/// The whole pipeline runs end to end at tiny scale.
///
/// Sample uniform inputs, measure real fuel in the fuzz-fit guest,
/// render an SVG per operation with the provenance stamp drawn in, and
/// emit the gallery. This is the gate's liveness check for the
/// instrument — seconds, not a real survey (full renders go through the
/// `just atlas` recipe) — and it needs the guest wasm, which the recipe
/// builds first (`just fuzzfit-build`).
#[test]
fn pipeline_smoke_samples_measures_and_renders() {
    let plan = Plan {
        base_seed: 0x5eed,
        samples_per_column: 3,
        max_bytes: 4,
    };
    let meta = RenderMeta {
        commit: "smoke".into(),
        base_seed: plan.base_seed,
        samples_per_column: plan.samples_per_column,
    };
    let out = std::env::temp_dir().join(format!("before-atlas-smoke-{}", std::process::id()));
    std::fs::create_dir_all(&out).expect("temp output dir");
    let samplers = Samplers::build(&plan);

    // One unary version row, one binary party row: together they exercise
    // both samplers, both operand arities, the split rule, the overlay
    // generators for two signatures, and the version rejection path.
    let mut rendered = Vec::new();
    for name in ["version_rank", "party_covers"] {
        let op = ROSTER
            .iter()
            .find(|op| op.name == name)
            .expect("smoke ops are roster rows");
        let atlas = run_op(&plan, &samplers, op);
        assert!(
            atlas.samples.iter().all(|s| s.fuel > 0),
            "a measured kernel call cannot consume zero fuel"
        );
        let path = render_op(&atlas, &meta, &out).expect("render must succeed");
        let svg = std::fs::read_to_string(&path).expect("rendered SVG exists");
        assert!(
            svg.contains("commit smoke") && svg.contains("seed 0x5eed"),
            "the provenance stamp must be drawn into the image"
        );
        rendered.push((name.to_string(), path));
    }
    let gallery = render_gallery(&rendered, &meta, &out).expect("gallery must render");
    let html = std::fs::read_to_string(&gallery).expect("gallery exists");
    assert!(
        html.contains("version_rank.svg") && html.contains("party_covers.svg"),
        "the gallery must link every rendered operation"
    );
    std::fs::remove_dir_all(&out).expect("smoke output cleans up");
}
