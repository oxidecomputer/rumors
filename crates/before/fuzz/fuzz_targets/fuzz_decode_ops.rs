//! Decode-then-operate fuzzing.
//!
//! Decode a value from the *front* of the input and use the trailing bytes as a
//! script that drives the full op set against it. This pushes
//! adversarially-shaped (but canonical) trees — ones the op-trace generator
//! never produces — through the working-form arithmetic and the repack-on-drop
//! boundary. The contract is simply: no panic, no overflow, on any
//! decoded-then-driven sequence.
//!
//! The first byte selects the value flavour, the next length-prefixed chunk is
//! the value's bytes, and the remainder is the op script (one op per byte).

#![no_main]

use libfuzzer_sys::fuzz_target;

use before::{Clock, Version};

fuzz_target!(|data: &[u8]| {
    let Some((&flavour, rest)) = data.split_first() else {
        return;
    };
    // A 1-byte length prefix carves the value bytes from the op script. Capped at the
    // remaining length so a hostile prefix can't index out of bounds.
    let Some((&len, rest)) = rest.split_first() else {
        return;
    };
    let split = (len as usize).min(rest.len());
    let (value_bytes, script) = rest.split_at(split);

    match flavour & 1 {
        // Decode a Clock and run the whole protocol against it.
        0 => {
            let Ok(mut clock) = Clock::decode(value_bytes) else {
                return;
            };
            drive_clock(&mut clock, script);
        }
        // Decode a Version (message) and exercise the version-facing ops.
        _ => {
            let Ok(mut clock) = Clock::decode(value_bytes) else {
                return;
            };
            if let Ok(msg) = Version::decode(script) {
                // Compare against, then receive, a hostile-but-canonical
                // message.
                let _ = *clock.version() >= msg;
                clock.recv(&msg);
            }
            let _ = clock.send();
        }
    }
});

/// Drive the full clock op set off a byte script: each byte selects one operation, so a
/// long script exercises a long op sequence on the decoded clock. `fork` keeps a stash of
/// forked children for `join`/`sync` to consume, so disjointness preconditions are met.
fn drive_clock(clock: &mut Clock, script: &[u8]) {
    let mut stash: Vec<Clock> = Vec::new();
    for &op in script {
        match op % 7 {
            0 => {
                clock.tick();
            }
            1 => stash.push(clock.fork()),
            2 => {
                if let Some(child) = stash.pop() {
                    // Re-join a disjoint fork; on the off chance it errors, keep the
                    // returned clock so we never lose the share.
                    if let Err(returned) = clock.join(child) {
                        stash.push(returned);
                    }
                }
            }
            3 => {
                if let Some(mut child) = stash.pop() {
                    let _ = clock.sync(&mut child);
                    stash.push(child);
                }
            }
            4 => {
                let msg = clock.send().clone();
                clock.recv(&msg);
            }
            5 => {
                if let Some(child) = stash.last() {
                    let _ = clock.version() < child.version();
                    let _ = clock.version().concurrent(child.version());
                }
            }
            _ => {
                let msg = clock.send().clone();
                let _ = *clock.version() >= msg;
            }
        }
    }
}
