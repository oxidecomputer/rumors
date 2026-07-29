//! A guided first session: two peers, one universe, two messages, one redaction.
//!
//! In this tutorial we will build a two-peer gossip network inside a single
//! process: seed a universe, send a message and see it, bootstrap a second
//! peer that arrives already holding it, keep the pair converged over a
//! long-lived connection, and finally redact the message and watch it
//! vanish from both replicas. Along the way we will meet
//! [`Peer`](crate::Peer), [`Rumors`](crate::Rumors),
//! [`Snapshot`](crate::Snapshot), the in-memory
//! [`Link`](crate::Link), and the
//! [`gossip_when`](crate::Rumors::gossip_when) driver.
//!
//! Everything runs over the in-memory link, so there is no network to
//! configure. Binding a real transport is the [`link`](crate::link)
//! module's topic; how a session works under the hood is the
//! [`reconciliation`](crate::reconciliation) page's. Neither is needed
//! here.
//!
//! Every step below is a complete program: paste it over your `main.rs`,
//! run it, and compare against the output shown.
//!
//! # Step 1: a place to stand
//!
//! Start a fresh binary crate (`cargo new meetings`) and add three
//! dependencies:
//!
//! ```toml
//! [dependencies]
//! rumors = "0.1"
//! tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
//! futures = "0.3"
//! ```
//!
//! We drive everything with Tokio for convenience only; the crate itself
//! is [runtime-independent](crate#runtime-independence) — sessions and
//! observers are plain futures and streams. `futures` supplies the
//! `StreamExt` adapter we will use in step 5.
//!
//! Replace `main.rs` with an async main that can report this crate's
//! errors:
//!
//! ```
//! #[tokio::main]
//! async fn main() -> Result<(), rumors::Error> {
//!     Ok(())
//! }
//! ```
//!
//! `cargo run` should compile and print nothing. That empty program is
//! the foundation every following step builds on.
//!
//! # Step 2: seed a universe
//!
//! A gossip network — a *universe* — begins when exactly one process calls
//! [`seed`](crate::Peer::seed); every other participant will join by
//! bootstrapping from a member. (In a real deployment, pick the seeder by
//! fiat — a `--seed` flag on one host is plenty; to start leaderlessly
//! instead, see [Bootstrapping without
//! consensus](crate::Peer#bootstrapping-without-consensus).) Seed the
//! universe and trade the [`Peer`](crate::Peer) for the
//! [`Rumors`](crate::Rumors) handle we will work with:
//!
//! ```
//! use rumors::Peer;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), rumors::Error> {
//!     let alice = Peer::<String>::seed().into_rumors();
//!
//!     println!("alice holds {} messages", alice.snapshot().len());
//!     Ok(())
//! }
//! ```
//!
//! The output should be exactly:
//!
//! ```text
//! alice holds 0 messages
//! ```
//!
//! Notice the type parameter: this universe carries `String` messages, and
//! every peer in it will agree on that.
//!
//! # Step 3: send a message and see it
//!
//! Now send something, and read it back through a
//! [`snapshot`](crate::Rumors::snapshot) — a cheap, point-in-time view of
//! the live set:
//!
//! ```
//! use rumors::Peer;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), rumors::Error> {
//!     let alice = Peer::<String>::seed().into_rumors();
//!
//!     alice.send("the meeting is at noon".to_string()).await?;
//!
//!     use futures::TryStreamExt;
//!     let mut messages = alice.snapshot().iter();
//!     while let Some((_key, _version, message)) = messages.try_next().await? {
//!         println!("alice holds: {message}");
//!     }
//!     Ok(())
//! }
//! ```
//!
//! ```text
//! alice holds: the meeting is at noon
//! ```
//!
//! A `send` commits when awaited; chaining several changes into one
//! atomic commit is [`Batch`](crate::Batch)'s job. Reading back goes
//! through a [`Snapshot`](crate::Snapshot), whose message stream yields a
//! key and a version alongside each message — we ignore them for now, and
//! the key returns in step 6. (With the in-memory set every stream item
//! is immediately ready and the error impossible; the `?` is the shape of
//! the API, not a hazard here.)
//!
//! # Step 4: bootstrap Bob
//!
//! A second peer does not call `seed` — it would mint a separate universe,
//! forever unable to gossip with this one. Instead it
//! [`bootstrap`](crate::Peer::bootstrap)s through any established member:
//! one session against Alice hands Bob a full replica and a donated
//! identity. Sessions run over a [`Link`](crate::Link);
//! [`link::memory`](crate::link::memory) makes the in-process pair, and
//! each link end is a conduit to exactly one counterparty. Alice serves
//! her end from a spawned task while Bob joins through the other:
//!
//! ```
//! use rumors::Peer;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), rumors::Error> {
//!     let alice = Peer::<String>::seed().into_rumors();
//!     alice.send("the meeting is at noon".to_string()).await?;
//!
//!     // Alice serves one gossip session on her end of the link...
//!     let (mut near, mut far) = rumors::link::memory();
//!     let serve = alice.clone();
//!     let server = tokio::spawn(async move { serve.gossip(&mut far).await.unwrap() });
//!
//!     // ...and Bob joins the universe through the other end.
//!     let bob = Peer::<String>::bootstrap()
//!         .join(&mut near)
//!         .await?
//!         .expect("alice is established, not herself bootstrapping")
//!         .into_rumors();
//!     server.await.expect("alice's serving task");
//!
//!     use futures::TryStreamExt;
//!     let mut messages = bob.snapshot().iter();
//!     while let Some((_key, _version, message)) = messages.try_next().await? {
//!         println!("bob holds: {message}");
//!     }
//!     Ok(())
//! }
//! ```
//!
//! ```text
//! bob holds: the meeting is at noon
//! ```
//!
//! Notice that Bob arrives converged: the message Alice sent before the
//! two ever met is already in his replica. Serving a bootstrap took
//! nothing special from Alice — an ordinary [`gossip`](crate::Rumors::gossip)
//! call handles the donation automatically.
//!
//! # Step 5: keep the pair converged
//!
//! To *keep* the pair converged, each side drives its own end of a long-lived
//! link — the *bridge* — with [`gossip_when`](crate::Rumors::gossip_when): the
//! driver initiates a session whenever its `when` stream ticks (if there's been
//! local change since the last gossip), and serves whatever the remote
//! initiates. That second half is what to remember about the bridge — a driver
//! serves only while polled, so a converged pair needs **one driver per end**,
//! each polled, even if only one end ever originates news. Reusing the
//! bootstrap link here would be legal (a session that ends `Ok` leaves its link
//! at a clean session boundary, ready for the next), but we moved its far end
//! into Alice's serving task, so we mint a fresh pair for the bridge:
//!
//! ```
//! use futures::StreamExt;
//! use rumors::Peer;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), rumors::Error> {
//!     let alice = Peer::<String>::seed().into_rumors();
//!     alice.send("the meeting is at noon".to_string()).await?;
//!
//!     let (mut near, mut far) = rumors::link::memory();
//!     let serve = alice.clone();
//!     let server = tokio::spawn(async move { serve.gossip(&mut far).await.unwrap() });
//!     let bob = Peer::<String>::bootstrap()
//!         .join(&mut near)
//!         .await?
//!         .expect("alice is established, not herself bootstrapping")
//!         .into_rumors();
//!     server.await.expect("alice's serving task");
//!
//!     // A second, long-lived link between them, one driver per end.
//!     let (mut alice_side, mut bob_side) = rumors::link::memory();
//!
//!     alice.send("bring the slides".to_string()).await?;
//!
//!     let mut alice_drive = alice.gossip_when(alice.changes(), &mut alice_side);
//!     let mut bob_drive = bob.gossip_when(bob.changes(), &mut bob_side);
//!
//!     // Alice's change signal initiates; Bob's driver serves. One
//!     // session converges the pair, and each driver reports it.
//!     let (pushed, served) = tokio::join!(alice_drive.next(), bob_drive.next());
//!     pushed.expect("alice's driver is running")?;
//!     served.expect("bob's driver is running")?;
//!
//!     assert_eq!(bob.snapshot().len(), 2);
//!     println!("bob holds {} messages", bob.snapshot().len());
//!     Ok(())
//! }
//! ```
//!
//! ```text
//! bob holds 2 messages
//! ```
//!
//! Providing [`changes`](crate::Rumors::changes) as the `when` stream
//! implements push-on-change; an interval stream would gossip on a cadence
//! instead. In an application these drivers live in long-running tasks,
//! polled for as long as the connection should stay converged — here we
//! poll each exactly once, for exactly one session.
//!
//! # Step 6: redact, and watch it vanish
//!
//! The meeting is over; take the message back. Redaction needs the
//! message's [`Key`](crate::Key), and keys come back out of observation —
//! snapshots and the [message streams](crate#how-should-you-observe-messages)
//! attach one to every message — so we find the key by looking, then hand
//! it to [`redact`](crate::Rumors::redact) and let the drivers spread the
//! deletion. With this final addition, the whole program reads:
//!
//! ```
//! use futures::StreamExt;
//! use rumors::Peer;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), rumors::Error> {
//!     let alice = Peer::<String>::seed().into_rumors();
//!     alice.send("the meeting is at noon".to_string()).await?;
//!
//!     let (mut near, mut far) = rumors::link::memory();
//!     let serve = alice.clone();
//!     let server = tokio::spawn(async move { serve.gossip(&mut far).await.unwrap() });
//!     let bob = Peer::<String>::bootstrap()
//!         .join(&mut near)
//!         .await?
//!         .expect("alice is established, not herself bootstrapping")
//!         .into_rumors();
//!     server.await.expect("alice's serving task");
//!
//!     let (mut alice_side, mut bob_side) = rumors::link::memory();
//!
//!     alice.send("bring the slides".to_string()).await?;
//!
//!     let mut alice_drive = alice.gossip_when(alice.changes(), &mut alice_side);
//!     let mut bob_drive = bob.gossip_when(bob.changes(), &mut bob_side);
//!
//!     let (pushed, served) = tokio::join!(alice_drive.next(), bob_drive.next());
//!     pushed.expect("alice's driver is running")?;
//!     served.expect("bob's driver is running")?;
//!
//!     assert_eq!(bob.snapshot().len(), 2);
//!     println!("bob holds {} messages", bob.snapshot().len());
//!
//!     // New: find the key by observing, redact, and drive one more session.
//!     use futures::TryStreamExt;
//!     let snapshot = alice.snapshot();
//!     let (key, _version, _message) = snapshot
//!         .iter()
//!         .try_filter(|(_, _, message)| {
//!             std::future::ready(message.as_str() == "the meeting is at noon")
//!         })
//!         .try_next()
//!         .await?
//!         .expect("alice still holds the meeting message");
//!     alice.redact(key).await?;
//!
//!     let (pushed, served) = tokio::join!(alice_drive.next(), bob_drive.next());
//!     pushed.expect("alice's driver is running")?;
//!     served.expect("bob's driver is running")?;
//!
//!     assert_eq!(alice.snapshot().len(), 1);
//!     assert_eq!(bob.snapshot().len(), 1);
//!     let mut messages = bob.snapshot().iter();
//!     while let Some((_key, _version, message)) = messages.try_next().await? {
//!         println!("bob still holds: {message}");
//!     }
//!     Ok(())
//! }
//! ```
//!
//! ```text
//! bob holds 2 messages
//! bob still holds: bring the slides
//! ```
//!
//! Notice what happened on Bob's side: the message is simply *gone*, on a
//! replica that never called `redact` — and no deletion record replaced it
//! anywhere; how that works without tombstones is the
//! [`reconciliation`](crate::reconciliation) page's story.
//!
//! # You have converged
//!
//! You have built a two-peer universe: seeded it, replicated a message to
//! a peer that joined later, kept both ends converged with change-driven
//! drivers, and redacted a message contagiously out of every replica.
//! Every piece here scales past the demo: more peers mean more
//! `bootstrap` calls and more driven links, nothing else new.
//!
//! Where to go next:
//!
//! - The full peer lifecycle — retirement, its outcomes, reclaiming the
//!   [`Peer`](crate::Peer) from its handles — is one runnable example on
//!   [`Peer`](crate::Peer).
//! - Reacting to messages as they arrive (instead of polling snapshots) is
//!   the [observers](crate#how-should-you-observe-messages) section's
//!   decision guide.
//! - A real deployment binds a network transport as a
//!   [`Link`](crate::Link) — the [`link`](crate::link) module states the
//!   contract and ships a conformance suite for your implementation.
//! - Surviving restarts without stranding identity is
//!   [`Bookmark`](crate::Bookmark)'s job.
//! - And for how a session actually reconciles two replicas —
//!   [`reconciliation`](crate::reconciliation).
