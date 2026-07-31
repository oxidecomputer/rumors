// STYLE EXEMPLAR — not part of the exposition document.
//
// This page exists to exercise and review `viz.typ`'s chart defaults
// before any data panel ships: one small chart from inline, derived
// data (the closed-form code lengths of Elias gamma and delta, the
// comparison behind the compactness chapter's crossover claim), so
// the axis, grid, type, color, and direct-labeling conventions are
// all visible on one page. Compile standalone:
// `typst compile viz-exemplar.typ`. Data panels for the document
// itself are a later round's work and will consume committed data
// files; nothing here is survey data.

#import "viz.typ": *
#import "fig.typ": fig-annot-size, gray-line

#set page(width: 420pt, height: auto, margin: 24pt)
#set text(font: "Libertinus Serif", size: 10pt)

#let bitlen(m) = calc.floor(calc.log(m, base: 2)) + 1

// Elias gamma: 2·⌊log2(m)⌋ + 1 bits for m = v + 1.
#let gamma-bits(v) = 2 * (bitlen(v + 1) - 1) + 1

// Elias delta: ⌊log2 m⌋ + 2·⌊log2(⌊log2 m⌋ + 1)⌋ + 1 bits for m = v + 1.
#let delta-bits(v) = {
  let l = bitlen(v + 1)
  (l - 1) + 2 * (bitlen(l) - 1) + 1
}

#let vs = range(0, 257)

*Style exemplar.* The chart defaults of `viz.typ`, exercised on
derived inline data: stored-code cost by coded payload value, for the
two integer codes the compactness chapter compares. Direct labels at
the data, no legend box; the one annotation marks the crossover.

#v(0.5em)

#align(center, chart(
  lq.diagram(
    xlabel: [coded payload value $v$],
    ylabel: [code length (bits)],
    lq.plot(vs, vs.map(gamma-bits), mark: none, label: none),
    lq.plot(vs, vs.map(delta-bits), mark: none, label: none),
    lq.vlines(31, stroke: (paint: gray-line, thickness: 0.5pt, dash: "dashed")),
    lq.place(185, 15.9)[#text(size: fig-annot-size, fill: chart-ink)[gamma]],
    lq.place(185, 13.1)[#text(size: fig-annot-size, fill: chart-accent)[delta]],
    lq.place(62, 2.2)[#text(size: fig-annot-size, fill: gray-line.darken(30%))[$v = 31$]],
  ),
))
