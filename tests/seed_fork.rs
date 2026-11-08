//! Seeding a network after the process forks.
//!
//! A forked child is a copy of its parent, generator state included, so a peer
//! that drew its network's secret from a generator living in the process would
//! seed the same network in the parent and in every child forked after one
//! shared draw: one secret, one identifier, one seed party, and two universes
//! that gossip as one. [`Peer::seed`] draws from the operating system on every
//! call instead.
//!
//! The test forks its own process, so it relies on nextest's process per test:
//! a harness running other tests on other threads could fork while one of them
//! holds a lock the child then needs.

#![cfg(unix)]

use std::io::{Read, Write, pipe};
use std::panic::catch_unwind;
use std::process::exit;

use fork::{Fork, WEXITSTATUS, WIFEXITED};
use rumors::Peer;

/// The child's exit status when it reported its network.
const REPORTED: i32 = 0;

/// The child's exit status when seeding or reporting failed.
const FAILED: i32 = 1;

/// The identifier of a freshly seeded network, as text.
fn seeded_network() -> String {
    Peer::<u64>::seed().network().to_string()
}

/// A network seeded in a forked child and one seeded in its parent after the
/// fork have different identifiers.
///
/// The parent seeds a network before it forks, so any generator state the
/// process keeps is initialized, and copied into the child, before either side
/// seeds again.
#[test]
fn networks_seeded_on_both_sides_of_a_fork_differ() {
    // A draw before the fork, as a peer that seeded earlier would make.
    let _ = seeded_network();
    let (mut reader, mut writer) = pipe().expect("the system creates a pipe");
    match fork::fork().expect("the system forks the test process") {
        Fork::Child => {
            drop(reader);
            // The child reports and exits without returning to the test
            // harness, whatever happens, so the harness never runs twice.
            let reported = catch_unwind(move || {
                writer
                    .write_all(seeded_network().as_bytes())
                    .expect("the parent holds the pipe's read end");
            });
            exit(if reported.is_ok() { REPORTED } else { FAILED });
        }
        Fork::Parent(child) => {
            drop(writer);
            let ours = seeded_network();
            let mut theirs = String::new();
            reader
                .read_to_string(&mut theirs)
                .expect("the child's report is text");
            let status = fork::waitpid(child).expect("the child is ours to wait for");
            assert!(
                WIFEXITED(status) && WEXITSTATUS(status) == REPORTED,
                "the child exits after reporting its network (status {status})",
            );
            assert_ne!(ours, theirs, "the parent and the child seeded one network");
        }
    }
}
