# Observations (lane L2)

These are documentation inaccuracies and design notes, for triage after the
audit. None is a contract violation by the code. Each says whether I verified
it or inferred it.

## 1. `Sum` and `FromIterator` state their cost in terms of a receiver they lack

`crates/before/src/version.rs:1298-1366`. All four impls (`Sum<Version>`,
`Sum<&Version>`, `FromIterator<Version>`, `FromIterator<&Version>`) reuse
`join_all`'s complexity text: the `not(doc)` string
"`O((|self| + |iter|) log k)` time", and the `version_join_all` fuelscape
island. The island's `contract` field reads the same,
`O((|self| + |iter|) log k)`; I verified this in
`crates/before/fuelscape/version_join_all.json`. These impls take no receiver,
so `|self|` names nothing at the reader's altitude. The fix is to restate the
bound as `O(|iter| log k)` for these impls. That needs either a separate
island or a contract string chosen per impl. The rendered fuelscape data is
committed, so this is not purely a prose edit. It is user-facing rustdoc,
so the owner rules.

## 2. The lattice module doc describes closures that do not exist

`crates/before/src/version/lattice.rs:24-25`: "each entry point passes its own
picking closure and the sweep never consults which operation it is running."
The code passes `Extreme` enum values to `Extreme::pick`, and it writes the
sweep twice (`hull_bits` and `Extreme::emit`); there is no shared sweep that a
closure parameterizes. This is maintainer-facing prose that disagrees with the
code. The simplification brief `simplification-lattice-one-sweep.md` makes the
structure match the doc. If that brief is not taken, restate the sentence in
terms of the two `Extreme`-parameterized loops.

## 3. `Version`'s order table conflates `<` and `<=`

`crates/before/src/version.rs:40-42` and the table row at line 50:
"`a < b` means every event in `a` is present in `b`", and the row
"`a < b`, `a <= b` | every event in `a` is present in `b`". That describes
`<=`. Strict `<` additionally requires `b` to hold an event that `a` lacks:
`a <= b` and `a != b`. A suggested restatement: "`a <= b` means every event in
`a` is present in `b`; `a < b` means that and more: `b` also records an event
`a` lacks." This is user-facing prose, so the owner rules.

## 4. "More efficient" on `join_all` and `meet_all` is a worst-case claim stated without qualification

`crates/before/src/version.rs:526-527` and `608-609`: "Prefer this to
iteratively `join`ing … as it is more efficient." The balanced fold costs
`O(D log k)`, against `O(k·D)` for a sequential fold in the worst case, so
the advice holds asymptotically in the worst case. It does not hold for every
input. If the receiver is a small version dominating large items, a
sequential fold costs `O(D)`: each join reads the small accumulator and one
item. The balanced fold combines the items among themselves first and pays
`O(D log k)`. This is inferred from the fold's structure in `src/fold.rs`; I
did not meter it. It is low priority; qualifying the sentence ("in the worst
case") would make it exact.

## 5. The board's depth disposition overstates what its cited test drives

`crates/before/src/testing/meter/board/coverage.rs:570-573`: "depth safety is
pinned by deep_tree_stack_safety, and every board family already scales
depth." The depth-100k tests do not drive the shape walks, the
concurrent-pair hull, or `Sum`/`FromIterator`, and the board's release-mode
families reach about 32,000 levels. See `machinery-deep-surfaces.md`, which
closes the gap. Once that test lands, the disposition is true as written for
these surfaces.

## 6. Constant factor, no action: the concurrent-pair hull walks twice

`Version::hull` (`crates/before/src/version.rs:940-971`) runs `partial_cmp`
until both directions are refuted, then a second, complete `hull_bits` sweep.
For concurrent pairs whose second refutation comes late, this reads the
operands nearly twice. The doc comment explains the trade: comparable pairs
share both input buffers and allocate nothing. Changing it would cost
allocation on comparable pairs, so I recommend leaving it. I did not meter
the factor.
