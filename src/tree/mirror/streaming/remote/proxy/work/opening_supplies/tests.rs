//! Properties of opening-supply radix pairing.

use std::{collections::BTreeSet, convert::Infallible};

use futures::stream;
use proptest::prelude::*;

use super::super::Ingress;
use super::{OpeningSupplies, State};
use crate::{
    Version,
    message::{Message, PayloadCodec, PayloadDepthLimit},
    tree::{
        arb::nth_party,
        mirror::streaming::{
            Backend, Local,
            remote::{adapter::DecodeError, proxy::Error},
        },
        typed::{self, Path, Prefix, height::Root as RootHeight},
    },
};

/// The in-memory backend's height-erased node.
type Erased = <Local as Backend>::Erased;

/// Make one payload-independent node for the pairing stream.
fn node() -> Erased {
    let mut version = Version::new();
    version.tick(&nth_party(0));
    <Local as Backend>::erase(typed::Node::leaf(version, Message::new(())))
}

/// Start a cursor on already-decoded groups, bypassing only wire decoding.
fn cursor(supplied: &BTreeSet<u8>) -> OpeningSupplies<Local> {
    let node = node();
    let groups: Vec<Result<_, DecodeError<Infallible>>> = supplied
        .iter()
        .map(|radix| Ok((*radix, node.clone())))
        .collect();
    OpeningSupplies {
        ingress: Ingress::new(
            Local,
            0,
            u64::MAX,
            PayloadCodec::new::<()>(PayloadDepthLimit::default()),
        ),
        state: State::Streaming {
            supplies: Box::pin(stream::iter(groups)),
            lookahead: None,
        },
    }
}

/// Return an arbitrary root prefix; the predecoded test stream never consults it.
fn root() -> typed::ErasedPrefix {
    Prefix::<RootHeight>::containing(&Path::from([0; 32])).erase()
}

/// Merge the two radix sets without using the production cursor.
fn expected(requested: &BTreeSet<u8>, supplied: &BTreeSet<u8>) -> (Vec<(u8, bool)>, bool) {
    let mut supplied = supplied.iter().copied().peekable();
    let mut answers = Vec::new();
    for radix in requested {
        match supplied.peek() {
            Some(next) if next < radix => return (answers, true),
            Some(next) if next == radix => {
                answers.push((*radix, true));
                supplied.next();
            }
            _ => answers.push((*radix, false)),
        }
    }
    (answers, supplied.next().is_some())
}

proptest! {
    /// Ordered pairing answers exactly the requested-and-supplied radices and
    /// rejects exactly when a supplied radix has no matching request.
    #[test]
    fn opening_supplies_pair_by_radix(
        requested in prop::collection::btree_set(any::<u8>(), 0..=32),
        supplied in prop::collection::btree_set(any::<u8>(), 0..=32),
    ) {
        let mut cursor = cursor(&supplied);
        let (answers, result) = pollster::block_on(async {
            let mut answers = Vec::new();
            for radix in &requested {
                match cursor.advance_to(root(), *radix).await {
                    Ok(node) => answers.push((*radix, node.is_some())),
                    Err(error) => return (answers, Err(error)),
                }
            }
            let result = cursor.finish().await;
            (answers, result)
        });

        let (expected_answers, expected_error) = expected(&requested, &supplied);
        prop_assert_eq!(answers, expected_answers);
        match result {
            Err(Error::UnaskedReply) => prop_assert!(expected_error),
            Err(error) => prop_assert!(false, "unexpected cursor error: {error:?}"),
            Ok(()) => prop_assert!(!expected_error),
        }
    }
}
