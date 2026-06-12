//! PTY-driven rumormill nodes: spawn the real binary under a
//! pseudo-terminal, type into it, and scrape what a human would see.
//!
//! Each [`Node`] owns the PTY master, a writer for keystrokes, and a reader
//! thread that feeds every byte into a [`Transcript`]: a vt100 screen model
//! (for scraping the live TUI) plus a bounded raw capture (for the plain
//! stderr lines printed before the TUI starts and after it exits — the
//! endpoint id announcement and the departure report).

use std::collections::VecDeque;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

use anyhow::Context as _;
use portable_pty::{Child, CommandBuilder, ExitStatus, MasterPty, PtySize, native_pty_system};

/// Terminal geometry for every spawned node: wide enough that the header
/// (name, net id, counters, merge notice) never truncates, tall enough that
/// the roster title and a tail of messages stay on screen.
pub const COLS: u16 = 120;
/// See [`COLS`].
pub const ROWS: u16 = 32;

/// The first chunk of a node's transcript, kept verbatim: the endpoint id
/// announcement lands here, before the TUI takes over the stream.
const HEAD_CAP: usize = 16 * 1024;

/// The rolling tail of a node's transcript: the departure lines land here,
/// after the TUI restores the screen. Everything between head and tail is
/// TUI redraw traffic, summarized by the vt100 screen instead.
const TAIL_CAP: usize = 128 * 1024;

/// Everything a node has ever written, bounded: a verbatim head, a rolling
/// tail, and a live vt100 screen model of what is currently displayed.
pub struct Transcript {
    parser: vt100::Parser,
    head: Vec<u8>,
    tail: VecDeque<u8>,
    /// The PTY reached EOF: the process exited (or its tty died).
    pub eof: bool,
}

impl Transcript {
    fn new() -> Self {
        Transcript {
            parser: vt100::Parser::new(ROWS, COLS, 0),
            head: Vec::new(),
            tail: VecDeque::new(),
            eof: false,
        }
    }

    fn ingest(&mut self, bytes: &[u8]) {
        self.parser.process(bytes);
        for &byte in bytes {
            if self.head.len() < HEAD_CAP {
                self.head.push(byte);
            } else {
                if self.tail.len() == TAIL_CAP {
                    self.tail.pop_front();
                }
                self.tail.push_back(byte);
            }
        }
    }

    fn screen(&self) -> String {
        self.parser.screen().contents()
    }

    fn raw(&self) -> String {
        let mut text = String::from_utf8_lossy(&self.head).into_owned();
        let tail: Vec<u8> = self.tail.iter().copied().collect();
        text.push_str(&String::from_utf8_lossy(&tail));
        text
    }
}

/// One rumormill process under a PTY.
pub struct Node {
    /// The `--name` it announces (and rosters display).
    pub name: String,
    writer: Box<dyn Write + Send>,
    /// Keeps the PTY alive; dropping the master hangs up the node's tty.
    _master: Box<dyn MasterPty>,
    child: Box<dyn Child + Send + Sync>,
    transcript: Arc<Mutex<Transcript>>,
    exited: Option<ExitStatus>,
}

impl Node {
    /// Spawn the rumormill binary under a fresh PTY, dialing `peer` if
    /// given (which also skips the connect dialog).
    pub fn spawn(bin: &Path, name: &str, peer: Option<&str>) -> anyhow::Result<Node> {
        let pty = native_pty_system()
            .openpty(PtySize {
                rows: ROWS,
                cols: COLS,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("opening a pty (exhausted? check `sysctl kern.tty.ptmx_max`)")?;
        let mut cmd = CommandBuilder::new(bin);
        cmd.args(["--name", name]);
        if let Some(peer) = peer {
            cmd.args(["--peer", peer]);
        }
        cmd.env("TERM", "xterm-256color");
        let child = pty.slave.spawn_command(cmd).context("spawning rumormill")?;
        drop(pty.slave);

        let writer = pty.master.take_writer().context("taking the pty writer")?;
        let mut reader = pty
            .master
            .try_clone_reader()
            .context("cloning the pty reader")?;
        let transcript = Arc::new(Mutex::new(Transcript::new()));
        let sink = Arc::clone(&transcript);
        // The reader thread drains the PTY for the node's whole life: a TUI
        // blocks on write once the PTY buffer fills, so an undrained node
        // is a wedged node. The thread exits on EOF and is not joined; it
        // holds only its `Arc`.
        std::thread::spawn(move || {
            let mut buf = [0u8; 8192];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => {
                        sink.lock().expect("transcript lock").eof = true;
                        return;
                    }
                    Ok(n) => sink.lock().expect("transcript lock").ingest(&buf[..n]),
                }
            }
        });

        Ok(Node {
            name: name.to_string(),
            writer,
            _master: pty.master,
            child,
            transcript,
            exited: None,
        })
    }

    /// Type `line` followed by Enter. Write errors are swallowed: a dead
    /// node's PTY returns EPIPE, and exits are accounted by
    /// [`poll_exit`](Self::poll_exit), not here.
    pub fn send_line(&mut self, line: &str) {
        let _ = self.writer.write_all(line.as_bytes());
        let _ = self.writer.write_all(b"\r");
        let _ = self.writer.flush();
    }

    /// Press Escape: quit (rumormill retires and exits).
    pub fn send_esc(&mut self) {
        let _ = self.writer.write_all(b"\x1b");
        let _ = self.writer.flush();
    }

    /// What a human at this node's terminal currently sees.
    pub fn screen(&self) -> String {
        self.transcript.lock().expect("transcript lock").screen()
    }

    /// The bounded raw transcript: head (pre-TUI stderr, endpoint id) plus
    /// rolling tail (post-TUI stderr, departure lines).
    pub fn raw(&self) -> String {
        self.transcript.lock().expect("transcript lock").raw()
    }

    /// Whether the node's PTY has reached EOF (the process is gone).
    pub fn eof(&self) -> bool {
        self.transcript.lock().expect("transcript lock").eof
    }

    /// The OS pid, while the child is alive.
    pub fn pid(&self) -> Option<u32> {
        self.child.process_id()
    }

    /// The exit status if the process has finished; reaps at most once and
    /// caches the result.
    pub fn poll_exit(&mut self) -> Option<ExitStatus> {
        if self.exited.is_none() {
            self.exited = self.child.try_wait().ok().flatten();
        }
        self.exited.clone()
    }

    /// Force-kill the process (the harness's last resort for wedged nodes).
    pub fn kill(&mut self) {
        let _ = self.child.kill();
    }
}

/// The whole room. Dropping the fleet kills every node still running: a
/// panicking or failing harness must not strand a hundred processes.
#[derive(Default)]
pub struct Fleet {
    /// Index 0 is the seed node every other node bootstraps from.
    pub nodes: Vec<Node>,
}

impl Drop for Fleet {
    fn drop(&mut self) {
        for node in &mut self.nodes {
            if node.poll_exit().is_none() {
                node.kill();
            }
        }
    }
}
