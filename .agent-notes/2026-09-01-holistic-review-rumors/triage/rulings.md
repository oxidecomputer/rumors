<!-- CAVEAT LECTOR: the ruling texts below are transcribed by Claude from Finch's decisions in triage sessions; the decisions are Finch's, the wording is Claude's unless quoted. Read with the ground rules in ../../README.md. -->

# Rulings

One numbered entry per decision, in the order made. A ruling names the
finding ids or the owner-decision number it disposes, states the decision
positively, and, for `model` and `defer`, names the home where the intent
now lives (a rustdoc sentence, an AGENTS.md line, a design document, a
shadow-tracker issue). `ledger.tsv` cites rulings by number in its
`ruling` column; `ledger.py check` refuses a `model`, `dispute`, `defer`,
or `fix-amended` row that cites none.

Format:

    ## T<n> (<date>): <one-line title>
    Disposes: <owner decision number and/or finding ids>
    Decision: <the ruling, stated positively>
    Home: <where the intent is recorded, for model/defer; "code" for a fix>
    Reasoning: <optional; the why, when it is not obvious from the decision>

Rulings are appended, never edited; a reversal is a new ruling that names
the one it supersedes.

---
