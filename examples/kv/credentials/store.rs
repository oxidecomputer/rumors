use std::fs;
use std::io::{ErrorKind, Write};
use std::path::Path;

use anyhow::{Context, Result};
use pem::Pem;
use tempfile::NamedTempFile;
use tracing::info;

use super::Credentials;

pub(super) const CREDENTIALS_FILE: &str = "credentials.pem";

impl Credentials {
    pub(super) fn store(&self, dir: &Path) -> Result<()> {
        let pem = pem::encode_many(&[
            Pem::new("CERTIFICATE", self.cert.to_vec()),
            Pem::new("PRIVATE KEY", self.key.secret_der()),
        ]);

        fs::create_dir_all(dir)?;
        let mut file = NamedTempFile::new_in(dir)?;
        file.write_all(pem.as_bytes())?;

        let path = dir.join(CREDENTIALS_FILE);
        match file.persist_noclobber(&path) {
            Err(error) if error.error.kind() == ErrorKind::AlreadyExists => {}
            Ok(_) => info!(
                path = %path.display(),
                "created network credentials; copy this file to other machines to let them join"
            ),
            Err(error) => {
                return Err(error.error).with_context(|| format!("creating {}", path.display()));
            }
        }
        Ok(())
    }
}
