//! How to compress the gossip wire.
//!
//! The wire is uncompressed: party identifiers appear inline in every
//! version vector exchanged during gossip, and version vectors travel
//! on every frame. That metadata channel is highly redundant and
//! compresses easily. Payload bytes (Blake3 hashes, your borsh-encoded
//! messages) generally do not compress further. Wrapping the reader
//! and writer in a streaming compressor is the recommended deployment.
//!
//! # Synchronous: `zstd`
//!
//! Wrap the sync I/O halves in `zstd::stream::{Encoder, Decoder}`.
//! `Encoder::new(write, level)` returns an
//! [`AutoFinishEncoder`](https://docs.rs/zstd/latest/zstd/stream/write/struct.Encoder.html)
//! once you call `.auto_finish()`, which flushes the trailer when
//! the encoder is dropped — important when `gossip` returns and you
//! want a complete frame on the wire.
//!
//! ```no_run
//! use rumors::sync::{Local, ignore};
//! use std::net::TcpStream;
//!
//! let raw_write = TcpStream::connect("127.0.0.1:9000")?;
//! let raw_read = raw_write.try_clone()?;
//!
//! let mut write = zstd::stream::Encoder::new(raw_write, 3)?.auto_finish();
//! let mut read = zstd::stream::Decoder::new(raw_read)?;
//!
//! let alice: Local<String, _> = Local::for_party("alice", 0).unwrap();
//! let _alice = alice.gossip(&mut read, &mut write, ignore)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Asynchronous: `async-compression`
//!
//! For the async surface, the `async-compression` crate exposes
//! `tokio`-aware encoder/decoder wrappers. (`async-compression` is not
//! a dependency of `rumors`; add it to your application's
//! `Cargo.toml`.)
//!
//! ```ignore
//! use async_compression::tokio::bufread::ZstdDecoder;
//! use async_compression::tokio::write::ZstdEncoder;
//! use rumors::{Local, ignore};
//! use tokio::io::BufReader;
//! use tokio::net::TcpStream;
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let stream = TcpStream::connect("127.0.0.1:9000").await?;
//! let (read, write) = stream.into_split();
//!
//! let mut read = ZstdDecoder::new(BufReader::new(read));
//! let mut write = ZstdEncoder::new(write);
//!
//! let alice: Local<String, _> = Local::for_party("alice", 0).unwrap();
//! let alice = alice.gossip(&mut read, &mut write, ignore).await?;
//! # let _ = alice;
//! # Ok(())
//! # }
//! ```
//!
//! # Picking a level
//!
//! `zstd` level 3 is a reasonable default: strong ratio on the
//! version-vector metadata, very fast. Higher levels gain little
//! because the payload bytes are already maximum-entropy hashes.
//!
//! # On the handshake
//!
//! The compressor sits *outside* the rumors protocol: the 8-byte
//! [`PROTOCOL_MAGIC`](crate::PROTOCOL_MAGIC) +
//! [`PROTOCOL_VERSION`](crate::PROTOCOL_VERSION) preamble is what your
//! peer reads first, but it reads it through the *decoder*, so the
//! peer must be using a matching compressor too. Either both sides
//! compress or neither does — the wire has no negotiation for this.
