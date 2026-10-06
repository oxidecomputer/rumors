<!-- CAVEAT LECTOR: written by Claude (Opus 5.5) as the lane section for the L3 auditor, modeled on the L6 codec lane; under review by Finch before launch. -->

# Lane L3: events (tick and its callers)

## Scope

This lane's theme is *events*: how history grows. Tick is the only operation
that adds events. It is the paper's fill-then-grow: inflate the version within
the party's region as cheaply as the paper's cost function allows. Every
`Clock` wrapper composes it with join. The theme also covers the inverse
question, posed by `min_ticks`: what is the fewest events that could have
produced a given version?

Some things to consider, as starting points rather than a checklist. They
are prompts for your own exploration: pursue whatever else the code
suggests, and expect the most valuable questions to be ones not listed
here.

- Does tick produce exactly the paper's result? The paper specifies it
  deterministically, so this is an exact comparison, not a family of
  acceptable answers.
- Do strict domination and region-locality hold on every canonical
  `(version, party)` pair, including pairs no history produces?
- Is `ticks(k)` equivalent to `k` ticks for every `k`, including counts beyond
  the machine word?
- Is each `Clock` wrapper exactly its documented composition?
- Is `min_ticks` a true floor over every history, and is it tight?

Tick has the crate's most intricate implementation (routing, probing,
prescanning, raising, memoization), so its time and memory bounds belong to
this theme as much as its answers do.

The identifiers below are a starting map, not a boundary. Follow the theme
wherever the code takes it, including into code this list does not name.

- **Entry points:** `Version::tick`, `Version::ticks`, `Party::tick`,
  `Party::ticks`, `Clock::tick`, `Clock::ticks`, `Clock::send`,
  `Clock::recv`, `Clock::recv_all`, `Clock::absorb`, `Clock::absorb_all`,
  `Version::min_ticks`.
- **Implementation:** `src/version/tick/`, `src/version/measure/min_ticks/`.

Neighboring lanes explore adjacent themes. When your work crosses into one,
establish what you found and report it, rather than continuing the
investigation there. L2 explores the join beneath the wrappers, and L1 the
parties that tick. L7 examines whether this lane's calls into the `suanpan`
accumulator respect its contract.

## Contracts to read first

- `Version::tick`: the result strictly dominates; history changes only within
  `party`'s region; projecting onto a disjoint party is unchanged.
- `Version::ticks`: "identical to `k` sequential ticks".
- `Version::min_ticks`: an exact floor over every history that could produce
  the version.
- The crate page's description of send, recv, and absorb in the two usage
  disciplines.

## Prior coverage to map

- **Tests:** `src/version/tick/tests.rs`, `tick/raise/tests.rs`,
  `src/version/measure/tests.rs` and the `min_ticks` subtrees,
  `tests/meter/tick_counts.rs`, `src/testing/grow_brute_force.rs`, and the
  tree oracle's event implementation.
- **September partitions:** `skyline-fill-grow.md`, `skyline-watermark.md`.
- **The paper:** `crates/before/reference/itc2008.md`, sections on event,
  fill, and grow.

## Closed fixes to re-attack

- Tick's memo and suspended-level storage bounded by a small multiple of the
  input.
- `min_ticks` held to the transient-heap ceiling.
- Fixed `u32` caps removed from the fill bookkeeping.

## Candidate leads (evaluate, don't confirm)

1. **`ticks(k)` equals `k` sequential ticks exactly**, for arbitrary
   canonical `(version, party)` pairs, including pairs no history reaches. For
   counts too large to iterate, check `ticks(a)` then `ticks(b)` against
   `ticks(a + b)`.
2. **Paper fidelity.** Tick's output equals the tree oracle's fill-then-grow
   result exactly, including where grow's cost function ties.
3. **`min_ticks` is a floor, and tight.** Every generated history performs
   at least `min_ticks` ticks. Some history achieves exactly that many:
   construct it, or compare against a brute-force search on small versions.
4. **The `Clock` wrappers.** `send`, `recv`, `recv_all`, `absorb`, and
   `absorb_all` equal their documented compositions of tick and join.
