//! One session as the simulation writes it and the renderer reads it.

use std::collections::BTreeMap;

use alloc_meter::Stats;
use rumors::testing::Holding;
use rumors_testkit::bench::link::Side;
use rumors_testkit::bench::session::{
    DatedAdmission, DatedCheck, Grants, Held, Holder, LedgerReading, MESSAGE_BYTES, Session, Step,
};
use serde::{Deserialize, Serialize};

use crate::grid::Pair;

/// The deepest scope whose admissions every record dates one by one
/// ([`LedgerRecord::dated`]).
///
/// That is depth 1, the top-level subtrees, whose questions the root's
/// answer asks and whose answers open them. A detailed record dates every
/// admission besides ([`LedgerRecord::admissions`]).
pub const DATED_DEPTH: usize = 1;

/// One session: its pair, budget, timing, and each side's ledger.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Record {
    /// Messages each replica holds.
    pub messages: u64,
    /// The share of each replica the other holds too, in percent.
    pub shared_percent: u64,
    /// The link's bandwidth-delay product, in bytes.
    pub bdp: u64,
    /// The bytes each fixture message encodes in; files written before
    /// the size was recorded hold the default's.
    #[serde(default = "default_message_bytes")]
    pub message_bytes: u64,
    /// Both sides' budget, in bytes, or `None` for an unlimited budget.
    pub budget: Option<u64>,
    /// The session's duration, in byte-times.
    pub duration: u64,
    /// The bytes each side sent, in [`Side::BOTH`] order.
    pub sent: [u64; 2],
    /// Each side's ledger, in the order of its first report.
    pub ledgers: Vec<LedgerRecord>,
    /// The most bytes both ledgers held together at once, floor slots
    /// included: what the session counted as held or reserved.
    pub joint_held_peak: u64,
    /// The most heap the session allocated beyond what was live when it
    /// began, both sides together: everything it actually held, its fixed
    /// state and the replicas' growth included.
    pub heap_peak: u64,
    /// The heap the session left allocated when it ended: the replicas'
    /// growth, less what it freed of the replicas it replaced.
    pub heap_net: i64,
}

/// What one side's ledger held and admitted.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LedgerRecord {
    /// The side whose session it belongs to, `"A"` or `"B"`; a record that
    /// lacks it reads it as unknown.
    #[serde(default)]
    pub side: Option<String>,
    /// The most it held outside its floor slots.
    pub budget_peak: u64,
    /// The most it held in all.
    pub held_peak: u64,
    /// The most its floor slots held at once.
    pub floor_peak: u64,
    /// The sum over floor slots of the largest grant each admitted.
    pub floor_bound: u64,
    /// The largest sum any invitation share check weighed: the share peak.
    ///
    /// With `budget_peak`, it gives the session's no-cost point, the least
    /// budget at which it runs as an unlimited session does. A record that
    /// lacks it reads it as zero.
    #[serde(default)]
    pub share_peak: u64,
    /// The check that weighed the share peak, if it made any: where and
    /// when the peak occurred, and its terms.
    #[serde(default)]
    pub share_peak_check: Option<CheckRecord>,
    /// Units that waited for admission.
    pub waited: u64,
    /// The bytes it held outside its floor slots, integrated over the
    /// session's byte-times.
    pub budget_area: u128,
    /// The bytes it held for each place, floor slots included, integrated
    /// over the session's byte-times, by the place's name.
    #[serde(default)]
    pub place_area: BTreeMap<String, u128>,
    /// The bytes its floor slots held, integrated over the session's
    /// byte-times.
    #[serde(default)]
    pub floor_area: u128,
    /// Its question grants at each depth that had any.
    pub questions: Vec<DepthGrants>,
    /// Its answer grants at each depth that had any.
    pub answers: Vec<DepthGrants>,
    /// Its admissions at depth [`DATED_DEPTH`] or shallower, dated, in
    /// order ([`LedgerReading::dated`]).
    #[serde(default)]
    pub dated: Vec<DatedRecord>,
    /// The bytes it held outside its floor slots over the session, as
    /// `[at, bytes]` steps ([`LedgerReading::profile`]).
    ///
    /// Every unlimited session records it, since the profile estimate reads
    /// it; a budgeted one records it only in a detailed record.
    #[serde(default)]
    pub profile: Vec<(u64, u64)>,
    /// The link time during which at least one unit waited for admission.
    #[serde(default)]
    pub waiting_time: u64,
    /// What each holder of its bytes held ([`LedgerReading::holders`]).
    #[serde(default)]
    pub holders: Vec<HolderRecord>,
    /// Every unit that waited, as `[queued, admitted, depth, question,
    /// bytes, floor]` ([`LedgerReading::waits`]): in a detailed record
    /// only.
    #[serde(default)]
    pub waits: Vec<(u64, u64, usize, bool, u64, bool)>,
    /// Every admission, as `[at, depth, question, floor, waited]`
    /// ([`LedgerReading::dated`]): in a detailed record only.
    #[serde(default)]
    pub admissions: Vec<(u64, usize, bool, bool, bool)>,
}

/// What one holder of a ledger's bytes held: one place's charges for units
/// of one kind at one depth, through floor slots or against the budget
/// ([`Holder`], [`Held`]).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HolderRecord {
    /// The place, by name.
    pub place: String,
    /// Whether the units are questions, if admission granted the charges.
    pub question: Option<bool>,
    /// The depth of the scope the units concern, if admission granted the
    /// charges.
    pub depth: Option<usize>,
    /// Whether the charges were built out of floor grants.
    pub floor: bool,
    /// The bytes it held, integrated over the session's byte-times.
    pub area: u128,
    /// The bytes it held while some unit waited for admission, integrated
    /// likewise.
    pub area_waiting: u128,
    /// The bytes that entered it.
    pub inflow: u64,
}

/// One dated admission ([`DatedAdmission`]).
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct DatedRecord {
    /// The link time of the admission, in byte-times.
    pub at: u64,
    /// The depth of the scope the unit concerns.
    pub depth: usize,
    /// Whether the unit is a question rather than an answer's own items.
    pub question: bool,
    /// Whether it took its depth's floor slot.
    pub floor: bool,
    /// Whether it waited for admission.
    pub waited: bool,
    /// Its grant's bytes.
    pub bytes: u64,
}

/// One invitation share check, dated ([`DatedCheck`]): the question it
/// decided and the terms it weighed, `invited + grant + α · (claimed +
/// need)`, with `α = pace_actual / pace_predicted` and the scaled term
/// rounded down.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct CheckRecord {
    /// The link time it was reported at, in byte-times.
    pub at: u64,
    /// The sum it weighed.
    pub sum: u64,
    /// The depth of the scope the question asks about.
    pub depth: usize,
    /// The bytes that depth's questions held against the budget, `I_d`.
    pub invited: u64,
    /// The question's grant, `c`.
    pub grant: u64,
    /// The depth's outstanding claims before the question's own, `N_d`.
    pub claimed: u64,
    /// The question's predicted need, `n`.
    pub need: u64,
    /// The depth's pace's numerator: bytes its answers kept.
    pub pace_actual: u64,
    /// The depth's pace's denominator: the needs it predicted.
    pub pace_predicted: u64,
    /// The bytes the ledger held outside its floor slots then, `L − F`.
    pub normal: u64,
}

impl CheckRecord {
    /// Records `dated`.
    pub fn of(dated: &DatedCheck) -> Self {
        let check = dated.check;
        Self {
            at: dated.at,
            sum: check.sum as u64,
            depth: check.depth,
            invited: check.invited as u64,
            grant: check.grant as u64,
            claimed: check.claimed as u64,
            need: check.need as u64,
            pace_actual: check.pace_actual,
            pace_predicted: check.pace_predicted,
            normal: check.normal as u64,
        }
    }
}

/// The name a record gives `side`.
pub fn side_name(side: Side) -> String {
    match side {
        Side::A => "A",
        Side::B => "B",
    }
    .to_owned()
}

/// The message size of a record written before the size was recorded.
fn default_message_bytes() -> u64 {
    MESSAGE_BYTES as u64
}

/// One ledger's grants to one kind of unit at one depth.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct DepthGrants {
    /// The depth of the scope the units concern.
    pub depth: usize,
    /// Units granted.
    pub units: u64,
    /// Bytes granted.
    pub bytes: u64,
    /// Units that waited.
    pub waited: u64,
    /// Units admitted through the floor slot.
    pub floor: u64,
    /// Bytes the grants returned unused once their units were built.
    pub returned: u64,
}

impl Record {
    /// Records `session`, which ran `pair` at `budget` and whose heap
    /// `heap` read, with every profile, wait, and admission if `detail`.
    pub fn of(
        pair: Pair,
        budget: Option<u64>,
        session: &Session,
        heap: &Stats,
        detail: bool,
    ) -> Self {
        let profiles = detail || budget.is_none();
        Self {
            messages: pair.messages,
            shared_percent: pair.shared_percent,
            bdp: pair.bdp,
            message_bytes: session.case.shape.message_bytes as u64,
            budget,
            duration: session.duration(),
            sent: Side::BOTH.map(|side| session.report.sent_by(side).bytes()),
            ledgers: session
                .admitted
                .ledgers
                .iter()
                .map(|reading| LedgerRecord::of(reading, profiles, detail))
                .collect(),
            joint_held_peak: session.admitted.joint_held_peak as u64,
            heap_peak: heap.peak_bytes as u64,
            heap_net: heap.net_bytes as i64,
        }
    }

    /// The pair the session ran.
    pub fn pair(&self) -> Pair {
        Pair {
            messages: self.messages,
            shared_percent: self.shared_percent,
            bdp: self.bdp,
        }
    }

    /// The bytes the busier direction carried.
    pub fn busier_bytes(&self) -> u64 {
        self.sent[0].max(self.sent[1])
    }

    /// The most either ledger held outside its floor slots: `Λ` when the
    /// budget is unlimited.
    pub fn budget_peak(&self) -> u64 {
        self.ledgers
            .iter()
            .map(|ledger| ledger.budget_peak)
            .max()
            .unwrap_or(0)
    }

    /// The larger of the two sides' question grant totals.
    pub fn question_grants(&self) -> u64 {
        self.ledgers
            .iter()
            .map(|ledger| ledger.questions.iter().map(|depth| depth.bytes).sum())
            .max()
            .unwrap_or(0)
    }

    /// The larger of the two sides' totals of what their questions kept
    /// out of their grants.
    pub fn question_bytes(&self) -> u64 {
        self.ledgers
            .iter()
            .map(|ledger| {
                ledger
                    .questions
                    .iter()
                    .map(|depth| depth.bytes - depth.returned)
                    .sum()
            })
            .max()
            .unwrap_or(0)
    }

    /// Units that waited, over both ledgers.
    pub fn waited(&self) -> u64 {
        self.ledgers.iter().map(|ledger| ledger.waited).sum()
    }
}

impl LedgerRecord {
    /// Records `reading`, with its profile if `profile`, and its waits and
    /// every admission if `detail`.
    fn of(reading: &LedgerReading, profile: bool, detail: bool) -> Self {
        Self {
            side: reading.side.map(side_name),
            budget_peak: reading.budget_peak as u64,
            held_peak: reading.held_peak as u64,
            floor_peak: reading.floor_peak as u64,
            floor_bound: reading.floor_bound as u64,
            share_peak: reading.share_peak as u64,
            share_peak_check: reading.share_peak_check.as_ref().map(CheckRecord::of),
            waited: reading.waited,
            budget_area: reading.budget_area,
            place_area: Holding::ALL
                .iter()
                .zip(reading.place_area)
                .map(|(place, area)| (format!("{place:?}"), area))
                .collect(),
            floor_area: reading.floor_area,
            questions: depths(&reading.questions),
            answers: depths(&reading.answers),
            dated: reading
                .dated
                .iter()
                .filter(|admission| admission.depth <= DATED_DEPTH)
                .map(DatedRecord::of)
                .collect(),
            profile: if profile {
                reading
                    .profile
                    .iter()
                    .map(|step| (step.at, step.bytes as u64))
                    .collect()
            } else {
                Vec::new()
            },
            waiting_time: reading.waiting_time,
            holders: reading
                .holders
                .iter()
                .map(|(holder, held)| HolderRecord::of(holder, held))
                .collect(),
            waits: if detail {
                reading
                    .waits
                    .iter()
                    .map(|wait| {
                        (
                            wait.queued,
                            wait.admitted,
                            wait.depth,
                            wait.question,
                            wait.bytes as u64,
                            wait.floor,
                        )
                    })
                    .collect()
            } else {
                Vec::new()
            },
            admissions: if detail {
                reading
                    .dated
                    .iter()
                    .map(|admission| {
                        (
                            admission.at,
                            admission.depth,
                            admission.question,
                            admission.floor,
                            admission.waited,
                        )
                    })
                    .collect()
            } else {
                Vec::new()
            },
        }
    }

    /// Its profile as the profile estimate reads it.
    pub fn steps(&self) -> Vec<Step> {
        self.profile
            .iter()
            .map(|&(at, bytes)| Step {
                at,
                bytes: usize::try_from(bytes).expect("a recorded holding fits a usize"),
            })
            .collect()
    }
}

impl HolderRecord {
    /// Records what `holder` held.
    fn of(holder: &Holder, held: &Held) -> Self {
        Self {
            place: format!("{:?}", holder.place),
            question: holder.unit.map(|unit| unit.question),
            depth: holder.unit.map(|unit| unit.depth),
            floor: holder.through_floor,
            area: held.area,
            area_waiting: held.area_waiting,
            inflow: held.inflow,
        }
    }
}

impl DatedRecord {
    /// Records `admission`.
    fn of(admission: &DatedAdmission) -> Self {
        Self {
            at: admission.at,
            depth: admission.depth,
            question: admission.question,
            floor: admission.floor,
            waited: admission.waited,
            bytes: admission.bytes as u64,
        }
    }
}

/// The depths of `grants` that granted anything.
pub fn depths(grants: &[Grants]) -> Vec<DepthGrants> {
    grants
        .iter()
        .enumerate()
        .filter(|(_, grants)| grants.units > 0)
        .map(|(depth, grants)| DepthGrants {
            depth,
            units: grants.units,
            bytes: grants.bytes,
            waited: grants.waited,
            floor: grants.floor,
            returned: grants.returned,
        })
        .collect()
}
