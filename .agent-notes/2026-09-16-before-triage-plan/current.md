# Current triage state

Work proceeds directly on `main`. The reviewed Before history, including the
public Query property and fused-query resource repair, is pushed at
`ab52f1d2`. The redundant remote `codex/before-triage` branch still exists; its
deletion was not authorized by the execution policy and it has no role in the
remaining work.

The last gate was clean across all legs. It ran immediately before the final
integration; the subsequent rebase added only upstream workflow dependency
updates and produced no conflict. Run `just gate` again before the next code
commit as usual.

## Active outcome

Reconcile the amplification board's heap headline with what its judgment
actually proves, then finish the remaining public time and auxiliary-space
claim audit.

The present heap judgment is:

- ordinary cells: `(peak transient bytes - 8,192) / encoded input bytes <= 24`;
- a few operations use explicit, documented constant units or tighter
  operation-specific ceilings;
- a separate four-point trend rejects scaling exponents above 1.15;
- the release acceptance run currently reports 3,154 green cells and no red
  cells.

The next investigation must decide whether the 8 KiB flat allowance is honest
fixed scaffolding or conceals meaningful small-input amplification, and state
the universal claim precisely. Do not preserve the number 24 merely because it
is already pinned. Check operation-specific denominators and ceilings as part
of the same argument, without adding another instrument.

After that, retain this priority order:

1. finish the public contract audit;
2. consolidate resource verification around the board and delete overlapping
   counters, envelope machinery, pins, and hooks;
3. close the remaining Party/Clock, Version/Span/Rank, and deep-traversal
   semantic gaps while separating the jobs of each oracle and driver;
4. re-audit Party, skyline, and codec production structure for simpler code;
5. settle the API and documentation, prune dependencies and artifacts, and
   reconcile every original finding.

The checklist records completion state; keep investigation history out of it.
