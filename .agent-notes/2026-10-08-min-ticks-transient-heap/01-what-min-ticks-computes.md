# 1. What `min_ticks` computes

## The definition

A `Version` records how many events each part of the party interval
`[0, 1)` has seen. It is a nonnegative step function over that interval,
stored as an *event tree*: a binary tree in which every node carries a
nonnegative *base*, a leaf's *height* is the sum of the bases on its path
from the root, and each internal node splits its interval into equal
halves. The canonical form of the tree is *normal*: every internal node
has pulled the minimum of its subtree up into its own base, so at least
one child of every internal node has base zero, and no internal node has
two equal leaves as children.

`Version::min_ticks` returns the fewest `tick`s that any history of
`fork`, `tick`, and `join` could have performed to produce the version.
The public documentation states it as a floor over all causal histories
([`version.rs`](../../crates/before/src/version.rs#L308-L343)), and the
tree oracle defines it as the sum of every base in the normal-form tree
([`testing/oracles/tree/version.rs`](../../crates/before/src/testing/oracles/tree/version.rs#L220-L234)).
A base can be read as the ticks that every leaf below the node received
together and that the parent's whole interval did not.

## The identity the code evaluates

Summing bases needs the tree's structure at every node. The streaming code
reads leaves left to right and uses an equivalent identity instead:

`min_ticks = Σ leaf heights − Σ internal-node subtree minima`.

The identity follows from normal form. Write `m(v)` for the minimum leaf
height in the subtree rooted at `v`. In normal form, `m(v)` is the sum of
the bases from the root down to `v`, so `base(v) = m(v) − m(parent(v))`,
with `m(parent(root)) = 0`. Summing over all nodes,

`Σ_v base(v) = Σ_v m(v) − Σ_v m(parent(v))`.

Every internal node is the parent of exactly two nodes, so the second sum
is `2 · Σ_internal m(u)`. The first sum is `Σ_leaves h + Σ_internal m(u)`,
because a leaf's minimum is its own height. Subtracting gives the identity.

The module documentation gives the same identity in words: every leaf
height counts the events that reached that leaf, so an event shared by
both children of an internal node is counted twice, and the node's
subtree minimum is subtracted once to correct it
([`min_ticks.rs`](../../crates/before/src/version/measure/min_ticks.rs#L1-L12)).

## A worked example

Take a root whose left child is a leaf of height 3 and whose right child
is an internal node `N` with leaves of heights 5 and 4. In normal form the
root's base is 3, its left leaf's base is 0, `N`'s base is 1, and `N`'s
leaves have bases 1 and 0.

- Summing bases gives `3 + 0 + 1 + 1 + 0 = 5`.
- The identity gives leaf heights `3 + 5 + 4 = 12`, minus the root's
  subtree minimum 3 and `N`'s subtree minimum 4, which is also 5.
- One history with 5 ticks exists: tick the whole interval 3 times, tick
  `N`'s half once (heights 3, 4, 4), and tick `N`'s left quarter once
  (heights 3, 5, 4).

Section 2 traces how `main`'s fold reaches 5 on this tree.

## How the input arrives

The fold never sees the tree as a tree. A version's canonical encoding is a
bit stream of topology and payloads in leaf order. The first payload is an
absolute height, and every later payload is the signed change from the
previous leaf's height, coded as a zigzag Elias-gamma code
([`PayloadKind`](../../crates/before/src/version/io/regions.rs#L54-L77)).
A change of `±1` costs a few bits, and a change near `2^b` costs about
`2b` bits. A reader yields, per leaf, the height change and how many
ancestors the walk closed and opened to reach it.

This encoding is what makes the problem hard. One wide change, such as a
jump to `2^40`, is paid for once in the input, but every later leaf height
contains it. Any computation that copies or rereads an absolute height per
leaf does work proportional to (number of leaves) × (width of the jump),
which is quadratic in the input. The machinery in section 2 exists to
avoid that.

## The resource contract

The crate's documentation promises that the auxiliary space of any
operation is "at most a small constant multiple of the input size", and
that more is a bug
([`lib.rs`](../../crates/before/src/lib.rs#L333-L340)). The amplification
board turns "small constant" into a number: a peak of at most
`1024 + 20 · n` bytes of *transient heap* for an input of `n` bytes, where
transient heap is the peak of additional live heap during the call
([`board/ceilings.rs`](../../crates/before/src/testing/meter/board/ceilings.rs#L15-L30)).
The `version_min_ticks` row uses the encoded version's byte length as `n`
([`board/ops.rs`](../../crates/before/src/testing/meter/board/ops.rs#L1166-L1179)).
The module documentation also promises that `min_ticks` "is linear in its
input" ([`measure.rs`](../../crates/before/src/version/measure.rs#L15-L19)).
The defect breaches the board's number. Of the repairs in section 4, one
breached it again at a scale the board cannot sample, and the last kept it
but breached linear time on a constructed input.
