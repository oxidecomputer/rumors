# Migrating the `design/` notes into `.agent-notes`

Design notes have moved here: the retired ones resurrected from git history, and
several live ones relocated from where they had accumulated. This note records
what moved, where it went, and the judgment calls the move required — including
the ones a reader might want reversed.

## What was resurrected

Forty-two files, drawn from the last revision at which each existed. Most were
retired together in `e13854de` ("Remove outdated design docs"); the rest went
one at a time as the thing they described moved inline into the code. One
document, [materialized backlogs](../2026-07-29-materialized-backlogs/README.md),
was never on `main` at all — it lives on the unmerged branch
`persistent-storage`, and was pulled from that branch's tip.

Each file is byte-identical to the blob it was recovered from. Nothing in any
body was edited: a resurrected document is a record of what someone believed on
the day they wrote it, and rewriting it to agree with today's code would destroy
exactly the thing worth keeping. Orientation — what the note is, what it
claimed, what became of it — lives in each directory's `README.md`, where it can
be honest about the gap.

## Judgment calls

**Directory dates are authoring dates, not the migration date.** The ground
rules ask for the current datestamp on a new note. Stamping all forty-two with
`2026-08-19` would have collapsed six weeks of chronology into one date and
destroyed the ordering the naming scheme exists to give — so each directory
carries the date its document was first written. This note, being genuinely new,
carries today's.

**Related documents were grouped into one note directory** where the
relationship is explicit in the documents themselves: a document and the scripts
that produced its numbers, a draft and the revision that replaced it, a campaign
and its retrospective. Everything else stands alone.

**Two directory names dropped an internal roster number** — `probe-ticks-68`
became `fused-multi-tick-probe` and `b05-uniformity-envelope` became
`uniformity-envelope` — because a bare task number means nothing outside the
roster it came from. The bodies keep their original titles.

## What else moved

Three things that were not retired, but were in the wrong place:

- **[The skyline exposition](../2026-07-27-skyline-exposition/README.md)**, from
  `design/exposition/`. The directory moved as a unit, so every relative
  include, import, and compile-time data load still resolves; the move was
  verified by compiling the document.
- **[The literature review](../2026-07-24-literature-review/README.md)**, from
  `design/literature-review.md`.
- **[The perf-probe report](../2026-08-04-perf-probe/README.md)**, from
  `crates/before/examples/PROBE-REPORT.md` — a dated investigation report, and
  the only non-buildable file in a directory of cargo examples.

## Cross-references

The resurrected bodies cite each other by their old `design/…` paths, and cite
code and commits that have since moved. The table below resolves the paths. For
the commits: several documents pin their hashes to the archive branch
`wave1/integration` rather than to any branch that survives, and say so in their
own headers.

Documents outside these notes cited retired `design/` paths too, and were
repaired two different ways:

- **Re-aimed at these notes**: `AGENTS.md`, which is the file that tells a
  reader where to look; and `formal/MODEL.md`, `formal/PLAN.md`, and
  `formal/PROGRESS.md`, which are themselves design documents, and a design
  document may cite another.
- **Dissolved**: 31 citations across 14 Lean sources under `formal/lean/`.
  Code does not cite design documents — the constraint belongs at the code
  that needs it — so each citation was replaced by the thing it was pointing
  at. Most already stated their constraint inline and simply lost a
  parenthetical. Four were load-bearing, naming where an unproven premise was
  argued, and those arguments were brought into the Lean prose: the W = 1
  structural argument for byte-level soundness of one-reply slots now sits in
  `Mux/Basic.lean`'s module doc, which already declared itself that ruling's
  canonical home, and the Kahn coverage argument for the `d5` corner now sits
  in `Statement.lean` beside the claim it supports.

## The path map

| Retired path | Resurrected as |
| --- | --- |
| `design/b05-envelope-sim.py` | [`2026-07-22-uniformity-envelope/envelope-sim.py`](../2026-07-22-uniformity-envelope/envelope-sim.py) |
| `design/b05-uniformity-envelope.md` | [`2026-07-22-uniformity-envelope/uniformity-envelope.md`](../2026-07-22-uniformity-envelope/uniformity-envelope.md) |
| `design/before-adversarial-resource-amplification.md` | [`2026-07-22-before-adversarial-resource-amplification/before-adversarial-resource-amplification.md`](../2026-07-22-before-adversarial-resource-amplification/before-adversarial-resource-amplification.md) |
| `design/before-constants-frontier.md` | [`2026-07-27-before-constants-frontier/before-constants-frontier.md`](../2026-07-27-before-constants-frontier/before-constants-frontier.md) |
| `design/before-formal-tick.md` | [`2026-07-26-before-formal-tick/before-formal-tick.md`](../2026-07-26-before-formal-tick/before-formal-tick.md) |
| `design/before-fuelscape-rustdoc.md` | [`2026-08-13-before-fuelscape-rustdoc/before-fuelscape-rustdoc.md`](../2026-08-13-before-fuelscape-rustdoc/before-fuelscape-rustdoc.md) |
| `design/before-fuzzfit-asymptotics.md` | [`2026-07-26-before-fuzzfit-asymptotics/before-fuzzfit-asymptotics.md`](../2026-07-26-before-fuzzfit-asymptotics/before-fuzzfit-asymptotics.md) |
| `design/before-lowhang-sweep.md` | [`2026-07-18-before-lowhang-sweep/before-lowhang-sweep.md`](../2026-07-18-before-lowhang-sweep/before-lowhang-sweep.md) |
| `design/before-ownership-gated-walks.md` | [`2026-08-04-before-ownership-gated-walks/before-ownership-gated-walks.md`](../2026-08-04-before-ownership-gated-walks/before-ownership-gated-walks.md) |
| `design/before-skyline-encoding.md` | [`2026-07-23-before-skyline-encoding/before-skyline-encoding.md`](../2026-07-23-before-skyline-encoding/before-skyline-encoding.md) |
| `design/before-tick-cost-spec.md` | [`2026-07-25-before-tick-cost-spec/before-tick-cost-spec.md`](../2026-07-25-before-tick-cost-spec/before-tick-cost-spec.md) |
| `design/before-version-entropy.md` | [`2026-07-27-before-version-entropy/before-version-entropy.md`](../2026-07-27-before-version-entropy/before-version-entropy.md) |
| `design/eager-absorption.md` | [`2026-07-21-eager-absorption/eager-absorption.md`](../2026-07-21-eager-absorption/eager-absorption.md) |
| `design/fold-unification-survey.md` | [`2026-07-29-fold-unification-survey/fold-unification-survey.md`](../2026-07-29-fold-unification-survey/fold-unification-survey.md) |
| `design/materialized-backlogs.md` | [`2026-07-29-materialized-backlogs/materialized-backlogs.md`](../2026-07-29-materialized-backlogs/materialized-backlogs.md) |
| `design/mux-latency.md` | [`2026-07-21-mux-latency/mux-latency.md`](../2026-07-21-mux-latency/mux-latency.md) |
| `design/mux-latency/gen.py` | [`2026-07-21-mux-latency/gen.py`](../2026-07-21-mux-latency/gen.py) |
| `design/mux-latency/instances.py` | [`2026-07-21-mux-latency/instances.py`](../2026-07-21-mux-latency/instances.py) |
| `design/mux-latency/latency_results.json` | [`2026-07-21-mux-latency/latency_results.json`](../2026-07-21-mux-latency/latency_results.json) |
| `design/mux-latency/model.py` | [`2026-07-21-mux-latency/model.py`](../2026-07-21-mux-latency/model.py) |
| `design/mux-latency/mux.py` | [`2026-07-21-mux-latency/mux.py`](../2026-07-21-mux-latency/mux.py) |
| `design/mux-latency/run_latency.py` | [`2026-07-21-mux-latency/run_latency.py`](../2026-07-21-mux-latency/run_latency.py) |
| `design/mux-latency/timed.py` | [`2026-07-21-mux-latency/timed.py`](../2026-07-21-mux-latency/timed.py) |
| `design/node-hash-preimage.md` | [`2026-07-18-node-hash-preimage/node-hash-preimage.md`](../2026-07-18-node-hash-preimage/node-hash-preimage.md) |
| `design/opening-supply-symmetrization.md` | [`2026-07-23-opening-supply-symmetrization/opening-supply-symmetrization.md`](../2026-07-23-opening-supply-symmetrization/opening-supply-symmetrization.md) |
| `design/own-version-view.md` | [`2026-07-27-own-version-view/own-version-view.md`](../2026-07-27-own-version-view/own-version-view.md) |
| `design/parent-placement.md` | [`2026-07-18-parent-placement/parent-placement.md`](../2026-07-18-parent-placement/parent-placement.md) |
| `design/probe-ticks-68.md` | [`2026-07-27-fused-multi-tick-probe/probe-ticks.md`](../2026-07-27-fused-multi-tick-probe/probe-ticks.md) |
| `design/review-link-transport-branch.md` | [`2026-07-23-review-link-transport/review-link-transport-branch.md`](../2026-07-23-review-link-transport/review-link-transport-branch.md) |
| `design/review-link-transport-execution.md` | [`2026-07-23-review-link-transport/review-link-transport-execution.md`](../2026-07-23-review-link-transport/review-link-transport-execution.md) |
| `design/review-link-transport-retrospective.md` | [`2026-07-23-review-link-transport/review-link-transport-retrospective.md`](../2026-07-23-review-link-transport/review-link-transport-retrospective.md) |
| `design/review-packet-link-transport.md` | [`2026-07-18-review-packet-link-transport/review-packet-link-transport.md`](../2026-07-18-review-packet-link-transport/review-packet-link-transport.md) |
| `design/routed-link.md` | [`2026-07-29-routed-link/routed-link.md`](../2026-07-29-routed-link/routed-link.md) |
| `design/single-socket-plan.md` | [`2026-07-21-single-socket/single-socket-plan.md`](../2026-07-21-single-socket/single-socket-plan.md) |
| `design/single-socket-retrospective.md` | [`2026-07-21-single-socket/single-socket-retrospective.md`](../2026-07-21-single-socket/single-socket-retrospective.md) |
| `design/single-socket.md` | [`2026-07-21-single-socket/single-socket.md`](../2026-07-21-single-socket/single-socket.md) |
| `design/spine-wrap-hash.md` | [`2026-07-18-node-hash-preimage/spine-wrap-hash.md`](../2026-07-18-node-hash-preimage/spine-wrap-hash.md) |
| `design/streaming-latency-serialization.md` | [`2026-07-17-streaming-latency-serialization/streaming-latency-serialization.md`](../2026-07-17-streaming-latency-serialization/streaming-latency-serialization.md) |
| `design/streaming-wire-deadlock.md` | [`2026-07-17-streaming-wire-deadlock/streaming-wire-deadlock.md`](../2026-07-17-streaming-wire-deadlock/streaming-wire-deadlock.md) |
| `design/sync-budget.md` | [`2026-07-22-sync-budget/sync-budget.md`](../2026-07-22-sync-budget/sync-budget.md) |
| `design/tracecheck.md` | [`2026-07-21-tracecheck/tracecheck.md`](../2026-07-21-tracecheck/tracecheck.md) |
| `design/tracecheck/causal-reference.py` | [`2026-07-21-tracecheck/causal-reference.py`](../2026-07-21-tracecheck/causal-reference.py) |

## What stayed behind

`design/` is not empty. The documents still there describe work that is current,
and this migration did not touch them.
