//! Documentation goldens (insta inline snapshots).

use insta::assert_snapshot;

use crate::codec::{encode_int, Base, BitsBuf, BitsView};
use crate::error::{Crossed, Decode, Overlap};
use crate::oracle;
use crate::testing::bridge::{from_oracle_party, from_oracle_version};
use crate::{Clock, Party, Rank, Version};

/// Render a bit stream most-significant-bit-first as a string of `'0'`/`'1'`, the same
/// order `encode_int` and the preorder codec emit. Empty stream renders as `""`.
fn bits_to_string(bits: BitsView<'_>) -> String {
    (0..bits.len())
        .map(|i| if bits.bit(i) { '1' } else { '0' })
        .collect()
}

/// Render bytes as space-separated two-digit hex, e.g. `[0x80, 0x01]` -> `"80 01"`.
fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// One Elias-gamma row: `n` then its code as an MSB-first bit string and the bit count.
fn gamma_row(n: u64) -> String {
    let mut bits = BitsBuf::new();
    encode_int(&mut bits, &Base::from(n));
    format!(
        "{:>20} -> {} ({} bits)",
        n,
        bits_to_string(crate::codec::built_view(&bits)),
        bits.len()
    )
}

/// Elias-gamma code of `m = n + 1`: `floor(log2 m)` leading zeros, then `m` in
/// `floor(log2 m) + 1` bits MSB-first.
///
/// The golden table pins the layout across
/// magnitudes — powers of two and their neighbours (where the unary prefix grows), plus
/// a value past `u64::MAX` to witness the arbitrary-width code: the integer
/// magnitude has no cap, so the code must extend cleanly beyond 64 bits.
#[test]
fn gamma_bit_layout_table() {
    let small: String = [0u64, 1, 2, 3, 4, 5, 6, 7, 8, 15, 16, 17, 255, 256]
        .into_iter()
        .map(gamma_row)
        .collect::<Vec<_>>()
        .join("\n");
    assert_snapshot!(small, @"
      0 -> 1 (1 bits)
      1 -> 010 (3 bits)
      2 -> 011 (3 bits)
      3 -> 00100 (5 bits)
      4 -> 00101 (5 bits)
      5 -> 00110 (5 bits)
      6 -> 00111 (5 bits)
      7 -> 0001000 (7 bits)
      8 -> 0001001 (7 bits)
     15 -> 000010000 (9 bits)
     16 -> 000010001 (9 bits)
     17 -> 000010010 (9 bits)
    255 -> 00000000100000000 (17 bits)
    256 -> 00000000100000001 (17 bits)
    ");

    // Arbitrary-width witness: 2^64 has no u64 representation, but the gamma code (and
    // therefore an event base of this magnitude) encodes and round-trips regardless.
    let mut big_bits = BitsBuf::new();
    let big = Base::from(1u8) << 64u32; // 2^64
    encode_int(&mut big_bits, &big);
    assert_snapshot!(
        format!(
            "2^64 -> {} ({} bits)",
            bits_to_string(crate::codec::built_view(&big_bits)),
            big_bits.len()
        ),
        @"2^64 -> 000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000001 (129 bits)"
    );
}

/// Render a party's debug form and binary encoding.
fn party_block(p: &Party) -> String {
    format!(
        "debug:   {p:?}\nbits:    {} ({} bits)\nbytes:   {}",
        bits_to_string(p.as_bits()),
        p.as_bits().len(),
        bytes_to_hex(&p.encode()),
    )
}

/// Render a version's debug form and binary encoding.
fn version_block(v: &Version) -> String {
    let bits = v.as_bits();
    format!(
        "debug:   {v:?}\nbits:    {} ({} bits)\nbytes:   {}",
        bits_to_string(bits),
        bits.len(),
        bytes_to_hex(&v.encode()),
    )
}

/// Canonical encoded forms of representative `Party` values: the seed (whole space), a
/// single fork, and a deeper asymmetric id.
///
/// Pins both the unpadded bit stream and the
/// padded bytes, so any change to the preorder id encoding or its padding shows up here.
#[test]
fn party_canonical_forms() {
    let seed = Party::seed();
    assert_snapshot!(party_block(&seed), @"
    debug:   Party(0b00)
    bits:    00 (2 bits)
    bytes:   20
    ");

    let mut half = Party::seed();
    drop(half.fork());
    assert_snapshot!(party_block(&half), @"
    debug:   Party(0b1000)
    bits:    1000 (4 bits)
    bytes:   88
    ");

    let deep = from_oracle_party(&oracle::Party::node(
        oracle::Party::Leaf(true),
        oracle::Party::node(oracle::Party::Leaf(false), oracle::Party::Leaf(true)),
    ));
    assert_snapshot!(party_block(&deep), @"
    debug:   Party(0b11000100)
    bits:    11000100 (8 bits)
    bytes:   c4 80
    ");
}

/// Canonical encoded forms of representative `Version` values: the empty event (seed
/// version `0`), a flat leaf with a multi-bit base, and a node with an asymmetric event
/// subtree.
///
/// Pins the event encoding (per-node base via the gamma code) and its padding.
#[test]
fn version_canonical_forms() {
    let zero = Version::new();
    assert_snapshot!(version_block(&zero), @"
    debug:   Version(0b11)
    bits:    11 (2 bits)
    bytes:   e0
    ");

    let mut leaf = Version::new();
    Party::seed().ticks(&mut leaf, 5u8);
    assert_snapshot!(version_block(&leaf), @"
    debug:   Version(0b100110)
    bits:    100110 (6 bits)
    bytes:   9a
    ");

    let node = from_oracle_version(&oracle::Version::node(
        1u8,
        oracle::Version::leaf(0u8),
        oracle::Version::node(0u8, oracle::Version::leaf(1u8), oracle::Version::leaf(0u8)),
    ));
    assert_snapshot!(version_block(&node), @"
    debug:   Version(0b01010010111010)
    bits:    01010010111010 (14 bits)
    bytes:   52 ea
    ");
}

/// A clock's binary encoding preserves the boundary between party and version.
#[test]
fn clock_canonical_form() {
    let mut c = Clock::seed();
    c.tick();
    // A `Clock`'s canonical stream is its `Party` bits followed by its `Version` bits,
    // with no padding between (padding is added only by `encode`). Rebuild that unpadded
    // concatenation here to show the boundary between the two halves.
    let mut bits = c.party().as_bits().to_buf();
    bits.extend_from_buf(&c.version().as_bits().to_buf());
    let fields = format!(
        "debug:   {c:?}\nbits:    {} ({} bits)\nbytes:   {}",
        bits_to_string(crate::codec::built_view(&bits)),
        bits.len(),
        bytes_to_hex(&c.encode()),
    );
    assert_snapshot!(fields, @"
    debug:   Clock { party: Party(0b00), version: Version(0b1010) }
    bits:    001010 (6 bits)
    bytes:   20 a8
    ");
}

/// One rank rendering row: a label, the rendered `Display`, and the
/// `Debug ≡ Display` witness inline (`Debug` delegates, and this block is
/// where that contract is pinned).
fn rank_row(label: &str, r: &Rank) -> String {
    assert_eq!(
        format!("{r}"),
        format!("{r:?}"),
        "Rank's Debug must render exactly its Display"
    );
    format!("{label:<22} {r}")
}

/// `Rank`'s rendered representation across every rendering regime: zero,
/// integral, fractional, normalized-after-subtraction, and a numerator
/// spilled past `u64`.
///
/// This block pins the type's decimal rendering, and every row also
/// witnesses `Debug ≡ Display`. The spilled row's digits are the
/// literal decimal of `2^100 + 1`: the rendering of a numerator wider
/// than `u64` must not differ from the machine-word rendering in anything
/// but length.
#[test]
fn rank_rendered_forms() {
    let integral = from_oracle_version(&oracle::Version::leaf(5u8)).rank();
    let half = from_oracle_version(&oracle::Version::node(
        0u8,
        oracle::Version::leaf(1u8),
        oracle::Version::leaf(0u8),
    ));
    let three_halves = from_oracle_version(&oracle::Version::node(
        0u8,
        oracle::Version::leaf(3u8),
        oracle::Version::leaf(0u8),
    ));
    // 3/2 − 1/2 = 2/2: the raw difference is even over 2^1, so the
    // subtraction's output normalizes back to the integral 1.
    let normalized = three_halves
        .rank()
        .checked_sub(&half.rank())
        .expect("3/2 dominates 1/2");
    // 2^100 + 1: odd, so the numerator stays spilled after normalization.
    let wide = (Base::from(1u8) << 100u32) + Base::from(1u8);
    let spilled = from_oracle_version(&oracle::Version::node(
        0u8,
        oracle::Version::leaf(wide),
        oracle::Version::leaf(0u8),
    ));

    let block = [
        rank_row("zero", &Rank::ZERO),
        rank_row("integral", &integral),
        rank_row("fractional", &half.rank()),
        rank_row("normalized after sub", &normalized),
        rank_row("spilled numerator", &spilled.rank()),
    ]
    .join("\n");
    assert_snapshot!(block, @r"
    zero                   0
    integral               5
    fractional             1/2
    normalized after sub   1
    spilled numerator      1267650600228229401496703205377/2
    ");
}

/// The error types' rendered `Display` strings, pinned verbatim.
///
/// These strings are the crate's error representation — what a caller's
/// logs and wrapped error chains show — so a wording change must be a
/// deliberate re-pin here, never a silent drift. `Decode::Io`'s rendering
/// wraps the underlying `std::io::Error`'s own Display output, which std
/// owns, so its row pins only the crate-owned prefix.
#[test]
fn error_display_strings() {
    let block = [
        format!("Overlap               {Overlap}"),
        format!("Crossed               {Crossed}"),
        format!("Decode::Truncated     {}", Decode::Truncated),
        format!("Decode::TrailingBits  {}", Decode::TrailingBits),
        format!("Decode::NotCanonical  {}", Decode::NotCanonical),
    ]
    .join("\n");
    assert_snapshot!(block, @"
    Overlap               parties are not disjoint
    Crossed               span endpoints cross: the start is not within the end
    Decode::Truncated     unexpected end of input
    Decode::TrailingBits  malformed or spurious trailing padding
    Decode::NotCanonical  input is not canonical
    ");

    let io = Decode::Io(std::io::Error::from(std::io::ErrorKind::UnexpectedEof));
    assert!(
        io.to_string().starts_with("read error: "),
        "Decode::Io renders with the crate-owned prefix: {io}"
    );
}
