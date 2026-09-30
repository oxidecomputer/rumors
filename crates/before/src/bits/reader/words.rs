//! Byte-backed words for the buffered bit reader.

use core::fmt::{self, Display};

use dsi_bitstream::traits::WordRead;

/// Native-order `u32` words over one encoded byte slice.
pub(super) struct ByteWords<'a> {
    /// Whole source bytes.
    body: &'a [u8],
    /// Masked final partial byte, when present.
    tail: Option<u8>,
    /// Byte offset of the next word.
    next: usize,
    /// Bytes available across `body` and `tail`.
    total: usize,
}

impl<'a> ByteWords<'a> {
    /// Start reading words at byte `start`.
    pub(super) fn new(body: &'a [u8], tail: Option<u8>, start: usize) -> Self {
        Self {
            body,
            tail,
            next: start,
            total: body.len() + usize::from(tail.is_some()),
        }
    }

    /// Read one source byte, filling past the stream with zero.
    fn byte_at(&self, index: usize) -> u8 {
        if index < self.body.len() {
            self.body[index]
        } else if index == self.body.len() {
            self.tail.unwrap_or(0)
        } else {
            0
        }
    }

    /// Gather the next native-order word for the big-endian bit reader.
    fn gather(&self) -> u32 {
        u32::from_ne_bytes([
            self.byte_at(self.next),
            self.byte_at(self.next + 1),
            self.byte_at(self.next + 2),
            self.byte_at(self.next + 3),
        ])
    }
}

/// The source has no byte-bearing word left.
#[derive(Debug)]
pub(super) struct OutOfBytes;

impl Display for OutOfBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("out of stream bytes")
    }
}

impl core::error::Error for OutOfBytes {}

impl WordRead for ByteWords<'_> {
    type Error = OutOfBytes;
    type Word = u32;

    fn read_word(&mut self) -> Result<u32, OutOfBytes> {
        if self.next >= self.total {
            return Err(OutOfBytes);
        }
        let word = self.gather();
        self.next += 4;
        Ok(word)
    }

    fn read_word_opt(&mut self) -> Option<u32> {
        if self.next >= self.total {
            return None;
        }
        let word = self.gather();
        self.next += 4;
        Some(word)
    }
}
