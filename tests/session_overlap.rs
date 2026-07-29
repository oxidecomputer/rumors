//! Overlapping-session convergence: the total-set instruments for the
//! interleaving regime that serial schedules never sample.
//!
//! A gossip session forks its working state when it starts and installs
//! when it ends; everything the peer does in between — local sends,
//! redactions, whole *other* sessions — lands between that fork and that
//! install. The suites here drive such overlaps deterministically, at
//! every poll prefix or at generated ones, and hold the network to the
//! only acceptable outcome: after quiescence, every peer holds exactly
//! the set the schedule's history prescribes. The symptom they exist to
//! catch is silent divergence — a message nobody redacted vanishing, or
//! peers converging on different sets — regardless of which layer's
//! defect produces it. The discovering incident was a dependency fault
//! (`imbl` issue 161) whose only visible face was exactly that symptom.

mod common;

use std::collections::BTreeMap;

use common::oracle::{readout, readout_multiset};
use common::overlap::{self, arb_overlap_schedule, execute_overlap_and_quiesce};
use common::wire::{bootstrap_fork, wire_gossip};
use proptest::prelude::*;
use rumors::{Key, Rumors};

/// Build the deterministic witness fleet: a seed peer holding `n` unit
/// messages and two bootstrapped forks, all converged (a bootstrap copies
/// the served content, so no further gossip is needed).
fn converged_trio(n: u64) -> (Rumors<u64>, Rumors<u64>, Rumors<u64>) {
    let a = rumors::Peer::seed().into_rumors();
    for v in 0..n {
        a.send(v);
    }
    let b = bootstrap_fork(&a);
    let c = bootstrap_fork(&a);
    (a, b, c)
}

/// Drive the three peers to agreement and return the common readout.
///
/// Panics if they fail to agree within a bounded number of full-mesh
/// rounds: overlapped sessions must still converge.
fn converge(a: &Rumors<u64>, b: &Rumors<u64>, c: &Rumors<u64>) -> BTreeMap<Key, u64> {
    const ROUNDS: usize = 8;
    for _ in 0..ROUNDS {
        wire_gossip(a, b);
        wire_gossip(a, c);
        wire_gossip(b, c);
        let (ra, rb, rc) = (
            readout(&a.snapshot()),
            readout(&b.snapshot()),
            readout(&c.snapshot()),
        );
        if ra == rb && rb == rc {
            return ra;
        }
    }
    panic!("three peers failed to agree within {ROUNDS} full-mesh rounds");
}

/// No interleaving of two honest sessions may lose a message nobody
/// redacted.
///
/// Peer A holds 25 messages, with forks B and C converged on them. B
/// redacts one message; A then runs a session with C *overlapped* around
/// its session with B — the C-session is opened first, parked after `n`
/// polls, resumed only after the B-session (which honors the redaction)
/// has fully installed. Swept over every message as the redaction target
/// and every parking prefix `n` up to the session's own length, the
/// converged outcome must always be exactly the original set minus the
/// one redacted message: one deliberate deletion, no collateral.
///
/// Discovering incident: `imbl` issue 161 — an upstream map-diff fault
/// whose downstream symptom was precisely an innocent leaf silently
/// deleted under this overlap, at 2 of the 25 sweep positions. The sweep
/// is total, so any regression with the same *symptom* fails here no
/// matter which layer produces it.
#[test]
fn overlapped_install_never_loses_innocent_messages() {
    const MESSAGES: u64 = 25;

    // Calibrate the poll sweep's ceiling: how many polls one converged
    // session takes on this exact fleet shape. The trigger window (fork
    // done, install pending) lies strictly inside it.
    let session_polls = {
        let (a, _b, c) = converged_trio(MESSAGES);
        let mut probe = overlap::open(&a, &c);
        let mut polls = 0usize;
        while !probe.step(1) {
            polls += 1;
            assert!(polls < 1 << 20, "calibration session never completed");
        }
        polls
    };

    let keys: Vec<Key> = {
        let (a, _, _) = converged_trio(MESSAGES);
        readout(&a.snapshot()).into_keys().collect()
    };

    let mut violations = Vec::new();
    for &redacted in &keys {
        for n in 0..=session_polls {
            let (a, b, c) = converged_trio(MESSAGES);
            let mut expected = readout(&a.snapshot());
            expected.remove(&redacted);

            b.redact(redacted);
            // S2 (A <-> C, both still converged at fork time) opens
            // first and parks after `n` polls...
            let s2 = {
                let mut s2 = overlap::open(&a, &c);
                s2.step(n);
                s2
            };
            // ...S1 (A <-> B) installs the honored redaction at A...
            wire_gossip(&a, &b);
            // ...and S2 resumes and installs after it.
            s2.finish();

            let converged = converge(&a, &b, &c);
            if converged != expected {
                violations.push((redacted, n, converged.len(), expected.len()));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "overlapped installs diverged from the one deliberate redaction \
         (redacted key, S2 poll prefix, converged len, expected len): {violations:?}",
    );
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 48,
        ..ProptestConfig::default()
    })]

    /// Generated overlapping-session schedules converge to the oracle.
    ///
    /// Fleets of 2–4 peers run schedules mixing sends, observed-key
    /// redactions, whole sessions, and sessions opened, parked at
    /// generated poll prefixes, and closed across other events —
    /// starting from a converged base large enough to span several
    /// radix-fan chunks. After every session closes and the fleet
    /// quiesces, all peers must hold the same live set, and that set
    /// must equal the spec-shaped oracle's projection (every insert not
    /// deliberately redacted). This is the standing sample of the
    /// interleaving space where install-time defects hide; the serial
    /// schedule suites cannot reach it by construction.
    #[test]
    fn overlapping_schedules_converge_to_the_oracle(
        schedule in arb_overlap_schedule(any::<u64>(), 2..=4, 24),
    ) {
        let (peers, oracle) = execute_overlap_and_quiesce(&schedule);
        let expected = oracle.expected_live();
        let readouts: Vec<_> = peers
            .iter()
            .map(|p| p.local.snapshot())
            .collect();
        for (i, snapshot) in readouts.iter().enumerate() {
            prop_assert_eq!(
                readout_multiset(snapshot),
                expected.clone(),
                "peer {} diverged from the oracle after quiescence",
                i,
            );
        }
        // Key-level identity across peers, not just value multisets:
        // the same content must live at the same keys everywhere.
        for pair in readouts.windows(2) {
            prop_assert_eq!(readout(&pair[0]), readout(&pair[1]));
        }
    }
}
