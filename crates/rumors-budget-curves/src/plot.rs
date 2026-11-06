//! Rendering one figure: extra time against budget, measured and estimated,
//! as an SVG that follows the reader's light or dark theme.
//!
//! # Theming
//!
//! `plotters` writes every colour as a literal. The figure is drawn with
//! placeholder colours, one per role (surface, inks, grid, each series),
//! and [`theme`] rewrites each placeholder into a CSS custom property that
//! an embedded stylesheet sets: light values by default, dark values under
//! `prefers-color-scheme: dark`, and dark values under rustdoc's
//! `data-theme` attribute when the SVG is inlined into a page, where the
//! page's theme wins over the system's in both directions.
//!
//! # Style
//!
//! The colours are the dataviz skill's reference palette: its first three
//! categorical slots, which keep every pair apart under colour vision
//! deficiency in both themes, and its chart inks. Each series also has its
//! own marker shape and a direct label, so no series is told apart by
//! colour alone.

use plotters::coord::Shift;
use plotters::prelude::*;
use plotters::style::text_anchor::{HPos, Pos, VPos};

/// The figure's width, in pixels.
const WIDTH: u32 = 820;
/// The figure's height, in pixels.
const HEIGHT: u32 = 500;
/// The figure's typeface: the system's interface sans.
const FONT: &str = "system-ui, -apple-system, sans-serif";
/// The lowest extra time the vertical axis shows, as a power of ten of
/// percent: 0.1%. Points below it, and points with no extra time, sit on
/// the axis's floor, labelled as such.
const FLOOR_DECADE: f64 = -1.0;

/// A colour role, drawn as a placeholder and themed afterwards.
#[derive(Clone, Copy)]
struct Role {
    /// The placeholder `plotters` writes.
    placeholder: RGBColor,
    /// The CSS custom property it becomes.
    property: &'static str,
    /// The light theme's value.
    light: &'static str,
    /// The dark theme's value.
    dark: &'static str,
}

/// The chart's surface.
const SURFACE: Role = role(1, "surface", "#fcfcfb", "#1a1a19");
/// Titles and direct labels.
const INK: Role = role(2, "ink", "#0b0b0b", "#f0efec");
/// Axis labels and descriptions.
const SECONDARY: Role = role(3, "ink-secondary", "#52514e", "#c3c2b7");
/// Reference lines and their labels.
const MUTED: Role = role(4, "ink-muted", "#898781", "#898781");
/// Gridlines.
const GRID: Role = role(5, "grid", "#e1e0d9", "#2c2c2a");
/// The axes.
const AXIS: Role = role(6, "axis", "#c3c2b7", "#383835");
/// The series, in the palette's fixed order.
const SERIES: [Role; 3] = [
    role(0x11, "series-1", "#2a78d6", "#3987e5"),
    role(0x12, "series-2", "#eb6834", "#d95926"),
    role(0x13, "series-3", "#1baf7a", "#199e70"),
];
/// Every role, for the stylesheet and the rewrite.
const ROLES: [Role; 9] = [
    SURFACE, INK, SECONDARY, MUTED, GRID, AXIS, SERIES[0], SERIES[1], SERIES[2],
];

/// The role numbered `index`, whose placeholder is `#FE00` and the index.
const fn role(index: u8, property: &'static str, light: &'static str, dark: &'static str) -> Role {
    Role {
        placeholder: RGBColor(0xFE, 0x00, index),
        property,
        light,
        dark,
    }
}

/// One measured curve and an estimate of it.
pub struct Curve {
    /// The series' name, in the reader's terms.
    pub name: String,
    /// Each measured budget, in bytes, and its slowdown.
    pub measured: Vec<(u64, f64)>,
    /// The estimated slowdown at a budget, in bytes.
    pub estimated: Box<dyn Fn(f64) -> f64>,
    /// The smallest budget from which up the measured slowdown stays
    /// within 1%, if any: where the curve starts to rise as the budget
    /// falls.
    pub flat_from: Option<u64>,
}

/// A vertical reference line: a candidate default budget.
pub struct Candidate {
    /// The budget, in bytes.
    pub budget: u64,
    /// The label.
    pub label: String,
}

/// One figure.
pub struct Figure {
    /// The title: the figure's one message.
    pub title: String,
    /// The curves, at most three.
    pub curves: Vec<Curve>,
    /// The extra times drawn as reference lines, as fractions of the
    /// unlimited time; the vertical axis labels each.
    pub thresholds: Vec<f64>,
    /// The vertical reference lines.
    pub candidates: Vec<Candidate>,
    /// The name of the estimate drawn beside each curve, in the
    /// legend's words, or `None` to draw none.
    pub estimate: Option<String>,
}

/// A byte count in binary units, compactly: "512 KiB", "1 MiB".
pub fn bytes(budget: f64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = budget;
    let mut unit = 0;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    if (value - value.round()).abs() < 0.05 {
        format!("{} {}", value.round(), UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

/// The vertical coordinate of a slowdown: the decade of its extra time in
/// percent, floored at [`FLOOR_DECADE`].
fn decade(slowdown: f64) -> f64 {
    let percent = (slowdown - 1.0) * 100.0;
    if percent <= 0.0 {
        FLOOR_DECADE
    } else {
        percent.log10().max(FLOOR_DECADE)
    }
}

/// The vertical axis's ticks: each slowdown and its label. Below a
/// doubling a tick reads as extra time, from one up as a multiple of the
/// unlimited time; the floor's tick reads as no measurable extra time.
const SLOWDOWN_TICKS: [(f64, &str); 10] = [
    (1.001, "none, or < +0.1%"),
    (1.01, "+1%"),
    (1.05, "+5%"),
    (1.25, "+25%"),
    (2.0, "2×"),
    (10.0, "10×"),
    (100.0, "100×"),
    (1_000.0, "1,000×"),
    (10_000.0, "10,000×"),
    (100_000.0, "100,000×"),
];

/// The label of the horizontal axis's tick at `octave`, a power of two in
/// bytes, every `step` octaves.
fn octave_label(octave: f64, step: i64) -> String {
    let rounded = octave.round();
    if (octave - rounded).abs() > 1e-6 || (rounded as i64).rem_euclid(step) != 0 {
        return String::new();
    }
    bytes(rounded.exp2())
}

/// Renders `figure` as a themed SVG document.
pub fn render(figure: &Figure) -> Result<String, Box<dyn std::error::Error>> {
    let mut svg = String::new();
    {
        let root = SVGBackend::with_string(&mut svg, (WIDTH, HEIGHT)).into_drawing_area();
        draw(&root, figure)?;
        root.present()?;
    }
    Ok(theme(&svg, &figure.title))
}

/// Draws `figure` on `root`.
fn draw(
    root: &DrawingArea<SVGBackend<'_>, Shift>,
    figure: &Figure,
) -> Result<(), Box<dyn std::error::Error>> {
    root.fill(&SURFACE.placeholder)?;
    let budgets = figure
        .curves
        .iter()
        .flat_map(|curve| curve.measured.iter().map(|&(budget, _)| budget))
        .filter(|&budget| budget > 0)
        .chain(figure.candidates.iter().map(|candidate| candidate.budget));
    let (low, high) = budgets.fold((u64::MAX, 0), |(low, high), budget| {
        (low.min(budget), high.max(budget))
    });
    let x_range = (low as f64).log2().floor()..(high as f64).log2().ceil() + 0.4;
    let top = figure
        .curves
        .iter()
        .flat_map(|curve| curve.measured.iter().map(|&(_, slowdown)| decade(slowdown)))
        .fold(2.0f64, f64::max)
        .ceil()
        + 0.3;
    let y_range = FLOOR_DECADE - 0.25..top;
    let octave_step = if x_range.end - x_range.start > 12.0 {
        2
    } else {
        1
    };

    let text = |size: f64, role: Role| (FONT, size).into_font().color(&role.placeholder);
    let mut chart = ChartBuilder::on(root)
        .caption(&figure.title, text(19.0, INK))
        .margin(18)
        .x_label_area_size(52)
        .y_label_area_size(150)
        .build_cartesian_2d(x_range.clone(), y_range.clone())?;
    chart
        .configure_mesh()
        .disable_mesh()
        .axis_style(AXIS.placeholder.stroke_width(1))
        .x_labels(((x_range.end - x_range.start) * 2.0) as usize)
        .y_labels(0)
        .x_label_formatter(&|octave| octave_label(*octave, octave_step))
        .x_desc("Memory budget per session (log scale)")
        .y_desc("Slowdown against an unlimited budget (log scale)")
        .label_style(text(15.0, SECONDARY))
        .axis_desc_style(text(16.0, SECONDARY))
        .draw()?;

    // The vertical axis's ticks, drawn by hand at the slowdowns they name:
    // a hairline gridline across the plot, and the label left of the axis,
    // on the drawing area itself so the plot's edge does not clip it.
    let grid = GRID.placeholder.stroke_width(1);
    for (slowdown, label) in SLOWDOWN_TICKS {
        let y = decade(slowdown);
        if y < y_range.start || y > y_range.end {
            continue;
        }
        chart.draw_series(LineSeries::new(
            [(x_range.start, y), (x_range.end, y)],
            grid,
        ))?;
        let (left, pixel_y) = chart.backend_coord(&(x_range.start, y));
        root.draw(&Text::new(
            label,
            (left - 8, pixel_y),
            text(15.0, SECONDARY).pos(Pos::new(HPos::Right, VPos::Center)),
        ))?;
    }
    for octave in (x_range.start.ceil() as i64)..=(x_range.end.floor() as i64) {
        if octave.rem_euclid(octave_step) == 0 {
            let x = octave as f64;
            chart.draw_series(LineSeries::new(
                [(x, y_range.start), (x, y_range.end)],
                grid,
            ))?;
        }
    }

    // Reference lines, muted: the extra-time thresholds, which the
    // vertical axis labels, and the candidate defaults, each labelled at
    // its top.
    let muted = MUTED.placeholder.stroke_width(1);
    for &threshold in &figure.thresholds {
        let y = decade(1.0 + threshold);
        chart.draw_series(LineSeries::new(
            [(x_range.start, y), (x_range.end, y)],
            muted,
        ))?;
    }
    let middle = (x_range.start + x_range.end) / 2.0;
    for (index, candidate) in figure.candidates.iter().enumerate() {
        let x = (candidate.budget as f64).log2();
        chart.draw_series(LineSeries::new(
            [(x, y_range.start), (x, y_range.end)],
            muted,
        ))?;
        // A label in the plot's right half sits left of its line, so the
        // plot's edge cannot clip it; each takes a row of its own.
        let (offset, anchor) = if x > middle {
            (-4, HPos::Right)
        } else {
            (4, HPos::Left)
        };
        chart.draw_series(std::iter::once(
            EmptyElement::at((x, y_range.end))
                + Text::new(
                    candidate.label.clone(),
                    (offset, 4 + 16 * index as i32),
                    text(13.0, MUTED).pos(Pos::new(anchor, VPos::Top)),
                ),
        ))?;
    }

    for (index, curve) in figure.curves.iter().enumerate() {
        let color = SERIES[index].placeholder;
        // The estimate: a thin line in the series' hue, drawn only where it
        // estimates extra time above the axis's floor.
        let samples = if figure.estimate.is_some() { 400 } else { 0 };
        let estimated: Vec<(f64, f64)> = (0..=samples)
            .map(|step| {
                x_range.start + (x_range.end - x_range.start) * f64::from(step) / f64::from(samples)
            })
            .map(|x| (x, decade((curve.estimated)(x.exp2()))))
            .filter(|&(_, y)| y > FLOOR_DECADE)
            .collect();
        chart.draw_series(LineSeries::new(estimated, color.stroke_width(1)))?;

        let points: Vec<(f64, f64)> = curve
            .measured
            .iter()
            .filter(|&&(budget, _)| budget > 0)
            .map(|&(budget, slowdown)| ((budget as f64).log2(), decade(slowdown)))
            .collect();
        chart
            .draw_series(LineSeries::new(points.clone(), color.stroke_width(2)))?
            .label(match curve.flat_from {
                Some(budget) => format!(
                    "{}: within 1% from {} up (ringed)",
                    curve.name,
                    bytes(budget as f64)
                ),
                None => curve.name.clone(),
            })
            .legend(move |(x, y)| {
                EmptyElement::at((x + 10, y))
                    + PathElement::new(vec![(-10, 0), (10, 0)], color.stroke_width(2))
            });
        // Each series' marker: a circle, a square, or a triangle by its
        // index, ringed in the surface colour.
        let ring = SURFACE.placeholder.filled();
        let fill = color.filled();
        match index {
            0 => {
                chart.draw_series(points.iter().map(|&at| {
                    EmptyElement::at(at)
                        + Circle::new((0, 0), 6, ring)
                        + Circle::new((0, 0), 4, fill)
                }))?;
            }
            1 => {
                chart.draw_series(points.iter().map(|&at| {
                    EmptyElement::at(at)
                        + Rectangle::new([(-6, -6), (6, 6)], ring)
                        + Rectangle::new([(-4, -4), (4, 4)], fill)
                }))?;
            }
            _ => {
                chart.draw_series(points.iter().map(|&at| {
                    EmptyElement::at(at)
                        + Polygon::new(vec![(0, -8), (7, 5), (-7, 5)], ring)
                        + Polygon::new(vec![(0, -5), (5, 3), (-5, 3)], fill)
                }))?;
            }
        }

        // Where the curve starts to rise: a hollow ring on the smallest
        // budget still within 1%. The series' legend entry names that
        // budget.
        if let Some(flat_from) = curve.flat_from
            && let Some(&(budget, slowdown)) = curve
                .measured
                .iter()
                .find(|&&(budget, _)| budget == flat_from)
        {
            let at = ((budget as f64).log2(), decade(slowdown));
            chart.draw_series(std::iter::once(
                EmptyElement::at(at) + Circle::new((0, 0), 10, color.stroke_width(2)),
            ))?;
        }
    }

    // One legend entry for every curve's estimate, in muted ink.
    if let Some(estimate) = &figure.estimate {
        chart
            .draw_series(std::iter::empty::<PathElement<(f64, f64)>>())?
            .label(format!("thin lines: {estimate}"))
            .legend(|(x, y)| {
                EmptyElement::at((x + 10, y))
                    + PathElement::new(vec![(-10, 0), (10, 0)], MUTED.placeholder.stroke_width(1))
            });
    }
    chart
        .configure_series_labels()
        // The lower left is where the curves are not: every series is far
        // slower than unlimited at its smallest budgets.
        .position(SeriesLabelPosition::LowerLeft)
        .margin(14)
        .background_style(SURFACE.placeholder.filled())
        .border_style(AXIS.placeholder.stroke_width(1))
        .label_font(text(14.0, INK))
        .draw()?;
    Ok(())
}

/// Rewrites the placeholder colours of `svg` into the stylesheet's custom
/// properties, and labels the document with `title` for assistive
/// technology.
///
/// Each tag's placeholder `fill` and `stroke` attributes become one
/// `style` attribute, since a tag may carry only one.
pub fn theme(svg: &str, title: &str) -> String {
    let mut out = String::with_capacity(svg.len() + 2048);
    let mut rest = svg;
    while let Some(start) = rest.find('<') {
        out.push_str(&rest[..start]);
        let end = start + rest[start..].find('>').expect("every SVG tag closes");
        out.push_str(&theme_tag(&rest[start..=end]));
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    let open = out.find("<svg ").expect("the document is an SVG");
    let close = open + out[open..].find('>').expect("the SVG tag closes");
    let escaped = title
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('"', "&quot;");
    let attributes = format!(
        " class=\"budget-plot\" role=\"img\" aria-label=\"{escaped}\" \
         style=\"max-width:100%;height:auto\""
    );
    let mut themed = String::with_capacity(out.len() + 2048);
    themed.push_str(&out[..open + "<svg".len()]);
    themed.push_str(&attributes);
    themed.push_str(&out[open + "<svg".len()..=close]);
    themed.push_str(&stylesheet());
    themed.push_str(&out[close + 1..]);
    themed
}

/// Moves `tag`'s placeholder colours into one `style` attribute.
fn theme_tag(tag: &str) -> String {
    let mut tag = tag.to_owned();
    let mut style = Vec::new();
    for attribute in ["fill", "stroke"] {
        for role in ROLES {
            let RGBColor(r, g, b) = role.placeholder;
            let literal = format!(" {attribute}=\"#{r:02X}{g:02X}{b:02X}\"");
            if tag.contains(&literal) {
                tag = tag.replace(&literal, "");
                style.push(format!("{attribute}:var(--{})", role.property));
            }
        }
    }
    if style.is_empty() {
        return tag;
    }
    let insert = if tag.ends_with("/>") {
        tag.len() - 2
    } else {
        tag.len() - 1
    };
    tag.insert_str(insert, &format!(" style=\"{}\"", style.join(";")));
    tag
}

/// The stylesheet that sets each role's custom property for the reader's
/// theme.
fn stylesheet() -> String {
    let declarations = |dark: bool| {
        ROLES
            .iter()
            .map(|role| {
                format!(
                    "--{}:{};",
                    role.property,
                    if dark { role.dark } else { role.light }
                )
            })
            .collect::<String>()
    };
    format!(
        "<style>.budget-plot{{{light}}}\
         @media (prefers-color-scheme: dark){{\
         svg.budget-plot:root,:root:not([data-theme=\"light\"]) .budget-plot{{{dark}}}}}\
         :root[data-theme=\"dark\"] .budget-plot,:root[data-theme=\"ayu\"] .budget-plot{{{dark}}}\
         </style>",
        light = declarations(false),
        dark = declarations(true),
    )
}
