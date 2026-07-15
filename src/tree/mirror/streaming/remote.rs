//! Wire-bound proxy for the streaming mirror.
//!
//! The transport carries one interleaved byte stream containing 17 logical
//! streams in each direction. [`codec`] defines the common frame grammar: the
//! signal densely encodes the product of fourteen frame states and 17 stream
//! ids as `state * 17 + stream`. The states are each of the four reaction forms
//! (`Match`, empty/nonempty `Query`, and `Supply`) continuing, closing its
//! reply, or closing its stream and reply, plus bare `ReplyEnd` and
//! `StreamEnd`. Values 238 through 255 are invalid.
//! The phase schedule narrows that syntactic product further: each speaker
//! admits 223 placements and rejects 15 immediately after the signal byte,
//! before any frame body is read or written.
//!
//! An empty query occupies its signal alone; a nonempty query's one-byte
//! count-minus-one admits every fan from 1 through 256. A supplied leaf is the
//! exact-length-delimited canonical borsh encoding of its
//! [`Version`](crate::Version) and [`Message<T>`](crate::message::Message).
//! Once its whole body arrives the frame codec decodes that backend-neutral
//! pair exactly once; constructing a backend leaf and validating its
//! content-derived path belong to the incoming adapter.
//!
//! [`adapter`] retains the question scope omitted from protocol replies. It
//! attaches each newly asked scope to the exact outgoing frame which makes the
//! question publishable, derives supplied radices from leaf content, and uses
//! the backend's existing conversion fold to reconstruct one node per ascending
//! leaf run.
//!
//! [`session`] performs the physical multiplexing. Each logical stream has one
//! incoming and one outgoing handoff slot; a full slot propagates pressure to
//! the transport. Outgoing frames carry an exact, cancellation-safe
//! acknowledgement which releases their attached question only after the
//! frame is written and flushed.

mod adapter;
mod codec;
mod session;
