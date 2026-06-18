//! Documentation goldens (insta inline snapshots).

use insta::assert_snapshot;

use crate::codec::{encode_int, Base, Bits, BitsSlice};
use crate::{Clock, Party, Version};

/// Render a bit stream most-significant-bit-first as a string of `'0'`/`'1'`, the same
/// order `encode_int` and the preorder codec emit. Empty stream renders as `""`.
fn bits_to_string(bits: &BitsSlice) -> String {
    bits.iter().map(|b| if *b { '1' } else { '0' }).collect()
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
    let mut bits = Bits::new();
    encode_int(&mut bits, &Base::from(n));
    format!(
        "{:>20} -> {} ({} bits)",
        n,
        bits_to_string(&bits),
        bits.len()
    )
}

/// Elias-gamma code of `m = n + 1`: `floor(log2 m)` leading zeros, then `m` in
/// `floor(log2 m) + 1` bits MSB-first. The golden table pins the layout across
/// magnitudes — powers of two and their neighbours (where the unary prefix grows), plus
/// a value past `u64::MAX` to witness the arbitrary-width (`BigUint`) code: the integer
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
    let mut big_bits = Bits::new();
    let big = Base::from(1u8) << 64u32; // 2^64
    encode_int(&mut big_bits, &big);
    assert_snapshot!(
        format!("2^64 -> {} ({} bits)", bits_to_string(&big_bits), big_bits.len()),
        @"2^64 -> 000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000001 (129 bits)"
    );
}

/// One canonical-form block: the value's `Display`, its unpadded preorder bit stream
/// (what byte-equality `Eq`/`Hash` compares), and the zero-padded `encode()` bytes.
fn party_block(p: &Party) -> String {
    format!(
        "display: {}\nbits:    {} ({} bits)\nbytes:   {}",
        p,
        bits_to_string(p.as_bits()),
        p.as_bits().len(),
        bytes_to_hex(&p.encode()),
    )
}

fn version_block(v: &Version) -> String {
    format!(
        "display: {}\nbits:    {} ({} bits)\nbytes:   {}",
        v,
        bits_to_string(v.as_bits()),
        v.as_bits().len(),
        bytes_to_hex(&v.encode()),
    )
}

/// Canonical encoded forms of representative `Party` values: the seed (whole space), a
/// single fork, and a deeper asymmetric id. Pins both the unpadded bit stream and the
/// padded bytes, so any change to the preorder id encoding or its padding shows up here.
#[test]
fn party_canonical_forms() {
    let seed = Party::seed();
    assert_snapshot!(party_block(&seed), @r"
    display: 1
    bits:    01 (2 bits)
    bytes:   40
    ");

    let half: Party = "(1, 0)".parse().unwrap();
    assert_snapshot!(party_block(&half), @r"
    display: (1, 0)
    bits:    10100 (5 bits)
    bytes:   a0
    ");

    let deep: Party = "(1, (0, 1))".parse().unwrap();
    assert_snapshot!(party_block(&deep), @"
    display: (1, (0, 1))
    bits:    10110001 (8 bits)
    bytes:   b1
    ");
}

/// Canonical encoded forms of representative `Version` values: the empty event (seed
/// version `0`), a flat leaf with a multi-bit base, and a node with an asymmetric event
/// subtree. Pins the event encoding (per-node base via the gamma code) and its padding.
#[test]
fn version_canonical_forms() {
    let zero = Version::new();
    assert_snapshot!(version_block(&zero), @r"
    display: 0
    bits:    01 (2 bits)
    bytes:   40
    ");

    let leaf = Version::try_from(5u64).unwrap();
    assert_snapshot!(version_block(&leaf), @r"
    display: 5
    bits:    000110 (6 bits)
    bytes:   18
    ");

    let node: Version = "(1, 0, (0, 1, 0))".parse().unwrap();
    assert_snapshot!(version_block(&node), @"
    display: (1, 0, (0, 1, 0))
    bits:    10100111001001 (14 bits)
    bytes:   a7 24
    ");
}

/// Canonical encoded form of a representative `Clock` (`Party` then `Version`, preorder),
/// plus its `Display`/`Debug`. A `Clock` is just its two halves concatenated, so this
/// pins the boundary between them in the byte stream.
#[test]
fn clock_canonical_form() {
    let mut c = Clock::seed();
    c.tick();
    // A `Clock`'s canonical stream is its `Party` bits followed by its `Version` bits,
    // with no padding between (padding is added only by `encode`). Rebuild that unpadded
    // concatenation here to show the boundary between the two halves.
    let mut bits = c.party().as_bits().to_bitvec();
    bits.extend_from_bitslice(c.version().as_bits());
    let fields = format!(
        "display: {c}\ndebug:   {c:?}\nbits:    {} ({} bits)\nbytes:   {}",
        bits_to_string(&bits),
        bits.len(),
        bytes_to_hex(&c.encode()),
    );
    assert_snapshot!(fields, @"
    display: (1, 1)
    debug:   Clock { party: 1, version: 1 }
    bits:    000010 (6 bits)
    bytes:   00 20
    ");
}

/// The paper's §5.1 worked example, rendered step by step as `Clock` `Display`. The same
/// run the clock-level `worked_example` correctness test drives, but here the *concrete
/// clock states* (id region + event tree) are pinned as a readable trace, so the example in the
/// paper has a literal counterpart in the test suite. (`Party`/`Clock` are not `Clone`,
/// so each line snapshots a value before it is consumed/mutated by the next step.)
#[test]
fn worked_example_5_1_states() {
    let mut log: Vec<String> = Vec::new();
    let mut note = |label: &str, c: &Clock| log.push(format!("{label:<24} {c}"));

    // seed, then fork into two participants.
    let mut p1 = Clock::seed();
    note("seed", &p1);
    let mut p2 = p1.fork();
    note("p1 after fork", &p1);
    note("p2 after fork", &p2);

    // p1 ticks, then forks again.
    p1.tick();
    note("p1 tick", &p1);
    let p1a = p1.fork();
    let mut p1b = p1;
    note("p1a (fork of p1)", &p1a);
    note("p1b (fork of p1)", &p1b);

    // p2 ticks twice.
    p2.tick();
    p2.tick();
    note("p2 tick x2", &p2);

    // p1b and p2 sync; their event trees reconcile to a common history.
    p1b.sync(&mut p2).expect("disjoint");
    note("p1b after sync", &p1b);
    note("p2 after sync", &p2);

    // Rejoin all three (recovering the whole-space id) and tick: the event tree collapses
    // to a single integer.
    let mut whole = p1a;
    whole.join(p1b).expect("disjoint");
    whole.join(p2).expect("disjoint");
    note("rejoined whole", &whole);
    whole.tick();
    note("whole after tick", &whole);

    assert_snapshot!(log.join("\n"), @"
    seed                     (1, 0)
    p1 after fork            ((1, 0), 0)
    p2 after fork            ((0, 1), 0)
    p1 tick                  ((1, 0), (0, 1, 0))
    p1a (fork of p1)         (((0, 1), 0), (0, 1, 0))
    p1b (fork of p1)         (((1, 0), 0), (0, 1, 0))
    p2 tick x2               ((0, 1), (0, 0, 2))
    p1b after sync           (((1, 0), 0), (1, 0, 1))
    p2 after sync            ((0, 1), (1, 0, 1))
    rejoined whole           (1, (1, 0, 1))
    whole after tick         (1, 2)
    ");
}
