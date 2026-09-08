// Pandoc template for a continuous, grayscale paper-review edition.
#set document(
  title: "Rumors — Persistent replicas and causal iteration",
  description: "Design proposal, implementation plan, and comparison appendix. September 8, 2026.",
)
#set text(font: "Charter", size: 11.5pt, lang: "en", region: "US", hyphenate: auto)
// Justify prose using paragraph-wide line breaking and language-aware hyphenation.
#set par(justify: true, linebreaks: "optimized", leading: 5pt, spacing: 9pt)
#set page(
  paper: "us-letter",
  margin: (left: 1in, right: 1in, top: 0.85in, bottom: 0.85in),
  numbering: "1",
  header: context if counter(page).get().first() > 1 {
    set text(font: "Helvetica Neue", size: 8pt, fill: luma(35%))
    [RUMORS / PERSISTENCE DESIGN #h(1fr) 8 SEPTEMBER 2026]
  },
  footer: context if counter(page).get().first() > 1 {
    align(center, text(size: 9pt, counter(page).display("1")))
  },
)
#set heading(numbering: none)
#show heading.where(level: 1): it => {
  set par(justify: false)
  set text(size: 22pt)
  set block(above: 0pt, below: 16pt)
  it
}
#show heading.where(level: 2): it => {
  set par(justify: false)
  set text(size: 14pt)
  set block(above: 15pt, below: 7pt)
  it
}
#show heading.where(level: 3): it => {
  set par(justify: false)
  set text(size: 12pt)
  set block(above: 11pt, below: 5pt)
  it
}
#set list(indent: 0pt, body-indent: 15pt, spacing: 4pt)
#set enum(indent: 0pt, body-indent: 18pt, spacing: 4pt)
#set terms(hanging-indent: 15pt)
#show raw: set text(font: "DejaVu Sans Mono", size: 9.3pt, hyphenate: false)
#show raw.where(block: false): it => {
  // Break long identifiers and source paths without visible added characters.
  let value = it.text.replace("/", "/\u{200b}").replace("_", "_\u{200b}").replace("::", "::\u{200b}").replace("<", "<\u{200b}")
  text(font: "DejaVu Sans Mono", size: 9.5pt, hyphenate: false, value)
}
#show raw.where(block: true): it => block(
  width: 100%, breakable: false, inset: 9pt, fill: luma(97%), radius: 2pt,
  above: 8pt, below: 8pt,
  { set par(justify: false, leading: 3.3pt); it },
)
#set table(
  inset: (x: 6pt, y: 7pt),
  stroke: (left: none, right: none, top: none, bottom: 0.35pt + luma(75%)),
  fill: (_, y) => if y == 0 { luma(95%) },
)
#set table.hline(stroke: 0.6pt + luma(45%))
#show table: it => {
  // Narrow table columns stay left aligned; justification stretches their gaps.
  set text(size: 10pt)
  set par(justify: false, leading: 3pt, spacing: 5pt)
  it
}
#show table.cell.where(y: 0): strong
#show quote: it => block(
  inset: (left: 12pt), stroke: (left: 1.5pt + luma(60%)),
  above: 8pt, below: 8pt, it.body,
)
#let horizontalRule = line(length: 100%, stroke: 0.5pt + luma(70%))
#let divider = horizontalRule
#let overview(body) = {
  // Keep the short orientation together without compressing the main chapters.
  set text(size: 11pt)
  set par(leading: 3.2pt, spacing: 6pt)
  body
}

#set par(justify: false)
#v(1.15in)
#text(font: "Helvetica Neue", size: 11pt, tracking: 1pt)[RUMORS]
#v(16pt)
#text(size: 31pt, weight: "bold")[Persistent replicas\
and causal iteration]
#v(18pt)
#text(size: 15pt)[Design and implementation plan]
#v(1fr)
#text(size: 11pt)[
  Proposal for review · 8 September 2026\
  Four chapters and a comparison appendix
]
#v(14pt)
#text(size: 10pt, fill: luma(30%))[
  Immutable storage. Library-owned snapshots and reference counts.\
  Concurrent preparation with bounded publication work.
]
#pagebreak()
// Split at a chapter boundary so neither contents page is a short spillover.
#{
  set text(size: 11pt)
  set par(justify: false, leading: 4pt, spacing: 4pt)
  show outline.entry.where(level: 1): set text(weight: "bold")
  outline(title: [Contents], depth: 2, indent: 12pt,
    target: selector(heading).before(<implementation>, inclusive: false))
  pagebreak()
  outline(title: [Contents — continued], depth: 2, indent: 12pt,
    target: selector(heading).after(<implementation>, inclusive: true))
}

#set par(justify: true)
$body$
