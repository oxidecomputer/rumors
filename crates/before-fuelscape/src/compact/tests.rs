use std::path::Path;

use crate::dump::DumpWriter;
use crate::ops::ROSTER;
use crate::render::{AtlasData, OverlayData, RenderMeta, SampleData};

use super::{compact, compact_dump, read, write, WidgetCol, RES};

/// A synthetic atlas whose strings exercise JSON metacharacters and the
/// multibyte glyphs the roster actually uses, and whose samples span two
/// columns with a rejected draw recorded.
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
    let dir = std::env::temp_dir().join(format!("before-compact-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp output dir");
    dir
}

fn meta() -> RenderMeta {
    RenderMeta {
        commit: "pin \"commit\"".into(),
        base_seed: u64::MAX,
        samples_per_column: 3,
    }
}

/// Compaction bins each column tightly at [`RES`] octaves: `k0` is the
/// lowest occupied bin, the first and last counts are nonzero, and the
/// counts sum to the column's sample count.
#[test]
fn compaction_bins_tightly_per_column() {
    let op = compact(&synthetic_atlas(), "`O(1)`.", "1").expect("synthetic atlas compacts");
    assert_eq!(op.sizes, vec![1, 2], "one column per distinct sample size");
    assert_eq!(op.res, RES, "the document carries the binning resolution");
    let expect_k0 = |fuel: f64| (libm::log2(fuel) / RES).floor() as i64;
    assert_eq!(
        op.cols[0],
        WidgetCol {
            k0: expect_k0(40.0),
            c: vec![1]
        }
    );
    let col = &op.cols[1];
    assert_eq!(col.k0, expect_k0(90.0), "k0 is the lowest occupied bin");
    assert_eq!(
        *col.c.last().expect("nonempty"),
        1,
        "the top bin holds the 130-fuel sample"
    );
    assert_eq!(
        col.c.iter().sum::<u32>(),
        2,
        "counts sum to the column's samples"
    );
    assert!(
        col.c.first() != Some(&0) && col.c.last() != Some(&0),
        "tight at both ends"
    );
}

/// A compact dataset round-trips losslessly through write and read:
/// meta, order, complexity strings (with JSON metacharacters and
/// multibyte glyphs), histograms, and overlay points.
#[test]
fn dataset_round_trips_losslessly() {
    let dir = temp_dir("roundtrip");
    let op = compact(
        &synthetic_atlas(),
        "vs `Version`: `O(|a| + |b|)` — \"worst\" \\ case ‖·‖",
        "n log n",
    )
    .expect("synthetic atlas compacts");
    write(&dir, &meta(), std::slice::from_ref(&op)).expect("dataset writes");
    let (loaded_meta, loaded) = read(&dir).expect("dataset loads");
    assert_eq!(loaded_meta, meta(), "meta round-trips exactly");
    assert_eq!(loaded, vec![op], "the widget data round-trips exactly");
    std::fs::remove_dir_all(&dir).expect("round-trip output cleans up");
}

/// Zero-fuel samples are rejected at compaction: the guest meters every
/// call, so a zero reading is a measurement bug, and the log-scale
/// histogram could not even place it.
#[test]
fn compaction_rejects_zero_fuel() {
    let mut data = synthetic_atlas();
    data.samples[1].fuel = 0;
    let err = compact(&data, "`O(1)`.", "1").expect_err("zero fuel must be rejected");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    assert!(
        err.to_string().contains("zero fuel"),
        "names the check: {err}"
    );
}

/// Tamper with one committed document field and the loader refuses it,
/// naming the file and check: the histogram-tightness, size-order,
/// banner, meta-uniformity, and empty-claim rejections are each alive.
#[test]
fn read_rejects_each_tampered_document() {
    let dir = temp_dir("tamper");
    let op = compact(&synthetic_atlas(), "`O(1)`.", "1").expect("synthetic atlas compacts");
    write(&dir, &meta(), std::slice::from_ref(&op)).expect("dataset writes");
    let op_path = dir.join("synthetic.json");
    let pristine = std::fs::read(&op_path).expect("op file exists");

    let tamper = |edit: &dyn Fn(&mut serde_json::Value), names: &str| {
        let mut doc: serde_json::Value = serde_json::from_slice(&pristine).expect("op file parses");
        edit(&mut doc);
        std::fs::write(
            &op_path,
            serde_json::to_vec(&doc).expect("tampered doc serializes"),
        )
        .expect("tampered doc writes");
        let err = read(Path::new(&dir)).expect_err("a tampered dataset must be rejected");
        assert_eq!(
            err.kind(),
            std::io::ErrorKind::InvalidData,
            "{names}: {err}"
        );
        assert!(
            err.to_string().contains(names),
            "must name the {names} check, got: {err}"
        );
    };

    tamper(
        &|doc| doc["format"] = "fuelscape-atlas-index".into(),
        "format",
    );
    tamper(&|doc| doc["meta"]["base_seed"] = 7.into(), "meta");
    tamper(&|doc| doc["op"]["claim"] = "".into(), "non-empty");
    tamper(&|doc| doc["op"]["res"] = 0.0.into(), "resolution");
    tamper(
        &|doc| doc["op"]["sizes"] = serde_json::json!([2, 1]),
        "ascending",
    );
    tamper(
        &|doc| doc["op"]["cols"][0]["c"] = serde_json::json!([0, 1]),
        "tight",
    );
    tamper(&|doc| doc["op"]["overlay"][0]["fuel"] = 0.into(), "overlay");

    std::fs::remove_dir_all(&dir).expect("tamper output cleans up");
}

/// Gzipped documents read transparently: a dataset whose files are all
/// stored as `.json.gz` loads byte-equal to the plain form (the
/// committed dump's storage; the same reader serves both).
#[test]
fn gzipped_documents_read_transparently() {
    let dir = temp_dir("gz");
    let op = compact(&synthetic_atlas(), "`O(1)`.", "1").expect("synthetic atlas compacts");
    write(&dir, &meta(), std::slice::from_ref(&op)).expect("dataset writes");
    let (plain_meta, plain_ops) = read(&dir).expect("plain dataset loads");
    for name in ["synthetic.json", super::INDEX_FILE] {
        let path = dir.join(name);
        let bytes = std::fs::read(&path).expect("document exists");
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        std::io::Write::write_all(&mut gz, &bytes).expect("gzip writes");
        let mut gz_path = path.clone().into_os_string();
        gz_path.push(".gz");
        std::fs::write(gz_path, gz.finish().expect("gzip finishes")).expect("gz file writes");
        std::fs::remove_file(&path).expect("plain file removes");
    }
    let (gz_meta, gz_ops) = read(&dir).expect("gzipped dataset loads");
    assert_eq!(
        (gz_meta, gz_ops),
        (plain_meta, plain_ops),
        "gz and plain forms load equal"
    );
    std::fs::remove_dir_all(&dir).expect("gz output cleans up");
}

/// `compact_dump` joins the dump against the roster by operation name
/// and stamps the row's contract and claim.
///
/// A dump whose recorded size measure differs from the roster row's is
/// refused, as is an operation the roster does not name: the
/// measurements describe an input space the roster no longer declares.
#[test]
fn compact_dump_joins_the_roster_and_rejects_measure_drift() {
    let spec = ROSTER
        .iter()
        .find(|spec| spec.name == "version_tick")
        .expect("the roster names version_tick");

    // A dump whose recorded measure matches the roster row compacts,
    // and the row's strings arrive in the written document.
    let dump_dir = temp_dir("join-dump");
    let out_dir = temp_dir("join-out");
    let mut data = synthetic_atlas();
    data.op_name = "version_tick".into();
    data.size_measure = spec.size_measure.into();
    let mut writer = DumpWriter::new(&dump_dir, meta()).expect("dump opens");
    writer.append(&data).expect("dump appends");
    compact_dump(&dump_dir, &out_dir).expect("a roster-matched dump compacts");
    let (_, ops) = read(&out_dir).expect("compacted dataset loads");
    assert_eq!(
        ops[0].contract, spec.contract,
        "the roster row's contract is stamped"
    );
    assert_eq!(
        ops[0].claim, spec.claim,
        "the roster row's claim is stamped"
    );

    // The same dump with a drifted size measure is refused.
    let drift_dir = temp_dir("drift-dump");
    data.size_measure = "some other input space".into();
    let mut writer = DumpWriter::new(&drift_dir, meta()).expect("dump opens");
    writer.append(&data).expect("dump appends");
    let err = compact_dump(&drift_dir, &out_dir).expect_err("measure drift must be refused");
    assert!(
        err.to_string().contains("size measure"),
        "names the check: {err}"
    );

    // An operation the roster does not name is refused.
    let alien_dir = temp_dir("alien-dump");
    let alien = synthetic_atlas();
    let mut writer = DumpWriter::new(&alien_dir, meta()).expect("dump opens");
    writer.append(&alien).expect("dump appends");
    let err = compact_dump(&alien_dir, &out_dir).expect_err("an alien op must be refused");
    assert!(err.to_string().contains("roster"), "names the check: {err}");

    for dir in [dump_dir, out_dir, drift_dir, alien_dir] {
        std::fs::remove_dir_all(&dir).expect("join output cleans up");
    }
}
