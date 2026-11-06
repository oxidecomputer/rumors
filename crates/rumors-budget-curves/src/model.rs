//! The model's quantities: each shape's unlimited sessions, read for what
//! the budget model speaks of rather than for a curve.
//!
//! For every shape the mode runs the unlimited session once on a
//! latency-only link, whose duration in one-way delays is the session's
//! hop count `h`, and once on each link with bandwidth, which reads the
//! busier direction's bytes `W`, the unlimited duration `T∞`, and each
//! side's `Λ` and share peak, each with where and when it occurred. One
//! JSON document per session is appended to the output file, and a rerun
//! skips every session the file already holds.

use std::collections::BTreeSet;
use std::fs::OpenOptions;
use std::path::Path;
use std::sync::Mutex;
use std::time::Instant;

use rayon::prelude::*;
use rumors_testkit::bench::link::{LinkConfig, Side};
use rumors_testkit::bench::session::{self, Case, LedgerReading, Shape};
use rumors_testkit::common::budget::BudgetChoice;
use serde::{Deserialize, Serialize};

use crate::child::Job;
use crate::record::{CheckRecord, DepthGrants, depths, side_name};
use crate::simulate::{append, read_lines};

/// One shape, as the mode's command line names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ShapeSpec {
    /// Messages both sides hold.
    pub shared: u64,
    /// Messages each side inserted that the other has never seen, in
    /// [`Side::BOTH`] order.
    pub inserted: [u64; 2],
    /// Messages both received that each side has redacted and the other
    /// still holds, in [`Side::BOTH`] order.
    pub redacted: [u64; 2],
    /// The draw of the pair's tree ([`Shape::draw`]); records that lack
    /// it are draw zero.
    #[serde(default)]
    pub draw: u64,
}

impl ShapeSpec {
    /// Parses `s:iA:iB` or `s:iA:iB:rA:rB`, either followed by `@draw`.
    ///
    /// # Errors
    ///
    /// If the counts are not three or five unsigned integers separated by
    /// colons, or the draw is not an unsigned integer.
    pub fn parse(text: &str) -> Result<Self, String> {
        let (counts, draw) = match text.split_once('@') {
            Some((counts, draw)) => (
                counts,
                draw.parse::<u64>()
                    .map_err(|error| format!("shape `{text}`: {error}"))?,
            ),
            None => (text, 0),
        };
        let counts = counts
            .split(':')
            .map(str::parse::<u64>)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("shape `{text}`: {error}"))?;
        match counts[..] {
            [shared, a, b] => Ok(Self {
                shared,
                inserted: [a, b],
                redacted: [0, 0],
                draw,
            }),
            [shared, a, b, ra, rb] => Ok(Self {
                shared,
                inserted: [a, b],
                redacted: [ra, rb],
                draw,
            }),
            _ => Err(format!("shape `{text}` is not s:iA:iB or s:iA:iB:rA:rB")),
        }
    }

    /// The test kit's shape, every message encoding in `message_bytes`.
    fn shape(self, message_bytes: usize) -> Shape {
        Shape {
            shared: self.shared,
            a: self.inserted[0],
            b: self.inserted[1],
            message_bytes,
            draw: self.draw,
            ..Shape::catch_up(0)
        }
        .with_redactions(self.redacted[0], self.redacted[1])
    }
}

/// The link a session ran over.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LinkSpec {
    /// The bandwidth-delay product, in bytes: twice the one-way delay.
    pub bdp: u64,
    /// Whether bytes serialize in no time ([`LinkConfig::latency_only`]).
    pub latency_only: bool,
}

impl LinkSpec {
    /// The link's configuration.
    fn config(self) -> LinkConfig {
        let link = LinkConfig::with_bdp(self.bdp);
        if self.latency_only {
            link.latency_only()
        } else {
            link
        }
    }
}

/// One unlimited session, read for the model's quantities.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelRecord {
    /// The replicas.
    pub shape: ShapeSpec,
    /// The bytes each message encodes in.
    pub message_bytes: u64,
    /// The link.
    pub link: LinkSpec,
    /// The session's duration, in byte-times: `T∞` on a link with
    /// bandwidth.
    pub duration: u64,
    /// The duration in one-way delays, on a latency-only link: the hop
    /// count `h`.
    pub hops: Option<u64>,
    /// The bytes each side sent, in [`Side::BOTH`] order; the larger is
    /// `W`.
    pub sent: [u64; 2],
    /// Each side's ledger, in [`Side::BOTH`] order.
    pub ledgers: Vec<ModelLedger>,
}

/// One side's ledger in an unlimited session.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelLedger {
    /// The side, `"A"` or `"B"`.
    pub side: Option<String>,
    /// Whether the side initiated: it asks the question about the root.
    pub initiator: bool,
    /// The most it held outside its floor slots: `Λ` for this side.
    pub budget_peak: u64,
    /// The link time it first held that much, in byte-times.
    pub budget_peak_at: u64,
    /// What each holder outside floor slots held then.
    pub budget_peak_holders: Vec<HolderBytes>,
    /// The largest sum a share check weighed: the share peak for this side.
    pub share_peak: u64,
    /// The check that weighed it, dated.
    pub share_peak_check: Option<CheckRecord>,
    /// The sum over floor slots of the largest grant each admitted.
    pub floor_bound: u64,
    /// Its question grants at each depth that had any.
    pub questions: Vec<DepthGrants>,
    /// Its answer grants at each depth that had any.
    pub answers: Vec<DepthGrants>,
}

/// What one holder outside floor slots held at a moment.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HolderBytes {
    /// The place, by name.
    pub place: String,
    /// Whether the units are questions, if admission granted the charges.
    pub question: Option<bool>,
    /// The depth of the scope the units concern, if admission granted
    /// them.
    pub depth: Option<usize>,
    /// The bytes it held.
    pub bytes: u64,
}

impl ModelLedger {
    /// Reads `reading`.
    fn of(reading: &LedgerReading) -> Self {
        Self {
            side: reading.side.map(side_name),
            initiator: reading.questions[0].units > 0,
            budget_peak: reading.budget_peak as u64,
            budget_peak_at: reading.budget_peak_at,
            budget_peak_holders: reading
                .budget_peak_holders
                .iter()
                .map(|(holder, bytes)| HolderBytes {
                    place: format!("{:?}", holder.place),
                    question: holder.unit.map(|unit| unit.question),
                    depth: holder.unit.map(|unit| unit.depth),
                    bytes: *bytes as u64,
                })
                .collect(),
            share_peak: reading.share_peak as u64,
            share_peak_check: reading.share_peak_check.as_ref().map(CheckRecord::of),
            floor_bound: reading.floor_bound as u64,
            questions: depths(&reading.questions),
            answers: depths(&reading.answers),
        }
    }
}

/// Runs `shape`'s unlimited session over `link`, every message encoding
/// in `message_bytes`, and returns its record's JSON document.
pub fn session(shape: ShapeSpec, link: LinkSpec, message_bytes: usize) -> String {
    let started = Instant::now();
    let case = Case {
        shape: shape.shape(message_bytes),
        budget: BudgetChoice::Bytes(usize::MAX),
        link: link.config(),
    };
    let session = session::run(case);
    let mut ledgers: Vec<ModelLedger> = session
        .admitted
        .ledgers
        .iter()
        .map(ModelLedger::of)
        .collect();
    ledgers.sort_by_key(|ledger| ledger.side.clone());
    let record = ModelRecord {
        shape,
        message_bytes: message_bytes as u64,
        link,
        duration: session.duration(),
        hops: link.latency_only.then(|| session.hops()),
        sent: Side::BOTH.map(|side| session.report.sent_by(side).bytes()),
        ledgers,
    };
    eprintln!(
        "{shape:?} over {link:?}: {} byte-times{} ({:.1} s)",
        record.duration,
        record
            .hops
            .map_or(String::new(), |hops| format!(", {hops} hops")),
        started.elapsed().as_secs_f64(),
    );
    serde_json::to_string(&record).expect("a record serializes")
}

/// Runs every shape's unlimited session over a latency-only link of
/// product `latency_bdp` and over each link in `links`, appending each to
/// `out` and skipping those it already holds.
///
/// Every message encodes in `message_bytes`.
///
/// # Errors
///
/// If `out` cannot be read or opened, or holds sessions of another
/// message size.
pub fn run(
    out: &Path,
    shapes: &[ShapeSpec],
    links: &[u64],
    latency_bdp: u64,
    message_bytes: usize,
    jobs: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let existing: Vec<ModelRecord> = read_lines(out)?;
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
    let done: BTreeSet<(ShapeSpec, LinkSpec)> = existing
        .iter()
        .map(|record| (record.shape, record.link))
        .collect();
    let mut sessions: Vec<(ShapeSpec, LinkSpec)> = Vec::new();
    for &shape in shapes {
        sessions.push((
            shape,
            LinkSpec {
                bdp: latency_bdp,
                latency_only: true,
            },
        ));
        for &bdp in links {
            sessions.push((
                shape,
                LinkSpec {
                    bdp,
                    latency_only: false,
                },
            ));
        }
    }
    sessions.retain(|key| !done.contains(key));
    let file = Mutex::new(OpenOptions::new().create(true).append(true).open(out)?);
    let pool = rayon::ThreadPoolBuilder::new().num_threads(jobs).build()?;
    pool.install(|| {
        sessions.into_par_iter().try_for_each(|(shape, link)| {
            let line = Job::Model {
                shape,
                link,
                message_bytes,
            }
            .run_in_child()?;
            append(&file, out, &line)
        })
    })?;
    Ok(())
}
