//! Property tests whose case count scales with the configured count.
//!
//! This module re-exports the case-count half of
//! [`rumors::testing::property`], the one copy of the scheme, which this
//! crate's suites and `rumors`'s own unit tests share. Its module
//! documentation explains why `proptest!` cannot scale a case count and how
//! [`run`] and [`divided_proptest!`](crate::divided_proptest) do.

pub use rumors::testing::property::{
    FLEET_DIVISOR, LOCAL_DIVISOR, SESSION_DIVISOR, divided_cases, run,
};
