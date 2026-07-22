use super::*;

/// The default budget equals the documented derivation: 256 full-fan query
/// frames of 4 354 bytes each, the decode side's largest non-supply reply.
#[test]
fn default_budget_matches_its_derivation() {
    assert_eq!(DEFAULT_TARGET_MESSAGE_SIZE, 1_114_624);
    assert_eq!(
        RunBudget::default(),
        RunBudget::from_bytes(DEFAULT_TARGET_MESSAGE_SIZE)
    );
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

/// The default budget stays within the `u32` framing header, so a
/// default-sized run is always representable on the wire.
#[test]
fn default_budget_fits_the_framing_header() {
    assert!(u32::try_from(DEFAULT_TARGET_MESSAGE_SIZE).is_ok());
}
