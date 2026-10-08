# Constant-factor brief S1: party results retain at most twice their size

## What happens now (verified)

A finished party keeps whatever capacity its writer reserved, because
`BitsWriter::finalize` (`crates/before/src/bits/writer.rs:300-303`) hands the
`Vec` to `Bytes::from` without shrinking it. Three writers reserve far more
than a collapsing result needs:

- `PartyWriter::for_join` (`src/party/io/writer.rs:80-83`) reserves
  `|self| + |other|` bits for every `join`, and so for `Clock::join`,
  `join_all`, and `sync`'s internal joins.
- `PartyRegionWriter::for_difference` (`src/party/io/writer.rs:273-276`)
  reserves `|self| + |other|` bits for `without`.
- `Removal::run` (`src/party/fork/remove.rs:165`) sizes the residual of
  `Party::forks` like the *share* it removes (`src/party/forks.rs:396-402`),
  so a small residual beside a large share keeps the share's size.

`Party::fork` is the exception: `fork_tree` sizes each half exactly
(`src/party/fork.rs:98-103`, "Sizing each result here prevents a small half
from retaining storage proportional to the source"), and
`asymmetric_fork_sizes_the_small_result_independently`
(`src/party/tests.rs:134-147`) holds it there.

Measured live heap held by the result after its inputs are dropped (probe
`crates/before/tests/audit_l1_retention.rs` on `explore/l1-identity`, counting
allocator `peak_alloc`, log `retention-2.log`; every result is one byte):

| depth `d` | join comb+cell | without (half+cell) minus cell | forks(1) residual | fork (control) |
|---:|---:|---:|---:|---:|
| 100 | 100 B | 75 B | 50 B | 1 B |
| 1,000 | 775 B | 525 B | 275 B | 1 B |
| 10,000 | 7,525 B | 5,025 B | 2,525 B | 1 B |
| 100,000 | 75,025 B | 50,025 B | 25,025 B | 1 B |

Denominated per output byte, retention is unbounded; per input byte it is
about one, so no documented resource bound is breached. The crate's own stance
for `fork` is what these results fall short of.

## Proposed structure

Seal every party through one rule: in `PartyWriter::finish`
(`src/party/io/writer.rs:233-237`), before `finalize`, release the buffer's
slack when its capacity exceeds twice its length (a small `BitsWriter` helper
around `Vec::shrink_to_fit`). `PartyRegionWriter::finish` and `Removal` both
finish through `PartyWriter::finish`, so the one site covers `join`,
`without`, and the forks residual. Exact-sized writers (`fork_tree`) never
take the branch. State the invariant on `PartyWriter::finish`: a finished
party retains at most twice its encoded size.

## Why the result is more obviously correct

The guarantee moves from per-call-site sizing discipline (one comment in
`fork_tree`) to the single place every party is sealed, so a new operation
cannot forget it.

## Tests

Extend the pattern of `asymmetric_fork_sizes_the_small_result_independently`
(which reads `Bits::allocation_capacity`, test-only) to the three cases in the
table: a deep comb joined with its complementary cell, `(half ∪ cell) \ cell`,
and the `forks(1)` residual of `[0, 1/4) ∪ deep cell`. Each asserts the
result's allocation capacity is at most twice its byte length. Better still, a
property over arbitrary pairs (`join` when disjoint, `without`) asserting the
same bound.

## Risk the builder must check

A shrinking `realloc` may move the buffer, adding up to half the old capacity
to the operation's peak heap. Run the board (`just board` legs in the gate)
and compare the `party_join`, `party_without`, `party_forks`,
`party_forks_full`, `party_join_all`, and clock rows against `baseline.md`.
Any ceiling that would rise, or any worst-case ranking flip, stops the work
for the owner per the pinned-instrument rules.

## Variant for other lanes

Version writers finalize through the same `BitsWriter::finalize`; any that
reserve by input size and then collapse (for example, joins of versions)
likely share this. That belongs to the algebra and measures lanes.

## Owner's ruling (question 73)

Option 1: adopt variant B (release a writer's slack when a result is
sealed) as a declared trade. The 13 worst-case rankings it moves are
re-pinned with their movement recorded, and the branch goes to review. The
cost, a transient heap rise of at most about one result's size on the
measured shapes, is stated in the commit message. The variant is saved in
`S1-retention-variants/variant-b-header.patch`.
