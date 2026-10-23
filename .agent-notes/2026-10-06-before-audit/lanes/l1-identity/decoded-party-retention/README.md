<!-- CAVEAT LECTOR: written by Claude (Opus 5.5) as auditor-o1, measuring observation O1 at main a3a5c4b35 on 2026-10-23. -->

# O1 at `main`: a party taken from a decoded clock keeps the clock's buffer

## Verdict

O1 still holds at `main` (`a3a5c4b35`). #106 seals every party a *writer*
builds, and every party and version *borsh* decodes, in a buffer of exactly
its encoded length. It does not touch the reader, serde-binary, or text decode paths. A
clock decoded through `Clock::decode` or serde's binary form keeps its party
and its version as two slices of one shared buffer. Any route that hands that
party on without rebuilding it keeps the whole buffer alive, plus a 24-byte
`bytes::Shared` header.

No public rustdoc clause is breached. The retention breaches the sizing rule
the owner gave during #106's review, as #106 applies that rule to borsh
decoding. It also conflicts with the private design note on
`Clock::decode_bytes`. Which of the two governs is a question for the owner
(below).

## Mechanism (verified by reading `a3a5c4b35` and `bytes` 1.11.1)

1. `Clock::decode` (`crates/before/src/clock.rs:886-890`) reads the input
   with `read_to_end` into a `Vec`, then converts it with `buf.into()`.
   `Bytes::from(Vec<u8>)` keeps the vector's full capacity: it adopts it
   without a header when `len == cap`, and otherwise allocates a `Shared`
   header that records `cap`. Serde's binary path does the same through
   `deserialize_bytes` (`src/serde_impls.rs:86-96`).
2. `Clock::decode_bytes` (`src/clock.rs:892-898`, documented as "shares its
   storage between the party and version") calls `Party::decode_prefix`
   (`src/party/io.rs:22-33`). That adopts `bytes.slice(..len)` and hands the
   version `buf.slice(party_bytes..)`. `Bytes::slice` is a reference-counted
   clone plus pointer arithmetic, and the first clone promotes the buffer to a
   `Shared` header (3 words: 24 bytes on 64-bit, 12 inferred on 32-bit).
3. Each part therefore pins the whole allocation. Once one part is dropped or
   replaced, the survivor holds the other part's dead bytes too.

The writer seal that #106 added (`Bits::from_canonical_vec`,
`src/bits/storage.rs:62-71`) runs only on writer output and borsh decoding.
Its comment states the reason it exists: "A sealed value lives as long as its
holder keeps it, so we release any spare capacity first."

## Readings

Instruments, both committed:

- **Live heap**: `alloc_meter::measure` around a closure that decodes, applies
  the route, and returns only the measured value. `live` is the reading's
  `net_bytes`: the heap the surviving value holds. `own` is the value's
  `as_bytes().len()`.
- **Capacity**: #106's `spare_capacity` helper (`src/party/tests.rs:181-196`),
  which reads `Bits::allocation_capacity`.

Command, test execution 1 of 8, exit status 0 (`2 tests run: 2 passed, 650
skipped`); the full log is `run-1.log` in this directory:

```
/Users/oxide/.claude/skills/building-on-illumos/scripts/on-illumos.sh /Users/oxide/src/rumors-slot-70 'unset CARGO_TARGET_DIR; export NEXTEST_TEST_THREADS=24; nice -n 10 cargo nextest run -p before --all-features --locked --lib --test o1_probe --no-capture -E "test(/o1_probe/)"'
```

The probe sources are `o1_probe.rs` (live heap, an integration test) and
`o1_probe_capacity.patch` (capacity, appended to `src/party/tests.rs`), both in
this directory. Both were removed from the worktree after the run.

### Family

Clock = the seed party (1 byte) beside a version whose only events sit in the
rightmost `2^-d` cell, at d = 100, 1,000, 10,000, and 100,000. The clock's
encoded length `n` is 40, 377, 3,752, and 37,502 bytes. The two-clock routes
pair a deep comb `[0, 1 - 2^-d)` with its complementary cell, each beside the
same version. Their `n` is the larger clock's length.

### Per route

All readings below are measured. `n` is the length of the clock whose buffer
the party came from.

| Route from a decoded clock to a party | Class | `live` at the four sizes (bytes) | As a function of `n` |
|---|---|---|---|
| `Clock::decode(&[u8])`, `into_parts` | retaining | 64; 401; 3,776; 37,526 | `max(n, 8) + 24` |
| `Clock::decode` of a reader implementing only `read`, `into_parts` | retaining | 88; 536; 4,120; 65,560 | `C(n) + 24`, with `C(n)` = `read_to_end`'s doubled capacity: 64; 512; 4,096; 65,536 |
| serde binary via postcard, `into_parts` | retaining | 64; 401; 3,776; 37,526 | `n + 24` |
| serde binary via ciborium, `into_parts` | retaining | 64; 401; 3,776; 65,560 | `n + 24`, or ciborium's grown read buffer + 24 for long byte strings |
| serde human-readable via serde_json, `into_parts` | independent of `n` | 32; 32; 32; 32 | constant: the hex path's 8-byte minimum buffer + 24 for a 1-byte party (adjacent finding below) |
| borsh, `into_parts` | exact | 1; 1; 1; 1 | `|party|` |
| `party().dangerously_alias()`, clock dropped | retaining | 64; 401; 3,776; 37,526 | `n + 24` |
| `tick()`, then `into_parts` | retaining | 64; 401; 3,776; 37,526 | `n + 24` |
| `tick()`, whole clock kept (own = 2 bytes) | retaining | 65; 402; 3,777; 37,527 | `n + 24 + |new version|` |
| `forks(0)` (or a fork iterator dropped before its first `next`), residual | retaining | 64; 401; 3,776; 37,526 | `n + 24` |
| `sync_all` over no others | retaining | 64; 401; 3,776; 37,526 | `n + 24` |
| `<[Clock; 1]>::from` | retaining | 64; 401; 3,776; 37,526 | `n + 24` |
| `join` that fails: the returned operand's party | retaining | 89; 651; 6,276; 62,526 (n = 65; 627; 6,252; 62,502) | `n + 24` of the operand's clock |
| `into_parts`, then `Party::without` a disjoint party | retaining | 89; 651; 6,276; 62,526 (same `n`) | `n + 24` |
| `fork()`: kept party, and child party | exact | 1; 1; 1; 1 | `|party|` |
| `forks(1)`: residual | exact | 1; 1; 1; 1 | `|party|` |
| `<[Clock; 2]>::from`: first share | exact | 1; 1; 1; 1 | `|party|` |
| `join` that succeeds (comb with cell) | exact | 1; 1; 1; 1 | `|party|` |
| `sync`: both parties | exact | 1; 1; 1; 1 | `|party|` |
| `sync_all` over one other: receiver | exact | 1; 1; 1; 1 | `|party|` |
| Dual, a version: `decode`, `into_parts`, party dropped (deep comb party) | retaining | 76; 526; 5,026; 50,026 (n = 52; 502; 5,002; 50,002) | `n + 24` |

The capacity instrument agrees route for route (`O1CAP` lines in `run-1.log`).
Examples at d = 100,000: "decode(&[u8]) into_parts: a 1-byte party keeps a
37503-byte buffer"; "decode(plain) into_parts: a 1-byte party keeps a
65536-byte buffer"; "forks(0) residual: a 50001-byte party keeps a
87503-byte buffer"; and "exact" for `fork`, `forks(1)`, and `sync`. (The
capacity probe puts its version's events in the leftmost cell rather than the
rightmost, so its `n` differs from the live-heap probe's by at most one
byte.)

### The two classes, and why

- **Exact**: every route whose result comes out of a party writer (`fork`,
  `forks(k ≥ 1)`, a successful `join`, `sync`, `sync_all` over others,
  `[Clock; N ≥ 2]`). Those results are sealed by `from_canonical_vec`, and
  `live == own` shows they are adopted without a header. #106's
  `built_parties_are_sized_exactly` already covers them over arbitrary
  operands.
- **Retaining**: every route that moves or aliases the decoded party without
  rebuilding it. `into_parts` and `dangerously_alias` hand the slice out.
  `tick`, `ticks`, `send`, `recv`, `absorb`, and the `|` operators replace
  only the version. `forks(0)`, `sync_all` over no others, and `[Clock; 1]`
  pass the party through `into_shares(1)` or an empty plan (`src/party/forks.rs:233-235`
  and `:315-321`). A failed `join` returns its operand. `Party::without`
  returns `self` when the parties are disjoint (`src/party.rs:532-533`). The
  retained excess is `n + 24 - |party|` with a slice reader, and up to about
  `2n + 24 - |party|` with a stream reader (inferred from std 1.97.1's
  doubling `default_read_to_end`; measured at 1.6n to 1.75n here).

Per input byte the retention is at most 1 (slice) or about 2 (stream reader).
Per output byte it is unbounded: at d = 100,000, a running clock whose live
state is 2 bytes holds 37,527.

## Judgment against the contract

- **Public rustdoc: no breach.** The crate's space promise
  (`src/lib.rs:333-340`) bounds *transient* auxiliary space: "the auxiliary
  space required to compute any operation is at most a small constant
  multiple of the input size". A result's retained heap is not auxiliary
  space, and it never exceeds about twice the decoded input. The complexity
  lines of `Clock::decode` and `into_parts` state time. No other public item
  promises how much heap a value holds. The "Space Efficiency" figures
  (`src/lib.rs:310-319`) speak of encoded sizes, not heap.
- **#106's sizing rule: breached, by its own precedent.** The owner's ruling
  during #106's review (recorded in
  `lanes/l1-identity/round-1/S1-result-buffer-retention.md`) reads: "Sealed,
  non-transient results are sized exactly: sealing shrinks whenever capacity
  exceeds length, which also lets `Bytes` adopt the buffer without a shared
  header until its first clone." #106 treats a decoded value as such a
  result: it moved borsh decoding onto `from_canonical_vec` and committed
  `borsh_decoded_values_are_sized_exactly` (`src/borsh_impls/tests.rs`), whose
  doc names the very situation the reader paths are in: "The reader collects
  the encoding byte by byte into a growing vector, which usually ends with
  spare capacity before the value is sealed." The reader and serde-binary
  decodes of the same types skip that seal, and the clock decode also shares
  one buffer between its parts.
- **Conflict to rule on.** `Clock::decode_bytes`'s doc states the sharing as
  the design ("shares its storage between the party and version"), and the
  identity lane declined to brief O1 because it "reverses a stated design
  choice". That note predates the #106 ruling. The two now disagree, and I
  have not picked a winner.

## Question for the owner

Does the #106 rule (sealed values sized exactly) govern decoded values on
every entry point, so that a decoded clock's parts get separate, exact
buffers? Or does `Clock::decode_bytes`'s shared-storage design stand, with the
retention accepted as a declared trade? The brief below assumes the former,
which is what the borsh precedent implies.

## Demonstrator brief

Write one commit on a branch from `main` that adds a failing test and nothing
else.

**Invariant.** A value decoded from bytes holds no more heap than its own
encoding, on every decode entry point. In particular, a party taken from a
decoded clock holds only its own bytes, whether the clock was split by
`into_parts` or kept running after its version was replaced.

**Where.** `crates/before/tests/meter/encode_allocation.rs`, beside
`clock_encode_allocates_once_at_its_length`. Widen the module doc from
"composite encoders" to the composite codecs: decoding is the other half of
the same allocation claim. Reuse `history` and `step` so the clocks are
realistic: fork shares of one seed carrying received events. The binary runs
under `--all-features`, as the landing check runs it.

**Why this instrument and not `allocation_capacity`.** For a slice at a
nonzero offset, `Bits::allocation_capacity` reports the capacity *after* the
offset. A version sliced from an exactly sized buffer after a copied-out party
would read "exact" while it still pinned the party's bytes and a 24-byte
header. `alloc_meter`'s `net_bytes` counts the whole allocation and the
header, so it measures exactly what the ruling promises.

**Form.** One proptest over the module's generated histories:

```rust
proptest! {
    /// A clock decoded from its encoding hands out parts that each hold no
    /// more heap than their own encoding.
    ///
    /// The decode shares nothing between the party and the version, so
    /// dropping or replacing either one releases its bytes. The cases are a
    /// split by `into_parts`, and a clock that keeps running after `tick`
    /// replaces its version.
    #[test]
    fn decoded_clock_parts_hold_only_their_encodings(
        clocks in 1usize..=8,
        steps in vec(step(), 0..48),
        pick in any::<Index>(),
    ) {
        let pool = history(clocks, &steps).clocks;
        let bytes = pool[pick.index(clocks)].encode();
        let decode = || Clock::decode(&bytes[..]).expect("a clock's own encoding decodes");
        let mut excess = Vec::new();

        let (stats, party) = alloc_meter::measure(|| decode().into_parts().0);
        excess.extend(over("decode, into_parts: party", stats, party.as_bytes().len()));
        let (stats, version) = alloc_meter::measure(|| decode().into_parts().1);
        excess.extend(over("decode, into_parts: version", stats, version.as_bytes().len()));
        let (stats, party) = alloc_meter::measure(|| {
            let mut running = decode();
            running.tick();
            running.into_parts().0
        });
        excess.extend(over("decode, tick, into_parts: party", stats, party.as_bytes().len()));

        prop_assert!(excess.is_empty(), "{}", excess.join("; "));
    }
}
```

with a helper documented as "Describe a measured value's heap beyond its
encoded length, if it holds any":

```rust
fn over(case: &str, stats: Stats, encoded: usize) -> Option<String> {
    let held = usize::try_from(stats.net_bytes).expect("the closure freed nothing it did not allocate");
    (held > encoded).then(|| format!("{case}: a {encoded}-byte value holds {held} bytes of heap"))
}
```

The assertion is `held <= encoded`, not equality. A fix may hand out the
static seed or the static empty version, which hold no heap.

**Optional second case (the adjacent finding below).** In the same property,
or a sibling one, decode the picked clock's party alone through
`Party::decode(&party_bytes[..])` and through a reader that implements only
`read` (a two-line local struct wrapping `&[u8]`), and apply the same `over`
check. Include it only if the coordinator scopes the adjacent finding into
this branch.

**Failure on the base commit.** Every case fails, because every version
encodes in at least one byte (`Version::new` is one static byte), so a decoded
clock's buffer is always longer than its party. The smallest case, one clock
and no steps, is the seed clock: 2 bytes, which `read_to_end` from a slice
places in a buffer of the 8-byte minimum capacity, adopted under a 24-byte
header. The expected message is then
`decode, into_parts: party: a 1-byte value holds 32 bytes of heap; decode,
into_parts: version: a 1-byte value holds 32 bytes of heap; decode, tick,
into_parts: party: a 1-byte value holds 32 bytes of heap`.
The figures are inferred from the measured `Party::decode(&[u8])` reading for
a 1-byte input (32 bytes); the demonstrator must quote the actual output.

**What not to do.** Do not assert a pinned byte count for any case; the
contract is "no more than its encoding". Do not filter out the seed clock.

## Fix sketch

Decode every value into storage sealed the way #106 seals writer output.

1. Change the owned decode entry points (`Party::decode_bytes`,
   `Version::decode_bytes`, `Clock::decode_bytes`, `Span::decode_bytes`,
   `Ranked::decode_bytes`) to take the `Vec<u8>` their callers already own
   (`read_to_end`'s buffer, `serde_bytes::ByteBuf::into_vec`, `hex::decode`'s
   output) instead of `bytes::Bytes`. Adopt it through
   `Bits::from_canonical_vec`, which shrinks it and lets `Bytes` take it
   without a header.
2. In `Clock::decode_bytes`, after validating the party prefix, copy the
   party's bytes into their own vector (`buf[..len].to_vec()`, sealed by
   `from_canonical_vec`). Then `buf.drain(..len)` moves the version to the
   front, and the version seals the remaining vector. The copy costs `|party|`
   bytes and the drain one move of the version, both linear, so
   `Clock::decode`'s `O(n)` time bound is unchanged. `Span::decode_bytes`
   (both endpoints share one buffer, `src/span/wire.rs:136-167`) and
   `Ranked::decode_bytes` (the version pins the rank's prefix bytes,
   `src/ranked.rs:283-296`) share the same mechanism and take the same repair.
   Both are inferred by reading, not measured.
3. `Party::decode_prefix` and `Version::decode_prefix` exist only to slice a
   shared buffer; their callers are `Clock::decode_bytes` and
   `Span::decode_bytes`. After the change, each reduces to a validator that
   returns the prefix's byte length.

Public API and wire and storage formats are unchanged. Every function named
is `pub(crate)`.

**Risk the fixer must check.** `shrink_to_fit` can reallocate, adding the new
block to the decode's peak while the old one is live, and the party copy adds
`|party|`. Run the board and compare `clock_decode`,
`clock_serde_deserialize`, `party_decode`, `party_parse`,
`party_serde_deserialize`, `span_decode`, `ranked_decode`, and their
`version_*` counterparts against `baseline.md`. A ceiling that would rise, or a
worst-case ranking that flips, stops the work for the owner.

## Adjacent finding: standalone decodes keep spare capacity

This is the same root cause without the sharing: decode paths adopt a growing
vector without the seal. It is measured in the same run, as `ADJ` lines.

| Entry point | `own`, then `live` (bytes) |
|---|---|
| `Party::decode(&[u8])` | 1, 32; 3, 32; 8, 8; 26, 26; 251, 251; 2,501, 2,501; 25,001, 25,001 |
| `Party::decode` of a reader implementing only `read` | 1, 32; 3, 32; 8, 64; 26, 82; 251, 280; 2,501, 4,120; 25,001, 32,792 |
| `Party::from_str` | 1, 32; 3, 32; 8, 8; 26, 56; 251, 280; 2,501, 4,120; 25,001, 32,792 |
| serde via ciborium | 1, 32; 3, 32; then exact up to 2,501; 25,001, 32,792 |
| serde via postcard | exact at every size |
| `Version::decode` of a reader implementing only `read` | 39, 88; 376, 536; 3,751, 4,120; 37,501, 65,560 |

A slice decode under 8 bytes lands in the 8-byte minimum buffer under a
24-byte header, so the crate docs' typical 3-byte party occupies 32 bytes. A
stream decode keeps `read_to_end`'s doubled capacity, as does `hex::decode`.
Postcard is exact because `serde_bytes` copies borrowed bytes into an exact
vector. Borsh is exact because #106 seals it. The fix sketch above closes this
too, since every owned decode path would seal through `from_canonical_vec`.
`rumors` decodes parties this way (`Party::decode(bytes)` with a `&[u8]` in
`src/tree/mirror/party.rs:125`), so this finding reaches `rumors`. O1 itself
does not: `rumors` calls no clock decode in non-test code (verified by grep).

## Coverage record and blind spots

- Established at `a3a5c4b35` with one `--no-run` build and one test
  execution (1 of 8 allowed), both exit status 0. Both instruments are shown
  to register retention in the same run, on the pass-through routes. That
  confirms they can fail, so their "exact" readings on the rebuilding routes
  carry weight without a separate calibration.
- Not measured: 32-bit targets (the header is 12 bytes there, inferred);
  `Span` and `Ranked` decodes; `Version::from_str` and serde human-readable
  versions (same `hex::decode` mechanism, inferred); `join_all`'s error path
  (it returns unjoined operands unchanged, like `join`, inferred by reading
  `src/clock.rs:337-347`).
- The worktree is reverted and clean. The box's copy at
  `~/src/rumors-slot-70` still holds `tests/o1_probe.rs` until the next sync's
  `--delete` removes it.
