//! Frozen prefixes of the running leaf height.
//!
//! The surrounding fold keeps recent height changes in one live accumulator.
//! When that value grows much wider than the next delta, this module freezes it
//! as another component of the absolute height. Leaves and subtree minima then
//! refer to the prefix of components that existed when they were observed.
//! Final settlement combines those references without reconstructing an
//! absolute height at every leaf.

use core::cmp::Ordering;

use num_bigint::{BigInt, BigUint, Sign};
use suanpan::Accumulator;

use crate::accumulator::BigIntAccumulator as _;

/// One leaf height expressed relative to the current frozen prefix.
pub struct LeafHeight {
    /// Signed height change since the prefix was frozen.
    offset: BigInt,
    /// Prefix that supplies the rest of the absolute height.
    prefix: usize,
}

impl LeafHeight {
    /// Signed height change since the prefix was frozen.
    pub fn offset(&self) -> &BigInt {
        &self.offset
    }

    /// Prefix that supplies the rest of the absolute height.
    pub fn prefix(&self) -> usize {
        self.prefix
    }
}

/// Components of the running height and the coefficients of their prefixes.
pub struct HeightPrefixes {
    /// Entry zero is the first leaf height; later entries are frozen changes.
    components: Vec<BigInt>,
    /// Signed coefficient of the prefix ending at each component.
    coefficients: Vec<i128>,
}

impl HeightPrefixes {
    /// Begin with the absolute height of the first leaf.
    pub fn new(first: BigUint) -> Self {
        Self {
            components: vec![BigInt::from(first)],
            coefficients: vec![0],
        }
    }

    /// Identify the prefix containing every component frozen so far.
    fn current(&self) -> usize {
        self.components.len() - 1
    }

    /// Add one leaf height to `total` and return its compact representation.
    ///
    /// The absolute height is `frozen_prefix + live`. The live offset is added
    /// immediately; the prefix coefficient is deferred until settlement.
    pub fn add_leaf(&mut self, live: &Accumulator, total: &mut Accumulator) -> LeafHeight {
        let (ordering, magnitude) = live.signed_magnitude();
        let negative = ordering == Ordering::Less;
        if negative {
            total.sub_biguint_shl(&magnitude, 0);
        } else {
            total.add_biguint_shl(&magnitude, 0);
        }
        *self
            .coefficients
            .last_mut()
            .expect("the opening height always provides one prefix") += 1;
        LeafHeight {
            offset: BigInt::from_biguint(
                if negative { Sign::Minus } else { Sign::Plus },
                magnitude,
            ),
            prefix: self.current(),
        }
    }

    /// Subtract `closes` uses of a subtree minimum anchored at `prefix`.
    pub fn subtract_minimum(&mut self, prefix: usize, closes: u64) {
        self.coefficients[prefix] -= i128::from(closes);
    }

    /// Move the live height change into a new prefix, then clear it.
    ///
    /// A zero change adds no component: every existing prefix still denotes
    /// the same value, so another index would carry no information.
    pub fn freeze(&mut self, live: &mut Accumulator) {
        let component = live.to_bigint();
        if component.sign() != Sign::NoSign {
            self.components.push(component);
            self.coefficients.push(0);
        }
        live.reset();
    }

    /// Add every frozen prefix's contribution to `total`.
    ///
    /// If `F_p` is the sum of components through `p`, the deferred part of the
    /// result is `Σ coefficient_p · F_p`. Reversing the sums gives one product
    /// per component:
    ///
    /// `Σ_p coefficient_p · F_p = Σ_c component_c · Σ_{p ≥ c} coefficient_p`.
    pub fn settle(self, total: &mut Accumulator) {
        let mut suffix = 0i128;
        for (component, coefficient) in self.components.iter().zip(&self.coefficients).rev() {
            suffix += coefficient;
            Self::fold_repeated(
                total,
                component.magnitude(),
                suffix.unsigned_abs(),
                (component.sign() == Sign::Minus) != (suffix < 0),
            );
        }
        debug_assert_eq!(
            suffix, 1,
            "a nonempty binary tree has one more leaf than internal nodes"
        );
    }

    /// Add or subtract `factor * count` in `total`.
    ///
    /// `count` is at most word-scale, so this performs one multiplication
    /// linear in `factor` rather than revisiting any absolute height.
    pub fn fold_repeated(total: &mut Accumulator, factor: &BigUint, count: u128, subtract: bool) {
        if count == 0 || *factor == BigUint::ZERO {
            return;
        }
        let mut product = factor.clone();
        product *= BigUint::from(count);
        if subtract {
            total.sub_biguint_shl(&product, 0);
        } else {
            total.add_biguint_shl(&product, 0);
        }
    }
}
