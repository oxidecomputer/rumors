use std::convert::Infallible;
use std::pin::pin;

use futures::{StreamExt, join};
use proptest::prelude::*;
use tokio::io::{DuplexStream, ReadHalf, WriteHalf};

use crate::tree::arb::arb_tree_root;
use crate::tree::mirror::streaming::Local;
use crate::tree::typed::{Node, Prefix, height::UnderRoot};

use super::*;

/// One wired-together level for a test: an in-memory stream's two ends,
/// dressed as the level's halves.
fn wired<I: BorshDeserialize>() -> (
    Outgoing<WriteHalf<DuplexStream>>,
    Incoming<ReadHalf<DuplexStream>, I>,
) {
    // Roomy enough that a test may write a whole level before reading it.
    let (ours, theirs) = tokio::io::duplex(1 << 20);
    let (_read_back, writer) = tokio::io::split(ours);
    let (reader, _write_back) = tokio::io::split(theirs);
    (Outgoing::new(writer), Incoming::new(reader))
}

/// Project a decoded-stream failure onto the wire violation it must be.
fn violation(fault: Fault<Infallible>) -> Violation {
    match fault {
        crate::tree::mirror::Error::Client(Error::Violation(violation)) => violation,
        crate::tree::mirror::Error::Client(other) => {
            panic!("expected a violation, got {other:?}")
        }
        crate::tree::mirror::Error::Server(never) => match never {},
    }
}

/// A same-variant, same-content comparison for decoded exchange messages
/// (the message type itself carries nodes and derives no `PartialEq`).
fn same(
    ours: &message::Exchanged<Local, (), UnderRoot>,
    theirs: &message::Exchanged<Local, (), UnderRoot>,
) -> bool {
    use message::Exchange::{Providing, Requested, Uncertain};
    ours.0 == theirs.0
        && match (&ours.1, &theirs.1) {
            (Requested, Requested) => true,
            (Uncertain(ours), Uncertain(theirs)) => ours == theirs,
            (Providing(ours), Providing(theirs)) => ours == theirs,
            _ => false,
        }
}

proptest! {
    /// Every item survives the borsh wire byte-for-byte.
    #[test]
    fn items_round_trip_borsh(root in arb_tree_root(0, 1..=16)) {
        let node = root.root.expect("at least one leaf means a root exists");
        let mut items: Vec<Item<(), UnderRoot>> = Vec::new();
        for (radix, child) in node.into_children() {
            let prefix = Prefix::new().push(radix);
            items.push(Item::Requested(prefix));
            items.push(Item::Uncertain(prefix, vec![(radix, child.hash())]));
            items.push(Item::End);
            for (version, message) in
                pollster::block_on(run_leaves(prefix, child))
            {
                items.push(Item::Leaf(version, message));
            }
        }
        for item in items {
            let bytes = borsh::to_vec(&item).expect("items serialize");
            let back: Item<(), UnderRoot> =
                borsh::from_slice(&bytes).expect("items deserialize");
            prop_assert_eq!(back, item);
        }
    }

    /// A message stream forwarded onto a level's stream and decoded back
    /// off of it is the same message stream.
    ///
    /// Headers ride whole, provided subtrees ride as leaf runs and
    /// reassemble at their derived prefixes, and the fin comes back as
    /// clean phase completion.
    #[test]
    fn adapters_round_trip(root in arb_tree_root(0, 1..=48)) {
        let node = root.root.expect("at least one leaf means a root exists");
        let children: Vec<_> = node.into_children().into_iter().collect();
        // The message enum deliberately derives no `Clone` (nodes clone
        // through the materiality trait), so build the sent and expected
        // streams independently from the same children.
        let messages = |children: &[(u8, Node<(), UnderRoot>)]| {
            children
                .iter()
                .enumerate()
                .map(|(index, (radix, child))| {
                    let prefix = Prefix::new().push(*radix);
                    let exchange = match index % 3 {
                        0 => message::Exchange::Requested,
                        1 => message::Exchange::Uncertain(
                            child
                                .clone()
                                .into_children()
                                .into_iter()
                                .map(|(radix, grandchild)| (radix, grandchild.hash()))
                                .collect(),
                        ),
                        _ => message::Exchange::Providing(child.clone()),
                    };
                    (prefix, exchange)
                })
                .collect::<Vec<message::Exchanged<Local, (), UnderRoot>>>()
        };
        let expected = messages(&children);

        let (outgoing, incoming) = wired();
        let forward = forward_exchanges::<Local, (), UnderRoot, _>(
            Local,
            futures::stream::iter(messages(&children)),
            outgoing,
        );
        let decode = exchanges::<Local, (), UnderRoot, _>(Local, incoming).collect::<Vec<_>>();
        let (forwarded, decoded) = pollster::block_on(async { join!(forward, decode) });
        forwarded.expect("forwarding onto an open stream succeeds");

        prop_assert_eq!(decoded.len(), expected.len());
        for (theirs, ours) in decoded.into_iter().zip(&expected) {
            let theirs = theirs.expect("honest levels decode");
            prop_assert!(same(&theirs, ours), "{:?} != {:?}", theirs.0, ours.0);
        }
    }
}

/// Explode one subtree into its leaf run (see [`codec::encode`]).
async fn run_leaves(
    prefix: Prefix<UnderRoot>,
    node: Node<(), UnderRoot>,
) -> Vec<(crate::Version, crate::message::Message<()>)> {
    codec::encode::<Local, (), UnderRoot>(Local, prefix, node)
        .map(|leaf| leaf.unwrap_or_else(|error| match error {}))
        .collect()
        .await
}

/// Decode a hand-scripted item sequence (finned behind the last item) and
/// expect a violation.
fn expect_violation<H, M, F, S>(items: Vec<Item<(), H>>, decode: F) -> Violation
where
    H: Height,
    F: FnOnce(Incoming<ReadHalf<DuplexStream>, Item<(), H>>) -> S,
    S: futures::Stream<Item = Result<M, Fault<Infallible>>>,
{
    let (mut outgoing, incoming) = wired();
    pollster::block_on(async {
        for item in items {
            outgoing.send(&item).await.expect("test stream stays open");
        }
        outgoing.finish().await.expect("test stream fins");
        let mut stream = pin!(decode(incoming));
        loop {
            match stream.next().await {
                Some(Ok(_)) => continue,
                Some(Err(fault)) => return violation(fault),
                None => panic!("expected a violation, stream ended cleanly"),
            }
        }
    })
}

/// Item kinds a level's message vocabulary cannot express are violations:
/// a bare `End` on an exchange level, `uncertain` on the closing level,
/// anything but a leaf run on the complete level.
#[test]
fn rejects_foreign_items() {
    use crate::tree::typed::height::{S, Z};

    assert_eq!(
        expect_violation(vec![Item::<(), UnderRoot>::End], |incoming| {
            exchanges::<Local, (), UnderRoot, _>(Local, incoming)
        }),
        Violation::UnexpectedItem,
    );

    let closing_prefix: Prefix<S<Z>> = Prefix::containing(&crate::tree::typed::Path::from(
        Prefix::from(crate::tree::Key::from([0u8; 32])),
    ));
    assert_eq!(
        expect_violation(
            vec![Item::<(), S<Z>>::Uncertain(
                closing_prefix,
                vec![(0, Hash::default())]
            )],
            |incoming| closing::<Local, (), _>(Local, incoming),
        ),
        Violation::UnexpectedItem,
    );

    let complete_prefix: Prefix<Z> = Prefix::from(crate::tree::Key::from([0u8; 32]));
    assert_eq!(
        expect_violation(
            vec![Item::<(), Z>::Requested(complete_prefix)],
            |incoming| { complete::<Local, (), _>(Local, incoming) }
        ),
        Violation::UnexpectedItem,
    );
}

/// Messages whose prefixes fail to strictly ascend within a level are
/// rejected: the walks' merge-join contract.
#[test]
fn rejects_disordered_messages() {
    let later = Prefix::new().push(1);
    let earlier = Prefix::new().push(0);
    assert_eq!(
        expect_violation(
            vec![
                Item::<(), UnderRoot>::Requested(later),
                Item::<(), UnderRoot>::Requested(earlier),
            ],
            |incoming| exchanges::<Local, (), UnderRoot, _>(Local, incoming),
        ),
        Violation::MessageOrder,
    );
}

/// A level stream that dies mid-item is a truncated session, not a clean
/// fin: partial bytes at end-of-stream are a violation.
#[test]
fn rejects_mid_item_truncation() {
    let bytes = borsh::to_vec(&Item::<(), UnderRoot>::Requested(Prefix::new().push(3)))
        .expect("items serialize");
    // Raw truncated bytes as the whole stream: EOF lands mid-item.
    let mut incoming: Incoming<&[u8], Item<(), UnderRoot>> =
        Incoming::new(&bytes[..bytes.len() - 1]);
    pollster::block_on(async {
        match incoming.next::<Infallible>().await {
            Err(Error::Violation(Violation::Truncated)) => {}
            other => panic!("expected truncation, got {other:?}"),
        }
    });
}
