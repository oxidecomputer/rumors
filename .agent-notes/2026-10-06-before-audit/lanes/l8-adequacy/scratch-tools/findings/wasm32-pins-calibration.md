# Calibration of the wasm32 boundary pins

Lane L8, auditor-l8. The pins (`crates/before/wasm32-pins/harness/tests/pins.rs`)
are the audit's instrument of record for 32-bit targets. Each pin's doc
comment names the defect it exists to catch. I injected each named defect (or
the closest site I could identify) and ran that pin. Seven of eleven
injections are caught. The four that are not each trace to a specific reason;
two show a pin's doc claiming more than its input can exercise.

## Method

- All work in a detached copy of `before`, `suanpan`, and `wasm32-pins` at
  `l8/probe/` on `explore/l8-adequacy` (rebuilt fresh from `crates/` before
  every injection by `l8/mkprobe.sh`), never in `crates/`.
- `l8/w32.sh` builds the guest (`wasm32-unknown-unknown`, release,
  `overflow-checks = false` as the pins workspace sets) and the harness, then
  runs the pins selected by a nextest filter.
- Baseline from the copy, no injection: all 8 pins pass
  (`w32/baseline.log`: "8 tests run: 8 passed (1 slow)").
- Each injection is an exact-count string swap (`w32/injections.py`, counts
  verified against `crates/` before running); verdicts in `w32/verdicts.txt`,
  logs in `w32/<id>.log`. Builds pick up injections: every file injected
  (reader.rs, storage.rs, rank.rs, forks.rs, suanpan's accumulator.rs) has at
  least one caught injection.

## Results

| injection | site | pin | verdict |
|---|---|---|---|
| decode-from-bytes | `bits/reader.rs:122` `bytes.len() as u64 * 8` as `(bytes.len() * 8) as u64` | version_decode | caught: `VersionDecode failed for (536870912, 0)`, `left: Failed(DecodeRejected)` |
| decode-live-len | `bits/reader.rs:268`, same narrowing | version_decode | caught |
| decode-padding-total | `bits/storage.rs:98`, same narrowing | version_decode | caught |
| rank-decode-frac-len | `rank.rs:686` fraction length `groups.len() * 8` in `usize` | rank_decode | caught |
| rank-accumulate-sign | `rank.rs:520` `sub_shifted_limbs` as `add_shifted_limbs` (the 32-bit-only path) | rank_arithmetic | caught |
| forks-count-narrow | `party/forks.rs:124` count narrowed through `as usize` | forks | caught |
| suanpan-landing-narrow | `suanpan/src/accumulator.rs:414` `usize::try_from(position)` as `position as usize` | suanpan landing | caught |
| compare-reader-open | `bits/reader.rs:148` `(position / 8) as usize` as `(position as usize) / 8` | version_compare | **not caught** |
| compare-gamma-window | `bits/reader/gamma/window.rs:28` `usize::try_from(position / 8)` as `(position as usize) / 8` | version_compare | **not caught** |
| join-live-usize | `bits/writer.rs` all four `self.live +=` sites computed in `usize` | version_join_output | **not caught** |
| rank-arith-route | `rank.rs:497` `alignment_fits` forced to `true` (always the contiguous route) | rank_arithmetic | **not caught** |

## Why the four survive

1. **The compare pin cannot reach a cursor position past 2^32.** Its large
   operand (`synthesis::version`) has exactly 2^32 live bits, so every live
   bit's position is below 2^32 and fits a 32-bit `usize`; only the end
   position does not. Trap probes confirm the two byte-index sites never see
   a large position: `assert!(position < 1 << 32)` at both sites passes, and
   so does the positive control `assert!(position < 1 << 20)` (verdicts under
   "trap probes"). The comparison reads the large stream sequentially from
   position 0, and the gamma window is consulted only at a code's start,
   where it finds the code wider than one word and falls back. The pin's doc
   ("Narrowing a cursor position before dividing it into a byte index would
   wrap the second input and misorder it against the smaller version",
   pins.rs:69-73) describes a mechanism its input never exercises. It still
   guards the 2^32 *length* along the comparison path.
2. **The join pin cannot see a wrapped writer length, by the writer's
   design.** `BitsWriter`'s appends use `live` only modulo 8
   (`append_bit`, `append_bits`, `extend_bytes` in `bits/writer.rs`), and
   `Bits::from_canonical` recomputes the stored length from the marker, so a
   `live` counter that wraps modulo 2^32 leaves the output bytes identical.
   The pin's doc ("A `usize` output length would wrap while appending or
   finalizing the second result", pins.rs:84-86) names a failure the current
   writer cannot exhibit on this path. A wrapped `live` would matter only
   where something reads it whole: `len()`, `truncate`, `read_word` bounds,
   or a reader opened over a writer (the split output's `finish`,
   `version/io/writer.rs:274-294`). No pin builds a writer past 2^32 bits on
   such a path.
3. **The rank-arithmetic pin cannot tell the two routes apart at its gap.**
   The contiguous route shifts by a `u64` (`self.num.clone() << (e -
   self.exp)`, rank.rs:198-201 and 957-960), and `num-bigint` 0.4.8 divides
   the shift into whole digits before narrowing
   (`(shift / bits).to_usize().expect("capacity overflow")`,
   `biguint/shift.rs:20`): a 2^32-bit shift needs 2^27 32-bit digits, which
   fits. So the contiguous route is correct at the pin's gap, and forcing it
   leaves the pin green (121 s against 79 s, the larger allocation). The
   pin's doc ("the latter must use the shifted accumulator path ... Wrapping
   or using the wrong route breaks the guest's exact inverse identities",
   pins.rs:112-117) is half false: the wrong route does not break the
   identities. The sign injection above shows the pin does check the
   accumulator path's arithmetic.

## What this means for 32-bit coverage

- Lengths and counts that cross 2^32 are well pinned: decode (three sites),
  rank exponents, fork counts, and digit landings each fail loudly under the
  narrowing their docs name.
- Positions are not: nothing in the pins places a live bit at or past 2^32,
  so narrowing of a cursor position anywhere on the read paths is
  undetectable by this instrument. A stream of at least `2^29 + 2` bytes
  whose decisive bit lies past 2^32, with different data at the wrapped
  offset, would close it; whether a guest can afford that allocation is for
  the pins' owner to judge (the decode pin already allocates `2^29 + 1`
  bytes).
- Two pin docs overstate what they check (items 1 and 3 above); item 2's doc
  names a failure the current writer design cannot produce. These are prose
  corrections at minimum.
- Inference worth a deeper look (lane L4 or whoever owns `Rank`): on every
  target I can see, the contiguous route is correct for every gap a
  constructible rank can have (a rank's exponent is bounded by the bits of
  the input that built it, so a 32-bit guest cannot hold one whose gap
  exceeds about 2^35, and `num-bigint` fails loudly, not wrongly, past
  `2^32` digits). If that holds, `Rank::accumulate` and `alignment_fits`
  exist for an overflow that cannot occur, and the stated rationale ("The
  accumulator handles larger gaps without narrowing the exponent",
  rank.rs:954-955) describes a narrowing the contiguous route does not
  perform. I have not verified the memory side of this claim.
