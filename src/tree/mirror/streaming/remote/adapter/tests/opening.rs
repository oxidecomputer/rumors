//! The distinguished opening question and its write-before-publish contract.

use crate::tree::{
    mirror::streaming::{
        Local,
        message::{Reaction, Reply},
    },
    typed::height::UnderRoot,
};

use super::{
    super::{Scope, decode_opening, encode_opening},
    hash, runtime,
};
use crate::tree::mirror::streaming::remote::codec::{End, Flow, Frame, Reaction as WireReaction};

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
                        Frame::Reaction(WireReaction::Query(_), Flow::End(End::Stream),)
                    ));
                    Ok::<_, &str>(())
                }),
        )
        .expect("successful write releases the question")
        .expect("opening asks one question");
    assert_eq!(question, Scope::opening(&listing));
}
