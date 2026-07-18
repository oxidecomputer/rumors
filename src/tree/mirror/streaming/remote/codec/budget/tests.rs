use super::*;

/// The default budget equals the documented derivation: 256 full-fan query
/// frames of 4 354 bytes each, the wire's existing largest message.
#[test]
fn default_budget_matches_its_derivation() {
    assert_eq!(DEFAULT_TARGET_MESSAGE_SIZE, 1_114_624);
    assert_eq!(RunBudget::default().bytes(), DEFAULT_TARGET_MESSAGE_SIZE);
}

/// The default budget stays within the `u32` framing header, so a
/// default-sized run is always representable on the wire.
#[test]
fn default_budget_fits_the_framing_header() {
    assert!(u32::try_from(DEFAULT_TARGET_MESSAGE_SIZE).is_ok());
}
