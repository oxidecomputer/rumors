//! The distinguished opening question and its write-before-publish contract.

use before::Version;

use crate::message::Message;
use crate::tree::{
    mirror::streaming::{
        Local,
        message::{Reaction, Reply},
    },
    typed::{
        self,
        height::{Height, S, UnderRoot, Z},
    },
};

use super::{
    super::{OpeningError, Scope, decode_opening, encode_opening},
    hash, runtime,
};
use crate::tree::mirror::streaming::remote::codec::{End, Flow, Frame, Reaction as WireReaction};

trait OpeningNode: Height {
    fn node() -> typed::Node<(), Self>;
}

impl OpeningNode for Z {
    fn node() -> typed::Node<(), Self> {
        typed::Node::leaf(Version::new(), Message::new(()))
    }
}

impl<H: OpeningNode> OpeningNode for S<H>
where
    S<H>: Height,
{
    fn node() -> typed::Node<(), Self> {
        typed::Node::beneath(H::node(), 0)
    }
}

/// The exceptional opening query round-trips while deriving its implicit root scope.
#[test]
fn opening_round_trips_with_its_root_scope() {
    let listing = vec![(3, hash(1)), (9, hash(2))];
    let reply = Reply::<Local, (), UnderRoot> {
        replies: vec![Reaction::Query(listing.clone())],
    };

    let encoded = encode_opening(reply).expect("canonical opening");
    let (frame, question) = encoded.into_parts();
    let scope = question.expect("opening publishes its question");
    assert_eq!(scope, Scope::opening(&listing));

    let (decoded, decoded_scope) = decode_opening::<Local, ()>(frame).expect("opening decodes");
    assert_eq!(decoded_scope, scope);
    let [Reaction::Query(decoded)] = decoded.replies.as_slice() else {
        panic!("opening must remain one query")
    };
    assert_eq!(decoded, &listing);
}

/// A derived question becomes visible only after its frame is successfully written.
#[test]
fn a_question_is_released_only_after_its_writer_succeeds() {
    let listing = vec![(3, hash(1))];
    let reply = || Reply::<Local, (), UnderRoot> {
        replies: vec![Reaction::Query(listing.clone())],
    };

    let failed = runtime().block_on(
        encode_opening(reply())
            .expect("canonical opening")
            .write_with(|_frame| async { Err::<(), _>("write failed") }),
    );
    assert_eq!(failed, Err("write failed"));

    let question = runtime()
        .block_on(
            encode_opening(reply())
                .expect("canonical opening")
                .write_with(|frame| async move {
                    assert!(matches!(
                        frame,
                        Frame::Reaction(WireReaction::Query(_), Flow::End)
                    ));
                    Ok::<_, &str>(())
                }),
        )
        .expect("successful write releases the question")
        .expect("opening asks one question");
    assert_eq!(question, Scope::opening(&listing));
}

/// Every semantic opening shape is either the one valid query or its exact typed rejection.
#[test]
fn opening_rejections_are_exhaustive() {
    for count in 0..=3 {
        let reply = Reply::<Local, (), UnderRoot> {
            replies: (0..count).map(|_| Reaction::Match).collect(),
        };
        let expected = if count == 1 {
            OpeningError::NotQuery
        } else {
            OpeningError::ReactionCount { count }
        };
        assert_eq!(encode_opening(reply).err(), Some(expected));
    }
    let supplied = Reply::<Local, (), UnderRoot> {
        replies: vec![Reaction::Supply(0, UnderRoot::node())],
    };
    assert_eq!(encode_opening(supplied).err(), Some(OpeningError::NotQuery));

    let flows = [Flow::Continue, Flow::End];
    let bodies = [
        WireReaction::Match,
        WireReaction::Query(Vec::new()),
        WireReaction::Query(vec![(1, hash(1))]),
        WireReaction::Supply(Version::new(), Message::new(())),
    ];
    let mut checked = 0;
    for body in bodies {
        for flow in flows {
            let valid = matches!(body, WireReaction::Query(_)) && flow == Flow::End;
            let decoded = decode_opening::<Local, ()>(Frame::Reaction(body.clone(), flow));
            if valid {
                decoded.expect("a reply-ending query is the canonical opening");
            } else {
                assert_eq!(decoded.err(), Some(OpeningError::InvalidFrame));
            }
            checked += 1;
        }
    }
    for end in [End::Reply, End::Stream] {
        assert_eq!(
            decode_opening::<Local, ()>(Frame::End(end)).err(),
            Some(OpeningError::InvalidFrame)
        );
        checked += 1;
    }
    assert_eq!(checked, 10);
}
