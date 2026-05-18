//! Group B — pairwise gossip semantics.

//! All "post-gossip equality" assertions compare *readouts* rather than
//! `Local` values directly: `Local::eq` includes the party tag, so two
//! peers can have identical content but distinct parties and never
//! compare equal. The public-API-meaningful equality is "live content
//! multiset, with consistent `Key`s" — which is exactly what `readout`
//! returns.

use proptest::collection::vec;
use proptest::prelude::*;
use rumors::{Key, Local};

use crate::oracle::readout;
use crate::peer::gossip_step_local;
use crate::wire::wire_gossip;

#[derive(Debug, Clone)]
pub enum LocalAction {
    Insert(u64),
    Redact(usize),
}

pub fn arb_local_actions() -> impl Strategy<Value = Vec<LocalAction>> {
    vec(
        prop_oneof![
            4 => any::<u64>().prop_map(LocalAction::Insert),
            1 => any::<usize>().prop_map(LocalAction::Redact),
        ],
        0..=16,
    )
}

pub fn build_local(party: &str, actions: &[LocalAction]) -> Local<u64> {
    let mut local: Local<u64> = Local::for_party(party);
    let mut keys: Vec<Key> = Vec::new();
    for a in actions {
        match a {
            LocalAction::Insert(v) => {
                local.message([*v], |k, _, _| keys.push(k));
            }
            LocalAction::Redact(idx) => {
                if !keys.is_empty() {
                    let k = keys[idx % keys.len()];
                    local.redact([k]);
                }
            }
        }
    }
    local
}

proptest! {
    /// B1: after one bidirectional `gossip_step`, the two peers' live
    /// content (as exposed through readout) is equal.
    #[test]
    fn b1_convergence(
        a_actions in arb_local_actions(),
        b_actions in arb_local_actions(),
    ) {
        let mut a = build_local("alice", &a_actions);
        let mut b = build_local("bob", &b_actions);
        gossip_step_local(&mut a, &mut b);
        prop_assert_eq!(readout(&a), readout(&b));
    }

    /// B2: the final pair `(a, b)` has identical content regardless of
    /// which side initiates the merge first.
    #[test]
    fn b2_symmetry(
        a_actions in arb_local_actions(),
        b_actions in arb_local_actions(),
    ) {
        let a0 = build_local("alice", &a_actions);
        let b0 = build_local("bob", &b_actions);

        // Forward: A merges B, then B merges A.
        let (mut a_fwd, mut b_fwd) = (a0.clone(), b0.clone());
        gossip_step_local(&mut a_fwd, &mut b_fwd);

        // Reverse: B merges A, then A merges B.
        let (mut a_rev, mut b_rev) = (a0.clone(), b0.clone());
        let a_snap = a_rev.clone();
        let b_snap = b_rev.clone();
        b_rev.process(a_snap, |_, _, _| {});
        a_rev.process(b_snap, |_, _, _| {});

        prop_assert_eq!(readout(&a_fwd), readout(&a_rev));
        prop_assert_eq!(readout(&b_fwd), readout(&b_rev));
    }

    /// B3: a second `gossip_step` immediately after the first is a
    /// no-op — fires zero `on_message` callbacks and doesn't change
    /// either peer's live content. (Same-party comparison here is
    /// safe: we're comparing each peer to itself before and after.)
    #[test]
    fn b3_idempotence(
        a_actions in arb_local_actions(),
        b_actions in arb_local_actions(),
    ) {
        let mut a = build_local("alice", &a_actions);
        let mut b = build_local("bob", &b_actions);
        gossip_step_local(&mut a, &mut b);

        let a_before = a.clone();
        let b_before = b.clone();

        let mut observed = 0usize;
        let a_snap = a.clone();
        let b_snap = b.clone();
        a.process(b_snap, |_, _, _| observed += 1);
        b.process(a_snap, |_, _, _| observed += 1);

        prop_assert_eq!(observed, 0, "no new observations on second gossip");
        prop_assert_eq!(&a, &a_before);
        prop_assert_eq!(&b, &b_before);
    }

    /// B4: bidirectional `Local::process` produces the same final
    /// `(a, b)` as driving the same two `Local`s through
    /// `Remote::gossip` over `tokio::io::duplex` — proving the wire
    /// protocol is faithful to the semantic merge.
    ///
    /// Smaller envelope: each case round-trips a tokio future, so we
    /// shrink schedule sizes here relative to the main suite.
    #[test]
    fn b4_process_matches_wire(
        a_actions in vec(
            prop_oneof![
                4 => any::<u64>().prop_map(LocalAction::Insert),
                1 => any::<usize>().prop_map(LocalAction::Redact),
            ],
            0..=8,
        ),
        b_actions in vec(
            prop_oneof![
                4 => any::<u64>().prop_map(LocalAction::Insert),
                1 => any::<usize>().prop_map(LocalAction::Redact),
            ],
            0..=8,
        ),
    ) {
        let a0 = build_local("alice", &a_actions);
        let b0 = build_local("bob", &b_actions);

        let mut a_proc = a0.clone();
        let mut b_proc = b0.clone();
        gossip_step_local(&mut a_proc, &mut b_proc);

        let (a_wire, b_wire) = wire_gossip(a0, b0);

        prop_assert_eq!(readout(&a_proc), readout(&a_wire));
        prop_assert_eq!(readout(&b_proc), readout(&b_wire));
        prop_assert_eq!(readout(&a_wire), readout(&b_wire));
    }

    /// B5a: `+` is commutative on live content — `readout(a + b) ==
    /// readout(b + a)`. We compare readouts (rather than `Local`s)
    /// because `Local::eq` includes the party tag and the two results
    /// have different parties.
    #[test]
    fn b5_add_commutative(
        a_actions in arb_local_actions(),
        b_actions in arb_local_actions(),
    ) {
        let a = build_local("alice", &a_actions);
        let b = build_local("bob", &b_actions);
        prop_assert_eq!(readout(&(a.clone() + b.clone())), readout(&(b + a)));
    }

    /// B5b: `+` is associative on live content — `readout((a + b) + c)
    /// == readout(a + (b + c))`.
    #[test]
    fn b5_add_associative(
        a_actions in arb_local_actions(),
        b_actions in arb_local_actions(),
        c_actions in arb_local_actions(),
    ) {
        let a = build_local("alice", &a_actions);
        let b = build_local("bob", &b_actions);
        let c = build_local("carol", &c_actions);
        prop_assert_eq!(
            readout(&((a.clone() + b.clone()) + c.clone())),
            readout(&(a + (b + c))),
        );
    }

    /// B5c: `+` is idempotent — `a + a == a` (same party on both
    /// sides, so direct `Local::eq` is meaningful here).
    #[test]
    fn b5_add_idempotent(a_actions in arb_local_actions()) {
        let a = build_local("alice", &a_actions);
        prop_assert_eq!(a.clone() + a.clone(), a);
    }
}
