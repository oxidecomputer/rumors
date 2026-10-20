//! PROBE (audit-rank-fuel, uncommitted scratch): deterministic counts for the
//! rank family. For each size, runs one operation and reports
//! - input bytes (the board's denominator: encoded version bytes),
//! - accumulator touches (suanpan touch meter),
//! - digits zero-filled by accumulator buffer growth (probe counter),
//! - every `add_product` operand pair, summed under a cost model that mirrors
//!   num-bigint 0.4.8's `mac3` dispatch, at 64-bit and 32-bit digits,
//! - one full-width product M(n) under the same model, for comparison,
//! - peak heap above the pre-call baseline.

use before::testing::meter::registry::Shape;
use before::testing::meter::{probe_take_products, reset_touch_ops, touch_ops};
use before::{Party, Rank, Version};
use suanpan::touch_meter::{
    PROBE_FILLED, PROBE_FILL_DEFERRED, PROBE_FILL_SEGMENT, PROBE_TREE_LOG, PROBE_TREE_MAX,
    PROBE_TREE_OPS,
};

/// Leaf digit products of num-bigint 0.4.8's `mac3` for operand lengths `a`, `b`.
fn mac3_cost(a: u64, b: u64) -> u64 {
    let (x, y) = if a < b { (a, b) } else { (b, a) };
    if x == 0 {
        return 0;
    }
    if x <= 32 {
        x * y
    } else if x * 2 <= y {
        let m2 = y / 2;
        mac3_cost(x, m2) + mac3_cost(x, y - m2)
    } else if x <= 256 {
        let half = x / 2;
        mac3_cost(x - half, y - half) + mac3_cost(half, half) + mac3_cost(x - half, y - half)
    } else {
        let i = y / 3 + 1;
        let x0 = x.min(i);
        let x1 = (x - x0).min(i);
        let x2 = x - x0 - x1;
        let y0 = i;
        let y1 = (y - y0).min(i);
        let y2 = y - y0 - y1;
        let _ = y1;
        mac3_cost(x0, y0) + 3 * mac3_cost(i + 1, i + 1) + mac3_cost(x2, y2)
    }
}

/// Cost of one `height * operand` product at `digit_bits`-bit digits.
fn product_cost(height_bits: u64, operand_bits: u64, digit_bits: u64) -> u64 {
    let h = height_bits.div_ceil(digit_bits).max(1);
    let o = operand_bits.div_ceil(digit_bits).max(1);
    if h == 1 || o == 1 {
        // num-bigint's scalar path: one pass over the longer operand.
        h.max(o)
    } else {
        mac3_cost(h, o)
    }
}

struct Reading {
    touches: u64,
    filled: u64,
    products: Vec<(u64, u64)>,
    heap: alloc_meter::Stats,
    tree_ops: u64,
    tree_log: u64,
    tree_max: u64,
    fill_segment: u64,
    fill_deferred: u64,
}

fn measure<R>(f: impl FnOnce() -> R) -> (R, Reading) {
    let _ = probe_take_products();
    reset_touch_ops();
    PROBE_FILLED.store(0, std::sync::atomic::Ordering::Relaxed);
    PROBE_TREE_OPS.store(0, std::sync::atomic::Ordering::Relaxed);
    PROBE_FILL_SEGMENT.store(0, std::sync::atomic::Ordering::Relaxed);
    PROBE_FILL_DEFERRED.store(0, std::sync::atomic::Ordering::Relaxed);
    PROBE_TREE_LOG.store(0, std::sync::atomic::Ordering::Relaxed);
    PROBE_TREE_MAX.store(0, std::sync::atomic::Ordering::Relaxed);
    let (heap, out) = alloc_meter::measure(f);
    let reading = Reading {
        touches: touch_ops(),
        filled: PROBE_FILLED.load(std::sync::atomic::Ordering::Relaxed),
        products: probe_take_products(),
        heap,
        tree_ops: PROBE_TREE_OPS.load(std::sync::atomic::Ordering::Relaxed),
        tree_log: PROBE_TREE_LOG.load(std::sync::atomic::Ordering::Relaxed),
        tree_max: PROBE_TREE_MAX.load(std::sync::atomic::Ordering::Relaxed),
        fill_segment: PROBE_FILL_SEGMENT.load(std::sync::atomic::Ordering::Relaxed),
        fill_deferred: PROBE_FILL_DEFERRED.load(std::sync::atomic::Ordering::Relaxed),
    };
    (out, reading)
}

fn report(label: &str, knob: usize, bytes: usize, r: &Reading) {
    let n_bits = 8 * bytes as u64;
    let count = r.products.len();
    let sum64: u64 = r.products.iter().map(|&(h, o)| product_cost(h, o, 64)).sum();
    let sum32: u64 = r.products.iter().map(|&(h, o)| product_cost(h, o, 32)).sum();
    let school64: u64 = r
        .products
        .iter()
        .map(|&(h, o)| h.div_ceil(64).max(1) * o.div_ceil(64).max(1))
        .sum();
    let max_h = r.products.iter().map(|p| p.0).max().unwrap_or(0);
    let max_o = r.products.iter().map(|p| p.1).max().unwrap_or(0);
    let full64 = product_cost(n_bits, n_bits, 64);
    let full32 = product_cost(n_bits, n_bits, 32);
    println!(
        "{label}\tknob={knob}\tbytes={bytes}\ttouch={}\ttouch/B={:.3}\tfilled_digits={}\tfilled/B={:.3}\tproducts={count}\tmax_h_bits={max_h}\tmax_o_bits={max_o}\tsum_model64={sum64}\tsum_school64={school64}\tM64(n)={full64}\tsum_model32={sum32}\tM32(n)={full32}\tallocs={}\treallocs={}\talloc_bytes={}\talloc_bytes/B={:.3}\tpeak/B={:.3}\ttree_ops={}\ttree_log={}\ttree_max={}\tfill_segment={}\tfill_deferred={}",
        r.touches,
        r.touches as f64 / bytes as f64,
        r.filled,
        r.filled as f64 / bytes as f64,
        r.heap.allocations,
        r.heap.reallocations,
        r.heap.bytes_allocated,
        r.heap.bytes_allocated as f64 / bytes as f64,
        r.heap.peak_bytes as f64 / bytes as f64,
        r.tree_ops,
        r.tree_log,
        r.tree_max,
        r.fill_segment,
        r.fill_deferred,
    );
}

fn small_ones(k: usize) -> Vec<Rank> {
    (0..k)
        .map(|i| {
            let mut version = Version::new();
            version.ticks(&Party::seed(), i as u64 % 7 + 1);
            version.rank()
        })
        .collect()
}

fn run_version(label: &str, knob: usize, v: Version, ops: &[&str]) {
    let bytes = v.encode().len();
    let mut w = v.clone();
    w.tick(&Party::seed());
    let pair_bytes = bytes + w.encode().len();
    for op in ops {
        match *op {
            "rank" => {
                let (_, r) = measure(|| v.rank());
                report(&format!("{label}:rank"), knob, bytes, &r);
            }
            "distance" => {
                let (_, r) = measure(|| v.distance(&w));
                report(&format!("{label}:distance"), knob, pair_bytes, &r);
            }
            "lag" => {
                let (_, r) = measure(|| v.lag(&w));
                report(&format!("{label}:lag"), knob, pair_bytes, &r);
            }
            "lag_rev" => {
                let (_, r) = measure(|| w.lag(&v));
                report(&format!("{label}:lag_rev"), knob, pair_bytes, &r);
            }
            "rank_sum" => {
                let a = v.rank();
                let ones = small_ones(bytes);
                let content = |r: &Rank| -> usize {
                    // `content_bits` is crate-private; the encoded length is
                    // within a constant of it and suffices for a ladder.
                    r.encode().len()
                };
                let n = content(&a) + ones.iter().map(content).sum::<usize>();
                let (_, r) = measure(|| std::iter::once(a).chain(ones).sum::<Rank>());
                report(&format!("{label}:rank_sum"), knob, n, &r);
            }
            other => panic!("unknown op {other}"),
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let family = args.get(1).map(String::as_str).unwrap_or("freeze-parade");
    let lo: u32 = args.get(2).map(|s| s.parse().unwrap()).unwrap_or(4);
    let hi: u32 = args.get(3).map(|s| s.parse().unwrap()).unwrap_or(12);
    let ops: Vec<&str> = args
        .get(4)
        .map(|s| s.split(',').collect())
        .unwrap_or_else(|| vec!["rank"]);
    if family == "comb-rank" {
        // The first summand's numerator alternates nonzero and zero 32-bit
        // digits, so its accumulator holds `m` zero ranges; then `k = 8m`
        // small integer ranks land just above it.
        for e in lo..=hi {
            let m = 1usize << e;
            let mut text = String::from("0.");
            for i in (0..=2 * m).rev() {
                let digit: u32 = if i % 2 == 0 { 1 } else { 0 };
                text.push_str(&format!("{digit:032b}"));
            }
            let a: Rank = text.parse().expect("canonical binary rank");
            let ones = small_ones(8 * m);
            let n = a.encode().len() + ones.iter().map(|r| r.encode().len()).sum::<usize>();
            let (_, r) = measure(|| std::iter::once(a).chain(ones).sum::<Rank>());
            report("comb-rank:rank_sum", m, n, &r);
        }
        return;
    }
    for e in lo..=hi {
        let k = 1usize << e;
        let v = match family {
            "freeze-parade" => Shape::FreezeParade.build1(k).version(),
            "freeze-pos" => Shape::FreezePosition.build1(k).version(),
            "promo-rearm" => Shape::PromotionRearm.build1(k).version(),
            "weight-comb" => Shape::WeightComb.build1(k).version(),
            "dense-suffix" => Shape::DenseSuffix.build2(k, k).version(),
            "wide-arming" => Shape::WideArming.build2(k, k).version(),
            "plateau-puncture" => Shape::PlateauPuncture.build2(k, k).version(),
            "lone-freeze" => Shape::LoneFreeze.build2(k, k).version(),
            "dense" => Shape::Dense.build1(k).version(),
            "arming-train-n" => Shape::ArmingTrain.build_train(k, 32, 32, false).version(),
            "arming-train-sq" => Shape::ArmingTrain.build_train(k, k.max(19), k, false).version(),
            other => panic!("unknown family {other}"),
        };
        run_version(family, k, v, &ops);
    }
}
