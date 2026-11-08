//! Turning a simulation's records into curves, tables, and figures.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use rumors_testkit::bench::session::{LAN_BDP, LONG_HAUL_BDP, METRO_BDP, Step};
use rumors_testkit::bench::slowdown::{Profile, Unlimited};

use crate::grid::Pair;
use crate::plot::{self, Candidate, Curve as PlotCurve, Figure, bytes};
use crate::record::{LedgerRecord, Record};
use crate::simulate;

/// The extra times the curves report crossing, as fractions: 1%, 5%, 25%.
const THRESHOLDS: [f64; 3] = [0.01, 0.05, 0.25];

/// The current default budget, beside which every candidate is judged.
const CURRENT_DEFAULT: u64 = rumors::default::SYNC_MEMORY_BUDGET as u64;

/// The budget a ledger admits work against under the memory budget `budget`:
/// what it leaves beside the floor and session allowances
/// ([`ledger_budget_within`](rumors::testing::ledger_budget_within)).
///
/// The estimates read the ledgers' excess, what their kinds and depths hold
/// beyond their bounds, so they take this budget; the measured curves and the
/// candidates are memory budgets.
fn admitted(budget: u64) -> u64 {
    let budget = usize::try_from(budget).unwrap_or(usize::MAX);
    u64::try_from(rumors::testing::ledger_budget_within(budget)).unwrap_or(u64::MAX)
}

/// The candidate defaults the figures mark: today's, and the two powers of two
/// below it.
const MARKED_CANDIDATES: [u64; 3] = [CURRENT_DEFAULT / 4, CURRENT_DEFAULT / 2, CURRENT_DEFAULT];

/// The design point: messages per replica.
const DESIGN_POINT: u64 = 1_000_000;

/// One pair's sessions: its unlimited session and each budget's.
struct Curve {
    /// The pair.
    pair: Pair,
    /// The session with an unlimited budget.
    unlimited: Record,
    /// The sessions at each budget, largest budget first.
    points: Vec<Record>,
}

impl Curve {
    /// `Λ`: the largest excess either side's ledger held with an unlimited
    /// budget.
    fn peak(&self) -> u64 {
        self.unlimited.budget_peak()
    }

    /// The lower estimate's inputs.
    fn model(&self) -> Unlimited {
        Unlimited {
            duration: self.unlimited.duration,
            rtt: self.pair.bdp,
            question_bytes: self.unlimited.question_bytes(),
        }
    }

    /// The profile estimate's input, if the unlimited session recorded each
    /// side's profile.
    fn profile(&self) -> Option<Profile> {
        let sides: Vec<Vec<Step>> = self
            .unlimited
            .ledgers
            .iter()
            .map(LedgerRecord::steps)
            .collect();
        if sides.iter().all(Vec::is_empty) {
            return None;
        }
        let sides: Vec<&[Step]> = sides.iter().map(Vec::as_slice).collect();
        Some(Profile::of_sides(self.unlimited.duration, &sides))
    }

    /// The area estimate at the ledger budget `budget` as a multiple of the
    /// unlimited time: `max(T∞, A / B)`.
    ///
    /// `A` is the larger side's budget area under the unlimited budget. The
    /// estimate treats the session's memory × time as free to move between
    /// moments; [`Profile`] compares it with the profile estimate.
    fn area_slowdown(&self, budget: u64) -> f64 {
        let area = self
            .unlimited
            .ledgers
            .iter()
            .map(|ledger| ledger.budget_area)
            .max()
            .unwrap_or(0);
        (area as f64 / budget as f64 / self.unlimited.duration as f64).max(1.0)
    }

    /// The session at `point`'s budget as a multiple of the unlimited one.
    fn slowdown(&self, point: &Record) -> f64 {
        point.duration as f64 / self.unlimited.duration as f64
    }

    /// Each measured budget and its slowdown, largest budget first.
    fn measured(&self) -> Vec<(u64, f64)> {
        self.points
            .iter()
            .map(|point| {
                (
                    point.budget.expect("a ladder point has a budget"),
                    self.slowdown(point),
                )
            })
            .collect()
    }

    /// The largest budget whose slowdown exceeds `1 + threshold`: where the
    /// curve first crosses it as the budget falls.
    fn first_exceeding(&self, threshold: f64) -> Option<u64> {
        self.measured()
            .into_iter()
            .find(|&(_, slowdown)| slowdown > 1.0 + threshold)
            .map(|(budget, _)| budget)
    }

    /// The smallest measured budget at which the session is no slower than
    /// `1 + threshold`, at that budget and at every larger one.
    fn within_from(&self, threshold: f64) -> Option<u64> {
        let mut within = None;
        for (budget, slowdown) in self.measured() {
            if slowdown > 1.0 + threshold {
                break;
            }
            within = Some(budget);
        }
        within
    }

    /// Adjacent measured budgets where the larger one ran slower: where speed
    /// fell as the budget rose.
    fn inversions(&self) -> Vec<(u64, u64, u64, u64)> {
        self.points
            .windows(2)
            .filter(|pair| pair[0].duration > pair[1].duration)
            .map(|pair| {
                (
                    pair[0].budget.unwrap_or(0),
                    pair[0].duration,
                    pair[1].budget.unwrap_or(0),
                    pair[1].duration,
                )
            })
            .collect()
    }
}

/// Groups `records` into curves, skipping pairs without an unlimited session.
fn curves(records: Vec<Record>) -> BTreeMap<Pair, Curve> {
    let mut unlimited = BTreeMap::new();
    let mut points: BTreeMap<Pair, Vec<Record>> = BTreeMap::new();
    for record in records {
        match record.budget {
            None => {
                unlimited.insert(record.pair(), record);
            }
            Some(_) => points.entry(record.pair()).or_default().push(record),
        }
    }
    unlimited
        .into_iter()
        .map(|(pair, unlimited)| {
            let mut points = points.remove(&pair).unwrap_or_default();
            points.sort_by_key(|point| std::cmp::Reverse(point.budget));
            points.dedup_by_key(|point| point.budget);
            (
                pair,
                Curve {
                    pair,
                    unlimited,
                    points,
                },
            )
        })
        .collect()
}

/// The link's name in the reader's terms.
fn link_name(bdp: u64) -> String {
    match bdp {
        LAN_BDP => "LAN (125 kB bandwidth-delay product: 1 Gb/s, 1 ms)".to_owned(),
        METRO_BDP => "metro link (1.25 MB: 1 Gb/s, 10 ms)".to_owned(),
        LONG_HAUL_BDP => "long-haul link (12.5 MB: 1 Gb/s, 100 ms)".to_owned(),
        other => format!("link with a {other} B bandwidth-delay product"),
    }
}

/// The link's short name.
fn link_short(bdp: u64) -> &'static str {
    match bdp {
        LAN_BDP => "LAN",
        METRO_BDP => "metro",
        LONG_HAUL_BDP => "long-haul",
        _ => "other link",
    }
}

/// The overlap's name in the reader's terms.
fn overlap_name(shared_percent: u64) -> String {
    match shared_percent {
        0 => "replicas share nothing".to_owned(),
        percent => format!("replicas share {percent}%"),
    }
}

/// A message count as a power of ten where it is one.
fn messages(count: u64) -> String {
    match count {
        100_000 => "10⁵".to_owned(),
        1_000_000 => "10⁶".to_owned(),
        10_000_000 => "10⁷".to_owned(),
        other => other.to_string(),
    }
}

/// The depths at which units waited, both ledgers together, as
/// `"q2: 812, a1: 3"` (questions about depth 2, answers at depth 1).
fn waits_by_depth(point: &Record) -> String {
    let mut waits: BTreeMap<(usize, char), u64> = BTreeMap::new();
    for ledger in &point.ledgers {
        for (kind, grants) in [('q', &ledger.questions), ('a', &ledger.answers)] {
            for depth in grants.iter().filter(|depth| depth.waited > 0) {
                *waits.entry((depth.depth, kind)).or_default() += depth.waited;
            }
        }
    }
    if waits.is_empty() {
        return "none".to_owned();
    }
    waits
        .into_iter()
        .map(|((depth, kind), waited)| format!("{kind}{depth}: {waited}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Writes one curve's table.
fn curve_table(out: &mut String, curve: &Curve) {
    let model = curve.model();
    let unlimited = &curve.unlimited;
    let pair = curve.pair;
    let _ = writeln!(
        out,
        "\n### {} messages per replica, {}, {}\n",
        messages(pair.messages),
        overlap_name(pair.shared_percent),
        link_name(pair.bdp),
    );
    let _ = writeln!(
        out,
        "Unlimited: {} byte-times ({:.2} round trips; {:.3} s at 1 Gb/s), busier direction {} B; \
         Λ = {} B ({}, {:.0} B per message); question grants {} B, of which kept R* = {} B, over {} questions; \
         estimate's knee from grants R·D/T∞ = {:.0} B, from kept bytes R*·D/T∞ = {:.0} B ({}); kinds and depths held at most {} B within their bounds.\n",
        unlimited.duration,
        unlimited.duration as f64 / pair.bdp as f64,
        unlimited.duration as f64 / 125e6,
        unlimited.busier_bytes(),
        curve.peak(),
        bytes(curve.peak() as f64),
        curve.peak() as f64 / pair.messages as f64,
        unlimited.question_grants(),
        model.question_bytes,
        unlimited
            .ledgers
            .iter()
            .map(|ledger| ledger
                .questions
                .iter()
                .map(|depth| depth.units)
                .sum::<u64>())
            .max()
            .unwrap_or(0),
        unlimited.question_grants() as f64 * pair.bdp as f64 / unlimited.duration as f64,
        model.knee(),
        bytes(model.knee()),
        unlimited
            .ledgers
            .iter()
            .map(|ledger| ledger.within_bounds_peak)
            .max()
            .unwrap_or(0),
    );
    for threshold in THRESHOLDS {
        let _ =
            writeln!(
                out,
                "- First exceeds +{:.0}% at {}; within it from {} up.",
                threshold * 100.0,
                curve.first_exceeding(threshold).map_or(
                    "no measured budget".to_owned(),
                    |budget| format!("{budget} B ({})", bytes(budget as f64))
                ),
                curve.within_from(threshold).map_or(
                    "no measured budget".to_owned(),
                    |budget| format!("{budget} B ({})", bytes(budget as f64))
                ),
            );
    }
    if let (Some(one), Some(quarter)) = (curve.first_exceeding(0.01), curve.first_exceeding(0.25)) {
        let _ = writeln!(
            out,
            "- Knee width: the +1% crossing's budget is {:.2}× the +25% crossing's.",
            one as f64 / quarter.max(1) as f64,
        );
    }
    let inversions = curve.inversions();
    if inversions.is_empty() {
        let _ = writeln!(
            out,
            "- Speed never falls as the budget rises across the measured budgets."
        );
    } else {
        for (larger, slower, smaller, faster) in inversions {
            let _ = writeln!(
                out,
                "- **Inversion:** {larger} B took {slower} byte-times, {smaller} B took {faster} \
                 ({} round trips faster at the smaller budget).",
                (slower - faster) as f64 / pair.bdp as f64,
            );
        }
    }
    let profile = curve.profile();
    let _ = writeln!(
        out,
        "\nEstimates: the lower estimate from kept question bytes, the area estimate `max(T∞, A / B)` from \
         the larger side's budget area, and the profile estimate from the larger side's budget-counted bytes \
         over time (`rumors_testkit::bench::slowdown`); a dash where the budget is zero or the unlimited \
         session recorded no profile.\n"
    );
    let _ = writeln!(
        out,
        "| budget | bytes | slowdown | lower estimate | area estimate | profile estimate | slowdown / profile | extra round trips | units waited | waits by depth | peak excess | ledgers' joint peak | heap peak |\n\
         |---|---|---|---|---|---|---|---|---|---|---|---|---|"
    );
    for point in &curve.points {
        let budget = point.budget.expect("a ladder point has a budget");
        let admits = admitted(budget);
        let estimate = |slowdown: &dyn Fn(u64) -> f64| {
            if admits == 0 {
                "-".to_owned()
            } else {
                format!("{:.4}", slowdown(admits))
            }
        };
        let profiled = |slowdown: &dyn Fn(&Profile) -> f64| {
            profile
                .as_ref()
                .filter(|_| admits > 0)
                .map_or("-".to_owned(), |profile| {
                    format!("{:.4}", slowdown(profile))
                })
        };
        let _ = writeln!(
            out,
            "| {} | {budget} | {:.4} | {} | {} | {} | {} | {:.2} | {} | {} | {} | {} | {} |",
            bytes(budget as f64),
            curve.slowdown(point),
            estimate(&|admits| model.slowdown_at(admits)),
            estimate(&|admits| curve.area_slowdown(admits)),
            profiled(&|profile| profile.slowdown_at(admits)),
            profiled(&|profile| curve.slowdown(point) / profile.slowdown_at(admits)),
            (point.duration as f64 - unlimited.duration as f64) / pair.bdp as f64,
            point.waited(),
            waits_by_depth(point),
            point.budget_peak(),
            point.joint_held_peak,
            point.heap_peak,
        );
    }
    let _ = writeln!(
        out,
        "\nUnlimited: ledgers' joint peak {} B, heap peak {} B, heap left at the end {} B.",
        unlimited.joint_held_peak, unlimited.heap_peak, unlimited.heap_net,
    );
    let _ = writeln!(
        out,
        "\nPer-depth grants under the unlimited budget (ledger: q = questions about depth, a = answers at depth; units / bytes granted / bytes kept):\n"
    );
    for (index, ledger) in unlimited.ledgers.iter().enumerate() {
        let questions = ledger
            .questions
            .iter()
            .map(|depth| {
                format!(
                    "q{}: {} / {} / kept {}",
                    depth.depth,
                    depth.units,
                    depth.bytes,
                    depth.bytes - depth.returned
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let answers = ledger
            .answers
            .iter()
            .map(|depth| {
                format!(
                    "a{}: {} / {} / kept {}",
                    depth.depth,
                    depth.units,
                    depth.bytes,
                    depth.bytes - depth.returned
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(
            out,
            "- ledger {index}: {questions}; {answers}; Λ_side = {} B; mean excess {:.0} B",
            ledger.budget_peak,
            ledger.budget_area as f64 / unlimited.duration as f64,
        );
    }
}

/// The smallest power of two from which up `curve`'s slowdown stays at most
/// `1 + threshold`.
///
/// That is: at it and at every measured power of two above it. If every
/// measured one below `Λ` exceeds the threshold, it is the smallest power of
/// two at or above `Λ`.
fn smallest_power_of_two(curve: &Curve, threshold: f64) -> u64 {
    let mut smallest = curve.peak().next_power_of_two();
    for (budget, slowdown) in curve.measured() {
        if !budget.is_power_of_two() {
            continue;
        }
        if slowdown <= 1.0 + threshold {
            smallest = smallest.min(budget);
        } else {
            break;
        }
    }
    smallest
}

/// Writes the default's candidates: the smallest power of two within each
/// slowdown threshold, and the slowdown at every power of two.
fn candidates(out: &mut String, curves: &BTreeMap<Pair, Curve>) {
    let reference: Vec<&Curve> = curves
        .values()
        .filter(|curve| curve.pair.messages == DESIGN_POINT && curve.pair.bdp == LONG_HAUL_BDP)
        .filter(|curve| matches!(curve.pair.shared_percent, 0 | 90))
        .collect();
    if reference.is_empty() {
        return;
    }
    let _ = writeln!(out, "\n## The default's candidates\n");
    let _ = writeln!(out, "On the reference link at 10⁶ messages per replica:\n");
    let rules = [
        ("no slowdown", 0.0),
        ("within 1%", 0.01),
        ("within 5%", 0.05),
    ];
    for (name, threshold) in rules {
        let per_curve: Vec<(u64, u64)> = reference
            .iter()
            .map(|curve| {
                (
                    curve.pair.shared_percent,
                    smallest_power_of_two(curve, threshold),
                )
            })
            .collect();
        let largest = per_curve
            .iter()
            .map(|&(_, budget)| budget)
            .max()
            .unwrap_or(0);
        let _ = writeln!(
            out,
            "- Smallest power of two with {name}: {} ({}).",
            bytes(largest as f64),
            per_curve
                .iter()
                .map(|&(shared, budget)| format!(
                    "{}: {}",
                    overlap_name(shared),
                    bytes(budget as f64)
                ))
                .collect::<Vec<_>>()
                .join("; "),
        );
    }
    for curve in &reference {
        let _ = writeln!(
            out,
            "- {}: Λ = {} ({}), smallest power of two at or above it {}.",
            overlap_name(curve.pair.shared_percent),
            curve.peak(),
            bytes(curve.peak() as f64),
            bytes(curve.peak().next_power_of_two() as f64),
        );
    }

    let _ = writeln!(
        out,
        "\nSlowdown at each power of two, every measured pair (1.0000 = none; a budget at or above Λ runs exactly as unlimited):\n"
    );
    let powers: Vec<u64> = (16..=29).map(|exponent| 1u64 << exponent).collect();
    let _ = write!(out, "| pair |");
    for power in &powers {
        let _ = write!(out, " {} |", bytes(*power as f64));
    }
    let _ = write!(out, "\n|---|");
    for _ in &powers {
        let _ = write!(out, "---|");
    }
    let _ = writeln!(out);
    for curve in curves.values() {
        let measured = curve.measured();
        let _ = write!(
            out,
            "| {}, {}, {} |",
            messages(curve.pair.messages),
            overlap_name(curve.pair.shared_percent),
            link_short(curve.pair.bdp),
        );
        for power in &powers {
            let cell = if *power >= curve.peak() {
                "1.0000".to_owned()
            } else {
                measured
                    .iter()
                    .find(|&&(budget, _)| budget == *power)
                    .map_or("".to_owned(), |&(_, slowdown)| format!("{slowdown:.4}"))
            };
            let _ = write!(out, " {cell} |");
        }
        let _ = writeln!(out);
    }
}

/// The figure of `pairs`' curves, named by `name`, with `title`, if every pair
/// has a curve.
///
/// Beside each curve it draws the profile estimate if every pair's unlimited
/// session recorded its profile, and the lower estimate otherwise, so one
/// figure never mixes the two.
fn figure(
    curves: &BTreeMap<Pair, Curve>,
    pairs: &[(Pair, String)],
    title: &str,
    candidates: &[u64],
) -> Option<Figure> {
    let chosen: Vec<&Curve> = pairs
        .iter()
        .map(|(pair, _)| curves.get(pair).filter(|curve| !curve.points.is_empty()))
        .collect::<Option<_>>()?;
    let profiles: Option<Vec<Profile>> = chosen.iter().map(|curve| curve.profile()).collect();
    let estimate = if profiles.is_some() {
        "the profile estimate"
    } else {
        "the analytic lower estimate"
    };
    let mut profiles = profiles.map(Vec::into_iter);
    let mut plotted = Vec::new();
    for (curve, (_, name)) in chosen.into_iter().zip(pairs) {
        let estimated: Box<dyn Fn(f64) -> f64> = match profiles.as_mut().and_then(Iterator::next) {
            Some(profile) => Box::new(move |budget| profile.slowdown_at(admitted(budget as u64))),
            None => {
                let model = curve.model();
                Box::new(move |budget| model.slowdown_at(admitted(budget as u64)))
            }
        };
        plotted.push(PlotCurve {
            name: name.clone(),
            measured: curve.measured(),
            estimated,
            flat_from: curve.within_from(0.01),
        });
    }
    Some(Figure {
        title: title.to_owned(),
        curves: plotted,
        thresholds: THRESHOLDS.to_vec(),
        candidates: candidates
            .iter()
            .map(|&budget| Candidate {
                budget,
                label: if budget == CURRENT_DEFAULT {
                    format!("{} (today's default)", bytes(budget as f64))
                } else {
                    bytes(budget as f64)
                },
            })
            .collect(),
        estimate: Some(estimate.to_owned()),
    })
}

/// Renders one figure per pair of the first series, overlaying each series'
/// curve for that pair, so two variants of the session compare on the same
/// axes.
///
/// At most three series are plotted, in the renderer's palette, with the first
/// series' depth-1 question total and `Λ` as reference lines, into `out` as
/// `<messages>-<shared>-<bdp>.svg`. The lower estimate is left out: it is one
/// variant's, and compares nothing across them.
pub fn compare(
    series: &[(String, std::path::PathBuf)],
    out: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let loaded = series
        .iter()
        .map(|(label, path)| Ok((label.clone(), curves(simulate::read(path)?))))
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
    fs::create_dir_all(out)?;
    let Some((_, first)) = loaded.first() else {
        return Ok(());
    };
    for (pair, reference) in first {
        let plotted: Vec<PlotCurve> = loaded
            .iter()
            .filter_map(|(label, curves)| {
                let curve = curves.get(pair).filter(|curve| !curve.points.is_empty())?;
                Some(PlotCurve {
                    name: label.clone(),
                    measured: curve.measured(),
                    estimated: Box::new(|_| 1.0),
                    flat_from: None,
                })
            })
            .take(3)
            .collect();
        if plotted.is_empty() {
            continue;
        }
        let depth_one = reference
            .unlimited
            .ledgers
            .iter()
            .filter_map(|ledger| ledger.questions.iter().find(|grants| grants.depth == 1))
            .map(|grants| grants.bytes)
            .max()
            .unwrap_or(0);
        let figure = Figure {
            title: format!(
                "{} messages, {}, {}",
                messages(pair.messages),
                overlap_name(pair.shared_percent),
                link_name(pair.bdp)
            ),
            curves: plotted,
            thresholds: THRESHOLDS.to_vec(),
            candidates: vec![
                Candidate {
                    budget: depth_one,
                    label: format!("depth-1 questions {}", bytes(depth_one as f64)),
                },
                Candidate {
                    budget: reference.peak(),
                    label: format!("Λ {}", bytes(reference.peak() as f64)),
                },
            ],
            estimate: None,
        };
        let name = format!("{}-{}-{}.svg", pair.messages, pair.shared_percent, pair.bdp);
        fs::write(out.join(name), plot::render(&figure)?)?;
    }
    Ok(())
}

/// Renders the plots and tables of the records in `data` into `out`.
pub fn render(data: &Path, out: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let curves = curves(simulate::read(data)?);
    fs::create_dir_all(out)?;
    let mut tables = String::from(
        "# Budget curves\n\nGenerated by `just budget-curves`. Slowdown is the session's duration over the same session's with an unlimited budget, on the link model.\n",
    );
    candidates(&mut tables, &curves);
    let chosen = MARKED_CANDIDATES;
    let _ = writeln!(tables, "\n## Curves");
    for curve in curves.values() {
        curve_table(&mut tables, curve);
    }

    let pair = |messages, shared_percent, bdp| Pair {
        messages,
        shared_percent,
        bdp,
    };
    let at = DESIGN_POINT;
    let figures = [
        (
            "reference-link",
            "Slowdown against budget: 10⁶ messages per replica, 12.5 MB link",
            vec![
                (pair(at, 0, LONG_HAUL_BDP), overlap_name(0)),
                (pair(at, 90, LONG_HAUL_BDP), overlap_name(90)),
            ],
        ),
        (
            "links-disjoint",
            "Slowdown against budget by link: 10⁶ messages, nothing shared",
            vec![
                (pair(at, 0, LAN_BDP), "LAN, 125 kB".to_owned()),
                (pair(at, 0, METRO_BDP), "metro, 1.25 MB".to_owned()),
                (pair(at, 0, LONG_HAUL_BDP), "long-haul, 12.5 MB".to_owned()),
            ],
        ),
        (
            "links-shared",
            "Slowdown against budget by link: 10⁶ messages, 90% shared",
            vec![
                (pair(at, 90, LAN_BDP), "LAN, 125 kB".to_owned()),
                (pair(at, 90, METRO_BDP), "metro, 1.25 MB".to_owned()),
                (pair(at, 90, LONG_HAUL_BDP), "long-haul, 12.5 MB".to_owned()),
            ],
        ),
        (
            "overlap",
            "Slowdown against budget by overlap: 10⁶ messages, 12.5 MB link",
            vec![
                (pair(at, 0, LONG_HAUL_BDP), overlap_name(0)),
                (pair(at, 90, LONG_HAUL_BDP), overlap_name(90)),
                (pair(at, 99, LONG_HAUL_BDP), overlap_name(99)),
            ],
        ),
        (
            "sizes-disjoint",
            "Slowdown against budget by replica size: nothing shared, 12.5 MB link",
            vec![
                (pair(100_000, 0, LONG_HAUL_BDP), "10⁵ messages".to_owned()),
                (pair(1_000_000, 0, LONG_HAUL_BDP), "10⁶ messages".to_owned()),
                (
                    pair(10_000_000, 0, LONG_HAUL_BDP),
                    "10⁷ messages".to_owned(),
                ),
            ],
        ),
        (
            "sizes-shared",
            "Slowdown against budget by replica size: 90% shared, 12.5 MB link",
            vec![
                (pair(100_000, 90, LONG_HAUL_BDP), "10⁵ messages".to_owned()),
                (
                    pair(1_000_000, 90, LONG_HAUL_BDP),
                    "10⁶ messages".to_owned(),
                ),
                (
                    pair(10_000_000, 90, LONG_HAUL_BDP),
                    "10⁷ messages".to_owned(),
                ),
            ],
        ),
    ];
    let _ = writeln!(tables, "\n## Figures\n");
    for (name, title, pairs) in figures {
        // A figure missing its largest size falls back to the sizes run.
        let available: Vec<(Pair, String)> = pairs
            .into_iter()
            .filter(|(pair, _)| {
                curves
                    .get(pair)
                    .is_some_and(|curve| !curve.points.is_empty())
            })
            .collect();
        if available.is_empty() {
            continue;
        }
        if let Some(figure) = figure(&curves, &available, title, &chosen) {
            fs::write(out.join(format!("{name}.svg")), plot::render(&figure)?)?;
            let _ = writeln!(tables, "- `{name}.svg`: {title}");
        }
    }
    fs::write(out.join("tables.md"), tables)?;
    Ok(())
}
