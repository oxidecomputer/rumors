//! Why a bookmark could not be loaded.

use super::format::BOOKMARK_FORMAT_VERSION;

/// Why a stored bookmark could not be turned back into a record.
///
/// [`Read`](Self::Read) reports unavailable storage. The other variants mean
/// the stored bytes cannot supply usable recovery rights.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum FormatError {
    /// The bytes end inside the frame: a truncated or empty file. (An
    /// *absent* bookmark is reported by [`load`](super::Bookmark::load)
    /// returning `None`, never as an empty frame.)
    #[error("bookmark truncated: the {len} bytes present end inside the frame")]
    Truncated {
        /// How many bytes were actually present.
        len: usize,
    },

    /// The bytes do not have the expected CBOR envelope structure.
    #[error("not a rumors bookmark: {defect}")]
    NotABookmark {
        /// Which part of the frame shape failed.
        #[source]
        defect: ciborium::de::Error<std::io::Error>,
    },

    /// A bookmark frame, but a format version this build does not understand.
    #[error(
        "unsupported bookmark format version {found} (this build writes {BOOKMARK_FORMAT_VERSION})"
    )]
    VersionMismatch {
        /// The format version the file declared.
        found: u64,
    },

    /// The integrity hash does not match the covered items: the file is
    /// corrupt.
    #[error("bookmark integrity hash mismatch: stored record is corrupt")]
    HashMismatch,

    /// The lent reader failed mid-stream, before a frame could be examined.
    #[error("reading the stored bookmark failed: {0}")]
    Read(#[source] std::io::Error),

    /// An intact frame contains an invalid record. The source gives CBOR,
    /// record-shape, or atom-decoding details for diagnosing the writer's error.
    #[error("the bookmark payload is not a valid recovery record: {0}")]
    Record(#[source] ciborium::de::Error<std::io::Error>),
}
