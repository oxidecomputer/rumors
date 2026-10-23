
/// Investigation probe, never committed: the mixed-sign accumulator operand
/// reaching `suanpan` through `Version::rank`'s interval addition.
#[cfg(feature = "touch-meter")]
#[test]
fn xp_mixed_sign_rank_probe() {
    use crate::bits::BitsWriter;
    for n in [64usize, 128, 256, 512, 1024] {
        for pattern in [true, false] {
            let (mut x, mut y, mut z) = (BigUint::ZERO, BigUint::ZERO, BigUint::ZERO);
            for k in 0..n {
                x += BigUint::from(u32::MAX) << (64 * k);
                y += BigUint::from(1u8) << (64 * k);
                z += BigUint::from(1u8) << (64 * k + 32);
            }
            let top = BigUint::from(3u8) << (64 * n);
            let (h1, h2, h3) = if pattern {
                (x.clone(), &x + &y + &top, top.clone())
            } else {
                (z.clone(), &z + &top, top.clone())
            };
            let c = BigUint::from(4u8) << (64 * n);
            let l = 64 * n + 129;
            let mut bits = BitsWriter::with_capacity(0);
            bits.push(true);
            bits.write_gamma(&BigUint::ZERO);
            for j in 1..=l {
                let base = if j == l - 1 {
                    h1.clone()
                } else if j == l {
                    &h3 - &h1
                } else {
                    BigUint::ZERO
                };
                bits.push(true);
                bits.write_gamma(&base);
                if j == l {
                    super::ev_leaf_wide(&mut bits, &(&h2 - &h3));
                    super::ev_leaf(&mut bits, 0);
                } else {
                    super::ev_leaf(&mut bits, 0);
                }
            }
            super::ev_leaf_wide(&mut bits, &c);
            let enc = super::Encoding::from_bits(bits);
            let v = enc.version();
            let s = (l + 1) as u64;
            let expected = (&h1 << 1u32) + &h2 + &h3 + (&c << l);
            super::reset_touch_ops();
            let r = v.rank();
            let t = super::touch_ops();
            assert_eq!(r, Rank::from_raw(expected, s));
            let bytes = enc.bytes.len();
            eprintln!(
                "XP n={n} pattern={pattern} bytes={bytes} touches={t} per_byte={:.2}",
                t as f64 / bytes as f64
            );
        }
    }
}
