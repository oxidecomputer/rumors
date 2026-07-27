//! The accumulator seam: [`suanpan`]'s cliff-immune signed accumulator
//! re-exported under this crate's local name.
//!
//! The representation, the cost guarantees, and both amortization
//! arguments live in [`suanpan`]'s crate docs; this module only binds the
//! names, and `before::meter` re-exports it.

#[cfg(feature = "limb-meter")]
pub use suanpan::touch_meter;
pub use suanpan::Accumulator as Accum;
