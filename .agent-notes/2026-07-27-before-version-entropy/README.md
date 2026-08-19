# The Version encoding against the information-theoretic floor

How far the packed `Version` encoding's worst case sits above the counting bound
for the set it covers, with Elias gamma as built and delta and omega as the
alternatives. Scope is deliberately half the question: this is the worst-case
counting side, and §8 explains why the workload-typical side — value histograms
over realistic corpora — must be read beside it before either result is acted
on.

- [`before-version-entropy.md`](before-version-entropy.md) — the framing (§1),
  the exact grammar extracted from the code (§2), four nested counting models
  (§3), the analytic constants (§4), the nonnegativity pruning (§5), the census
  machinery and its pins (§6), results (§7), what this does and does not say
  (§8), the proposed docs claim (§9), and what was deferred (§11).

Every grammar fact was extracted from the tree at commit `cb6a252d`, with the
enforcement point named at each.

---

Resurrected from `design/before-version-entropy.md`, written 2026-07-27, retired in `e13854de` (Remove outdated design docs). The body below is verbatim: its `design/…` cross-references resolve through the [migration map](../2026-08-19-design-directory-migration/README.md).
