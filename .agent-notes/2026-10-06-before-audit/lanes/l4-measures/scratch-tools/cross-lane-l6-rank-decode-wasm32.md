# Cross-lane lead for L6: `Rank::decode` on wasm32 panics on a long fraction

Status: inferred from reading the code and the standard library's `Vec`
growth rule; NOT demonstrated. Routed to L6 (codec mechanics) per the L4 lane
section; I did not build a reproduction.

## Mechanism

`Rank::decode_stream` (`crates/before/src/rank.rs:660-670`, base 58285ca5)
buffers every fraction group in `let mut groups: Vec<u8> = Vec::new();` with
`groups.push(group)`, one byte per nine input bits, until the closing bit.
The buffer is fed only by bits actually read, as documented, so a streaming
reader can grow it without bound.

On a 32-bit target, `Vec<u8>` capacity cannot exceed `isize::MAX = 2^31 - 1`.
Amortized growth doubles: at `len == cap == 2^30` the next `push` requests
capacity `2^31`, `Layout::array::<u8>(2^31)` fails, and `RawVec` calls
`capacity_overflow()`, which **panics** ("capacity overflow"). That is
distinct from allocator exhaustion, which aborts via `handle_alloc_error`.
Reaching it needs `2^30` groups, about `9 * 2^30 / 8` bytes = 1.13 GiB of
canonical input through `Rank::decode` (incremental `Read`) or
`BorshDeserialize for Rank` (byte-at-a-time `read_exact`). The groups buffer
then holds 1 GiB, after one 512 MiB-to-1 GiB reallocation (1.5 GiB transient),
which is inside wasm32's 4 GiB address space.

## Contract clause at stake

- `Rank::decode` `# Errors` (`rank.rs:332-341`) lists `Truncated`,
  `TrailingBits`, `NotCanonical` (integral header width), and `Io`; it lists
  no size-limit error and no `# Panics` section.
- Audit contract: every operation is total and panic-free over any input; an
  input that cannot be handled gets a documented error. 32-bit targets are in
  scope.

## Why the input is legitimate

The stream is a canonical encoding of a value `Rank` can represent on wasm32:
for example `2^-(2^33)` is stored as `num = 1, exp = 2^33` (a few bytes). A
64-bit peer produces such a rank from any version of depth `2^33`, which is
about 1 GiB stored, and its canonical encoding is ~1.13 GiB. A wasm32 decoder
of that stream panics instead of returning an error. The same wasm32 process
can `encode_to` a streaming writer without trouble (`BitWriter` stages 256
bytes), so the encode/decode round trip is asymmetric.

## Related comment premise

`Rank::sum_iter`'s comment (`rank.rs:1035-1043`) argues that a decoded
exponent stays "under 2^35 even if a whole 32-bit address space were one
fraction", which is the bound that keeps suanpan's documented `2^37`
digit-position panic unreachable. That premise holds today only because this
`Vec` growth panic (or allocator exhaustion) stops the decoder first: the
fraction length is bounded by a panic, not by an error.

## Suggested demonstration (for L6 or a demonstrator)

A new `wasm32-pins` check with a lazily synthesized reader (no input buffer):
header bit `0`, then `2^30 + 1` groups whose bodies are all zero except a
final `0x80` group, then the close bit. Expected on base: the guest traps
(panic -> `unreachable`), where a total decoder would return an error.
Cost: about 1.2 GB streamed through the decoder inside the guest; the existing
`RankDecode` pin already streams 604 MB, so this is ~2x that.

## Candidate repairs (for the fix note, not ruled)

1. Count leading all-zero groups instead of storing them, and grow storage
   only from the first nonzero group; this makes sparse fractions cost
   O(value) memory but does not bound dense ones.
2. `groups.try_reserve(1)` and map failure to a `Decode` error. No existing
   variant names "too large for this platform"; `NotCanonical` would stretch
   its documented meaning ("violates canonical-form or semantic invariants"),
   so this is a design question (new variant = public API change).
3. Document a `# Panics` / platform limit instead (weakest).

Shared mechanism: `Version::decode` and any other decoder that buffers
input-proportional state in a `Vec` on wasm32 should be checked for the same
capacity-overflow panic.
