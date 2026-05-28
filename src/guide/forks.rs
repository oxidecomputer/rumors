//! How to gossip with many peers in parallel.
//!
//! Only one [`Local<T, Original>`](crate::Local) may exist per party
//! per process, and only the [`Original`](crate::Original) may
//! [`message`](crate::Local::message) or [`redact`](crate::Local::redact).
//! To gossip with several remote peers simultaneously, hold the
//! original in one task and hand a [`fork`](crate::Local::fork) — a
//! cheap structurally-shared snapshot — to each gossip task. When the
//! tasks complete, fold their results back into the original with
//! [`process`](crate::Local::process) (or the `+` / `+=` operators on
//! [`Forked`](crate::Forked) sides).
//!
//! ```no_run
//! # // Auto-deriving Send across the deep mirror-protocol future hits the
//! # // default doctest recursion limit; bump it (the library itself sets
//! # // this in its crate root).
//! # #![recursion_limit = "256"]
//! use rumors::{Local, ignore};
//! use tokio::net::TcpStream;
//!
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut alice: Local<String, _> = Local::for_party("alice", 0).unwrap();
//! alice.message(["greetings".to_string()], ignore).await;
//!
//! let peers = ["bob:9000", "carol:9000", "dave:9000"];
//! let mut handles = Vec::new();
//! for peer in peers {
//!     // One fork per outbound gossip — cheap (copy-on-write share).
//!     let fork = alice.fork();
//!     handles.push(tokio::spawn(async move {
//!         let stream = TcpStream::connect(peer).await?;
//!         let (mut r, mut w) = stream.into_split();
//!         fork.gossip(&mut r, &mut w, ignore).await
//!     }));
//! }
//!
//! // Fold each completed fork's observations back into alice.
//! for handle in handles {
//!     let result = handle.await.expect("task panic")?;
//!     alice.process(result.fork(), ignore).await;
//! }
//! # Ok(())
//! # }
//! ```
//!
//! `alice.process(fork, ...)` is the typed checkpoint: it accepts only
//! a [`Forked`](crate::Forked), so the compiler prevents you from
//! accidentally feeding a fork's observations back into a different
//! party's `Original` (which would be a soundness violation —
//! party-identifier uniqueness is at stake).
//!
//! # On native threads
//!
//! For the synchronous surface
//! ([`sync::Local`](crate::sync::Local)), the equivalent pattern uses
//! [`std::thread::spawn`] and
//! [`sync::Local::process`](crate::sync::Local::process). Forks of
//! [`sync::Local<T, Forked>`](crate::sync::Local) are `Send` for any
//! `T: Send`, so they move into threads freely.
