//! Rendering: one log-log heatmap per operation, plus the gallery page.
//!
//! Each operation's canvas shows three layers, visually distinct and each
//! labeled in place (identity never rides on color alone):
//!
//! - the **bulk cloud**: per-column conditional fuel distributions as a
//!   2D histogram, one sequential blue ramp (light → dark), normalized
//!   within each column so `p(fuel | size)` is what the eye compares;
//! - **reference slopes** at exponents 1 and 2 and the `n·log n` curve,
//!   recessive dashed gray, anchored at the smallest multi-byte column's
//!   median and labeled at their right ends;
//! - the **adversarial overlay**: the committed family generators as
//!   orange crosses, each family direct-labeled at its largest point.
//!
//! Every render carries a provenance stamp drawn into the image: commit,
//! base seed, samples per column, the row's declared size measure
//! (verbatim from its roster entry), and the fuel currency.

use std::io;
use std::path::{Path, PathBuf};

use plotters::prelude::*;
use plotters::style::text_anchor::{HPos, Pos, VPos};

use crate::plan::OpAtlas;

#[cfg(test)]
mod tests;

/// Provenance drawn into every output.
pub struct RenderMeta {
    /// The commit the run measured (from the recipe's `FUELSCAPE_TIP`).
    pub commit: String,
    /// The base seed every cell derived from.
    pub base_seed: u64,
    /// Samples per size column.
    pub samples_per_column: usize,
}

/// Chart surface (light): the reference palette's chart surface.
const SURFACE: RGBColor = RGBColor(0xfc, 0xfc, 0xfb);
/// Primary ink for titles, labels, and the stamp.
const INK: RGBColor = RGBColor(0x0b, 0x0b, 0x0b);
/// Secondary ink for axis text and captions.
const INK_SOFT: RGBColor = RGBColor(0x52, 0x51, 0x4e);
/// The sequential ramp's light end (low density).
const RAMP_LO: (f64, f64, f64) = (214.0, 230.0, 248.0);
/// The sequential ramp's dark end (peak density), same blue hue.
const RAMP_HI: (f64, f64, f64) = (16.0, 60.0, 110.0);
/// The adversarial overlay accent (categorical slot 2; validated against
/// the ramp's blue).
const ACCENT: RGBColor = RGBColor(0xeb, 0x68, 0x34);
/// Reference-slope gray.
const GUIDE: RGBColor = RGBColor(0x9a, 0x99, 0x94);

/// Fuel-axis histogram bins across the whole y range.
const FUEL_BINS: usize = 56;

/// One sequential-ramp color at density `t` in `[0, 1]` (light → dark).
fn ramp(t: f64) -> RGBColor {
    let mix = |lo: f64, hi: f64| (lo + (hi - lo) * t).round().clamp(0.0, 255.0) as u8;
    RGBColor(
        mix(RAMP_LO.0, RAMP_HI.0),
        mix(RAMP_LO.1, RAMP_HI.1),
        mix(RAMP_LO.2, RAMP_HI.2),
    )
}

/// `log2` with a floor of 1 so a degenerate zero reading cannot produce
/// an infinite coordinate.
fn lg(v: u64) -> f64 {
    (v.max(1) as f64).log2()
}

/// The median of a nonempty slice (mean of the middle pair when even).
fn median(sorted: &[u64]) -> f64 {
    let n = sorted.len();
    if n % 2 == 1 {
        sorted[n / 2] as f64
    } else {
        (sorted[n / 2 - 1] as f64 + sorted[n / 2] as f64) / 2.0
    }
}

/// Format `2^v` as a human-readable count for axis ticks.
fn pow2_label(v: f64) -> String {
    let x = 2f64.powf(v);
    if x >= 1e9 {
        format!("{:.1}G", x / 1e9)
    } else if x >= 1e6 {
        format!("{:.1}M", x / 1e6)
    } else if x >= 1e3 {
        format!("{:.1}k", x / 1e3)
    } else {
        format!("{x:.0}")
    }
}

/// Render one operation's atlas to `dir/<op>.svg`, returning the path.
pub fn render_op(atlas: &OpAtlas, meta: &RenderMeta, dir: &Path) -> io::Result<PathBuf> {
    let path = dir.join(format!("{}.svg", atlas.op.name));
    let (width, height) = (960u32, 640u32);
    let root = SVGBackend::new(&path, (width, height)).into_drawing_area();
    root.fill(&SURFACE).map_err(draw_err)?;

    // Column geometry: sizes are doublings, so log2(size) lands on an
    // integer grid with unit spacing.
    let mut sizes: Vec<usize> = atlas.samples.iter().map(|s| s.size).collect();
    sizes.sort_unstable();
    sizes.dedup();
    assert!(!sizes.is_empty(), "an atlas without samples cannot render");
    let x_lo = lg(sizes[0] as u64) - 0.55;
    let x_hi = lg(*sizes.last().unwrap() as u64) + 0.55;

    let fuel_lo = atlas
        .samples
        .iter()
        .map(|s| s.fuel)
        .chain(atlas.overlay.iter().map(|o| o.fuel))
        .min()
        .unwrap_or(1);
    let fuel_hi = atlas
        .samples
        .iter()
        .map(|s| s.fuel)
        .chain(atlas.overlay.iter().map(|o| o.fuel))
        .max()
        .unwrap_or(2);
    let y_lo = lg(fuel_lo) - 0.4;
    let y_hi = lg(fuel_hi) + 0.7;

    let (chart_area, caption_area) = root.split_vertically(height - 58);

    // One-operand rows take the whole column size (the party-fold row's
    // single party included: its shares are guest-minted, not input
    // bytes); everything else plots a total (the stamp carries the row's
    // exact measure declaration).
    let unary = matches!(atlas.op.inputs, crate::ops::Inputs::Packed(operands) if operands.len() == 1)
        || matches!(atlas.op.inputs, crate::ops::Inputs::PartyShares);
    let title = format!("{} — p(fuel | size)", atlas.op.name);
    let mut chart = ChartBuilder::on(&chart_area)
        .caption(title, ("sans-serif", 17).into_font().color(&INK))
        .margin(10)
        .x_label_area_size(42)
        .y_label_area_size(64)
        .build_cartesian_2d(x_lo..x_hi, y_lo..y_hi)
        .map_err(draw_err)?;

    chart
        .configure_mesh()
        .disable_mesh()
        .axis_style(INK_SOFT.stroke_width(1))
        .label_style(("sans-serif", 12).into_font().color(&INK_SOFT))
        .x_desc(if unary {
            "input size (bytes, log scale)"
        } else {
            "total input size (bytes, log scale)"
        })
        .y_desc("fuel (wasm instructions, log scale)")
        .x_labels(sizes.len().min(12))
        .x_label_formatter(&|v| pow2_label((*v).round()))
        .y_labels(8)
        .y_label_formatter(&|v| pow2_label(*v))
        .draw()
        .map_err(draw_err)?;

    // The bulk cloud: per-column histograms over one global bin grid,
    // normalized per column (each column's mode is the ramp's dark end).
    let bin_h = (y_hi - y_lo) / FUEL_BINS as f64;
    let mut column_medians: Vec<(usize, f64)> = Vec::new();
    for &size in &sizes {
        let mut fuels: Vec<u64> = atlas
            .samples
            .iter()
            .filter(|s| s.size == size)
            .map(|s| s.fuel)
            .collect();
        fuels.sort_unstable();
        column_medians.push((size, median(&fuels)));
        let mut bins = vec![0u32; FUEL_BINS];
        for &f in &fuels {
            let idx = ((lg(f) - y_lo) / bin_h).floor() as usize;
            bins[idx.min(FUEL_BINS - 1)] += 1;
        }
        let peak = *bins.iter().max().unwrap() as f64;
        let cx = lg(size as u64);
        chart
            .draw_series(
                bins.iter()
                    .enumerate()
                    .filter(|(_, &c)| c > 0)
                    .map(|(i, &c)| {
                        let lo = y_lo + i as f64 * bin_h;
                        // A slight vertical inset keeps a visible gap between
                        // occupied bins (the fills-need-spacers rule).
                        Rectangle::new(
                            [
                                (cx - 0.42, lo + 0.06 * bin_h),
                                (cx + 0.42, lo + 0.94 * bin_h),
                            ],
                            ramp(c as f64 / peak).filled(),
                        )
                    }),
            )
            .map_err(draw_err)?;
    }

    // Reference slopes, anchored at the smallest column of at least two
    // bytes (n log n is degenerate at one byte): fuel ∝ n, n², n·log₂ n.
    if let Some(&(n0, m0)) = column_medians.iter().find(|(s, _)| *s >= 2) {
        let x0 = lg(n0 as u64);
        let y0 = m0.max(1.0).log2();
        let steps: Vec<f64> = (0..=100)
            .map(|i| x0 + (x_hi - 0.15 - x0) * i as f64 / 100.0)
            .collect();
        #[allow(clippy::type_complexity)]
        // an inline label/curve trio; a named alias would carry no meaning
        let curves: [(&str, Box<dyn Fn(f64) -> f64>); 3] = [
            ("∝ n", Box::new(move |x| y0 + (x - x0))),
            ("∝ n²", Box::new(move |x| y0 + 2.0 * (x - x0))),
            (
                "∝ n·log n",
                Box::new(move |x| y0 + (x - x0) + (x / x0.max(1.0)).log2().max(0.0)),
            ),
        ];
        // Each curve stops where it leaves the plot (a steeper guide often
        // exits through the top) and carries its label at the exit point,
        // hanging below-left so the labels of steep and shallow guides
        // cannot pile up in one corner.
        let y_cap = y_hi - 0.1;
        for (label, f) in &curves {
            let points: Vec<(f64, f64)> = steps
                .iter()
                .map(|&x| (x, f(x)))
                .take_while(|&(_, y)| y <= y_cap)
                .collect();
            let Some(&(lx, ly)) = points.last() else {
                continue;
            };
            chart
                .draw_series(DashedLineSeries::new(
                    points.iter().copied(),
                    4,
                    3,
                    GUIDE.stroke_width(1),
                ))
                .map_err(draw_err)?;
            let exited_top = lx < x_hi - 0.3;
            chart
                .draw_series(std::iter::once(Text::new(
                    (*label).to_string(),
                    if exited_top {
                        (lx - 0.06, ly - 0.25)
                    } else {
                        (lx - 0.06, ly + 0.35)
                    },
                    ("sans-serif", 12)
                        .into_font()
                        .color(&INK_SOFT)
                        .pos(Pos::new(HPos::Right, VPos::Center)),
                )))
                .map_err(draw_err)?;
        }
    }

    // The adversarial overlay: orange crosses, one direct label per
    // family at its largest point, labels nudged apart when they collide.
    chart
        .draw_series(
            atlas
                .overlay
                .iter()
                .map(|p| Cross::new((lg(p.size as u64), lg(p.fuel)), 4, ACCENT.stroke_width(2))),
        )
        .map_err(draw_err)?;
    let mut anchors: Vec<(&str, f64, f64)> = Vec::new();
    for point in &atlas.overlay {
        let (x, y) = (lg(point.size as u64), lg(point.fuel));
        match anchors
            .iter_mut()
            .find(|(name, _, _)| *name == point.family)
        {
            Some(slot) if slot.1 <= x => *slot = (point.family, x, y),
            Some(_) => {}
            None => anchors.push((point.family, x, y)),
        }
    }
    anchors.sort_by(|a, b| a.2.total_cmp(&b.2));
    let min_gap = (y_hi - y_lo) / 26.0;
    for i in 1..anchors.len() {
        if anchors[i].2 - anchors[i - 1].2 < min_gap {
            anchors[i].2 = anchors[i - 1].2 + min_gap;
        }
    }
    // Labels sit on the outward side of their point (left of right-half
    // points, right of left-half points), so no label runs off the canvas.
    let x_mid = (x_lo + x_hi) / 2.0;
    chart
        .draw_series(anchors.iter().map(|(name, x, y)| {
            let (at, hpos) = if *x > x_mid {
                ((x - 0.09, *y), HPos::Right)
            } else {
                ((x + 0.09, *y), HPos::Left)
            };
            Text::new(
                (*name).to_string(),
                at,
                ("sans-serif", 12)
                    .into_font()
                    .color(&ACCENT)
                    .pos(Pos::new(hpos, VPos::Center)),
            )
        }))
        .map_err(draw_err)?;

    // The caption strip: a layer key and the provenance stamp.
    caption_area.fill(&SURFACE).map_err(draw_err)?;
    let rejected: u64 = atlas.samples.iter().map(|s| s.rejected).sum();
    let accepted = atlas.samples.len() as u64;
    let acceptance = if rejected > 0 {
        format!(
            " · sampler acceptance {:.0}%",
            100.0 * accepted as f64 / (accepted + rejected) as f64
        )
    } else {
        String::new()
    };
    let key = "▮ bulk density (per-column normalized)   ✕ committed adversarial families   ┄ reference slopes";
    let stamp = format!(
        "commit {} · seed {:#x} · {} samples/column · measure: {}{} · fuel: wasmtime instruction metering",
        meta.commit, meta.base_seed, meta.samples_per_column, atlas.op.size_measure, acceptance,
    );
    caption_area
        .draw_text(
            key,
            &("sans-serif", 12).into_font().color(&INK_SOFT),
            (74, 8),
        )
        .map_err(draw_err)?;
    caption_area
        .draw_text(
            &stamp,
            &("sans-serif", 11).into_font().color(&INK_SOFT),
            (74, 28),
        )
        .map_err(draw_err)?;

    root.present().map_err(draw_err)?;
    drop(chart);
    drop(caption_area);
    drop(chart_area);
    drop(root);
    Ok(path)
}

/// Render the small-multiples gallery page linking every per-op SVG.
pub fn render_gallery(
    ops: &[(String, PathBuf)],
    meta: &RenderMeta,
    dir: &Path,
) -> io::Result<PathBuf> {
    let path = dir.join("index.html");
    let mut figures = String::new();
    for (op, svg) in ops {
        let file = svg
            .file_name()
            .expect("per-op renders are files")
            .to_string_lossy();
        figures.push_str(&format!(
            "    <figure><a href=\"{file}\"><img src=\"{file}\" alt=\"{op} fuel atlas\"></a>\
             <figcaption>{op}</figcaption></figure>\n"
        ));
    }
    let html = format!(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
         <title>before population atlas</title>\n<style>\n\
         body {{ background: #fcfcfb; color: #0b0b0b; font: 14px/1.5 sans-serif; margin: 2rem; }}\n\
         p.stamp {{ color: #52514e; font-size: 12px; }}\n\
         div.grid {{ display: grid; grid-template-columns: repeat(auto-fill, minmax(460px, 1fr)); gap: 1rem; }}\n\
         figure {{ margin: 0; }} img {{ width: 100%; height: auto; }}\n\
         figcaption {{ color: #52514e; font-size: 12px; text-align: center; }}\n\
         </style>\n</head>\n<body>\n\
         <h1>before population atlas</h1>\n\
         <p>p(fuel | size) per public operation: uniform draws from exact-size canonical input\n\
         spaces (bulk, blue density), committed adversarial families (orange crosses), reference\n\
         slopes (dashed). Audit view only — enforcement lives in the envelope suite and the\n\
         fuzz-fit bands.</p>\n\
         <p class=\"stamp\">commit {} · seed {:#x} · {} samples/column · fuel: wasmtime instruction metering</p>\n\
         <div class=\"grid\">\n{figures}</div>\n</body>\n</html>\n",
        meta.commit, meta.base_seed, meta.samples_per_column,
    );
    std::fs::write(&path, html)?;
    Ok(path)
}

/// Adapt plotters' backend-generic error to `io::Error` (the SVG backend's
/// failures are file I/O underneath).
fn draw_err<E: std::error::Error + Send + Sync + 'static>(e: E) -> io::Error {
    io::Error::other(e)
}
