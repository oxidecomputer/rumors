//! Running the grid: each pair's unlimited session, then its budgets.

use std::collections::BTreeSet;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::sync::Mutex;
use std::time::Instant;

use rayon::prelude::*;
use rumors::testing::ledger_budget_within;
use rumors_testkit::bench::link::LinkConfig;
use rumors_testkit::bench::session::{self, Case};
use rumors_testkit::common::budget::BudgetChoice;
use serde::de::DeserializeOwned;

use crate::child::Job;
use crate::grid::{Ladder, Pair};
use crate::record::Record;

/// The key a session is stored under: its pair and budget.
type Key = (Pair, Option<u64>);

/// Reads every record in `path`, or none if it does not exist.
pub fn read(path: &Path) -> Result<Vec<Record>, Box<dyn std::error::Error>> {
    read_lines(path)
}

/// Reads every JSON document in `path`, one per line, or none if it does not
/// exist.
pub fn read_lines<T: DeserializeOwned>(path: &Path) -> Result<Vec<T>, Box<dyn std::error::Error>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let mut records = Vec::new();
    for line in BufReader::new(File::open(path)?).lines() {
        let line = line?;
        if !line.trim().is_empty() {
            records.push(serde_json::from_str(&line)?);
        }
    }
    Ok(records)
}

/// Runs `pair` at the memory budget `budget` (`None` for unlimited), every
/// message encoding in `message_bytes` bytes, and returns its record's JSON
/// document, a detailed one if `detail` ([`Record::of`]).
///
/// The budget applies as given, through the test path, which sets the ledger
/// budget the memory budget leaves beside the floor and session allowances: the
/// ladder's rungs are never below the public minimum, and an explicit budget
/// below it probes the regime the public setter rules out.
pub fn session(pair: Pair, budget: Option<u64>, message_bytes: usize, detail: bool) -> String {
    let started = Instant::now();
    let budget_choice = match budget {
        None => BudgetChoice::Bytes(usize::MAX),
        Some(budget) => BudgetChoice::Ledger(ledger_budget_within(
            usize::try_from(budget).expect("a budget the simulation runs fits a usize"),
        )),
    };
    let case = Case {
        shape: pair.shape().with_message_bytes(message_bytes),
        budget: budget_choice,
        link: LinkConfig::with_bdp(pair.bdp),
    };
    let (session, heap) = session::run_within(case, |session| alloc_meter::measure(session).0);
    let record = Record::of(pair, budget, &session, &heap, detail);
    eprintln!(
        "{} messages, {}% shared, BDP {} B, budget {}: {} byte-times, {} waited ({:.1} s)",
        pair.messages,
        pair.shared_percent,
        pair.bdp,
        budget.map_or("unlimited".to_owned(), |budget| budget.to_string()),
        record.duration,
        record.waited(),
        started.elapsed().as_secs_f64(),
    );
    serde_json::to_string(&record).expect("a record serializes")
}

/// Appends `line` to `out` and flushes it, so a run cut short keeps every
/// session it finished.
///
/// # Errors
///
/// If the file refuses the line.
pub fn append(out: &Mutex<File>, path: &Path, line: &str) -> Result<(), String> {
    let mut out = out.lock().expect("no writer panics holding the file");
    writeln!(out, "{line}")
        .and_then(|()| out.flush())
        .map_err(|error| format!("appending a session to {}: {error}", path.display()))
}

/// Runs every pair's unlimited session and, with a ladder, its budgets, every
/// message encoding in `message_bytes` bytes, appending each to `out`, detailed
/// if `detail`, and skipping those it already holds.
///
/// # Errors
///
/// If `out` holds sessions of another message size: a file's sessions are keyed
/// by pair and budget alone, so it holds one message size.
pub fn run(
    out: &Path,
    pairs: &[Pair],
    ladder: Option<Ladder>,
    message_bytes: usize,
    detail: bool,
    jobs: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let existing = read(out)?;
    if let Some(record) = existing
        .iter()
        .find(|record| record.message_bytes != message_bytes as u64)
    {
        return Err(format!(
            "{} holds sessions of {}-byte messages, not {message_bytes}",
            out.display(),
            record.message_bytes
        )
        .into());
    }
    let done: BTreeSet<Key> = existing
        .iter()
        .map(|record| (record.pair(), record.budget))
        .collect();
    let file = Mutex::new(OpenOptions::new().create(true).append(true).open(out)?);
    let run_session = |pair: Pair, budget: Option<u64>| -> Result<String, String> {
        let line = Job::Simulate {
            pair,
            budget,
            message_bytes,
            detail,
        }
        .run_in_child()?;
        append(&file, out, &line)?;
        Ok(line)
    };
    let pool = rayon::ThreadPoolBuilder::new().num_threads(jobs).build()?;
    pool.install(|| -> Result<(), String> {
        let unlimited: Vec<Record> = pairs
            .par_iter()
            .map(|&pair| {
                match existing
                    .iter()
                    .find(|record| record.pair() == pair && record.budget.is_none())
                {
                    Some(record) => Ok(record.clone()),
                    None => serde_json::from_str(&run_session(pair, None)?).map_err(|error| {
                        format!("reading back the unlimited session of {pair:?}: {error}")
                    }),
                }
            })
            .collect::<Result<_, String>>()?;
        let Some(ladder) = &ladder else {
            return Ok(());
        };
        let mut pending: Vec<Key> = unlimited
            .iter()
            .flat_map(|record| {
                ladder
                    .budgets(record.budget_peak())
                    .into_iter()
                    .map(move |budget| (record.pair(), Some(budget)))
            })
            .filter(|key| !done.contains(key))
            .collect();
        // The largest pairs first, so the pool ends on short sessions
        // rather than waiting out a long one.
        pending.sort_by_key(|(pair, _)| std::cmp::Reverse(pair.messages));
        pending
            .into_par_iter()
            .try_for_each(|(pair, budget)| run_session(pair, budget).map(drop))
    })?;
    Ok(())
}
