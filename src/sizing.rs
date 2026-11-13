// TODO-docs: Restate this guide's figures from measurements on physical
// links. They come from the test kit's simulated link, which leaves out
// loss, congestion control, and the time computation takes.
//! Guidance on choosing a
//! [`Peer::sync_memory_budget`](crate::Peer::sync_memory_budget).
//!
//! # Why does this parameter exist?
//!
//! `rumors` uses internal pipelining to cover for network latency, keeping your
//! network busy in both directions as much as possible, and therefore
//! dramatically speeding up gossip.
//! [`Peer::sync_memory_budget`](crate::Peer::sync_memory_budget) constrains how
//! much transient memory `rumors` is permitted to use for this pipeline during
//! each gossip session. When actual memory usage approaches this bound,
//! `rumors` will apply backpressure internally to slow the session down and
//! keep memory usage lower. This will slow down gossip in some specific
//! conditions, but ensure that `rumors` does not consume unexpected resources.
//!
//! In many situations, you will never need to tune this parameter, but if
//! you want to ensure memory usage is bounded more tightly (for example if you
//! expect many concurrent gossip sessions), you may want to adjust it.
//!
//! **Important:** [`Peer::sync_memory_budget`](crate::Peer::sync_memory_budget)
//! constrains the memory used by *one gossip session*. If `N` concurrent gossip
//! sessions could be happening at once, the total possible memory used could be
//! as much as `N` times the configured budget.
//! # How to use this guide
//!
//! This guide helps you choose a memory budget from what you know about your
//! deployment.
//!
//! There are two ways to make this decision:
//!
//! 1. Start with the memory you can afford to spend in the pessimal case, and
//!    find out how such a budget might affect latency; or
//! 2. Start with the parameters of your deployment, and find out what memory
//!    budget would be required to prevent backpressure.
//!
//! You will want to estimate these things about your intended deployment:
//!
//! - the bandwidth and latency of your anticipated network link, and their
//!   resultant bandwidth-delay product (BDP),
//! - roughly how many messages each replica will likely hold at any given time,
//!   and
//! - how much they will likely differ during a single gossip session.
//!
//! # What consumes transient memory?
//!
//! The biggest thing to take away is that the amount of transient memory
//! *actually used* during gossip (notwithstanding the configured budget) will
//! *increase* as:
//!
//! - the bandwidth-delay product (BDP) of your network link increases, or
//! - the amount of mutual divergence between two replicas increases, or
//! - the number of messages stored by each replica increases, or
//! - the mean size of each message *decreases* (larger messages means *less*
//!   gossip overhead!).
//!
//! The correlation is not linear in all these variables: in order to use a lot
//! of memory during synchronization, you really need *all three* to be quite
//! high. In particular, the following scenarios might *seem* like they could
//! use a lot of memory, but *they will not*:
//!
//! - highly divergent peers with many small messages, but on a *low-latency*
//!   network: **low memory cost**;
//! - peers with many small messages on a high latency network, but *gossiping
//!   frequently in small batches*: **low memory cost**;
//! - highly divergent peers on a high latency network, but *with a small number
//!   of small messages at any given time*: **low memory cost**.
//! - highly divergent peers on a high latency network with many messages, but
//!   *with large mean message size*: **low memory cost**.
//!
//! So, **high memory cost** occurs only with all of **high BDP** + **high
//! divergence** + **high number of messages** + **small message size**.
//!
//! # Choosing a budget
//!
//! Most of the time, you are best served by the default. If you have tighter
//! memory constraints for the pessimal scenario outlined above, continue
//! reading.
//!
//! ## The default and the minimum
//!
//! The default sync memory budget is
//! [`default::SYNC_MEMORY_BUDGET`](crate::default::SYNC_MEMORY_BUDGET) = **512
//! MiB**. This will not constrain the speed of gossip in most realistic
//! situations.
//!
//! The minimum is about **36.4 MiB**. Any specified budget below the minimum is
//! implicitly rounded up to the minimum. This will constrain the speed of
//! gossip in some situations, especially as latency, mutual divergence, and
//! message set size increase, and as mean message size decreases.
//!
//! Read on to understand what happens in between these extremes.
//!
//! # Estimating the effects of memory budget
//!
//! Running the real `rumors` gossip code on an in-memory deterministic
//! simulation harness, we can measure estimates for the amount of memory
//! consumed in different scenarios.
//!
//! The simulated link leaves out loss, congestion control, and the wall-clock
//! effect of computation time --- all of these slow down real gossip sessions
//! and therefore should *decrease* memory pressure relative to this worst-case
//! simulation.
//!
//! ### Parameters
//!
//! The figures in the rest of this guide are
//! based on the following simulation parameters:
//!
//! - Message size: fixed at 100 B (everything gets more favorable as this
//!   *increases*);
//! - Bandwidth: fixed at 1 Gb/s;
//! - Round trip latency:
//!   - 1 ms (BDP = 125 kB);
//!   - 10 ms (BDP = 1.25 MB);
//!   - 100 ms (BDP = 12.5 MB).
//!
//! **Note:** We only vary latency and not bandwidth in the figures below,
//! because varying bandwidth has the same effect. You might find this
//! surprising! Shouldn't constrained bandwidth slow us down differently than
//! latency? Not so here: constraining bandwidth means there's less to pipeline,
//! since less data can be in flight at the same time. This is a dual effect to
//! the role of latency: decreasing latency *also* means there's less to
//! pipeline, since it also means less data can be in flight at the same time.
//!
//! ## The required budget to achieve zero slowdown
//!
//! In the table below, we can see the minimal memory budget which results in a
//! 0% slowdown for gossip in simulation.
//!
//! | Messages per replica, % in common | 1 ms (BDP = 125 kB) | 10 ms (BDP = 1.25 MB) | 100 ms   (BDP = 12.5 MB) |
//! |-----------------------------------|---------------------|-----------------------|--------------------------|
//! | 100,000, none                     | 36.7 MiB            | 78.5 MiB              | 117.3 MiB                |
//! | 100,000, 90%                      | 52.6 MiB            | 67.8 MiB              | 67.8 MiB                 |
//! | 100,000, 99%                      | 41.7 MiB            | 67.8 MiB              | 67.8 MiB                 |
//! | 1,000,000, none                   | 36.8 MiB            | 74.2 MiB              | 421.1 MiB                |
//! | 1,000,000, 90%                    | 36.8 MiB            | 72.9 MiB              | 422.1 MiB                |
//! | 1,000,000, 99%                    | 38.6 MiB            | 102.3 MiB             | 214.3 MiB                |
//!
//!
//!
//! ### What does this tell us?
//!
//! - **Near BDP = 125 kB** (i.e. 1 ms latency at 1 Gb/s), gossip sessions use
//!   very close to the minimum budget.
//! - **Near 1.25 MB** (i.e. 10 ms latency at 1 Gb/s), a moderate budget
//!   suffices, around 100 MiB.
//! - **Near 12.5 MB** (i.e. 100 ms latency 1 Gb/s), the default budget is
//!   required to prevent slowdown during gossip.
//!
//! ## The latency cost of a smaller budget
//!
//! When you set a budget that is lower than the amount of memory the gossip
//! session would use, if unconstrained, `rumors` exerts backpressure.
//!
//! Based on the same simulation parameters as above, we compute the estimated
//! slowdown for a gossip session running at the *minimum budget*, relative to
//! the speed with which it would complete if unconstrained.
//!
//! Slowdown is measured
//! in "×", where "1.00 ×" means zero slowdown, and "2.00 ×" would mean "twice
//! as slow".
//!
//! | Messages per replica, % in common | 1 ms (BDP = 125 kB) | 10 ms (BDP = 1.25 MB) | 100 ms   (BDP = 12.5 MB) |
//! |-----------------------------------|---------------------|-----------------------|--------------------------|
//! | 100,000, none                     | 1.00 ×              | 1.97 ×                | 5.18 ×                   |
//! | 100,000, 90%                      | 1.00 ×              | 2.40 ×                | 2.93 ×                   |
//! | 100,000, 99%                      | 1.05 ×              | 2.27 ×                | 2.69 ×                   |
//! | 1,000,000, none                   | 1.00 ×              | 1.93 ×                | 14.5 ×                   |
//! | 1,000,000, 90%                    | 1.00 ×              | 5.56 ×                | 26.7 ×                   |
//! | 1,000,000, 99%                    | 1.01 ×              | 5.41 ×                | 9.80 ×                   |
//!
//! ### What does this tell us?
//!
//! - **Near BDP = 125 kB** (i.e. 1 ms latency at 1 Gb/s), the minimum budget
//!   will exert essentially no backpressure.
//! - **Near 1.25 MB** (i.e. 10 ms latency at 1 Gb/s), large message sets will
//!   see noticeable slowdown, especially when divergent.
//! - **Near 12.5 MB** (i.e. 100 ms latency 1 Gb/s), medium and especially large
//!   message sets will see significant slowdown.
