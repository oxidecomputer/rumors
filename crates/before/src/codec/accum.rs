//! The accumulator seam: `suanpan`'s cliff-immune signed accumulator under
//! this crate's local name.
//!
//! The representation (redundant balanced base-2^32 digits), the cost
//! guarantees, and both amortization arguments live in [`suanpan`]'s crate
//! docs; this module only binds the seam. `Base` drives the accumulator's
//! width-dispatched entry points (`add_base`, `sub_base_shl`, …) through
//! the [`suanpan::Magnitude`] implementation below: a word-scale magnitude
//! takes the amortized-O(1) small path, a spilled one the O(operand limbs)
//! wide path, with `Base`'s inline storage answering the dispatch read in
//! O(1). The skyline sweeps and the rank fold consume the re-export as
//! `Accum`; `before::meter` re-exports this module so the resource-envelope
//! suite can drive delta streams and pin digit-touch cost.

use dashu_int::UBig;

#[cfg(feature = "limb-meter")]
pub use suanpan::touch_meter;
pub use suanpan::Accumulator as Accum;

use super::Base;

impl suanpan::Magnitude for Base {
    fn to_word(&self) -> Option<u64> {
        self.to_u64()
    }

    fn as_wide(&self) -> &UBig {
        &self.0
    }
}

#[cfg(test)]
mod tests;
