use std::collections::BTreeSet;

use proptest::prelude::*;

use super::*;
use crate::calibrate::probe_points;
use crate::point::budget_text;

/// The shipped `quick` grid parses, and its one group is the product of
/// its axes: two pairs, two rates, two round trips, two budgets.
#[test]
fn quick_grid_is_its_product() {
    let grid = Grid::parse(include_str!("../../grids/quick.toml")).unwrap();
    assert_eq!(grid.repetitions, Some(3));
    assert_eq!(grid.points().unwrap().len(), 2 * 2 * 2 * 2);
}

/// Every shipped grid runs its sessions over TLS 1.3 by taking the tool's
/// default, as its header says.
///
/// `tls` appears in neither its `[defaults]` table nor any group's axes or
/// `pairs` entries, and every point the grid plans runs over TLS.
#[test]
fn shipped_grids_run_over_tls_by_default() {
    for (file, text) in [
        ("quick", include_str!("../../grids/quick.toml")),
        ("smoke", include_str!("../../grids/smoke.toml")),
        ("reference", include_str!("../../grids/reference.toml")),
    ] {
        let grid = Grid::parse(text).unwrap();
        assert!(
            grid.defaults.iter().all(|(key, _)| key != "tls"),
            "the {file} grid's defaults set `tls`"
        );
        for group in &grid.groups {
            let sets_tls = group.axes.iter().any(|(axis, _)| axis == "tls")
                || group.pairs.iter().flatten().any(|(key, _)| key == "tls");
            assert!(
                !sets_tls,
                "the {file} grid's group `{}` sets `tls`",
                group.name
            );
        }
        let (points, _) = grid.plan().unwrap();
        assert!(!points.is_empty());
        assert!(
            points.iter().all(|(_, point)| point.transport.tls),
            "the {file} grid plans a point over plain TCP"
        );
    }
}

/// The `tls` axis chooses the stack over its whole domain: a point runs
/// over TLS when the axis is absent or `on`, and over plain TCP only when
/// it is `off`.
#[test]
fn tls_axis_defaults_to_on() {
    for (axis, tls) in [
        ("", true),
        ("tls = [\"on\"]\n", true),
        ("tls = [\"off\"]\n", false),
    ] {
        let grid = Grid::parse(&format!("[[group]]\nname = \"g\"\n{axis}")).unwrap();
        let points = grid.points().unwrap();
        assert_eq!(points.len(), 1, "{axis:?}");
        assert_eq!(points[0].1.transport.tls, tls, "{axis:?}");
    }
}

/// A key that is not an axis is refused, and the error lists the axes.
#[test]
fn unknown_axis_is_refused() {
    let error = Grid::parse("[[group]]\nlatency = [\"1ms\"]\n").unwrap_err();
    assert!(format!("{error:#}").contains("rtt"), "{error:#}");
}

proptest! {
    /// A group's points are every combination of its axes and pairs: the
    /// count is the product of their lengths, and each axis value appears
    /// in exactly its share of the points.
    #[test]
    fn group_points_are_the_product(
        rtts in proptest::collection::vec(1u64..500, 1..4).prop_map(BTreeSet::from_iter),
        budgets in proptest::collection::vec(0u64..1 << 30, 1..4).prop_map(BTreeSet::from_iter),
        pairs in 0usize..4,
    ) {
        let rtt_values: Vec<String> = rtts.iter().map(|ms| format!("{ms}ms")).collect();
        let budget_values: Vec<String> = budgets.iter().map(u64::to_string).collect();
        let group = Group {
            name: "g".into(),
            axes: vec![("rtt".into(), rtt_values), ("budget".into(), budget_values)],
            pairs: (0..pairs as u64)
                .map(|shared| vec![("shared".to_string(), shared.to_string())])
                .collect(),
            max_wire_seconds: None,
        };
        let grid = Grid { groups: vec![group], ..Grid::default() };
        let points = grid.points().unwrap();
        prop_assert_eq!(points.len(), rtts.len() * budgets.len() * pairs.max(1));
        for ms in &rtts {
            let rtt: Rtt = format!("{ms}ms").parse().unwrap();
            let with = points
                .iter()
                .filter(|(_, p)| matches!(p.link, Link::Emulated { rtt: r, .. } if r == rtt))
                .count();
            prop_assert_eq!(with, budgets.len() * pairs.max(1));
        }
    }

    /// Every requested budget, the default and unlimited included, prints
    /// as text the budget axis parses back to the same request, so a
    /// point's key names exactly the budget it ran.
    #[test]
    fn budget_text_parses_back(
        requested in prop_oneof![
            Just(None),
            Just(Some(UNLIMITED_BUDGET)),
            (0u64..1 << 20).prop_map(|kib| Some(kib << 10)),
            any::<u64>().prop_map(Some),
        ],
    ) {
        let mut draft = Draft::default();
        draft.set("budget", &budget_text(requested)).unwrap();
        prop_assert_eq!(draft.0.settings.a.budget, requested);
    }

    /// The shards of a run partition its sessions: each session belongs to
    /// exactly one of the `n` shards.
    #[test]
    fn shards_partition_sessions(count in 1u32..8, shared in proptest::collection::vec(0u64..1000, 1..20)) {
        let points: Vec<(String, Point)> = shared
            .iter()
            .map(|&s| {
                let mut draft = Draft::default();
                draft.set("shared", &s.to_string()).unwrap();
                ("g".to_string(), draft.0)
            })
            .collect();
        for session in sessions(&points, 3, None) {
            let owners = (1..=count).filter(|&index| Shard { index, count }.owns(&session)).count();
            prop_assert_eq!(owners, 1);
        }
    }

    /// A point is rerun with raised buffers exactly when the usable window
    /// is smaller than what the session could keep in flight, and the
    /// raised limit's window covers that amount.
    #[test]
    fn raised_buffer_exactly_when_window_binds(
        mbit in 1u64..10_000,
        ms in 1u64..300,
        window in 1u64..1 << 28,
        insert in 0u64..1_000_000,
    ) {
        let mut draft = Draft::default();
        draft.set("rate", &format!("{mbit}Mbit")).unwrap();
        draft.set("rtt", &format!("{ms}ms")).unwrap();
        draft.set("insert_a", &insert.to_string()).unwrap();
        let point = draft.0;
        let in_flight = (mbit as f64 * 1e6 / 8.0 * ms as f64 / 1e3).min(point.model().busier_bytes());
        match raised_buffer(&point, window, 0.5) {
            None => prop_assert!(in_flight <= window as f64),
            Some(bytes) => {
                prop_assert!(in_flight > window as f64);
                prop_assert!(bytes as f64 * 0.5 >= in_flight || bytes == MAX_RAISED_BUFFER);
            }
        }
    }
}

/// The `unlimited` spelling, in any case, requests [`UNLIMITED_BUDGET`] on
/// the sides its axis names, and leaves the other side's budget alone.
#[test]
fn unlimited_budget_spelling() {
    for spelling in ["unlimited", "Unlimited", " UNLIMITED "] {
        let mut draft = Draft::default();
        draft.set("budget", spelling).unwrap();
        assert_eq!(draft.0.settings.a.budget, Some(UNLIMITED_BUDGET));
        assert_eq!(draft.0.settings.b.budget, Some(UNLIMITED_BUDGET));
        let mut draft = Draft::default();
        draft.set("budget_b", spelling).unwrap();
        assert_eq!(draft.0.settings.a.budget, None);
        assert_eq!(draft.0.settings.b.budget, Some(UNLIMITED_BUDGET));
    }
}

/// A `pairs` entry describes its pair whole: counts it leaves out are
/// zero, not the defaults or the `[defaults]` table's.
#[test]
fn pairs_entries_zero_unnamed_counts() {
    let grid = Grid::parse("[defaults]\ninsert_b = 5\n[[group]]\npairs = [{ insert_a = 20000 }]\n")
        .unwrap();
    let points = grid.points().unwrap();
    assert_eq!(
        points[0].1.replicas,
        Replicas {
            insert_a: 20_000,
            ..Replicas::default()
        }
    );
}

/// The published reference grid parses under the current axes, every
/// group contributes points, and the grid holds its header's claims.
///
/// A change to the grid format that would break the file users run fails
/// here first. The header's claims are that a full run is 2,568 points,
/// each run three times, for 7,704 sessions, and that only the diagonal
/// group runs the `unlimited` budget.
#[test]
fn reference_grid_parses() {
    let grid = Grid::parse(include_str!("../../grids/reference.toml")).unwrap();
    let points = grid.points().unwrap();
    for group in &grid.groups {
        assert!(
            points.iter().any(|(name, _)| *name == group.name),
            "group `{}` has no points",
            group.name
        );
    }
    assert_eq!(points.len(), 2_568);
    assert_eq!(grid.repetitions, Some(3));
    assert_eq!(points.len() * 3, 7_704);
    let unlimited: BTreeSet<&str> = points
        .iter()
        .filter(|(_, point)| point.settings.a.budget == Some(UNLIMITED_BUDGET))
        .map(|(name, _)| name.as_str())
        .collect();
    assert_eq!(unlimited, BTreeSet::from(["diagonal"]));
}

/// The usable window under Linux's default TCP limits: the smaller of the
/// default `tcp_rmem` maximum (6 MiB) and `tcp_wmem` maximum (4 MiB), at the
/// payload share of a half the tool assumes when the kernel does not say.
const LINUX_DEFAULT_WINDOW: (u64, f64) = (4 << 20, 0.5);

/// The shipped smoke grid touches every path a reference run takes, as its
/// header promises.
///
/// It has one repetition; every reference group; every budget spelling; a
/// point the wire-time limit skips; a point a host at Linux's default TCP
/// limits reruns with raised limits; the pair the reference run's
/// calibration probes, its heaviest, at the `unlimited` budget; and a
/// fresh connection.
#[test]
fn smoke_grid_touches_every_reference_path() {
    let smoke = Grid::parse(include_str!("../../grids/smoke.toml")).unwrap();
    let reference = Grid::parse(include_str!("../../grids/reference.toml")).unwrap();
    let points = smoke.points().unwrap();
    assert_eq!(smoke.repetitions, Some(1));

    let names = |grid: &Grid| -> BTreeSet<String> {
        grid.groups.iter().map(|group| group.name.clone()).collect()
    };
    assert_eq!(names(&smoke), names(&reference));
    for group in &smoke.groups {
        assert!(
            points.iter().any(|(name, _)| *name == group.name),
            "group `{}` has no points",
            group.name
        );
    }

    let spellings: BTreeSet<&str> = smoke
        .groups
        .iter()
        .flat_map(|group| &group.axes)
        .filter(|(axis, _)| axis == "budget")
        .flat_map(|(_, values)| values.iter().map(String::as_str))
        .collect();
    for spelling in ["0", "128MiB", "default", "unlimited"] {
        assert!(spellings.contains(spelling), "no `{spelling}` budget");
    }

    let skips = smoke.groups.iter().any(|group| {
        let product: usize = group.axes.iter().map(|(_, values)| values.len()).product();
        let planned = points
            .iter()
            .filter(|(name, _)| *name == group.name)
            .count();
        planned < product * group.pairs.len().max(1)
    });
    assert!(skips, "no point is skipped for its wire time");

    let (window, share) = LINUX_DEFAULT_WINDOW;
    assert!(
        points
            .iter()
            .any(|(_, point)| raised_buffer(point, window, share).is_some()),
        "no point needs raised TCP limits at Linux's defaults"
    );
    let reference_sessions = sessions(&reference.points().unwrap(), 1, None);
    let (heaviest, _) = probe_points(&reference_sessions).unwrap();
    assert!(
        points.iter().any(|(_, point)| {
            point.replicas == heaviest.replicas
                && point.settings.a.budget == Some(UNLIMITED_BUDGET)
                && point.settings.b.budget == Some(UNLIMITED_BUDGET)
        }),
        "no point runs the reference grid's heaviest pair, {:?}, at the unlimited budget",
        heaviest.replicas
    );
    assert!(
        points
            .iter()
            .any(|(_, point)| point.connection == Connection::Fresh),
        "no fresh connection"
    );
}

/// A grid's plan partitions each group's product into planned points and
/// skips, each skip with a reason.
///
/// The points and skips together number the product of the group's axes
/// and pairs, and no point is both planned and skipped. The shipped grids,
/// which skip points, are the cases.
#[test]
fn plan_partitions_each_group_product() {
    for text in [
        include_str!("../../grids/smoke.toml"),
        include_str!("../../grids/reference.toml"),
    ] {
        let grid = Grid::parse(text).unwrap();
        let (points, skipped) = grid.plan().unwrap();
        assert!(!skipped.is_empty());
        for group in &grid.groups {
            let product: usize = group.axes.iter().map(|(_, values)| values.len()).product();
            let planned: BTreeSet<String> = points
                .iter()
                .filter(|(name, _)| *name == group.name)
                .map(|(_, point)| point.key())
                .collect();
            let skips: Vec<&Skip> = skipped.iter().filter(|s| s.group == group.name).collect();
            assert_eq!(
                planned.len() + skips.len(),
                product * group.pairs.len().max(1)
            );
            for skip in skips {
                assert!(!skip.reason.is_empty());
                assert!(!planned.contains(&skip.point));
            }
        }
    }
}
