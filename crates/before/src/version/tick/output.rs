//! Builds a simplified version only after it diverges from the input.
//!
//! The tick walk compares each simplified leaf with the input it replaces.
//! While they match, [`Output`] records only the end of the shared prefix.
//! The first difference copies that prefix into a [`VersionWriter`] and subsequent
//! leaves build the simplified version normally. If no difference appears,
//! no simplified output is allocated.
//!
//! A difference means either different topology or a different payload at the
//! same leaf. This distinction matters for the first leaf, whose payload is
//! an absolute height rather than a delta.

use crate::version::io::regions::VersionSubtreeReader;
use crate::version::io::tree::VersionTreeReader;
use num_bigint::{BigInt, BigUint};

use crate::version::io::writer::VersionWriter;
use crate::Version;

/// Simplification output, retained lazily until its first difference.
///
/// The empty prefix has its own state because the first output leaf needs an
/// absolute payload. Later leaves need deltas from their predecessor.
// There is only one output state per walk. Keeping its builder inline avoids
// an allocation and an indirection on every emitted leaf.
#[allow(clippy::large_enum_variant)]
pub enum Output {
    /// No leaf has been matched or emitted. The next payload is absolute.
    Unstarted,
    /// Every simplified leaf so far matches the input. The prefix ending at
    /// `matched_end` can serve as output without allocating a copy.
    Verbatim {
        /// Input position immediately after the last matched leaf's payload.
        matched_end: u64,
    },
    /// Simplification differs from the input, so the builder holds the output.
    ///
    /// On entry with an empty prefix, the caller must write the first absolute
    /// payload before resuming the walk. All later emissions can then use
    /// deltas without another first-leaf flag.
    Built(VersionWriter),
}

impl Output {
    /// Whether simplification has matched the input so far.
    pub fn is_verbatim(&self) -> bool {
        matches!(self, Output::Unstarted | Output::Verbatim { .. })
    }

    /// Whether no leaf has been emitted or matched: the next emission is
    /// the output's first leaf, coded absolute rather than as a delta.
    ///
    /// Read before a divergence materializes the prefix — [`Output::Built`]
    /// does not record whether it was entered empty.
    pub fn is_unstarted(&self) -> bool {
        matches!(self, Output::Unstarted)
    }

    /// Record an unchanged leaf or subtree whose last payload ends at `end`.
    ///
    /// Returns `true` if retaining the input prefix is sufficient. Once output
    /// has been built, returns `false` so the caller writes this result too.
    pub fn note_match(&mut self, end: u64) -> bool {
        match self {
            Output::Unstarted | Output::Verbatim { .. } => {
                *self = Output::Verbatim { matched_end: end };
                true
            }
            Output::Built(_) => false,
        }
    }

    /// Access materialized output.
    ///
    /// # Panics
    ///
    /// Panics on a verbatim walk — unreachable there: matched emissions
    /// return before their bodies, and diverging ones materialize first.
    fn writer(&mut self) -> &mut VersionWriter {
        match self {
            Output::Built(writer) => writer,
            Output::Unstarted | Output::Verbatim { .. } => {
                unreachable!("a verbatim emission is matched or has diverged")
            }
        }
    }

    /// Append an absolute leaf height to materialized output.
    pub fn height(&mut self, depth: u64, height: &BigUint) {
        self.writer().height(depth, height);
    }

    /// Append a signed change from the preceding output leaf.
    pub fn change(&mut self, depth: u64, change: &BigInt) {
        self.writer().change(depth, change);
    }

    /// Splice the remainder of a canonical multi-leaf subtree verbatim
    /// ([`VersionWriter::copy_subtree_remainder`]).
    ///
    /// The caller has already emitted the subtree's first leaf. The writer
    /// verifies that it remains at the supplied depth.
    ///
    /// # Panics
    ///
    /// Panics on a verbatim walk: the splice runs post-divergence.
    #[allow(clippy::too_many_arguments)] // (src, start, end) is one logical range argument
    pub fn copy_subtree_remainder(
        &mut self,
        src: &Version,
        start: u64,
        end: u64,
        root_depth: u64,
        first_rel_depth: u64,
        last_rel_depth: u64,
        last_code_len: u64,
    ) {
        match self {
            Output::Built(builder) => builder.copy_subtree_remainder(
                src,
                start,
                end,
                root_depth,
                first_rel_depth,
                last_rel_depth,
                last_code_len,
            ),
            Output::Unstarted | Output::Verbatim { .. } => {
                unreachable!("the region splice runs post-divergence")
            }
        }
    }

    /// Materialize the matched prefix at the first divergence.
    ///
    /// Reads the matched prefix's topology and copies its payloads through a
    /// fresh builder. The prefix is empty for an unstarted walk. Once output
    /// has been built, this does nothing.
    ///
    /// The prefix is traversed iteratively and copied once.
    pub fn materialize(&mut self, version: &Version) {
        let matched_end = match self {
            Output::Unstarted => 0,
            Output::Verbatim { matched_end } => *matched_end,
            Output::Built(_) => return,
        };
        let mut builder = VersionWriter::new();
        let mut cursor = VersionTreeReader::new(version);
        let mut walk = VersionSubtreeReader::new();
        while cursor.position() < matched_end {
            // The prefix ends after a complete leaf payload. The differing
            // region follows it, so the source cannot end before this bound.
            let depth = walk
                .descend(&mut cursor)
                .expect("a matched prefix is a proper prefix of the tiling");
            builder.copy_payload(depth, cursor.payload_range());
        }
        debug_assert_eq!(
            cursor.position(),
            matched_end,
            "a matched prefix ends on a plateau boundary"
        );
        *self = Output::Built(builder);
    }

    /// Return the built stream, or `None` if the complete version was unchanged.
    pub fn finish(self, version: &Version) -> Option<Version> {
        let matched_end = match self {
            Output::Built(builder) => return Some(builder.finish()),
            Output::Unstarted => 0,
            Output::Verbatim { matched_end } => matched_end,
        };
        debug_assert_eq!(
            matched_end,
            version.stored_len(),
            "an unchanged walk matches every input plateau",
        );
        None
    }
}
