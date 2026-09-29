<!-- CAVEAT LECTOR: written by Codex for owner review. Verify against the
current implementation and measurement before treating it as documentation. -->

# Evidence for the `before` crate-page performance claims

This note records what the contract audit and the representation-space
measurement establish. It does not propose wording for `src/lib.rs`.

## Representation space

`tests/representation_space.rs` measures the complete live representation: the
inline size of a `Clock` plus the live heap it owns after construction
temporaries have been dropped. The comparison uses the current recursive oracle,
including its `Arc` sharing and `BigUint` representation, rather than estimating
a hypothetical implementation.

The favorable example starts with 512 balanced parties. Party `i` records
`popcount(i) + 1` events, so all counts are at most ten, and the parties are
then reunited. Production and oracle clocks execute the same history. On the
current 64-bit target, the final production clock occupies 433 bytes and the
oracle clock 57,288 bytes: 132.3× as much. The test asserts the weaker claim
that the oracle occupies at least 100× as much.

The ratio is not universal. The comparison caps event counts at `2^64` and
also exercises balanced histories whose leaf values alternate between zero and
that cap. At a fixed leaf count, every full recursive tree has the same node
count and number of child allocations regardless of branching shape. The
production form also spends one topology bit per node, but its payload codes
differences between adjacent leaves. Alternating the two extremes makes every
difference as wide as the cap permits without increasing the recursive tree's
node count. This is therefore the least favorable evident large-tree family
under the chosen count bound.

The measured oracle/production ratios for 1 through 128 leaves are 0.53, 1.39,
3.01, 3.88, 5.07, 5.96, 6.53, and 6.85. The test continues through 256 and 512
leaves and asserts that the ratio remains below 8×. At one leaf, the recursive
oracle is smaller: there is no recursive structure to eliminate, and the
production gamma code is wider than the oracle's inline integer.

Consequences for the crate page:

- “Approximately 100×” is supported as a scoped example of a balanced,
  structurally rich history with modest event counts.
- It is not a lower bound over all valid clocks, even when event counts do not
  exceed `2^64`.
- Branching shape is immaterial to an unshared recursive tree once node count
  and integer storage are fixed. It is not immaterial to the production payload
  unless the adjacent leaf-value sequence is also fixed.
- The thresholds are allocator- and integer-backend observations, not a
  representation theorem. The test deliberately uses broad inequalities.

## The space-consumption figure

`results/space_consumption/space.csv` and its figure reproduce the paper's two
workloads using this crate's canonical bytes. They measure the mean size of a
whole `Clock`, not resident memory and not the `Party` and `Version` components
separately. The committed data uses populations 4, 8, 16, 32, 64, and 128; 100
runs; 100,000 dynamic iterations; and 25,000 static iterations.

The existing prose's 100-party component estimates and one-million-event
description are approximate readings or extrapolations, not literal rows in the
committed CSV. They may remain useful, but the figure itself directly supports
only the workload and totals it actually measures.

## Time and transient-space headlines

The crate-wide “asymptotically linear” wording cannot describe every public
operation. Among the documented exceptions are multiplication-sensitive rank
arithmetic, balanced folds with logarithmic factors, query work that depends on
the number of bounds, and projections whose required output may be quadratic in
the input.

Likewise, there is no single input-only auxiliary-space bound for the whole
crate. Individual contracts may depend on required output or query size. The
method-level complexity sections are the contracts; a crate-page summary should
defer to them rather than flattening them into one universal claim.
