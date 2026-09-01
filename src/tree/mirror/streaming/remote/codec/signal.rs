//! The frame opener's two items — the stream a frame rides and its
//! semantic state — and the phase schedule that admits a state on a stream.

use crate::observe::Role;
use crate::tree::mirror::cbor;
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

    /// Return this stream's wire index.
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

    /// Classify the protocol phase carried by this speaker's stream.
    fn class(self, speaker: Speaker) -> StreamClass {
        match (speaker, self.0) {
            (Speaker::Initiator, Self::FIRST) => StreamClass::OpeningSupplies,
            (Speaker::Responder, Self::FIRST) => StreamClass::OpeningReply,
            (Speaker::Initiator, Self::MAX) => StreamClass::LeafParentReplies,
            (Speaker::Responder, Self::MAX) => StreamClass::TerminalLeafReplies,
            (_, _) => StreamClass::InteriorReplies,
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

impl Speaker {
    /// Return the role speaking in the opposite transport direction.
    pub fn other(self) -> Self {
        match self {
            Speaker::Initiator => Speaker::Responder,
            Speaker::Responder => Speaker::Initiator,
        }
    }

    /// This role in the observation hook's public vocabulary.
    pub fn role(self) -> Role {
        match self {
            Speaker::Initiator => Role::Initiator,
            Speaker::Responder => Role::Responder,
        }
    }
}

/// The phase-specific signal grammar of a logical stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum StreamClass {
    /// The initiator's early whole-subtree supplies: its exclusive root
    /// children, shipped at the opening without waiting to be asked (the
    /// opening *question* itself never occupies a frame — its content
    /// rides the greeting).
    #[error("the initiator's opening supplies")]
    OpeningSupplies,
    #[error("the responder's opening reply")]
    OpeningReply,
    #[error("an interior reply stream")]
    InteriorReplies,
    #[error("the initiator's leaf-parent replies")]
    LeafParentReplies,
    #[error("the responder's terminal leaf replies")]
    TerminalLeafReplies,
}

/// A logical reply boundary or transport-level stream boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum End {
    /// End the current reply while leaving its stream open.
    Reply,
    /// End a logical stream between replies.
    Stream,
}

/// Whether another reaction follows or this reaction ends its reply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flow {
    /// Another reaction follows in the current reply.
    Continue,
    /// This reaction ends its reply.
    End,
}

impl Flow {
    /// Signal states occupied by each reaction form's flow variants.
    const STATE_COUNT: u8 = 2;

    /// Offset of a continuing reaction within its reaction form.
    const CONTINUE_STATE: u8 = 0;

    /// Offset of a reply-ending reaction within its reaction form.
    const REPLY_END_STATE: u8 = 1;

    fn offset(self) -> u8 {
        match self {
            Flow::Continue => Self::CONTINUE_STATE,
            Flow::End => Self::REPLY_END_STATE,
        }
    }
}

/// The semantic state a frame carries in its state item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    Match(Flow),
    QueryEmpty(Flow),
    Query(Flow),
    Supply(Flow),
    End(End),
}

impl Signal {
    /// Distance between adjacent state codes.
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

    /// Semantic states in the wire's roster; state codes run from zero to
    /// one below this.
    pub const STATE_COUNT: u8 = Flow::STATE_COUNT * Self::REACTION_COUNT + Self::END_COUNT;

    /// The wire's state roster: the signal each state code names.
    const STATES: [Signal; Self::STATE_COUNT as usize] = [
        Signal::Match(Flow::Continue),
        Signal::Match(Flow::End),
        Signal::QueryEmpty(Flow::Continue),
        Signal::QueryEmpty(Flow::End),
        Signal::Query(Flow::Continue),
        Signal::Query(Flow::End),
        Signal::Supply(Flow::Continue),
        Signal::Supply(Flow::End),
        Signal::End(End::Reply),
        Signal::End(End::Stream),
    ];

    /// The state code this signal travels as.
    pub fn state(self) -> u8 {
        match self {
            Signal::Match(flow) => Self::MATCH_STATE + flow.offset(),
            Signal::QueryEmpty(flow) => Self::QUERY_EMPTY_STATE + flow.offset(),
            Signal::Query(flow) => Self::QUERY_STATE + flow.offset(),
            Signal::Supply(flow) => Self::SUPPLY_STATE + flow.offset(),
            Signal::End(End::Reply) => Self::REPLY_END_STATE,
            Signal::End(End::Stream) => Self::STREAM_END_STATE,
        }
    }

    /// The signal a state code names.
    pub fn from_state(state: u8) -> Result<Self, InvalidSignalState> {
        Self::STATES
            .get(usize::from(state))
            .copied()
            .ok_or(InvalidSignalState { state })
    }
}

/// A state code outside the wire's state roster.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("signal state {state} names no frame state")]
pub struct InvalidSignalState {
    state: u8,
}

impl InvalidSignalState {
    /// Return the rejected state code.
    pub fn state(self) -> u8 {
        self.state
    }
}

/// A signal placed on a stream: the two unsigned-int items every frame
/// opens with, the stream's index then the signal's state code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WireSignal {
    stream: Stream,
    signal: Signal,
}

impl WireSignal {
    /// Bytes the two items occupy on the wire: every stream index and
    /// state code is below 24, so each item is a one-byte head.
    pub const ENCODED_LEN: usize =
        cbor::head_len(Stream::MAX as u64) + cbor::head_len((Signal::STATE_COUNT - 1) as u64);

    /// Pair a stream with a signal valid for its speaker and protocol phase.
    #[cfg(test)]
    pub fn new(
        speaker: Speaker,
        stream: Stream,
        signal: Signal,
    ) -> Result<Self, InvalidSignalPlacement> {
        Self { stream, signal }.validate(speaker)
    }

    /// Interpret a frame's decoded stream and state items for `speaker`:
    /// an index outside the logical streams or a code outside the state
    /// roster is reserved, and a known pair outside the phase schedule is
    /// an invalid placement.
    pub fn decode(speaker: Speaker, index: u64, state: u64) -> Result<Self, DecodeSignalError> {
        let stream = u8::try_from(index)
            .ok()
            .and_then(|index| Stream::new(index).ok())
            .ok_or(DecodeSignalError::Stream { index })?;
        let signal = u8::try_from(state)
            .ok()
            .and_then(|state| Signal::from_state(state).ok())
            .ok_or(DecodeSignalError::State { stream, state })?;
        Self { stream, signal }
            .validate(speaker)
            .map_err(DecodeSignalError::Placement)
    }

    /// Enforce the signal subset admitted by this speaker's stream phase.
    fn validate(self, speaker: Speaker) -> Result<Self, InvalidSignalPlacement> {
        let class = self.stream.class(speaker);
        let valid = match class {
            // One supplies-only reply (empty when pruning left nothing),
            // then the stream end: the opening carries answers the
            // responder is about to ask for, never questions of its own.
            StreamClass::OpeningSupplies => {
                matches!(self.signal, Signal::Supply(_) | Signal::End(_))
            }
            StreamClass::OpeningReply => true,
            StreamClass::InteriorReplies => true,
            StreamClass::LeafParentReplies => !matches!(self.signal, Signal::Query(_)),
            StreamClass::TerminalLeafReplies => {
                matches!(self.signal, Signal::Supply(Flow::End) | Signal::End(_))
            }
        };
        if valid {
            Ok(self)
        } else {
            Err(InvalidSignalPlacement {
                stream: self.stream,
                signal: self.signal,
                class,
            })
        }
    }

    /// Separate the checked stream and semantic signal.
    pub fn into_parts(self) -> (Stream, Signal) {
        (self.stream, self.signal)
    }
}

/// A known signal placed on a stream where the protocol forbids it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("signal {signal:?} on stream {} is invalid for {class}", stream.index())]
pub struct InvalidSignalPlacement {
    stream: Stream,
    signal: Signal,
    class: StreamClass,
}

impl InvalidSignalPlacement {
    /// Return the stream the signal was placed on.
    pub fn stream(self) -> Stream {
        self.stream
    }

    /// Return the rejected signal.
    pub fn signal(self) -> Signal {
        self.signal
    }

    /// Return the protocol phase whose signal grammar was violated.
    pub fn class(self) -> StreamClass {
        self.class
    }
}

/// A frame opener the speaker's grammar rejects: a reserved stream index
/// or state code, or a known signal in an invalid phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DecodeSignalError {
    /// The stream item names no logical stream.
    #[error("frame names stream {index}, outside the logical streams")]
    Stream { index: u64 },
    /// The state item names no frame state; the stream it rides is known
    /// and reported as the error's origin.
    #[error("frame carries state {state}, outside the state roster")]
    State { stream: Stream, state: u64 },
    #[error(transparent)]
    Placement(#[from] InvalidSignalPlacement),
}

impl DecodeSignalError {
    /// Return the stream the rejected frame rides, when its stream item
    /// was valid.
    pub fn stream(self) -> Option<Stream> {
        match self {
            DecodeSignalError::Stream { .. } => None,
            DecodeSignalError::State { stream, .. } => Some(stream),
            DecodeSignalError::Placement(invalid) => Some(invalid.stream()),
        }
    }
}

#[cfg(test)]
mod tests;
