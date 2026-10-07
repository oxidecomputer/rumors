# Lane L1 observations

Items here are not contract breaches. Each says whether it was verified or
inferred.

## O1. A party taken from a decoded clock keeps the whole clock buffer (verified)

`Clock::decode_bytes` (`crates/before/src/clock.rs`, "shares its storage
between the party and version") adopts the party as a `Bytes` slice of the
clock's buffer (`Party::decode_prefix`, `src/party/io.rs:26-39`). After
`into_parts`, a long-lived party keeps the version's bytes alive once the
version is dropped. Measured (`retention-6.log`, probe
`tests/audit_l1_retention.rs` on the explore branch): a one-byte seed party
retains 64 B, 401 B, 3,776 B, and 37,526 B for versions of 39 B, 376 B,
3,751 B, and 37,501 B. Retention stays within the input size, so no resource
claim is breached. Copying the party's bytes at decode (they are typically a
few bytes, and validation already reads them) would decouple the two.
Classification: design trade, for triage. It reverses a stated design choice,
so I did not brief it.

## O2. `Party::forks` docs call an additive term a "factor" (verified by reading)

`src/party.rs:305-308`: "every resultant Party produced here increases in
size by only a logarithmic factor." Each share is `O(|self| + log k)` bits:
the balanced plan adds `O(log k)` levels, an additive term. Suggested:
"increases in size by only `O(log k)` bits". Doc correction toward the code.

## O3. The per-step cost names a quantity that changes during iteration (verified by reading)

`PartyForks` docs (`src/party/forks.rs:359-361`): "Each `next` costs
`O(|p| + log k)` ... with `|p|` the borrowed party's size." The borrowed party
is rebuilt after every step. The bound holds when `|p|` is the size at
construction: the residual stays within `|p| + O(log k)`, since it differs
from the original only along the two boundary paths of the taken range
(inferred from `remove.rs`; consistent with the board's `party_forks_full`
work model). Suggested: say "the party's size when the iterator was created".

## O4. Size-hint wording for wide counts understates the behavior (verified)

`PartyForks` and `ClockForks` docs: "Wider counts report a sound lower bound
and no upper bound." Counts up to `2 * usize::MAX` become exact once the
remainder reaches `usize::MAX` (pinned by `adjacent_wide_count_becomes_exact`
and the wasm32 `forks_accept_the_first_count_past_usize` pin). Superseded by
D1 if the owner rules for exact hints; otherwise the wording should say which
wide counts become exact.

## O5. `fold::balanced_try_fold` stores its level counter as `usize` (verified by reading)

`src/fold.rs:23-26`: `weight` counts merge levels (at most about 64 for any
input count, including iterators longer than memory, such as `join_all` over
a huge `forks` count). Behavior is identical on every target; under the
`usize`-invariance lens, a fixed-width `u32` would state that the quantity is
not a memory count. Nit; simplification if anyone touches the fold.

## O6. The fork iterators are fused but do not say so (verified by reading)

`PartyForks::next` and `ClockForks::next` return `None` forever after
exhaustion (`Plan::is_empty` stays true), but neither implements
`FusedIterator`, while `Regions`, `Plateaus`, `Overlay`, and `Cells` do.
Adding the impls is a public API addition, so: design proposal.

**Owner's ruling.** Implement `FusedIterator` for the fork iterators; the
docs branch carries it.

## O7. `join_all` and `sync_all` complexity terms lack their derivation (verified by reading)

`Party::join_all`, `Clock::join_all`, and `Clock::sync_all` document
`O((|self| + |iter|) log k + (|self| + |iter|) log |self|)`. My reading of the
code bounds the work by the first term alone (at most `log k + 1` groups, each
joined once into a receiver of at most `|self| + |iter|`). The
`log |self|` term's source is not stated beside the claim. Per the crate's own
writing rule ("the argument lives where the claim lives"), either derive it at
the doc or drop it. Note also that `log k` should read as `max(1, log k)`,
since a single operand still costs a join.

## O8. Committed generators stop at depth 4 (verified, measured)

From `stats-committed.log` (explore probe `explore_committed_generator_stats`)
against `stats-1.log` (my `generator_stats`):

| | committed arbitrary | committed organic | explore `arb_set` |
|---|---|---|---|
| tree depth | at most 4 | at most 8 | 48% above 32, up to about 120 |
| encoded bytes | at most 4 | at most 3 | up to 128 |
| two-child branches | at most 4 | none in 93% | 32 or more in 42% |
| longest unary chain | at most 4 | (not measured) | up to 128 |
| arbitrary pairs disjoint / nested | 14% / 49% | (all disjoint) | 15% / 51% |
| `join_all` families accepted | 5.5% | | always, by construction (`arb_disjoint`) |

Every one of my calibration mutants that lives in ordinary walk code was also
caught by the committed suite (`calib-run-8.log`), so I have no constructible
failure that only the deeper generators catch, and I do not brief them. Their
reach is recorded for the instrument triage.

## O9. Retention in version writers (inferred; for the algebra and measures lanes)

S1's mechanism (`BitsWriter::finalize` keeps reserved capacity) is shared by
the version writers. Any version operation that reserves by input size and
then collapses likely retains the same way. Not measured.
