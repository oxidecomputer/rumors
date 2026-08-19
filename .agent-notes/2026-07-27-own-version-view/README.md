# OwnVersion: the lazy projection view

Landed. The projection `v / &p` is the one operation in `before` whose output
size is not derivable from its inputs — output Θ(|v|·|p|) bits on a Θ(|v|+|p|)
input, measured at 45–119× the input on the board's probe. `OwnVersion` makes
the projection a lazy view rather than a materialized value, so the blowup is
never paid unless something demands it. Designed in conversation with the owner;
every DECIDED entry is an owner ruling.

- [`own-version-view.md`](own-version-view.md) — motivation, the decided shape,
  the specification, semantic notes for the implementer, the landing kit,
  sequencing, and the decision record.

---

Resurrected from `design/own-version-view.md`, written 2026-07-27, retired in `e13854de` (Remove outdated design docs). The body below is verbatim: its `design/…` cross-references resolve through the [migration map](../2026-08-19-design-directory-migration/README.md).
