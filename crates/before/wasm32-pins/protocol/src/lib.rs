//! The typed boundary between the wasm32 guest and its native test harness.
//!
//! Large test operands are synthesized in wasm so their sizes and indices are
//! governed by a 32-bit `usize`. The host therefore sends only a check and two
//! integer parameters. The guest returns [`PASS`] or a [`Failure`]; a trap is
//! reserved for a panic in the code under test, apart from the explicit
//! harness-trap control.

#![no_std]

/// A distinct behavior that must execute with a 32-bit `usize`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Check {
    /// Validate the guest, decoder, and typed-failure channel on small input.
    Liveness = 0,
    /// Deliberately trap, proving that the harness does not report traps as passes.
    HarnessTrap = 1,
    /// Exercise fork counts that cannot fit a 32-bit `usize`.
    Forks = 2,
    /// Decode a canonical version whose byte length is parameter `a`.
    VersionDecode = 3,
    /// Decode a canonical rank whose fractional exponent is parameter `a`.
    RankDecode = 4,
    /// Compare canonical versions whose larger byte length is parameter `a`.
    VersionCompare = 5,
    /// Join complementary versions synthesized from widths `a` and `b`.
    VersionJoinEmitted = 6,
    /// Exercise the rank-arithmetic path selected by parameter `a`.
    RankArithmetic = 7,
    /// Exercise the shifted-landing path selected by parameter `a`.
    SuanpanLanding = 8,
}

impl TryFrom<u32> for Check {
    type Error = ();

    /// Rejects values that name no check, keeping dispatch errors in-band.
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Liveness),
            1 => Ok(Self::HarnessTrap),
            2 => Ok(Self::Forks),
            3 => Ok(Self::VersionDecode),
            4 => Ok(Self::RankDecode),
            5 => Ok(Self::VersionCompare),
            6 => Ok(Self::VersionJoinEmitted),
            7 => Ok(Self::RankArithmetic),
            8 => Ok(Self::SuanpanLanding),
            _ => Err(()),
        }
    }
}

/// A check completed and established its expected behavior.
pub const PASS: i32 = 0;

/// A typed check failure returned without trapping the guest.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum Failure {
    /// The host supplied a check number unknown to this guest.
    UnknownCheck = 1,
    /// The parameters do not satisfy the selected check's preconditions.
    InvalidArguments = 2,
    /// The requested synthetic input cannot be represented or allocated.
    Synthesis = 3,
    /// A public decoder rejected a canonical synthetic input.
    DecodeRejected = 4,
    /// A public decoder accepted an intentionally malformed input.
    DecodeAccepted = 5,
    /// A decoder or operation produced the wrong stored bytes.
    WrongBytes = 6,
    /// A decoder or operation produced the wrong bit length.
    WrongLength = 7,
    /// An operation produced the wrong semantic value or ordering.
    WrongValue = 8,
    /// A fork iterator ended before yielding the requested first child.
    Exhausted = 9,
}

impl Failure {
    /// Decodes a guest return value, rejecting values from a mismatched guest.
    pub fn from_code(code: i32) -> Option<Self> {
        match code {
            1 => Some(Self::UnknownCheck),
            2 => Some(Self::InvalidArguments),
            3 => Some(Self::Synthesis),
            4 => Some(Self::DecodeRejected),
            5 => Some(Self::DecodeAccepted),
            6 => Some(Self::WrongBytes),
            7 => Some(Self::WrongLength),
            8 => Some(Self::WrongValue),
            9 => Some(Self::Exhausted),
            _ => None,
        }
    }

    /// Returns the stable integer transferred across the wasm ABI.
    pub const fn code(self) -> i32 {
        self as i32
    }
}
