# Further observations

Observations the discussion brought to mind that the other sections do not
already hold, gathered when Finch asked for anything else worth capturing.
Part of the [presentation note](README.md). Each is a proposal or an
observation, not a ruling, unless it says otherwise.

## Notation

**The readable notation is superlinear in the encoding (measured; ruled
acceptable).** The notation prints absolute positions and absolute heights.
The encoding stores depth deltas and height deltas. So output can grow
quadratically in encoded size. The [probe](probe/)'s `amplify` binary
measures both halves:

| Input | Encoded bytes | Notation size |
|---|---|---|
| alternating spine, 8 / 16 / 32 / 60 levels | 8 / 16 / 30 / 54 | 197 / 508 / 1,516 / 4,512 chars |
| 65 runs over a base of 20 / 80 / 320 / 1,280 decimal digits | 57 / 106 / 306 / 1,103 | 1,300 / 5,200 / 20,800 / 83,200 height digits |

The spine's boundaries look like `0.01010101`, so no run-length shorthand
helps. This is the hazard the rank integrator's docs name for absolute
positions. Finch rules it acceptable for `Debug`. Its bound still belongs in
its docs, as `Display` documents its own, with a fuelscape panel. Optional
mitigations: run-length shorthand for runs of a repeated digit (which helps
the spines that repeated `fork` builds, such as `0.000001`), and `{:.N?}`
truncation.

**Logging.** In `tracing`, `?v` will print the readable form and `%v` the
hex. For large versions in production logs, `%v` (or truncated `Debug`) is
the right choice; `rumors-tracing` should choose deliberately.

**Snapshot tests.** Readable `Debug` would make semantic `insta` snapshots
legible. But a snapshot that contains it pins the notation, so changing the
notation becomes a re-accept under the snapshot rules. Decide whether `Debug`
output may appear in snapshots.

**Collapse to a version vector when regions are flat.** With a legend, if
every owner's region has one height, the annotated version is exactly a
version vector: `{alice: 3, bob: 2, carol: 1}`. That holds under static
membership with single-interval parties. Rendering could use the compact
form whenever it is exact, and expand only owners whose regions vary. This
is the principled version of round one's pseudo-version-vector table.

**Projections render as their materialization (ruled; see the README).** An
earlier proposal marked ranges outside the party with a wildcard arm
(`0.1..1 => _`), to tell "outside this party" from "no events here". Finch
ruled instead that a projected view prints exactly as the masked version
does, save the type name, and the ruling holds up: the view compares equal
to its materialization, so a wildcard would render two equal values
differently.

**Point spans read as versions.** `Span::at(v)` has a single height in every
arm, so its `Debug` matches `v`'s apart from the type name, a useful
consistency check on the band notation.

**Explanations for failing predicates.** The table renderer extends past
diffs:

- `span.place(&v)` as three columns, `lo`, `v`, `hi`, marking rows where `v`
  falls below or above the band;
- `query.contains(&v)` naming the conjunct that failed (floor, ceiling, or a
  particular hole), with its diff;
- an assertion such as `assert_within!(v, span)` built on the first.

**Macro names.** `assert_le!` and similar names collide with numeric
assertion crates such as `more_asserts`. Names drawn from the crate's own
vocabulary (`assert_before!` for `<`, `assert_concurrent!`,
`assert_within!`) would read better at call sites and avoid the clash. Path
qualification (`before::assert_le!`) also avoids it, at the cost of
verbosity.

**Smaller choices.** Whether pretty output aligns the `=>` arrows as well as
the owner comments, and whether very deep positions use a shorthand
(`0.0{19}1`), are open.

**`before-viz`.** Its glyph uses a linear axis with a width cap, so narrow
regions vanish past the cap. An ordinal axis or a minimum width per region
would keep them visible. Showing the textual notation on hover would give
the visual and textual forms one shared vocabulary.

## Semantics

**Distributional and geometric measures.** `rank`, width, maximum, and
minimum depend only on how much identity space sits at each height. They are
unchanged by any rearrangement of ids that preserves width. `min_ticks` and
piece counts depend on the arrangement. With half the space at height 1,
`0..0.1` is one brick, and `0.01..0.11` is two (`0.01..0.1` and `0.1..0.11`).
In the [02-measures.md](02-measures.md) table, the weighted column is the
distributional one and the counted column the geometric one. The vertical
measures are distributional and depth-independent at once.

**A probabilistic reading.** For an id `x` drawn uniformly from `[0, 1)`:

- `rank(v)` is the expected height `E[v(x)]`;
- width is the probability that `x` has seen any event;
- maximum and minimum are the worst and best cases.

"The average number of events seen per point of identity space" may be a
friendlier gloss for `Rank`'s docs than "exact area".

**A profile view.** Listing each floor's width forgets positions but keeps
the distribution: `≥3: 0.01, ≥2: 0.011, ≥1: 1`. It is monotone under the
causal order floor by floor, and lossy. It reads as a compact "shape of
knowledge" for dashboards or summaries.

**Identity space is atomless, and that is why `rank` is dyadic.** Every
dyadic region can be split, which is what makes `fork` total. So no version
is indivisible under join, and strict chains of every length fit below
`flat(1)`. That rules out any integer-valued, strictly order-respecting
measure (proved in the skew section of 02-measures.md). The property that
makes `fork` always succeed is the one that forces `rank` to take
non-integer values.

**An identity leak pins the floor forever (reasoned).** Only a region's
owner raises it, since `fill` acts only within the ticking party's region
and joins never exceed an existing height. So a region whose party was
dropped can never rise above the highest height any version holds there.
The global minimum, and any check of the form "everyone has seen `k` full
layers", stalls permanently; the peak is unaffected. `Party`'s `#[must_use]`
message says discarding "loses its share of identity space"; the
consequence for measures and for any stability logic built on minima is
broader, and may deserve a sentence in `Party`'s docs.

**Ownership attributes the future, not the past.** The current owner is the
only party that can raise a region next, so "who will write here" is exact,
while "who wrote here" is lost after recycling.

**Positions record fork lineage.** A party's binary prefixes record the
sequence of splits that produced it, up to joins and normalization: a
region at `0.10..0.11` was split right, then left. This is structural
history, not attribution of events.

**Scalar time inside versions.** The peak behaves as a scalar,
Lamport-style clock that weakly respects causality, and `rank` as one that
strictly respects it at the price of dyadic values. A caller wanting a cheap
scalar timestamp would choose between them by that trade.

## API

**A structured twin of the notation.** `shape` yields depths and rises and
deliberately never materializes positions. A `runs()` iterator for
versions, and an owned-ranges iterator for parties, yielding maximal runs
with positions, would expose exactly what `Debug` prints. Evidence that users
need it: the probe hand-rolled positions as `u128` numerators capped at
depth 64, and `before-viz` derives positions from depths in TypeScript. It
needs a public type for dyadic positions, and it inherits the same
superlinear size as `Debug`, which is presumably why `shape` avoids it. It
must either document that or yield positions relatively, as a shared prefix
plus a new suffix.

**One dyadic type, or several?** Positions, widths (`lag_width`), and ranks
are all nonnegative dyadic rationals. `Rank` already is one, with a
canonical, order-preserving encoding. Whether positions and widths reuse it,
share a new type, or get one each is a naming and semantics decision. `lag`
and `distance` already return `Rank` for areas that are not the rank of any
version.

**Doctests that show `Debug` output** would document the notation and test
it in one place, at the cost of updating examples when the notation
changes.

**A clock rendering that marks ownership inline**, which answers "what does
my own region look like" at a glance where a struct form separates party and
version, is now ruled: `Clock { 0..0.1 => 3 /* own */, 0.1..0.11 => 2, 0.11..1 => 1 }`.
See the README's Clock section.

**A "which fork API?" table** in the docs, covering `fork`, `forks(k)`,
`From<Party> for [Party; N]`, and the proposed `into_forks(n)` and
`forking()`. The current docs are accurate; a decision table would route a
reader faster. The probe's misreading of `forks` came from not reading those
docs, not from an error in them.

## Process

**Probes against the public API produce design feedback.** One small
external crate turned up three things: the public API lacks positions, the
current `Debug` was opaque in a real assertion failure, and the `forks`
contract is easy to assume wrongly without its docs. Keeping such probes
with notes makes the evidence reproducible.

**Instruments before the notation lands.** Before the new `Debug` is
written, the test-only parser with its round-trip proptest, and a fuelscape
panel with a documented bound, give it a committed baseline. Per the
amplification finding above, that bound is not linear.
