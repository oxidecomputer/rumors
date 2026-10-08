
/// L8 witness (probe only): the gamma window decodes 59- to 63-bit codes at
/// every unaligned start, where the code's last bits come from the ninth
/// loaded byte.
#[test]
fn l8_witness_window_unaligned_long_codes() {
    let values: [u64; 6] = [
        (1u64 << 32) - 2,
        (1u64 << 31) + 0x2AAA_AAAA,
        (1u64 << 30) + 0x1555_5555,
        (1u64 << 29) + 12_345,
        1u64 << 31,
        (1u64 << 30) + 6,
    ];
    let mut failures = Vec::new();
    for shift in 1..8u64 {
        for &n in &values {
            let mut bits = BitsWriter::new();
            for _ in 0..shift {
                bits.push(true);
            }
            bits.write_gamma(&BigUint::from(n));
            let code_len = bits.len() - shift;
            for i in 0..16 {
                bits.push(i % 3 == 0);
            }
            let got = BitsReader::gamma_from_window(bits.as_raw_slice(), bits.len(), shift);
            if got != Some((n, shift + code_len)) {
                failures.push((shift, n, code_len, got));
            }
        }
    }
    eprintln!("L8_MUT={:?} window failures: {} of 42", std::env::var("L8_MUT").ok(), failures.len());
    for f in failures.iter().take(4) {
        eprintln!("  {f:?}");
    }
    assert!(failures.is_empty());
}
