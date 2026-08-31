use super::*;

/// The default budget equals the documented derivation: 256 full-fan
/// query frames, the decode side's largest non-supply reply, pinned by
/// value so a change to any wire constant it derives from moves this
/// assert loudly.
#[test]
fn default_budget_matches_its_derivation() {
    assert_eq!(DEFAULT_TARGET_MESSAGE_SIZE, 1_830_400);
    assert_eq!(
        RunBudget::default(),
        RunBudget::from_bytes(DEFAULT_TARGET_MESSAGE_SIZE)
    );
}

/// The full-fan frame constant prices exactly what the encoder writes.
///
/// One query frame carrying every radix, constructed and encoded, is
/// byte-for-byte the closed form's value — so the default budget's
/// derivation cannot drift from the wire.
#[test]
fn full_fan_frame_len_matches_an_actual_encode() {
    use crate::tree::typed::{Hash, hash::MERKLE_HASH_LEN};

    use super::super::signal::{Flow, Speaker, Stream};
    use super::super::{Frame, Reaction, WireFrame, encode};

    let children: Vec<(u8, Hash)> = (0..=u8::MAX)
        .map(|radix| (radix, Hash([radix; MERKLE_HASH_LEN])))
        .collect();
    let frame: WireFrame = (
        Stream::new(8).expect("an interior stream exists"),
        Frame::Reaction(Reaction::Query(children), Flow::Continue),
    );
    let mut encoded = Vec::new();
    encode(Speaker::Initiator, &frame, &mut encoded).expect("a full-fan query encodes");
    assert_eq!(encoded.len(), FULL_FAN_QUERY_FRAME_LEN);
}

/// The budget's admission boundary charges the whole wire frame.
///
/// A record is admitted exactly while the frame — the signal-and-length
/// envelope plus run body plus the candidate record — stays within the
/// budget, so the flush accounting prices frames, not only run bodies.
#[test]
fn admission_charges_the_frame_envelope() {
    let (body, record) = (10, 3);
    let exact = RunBudget::from_bytes(SUPPLY_FRAME_OVERHEAD + body + record);
    assert!(exact.admits(body, record));
    assert!(!exact.admits(body + 1, record));
    assert!(!exact.admits(body, record + 1));

    // A budget covering only body and record undercounts the frame and
    // must not admit: the envelope is charged too.
    let body_only = RunBudget::from_bytes(body + record);
    assert!(!body_only.admits(body, record));
}

/// The default budget stays within the wire's run byte cap, so a
/// default-sized run is always representable on the wire.
#[test]
fn default_budget_fits_the_framing_header() {
    assert!(u32::try_from(DEFAULT_TARGET_MESSAGE_SIZE).is_ok());
}

/// Budgets above the wire's run byte cap saturate to it, so a run the
/// budget admits always stays within the cap instead of buffering past
/// 4 GiB and deterministically failing at the run head.
///
/// The boundary is checked at the admitted maximum: the largest
/// body-plus-record the saturated budget accepts still leaves the
/// flushed frame's body within the cap, envelope included. The
/// negative control shows the ceiling binds: one byte past the admitted
/// maximum is refused, so the saturated budget is a real bound, not a
/// pass-through.
#[test]
fn over_ceiling_budgets_saturate_to_the_framing_ceiling() {
    let budget = RunBudget::from_bytes(usize::MAX);
    assert_eq!(budget.bytes(), MAX_RUN_BUDGET_BYTES);

    // The admitted maximum: envelope + body + record == the saturated
    // budget exactly.
    let body = MAX_RUN_BUDGET_BYTES - SUPPLY_FRAME_OVERHEAD - 1;
    assert!(budget.admits(body, 1));
    assert!(
        super::super::frame::checked_run_len(body + 1).is_ok(),
        "an admitted flush must stay within the run byte cap",
    );
    // Negative control: the ceiling genuinely binds.
    assert!(!budget.admits(body + 1, 1));
}
