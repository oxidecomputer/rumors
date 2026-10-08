# 5. Lessons

Each lesson below is stated for `min_ticks`, then in the general form a
future design can be checked against.

## 5.1 A fixed-width field creates a second regime the board cannot reach

A packed representation with an exact fallback has two cost paths: values
that fit the field, and values that spill. Both give the right answer, so
value tests pass on both. The cost contract has to hold on both as well,
and the board judges cost only on the inputs it samples. Its samples are
small: the largest `jump-rising-spine` sample is about 40 KB (measured:
`parent-board.log`). A field whose capacity is crossed only by inputs
larger than that has a second regime the board never runs, and a board on
which every cell passes then certifies only the first regime.

The fields `min_ticks` has used show the pattern (code, and section 4):

- `main`'s `i32` offset field is crossed at small sizes, by any rising
  spine entered by a jump between `2^31` and `2^288`. The board misses it
  only because it lacks that family.
- Design A removed that crossing by re-anchoring, which creates one prefix
  per crossing. The 23-bit prefix field is then crossed only past `2^23`
  prefixes, at least about 66 MB of input, where the spill regime returned
  at 67 B/B while the board passed every cell.
- `main`'s own 23-bit prefix field is crossed only past `2^23` width
  freezes. Each width freeze needs a live change at least `2^256` wide,
  about 513 input bits of its own (`redesign.md`, section 13), so the
  crossing needs over 500 MB of input. It leads to the same spill path
  that the offset field reaches at small sizes, so it adds no new cost
  path, but it is a threshold the board cannot reach. The instrument
  survey states that no unreached threshold is known at `main`; this one
  appears to qualify (an inference from the code, not measured).
- The 8-bit close count spills past 255 closes on one minimum. Nothing in
  the records says whether a board family exercises that spill at a heap
  density that matters; I did not check.

A test-only narrower bound, as design A used, makes the far regime's
*values* testable at small sizes, but not its *costs*, because the board
measures production code.

In general: for each fixed-width field, name the smallest input that
crosses it and the per-element cost after crossing. If the board does not
sample such an input, make the field reachable at board sizes or remove
it. The owner's ruling states the requirement as "no field has a capacity
the board cannot reach". The audit's notes propose listing every such
threshold in `before` and `suanpan` as a separate task (reported:
`STATE.md`); nobody has done that enumeration.

## 5.2 Storing a difference against a neighbour turns an O(1) move into O(width) work

On `main`, `RangeMinima` moves boundaries and payloads and never reads them
on a push or pop, and a suspended contribution is an opaque word moved by
value (section 2). A move costs O(1) whatever the width of the value it
moves. A wide value is read when a drop crosses it, and that read
destroys it.

A difference encoding needs both operands whenever it encodes or decodes.
If one neighbour is wide and long-lived, and the slot next to it is
filled and emptied repeatedly, every cycle reads the wide neighbour. The
input paid for that width once, so the total work is (width) × (cycles),
which can be quadratic in the input. Both quadratic designs are instances:

- Design B derived a resumed minimum's height from the boundary popped to
  reach it. A boundary is itself a stored difference between adjacent
  minima, and `reveal-comb` creates and pops the same width-`b` boundary at
  each of `k` sites: `k · b` work against about `k + b` input.
- Design D encoded each record as a difference from its outer neighbour,
  so every push and pop above a wide outer record subtracted at its full
  width: about `2k²` limbs for a `k`-tooth comb.

Both designs had a space argument that was correct and a time argument
that borrowed it. Records' stretches of input are disjoint along the stack
*at one moment*, which bounds stored bits. Over the whole run, the same
stretch is charged again each time a new record occupies the slot above
the same wide neighbour. A time argument must charge each unit of work to
input that the work consumes, or to a value it destroys, as `RangeMinima`'s
crossing does; a value that survives being read can be read again for free.

Design C1 is consistent with this lesson (reasoned, not measured by any
time meter): re-anchoring keeps every offset within `i32`, so every
difference it stores is at most 33 bits and the wide part lives in a
frozen component read once at settlement. It was stopped for a different
reason (section 4).

In general: for every operation that reads a stored wide value, ask
whether the value survives the read. If it does, construct an input that
reads it again without new input, by reading the implementation and
building the cycle, rather than arguing it cannot happen. That is how both
quadratic cases were found.

## 5.3 The board needs a time meter over `num-bigint` work

The board measures time in two currencies: touch, which counts `suanpan`
accumulator digit work, and scan, which counts encoded bits read (code:
[`currency.rs`](../../crates/before/src/testing/meter/board/currency.rs#L9-L16),
[`accumulator.rs`](../../crates/suanpan/src/accumulator.rs#L42-L49)).
`min_ticks` also does `num-bigint` arithmetic: on `main`, `add_leaf`'s
offset copy, the spill store's clone, and the multiplications at
settlement; on design D, every record push and pop. None of it is counted.
A redesign that moves work from `suanpan` into `num-bigint` therefore
looks free on the board, which is what happened to design D: every touch
and scan reading equalled `main`'s while the limb work grew as `k²`.

Until such a meter exists, no claim that a `min_ticks` design is linear can
be checked by the board, including the claim for `main` (section 2). The
instrument survey proposes two forms (reported: `instrument-survey.md`,
section 1.1):

- the reviewer's two comb shapes as `min_ticks` families on the wasm fuel
  ladder of the fuzz-fit harness, which counts every executed instruction
  and so needs no hook at any call site; or
- a limb counter charged at each `num-bigint` call site, as the reviewer's
  probe did, which misses any call site nobody hooks.

Whichever is built, it should come with a committed demonstration that
design D fails it and `main` passes, so that the instrument is known to
detect the failure it exists for.

## 5.4 A fitted exponent cannot tell a smaller fixed cost from growth

Designs C1 and C2 were stopped by the rule that no growth exponent may
rise. For heap, the rule is unreliable in one direction: if heap is about
`a + b·n` and a change shrinks `a`, the two-point log-log slope rises
toward 1 with no change in asymptotics. The board renders exponents to
0.01 and densities to 0.1, so a comparison of rendered text cannot
separate the cases (reported: `instrument-survey.md`, section 1.7, which
cites C2 and an unrelated fix as the two times this stopped work). A
proposal for exact capture and comparison of board runs, classifying each
difference as constant or growing, exists as question 95 (reported:
`QUESTIONS.md`). For touch, C1's rises were attributed to an added linear
term on families that were sublinear, which is growth on those families
and not a fixed shift (reported: the fixer's attribution).
