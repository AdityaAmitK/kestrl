//! Creation and storage of the Kestrl root certificate authority.

use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

use rcgen::{CertificateParams, DistinguishedName, DnType, IsCa, KeyPair, KeyUsagePurpose};

use crate::{Error, Result};

/// Paths to the persisted Kestrl root certificate and private key.
#[derive(Debug, Clone)]
pub struct CertificateAuthorityFiles {
    /// PEM certificate path.
    pub certificate: PathBuf,
    /// PEM private key path.
    pub private_key: PathBuf,
}

/// Return the directory containing Kestrl's certificate authority files.
pub fn files() -> Result<CertificateAuthorityFiles> {
    let directory = crate::paths::data_dir()?.join("ca");
    Ok(CertificateAuthorityFiles {
        certificate: directory.join("kestrl-ca.pem"),
        private_key: directory.join("kestrl-ca.key"),
    })
}

/// Ensure a root CA exists and return its file paths.
pub fn ensure() -> Result<CertificateAuthorityFiles> {
    let paths = files()?;
    if paths.certificate.is_file() && paths.private_key.is_file() {
        return Ok(paths);
    }
    let directory = paths.certificate.parent().ok_or(Error::NoDataDir)?;
    fs::create_dir_all(directory)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(directory, fs::Permissions::from_mode(0o700))?;
    }

    let mut params = CertificateParams::default();
    params.is_ca = IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
    params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
    let mut name = DistinguishedName::new();
    name.push(DnType::CommonName, "Kestrl Root CA");
    params.distinguished_name = name;
    let key = KeyPair::generate().map_err(|error| Error::Capture(error.to_string()))?;
    let certificate = params
        .self_signed(&key)
        .map_err(|error| Error::Capture(error.to_string()))?;
    fs::write(&paths.certificate, certificate.pem())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(&paths.private_key)?
            .write_all(key.serialize_pem().as_bytes())?;
    }
    #[cfg(not(unix))]
    OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&paths.private_key)?
        .write_all(key.serialize_pem().as_bytes())?;
    Ok(paths)
}
