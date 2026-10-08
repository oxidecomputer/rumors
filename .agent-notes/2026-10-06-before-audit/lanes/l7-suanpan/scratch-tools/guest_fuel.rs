
// ---- L7 explore: fuel probes (appended to guest/src/checks.rs) ----

/// MSB-first bit sink for the meter's construction language.
struct L7Bits {
    bytes: Vec<u8>,
    len: u64,
}

impl L7Bits {
    fn new() -> Self {
        Self { bytes: Vec::new(), len: 0 }
    }

    fn push(&mut self, bit: bool) {
        if self.len % 8 == 0 {
            self.bytes.push(0);
        }
        if bit {
            let last = self.bytes.last_mut().expect("a byte was pushed");
            *last |= 0x80 >> (self.len % 8);
        }
        self.len += 1;
    }

    /// Elias gamma of `value + 1` for a word-sized value.
    fn gamma_word(&mut self, value: u64) {
        let mantissa = value + 1;
        let bits = 64 - mantissa.leading_zeros();
        for _ in 0..bits - 1 {
            self.push(false);
        }
        for i in (0..bits).rev() {
            self.push(mantissa >> i & 1 == 1);
        }
    }

    /// Elias gamma of `sparse(r) + 1`, whose set bits are 0, 2, and `64 i + 2`
    /// for `1 <= i < r`.
    fn gamma_sparse(&mut self, r: u64) {
        let top = 64 * (r - 1) + 2;
        for _ in 0..top {
            self.push(false);
        }
        for j in (0..=top).rev() {
            self.push(j == 0 || (j >= 2 && (j - 2) % 64 == 0));
        }
    }

    fn node(&mut self, internal: bool, base: u64) {
        self.push(internal);
        self.gamma_word(base);
    }
}

/// The limbs of the sparse wide base `sum_{i < r} 4 * 2^(64 i)`: `r` nonzero
/// digits with a zero digit between each pair, so one deposit records `r - 1`
/// zero ranges.
fn l7_sparse_limbs(r: u64) -> impl Iterator<Item = u64> {
    core::iter::repeat_n(4u64, usize::try_from(r).expect("probe sizes fit"))
}

/// A version: root base `sparse(r)` over a complete tree of depth `d` whose
/// bottom pairs are leaves `(1, 0)`, so heights alternate `H + 1`, `H`.
fn l7_version(r: u64, d: u32) -> Version {
    let mut bits = L7Bits::new();
    bits.push(true);
    bits.gamma_sparse(r);
    fn subtree(bits: &mut L7Bits, depth: u32) {
        if depth == 0 {
            bits.node(false, 1);
            bits.node(false, 0);
            return;
        }
        bits.node(true, 0);
        subtree(bits, depth - 1);
        bits.node(true, 0);
        subtree(bits, depth - 1);
    }
    // The root's two children are internal pair trees of depth d - 1 each.
    bits.node(true, 0);
    subtree(&mut bits, d - 1);
    bits.node(true, 0);
    subtree(&mut bits, d - 1);
    let encoding = before::testing::meter::Encoding {
        bits: usize::try_from(bits.len).expect("probe sizes fit"),
        bytes: bits.bytes,
    };
    encoding.version()
}

/// Fuel probes: `mode` selects the workload, `size` its scale.
///
/// - 0: an accumulator holding `size` sparse digits (baseline);
/// - 1: mode 0 plus 2^16 alternating word updates at digit zero;
/// - 2: build and encode the probe version (baseline), `size` sparse digits
///   and `32 * size` leaves;
/// - 3: mode 2 plus `Version::decode` of the encoding;
/// - 4: mode 2 plus `Version::new() <= v` (a full comparison walk).
fn l7_fuel(mode: u64, size: u64) -> Result<(), Failure> {
    match mode {
        0 | 1 => {
            let mut acc = Accumulator::new();
            acc.add_shifted_limbs(0, l7_sparse_limbs(size));
            if mode == 1 {
                for _ in 0..(1u32 << 16) {
                    acc += 1_u64;
                    acc -= 1_u64;
                }
            }
            core::hint::black_box(&acc);
            Ok(())
        }
        2..=4 => {
            let leaves = 32 * size;
            let d = 63 - leaves.leading_zeros();
            let version = l7_version(size, d);
            let bytes = version.encode();
            match mode {
                3 => {
                    let decoded = Version::decode(&bytes[..]).map_err(|_| Failure::DecodeRejected)?;
                    core::hint::black_box(&decoded);
                }
                4 => {
                    let zero = Version::new();
                    if !(zero <= version) {
                        return Err(Failure::WrongValue);
                    }
                }
                _ => {}
            }
            core::hint::black_box(&bytes);
            Ok(())
        }
        _ => Err(Failure::InvalidArguments),
    }
}
