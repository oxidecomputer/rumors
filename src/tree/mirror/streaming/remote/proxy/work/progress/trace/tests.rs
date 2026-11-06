use super::*;

/// A fully ordered trace satisfies both publication ledgers.
#[test]
fn accepts_complete_publication_order() {
    let (_, trace) = with_trace(|| {
        record(0, Kind::WireFrame { questions: 2 }, 0);
        record(0, Kind::LocalQuestion, 0);
        record(0, Kind::LocalQuestion, 0);
        record(0, Kind::TakenReply { scopes: 1 }, 0);
        record(0, Kind::NextScope, 0);
    });
    trace.assert_valid();
}

/// Independent height streams may interleave without violating either ledger.
#[test]
fn accepts_interleaved_heights() {
    let (_, trace) = with_trace(|| {
        record(0, Kind::WireFrame { questions: 1 }, 2);
        record(0, Kind::WireFrame { questions: 0 }, 1);
        record(0, Kind::LocalQuestion, 2);
        record(0, Kind::TakenReply { scopes: 1 }, 1);
        record(0, Kind::TakenReply { scopes: 0 }, 2);
        record(0, Kind::NextScope, 1);
    });
    trace.assert_valid();
}

/// A question cannot become internal state before the frame carrying it.
#[test]
#[should_panic(expected = "preceded its wire frame")]
fn rejects_question_before_wire_frame() {
    let (_, trace) = with_trace(|| record(0, Kind::LocalQuestion, 0));
    trace.assert_valid();
}

/// A dependent scope cannot precede the taken reply which creates it.
#[test]
#[should_panic(expected = "preceded its taken reply")]
fn rejects_scope_before_taken_reply() {
    let (_, trace) = with_trace(|| record(0, Kind::NextScope, 0));
    trace.assert_valid();
}

/// Consecutive wire replies cannot interleave their question batches.
#[test]
#[should_panic(expected = "overtook 1 prior questions")]
fn rejects_next_wire_frame_before_questions() {
    let (_, trace) = with_trace(|| {
        record(0, Kind::WireFrame { questions: 1 }, 0);
        record(0, Kind::WireFrame { questions: 0 }, 0);
    });
    trace.assert_valid();
}

/// Consecutive taken replies cannot interleave their dependent scopes.
#[test]
#[should_panic(expected = "overtook 1 prior scopes")]
fn rejects_next_taken_reply_before_scopes() {
    let (_, trace) = with_trace(|| {
        record(0, Kind::TakenReply { scopes: 1 }, 0);
        record(0, Kind::TakenReply { scopes: 0 }, 0);
    });
    trace.assert_valid();
}

/// An empty trace, which both ordering assertions accept, fails the
/// divergent-session floor by name.
#[test]
#[should_panic(expected = "exactly one greeting-seeded opening reply, found 0")]
fn empty_trace_fails_the_divergent_session_floor() {
    let (_, trace) = with_trace(|| ());
    trace.assert_valid();
    trace.assert_takes_answer_published_questions();
    trace.assert_covers_divergent_session();
}

/// The least a divergent session records meets the floor: the responder
/// proxy's opening wire frame, its question, and the answer its walk takes,
/// and the initiator proxy's greeting-seeded opening reply.
#[test]
fn minimal_divergent_session_meets_the_floor() {
    let (_, trace) = with_trace(|| {
        record(0, Kind::WireFrame { questions: 1 }, UnderRoot::HEIGHT);
        record(0, Kind::LocalQuestion, UnderRoot::HEIGHT);
        record(1, Kind::TakenReply { scopes: 1 }, Root::HEIGHT);
        record(1, Kind::NextScope, Root::HEIGHT);
        record(0, Kind::TakenReply { scopes: 0 }, UnderRoot::HEIGHT);
    });
    trace.assert_valid();
    trace.assert_takes_answer_published_questions();
    trace.assert_covers_divergent_session();
}

/// A taken answer following its published question answers it.
#[test]
fn accepts_take_after_published_question() {
    let (_, trace) = with_trace(|| {
        record(0, Kind::LocalQuestion, 1);
        record(0, Kind::TakenReply { scopes: 0 }, 1);
    });
    trace.assert_takes_answer_published_questions();
}

/// A taken answer with no question published under its label answers
/// nothing: the stage taking it and the encoder disagree about the
/// question's height.
#[test]
#[should_panic(expected = "has no published question to answer")]
fn rejects_take_before_published_question() {
    let (_, trace) = with_trace(|| record(0, Kind::TakenReply { scopes: 0 }, 1));
    trace.assert_takes_answer_published_questions();
}

/// Another endpoint's published question cannot pair with an answer this
/// endpoint takes: each endpoint answers only its own questions.
#[test]
#[should_panic(expected = "has no published question to answer")]
fn rejects_take_answering_the_other_endpoints_question() {
    let (_, trace) = with_trace(|| {
        record(0, Kind::LocalQuestion, 1);
        record(1, Kind::TakenReply { scopes: 0 }, 1);
    });
    trace.assert_takes_answer_published_questions();
}

/// Terminal answers may complete while an internal answer still publishes
/// leaf scopes: their questions belong to independent height-zero and
/// height-one queues.
#[test]
fn accepts_terminal_answer_during_internal_scope_publication() {
    let (_, trace) = with_trace(|| {
        record(0, Kind::WireFrame { questions: 1 }, 1);
        record(0, Kind::LocalQuestion, 1);
        record(0, Kind::TakenReply { scopes: 1 }, 1);
        record(0, Kind::WireFrame { questions: 1 }, 0);
        record(0, Kind::LocalQuestion, 0);
        record(0, Kind::TakenReply { scopes: 0 }, 0);
        record(0, Kind::NextScope, 1);
    });
    trace.assert_valid();
    trace.assert_takes_answer_published_questions();
}

/// Walk and terminal leaf publications use independent ledgers even when
/// their height-zero activity interleaves on one endpoint.
#[test]
fn accepts_overlapping_walk_and_terminal_leaf_publications() {
    let (_, trace) = with_trace(|| {
        record(0, Kind::TakenReply { scopes: 1 }, 0);
        record_stage(0, Stage::Terminal, Kind::TakenReply { scopes: 0 }, 0);
        record(0, Kind::NextScope, 0);
    });
    trace.assert_valid();
}

/// An unanswered parent question cannot stand in for a leaf question.
#[test]
#[should_panic(expected = "has no published question to answer")]
fn rejects_leaf_take_answering_a_parent_question() {
    let (_, trace) = with_trace(|| {
        record(0, Kind::LocalQuestion, 1);
        record(0, Kind::TakenReply { scopes: 0 }, 0);
    });
    trace.assert_takes_answer_published_questions();
}

/// An unanswered leaf question cannot stand in for a parent question.
#[test]
#[should_panic(expected = "has no published question to answer")]
fn rejects_parent_take_answering_a_leaf_question() {
    let (_, trace) = with_trace(|| {
        record(0, Kind::LocalQuestion, 0);
        record(0, Kind::TakenReply { scopes: 0 }, 1);
    });
    trace.assert_takes_answer_published_questions();
}

/// Exactly one greeting-seeded opening reply per endpoint answers the
/// greeting itself; a second answer taken at the root has no question to
/// answer.
#[test]
#[should_panic(expected = "has no published question to answer")]
fn rejects_a_second_greeting_seeded_opening() {
    let (_, trace) = with_trace(|| {
        record(0, Kind::TakenReply { scopes: 1 }, Root::HEIGHT);
        record(0, Kind::TakenReply { scopes: 1 }, Root::HEIGHT);
    });
    trace.assert_takes_answer_published_questions();
}

/// A question published and never taken leaves the session unbalanced:
/// some endpoint asked it and never consumed an answer.
#[test]
#[should_panic(expected = "published 2 questions at height 1 but took 1 answers")]
fn rejects_a_published_question_never_taken() {
    let (_, trace) = with_trace(|| {
        record(0, Kind::LocalQuestion, 1);
        record(0, Kind::LocalQuestion, 1);
        record(0, Kind::TakenReply { scopes: 0 }, 1);
    });
    trace.assert_takes_answer_published_questions();
}
