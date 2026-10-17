use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::Error;

/// Identifies a device: the BLAKE3 hash of its certificate (DER encoding).
///
/// Two devices can only share an id if they share a certificate, which requires the
/// same private key.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(into = "String", try_from = "String")]
pub struct DeviceId([u8; 32]);

impl DeviceId {
    /// Computes the id of the device that owns `cert_der`.
    pub fn from_certificate(cert_der: &[u8]) -> Self {
        Self(*blake3::hash(cert_der).as_bytes())
    }

    /// Wraps raw id bytes.
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// The raw id bytes.
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// A short, human-friendly form such as `3F9A-C210`, for display only.
    ///
    /// Never use it to identify a device: it is far too short to be unique.
    pub fn short(&self) -> String {
        let hex = hex::encode_upper(&self.0[..4]);
        format!("{}-{}", &hex[..4], &hex[4..])
    }
}

impl fmt::Display for DeviceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&hex::encode(self.0))
    }
}

impl fmt::Debug for DeviceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DeviceId({})", self.short())
    }
}

impl FromStr for DeviceId {
    type Err = Error;

    /// Parses the 64-character lowercase or uppercase hex form.
    fn from_str(s: &str) -> Result<Self, Error> {
        let mut bytes = [0u8; 32];
        hex::decode_to_slice(s, &mut bytes).map_err(|_| Error::InvalidDeviceId)?;
        Ok(Self(bytes))
    }
}

impl From<DeviceId> for String {
    fn from(id: DeviceId) -> Self {
        id.to_string()
    }
}

impl TryFrom<String> for DeviceId {
    type Error = Error;

    fn try_from(s: String) -> Result<Self, Error> {
        s.parse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trip() {
        let id = DeviceId::from_certificate(b"example certificate");
        let parsed: DeviceId = id.to_string().parse().unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn short_form_has_two_groups() {
        let id = DeviceId::from_bytes([0xab; 32]);
        assert_eq!(id.short(), "ABAB-ABAB");
    }

    #[test]
    fn rejects_bad_input() {
        assert!("zz".parse::<DeviceId>().is_err());
        assert!("ab".repeat(31).parse::<DeviceId>().is_err());
    }
}
