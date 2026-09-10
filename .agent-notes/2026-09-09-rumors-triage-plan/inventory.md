# Evidence for the proposed restart

Checked on 2026-09-09 against `main` at
`90e4509dc66330affb38afa74813a8dd500edd93`. These are inventory findings,
not a fresh correctness verdict on the crate or prepared branches.

## Sources and scope

Read the crate and relevant tree/transport module introductions, the
review overview, triage plan and workflow, the recorded rulings, the
handoff, status and merge queue, branch review summaries and outstanding
questions, and relevant briefs. Parsed all ledger rows and inspected
selected source and diffs to check the planning conclusions. Did not
repeat the original file-by-file review or rerun the branches' test suites.

Source records:

- [Original review](../2026-09-01-holistic-review-rumors/README.md)
- [Rulings](../2026-09-01-holistic-review-rumors/triage/rulings.md)
- [Ledger](../2026-09-01-holistic-review-rumors/triage/ledger.tsv)
- [Additional findings](../2026-09-01-holistic-review-rumors/triage/new-findings.md)
- [Handoff](../2026-09-01-holistic-review-rumors/triage/HANDOFF.md)
- [Shared status](../STATUS.md) and [merge queue](../merge-queue.md)

## The ledger is a record of decisions, not a reliable progress display

Ledger rows have ruling references. Their categories are not independent
tasks or reliable estimates of effort.
The original severity labels also do not cover every subsequently found
failure.

The checker validates row structure and presence of receipts, but does
not check Git ancestry or the asserted fix. Swarm entries cite
`2cb24d45`, which is not an ancestor of current `main`; the deletion is
present in main's history at `6e5eb555`. Reconcile those receipts in the checklist, rather than treating the
entries as missing work.

The status page's top summary says there are no ready Rumors packets;
its detailed rows and the final handoff list ready work. Several older
summaries still describe resolved questions as open. Use current code,
later rulings, and exact branch tips to reconstruct the next item.

## Prepared work that remains available

The listed worktrees were clean when inspected.

| Branch under `triage/` | Tip | Declared base |
|---|---|---|
| `p2-commit-path` | `afc7da22` | `1336daa0` |
| `p2-link` | `95c429e2` | `7aa2b9a1` |
| `p1-collision-mode` | `f6d9a0af` | `d0dcb9f5` |
| `p2-deep-geometry` | `f74324a7` | `7858b35e` |
| `p2-vanish-liveness` | `c9e7465b` | `9c8ce16c` |
| `p1-envelope` | `10f4bb6f` | `0fad870c` |
| `p1-proptest-ci` | `a34859ed` | `a07827ed` |
| `p1-generators` | `f21541da` | `a34859ed` |

The deep-geometry branch starts from an earlier collision-mode tip with
31-byte shared prefixes. The later collision branch uses 28-byte prefixes
and a separate deeper sweep. Integrating those branches is more than
accepting conflict markers. Deep-geometry and vanish-liveness also change
the same protocol files, including `remote/proxy/work.rs`, with
different terminal error precedence.

The old records report numerous successful verification rounds, but also
report that temporary raw logs were deleted by system cleanup. Committed
summaries remain. Recheck the retained implementation; do not reconstruct
or rerun every historical review round.

## Concrete complexity to leave behind

- The commit-path packet embeds an earlier version of itself. Its opening goal
  still describes the discarded sink design before appending the newer
  design. A review of the final code needs neither duplication.
- In the envelope branch's `tests/protocol_overhead.rs`,
  `crate_doc_text` and `operating_envelope_figures_are_one_line` read
  `src/lib.rs` and require exact documentation phrases. Formatting helpers
  exist to reproduce commas and superscripts in those phrases. Retain
  useful numerical evidence independently of the prose.
- In the same file, `every_cell_is_measured_by_its_own_test` reads its own
  source to find matching macro invocations for a separate cell table.
  Generate or iterate the cases from one definition if the grid is kept.
- The commit-path branch introduces an allocation-count binary
  alongside the ownership fix. Judge whether an existing benchmark answers
  the performance question; repairing destructor safety does not require
  preserving this instrument.
- The generator branch substantially expands `tools/caselint` and adds a
  rejection tripwire binary and global configuration. Direct generator
  improvements can be assessed separately from this enforcement system.

## Questions to resolve with the affected code

These need focused investigation or a concrete recommendation, not a
new round of triage for every finding:

- Commit ownership: ensure destructors can read and write the replica
  after all relevant locks are released; assess the optimistic fallback's
  actual progress guarantee. Do not automatically retain tests coupled to
  its retry count or internal representation.
- Memory census: the commit-path branch changes the formula to
  `peak - before`. Establish the useful resource claim before deciding
  whether to retain the measurement or its ceiling.
- Deep protocol: reproduce the reported duplicated-reply stall, the
  deep-geometry conformance-floor failure, and residual error-precedence
  cases on the combined implementation. Some packet recommendations
  propose exceptions or follow-ups; those are not completed fixes.
- Pooling: T167 requires consumer evidence for the changed interface.
  Current Sush compatibility work follows the standing workflow in the
  [plan](README.md); the old report's draft patch is only a reference.
- Existing safeguards: current AGENTS.md's trust model and seed-retention
  rules govern the work. Older suggestions about adversary economics or
  deleting regression seeds must not be followed mechanically.

The follow-up [checklist](checklist.md) assigns the original findings,
rulings, and later reports. A one-off independent comparison
against the source reports found no missing or multiply assigned finding
IDs. No implementation, old ruling, ledger disposition, branch, or snapshot
was changed during this coverage pass. Notes-only edits needed no gate run.
