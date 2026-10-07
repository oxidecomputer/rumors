# L6 observations

Substantive simplifications, documentation findings, design notes, and test
gaps that do not rise to a defect. Each item says whether I verified it or
inferred it. Line numbers are at `58285ca5`.

## Documentation

1. **The borsh error mapping is unstated in public rustdoc.** (Verified by
   reading.) Truncation surfaces as the reader's own `io::Error` of kind
   `UnexpectedEof` (`Decode::Io` passed through, `borsh_impls.rs:139-147`);
   every other rejection is `InvalidData` wrapping the `Decode` variant. The
   suite pins this (`borsh_impls/tests.rs:78`, `:155`, and the decode differentials from `:458`),
   but a user has no documented way to tell a truncated stream from a
   malformed one. The crate page's "Crate features" bullet for `borsh`
   (`lib.rs:398-400`) is the natural home, or the six impls' docs.
2. **Six borsh impls carry no doc comment.** (Verified.) `BorshSerialize` and
   `BorshDeserialize` for `Party`, `Version`, and `Clock`
   (`borsh_impls.rs:149-194`) are undocumented, against AGENTS.md's "Give every
   item a doc comment ... trait impls ... included". The `Rank`, `Ranked`,
   `Span`, and `Count` impls are documented.
3. **Borsh `Span` docs call the endpoints "the meet" and "the join".**
   (Verified.) `borsh_impls.rs:257-262` and `:269-275`. Every other `Span`
   surface says lower and upper endpoint (`lo`/`hi`). The meet/join reading is
   true only by the identity `lo = lo ∧ hi` for an ordered pair; the plain
   names read better and match `Span::encode`'s own doc.
4. **`Decode::Io` both displays and returns its source.** (Verified.)
   `error.rs:124-126`: `#[error("read error: {0}")]` with `#[source]`, so error
   chain renderers (`anyhow`'s `{:#}`, `eyre`) print the inner message twice.
   `error/tests.rs:10-15` pins both behaviors. Owner-ruled territory
   (fresh-eyes-5 in the September review); listed only so the trade is
   visible.
5. **The encoders' partial-write contract is unstated.** (Verified by probe,
   `probe_encoders_under_failing_writers`.) Every `encode_to` and borsh
   `serialize` writes a prefix of the canonical encoding and returns the
   writer's error, but no `# Errors` section says that bytes may already have
   been written. `std::io::Write::write_all` documents the same hazard
   ("unspecified how many bytes were written"); one sentence per `# Errors`
   would inherit it.

## Constant factors

6. **`Ranked` decoding materializes the rank prefix and then discards it.**
   (Verified by reading and by peak-heap readings at four sizes.)
   `ranked.rs:286-293` and
   `borsh_impls.rs:241-251` call `Rank::decode_stream`, which builds the
   integral `BigUint`, the fraction groups, and a `BigUint::from_bytes_be` image
   (`rank.rs:653-714`), only to drop the result and then re-encode
   `version.rank()` through `MatchingWriter` against the prefix bytes. A
   validate-only pass over the rank header and groups (no materialization)
   would remove one `O(‖r‖)` allocation and copy per decode. Small, but it
   changes `rank_decode`-adjacent code that the fuelscape bands pin, so I
   class it as an observation, not a brief. (Comparing the decoded prefix rank
   with `version.rank()` by `==` would be simpler still but raises peak memory
   by one rank, which the current code deliberately avoids.)

   Readings (explore `crates/before/tests/l6_resource.rs`, commit `34e6cf2e`,
   inputs 16 KiB to 1 MiB, ratio constant at every size, so linear): a key
   whose rank prefix is a wide integral and whose version is `Version::new()`
   peaks at 5.00 heap bytes per input byte through `Ranked::decode` and 6.00
   through borsh; a long-fraction prefix peaks at 3.67; decoding a version of
   the same size alone peaks at 2.50. The canonical families on the board never
   produce a rank prefix wider than its version, so the board does not see
   this constant.
7. **`Rank::decode` has two parse paths.** (Verified by reading and by the
   chunked-reader probe.) A 64-byte slice attempt, then a restart through the
   streaming closure (`rank.rs:366-428`). Both are correct under short reads,
   `Interrupted`, and failing readers (probe calibrated: turning
   `Interrupted` into end-of-input in the first loop fails it). A single
   streaming path would be shorter and remove the restart reasoning, but it
   would move `rank_decode`'s wasm fuel, which the fuelscape pins hold, so it
   belongs with a fuel re-pin decision.

## Test-suite gaps (not wrong, but weaker than they could be)

8. **The flush generators reject about seven draws in eight.** (Verified by
   reading.) `arb_flush_version` and `arb_flush_party`
   (`testing/generators.rs:294-310`) filter for live bit lengths divisible by
   eight. The repo standard is that generators build constrained values
   directly. They work today because `prop_filter` rejects locally, but they
   spend most of their draws on rejected values.
9. **The borsh decode differentials share the validators with production.**
   (Verified by reading.) `version_wire_decode_matches_bitwise_reference` and
   its siblings (`borsh_impls/tests.rs:458-760`) call
   `validate::from_reader` and `dominating_from` on both sides, so they test
   the streaming cursor, not canonicity. Canonicity is covered elsewhere
   (single-bit mutations, planted pairs, truncation sweeps), but no committed
   test checks the validators against an independently written specification.
   See the machinery brief.
10. **No committed property compares rejection classes across the serde
    entries.** (Verified by reading.) `serde_impls/tests.rs` checks round trips
    and that a handful of defective payloads are rejected; it never compares
    error classes or the accept set with the slice decoders. The explore
    harness found them in exact agreement (postcard accept set; CBOR error
    message naming an applicable class), so this is a coverage gap, not a
    defect.

11a. **Three decoder checks rest on one or two point tests.** (Verified by
    mutation, `adequacy.log` and `adequacy2.log` in the scratch directory.)
    Raising the rank header's representation bound from `rho >= 64` to
    `rho >= 65` (`rank.rs`, the `TooWide` check) fails only
    `version::tests::rank_decoding_rejects_each_malformed_input_class`.
    Disabling the span admission walk's collapsible-pair rejection, at an
    ordinary close (`admit.rs:129`) or in `finish` (`admit.rs:188-191`),
    fails only `borsh_impls::tests::span_borsh_rejects_collapsible_join` and
    `span::tests::span_decode_rejects_each_malformed_input_class`. Each is
    caught today; a single test edit would blind it. My explore harness
    catches all three as well (it plants violations at every position and
    header widths 64..69), but it adds no class the suite misses, so I do not
    brief it.

## 32-bit

11. **Only `Version::decode` and `Rank::decode` have 32-bit boundary pins.**
    (Verified.) `wasm32-pins/harness/tests/pins.rs:56-110`. `Party::decode_prefix`
    (`party/io.rs:28`), `Span::decode_bytes` (`span/wire.rs:135`), and the borsh
    `StreamBitsReader` position (`borsh_impls.rs:59`) do their boundary
    arithmetic in `u64` and narrow only quantities bounded by a buffer length;
    I inventoried every narrowing conversion in the lane (the list is in the
    coverage record) and found none that can wrap. A pin for any of them needs
    a value of at least 512 MiB whose *first* component crosses `2^32` bits
    (a party, or a span's lower endpoint), roughly doubling the existing
    guest's memory; I judge the cost above the value, absent a constructible
    failure.
