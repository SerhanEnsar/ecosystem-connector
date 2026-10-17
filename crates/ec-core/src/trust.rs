use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::fsutil::{read_optional, write_private};
use crate::{DeviceId, Error, Result, sanitize_device_name};

const TRUST_FILE: &str = "trusted-devices.json";

/// A device this device has paired with.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrustedDevice {
    /// The paired device's id (hash of its certificate).
    pub id: DeviceId,
    /// Its name at pairing time.
    pub name: String,
    /// When pairing completed, in seconds since the Unix epoch.
    pub paired_at: u64,
}

#[derive(Default, Serialize, Deserialize)]
struct TrustFile {
    version: u32,
    devices: Vec<TrustedDevice>,
}

/// The set of paired devices, stored as JSON next to the identity.
///
/// Only devices in this store can open a regular connection to this device.
#[derive(Debug)]
pub struct TrustStore {
    path: PathBuf,
    devices: BTreeMap<DeviceId, TrustedDevice>,
}

impl TrustStore {
    /// Loads the store from `dir`, or starts an empty one.
    pub fn load(dir: &Path) -> Result<Self> {
        let path = dir.join(TRUST_FILE);
        let devices = match read_optional(&path)? {
            Some(data) => {
                let file: TrustFile =
                    serde_json::from_slice(&data).map_err(|e| Error::Corrupt {
                        path: path.clone(),
                        reason: e.to_string(),
                    })?;
                file.devices.into_iter().map(|d| (d.id, d)).collect()
            }
            None => BTreeMap::new(),
        };
        Ok(Self { path, devices })
    }

    /// Adds or replaces a device and saves the store.
    pub fn add(&mut self, id: DeviceId, name: &str) -> Result<TrustedDevice> {
        let device = TrustedDevice {
            id,
            name: sanitize_device_name(name),
            paired_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        };
        self.devices.insert(id, device.clone());
        self.save()?;
        Ok(device)
    }

    /// Removes a device and saves the store. Returns whether it was present.
    pub fn remove(&mut self, id: &DeviceId) -> Result<bool> {
        let removed = self.devices.remove(id).is_some();
        if removed {
            self.save()?;
        }
        Ok(removed)
    }

    /// Looks up a device.
    pub fn get(&self, id: &DeviceId) -> Option<&TrustedDevice> {
        self.devices.get(id)
    }

    /// Whether `id` is paired.
    pub fn contains(&self, id: &DeviceId) -> bool {
        self.devices.contains_key(id)
    }

    /// All paired devices, ordered by id.
    pub fn devices(&self) -> impl Iterator<Item = &TrustedDevice> {
        self.devices.values()
    }

    /// Finds a paired device whose id starts with `prefix` (hex, with or without the
    /// dash of the short form). Returns `None` when there is no match or more than one.
    pub fn find_by_prefix(&self, prefix: &str) -> Option<&TrustedDevice> {
        let prefix = prefix.replace('-', "").to_ascii_lowercase();
        if prefix.is_empty() {
            return None;
        }
        let mut matches = self
            .devices
            .values()
            .filter(|d| d.id.to_string().starts_with(&prefix));
        let first = matches.next()?;
        matches.next().is_none().then_some(first)
    }

    fn save(&self) -> Result<()> {
        let file = TrustFile {
            version: 1,
            devices: self.devices.values().cloned().collect(),
        };
        let data = serde_json::to_vec_pretty(&file).map_err(|e| Error::Corrupt {
            path: self.path.clone(),
            reason: e.to_string(),
        })?;
        write_private(&self.path, &data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(byte: u8) -> DeviceId {
        DeviceId::from_bytes([byte; 32])
    }

    #[test]
    fn devices_are_persisted() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = TrustStore::load(dir.path()).unwrap();
        store.add(id(1), "Phone").unwrap();
        store.add(id(2), "Tablet").unwrap();

        let mut reloaded = TrustStore::load(dir.path()).unwrap();
        assert_eq!(reloaded.devices().count(), 2);
        assert!(reloaded.remove(&id(1)).unwrap());
        assert!(!TrustStore::load(dir.path()).unwrap().contains(&id(1)));
    }

    #[test]
    fn prefix_lookup_requires_a_unique_match() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = TrustStore::load(dir.path()).unwrap();
        store.add(DeviceId::from_bytes([0xab; 32]), "A").unwrap();
        let mut other = [0xab; 32];
        other[31] = 0;
        store.add(DeviceId::from_bytes(other), "B").unwrap();
        store.add(id(0xcd), "C").unwrap();

        assert!(store.find_by_prefix("abab").is_none(), "ambiguous");
        assert_eq!(store.find_by_prefix("CDCD-CDCD").unwrap().name, "C");
        assert!(store.find_by_prefix("").is_none());
    }

    #[test]
    fn corrupt_file_is_reported() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(TRUST_FILE), b"{not json").unwrap();
        assert!(TrustStore::load(dir.path()).is_err());
    }
}
