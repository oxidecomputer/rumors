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
