# Eager absorption: K-deep reply parking on one socket

A feasibility assessment, not a design of record, answering a proposal from
Finch: convert incoming wire frames into *logical* protocol replies at arrival
and park up to K of them per stream at the demux boundary, making a
K-reply-denominated window sound on a single socket. The claim under test was
that this is a custody change — where bytes live before the cursor reaches them
— and not a tree-semantics change. The note answers from the code, section by
section, and states its verdict first.

- [`eager-absorption.md`](eager-absorption.md) — verdict (§1), how a provision
  run flows (§2), plumbing (§3), version bounds (§4), violations (§5), torn
  state (§6), K-window bookkeeping (§7), and the honest unknowns (§8).

Input to the [single-socket campaign](../2026-07-21-single-socket/README.md),
which was ultimately declined.

---

Resurrected from `design/eager-absorption.md`, written 2026-07-21, retired in `e13854de` (Remove outdated design docs). The body below is verbatim: its `design/…` cross-references resolve through the [migration map](../2026-08-19-design-directory-migration/README.md).
