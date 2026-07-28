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
  text(size: 9.5pt, style: "italic")[A companion to an implementation of #emph[Interval Tree Clocks] (Almeida, Baquero & Fonte, 2008)]
  v(1.2em)
})

#block(inset: (x: 2.2em), text(size: 9.5pt)[
  *Abstract.* The Interval Tree Clock paper gives elegant recursive
  equations over trees. Transcribing those equations directly into a
  program yields an implementation that is correct — and quadratic,
  in time and in transient memory, on inputs any peer can legally
  present: ordinary canonical values, cheap to spell whether or not
  an honest history would ever produce them. Measured on the direct
  transcription before any cure, a twenty-nine-kilobyte operand
  pair cost nearly two hundred megabytes of transient memory inside
  one comparison, and a value half a megabyte wide — its
  self-delimiting code twice that — took over fourteen seconds to
  decode. This document develops, from first principles, a
  representation under which every primitive clock operation is a
  bounded number of linear passes over its packed operands and its
  mandatory output: one pass for most, two where a lookahead or a
  pre-pass earns its keep, and for the composites the sum of their
  parts. One derivational gap remains, stated where it lives and
  held by a pinned measurement instead.

  The representation is the *skyline*, a delta-coded spelling of
  the step function a clock's event component denotes, paired with
  a redundant signed-digit *accumulator* that makes every running
  quantity cheap to maintain and, outside one write-only mode,
  cheap to ask about. We derive each operation as a sweep and give
  the informal argument that each is asymptotically optimal. We
  then derive a worst-case counting bound — the coding's longest
  spelling against the longest any code must have — which places
  the encoding, over the family of values it covers, within $4.3%$
  of the information-theoretic floor asymptotically and $6.7%$ at
  realistic hundred-byte sizes. Last, we examine the constant
  factors: why the representation is the shape caches, branch
  predictors, and word-parallel decoders want. A single thesis
  organizes the design: every bit is touched a bounded number of
  times, and every touch is paid for by an input code consumed, an
  output code emitted, or the retirement of accumulator state some
  earlier code already paid for. That discipline makes the
  implementation not just fast on friendly inputs but *resilient
  to arbitrary adverse inputs*: no input, of any magnitude, depth,
  or shape, costs more than a fixed multiple of the bits the
  operation reads plus the bits it must write. Every known boundary
  of the argument lives where it binds; all seven are collected at
  the close.
])

#v(0.5em)

#block(inset: (x: 2.2em), text(size: 9.5pt)[
  *Implementation note.* The mechanisms and measurements described
  here are realized in the `before` and `suanpan` Rust crates; the
  document stands alone.
])

#v(1em)

#outline(depth: 3, indent: 1.2em)
#pagebreak()

#include "01-model.typ"
#include "02-naive.typ"
#include "03-skyline.typ"
#include "04-accumulator.typ"
#include "05-operations.typ"
#include "06-compactness.typ"
#include "07-machine.typ"
#include "08-resilience.typ"
