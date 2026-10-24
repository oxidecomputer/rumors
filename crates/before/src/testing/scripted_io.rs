//! Reader and writer doubles that pace their bytes on a script, for the
//! properties that require a decoder's verdict to depend only on the bytes it
//! reads, and an encoder's output only on the bytes it writes.
//!
//! `std::io::Read` lets a reader return fewer bytes than asked, return
//! `Interrupted` before any answer, and fail partway, and `std::io::Write`
//! lets a writer do the same. A [`ScriptedReader`] and a [`ScriptedWriter`]
//! do each of these on a generated [`IoSchedule`], so a decoder or encoder
//! that mishandles a short read or write, an interrupt, or a failure meets the
//! case on some draw.

use std::io;

use proptest::prelude::*;

use crate::rank::DECODE_CHUNK_BYTES;

/// How a [`ScriptedReader`] paces the bytes it serves, or a
/// [`ScriptedWriter`] the bytes it accepts.
#[derive(Debug)]
pub(crate) struct IoSchedule {
    /// The most bytes one call serves or accepts.
    pub(crate) chunk: usize,
    /// The `Interrupted` errors returned before each answer, cycled: answer
    /// `i` is preceded by `interrupts[i % interrupts.len()]` of them.
    pub(crate) interrupts: Vec<u8>,
}

/// Generates schedules from one byte per call to calls wider than
/// `Rank::decode`'s buffer, with up to two interrupts before each answer.
pub(crate) fn arb_io_schedule() -> impl Strategy<Value = IoSchedule> {
    (
        prop_oneof![Just(1), 1..=2 * DECODE_CHUNK_BYTES],
        prop::collection::vec(0u8..=2, 1..=4),
    )
        .prop_map(|(chunk, interrupts)| IoSchedule { chunk, interrupts })
}

/// A reader that serves its input on an [`IoSchedule`], and can fail
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
    schedule: &'a IoSchedule,
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
    pub(crate) fn new(input: &'a [u8], schedule: &'a IoSchedule, fail_at: Option<usize>) -> Self {
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

    /// The number of input bytes the reader has served so far.
    #[cfg(feature = "borsh")]
    pub(crate) fn served(&self) -> usize {
        self.served
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

/// A writer that accepts bytes on an [`IoSchedule`], and can fail partway.
///
/// An *answer* is a call that accepts bytes or fails; the schedule's
/// interrupts come before each one, and each answer accepts at most the
/// schedule's chunk. Given a limit, the writer accepts the bytes before it and
/// then fails with kind `Other` instead of accepting more.
pub(crate) struct ScriptedWriter<'a> {
    /// The bytes accepted, in order.
    written: Vec<u8>,
    /// The chunk size and interrupt cycle.
    schedule: &'a IoSchedule,
    /// The number of bytes after which the writer fails, if it does.
    limit: Option<usize>,
    /// Answers given so far, which index the interrupt cycle.
    answers: usize,
    /// Interrupts already returned before the pending answer.
    interrupted: u8,
    /// Whether the writer has returned its failure.
    failed: bool,
}

impl<'a> ScriptedWriter<'a> {
    /// Builds a writer that fails after `limit` bytes, if given.
    pub(crate) fn new(schedule: &'a IoSchedule, limit: Option<usize>) -> Self {
        ScriptedWriter {
            written: Vec::new(),
            schedule,
            limit,
            answers: 0,
            interrupted: 0,
            failed: false,
        }
    }

    /// Whether the writer has returned its scripted failure to the encoder.
    pub(crate) fn has_failed(&self) -> bool {
        self.failed
    }

    /// The bytes the writer has accepted, in order.
    pub(crate) fn written(&self) -> &[u8] {
        &self.written
    }
}

/// Returns the schedule's pending interrupt, or else the next answer; an
/// empty write accepts nothing at once.
///
/// # Panics
///
/// Panics when called after it has failed. An encoder must report the first
/// error that is not `Interrupted`; one that retries it instead would loop
/// forever against this writer, and the panic makes that loop fail the test
/// rather than hang it.
impl io::Write for ScriptedWriter<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        assert!(
            !self.failed,
            "the encoder wrote again after its writer failed with a non-retryable error"
        );
        if bytes.is_empty() {
            return Ok(0);
        }
        let interrupts = &self.schedule.interrupts;
        if self.interrupted < interrupts[self.answers % interrupts.len()] {
            self.interrupted += 1;
            return Err(io::ErrorKind::Interrupted.into());
        }
        self.interrupted = 0;
        self.answers += 1;
        if self.limit == Some(self.written.len()) {
            self.failed = true;
            return Err(io::Error::other("scripted writer failure"));
        }
        let room = self
            .limit
            .map_or(usize::MAX, |limit| limit - self.written.len());
        let len = bytes.len().min(self.schedule.chunk).min(room);
        self.written.extend_from_slice(&bytes[..len]);
        Ok(len)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
