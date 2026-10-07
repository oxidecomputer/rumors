<!-- CAVEAT LECTOR: written by Claude (Opus 5.5), the audit coordinator, recording the owner's ruling on question 59. -->

# Where the audit stops taking on new work

## The question

The audit had twenty-odd branches ready for review, eight in flight, a
finite queue of briefed items, and one open-ended source of new work: the
adequacy lane's mutation campaign, whose survivors can always be met with
more tests or more exotic mutants. The owner asked when it reaches
diminishing returns.

The campaign itself is bounded: cargo-mutants applied a fixed operator
catalog once per site, over every group, about 2,415 viable mutants with
about 225 survivors. No defect in production code came from it. Of the 61
survivors classified in round 1, six were real test gaps, all in suanpan's
`normalize`.

## Owner's ruling

Finish everything in flight and everything in the dispatch queue, plus the
adequacy lane's last round, then begin the final phase (instrument survey,
one full `just gate` at `main`, retirement). The last round classifies every
campaign survivor, briefs only real test gaps (a reachable value change no
test detects) and trait behavior callers rely on, and routes cost-only
survivors to the instrument survey. No new auditor rounds and no further
mutant generation. Anything new the remaining work turns up goes to a
follow-up list for after the audit.
