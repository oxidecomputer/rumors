# 2. How `main` computes it

`main` evaluates `Σ leaf heights − Σ internal-node subtree minima` in one
pass over the leaves, holding no tree and no absolute height per leaf. The
fold is short
([`Count::min_ticks_for`](../../crates/before/src/version/measure/min_ticks.rs#L28-L90));
the work happens in three structures, each answering one question.

| Structure | Question it answers | Where |
|---|---|---|
| `HeightPrefixes` | How do we add up leaf heights without copying a wide absolute height at every leaf? | [`heights.rs`](../../crates/before/src/version/measure/min_ticks/minima/heights.rs) |
| `RangeMinima` | What is the minimum of every open subtree, without storing a wide absolute minimum per subtree? | [`range_minima.rs`](../../crates/before/src/version/range_minima.rs) and its submodules |
| `ContributionStore` | Which leaf supplied each minimum, and how many closed subtrees use it? | [`contributions.rs`](../../crates/before/src/version/measure/min_ticks/minima/contributions.rs) |

Everything below is (code: `main` at `e7e107a2`) unless labelled otherwise.

## The fold, leaf by leaf

For each leaf after the first, the fold
([`min_ticks.rs`](../../crates/before/src/version/measure/min_ticks.rs#L46-L76)):

1. reads the leaf's height change `δ` and the depth at which the walk
   turned;
2. adds `δ` to the running height in both trackers;
3. closes every subtree the walk left, innermost first, and opens every
   subtree it entered;
4. applies the freeze test described below;
5. adds the leaf's height to the answer and lets the leaf update the
   minima of every open subtree.

At the end it closes the remaining subtrees and settles the frozen
prefixes (lines 78 to 89).

## Frozen height prefixes: summing heights cheaply

The running height is written `F + L`. `L`, the *live change*, is one
`suanpan::Accumulator` holding the height changes folded since the last
freeze. `F` is a sum of *frozen components*: entry 0 is the first leaf's
absolute height, and each later entry is a live change that was frozen.
The *prefix* `p` means the sum of components `0..=p`
([`HeightPrefixes`](../../crates/before/src/version/measure/min_ticks/minima/heights.rs#L37-L43)).

Each leaf is recorded as `(offset, prefix)` with `offset = L`, so its
height is `F_p + offset`
([`LeafHeight`](../../crates/before/src/version/measure/min_ticks/minima/heights.rs#L17-L35)).
`add_leaf` adds the offset to the answer at once and adds `+1` to a
per-prefix coefficient, deferring the `F_p` part
([lines 59–82](../../crates/before/src/version/measure/min_ticks/minima/heights.rs#L59-L82)).
A settled minimum does the reverse: it subtracts `closes × offset` from the
answer and `closes` from its prefix's coefficient
([lines 84–87](../../crates/before/src/version/measure/min_ticks/minima/heights.rs#L84-L87)).
At the end, `settle` adds `Σ_p coefficient_p · F_p`, rewritten as
`Σ_c component_c · Σ_{p ≥ c} coefficient_p`, so each frozen component
takes part in one multiplication however many leaves used it
([lines 102–124](../../crates/before/src/version/measure/min_ticks/minima/heights.rs#L102-L124)).

*Why freeze at all.* `add_leaf` reads `L` at every leaf, and a minimum's
record holds a copy of it. If `L` were allowed to keep a wide early jump
while later changes are one tick each, every leaf would copy that width,
which is quadratic. So the fold freezes `L` into a new component, and
restarts it at zero, whenever `L` is stored wider than the next change by
more than `HEIGHT_FREEZE_ALLOWANCE_DIGITS = 8` base-`2^32` digits, a
256-bit allowance
([`min_ticks.rs`](../../crates/before/src/version/measure/min_ticks.rs#L64-L72),
[`measure.rs`](../../crates/before/src/version/measure.rs#L21-L26)).
The allowance keeps small fluctuations in width from freezing over and
over. It also means that `L` can stay up to 256 bits wider than every
recent change without being frozen; section 3 shows why that matters.

## Range minima: the minimum of every open subtree

Open subtrees are properly nested, so their minima satisfy
`outer_min ≤ … ≤ inner_min`. `RangeMinima` calls an open subtree a
*range*. A range is *pending* until its first leaf and *armed* afterwards
([module doc](../../crates/before/src/version/range_minima.rs#L1-L71)).

Instead of one absolute minimum per range, it stores the differences
between adjacent armed minima, called *boundaries*
([`Boundaries`](../../crates/before/src/version/range_minima/boundaries.rs#L27-L41)).
Equal adjacent minima form counted zero runs. A positive difference is a
`u64` word in a packed bit stack or, if wider, an `Accumulator`
([`Boundary`](../../crates/before/src/version/range_minima/boundary.rs#L16-L22)).
One *anchor* relates the running height to the innermost minimum
([`anchor.rs`](../../crates/before/src/version/range_minima/anchor.rs#L1-L15)).

Each positive boundary also carries one caller-owned *payload*: the
outer range's state, suspended while an inner range has a higher minimum.
`min_ticks` uses this payload for its contributions, described next; the
tick walk and its prescan use `()`.

The updates, in the module's own terms:

- *Arming* (a leaf reaches new pending ranges). If the leaf is above the
  innermost minimum, push a positive boundary `v − m` and suspend the
  current payload on it. If it is equal, extend the zero run. If it is
  below, retire the payload and propagate the drop outward
  ([`finish_arming`](../../crates/before/src/version/range_minima.rs#L208-L239)).
- *Undercut* (a later leaf falls below the innermost minimum). The drop
  propagates outward: zero runs pass it through, each positive boundary
  absorbs as much as it can, and every boundary it crosses is consumed and
  its payload retired
  ([`propagate_drop`](../../crates/before/src/version/range_minima.rs#L258-L294)).
- *Close*. Pop one record. An equal parent changes nothing; a lower parent
  moves its boundary into the anchor's deferred distance and returns its
  payload as `Close::Lower`
  ([`close`](../../crates/before/src/version/range_minima.rs#L296-L329)).

The cost argument (module doc, lines 57–71) rests on one property worth
stating plainly, because sections 4 and 5 turn on it: **`RangeMinima`
moves boundary buffers and payloads; it does not read them when it pushes
or pops.** A wide boundary is read when a drop actually crosses it, which
consumes it, and otherwise only changes owner. A consuming merge keeps the
wider buffer and reads the narrower one.

## Contributions: who supplied each minimum

The answer subtracts each internal node's minimum once. Many nested
subtrees often share one minimum, supplied by one leaf. `SubtreeMinima`
therefore attaches a *contribution* to each distinct minimum, not to each
subtree: the supplying leaf's offset, its prefix, and the number of
subtree closes charged to it
([`Contribution`](../../crates/before/src/version/measure/min_ticks/minima/contributions.rs#L16-L24)).
A close at an equal minimum only increments the count. When the minimum
changes or leaves the stack, one multiplication settles the whole
contribution
([`close_subtree`](../../crates/before/src/version/measure/min_ticks/minima.rs#L142-L176),
[`observe_leaf`](../../crates/before/src/version/measure/min_ticks/minima.rs#L178-L236)).

The innermost minimum's contribution is held in `current_contribution`.
Every outer contribution waits as the payload of the positive boundary
that separates its range from the next range inward
([`SubtreeMinima`](../../crates/before/src/version/measure/min_ticks/minima.rs#L112-L120)).

### The packed contribution word

A contribution is stored as one `u64`, `StoredContribution`
([lines 51–79](../../crates/before/src/version/measure/min_ticks/minima/contributions.rs#L51-L79)):

| Bits | Field | Capacity |
|---|---|---|
| 0–31 | offset from the prefix | signed 32 bits (`i32`) |
| 32–54 | prefix index | 23 bits (`2^23` prefixes) |
| 55–62 | close count | 8 bits (255 closes) |
| 63 | spill flag | — |

When the offset or prefix does not fit, `ContributionStore::store` *spills*
it: the word's low 63 bits become an index into
`slots: Vec<Option<Contribution>>`, a slot holding the exact offset as a
`BigInt`, the prefix as a `usize`, and the count as a `u64`
([`inline`](../../crates/before/src/version/measure/min_ticks/minima/contributions.rs#L82-L91),
[`spill`](../../crates/before/src/version/measure/min_ticks/minima/contributions.rs#L151-L165)).
A count that passes 255 spills the same way
([`increment`](../../crates/before/src/version/measure/min_ticks/minima/contributions.rs#L167-L184)).
Settling frees the slot for reuse. On a 64-bit target a slot is 48 bytes:
a `BigInt` is a 24-byte `Vec` header plus a sign, padded to 32, then 8 for
the prefix and 8 for the count (code, by layout; the defect record states
the same figure). The `BigInt`'s digits are a separate allocation.

The word makes the common case cheap: a suspended minimum is one 8-byte
payload, moved by value. Spilling makes every case exact. The cost of that
split is the subject of section 3.

## The worked example, traced

The tree from section 1 has leaves 3, 5, and 4; the root `R` covers all
three, and `N` covers the last two. The first leaf's height 3 becomes
component 0, so every offset below is relative to 3.

| Step | Live change `L` | Answer so far | Coefficient of prefix 0 | Minima state |
|---|---|---|---|---|
| leaf 3: open `R`, arm it | 0 | `+0` | 1 | `R` armed at 3; current contribution `(0, p0, 0 closes)` |
| leaf 5 (`δ = +2`): open `N`, arm it | 2 | `+2` | 2 | 5 is above 3: push boundary 2, suspend `R`'s contribution on it; current `(2, p0, 0)` |
| leaf 4 (`δ = −1`) | 1 | `+1` | 3 | 4 undercuts 5: settle `(2, p0, 0)`, which subtracts nothing; current `(1, p0, 0)`; boundary 2 shrinks to 1 |
| close `N` | — | `−1·1` | 2 | `Close::Lower`: settle `(1, p0, 1 close)`; resume `R`'s contribution |
| close `R` | — | `−1·0` | 1 | `Close::Retired`: settle `(0, p0, 1 close)` |
| settle prefixes | — | `+1·3` | — | component 3 times coefficient 1 |

The answer is `0 + 2 + 1 − 1 − 0 + 3 = 5`, as section 1 computed.

## Why the time is linear

The module documentation claims linear time
([`measure.rs`](../../crates/before/src/version/measure.rs#L15-L19)),
and the structure supports it (code): each change is folded into `L` once;
the freeze rule keeps `L` within 256 bits of the change that reached the
leaf, so the per-leaf copy is paid by that change's code; each boundary is
created by input and consumed at most once; each close is O(1); each frozen
component enters one product at settlement.

Two of these steps are not measured by any committed instrument.
`add_leaf` and the spill path use `num-bigint`, and the board's time
measures count only `suanpan` accumulator digit work and encoded bits
(section 5). `main`'s linear time in `num-bigint` work rests on the
argument above, not on a measurement.
