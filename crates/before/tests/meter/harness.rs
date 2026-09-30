//! The deterministic measurement harness and its liveness controls.

use super::*;

// ─── meter liveness canaries ────────────────────────────────────────────────

/// Size of the canary allocation that proves the heap meter is live.
const CANARY_ALLOC_BYTES: usize = 1 << 20;

/// The heap meter registers a known allocation.
///
/// A canary buffer reads back a peak delta at least its own size, so a lost
/// `#[global_allocator]` line or a broken peak reader (either of which
/// would pass every upper-bound envelope vacuously at zero) fails loudly
/// here instead.
#[test]
fn heap_meter_registers_known_allocation() {
    HEAP.reset_peak_usage();
    let baseline = HEAP.current_usage();
    let buf = std::hint::black_box(vec![0u8; CANARY_ALLOC_BYTES]);
    let peak = HEAP.peak_usage().saturating_sub(baseline);
    assert!(
        peak >= CANARY_ALLOC_BYTES,
        "heap meter read {peak} B for a {CANARY_ALLOC_BYTES} B canary allocation: \
         the counting allocator is not measuring this binary"
    );
    drop(buf);
}

/// The dense-spine decode registers at least its encoded input size.
///
/// The decoded version owns a copy of the encoded bits, so the one big
/// scenario here has a floor as well as a ceiling, and a dead heap meter
/// cannot slide a big scenario under its envelope at zero.
#[test]
fn heap_meter_floor_on_decode_dense() {
    let p = Shape::Dense.build1(DENSE_DEPTH);
    HEAP.reset_peak_usage();
    let baseline = HEAP.current_usage();
    let v = version_of(&p);
    let peak = HEAP.peak_usage().saturating_sub(baseline);
    assert!(
        peak >= p.bytes.len(),
        "decode_dense peak {peak} B is under its {} B encoded input: \
         the decoded version alone must allocate at least that",
        p.bytes.len(),
    );
    drop(v);
}

// ─── measurement harness ────────────────────────────────────────────────────

/// Appended to every envelope failure: the first cause to rule out is a
/// shared-process test runner, under which the process-global meters bleed
/// other tests' work into the scenario being measured.
pub const ISOLATION_NOTE: &str = "note: the meters are process-global and meaningful only one \
     scenario per process: run under cargo nextest, not a shared-process cargo test";

/// One counter column of the harness: its MEASURED key, its unit in
/// failure messages, its counter's reset and read, and the row's pin.
#[derive(Clone, Copy)]
struct Column {
    /// The key of the column's `key=reading` fragment on the MEASURED line.
    key: &'static str,
    /// The unit named when the column's ceiling is exceeded.
    unit: &'static str,
    /// Reset the counter before the scenario body.
    reset: fn(),
    /// Read the counter after the body.
    read: fn() -> u64,
    /// The row's pin for this column.
    pin: fn(&Envelope) -> Bound,
    /// The row's pin for this column, writable (the harness self-test's
    /// probe).
    pin_mut: fn(&mut Envelope) -> &mut Bound,
}

/// The counter columns, in MEASURED-line order.
const COLUMNS: [Column; 2] = [
    Column {
        key: "touches",
        unit: "accumulator digit touches",
        reset: suanpan::touch_meter::reset,
        read: suanpan::touch_meter::touches,
        pin: |env| env.touch,
        pin_mut: |env| &mut env.touch,
    },
    Column {
        key: "scan_bits",
        unit: "scanned bits",
        reset: meter::reset_scan_bits,
        read: meter::scan_bits,
        pin: |env| env.scan,
        pin_mut: |env| &mut env.scan,
    },
];

/// Run one scenario body under every meter and assert its envelope.
///
/// Prints the MEASURED line (visible under `--no-capture` or on failure) so
/// re-pinning never requires editing the harness. The scenario's result is
/// returned alive, so the peak includes the fully materialized output, and
/// is dropped by the caller after measurement.
pub fn metered<R>(name: &str, input_bytes: usize, env: &Envelope, f: impl FnOnce() -> R) -> R {
    for column in &COLUMNS {
        (column.reset)();
    }
    HEAP.reset_peak_usage();
    let baseline = HEAP.current_usage();
    let r = f();
    let peak_heap = HEAP.peak_usage().saturating_sub(baseline);
    let readings = COLUMNS.map(|column| (column.read)());
    let mut line = format!("MEASURED {name}: input_bytes={input_bytes} peak_heap={peak_heap}");
    for (column, reading) in COLUMNS.iter().zip(readings) {
        write!(line, " {}={reading}", column.key).expect("a String write cannot fail");
    }
    eprintln!("{line}");
    assert!(
        peak_heap <= env.peak_heap,
        "{name}: peak heap {peak_heap} B exceeds the pinned envelope {} B (input {input_bytes} B): {ISOLATION_NOTE}",
        env.peak_heap,
    );
    for (column, reading) in COLUMNS.iter().zip(readings) {
        let bound = (column.pin)(env);
        assert!(
            reading <= bound.ceiling,
            "{name}: {reading} {} exceed the pinned envelope {}: {ISOLATION_NOTE}",
            column.unit,
            bound.ceiling,
        );
        match bound.floor {
            Floor::Tripwire(floor) => assert!(
                reading >= floor,
                "{name}: the {} counter reads {reading}, below the {floor} improvement \
                 tripwire (measured x0.75): attribute the drop; an improvement re-pins \
                 the band, and a dead meter is the bypass this column exists to catch",
                column.key,
            ),
            Floor::LiveBits { streams, tail_bits } => {
                let floor = (8 * input_bytes as u64).saturating_sub(streams * (8 + tail_bits));
                assert!(
                    reading >= floor,
                    "{name}: the {} counter reads {reading}, under the {floor}-bit liveness floor \
                     over {input_bytes} input bytes: the walk left the metered \
                     primitives",
                    column.key,
                );
            }
        }
    }
    r
}

/// Magnitude (bits) of the harness self-test's probe operand.
const HARNESS_PROBE_MAGNITUDE_BITS: usize = 1_024;

/// Depth of the harness self-test's probe operand.
const HARNESS_PROBE_DEPTH: usize = 64;

/// The harness judges every column.
///
/// A body that moves every counter fails under a ceiling below its
/// reading on any one column, under either floor above its
/// reading on any one column, and under a zero heap ceiling.
///
/// The harness's own negative control: a column whose assert is skipped, a
/// reset that leaves a stale reading, or a read wired to a counter the body
/// never moves shows up here as a probe that passes when it must not.
#[test]
fn harness_judges_every_column() {
    let v = version_of(&Shape::Bigroot.build2(HARNESS_PROBE_MAGNITUDE_BITS, HARNESS_PROBE_DEPTH));
    let input = v.encode().len();
    let open = Envelope {
        peak_heap: usize::MAX,
        touch: band(u64::MAX, 0),
        scan: band(u64::MAX, 0),
    };
    HEAP.reset_peak_usage();
    let baseline = HEAP.current_usage();
    let r = metered("harness_probe", input, &open, || v.rank());
    // Read the counters before anything formats the result: the readings
    // must be the probe body's alone.
    let readings = COLUMNS.map(|column| (column.read)());
    assert!(
        HEAP.peak_usage().saturating_sub(baseline) > 0,
        "the probe body must allocate"
    );
    consumed(r);
    let fails_over = |env: &Envelope, input: usize| {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            consumed(metered("harness_probe", input, env, || v.rank()))
        }))
        .is_err()
    };
    let fails = |env: &Envelope| fails_over(env, input);
    let closed = Envelope {
        peak_heap: 0,
        ..open
    };
    assert!(fails(&closed), "a zero heap ceiling must fail the probe");
    for (column, reading) in COLUMNS.iter().zip(readings) {
        assert!(
            reading > 0,
            "the probe body must move the {} counter",
            column.key
        );
        let mut over = open;
        *(column.pin_mut)(&mut over) = band(reading - 1, 0);
        assert!(
            fails(&over),
            "{}: a ceiling under the reading must fail the probe",
            column.key
        );
        let mut under = open;
        *(column.pin_mut)(&mut under) = band(u64::MAX, reading + 1);
        assert!(
            fails(&under),
            "{}: a floor over the reading must fail the probe",
            column.key
        );
        let mut unread = open;
        *(column.pin_mut)(&mut unread) = whole_input(u64::MAX, 1);
        assert!(
            fails_over(
                &unread,
                usize::try_from(reading + 1).expect("a reading fits usize")
            ),
            "{}: a whole-input floor over the reading must fail the probe",
            column.key
        );
    }
}

/// Lift a generated shape into a [`Version`], outside any measurement.
pub fn version_of(p: &meter::Encoding) -> Version {
    p.version()
}

/// Decode a generated shape as a [`Party`], outside any measurement.
pub fn party_of(p: &meter::Encoding) -> Party {
    Party::decode(&p.bytes[..]).expect("generated shape is strict normal form")
}

/// An emit kernel's output on two versions' version streams, built outside
/// measurement: the byte-identity leg of the public join and meet rows.
pub fn emitted(kernel: fn(&Version, &Version) -> Version, a: &Version, b: &Version) -> Version {
    kernel(a, b)
}

/// The rank kernel's answer on a version's version stream, built outside
/// measurement: the identity leg of the public rank rows.
pub fn kernel_rank(v: &Version) -> before::Rank {
    meter::version::rank(v)
}

/// Assert a scenario result is consumed, so the operation cannot be
/// dead-code-eliminated and the walk provably ran to completion.
pub fn consumed<T: Debug>(v: T) -> String {
    format!("{v:?}")
}
