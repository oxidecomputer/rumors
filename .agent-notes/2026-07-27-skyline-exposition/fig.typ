// Figure helpers for the skyline exposition. The hand-drawn helpers
// here are pure Typst — no external assets, no packages; the
// document's data charts share their defaults through `viz.typ`,
// which carries the one pinned plotting package (lilaq).
//
// The palette is semantic, not decorative: ink is structure (topology
// bits, first operands, primary linework), accent is content (payload
// bits, second operands) — a cool/warm pair whose hue and lightness
// gaps keep it separable under the common color-vision deficiencies
// and in grayscale print.

#let ink = rgb("#1a3a5c")
#let ink-light = ink.lighten(72%)
#let accent = rgb("#a4551e")
#let accent-light = accent.lighten(75%)
#let gray-line = rgb("#999999")

// Type tokens for text inside figures. Nothing a figure renders may
// go below `fig-label-size` at print.
#let fig-label-size = 7pt // cell labels in strips, lanes, and bitrows
#let fig-annot-size = 7.5pt // annotation lines, axis digits, oprow labels
#let fig-legend-size = 8.5pt // legends, card compartments, tree nodes

// ---------------------------------------------------------------------
// A skyline: a step function over [0, 1), drawn as filled plateaus.
//
// `plateaus`: array of (width, height) pairs, widths summing to 1.0,
// heights in integer units. `ticks`: array of (position, label) pairs
// for the x axis. `unit`: point size of one height unit.
// `bare: true` draws plateaus and baseline only — no grid, no axis
// digits, no height labels — for marks and ornament-scale uses (the
// title block's signature drawing of the running example).
#let skyline(
  plateaus,
  w: 220pt,
  unit: 26pt,
  fill: ink-light,
  stroke: ink,
  ticks: (),
  y-max: none,
  show-heights: true,
  bare: false,
) = {
  let maxh = plateaus.fold(0, (m, p) => calc.max(m, p.at(1)))
  let ymax = if y-max != none { y-max } else { maxh }
  let h = (ymax + 0.55) * unit
  let pad-l = if bare { 0pt } else { 24pt }
  box(width: w + pad-l + if bare { 0pt } else { 10pt }, height: h + if bare { 1pt } else { 16pt }, {
    // y-axis grid lines, with digit labels only where the drawing is
    // quantitative: a schematic skyline (`show-heights: false` — card
    // drawings whose heights stand in for off-scale values) keeps the
    // gridlines for shape reading but drops the digits, which would
    // mislabel it.
    if not bare {
      for lvl in range(0, ymax + 1) {
        let y = h - lvl * unit
        place(top + left, dx: 24pt, dy: y - 0.25pt, line(
          length: w,
          stroke: (paint: gray-line.lighten(50%), thickness: 0.4pt, dash: "dotted"),
        ))
        if show-heights {
          place(top + left, dx: 0pt, dy: y - 5pt, box(width: 20pt,
            align(right, text(size: fig-annot-size, fill: gray-line.darken(20%), str(lvl)))))
        }
      }
    }
    // plateaus
    let x = 0.0
    for p in plateaus {
      let (pw, ph) = p
      let px = pad-l + x * w
      let pwid = pw * w
      if ph > 0 {
        place(top + left, dx: px, dy: h - ph * unit,
          rect(width: pwid, height: ph * unit, fill: fill,
               stroke: (paint: stroke, thickness: 0.9pt)))
      } else {
        place(top + left, dx: px, dy: h - 0.45pt,
          line(length: pwid, stroke: (paint: stroke, thickness: 1.6pt)))
      }
      if show-heights and not bare {
        place(top + left, dx: px, dy: h - ph * unit - 11pt,
          box(width: pwid, align(center, text(size: 8pt, fill: stroke.darken(10%), str(ph)))))
      }
      x = x + pw
    }
    // baseline
    place(top + left, dx: pad-l, dy: h, line(length: w, stroke: 0.7pt + black))
    // x ticks
    for t in ticks {
      let (pos, lab) = t
      place(top + left, dx: pad-l + pos * w, dy: h, line(angle: 90deg, length: 3pt, stroke: 0.7pt + black))
      place(top + left, dx: pad-l + pos * w - 12pt, dy: h + 5pt,
        box(width: 24pt, align(center, text(size: fig-annot-size, lab))))
    }
  })
}

// ---------------------------------------------------------------------
// Two skylines overlaid as outlines (no fill), with elementary-interval
// boundaries marked. Each skyline: array of (width, height).
#let overlay(
  a, b,
  w: 260pt,
  unit: 24pt,
  ticks: (),
  label-a: $a$,
  label-b: $b$,
) = {
  let maxof(s) = s.fold(0, (m, p) => calc.max(m, p.at(1)))
  let ymax = calc.max(maxof(a), maxof(b))
  let h = (ymax + 0.6) * unit
  // collect boundaries of both partitions
  let bounds(s) = {
    let acc = ()
    let x = 0.0
    for p in s {
      x = x + p.at(0)
      if x < 0.999 { acc.push(x) }
    }
    acc
  }
  let all-bounds = (bounds(a) + bounds(b)).dedup()
  // step outline for one skyline
  let outline(s, paint, offset) = {
    let x = 0.0
    for (i, p) in s.enumerate() {
      let (pw, ph) = p
      let y = h - ph * unit - offset
      place(top + left, dx: 24pt + x * w, dy: y,
        line(length: pw * w, stroke: (paint: paint, thickness: 1.5pt)))
      // riser to next plateau
      if i + 1 < s.len() {
        let nh = s.at(i + 1).at(1)
        let y2 = h - nh * unit - offset
        let top-y = calc.min(y.pt(), y2.pt()) * 1pt
        let len = calc.abs(y.pt() - y2.pt()) * 1pt
        place(top + left, dx: 24pt + (x + pw) * w, dy: top-y,
          line(angle: 90deg, length: len, stroke: (paint: paint, thickness: 1.1pt, dash: "solid")))
      }
      x = x + pw
    }
  }
  box(width: w + 60pt, height: h + 16pt, {
    for lvl in range(0, ymax + 1) {
      let y = h - lvl * unit
      place(top + left, dx: 0pt, dy: y - 5pt, box(width: 20pt,
        align(right, text(size: fig-annot-size, fill: gray-line.darken(20%), str(lvl)))))
      place(top + left, dx: 24pt, dy: y - 0.25pt, line(
        length: w, stroke: (paint: gray-line.lighten(55%), thickness: 0.4pt, dash: "dotted")))
    }
    // elementary-interval boundaries
    for bx in all-bounds {
      place(top + left, dx: 24pt + bx * w, dy: 2pt,
        line(angle: 90deg, length: h - 2pt,
          stroke: (paint: gray-line, thickness: 0.6pt, dash: "dashed")))
    }
    outline(a, ink, 0pt)
    outline(b, accent, 2.2pt)
    place(top + left, dx: 24pt, dy: h, line(length: w, stroke: 0.7pt + black))
    for t in ticks {
      let (pos, lab) = t
      place(top + left, dx: 24pt + pos * w - 12pt, dy: h + 5pt,
        box(width: 24pt, align(center, text(size: fig-annot-size, lab))))
    }
    // legend, on a paper-white knockout so it stays legible over
    // lines and gridlines behind it
    place(top + right, dx: 0pt, dy: 2pt, box(fill: white, inset: 3pt, {
      stack(dir: ttb, spacing: 4pt,
        stack(dir: ltr, spacing: 4pt,
          box(baseline: -2.5pt, line(length: 14pt, stroke: 1.5pt + ink)), text(size: fig-legend-size, label-a)),
        stack(dir: ltr, spacing: 4pt,
          box(baseline: -2.5pt, line(length: 14pt, stroke: 1.5pt + accent)), text(size: fig-legend-size, label-b)))
    }))
  })
}

// ---------------------------------------------------------------------
// A packed bit stream: a row of cells, each (bits, role, label) with
// role "t" (topology), "p" (payload), or "x" (neutral). Renders bits in
// monospace with a caption row underneath. Block-level; wrap in
// `align(center, ..)` at the call site if desired.
#let bitrow(cells) = {
  stack(dir: ltr, spacing: 5pt, ..cells.map(c => {
    let (bits, role, label) = c
    let bg = if role == "t" { ink-light } else if role == "p" { accent-light } else { white }
    grid(columns: 1, align: center, row-gutter: 2.5pt,
      box(inset: (x: 4pt, y: 3.5pt), stroke: 0.6pt + gray-line.darken(30%), fill: bg,
        raw(bits)),
      text(size: fig-label-size, fill: gray-line.darken(35%), label))
  }))
}

// ---------------------------------------------------------------------
// Accumulator digit lanes: an array of digit strings, index 0 leftmost
// rendered as the most significant on the LEFT. Each entry (value,
// note) rendered as a lane box.
#let lanes(digits, caption: none) = {
  align(center, stack(dir: ltr, spacing: 3pt, ..digits.map(d => {
    let (val, idx) = d
    grid(columns: 1, align: center, row-gutter: 2.5pt,
      box(inset: (x: 8pt, y: 4.5pt), stroke: 0.7pt + ink, fill: ink-light.lighten(40%),
        raw(val)),
      text(size: fig-label-size, fill: gray-line.darken(35%), [#idx]))
  })))
}

// ---------------------------------------------------------------------
// Simple binary tree rendering for small examples: node = (content,
// left, right) with `none` for absent children. Returns a box.
#let tree(node, dx: 30pt, dy: 24pt, node-fill: white, r: 9pt) = {
  // measure depth
  let depth(n) = {
    if n == none { 0 } else { 1 + calc.max(depth(n.at(1)), depth(n.at(2))) }
  }
  let d = depth(node)
  let width = dx * calc.pow(2, d - 1)
  let height = dy * d
  box(width: width, height: height + 6pt, {
    let draw(n, cx, cy, spread) = {
      if n == none { return }
      let (lab, l, rr) = n
      if l != none {
        place(top + left, dx: cx, dy: cy, line(start: (0pt, 0pt), end: (-spread, dy), stroke: 0.7pt + gray-line.darken(20%)))
        draw(l, cx - spread, cy + dy, spread / 2)
      }
      if rr != none {
        place(top + left, dx: cx, dy: cy, line(start: (0pt, 0pt), end: (spread, dy), stroke: 0.7pt + gray-line.darken(20%)))
        draw(rr, cx + spread, cy + dy, spread / 2)
      }
      place(top + left, dx: cx - r, dy: cy - r,
        box(width: 2 * r, height: 2 * r, fill: node-fill, stroke: 0.8pt + ink,
          radius: r, align(center + horizon, text(size: fig-legend-size, lab))))
    }
    draw(node, width / 2, r + 1pt, width / 4)
  })
}


// Algorithm/pseudocode block.
#let algo(title: none, body) = block(
  width: 100%, inset: (x: 10pt, y: 8pt), radius: 2pt,
  fill: luma(247), stroke: (left: 2pt + ink, rest: none),
  {
    if title != none { text(size: 9pt, weight: "bold", smallcaps(title)); v(4pt) }
    set text(size: 9pt)
    body
  },
)

// ---------------------------------------------------------------------
// Attack cards: one adversarial family per card. Three compartments,
// uniform across every family in the document — the input shape the
// attacker constructs (drawn), the work it extracts from the design it
// refutes, and the landed mechanism that defeats it. A family whose
// point is a floor rather than a cure passes `cure: none` and states
// the floor in `forces`.
#let attack(
  name,
  target,
  input,
  forces,
  cure: none,
) = block(
  width: 100%, stroke: 0.7pt + gray-line.darken(10%), radius: 3pt,
  inset: 0pt, clip: true,
  stack(
    dir: ttb,
    block(width: 100%, fill: ink-light.lighten(45%), inset: (x: 9pt, y: 5pt),
      grid(columns: (auto, 1fr), align: (left, right),
        text(size: fig-legend-size, weight: "bold", smallcaps(name)),
        text(size: fig-legend-size, style: "italic")[attacks: #target])),
    block(width: 100%, inset: (x: 10pt, y: 9pt), align(center, input)),
    block(width: 100%, inset: (x: 9pt, y: 6pt),
      stroke: (top: 0.5pt + gray-line.lighten(30%)),
      text(size: fig-legend-size)[#text(fill: accent.darken(10%), weight: "bold",
        smallcaps("extracts")) #h(0.6em) #forces]),
    ..if cure != none {
      (block(width: 100%, inset: (x: 9pt, y: 6pt),
        stroke: (top: 0.5pt + gray-line.lighten(30%)),
        text(size: fig-legend-size)[#text(fill: ink, weight: "bold",
          smallcaps("defeated by")) #h(0.6em) #cure]),)
    } else { () },
  ),
)

// ---------------------------------------------------------------------
// A schematic packed-stream strip for attack cards: cells of
// (label, width, role), role "t" topology, "p" payload, "w" wide
// payload, "x" neutral (ellipses, annotations).
#let codestrip(cells, h: 15pt) = stack(dir: ltr, spacing: 1.5pt, ..cells.map(c => {
  let (label, w, role) = c
  let bg = if role == "t" { ink-light } else if role == "p" { accent-light } else if role == "w" { accent-light.darken(12%) } else { white }
  let strk = if role == "x" { none } else { 0.6pt + gray-line.darken(20%) }
  box(width: w, height: h, fill: bg, stroke: strk,
    align(center + horizon, text(size: fig-label-size, label)))
}))

// A labeled row for multi-operand attack cards: a small left label, a
// drawing on the right.
#let oprow(label, body) = grid(
  columns: (52pt, 1fr), align: (right + horizon, left + horizon),
  column-gutter: 8pt,
  text(size: fig-annot-size, fill: gray-line.darken(40%), label), body,
)
