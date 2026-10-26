//! Tests for the divided property runner.

use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use proptest::prelude::*;

use super::run;

/// The environment variable that marks the probe's child process.
const PROBE: &str = "BEFORE_DIVIDED_CASES_PROBE";

/// The text the probe prints before the number of cases its property ran.
const REPORT: &str = "divided property cases: ";

/// A property divided by 8 runs 12 cases when `PROPTEST_CASES` sets the
/// configured count to 96: the divisor applies to the count the environment
/// sets, not only to the default.
///
/// Proptest reads `PROPTEST_CASES` once per process, so the test re-runs
/// itself in a child process with the variable set, and the child reports the
/// cases its property executed. The parent requires the report, so a child
/// that ran no probe fails the test rather than passing it.
#[test]
fn divided_properties_honor_their_divisor_when_proptest_cases_is_set() {
    if std::env::var_os(PROBE).is_some() {
        static CASES: AtomicU32 = AtomicU32::new(0);
        run(8, "divided_probe", file!(), any::<u8>(), |_| {
            CASES.fetch_add(1, Ordering::Relaxed);
            Ok(())
        });
        println!("{REPORT}{}", CASES.load(Ordering::Relaxed));
        return;
    }
    let (_, test_path) = module_path!()
        .split_once("::")
        .expect("a test module path names its crate first");
    let test =
        format!("{test_path}::divided_properties_honor_their_divisor_when_proptest_cases_is_set");
    let output = Command::new(std::env::current_exe().expect("the test binary has a path"))
        .args(["--exact", &test, "--nocapture", "--test-threads", "1"])
        .env(PROBE, "1")
        .env("PROPTEST_CASES", "96")
        .output()
        .expect("the test binary re-runs");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "the probe failed ({}):\n{stdout}\n{stderr}",
        output.status
    );
    let reported = stdout
        .lines()
        // libtest prints the test's name on the line the report continues.
        .find_map(|line| line.split_once(REPORT).map(|(_, cases)| cases))
        .unwrap_or_else(|| panic!("the probe reported no case count:\n{stdout}\n{stderr}"));
    assert_eq!(
        reported, "12",
        "a property divided by 8 must run 96 / 8 cases"
    );
}
