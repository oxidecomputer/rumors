//! The task set's failure routing, checked against a model of its rules.
//!
//! Each scripted member completes at a scripted *step*, with a scripted
//! exit. Members completing at one step complete in an order the model
//! leaves open. Each step is reached in one of two ways, also scripted:
//!
//! - *within* the session: the last member to complete at the step before
//!   advances the clock while the set polls it, so the set notices the
//!   wakes, as it notices a member handing work to another;
//! - from *outside*: the harness advances the clock only once the session
//!   is at rest, polled without being woken, as an event arriving from the
//!   peer would.
//!
//! In a session, the local participant's walk runs beside the set, so work
//! that follows within the session can look to the set like work that waits
//! for an outside event. The rules let that difference change which failure
//! a failed session reports, never whether a session succeeds once a pipe
//! failure has been reported, and the harness checks both by scripting each
//! step either way.

use std::{
    collections::BTreeMap,
    io, mem,
    pin::{Pin, pin},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll, Wake, Waker},
};

use futures::future::BoxFuture;
use proptest::prelude::*;

use super::drive;
use crate::tree::mirror::streaming::remote::{
    adapter::EncodeError,
    codec::{self, DecodeError, DecodeErrorKind, EncodeErrorKind, FramePart, Origin, Speaker},
    pipe::PipeError,
    proxy::Error,
};

/// The step of a member that never completes.
const NEVER: usize = usize::MAX;

/// The last step at which a scripted member completes.
const LAST_STEP: usize = 3;

/// The most walk tasks a scripted session has.
const MAX_TASKS: usize = 4;

/// The terminal future's output when it succeeds.
const FINISHED: u32 = 42;

/// The polls after which a scripted session is judged to spin.
const POLL_LIMIT: usize = 10_000;

/// How a scripted walk task, terminal future or multiplexer completes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Exit {
    /// Successfully.
    Ok,
    /// With a walk error.
    Walk,
    /// With a write failure.
    Write,
}

/// How the scripted demultiplexer fails.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Receive {
    /// The peer departed.
    Departed,
    /// A frame answered no question: the peer broke the protocol.
    Violation,
    /// The pipe ended inside a frame: not a violation.
    Truncated,
}

/// When a scripted member completes, and how.
#[derive(Clone, Copy, Debug)]
struct Scripted<X> {
    /// The step at which it completes, or [`NEVER`].
    step: usize,
    /// How it completes.
    exit: X,
}

/// A scripted session: its walk tasks, terminal future, multiplexer and
/// demultiplexer, and how each step is reached.
///
/// A walk member's errors name the member: walk task `i` is member `i`,
/// the terminal future follows the tasks, and the multiplexer follows it.
#[derive(Clone, Debug)]
struct Script {
    /// Whether each step, by number, follows within the session from the
    /// scripted step before it, rather than from outside.
    ///
    /// The first scripted step follows no other, so the session starts at
    /// step 0 and the harness reaches a later first step from outside.
    within: [bool; LAST_STEP + 1],
    /// The walk tasks.
    tasks: Vec<Scripted<Exit>>,
    /// The terminal future.
    finish: Scripted<Exit>,
    /// The multiplexer, which never reports a walk error.
    send: Scripted<Exit>,
    /// The demultiplexer.
    receive: Scripted<Receive>,
}

impl Script {
    /// The walk tasks, the terminal future and the multiplexer, each with
    /// the member number its errors carry.
    fn walk(&self) -> impl Iterator<Item = (u8, Scripted<Exit>)> + '_ {
        self.tasks
            .iter()
            .copied()
            .chain([self.finish, self.send])
            .enumerate()
            .map(|(member, scripted)| (u8::try_from(member).expect("few members"), scripted))
    }

    /// The scripted steps at which some member completes, in order.
    fn steps(&self) -> Vec<usize> {
        let mut steps: Vec<usize> = self
            .walk()
            .map(|(_, scripted)| scripted.step)
            .chain([self.receive.step])
            .filter(|&step| step != NEVER)
            .collect();
        steps.sort_unstable();
        steps.dedup();
        steps
    }

    /// The step at which the walk completes, if every walk member succeeds.
    fn completion(&self) -> Option<usize> {
        self.walk()
            .map(|(_, scripted)| {
                (scripted.exit == Exit::Ok && scripted.step != NEVER).then_some(scripted.step)
            })
            .try_fold(0, |last, step| Some(last.max(step?)))
    }

    /// The first step at which a pipe member reports a failure, if any does.
    fn first_pipe_failure(&self) -> Option<usize> {
        self.walk()
            .filter(|(_, scripted)| scripted.exit == Exit::Write)
            .map(|(_, scripted)| scripted.step)
            .chain([self.receive.step])
            .filter(|&step| step != NEVER)
            .min()
    }
}

/// How a session ended, comparable across the driver and the model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Seen {
    /// It succeeded.
    Completed,
    /// It failed with member `0`'s walk error.
    Walk(u8),
    /// It failed with member `0`'s write failure.
    Write(u8),
    /// It failed with the demultiplexer's failure.
    Received(Receive),
    /// It came to rest with nothing left to happen.
    Pending,
}

/// The outcomes the rules allow for `script`.
///
/// Steps run in order. At each, a walk error fails the session (rule 1).
/// Otherwise a completed walk decides it (rule 3): it succeeds unless the
/// demultiplexer has failed, at this step or at the next, if the step's last
/// completion reaches the next from within. Otherwise, once a pipe failure
/// is known, the session continues into a next step that follows within
/// the session, and comes to rest before one that follows from outside, or
/// after the last; at rest it fails with the highest-ranked pipe failure
/// (rule 2). Where several members could decide within one step, each is
/// allowed.
fn model(script: &Script) -> Vec<Seen> {
    let steps = script.steps();
    let mut walking = script.tasks.len() + 2;
    let mut written: Option<Vec<u8>> = None;
    let mut received = None;
    for (index, &step) in steps.iter().enumerate() {
        let next = steps.get(index + 1).copied();
        let next_within = next.is_some_and(|next| script.within[next]);
        let at_step = || script.walk().filter(move |(_, s)| s.step == step);
        let walk_errors: Vec<Seen> = at_step()
            .filter(|(_, s)| s.exit == Exit::Walk)
            .map(|(member, _)| Seen::Walk(member))
            .collect();
        if !walk_errors.is_empty() {
            return walk_errors;
        }
        let writes: Vec<u8> = at_step()
            .filter(|(_, s)| s.exit == Exit::Write)
            .map(|(member, _)| member)
            .collect();
        if written.is_none() && !writes.is_empty() {
            written = Some(writes);
        }
        walking -= at_step().filter(|(_, s)| s.exit == Exit::Ok).count();
        if script.receive.step == step {
            received = Some(script.receive.exit);
        }
        if walking == 0 {
            // Only the demultiplexer can still complete, so the step's last
            // completion is the walk's, and it wakes the demultiplexer if
            // the next step is the demultiplexer's and follows within.
            let woken =
                (next_within && next == Some(script.receive.step)).then_some(script.receive.exit);
            return vec![match received.or(woken) {
                None => Seen::Completed,
                Some(failure) => Seen::Received(failure),
            }];
        }
        if next_within {
            continue;
        }
        match (received, &written) {
            (Some(Receive::Violation), _) => return vec![Seen::Received(Receive::Violation)],
            (_, Some(writes)) => return writes.iter().map(|&member| Seen::Write(member)).collect(),
            (Some(failure), None) => return vec![Seen::Received(failure)],
            (None, None) => {}
        }
    }
    vec![Seen::Pending]
}

/// The scripted session's current step, the members waiting for later
/// ones, and how each step is reached.
struct Clock {
    /// The current step.
    step: usize,
    /// Each waiting member's step and waker.
    waiting: Vec<(usize, Waker)>,
    /// How many members scripted for each step have yet to complete.
    remaining: BTreeMap<usize, usize>,
    /// Whether each step follows within the session ([`Script::within`]).
    within: [bool; LAST_STEP + 1],
}

impl Clock {
    /// A clock at step 0 for `script`'s members.
    fn new(script: &Script) -> Self {
        let mut remaining = BTreeMap::new();
        let members = script.walk().map(|(_, scripted)| scripted.step);
        for step in members
            .chain([script.receive.step])
            .filter(|&step| step != NEVER)
        {
            *remaining.entry(step).or_default() += 1;
        }
        Self {
            step: 0,
            waiting: Vec::new(),
            remaining,
            within: script.within,
        }
    }

    /// Note that a member scripted for `step` completed. If it was the
    /// step's last and the next scripted step follows within the session,
    /// advance to that step, returning the wakers of the members it makes
    /// ready.
    fn complete(&mut self, step: usize) -> Vec<Waker> {
        let left = self
            .remaining
            .get_mut(&step)
            .expect("a completing member is counted at its step");
        *left -= 1;
        if *left > 0 {
            return Vec::new();
        }
        self.remaining.remove(&step);
        match self.remaining.range(step + 1..).next() {
            Some((&next, _)) if self.within[next] => self.advance(next),
            _ => Vec::new(),
        }
    }

    /// Advance to step `next`, returning the wakers of the members due by
    /// then.
    fn advance(&mut self, next: usize) -> Vec<Waker> {
        self.step = next;
        let (due, waiting) = mem::take(&mut self.waiting)
            .into_iter()
            .partition(|&(step, _)| step <= next);
        self.waiting = waiting;
        due.into_iter().map(|(_, waker)| waker).collect()
    }
}

/// A future that completes with its output once the clock reaches its
/// step.
struct At<T> {
    /// The step at which it completes.
    step: usize,
    /// Its output, until it completes.
    output: Option<T>,
    /// The session's clock.
    clock: Arc<Mutex<Clock>>,
}

/// Complete at the scripted step, registering to be woken until then.
impl<T: Unpin> Future for At<T> {
    type Output = T;

    /// Completes if the clock has reached the step, waking the next step's
    /// members if this completion reaches it within the session; otherwise
    /// waits for the step.
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        let this = self.get_mut();
        let mut clock = this
            .clock
            .lock()
            .expect("no test thread panics holding the clock");
        if clock.step < this.step {
            clock.waiting.push((this.step, cx.waker().clone()));
            return Poll::Pending;
        }
        let woken = clock.complete(this.step);
        // Wake outside the lock: a waker may poll nothing, but the clock
        // need not rely on that.
        drop(clock);
        woken.into_iter().for_each(Waker::wake);
        Poll::Ready(
            this.output
                .take()
                .expect("a completed member is not polled again"),
        )
    }
}

/// The backend error a scripted walk error carries: the member's number.
#[derive(Debug, thiserror::Error)]
#[error("member {0} failed")]
struct Injected(u8);

/// A future completing with `output` once `clock` reaches `step`.
fn at<T>(clock: &Arc<Mutex<Clock>>, step: usize, output: T) -> At<T> {
    At {
        step,
        output: Some(output),
        clock: Arc::clone(clock),
    }
}

/// A waker that notes whether it was woken.
#[derive(Default)]
struct Flag(AtomicBool);

/// Note the wake.
impl Wake for Flag {
    /// Notes the wake.
    fn wake(self: Arc<Self>) {
        self.0.store(true, Ordering::Relaxed);
    }
}

/// The walk error member `member` reports.
fn walk_error(member: u8) -> Error<Injected> {
    Error::Encode(EncodeError::Backend(Injected(member)))
}

/// The write failure member `member` reports.
fn write_failure(member: u8) -> Error<Injected> {
    Error::Pipe(PipeError::Encode(codec::EncodeError {
        origin: Origin::direction(Speaker::Initiator),
        kind: EncodeErrorKind::Write {
            part: FramePart::FrameHead,
            source: io::Error::other(member.to_string()),
        },
    }))
}

/// A scripted walk member's result.
fn exit_result(member: u8, exit: Exit) -> Result<(), Error<Injected>> {
    match exit {
        Exit::Ok => Ok(()),
        Exit::Walk => Err(walk_error(member)),
        Exit::Write => Err(write_failure(member)),
    }
}

/// The scripted demultiplexer's failure.
fn receive_failure(receive: Receive) -> PipeError {
    match receive {
        Receive::Departed => PipeError::Departed(io::ErrorKind::UnexpectedEof.into()),
        Receive::Violation => PipeError::Unasked {
            origin: Origin::direction(Speaker::Responder),
        },
        Receive::Truncated => PipeError::Decode(DecodeError::direction(
            Speaker::Responder,
            DecodeErrorKind::Truncated {
                missing: FramePart::FrameHead,
                source: io::ErrorKind::UnexpectedEof.into(),
            },
        )),
    }
}

/// Classify the driver's outcome.
fn seen(outcome: Result<u32, Error<Injected>>) -> Seen {
    match outcome {
        Ok(output) => {
            assert_eq!(
                output, FINISHED,
                "the session's output is the terminal future's"
            );
            Seen::Completed
        }
        Err(Error::Encode(EncodeError::Backend(Injected(member)))) => Seen::Walk(member),
        Err(Error::Pipe(PipeError::Encode(codec::EncodeError {
            kind: EncodeErrorKind::Write { source, .. },
            ..
        }))) => Seen::Write(
            source
                .to_string()
                .parse()
                .expect("scripted write failures name a member"),
        ),
        Err(Error::Pipe(PipeError::Departed(_))) => Seen::Received(Receive::Departed),
        Err(Error::Pipe(PipeError::Unasked { .. })) => Seen::Received(Receive::Violation),
        Err(Error::Pipe(PipeError::Decode(_))) => Seen::Received(Receive::Truncated),
        Err(other) => panic!("an unscripted error: {other:?}"),
    }
}

/// Run `script` through the driver: members advance the clock to steps
/// that follow within the session, and the harness advances it to the next
/// step each time the session comes to rest.
fn run(script: &Script) -> Seen {
    let clock = Arc::new(Mutex::new(Clock::new(script)));
    let members: Vec<(u8, Scripted<Exit>)> = script.walk().collect();
    let (tasks, rest) = members.split_at(script.tasks.len());
    let [(finish_member, finish), (send_member, send)] = rest else {
        unreachable!("the walk ends with the terminal future and the multiplexer");
    };
    let tasks: Vec<BoxFuture<'static, Result<(), Error<Injected>>>> = tasks
        .iter()
        .map(|&(member, s)| Box::pin(at(&clock, s.step, exit_result(member, s.exit))) as _)
        .collect();
    let finish = Box::pin(at(
        &clock,
        finish.step,
        exit_result(*finish_member, finish.exit).map(|()| FINISHED),
    ));
    let send = Box::pin(at(&clock, send.step, exit_result(*send_member, send.exit)));
    let receive = Box::pin(at(
        &clock,
        script.receive.step,
        receive_failure(script.receive.exit),
    ));
    let mut session = pin!(drive(tasks, finish, send, receive));
    let flag = Arc::new(Flag::default());
    let waker = Waker::from(Arc::clone(&flag));
    for _ in 0..POLL_LIMIT {
        flag.0.store(false, Ordering::Relaxed);
        if let Poll::Ready(outcome) = session.as_mut().poll(&mut Context::from_waker(&waker)) {
            return seen(outcome);
        }
        if flag.0.load(Ordering::Relaxed) {
            continue;
        }
        // At rest: advance to the next step a member waits for.
        let woken: Vec<Waker> = {
            let mut clock = clock
                .lock()
                .expect("no test thread panics holding the clock");
            match clock.waiting.iter().map(|&(step, _)| step).min() {
                None | Some(NEVER) => return Seen::Pending,
                Some(next) => clock.advance(next),
            }
        };
        woken.into_iter().for_each(Waker::wake);
    }
    panic!("the session spun for {POLL_LIMIT} polls");
}

/// A member's step: usually one of the first few, sometimes never.
fn step() -> impl Strategy<Value = usize> {
    prop_oneof![4 => 0..=LAST_STEP, 1 => Just(NEVER)]
}

/// A walk task's or terminal future's exit: usually success.
fn walk_exit() -> impl Strategy<Value = Exit> {
    prop_oneof![4 => Just(Exit::Ok), 1 => Just(Exit::Walk), 1 => Just(Exit::Write)]
}

/// A scripted member.
fn scripted<X: std::fmt::Debug>(
    exit: impl Strategy<Value = X>,
) -> impl Strategy<Value = Scripted<X>> {
    (step(), exit).prop_map(|(step, exit)| Scripted { step, exit })
}

/// A scripted session.
fn script() -> impl Strategy<Value = Script> {
    (
        any::<[bool; LAST_STEP + 1]>(),
        proptest::collection::vec(scripted(walk_exit()), 0..=MAX_TASKS),
        scripted(walk_exit()),
        scripted(prop_oneof![3 => Just(Exit::Ok), 1 => Just(Exit::Write)]),
        scripted(prop_oneof![
            Just(Receive::Departed),
            Just(Receive::Violation),
            Just(Receive::Truncated),
        ]),
    )
        .prop_map(|(within, tasks, finish, send, receive)| Script {
            within,
            tasks,
            finish,
            send,
            receive,
        })
}

proptest! {
    /// Whatever its members do, whenever, and whether each step follows
    /// within the session or from outside, a session ends as the routing
    /// rules say.
    ///
    /// A walk error ends it at once; a completed walk ends it in success
    /// unless the demultiplexer failed; otherwise, once a pipe failure is
    /// known and the session comes to rest, it fails with a violation over
    /// a write failure over the demultiplexer's other failures. Apart from
    /// the model, a pipe failure reported at or before the step at which
    /// the walk completes never lets the session succeed, however its
    /// steps are reached.
    #[test]
    fn sessions_end_as_the_routing_rules_say(script in script()) {
        let allowed = model(&script);
        let actual = run(&script);
        prop_assert!(allowed.contains(&actual), "{actual:?} is not among {allowed:?}");
        if let (Some(completion), Some(failure)) = (script.completion(), script.first_pipe_failure())
            && failure <= completion
        {
            prop_assert_ne!(
                actual,
                Seen::Completed,
                "a pipe failure at step {} let a walk completing at step {} succeed",
                failure,
                completion
            );
        }
    }
}
