use std::fmt;
use std::path::Path;

use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair, PKCS_ED25519};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::fsutil::{create_private_dir, read_optional, write_private};
use crate::{DeviceId, Error, Result, sanitize_device_name};

const KEY_FILE: &str = "identity.key";
const CERT_FILE: &str = "identity.crt";
const DEVICE_FILE: &str = "device.json";

/// The TLS server name every device certificate is issued for.
///
/// Devices are authenticated by their pinned [`DeviceId`], not by name, so the same
/// fixed name is used everywhere.
pub const CERT_SERVER_NAME: &str = "ecosystem-connector";

/// This device's long-term identity: an Ed25519 key, a self-signed certificate for it,
/// and a user-visible name.
pub struct DeviceIdentity {
    key_der: Zeroizing<Vec<u8>>,
    cert_der: Vec<u8>,
    id: DeviceId,
    name: String,
}

#[derive(Serialize, Deserialize)]
struct DeviceFile {
    name: String,
}

impl DeviceIdentity {
    /// Loads the identity stored in `dir`, or creates and stores a new one named
    /// `default_name` if there is none.
    pub fn load_or_create(dir: &Path, default_name: &str) -> Result<Self> {
        create_private_dir(dir)?;
        let key_path = dir.join(KEY_FILE);
        let cert_path = dir.join(CERT_FILE);

        let identity = match (read_optional(&key_path)?, read_optional(&cert_path)?) {
            (Some(key), Some(cert)) => Self::from_parts(Zeroizing::new(key), cert, &key_path)?,
            (None, None) => {
                let identity = Self::generate(default_name)?;
                write_private(&key_path, &identity.key_der)?;
                write_private(&cert_path, &identity.cert_der)?;
                identity
            }
            _ => {
                return Err(Error::Corrupt {
                    path: dir.to_path_buf(),
                    reason: format!("{KEY_FILE} and {CERT_FILE} must both exist or both be absent"),
                });
            }
        };

        let device_path = dir.join(DEVICE_FILE);
        let name = match read_optional(&device_path)? {
            Some(data) => {
                let file: DeviceFile =
                    serde_json::from_slice(&data).map_err(|e| Error::Corrupt {
                        path: device_path.clone(),
                        reason: e.to_string(),
                    })?;
                sanitize_device_name(&file.name)
            }
            None => {
                let name = sanitize_device_name(default_name);
                write_device_file(&device_path, &name)?;
                name
            }
        };
        Ok(Self { name, ..identity })
    }

    /// Changes the device name and stores it in `dir`.
    pub fn rename(&mut self, dir: &Path, name: &str) -> Result<()> {
        let name = sanitize_device_name(name);
        write_device_file(&dir.join(DEVICE_FILE), &name)?;
        self.name = name;
        Ok(())
    }

    /// Creates a new identity in memory without storing it. Used by tests.
    pub fn generate(name: &str) -> Result<Self> {
        let key = KeyPair::generate_for(&PKCS_ED25519)?;
        let mut params = CertificateParams::new(vec![CERT_SERVER_NAME.to_owned()])?;
        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, CERT_SERVER_NAME);
        params.distinguished_name = dn;
        // Validity is not used for trust decisions (pinning is), but keep it wide so
        // strict parsers never reject the certificate as expired.
        params.not_before = rcgen::date_time_ymd(2024, 1, 1);
        params.not_after = rcgen::date_time_ymd(4096, 1, 1);
        let cert = params.self_signed(&key)?;

        let cert_der = cert.der().to_vec();
        Ok(Self {
            key_der: Zeroizing::new(key.serialize_der()),
            id: DeviceId::from_certificate(&cert_der),
            cert_der,
            name: sanitize_device_name(name),
        })
    }

    fn from_parts(key_der: Zeroizing<Vec<u8>>, cert_der: Vec<u8>, key_path: &Path) -> Result<Self> {
        KeyPair::try_from(key_der.as_slice()).map_err(|e| Error::Corrupt {
            path: key_path.to_path_buf(),
            reason: e.to_string(),
        })?;
        Ok(Self {
            id: DeviceId::from_certificate(&cert_der),
            key_der,
            cert_der,
            name: String::new(),
        })
    }

    /// This device's id.
    pub fn id(&self) -> DeviceId {
        self.id
    }

    /// This device's name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The certificate (DER) presented in TLS handshakes.
    pub fn cert_der(&self) -> &[u8] {
        &self.cert_der
    }

    /// The private key in PKCS#8 DER form. Handle with care.
    pub fn key_der(&self) -> &[u8] {
        &self.key_der
    }
}

impl fmt::Debug for DeviceIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Never print the key.
        f.debug_struct("DeviceIdentity")
            .field("id", &self.id)
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

fn write_device_file(path: &Path, name: &str) -> Result<()> {
    let data = serde_json::to_vec_pretty(&DeviceFile {
        name: name.to_owned(),
    })
    .map_err(|e| Error::Corrupt {
        path: path.to_path_buf(),
        reason: e.to_string(),
    })?;
    write_private(path, &data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_survives_reload() {
        let dir = tempfile::tempdir().unwrap();
        let first = DeviceIdentity::load_or_create(dir.path(), "Laptop").unwrap();
        let second = DeviceIdentity::load_or_create(dir.path(), "Other name").unwrap();
        assert_eq!(first.id(), second.id());
        assert_eq!(second.name(), "Laptop");
    }

    #[test]
    fn rename_is_persisted_and_sanitized() {
        let dir = tempfile::tempdir().unwrap();
        let mut identity = DeviceIdentity::load_or_create(dir.path(), "Laptop").unwrap();
        identity.rename(dir.path(), "Work\u{1b} Mac").unwrap();
        let reloaded = DeviceIdentity::load_or_create(dir.path(), "x").unwrap();
        assert_eq!(reloaded.name(), "Work Mac");
    }

    #[test]
    fn missing_certificate_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        DeviceIdentity::load_or_create(dir.path(), "Laptop").unwrap();
        std::fs::remove_file(dir.path().join(CERT_FILE)).unwrap();
        assert!(DeviceIdentity::load_or_create(dir.path(), "Laptop").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn key_file_is_private() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        DeviceIdentity::load_or_create(dir.path(), "Laptop").unwrap();
        let mode = std::fs::metadata(dir.path().join(KEY_FILE))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    #[test]
    fn debug_output_hides_the_key() {
        let identity = DeviceIdentity::generate("Laptop").unwrap();
        assert!(!format!("{identity:?}").contains("key_der"));
    }
}
