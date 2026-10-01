//! The four independent properties exercised by coverage-guided fuzzing.

pub mod decode;
pub mod differential;
pub mod laws;
pub mod operations;

use crate::input::Target;

impl Target {
    /// Run this target's exact oracle on one input.
    pub fn run(self, data: &[u8]) {
        match self {
            Self::Decode => decode::run(data),
            Self::DecodeDifferential => differential::run(data),
            Self::DecodeOperations => operations::run(data),
            Self::Laws => laws::run(data),
        }
    }
}

/// Expand one operation over every value with a canonical raw byte encoding.
///
/// This is the sole type list shared by canonical-decode and transport
/// differential fuzzing. Adding a raw wire type here reaches both targets.
macro_rules! for_each_wire_type {
    ($operation:ident) => {
        $operation!(Party, "party", Scalar);
        $operation!(Version, "version", Scalar);
        $operation!(Clock, "clock", Scalar);
        $operation!(Rank, "rank", Scalar);
        $operation!(Ranked<'static>, "ranked key", Ranked);
        $operation!(Span<'static>, "span", Span);
    };
}

pub(super) use for_each_wire_type;
