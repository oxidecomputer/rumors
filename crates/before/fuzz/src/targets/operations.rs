//! Public clock operations driven from arbitrary canonical storage.
//!
//! Ordinary operation traces begin from values the API produced. This target
//! first decodes an arbitrary clock, then either drives a stateful operation
//! script or compares against and receives an arbitrary decoded version. It
//! therefore reaches valid tree shapes that generated traces may miss.

use before::{Clock, Version};

use crate::input::{operations_input, Operations};

/// Run one decode-then-operate input.
pub fn run(data: &[u8]) {
    let Some(input) = operations_input(data) else {
        return;
    };
    let Ok(mut clock) = Clock::decode(input.clock) else {
        return;
    };
    match input.operations {
        Operations::Script => drive(&mut clock, input.tail),
        Operations::Message => {
            if let Ok(message) = Version::decode(input.tail) {
                let _ = *clock.version() >= message;
                clock.recv(&message);
            }
            let _ = clock.send();
        }
    }
}

/// Apply one public clock operation per script byte.
///
/// Forked children remain in `stash` until join or synchronization consumes
/// them. Every such child is disjoint from `clock`: it came from `clock.fork()`
/// or from the re-split performed by `clock.sync()`. An overlap rejection is
/// therefore a defect in those operations and must fail the fuzz target.
fn drive(clock: &mut Clock, script: &[u8]) {
    let mut stash = Vec::new();
    for &operation in script {
        match operation % 8 {
            0 => {
                clock.tick();
            }
            1 => stash.push(clock.fork()),
            2 => {
                if let Some(child) = stash.pop() {
                    clock
                        .join(child)
                        .expect("a child forked from this clock is disjoint from it");
                }
            }
            3 => {
                if let Some(mut child) = stash.pop() {
                    clock
                        .sync(&mut child)
                        .expect("a child forked from this clock is disjoint from it");
                    stash.push(child);
                }
            }
            4 => {
                let message = clock.send().clone();
                clock.recv(&message);
            }
            5 => {
                if let Some(child) = stash.last() {
                    let _ = clock.version() < child.version();
                    let _ = clock.version().concurrent(child.version());
                }
            }
            6 => {
                let message = clock.send().clone();
                let _ = *clock.version() >= message;
            }
            _ => {
                clock.ticks(1u128 << 100);
            }
        }
    }
}
