//! Optional connection-lifecycle tracing, for harnesses and field debugging.
//!
//! The TUI owns the terminal, so connection errors are invisible at runtime:
//! the status line counts them but swallows every detail. When the
//! `RUMORMILL_TRACE` environment variable names a file, the transport
//! appends one timestamped line per notable event (dial failures, drive
//! terminations, merge verdicts, resets) — enough to reconstruct why a node
//! fell out of a room without attaching a debugger to a hundred processes.
//! Unset, the cost is one atomic load per call.

use std::fs::{File, OpenOptions};
use std::io::Write as _;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

/// The trace sink, opened lazily on first use: `None` when the environment
/// variable is unset or the file cannot be opened.
static SINK: OnceLock<Option<Mutex<File>>> = OnceLock::new();

/// Append one line to the trace file, if tracing is enabled.
pub fn trace(line: impl FnOnce() -> String) {
    let sink = SINK.get_or_init(|| {
        let path = std::env::var_os("RUMORMILL_TRACE")?;
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .ok()?;
        Some(Mutex::new(file))
    });
    if let Some(file) = sink {
        let at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let mut file = file.lock().expect("trace sink lock");
        let _ = writeln!(file, "{at} {}", line());
    }
}
