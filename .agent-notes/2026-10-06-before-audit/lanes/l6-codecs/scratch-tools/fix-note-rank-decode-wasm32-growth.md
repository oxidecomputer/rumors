# Fix note: input-proportional buffers that panic past 2^30 entries on 32-bit targets

Root cause, as I understand it: decoders grow input-proportional buffers with
infallible `Vec::push`. Amortized doubling asks for `2 · 2^30` bytes once a
`Vec<u8>` holds `2^30`, which exceeds `isize::MAX` on a 32-bit target, and
`push` turns that into `capacity_overflow()`, a panic. Memory need not be
exhausted. The demonstrated sites are `Rank::decode_stream`'s `groups`
(`rank.rs:660-669`) and borsh's `StreamBitsReader.bytes`
(`borsh_impls.rs:104`).

## Candidate repairs (none ruled)

1. **Stop buffering what the value does not need (rank only).** Count leading
   all-zero groups in a `u64` instead of storing them, and assemble the
   numerator from the first nonzero group on, ideally straight into `u64`
   limbs rather than through a byte image (`rank.rs:690-714` builds
   `integral.to_bytes_be()` plus every group, then
   `BigUint::from_bytes_be`). This makes the demonstrated sparse input cost
   `O(1)` memory and removes one full-size intermediate copy for dense inputs.
   It does not by itself make dense ranks total: a dense numerator of
   `2^34` bits cannot exist in a 32-bit address space at all.
2. **Fallible growth everywhere input drives allocation.** Use
   `try_reserve` at each input-proportional growth and map refusal to an
   error. `Decode` has no variant for "too large for this target". It is
   `#[non_exhaustive]`, so adding one is source-compatible, but it is a public
   API addition and needs the owner's ruling (question Q4).
   `Decode::NotCanonical` would misstate the cause; `Decode::Io(OutOfMemory)`
   matches what `std`'s `read_to_end` already returns for the reader entry
   points, but `Io` is documented as "the underlying reader failed".
3. **Document a platform limit** (`# Panics` on every affected decoder).
   This is the weakest option, and it conflicts with the panic-free contract.

Repairs 1 and 2 compose: repair 1 keeps representable sparse values decodable,
and repair 2 turns everything else into a documented error.

## Sites sharing the mechanism (inferred unless marked demonstrated; verify each)

- `rank.rs:741-748`, `BitSink::push` for the integral mantissa: the same
  doubling past `2^30` bytes, which takes an integral part over `2^33` bits.
- `borsh_impls.rs:104`, `StreamBitsReader::read_bit`'s `bytes.push`
  (**demonstrated**): every borsh `Party`, `Version`, `Clock`, `Span` decode
  over `2^30` bytes. The same bytes decode through `T::decode(&slice)` on the
  same target, because `<&[u8] as Read>::read_to_end` reserves exactly. The
  encoding's length is unknown in advance, so this buffer is inherently
  input-proportional; route its growth through repair 2.
- `borsh_impls.rs:245`, `Ranked`'s borsh `rank_bytes.push`.
- `bits/stack/bit.rs:42`, `BitStack::push`'s `words.push`: a `Vec<u64>`
  hits `isize::MAX` at `2^28` words, so doubling fails at `2^27` words,
  `2^33` bits of stack, roughly trees `2^32` levels deep (party validator:
  two bits per open branch). The inputs needed are about 1 to 1.5 GiB.
- `bits/writer.rs:147` and `:197-202`, `BitsWriter` appends: operation
  outputs that outgrow their `with_capacity` hint.
- Not a panic, but target-dependent: the `read_to_end`-based decoders return
  `Decode::Io(OutOfMemory)` past `2^30` bytes from non-slice readers on
  32-bit (`std` grows fallibly), while a 64-bit build decodes.

The `Rank::sum_iter` comment cited by L4 (`rank.rs`, the "under 2^35 even if a
whole 32-bit address space were one fraction" premise) currently holds only
because this panic stops the decoder first; a repair that makes long
fractions decodable must recheck that premise against `suanpan`'s `2^37`
digit-position limit.
