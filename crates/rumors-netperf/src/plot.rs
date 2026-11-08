//! Static SVG line charts for the report.
//!
//! Each chart plots a few series of points on log–log or linear axes, with a
//! legend. The colours are the Okabe–Ito palette, chosen to stay
//! distinguishable under the common colour-vision deficiencies.

use std::path::Path;

use anyhow::{Result, anyhow};
use plotters::prelude::*;

/// The Okabe–Ito palette, without its black, in its published order.
const PALETTE: [RGBColor; 7] = [
    RGBColor(0xe6, 0x9f, 0x00),
    RGBColor(0x56, 0xb4, 0xe9),
    RGBColor(0x00, 0x9e, 0x73),
    RGBColor(0xd5, 0x5e, 0x00),
    RGBColor(0x00, 0x72, 0xb2),
    RGBColor(0xcc, 0x79, 0xa7),
    RGBColor(0xf0, 0xe4, 0x42),
];

/// One series: its legend label and its points, in x order.
pub type Series = (String, Vec<(f64, f64)>);

/// A chart's description.
#[derive(Clone, Debug)]
pub struct Chart<'a> {
    /// The title above the plot.
    pub title: &'a str,
    /// The x axis's label, with its unit.
    pub x_label: &'a str,
    /// The y axis's label, with its unit.
    pub y_label: &'a str,
    /// Whether the x axis is logarithmic.
    pub log_x: bool,
    /// Whether the y axis is logarithmic.
    pub log_y: bool,
}

/// The range spanning `values`, padded so no point sits on the frame; for a
/// logarithmic axis, positive values only.
fn range(values: impl Iterator<Item = f64>, log: bool) -> Option<(f64, f64)> {
    let (mut low, mut high) = (f64::INFINITY, f64::NEG_INFINITY);
    for value in values.filter(|v| v.is_finite() && (!log || *v > 0.0)) {
        low = low.min(value);
        high = high.max(value);
    }
    if low > high {
        return None;
    }
    Some(if log {
        (low / 1.5, high * 1.5)
    } else {
        let pad = ((high - low) * 0.08).max(high.abs() * 0.05).max(1e-9);
        (low - pad, high + pad)
    })
}

/// An axis label for `value`: three significant figures, with no trailing
/// zeros, so a logarithmic axis's ticks never print binary rounding noise such
/// as `0.6000000000000001`.
pub fn tick(value: f64) -> String {
    if value == 0.0 || !value.is_finite() {
        return format!("{value}");
    }
    let decimals = (2 - value.abs().log10().floor() as i32).max(0) as usize;
    let text = format!("{value:.decimals$}");
    if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        text
    }
}

/// Draws `series` into an SVG at `path`. Draws nothing, and succeeds, when no
/// series has a point the axes can show.
pub fn line_chart(path: &Path, chart: &Chart, series: &[Series]) -> Result<bool> {
    let xs = series
        .iter()
        .flat_map(|(_, points)| points.iter().map(|p| p.0));
    let ys = series
        .iter()
        .flat_map(|(_, points)| points.iter().map(|p| p.1));
    let (Some(x_range), Some(y_range)) = (range(xs, chart.log_x), range(ys, chart.log_y)) else {
        return Ok(false);
    };
    let root = SVGBackend::new(path, (900, 560)).into_drawing_area();
    root.fill(&WHITE)
        .map_err(|e| anyhow!("drawing {}: {e}", path.display()))?;
    let mut builder = ChartBuilder::on(&root);
    builder
        .caption(chart.title, ("sans-serif", 22))
        .margin(16)
        .x_label_area_size(48)
        .y_label_area_size(72);
    macro_rules! draw {
        ($context:expr) => {{
            let mut context = $context.map_err(|e| anyhow!("drawing {}: {e}", path.display()))?;
            context
                .configure_mesh()
                .x_desc(chart.x_label)
                .y_desc(chart.y_label)
                .x_label_formatter(&|x| tick(*x))
                .y_label_formatter(&|y| tick(*y))
                .label_style(("sans-serif", 14))
                .draw()
                .map_err(|e| anyhow!("drawing {}: {e}", path.display()))?;
            for (index, (label, points)) in series.iter().enumerate() {
                let color = PALETTE[index % PALETTE.len()];
                let shown: Vec<(f64, f64)> = points
                    .iter()
                    .copied()
                    .filter(|(x, y)| (!chart.log_x || *x > 0.0) && (!chart.log_y || *y > 0.0))
                    .collect();
                context
                    .draw_series(LineSeries::new(shown.clone(), color.stroke_width(2)))
                    .map_err(|e| anyhow!("drawing {}: {e}", path.display()))?
                    .label(label.clone())
                    .legend(move |(x, y)| {
                        PathElement::new([(x, y), (x + 18, y)], color.stroke_width(2))
                    });
                context
                    .draw_series(shown.into_iter().map(|p| Circle::new(p, 3, color.filled())))
                    .map_err(|e| anyhow!("drawing {}: {e}", path.display()))?;
            }
            context
                .configure_series_labels()
                .background_style(WHITE.mix(0.85))
                .border_style(BLACK.mix(0.3))
                .label_font(("sans-serif", 13))
                .position(SeriesLabelPosition::UpperLeft)
                .draw()
                .map_err(|e| anyhow!("drawing {}: {e}", path.display()))?;
        }};
    }
    let (x0, x1) = x_range;
    let (y0, y1) = y_range;
    match (chart.log_x, chart.log_y) {
        (true, true) => {
            draw!(builder.build_cartesian_2d((x0..x1).log_scale(), (y0..y1).log_scale()))
        }
        (true, false) => draw!(builder.build_cartesian_2d((x0..x1).log_scale(), y0..y1)),
        (false, true) => draw!(builder.build_cartesian_2d(x0..x1, (y0..y1).log_scale())),
        (false, false) => draw!(builder.build_cartesian_2d(x0..x1, y0..y1)),
    }
    root.present()
        .map_err(|e| anyhow!("writing {}: {e}", path.display()))?;
    Ok(true)
}

#[cfg(test)]
mod tests;
