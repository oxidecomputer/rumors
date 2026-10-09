//! Representation access used only by tests and resource instrumentation.

use crate::bits::BitsWriter;
use crate::Party;

/// Copy a Party's meaningful bits into mutable instrumentation storage.
pub(crate) fn bits(party: &Party) -> BitsWriter {
    let len = party.stored_len();
    let mut writer = BitsWriter::with_capacity(len);
    writer.splice(&party.0, 0, len);
    writer
}

/// Consume a party and report the byte capacity of the buffer holding it.
///
/// # Panics
///
/// Panics if another value shares the buffer, or if the party is held in
/// static storage, as the seed is.
#[cfg(all(test, feature = "borsh"))]
pub(crate) fn allocation_capacity(party: Party) -> usize {
    party.0.allocation_capacity()
}
