//! Every session in a process of its own.
//!
//! A large session leaves the allocator in a state that slows the next
//! large session in the same process. On illumos, a second session of 10⁷
//! messages, run in the process that had just freed the first one's heap of
//! about 20 GB, took about six times as long as the same session in a fresh
//! process, the time going to libumem's `vmem_xalloc` while the session
//! built its fixture. So every mode that runs sessions starts each one in a
//! child process of this executable, through the hidden `session` command,
//! which takes the session's [`Job`] as JSON.
//!
//! The child runs the job and prints its JSON document as the one line of
//! its standard output; the parent appends that line verbatim, so a record
//! reads the same whichever process ran it. The child's progress goes to
//! the standard error it shares with the parent. Sessions still run
//! `--jobs` at a time: each thread of the parent's pool waits on one child.

use std::process::{Command, Stdio};

use serde::{Deserialize, Serialize};

use crate::grid::Pair;
use crate::model::{LinkSpec, ShapeSpec};
use crate::{count, model, simulate};

/// One session, as the parent hands it to a child.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Job {
    /// A `simulate` session ([`simulate::session`]).
    Simulate {
        /// The pair and link.
        pair: Pair,
        /// The budget, or `None` for unlimited.
        budget: Option<u64>,
        /// The bytes each fixture message encodes in.
        message_bytes: usize,
        /// Whether to record the session in detail.
        detail: bool,
    },
    /// A `model` session ([`model::session`]).
    Model {
        /// The replicas.
        shape: ShapeSpec,
        /// The link.
        link: LinkSpec,
        /// The bytes each fixture message encodes in.
        message_bytes: usize,
    },
    /// A `count` of one size's root question grants ([`count`]).
    Count {
        /// Messages each replica holds.
        messages: u64,
        /// The share of each replica the other holds too, in percent.
        overlap: u64,
        /// The bytes each fixture message encodes in.
        message_bytes: usize,
    },
}

impl Job {
    /// Runs the job in this process and returns its JSON document.
    pub fn run_here(&self) -> String {
        match *self {
            Self::Simulate {
                pair,
                budget,
                message_bytes,
                detail,
            } => simulate::session(pair, budget, message_bytes, detail),
            Self::Model {
                shape,
                link,
                message_bytes,
            } => model::session(shape, link, message_bytes),
            Self::Count {
                messages,
                overlap,
                message_bytes,
            } => count(messages, overlap, message_bytes),
        }
    }

    /// Runs the job in a child process of this executable and returns the
    /// JSON document it printed.
    ///
    /// # Errors
    ///
    /// If this executable cannot be found or started, if the child fails,
    /// or if it prints anything but one line.
    pub fn run_in_child(&self) -> Result<String, String> {
        let job = serde_json::to_string(self).expect("a job serializes");
        let program = std::env::current_exe()
            .map_err(|error| format!("finding this executable to run {self:?}: {error}"))?;
        let output = Command::new(program)
            .arg("session")
            .arg(&job)
            .stdin(Stdio::null())
            .stderr(Stdio::inherit())
            .output()
            .map_err(|error| format!("starting the child for {self:?}: {error}"))?;
        if !output.status.success() {
            return Err(format!("the child for {self:?} failed: {}", output.status));
        }
        let printed = String::from_utf8(output.stdout)
            .map_err(|error| format!("the child for {self:?} printed non-UTF-8: {error}"))?;
        match printed.strip_suffix('\n') {
            Some(line) if !line.contains('\n') => Ok(line.to_owned()),
            _ => Err(format!(
                "the child for {self:?} printed {printed:?}, not one line"
            )),
        }
    }
}
