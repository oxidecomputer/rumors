# tracecheck: the executable trace validator

A plan, chartered by Finch, for turning the model's correspondence table into a
per-trace fact checkable at run time: an executable validator that consumes
protocol traces and reports where an implementation departs from the modeled
schedule. No code was written; the document is the deliverable, and it is
explicit about its own negative space — what tracecheck is *not*.

- [`tracecheck.md`](tracecheck.md) — purpose (§1), trust posture (§2),
  architecture (§3), Rust-side integration (§4), staging (§6), non-goals (§7),
  and open questions for Finch (§8).
- [`causal-reference.py`](causal-reference.py) — the reference implementation of
  the causal check the design builds on.

The body's original sequencing constraint (blocked on the single-socket
refactor) was superseded in place by a 2026-07-23 amendment after that campaign
was [declined](../2026-07-21-single-socket/README.md); the obligations attach
against the `Link` transport instead.

---

Resurrected from `design/tracecheck.md and design/tracecheck/causal-reference.py`, written 2026-07-21, retired in `e13854de` (Remove outdated design docs). The body below is verbatim: its `design/…` cross-references resolve through the [migration map](../2026-08-19-design-directory-migration/README.md).
