use super::*;

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
/// environment; an unchanged copy has no difference and the same
/// fingerprint. Every part is tried, the build by its executable's digest alone, as a
/// change to the library alone makes; the load, which is no part of the
/// identity, changes nothing.
#[test]
fn each_identity_part_alone_tells_environments_apart() {
    let base = Environment::capture().unwrap();
    let changed = |part: &str| -> Environment {
        let mut other = base.clone();
        match part {
            "host name" => other.hostname.push_str("-other"),
            "operating system" => other.os.push_str("-other"),
            "kernel release" => other.kernel_release.push_str("-other"),
            "kernel version" => other.kernel_version.push_str("-other"),
            "processor" => other.cpu_model.push_str("-other"),
            "logical processors" => other.logical_cpus += 1,
            "memory" => other.memory_bytes = Some(other.memory_bytes.unwrap_or(0) + 1),
            "TCP settings" => other.tcp.settings.push(("other".into(), "1".into())),
            "build" => other.build.executable = Some("0".repeat(64)),
            _ => unreachable!("every identity part has a change above"),
        }
        other
    };
    for (part, _) in base.identity() {
        let other = changed(part);
        let differences = base.differences(&other);
        assert_eq!(differences.len(), 1, "{part}: {differences:?}");
        assert!(
            differences[0].starts_with(&format!("the {part} is")),
            "{part}: {differences:?}"
        );
        assert_ne!(base.fingerprint(), other.fingerprint(), "{part}");
    }
    let reloaded = Environment {
        load_average: Some([99.0; 3]),
        ..base.clone()
    };
    assert_eq!(base.differences(&reloaded), Vec::<String>::new());
    assert_eq!(base.fingerprint(), reloaded.fingerprint());
}
