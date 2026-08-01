// The skyline exposition: how an Interval Tree Clock becomes a compact
// bit-packed value whose every operation is a funded linear sweep.
//
// Standalone companion document. Compile: `typst compile itc-skyline.typ`

#import "fig.typ": *

// The document's type system. One serif family throughout (Libertinus,
// the face Typst embeds, so the build renders identically on any
// machine); the personality lives in treatment, not family: level-1
// headings and the running head speak the same small-caps voice as the
// attack cards' compartment labels, so the document and its figures
// form one system. Sizes: 20/14/11/10 display-to-body, 9pt captions
// and code, figure text per fig.typ's tokens (7pt floor).
#set document(title: "Interval Tree Clocks, Compiled to a Skyline")
#set page(
  paper: "a4",
  margin: (x: 2.6cm, y: 2.8cm),
  numbering: "1",
  // Running head: the current chapter, on every page after the
  // contents — except pages that open a chapter, which announce
  // themselves.
  header: context {
    let pg = here().page()
    if pg <= 2 { return }
    let chapters = query(heading.where(level: 1))
    if chapters.any(h => h.location().page() == pg) { return }
    let prev = chapters.filter(h => h.location().page() < pg)
    if prev.len() == 0 { return }
    let ch = prev.last()
    align(center, text(size: 8pt, fill: gray-line.darken(30%),
      tracking: 0.06em,
      smallcaps[#counter(heading).at(ch.location()).at(0) · #ch.body]))
  },
)
#set text(font: "Libertinus Serif", size: 10pt)
#set par(justify: true, leading: 0.65em)
#set heading(numbering: "1.1")
#show heading: it => { v(0.35em); it; v(0.25em) }
#show heading.where(level: 1): it => {
  v(0.9em)
  let head = if it.numbering == none { it.body } else {
    [#counter(heading).display() #h(0.45em) #it.body]
  }
  block(text(size: 14pt, weight: "bold", tracking: 0.02em,
    smallcaps(head)))
  v(0.45em)
}
#set math.equation(numbering: none)
#show figure.caption: it => text(size: 9pt, it)
#show raw: set text(size: 9pt)

// Title block ---------------------------------------------------------
//
// The mark above the title is the running example itself — the value
// (0, 1, (0, 0, 2)) whose sixteen bits thread the whole document (the
// first skyline figure, the packed stream, the worked join, the worked
// tick) — drawn by the same helper that draws every figure.

#align(center, {
  v(1.6em)
  skyline(((0.5, 1), (0.25, 0), (0.25, 2)), w: 132pt, unit: 11pt,
    bare: true)
  v(1.4em)
  text(size: 20pt, weight: "bold")[Interval Tree Clocks, Compiled to a Skyline]
  v(0.7em)
  text(size: 11.5pt)[The representation, the algorithms, and the costs]
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
  an honest history would ever produce them. Before the cures, the
  committed instruments read fifty-six megabytes of transient
  memory — thousands of times the operand's fifteen kilobytes —
  inside one comparison, and 122 million machine-word writes to
  decode one sixteen-kilobyte value, sixty thousand times the
  cured, linear count. From first principles, this document develops a
  representation under which every primitive clock operation runs
  as a bounded number of linear passes over its packed operands and
  its mandatory output: one pass for most, two where a lookahead or
  a pre-pass earns its keep. One family of operations is honestly
  priced above the reading floor: the exact area measures, whose
  answer can embed the product of two arbitrary integers — a
  reduction proves one integer multiplication mandatory — and whose
  worst case is held within one logarithmic factor of that floor.

  The representation is the *skyline*, a delta-coded spelling of
  the step function a clock's event component denotes, paired with
  a redundant signed-digit *accumulator* that makes every running
  quantity cheap to maintain and, outside one write-only mode,
  cheap to ask about. We derive each operation as a sweep and give
  the informal argument that each is asymptotically optimal. We
  then derive a worst-case counting bound — the coding's longest
  spelling against the longest any code must have. Over the family
  of values it covers, the bound places the encoding within $4.3%$
  of the information-theoretic floor asymptotically and $6.7%$ at
  realistic hundred-byte sizes. Last, we examine the constant
  factors: why the representation is the shape caches, branch
  predictors, and word-parallel decoders want. A single thesis
  organizes the design: every bit is touched a bounded number of
  times, and every touch is paid for by an input code consumed, an
  output code emitted, or accumulator state retired that some
  earlier code already paid for. That discipline makes the
  implementation not just fast on friendly inputs but *resilient
  to arbitrary adverse inputs*: no input, of any magnitude, depth,
  or shape, costs more than a fixed multiple of the bits the
  operation reads plus the bits it must write — and where an answer
  is provably as hard as a multiplication, no more than the
  multiplication bound the reduction demands. Every known boundary
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
