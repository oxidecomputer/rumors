//! The dense signal byte and its semantic components.

/// One of the 17 logical streams carried in a direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Stream(u8);

impl Stream {
    const COUNT: u8 = 17;
    const MAX: u8 = Self::COUNT - 1;

    /// Validate a wire stream index.
    pub fn new(index: u8) -> Result<Self, StreamError> {
        if index < Self::COUNT {
            Ok(Self(index))
        } else {
            Err(StreamError::Invalid { index })
        }
    }

    /// Return this stream's five-bit wire index.
    pub fn index(self) -> u8 {
        self.0
    }

    /// Find the stream carrying nodes at `height` for `speaker`.
    pub fn at_height(speaker: Speaker, height: usize) -> Option<Self> {
        let index = match (speaker, height) {
            (_, 31) => 0,
            (Speaker::Initiator, height) if height <= 30 && height.is_multiple_of(2) => {
                (32 - height) / 2
            }
            (Speaker::Responder, height) if height <= 29 && !height.is_multiple_of(2) => {
                (31 - height) / 2
            }
            (Speaker::Responder, 0) => 16,
            _ => return None,
        };
        Some(Self(index as u8))
    }

    /// Find the node height carried by this stream for `speaker`.
    pub fn height(self, speaker: Speaker) -> usize {
        match (speaker, self.0) {
            (_, 0) => 31,
            (Speaker::Initiator, index) => 32 - usize::from(index) * 2,
            (Speaker::Responder, 16) => 0,
            (Speaker::Responder, index) => 31 - usize::from(index) * 2,
        }
    }
}

/// A programmatic stream index outside the wire's 17 streams.
#[derive(Debug, Clone, Copy, thiserror::Error, PartialEq, Eq)]
pub enum StreamError {
    #[error("wire stream index {index} is outside 0..=16")]
    Invalid { index: u8 },
}

/// The elected protocol role speaking in one transport direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Speaker {
    Initiator,
    Responder,
}

/// A logical reply or stream boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum End {
    /// End the current reply while leaving its stream open.
    Reply,
    /// End the stream and its current reply.
    Stream,
}

/// Whether another reaction follows or this reaction ends a boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flow {
    /// Another reaction follows in the current reply.
    Continue,
    /// This reaction ends its reply or stream.
    End(End),
}

impl Flow {
    fn offset(self) -> u8 {
        match self {
            Flow::Continue => 0,
            Flow::End(End::Reply) => 1,
            Flow::End(End::Stream) => 2,
        }
    }
}

/// The semantic state carried alongside a stream id in one signal byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Signal {
    Match(Flow),
    QueryEmpty(Flow),
    Query(Flow),
    Supply(Flow),
    End(End),
}

impl Signal {
    const STATES: [Signal; 14] = [
        Signal::Match(Flow::Continue),
        Signal::Match(Flow::End(End::Reply)),
        Signal::Match(Flow::End(End::Stream)),
        Signal::QueryEmpty(Flow::Continue),
        Signal::QueryEmpty(Flow::End(End::Reply)),
        Signal::QueryEmpty(Flow::End(End::Stream)),
        Signal::Query(Flow::Continue),
        Signal::Query(Flow::End(End::Reply)),
        Signal::Query(Flow::End(End::Stream)),
        Signal::Supply(Flow::Continue),
        Signal::Supply(Flow::End(End::Reply)),
        Signal::Supply(Flow::End(End::Stream)),
        Signal::End(End::Reply),
        Signal::End(End::Stream),
    ];

    const STATE_COUNT: u8 = Self::STATES.len() as u8;

    fn state(self) -> u8 {
        match self {
            Signal::Match(flow) => flow.offset(),
            Signal::QueryEmpty(flow) => 3 + flow.offset(),
            Signal::Query(flow) => 6 + flow.offset(),
            Signal::Supply(flow) => 9 + flow.offset(),
            Signal::End(End::Reply) => 12,
            Signal::End(End::Stream) => 13,
        }
    }

    fn from_state(state: u8) -> Result<Self, InvalidSignalState> {
        Self::STATES
            .get(usize::from(state))
            .copied()
            .ok_or(InvalidSignalState { state })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct InvalidSignalState {
    state: u8,
}

/// A semantic signal paired with the logical stream encoded beside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct WireSignal {
    stream: Stream,
    signal: Signal,
}

impl WireSignal {
    pub(super) const BYTE_COUNT: u8 = Signal::STATE_COUNT * Stream::COUNT;

    pub(super) fn new(stream: Stream, signal: Signal) -> Self {
        Self { stream, signal }
    }

    /// Parse a dense wire byte into its stream and semantic signal.
    pub(super) fn from_byte(byte: u8) -> Result<Self, InvalidWireSignal> {
        let stream = Stream(byte % Stream::COUNT);
        let signal = Signal::from_state(byte / Stream::COUNT)
            .map_err(|_| InvalidWireSignal { byte, stream })?;
        Ok(Self { stream, signal })
    }

    /// Render the paired stream and semantic signal as one dense wire byte.
    pub(super) fn to_byte(self) -> u8 {
        self.signal.state() * Stream::COUNT + self.stream.index()
    }

    pub(super) fn into_parts(self) -> (Stream, Signal) {
        (self.stream, self.signal)
    }
}

/// A reserved dense signal byte and the stream encoded within it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct InvalidWireSignal {
    byte: u8,
    stream: Stream,
}

impl InvalidWireSignal {
    pub(super) fn byte(self) -> u8 {
        self.byte
    }

    pub(super) fn stream(self) -> Stream {
        self.stream
    }
}

#[cfg(test)]
mod tests;
