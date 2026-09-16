//! Lazy balanced-fork iterators: [`iter::Party`](Party) and
//! [`iter::Clock`](Clock).
//!
//! They hand out `n` shallow shares of a [`Party`](crate::Party) (or
//! [`Clock`](crate::Clock)) in one balanced split, generating each share on
//! demand and folding any unconsumed shares back into `self` when dropped
//! before full consumption.
//!
//! See [`Party::forks`](crate::Party::forks) and
//! [`Clock::forks](crate::Clock::forks).
//!
//! ```
//! use before::{iter, Party};
//! let mut p = Party::seed();
//! let forks: iter::Party<'_> = p.forks(3u64);
//! assert_eq!(forks.size_hint(), (3, Some(3)));
//! let shares: Vec<Party> = forks.collect();
//! assert_eq!(shares.len(), 3);
//! ```
pub use crate::{clock::Forks as Clock, party::Forks as Party};
