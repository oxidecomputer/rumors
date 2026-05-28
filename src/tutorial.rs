//! Tutorial: gossip two peers in one process, then redact a rumor.
//!
//! This is a guided lesson. By the end you will have driven the full
//! lifecycle of a [`Local`](crate::Local) rumor set — originate, gossip,
//! redact, persist — through working code that compiles as you read.
//! Each step builds conceptually on the previous and ends in an
//! `assert!` you can run.
//!
//! No prior familiarity with `rumors` or CRDTs is assumed. If you would
//! like the design picture first, read [`crate::explanation`]; if you
//! want a recipe for a specific task, see [`crate::guide`]. Otherwise,
//! start here.
//!
//! # 1. Create a peer
//!
//! A peer is identified by an arbitrary byte string; the caller must
//! keep these globally unique across the gossip network. The second
//! argument is the local event counter to resume from — for a brand-new
//! party, pass `0`.
//!
//! ```
//! use rumors::Local;
//!
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() {
//! let alice = Local::<String, _>::for_party("alice", 0).unwrap();
//! assert_eq!(alice.event(), 0); // no operations applied yet
//! # }
//! ```
//!
//! [`Local::for_party`](crate::Local::for_party) returns a
//! [`Local<T, Original>`](crate::Local) — the singleton that may
//! originate new messages and redactions for this party.
//!
//! # 2. Originate a message
//!
//! Inserting a message fires the `on_message` callback once for each
//! newly-observed message, with an opaque [`Key`](crate::Key) (you'll
//! need it later if you want to redact), the causal
//! [`Version`](crate::Version) at which it was observed, and the
//! message itself wrapped in an [`Arc`](std::sync::Arc).
//!
//! ```
//! use rumors::{Local, Key};
//!
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() {
//! let mut alice = Local::<String, _>::for_party("alice", 0).unwrap();
//! let mut keys: Vec<Key> = Vec::new();
//! alice.message(
//!     ["hello".to_string(), "world".to_string()],
//!     async |k, _v, _m| keys.push(k),
//! ).await;
//! assert_eq!(keys.len(), 2);
//! assert_eq!(alice.event(), 2); // two operations applied
//! # }
//! ```
//!
//! The two keys are distinct even though the values would have been
//! interchangeable: every insert advances the local version vector
//! before the key is derived, so two inserts of the same value get
//! two different keys.
//!
//! # 3. Bring up a second peer and gossip
//!
//! Two peers reconcile by exchanging only the parts of their rumor sets
//! that differ. For a runnable example without a real network, we use
//! [`tokio::io::duplex`] as an in-memory bidirectional pipe.
//!
//! Both sides must drive [`Local::gossip`](crate::Local::gossip)
//! concurrently — the protocol expects symmetric activity. We drive
//! them together with the `tokio::join!` macro.
//!
//! ```
//! use rumors::{Local, ignore};
//!
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() {
//! // Set up the pipe.
//! let (a, b) = tokio::io::duplex(1024);
//! let (mut a_r, mut a_w) = tokio::io::split(a);
//! let (mut b_r, mut b_w) = tokio::io::split(b);
//!
//! // Alice has a message; bob does not.
//! let mut alice: Local<String, _> = Local::for_party("alice", 0).unwrap();
//! alice.message(["hello".to_string()], ignore).await;
//! let bob: Local<String, _> = Local::for_party("bob", 0).unwrap();
//!
//! // Gossip. Bob's callback fires for every message he learns.
//! let mut bob_learned: Vec<String> = Vec::new();
//! let (alice_out, bob_out) = tokio::join!(
//!     alice.gossip(&mut a_r, &mut a_w, ignore),
//!     bob.gossip(&mut b_r, &mut b_w,
//!         async |_k, _v, m| bob_learned.push(m.as_ref().clone()),
//!     ),
//! );
//! let (_alice, _bob) = (alice_out.unwrap(), bob_out.unwrap());
//!
//! assert_eq!(bob_learned, vec!["hello".to_string()]);
//! # }
//! ```
//!
//! Alice's callback didn't fire — she had nothing new to learn from
//! bob. Convergence is symmetric in *outcome* (both peers agree) but
//! asymmetric in *observation* (callbacks fire only for what's new on
//! the receiving side).
//!
//! # 4. Redact a message
//!
//! Any peer holding a [`Key`](crate::Key) can [`redact`] it. The
//! redaction propagates through the same gossip protocol as the
//! original message: once any peer has it, transitive gossip evicts
//! the message network-wide without consensus.
//!
//! [`redact`]: crate::Local::redact
//!
//! ```
//! use rumors::{Local, Key};
//!
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() {
//! let mut alice = Local::<String, _>::for_party("alice", 0).unwrap();
//! let mut keys: Vec<Key> = Vec::new();
//! alice.message(
//!     ["stale announcement".to_string()],
//!     async |k, _, _| keys.push(k),
//! ).await;
//!
//! // Redact. After this, the rumor will not propagate via gossip.
//! alice.redact(keys);
//! assert_eq!(alice.event(), 2); // insert plus redact, both count
//! # }
//! ```
//!
//! To verify that the rumor has actually been evicted from a peer's
//! live set, gossip with a fresh, empty peer and observe that nothing
//! arrives: an empty third-party absorbs only live messages, so a
//! redacted rumor produces no callbacks.
//!
//! # 5. Persist state across restarts
//!
//! When a peer is created via [`Local::for_party`](crate::Local::for_party),
//! the second argument seeds the local event counter. If the same party
//! identifier is ever reused — across process restarts, for instance —
//! the new instance's `start` value **must** be at least as large as
//! the previous instance's [`event()`](crate::Local::event). Violating
//! this can contagiously corrupt the rumor set network-wide, so save
//! `event()` durably before shutdown and reload it on startup.
//!
//! ```
//! use rumors::{Local, ignore};
//!
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() {
//! let mut alice = Local::<String, _>::for_party("alice", 0).unwrap();
//! alice.message(["greetings".to_string()], ignore).await;
//!
//! // Imagine the process is about to exit. Save event() somewhere durable.
//! let resume_at = alice.event();
//! drop(alice);
//!
//! // On the next run, pass the saved value back as `start`.
//! let alice = Local::<String, _>::for_party("alice", resume_at).unwrap();
//! assert_eq!(alice.event(), resume_at);
//! # }
//! ```
//!
//! See the [`persist`](crate::guide::persist) how-to for a working
//! recipe that writes the counter to a file atomically.
//!
//! # Where to next
//!
//! - To put a peer on a real socket, see
//!   [`crate::guide::sockets`].
//! - To run gossip with many peers in parallel, see
//!   [`crate::guide::forks`].
//! - To shrink the wire footprint, see
//!   [`crate::guide::compress`].
//! - For the design picture — what `rumors` is, isn't, and why — see
//!   [`crate::explanation`].
