//! Nodes of the kv example run as processes, driven over HTTP, and the
//! waiting, relaying, and bookmark probing the scenarios need.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Display;
use std::fs::{self, File, TryLockError};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{Arc, LazyLock, Mutex, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use http::StatusCode;
use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;
use rumors::{FileBookmark, TokioBlocking};
use tempfile::TempDir;
use ureq::Agent;

/// How long a scenario waits for any one expected state: a node starting,
/// writes converging, a process exiting. Reconnecting after a peer restarts
/// waits out the example's dial backoff, a few seconds at most here, so a
/// miss means the state is not coming.
pub const DEADLINE: Duration = Duration::from_secs(30);

/// How often [`eventually`] looks again.
const POLL: Duration = Duration::from_millis(50);

/// The name of the example's credentials file inside its credentials
/// directory.
pub const CREDENTIALS_FILE: &str = "credentials.pem";

/// The start of the event a node logs when it creates the network's
/// credentials.
pub const CREATED_CREDENTIALS: &str = "created network credentials";

/// The content type of the example's form-encoded writes.
pub const FORM: &str = "application/x-www-form-urlencoded";

/// Poll `probe` until it returns `Ok`, failing the test with its last `Err`,
/// a description of the state it saw, once [`DEADLINE`] passes.
pub fn eventually<T>(mut probe: impl FnMut() -> Result<T, String>) -> T {
    let deadline = Instant::now() + DEADLINE;
    loop {
        match probe() {
            Ok(value) => return value,
            Err(seen) if Instant::now() >= deadline => {
                panic!("gave up after {DEADLINE:?}: {seen}")
            }
            Err(_) => thread::sleep(POLL),
        }
    }
}

/// Succeed when every node in `nodes` reads `key` as `want`, describing the
/// first that does not otherwise.
pub fn all_read(nodes: &[&Node], key: &str, want: Option<&str>) -> Result<(), String> {
    nodes.iter().try_for_each(|node| node.reads(key, want))
}

/// The kv example's executable, built once per test process.
///
/// The build uses the test's own profile, dev or release, so a test run reuses
/// what an earlier one built; when nothing changed, cargo finishes in a
/// fraction of a second. Start the executable only while holding
/// [`executable_lock`] shared.
fn kv_binary() -> &'static Path {
    static BINARY: OnceLock<PathBuf> = OnceLock::new();
    BINARY.get_or_init(|| {
        let lock = executable_lock();
        lock.lock().expect("locking the kv example's executable");
        escargot::CargoBuild::new()
            .manifest_path(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
            .example("kv")
            .current_release()
            .current_target()
            .run()
            .unwrap_or_else(|error| panic!("cannot build the kv example: {error}"))
            .path()
            .to_owned()
    })
}

/// The lock between this suite's builds of the example and its starts of the
/// executable, across test processes.
///
/// Cargo replaces the executable on every build, even one with nothing to
/// rebuild, so a process starting it while another builds can find no file
/// there. Builds take the lock exclusively and starts take it shared; it
/// releases when the returned file closes.
fn executable_lock() -> File {
    let path = concat!(env!("CARGO_TARGET_TMPDIR"), "/kv-example.lock");
    File::options()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path)
        .unwrap_or_else(|error| panic!("cannot open {path}: {error}"))
}

/// The HTTP client every request goes through: it reports every status as a
/// response, and gives up on a node that stops answering.
static AGENT: LazyLock<Agent> = LazyLock::new(|| {
    Agent::new_with_config(
        Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(DEADLINE))
            .build(),
    )
});

/// One scenario's temporary directory, holding the credentials and bookmarks
/// directories its nodes share unless a node is given others.
pub struct Dirs {
    /// The directory, deleted when the scenario ends.
    root: TempDir,
}

impl Dirs {
    /// A fresh, empty directory.
    pub fn new() -> Self {
        Self {
            root: tempfile::tempdir().expect("creating a temporary directory"),
        }
    }

    /// The credentials directory, which no node has created yet.
    pub fn credentials(&self) -> PathBuf {
        self.root.path().join("credentials")
    }

    /// The bookmarks directory, which no node has created yet.
    pub fn bookmarks(&self) -> PathBuf {
        self.root.path().join("bookmarks")
    }

    /// A node named `name`, for diagnostics, using these directories and
    /// listening on loopback ports the operating system picks.
    pub fn node(&self, name: &str) -> Launch {
        Launch {
            name: name.to_owned(),
            credentials: self.credentials(),
            bookmarks: self.bookmarks(),
            gossip: "127.0.0.1:0".to_owned(),
            http: "127.0.0.1:0".to_owned(),
            peers: Vec::new(),
        }
    }
}

/// The command line of a node not yet started.
pub struct Launch {
    /// The node's name in diagnostics.
    name: String,
    /// `--credentials`.
    credentials: PathBuf,
    /// `--bookmarks`.
    bookmarks: PathBuf,
    /// `--gossip`.
    gossip: String,
    /// `--http`.
    http: String,
    /// Each `--peer`, in order.
    peers: Vec<String>,
}

impl Launch {
    /// Dial `address`, passed to `--peer` as written.
    pub fn peer(mut self, address: impl Display) -> Self {
        self.peers.push(address.to_string());
        self
    }

    /// Listen for gossip on `address` instead.
    pub fn gossip(mut self, address: &str) -> Self {
        address.clone_into(&mut self.gossip);
        self
    }

    /// Serve HTTP on `address` instead.
    pub fn http(mut self, address: &str) -> Self {
        address.clone_into(&mut self.http);
        self
    }

    /// Start the node and wait until it logs that it is ready.
    pub fn start(self) -> Node {
        let mut node = self.spawn();
        node.wait_listening();
        node
    }

    /// Start the node without waiting for it.
    ///
    /// The node logs at `info` whatever the test's own environment says, since
    /// the fixture reads its log.
    pub fn spawn(self) -> Node {
        let binary = kv_binary();
        let lock = executable_lock();
        lock.lock_shared()
            .expect("locking the kv example's executable");
        let mut command = Command::new(binary);
        command
            .arg("--credentials")
            .arg(&self.credentials)
            .arg("--bookmarks")
            .arg(&self.bookmarks)
            .args(["--gossip", &self.gossip, "--http", &self.http])
            .args(self.peers.iter().flat_map(|peer| ["--peer", peer]))
            .env("RUST_LOG", "info")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        let mut child = command
            .spawn()
            .unwrap_or_else(|error| panic!("cannot start {}: {error}", self.name));
        drop(lock);

        let stderr = child.stderr.take().expect("stderr is piped");
        let log = Arc::new(Mutex::new(Vec::new()));
        let reader = thread::spawn({
            let log = log.clone();
            move || {
                for line in BufReader::new(stderr).lines() {
                    let Ok(line) = line else { break };
                    log.lock()
                        .expect("no reader panics holding the log")
                        .push(line);
                }
            }
        });

        Node {
            name: self.name,
            child,
            log,
            reader: Some(reader),
            addresses: None,
        }
    }
}

/// A running node of the kv example.
///
/// Dropping it kills the process, so no test leaves one behind, and prints
/// its log when the test is failing.
pub struct Node {
    /// The node's name in diagnostics.
    name: String,
    /// The process.
    child: Child,
    /// Every line the process has written to stderr so far.
    log: Arc<Mutex<Vec<String>>>,
    /// The thread collecting the log, until the process exits and it is
    /// joined.
    reader: Option<JoinHandle<()>>,
    /// The gossip and HTTP addresses, once the node has logged them.
    addresses: Option<(SocketAddr, SocketAddr)>,
}

impl Node {
    /// The address the node gossips on.
    pub fn gossip(&self) -> SocketAddr {
        self.addresses
            .expect("the node was started, not only spawned")
            .0
    }

    /// The address the node serves HTTP on.
    pub fn http(&self) -> SocketAddr {
        self.addresses
            .expect("the node was started, not only spawned")
            .1
    }

    /// Wait until the node logs its `listening` event, and record the
    /// addresses it names.
    pub fn wait_listening(&mut self) {
        let addresses = eventually(|| {
            if let Some(addresses) = self.log_lines().iter().find_map(|line| listening(line)) {
                return Ok(addresses);
            }
            match self.child.try_wait() {
                Ok(Some(status)) => panic!("{} exited before listening: {status}", self.name),
                _ => Err(format!("{} has not logged that it is listening", self.name)),
            }
        });
        self.addresses = Some(addresses);
    }

    /// Every line the node has logged so far.
    pub fn log_lines(&self) -> Vec<String> {
        self.log
            .lock()
            .expect("no reader panics holding the log")
            .clone()
    }

    /// Whether any line the node has logged contains `needle`.
    pub fn logged(&self, needle: &str) -> bool {
        self.log_lines().iter().any(|line| line.contains(needle))
    }

    /// The network the node took on most recently, by seeding or joining it,
    /// as far as its log shows so far.
    ///
    /// A node that gives way logs nothing until it joins the next network, so
    /// this names the network it left until then.
    pub fn network(&self) -> Option<String> {
        self.log_lines()
            .iter()
            .rev()
            .find_map(|line| took_on(line).map(str::to_owned))
    }

    /// Send SIGINT, as Ctrl-C does, and wait for the process to exit.
    pub fn interrupt(&mut self) -> ExitStatus {
        let pid = Pid::from_raw(
            i32::try_from(self.child.id()).expect("process IDs come from a pid_t, an i32"),
        );
        kill(pid, Signal::SIGINT)
            .unwrap_or_else(|error| panic!("cannot interrupt {}: {error}", self.name));
        self.wait_exit()
    }

    /// Send SIGKILL and wait for the process to exit.
    pub fn kill(&mut self) {
        self.child
            .kill()
            .unwrap_or_else(|error| panic!("cannot kill {}: {error}", self.name));
        self.wait_exit();
    }

    /// Wait for the process to exit, then for its whole log.
    pub fn wait_exit(&mut self) -> ExitStatus {
        let status = eventually(|| match self.child.try_wait() {
            Ok(Some(status)) => Ok(status),
            Ok(None) => Err(format!("{} is still running", self.name)),
            Err(error) => panic!("cannot wait for {}: {error}", self.name),
        });
        if let Some(reader) = self.reader.take() {
            reader.join().expect("the log reader does not panic");
        }
        status
    }

    /// Succeed when the node reads `key` as `want`, describing what it read
    /// otherwise.
    pub fn reads(&self, key: &str, want: Option<&str>) -> Result<(), String> {
        let got = self.get(key);
        if got.as_deref() == want {
            Ok(())
        } else {
            Err(format!(
                "{} reads {key} as {got:?}, not {want:?}",
                self.name
            ))
        }
    }

    /// `GET /{key}`: the value, or `None` for a 404.
    pub fn get(&self, key: &str) -> Option<String> {
        let (status, body) = self.respond(AGENT.get(self.url(&format!("/{key}"))).call());
        match status {
            StatusCode::OK => Some(body),
            StatusCode::NOT_FOUND => None,
            _ => panic!("{} answered GET /{key} with {status}: {body}", self.name),
        }
    }

    /// `GET /`: every entry.
    pub fn list(&self) -> BTreeMap<String, String> {
        let (status, body) = self.respond(AGENT.get(self.url("/")).call());
        assert_eq!(
            status,
            StatusCode::OK,
            "{} answered GET / with {body}",
            self.name
        );
        serde_json::from_str(&body)
            .unwrap_or_else(|error| panic!("{} listed {body:?}: {error}", self.name))
    }

    /// `POST /` of `pairs`, form-encoded, which must succeed.
    pub fn set(&self, pairs: &[(&str, &str)]) {
        let status = self.post_form(pairs);
        assert_eq!(
            status,
            StatusCode::NO_CONTENT,
            "{} answered {pairs:?}",
            self.name
        );
    }

    /// `POST /` of `pairs`, form-encoded.
    pub fn post_form(&self, pairs: &[(&str, &str)]) -> StatusCode {
        let response = AGENT.post(self.url("/")).send_form(pairs.iter().copied());
        self.respond(response).0
    }

    /// `POST /` of `body` exactly, with `content_type` if given.
    pub fn post_raw(&self, body: &str, content_type: Option<&str>) -> StatusCode {
        let request = AGENT.post(self.url("/"));
        let request = match content_type {
            Some(content_type) => request.header("content-type", content_type),
            None => request,
        };
        self.respond(request.send(body)).0
    }

    /// `DELETE /{key}`.
    pub fn delete(&self, key: &str) -> StatusCode {
        self.respond(AGENT.delete(self.url(&format!("/{key}"))).call())
            .0
    }

    /// The URL of `path` on the node's HTTP address.
    fn url(&self, path: &str) -> String {
        format!("http://{}{path}", self.http())
    }

    /// The status and body of `response`, failing the test if the request
    /// got none.
    fn respond(
        &self,
        response: Result<ureq::http::Response<ureq::Body>, ureq::Error>,
    ) -> (StatusCode, String) {
        let mut response =
            response.unwrap_or_else(|error| panic!("{} did not answer: {error}", self.name));
        let body = response
            .body_mut()
            .read_to_string()
            .unwrap_or_else(|error| panic!("{} sent an unreadable body: {error}", self.name));
        (response.status(), body)
    }
}

impl Drop for Node {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        if thread::panicking() {
            if let Some(reader) = self.reader.take() {
                let _ = reader.join();
            }
            eprintln!("----- {} logged:", self.name);
            for line in self.log_lines() {
                eprintln!("{line}");
            }
        }
    }
}

/// The gossip and HTTP addresses a node's `listening` event names, if `line`
/// is that event.
fn listening(line: &str) -> Option<(SocketAddr, SocketAddr)> {
    let fields = event(line, "listening")?;
    Some((
        field(fields, "gossip")?.parse().ok()?,
        field(fields, "http")?.parse().ok()?,
    ))
}

/// The network a node's `seeded network` or `joined network` event names, if
/// `line` is either.
fn took_on(line: &str) -> Option<&str> {
    let fields = event(line, "seeded network").or_else(|| event(line, "joined network"))?;
    field(fields, "network")
}

/// The fields of `line`, if it is an event whose message is `message`.
fn event<'a>(line: &'a str, message: &str) -> Option<&'a str> {
    Some(line.split_once(&format!(": {message} "))?.1)
}

/// The value of the field `name` among an event's `fields`.
fn field<'a>(fields: &'a str, name: &str) -> Option<&'a str> {
    fields
        .split_whitespace()
        .find_map(|field| field.strip_prefix(name)?.strip_prefix('='))
}

/// A loopback listener that leaves connections waiting in its backlog until
/// [`Relay::forward`] splices them through to a node.
///
/// A node told to dial the relay connects at once, and its TLS handshake
/// waits for an answer until the test forwards: so the test decides when the
/// node first meets its peer, though the peer's address did not exist when
/// the node started.
pub struct Relay {
    /// The listener, not yet accepting.
    listener: TcpListener,
}

impl Relay {
    /// A relay on a loopback port the operating system picks.
    pub fn new() -> Self {
        Self {
            listener: TcpListener::bind("127.0.0.1:0").expect("binding a loopback port"),
        }
    }

    /// The address to dial.
    pub fn address(&self) -> SocketAddr {
        self.listener
            .local_addr()
            .expect("a bound listener has an address")
    }

    /// Accept every connection from now on, waiting ones first, and splice
    /// each to `target`.
    pub fn forward(self, target: SocketAddr) {
        thread::spawn(move || {
            for inbound in self.listener.incoming() {
                let Ok(inbound) = inbound else { return };
                let Ok(outbound) = TcpStream::connect(target) else {
                    continue;
                };
                splice(inbound, outbound);
            }
        });
    }
}

/// Copy bytes both ways between `one` and `other` until each direction ends,
/// passing each end of stream on.
fn splice(one: TcpStream, other: TcpStream) {
    let pipe = |mut from: TcpStream, mut to: TcpStream| {
        thread::spawn(move || {
            let _ = io::copy(&mut from, &mut to);
            let _ = to.shutdown(Shutdown::Write);
        });
    };
    let (Ok(one_again), Ok(other_again)) = (one.try_clone(), other.try_clone()) else {
        return;
    };
    pipe(one, other);
    pipe(other_again, one_again);
}

/// The bookmark slots present in `dir`, and which of them a live process
/// holds.
///
/// The test process opens each slot itself: a slot's lock is per open file,
/// so this process contends for it like any node, and a slot it can open is
/// one no node holds. Call this only while no node is starting, since a node
/// claiming a slot this process holds for the moment moves on to the next.
pub fn slots(dir: &Path) -> (BTreeSet<u32>, BTreeSet<u32>) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("building a runtime");
    let mut present = BTreeSet::new();
    let mut held = BTreeSet::new();
    for entry in fs::read_dir(dir).expect("reading the bookmarks directory") {
        let path = entry.expect("reading a directory entry").path();
        let Some(slot) = path
            .file_name()
            .and_then(|name| name.to_str()?.strip_suffix(".bookmark")?.parse().ok())
        else {
            continue;
        };
        present.insert(slot);
        match runtime.block_on(FileBookmark::new(&path, TokioBlocking)) {
            Ok(_) => {}
            Err(TryLockError::WouldBlock) => {
                held.insert(slot);
            }
            Err(TryLockError::Error(error)) => panic!("cannot open {}: {error}", path.display()),
        }
    }
    (present, held)
}

/// An HTTP connection that has finished one request and stays open, idle, in
/// keep-alive.
pub fn idle_keep_alive(address: SocketAddr) -> TcpStream {
    let mut stream = TcpStream::connect(address).expect("connecting to the node");
    stream
        .write_all(b"GET / HTTP/1.1\r\nHost: kv\r\n\r\n")
        .expect("sending a request");
    let mut reader = BufReader::new(stream.try_clone().expect("cloning the stream"));
    let mut length = 0;
    loop {
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .expect("reading the response head");
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':')
            && name.eq_ignore_ascii_case("content-length")
        {
            length = value.trim().parse().expect("a numeric content length");
        }
    }
    let mut body = vec![0; length];
    reader
        .read_exact(&mut body)
        .expect("reading the response body");
    stream
}
