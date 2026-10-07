<!-- CAVEAT LECTOR: the round-1 report of the L1 (identity) auditor, Claude Opus 5.5, condensed by the coordinator, who verified the explore branch's signatures and read D1 and MB3 in full; other results are the auditor's own claims. -->

# Lane L1, round 1: report

The explore branch is `explore/l1-identity` at `26cbae22`, with production
code unchanged from `58285ca5`. All its commits are signed. The sibling files
in this directory are the auditor's deliverables, including a full
instrument inventory ([`inventory.md`](inventory.md)).

## Verdict

- One low-severity defect, two questions, three proven test gaps, and no
  behavioral defect. The lane is at diminishing returns.
- A black-box suite checked identity against an interval-set model that
  shares no code with production or the tree oracle. It passed at 100,000
  cases, and again at 20,000 with fresh seeds.
- Leads 2 to 5 hold. Lead 1 is disputed, which produced D1.
- Every fork-step and full-drain cost ratio is flat across four sizes.

## Defect and questions

- **D1 (low):** [`D1-fork-size-hint-width.md`](D1-fork-size-hint-width.md).
  The fork iterators' `size_hint` depends on `usize` width, because the
  initial count is classified against `usize::MAX`. The current rustdoc
  allows this, so the fix waits on question 9 in `QUESTIONS.md`.
- **`Clock::from_parts`:** pairing a live party with an older version
  re-issues a stamp. The model accepts this; whether to document it at
  `from_parts` is question 10.

## Briefs

Each of MB1 to MB3 rests on a mutant the committed suite misses.

- **MB1:** [`MB1-join-all-multiplicity.md`](MB1-join-all-multiplicity.md).
  `join_all` must return each rejected region exactly once. Mutants M19 and
  M20 return a group twice and pass all committed tests, because every
  conservation check compares unions.
- **MB2:** [`MB2-deep-identity-probe.md`](MB2-deep-identity-probe.md). The
  fork iterators' walks have no deep coverage. Mutant M26, a recursive
  removal descent, passes the committed suite.
- **MB3:** [`MB3-opt-level-0-stack-safety.md`](MB3-opt-level-0-stack-safety.md).
  The workspace builds `before` at `opt-level = 2` even in dev, and LLVM
  removes self tail calls there. So no committed test proves stack safety
  for a downstream crate building at `opt-level = 0`. Mutant M25, a
  tail-recursive descent, passes everything at the workspace profile and
  overflows at `opt-level = 0`. The brief adds a gate recipe that runs the
  deep tests at `opt-level = 0`.
- **S1, a constant factor:**
  [`S1-result-buffer-retention.md`](S1-result-buffer-retention.md).
  `BitsWriter::finalize` keeps its reserved capacity. One-byte results of
  `join`, `without`, and `Clock::join` retain about one byte per input byte.
- **SB1, a simplification:**
  [`SB1-sync-all-merge-via-join-all.md`](SB1-sync-all-merge-via-join-all.md).
  `sync_all` repeats `Clock::join_all`'s fold.

## Observations

These are in [`observations.md`](observations.md):

- **O1:** a party taken from a decoded clock keeps the whole buffer alive.
- **O2 to O4:** precision of the fork-cost and size-hint docs.
- **O5:** `fold.rs`'s `weight` is a `usize` level counter.
- **O6:** the fork iterators are fused but don't implement `FusedIterator`.
- **O7:** the `log |self|` term in the `join_all` and `sync_all` complexity
  has no stated derivation.
- **O8:** the committed party generators stop at depth 4.
- **O9:** version writers likely share S1's retention.

## Coverage

See [`coverage.md`](coverage.md) and [`NOTES.md`](NOTES.md).

## Owner's ruling

On `Clock::from_parts`: yes. Its rustdoc states that pairing a party with a
version older than its latest tick reproduces stamps the party already
issued, linking the crate page's model.
