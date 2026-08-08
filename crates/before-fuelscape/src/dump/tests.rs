use std::path::Path;

use crate::ops::ROSTER;
use crate::plan::{run_op, Plan, Samplers};
use crate::render::{render_gallery, render_op, AtlasData, OverlayData, RenderMeta, SampleData};

use super::{read, DumpWriter};

/// A synthetic atlas exercising the strings the format must carry
/// faithfully: JSON metacharacters and the multibyte glyphs the roster's
/// measure declarations and family labels actually use.
fn synthetic_atlas() -> AtlasData {
    AtlasData {
        op_name: "synthetic".into(),
        unary: true,
        size_measure: "bytes — split \"uniform\" over k ≈ n/2 \\ compositions".into(),
        samples: vec![
            SampleData {
                size: 1,
                arity: 1,
                fuel: 40,
                rejected: 0,
            },
            SampleData {
                size: 2,
                arity: 1,
                fuel: 90,
                rejected: 3,
            },
            SampleData {
                size: 2,
                arity: 2,
                fuel: 130,
                rejected: 0,
            },
        ],
        overlay: vec![OverlayData {
            family: "id_spine × hugeleaf⁴".into(),
            size: 2,
            fuel: 700,
        }],
    }
}

/// A per-test temporary directory, cleaned up by the caller.
fn temp_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("before-fuelscape-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp output dir");
    dir
}

/// The measurement/render split's two-ways pin: rendering straight from
/// measured atlases and rendering from their persisted dump produce
/// byte-identical SVGs and gallery.
///
/// The dump also round-trips every atlas losslessly (meta, order,
/// samples, overlay). This is what makes `--dump` + `--render-from` a
/// faithful substitute for re-measuring: a figure adjustment costs a
/// replay, never a survey.
#[test]
fn dump_and_rerender_matches_direct_render_byte_for_byte() {
    let plan = Plan {
        base_seed: 0x5eed,
        samples_per_column: 3,
        max_bytes: 8,
    };
    let meta = RenderMeta {
        commit: "pin".into(),
        base_seed: plan.base_seed,
        samples_per_column: plan.samples_per_column,
    };
    let root = temp_dir("dump-pin");
    let (direct_dir, dump_dir, replay_dir) =
        (root.join("direct"), root.join("dump"), root.join("replay"));
    for dir in [&direct_dir, &replay_dir] {
        std::fs::create_dir_all(dir).expect("temp output dir");
    }

    // Measure once, rendering directly and dumping the same data.
    let samplers = Samplers::build(&plan);
    let mut writer = DumpWriter::new(&dump_dir, meta.clone()).expect("dump opens");
    let mut measured = Vec::new();
    let mut direct = Vec::new();
    for op in ROSTER {
        let data = AtlasData::from_atlas(&run_op(&plan, &samplers, op));
        let path = render_op(&data, &meta, &direct_dir, 1.0).expect("render must succeed");
        writer.append(&data).expect("dump must append");
        direct.push((data.op_name.clone(), path));
        measured.push(data);
    }
    let direct_gallery = render_gallery(&direct, &meta, &direct_dir).expect("gallery must render");

    // Load the dump: provenance and every atlas come back exactly.
    let (loaded_meta, loaded) = read(&dump_dir).expect("dump must load");
    assert_eq!(loaded_meta, meta, "the dump must return the run's meta");
    assert_eq!(loaded, measured, "the dump must round-trip every atlas");

    // Replay rendering from the loaded dump: byte-identical output.
    let mut replay = Vec::new();
    for data in &loaded {
        let path = render_op(data, &loaded_meta, &replay_dir, 1.0).expect("render must succeed");
        replay.push((data.op_name.clone(), path));
    }
    let replay_gallery =
        render_gallery(&replay, &loaded_meta, &replay_dir).expect("gallery must render");
    for ((name, direct_svg), (_, replay_svg)) in direct.iter().zip(&replay) {
        assert_eq!(
            std::fs::read(direct_svg).expect("direct SVG exists"),
            std::fs::read(replay_svg).expect("replayed SVG exists"),
            "{name}: the replayed SVG must be byte-identical to the direct render"
        );
    }
    assert_eq!(
        std::fs::read(&direct_gallery).expect("direct gallery exists"),
        std::fs::read(&replay_gallery).expect("replayed gallery exists"),
        "the replayed gallery must be byte-identical to the direct render's"
    );
    std::fs::remove_dir_all(&root).expect("pin output cleans up");
}

/// The dump format carries every string it stores faithfully.
///
/// JSON metacharacters (quotes, backslashes) and the multibyte glyphs
/// the roster's measure declarations and family labels use round-trip
/// to equality, and the run's meta comes back exactly.
#[test]
fn synthetic_atlas_round_trips_losslessly() {
    let meta = RenderMeta {
        commit: "synthetic \"commit\"".into(),
        base_seed: u64::MAX,
        samples_per_column: 3,
    };
    let dir = temp_dir("dump-roundtrip");
    let data = synthetic_atlas();
    let mut writer = DumpWriter::new(&dir, meta.clone()).expect("dump opens");
    writer.append(&data).expect("dump must append");

    let (loaded_meta, loaded) = read(&dir).expect("dump must load");
    assert_eq!(loaded_meta, meta, "meta must round-trip exactly");
    assert_eq!(loaded, vec![data], "the atlas must round-trip exactly");
    std::fs::remove_dir_all(&dir).expect("round-trip output cleans up");
}

/// The loader's grid verification is alive: a dump whose stored grid
/// disagrees with its raw samples is rejected.
///
/// Altering one sample's fuel through the JSON leaves the stored grid
/// stale, and `read` must refuse it rather than silently render figures
/// that contradict the persisted cells downstream documents read.
#[test]
fn read_rejects_a_dump_whose_grid_disagrees_with_its_samples() {
    let meta = RenderMeta {
        commit: "tamper".into(),
        base_seed: 7,
        samples_per_column: 3,
    };
    let dir = temp_dir("dump-tamper");
    let mut writer = DumpWriter::new(&dir, meta).expect("dump opens");
    writer.append(&synthetic_atlas()).expect("dump must append");

    // Tamper with one sample's fuel through the JSON itself, leaving
    // the stored grid stale.
    let op_path = dir.join("synthetic.json");
    let mut doc: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&op_path).expect("op file exists"))
            .expect("op file parses");
    doc["op"]["samples"][0]["fuel"] = serde_json::json!(41);
    std::fs::write(
        &op_path,
        serde_json::to_vec(&doc).expect("tampered doc serializes"),
    )
    .expect("tampered doc writes");

    let err = read(Path::new(&dir)).expect_err("a tampered dump must be rejected");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    assert!(
        err.to_string().contains("stored grid"),
        "the rejection must name the grid check, got: {err}"
    );
    std::fs::remove_dir_all(&dir).expect("tamper output cleans up");
}
