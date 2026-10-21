// Scratch input families for `Rank`'s `Sum` (rank-sum-remeasure). Included
// verbatim by the wasm fuel guest and by the native counter probe, so both
// measure identical summand lists. Each family returns its summands in the
// order `Sum` consumes them: the first, then the rest.

use before::testing::meter::registry::Shape;
use before::{Party, Rank, Version};

/// `k` small integer ranks, each the seed party ticked 1 to 7 times: the
/// board's `rank_sum` summand list.
fn small_ones(k: usize) -> Vec<Rank> {
    (0..k)
        .map(|i| {
            let mut version = Version::new();
            version.ticks(&Party::seed(), i as u64 % 7 + 1);
            version.rank()
        })
        .collect()
}

/// `0.` followed by `2m + 1` 32-bit digits alternating 1 and 0, starting and
/// ending with 1: `m` interior zero digits, exponent `32 (2m + 1)`.
fn comb_rank(m: usize) -> Rank {
    let mut text = String::from("0.");
    for i in (0..=2 * m).rev() {
        let digit: u32 = if i % 2 == 0 { 1 } else { 0 };
        text.push_str(&format!("{digit:032b}"));
    }
    text.parse().expect("canonical binary rank")
}

/// `0.` followed by `bits` ones: `1 - 2^-bits`, every 32-bit digit nonzero.
fn ones_fraction(bits: usize) -> Rank {
    let mut text = String::from("0.");
    text.push_str(&"1".repeat(bits));
    text.parse().expect("canonical binary rank")
}

/// `2^64 + comb_rank(m)`: the comb's digits below the binary point, a zero
/// units digit and one more zero digit, then a top digit of 1.
fn comb_below(m: usize) -> Rank {
    let mut text = String::from("1");
    text.push_str(&"0".repeat(64));
    text.push('.');
    for i in (0..=2 * m).rev() {
        let digit: u32 = if i % 2 == 0 { 1 } else { 0 };
        text.push_str(&format!("{digit:032b}"));
    }
    text.parse().expect("canonical binary rank")
}

/// Summed encoded bytes of a summand list: the round-one denominator.
fn encoded_bytes(first: &Rank, rest: &[Rank]) -> usize {
    first.encode().len() + rest.iter().map(|r| r.encode().len()).sum::<usize>()
}

/// Build family `op` at knob exponent `e`, as (first summand, the rest,
/// summed encoded bytes).
///
/// - 2, `comb-rank`: the comb with `m = 2^e`, then `8m` small ranks.
/// - 3, `dense-suffix`: `DS(2^e, 2^e)`'s rank, then one small rank per
///   encoded version byte (the board's `rank_sum` cell).
/// - 5, `ones-ladder`: `1 - 2^-(32 * 2^j)` for `j = 0..=e`. Every summand
///   needs a larger exponent than the last, so each forces `acc <<= shift`
///   on a dense held value.
/// - 6, `comb-ladder`: `comb_rank(2^j)` for `j = 0..=e`. Each summand forces
///   a shift, and the held value has zero digits, so every shift rebuilds a
///   written set in bitset form.
/// - 7, `comb-rank-reversed`: `comb-rank`'s summands with the comb last, so
///   the comb lands below an existing run after one shift (the dual order).
/// - 8, `comb-below`: `comb_below(2^e)`, then `8 * 2^e` small ranks. The
///   first summand leaves the run at its top digit, two digits above the
///   units digit, so every small summand writes below the run, into the
///   bitset form: the beside-run path, once per summand.
///
/// Families 2 and 3 allocate in exactly the round-one guest's order
/// (`comb_rank`, then `small_ones` collected on its own, then the byte
/// count, with the version still alive), so the allocator state at
/// measurement matches that guest's too.
pub fn family(op: u32, e: u32) -> (Rank, Vec<Rank>, usize) {
    let k = 1usize << e;
    match op {
        2 => {
            let first = comb_rank(k);
            let rest = small_ones(8 * k);
            let bytes = encoded_bytes(&first, &rest);
            (first, rest, bytes)
        }
        3 => {
            let version = Shape::DenseSuffix.build2(k, k).version();
            let n = version.encode().len();
            let first = version.rank();
            let rest = small_ones(n);
            let bytes = encoded_bytes(&first, &rest);
            (first, rest, bytes)
        }
        5 => {
            let first = ones_fraction(32);
            let rest: Vec<Rank> = (1..=e).map(|j| ones_fraction(32 << j)).collect();
            let bytes = encoded_bytes(&first, &rest);
            (first, rest, bytes)
        }
        6 => {
            let first = comb_rank(1);
            let rest: Vec<Rank> = (1..=e).map(|j| comb_rank(1 << j)).collect();
            let bytes = encoded_bytes(&first, &rest);
            (first, rest, bytes)
        }
        7 => {
            let mut rest = small_ones(8 * k);
            let first = rest.remove(0);
            rest.push(comb_rank(k));
            let bytes = encoded_bytes(&first, &rest);
            (first, rest, bytes)
        }
        8 => {
            let first = comb_below(k);
            let rest = small_ones(8 * k);
            let bytes = encoded_bytes(&first, &rest);
            (first, rest, bytes)
        }
        _ => panic!("unknown family {op}"),
    }
}
