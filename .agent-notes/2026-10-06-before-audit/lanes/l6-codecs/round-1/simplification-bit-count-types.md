# Simplification brief: type bit counts in the codec layer as fixed-width integers

Kind: self-contained, behavior-preserving, no public API or format change.
Base: `main` at `58285ca5`. Motivated by the `usize`-invariance clause (a
`usize` should only index or count what memory holds).

## Current code

Three internal quantities count *bits*, not memory, but are typed `usize`:

- `BitsWriter::reserve(&mut self, width: usize) -> u64`
  (`crates/before/src/bits/writer.rs:250-259`). Its loop narrows back to `u32`
  per chunk (`remaining.min(u64::BITS as usize) as u32`) and every caller
  passes the party tag width.
- `const TAG_BITS: usize = 2` (`crates/before/src/party/io/writer.rs:68`),
  immediately re-widened in `TERMINAL_PAIR_BITS: u64 = 3 * TAG_BITS as u64`
  (`:72`) and passed to `reserve`.
- `scan::record_bits(n: usize)` (`crates/before/src/testing/instrument/scan.rs`),
  beside `record_bits_u64(n: u64)` for the same counter; callers cast `u32`
  lengths up to `usize` (`bits/reader.rs:181`, `bits/writer.rs:162`,
  `bits/writer.rs:241`) only for the function to cast to `u64`. One caller,
  `bits/reader/gamma.rs:50`, passes dsi-bitstream's `usize` code length; it
  takes `u64::try_from(used)` or `used as u64` (bounded by the table width).

Every value is at most 64 today, so no behavior differs by target; the cost is
that a reader must check each site to know that.

## Proposed structure

- `reserve(&mut self, width: u32) -> u64`, with the loop over `u32`.
- `const TAG_BITS: u32 = 2` and `TERMINAL_PAIR_BITS: u64 = 3 * TAG_BITS as u64`
  (or `u64::from`).
- One recording entry, `record_bits(n: u64)`; delete `record_bits_u64` and
  write `u64::from(len)` at the `u32` call sites. (If the reviewer prefers a
  narrower diff, keep both names but make the narrow one take `u32`.)

## Why the result is more obviously correct

The `usize` audit of the codec layer then ends at memory indexes and lengths:
every remaining `usize` addresses a buffer. A reader no longer needs to prove
that a pointer-width bit count never exceeds 64.

## Coverage

- `party/io/writer/tests.rs` and the diff table cover the reserved tags;
  `bits/tests.rs` covers `reserve` through the writer's history properties.
- The scan meter's floors and ceilings on the board read the same counter; the
  recorded totals cannot change because every value is unchanged. Run
  `just gate` and require the board leg to reproduce the baseline exactly.
