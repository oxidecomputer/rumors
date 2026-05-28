//! How to gossip over a real TCP connection.
//!
//! [`Local::gossip`](crate::Local::gossip) wants an
//! [`AsyncRead`](tokio::io::AsyncRead) and an
//! [`AsyncWrite`](tokio::io::AsyncWrite) as **two** mutable
//! references. A `tokio::net::TcpStream` is both, but you can't take
//! two `&mut` borrows of the same value, so use `into_split` on the
//! stream to break it into owned read and write halves.
//!
//! ```no_run
//! use rumors::{Local, ignore};
//! use tokio::net::TcpStream;
//!
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let stream = TcpStream::connect("127.0.0.1:9000").await?;
//! let (mut read, mut write) = stream.into_split();
//!
//! let alice: Local<String, _> = Local::for_party("alice", 0).unwrap();
//! let alice = alice.gossip(&mut read, &mut write, ignore).await?;
//! # let _ = alice;
//! # Ok(())
//! # }
//! ```
//!
//! Drive both ends concurrently. The peer accepting the connection
//! does the same with its accepted `TcpStream`.
//! The 8-byte protocol handshake is automatic; an incompatible peer
//! surfaces as [`Error::MagicMismatch`](crate::Error::MagicMismatch) or
//! [`Error::VersionMismatch`](crate::Error::VersionMismatch) on the
//! return of [`gossip`](crate::Local::gossip), before any rumor-set
//! state is touched.
//!
//! # Synchronous I/O
//!
//! The sync surface ([`sync::Local::gossip`](crate::sync::Local::gossip))
//! takes [`Read`](std::io::Read) and [`Write`](std::io::Write)
//! similarly. For a [`std::net::TcpStream`], use
//! [`try_clone`](std::net::TcpStream::try_clone) to get a second
//! handle for the reader.
//!
//! ```no_run
//! use rumors::sync::{Local, ignore};
//! use std::net::TcpStream;
//!
//! let write = TcpStream::connect("127.0.0.1:9000")?;
//! let mut read = write.try_clone()?;
//! let mut write = write;
//!
//! let alice: Local<String, _> = Local::for_party("alice", 0).unwrap();
//! let _alice = alice.gossip(&mut read, &mut write, ignore)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Connection lifecycle — accept, retry, backoff, reconnect after a
//! peer disconnects mid-session — is your responsibility. A clean
//! return from [`gossip`](crate::Local::gossip) means reconciliation
//! completed; the underlying socket is still open and reusable for
//! another session, application traffic, or shutdown.
