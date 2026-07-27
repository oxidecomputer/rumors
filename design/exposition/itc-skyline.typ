// The skyline exposition: how an Interval Tree Clock becomes a compact
// bit-packed value whose every operation is a funded linear sweep.
//
// Standalone companion document. Compile: `typst compile itc-skyline.typ`

#import "fig.typ": *

#set page(
  paper: "a4",
  margin: (x: 2.6cm, y: 2.8cm),
  numbering: "1",
)
#set text(size: 10pt)
#set par(justify: true, leading: 0.62em)
#set heading(numbering: "1.1")
#show heading: it => { v(0.35em); it; v(0.25em) }
#show heading.where(level: 1): it => { v(0.8em); text(size: 14pt, it); v(0.35em) }
#set math.equation(numbering: none)
#show figure.caption: it => text(size: 9pt, it)
#show raw: set text(size: 9pt)

// Title block ---------------------------------------------------------

#align(center, {
  v(1.2em)
  text(size: 19pt, weight: "bold")[Interval Tree Clocks, Compiled to a Skyline]
  v(0.7em)
  text(size: 11.5pt)[The representation, the algorithms, and the costs, from first principles]
  v(0.5em)
  text(size: 9.5pt, style: "italic")[A companion exposition to an implementation of Almeida, Baquero & Fonte's #emph[Interval Tree Clocks] (2008)]
  v(1.2em)
})

#block(inset: (x: 2.2em), text(size: 9.5pt)[
  *Abstract.* The Interval Tree Clock paper gives elegant recursive
  equations over trees. Transcribing those equations directly into a
  program yields an implementation that is correct — and quadratic, in
  time and in transient memory, on inputs it must be expected to meet:
  measured before the cures, a thirty-kilobyte operand pair cost
  hundreds of megabytes of transient memory inside one comparison, and
  a half-megabyte value took over fourteen seconds to decode. This document
  develops, from first principles, a representation under which every
  clock operation is a bounded number of linear passes over packed
  bits — one pass for most operations, two for the hardest: the
  *skyline*, a delta-coded spelling of the clock's step function,
  paired with a redundant signed-digit *accumulator* that makes every
  running quantity cheap to maintain and cheap to ask about. We derive
  each operation as a sweep, give the informal argument that each is
  asymptotically optimal, derive a counting bound placing the encoding
  within $4.3%$ of the information-theoretic floor for the family of
  values it covers, and examine the constant factors — why the
  representation is the shape caches, branch predictors, and
  word-parallel decoders want. A single thesis organizes the design:
  every bit is touched a bounded number of times, and every touch is
  paid for by an input code consumed, an output code emitted, or the
  death of digits already paid for. That discipline is what makes the
  implementation not just fast on friendly inputs but *resilient to
  arbitrary adverse inputs*: no input, of any magnitude, depth, or
  shape, costs more than a fixed multiple of what it reads and writes.
])

#v(1em)

#outline(depth: 2, indent: 1.2em)
#pagebreak()

#include "01-model.typ"
#include "02-naive.typ"
#include "03-skyline.typ"
#include "04-accumulator.typ"
#include "05-operations.typ"
#include "06-compactness.typ"
#include "07-machine.typ"
#include "08-resilience.typ"
