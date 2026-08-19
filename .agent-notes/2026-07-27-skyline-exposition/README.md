# Interval Tree Clocks, Compiled to a Skyline

A standalone book-length exposition, in Typst, of how an Interval Tree Clock
becomes a compact bit-packed value whose every operation is a funded linear
sweep. It is the readable account of the `before` representation: it builds the
model from scratch, shows the naive encoding failing, derives the skyline that
replaces it, and then prices every operation against measured data.

Compile it with `typst compile itc-skyline.typ` — 110 pages, no arguments, no
network. The build is self-contained: the fonts are the ones Typst embeds, so it
renders identically anywhere.

## The document

- [`itc-skyline.typ`](itc-skyline.typ) — the root. Sets the type system and
  includes the chapters in order.
- [`01-model.typ`](01-model.typ) — the model: what an ITC is and what it must do.
- [`02-naive.typ`](02-naive.typ) — the naive encoding, and where it fights back.
- [`03-skyline.typ`](03-skyline.typ) — the skyline: the representation that replaces it.
- [`04-accumulator.typ`](04-accumulator.typ) — the accumulator, and what a funded sweep is.
- [`05-operations.typ`](05-operations.typ) — every operation as a sweep. The longest chapter by far.
- [`06-compactness.typ`](06-compactness.typ) — compactness: how close the encoding sits to the floor.
- [`07-machine.typ`](07-machine.typ) — the machine chapter: measured instruction counts, replayed from the atlas below.
- [`08-resilience.typ`](08-resilience.typ) — resilience: what happens when inputs are hostile or malformed.

## Its apparatus

- [`fig.typ`](fig.typ) — the figure vocabulary and design tokens shared by every chapter.
- [`viz.typ`](viz.typ) — the chart defaults: a semantic ink/accent cycle, quiet
  grid, direct labels, viridis heatmaps, against a pinned lilaq.
- [`viz-exemplar.typ`](viz-exemplar.typ) — a standalone style exemplar on
  derived inline data, so the chart language can be inspected without building
  the book.
- `data/fuelscape-8k-2000/` — the survey of record that chapter 7 replays: 63
  panels, 8 KiB span, 2000 samples per cell, stamped `2db1a20e`. Read by
  `07-machine.typ` at compile time, so the numbers in the book are never
  transcribed by hand.

---

Moved from `design/exposition/` (first drafted 2026-07-27, last revised
2026-08-06). Contents are unchanged and the whole directory moved as a unit, so
every relative include, import, and data load still resolves; the move was
verified by compiling the document.
