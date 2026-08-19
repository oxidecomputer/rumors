# Fuelscape widgets in the `before` rustdoc

The plan for putting an interactive fuelscape into every public operation's
`# Complexity` section: a `<details>` expander whose summary states the measured
instruction-count claim and whose body is a hypothesis-testing widget. The data
of record is a committed compact dataset derived from a fuelscape atlas dump,
and the pipeline is pure Rust — `build.rs` as a formatter, no Python anywhere.

- [`before-fuelscape-rustdoc.md`](before-fuelscape-rustdoc.md) — pipeline
  architecture (§1), the compact data format (§2), where complexity claims live
  and what they mean (§3), `build.rs` as a pure formatter (§4), doc attachment
  and its two totality directions (§5), widget assets and rustdoc integration
  (§6), verification wiring (§7), decisions (§8), execution phases (§9), and
  non-goals (§10).

The note states its goal beside its mechanism throughout, and says so: where a
rule below conflicts with the goal, the goal wins and the conflict is a finding.

---

Resurrected from `design/before-fuelscape-rustdoc.md`, written 2026-08-13, retired in `e13854de` (Remove outdated design docs). The body below is verbatim: its `design/…` cross-references resolve through the [migration map](../2026-08-19-design-directory-migration/README.md).
