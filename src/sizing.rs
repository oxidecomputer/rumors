// TODO-docs: Restate every figure in this guide from measurements on real
// links, then rewrite the guide around them. No measurement on a real link
// backs these claims of the guide:
// - that a session slows only "slightly" before its memory use reaches the
//   budget;
// - that a link with a larger bandwidth-delay product needs a larger budget
//   to keep it busy, and that a session can use more budget on a link with
//   a long round trip;
// - that the budget a session can use before nothing waits grows in
//   proportion to the messages the larger replica holds.
// The minimum budget is `MIN_SYNC_MEMORY_BUDGET`, and a session raises
// every smaller budget to it, so a figure measured at a smaller budget
// describes a budget no session runs on; measure each figure at or above
// the minimum.
//! Choosing a synchronization memory budget.
//!
//! [`Peer::sync_memory_budget`](crate::Peer::sync_memory_budget) sets the
//! most memory a session holds, its messages on their way included. Beyond
//! what every budget sets aside, it buys room for keeping questions in
//! flight while their answers cross the link, so that a session overlaps
//! its round trips instead of waiting out each one. The
//! setter documents what the budget covers, what it does not, and how
//! closely a session keeps to it.
//!
//! # What the budget buys
//!
//! A session finds what two replicas hold differently by asking questions
//! about parts of the two sets: each question asks the peer what it holds
//! in one part, and the peer's answer carries the messages of that part
//! the session lacks, or narrower questions. Each question in flight costs
//! memory: what the session built to ask it, and room set aside for the
//! answer it invites.
//!
//! - **Every budget sets aside** room for one question and one answer at
//!   each level of the session's comparison, so that it always makes
//!   progress: about 26.5 MiB, the most that work can hold, though it
//!   realistically holds a megabyte or two. Every budget also sets aside
//!   room for the session's fixed state, which it holds whatever the
//!   budget, about 7 MiB, and for the messages on their way through the
//!   connection, about two messages' worth at the message-size target in
//!   each direction, about 5.7 MiB at the default target. The setter's
//!   documentation says more.
//! - **The smallest budget,**
//!   [`MIN_SYNC_MEMORY_BUDGET`](crate::MIN_SYNC_MEMORY_BUDGET), adds room
//!   for the largest single question or answer beside what every budget
//!   sets aside. A session raises any smaller budget, zero included, to
//!   it. At the smallest budget a session pipelines little: beyond one
//!   question and one answer at each level of its comparison, it keeps in
//!   flight only what the added room holds, and otherwise waits for
//!   answers before it asks its next questions. This is the declared
//!   behavior of such a budget, not a failure; the session completes,
//!   holding at most the smallest budget.
//! - **A larger budget** keeps more questions in flight at once, up to the
//!   point where nothing waits for memory. Beyond that point more budget
//!   changes nothing: a session that never needs more runs exactly as it
//!   would with an unlimited budget.
//!
//! The budget counts memory held and room set aside for answers not yet
//! received, which may never be used in full. So a session can slow
//! slightly before its memory use reaches the budget, most of all between
//! nearly identical replicas, whose answers carry little.
//!
//! # Choosing a budget
//!
//! Choose the budget as the memory you are willing to let one session hold
//! for its work in flight, and multiply it by the sessions you run at once.
//! Leave room besides for the replica, its growth from what sessions
//! receive, observers, what each session holds beyond its budget, such as
//! its tasks and any message larger than the budget covers (the setter's
//! documentation lists each), and your transport's buffers.
//!
//! Start from the default of 512 MiB if you can afford it. Lower it when
//! memory is scarce: a smaller budget is still correct, and only limits how
//! much work overlaps each round trip, so a session that runs short of
//! budget takes longer, more so on a link with a long round trip. Raise it
//! when your sets are large, your links have a large bandwidth-delay
//! product (fast links with a long round trip), and measured
//! synchronizations between very different replicas spend their time
//! waiting for replies. A higher budget has little benefit
//! when the session already keeps the link busy, or when processing speed
//! limits progress.
//!
//! Each peer's budget bounds its own side of a session, so a more
//! constrained peer limits its own memory, and can set the session's pace.
//!
//! # Why the link matters
//!
//! To keep a link busy, a session needs enough questions in flight that
//! their answers cover the wait for replies. The **bandwidth-delay
//! product**, `BDP = bandwidth × RTT`, expresses that amount in bytes: 1
//! Gb/s with a 100 ms round trip and 100 Gb/s with a 1 ms round trip both
//! have a BDP of 12.5 MB. A link with a larger BDP needs more in flight,
//! and so a larger budget, to keep it busy.
//!
//! How much budget a session can use grows with how much of the two sets
//! differs. Replicas that agree, or a peer joining with an empty replica,
//! need little: a joining peer receives the set as a stream. Two large
//! replicas that differ throughout, as when a peer has fallen far behind or
//! two peers accumulated many messages apart, can use budget in proportion
//! to the messages the larger replica holds before nothing waits, more on
//! links with a long round trip. Measure your workload to choose the
//! budget.
