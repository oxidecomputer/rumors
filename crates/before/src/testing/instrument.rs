//! Internal measurement hooks compiled into production paths.
//!
//! Without the `meter` feature these hooks reduce to no-ops. Keeping them in
//! one namespace separates measurement from the domain modules whose work they
//! observe.

pub(crate) mod range_minima;
pub(crate) mod scan;
pub(crate) mod span_hull;
