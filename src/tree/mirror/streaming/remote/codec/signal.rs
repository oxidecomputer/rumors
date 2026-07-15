//! The dense signal byte and its semantic components.

use crate::tree::typed::height::{Height, Root, UnderRoot, Z};

/// Lowest node height carried by a logical stream.
pub const LEAF_HEIGHT: usize = <Z as Height>::HEIGHT;

/// Highest node height carried on the wire, immediately beneath the root.
pub const HIGHEST_STREAM_HEIGHT: usize = <UnderRoot as Height>::HEIGHT;

/// Number of streamed node heights, also the first height outside their range.
pub const STREAMED_HEIGHT_COUNT: usize = <Root as Height>::HEIGHT;

/// Successive streams for one speaker descend two node heights at a time.
const STREAM_HEIGHT_STRIDE: usize = 2;

/// Distance remainder selecting an initiator-owned interior height.
const INITIATOR_HEIGHT_PHASE: usize = 1;

/// Distance remainder selecting a responder-owned interior height.
const RESPONDER_HEIGHT_PHASE: usize = 0;

/// One of the logical streams carried in a direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Stream(u8);

impl Stream {
    /// Logical streams multiplexed into each transport direction.
    pub const COUNT: u8 = 17;

    /// Index of the final logical stream in a direction.
    pub const MAX: u8 = Self::COUNT - 1;

    /// Index shared by both speakers for the first, under-root stream.
    const FIRST: u8 = 0;

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
        if height == HIGHEST_STREAM_HEIGHT {
            return Some(Self(Self::FIRST));
        }
        if height == LEAF_HEIGHT {
            return Some(Self(Self::MAX));
        }
        let distance = HIGHEST_STREAM_HEIGHT.checked_sub(height)?;
        let div_rem = (
            distance / STREAM_HEIGHT_STRIDE,
            distance % STREAM_HEIGHT_STRIDE,
        );
        let index = match (speaker, div_rem) {
            (Speaker::Initiator, (quotient, INITIATOR_HEIGHT_PHASE)) => {
                quotient + INITIATOR_HEIGHT_PHASE
            }
            (Speaker::Responder, (quotient, RESPONDER_HEIGHT_PHASE)) => quotient,
            _ => return None,
        };
        Some(Self(u8::try_from(index).expect(
            "a streamed tree height yields a one-byte stream index",
        )))
    }

    /// Find the node height carried by this stream for `speaker`.
    pub fn height(self, speaker: Speaker) -> usize {
        match (speaker, self.0) {
            (_, Self::FIRST) => HIGHEST_STREAM_HEIGHT,
            (Speaker::Initiator, index) => {
                STREAMED_HEIGHT_COUNT - usize::from(index) * STREAM_HEIGHT_STRIDE
            }
            (Speaker::Responder, Self::MAX) => LEAF_HEIGHT,
            (Speaker::Responder, index) => {
                HIGHEST_STREAM_HEIGHT - usize::from(index) * STREAM_HEIGHT_STRIDE
            }
        }
    }
}

/// A programmatic stream index outside the wire's logical streams.
#[derive(Debug, Clone, Copy, thiserror::Error, PartialEq, Eq)]
pub enum StreamError {
    #[error("wire stream index {index} is outside the valid range")]
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
    /// Signal states occupied by each reaction form's flow variants.
    const STATE_COUNT: u8 = 3;

    /// Offset of a continuing reaction within its reaction form.
    const CONTINUE_STATE: u8 = 0;

    /// Offset of a reply-ending reaction within its reaction form.
    const REPLY_END_STATE: u8 = 1;

    /// Offset of a stream-ending reaction within its reaction form.
    const STREAM_END_STATE: u8 = 2;

    fn offset(self) -> u8 {
        match self {
            Flow::Continue => Self::CONTINUE_STATE,
            Flow::End(End::Reply) => Self::REPLY_END_STATE,
            Flow::End(End::Stream) => Self::STREAM_END_STATE,
        }
    }
}

/// The semantic state carried alongside a stream id in one signal byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    Match(Flow),
    QueryEmpty(Flow),
    Query(Flow),
    Supply(Flow),
    End(End),
}

impl Signal {
    /// Distance between adjacent dense semantic state codes.
    const STATE_STRIDE: u8 = 1;

    /// Reaction forms represented by the signal grammar.
    const REACTION_COUNT: u8 = 4;

    /// Bare end forms represented by the signal grammar.
    const END_COUNT: u8 = 2;

    /// First state occupied by a match reaction.
    const MATCH_STATE: u8 = Flow::CONTINUE_STATE;

    /// First state occupied by an empty-query reaction.
    const QUERY_EMPTY_STATE: u8 = Self::MATCH_STATE + Flow::STATE_COUNT;

    /// First state occupied by a nonempty-query reaction.
    const QUERY_STATE: u8 = Self::QUERY_EMPTY_STATE + Flow::STATE_COUNT;

    /// First state occupied by a supplied-leaf reaction.
    const SUPPLY_STATE: u8 = Self::QUERY_STATE + Flow::STATE_COUNT;

    /// State occupied by a bare reply end.
    const REPLY_END_STATE: u8 = Self::SUPPLY_STATE + Flow::STATE_COUNT;

    /// State occupied by a bare stream end.
    const STREAM_END_STATE: u8 = Self::REPLY_END_STATE + Self::STATE_STRIDE;

    /// Total semantic states in a signal, before pairing with a stream.
    const STATE_COUNT: u8 = Flow::STATE_COUNT * Self::REACTION_COUNT + Self::END_COUNT;

    const STATES: [Signal; Self::STATE_COUNT as usize] = [
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

    fn state(self) -> u8 {
        match self {
            Signal::Match(flow) => Self::MATCH_STATE + flow.offset(),
            Signal::QueryEmpty(flow) => Self::QUERY_EMPTY_STATE + flow.offset(),
            Signal::Query(flow) => Self::QUERY_STATE + flow.offset(),
            Signal::Supply(flow) => Self::SUPPLY_STATE + flow.offset(),
            Signal::End(End::Reply) => Self::REPLY_END_STATE,
            Signal::End(End::Stream) => Self::STREAM_END_STATE,
        }
    }

    fn from_state(state: u8) -> Result<Self, InvalidSignalState> {
        Self::STATES
            .get(usize::from(state))
            .copied()
            .ok_or(InvalidSignalState { state })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("semantic signal state {state} is outside the valid range")]
struct InvalidSignalState {
    state: u8,
}

impl InvalidSignalState {
    fn state(self) -> u8 {
        self.state
    }
}

/// A semantic signal paired with the logical stream encoded beside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WireSignal {
    stream: Stream,
    signal: Signal,
}

impl WireSignal {
    /// Bytes occupied by a densely encoded signal.
    pub const ENCODED_LEN: usize = std::mem::size_of::<u8>();

    /// Byte values occupied by the valid `(signal state, stream)` product.
    pub const BYTE_COUNT: u8 = Signal::STATE_COUNT * Stream::COUNT;

    /// Pair a checked stream with a semantic signal.
    pub fn new(stream: Stream, signal: Signal) -> Self {
        Self { stream, signal }
    }

    /// Parse a dense wire byte into its stream and semantic signal.
    pub fn from_byte(byte: u8) -> Result<Self, InvalidWireSignal> {
        let stream = Stream(byte % Stream::COUNT);
        let signal =
            Signal::from_state(byte / Stream::COUNT).map_err(|source| InvalidWireSignal {
                byte,
                stream,
                source,
            })?;
        Ok(Self { stream, signal })
    }

    /// Render the paired stream and semantic signal as one dense wire byte.
    pub fn to_byte(self) -> u8 {
        self.signal.state() * Stream::COUNT + self.stream.index()
    }

    /// Separate the checked stream and semantic signal.
    pub fn into_parts(self) -> (Stream, Signal) {
        (self.stream, self.signal)
    }
}

/// A reserved dense signal byte and the stream encoded within it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("signal byte {byte:#04x} encodes an invalid semantic state")]
pub struct InvalidWireSignal {
    byte: u8,
    stream: Stream,
    #[source]
    source: InvalidSignalState,
}

impl InvalidWireSignal {
    /// Return the rejected dense wire byte.
    pub fn byte(self) -> u8 {
        self.byte
    }

    /// Return the stream component which was valid independently of the state.
    pub fn stream(self) -> Stream {
        self.stream
    }

    /// Return the invalid semantic state component.
    pub fn state(self) -> u8 {
        self.source.state()
    }
}

#[cfg(test)]
mod tests;
