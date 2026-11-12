use std::path::Path;

use anyhow::{Context, Result};
use rustls::pki_types::pem::PemObject;

use super::Credentials;
use super::store::CREDENTIALS_FILE;

impl Credentials {
    pub(crate) fn load_or_create(dir: &Path) -> Result<Self> {
        fn read_pem<T: PemObject>(path: &Path) -> Result<T> {
            T::from_pem_file(path).with_context(|| format!("reading {}", path.display()))
        }

        let path = dir.join(CREDENTIALS_FILE);
        if !path.exists() {
            Self::generate()?.store(dir)?;
        }

        Ok(Self {
            cert: read_pem(&path)?,
            key: read_pem(&path)?,
        })
    }
}
