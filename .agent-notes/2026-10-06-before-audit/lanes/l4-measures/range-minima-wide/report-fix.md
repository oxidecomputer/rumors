## Verdict: design 1 is built, and every check passes

Branch `fix/range-minima-wide-boundaries` in `/Users/oxide/src/rumors-slot-58`. The tip is **`dc6a8ea01`** and the tree is clean.

- `535cddfa9` "Add the wide-step spine to the before amplification board": the demonstration. I amended it to split one test doc summary that `doclint` rejected (over 220 characters); its code is identical to `e9ab04ba2`.
- `dc6a8ea01` "before: Store wide range-minimum boundaries in proportion to their width": the fix.
- Both commits are unsigned (`--no-gpg-sign`). No landing check was started.

## One addition to the ruled design, which you should know about

The candidate patch alone raised `seam_stop` from 24330 to 40728 touches, over its ceiling of 30413. That shape keeps one five-digit boundary alive while k residues die against it. With the patch, each hop popped it, rebuilt it, and copied it back. That is a reread of the surviving boundary on every hop: exactly what `STOPPING_DIFF_BAND` exists to catch.

My fix keeps the innermost wide boundary live as its accumulator (`live_wide`), the way `top_zeros` already keeps the top zero run. A boundary is copied only when a later push covers it, so each copy is charged to that push, and a drop's surviving remainder goes straight back without being copied. With this, `seam_stop` is unchanged at 24330.

I also simplified the copy to one read path (`signed_magnitude`), with an inline `u128` rebuild for values of at most two limbs. That costs fewer touches than the patch's `u128` conversion: 4 per copy against 6.

## How the fix stores boundaries

A wide boundary is stored in one of three ways:
- **live:** the innermost one stays as its accumulator;
- **copied:** once covered, a boundary of at most `INLINE_BOUNDARY_BITS` (256) working bits goes into `compact` as its limbs, with their count on top;
- **whole:** a wider one moves as its accumulator, unread.

The docs in `range_minima.rs`, `boundary.rs`, and `boundaries.rs` state what is stored and why its size follows the boundary's width.

## Ceilings, bands, and the pin

- **`DESCENDING_TOUCH_CEILING`: 23099 → 27195 (23,099 + 4k).** The doc derives it: the cascade's existing bound, plus one copy per stacked boundary. A copy is one sign read decided at the top digit (one touch, since the top digit is 5) and one pass over three digits. I confirmed those costs with a scratch probe (removed). The reading is 21064 → 24133.
- **`DESCENDING_PARK_SURPLUS_BAND`: unchanged, because its reading didn't move.** The plunge's control went 22856 → 25925, so the surplus stays exactly 1792. The copies happen during the arming the two runs share, so they cancel. Its purpose, catching a per-hop residue read, still states cleanly; I added one sentence saying the copy cancels.
- **`DESCENDING_TOUCH_FLOOR`:** I dropped "spilled" from its doc, since a copied boundary comes back inline.
- **`WIDE_STEP_PAIRS_HEAP_CEILINGS`: 292580/584980 → 65120/130080**, which is ×1.25 of the new readings 52096/104064 (were 234064/467984). I restated its doc as what the reading measures (each wide level's copied chunk and boundary), replacing its justification by a broken variant.
- **Pin:** `version_min_ticks × heap` moves from `propagate-seam` to `wide-step-pairs` at both scales. `wide-step-pairs` is unchanged at 6.2 B/B; `propagate-seam` fell from 12.7 to 1.8.

## Board comparison against the base `6a8dab7cc`

The tip reads `5637 green / 0 red`, with `worst-case pin: clean (272 pinned rows verified at 2 sampling scales)`. The comparison: `16911 rows: 4 growing, 8 uneven, 18 constant, 276 only-after, 16605 unchanged`. The 276 only-after rows are the new family.

- **`version_min_ticks × wide-step-spine`** (against the demonstration capture): heap 25.9/25.8 → 1.7/1.7 B/B, small sample 15.3 → 3.6, so the cell goes red → green. Touch 2.1 → 2.3 B/B (+6234 at 34565 B), about three per copied boundary.
- **`version_min_ticks × propagate-seam`:** heap 12.7/12.7 → 1.8/1.7 B/B, small 15.9 → 4.8 (−770688 bytes at 70210 B). Touch +0.175 B/B (+3069 at 17602 B), three per copied boundary. This is the only touch row that moved.
- **Constant −512 bytes** on `version_tick`, `version_ticks`, `clock_tick`, and `version_min_ticks` rows over these families: `reveal-comb`, `dominated-undercut`, `memo-fanout`, `memo-oscillating`, `jump-pair`, `tooth-tail`, `jump-comb`, `cancelling-chain`, `latent-ladder`, and the small sample of `freeze-parade`. A lone wide boundary is now held live instead of in a four-slot `Vec<Accumulator>`.
- **Uneven decreases** (−37 to −512 bytes) on `reveal-hifloor`, `reveal-comb`, `pure-comb`, and `memo-oscillating`. I attribute them to the same cause by inference.
- **Small rises:** `version_min_ticks × promo-rearm` and `× dense-suffix` grow +128 to +1024 bytes over inputs of 126 KB to 1 MB (about 0.001 B/B). `× arming-train` and `× wide-step-pairs` rise +32 to +64 bytes. I infer, without measuring it directly, that these come from the new form tag: one bit per stored wide boundary.

## Checks at the tip

All of these passed at the tip tree with the two summaries still long; the only later changes are the two doc comments `doclint` flagged.
- `cargo fmt --all --check`, `just clippy`, `just clippy-default`, `just testdoc`: exit 0.
- `just test-all`: 1953 tests run, 1953 passed, 2 skipped.
- `just doctest`: exit 0, including `before` at 190 passed.
- `just doclint`: first failed on two summaries over 220 characters. I fixed them (the test doc via the amend, the ceiling doc in the fix) and re-ran it: exit 0.
- Rustdoc for `before` with `-D warnings`, public and `--document-private-items`, in separate target dirs: both exit 0.
- `meter` binary in full: 66/66 passed (run before the ceiling edits).

## One observation, not changed

`MAX_HEAP_BYTES_PER_INPUT_BYTE` is 18. Its stated rule, applied to the non-query heap rows in the captures, gives 16 at the base and 15 at the tip. The tip's maximum is `party_forks × ascend-*` at 11.99, since `propagate-seam` was the base's maximum at 12.69. But the base doesn't reproduce 18, so my filter must be missing some rows' declared denominators or models. I left the shared ceiling alone; someone who knows those exclusions could lower it.

## Files

In `/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/fixer-range-minima-wide/`:
- `verify/` (each check's log and `status`; `verify/compare.log` is the tip-against-base comparison)
- `tip.capture`, `base.capture`, `demo-commit.capture`
- `fix3-meter.log`, `touch-probe*.log`, `touch_probe.rs`, `NOTES.md`