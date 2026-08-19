# Ownership-gated walks

Landed. A walk over a `Version` visits id-space the party doesn't own and does
nothing with it; gating the leaf cursor on ownership skips that space outright.
All three consumers carry the gate: `fill`/`tick`/`ticks` including the
pre-scan, the masked co-walk at every comparison arity, and `query::project`.

- [`before-ownership-gated-walks.md`](before-ownership-gated-walks.md) —
  findings from execution, the observation, the gated leaf cursor abstraction,
  consumers in payoff order, instruments and acceptance, the cost model, and the
  risks.

Worth reading for its lead finding, which is a caution about corpora: the bench
corpus's tick pair turned out not to be hole-shaped, so the corpus could not
have shown the win it was being used to measure.

---

Resurrected from `design/before-ownership-gated-walks.md`, written 2026-08-04, retired in `e13854de` (Remove outdated design docs). The body below is verbatim: its `design/…` cross-references resolve through the [migration map](../2026-08-19-design-directory-migration/README.md).
