# Current triage state

Work proceeds directly on `main`. The reviewed resource-contract work is pushed
at `317f0f89`.

The latest increment adds one isolated allocator test comparing complete
production and recursive-oracle representations. It establishes the 100×
example on a balanced 512-party history and exercises the least favorable
evident family under a `2^64` event-count cap. The findings are recorded in
`../2026-09-29-before-crate-page-claims/README.md`. In `src/lib.rs`, only the
owner's current headline edit is included; final time, transient-space, and
quantitative wording is explicitly deferred to the owner's later rewrite. The
full gate is clean.

## Active outcome

Simplify production structure around the public vocabulary. Finish Party first,
then apply the same standard to Version and related public types: whole-value
behavior belongs on the public type, partial traversal belongs on cursors, and
substantial helper state belongs in focused modules. Rewrite touched prose until
the invariants and control flow are concise and teachable.

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
