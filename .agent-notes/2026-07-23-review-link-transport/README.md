# The link-transport review campaign

A multi-agent skeptical review of the `link-transport` branch against main —
106 commits, 226 files — run as 21 independent passes with every
correctness-bearing finding adversarially re-verified by skeptics instructed to
refute it. 143 raw findings became 82 ruled ones (R1–R82). The three documents
are the findings, the execution state, and the lessons.

- [`review-link-transport-branch.md`](review-link-transport-branch.md) — the
  findings document: majors, minors, nits, hard-rule sweeps, documentation
  accuracy, questions for the author, what was examined and dismissed, what was
  positively assured, and the residual risks and test gaps.
- [`review-link-transport-execution.md`](review-link-transport-execution.md) —
  the execution ledger: rulings, branch inventory, integration plan, queued
  work, open decisions, and environment lessons. Written to be
  session-survivable, so a fresh context could resume without the conversation.
- [`review-link-transport-retrospective.md`](review-link-transport-retrospective.md)
  — what the campaign taught: the practices that carried it, the failure modes
  and their counter-rules, and what it changed about the codebase's epistemics.

The retrospective is the one to read if you are reading only one; several of its
counter-rules were promoted into standing doctrine.

---

Resurrected from `design/review-link-transport-{branch,execution,retrospective}.md`, written 2026-07-23 – 2026-07-24, retired in `e13854de` (Remove outdated design docs). The body below is verbatim: its `design/…` cross-references resolve through the [migration map](../2026-08-19-design-directory-migration/README.md).
