//! Wire-bound proxy for the streaming mirror.
//!
//! The transport carries one interleaved byte stream containing 17 logical
//! streams in each direction. [`codec`] defines the common frame grammar: the
//! signal densely encodes the product of fourteen frame states and 17 stream
//! ids as `state * 17 + stream`. The states are each of the four reaction forms
//! (`Match`, empty/nonempty `Query`, and `Supply`) continuing, closing its
//! reply, or closing its stream and reply, plus bare `ReplyEnd` and
//! `StreamEnd`. Values 238 through 255 are invalid.
//!
//! An empty query occupies its signal alone; a nonempty query's one-byte
//! count-minus-one admits every fan from 1 through 256. A supplied leaf is the
//! exact-length-delimited canonical borsh encoding of its
//! [`Version`](crate::Version) and [`Message<T>`](crate::message::Message).
//! Once its whole body arrives the frame codec decodes that backend-neutral
//! pair exactly once; constructing a backend leaf and validating its
//! content-derived path belong to the incoming adapter.

mod codec;
