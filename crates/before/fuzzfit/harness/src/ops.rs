//! The program vocabulary, the native mirror, and the denomination rules.
//!
//! A *program* is a sequence of [`Op`]s over a register file, one op per
//! public `before` operation, mirroring the guest ABI one-to-one. Programs
//! are the harness's only unit of execution: strategies generate them,
//! the mirror executes them natively, and the driver replays them in the
//! guest under fuel metering.
//!
//! # Denomination
//!
//! Every measured step is judged against its *denominated size* in bits,
//! following the parent crate's denomination criterion of record: packed
//! operand bits for every operation except the classes
//! whose mandatory output is asymptotically larger than any constant times
//! their input —
//!
//! - **text I/O** (`Display`/`FromStr`) is judged against packed input +
//!   text output (or text input + packed output), output read from the
//!   actual result;
//! - **output-dominated projection** (`Version / Party`,
//!   `Clock::own_version`) and **balanced share splitting**
//!   (`Party::forks(n)`, whose output is `n` packed parties) are judged
//!   against input + packed output (canonical coding cannot be padded);
//! - **rank operations** denominate against value content, proxied here by
//!   the rank's decimal rendering (digits ∝ numerator bits; the constant
//!   folds into the fit's intercept).
//!
//! # The mirror
//!
//! [`Mirror`] executes each op natively over the same register discipline
//! the guest uses (linear where the API is linear), computing the step's
//! denominator from real operand sizes and predicting the guest's return
//! code. Where an operation can reject (joining overlapping parties — the
//! cross-universe generators reach this arm deliberately), the mirror's
//! native outcome is the prediction; a guest that disagrees fails the
//! differential immediately.

use std::cmp::Ordering;

use before::{Clock, Party, Rank, Version};

/// A register index in both the mirror's and the guest's file.
pub type Reg = u32;

/// One public operation over registers; the harness's program alphabet.
///
/// Field conventions mirror the guest ABI: `dst` slots are written, plain
/// operand slots are read, and operands documented as consumed follow the
/// API's own linearity (`Version` join/meet consume `a`; `Party`/`Clock`
/// join consume `b`; `into_parts`/`from_parts` consume their sources).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    /// `Clock::seed` — a new universe. Programs in the coupled regime call
    /// this exactly once; the independent regime seeds several.
    ClockSeed { dst: Reg },
    /// `Clock::tick`.
    ClockTick { c: Reg },
    /// `Clock::fork`.
    ClockFork { dst: Reg, src: Reg },
    /// `Clock::join` (consumes `b`; rejects on overlap).
    ClockJoin { a: Reg, b: Reg },
    /// `Clock::send`.
    ClockSend { c: Reg },
    /// `Clock::recv`.
    ClockRecv { c: Reg, v: Reg },
    /// `Clock::sync` (rejects on overlap).
    ClockSync { a: Reg, b: Reg },
    /// `Clock::own_version` (output-dominated row).
    ClockOwnVersion { dst: Reg, src: Reg },
    /// `Clock::version`, cloned out (the clock-to-version bridge).
    ClockVersion { dst: Reg, src: Reg },
    /// `Clock::into_parts`.
    ClockIntoParts { dst_p: Reg, dst_v: Reg, src: Reg },
    /// `Clock::from_parts`.
    ClockFromParts { dst: Reg, p: Reg, v: Reg },
    /// `Clock::encode` into the stage.
    ClockEncode { src: Reg },
    /// `Clock::decode` from the stage.
    ClockDecode { dst: Reg },
    /// `Version::tick`.
    VersionTick { v: Reg, p: Reg },
    /// `Version | Version` (consumes `a`).
    VersionJoin { dst: Reg, a: Reg, b: Reg },
    /// `Version & Version` (consumes `a`).
    VersionMeet { dst: Reg, a: Reg, b: Reg },
    /// `&Version / &Party` (output-dominated row).
    VersionProject { dst: Reg, v: Reg, p: Reg },
    /// `PartialOrd` on versions.
    VersionCmp { a: Reg, b: Reg },
    /// `Version::concurrent`.
    VersionConcurrent { a: Reg, b: Reg },
    /// `Version::rank`.
    VersionRank { dst: Reg, src: Reg },
    /// `Version::distance`.
    VersionDistance { dst: Reg, a: Reg, b: Reg },
    /// `Version::lag`.
    VersionLag { dst: Reg, a: Reg, b: Reg },
    /// `Version::min_ticks`.
    VersionMinTicks { src: Reg },
    /// `Version::join_all` over `src..src + n` (consumes the range).
    VersionJoinAll { dst: Reg, src: Reg, n: u32 },
    /// `Version::meet_all` over `src..src + n` (consumes the range).
    VersionMeetAll { dst: Reg, src: Reg, n: u32 },
    /// `Version::encode` into the stage.
    VersionEncode { src: Reg },
    /// `Version::decode` from the stage.
    VersionDecode { dst: Reg },
    /// `Version` `Display` into the stage.
    VersionDisplay { src: Reg },
    /// `Version` `FromStr` from the stage.
    VersionFromstr { dst: Reg },
    /// `Party::seed` (the party half of a fresh universe).
    PartySeed { dst: Reg },
    /// `Party::fork`.
    PartyFork { dst: Reg, src: Reg },
    /// `Party::forks(n)`: balanced shares into `dst..dst + n`.
    PartyForks { dst: Reg, src: Reg, n: u32 },
    /// `Party::join` (consumes `b`; rejects on overlap).
    PartyJoin { a: Reg, b: Reg },
    /// `Party::is_disjoint`.
    PartyIsDisjoint { a: Reg, b: Reg },
    /// `Party::covers`.
    PartyCovers { a: Reg, b: Reg },
    /// `Party::without` (consumes `a`; an empty difference reports as a
    /// rejection).
    PartyWithout { dst: Reg, a: Reg, b: Reg },
    /// `Party::encode` into the stage.
    PartyEncode { src: Reg },
    /// `Party::decode` from the stage.
    PartyDecode { dst: Reg },
    /// `Party` `Display` into the stage.
    PartyDisplay { src: Reg },
    /// `Party` `FromStr` from the stage.
    PartyFromstr { dst: Reg },
    /// `Rank + &Rank` (consumes `a`).
    RankAdd { dst: Reg, a: Reg, b: Reg },
    /// `Ord` on ranks.
    RankCmp { a: Reg, b: Reg },
    /// `Rank::checked_sub` (underflow reports as a rejection).
    RankCheckedSub { dst: Reg, a: Reg, b: Reg },
}

impl Op {
    /// The guest kernel this op calls; also the calibration's band key.
    pub fn kernel(&self) -> &'static str {
        match self {
            Op::ClockSeed { .. } => "ff_clock_seed",
            Op::ClockTick { .. } => "ff_clock_tick",
            Op::ClockFork { .. } => "ff_clock_fork",
            Op::ClockJoin { .. } => "ff_clock_join",
            Op::ClockSend { .. } => "ff_clock_send",
            Op::ClockRecv { .. } => "ff_clock_recv",
            Op::ClockSync { .. } => "ff_clock_sync",
            Op::ClockOwnVersion { .. } => "ff_clock_own_version",
            Op::ClockVersion { .. } => "ff_clock_version",
            Op::ClockIntoParts { .. } => "ff_clock_into_parts",
            Op::ClockFromParts { .. } => "ff_clock_from_parts",
            Op::ClockEncode { .. } => "ff_clock_encode",
            Op::ClockDecode { .. } => "ff_clock_decode",
            Op::VersionTick { .. } => "ff_version_tick",
            Op::VersionJoin { .. } => "ff_version_join",
            Op::VersionMeet { .. } => "ff_version_meet",
            Op::VersionProject { .. } => "ff_version_project",
            Op::VersionCmp { .. } => "ff_version_cmp",
            Op::VersionConcurrent { .. } => "ff_version_concurrent",
            Op::VersionRank { .. } => "ff_version_rank",
            Op::VersionDistance { .. } => "ff_version_distance",
            Op::VersionLag { .. } => "ff_version_lag",
            Op::VersionMinTicks { .. } => "ff_version_min_ticks",
            Op::VersionJoinAll { .. } => "ff_version_join_all",
            Op::VersionMeetAll { .. } => "ff_version_meet_all",
            Op::VersionEncode { .. } => "ff_version_encode",
            Op::VersionDecode { .. } => "ff_version_decode",
            Op::VersionDisplay { .. } => "ff_version_display",
            Op::VersionFromstr { .. } => "ff_version_fromstr",
            Op::PartySeed { .. } => "ff_party_seed",
            Op::PartyFork { .. } => "ff_party_fork",
            Op::PartyForks { .. } => "ff_party_forks",
            Op::PartyJoin { .. } => "ff_party_join",
            Op::PartyIsDisjoint { .. } => "ff_party_is_disjoint",
            Op::PartyCovers { .. } => "ff_party_covers",
            Op::PartyWithout { .. } => "ff_party_without",
            Op::PartyEncode { .. } => "ff_party_encode",
            Op::PartyDecode { .. } => "ff_party_decode",
            Op::PartyDisplay { .. } => "ff_party_display",
            Op::PartyFromstr { .. } => "ff_party_fromstr",
            Op::RankAdd { .. } => "ff_rank_add",
            Op::RankCmp { .. } => "ff_rank_cmp",
            Op::RankCheckedSub { .. } => "ff_rank_checked_sub",
        }
    }

    /// The guest call's u32 arguments, in the kernel's parameter order.
    pub fn args(&self) -> Vec<u32> {
        match *self {
            Op::ClockSeed { dst } | Op::PartySeed { dst } => vec![dst],
            Op::ClockTick { c } | Op::ClockSend { c } => vec![c],
            Op::ClockFork { dst, src }
            | Op::ClockOwnVersion { dst, src }
            | Op::ClockVersion { dst, src }
            | Op::PartyFork { dst, src }
            | Op::VersionRank { dst, src } => vec![dst, src],
            Op::ClockJoin { a, b }
            | Op::ClockSync { a, b }
            | Op::PartyJoin { a, b }
            | Op::PartyIsDisjoint { a, b }
            | Op::PartyCovers { a, b }
            | Op::VersionCmp { a, b }
            | Op::VersionConcurrent { a, b }
            | Op::RankCmp { a, b } => vec![a, b],
            Op::ClockRecv { c, v } => vec![c, v],
            Op::ClockIntoParts { dst_p, dst_v, src } => vec![dst_p, dst_v, src],
            Op::ClockFromParts { dst, p, v } => vec![dst, p, v],
            Op::VersionTick { v, p } => vec![v, p],
            Op::VersionJoin { dst, a, b }
            | Op::VersionMeet { dst, a, b }
            | Op::VersionDistance { dst, a, b }
            | Op::VersionLag { dst, a, b }
            | Op::PartyWithout { dst, a, b }
            | Op::RankAdd { dst, a, b }
            | Op::RankCheckedSub { dst, a, b } => vec![dst, a, b],
            Op::VersionProject { dst, v, p } => vec![dst, v, p],
            Op::VersionJoinAll { dst, src, n }
            | Op::VersionMeetAll { dst, src, n }
            | Op::PartyForks { dst, src, n } => vec![dst, src, n],
            Op::VersionMinTicks { src }
            | Op::VersionEncode { src }
            | Op::VersionDisplay { src }
            | Op::PartyEncode { src }
            | Op::PartyDisplay { src }
            | Op::ClockEncode { src } => vec![src],
            Op::VersionDecode { dst }
            | Op::VersionFromstr { dst }
            | Op::PartyDecode { dst }
            | Op::PartyFromstr { dst }
            | Op::ClockDecode { dst } => vec![dst],
        }
    }

    /// Whether the kernel returns i64 through the typed path.
    pub fn returns_i64(&self) -> bool {
        matches!(self, Op::VersionMinTicks { .. })
    }
}

/// A natively held register value.
enum NVal {
    V(Version),
    P(Party),
    C(Clock),
    R(Rank),
}

/// What one mirrored step tells the driver.
pub struct Step {
    /// The denominated size, in bits (never zero: floored at 1).
    pub denom_bits: u64,
    /// The return value the guest must produce (`0` success, `-2` a
    /// predicted operation rejection, comparison codes for query ops, the
    /// actual value for i64 kernels).
    pub expect: i64,
}

/// A mirrored execution error: the program is malformed (a generator bug,
/// never a `before` bug).
#[derive(Debug)]
pub struct Malformed {
    /// Which op misfired.
    pub op: String,
}

/// Success code the guest returns.
const OK: i64 = 0;
/// The guest's operation-rejection code (`ERR_OP`).
const ERR_OP: i64 = -2;

/// The native mirror: executes programs natively, computing denominators
/// and expected outcomes for the guest replay.
#[derive(Default)]
pub struct Mirror {
    regs: Vec<Option<NVal>>,
    stage: Vec<u8>,
}

impl Mirror {
    /// A fresh mirror (empty register file and stage).
    pub fn new() -> Mirror {
        Mirror::default()
    }

    fn put(&mut self, dst: Reg, val: NVal) {
        let dst = dst as usize;
        if self.regs.len() <= dst {
            self.regs.resize_with(dst + 1, || None);
        }
        self.regs[dst] = Some(val);
    }

    fn take(&mut self, src: Reg) -> Option<NVal> {
        self.regs.get_mut(src as usize).and_then(Option::take)
    }

    fn version(&self, reg: Reg) -> Option<&Version> {
        match self.regs.get(reg as usize) {
            Some(Some(NVal::V(v))) => Some(v),
            _ => None,
        }
    }

    fn party(&self, reg: Reg) -> Option<&Party> {
        match self.regs.get(reg as usize) {
            Some(Some(NVal::P(p))) => Some(p),
            _ => None,
        }
    }

    fn clock(&self, reg: Reg) -> Option<&Clock> {
        match self.regs.get(reg as usize) {
            Some(Some(NVal::C(c))) => Some(c),
            _ => None,
        }
    }

    fn rank(&self, reg: Reg) -> Option<&Rank> {
        match self.regs.get(reg as usize) {
            Some(Some(NVal::R(r))) => Some(r),
            _ => None,
        }
    }

    /// A rank's value-content proxy: bits of its decimal rendering.
    fn rank_bits(r: &Rank) -> u64 {
        (r.to_string().len() as u64) * 8
    }

    /// Read a register's canonical bytes (ranks render as text), for the
    /// end-of-program differential against the guest.
    pub fn snapshot(&self, reg: Reg) -> Option<Vec<u8>> {
        match self.regs.get(reg as usize) {
            Some(Some(NVal::V(v))) => Some(v.encode()),
            Some(Some(NVal::P(p))) => Some(p.encode()),
            Some(Some(NVal::C(c))) => Some(c.encode()),
            Some(Some(NVal::R(r))) => Some(r.to_string().into_bytes()),
            _ => None,
        }
    }

    /// Registers currently live, tagged `'v' | 'p' | 'c' | 'r'`.
    pub fn live_regs(&self) -> Vec<(Reg, u8)> {
        self.regs
            .iter()
            .enumerate()
            .filter_map(|(i, slot)| {
                let tag = match slot {
                    Some(NVal::V(_)) => b'v',
                    Some(NVal::P(_)) => b'p',
                    Some(NVal::C(_)) => b'c',
                    Some(NVal::R(_)) => b'r',
                    None => return None,
                };
                Some((i as Reg, tag))
            })
            .collect()
    }

    /// Execute one op natively; return its denominator and expected guest
    /// return, or `Malformed` when the program itself is invalid.
    pub fn step(&mut self, op: &Op) -> Result<Step, Malformed> {
        let malformed = || Malformed {
            op: format!("{op:?}"),
        };
        fn done(denom_bits: u64, expect: i64) -> Result<Step, Malformed> {
            Ok(Step {
                denom_bits: denom_bits.max(1),
                expect,
            })
        }
        match *op {
            Op::ClockSeed { dst } => {
                self.put(dst, NVal::C(Clock::seed()));
                done(8, OK)
            }
            Op::PartySeed { dst } => {
                self.put(dst, NVal::P(Party::seed()));
                done(8, OK)
            }
            Op::ClockTick { c } => {
                let denom = self.clock(c).ok_or_else(malformed)?.encoded_bits() as u64;
                match self.regs.get_mut(c as usize) {
                    Some(Some(NVal::C(clock))) => {
                        clock.tick();
                        done(denom, OK)
                    }
                    _ => Err(malformed()),
                }
            }
            Op::ClockSend { c } => {
                let denom = self.clock(c).ok_or_else(malformed)?.encoded_bits() as u64;
                match self.regs.get_mut(c as usize) {
                    Some(Some(NVal::C(clock))) => {
                        clock.send();
                        done(denom, OK)
                    }
                    _ => Err(malformed()),
                }
            }
            Op::ClockFork { dst, src } => {
                let denom = self.clock(src).ok_or_else(malformed)?.encoded_bits() as u64;
                let forked = match self.regs.get_mut(src as usize) {
                    Some(Some(NVal::C(clock))) => clock.fork(),
                    _ => return Err(malformed()),
                };
                self.put(dst, NVal::C(forked));
                done(denom, OK)
            }
            Op::ClockJoin { a, b } => {
                let denom = (self.clock(a).ok_or_else(malformed)?.encoded_bits()
                    + self.clock(b).ok_or_else(malformed)?.encoded_bits())
                    as u64;
                let Some(NVal::C(cb)) = self.take(b) else {
                    return Err(malformed());
                };
                match self.regs.get_mut(a as usize) {
                    Some(Some(NVal::C(ca))) => match ca.join(cb) {
                        Ok(_) => done(denom, OK),
                        Err(rejected) => {
                            self.put(b, NVal::C(rejected));
                            done(denom, ERR_OP)
                        }
                    },
                    _ => Err(malformed()),
                }
            }
            Op::ClockRecv { c, v } => {
                let denom = (self.clock(c).ok_or_else(malformed)?.encoded_bits()
                    + self.version(v).ok_or_else(malformed)?.encoded_bits())
                    as u64;
                let version = self.version(v).ok_or_else(malformed)?.clone();
                match self.regs.get_mut(c as usize) {
                    Some(Some(NVal::C(clock))) => {
                        clock.recv(&version);
                        done(denom, OK)
                    }
                    _ => Err(malformed()),
                }
            }
            Op::ClockSync { a, b } => {
                let denom = (self.clock(a).ok_or_else(malformed)?.encoded_bits()
                    + self.clock(b).ok_or_else(malformed)?.encoded_bits())
                    as u64;
                // Two mutable borrows out of one file: take, sync, put back.
                let Some(NVal::C(mut cb)) = self.take(b) else {
                    return Err(malformed());
                };
                let outcome = match self.regs.get_mut(a as usize) {
                    Some(Some(NVal::C(ca))) => ca.sync(&mut cb).map(|_| ()),
                    _ => return Err(malformed()),
                };
                self.put(b, NVal::C(cb));
                done(denom, if outcome.is_ok() { OK } else { ERR_OP })
            }
            Op::ClockOwnVersion { dst, src } => {
                let clock = self.clock(src).ok_or_else(malformed)?;
                let input = clock.encoded_bits() as u64;
                let own = clock.own_version();
                // Output-dominated row: input + packed output, output read
                // from the actual result.
                let denom = input + own.encoded_bits() as u64;
                self.put(dst, NVal::V(own));
                done(denom, OK)
            }
            Op::ClockVersion { dst, src } => {
                let clock = self.clock(src).ok_or_else(malformed)?;
                let version = clock.version().clone();
                let denom = clock.encoded_bits() as u64;
                self.put(dst, NVal::V(version));
                done(denom, OK)
            }
            Op::ClockIntoParts { dst_p, dst_v, src } => {
                let denom = self.clock(src).ok_or_else(malformed)?.encoded_bits() as u64;
                let Some(NVal::C(clock)) = self.take(src) else {
                    return Err(malformed());
                };
                let (party, version) = clock.into_parts();
                self.put(dst_p, NVal::P(party));
                self.put(dst_v, NVal::V(version));
                done(denom, OK)
            }
            Op::ClockFromParts { dst, p, v } => {
                let denom = (self.party(p).ok_or_else(malformed)?.encoded_bits()
                    + self.version(v).ok_or_else(malformed)?.encoded_bits())
                    as u64;
                let Some(NVal::P(party)) = self.take(p) else {
                    return Err(malformed());
                };
                let Some(NVal::V(version)) = self.take(v) else {
                    return Err(malformed());
                };
                self.put(dst, NVal::C(Clock::from_parts(party, version)));
                done(denom, OK)
            }
            Op::ClockEncode { src } => {
                let clock = self.clock(src).ok_or_else(malformed)?;
                let denom = clock.encoded_bits() as u64;
                self.stage = clock.encode();
                done(denom, OK)
            }
            Op::ClockDecode { dst } => {
                let denom = (self.stage.len() as u64) * 8;
                let clock = Clock::decode(self.stage.as_slice()).map_err(|_| malformed())?;
                self.put(dst, NVal::C(clock));
                done(denom, OK)
            }
            Op::VersionTick { v, p } => {
                let denom = (self.version(v).ok_or_else(malformed)?.encoded_bits()
                    + self.party(p).ok_or_else(malformed)?.encoded_bits())
                    as u64;
                let Some(NVal::P(party)) = self.take(p) else {
                    return Err(malformed());
                };
                let ticked = match self.regs.get_mut(v as usize) {
                    Some(Some(NVal::V(version))) => {
                        party.tick(version);
                        true
                    }
                    _ => false,
                };
                self.put(p, NVal::P(party));
                if ticked {
                    done(denom, OK)
                } else {
                    Err(malformed())
                }
            }
            Op::VersionJoin { dst, a, b } => {
                let denom = (self.version(a).ok_or_else(malformed)?.encoded_bits()
                    + self.version(b).ok_or_else(malformed)?.encoded_bits())
                    as u64;
                let Some(NVal::V(va)) = self.take(a) else {
                    return Err(malformed());
                };
                let vb = self.version(b).ok_or_else(malformed)?;
                self.put(dst, NVal::V(va | vb));
                done(denom, OK)
            }
            Op::VersionMeet { dst, a, b } => {
                let denom = (self.version(a).ok_or_else(malformed)?.encoded_bits()
                    + self.version(b).ok_or_else(malformed)?.encoded_bits())
                    as u64;
                let Some(NVal::V(va)) = self.take(a) else {
                    return Err(malformed());
                };
                let vb = self.version(b).ok_or_else(malformed)?;
                self.put(dst, NVal::V(va & vb));
                done(denom, OK)
            }
            Op::VersionProject { dst, v, p } => {
                let version = self.version(v).ok_or_else(malformed)?;
                let party = self.party(p).ok_or_else(malformed)?;
                let input = (version.encoded_bits() + party.encoded_bits()) as u64;
                let projected = version / party;
                // Output-dominated row: input + packed output.
                let denom = input + projected.encoded_bits() as u64;
                self.put(dst, NVal::V(projected));
                done(denom, OK)
            }
            Op::VersionCmp { a, b } => {
                let va = self.version(a).ok_or_else(malformed)?;
                let vb = self.version(b).ok_or_else(malformed)?;
                let denom = (va.encoded_bits() + vb.encoded_bits()) as u64;
                let expect = match va.partial_cmp(vb) {
                    Some(Ordering::Less) => 0,
                    Some(Ordering::Equal) => 1,
                    Some(Ordering::Greater) => 2,
                    None => 3,
                };
                done(denom, expect)
            }
            Op::VersionConcurrent { a, b } => {
                let va = self.version(a).ok_or_else(malformed)?;
                let vb = self.version(b).ok_or_else(malformed)?;
                let denom = (va.encoded_bits() + vb.encoded_bits()) as u64;
                done(denom, i64::from(va.concurrent(vb)))
            }
            Op::VersionRank { dst, src } => {
                let version = self.version(src).ok_or_else(malformed)?;
                let denom = version.encoded_bits() as u64;
                let rank = version.rank();
                self.put(dst, NVal::R(rank));
                done(denom, OK)
            }
            Op::VersionDistance { dst, a, b } => {
                let va = self.version(a).ok_or_else(malformed)?;
                let vb = self.version(b).ok_or_else(malformed)?;
                let denom = (va.encoded_bits() + vb.encoded_bits()) as u64;
                let rank = va.distance(vb);
                self.put(dst, NVal::R(rank));
                done(denom, OK)
            }
            Op::VersionLag { dst, a, b } => {
                let va = self.version(a).ok_or_else(malformed)?;
                let vb = self.version(b).ok_or_else(malformed)?;
                let denom = (va.encoded_bits() + vb.encoded_bits()) as u64;
                let rank = va.lag(vb);
                self.put(dst, NVal::R(rank));
                done(denom, OK)
            }
            Op::VersionMinTicks { src } => {
                let version = self.version(src).ok_or_else(malformed)?;
                let denom = version.encoded_bits() as u64;
                done(denom, version.min_ticks() as i64)
            }
            Op::VersionJoinAll { dst, src, n } => {
                let mut denom = 0u64;
                let mut operands = Vec::with_capacity(n as usize);
                for i in 0..n {
                    let v = match self.take(src + i) {
                        Some(NVal::V(v)) => v,
                        _ => return Err(malformed()),
                    };
                    denom += v.encoded_bits() as u64;
                    operands.push(v);
                }
                self.put(dst, NVal::V(Version::join_all(operands)));
                done(denom, OK)
            }
            Op::VersionMeetAll { dst, src, n } => {
                let mut denom = 0u64;
                let mut operands = Vec::with_capacity(n as usize);
                for i in 0..n {
                    let v = match self.take(src + i) {
                        Some(NVal::V(v)) => v,
                        _ => return Err(malformed()),
                    };
                    denom += v.encoded_bits() as u64;
                    operands.push(v);
                }
                match Version::meet_all(operands) {
                    Some(met) => {
                        self.put(dst, NVal::V(met));
                        done(denom, OK)
                    }
                    None => done(denom, ERR_OP),
                }
            }
            Op::VersionEncode { src } => {
                let version = self.version(src).ok_or_else(malformed)?;
                let denom = version.encoded_bits() as u64;
                self.stage = version.encode();
                done(denom, OK)
            }
            Op::VersionDecode { dst } => {
                let denom = (self.stage.len() as u64) * 8;
                let version = Version::decode(self.stage.as_slice()).map_err(|_| malformed())?;
                self.put(dst, NVal::V(version));
                done(denom, OK)
            }
            Op::VersionDisplay { src } => {
                let version = self.version(src).ok_or_else(malformed)?;
                let input = version.encoded_bits() as u64;
                let text = version.to_string();
                // Text I/O: packed input + text output, output read from
                // the actual result.
                let denom = input + (text.len() as u64) * 8;
                self.stage = text.into_bytes();
                done(denom, OK)
            }
            Op::VersionFromstr { dst } => {
                let text_bits = (self.stage.len() as u64) * 8;
                let text = std::str::from_utf8(&self.stage).map_err(|_| malformed())?;
                let version: Version = text.parse().map_err(|_| malformed())?;
                // Text I/O: text input + packed output.
                let denom = text_bits + version.encoded_bits() as u64;
                self.put(dst, NVal::V(version));
                done(denom, OK)
            }
            Op::PartyFork { dst, src } => {
                let denom = self.party(src).ok_or_else(malformed)?.encoded_bits() as u64;
                let forked = match self.regs.get_mut(src as usize) {
                    Some(Some(NVal::P(party))) => party.fork(),
                    _ => return Err(malformed()),
                };
                self.put(dst, NVal::P(forked));
                done(denom, OK)
            }
            Op::PartyForks { dst, src, n } => {
                let input = self.party(src).ok_or_else(malformed)?.encoded_bits() as u64;
                let shares = match self.regs.get_mut(src as usize) {
                    Some(Some(NVal::P(party))) => party.forks(n as usize).collect::<Vec<_>>(),
                    _ => return Err(malformed()),
                };
                // Share splitting: the output is n packed parties.
                let denom = input + shares.iter().map(|s| s.encoded_bits() as u64).sum::<u64>();
                for (i, share) in shares.into_iter().enumerate() {
                    self.put(dst + i as u32, NVal::P(share));
                }
                done(denom, OK)
            }
            Op::PartyJoin { a, b } => {
                let denom = (self.party(a).ok_or_else(malformed)?.encoded_bits()
                    + self.party(b).ok_or_else(malformed)?.encoded_bits())
                    as u64;
                let Some(NVal::P(pb)) = self.take(b) else {
                    return Err(malformed());
                };
                match self.regs.get_mut(a as usize) {
                    Some(Some(NVal::P(pa))) => match pa.join(pb) {
                        Ok(()) => done(denom, OK),
                        Err(rejected) => {
                            self.put(b, NVal::P(rejected));
                            done(denom, ERR_OP)
                        }
                    },
                    _ => Err(malformed()),
                }
            }
            Op::PartyIsDisjoint { a, b } => {
                let pa = self.party(a).ok_or_else(malformed)?;
                let pb = self.party(b).ok_or_else(malformed)?;
                let denom = (pa.encoded_bits() + pb.encoded_bits()) as u64;
                done(denom, i64::from(pa.is_disjoint(pb)))
            }
            Op::PartyCovers { a, b } => {
                let pa = self.party(a).ok_or_else(malformed)?;
                let pb = self.party(b).ok_or_else(malformed)?;
                let denom = (pa.encoded_bits() + pb.encoded_bits()) as u64;
                done(denom, i64::from(pa.covers(pb)))
            }
            Op::PartyWithout { dst, a, b } => {
                let denom = (self.party(a).ok_or_else(malformed)?.encoded_bits()
                    + self.party(b).ok_or_else(malformed)?.encoded_bits())
                    as u64;
                let Some(NVal::P(pa)) = self.take(a) else {
                    return Err(malformed());
                };
                let pb = self.party(b).ok_or_else(malformed)?;
                match pa.without(pb) {
                    Some(diff) => {
                        self.put(dst, NVal::P(diff));
                        done(denom, OK)
                    }
                    None => done(denom, ERR_OP),
                }
            }
            Op::PartyEncode { src } => {
                let party = self.party(src).ok_or_else(malformed)?;
                let denom = party.encoded_bits() as u64;
                self.stage = party.encode();
                done(denom, OK)
            }
            Op::PartyDecode { dst } => {
                let denom = (self.stage.len() as u64) * 8;
                let party = Party::decode(self.stage.as_slice()).map_err(|_| malformed())?;
                self.put(dst, NVal::P(party));
                done(denom, OK)
            }
            Op::PartyDisplay { src } => {
                let party = self.party(src).ok_or_else(malformed)?;
                let input = party.encoded_bits() as u64;
                let text = party.to_string();
                let denom = input + (text.len() as u64) * 8;
                self.stage = text.into_bytes();
                done(denom, OK)
            }
            Op::PartyFromstr { dst } => {
                let text_bits = (self.stage.len() as u64) * 8;
                let text = std::str::from_utf8(&self.stage).map_err(|_| malformed())?;
                let party: Party = text.parse().map_err(|_| malformed())?;
                let denom = text_bits + party.encoded_bits() as u64;
                self.put(dst, NVal::P(party));
                done(denom, OK)
            }
            Op::RankAdd { dst, a, b } => {
                let denom = Self::rank_bits(self.rank(a).ok_or_else(malformed)?)
                    + Self::rank_bits(self.rank(b).ok_or_else(malformed)?);
                let Some(NVal::R(ra)) = self.take(a) else {
                    return Err(malformed());
                };
                let rb = self.rank(b).ok_or_else(malformed)?;
                self.put(dst, NVal::R(ra + rb));
                done(denom, OK)
            }
            Op::RankCmp { a, b } => {
                let ra = self.rank(a).ok_or_else(malformed)?;
                let rb = self.rank(b).ok_or_else(malformed)?;
                let denom = Self::rank_bits(ra) + Self::rank_bits(rb);
                let expect = match ra.cmp(rb) {
                    Ordering::Less => 0,
                    Ordering::Equal => 1,
                    Ordering::Greater => 2,
                };
                done(denom, expect)
            }
            Op::RankCheckedSub { dst, a, b } => {
                let ra = self.rank(a).ok_or_else(malformed)?;
                let rb = self.rank(b).ok_or_else(malformed)?;
                let denom = Self::rank_bits(ra) + Self::rank_bits(rb);
                match ra.checked_sub(rb) {
                    Some(diff) => {
                        self.put(dst, NVal::R(diff));
                        done(denom, OK)
                    }
                    None => done(denom, ERR_OP),
                }
            }
        }
    }
}
