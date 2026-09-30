//! The newest leaf while its canonical fate is still undecided.

/// A payload held back until the next leaf determines whether it collapses.
pub enum PendingPayload {
    /// A code small enough to remain in one word outside the output.
    Narrow {
        /// The code, right-aligned.
        bits: u64,
        /// The code's length in bits.
        len: u8,
    },
    /// A code too wide to stage, already ending the output payload stream.
    Wide {
        /// The code's length in bits.
        len: u64,
    },
}

impl PendingPayload {
    /// The payload's encoded length.
    pub fn len(&self) -> u64 {
        match self {
            PendingPayload::Narrow { len, .. } => u64::from(*len),
            PendingPayload::Wide { len } => *len,
        }
    }
}
