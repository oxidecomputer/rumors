## Verdict: stopped with a proposal

The defect is real and lies where the record says. The demonstration commit is done, and it fails exactly as intended. I did not commit a fix: every design I measured either raises a committed ceiling, which is a stop condition, or fails the brief's goal. The owner has to rule on the trade below.

## Branch and commits

- Branch `fix/range-minima-wide-boundaries`, slot `/Users/oxide/src/rumors-slot-58`.
- Tip `e9ab04ba2` "Add the wide-step spine to the before amplification board". Its parent is `6a8dab7cc`, which I verified as HEAD at the start.
- The slot's tree is clean.
- Unsigned commit: `e9ab04ba2`. I used `--no-gpg-sign` as instructed.

## The demonstration (`e9ab04ba2`)

It adds the family `wide-step-spine`, `WS(e, d)`: a right spine whose nested minima each rise by `2^e`. It uses `e = 64`, the smallest power of two wider than a word, and the input pays one 129-bit code per level. The default depth is a few levels past a power of two, so every sampled size lands just after a doubling reallocation.

Wiring follows the `jump-rising-spine` and `wide-step-pairs` precedent:
- builder plus a closed-form shape property in `testing/meter.rs` and `tests.rs`;
- registry `Shape` and `FamilyId` entries;
- `family.rs` constants, whose docs quote no sizes;
- `verdict_matrix`;
- the worst-case pin's `VERSION_STREAM_FAMILIES` and the four inline exact-tie lists (`clock_split_array × heap`, `clock_borsh_deserialize × scan`).

I verified the board at exactly `e9ab04ba2` in a detached scratch worktree:
```
RED version_min_ticks wide-step-spine  4307->8630  B heap 25.9/B  <- heap constant
RED version_min_ticks wide-step-spine 17275->34565 B heap 25.8/B  <- heap constant
amp-board: 5636 green / 1 red (5637 cells)   (small-input sweep 5637 green; that sample reads 15.3/B)
worst-case pin drift: version_min_ticks x heap ... pinned worst propagate-seam, live worst wide-step-spine (both scales)
```
That pin drift is the defect's own ranking; there is no other drift. The base `6a8dab7cc` reads `5545 green / 0 red` with the pin clean, matching `baseline.md`. `amp-board compare base demo` gives `16911 rows: 276 only-after, 16635 unchanged`: only the new family's rows. Focused tests (new property, registry, verdict matrix) passed 9/9, and clippy for `before` is clean.

## Root cause, confirmed by measurement

`Boundaries` stores every boundary whose working width is over 64 bits as a whole `Accumulator` in `wide_values: Vec<Accumulator>`. `size_of::<Accumulator>()` is 128, which I measured. A 65-bit value sits inline with no digit buffer, so the whole cost is that slot, up to three times over at a reallocation peak, against about 16.5 input bytes per level.

The probe (`min_ticks` peak heap per input byte on `WS(e, d)`, base):

| `e` | depth just past a power of two | depth at a power of two |
|---|---|---|
| 63 | 2.6 | 2.7 |
| 64 / 65 | 26.2 / 25.8 | 13.1 / 12.9 |
| 97 | 17.5 | 8.8 |
| 128 | 13.6 | 6.8 |
| 256 | 8.1 | 4.1 |
| 512 | 4.8 | 2.8 |

The tick walk reads under 0.04 on every row, so it is unaffected.

Two points beyond the record:
- **The record's `reference-dense e=65` shares the cause.** It reads 13.4 at depth 2000 but 25.5 at depth 2100. Its "13.4 after #131" was an alignment accident.
- **The pinned `min_ticks` heap worst, `propagate-seam` at 12.7 B/B, is mostly this same cost.** Copying wide boundaries drops it to 1.8.

## Why I stopped

A fix has to keep each wide move O(1). Lesson 5.2 forbids reading a large value on every push and pop, because `reveal-comb` circulates one wide value between a boundary and the deferred distance. That leaves two designs. Both patches are saved and verified to reverse-apply cleanly.

**1. Copy, mirroring `min_ticks`' `ChunkStack` (`candidate-copy.patch`).** Wide boundaries of at most 256 working bits go into `compact` as limbs plus a count, using a digit-free `u128` path when the value fits. Wider ones move whole. The docs in `range_minima.rs`, `boundary.rs`, and `boundaries.rs` are restated to match.
- Heap becomes proportional to width:
  - `WS` reads 1.05 to 1.9 for every `e` from 64 to 256;
  - `WP(65)` reads 2.8 to 3.0;
  - on the board, `wide-step-spine` drops 25.9 to 1.7 and `propagate-seam` 12.7 to 1.8.
- But copying a digit-form boundary reads its digits, about 2 to 4 touches per wide boundary pushed:
  - The committed meter `skyline_min_ticks_descending_boundary_is_flat_per_unit` fails. `seam_plunge` large reads 25160 against `DESCENDING_TOUCH_CEILING` 23099; the base reads 21064.
  - Its surplus band fails as well: the surplus is −253 against a floor of 1514.
  - Board touch rises (still green): `propagate-seam` +0.23/B, `wide-step-spine` +0.15/B, and one-size rises on `dominated-undercut` (+1920 on `min_ticks`, +1792 on the tick rows) and `cancelling-chain` (+6630).
  - The new form tag adds heap of at most about 0.001 B/B on `promo-rearm`, `dense-suffix`, `arming-train`, and `wide-step-pairs`.
- `seam-plunge`'s boundaries are three-digit digit-form values. So no copy threshold avoids the touch cost, and even the minimum possible read (about 24100 touches) would exceed the ceiling.

**2. Box the whole accumulators (`candidate-box.patch`, three lines).**
- Touch-neutral: no touch row moves, the seam meters are unchanged, and every cell is green.
- `wide-step-spine` lands at 12.2 B/B and `propagate-seam` at 10.9, with constant gains of 352 to 480 bytes on many rows.
- It leaves a fixed 128-byte box per wide boundary, so it is not proportional to width, and the pin flips to `wide-step-spine`.
- Heap rises on `promo-rearm` and `dense-suffix` (about +0.065/B), `wide-step-pairs` (+0.098/B), and `arming-train` (+0.005/B).
- It adds an allocation per wide push.

A third option, copying only inline accumulators, would need a new public `suanpan` API to detect that form. That API is a stop condition too.

**Decision needed:**
1. **Recommended: design 1.** It meets the goal of storage proportional to width. It needs the owner to re-pin `DESCENDING_TOUCH_CEILING` and re-derive `DESCENDING_PARK_SURPLUS_BAND`; the surplus going negative is the copy cost, not the per-hop residue read the band exists to catch. It should also lower `WIDE_STEP_PAIRS_HEAP_CEILINGS`, which the fix improves, and re-pin `version_min_ticks × heap`, where `wide-step-pairs` (6.2) becomes the worst.
2. **Alternative: design 2.** It is acceptable only if "within the ceiling" is enough, and it still needs the four small heap rises ruled on.

## Not done because of the stop

The full verification list (fmt, both clippy recipes, `test-all`, doctests, doclint, rustdoc), the tip capture, and re-pins. They apply only once a fix commit exists. The demonstration commit had focused checks only, as the brief allows.

## Files

Everything is in `/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/fixer-range-minima-wide/`:
- `candidate-copy.patch`, `candidate-box.patch` (apply on `e9ab04ba2`), `rmw_probe.rs` (the uncommitted probe)
- `base.capture`, `demo-commit.capture`, `copy.capture`, `box.capture`
- `demo-commit-board.log`, `base-vs-demo-compare.log`, `copy-compare.log`, `box-compare.log`
- `probe-base.log`, `probe-fix.log`, `fix2-minimum.log`, `base-minimum.log`
- `NOTES.md`

The scratch worktree and its build directory are removed. Archive these files before a reboot clears `/private/tmp`.