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
