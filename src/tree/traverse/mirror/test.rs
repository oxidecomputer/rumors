use std::convert::Infallible;

use proptest::collection::vec;
use proptest::prelude::*;

use crate::tree::arb::arb_root_tree;
use crate::tree::traverse::{Action, act};
use crate::tree::typed::{Node, Path, height::Root};
use crate::{Key, Message, Version};

use super::*;

fn mirror_direct<P, T>(
    a: Option<Node<P, T, Root>>,
    b: Option<Node<P, T, Root>>,
    a_to_b: impl FnMut(&Version<P>, Key, &Message<T>),
    b_to_a: impl FnMut(&Version<P>, Key, &Message<T>),
) -> Option<Node<P, T, Root>>
where
    P: Clone + Ord + AsRef<[u8]>,
    T: Clone,
{
    match (|| -> Result<Infallible, Option<Node<P, T, Root>>> {
        let version_a = a.as_ref().map(|n| n.version().clone()).unwrap_or_default();
        let version_b = b.as_ref().map(|n| n.version().clone()).unwrap_or_default();

        let (m, a) = exchange::initiator(a, &version_b, b_to_a);
        let (m, b) = exchange::responder(b, &version_a, a_to_b, m);

        // The initiator's first round is `open` (consuming the responder's
        // `Opening`); the next 14 rounds are alternating `exchange`s; the
        // initiator's last round is `close_initiator` (emitting `Closing`);
        // the responder closes with `complete_responder`.
        let (m, a) = a.open_initiator(m);

        seq_macro::seq!(_ in 0..14 {
            let (m, b) = b?.exchange(m);
            let (m, a) = a?.exchange(m);
        });

        let (m, b) = b?.exchange(m);
        let (m, a) = a?.close_initiator(m);

        let (m, b) = b?.complete_responder(m);
        let node_a = a?.complete_initiator(m);

        b?;
        node_a
    })() {
        Err(node_a) => node_a,
    }
}

fn null<P: Ord, T>(v: &Version<P>, k: Key, m: &Message<T>) {}

proptest! {

    /// Mirroring a node with itself is a no-op: the two replicas have
    /// identical content and versions, so the reconciled tree is unchanged.
    #[test]
    fn idempotent(a in arb_root_tree("a", 0..=8)) {
        prop_assert_eq!(mirror_direct(a.clone(), a.clone(), null, null), a);
    }

    /// The reconciled tree is the same regardless of which replica
    /// initiates and which responds.
    #[test]
    fn commutative(
        a in arb_root_tree("a", 0..=8),
        b in arb_root_tree("b", 0..=8),
    ) {
        prop_assert_eq!(
            mirror_direct(a.clone(), b.clone(), null, null),
            mirror_direct(b, a, null, null),
        );
    }

    /// Re-mirroring the result with a peer already synced with is a no-op:
    /// the result already contains everything the peer had.
    #[test]
    fn absorptive(
        a in arb_root_tree("a", 0..=8),
        b in arb_root_tree("b", 0..=8),
    ) {
        let ab = mirror_direct(a, b.clone(), null, null);
        prop_assert_eq!(mirror_direct(ab.clone(), b, null, null), ab);
    }

    /// Three-way mirror is order-independent: syncing (a,b) then c
    /// produces the same tree as syncing a then (b,c).
    #[test]
    fn associative(
        a in arb_root_tree("a", 0..=4),
        b in arb_root_tree("b", 0..=4),
        c in arb_root_tree("c", 0..=4),
    ) {
        let ab_c = mirror_direct(mirror_direct(a.clone(), b.clone(), null, null), c.clone(), null, null);
        let a_bc = mirror_direct(a, mirror_direct(b, c, null, null), null, null);
        prop_assert_eq!(ab_c, a_bc);
    }

    /// Mirror is equivalent to replaying both sides' full action
    /// histories — inserts and forgets — through a single `act` call.
    #[test]
    fn equivalent_to_cross_react(
        entries_a in vec((any::<[u8; 32]>(), any::<bool>()), 0..=8),
        entries_b in vec((any::<[u8; 32]>(), any::<bool>()), 0..=8),
    ) {
        let make_actions = |party: &str, entries: &[([u8; 32], bool)]| -> Vec<_> {
            let n = entries.len();
            let mut actions: Vec<_> = entries.iter().enumerate().map(|(i, (bytes, _))| {
                let path = Path::from(*bytes);
                let version = Version::from((party.to_string(), i as u64 + 1));
                (path, version, Action::Insert(Message::new(())))
            }).collect();
            for (j, (bytes, forget)) in entries.iter().enumerate() {
                if *forget {
                    let path = Path::from(*bytes);
                    let version = Version::from((party.to_string(), (n + j + 1) as u64));
                    actions.push((path, version, Action::Forget));
                }
            }
            actions
        };

        let actions_a = make_actions("a", &entries_a);
        let actions_b = make_actions("b", &entries_b);

        let node_a = act(None, actions_a.clone());
        let node_b = act(None, actions_b.clone());

        let mirrored = mirror_direct(node_a, node_b, null, null);

        let mut all_actions = actions_a;
        all_actions.extend(actions_b);
        let expected = act(None, all_actions);

        prop_assert_eq!(mirrored, expected);
    }
}
