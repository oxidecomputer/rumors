<!-- CAVEAT LECTOR: a builder brief written by the coordinator (Claude Opus 5.5), collecting the audit's documentation findings into one branch. -->

# Builder brief: the documentation branch

Kind: prose corrections and one approved API addition. One commit per item,
so the owner can take any subset. No value, wire format, or storage format
changes.

## Rules for every item

- Correct prose toward the code. When the prose and the code disagree, the
  code wins, unless the item says the code is wrong.
- State what is, in the present tense; no ghost references, no performance
  values, no hand-maintained counts or enumerations.
- Read each item's source record before acting. Verify the finding against
  the tree at your base; drop an item the tree no longer supports, and say
  why.
- An item whose file is also changed by an unlanded branch is *deferred*,
  not built: list it with the conflicting branch. Check with
  `git merge-tree` against every local branch matching `audit/*`, `fix/*`,
  and `simplify/*` (not `archive/*`). A branch that already makes the same
  correction makes the item moot; say so.

## Items

Source records are under `lanes/<lane>/round-1/` unless stated.

1. **Algebra lane (L2), observations 1, 3, 4** (`l2-algebra/round-1/
   observations.md`). Observation 1 also needs the contract text in
   `fuelscape/version_join_all.json` hand-edited (the owner allowed it).
2. **Measures lane (L4):** the prose brief
   (`l4-measures/round-1/simplification-rank-normalization-prose.md`), and
   observations O1 and O8 (`observations.md`).
3. **Spans lane (L5):** the doc defects (`l5-spans/round-1/doc-defects.md`)
   and the tie-order corrections (`tie-order.md`). Also: `causally/tests.rs`
   (around lines 37, 107, 609-610, 656) claims the two-party grid is the
   whole interval. It is not (`a1 & q` is a canonical version strictly
   inside it, off the grid). The census is exact for a different reason:
   every witness for emptiness or non-fullness is a lattice endpoint (`lo`,
   `hi`, `lo ∨ floor`, `hi ∧ ceiling`), all on the grid sublattice. State
   that argument.
4. **Codecs lane (L6)** (`l6-codecs/round-1/`): the `TrailingBits` doc
   widening (owner approved), the borsh observations, and the CBOR bridging
   sentence (owner ruled; the ruling is in its record).
5. **Suanpan lane (L7) O3** (`l7-suanpan/round-*/`, find O3).
6. **Identity lane (L1)** (`l1-identity/round-1/observations.md`):
   O2, O3, O4; O6, implementing `FusedIterator` for the fork iterators
   (owner approved: an API addition, so add a test that the iterator stays
   exhausted after returning `None`); and the `from_parts` hazard (owner
   ruled; find the ruling in the lane's records).
7. **suanpan `read.rs`** uses "high part" for two things: the signed final
   carry `c` (lines 3-9; "at most 3 in magnitude" is exact for it) and the
   unsigned digit at position `d` (lines 13-14, the `high` variable; its
   tight bound is 2). Call `c` "the final carry", as the `expect` messages at
   lines 90 and 99 already do, and state that the high part is at most 2.
   This is #27's file: defer if #27's branch conflicts.
8. **suanpan `digits.rs:24-25`** says `highest_nonzero` is zero "when the
   value is zero". That is false for a zero held in several digits; the field
   doc at line 65 ("when every digit is zero") is right. Align the module doc.
9. **`range_minima/boundary.rs:9`**: "Width leading-digit comparisons" reads
   as a typo; restate the sentence.
10. **`recurse.rs`'s `RED_ZONE` rustdoc** cites a measured frame size
    ("roughly 0.5 KiB/level", "8x cushion"), against the rule that tree prose
    states no performance values. Restate it as the rule behind the
    constant: what the red zone must exceed and why, without the reading.
    If the constant cannot be justified without a reading, say so and leave
    it for the owner.
11. **`bits/writer.rs:72`**: `BitsWriter::repeat`'s `expect` message "a
    repeated buffer is allocatable" states the wrong proof. Its only
    production caller (`version/tick/route.rs:235`) passes the stored length
    of a party already in memory, so the byte count never exceeds that
    party's. Say so, for example "the route spans a party held in memory".

Item 6's AGENTS.md recursion rule is not in this branch: it waits on the
owner's question 68.

## Verification

`just readme` if any crate-level rustdoc changes (the READMEs are derived),
both rustdoc builds with `-D warnings`, and one landing check at the tip.
Report each item as built (with SHA), deferred (with the conflicting
branch), moot, or dropped (with the reason).
