//! The typed boundary between the wasm32 guest and its native test harness.
//!
//! Large test operands are synthesized in wasm so their sizes and indices are
//! governed by a 32-bit `usize`. The host therefore sends only a check and two
//! integer parameters. The guest returns [`PASS`] or a [`Failure`]. It traps
//! only when the code under test panics or exhausts the guest's memory, apart
//! from the harness controls, which trap deliberately in each of those ways.

#![no_std]

use strum_macros::FromRepr;

/// A distinct behavior that must execute with a 32-bit `usize`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, FromRepr)]
#[repr(u32)]
pub enum Check {
    /// Validate the guest, decoder, and typed-failure channel on small input.
    Liveness = 0,
    /// Deliberately panic, proving that the harness reports a panic's trap and
    /// message rather than a pass.
    HarnessPanic = 1,
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
    /// Exercise the rank-arithmetic case selected by parameter `a`.
    RankArithmetic = 7,
    /// Exercise the shifted-landing path selected by parameter `a`.
    SuanpanLanding = 8,
    /// Reserve parameter `a` bits of storage in two accumulators at once,
    /// more than the guest can honor.
    SuanpanReserve = 9,
    /// Shift zero, stored in the form selected by parameter `a`, onto digit position `2^32 - 1`.
    SuanpanZeroShift = 10,
    /// Compact a cancelled stored top while asking for stability under width `a`.
    SuanpanStabilityWidth = 11,
    /// Deliberately exhaust the guest's memory, proving that the harness tells
    /// an allocation-failure abort from a panic.
    HarnessAllocationFailure = 12,
}

impl From<Check> for u32 {
    /// Returns the stable number that names `check` across the wasm ABI.
    fn from(check: Check) -> Self {
        check as u32
    }
}

impl TryFrom<u32> for Check {
    type Error = ();

    /// Decodes a check number, the inverse of `u32::from`.
    ///
    /// Rejects values that name no check, keeping dispatch errors in-band.
    /// `FromRepr` derives this decoder from the declared numbers, so it cannot
    /// disagree with them.
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::from_repr(value).ok_or(())
    }
}

/// A check completed and established its expected behavior.
pub const PASS: i32 = 0;

/// A typed check failure returned without trapping the guest.
#[derive(Clone, Copy, Debug, Eq, PartialEq, FromRepr)]
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

// The harness reads `PASS` as success before it decodes a failure, so a failure
// numbered `PASS` would be reported as a pass.
const _: () = assert!(
    Failure::from_repr(PASS).is_none(),
    "a failure is numbered PASS"
);

impl Failure {
    /// Decodes a guest return value, the inverse of [`Failure::code`].
    ///
    /// Rejects values that name no failure, such as those from a mismatched
    /// guest. `FromRepr` derives this decoder from the declared codes, so it
    /// cannot disagree with them.
    pub fn from_code(code: i32) -> Option<Self> {
        Self::from_repr(code)
    }

    /// Returns the stable integer transferred across the wasm ABI.
    pub const fn code(self) -> i32 {
        self as i32
    }
}
