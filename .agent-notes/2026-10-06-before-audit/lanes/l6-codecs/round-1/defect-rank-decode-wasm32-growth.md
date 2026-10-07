# Defect: decoders panic on wasm32 once an input-proportional buffer passes 2^30 bytes

Demonstrated at two independent sites: `Rank::decode`'s fraction-group
buffer, and borsh's stream buffer (every borsh `Party`/`Version`/`Clock`/`Span`
decode).

Severity: medium. The trigger is a canonical input of about 1.13 GiB, only on
targets whose `usize` is 32 bits; the result is a panic where the contract
promises a value or a documented error. Status: **demonstrated** on
ox-east-1 through `wasm32-pins` (explore commit `679cc2e5`). Root cause
isolated. Credit: inferred first by the L4 auditor
(`.agent-notes/2026-10-06-before-audit/lanes/l4-measures/round-1/cross-lane-l6-rank-decode-wasm32.md`);
I evaluated, generalized, and demonstrated it.

## Contract clauses breached

- `Rank::decode`'s `# Errors` (`crates/before/src/rank.rs:332-342`) lists
  `Truncated`, `TrailingBits`, `NotCanonical` (header width outside the
  format's `u64` range), and `Io`. There is no `# Panics` section and no size
  limit. The audit contract (`briefs/common.md`) requires every operation to be
  total and panic-free over any canonical input.
- The `usize`-invariance clause (`briefs/common.md`): the same input decodes on
  a 64-bit target and panics on a 32-bit one.

The input is legitimate. It is the canonical encoding of
`2^-(8·(2^30+1))`, whose stored form on wasm32 is `num = 1`,
`exp = 8,589,934,600` (a `u64`), a few bytes. A 64-bit peer produces such
ranks from versions about 1 GiB deep.

## Reproduction (verified)

Explore commit `679cc2e5` adds a guest check `RankDecodeGroups(g)`. It streams
a lazily generated canonical `2^-(8g)` through `Rank::decode` from a `Read`
with no input buffer: integral header `0`, then `g` groups whose digits are
all zero but the last, then the close bit. For `g <= 4096` the check also
compares the result with the existing in-memory `synthesis::unit_fraction`,
which validates the lazy stream. Command (on the box, after syncing the
worktree):

```
on-illumos.sh /Users/oxide/src/rumors-audit-l6-codecs 'unset CARGO_TARGET_DIR; root=$PWD; cd crates/before/wasm32-pins; cargo build --locked -p wasm32-pins-guest --release --target wasm32-unknown-unknown --target-dir $root/target/wasm32-pins && cargo build --locked -p wasm32-pins-harness --tests --release && WASM32_PINS_GUEST_WASM=$root/target/wasm32-pins/wasm32-unknown-unknown/release/wasm32_pins_guest.wasm cargo nextest run --locked --cargo-profile release -E "test(/l6_/)" --test-threads 4 --success-output immediate'
```

Exit status 0 (the probes print rather than assert). Output, verbatim
(`wasm1.log`):

```
L6 groups 1000: Passed
L6 groups 4096: Passed
L6 vec 2^30: Passed
L6 groups 2^30+1: Trapped(UnreachableCodeReached)     (124 s)
L6 groups 2^30: Passed                                (128 s)
```

`L6 vec 2^30: Passed` comes from a second probe, `VecGrowthBoundary(2^30)`.
On a full `Vec<u8>` of length and capacity `2^30`, `try_reserve(1)` fails
with the *capacity-overflow* error ("computed capacity exceeded the
collection's maximum"), not an allocator error.

### Second site: borsh's stream buffer (verified)

Explore commits `3854b4c1` and `431011b3` (the guest gains `before`'s `borsh`
feature; the pins lockfile adds `borsh` 1.6.1 and `cfg_aliases` 0.2.1, the
versions the main workspace locks). `WideLeafReader(k)` lazily streams the
canonical one-leaf version of height `2^k` (`ceil((2k + 3) / 8)` bytes).
`VersionBorshWideLeaf(k)` decodes it with `Version::deserialize_reader`;
`VersionReaderWideLeaf(k)` decodes the same stream with `Version::decode`.
Same command shape, filter `test(/l6_version/)`; output verbatim
(`wasm2.log`):

```
L6 borsh k=1000: Passed
L6 reader k=1000: Passed
L6 reader 2^30+5 bytes: Failed(OutOfMemory)          (16 s)
L6 borsh 2^30+5 bytes: Trapped(UnreachableCodeReached)  (142 s)
L6 borsh 2^30 bytes: Passed                           (148 s)
```

The at-limit borsh decode (`k = 2^32 - 2`, exactly `2^30` bytes) completes
its whole validation, including a `2^32`-bit gamma code and the height
accumulator. The over-limit decode (`k = 2^32 + 16`) traps when
`StreamBitsReader.bytes` (`borsh_impls.rs:104`) grows past `2^30`. The reader
entry gets `Decode::Io(OutOfMemory)` instead, from `std`'s fallible
`read_to_end`. So one input yields a value on 64-bit, a panic through borsh
on wasm32, and an `Io` error through `Version::decode` on wasm32.

## Why the trap is the capacity panic, not memory exhaustion

The `2^30` case succeeds, and it peaks *higher* than the `2^30 + 1` case does
before trapping: it holds a 1 GiB group buffer and a 1 GiB byte image at
once, while the `+1` case traps at the first growth of the group buffer past
`2^30`. Infallible `Vec::push` routes the same refusal that `try_reserve`
reported into `alloc::raw_vec::capacity_overflow()`, a panic, which wasm32
lowers to `unreachable`. When the builder's panic-message capture
(`audit/wasm32-trap-diagnosis`) lands, the message should read "capacity
overflow".

## Root cause

`Rank::decode_stream` buffers every fraction group in
`let mut groups: Vec<u8> = Vec::new();` with `groups.push(group)`
(`rank.rs:660`, `:669`). Amortized growth doubles the capacity: 8, 16, ...,
`2^30`. The push after that requests capacity `2^31`, which exceeds
`isize::MAX` on a 32-bit target, so `Layout::array::<u8>` fails and `push`
panics. The limit has nothing to do with memory actually available: wasm32
has 4 GiB, and the decode at `2^30` groups fits.

## Failure family

- Appears: any canonical rank stream needing more than `2^30` entries in an
  infallibly grown buffer, on a 32-bit target, through any entry point that
  reaches `decode_stream`: `Rank::decode`, `Rank`'s borsh and binary serde
  impls, and `Ranked::decode` and its borsh/serde impls. Dense and sparse
  fractions alike.
- Disappears: at or below `2^30` groups (demonstrated); on 64-bit targets
  (by inspection: `isize::MAX` is `2^63`).
- Demonstrated a second time at borsh's `StreamBitsReader.bytes`
  (`borsh_impls.rs:104`), above. The same mechanism, inferred and not
  demonstrated, at other input-proportional buffers grown by `push`:
  - the integral mantissa `BitSink` (`rank.rs:741-748`) for an integral part
    over `2^33` bits;
  - `Ranked`'s borsh `rank_bytes` (`borsh_impls.rs:245`);
  - `BitStack.words` (`bits/stack/bit.rs:42`), the validators' frame stacks,
    for trees over about `2^32` levels deep;
  - `BitsWriter` growth (`bits/writer.rs:147`, `:197-202`) when an
    operation's output outgrows its capacity hint.
- A related, non-panicking variant, demonstrated for `Version::decode`: the
  `read_to_end`-based decoders (`Party`, `Version`, `Clock`, `Span`,
  `Ranked::decode` from a non-slice reader) get `std`'s fallible growth and
  return `Decode::Io(OutOfMemory)` past `2^30` bytes on 32-bit. That is total
  but target-dependent. Reading from a `&[u8]` reserves exactly and does not
  hit it.

## API and format preservation

Any repair is internal; the wire format is untouched. Whether the repaired
decoder should *succeed* for representable values or return a typed error for
"too large for this target" is a design question (question Q4): `Decode` is
`#[non_exhaustive]`, so a new variant is additive but still a public API
addition that needs the owner's ruling.

## Owner's ruling

Option A. A decoder succeeds whenever the decoded value fits in the target's
memory: its buffers grow without doubling past the target's limit, reserving
with `try_reserve`. When the value cannot fit, the decoder returns
`Decode::Io` with `ErrorKind::OutOfMemory`, the error the reader path already
returns, so every entry point agrees. No new `Decode` variant.


## Owner's ruling on the suanpan site

The `before`-side fix found that the same mechanism remains in suanpan: a
dense height accumulates in suanpan's `Vec<i64>` digit storage, which grows by
doubling, so decoding a version whose leaf holds a dense height of about
2^32 bits (about 1 GiB of input) still traps on wasm32. The owner ruled to
accept that panic: on 32-bit targets, suanpan's storage growth past 1 GiB
panics, and suanpan's growth stays as it is. The documentation states the
limit.

The owner then extended the same rule to `before`'s own decoder buffers,
which supersedes option A for 32-bit growth: on 32-bit targets, a decode
whose buffers grow past 1 GiB panics, and the documentation states the
limit. Supporting the rest of a 32-bit address space would take growth
machinery at every buffer, including some that live in dependencies
(`serde_bytes`, `num-bigint`) and in std's composite readers, and the owner
judged that effort too large. The fix branch keeps its wasm32 pins,
restated to assert the documented limit, and drops its growth machinery.
