use proptest::prelude::*;

use super::*;
use crate::grid::{Grid, Kind, Session};

/// Rows read back from `sessions.csv` exactly as written, optional and quoted
/// fields included, so the report reads what the run recorded.
#[test]
fn rows_round_trip_through_csv() {
    let point = Grid::parse(include_str!("../../grids/quick.toml"))
        .unwrap()
        .points()
        .unwrap()[0]
        .1;
    let session = Session {
        point,
        group: "a, \"quoted\" group".into(),
        kind: Kind::Raised,
        repetition: 2,
    };
    let mut row = Row::new(
        &session,
        Placement {
            sequence: 7,
            slot: 1,
            jobs: 4,
            load_1m: Some(0.5),
            queues: Some((40, 41)),
            prepared: Some(Interval {
                began: 1_767_225_600.125,
                ended: 1_767_225_603.5,
            }),
            timed: Some(Interval {
                began: 1_767_225_610.25,
                ended: 1_767_225_611.25,
            }),
        },
    );
    row.wall_seconds = Some(0.125);
    row.error = Some("side A: it failed, badly".into());
    let directory =
        std::env::temp_dir().join(format!("netperf-record-test-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("sessions.csv");
    let mut writer = Writer::create(&path).unwrap();
    writer.append(&row).unwrap();
    writer
        .append(&Row::new(&session, Placement::default()))
        .unwrap();
    drop(writer);
    let read_back = read(&path).unwrap();
    std::fs::remove_dir_all(&directory).unwrap();
    assert_eq!(read_back.len(), 2);
    assert_eq!(format!("{:?}", read_back[0]), format!("{row:?}"));
}

/// A fresh path for one test case's `sessions.csv`, named for `test`.
fn scratch(test: &str) -> std::path::PathBuf {
    static CASES: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let directory = std::env::temp_dir().join(format!(
        "netperf-record-{test}-{}-{}",
        std::process::id(),
        CASES.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).unwrap();
    directory.join("sessions.csv")
}

/// Rows whose errors hold the bytes that make a prefix of a row look whole:
/// separators, quotes, and line breaks.
///
/// Every error is non-empty: CSV writes an empty string and an absent one
/// alike, so an empty error reads back as none.
fn rows() -> impl Strategy<Value = Vec<Row>> {
    let point = Grid::parse(include_str!("../../grids/quick.toml"))
        .unwrap()
        .points()
        .unwrap()[0]
        .1;
    proptest::collection::vec(
        (proptest::option::of("[a-z,\"\n ]{1,12}"), any::<bool>()),
        1..5,
    )
    .prop_map(move |drawn| {
        drawn
            .into_iter()
            .enumerate()
            .map(|(sequence, (error, measured))| {
                let session = Session {
                    point,
                    group: "g".into(),
                    kind: Kind::Grid,
                    repetition: sequence as u32,
                };
                let mut row = Row::new(
                    &session,
                    Placement {
                        sequence: sequence as u64,
                        ..Placement::default()
                    },
                );
                row.wall_seconds = measured.then_some(0.1 + sequence as f64 / 3.0);
                row.error = error;
                row
            })
            .collect()
    })
}

proptest! {
    /// A `sessions.csv` cut at any byte recovers exactly the rows that lie
    /// whole before the cut, and counts the rest as torn.
    ///
    /// A writer resumed after the whole rows then appends the remaining
    /// rows so the file reads back as if it had never been cut.
    #[test]
    fn recovery_keeps_exactly_the_whole_rows(
        rows in rows(),
        cut in any::<prop::sample::Index>(),
    ) {
        let path = scratch("recover");
        let mut writer = Writer::create(&path).unwrap();
        // Where the whole part may end: after the header, and after each row.
        let mut ends = vec![header().unwrap().len() as u64];
        for row in &rows {
            writer.append(row).unwrap();
            ends.push(std::fs::metadata(&path).unwrap().len());
        }
        drop(writer);
        let cut = cut.index(*ends.last().unwrap() as usize + 1) as u64;
        std::fs::OpenOptions::new().write(true).open(&path).unwrap().set_len(cut).unwrap();

        let recovered = recover(&path).unwrap();
        let reached = ends.iter().filter(|&&end| end <= cut).count();
        let whole = reached.checked_sub(1).map_or(0, |last| ends[last]);
        let kept = reached.saturating_sub(1);
        prop_assert_eq!(recovered.whole, whole);
        prop_assert_eq!(recovered.torn, cut - whole);
        prop_assert_eq!(format!("{:?}", recovered.rows), format!("{:?}", &rows[..kept]));

        let mut writer = Writer::resume(&path, recovered.whole).unwrap();
        for row in &rows[kept..] {
            writer.append(row).unwrap();
        }
        drop(writer);
        let read_back = read(&path).unwrap();
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
        prop_assert_eq!(format!("{read_back:?}"), format!("{rows:?}"));
    }
}

/// A malformed row with whole rows after it is refused rather than dropped: a
/// run cut short tears only its last row, so this file was damaged some other
/// way, and truncating it would discard sessions.
#[test]
fn malformed_row_before_the_end_is_refused() {
    let path = scratch("malformed");
    let session = Session {
        point: Grid::parse(include_str!("../../grids/quick.toml"))
            .unwrap()
            .points()
            .unwrap()[0]
            .1,
        group: "g".into(),
        kind: Kind::Grid,
        repetition: 0,
    };
    let mut writer = Writer::create(&path).unwrap();
    for _ in 0..3 {
        writer
            .append(&Row::new(&session, Placement::default()))
            .unwrap();
    }
    drop(writer);
    let mut bytes = std::fs::read(&path).unwrap();
    let second = header().unwrap().len() + 1;
    bytes[second] = b'x';
    std::fs::write(&path, &bytes).unwrap();
    let error = format!("{:#}", recover(&path).unwrap_err());
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    assert!(error.contains("row 1 is malformed"), "{error}");
}
