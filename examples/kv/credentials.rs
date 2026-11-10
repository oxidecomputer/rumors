use std::fs;
use std::io::{ErrorKind, Write};
use std::path::Path;

use anyhow::{Context, Result};
use rcgen::{CertificateParams, ExtendedKeyUsagePurpose, KeyPair};
use tempfile::NamedTempFile;

pub const NETWORK_NAME: &str = "kv.example.rumors.invalid";

const CREDENTIALS_FILE: &str = "credentials.pem";

fn create(dir: &Path) -> Result<()> {
    let mut params = CertificateParams::new([NETWORK_NAME.to_owned()])?;
    params.extended_key_usages = vec![
        ExtendedKeyUsagePurpose::ServerAuth,
        ExtendedKeyUsagePurpose::ClientAuth,
    ];
    let key = KeyPair::generate()?;
    let cert = params.self_signed(&key)?;

    fs::create_dir_all(dir)?;
    let mut file = NamedTempFile::new_in(dir)?;
    file.write_all(format!("{}{}", cert.pem(), key.serialize_pem()).as_bytes())?;
    let path = dir.join(CREDENTIALS_FILE);
    match file.persist_noclobber(&path) {
        Ok(_) => eprintln!(
            "created network credentials at {}; copy this file to other machines to let them join",
            path.display()
        ),
        Err(error) if error.error.kind() == ErrorKind::AlreadyExists => {}
        Err(error) => {
            return Err(error.error).with_context(|| format!("creating {}", path.display()));
        }
    }
    Ok(())
}

use rustls::pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject};

pub struct Credentials {
    pub cert: CertificateDer<'static>,
    pub key: PrivateKeyDer<'static>,
}

impl Credentials {
    pub fn load_or_create(dir: &Path) -> Result<Self> {
        fn read_pem<T: PemObject>(path: &Path) -> Result<T> {
            T::from_pem_file(path).with_context(|| format!("reading {}", path.display()))
        }

        let path = dir.join(CREDENTIALS_FILE);
        if !path.exists() {
            create(dir)?;
        }
        Ok(Self {
            cert: read_pem(&path)?,
            key: read_pem(&path)?,
        })
    }
}
