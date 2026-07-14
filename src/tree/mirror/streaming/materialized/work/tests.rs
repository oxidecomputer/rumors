//! The contract of [`assemble`], the session's reassembly primitive.
//!
//! It must fill `Pending` slots from the level stream strictly in order,
//! honor `Ready(None)` as deletion, resolve the empty resolution to `None`
//! through [`Backend::parent`], chain across two instances, and pass
//! errors through unchanged. Counterparty-fault classification lives in
//! [`violations`].

use std::convert::Infallible;

use futures::stream::{self, StreamExt, TryStreamExt};

use super::assemble;

mod violations;

use crate::{
    Version,
    message::Message,
    tree::{
        arb::nth_party,
        mirror::streaming::{
            Backend, Local,
            materialized::{Error, Resolution, Resolve, Violation},
        },
        typed::{
            self, Path, Prefix,
            height::{S, Z},
        },
    },
};

/// A distinct leaf per call: the versions differ so hashes do.
fn leaf(version: &mut Version) -> typed::Node<(), Z> {
    version.tick(&nth_party(0));
    typed::Node::leaf(version.clone(), Message::new(()))
}

/// The height-`Z` key with `parent` as its first byte and `radix` as its
/// last, zeros between.
fn key(parent: u8, radix: u8) -> Prefix<Z> {
    let mut bytes = [0u8; 32];
    bytes[0] = parent;
    bytes[31] = radix;
    Path::from(bytes).into()
}

/// The parent-height prefix of [`key`]`(parent, _)`.
fn parent_prefix(parent: u8) -> Prefix<S<Z>> {
    key(parent, 0).pop().0
}

/// Build the expected parent of a radix group directly through the backend.
fn parent_of(
    prefix: Prefix<S<Z>>,
    children: Vec<(u8, Option<typed::Node<(), Z>>)>,
) -> Option<typed::Node<(), S<Z>>> {
    pollster::block_on(Local.parent(prefix, children)).unwrap_or_else(|e| match e {})
}

type Item = Result<Resolution<Local, (), Z>, Error<Infallible>>;
type Level = Result<Option<typed::Node<(), Z>>, Error<Infallible>>;

/// Drive [`assemble`] over in-memory streams and collect its outputs.
fn assembled(
    resolutions: Vec<Item>,
    level: Vec<Level>,
) -> Vec<Result<Option<typed::Node<(), S<Z>>>, Error<Infallible>>> {
    pollster::block_on(
        assemble(Local, stream::iter(resolutions), stream::iter(level)).collect::<Vec<_>>(),
    )
}

/// `Pending` slots are filled from the level stream strictly in order,
/// interleaved with `Ready` slots at their stated radices.
#[test]
fn fills_pendings_in_order() {
    let mut version = Version::new();
    let (a, b, c) = (leaf(&mut version), leaf(&mut version), leaf(&mut version));

    let resolution = Resolution {
        prefix: parent_prefix(3),
        resolved: vec![
            (1, Resolve::Pending),
            (2, Resolve::Ready(Some(b.clone()))),
            (5, Resolve::Pending),
        ],
    };
    let output = assembled(
        vec![Ok(resolution)],
        vec![Ok(Some(a.clone())), Ok(Some(c.clone()))],
    );

    let expected = parent_of(
        parent_prefix(3),
        vec![(1, Some(a)), (2, Some(b)), (5, Some(c))],
    );
    assert_eq!(
        output
            .into_iter()
            .map(|item| item.expect("no errors were fed in").map(|node| node.hash()))
            .collect::<Vec<_>>(),
        vec![expected.map(|node| node.hash())],
    );
}

/// A `Ready(None)` slot is a deletion: it reaches `Backend::parent` as a
/// `None` entry, and the parent assembles from the survivors alone.
#[test]
fn ready_none_is_deletion() {
    let mut version = Version::new();
    let a = leaf(&mut version);

    let resolution = Resolution {
        prefix: parent_prefix(3),
        resolved: vec![
            (1, Resolve::Ready(Some(a.clone()))),
            (2, Resolve::Ready(None)),
        ],
    };
    let output = assembled(vec![Ok(resolution)], vec![]);

    let expected = parent_of(parent_prefix(3), vec![(1, Some(a))]);
    assert_eq!(
        output
            .into_iter()
            .map(|item| item.expect("no errors were fed in").map(|node| node.hash()))
            .collect::<Vec<_>>(),
        vec![expected.map(|node| node.hash())],
    );
}

/// The empty resolution — the pruned-to-nothing reply to a request — reaches
/// `Backend::parent` with an empty group and resolves to `None`.
#[test]
fn empty_resolution_assembles_to_none() {
    let resolution = Resolution {
        prefix: parent_prefix(3),
        resolved: vec![],
    };
    let output = assembled(vec![Ok(resolution)], vec![]);
    assert!(matches!(output.as_slice(), [Ok(None)]));
}

/// An all-deleted resolution cascades: every slot `Ready(None)` assembles to
/// a `None` return, deleting the scope one level up.
#[test]
fn all_deleted_resolution_assembles_to_none() {
    let resolution = Resolution {
        prefix: parent_prefix(3),
        resolved: vec![(1, Resolve::Ready(None)), (2, Resolve::Ready(None))],
    };
    let output = assembled(vec![Ok(resolution)], vec![]);
    assert!(matches!(output.as_slice(), [Ok(None)]));
}

/// Two chained instances reproduce a stage's shape: the lower assembler's
/// outputs fill the upper assembler's `Pending`s, in order.
#[test]
fn chains_two_instances() {
    let mut version = Version::new();
    let (a, b) = (leaf(&mut version), leaf(&mut version));

    let lower: Vec<Result<Resolution<Local, (), Z>, Error<Infallible>>> = vec![Ok(Resolution {
        prefix: parent_prefix(3),
        resolved: vec![
            (1, Resolve::Ready(Some(a.clone()))),
            (7, Resolve::Ready(Some(b.clone()))),
        ],
    })];
    let upper: Vec<Result<Resolution<Local, (), S<Z>>, Error<Infallible>>> = vec![Ok(Resolution {
        prefix: parent_prefix(3).pop().0,
        resolved: vec![(3, Resolve::Pending)],
    })];

    let chained = assemble(
        Local,
        stream::iter(upper),
        assemble(Local, stream::iter(lower), stream::empty()),
    );
    let output =
        pollster::block_on(chained.try_collect::<Vec<_>>()).expect("no errors were fed in");

    let inner = parent_of(parent_prefix(3), vec![(1, Some(a)), (7, Some(b))])
        .expect("a non-empty all-real group constructs its parent");
    let expected =
        pollster::block_on(Local.parent(parent_prefix(3).pop().0, vec![(3, Some(inner))]))
            .unwrap_or_else(|e| match e {});
    assert_eq!(
        output
            .into_iter()
            .map(|node| node.map(|node| node.hash()))
            .collect::<Vec<_>>(),
        vec![expected.map(|node| node.hash())],
    );
}

/// An error arriving on the resolutions stream ends the output with that
/// error: nothing downstream sees a partial scope.
#[test]
fn resolution_error_passes_through() {
    let output = assembled(vec![Err(Error::Violation(Violation::UnaskedReply))], vec![]);
    assert!(matches!(
        output.as_slice(),
        [Err(Error::Violation(Violation::UnaskedReply))],
    ));
}

/// An error arriving on the level stream surfaces through the `Pending`
/// slot that pulled it.
#[test]
fn level_error_passes_through() {
    let resolution = Resolution {
        prefix: parent_prefix(3),
        resolved: vec![(1, Resolve::Pending)],
    };
    let output = assembled(
        vec![Ok(resolution)],
        vec![Err(Error::Violation(Violation::UnansweredQuery))],
    );
    assert!(matches!(
        output.as_slice(),
        [Err(Error::Violation(Violation::UnansweredQuery))],
    ));
}
