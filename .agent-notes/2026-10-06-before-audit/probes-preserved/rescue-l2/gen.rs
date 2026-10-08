//! Tape-driven generators: a proptest byte vector is consumed as a stream of
//! choices, so shrinking the tape (shorter, smaller bytes) shrinks the
//! structure toward shallow, zero-height values.

use num_bigint::BigUint;

use crate::model::{self, canon, refine, vdepths, PM, VM};

/// A stream of choices read from a byte tape; exhausted tapes read zero.
pub struct Tape<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Tape<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Tape { data, pos: 0 }
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn byte(&mut self) -> u8 {
        let b = self.data.get(self.pos).copied().unwrap_or(0);
        self.pos += 1;
        b
    }

    /// A choice in `0..n` (`0` when `n <= 1`).
    pub fn below(&mut self, n: usize) -> usize {
        if n <= 1 {
            return 0;
        }
        let x = usize::from(self.byte()) | (usize::from(self.byte()) << 8);
        x % n
    }

    /// True with probability about `num / 256`.
    pub fn chance(&mut self, num: u8) -> bool {
        self.byte() < num
    }
}

fn pow2(k: u64) -> BigUint {
    BigUint::from(1u8) << k
}

/// A dyadic partition built by repeated leaf splits under one of several
/// strategies: uniform (bushy), always-leftmost (left spine), always-rightmost
/// (right spine), random-walk deepening (zigzag spine), and mixtures (combs).
pub fn partition(t: &mut Tape, max_splits: usize, max_depth: u64) -> Vec<u64> {
    let strategy = t.below(6);
    let n = t.below(max_splits + 1);
    let mut ds = vec![0u64];
    let mut focus = 0usize;
    for _ in 0..n {
        let len = ds.len();
        let i = match strategy {
            0 => t.below(len),
            1 => 0,
            2 => len - 1,
            3 => focus.min(len - 1),
            4 => {
                if t.chance(128) {
                    t.below(len)
                } else {
                    focus.min(len - 1)
                }
            }
            _ => {
                if t.chance(220) {
                    focus.min(len - 1)
                } else {
                    t.below(len)
                }
            }
        };
        if ds[i] >= max_depth {
            focus = t.below(len);
            continue;
        }
        let d = ds[i] + 1;
        ds[i] = d;
        ds.insert(i + 1, d);
        focus = i + t.below(2);
    }
    ds
}

/// One height level: a base plus an offset, chosen so differences between
/// levels land on the narrow/wide code boundaries (signed deltas at `2^31`,
/// absolute heights at `2^32 - 2`) and well past the machine word.
pub fn level(t: &mut Tape) -> BigUint {
    let base = match t.below(10) {
        0..=4 => BigUint::ZERO,
        5 => pow2(31),
        6 => pow2(32) - 2u32,
        7 => pow2(64) - 1u32,
        8 => pow2(128),
        _ => pow2(t.below(700) as u64),
    };
    let off = match t.below(16) {
        0..=2 => BigUint::ZERO,
        3 => BigUint::from(1u8),
        4 => BigUint::from(2u8),
        5 => BigUint::from(3u8),
        6 => pow2(31) - 1u32,
        7 => pow2(31),
        8 => pow2(31) + 1u32,
        9 => pow2(32) - 2u32,
        10 => pow2(32) - 1u32,
        11 => pow2(32),
        12 => pow2(63),
        13 => pow2(64),
        14 => pow2(64) + 1u32,
        _ => pow2(t.below(300) as u64),
    };
    base + off
}

/// A small level set shared by correlated values, so equal heights (and
/// therefore collapses and ties) are common.
pub fn levels(t: &mut Tape) -> Vec<BigUint> {
    let n = 1 + t.below(4);
    let mut out: Vec<BigUint> = (0..n).map(|_| level(t)).collect();
    out.push(BigUint::ZERO);
    out
}

/// Size limits for one generated case.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub splits: usize,
    pub depth: u64,
}

/// A canonical version whose leaves draw from `lv`.
pub fn version(t: &mut Tape, lv: &[BigUint], lim: Limits) -> VM {
    let ds = partition(t, lim.splits, lim.depth);
    canon(
        ds.into_iter()
            .map(|d| (d, lv[t.below(lv.len())].clone()))
            .collect(),
    )
}

/// A sparse nonnegative function: mostly zero, a few cells at some level.
fn sparse(t: &mut Tape, lv: &[BigUint], lim: Limits) -> VM {
    let ds = partition(t, lim.splits, lim.depth);
    canon(
        ds.into_iter()
            .map(|d| {
                let h = if t.chance(64) {
                    lv[t.below(lv.len())].clone()
                } else {
                    BigUint::ZERO
                };
                (d, h)
            })
            .collect(),
    )
}

/// A perturbation of `a`: plus, saturating minus, max, or min with a sparse
/// function. Comparable and nearly comparable pairs are common.
pub fn perturb(t: &mut Tape, a: &VM, lv: &[BigUint], lim: Limits) -> VM {
    let g = sparse(t, lv, lim);
    match t.below(4) {
        0 => model::pointwise(a, &g, |x, y| x + y),
        1 => model::pointwise(a, &g, |x, y| if x > y { x - y } else { BigUint::ZERO }),
        2 => model::join(a, &g),
        _ => model::meet(a, &g),
    }
}

/// Two operands whose join (`upper = true`) or meet is exactly `target`:
/// on every cell of a refinement one side equals the target and the other
/// sits below it (join) or above it (meet), so the output collapses wherever
/// the target is flat while the operands keep their own structure.
pub fn decomposition(
    t: &mut Tape,
    target: &VM,
    lv: &[BigUint],
    lim: Limits,
    upper: bool,
) -> (VM, VM) {
    let r = partition(t, lim.splits, lim.depth);
    let cells = refine(&[vdepths(target), r]);
    let mut a = Vec::with_capacity(cells.len());
    let mut b = Vec::with_capacity(cells.len());
    for (d, w) in cells {
        let h = &target[w[0].0].1;
        let delta = if t.chance(128) {
            BigUint::from(1u8 + (t.byte() % 3))
        } else {
            lv[t.below(lv.len())].clone()
        };
        let other = if upper {
            if *h > delta {
                h - &delta
            } else {
                BigUint::ZERO
            }
        } else {
            h + &delta
        };
        match t.below(3) {
            0 => {
                a.push((d, h.clone()));
                b.push((d, other));
            }
            1 => {
                a.push((d, other));
                b.push((d, h.clone()));
            }
            _ => {
                a.push((d, h.clone()));
                b.push((d, h.clone()));
            }
        }
    }
    (canon(a), canon(b))
}

/// A pair of versions from one of several correlated families.
pub fn pair(t: &mut Tape, lv: &[BigUint], lim: Limits) -> (VM, VM, &'static str) {
    match t.below(6) {
        0 => (version(t, lv, lim), version(t, lv, lim), "independent"),
        1 | 2 => {
            let a = version(t, lv, lim);
            let b = perturb(t, &a, lv, lim);
            if t.chance(128) {
                (a, b, "perturbed")
            } else {
                (b, a, "perturbed-rev")
            }
        }
        3 => {
            let target = version(t, lv, lim);
            let (a, b) = decomposition(t, &target, lv, lim, true);
            (a, b, "join-decomposition")
        }
        4 => {
            let target = version(t, lv, lim);
            let (a, b) = decomposition(t, &target, lv, lim, false);
            (a, b, "meet-decomposition")
        }
        _ => {
            let a = version(t, lv, lim);
            (a.clone(), a, "equal")
        }
    }
}

/// A party region list with at least one owned region, and its canonical
/// encoding.
pub fn party(t: &mut Tape, lim: Limits) -> (PM, Vec<u8>) {
    let ds = partition(t, lim.splits, lim.depth);
    let style = t.below(4);
    let mut p: PM = ds
        .iter()
        .enumerate()
        .map(|(i, &d)| {
            let owned = match style {
                0 => t.chance(128),
                1 => t.chance(220),
                2 => t.chance(40),
                _ => i % 2 == 0,
            };
            (d, owned)
        })
        .collect();
    if !p.iter().any(|(_, o)| *o) {
        let i = t.below(p.len());
        p[i].1 = true;
    }
    let bytes = model::encode_party(&p).expect("at least one region is owned");
    (p, bytes)
}
