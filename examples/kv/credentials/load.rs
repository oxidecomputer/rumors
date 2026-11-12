use rustls::pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject};

struct Credentials {
    cert: CertificateDer<'static>,
    key: PrivateKeyDer<'static>,
}

impl Credentials {
    fn load_or_create(dir: &Path) -> Result<Self> {
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
