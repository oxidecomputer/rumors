//! Lazy iterators returned by [`Party::forks`](crate::Party::forks) and
//! [`Clock::forks`](crate::Clock::forks).
//!
//! Each iterator divides one party into balanced shares and constructs those
//! shares on demand.
//!
//! ```
//! use before::{iter, Party};
//! let mut p = Party::seed();
//! let forks: iter::PartyForks<'_> = p.forks(3u64);
//! assert_eq!(forks.size_hint(), (3, Some(3)));
//! let shares: Vec<Party> = forks.collect();
//! assert_eq!(shares.len(), 3);
//! ```
pub use crate::{clock::ClockForks, party::PartyForks};
