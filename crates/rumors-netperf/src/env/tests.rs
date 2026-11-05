use super::*;

/// `base` with the identity part named `part` changed and every other part
/// as it was: the build by its executable's digest alone, as a change to
/// the library alone makes.
pub(crate) fn changed(base: &Environment, part: &str) -> Environment {
    let mut other = base.clone();
    match part {
        HOST_NAME => other.hostname.push_str("-other"),
        "operating system" => other.os.push_str("-other"),
        "kernel release" => other.kernel_release.push_str("-other"),
        "kernel version" => other.kernel_version.push_str("-other"),
        "processor" => other.cpu_model.push_str("-other"),
        "logical processors" => other.logical_cpus += 1,
        "memory" => other.memory_bytes = Some(other.memory_bytes.unwrap_or(0) + 1),
        "TCP settings" => other.tcp.settings.push(("other".into(), "1".into())),
        "congestion control" => {
            let name = other.congestion_control.unwrap_or_default();
            other.congestion_control = Some(format!("{name}-other"));
        }
        "build" => other.build.executable = Some("0".repeat(64)),
        _ => unreachable!("every identity part has a change above"),
    }
    other
}

/// The executable digest is the SHA-256 of the file this process runs,
/// in lower-case hexadecimal, and the same on every call.
///
/// The reference reads the whole file by its path in one piece, apart from
/// the platform's name for the running image and the chunked reads the
/// digest takes, so the two agree only if both hash the same bytes.
#[test]
fn executable_digest_hashes_the_running_executable() {
    let bytes = fs::read(std::env::current_exe().unwrap()).unwrap();
    let expected = hex(&Sha256::digest(&bytes));
    let digest = executable_digest().unwrap();
    assert_eq!(digest, expected);
    assert_eq!(digest.len(), 64);
    assert!(
        digest
            .chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
    );
    assert_eq!(executable_digest().unwrap(), digest);
}

/// An environment changed in one part of its identity alone differs from
/// the original in exactly that part, and has another fingerprint.
///
/// [`Environment::differences`] names the part with both values, so
/// neither a resumed run nor the calibration cache accepts the changed
/// environment, and [`Environment::differences_beyond_host`] names it too
/// for every part but the host's name, the one part shards may differ in.
/// An unchanged copy has no difference and the same fingerprint. Every
/// part is tried, the build by its executable's digest alone; the load,
/// which is no part of the identity, changes nothing.
#[test]
fn each_identity_part_alone_tells_environments_apart() {
    let base = Environment::capture().unwrap();
    for (part, ours) in base.identity() {
        let other = changed(&base, part);
        let theirs = other
            .identity()
            .into_iter()
            .find(|(name, _)| *name == part)
            .map(|(_, value)| value)
            .unwrap();
        let difference = Difference { part, ours, theirs };
        assert_eq!(base.differences(&other), vec![difference.clone()], "{part}");
        assert_eq!(
            base.differences_beyond_host(&other),
            if part == HOST_NAME {
                vec![]
            } else {
                vec![difference]
            },
            "{part}"
        );
        assert_ne!(base.fingerprint(), other.fingerprint(), "{part}");
    }
    let reloaded = Environment {
        load_average: Some([99.0; 3]),
        ..base.clone()
    };
    assert_eq!(base.differences(&reloaded), vec![]);
    assert_eq!(base.fingerprint(), reloaded.fingerprint());
}

/// The fields of a recorded environment that are no part of its identity,
/// by their paths in `environment.json`, each for its reason.
///
/// - `load_average` changes from run to run on any host.
/// - The TCP limits' buffers, payload share, and usable window are
///   computed from the platform's settings, which the identity names
///   whole.
const OUTSIDE_IDENTITY: [&str; 5] = [
    "load_average",
    "tcp/max_receive_buffer",
    "tcp/max_send_buffer",
    "tcp/payload_share",
    "tcp/usable_window",
];

/// An environment whose every optional field holds a value, so that each
/// recorded field appears in its JSON.
fn populated() -> Environment {
    Environment {
        build: Build {
            version: "0.1.0".into(),
            revision: "0123456789ab".into(),
            rustc: "rustc 1.0.0".into(),
            target: "x86_64-unknown-linux-gnu".into(),
            profile: "release".into(),
            executable: Some("a".repeat(64)),
        },
        hostname: "host".into(),
        os: "Linux".into(),
        kernel_release: "6.12.0".into(),
        kernel_version: "#1 SMP".into(),
        cpu_model: "processor".into(),
        logical_cpus: 32,
        memory_bytes: Some(1 << 30),
        tcp: TcpLimits {
            settings: vec![("net.ipv4.tcp_rmem".into(), "4096 131072 6291456".into())],
            max_receive_buffer: Some(6_291_456),
            max_send_buffer: Some(4_194_304),
            payload_share: 0.5,
            usable_window: Some(2_097_152),
        },
        congestion_control: Some("cubic".into()),
        load_average: Some([1.0, 2.0, 3.0]),
    }
}

/// The path of every leaf of `value`, `/`-separated: object keys by name,
/// array elements by index.
fn leaves(value: &serde_json::Value, prefix: &str, out: &mut Vec<String>) {
    let join = |key: &str| {
        if prefix.is_empty() {
            key.to_string()
        } else {
            format!("{prefix}/{key}")
        }
    };
    match value {
        serde_json::Value::Object(map) => {
            for (key, child) in map {
                leaves(child, &join(key), out);
            }
        }
        serde_json::Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                leaves(child, &join(&index.to_string()), out);
            }
        }
        _ => out.push(prefix.to_string()),
    }
}

/// `value` with the leaf at `path` changed to another value of its type.
///
/// # Panics
///
/// If the leaf is null: [`populated`] leaves no field empty.
fn change_leaf(value: &mut serde_json::Value, path: &str) {
    let leaf = path.split('/').fold(value, |node, key| match node {
        serde_json::Value::Array(items) => &mut items[key.parse::<usize>().unwrap()],
        node => &mut node[key],
    });
    *leaf = match &*leaf {
        serde_json::Value::String(text) => format!("{text}-other").into(),
        serde_json::Value::Bool(flag) => (!flag).into(),
        serde_json::Value::Number(number) => match number.as_u64() {
            Some(whole) => (whole + 1).into(),
            None => (number.as_f64().unwrap() + 0.5).into(),
        },
        other => panic!("the populated environment leaves `{path}` as {other}"),
    };
}

/// Every field an environment records is part of its identity, but those
/// [`OUTSIDE_IDENTITY`] names.
///
/// Changing any other one field changes [`Environment::differences`] and
/// the fingerprint, so neither a resumed run, nor the calibration cache,
/// nor `check` across shards accepts environments that differ in it.
/// Every leaf of the recorded JSON is changed alone, so a field the record
/// gains is judged without this test naming it. Changing a field
/// [`OUTSIDE_IDENTITY`] names changes neither, so the list stays exact.
#[test]
fn every_recorded_field_but_the_named_ones_is_identity() {
    let base = populated();
    let json = serde_json::to_value(&base).unwrap();
    let mut paths = Vec::new();
    leaves(&json, "", &mut paths);
    for path in &paths {
        let mut changed = json.clone();
        change_leaf(&mut changed, path);
        let other: Environment = serde_json::from_value(changed).unwrap();
        let outside = OUTSIDE_IDENTITY
            .iter()
            .any(|field| path == field || path.starts_with(&format!("{field}/")));
        assert_eq!(
            base.differences(&other).is_empty(),
            outside,
            "changing `{path}` alone"
        );
        assert_eq!(
            base.fingerprint() == other.fingerprint(),
            outside,
            "changing `{path}` alone"
        );
    }
    for field in OUTSIDE_IDENTITY {
        assert!(
            paths
                .iter()
                .any(|path| path == field || path.starts_with(&format!("{field}/"))),
            "`{field}` names no recorded field"
        );
    }
}
