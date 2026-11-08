// TODO-docs: Restate this guide's figures from measurements on physical
// links. They come from the test kit's simulated link, which leaves out
// loss, congestion control, and the time computation takes.
//! Choosing a synchronization memory budget.
//!
//! [`Peer::sync_memory_budget`](crate::Peer::sync_memory_budget) sets how much
//! memory each synchronization session may hold. This guide helps you choose
//! its value from what you know about your deployment: the bandwidth and
//! round-trip time of your links, roughly how many messages your replicas hold,
//! and how much they differ. The setter's documentation states the exact
//! contract: what the budget counts, what it leaves out, and how closely a
//! session keeps to it.
//!
//! # What the budget does
//!
//! A session finds what two replicas hold differently by asking its peer about
//! parts of the set, and it keeps many questions in flight at once so that it
//! does not wait out a round trip for each one. Everything in flight holds
//! memory. The budget limits that memory: within it, a session keeps in flight
//! as much as it can use, and at the budget, it waits for answers to free
//! memory before it asks more.
//!
//! The budget therefore trades memory for time. A session on a smaller budget
//! reaches the same result as on a larger one; it only takes longer, because it
//! waits out more round trips. The budget is a limit, not a reservation: a
//! session holds only what its work in flight needs, so a budget larger than a
//! session can use costs that session nothing.
//!
//! The budget applies to each session separately, and sessions running at once
//! can each hold up to it. Each peer's budget limits only its own side of a
//! session, though a remote peer with a smaller budget can slow the session for
//! both sides.
//!
//! # The default and the minimum
//!
//! The default is
//! [`default::SYNC_MEMORY_BUDGET`](crate::default::SYNC_MEMORY_BUDGET), 512
//! MiB.
//!
//! The minimum is [`MIN_SYNC_MEMORY_BUDGET`](crate::MIN_SYNC_MEMORY_BUDGET),
//! about 36.4 MiB on 64-bit targets, and a session raises any smaller budget,
//! zero included, to it. The minimum covers what every session sets aside
//! whatever its budget, as the setter's documentation itemizes. A
//! [`target_message_size`](crate::Peer::target_message_size) below the default
//! lowers it a little.
//!
//! # Choosing a budget
//!
//! Keep the default unless memory is tight. In the simulations behind this
//! guide ([described below](#how-far-to-trust-these-figures)), a budget of at
//! most 422.1 MiB was enough for every session between replicas of up to
//! 1,000,000 messages to run exactly as fast as with no limit, on every link
//! and however much the replicas differed, so the default leaves room to spare.
//!
//! To size a deployment, count on each session holding up to its budget:
//! synchronization can hold the budget times the number of sessions you run at
//! once. Leave room besides for what the budget does not cover, such as the
//! replica's growth, observers' backlogs, and your transport's buffers ([the
//! setter lists each](crate::Peer#what-the-budget-does-not-cover)).
//!
//! When memory is tight, start from your link's *bandwidth-delay product*: its
//! bandwidth times its round-trip time, which is how many bytes the link
//! carries in one round trip. The larger the product, the more work a session
//! must keep in flight to keep the link busy, and the more budget it can use.
//! The simulated links all ran at 1 Gb/s, with round trips of 1 ms (a LAN, with
//! a product of 125 kB), 10 ms (a metro link, 1.25 MB), and 100 ms (a long-haul
//! link, 12.5 MB). Each figure below is a budget that was enough for a session
//! to run exactly as fast as with no limit:
//!
//! | Messages per replica, shared | 125 kB   | 1.25 MB   | 12.5 MB   |
//! |------------------------------|----------|-----------|-----------|
//! | 100,000, none                | 36.7 MiB | 78.5 MiB  | 117.3 MiB |
//! | 100,000, 90%                 | 52.6 MiB | 67.8 MiB  | 67.8 MiB  |
//! | 100,000, 99%                 | 41.7 MiB | 67.8 MiB  | 67.8 MiB  |
//! | 1,000,000, none              | 36.8 MiB | 74.2 MiB  | 421.1 MiB |
//! | 1,000,000, 90%               | 36.8 MiB | 72.9 MiB  | 422.1 MiB |
//! | 1,000,000, 99%               | 38.6 MiB | 102.3 MiB | 214.3 MiB |
//!
//! Read the table by your link's product:
//!
//! - **Near 125 kB, a short round trip,** the minimum costs almost nothing. At
//!   the minimum, every simulated session on the LAN took less than one round
//!   trip longer than with no limit, and at most 5% longer in all; between
//!   replicas of 1,000,000 messages that share nothing, it took as long to
//!   within a hundredth of a percent.
//! - **Near 1.25 MB,** a moderate budget suffices: on the metro link, 102.3 MiB
//!   was enough for every pair of replicas, at either size.
//! - **Near 12.5 MB, a long round trip,** the budget a session can use grows
//!   with the replicas. Beyond the minimum, budget a few hundred bytes for each
//!   message a replica holds: about 400 B at 1,000,000 messages, or 190 B when
//!   the replicas share 99%, and up to 850 B at 100,000 messages. At 1,000,000
//!   messages, replicas that share 90% needed as much as replicas that share
//!   nothing.
//!
//! These figures cover replicas of up to 1,000,000 messages and products of up
//! to 12.5 MB. For larger replicas or links, or whenever the choice matters,
//! measure on your own link. Each session reports
//! [`SessionStats::budget_waits`](crate::SessionStats::budget_waits) in the
//! [`Gossiped`](crate::Gossiped) it returns, and a count of zero means the
//! budget never held that session back. Raise the budget if sessions you care
//! about report waits and take longer than you can accept.
//!
//! # What a low or a high budget costs
//!
//! A budget below what a session can use makes the session slower, and more so
//! the longer the round trip. At the minimum, the simulated sessions took this
//! many times as long as with no limit:
//!
//! | Messages per replica, shared | 125 kB | 1.25 MB | 12.5 MB |
//! |------------------------------|--------|---------|---------|
//! | 100,000, none                | 1.00   | 1.97    | 5.18    |
//! | 100,000, 90%                 | 1.00   | 2.40    | 2.93    |
//! | 100,000, 99%                 | 1.05   | 2.27    | 2.69    |
//! | 1,000,000, none              | 1.00   | 1.93    | 14.5    |
//! | 1,000,000, 90%               | 1.00   | 5.56    | 26.7    |
//! | 1,000,000, 99%               | 1.01   | 5.41    | 9.80    |
//!
//! For example, between replicas of 1,000,000 messages that share nothing, a
//! session on the long-haul link took 1.5 s with no limit and 22 s at the
//! minimum. Between the minimum and the budgets in the first table, the
//! slowdown shrinks as the budget grows: no simulated session ran more than
//! about 0.002 round trips slower at a larger budget than at a smaller one.
//!
//! A budget above what a session can use changes nothing for that session: it
//! runs exactly as it would with no limit. The cost of a high budget is the
//! memory it lets each session hold when the session can use it, times the
//! number of sessions you run at once.
//!
//! # How far to trust these figures
//!
//! The figures come from the library's own session code, run over a simulated
//! link. Both peers ran on the same budget, with replicas of 100,000 or
//! 1,000,000 messages of 100 bytes each, sharing none, 90%, or 99% of their
//! messages, over the three links above. Each pair of replicas ran once with no
//! limit and then at budgets from the minimum upward.
//!
//! - The simulated link leaves out loss, congestion control, and the time
//!   computation takes. It is described entirely by its bandwidth-delay
//!   product: a 1 Gb/s link with a 100 ms round trip behaves exactly as a 100
//!   Gb/s link with a 1 ms round trip.
//! - Other message sizes, other overlaps, and larger replicas were not
//!   simulated.
//! - Some figures in the first table are computed from the session with no
//!   limit, as the smallest budget that could never have made it wait, rather
//!   than measured at that budget.
//! - The sessions ran on 64-bit Linux on x86-64, where they are exactly
//!   repeatable. Rerunning a sample on illumos gave slightly different memory
//!   counts, and durations that differed by at most about 0.002 round trips.
//!
//! Use the figures to choose a starting point, and measure on your own link
//! where the choice matters.
