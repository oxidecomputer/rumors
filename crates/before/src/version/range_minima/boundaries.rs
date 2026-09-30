//! Compact LIFO storage for the differences between armed range minima.
//!
//! Equal adjacent minima form counted zero runs. Positive boundaries carry one
//! opaque client payload apiece. Separate bit tags, packed integers, and sparse
//! vectors keep a small boundary from paying for a wide accumulator and payload
//! in every record. All columns are LIFO, so they need no per-record indices.

use suanpan::Accumulator;

use crate::bits::stack::{BitStack, PackedU64Stack};

use super::boundary::Boundary;

/// One logical record, starting with the innermost boundary when popped.
pub(super) enum Entry<P> {
    /// This many adjacent pairs of ranges share their minimum.
    Equal(u64),
    /// The outer minimum is lower, and its client state is suspended here.
    Positive {
        /// Difference between the inner and outer minima.
        boundary: Boundary,
        /// Client state to resume when the outer minimum becomes current.
        payload: P,
    },
}

/// Synchronized storage columns for zero runs and positive boundaries.
pub(super) struct Boundaries<P> {
    /// Whether each stored record is positive rather than an equal-minimum run.
    positive: BitStack,
    /// Whether each positive boundary needs arbitrary precision.
    wide: BitStack,
    /// Zero-run counts and word-sized positive differences.
    compact: PackedU64Stack,
    /// Top zero run; extending it must cost O(1), regardless of its count.
    top_zeros: u64,
    /// One payload per positive boundary.
    payloads: Vec<P>,
    /// One accumulator per wide boundary.
    wide_values: Vec<Accumulator>,
}

/// Keep the storage columns in lockstep as complete records enter and leave.
impl<P> Boundaries<P> {
    /// Construct an empty boundary stack without allocating.
    pub(super) fn new() -> Self {
        Self {
            positive: BitStack::new(),
            wide: BitStack::new(),
            compact: PackedU64Stack::new(),
            top_zeros: 0,
            payloads: Vec::new(),
            wide_values: Vec::new(),
        }
    }

    /// Whether no pair of armed ranges remains separated by a boundary.
    pub(super) fn is_empty(&self) -> bool {
        self.top_zeros == 0 && self.positive.len() == 0
    }

    /// Store the current zero run before a positive boundary covers it.
    fn flush_zeros(&mut self) {
        if self.top_zeros != 0 {
            self.positive.push(false);
            self.compact.push(self.top_zeros);
            self.top_zeros = 0;
        }
    }

    /// Suspend the outer minimum's payload on one positive boundary.
    pub(super) fn push_positive(&mut self, boundary: Boundary, payload: P) {
        self.flush_zeros();
        self.positive.push(true);
        match boundary {
            Boundary::Word(word) => {
                self.wide.push(false);
                self.compact.push(word);
            }
            Boundary::Wide(wide) => {
                self.wide.push(true);
                self.wide_values.push(wide);
            }
        }
        self.payloads.push(payload);
    }

    /// Extend the equal-minimum run, merging adjacent runs in constant time.
    pub(super) fn push_equal(&mut self, count: u64) {
        self.top_zeros += count;
    }

    /// Pop a complete record, consuming only the columns selected by its tags.
    pub(super) fn pop(&mut self) -> Option<Entry<P>> {
        if self.top_zeros != 0 {
            return Some(Entry::Equal(core::mem::take(&mut self.top_zeros)));
        }
        if !self.positive.pop()? {
            return Some(Entry::Equal(self.compact.pop()));
        }
        let boundary = if self
            .wide
            .pop()
            .expect("each positive boundary has a width tag")
        {
            Boundary::Wide(self.wide_values.pop().expect("each wide tag has a value"))
        } else {
            Boundary::Word(self.compact.pop())
        };
        Some(Entry::Positive {
            boundary,
            payload: self
                .payloads
                .pop()
                .expect("each positive boundary has a payload"),
        })
    }
}
