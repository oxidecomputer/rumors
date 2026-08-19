// Chart defaults for the exposition's data panels: the one place the
// document's plotting conventions live, so every panel any chapter
// draws — and every panel a later round adds — shares one axis, grid,
// type, and color system. The hand-drawn diagram helpers stay in
// `fig.typ` (pure Typst); this module carries the document's single
// pinned plotting package.
//
// Conventions, fixed here once:
// - Text: the document's own face at fig.typ's token sizes — axis
//   titles at `fig-legend-size`, tick labels at `fig-annot-size` —
//   so a chart's smallest text obeys the same 7pt print floor as
//   every figure.
// - Grid: dotted hairlines in the figure gray, y-axis only by
//   default — quiet enough to sit under data, never over it.
// - Color: the categorical cycle starts at fig.typ's semantic pair
//   (ink, then accent) and extends within the same two families,
//   ordered dark-to-light so adjacent series stay separable in
//   grayscale print.
// - Heatmaps: viridis — perceptually uniform, robust under the
//   common color-vision deficiencies, and monotone in lightness, so
//   it survives grayscale.
// - Series naming: prefer direct labels placed at the data (via
//   `lq.place`) over legend boxes; a legend is the fallback for
//   panels too dense to label directly.
// - Small multiples: panels in one family share axis ranges and tick
//   conventions; fix them in the shared call site, never per panel.

#import "@preview/lilaq:0.6.0" as lq
#import "fig.typ": accent, fig-annot-size, fig-legend-size, gray-line, ink

// The categorical cycle: ink and accent first (the semantic pair every
// hand-drawn figure already speaks), then a lighter step of each
// family for third and fourth series.
#let chart-ink = ink
#let chart-accent = accent
#let chart-slate = rgb("#6c8db0")
#let chart-sand = rgb("#c99a62")
#let chart-cycle = (chart-ink, chart-accent, chart-slate, chart-sand)

// The heatmap colormap of record.
#let heat-map = color.map.viridis

// Wrap a `lq.diagram` call site: `#show: chart-defaults` in a scope,
// or `#chart(lq.diagram(...))`.
#let chart-defaults(body) = {
  show: lq.set-diagram(
    cycle: chart-cycle,
    width: 240pt,
    height: 140pt,
    xaxis: (mirror: false, subticks: none),
    yaxis: (mirror: false, subticks: none),
  )
  show: lq.set-grid(
    stroke: (paint: gray-line.lighten(40%), thickness: 0.4pt, dash: "dotted"),
  )
  show: lq.cond-set(lq.grid.with(kind: "x"), stroke: none)
  show: lq.set-tick(inset: 2.5pt)
  show: lq.set-legend(pad: 0.5em, radius: 1pt, stroke: 0.5pt + gray-line)
  show: lq.show_(lq.label, it => text(size: fig-legend-size, it))
  set text(size: fig-annot-size)
  body
}

#let chart(body) = chart-defaults(body)
