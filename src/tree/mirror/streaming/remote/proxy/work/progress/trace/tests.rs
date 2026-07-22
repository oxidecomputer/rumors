use super::*;

/// A fully ordered trace satisfies both publication ledgers.
#[test]
fn accepts_complete_publication_order() {
    let (_, trace) = with_trace(|| {
        record(0, Kind::WireReply { questions: 2 }, 0);
        record(0, Kind::LocalQuestion, 0);
        record(0, Kind::LocalQuestion, 0);
        record(0, Kind::DecodedReply { scopes: 1 }, 0);
        record(0, Kind::NextScope, 0);
    });
    trace.assert_valid();
}

/// Independent height streams may interleave without violating either ledger.
#[test]
fn accepts_interleaved_heights() {
    let (_, trace) = with_trace(|| {
        record(0, Kind::WireReply { questions: 1 }, 2);
        record(0, Kind::WireReply { questions: 0 }, 1);
        record(0, Kind::LocalQuestion, 2);
        record(0, Kind::DecodedReply { scopes: 1 }, 1);
        record(0, Kind::DecodedReply { scopes: 0 }, 2);
        record(0, Kind::NextScope, 1);
    });
    trace.assert_valid();
}

/// A question cannot become internal state before its complete wire reply.
#[test]
#[should_panic(expected = "preceded its wire reply")]
fn rejects_question_before_wire_reply() {
    let (_, trace) = with_trace(|| record(0, Kind::LocalQuestion, 0));
    trace.assert_valid();
}

/// A dependent scope cannot precede the decoded reply which creates it.
#[test]
#[should_panic(expected = "preceded its decoded reply")]
fn rejects_scope_before_decoded_reply() {
    let (_, trace) = with_trace(|| record(0, Kind::NextScope, 0));
    trace.assert_valid();
}

/// Consecutive wire replies cannot interleave their question batches.
#[test]
#[should_panic(expected = "overtook 1 prior questions")]
fn rejects_next_wire_reply_before_questions() {
    let (_, trace) = with_trace(|| {
        record(0, Kind::WireReply { questions: 1 }, 0);
        record(0, Kind::WireReply { questions: 0 }, 0);
    });
    trace.assert_valid();
}

/// Consecutive decoded replies cannot interleave their dependent scopes.
#[test]
#[should_panic(expected = "overtook 1 prior scopes")]
fn rejects_next_decoded_reply_before_scopes() {
    let (_, trace) = with_trace(|| {
        record(0, Kind::DecodedReply { scopes: 1 }, 0);
        record(0, Kind::DecodedReply { scopes: 0 }, 0);
    });
    trace.assert_valid();
}

/// A decode following its flushed question satisfies registration causality.
#[test]
fn accepts_decode_after_flushed_question() {
    let (_, trace) = with_trace(|| {
        record(0, Kind::LocalQuestion, 2);
        record(0, Kind::DecodedReply { scopes: 0 }, 1);
    });
    trace.assert_registration_causality();
}

/// A decode with no flushed question at the scope's height is a causality
/// violation: the reply arrived before the question that scopes it.
#[test]
#[should_panic(expected = "arrived before the question that scopes it")]
fn rejects_decode_before_flushed_question() {
    let (_, trace) = with_trace(|| record(0, Kind::DecodedReply { scopes: 0 }, 1));
    trace.assert_registration_causality();
}

/// Another endpoint's flushed question cannot scope this endpoint's decode:
/// the registration FIFO is endpoint-local.
#[test]
#[should_panic(expected = "arrived before the question that scopes it")]
fn rejects_decode_scoped_by_the_other_endpoint() {
    let (_, trace) = with_trace(|| {
        record(0, Kind::LocalQuestion, 2);
        record(1, Kind::DecodedReply { scopes: 0 }, 1);
    });
    trace.assert_registration_causality();
}

/// Leaf-height decodes drain both the last internal stage's height-1 scopes
/// and the terminal height-0 leaf questions, and no more than their sum.
#[test]
#[should_panic(expected = "arrived before the question that scopes it")]
fn rejects_leaf_decode_beyond_both_leaf_question_sources() {
    let (_, trace) = with_trace(|| {
        record(0, Kind::LocalQuestion, 1);
        record(0, Kind::LocalQuestion, 0);
        record(0, Kind::DecodedReply { scopes: 0 }, 0);
        record(0, Kind::DecodedReply { scopes: 0 }, 0);
        record(0, Kind::DecodedReply { scopes: 0 }, 0);
    });
    trace.assert_registration_causality();
}

/// Exactly one greeting-seeded opening reply per endpoint is scoped by the
/// greeting itself; a second under-root decode has no question to pair with.
#[test]
#[should_panic(expected = "arrived before the question that scopes it")]
fn rejects_a_second_greeting_seeded_opening() {
    let (_, trace) = with_trace(|| {
        record(0, Kind::DecodedReply { scopes: 1 }, UnderRoot::HEIGHT);
        record(0, Kind::DecodedReply { scopes: 1 }, UnderRoot::HEIGHT);
    });
    trace.assert_registration_causality();
}
