<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as the before triage's hand-off to the rumors triage (ruling 92 in rulings.md); the observations are the before review's dependence sweep's and are unverified here. Read with the ground rules in ../../README.md. -->

# Rows carried to the rumors ledger

Seed these into `../../2026-09-01-holistic-review-rumors/triage/ledger.tsv`
under a note citing this review (`.agent-notes/2026-09-01-holistic-review-before`,
dependence document, open questions 6, 7, and 11). Each is a hypothesis
for the rumors triage to verify, not a finding of record; severity is
unassigned here.

| proposed id | where (as the sweep recorded it) | observation | note |
|---|---|---|---|
| before-carried-1 | `src/gossip.rs:1395` (`PartyGuard::drop`) | Recovers a speculative fork with only a `debug_assert!(false)`; in release the fork's share is lost silently. The rumors inventory sweep already names this guard; merge with that row. | dependence open question 11 |
| before-carried-2 | `parse_record` | Accepts the version atom's CBOR byte-string head without spelling judgment (no canonical-head check before decode). | dependence open question 11 |
| before-carried-3 | `src/tree.rs:83-89` | The collision premise also assumes SHA3-256 path collision freedom; the stated premise names only the leaf hash. | dependence open question 11 |
| before-carried-4 (owner item) | `meter::span_traffic` | Record one reading of the span ladder's rung mix, so rumors' pricing of spans rests on a committed number. | dependence open question 6 |
| before-carried-5 (owner item) | fuzz-fit small-band roster | Add `Party::fork` to the roster, or state at the roster why one fork per bootstrap is not hot. | dependence open question 7 |

Also for the rumors plan: the before dependence document's table *Costs
rumors relies on* is the input to the rumors performance phase, and
before's P1 and P2 land first (before ruling 5). Under before ruling 87,
rumors' public rustdoc never cites a before law name; any such citation
found is a rumors-ledger finding.
