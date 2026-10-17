//! Describing files as chunks with BLAKE3 hashes.

use std::fs::File;
use std::path::{Path, PathBuf};

use crate::fileio::read_exact_at;
use crate::path::RelPath;
use crate::{Error, Result};

/// Default chunk size. Large enough to keep per-chunk overhead negligible, small enough
/// that a lost chunk costs little to resend.
pub const DEFAULT_CHUNK_SIZE: u32 = 4 * 1024 * 1024;
/// Largest chunk size; used only for very large files.
pub const MAX_CHUNK_SIZE: u32 = 64 * 1024 * 1024;
/// Most chunks in one file. Bounds the size of a manifest.
pub const MAX_CHUNKS_PER_FILE: u64 = 8192;
/// Most files in one transfer.
pub const MAX_FILES: usize = 100_000;

/// The BLAKE3 hash of one chunk.
pub type ChunkHash = [u8; 32];

/// One file of a transfer, as both sides see it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileEntry {
    /// Where the file goes, relative to the receive folder.
    pub path: RelPath,
    /// Size in bytes.
    pub size: u64,
    /// Size of every chunk except possibly the last.
    pub chunk_size: u32,
    /// Hash of each chunk, in order.
    pub hashes: Vec<ChunkHash>,
}

impl FileEntry {
    /// Number of chunks a file of `size` bytes has with `chunk_size`.
    pub fn chunk_count_for(size: u64, chunk_size: u32) -> u64 {
        size.div_ceil(u64::from(chunk_size))
    }

    /// Number of chunks.
    pub fn chunk_count(&self) -> u32 {
        self.hashes.len() as u32
    }

    /// Byte offset of chunk `index`.
    pub fn chunk_offset(&self, index: u32) -> u64 {
        u64::from(index) * u64::from(self.chunk_size)
    }

    /// Length of chunk `index` in bytes.
    pub fn chunk_len(&self, index: u32) -> u32 {
        let remaining = self.size - self.chunk_offset(index);
        remaining.min(u64::from(self.chunk_size)) as u32
    }

    /// Checks that size, chunk size and hash count agree.
    pub fn validate(&self) -> Result<()> {
        if !(64 * 1024..=MAX_CHUNK_SIZE).contains(&self.chunk_size) {
            return Err(Error::Protocol("chunk size out of range"));
        }
        let count = Self::chunk_count_for(self.size, self.chunk_size);
        if count > MAX_CHUNKS_PER_FILE {
            return Err(Error::TooLarge("file has too many chunks"));
        }
        if count != self.hashes.len() as u64 {
            return Err(Error::Protocol("chunk hash count does not match the size"));
        }
        Ok(())
    }
}

/// The chunk size used for a file of `size` bytes.
pub fn chunk_size_for(size: u64) -> Result<u32> {
    let mut chunk = DEFAULT_CHUNK_SIZE;
    while FileEntry::chunk_count_for(size, chunk) > MAX_CHUNKS_PER_FILE {
        chunk = chunk
            .checked_mul(2)
            .filter(|c| *c <= MAX_CHUNK_SIZE)
            .ok_or(Error::TooLarge("file is larger than 512 GiB"))?;
    }
    Ok(chunk)
}

/// A local file to send.
#[derive(Clone, Debug)]
pub struct SourceFile {
    /// Where it is on this device.
    pub local: PathBuf,
    /// How it is described to the receiver.
    pub entry: FileEntry,
}

/// Lists the files under `paths`. Folders are walked recursively and keep their own name
/// as the first path component. Symbolic links are skipped, never followed.
pub fn collect(paths: &[PathBuf]) -> Result<Vec<(PathBuf, RelPath, u64)>> {
    let mut out = Vec::new();
    for path in paths {
        let meta = std::fs::symlink_metadata(path).map_err(|e| Error::io("read", path, e))?;
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or(Error::UnsafePath("file name is not valid UTF-8"))?
            .to_owned();
        if meta.is_dir() {
            walk(path, &mut vec![name], &mut out)?;
        } else if meta.is_file() {
            out.push((path.clone(), RelPath::new(vec![name])?, meta.len()));
        }
        if out.len() > MAX_FILES {
            return Err(Error::TooLarge("too many files"));
        }
    }
    Ok(out)
}

fn walk(
    dir: &Path,
    prefix: &mut Vec<String>,
    out: &mut Vec<(PathBuf, RelPath, u64)>,
) -> Result<()> {
    let entries = std::fs::read_dir(dir).map_err(|e| Error::io("list", dir, e))?;
    let mut entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let path = entry.path();
        let Ok(meta) = std::fs::symlink_metadata(&path) else {
            continue;
        };
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        prefix.push(name);
        if meta.is_dir() {
            walk(&path, prefix, out)?;
        } else if meta.is_file() {
            out.push((path, RelPath::new(prefix.clone())?, meta.len()));
            if out.len() > MAX_FILES {
                return Err(Error::TooLarge("too many files"));
            }
        }
        prefix.pop();
    }
    Ok(())
}

/// Hashes every chunk of `path`, using several threads.
pub fn hash_chunks(path: &Path, size: u64, chunk_size: u32) -> Result<Vec<ChunkHash>> {
    let count = FileEntry::chunk_count_for(size, chunk_size) as usize;
    let mut hashes = vec![[0u8; 32]; count];
    if count == 0 {
        return Ok(hashes);
    }
    let threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .min(8)
        .min(count);

    let results: Vec<Result<Vec<(usize, ChunkHash)>>> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..threads)
            .map(|t| {
                scope.spawn(move || -> Result<Vec<(usize, ChunkHash)>> {
                    let file = File::open(path).map_err(|e| Error::io("open", path, e))?;
                    let mut buf = vec![0u8; chunk_size as usize];
                    let mut out = Vec::new();
                    for index in (t..count).step_by(threads) {
                        let offset = index as u64 * u64::from(chunk_size);
                        let len = (size - offset).min(u64::from(chunk_size)) as usize;
                        read_exact_at(&file, &mut buf[..len], offset)
                            .map_err(|e| Error::io("read", path, e))?;
                        out.push((index, *blake3::hash(&buf[..len]).as_bytes()));
                    }
                    Ok(out)
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| {
                h.join()
                    .unwrap_or(Err(Error::Protocol("hashing thread panicked")))
            })
            .collect()
    });
    for part in results {
        for (index, hash) in part? {
            hashes[index] = hash;
        }
    }
    Ok(hashes)
}

/// Collects and hashes the files under `paths`.
pub fn prepare(paths: &[PathBuf]) -> Result<Vec<SourceFile>> {
    collect(paths)?
        .into_iter()
        .map(|(local, path, size)| {
            let chunk_size = chunk_size_for(size)?;
            let hashes = hash_chunks(&local, size, chunk_size)?;
            Ok(SourceFile {
                local,
                entry: FileEntry {
                    path,
                    size,
                    chunk_size,
                    hashes,
                },
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_size_grows_for_huge_files() {
        assert_eq!(chunk_size_for(0).unwrap(), DEFAULT_CHUNK_SIZE);
        assert_eq!(chunk_size_for(32 << 30).unwrap(), DEFAULT_CHUNK_SIZE);
        assert_eq!(chunk_size_for(40 << 30).unwrap(), DEFAULT_CHUNK_SIZE * 2);
        assert!(chunk_size_for(600 << 30).is_err());
    }

    #[test]
    fn chunk_lengths_cover_the_file() {
        let entry = FileEntry {
            path: RelPath::new(vec!["a".into()]).unwrap(),
            size: 10 * 1024 * 1024 + 5,
            chunk_size: DEFAULT_CHUNK_SIZE,
            hashes: vec![[0; 32]; 3],
        };
        entry.validate().unwrap();
        let total: u64 = (0..3).map(|i| u64::from(entry.chunk_len(i))).sum();
        assert_eq!(total, entry.size);
        assert_eq!(entry.chunk_len(2), 2 * 1024 * 1024 + 5);
    }

    #[test]
    fn hashes_match_single_threaded_hashing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("f.bin");
        let data: Vec<u8> = (0..300_000u32).map(|i| (i % 251) as u8).collect();
        std::fs::write(&path, &data).unwrap();
        let hashes = hash_chunks(&path, data.len() as u64, 64 * 1024).unwrap();
        assert_eq!(hashes.len(), 5);
        for (i, chunk) in data.chunks(64 * 1024).enumerate() {
            assert_eq!(hashes[i], *blake3::hash(chunk).as_bytes());
        }
    }

    #[cfg(unix)]
    #[test]
    fn collect_walks_folders_and_skips_links() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("Docs");
        std::fs::create_dir_all(root.join("sub")).unwrap();
        std::fs::write(root.join("a.txt"), b"a").unwrap();
        std::fs::write(root.join("sub/b.txt"), b"bb").unwrap();
        std::os::unix::fs::symlink("/etc/hosts", root.join("link")).unwrap();
        let files = collect(&[root]).unwrap();
        let names: Vec<String> = files.iter().map(|(_, p, _)| p.to_string()).collect();
        assert_eq!(names, ["Docs/a.txt", "Docs/sub/b.txt"]);
    }
}
