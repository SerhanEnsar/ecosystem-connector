//! User settings that the core itself needs, stored next to the trust store.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::fsutil::{read_optional, write_private};
use crate::{Error, Result};

const SETTINGS_FILE: &str = "settings.json";

/// The saved settings. Every field is optional, so older and newer files both load.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    /// Folder chosen for received files. `None` means the app's default folder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receive_dir: Option<PathBuf>,
}

impl Settings {
    /// Loads the settings from `dir`, or the defaults when there are none yet.
    pub fn load(dir: &Path) -> Result<Self> {
        let path = dir.join(SETTINGS_FILE);
        match read_optional(&path)? {
            Some(data) => serde_json::from_slice(&data).map_err(|e| Error::Corrupt {
                path,
                reason: e.to_string(),
            }),
            None => Ok(Self::default()),
        }
    }

    /// Saves the settings to `dir`.
    pub fn save(&self, dir: &Path) -> Result<()> {
        let path = dir.join(SETTINGS_FILE);
        let data = serde_json::to_vec_pretty(self).map_err(|e| Error::Corrupt {
            path: path.clone(),
            reason: e.to_string(),
        })?;
        write_private(&path, &data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_gives_defaults() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(Settings::load(dir.path()).unwrap(), Settings::default());
    }

    #[test]
    fn settings_are_persisted() {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings {
            receive_dir: Some(PathBuf::from("/somewhere/else")),
        };
        settings.save(dir.path()).unwrap();
        assert_eq!(Settings::load(dir.path()).unwrap(), settings);
    }

    #[test]
    fn unknown_fields_are_ignored() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(SETTINGS_FILE), br#"{"later": true}"#).unwrap();
        assert_eq!(Settings::load(dir.path()).unwrap(), Settings::default());
    }
}
