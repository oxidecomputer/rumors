//! Conforming storage and deliberate violations of the bookmark contract.

use std::cell::{Cell, RefCell};
use std::future::{pending, ready};
use std::io::{self, Cursor};
use std::panic::{AssertUnwindSafe, catch_unwind};

use proptest::prelude::*;

use super::*;
use crate::testing::run_to_quiescence;

/// A storage behavior to exercise, including legal outcomes of interruptions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Behavior {
    /// Replace every record completely.
    Reliable,
    /// Leave the previous record on the second store's error.
    FailBefore,
    /// Replace completely before the second store reports an error.
    FailAfter,
    /// Incorrectly retain a partial second store after reporting an error.
    FailPartial,
    /// Leave the previous record while the second store stays pending.
    PauseBefore,
    /// Replace completely while the second store stays pending.
    PauseAfter,
    /// Incorrectly expose a partial second store while pending.
    PausePartial,
    /// Incorrectly consume the record when opening a reader.
    Consume,
    /// Incorrectly append new contents to existing storage.
    Append,
    /// Incorrectly treat present empty bytes as absent storage.
    LoseEmpty,
    /// Incorrectly report success after retaining only a prefix.
    Truncate,
    /// Fail the second store, then incorrectly discard the successful retry.
    LoseRetry,
}

/// In-process storage whose returned futures require no `Sync` bound on the adapter.
struct Memory {
    /// The complete record, except in deliberately broken modes.
    bytes: RefCell<Option<Vec<u8>>>,
    /// Stores attempted, used to inject a failure at the requested replacement.
    stores: Cell<usize>,
    /// The behavior this fixture exhibits.
    behavior: Behavior,
    /// Prefix length used by partial-write fixtures.
    cut: usize,
}

/// Construct an isolated fixture with a selected behavior.
impl Memory {
    /// Start with absent storage and no attempted writes.
    fn new(behavior: Behavior, cut: usize) -> Self {
        Self {
            bytes: RefCell::new(None),
            stores: Cell::new(0),
            behavior,
            cut,
        }
    }
}

/// Model complete replacements, interruptions, and their deliberately broken
/// counterparts.
impl Bookmark for Memory {
    /// An injected failure on the configured store.
    type Error = io::Error;
    /// A fresh reader over the stored bytes.
    type Reader = Cursor<Vec<u8>>;

    /// Return the record, consuming it only in the broken-load fixture.
    fn load(&self) -> impl Future<Output = io::Result<Option<Self::Reader>>> + Send {
        let bytes = if self.behavior == Behavior::Consume {
            self.bytes.borrow_mut().take()
        } else {
            self.bytes.borrow().clone()
        };
        ready(Ok(bytes.map(Cursor::new)))
    }

    /// Apply a replacement or injected fault, then complete or remain pending.
    fn store(&self, mut bytes: Vec<u8>) -> impl Future<Output = io::Result<()>> + Send {
        use Behavior::*;
        let call = self.stores.get() + 1;
        self.stores.set(call);
        let mode = if call == 2 { self.behavior } else { Reliable };
        let fails = matches!(mode, FailBefore | FailAfter | FailPartial | LoseRetry);
        let pauses = matches!(mode, PauseBefore | PauseAfter | PausePartial);
        if matches!(mode, FailPartial | PausePartial) || self.behavior == Truncate {
            bytes.truncate(self.cut % bytes.len().max(1));
        }
        if self.behavior == Append {
            self.bytes
                .borrow_mut()
                .get_or_insert_default()
                .extend(bytes);
        } else if self.behavior == LoseEmpty && bytes.is_empty() {
            *self.bytes.borrow_mut() = None;
        } else if !matches!(mode, FailBefore | PauseBefore | LoseRetry)
            && !(self.behavior == LoseRetry && call > 2)
        {
            *self.bytes.borrow_mut() = Some(bytes);
        }
        async move {
            if pauses {
                pending::<()>().await;
            }
            if fails {
                Err(io::Error::other("injected store failure"))
            } else {
                Ok(())
            }
        }
    }
}

/// Require a probe to reject a fixture for the intended contract violation.
fn rejects(future: impl Future<Output = ()>, expected: &str) {
    let error = catch_unwind(AssertUnwindSafe(|| run_to_quiescence(future).unwrap()))
        .expect_err("the faulty bookmark must be rejected");
    let message = error
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| error.downcast_ref::<&str>().copied())
        .unwrap_or("");
    assert!(
        message.contains(expected),
        "unexpected rejection: {message}"
    );
}

/// The complete suite accepts repeatable, immediately completing storage.
#[test]
fn reliable_storage_conforms() {
    run_to_quiescence(check(async || Memory::new(Behavior::Reliable, 0), pending)).unwrap();
}

/// Failed stores may leave either complete record, but later writes take precedence.
#[test]
fn complete_error_outcomes_conform() {
    for mode in [Behavior::FailBefore, Behavior::FailAfter] {
        run_to_quiescence(check_failed_store(async || Memory::new(mode, 0), pending)).unwrap();
    }
}

/// Cancellation may leave either complete record, with future writes still usable.
#[test]
fn complete_cancellation_outcomes_conform() {
    for mode in [Behavior::PauseBefore, Behavior::PauseAfter] {
        run_to_quiescence(check_cancelled_store(
            async || Memory::new(mode, 0),
            pending,
        ))
        .unwrap();
    }
}

proptest! {
    /// Every partial prefix is rejected after an error or cancellation.
    #[test]
    fn partial_interrupted_stores_are_rejected(cut: usize, cancel: bool) {
        let expected = "interrupted store leaves a complete previous or replacement record";
        if cancel {
            rejects(check_cancelled_store(async || Memory::new(Behavior::PausePartial, cut), pending), expected);
        } else {
            rejects(check_failed_store(async || Memory::new(Behavior::FailPartial, cut), pending), expected);
        }
    }

    /// Reporting success with a truncated record is rejected at every prefix.
    #[test]
    fn incomplete_success_is_rejected(cut: usize) {
        rejects(check_storage(async || Memory::new(Behavior::Truncate, cut), pending), "exact stored bytes");
    }
}

/// Loads cannot consume storage, stores cannot append, and empty bytes remain present.
#[test]
fn broken_storage_semantics_are_rejected() {
    for mode in [Behavior::Consume, Behavior::Append, Behavior::LoseEmpty] {
        rejects(
            check_storage(async || Memory::new(mode, 0), pending),
            "exact stored bytes",
        );
    }
}

/// A successful retry must replace the record left by a failed store.
#[test]
fn lost_retry_is_rejected() {
    rejects(
        check_failed_store(async || Memory::new(Behavior::LoseRetry, 0), pending),
        "exact stored bytes",
    );
}

/// A failure probe must actually reach an error in the supplied fixture.
#[test]
fn failure_injection_is_required() {
    rejects(
        check_failed_store(async || Memory::new(Behavior::Reliable, 0), pending),
        "fixture must fail the second store",
    );
}

/// Deadlines name the affected check and also cover a stalled factory.
#[test]
fn deadlines_cover_construction_and_name_the_check() {
    rejects(
        check_storage(async || pending::<Memory>().await, || ready(())),
        "bookmark::check_storage timed out",
    );
    rejects(
        check_cancelled_store(async || pending::<Memory>().await, || ready(())),
        "bookmark::check_cancelled_store timed out",
    );
    rejects(
        check_failed_store(async || pending::<Memory>().await, || ready(())),
        "bookmark::check_failed_store timed out",
    );
}
