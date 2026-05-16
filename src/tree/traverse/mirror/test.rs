use std::convert::Infallible;

use proptest::collection::vec;
use proptest::prelude::*;

use crate::tree::arb::arb_root_tree;
use crate::tree::traverse::{Action, act};
use crate::tree::typed::height::Root;
use crate::tree::typed::{Node, Path};
use crate::{Key, Message, Version};

use super::protocol::*;
use super::*;

/// Drive the mirror protocol entirely through the abstract trait family in
/// [`super::protocol`]. Every step calls a trait method --- the inherent
/// methods on `local::Exchange` are private, so the only way this driver
/// could compile and converge is if the trait abstraction is faithful.
fn mirror_direct<P, T, ASend, ARecv, BSend, BRecv>(
    a: Option<Node<P, T, Root>>,
    b: Option<Node<P, T, Root>>,
    a_send: ASend,
    a_recv: ARecv,
    b_send: BSend,
    b_recv: BRecv,
) -> Option<Node<P, T, Root>>
where
    ASend: FnMut(&Version<P>, Key, &Message<T>),
    ARecv: FnMut(&Version<P>, Key, &Message<T>),
    BSend: FnMut(&Version<P>, Key, &Message<T>),
    BRecv: FnMut(&Version<P>, Key, &Message<T>),
    P: Clone + Ord + AsRef<[u8]>,
    T: Clone,
{
    match (|| -> Result<Infallible, Option<Node<P, T, Root>>> {
        // Before the exchange protocol, the parties exchange the version
        // vectors of their root nodes; this may be done simultaneously, whereas
        // the remainder of the protocol is sequential and alternating.
        let version_a = a.as_ref().map(|n| n.version().clone()).unwrap_or_default();
        let version_b = b.as_ref().map(|n| n.version().clone()).unwrap_or_default();

        // Initialize each side of the protocol -- these are inherent methods,
        // not part of the protocol specification, because the protocol does not
        // necessarily specify how a counterparty is initialized.
        let a = local::Exchange::new(a, &version_b, a_send, a_recv);
        let b = local::Exchange::new(b, &version_a, b_send, b_recv);

        // The initiator's first round constructs its state via
        // `Initiator::initiator` and emits the opening message; the responder
        // consumes it via `Responder::responder`. Each constructor's `Self`
        // type is deduced by the compiler from the argument types and the
        // single applicable `Initiator` / `Responder` impl on `local::Exchange`.
        let (m, a) = a.initiator();
        let (m, b) = b.responder(m);

        // From here every step is a trait method on the state value. Method
        // syntax resolves to the trait because the inherent methods on
        // `local::Exchange` are private and the traits are in scope above.
        let (m, a) = a.open_initiator(m);

        // The next 14 rounds are alternating `exchange`s.
        seq_macro::seq!(_ in 0..14 {
            let (m, b) = b?.exchange(m);
            let (m, a) = a?.exchange(m);
        });

        // The initiator's penultimate round is `close_initiator` (emitting
        // `Closing`).
        let (m, b) = b?.exchange(m);
        let (m, a) = a?.close_initiator(m);

        // The responder closes with `complete_responder`, which is locally
        // consumed by the initiator using `complete_initiator`.
        let (m, b) = b?.complete_responder(m);
        let node_a = a?.complete_initiator(m);

        b?;
        node_a
    })() {
        Err(node_a) => node_a,
    }
}

/// When we're not monitoring messages, this placeholder function eats them.
fn x<P: Ord, T>(_v: &Version<P>, _k: Key, _m: &Message<T>) {}

proptest! {

    /// Mirroring a node with itself is a no-op: the two replicas have
    /// identical content and versions, so the reconciled tree is unchanged.
    #[test]
    fn idempotent(a in arb_root_tree("a", 0..=8)) {
        prop_assert_eq!(mirror_direct(a.clone(), a.clone(), x, x, x, x), a);
    }

    /// The reconciled tree is the same regardless of which replica
    /// initiates and which responds.
    #[test]
    fn commutative(
        a in arb_root_tree("a", 0..=8),
        b in arb_root_tree("b", 0..=8),
    ) {
        prop_assert_eq!(
            mirror_direct(a.clone(), b.clone(), x, x, x, x),
            mirror_direct(b, a, x, x, x, x),
        );
    }

    /// Re-mirroring the result with a peer already synced with is a no-op:
    /// the result already contains everything the peer had.
    #[test]
    fn absorptive(
        a in arb_root_tree("a", 0..=8),
        b in arb_root_tree("b", 0..=8),
    ) {
        let ab = mirror_direct(a, b.clone(), x, x, x, x);
        prop_assert_eq!(mirror_direct(ab.clone(), b, x, x, x, x), ab);
    }

    /// Three-way mirror is order-independent: syncing (a,b) then c
    /// produces the same tree as syncing a then (b,c).
    #[test]
    fn associative(
        a in arb_root_tree("a", 0..=4),
        b in arb_root_tree("b", 0..=4),
        c in arb_root_tree("c", 0..=4),
    ) {
        let ab_c = mirror_direct(mirror_direct(a.clone(), b.clone(), x, x, x, x), c.clone(), x, x, x, x);
        let a_bc = mirror_direct(a, mirror_direct(b, c, x, x, x, x), x, x, x, x);
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

        let mirrored = mirror_direct(node_a, node_b, x, x, x, x);

        let mut all_actions = actions_a;
        all_actions.extend(actions_b);
        let expected = act(None, all_actions);

        prop_assert_eq!(mirrored, expected);
    }
}
