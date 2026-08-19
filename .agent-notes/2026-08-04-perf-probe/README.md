# perf-probe: where `tick` and `|` spend their cycles

A measurement investigation, not a design: why `before`'s `tick` and merge
(`|`) ran several times slower than the oracle, where the cycles actually
went, and what closing the gap would take. It profiles rather than
speculates — criterion medians as ground truth, op loops over the bench
suite's own corpus recipe, and call-stack profiles attributed against those
medians — and it is explicit about what is *not* the problem, which is the
half of a profile most reports omit.

- [`probe-report.md`](probe-report.md) — ground truth, where the cycles go,
  the drivers ranked, what is not the problem, the parity assessment, the
  campaign results, and a second round on ownership-gated walks.

The instruments it names — `perf_probe.rs` (op loops) and `emit_probe.rs`
(output-primitive microbenchmarks) — are cargo examples and remain at
`crates/before/examples/`, where they can still be run.

The second-round section reports the landing measured by
[ownership-gated walks](../2026-08-04-before-ownership-gated-walks/README.md);
that note is the design, this one is the measurement.

---

Moved from `crates/before/examples/PROBE-REPORT.md` (written 2026-08-04). A
dated investigation report is a note, not a build artifact, and it was the
only non-buildable file in a directory of cargo examples. Contents unchanged
apart from one cross-reference, re-aimed at the sibling note above.
