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
//!
//! The renderer's input is [`AtlasData`], a plain-data form deliberately
//! decoupled from the roster: a measuring run converts its [`OpAtlas`]
//! with [`AtlasData::from_atlas`], and a persisted run loads the same
//! type back from a dump ([`crate::dump`]), so both paths feed the
//! identical renderer. The heatmap's binned form is computed by
//! [`aggregate`] into a [`HeatGrid`]; the drawing consumes the grid, and
//! the dump persists it, so the plotted cells and the persisted cells
//! are one computation.

use std::io;
use std::path::{Path, PathBuf};

use plotters::prelude::*;
use plotters::style::text_anchor::{HPos, Pos, VPos};
use serde::{Deserialize, Serialize};

use crate::plan::OpAtlas;

#[cfg(test)]
mod tests;

/// Provenance drawn into every output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RenderMeta {
    /// The commit the run measured (from the recipe's `FUELSCAPE_TIP`).
    pub commit: String,
    /// The base seed every cell derived from.
    pub base_seed: u64,
    /// Samples per size column, on average: the plan splits each row's
    /// budget across columns by expected fuel spread
    /// (`Plan::samples_for`).
    pub samples_per_column: usize,
}

/// The run parameters every document of one dataset shares.
///
/// The commit is deliberately absent: a dataset accretes across
/// measuring runs (a new operation's panel is measured alone rather
/// than re-measuring the whole atlas), so each operation document
/// carries its own measurement commit ([`RenderMeta`]) while the
/// parameters that make readings comparable — the seed schedule and the
/// per-column budget — stay dataset-wide and uniform.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunParams {
    /// The base seed every cell derived from.
    pub base_seed: u64,
    /// Samples per size column, on average ([`RenderMeta`]'s field).
    pub samples_per_column: usize,
}

impl From<&RenderMeta> for RunParams {
    fn from(meta: &RenderMeta) -> RunParams {
        RunParams {
            base_seed: meta.base_seed,
            samples_per_column: meta.samples_per_column,
        }
    }
}

/// One operation's atlas as plain data: everything one panel render
/// consumes, decoupled from the roster so a live measuring run and a
/// loaded dump feed the identical renderer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AtlasData {
    /// The operation's atlas name (also the output file stem).
    pub op_name: String,
    /// Whether the x axis is one operand's own size (`true`) rather
    /// than a total over several operands (`false`); picks the axis
    /// caption.
    pub unary: bool,
    /// The row's declared size measure, stamped verbatim.
    pub size_measure: String,
    /// Every bulk sample, all columns.
    pub samples: Vec<SampleData>,
    /// The adversarial family points.
    pub overlay: Vec<OverlayData>,
}

/// One measured bulk sample, as plain data (the persisted form of
/// [`crate::plan::CellSample`], field for field).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SampleData {
    /// The column's total input size in packed bytes.
    pub size: usize,
    /// The sample's drawn arity.
    pub arity: usize,
    /// Fuel consumed by the one measured kernel call.
    pub fuel: u64,
    /// Whole-sample rejections spent drawing the inputs.
    pub rejected: u64,
}

/// One measured adversarial overlay point, as plain data (the persisted
/// form of [`crate::plan::OverlayPoint`], the family name owned).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverlayData {
    /// The family generator's name.
    pub family: String,
    /// Total packed input bytes.
    pub size: usize,
    /// Fuel consumed by the one measured kernel call.
    pub fuel: u64,
}

impl AtlasData {
    /// Convert a measured [`OpAtlas`] into the renderer's plain-data
    /// form, deriving the roster-bound fields (name, axis kind, size
    /// measure) from the roster row.
    pub fn from_atlas(atlas: &OpAtlas) -> AtlasData {
        // One-operand rows take the whole column size (the party-fold
        // row's single party included: its shares are guest-minted, not
        // input bytes); everything else plots a total (the stamp carries
        // the row's exact measure declaration).
        let unary = matches!(atlas.op.inputs, crate::ops::Inputs::Packed(operands) if operands.len() == 1)
            || matches!(atlas.op.inputs, crate::ops::Inputs::PartyShares);
        AtlasData {
            op_name: atlas.op.name.to_string(),
            unary,
            size_measure: atlas.op.size_measure.to_string(),
            samples: atlas
                .samples
                .iter()
                .map(|s| SampleData {
                    size: s.size,
                    arity: s.arity,
                    fuel: s.fuel,
                    rejected: s.rejected,
                })
                .collect(),
            overlay: atlas
                .overlay
                .iter()
                .map(|p| OverlayData {
                    family: p.family.to_string(),
                    size: p.size,
                    fuel: p.fuel,
                })
                .collect(),
        }
    }
}

/// The heatmap's binned form: axis domains and per-column histograms,
/// exactly the cells [`render_op`] draws.
///
/// Computed from an [`AtlasData`] by [`aggregate`]; persisted alongside
/// the raw samples by [`crate::dump`] so downstream consumers read the
/// plotted cells without redoing the binning. Coordinates are the plot's
/// own: the domains and bin grid live in `log2` space, medians in raw
/// fuel units.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HeatGrid {
    /// Left x-domain edge, `log2(bytes)`.
    pub x_lo: f64,
    /// Right x-domain edge, `log2(bytes)`.
    pub x_hi: f64,
    /// Bottom y-domain edge, `log2(fuel)`.
    pub y_lo: f64,
    /// Top y-domain edge, `log2(fuel)`.
    pub y_hi: f64,
    /// Fuel-axis bin count across the whole y domain (bin `i` spans
    /// `y_lo + i·(y_hi − y_lo)/fuel_bins` upward, half-open).
    pub fuel_bins: usize,
    /// One histogram per size column, ascending by size.
    pub columns: Vec<HeatColumn>,
}

/// One size column's conditional fuel histogram.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HeatColumn {
    /// The column's total input size in packed bytes.
    pub size: usize,
    /// The column's median fuel, in raw fuel units (the reference-slope
    /// anchor).
    pub median: f64,
    /// The column's peak bin count (the per-column normalizer: a cell's
    /// plotted density is `count / peak`).
    pub peak: u32,
    /// Sample counts per fuel bin, index aligned to the grid's bin grid.
    pub counts: Vec<u32>,
}

/// Bin an atlas into its [`HeatGrid`]: axis domains over samples and
/// overlay together, then one per-column histogram and median.
///
/// # Panics
///
/// Panics if the atlas has no samples: every roster row has at least
/// one column, and the dump loader rejects empty sample lists, so an
/// empty atlas here is a programmer error.
pub fn aggregate(data: &AtlasData) -> HeatGrid {
    // Column geometry: sizes are doublings, so log2(size) lands on an
    // integer grid with unit spacing.
    let mut sizes: Vec<usize> = data.samples.iter().map(|s| s.size).collect();
    sizes.sort_unstable();
    sizes.dedup();
    assert!(!sizes.is_empty(), "an atlas without samples cannot render");
    let x_lo = lg(sizes[0] as u64) - 0.55;
    let x_hi = lg(*sizes.last().unwrap() as u64) + 0.55;

    let fuel_lo = data
        .samples
        .iter()
        .map(|s| s.fuel)
        .chain(data.overlay.iter().map(|o| o.fuel))
        .min()
        .unwrap_or(1);
    let fuel_hi = data
        .samples
        .iter()
        .map(|s| s.fuel)
        .chain(data.overlay.iter().map(|o| o.fuel))
        .max()
        .unwrap_or(2);
    let y_lo = lg(fuel_lo) - 0.4;
    let y_hi = lg(fuel_hi) + 0.7;

    let bin_h = (y_hi - y_lo) / FUEL_BINS as f64;
    let columns = sizes
        .iter()
        .map(|&size| {
            let mut fuels: Vec<u64> = data
                .samples
                .iter()
                .filter(|s| s.size == size)
                .map(|s| s.fuel)
                .collect();
            fuels.sort_unstable();
            let median = median(&fuels);
            let mut counts = vec![0u32; FUEL_BINS];
            for &f in &fuels {
                let idx = ((lg(f) - y_lo) / bin_h).floor() as usize;
                counts[idx.min(FUEL_BINS - 1)] += 1;
            }
            let peak = *counts.iter().max().unwrap();
            HeatColumn {
                size,
                median,
                peak,
                counts,
            }
        })
        .collect();

    HeatGrid {
        x_lo,
        x_hi,
        y_lo,
        y_hi,
        fuel_bins: FUEL_BINS,
        columns,
    }
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
///
/// Routed through `libm` rather than the platform's math library: a
/// dump commits the [`HeatGrid`] this function shapes, and the loader
/// re-derives that grid bit-for-bit on whatever host opens the dump —
/// platform libms disagree by an ulp at bin boundaries, `libm`'s
/// pure-Rust kernels do not.
fn lg(v: u64) -> f64 {
    libm::log2(v.max(1) as f64)
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
    let x = libm::exp2(v);
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
///
/// `font_scale` multiplies every text size and every piece of
/// text-metric-derived geometry (label areas, the caption strip, label
/// collision gaps) for print output; `1.0` is the reference rendering.
pub fn render_op(
    data: &AtlasData,
    meta: &RenderMeta,
    dir: &Path,
    font_scale: f64,
) -> io::Result<PathBuf> {
    let path = dir.join(format!("{}.svg", data.op_name));
    let (width, height) = (960u32, 640u32);
    // Text-metric-derived pixel geometry, scaled with the fonts so
    // larger print text keeps its clearances.
    let px = |v: f64| (v * font_scale).round() as u32;
    let root = SVGBackend::new(&path, (width, height)).into_drawing_area();
    root.fill(&SURFACE).map_err(draw_err)?;

    let grid = aggregate(data);
    let HeatGrid {
        x_lo,
        x_hi,
        y_lo,
        y_hi,
        ..
    } = grid;

    let (chart_area, caption_area) = root.split_vertically(height - px(58.0));

    let title = format!("{} — p(fuel | size)", data.op_name);
    let mut chart = ChartBuilder::on(&chart_area)
        .caption(
            title,
            ("sans-serif", 17.0 * font_scale).into_font().color(&INK),
        )
        .margin(10)
        .x_label_area_size(px(42.0))
        .y_label_area_size(px(64.0))
        .build_cartesian_2d(x_lo..x_hi, y_lo..y_hi)
        .map_err(draw_err)?;

    chart
        .configure_mesh()
        .disable_mesh()
        .axis_style(INK_SOFT.stroke_width(1))
        .label_style(
            ("sans-serif", 12.0 * font_scale)
                .into_font()
                .color(&INK_SOFT),
        )
        .x_desc(if data.unary {
            "input size (bytes, log scale)"
        } else {
            "total input size (bytes, log scale)"
        })
        .y_desc("fuel (wasm instructions, log scale)")
        .x_labels(grid.columns.len().min(12))
        .x_label_formatter(&|v| pow2_label((*v).round()))
        .y_labels(8)
        .y_label_formatter(&|v| pow2_label(*v))
        .draw()
        .map_err(draw_err)?;

    // The bulk cloud: per-column histograms over one global bin grid,
    // normalized per column (each column's mode is the ramp's dark end).
    let bin_h = (y_hi - y_lo) / grid.fuel_bins as f64;
    for column in &grid.columns {
        let peak = column.peak as f64;
        let cx = lg(column.size as u64);
        chart
            .draw_series(
                column
                    .counts
                    .iter()
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
    if let Some((n0, m0)) = grid
        .columns
        .iter()
        .find(|c| c.size >= 2)
        .map(|c| (c.size, c.median))
    {
        let x0 = lg(n0 as u64);
        let y0 = libm::log2(m0.max(1.0));
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
                Box::new(move |x| y0 + (x - x0) + libm::log2(x / x0.max(1.0)).max(0.0)),
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
                        (lx - 0.06, ly - 0.25 * font_scale)
                    } else {
                        (lx - 0.06, ly + 0.35 * font_scale)
                    },
                    ("sans-serif", 12.0 * font_scale)
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
            data.overlay
                .iter()
                .map(|p| Cross::new((lg(p.size as u64), lg(p.fuel)), 4, ACCENT.stroke_width(2))),
        )
        .map_err(draw_err)?;
    let mut anchors: Vec<(&str, f64, f64)> = Vec::new();
    for point in &data.overlay {
        let (x, y) = (lg(point.size as u64), lg(point.fuel));
        match anchors
            .iter_mut()
            .find(|(name, _, _)| *name == point.family)
        {
            Some(slot) if slot.1 <= x => *slot = (point.family.as_str(), x, y),
            Some(_) => {}
            None => anchors.push((point.family.as_str(), x, y)),
        }
    }
    anchors.sort_by(|a, b| a.2.total_cmp(&b.2));
    let min_gap = (y_hi - y_lo) / 26.0 * font_scale;
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
                ("sans-serif", 12.0 * font_scale)
                    .into_font()
                    .color(&ACCENT)
                    .pos(Pos::new(hpos, VPos::Center)),
            )
        }))
        .map_err(draw_err)?;

    // The caption strip: a layer key and the provenance stamp.
    caption_area.fill(&SURFACE).map_err(draw_err)?;
    let rejected: u64 = data.samples.iter().map(|s| s.rejected).sum();
    let accepted = data.samples.len() as u64;
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
        "commit {} · seed {:#x} · {} samples/column (avg; spread-weighted) · measure: {}{} · fuel: wasmtime instruction metering",
        meta.commit, meta.base_seed, meta.samples_per_column, data.size_measure, acceptance,
    );
    caption_area
        .draw_text(
            key,
            &("sans-serif", 12.0 * font_scale)
                .into_font()
                .color(&INK_SOFT),
            (px(74.0) as i32, px(8.0) as i32),
        )
        .map_err(draw_err)?;
    caption_area
        .draw_text(
            &stamp,
            &("sans-serif", 11.0 * font_scale)
                .into_font()
                .color(&INK_SOFT),
            (px(74.0) as i32, px(28.0) as i32),
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
///
/// The stamp carries the dataset-wide run parameters; each panel's own
/// SVG carries its measurement commit.
pub fn render_gallery(
    ops: &[(String, PathBuf)],
    params: &RunParams,
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
         <p class=\"stamp\">seed {:#x} · {} samples/column (avg; spread-weighted) · fuel: wasmtime instruction metering · each panel carries its measurement commit</p>\n\
         <div class=\"grid\">\n{figures}</div>\n</body>\n</html>\n",
        params.base_seed, params.samples_per_column,
    );
    std::fs::write(&path, html)?;
    Ok(path)
}

/// Adapt plotters' backend-generic error to `io::Error` (the SVG backend's
/// failures are file I/O underneath).
fn draw_err<E: std::error::Error + Send + Sync + 'static>(e: E) -> io::Error {
    io::Error::other(e)
}
