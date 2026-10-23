//! A reader double that paces its bytes on a script, for the properties that
//! require a decoder's verdict to depend only on the bytes it reads.
//!
//! `std::io::Read` lets a reader return fewer bytes than asked, return
//! `Interrupted` before any answer, and fail partway. A [`ScriptedReader`]
//! does each of these on a generated [`ReadSchedule`], so a decoder that
//! mishandles a short read, an interrupt, or a failure meets the case on
//! some draw.

use std::io;

use proptest::prelude::*;

use crate::rank::DECODE_CHUNK_BYTES;

/// How a [`ScriptedReader`] paces the bytes it serves.
#[derive(Debug)]
pub(crate) struct ReadSchedule {
    /// The most bytes one call serves.
    pub(crate) chunk: usize,
    /// The `Interrupted` errors returned before each answer, cycled: answer
    /// `i` is preceded by `interrupts[i % interrupts.len()]` of them.
    pub(crate) interrupts: Vec<u8>,
}

/// Generates schedules from one byte per call to calls wider than
/// `Rank::decode`'s buffer, with up to two interrupts before each answer.
pub(crate) fn arb_read_schedule() -> impl Strategy<Value = ReadSchedule> {
    (
        prop_oneof![Just(1), 1..=2 * DECODE_CHUNK_BYTES],
        prop::collection::vec(0u8..=2, 1..=4),
    )
        .prop_map(|(chunk, interrupts)| ReadSchedule { chunk, interrupts })
}

/// A reader that serves its input on a [`ReadSchedule`], and can fail
/// partway.
///
/// An *answer* is a call that serves bytes, reports the end of input, or
/// fails; the schedule's interrupts come before each one. Given a failure
/// offset, the reader serves the bytes before it and then fails with kind
/// `Other` instead of serving more or reporting the end.
pub(crate) struct ScriptedReader<'a> {
    /// The bytes served, in order.
    input: &'a [u8],
    /// The chunk size and interrupt cycle.
    schedule: &'a ReadSchedule,
    /// The offset at which the reader fails, if it does.
    fail_at: Option<usize>,
    /// Bytes of `input` served so far.
    served: usize,
    /// Answers given so far, which index the interrupt cycle.
    answers: usize,
    /// Interrupts already returned before the pending answer.
    interrupted: u8,
    /// Whether the reader has returned its failure.
    failed: bool,
}

impl<'a> ScriptedReader<'a> {
    /// Builds a reader over `input` that fails at offset `fail_at`, if given.
    ///
    /// # Panics
    ///
    /// Panics if `fail_at` lies past the end of `input`.
    pub(crate) fn new(input: &'a [u8], schedule: &'a ReadSchedule, fail_at: Option<usize>) -> Self {
        assert!(
            fail_at.is_none_or(|offset| offset <= input.len()),
            "a scripted failure must fall within the input or at its end"
        );
        ScriptedReader {
            input,
            schedule,
            fail_at,
            served: 0,
            answers: 0,
            interrupted: 0,
            failed: false,
        }
    }

    /// Whether the reader has returned its scripted failure to the decoder.
    pub(crate) fn has_failed(&self) -> bool {
        self.failed
    }
}

/// Returns the schedule's pending interrupt, or else the next answer.
///
/// # Panics
///
/// Panics when called after it has failed. A decoder must report the first
/// error that is not `Interrupted`; one that retries it instead would loop
/// forever against this reader, and the panic makes that loop fail the test
/// rather than hang it.
impl io::Read for ScriptedReader<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        assert!(
            !self.failed,
            "the decoder read again after its reader failed with a non-retryable error"
        );
        let interrupts = &self.schedule.interrupts;
        if self.interrupted < interrupts[self.answers % interrupts.len()] {
            self.interrupted += 1;
            return Err(io::ErrorKind::Interrupted.into());
        }
        self.interrupted = 0;
        self.answers += 1;
        if self.fail_at == Some(self.served) {
            self.failed = true;
            return Err(io::Error::other("scripted reader failure"));
        }
        let end = self.fail_at.unwrap_or(self.input.len());
        let len = buf.len().min(self.schedule.chunk).min(end - self.served);
        buf[..len].copy_from_slice(&self.input[self.served..][..len]);
        self.served += len;
        Ok(len)
    }
}
