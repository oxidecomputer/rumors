<!-- CAVEAT LECTOR: written by Claude (Opus 5.5) as the lane section for the L7 auditor, modeled on the L6 codec lane; under review by Finch before launch. -->

# Lane L7: `suanpan` and its uses in `before`

## Scope

This lane's theme is *exact arithmetic with deferred work*. `suanpan`'s
accumulator postpones carry and cancellation, so updates that oscillate across
a carry boundary stay cheap. That design makes the stored form depend on
history: many internal states represent the same integer, and some queries
mutate the state they inspect.

The theme extends to how `before` uses the accumulator. Many of `before`'s
algorithms (comparison, lattice operations, measurement, placement, range
minima, tick, validation) accumulate through it, and their own cost claims
rest on its bounds. Those bounds are stated in terms of the accumulator's
working width and are amortized over one accumulator's operation sequence.
A use can therefore be correct in value while still breaking the performance
contract it relies on, so this lane examines whether each use is both right
and inside the regime where the accumulator's guarantees apply.

Some things to consider, as starting points rather than a checklist. They
are prompts for your own exploration: pursue whatever else the code
suggests, and expect the most valuable questions to be ones not listed
here.

- Does every observation depend only on the mathematical value, never on the
  history that produced it?
- Do the amortized bounds hold against sequences engineered to defeat the
  amortization?
- Are the one-sided queries (`is_known_zero`, `cmp_zero_stable_under`)
  sound?
- Are conversions exact at every primitive boundary?
- Are digit positions safe from overflow on 32-bit targets?
- Does `before` rely on amortization across accumulators it creates and
  discards, where no amortization can occur?
- Does `before` call operations that cost time linear in the working width
  (cloning, reading a magnitude, normalizing, negating, converting) inside
  loops, where that width reflects accumulated history rather than the
  current value?
- Does each `before` cost claim survive when composed with the
  accumulator's bounds, including its logarithmic factors?
- Is the bridge between the accumulator and `num_bigint` exact in sign and
  magnitude?

The identifiers below are a starting map, not a boundary. Follow the theme
wherever the code takes it, including into code this list does not name.

- **Crate:** all of `crates/suanpan`. That covers `Accumulator` with every
  operation in its crate docs' table, and the `touch_meter` feature. Unless a
  path below names another crate, it is relative to `crates/suanpan`.
- **Uses in `before`:** find them by searching `crates/before/src` for
  `suanpan`. The bridge to `num_bigint` is `crates/before/src/accumulator.rs`.

Neighboring lanes explore adjacent themes. When your work crosses into one,
establish what you found and report it, rather than continuing the
investigation there. L4 explores the measures built on the accumulator, and
L2 and L3 the algorithms that call it. This lane examines whether those calls
respect the accumulator's contract; the algorithms' other behavior belongs to
those lanes.

## Contracts to read first

- The crate page: the cost table, its definitions of working width `W`,
  operand width `A`, and growth `G`, the amortization statement, and
  `normalize`'s `Q <= W + 32`.
- `cmp_zero_stable_under`: `Some(ordering)` means every adjustment within the
  bound leaves that comparison unchanged.
- `is_known_zero`: `true` proves zero.
- The primitive `TryFrom` conversions return the original accumulator on
  failure.
- The `# Complexity` sections of the `before` operations whose
  implementations accumulate through `suanpan`, and the module docs of
  `crates/before/src/accumulator.rs`.

## Prior coverage to map

- **Tests:** `src/accumulator/tests/` (differential, metered, primitives,
  representation, surface, witnesses), `src/accumulator/digits/tests.rs`,
  `tests/amortized_sequences.rs`, and the wasm32 `SuanpanLanding` check.
- **In `before`:** `crates/before/src/accumulator/tests.rs`, the touch column
  of the amplification board, and the touch-meter pins under
  `crates/before/tests/meter/`.
- **September partitions:** `suanpan.md`, `suanpan-tests.md`.

## Closed fixes to re-attack

- Digit positions computed without intermediate `usize` overflow, on 32-bit
  targets too.
- Fixed-width updates staying allocation-free.

## Candidate leads (evaluate, don't confirm)

1. **History independence.** Accumulators holding equal values reached by
   different operation histories, including interleaved `cmp_zero` and
   `normalize`, agree on every observation.
2. **Amortized bounds.** Build sequences oscillating across carry
   boundaries at several scales at once, and measure touch counts against
   the documented amortized bounds.
3. **`cmp_zero_stable_under` is sound.** Adjust by amounts exactly at the
   stated bound and just past it.
4. **Wide shifts and narrow targets.** Large shifts, and every conversion
   limit, on 64-bit and 32-bit targets.
5. **Uses that defeat amortization.** Look for `before` call sites that
   create a fresh accumulator per loop iteration, or perform
   width-linear operations on an accumulator whose working width has grown
   past its current value. Construct the input that makes such a site
   dominate its operation's cost, and measure it with the touch and heap
   meters.
