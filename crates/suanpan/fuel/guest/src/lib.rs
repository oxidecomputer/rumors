//! Runs `suanpan` programs over a pool of accumulators inside a wasm guest,
//! reading Wasmtime fuel after every step.
//!
//! The harness (the `suanpan-fuel-harness` crate) writes an encoded
//! [`Program`] into the buffer [`input_buffer`] returns and calls [`run`],
//! once per fresh instance. The guest decodes the program and expands its
//! limb operands, then opens one measured window and runs every repetition
//! of the steps inside it, reading the instance's consumed fuel once before
//! the first step and once after each step. The readings, and one
//! observation word per step (`suanpan_fuel_protocol::observe_*`), are
//! left for [`output_ptr`] and [`output_len`].
//!
//! Everything the window does not need is prepared before it opens: the
//! decoded steps, the limb operands, the pool, and the output vectors, whose
//! capacity covers every reading, so recording a reading allocates nothing.
//! Inside the window, fuel counts the steps, the loop that dispatches them,
//! the observation of each answer, and one host call per reading: a fixed
//! cost per step.
//!
//! The global allocator is the fuel board's window allocator, with every
//! reallocation made to move its block (the `moving` module). The window runs on a
//! fresh, primed heap, so a reading does not depend on what decoding and
//! expansion left in the heap; and no block grows in place, so a reading does
//! not depend on where earlier blocks landed. The window allocator's module
//! and the panic record are the board guest's own source files, included by
//! path rather than through a shared crate: moving them into a crate would
//! change the board guest's compiled code, and with it the fuel its ceilings
//! were calibrated against. Their prose speaks of the board's cells; here
//! the one measured window is a whole program run.

mod moving;
#[path = "../../../../before/board-fuel/guest/src/panic_record.rs"]
mod panic_record;
#[path = "../../../../before/board-fuel/guest/src/window.rs"]
mod window;

use std::hash::{DefaultHasher, Hash, Hasher};
use std::hint::black_box;
use std::mem;
use std::sync::{Mutex, MutexGuard, PoisonError};

use suanpan::Accumulator;
use suanpan_fuel_protocol::{
    observe_bool, observe_conversion, observe_magnitude, observe_ordering, observe_stable,
    Direction, Form, Member, Primitive, Program, Read, SignQuery, Step, NO_OBSERVATION, POOL,
};

#[global_allocator]
static ALLOCATOR: moving::MovingAllocator = moving::MovingAllocator;

// A wasm import can only be declared in an `unsafe extern` block, and fuel
// can only be read through an import. The block declares one function, which
// takes no arguments, touches no guest memory, and returns an integer, so it
// is declared `safe` and no call site is `unsafe`. It lives in this tool-side
// guest alone, which no production crate links; the guest's only other
// `unsafe` code is its global allocator, in `window` and `moving`.
#[link(wasm_import_module = "suanpan_fuel")]
unsafe extern "C" {
    /// Returns the fuel this instance has consumed so far, read by the host
    /// from the store.
    ///
    /// Safe to call: the host function takes no arguments, touches no guest
    /// memory, and returns a plain integer.
    safe fn fuel_used() -> u64;
}

/// The encoded program the host writes before calling [`run`].
static INPUT: Mutex<Vec<u8>> = Mutex::new(Vec::new());

/// The readings and observations [`run`] leaves for the host.
static OUTPUT: Mutex<Vec<u64>> = Mutex::new(Vec::new());

/// Locks one of the guest's buffers.
///
/// A panic aborts this guest without unwinding, so no guard is dropped
/// mid-panic and the lock is never poisoned; recovering the inner buffer
/// keeps the exports total regardless.
fn lock<T>(buffer: &'static Mutex<Vec<T>>) -> MutexGuard<'static, Vec<T>> {
    buffer.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Converts a wasm32 address or length to the `u32` the host reads.
fn wire(value: usize) -> u32 {
    u32::try_from(value).expect("a wasm32 address or length fits in u32")
}

/// Makes the input buffer `bytes` bytes long, zeroed, and returns its
/// linear-memory address, for the host to write an encoded program into.
#[no_mangle]
pub extern "C" fn input_buffer(bytes: u32) -> u32 {
    let mut input = lock(&INPUT);
    *input = vec![0; usize::try_from(bytes).expect("a u32 fits in wasm32's usize")];
    wire(input.as_ptr().addr())
}

/// Runs the program in the input buffer, leaving its readings and
/// observations in the output.
///
/// Returns 0, or 1 if the input is not an encoded program. A panic in the
/// code under test traps instead, with its message in the panic record.
///
/// The output holds `1 + n` fuel readings, where `n` is the number of steps
/// times the number of repetitions: the instance's consumed fuel before the
/// first step and after each step. Then follow the `n` observation words, in
/// step order.
#[no_mangle]
pub extern "C" fn run() -> u32 {
    panic_record::install();
    let bytes = mem::take(&mut *lock(&INPUT));
    let Ok(program) = Program::decode(&bytes) else {
        return 1;
    };
    let output = execute(&program);
    *lock(&OUTPUT) = output;
    0
}

/// Returns the linear-memory address of the output's first word.
#[no_mangle]
pub extern "C" fn output_ptr() -> u32 {
    wire(lock(&OUTPUT).as_ptr().addr())
}

/// Returns the output's length in words.
#[no_mangle]
pub extern "C" fn output_len() -> u32 {
    wire(lock(&OUTPUT).len())
}

/// A step ready to run: the step, and its limb operand expanded.
struct Prepared {
    /// The step.
    step: Step,
    /// The limbs of a [`Step::Limbs`] operand, lowest first; empty for every
    /// other step.
    limbs: Vec<u64>,
}

/// Runs `program` in one measured window and returns its output, laid out as
/// [`run`] describes.
fn execute(program: &Program) -> Vec<u64> {
    let steps: Vec<Prepared> = program
        .steps
        .iter()
        .map(|step| Prepared {
            limbs: match step {
                Step::Limbs { input, .. } => input.expand(),
                _ => Vec::new(),
            },
            step: step.clone(),
        })
        .collect();
    let repetitions = usize::try_from(program.repetitions).expect("a u32 fits in wasm32's usize");
    let count = steps
        .len()
        .checked_mul(repetitions)
        .expect("a program's step count fits in wasm32's usize");
    let mut readings = Vec::with_capacity(count + 1);
    let mut observations = Vec::with_capacity(count);
    let mut pool: [Accumulator; POOL] = Default::default();

    window::enter();
    readings.push(fuel_used());
    for _ in 0..repetitions {
        for step in &steps {
            observations.push(apply(&mut pool, step));
            readings.push(fuel_used());
        }
    }
    window::exit();

    readings.extend_from_slice(&observations);
    readings
}

/// Runs one step on the pool and returns its observation word.
fn apply(pool: &mut [Accumulator; POOL], prepared: &Prepared) -> u64 {
    match &prepared.step {
        Step::Primitive {
            member,
            direction,
            value,
        } => {
            apply_primitive(&mut pool[member.index()], *direction, *value);
            NO_OBSERVATION
        }
        Step::Limbs {
            member,
            direction,
            shift,
            ..
        } => {
            let value = &mut pool[member.index()];
            let limbs = prepared.limbs.iter().copied();
            match direction {
                Direction::Add => value.add_shifted_limbs(*shift, limbs),
                Direction::Sub => value.sub_shifted_limbs(*shift, limbs),
            }
            NO_OBSERVATION
        }
        Step::Operand {
            receiver,
            operand,
            direction,
            form,
        } => {
            apply_operand(pool, *receiver, *operand, *direction, *form);
            NO_OBSERVATION
        }
        Step::Negate { member, borrowed } => {
            let value = &mut pool[member.index()];
            *value = if *borrowed {
                -&*value
            } else {
                -mem::take(value)
            };
            NO_OBSERVATION
        }
        Step::Shift { member, shift } => {
            pool[member.index()] <<= *shift;
            NO_OBSERVATION
        }
        Step::Clear { member } => {
            pool[member.index()].clear();
            NO_OBSERVATION
        }
        Step::Sign { member, query } => {
            let value = &pool[member.index()];
            match *query {
                SignQuery::Sign => observe_ordering(value.sign()),
                SignQuery::IsZero => observe_bool(value.is_zero()),
                SignQuery::IsPositive => observe_bool(value.is_positive()),
                SignQuery::IsNegative => observe_bool(value.is_negative()),
                SignQuery::StableUnder(bits) => observe_stable(value.cmp_zero_stable_under(bits)),
            }
        }
        Step::ComparePrimitive { member, value } => observe_ordering(
            pool[member.index()]
                .partial_cmp(value)
                .expect("an accumulator and a primitive are always ordered"),
        ),
        Step::Convert { member } => {
            let slot = &mut pool[member.index()];
            match i64::try_from(mem::take(slot)) {
                Ok(converted) => {
                    *slot = Accumulator::from(converted);
                    observe_conversion(Some(converted))
                }
                Err(value) => {
                    *slot = value;
                    observe_conversion(None)
                }
            }
        }
        Step::Compare { left, right } => {
            observe_ordering(pool[left.index()].cmp(&pool[right.index()]))
        }
        Step::Read { member, read } => {
            let value = &pool[member.index()];
            match read {
                Read::Signed => {
                    let (sign, limbs) = value.signed_magnitude();
                    let low = limbs.first().copied().unwrap_or(0);
                    observe_magnitude(sign, bit_length(&limbs, 0), low)
                }
                Read::Scaled => {
                    let (sign, limbs, shift) = value.scaled_signed_magnitude();
                    observe_magnitude(sign, bit_length(&limbs, shift), 0)
                }
                Read::Hash => {
                    let mut hasher = DefaultHasher::new();
                    value.hash(&mut hasher);
                    black_box(hasher.finish());
                    NO_OBSERVATION
                }
            }
        }
        Step::Clone { receiver, source } => {
            let copy = pool[source.index()].clone();
            pool[receiver.index()] = copy;
            NO_OBSERVATION
        }
        Step::CloneFrom { receiver, source } => {
            if receiver == source {
                let value = &mut pool[receiver.index()];
                *value = value.clone();
            } else {
                let (target, source) = two_members(pool, *receiver, *source);
                target.clone_from(source);
            }
            NO_OBSERVATION
        }
        Step::AbsDiff {
            receiver,
            operand,
            take,
        } => {
            let other = if *take && receiver != operand {
                mem::take(&mut pool[operand.index()])
            } else {
                pool[operand.index()].clone()
            };
            let value = mem::take(&mut pool[receiver.index()]);
            let (order, difference) = value.abs_diff(other);
            pool[receiver.index()] = difference;
            observe_ordering(order)
        }
        Step::Sum {
            receiver,
            members,
            owned,
        } => {
            let sum: Accumulator = if *owned {
                members
                    .iter()
                    .map(|member| pool[member.index()].clone())
                    .sum()
            } else {
                members.iter().map(|member| &pool[member.index()]).sum()
            };
            pool[receiver.index()] = sum;
            NO_OBSERVATION
        }
    }
}

/// Returns the bit length of `limbs · 2^shift`, for minimal `limbs`.
fn bit_length(limbs: &[u64], shift: u64) -> u64 {
    match limbs.last() {
        None => 0,
        Some(top) => shift + 64 * (limbs.len() as u64 - 1) + u64::from(64 - top.leading_zeros()),
    }
}

/// Adds or subtracts a primitive operand.
fn apply_primitive(value: &mut Accumulator, direction: Direction, operand: Primitive) {
    match (direction, operand) {
        (Direction::Add, Primitive::I64(v)) => *value += v,
        (Direction::Sub, Primitive::I64(v)) => *value -= v,
        (Direction::Add, Primitive::U64(v)) => *value += v,
        (Direction::Sub, Primitive::U64(v)) => *value -= v,
        (Direction::Add, Primitive::I128(v)) => *value += v,
        (Direction::Sub, Primitive::I128(v)) => *value -= v,
        (Direction::Add, Primitive::U128(v)) => *value += v,
        (Direction::Sub, Primitive::U128(v)) => *value -= v,
    }
}

/// Borrows two different members mutably.
///
/// # Panics
///
/// Panics if `first == second`.
fn two_members(
    pool: &mut [Accumulator; POOL],
    first: Member,
    second: Member,
) -> (&mut Accumulator, &mut Accumulator) {
    let (first, second) = (first.index(), second.index());
    assert_ne!(first, second, "two members must differ to borrow both");
    let (low, high) = pool.split_at_mut(first.max(second));
    if first < second {
        (&mut low[first], &mut high[0])
    } else {
        (&mut high[0], &mut low[second])
    }
}

/// Adds or subtracts `operand` into `receiver` in one [`Form`].
fn apply_operand(
    pool: &mut [Accumulator; POOL],
    receiver: Member,
    operand: Member,
    direction: Direction,
    form: Form,
) {
    if receiver == operand {
        let value = &mut pool[receiver.index()];
        if form == Form::Borrowed {
            #[allow(
                clippy::eq_op,
                reason = "one accumulator as both operands is the case under test"
            )]
            let result = match direction {
                Direction::Add => &*value + &*value,
                Direction::Sub => &*value - &*value,
            };
            *value = result;
            return;
        }
        // Every other form borrows the receiver mutably or consumes it, so
        // the member lends a clone of itself.
        let copy = value.clone();
        apply_borrowed(value, &copy, direction, form);
        return;
    }
    let (target, source) = two_members(pool, receiver, operand);
    if form == Form::OwnedTake {
        apply_owned(target, mem::take(source), direction);
    } else {
        apply_borrowed(target, source, direction, form);
    }
}

/// Applies `operand` to `receiver` in `form`, leaving `operand` with its
/// owner; the owned forms consume a clone of it.
fn apply_borrowed(
    receiver: &mut Accumulator,
    operand: &Accumulator,
    direction: Direction,
    form: Form,
) {
    match (form, direction) {
        (Form::Borrowed, Direction::Add) => *receiver += operand,
        (Form::Borrowed, Direction::Sub) => *receiver -= operand,
        (Form::Shifted(shift), Direction::Add) => receiver.add_shifted(shift, operand),
        (Form::Shifted(shift), Direction::Sub) => receiver.sub_shifted(shift, operand),
        (Form::OwnedClone | Form::OwnedTake, _) => {
            apply_owned(receiver, operand.clone(), direction);
        }
        (Form::Reversed, _) => {
            let value = mem::take(receiver);
            *receiver = match direction {
                Direction::Add => operand + value,
                Direction::Sub => operand - value,
            };
        }
    }
}

/// Replaces `receiver` by `receiver ± operand`, consuming both.
fn apply_owned(receiver: &mut Accumulator, operand: Accumulator, direction: Direction) {
    let value = mem::take(receiver);
    *receiver = match direction {
        Direction::Add => value + operand,
        Direction::Sub => value - operand,
    };
}
