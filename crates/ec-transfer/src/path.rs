//! Validation of file paths received from another device.
//!
//! A peer chooses the names of the files it sends, so every name is treated as hostile:
//! it must not escape the receive folder, overwrite anything, or rely on characters that
//! some operating system interprets specially. [`RelPath`] only accepts paths that are
//! safe on macOS, Windows, Linux and Android alike.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// Most directory levels in a received path.
pub const MAX_DEPTH: usize = 32;
/// Longest single name, in bytes (the common file system limit).
pub const MAX_COMPONENT_BYTES: usize = 255;
/// Longest whole relative path, in bytes.
pub const MAX_PATH_BYTES: usize = 1024;

/// A validated relative path, for example `["Reports", "2026", "q3.pdf"]`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "Vec<String>", into = "Vec<String>")]
pub struct RelPath(Vec<String>);

impl RelPath {
    /// Validates `components`. See the module documentation for the rules.
    pub fn new(components: Vec<String>) -> Result<Self> {
        if components.is_empty() || components.len() > MAX_DEPTH {
            return Err(Error::UnsafePath("wrong number of path components"));
        }
        let total: usize = components.iter().map(|c| c.len() + 1).sum();
        if total > MAX_PATH_BYTES {
            return Err(Error::UnsafePath("path too long"));
        }
        for component in &components {
            check_component(component)?;
        }
        Ok(Self(components))
    }

    /// The path components.
    pub fn components(&self) -> &[String] {
        &self.0
    }

    /// The last component (the file name).
    pub fn file_name(&self) -> &str {
        // `new` guarantees at least one component.
        self.0.last().map(String::as_str).unwrap_or_default()
    }

    /// The path as a relative `PathBuf` for this platform.
    pub fn to_path_buf(&self) -> PathBuf {
        self.0.iter().collect()
    }
}

impl std::fmt::Display for RelPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0.join("/"))
    }
}

impl TryFrom<Vec<String>> for RelPath {
    type Error = Error;

    fn try_from(components: Vec<String>) -> Result<Self> {
        Self::new(components)
    }
}

impl From<RelPath> for Vec<String> {
    fn from(path: RelPath) -> Self {
        path.0
    }
}

fn check_component(c: &str) -> Result<()> {
    if c.is_empty() || c == "." || c == ".." {
        return Err(Error::UnsafePath("empty, '.' or '..' component"));
    }
    if c.len() > MAX_COMPONENT_BYTES {
        return Err(Error::UnsafePath("name too long"));
    }
    if c.chars().any(is_forbidden_char) {
        return Err(Error::UnsafePath("forbidden character in name"));
    }
    // Windows silently drops trailing dots and spaces, so "a.txt." would become "a.txt".
    if c.ends_with('.') || c.ends_with(' ') {
        return Err(Error::UnsafePath("name ends with a dot or space"));
    }
    if is_windows_reserved(c) {
        return Err(Error::UnsafePath("reserved device name"));
    }
    Ok(())
}

fn is_forbidden_char(ch: char) -> bool {
    matches!(ch, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
        || ch.is_control()
        // Bidirectional overrides can disguise an extension ("txt.exe" shown as "exe.txt").
        || matches!(ch, '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}' | '\u{200E}' | '\u{200F}')
}

/// `CON`, `NUL`, `COM1`, `LPT9`, ... with or without an extension, any case.
fn is_windows_reserved(c: &str) -> bool {
    let stem = c
        .split('.')
        .next()
        .unwrap_or(c)
        .trim_end()
        .to_ascii_uppercase();
    match stem.as_str() {
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$" => true,
        _ => {
            // `strip_prefix` respects character boundaries; slicing by byte count
            // panics on multi-byte names (found by fuzzing).
            let digit = stem
                .strip_prefix("COM")
                .or_else(|| stem.strip_prefix("LPT"));
            digit.is_some_and(|d| {
                matches!(
                    d,
                    "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                )
            })
        }
    }
}

/// Creates the folders of `rel` inside `root` and returns a file path that does not exist
/// yet, adding ` (1)`, ` (2)`, ... to the name when needed. Never follows symbolic links:
/// if any folder on the way is a link or a file, it fails.
pub fn unique_destination(root: &Path, rel: &RelPath) -> Result<PathBuf> {
    let components = rel.components();
    let mut dir = root.to_path_buf();
    for name in &components[..components.len() - 1] {
        dir.push(name);
        ensure_real_dir(&dir)?;
    }

    let file_name = rel.file_name();
    let (stem, ext) = match file_name.rfind('.') {
        Some(i) if i > 0 => (&file_name[..i], &file_name[i..]),
        _ => (file_name, ""),
    };
    for n in 0..10_000u32 {
        let candidate = if n == 0 {
            dir.join(file_name)
        } else {
            dir.join(format!("{stem} ({n}){ext}"))
        };
        match fs::symlink_metadata(&candidate) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(candidate),
            Err(e) => return Err(Error::io("check", &candidate, e)),
            Ok(_) => continue,
        }
    }
    Err(Error::UnsafePath("too many files with the same name"))
}

/// Makes sure `dir` is a real directory (not a symbolic link), creating it if needed.
pub(crate) fn ensure_real_dir(dir: &Path) -> Result<()> {
    match fs::symlink_metadata(dir) {
        Ok(meta) if meta.is_dir() => Ok(()),
        Ok(_) => Err(Error::UnsafePath("a path component is a file or a link")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir(dir).map_err(|e| Error::io("create directory", dir, e))
        }
        Err(e) => Err(Error::io("check", dir, e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rel(parts: &[&str]) -> Result<RelPath> {
        RelPath::new(parts.iter().map(|s| s.to_string()).collect())
    }

    #[test]
    fn accepts_normal_names() {
        for ok in [
            &["a.pdf"][..],
            &["Photos", "IMG 0001.HEIC"],
            &["Ünïcødé", "资料.docx"],
            &[".hidden"],
        ] {
            assert!(rel(ok).is_ok(), "{ok:?}");
        }
    }

    #[test]
    fn rejects_traversal_and_special_names() {
        for bad in [
            &[][..],
            &[""],
            &[".."],
            &["a", "..", "b"],
            &["/etc/passwd"],
            &["C:"],
            &["a\\..\\b"],
            &["nul"],
            &["CON.txt"],
            &["com1.log"],
            &["LPT9"],
            &["file."],
            &["file "],
            &["a\u{0}b"],
            &["a\nb"],
            &["invoice\u{202E}fdp.exe"],
        ] {
            assert!(rel(bad).is_err(), "accepted {bad:?}");
        }
        assert!(rel(&vec!["d"; MAX_DEPTH + 1]).is_err());
    }

    #[test]
    fn multi_byte_names_do_not_panic() {
        // Regression: found by the rel_path fuzz target.
        assert!(rel(&["~=\u{5be}"]).is_ok());
        assert!(rel(&["é"]).is_ok());
        assert!(rel(&["COMé"]).is_ok());
        assert!(rel(&["x".repeat(MAX_COMPONENT_BYTES + 1).as_str()]).is_err());
    }

    #[test]
    fn deserializing_validates() {
        let bytes = postcard::to_stdvec(&vec!["..".to_owned(), "x".to_owned()]).unwrap();
        assert!(postcard::from_bytes::<RelPath>(&bytes).is_err());
        let bytes = postcard::to_stdvec(&vec!["ok.txt".to_owned()]).unwrap();
        assert!(postcard::from_bytes::<RelPath>(&bytes).is_ok());
    }

    #[test]
    fn destination_never_overwrites() {
        let root = tempfile::tempdir().unwrap();
        let path = rel(&["docs", "report.pdf"]).unwrap();
        let first = unique_destination(root.path(), &path).unwrap();
        fs::write(&first, b"1").unwrap();
        let second = unique_destination(root.path(), &path).unwrap();
        assert_eq!(second.file_name().unwrap(), "report (1).pdf");
        assert!(second.starts_with(root.path().join("docs")));
    }

    #[cfg(unix)]
    #[test]
    fn destination_refuses_symlinked_folders() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(outside.path(), root.path().join("docs")).unwrap();
        let path = rel(&["docs", "report.pdf"]).unwrap();
        assert!(unique_destination(root.path(), &path).is_err());
    }
}
