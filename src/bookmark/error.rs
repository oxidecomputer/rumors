//! Why a bookmark could not be loaded.

use super::format::BOOKMARK_FORMAT_VERSION;

/// Why a stored [`Bookmark`](crate::Bookmark) could not be read from storage.
///
/// [`Read`](Self::Read) reports a storage I/O issue. The other variants mean
/// the stored bookmark is malformatted.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum FormatError {
    /// A truncated or empty bookmark. (An *absent* bookmark is reported by
    /// [`load`](super::Bookmark::load) returning `None`, not as an empty
    /// frame.)
    #[error("bookmark truncated: the {len} bytes present end inside the frame")]
    Truncated {
        /// How many bytes were present.
        len: usize,
    },

    /// The bytes read do not have the expected CBOR envelope structure.
    #[error("not a rumors bookmark")]
    NotABookmark {
        /// The specific part of the structure which was not correct.
        #[source]
        defect: ciborium::de::Error<std::io::Error>,
    },

    /// The bytes decode to a valid bookmark frame, but a format version this
    /// version of this crate does not understand.
    #[error(
        "unsupported bookmark format version {found} (this build writes {BOOKMARK_FORMAT_VERSION})"
    )]
    VersionMismatch {
        /// The format version the bookmark declared.
        found: u64,
    },

    /// The file is corrupt.
    #[error("bookmark integrity hash mismatch: stored record is corrupt")]
    HashMismatch,

    /// The reader failed mid-stream.
    #[error("reading the stored bookmark failed")]
    Read(#[source] std::io::Error),

    /// An intact record frame contains an invalid record.
    ///
    /// The error source gives CBOR, record-shape, or atom-decoding details for
    /// diagnosing the writer's error.
    #[error("the bookmark payload is not a valid recovery record")]
    Record(#[source] ciborium::de::Error<std::io::Error>),
}
