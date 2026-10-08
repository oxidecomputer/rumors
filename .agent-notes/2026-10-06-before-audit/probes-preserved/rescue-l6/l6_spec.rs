//! Auditor L6: an independent specification codec for the six wire types.
//!
//! Explore-branch scaffolding. Everything that computes a verdict here is
//! written from the documented formats alone (crate docs, `bits`,
//! `version::io`, `party::io`, `rank` module docs): no production reader,
//! validator, writer, or rank fold participates. Production enters only as the
//! subject, and through `bridge::to_oracle_*` to read back the value it
//! accepted.

#![allow(dead_code)]

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

use borsh::BorshDeserialize;
use num_bigint::{BigInt, BigUint, Sign};
use proptest::prelude::*;
use proptest::test_runner::{Config, TestRunner};

use crate::error::{Decode, ParseValue};
use crate::testing::bridge::{to_oracle_party, to_oracle_version};
use crate::testing::oracles::tree;
use crate::{Clock, Party, Rank, Ranked, Span, Version};

// ───────────────────────────── bits ─────────────────────────────

pub(crate) fn unpack(bytes: &[u8]) -> Vec<bool> {
    bytes
        .iter()
        .flat_map(|b| (0..8).map(move |i| b >> (7 - i) & 1 == 1))
        .collect()
}

pub(crate) fn pack(bits: &[bool]) -> Vec<u8> {
    bits.chunks(8)
        .map(|c| {
            c.iter()
                .enumerate()
                .fold(0u8, |acc, (i, &b)| acc | (u8::from(b) << (7 - i)))
        })
        .collect()
}

/// Marker-pad a live stream: the live bits, one `1`, zeros to the boundary.
pub(crate) fn seal(bits: &[bool]) -> Vec<u8> {
    if bits.is_empty() {
        return Vec::new();
    }
    let mut v = bits.to_vec();
    v.push(true);
    pack(&v)
}

fn gamma(n: &BigUint, out: &mut Vec<bool>) {
    let m = n + 1u32;
    let k = m.bits() - 1;
    for _ in 0..k {
        out.push(false);
    }
    for i in (0..=k).rev() {
        out.push(m.bit(i));
    }
}

fn zigzag(d: &BigInt) -> BigUint {
    if d.sign() == Sign::Minus {
        (d.magnitude() << 1u32) - 1u32
    } else {
        d.magnitude() << 1u32
    }
}

fn unzigzag(c: &BigUint) -> BigInt {
    if c.bit(0) {
        -BigInt::from((c + 1u32) >> 1u32)
    } else {
        BigInt::from(c >> 1u32)
    }
}

struct Cur<'a> {
    bits: &'a [bool],
    pos: usize,
}

impl Cur<'_> {
    fn bit(&mut self) -> Option<bool> {
        let b = *self.bits.get(self.pos)?;
        self.pos += 1;
        Some(b)
    }
    fn gamma(&mut self) -> Option<BigUint> {
        let mut k = 0u64;
        while !self.bit()? {
            k += 1;
        }
        let mut m = BigUint::from(1u8);
        for _ in 0..k {
            m <<= 1u32;
            if self.bit()? {
                m |= BigUint::from(1u8);
            }
        }
        Some(m - 1u32)
    }
}

// ───────────────────────────── classes and verdicts ─────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Class {
    Truncated,
    TrailingBits,
    NotCanonical,
}

fn class_of(e: &Decode) -> Option<Class> {
    match e {
        Decode::Truncated => Some(Class::Truncated),
        Decode::TrailingBits => Some(Class::TrailingBits),
        Decode::NotCanonical => Some(Class::NotCanonical),
        _ => None,
    }
}

fn display_of(c: Class) -> String {
    match c {
        Class::Truncated => Decode::Truncated.to_string(),
        Class::TrailingBits => Decode::TrailingBits.to_string(),
        Class::NotCanonical => Decode::NotCanonical.to_string(),
    }
}

/// The spec's verdict on one input.
#[derive(Clone, Debug)]
pub(crate) enum Verdict<T> {
    Accept(T),
    Reject {
        /// Every defect class present in the input.
        applicable: BTreeSet<Class>,
        /// The first-detected class under the documented sequential parse: the
        /// production decoder's expected report (informational, beyond the
        /// documented precedence).
        first: Class,
        /// The classes a documented precedence rule restricts the report to.
        required: Option<BTreeSet<Class>>,
    },
}

impl<T> Verdict<T> {
    fn reject(first: Class, applicable: BTreeSet<Class>) -> Self {
        Verdict::Reject {
            applicable,
            first,
            required: None,
        }
    }
    fn map<U>(self, f: impl FnOnce(T) -> U) -> Verdict<U> {
        match self {
            Verdict::Accept(t) => Verdict::Accept(f(t)),
            Verdict::Reject {
                applicable,
                first,
                required,
            } => Verdict::Reject {
                applicable,
                first,
                required,
            },
        }
    }
    fn is_accept(&self) -> bool {
        matches!(self, Verdict::Accept(_))
    }
}

// ───────────────────────────── spec values ─────────────────────────────

/// An event tree with absolute leaf heights (signed, so generators can plant
/// negative heights).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SV {
    Leaf(BigInt),
    Node(Box<SV>, Box<SV>),
}

/// An id tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SP {
    Owned,
    Unowned,
    Node(Box<SP>, Box<SP>),
}

impl SV {
    fn leaves(&self) -> Vec<(u64, BigInt)> {
        let mut out = Vec::new();
        let mut stack = vec![(self, 0u64)];
        while let Some((n, d)) = stack.pop() {
            match n {
                SV::Leaf(h) => out.push((d, h.clone())),
                SV::Node(l, r) => {
                    stack.push((r, d + 1));
                    stack.push((l, d + 1));
                }
            }
        }
        out
    }

    /// Collapse equal sibling leaves bottom-up: the minimal dyadic tree.
    pub(crate) fn normalize(&self) -> SV {
        match self {
            SV::Leaf(h) => SV::Leaf(h.clone()),
            SV::Node(l, r) => {
                let (l, r) = (l.normalize(), r.normalize());
                match (&l, &r) {
                    (SV::Leaf(a), SV::Leaf(b)) if a == b => SV::Leaf(a.clone()),
                    _ => SV::Node(Box::new(l), Box::new(r)),
                }
            }
        }
    }

    fn min_height(&self) -> BigInt {
        self.leaves().into_iter().map(|(_, h)| h).min().unwrap()
    }

    /// Pointwise combination of two step functions.
    fn zip(&self, other: &SV, f: &dyn Fn(&BigInt, &BigInt) -> BigInt) -> SV {
        match (self, other) {
            (SV::Leaf(a), SV::Leaf(b)) => SV::Leaf(f(a, b)),
            (SV::Node(l, r), SV::Leaf(_)) => {
                SV::Node(Box::new(l.zip(other, f)), Box::new(r.zip(other, f)))
            }
            (SV::Leaf(_), SV::Node(l, r)) => {
                SV::Node(Box::new(self.zip(l, f)), Box::new(self.zip(r, f)))
            }
            (SV::Node(al, ar), SV::Node(bl, br)) => {
                SV::Node(Box::new(al.zip(bl, f)), Box::new(ar.zip(br, f)))
            }
        }
    }

    /// Whether `self <= other` pointwise.
    fn le(&self, other: &SV) -> bool {
        match (self, other) {
            (SV::Leaf(a), SV::Leaf(b)) => a <= b,
            (SV::Node(l, r), SV::Leaf(_)) => l.le(other) && r.le(other),
            (SV::Leaf(_), SV::Node(l, r)) => self.le(l) && self.le(r),
            (SV::Node(al, ar), SV::Node(bl, br)) => al.le(bl) && ar.le(br),
        }
    }

    /// The exact area under the step function, normalized `(num, exp)`.
    fn rank(&self) -> (BigUint, u64) {
        let leaves = self.leaves();
        let depth = leaves.iter().map(|(d, _)| *d).max().unwrap();
        let mut num = BigInt::from(0u8);
        for (d, h) in &leaves {
            num += h << (depth - d);
        }
        let num = num.to_biguint().expect("a nonnegative area");
        normalize_rank(num, depth)
    }
}

fn normalize_rank(num: BigUint, exp: u64) -> (BigUint, u64) {
    match num.trailing_zeros() {
        None => (BigUint::ZERO, 0),
        Some(tz) => {
            let s = tz.min(exp);
            (num >> s, exp - s)
        }
    }
}

impl SP {
    fn normalize(&self) -> SP {
        match self {
            SP::Node(l, r) => {
                let (l, r) = (l.normalize(), r.normalize());
                match (&l, &r) {
                    (SP::Owned, SP::Owned) => SP::Owned,
                    (SP::Unowned, SP::Unowned) => SP::Unowned,
                    _ => SP::Node(Box::new(l), Box::new(r)),
                }
            }
            leaf => leaf.clone(),
        }
    }
    /// Collapse only empty pairs, keeping collapsible owned pairs: an
    /// encodable, possibly non-canonical spelling.
    fn drop_empty(&self) -> SP {
        match self {
            SP::Node(l, r) => {
                let (l, r) = (l.drop_empty(), r.drop_empty());
                match (&l, &r) {
                    (SP::Unowned, SP::Unowned) => SP::Unowned,
                    _ => SP::Node(Box::new(l), Box::new(r)),
                }
            }
            leaf => leaf.clone(),
        }
    }
    fn from_oracle(t: &tree::Party) -> SP {
        match t {
            tree::Party::Leaf(true) => SP::Owned,
            tree::Party::Leaf(false) => SP::Unowned,
            tree::Party::Node(l, r) => {
                SP::Node(Box::new(SP::from_oracle(l)), Box::new(SP::from_oracle(r)))
            }
        }
    }
}

fn oracle_version_leaves(t: &tree::Version) -> Vec<(u64, BigInt)> {
    let mut out = Vec::new();
    let mut stack = vec![(t, 0u64, BigUint::ZERO)];
    while let Some((n, d, off)) = stack.pop() {
        match n {
            tree::Version::Leaf(b) => out.push((d, BigInt::from(off + b))),
            tree::Version::Node(b, l, r) => {
                let off = off + b;
                stack.push((r, d + 1, off.clone()));
                stack.push((l, d + 1, off));
            }
        }
    }
    out
}

// ───────────────────────────── spec encoders ─────────────────────────────

/// The live bits of an event tree: preorder flags (`0` internal, `1` leaf),
/// each leaf followed by gamma(absolute) for the first leaf and
/// gamma(zigzag(delta)) after. A negative first height is clamped to zero.
pub(crate) fn sv_bits(t: &SV) -> Vec<bool> {
    let mut out = Vec::new();
    let mut prev: Option<BigInt> = None;
    let mut stack = vec![t];
    while let Some(n) = stack.pop() {
        match n {
            SV::Node(l, r) => {
                out.push(false);
                stack.push(r);
                stack.push(l);
            }
            SV::Leaf(h) => {
                out.push(true);
                match &prev {
                    None => {
                        let a = if h.sign() == Sign::Minus {
                            BigUint::ZERO
                        } else {
                            h.magnitude().clone()
                        };
                        gamma(&a, &mut out);
                        prev = Some(BigInt::from(a));
                    }
                    Some(p) => {
                        gamma(&zigzag(&(h - p)), &mut out);
                        prev = Some(h.clone());
                    }
                }
            }
        }
    }
    out
}

/// The live bits of an id tree: two presence bits per node, `00` an owned
/// terminal, absent children unencoded.
pub(crate) fn sp_bits(t: &SP) -> Vec<bool> {
    let mut out = Vec::new();
    let mut stack = vec![t];
    while let Some(n) = stack.pop() {
        match n {
            SP::Owned => {
                out.push(false);
                out.push(false);
            }
            SP::Unowned => panic!("an unowned region is never encoded as a node"),
            SP::Node(l, r) => {
                let (lp, rp) = (**l != SP::Unowned, **r != SP::Unowned);
                assert!(lp || rp, "drop_empty first");
                out.push(lp);
                out.push(rp);
                if rp {
                    stack.push(r);
                }
                if lp {
                    stack.push(l);
                }
            }
        }
    }
    out
}

/// The rank stream from the `rank` module doc.
pub(crate) fn rank_bits(num: &BigUint, exp: u64) -> Vec<bool> {
    let mut out = Vec::new();
    let m = (num >> exp) + 1u32;
    let w = m.bits();
    let rho = u64::from(64 - w.leading_zeros()) - 1;
    for _ in 0..rho {
        out.push(true);
    }
    out.push(false);
    for i in (0..rho).rev() {
        out.push(w >> i & 1 == 1);
    }
    for i in (0..w - 1).rev() {
        out.push(m.bit(i));
    }
    let groups = exp.div_ceil(8);
    for g in 0..groups {
        out.push(true);
        for j in g * 8 + 1..=(g + 1) * 8 {
            out.push(j <= exp && num.bit(exp - j));
        }
    }
    out.push(false);
    out
}

// ───────────────────────────── spec lenient parsers ─────────────────────────────

/// Where the first canonicity violation became detectable, and its kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Violation {
    Collapsible(usize),
    Negative(usize),
}

impl Violation {
    fn at(self) -> usize {
        match self {
            Violation::Collapsible(p) | Violation::Negative(p) => p,
        }
    }
}

pub(crate) struct TreeParse<T> {
    /// The tree and its end position, or `None` if the bits ran out.
    tree: Option<(T, usize)>,
    /// Bit position at which the bits ran out.
    ran_out_at: usize,
    /// Every violation, in detection order.
    violations: Vec<Violation>,
}

impl<T> TreeParse<T> {
    fn first_violation(&self) -> Option<Violation> {
        self.violations.first().copied()
    }
    fn has_collapsible(&self) -> bool {
        self.violations
            .iter()
            .any(|v| matches!(v, Violation::Collapsible(_)))
    }
}

/// Parse one event tree leniently, recording violations.
pub(crate) fn sv_parse(bits: &[bool]) -> TreeParse<SV> {
    enum Frame {
        NeedLeft,
        NeedRight(SV),
    }
    let mut cur = Cur { bits, pos: 0 };
    let mut stack: Vec<Frame> = Vec::new();
    let mut violations = Vec::new();
    let mut prev: Option<BigInt> = None;
    loop {
        let Some(flag) = cur.bit() else {
            return TreeParse {
                tree: None,
                ran_out_at: cur.pos,
                violations,
            };
        };
        if !flag {
            stack.push(Frame::NeedLeft);
            continue;
        }
        let Some(code) = cur.gamma() else {
            return TreeParse {
                tree: None,
                ran_out_at: cur.pos,
                violations,
            };
        };
        let h = match &prev {
            None => BigInt::from(code),
            Some(p) => p + unzigzag(&code),
        };
        if prev.is_some() && h.sign() == Sign::Minus {
            violations.push(Violation::Negative(cur.pos));
        }
        prev = Some(h.clone());
        let mut done = SV::Leaf(h);
        loop {
            match stack.pop() {
                None => {
                    return TreeParse {
                        tree: Some((done, cur.pos)),
                        ran_out_at: cur.pos,
                        violations,
                    }
                }
                Some(Frame::NeedLeft) => {
                    stack.push(Frame::NeedRight(done));
                    break;
                }
                Some(Frame::NeedRight(left)) => {
                    if let (SV::Leaf(a), SV::Leaf(b)) = (&left, &done) {
                        if a == b {
                            violations.push(Violation::Collapsible(cur.pos));
                        }
                    }
                    done = SV::Node(Box::new(left), Box::new(done));
                }
            }
        }
    }
}

/// Parse one id tree leniently, recording collapsible owned pairs.
pub(crate) fn sp_parse(bits: &[bool]) -> TreeParse<SP> {
    struct Frame {
        right_present: bool,
        left: Option<SP>,
    }
    let mut cur = Cur { bits, pos: 0 };
    let mut stack: Vec<Frame> = Vec::new();
    let mut violations = Vec::new();
    loop {
        let (Some(l), Some(r)) = (cur.bit(), cur.bit()) else {
            return TreeParse {
                tree: None,
                ran_out_at: cur.pos,
                violations,
            };
        };
        if l || r {
            stack.push(Frame {
                right_present: r,
                left: if l { None } else { Some(SP::Unowned) },
            });
            continue;
        }
        let mut done = SP::Owned;
        loop {
            match stack.pop() {
                None => {
                    return TreeParse {
                        tree: Some((done, cur.pos)),
                        ran_out_at: cur.pos,
                        violations,
                    }
                }
                Some(Frame {
                    right_present,
                    left: None,
                }) => {
                    if right_present {
                        stack.push(Frame {
                            right_present,
                            left: Some(done),
                        });
                        break;
                    }
                    done = SP::Node(Box::new(done), Box::new(SP::Unowned));
                }
                Some(Frame {
                    left: Some(left), ..
                }) => {
                    if left == SP::Owned && done == SP::Owned {
                        violations.push(Violation::Collapsible(cur.pos));
                    }
                    done = SP::Node(Box::new(left), Box::new(done));
                }
            }
        }
    }
}

/// A rank stream's lenient parse.
pub(crate) enum RankParse {
    /// Bits ran out.
    Truncated,
    /// The header declared `rho >= 64`.
    TooWide,
    /// The close bit ended at `end`; `last_group_zero` flags non-minimal
    /// packing.
    Complete {
        num: BigUint,
        exp: u64,
        end: usize,
        last_group_zero: bool,
    },
}

pub(crate) fn rank_parse(bits: &[bool]) -> RankParse {
    let mut cur = Cur { bits, pos: 0 };
    macro_rules! bit {
        () => {
            match cur.bit() {
                Some(b) => b,
                None => return RankParse::Truncated,
            }
        };
    }
    let mut rho = 0u64;
    while bit!() {
        rho += 1;
    }
    if rho >= 64 {
        return RankParse::TooWide;
    }
    let mut w = 1u64;
    for _ in 0..rho {
        w = w << 1 | u64::from(bit!());
    }
    let mut m = BigUint::from(1u8);
    for _ in 0..w - 1 {
        m <<= 1u32;
        if bit!() {
            m |= BigUint::from(1u8);
        }
    }
    let integral = m - 1u32;
    let mut digits: Vec<bool> = Vec::new();
    let mut last_group_zero = false;
    while bit!() {
        let mut any = false;
        for _ in 0..8 {
            let b = bit!();
            any |= b;
            digits.push(b);
        }
        last_group_zero = !any;
    }
    let end = cur.pos;
    while digits.last() == Some(&false) {
        digits.pop();
    }
    let exp = digits.len() as u64;
    let mut num = integral;
    for d in digits {
        num = (num << 1u32) | BigUint::from(u8::from(d));
    }
    RankParse::Complete {
        num,
        exp,
        end,
        last_group_zero,
    }
}

// ───────────────────────────── slice verdicts ─────────────────────────────

/// The structural verdict on a marker-padded stream whose tree ended at `end`
/// of `total` readable bits.
fn padding(bits: &[bool], end: usize) -> Option<Class> {
    let rem = bits.len() - end;
    if rem == 0 {
        return Some(Class::Truncated);
    }
    if rem > 8 || !bits[end] || bits[end + 1..].iter().any(|&b| b) {
        return Some(Class::TrailingBits);
    }
    None
}

/// Combine a tree parse with its structural padding verdict into a slice
/// verdict.
fn tree_verdict<T>(p: TreeParse<T>, bits: &[bool]) -> Verdict<T> {
    let mut applicable = BTreeSet::new();
    let v = p.first_violation();
    if v.is_some() {
        applicable.insert(Class::NotCanonical);
    }
    match p.tree {
        None => {
            applicable.insert(Class::Truncated);
            let first = if v.is_some() {
                Class::NotCanonical
            } else {
                Class::Truncated
            };
            Verdict::reject(first, applicable)
        }
        Some((t, end)) => {
            if v.is_some() {
                if let Some(c) = padding(bits, end) {
                    applicable.insert(c);
                }
                return Verdict::reject(Class::NotCanonical, applicable);
            }
            match padding(bits, end) {
                None => Verdict::Accept(t),
                Some(c) => {
                    applicable.insert(c);
                    Verdict::reject(c, applicable)
                }
            }
        }
    }
}

pub(crate) fn version_verdict(bytes: &[u8]) -> Verdict<SV> {
    let bits = unpack(bytes);
    tree_verdict(sv_parse(&bits), &bits)
}

pub(crate) fn party_verdict(bytes: &[u8]) -> Verdict<SP> {
    let bits = unpack(bytes);
    tree_verdict(sp_parse(&bits), &bits)
}

/// A byte-aligned prefix stage: the tree, its byte count, or the stage's
/// rejection.
fn prefix_stage<T>(p: TreeParse<T>, bits: &[bool]) -> Result<(T, usize), Verdict<()>> {
    let total = bits.len();
    let mut applicable = BTreeSet::new();
    let v = p.first_violation();
    if v.is_some() {
        applicable.insert(Class::NotCanonical);
    }
    let Some((t, end)) = p.tree else {
        applicable.insert(Class::Truncated);
        let first = if v.is_some() {
            Class::NotCanonical
        } else {
            Class::Truncated
        };
        return Err(Verdict::reject(first, applicable));
    };
    let eb = (end + 1).div_ceil(8);
    let structural = if eb * 8 > total {
        Some(Class::Truncated)
    } else {
        padding(&bits[..eb * 8], end)
    };
    if let Some(c) = structural {
        applicable.insert(c);
    }
    if v.is_some() {
        return Err(Verdict::reject(Class::NotCanonical, applicable));
    }
    match structural {
        Some(c) => Err(Verdict::reject(c, applicable)),
        None => Ok((t, eb)),
    }
}

pub(crate) fn clock_verdict(bytes: &[u8]) -> Verdict<(SP, SV)> {
    let bits = unpack(bytes);
    let (p, eb) = match prefix_stage(sp_parse(&bits), &bits) {
        Ok(x) => x,
        Err(r) => return r.map(|_| unreachable!()),
    };
    version_verdict(&bytes[eb..]).map(|v| (p, v))
}

pub(crate) fn rank_verdict(bytes: &[u8]) -> Verdict<(BigUint, u64)> {
    let bits = unpack(bytes);
    match rank_parse(&bits) {
        RankParse::Truncated => {
            Verdict::reject(Class::Truncated, [Class::Truncated].into_iter().collect())
        }
        RankParse::TooWide => Verdict::reject(
            Class::NotCanonical,
            [Class::NotCanonical].into_iter().collect(),
        ),
        RankParse::Complete {
            num,
            exp,
            end,
            last_group_zero,
        } => {
            let used = end.div_ceil(8);
            let pad_dirty = bits[end..used * 8].iter().any(|&b| b);
            if pad_dirty || last_group_zero || used < bytes.len() {
                Verdict::reject(
                    Class::TrailingBits,
                    [Class::TrailingBits].into_iter().collect(),
                )
            } else {
                Verdict::Accept((num, exp))
            }
        }
    }
}

/// The rank stage of a composite: consumed bytes, or the stage's rejection.
fn rank_stage(bytes: &[u8]) -> Result<((BigUint, u64), usize), Verdict<()>> {
    let bits = unpack(bytes);
    match rank_parse(&bits) {
        RankParse::Truncated => Err(Verdict::reject(
            Class::Truncated,
            [Class::Truncated].into_iter().collect(),
        )),
        RankParse::TooWide => Err(Verdict::reject(
            Class::NotCanonical,
            [Class::NotCanonical].into_iter().collect(),
        )),
        RankParse::Complete {
            num,
            exp,
            end,
            last_group_zero,
        } => {
            let used = end.div_ceil(8);
            let pad_dirty = bits[end..used * 8].iter().any(|&b| b);
            if pad_dirty || last_group_zero {
                Err(Verdict::reject(
                    Class::TrailingBits,
                    [Class::TrailingBits].into_iter().collect(),
                ))
            } else {
                Ok(((num, exp), used))
            }
        }
    }
}

pub(crate) fn ranked_verdict(bytes: &[u8]) -> Verdict<SV> {
    let (rank, used) = match rank_stage(bytes) {
        Ok(x) => x,
        Err(r) => return r.map(|_| unreachable!()),
    };
    match version_verdict(&bytes[used..]) {
        Verdict::Accept(v) => {
            if v.rank() == rank {
                Verdict::Accept(v)
            } else {
                Verdict::reject(
                    Class::NotCanonical,
                    [Class::NotCanonical].into_iter().collect(),
                )
            }
        }
        r => r,
    }
}

pub(crate) fn span_verdict(bytes: &[u8]) -> Verdict<(SV, SV)> {
    let bits = unpack(bytes);
    let (lo, eb) = match prefix_stage(sv_parse(&bits), &bits) {
        Ok(x) => x,
        Err(r) => return r.map(|_| unreachable!()),
    };
    let hi_bytes = &bytes[eb..];
    let hi_bits = unpack(hi_bytes);
    let p = sv_parse(&hi_bits);
    let has_violation = p.first_violation().is_some();
    let has_collapsible = p.has_collapsible();
    let structural = match &p.tree {
        None => Some(Class::Truncated),
        Some((_, end)) => padding(&hi_bits, *end),
    };
    let mut applicable = BTreeSet::new();
    if let Some(c) = structural {
        applicable.insert(c);
    }
    if has_violation {
        applicable.insert(Class::NotCanonical);
    }
    // Ordering is a defect whenever both trees are whole and canonical, even
    // when `hi`'s padding is dirty: the documented precedence then decides.
    let crossed = match (&p.tree, has_violation) {
        (Some((hi, _)), false) => !lo.le(hi),
        _ => false,
    };
    if crossed {
        applicable.insert(Class::NotCanonical);
    }
    if applicable.is_empty() {
        let (hi, _) = p.tree.expect("structurally whole");
        return Verdict::Accept((lo, hi));
    }
    // The admission walk's first event: a collapsible pair is reported where
    // detected; negative heights and ordering defer to structure.
    let first_collapsible = p.violations.iter().find_map(|v| match v {
        Violation::Collapsible(at) => Some(*at),
        _ => None,
    });
    let first = match (first_collapsible, &p.tree, structural) {
        (Some(_), _, _) => Class::NotCanonical,
        (None, _, Some(c)) => c,
        (None, _, None) => Class::NotCanonical,
    };
    // Documented: structural encoding errors take precedence over endpoint
    // ordering. With a structural defect in `hi`, only that class or a
    // component non-canonicity may be reported.
    let required = structural.map(|c| {
        let mut r: BTreeSet<Class> = [c].into_iter().collect();
        if has_violation {
            r.insert(Class::NotCanonical);
        }
        r
    });
    let _ = has_collapsible;
    Verdict::Reject {
        applicable,
        first,
        required,
    }
}

// ───────────────────────────── borsh stream verdicts ─────────────────────────────

/// A stream verdict: accepted value and bytes consumed, or the rejection
/// (with `Truncated` standing for the reader's `UnexpectedEof`).
type StreamVerdict<T> = Verdict<(T, usize)>;

/// Consume a marker-padded tree's padding from a stream: the marker, then
/// zeros to the byte boundary.
fn stream_padding(bits: &[bool], end: usize) -> Result<usize, Class> {
    match bits.get(end) {
        None => return Err(Class::Truncated),
        Some(false) => return Err(Class::TrailingBits),
        Some(true) => {}
    }
    let mut pos = end + 1;
    while !pos.is_multiple_of(8) {
        match bits.get(pos) {
            None => return Err(Class::Truncated),
            Some(true) => return Err(Class::TrailingBits),
            Some(false) => pos += 1,
        }
    }
    Ok(pos / 8)
}

/// One marker-padded tree read from a stream, first-event semantics.
fn stream_tree<T>(p: TreeParse<T>, bits: &[bool]) -> StreamVerdict<T> {
    if let Some(v) = p.first_violation() {
        let _ = v;
        return Verdict::reject(
            Class::NotCanonical,
            [Class::NotCanonical].into_iter().collect(),
        );
    }
    let Some((t, end)) = p.tree else {
        return Verdict::reject(Class::Truncated, [Class::Truncated].into_iter().collect());
    };
    match stream_padding(bits, end) {
        Ok(used) => Verdict::Accept((t, used)),
        Err(c) => Verdict::reject(c, [c].into_iter().collect()),
    }
}

fn stream_version(bytes: &[u8]) -> StreamVerdict<SV> {
    let bits = unpack(bytes);
    stream_tree(sv_parse(&bits), &bits)
}

fn stream_party(bytes: &[u8]) -> StreamVerdict<SP> {
    let bits = unpack(bytes);
    stream_tree(sp_parse(&bits), &bits)
}

fn stream_rank(bytes: &[u8]) -> StreamVerdict<(BigUint, u64)> {
    match rank_stage(bytes) {
        Ok((r, used)) => Verdict::Accept((r, used)),
        Err(e) => e.map(|_| unreachable!()),
    }
}

fn stream_clock(bytes: &[u8]) -> StreamVerdict<(SP, SV)> {
    let (p, a) = match stream_party(bytes) {
        Verdict::Accept(x) => x,
        r => return r.map(|_| unreachable!()),
    };
    match stream_version(&bytes[a..]) {
        Verdict::Accept((v, b)) => Verdict::Accept(((p, v), a + b)),
        r => r.map(|_| unreachable!()),
    }
}

fn stream_ranked(bytes: &[u8]) -> StreamVerdict<SV> {
    let (rank, a) = match stream_rank(bytes) {
        Verdict::Accept(x) => x,
        r => return r.map(|_| unreachable!()),
    };
    match stream_version(&bytes[a..]) {
        Verdict::Accept((v, b)) => {
            if v.rank() == rank {
                Verdict::Accept((v, a + b))
            } else {
                Verdict::reject(
                    Class::NotCanonical,
                    [Class::NotCanonical].into_iter().collect(),
                )
            }
        }
        r => r.map(|_| unreachable!()),
    }
}

fn stream_span(bytes: &[u8]) -> StreamVerdict<(SV, SV)> {
    let (lo, a) = match stream_version(bytes) {
        Verdict::Accept(x) => x,
        r => return r.map(|_| unreachable!()),
    };
    let rest = &bytes[a..];
    let bits = unpack(rest);
    let p = sv_parse(&bits);
    // First event in the admission walk: a collapsible pair, else running out
    // or the padding, else negative/ordering.
    if p.has_collapsible() {
        // A collapsible pair detected before the bits run out.
        return Verdict::reject(
            Class::NotCanonical,
            [Class::NotCanonical].into_iter().collect(),
        );
    }
    let has_violation = p.first_violation().is_some();
    let Some((hi, end)) = p.tree else {
        return Verdict::reject(Class::Truncated, [Class::Truncated].into_iter().collect());
    };
    let used = match stream_padding(&bits, end) {
        Ok(u) => u,
        Err(c) => return Verdict::reject(c, [c].into_iter().collect()),
    };
    if has_violation || !lo.le(&hi) {
        return Verdict::reject(
            Class::NotCanonical,
            [Class::NotCanonical].into_iter().collect(),
        );
    }
    Verdict::Accept(((lo, hi), a + used))
}

// ───────────────────────────── production observation ─────────────────────────────

/// What production said, reduced to the spec's vocabulary.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Seen {
    Ok,
    Err(Class),
    Other(String),
}

fn seen<T>(r: &Result<T, Decode>) -> Seen {
    match r {
        Ok(_) => Seen::Ok,
        Err(e) => match class_of(e) {
            Some(c) => Seen::Err(c),
            None => Seen::Other(format!("{e:?}")),
        },
    }
}

fn borsh_seen<T>(r: &Result<T, borsh::io::Error>) -> Seen {
    match r {
        Ok(_) => Seen::Ok,
        Err(e) if e.kind() == borsh::io::ErrorKind::UnexpectedEof => Seen::Err(Class::Truncated),
        Err(e) => match e
            .get_ref()
            .and_then(|inner| inner.downcast_ref::<Decode>())
            .and_then(class_of)
        {
            Some(c) => Seen::Err(c),
            None => Seen::Other(format!("{e:?}")),
        },
    }
}

/// Statistics over one harness run.
#[derive(Default)]
struct Stats {
    counts: BTreeMap<String, u64>,
}

impl Stats {
    fn bump(&mut self, key: String) {
        *self.counts.entry(key).or_default() += 1;
    }
}

/// Judge one observation against a verdict; returns an error message for a
/// contract breach, and records informational first-event mismatches.
fn judge<T>(
    what: &str,
    verdict: &Verdict<T>,
    observed: &Seen,
    stats: &mut Stats,
) -> Result<(), String> {
    match (verdict, observed) {
        (Verdict::Accept(_), Seen::Ok) => {
            stats.bump(format!("{what}: accept"));
            Ok(())
        }
        (Verdict::Accept(_), other) => {
            Err(format!("{what}: spec accepts, production said {other:?}"))
        }
        (
            Verdict::Reject {
                applicable,
                first,
                required,
            },
            Seen::Err(c),
        ) => {
            stats.bump(format!("{what}: reject {c:?}"));
            if !applicable.contains(c) {
                return Err(format!(
                    "{what}: production reported {c:?}, not among applicable {applicable:?}"
                ));
            }
            if let Some(req) = required {
                if !req.contains(c) {
                    return Err(format!(
                        "{what}: production reported {c:?}, documented precedence requires {req:?}"
                    ));
                }
            }
            if c != first {
                stats.bump(format!(
                    "{what}: first-event model said {first:?}, saw {c:?}"
                ));
            }
            Ok(())
        }
        (Verdict::Reject { applicable, .. }, other) => Err(format!(
            "{what}: spec rejects ({applicable:?}), production said {other:?}"
        )),
    }
}

// ───────────────────────────── value read-back ─────────────────────────────

fn party_value(p: &Party) -> SP {
    SP::from_oracle(&to_oracle_party(p)).normalize()
}

fn version_leaves(v: &Version) -> Vec<(u64, BigInt)> {
    oracle_version_leaves(&to_oracle_version(v))
}

// ───────────────────────────── generators ─────────────────────────────

fn arb_height() -> impl Strategy<Value = BigInt> {
    prop_oneof![
        10 => (0i64..4).prop_map(BigInt::from),
        2 => any::<u64>().prop_map(BigInt::from),
        1 => (u64::MAX - 2..=u64::MAX).prop_map(|x| BigInt::from(x) + 1u32),
        2 => (30u32..300, -2i64..3).prop_map(|(k, d)| (BigInt::from(1u8) << k) + d),
        1 => (-3i64..0).prop_map(BigInt::from),
        1 => (30u32..200).prop_map(|k| -(BigInt::from(1u8) << k)),
    ]
}

fn arb_sv() -> impl Strategy<Value = SV> {
    arb_height()
        .prop_map(SV::Leaf)
        .prop_recursive(6, 40, 2, |inner| {
            (inner.clone(), inner).prop_map(|(l, r)| SV::Node(Box::new(l), Box::new(r)))
        })
}

/// A deep spine: one path of 30..400 levels, each level's sibling a leaf,
/// leaning left or right per level; exercises bit-stack spills.
fn arb_sv_deep() -> impl Strategy<Value = SV> {
    (
        arb_height(),
        proptest::collection::vec((any::<bool>(), arb_height()), 30..400),
    )
        .prop_map(|(bottom, levels)| {
            let mut t = SV::Leaf(bottom);
            for (left, h) in levels {
                let leaf = Box::new(SV::Leaf(h));
                t = if left {
                    SV::Node(Box::new(t), leaf)
                } else {
                    SV::Node(leaf, Box::new(t))
                };
            }
            t
        })
}

/// Shallow bushy trees most of the time, deep spines sometimes.
fn arb_sv_any() -> impl Strategy<Value = SV> {
    prop_oneof![4 => arb_sv(), 1 => arb_sv_deep()]
}

/// A deep id spine whose siblings are owned or unowned.
fn arb_sp_deep() -> impl Strategy<Value = SP> {
    proptest::collection::vec((any::<bool>(), any::<bool>()), 30..400).prop_map(|levels| {
        let mut t = SP::Owned;
        for (left, owned) in levels {
            let sib = Box::new(if owned { SP::Owned } else { SP::Unowned });
            t = if left {
                SP::Node(Box::new(t), sib)
            } else {
                SP::Node(sib, Box::new(t))
            };
        }
        t
    })
}

fn arb_sp_any() -> impl Strategy<Value = SP> {
    prop_oneof![4 => arb_sp(), 1 => arb_sp_deep()]
}

/// A nonnegative event tree (canonical after `normalize`).
fn arb_sv_nonneg() -> impl Strategy<Value = SV> {
    arb_sv_any().prop_map(|t| {
        let m = t.min_height();
        if m.sign() == Sign::Minus {
            t.zip(&SV::Leaf(BigInt::from(0u8)), &|a: &BigInt, _: &BigInt| {
                a - &m
            })
        } else {
            t
        }
    })
}

fn arb_sp() -> impl Strategy<Value = SP> {
    prop_oneof![3 => Just(SP::Owned), 2 => Just(SP::Unowned)].prop_recursive(7, 48, 2, |inner| {
        (inner.clone(), inner).prop_map(|(l, r)| SP::Node(Box::new(l), Box::new(r)))
    })
}

/// An encodable id tree: nonempty, empty pairs dropped, owned pairs kept with
/// probability `keep_raw`.
fn sp_encodable(t: &SP, raw: bool) -> SP {
    let t = if raw { t.drop_empty() } else { t.normalize() };
    if t == SP::Unowned {
        SP::Owned
    } else {
        t
    }
}

/// How a live bit stream becomes an input byte string.
#[derive(Clone, Debug)]
enum Edit {
    None,
    FlipBit(usize),
    TruncateBits(usize),
    InsertBit(usize, bool),
    DeleteBit(usize),
}

#[derive(Clone, Debug)]
enum Seal {
    Normal,
    NoMarker,
    MarkerThenJunk(u8),
    ExtraZeroByte,
    ExtraByte(u8),
    DropLastByte,
    Raw,
}

fn arb_edit() -> impl Strategy<Value = Edit> {
    prop_oneof![
        6 => Just(Edit::None),
        2 => any::<usize>().prop_map(Edit::FlipBit),
        2 => any::<usize>().prop_map(Edit::TruncateBits),
        1 => (any::<usize>(), any::<bool>()).prop_map(|(i, b)| Edit::InsertBit(i, b)),
        1 => any::<usize>().prop_map(Edit::DeleteBit),
    ]
}

fn arb_seal() -> impl Strategy<Value = Seal> {
    prop_oneof![
        8 => Just(Seal::Normal),
        1 => Just(Seal::NoMarker),
        1 => any::<u8>().prop_map(Seal::MarkerThenJunk),
        1 => Just(Seal::ExtraZeroByte),
        1 => any::<u8>().prop_map(Seal::ExtraByte),
        1 => Just(Seal::DropLastByte),
        1 => Just(Seal::Raw),
    ]
}

fn apply_edit(mut bits: Vec<bool>, edit: &Edit) -> Vec<bool> {
    let n = bits.len();
    match *edit {
        Edit::None => {}
        Edit::FlipBit(i) if n > 0 => bits[i % n] = !bits[i % n],
        Edit::TruncateBits(i) => bits.truncate(i % (n + 1)),
        Edit::InsertBit(i, b) => bits.insert(i % (n + 1), b),
        Edit::DeleteBit(i) if n > 0 => {
            bits.remove(i % n);
        }
        _ => {}
    }
    bits
}

/// Seal a marker-padded stream.
fn apply_seal(bits: &[bool], seal_how: &Seal) -> Vec<u8> {
    match seal_how {
        Seal::Normal => seal(bits),
        Seal::NoMarker => pack(bits),
        Seal::MarkerThenJunk(j) => {
            let mut v = bits.to_vec();
            v.push(true);
            let mut i = 0;
            while !v.len().is_multiple_of(8) {
                v.push(j >> (i % 8) & 1 == 1);
                i += 1;
            }
            pack(&v)
        }
        Seal::ExtraZeroByte => {
            let mut b = seal(bits);
            b.push(0);
            b
        }
        Seal::ExtraByte(x) => {
            let mut b = seal(bits);
            b.push(*x);
            b
        }
        Seal::DropLastByte => {
            let mut b = seal(bits);
            b.pop();
            b
        }
        Seal::Raw => pack(bits),
    }
}

/// Byte-level edits after sealing.
#[derive(Clone, Debug)]
enum ByteEdit {
    None,
    Flip(usize),
    Cut(usize),
    Append(Vec<u8>),
}

fn arb_byte_edit() -> impl Strategy<Value = ByteEdit> {
    prop_oneof![
        8 => Just(ByteEdit::None),
        1 => any::<usize>().prop_map(ByteEdit::Flip),
        1 => any::<usize>().prop_map(ByteEdit::Cut),
        1 => proptest::collection::vec(any::<u8>(), 1..4).prop_map(ByteEdit::Append),
    ]
}

fn apply_byte_edit(mut b: Vec<u8>, e: &ByteEdit) -> Vec<u8> {
    match e {
        ByteEdit::None => {}
        ByteEdit::Flip(i) if !b.is_empty() => {
            let bit = i % (b.len() * 8);
            b[bit / 8] ^= 0x80 >> (bit % 8);
        }
        ByteEdit::Cut(i) => b.truncate(i % (b.len() + 1)),
        ByteEdit::Append(x) => b.extend_from_slice(x),
        _ => {}
    }
    b
}

/// A version input: a tree, canonical or raw, edited and sealed.
fn arb_version_input() -> impl Strategy<Value = Vec<u8>> {
    prop_oneof![
        12 => (arb_sv_any(), any::<u8>(), arb_edit(), arb_seal()).prop_map(|(t, mode, e, s)| {
            let t = match mode % 4 {
                0 => t,                // raw: may be collapsible or negative
                _ => {
                    // canonical nonnegative
                    let m = t.min_height();
                    let t = if m.sign() == Sign::Minus {
                        t.zip(&SV::Leaf(BigInt::from(0u8)), &|a: &BigInt, _: &BigInt| a - &m)
                    } else {
                        t
                    };
                    t.normalize()
                }
            };
            apply_seal(&apply_edit(sv_bits(&t), &e), &s)
        }),
        1 => proptest::collection::vec(any::<u8>(), 0..24),
    ]
}

fn arb_party_input() -> impl Strategy<Value = Vec<u8>> {
    prop_oneof![
        12 => (arb_sp_any(), any::<bool>(), arb_edit(), arb_seal()).prop_map(|(t, raw, e, s)| {
            apply_seal(&apply_edit(sp_bits(&sp_encodable(&t, raw)), &e), &s)
        }),
        1 => proptest::collection::vec(any::<u8>(), 0..24),
    ]
}

/// A normalized rank value.
fn arb_rank_value() -> impl Strategy<Value = (BigUint, u64)> {
    (
        prop_oneof![
            4 => (0u64..4).prop_map(BigUint::from),
            2 => any::<u64>().prop_map(BigUint::from),
            1 => (60u32..400).prop_map(|k| (BigUint::from(1u8) << k) - 1u32),
            1 => (60u32..400).prop_map(|k| BigUint::from(1u8) << k),
        ],
        prop_oneof![4 => proptest::collection::vec(any::<bool>(), 0..200), 1 => proptest::collection::vec(any::<bool>(), 200..3000)],
    )
        .prop_map(|(int, mut digits)| {
            while digits.last() == Some(&false) {
                digits.pop();
            }
            let exp = digits.len() as u64;
            let mut num = int;
            for d in digits {
                num = (num << 1u32) | BigUint::from(u8::from(d));
            }
            (num, exp)
        })
}

#[derive(Clone, Debug)]
enum RankEdit {
    None,
    Bits(Edit),
    TooWideHeader(u8),
    ZeroGroup,
    DirtyPad(u8),
}

fn arb_rank_input() -> impl Strategy<Value = Vec<u8>> {
    prop_oneof![
        12 => (
            arb_rank_value(),
            prop_oneof![
                5 => Just(RankEdit::None),
                3 => arb_edit().prop_map(RankEdit::Bits),
                1 => (64u8..70).prop_map(RankEdit::TooWideHeader),
                1 => Just(RankEdit::ZeroGroup),
                1 => (1u8..=255).prop_map(RankEdit::DirtyPad),
            ],
            arb_byte_edit(),
        )
            .prop_map(|((num, exp), re, be)| {
                let mut bits = rank_bits(&num, exp);
                match &re {
                    RankEdit::None => {}
                    RankEdit::Bits(e) => bits = apply_edit(bits, e),
                    RankEdit::TooWideHeader(r) => {
                        let mut v = vec![true; usize::from(*r)];
                        v.push(false);
                        v.extend(bits);
                        bits = v;
                    }
                    RankEdit::ZeroGroup => {
                        bits.pop();
                        bits.push(true);
                        bits.extend([false; 8]);
                        bits.push(false);
                    }
                    RankEdit::DirtyPad(x) => {
                        let mut b = pack(&bits);
                        let used = bits.len() % 8;
                        if used != 0 {
                            *b.last_mut().unwrap() |= x >> used;
                        } else {
                            b.push(*x);
                        }
                        return apply_byte_edit(b, &be);
                    }
                }
                apply_byte_edit(pack(&bits), &be)
            }),
        1 => proptest::collection::vec(any::<u8>(), 0..24),
    ]
}

fn arb_clock_input() -> impl Strategy<Value = Vec<u8>> {
    prop_oneof![
        (arb_party_input(), arb_version_input(), arb_byte_edit())
            .prop_map(|(p, v, e)| apply_byte_edit([p, v].concat(), &e)),
        (arb_sp_any(), arb_sv_nonneg(), arb_byte_edit()).prop_map(|(p, v, e)| {
            let p = seal(&sp_bits(&sp_encodable(&p, false)));
            let v = seal(&sv_bits(&v.normalize()));
            apply_byte_edit([p, v].concat(), &e)
        }),
    ]
}

/// Span inputs: ordered, equal, crossed, and arbitrary pairs, each component
/// possibly edited.
fn arb_span_input() -> impl Strategy<Value = Vec<u8>> {
    (
        arb_sv_nonneg(),
        arb_sv_nonneg(),
        0u8..5,
        prop_oneof![6 => Just(None), 1 => arb_version_input().prop_map(Some)],
        prop_oneof![6 => Just(None), 1 => arb_version_input().prop_map(Some)],
        arb_byte_edit(),
    )
        .prop_map(|(a, b, mode, lo_raw, hi_raw, e)| {
            let (lo, hi) = match mode {
                0 => (
                    a.clone(),
                    a.zip(&b, &|x: &BigInt, y: &BigInt| x.max(y).clone()),
                ),
                1 => (a.clone(), a.clone()),
                2 => (
                    a.clone(),
                    a.zip(&b, &|x: &BigInt, y: &BigInt| x.min(y).clone()),
                ),
                3 => (
                    a.zip(&b, &|x: &BigInt, y: &BigInt| x.min(y).clone()),
                    a.clone(),
                ),
                _ => (a, b),
            };
            let lo = lo_raw.unwrap_or_else(|| seal(&sv_bits(&lo.normalize())));
            let hi = hi_raw.unwrap_or_else(|| seal(&sv_bits(&hi.normalize())));
            apply_byte_edit([lo, hi].concat(), &e)
        })
}

fn arb_ranked_input() -> impl Strategy<Value = Vec<u8>> {
    (
        arb_sv_nonneg(),
        arb_sv_nonneg(),
        0u8..4,
        prop_oneof![6 => Just(None), 1 => arb_version_input().prop_map(Some)],
        prop_oneof![8 => Just(None), 1 => arb_rank_input().prop_map(Some)],
        arb_byte_edit(),
    )
        .prop_map(|(a, b, mode, v_raw, r_raw, e)| {
            let a = a.normalize();
            let b = b.normalize();
            let (num, exp) = match mode {
                0 | 1 => a.rank(),
                _ => b.rank(),
            };
            let rank = r_raw.unwrap_or_else(|| pack(&rank_bits(&num, exp)));
            let v = v_raw.unwrap_or_else(|| seal(&sv_bits(&a)));
            apply_byte_edit([rank, v].concat(), &e)
        })
}

// ───────────────────────────── the differential ─────────────────────────────

fn hex_entry<T: core::str::FromStr<Err = ParseValue>>(bytes: &[u8]) -> Seen {
    match hex::encode(bytes).parse::<T>() {
        Ok(_) => Seen::Ok,
        Err(ParseValue::InvalidEncoding(e)) => match class_of(&e) {
            Some(c) => Seen::Err(c),
            None => Seen::Other(format!("{e:?}")),
        },
        Err(e) => Seen::Other(format!("{e:?}")),
    }
}

fn postcard_entry<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> bool {
    let frame = postcard::to_allocvec(serde_bytes::Bytes::new(bytes)).unwrap();
    postcard::from_bytes::<T>(&frame).is_ok()
}

fn cbor_entry<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<(), String> {
    let mut frame = Vec::new();
    ciborium::ser::into_writer(serde_bytes::Bytes::new(bytes), &mut frame).unwrap();
    ciborium::de::from_reader::<T, _>(&frame[..])
        .map(|_| ())
        .map_err(|e| format!("{e:?}"))
}

/// Check the serde entries agree with the spec's accept/reject, and that
/// CBOR's message names an applicable class.
fn check_serde<T: serde::de::DeserializeOwned, V>(
    what: &str,
    bytes: &[u8],
    verdict: &Verdict<V>,
) -> Result<(), String> {
    let accept = verdict.is_accept();
    if postcard_entry::<T>(bytes) != accept {
        return Err(format!("{what} postcard: spec accept={accept}"));
    }
    match (cbor_entry::<T>(bytes), verdict) {
        (Ok(()), Verdict::Accept(_)) => Ok(()),
        (Err(m), Verdict::Reject { applicable, .. }) => {
            if applicable.iter().any(|c| m.contains(&display_of(*c))) {
                Ok(())
            } else {
                Err(format!(
                    "{what} cbor: message {m} names no class in {applicable:?}"
                ))
            }
        }
        (r, _) => Err(format!("{what} cbor: {r:?} but spec accept={accept}")),
    }
}

/// Check a borsh entry against the stream verdict, on `bytes ++ junk`.
fn check_borsh<T: BorshDeserialize, V>(
    what: &str,
    bytes: &[u8],
    junk: &[u8],
    stream: &StreamVerdict<V>,
    stats: &mut Stats,
) -> Result<(), String> {
    let input = [bytes, junk].concat();
    let mut reader: &[u8] = &input;
    let r = T::deserialize_reader(&mut reader);
    let consumed = input.len() - reader.len();
    judge(&format!("{what} borsh"), stream, &borsh_seen(&r), stats)?;
    if let Verdict::Accept((_, used)) = stream {
        if *used != consumed {
            return Err(format!(
                "{what} borsh: consumed {consumed} bytes, spec consumes {used}"
            ));
        }
    }
    Ok(())
}

/// Decode through a chunked, `Interrupted`-injecting reader whose shape is
/// derived from the junk bytes, and require the slice verdict.
fn check_chaos<T>(
    what: &str,
    bytes: &[u8],
    junk: &[u8],
    slice: &Seen,
    decode: impl Fn(&mut dyn std::io::Read) -> Result<T, Decode>,
) -> Result<(), String> {
    let max = 1 + usize::from(junk.first().copied().unwrap_or(7)) % 70;
    let every = if junk.len() % 2 == 0 {
        0
    } else {
        2 + junk.len() % 3
    };
    let mut reader = crate::testing::l6_probes::Chaos::new(bytes, max, every, None);
    let chunked = seen(&decode(&mut reader));
    if &chunked != slice {
        return Err(format!(
            "{what} chunked reader (max {max}, every {every}) said {chunked:?}, slice said {slice:?}"
        ));
    }
    Ok(())
}

/// Human-readable spans: JSON records accept exactly the ordered canonical
/// pairs.
fn check_json_span(lo: &[u8], hi: &[u8], expect: bool) -> Result<(), String> {
    let json = format!(
        "{{\"lo\":\"{}\",\"hi\":\"{}\"}}",
        hex::encode(lo),
        hex::encode(hi)
    );
    let got = serde_json::from_str::<Span<'static>>(&json);
    if got.is_ok() != expect {
        return Err(format!(
            "span json {json}: {got:?}, expected accept={expect}"
        ));
    }
    Ok(())
}

fn run_harness<S: Strategy<Value = Vec<u8>>>(
    name: &str,
    cases: u32,
    strategy: S,
    check: impl Fn(&[u8], &[u8], &mut Stats) -> Result<(), String>,
) {
    let stats = RefCell::new(Stats::default());
    let mut runner = TestRunner::new(Config {
        cases,
        failure_persistence: None,
        ..Config::default()
    });
    let result = runner.run(
        &(strategy, proptest::collection::vec(any::<u8>(), 0..6)),
        |(bytes, junk)| {
            check(&bytes, &junk, &mut stats.borrow_mut())
                .map_err(|m| TestCaseError::fail(format!("{m}\ninput: {bytes:02x?}")))
        },
    );
    println!("== {name} ==");
    for (k, v) in &stats.borrow().counts {
        println!("  {v:>7}  {k}");
    }
    if let Err(e) = result {
        panic!("{name}: {e}");
    }
}

fn cases() -> u32 {
    std::env::var("L6_CASES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(2000)
}

fn sv_to_oracle(t: &SV) -> tree::Version {
    match t {
        SV::Leaf(h) => tree::Version::leaf(h.to_biguint().expect("nonnegative")),
        SV::Node(l, r) => tree::Version::node(0u8, sv_to_oracle(l), sv_to_oracle(r)),
    }
}

fn sp_to_oracle(t: &SP) -> tree::Party {
    match t {
        SP::Owned => tree::Party::Leaf(true),
        SP::Unowned => tree::Party::Leaf(false),
        SP::Node(l, r) => tree::Party::node(sp_to_oracle(l), sp_to_oracle(r)),
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(cases()))]
    /// Spec-format agreement: production encoders, and the production writer
    /// driven by join and meet, emit exactly the spec's canonical bytes.
    #[test]
    fn l6_spec_encoders_agree_with_production(
        a in arb_sv_nonneg(),
        b in arb_sv_nonneg(),
        p in arb_sp(),
        (num, exp) in arb_rank_value(),
    ) {
        let (a, b) = (a.normalize(), b.normalize());
        let va = crate::testing::bridge::from_oracle_version(&sv_to_oracle(&a));
        let vb = crate::testing::bridge::from_oracle_version(&sv_to_oracle(&b));
        prop_assert_eq!(va.encode(), seal(&sv_bits(&a)));
        let join = a.zip(&b, &|x: &BigInt, y: &BigInt| x.max(y).clone()).normalize();
        let meet = a.zip(&b, &|x: &BigInt, y: &BigInt| x.min(y).clone()).normalize();
        prop_assert_eq!(va.join(&vb).encode(), seal(&sv_bits(&join)));
        prop_assert_eq!(va.meet(&vb).encode(), seal(&sv_bits(&meet)));
        let span = va.span(&vb);
        prop_assert_eq!(
            span.encode(),
            [seal(&sv_bits(&meet)), seal(&sv_bits(&join))].concat()
        );
        let (n, e) = a.rank();
        prop_assert_eq!(va.rank().encode(), pack(&rank_bits(&n, e)));
        prop_assert_eq!(
            Ranked::from(&va).encode(),
            [pack(&rank_bits(&n, e)), seal(&sv_bits(&a))].concat()
        );
        let sp = sp_encodable(&p, false);
        let party = crate::testing::bridge::from_oracle_party(&sp_to_oracle(&sp));
        prop_assert_eq!(party.encode(), seal(&sp_bits(&sp)));
        prop_assert_eq!(
            Clock::from_parts(party.dangerously_alias(), va.clone()).encode(),
            [seal(&sp_bits(&sp)), seal(&sv_bits(&a))].concat()
        );
        prop_assert_eq!(Rank::from_raw(num.clone(), exp).encode(), pack(&rank_bits(&num, exp)));
    }
}

#[test]
fn l6_spec_version() {
    run_harness(
        "version",
        cases(),
        arb_version_input(),
        |bytes, junk, stats| {
            let verdict = version_verdict(bytes);
            let prod = Version::decode(bytes);
            judge("version slice", &verdict, &seen(&prod), stats)?;
            check_chaos("version", bytes, junk, &seen(&prod), |r| Version::decode(r))?;
            if let (Verdict::Accept(t), Ok(v)) = (&verdict, &prod) {
                if version_leaves(v) != t.leaves() {
                    return Err(format!(
                        "version value: production {:?} spec {:?}",
                        version_leaves(v),
                        t.leaves()
                    ));
                }
                if v.as_bytes() != bytes {
                    return Err("version bytes not adopted".into());
                }
            }
            judge(
                "version text",
                &verdict,
                &hex_entry::<Version>(bytes),
                stats,
            )?;
            check_serde::<Version, _>("version", bytes, &verdict)?;
            check_borsh::<Version, _>(
                "version",
                bytes,
                junk,
                &stream_version(&[bytes, junk].concat()),
                stats,
            )
        },
    );
}

#[test]
fn l6_spec_party() {
    run_harness("party", cases(), arb_party_input(), |bytes, junk, stats| {
        let verdict = party_verdict(bytes);
        let prod = Party::decode(bytes);
        judge("party slice", &verdict, &seen(&prod), stats)?;
        check_chaos("party", bytes, junk, &seen(&prod), |r| Party::decode(r))?;
        if let (Verdict::Accept(t), Ok(p)) = (&verdict, &prod) {
            if party_value(p) != t.normalize() {
                return Err(format!(
                    "party value: production {:?} spec {:?}",
                    party_value(p),
                    t
                ));
            }
        }
        judge("party text", &verdict, &hex_entry::<Party>(bytes), stats)?;
        check_serde::<Party, _>("party", bytes, &verdict)?;
        check_borsh::<Party, _>(
            "party",
            bytes,
            junk,
            &stream_party(&[bytes, junk].concat()),
            stats,
        )
    });
}

#[test]
fn l6_spec_rank() {
    run_harness("rank", cases(), arb_rank_input(), |bytes, junk, stats| {
        let verdict = rank_verdict(bytes);
        let prod = Rank::decode(bytes);
        judge("rank slice", &verdict, &seen(&prod), stats)?;
        check_chaos("rank", bytes, junk, &seen(&prod), |r| Rank::decode(r))?;
        if let (Verdict::Accept((num, exp)), Ok(r)) = (&verdict, &prod) {
            let (pn, pe) = r.raw_parts();
            if (pn, pe) != (num, *exp) {
                return Err(format!(
                    "rank value: production ({pn}, {pe}) spec ({num}, {exp})"
                ));
            }
        }
        check_serde::<Rank, _>("rank", bytes, &verdict)?;
        check_borsh::<Rank, _>(
            "rank",
            bytes,
            junk,
            &stream_rank(&[bytes, junk].concat()),
            stats,
        )
    });
}

#[test]
fn l6_spec_clock() {
    run_harness("clock", cases(), arb_clock_input(), |bytes, junk, stats| {
        let verdict = clock_verdict(bytes);
        let prod = Clock::decode(bytes);
        judge("clock slice", &verdict, &seen(&prod), stats)?;
        check_chaos("clock", bytes, junk, &seen(&prod), |r| Clock::decode(r))?;
        if let (Verdict::Accept((p, v)), Ok(c)) = (&verdict, &prod) {
            if party_value(c.party()) != p.normalize() || version_leaves(c.version()) != v.leaves()
            {
                return Err("clock value mismatch".into());
            }
        }
        check_serde::<Clock, _>("clock", bytes, &verdict)?;
        check_borsh::<Clock, _>(
            "clock",
            bytes,
            junk,
            &stream_clock(&[bytes, junk].concat()),
            stats,
        )
    });
}

#[test]
fn l6_spec_span() {
    run_harness("span", cases(), arb_span_input(), |bytes, junk, stats| {
        let verdict = span_verdict(bytes);
        let prod = Span::decode(bytes);
        judge("span slice", &verdict, &seen(&prod), stats)?;
        check_chaos("span", bytes, junk, &seen(&prod), |r| Span::decode(r))?;
        if let (Verdict::Accept((lo, hi)), Ok(s)) = (&verdict, &prod) {
            if version_leaves(s.lo()) != lo.leaves() || version_leaves(s.hi()) != hi.leaves() {
                return Err("span value mismatch".into());
            }
            stats.bump(format!("span: accepted equal={}", lo == hi));
        }
        // Split at the spec's lo boundary when lo is a whole canonical
        // prefix, and judge the JSON record form on the two components.
        let bits = unpack(bytes);
        if let Ok((lo, eb)) = prefix_stage(sv_parse(&bits), &bits) {
            if let Verdict::Accept(hi) = version_verdict(&bytes[eb..]) {
                check_json_span(&bytes[..eb], &bytes[eb..], lo.le(&hi))?;
                stats.bump(format!("span json: ordered={}", lo.le(&hi)));
            }
        }
        check_serde::<Span<'static>, _>("span", bytes, &verdict)?;
        check_borsh::<Span<'static>, _>(
            "span",
            bytes,
            junk,
            &stream_span(&[bytes, junk].concat()),
            stats,
        )
    });
}

#[test]
fn l6_spec_ranked() {
    run_harness(
        "ranked",
        cases(),
        arb_ranked_input(),
        |bytes, junk, stats| {
            let verdict = ranked_verdict(bytes);
            let prod = Ranked::decode(bytes);
            judge("ranked slice", &verdict, &seen(&prod), stats)?;
            check_chaos("ranked", bytes, junk, &seen(&prod), |r| Ranked::decode(r))?;
            if let (Verdict::Accept(v), Ok(r)) = (&verdict, &prod) {
                if version_leaves(r.version()) != v.leaves() {
                    return Err("ranked value mismatch".into());
                }
            }
            check_serde::<Ranked<'static>, _>("ranked", bytes, &verdict)?;
            check_borsh::<Ranked<'static>, _>(
                "ranked",
                bytes,
                junk,
                &stream_ranked(&[bytes, junk].concat()),
                stats,
            )
        },
    );
}

/// Reach histograms: the spec harness's generators against the committed
/// `arb_oracle_version` / `arb_oracle_party_nonempty` (explore measurement).
#[test]
fn l6_reach_histograms() {
    use proptest::strategy::ValueTree;
    use std::collections::BTreeMap;
    let n = 20_000;
    let mut runner = TestRunner::deterministic();
    let bucket = |x: u64| -> u64 {
        match x {
            0..=4 => x,
            5..=8 => 8,
            9..=32 => 32,
            33..=128 => 128,
            _ => 512,
        }
    };
    let mut mine: BTreeMap<String, u64> = BTreeMap::new();
    let mut theirs: BTreeMap<String, u64> = BTreeMap::new();
    let input = arb_version_input();
    let committed = crate::testing::generators::arb_oracle_version();
    for _ in 0..n {
        let bytes = input.new_tree(&mut runner).unwrap().current();
        let p = sv_parse(&unpack(&bytes));
        let wide_neg = p
            .violations
            .iter()
            .any(|v| matches!(v, Violation::Negative(_)));
        *mine
            .entry(format!("negative violation present: {wide_neg}"))
            .or_default() += 1;
        *mine
            .entry(format!(
                "collapsible violation present: {}",
                p.has_collapsible()
            ))
            .or_default() += 1;
        if let Some((t, _)) = &p.tree {
            let leaves = t.leaves();
            let depth = leaves.iter().map(|(d, _)| *d).max().unwrap();
            let bits = leaves
                .iter()
                .map(|(_, h)| h.magnitude().bits())
                .max()
                .unwrap();
            *mine
                .entry(format!("depth<= {:>3}", bucket(depth)))
                .or_default() += 1;
            *mine
                .entry(format!("height bits<= {:>3}", bucket(bits)))
                .or_default() += 1;
        } else {
            *mine.entry("incomplete tree".into()).or_default() += 1;
        }
        let t = committed.new_tree(&mut runner).unwrap().current();
        let leaves = oracle_version_leaves(&t);
        let depth = leaves.iter().map(|(d, _)| *d).max().unwrap();
        let bits = leaves
            .iter()
            .map(|(_, h)| h.magnitude().bits())
            .max()
            .unwrap();
        *theirs
            .entry(format!("depth<= {:>3}", bucket(depth)))
            .or_default() += 1;
        *theirs
            .entry(format!("height bits<= {:>3}", bucket(bits)))
            .or_default() += 1;
    }
    println!("== spec harness version inputs ({n}) ==");
    for (k, v) in &mine {
        println!("  {v:>6}  {k}");
    }
    println!("== committed arb_oracle_version ({n}) ==");
    for (k, v) in &theirs {
        println!("  {v:>6}  {k}");
    }
    let mut mine_p: BTreeMap<String, u64> = BTreeMap::new();
    let mut theirs_p: BTreeMap<String, u64> = BTreeMap::new();
    let pin = arb_party_input();
    let pc = crate::testing::generators::arb_oracle_party_nonempty();
    fn sp_depth(t: &SP) -> u64 {
        match t {
            SP::Node(l, r) => 1 + sp_depth(l).max(sp_depth(r)),
            _ => 0,
        }
    }
    for _ in 0..n {
        let bytes = pin.new_tree(&mut runner).unwrap().current();
        let p = sp_parse(&unpack(&bytes));
        *mine_p
            .entry(format!(
                "owned-pair violation present: {}",
                p.has_collapsible()
            ))
            .or_default() += 1;
        if let Some((t, _)) = &p.tree {
            *mine_p
                .entry(format!("depth<= {:>3}", bucket(sp_depth(t))))
                .or_default() += 1;
        }
        let t = pc.new_tree(&mut runner).unwrap().current();
        *theirs_p
            .entry(format!(
                "depth<= {:>3}",
                bucket(sp_depth(&SP::from_oracle(&t)))
            ))
            .or_default() += 1;
    }
    println!("== spec harness party inputs ({n}) ==");
    for (k, v) in &mine_p {
        println!("  {v:>6}  {k}");
    }
    println!("== committed arb_oracle_party_nonempty ({n}) ==");
    for (k, v) in &theirs_p {
        println!("  {v:>6}  {k}");
    }
}
