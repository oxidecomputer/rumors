<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) with Finch in the before triage's session of 2026-09-02 (ruling 105 in ../rulings.md); the intent is Finch's, the wording is Claude's. Read with the ground rules in ../../README.md. -->

# Statement of intent: prose in `before`

This triage touches nearly every file in `before`. Every lane that edits a
file also owns the prose it passes through, and the standard is not "no
longer wrong" but "as good as this library deserves". The goal, above the
individual findings, is that a reader at any altitude meets exactly the
sentences they need, in the order they need them, and nothing else. The
writing-style guide (`~/.claude/writing-style.md`) is the doctrine of
record; this statement says what to reach for in this crate and how we will
know.

## Three tests, applied to every paragraph you touch

1. **Altitude.** Ask who is reading this paragraph and what they are
   holding. A crate-root reader holds nothing yet and wants why and a tour.
   A type's reader holds the type and wants its guarantees. A method's
   reader holds a call site and wants the contract, the hazards, and the
   cost. A maintainer inside a module holds the implementation and wants the
   invariant and the argument. A sentence pitched at the wrong reader is a
   defect even when true: an internal mechanism named in public rustdoc, a
   contract restated in a private comment, a design rationale in a test doc.
   Move it to the layer that owns it, or delete it.

2. **Concision.** Every sentence competes with the contract the reader came
   for. Before improving a sentence, ask whether it should exist. Delete
   restatement, hedging, narration of history, enumeration the code can
   change, motivation that changes nothing the reader does, and any claim no
   instrument or argument backs. What survives is stated once, in default
   word order, in the present tense. The cheapest way to make a paragraph
   clear is usually to make it shorter.

3. **Legibility.** A reader should be able to scan the doc and find the
   sentence that answers their question by shape alone: the first sentence
   states what the item is and does; hazards and costs sit under their
   headings; hard claims are said twice, precisely and then plainly; an
   argument reads forward from premise to conclusion with each step
   checkable against the code beside it. Coined words, opaque ids, and
   citations by string are illegible by construction and are gone under
   rulings 43, 52, and 98.

## What the reviewer checks

Every doc comment's first sentence stands alone in a module listing. No
public sentence names an implementation concept. No sentence in the diff
could be falsified by a code change that leaves the prose untouched (counts,
line numbers, readings, test names) unless a mechanical check holds it.
Every hard claim has its argument or its instrument beside it. And the diff
is net shorter in prose unless the lane report says what the added
sentences buy; the reviewer judges that justification.

## What this is not

Not a rewrite pass: touch prose in the files your lane already edits, and
hand the rest back as findings; one dedicated fresh-eyes prose pass per
crate follows the code lanes, when the code has stopped moving. Not a style
imposition on Finch's own words: paragraphs he wrote and has said he will
rewrite (the identity-ladder essay) stay untouched. And not a place for
taste disputes to stall a lane: when a sentence could go two ways, take the
shorter one and note the choice in the lane report.
