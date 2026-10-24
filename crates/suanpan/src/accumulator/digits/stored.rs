//! The update the stored digits receive: an update of the value with the
//! flag applied.

use crate::accumulator::operand::Update;

/// An update of the value with the flag applied, as the stored digits
/// receive it.
///
/// The field is private to this module, so the only way to make one from an
/// [`Update`] is [`new`](Self::new), which takes the flag and applies it;
/// [`reversed`](Self::reversed) reverses one already made. A deposit path
/// that takes a `StoredUpdate` therefore receives an update whose flag was
/// applied exactly once, where the value-level [`Update`] entered.
#[derive(Clone, Copy)]
pub(super) struct StoredUpdate(Update);

/// Construct and apply updates as the stored digits receive them.
impl StoredUpdate {
    /// Return `update` as it changes the stored digits' sum: `update` itself,
    /// or its reverse when `negated` says the value is that sum's negation.
    pub(super) fn new(update: Update, negated: bool) -> StoredUpdate {
        StoredUpdate(if negated { update.reversed() } else { update })
    }

    /// Apply this update to a contribution destined for the stored digits.
    pub(super) fn apply(self, contribution: i128) -> i128 {
        self.0.apply(contribution)
    }

    /// The opposite update, for a contribution of the other sign.
    pub(super) fn reversed(self) -> StoredUpdate {
        StoredUpdate(self.0.reversed())
    }
}
