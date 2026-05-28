//! How to continue a party across process restarts.
//!
//! Save the local event counter durably before shutdown; reload it
//! on startup and pass it back as `start`. The new instance's `start`
//! **must** be at least as large as the previous instance's
//! [`event()`](crate::Local::event), or you will silently and
//! contagiously corrupt the rumor set across the network. There is
//! no runtime check for this across processes.
//!
//! Atomic writes matter: a torn write that leaves a smaller counter
//! on disk would re-introduce the corruption hazard on the next run.
//! Use the standard write-rename pattern.
//!
//! ```no_run
//! use rumors::{Local, ignore};
//! use std::fs;
//! use std::path::Path;
//!
//! /// Read the persisted counter, defaulting to 0 if no checkpoint exists.
//! fn load_counter(path: &Path) -> std::io::Result<u64> {
//!     match fs::read_to_string(path) {
//!         Ok(s) => Ok(s.trim().parse().expect("corrupted counter file")),
//!         Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(0),
//!         Err(e) => Err(e),
//!     }
//! }
//!
//! /// Atomically replace the counter file: write to a sibling, then rename.
//! fn save_counter(path: &Path, value: u64) -> std::io::Result<()> {
//!     let tmp = path.with_extension("tmp");
//!     fs::write(&tmp, value.to_string())?;
//!     fs::rename(&tmp, path)
//! }
//!
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() -> std::io::Result<()> {
//! let path = Path::new("/var/lib/myapp/alice.counter");
//! # let path = Path::new("/tmp/alice.counter.example");
//!
//! let start = load_counter(path)?;
//! let mut alice: Local<String, _> = Local::for_party("alice", start)
//!     .expect("alice already running in this process");
//!
//! alice.message(["resumed".to_string()], ignore).await;
//!
//! // ... application work ...
//!
//! // On shutdown, checkpoint the event counter before dropping the Local.
//! save_counter(path, alice.event())?;
//! # let _ = fs::remove_file(path);
//! # Ok(())
//! # }
//! ```
