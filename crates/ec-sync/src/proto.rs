//! Messages on the sync control stream (kind [`SYNC_KIND`]). See `docs/PROTOCOL.md`.
//!
//! Everything a peer sends is checked by [`SyncMessage::validate`] before use.

use ec_net::{RecvStream, SendStream, read_msg, write_msg};
use ec_transfer::manifest::MAX_CHUNK_SIZE;
use ec_transfer::proto::MAX_HASHES_PER_MSG;
use ec_transfer::{FileEntry, RelPath};
use serde::{Deserialize, Serialize};

use crate::{Error, LinkId, Result, VersionVector};

/// Stream kind of the sync control stream (bidirectional).
pub const SYNC_KIND: u8 = 4;
/// Longest file name, in bytes.
pub const MAX_NAME_BYTES: usize = 255;
/// Most devices in one version vector.
pub const MAX_VECTOR_DEVICES: usize = 16;
/// Most entries in one [`SyncMessage::Summary`] message. With
/// [`MAX_VECTOR_DEVICES`] this keeps a message well under the 64 KiB frame limit.
pub const SUMMARY_BATCH: usize = 32;

/// The size of a version; its chunk hashes follow in [`SyncMessage::Hashes`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentHeader {
    /// Size in bytes.
    pub size: u64,
    /// Size of every chunk except possibly the last.
    pub chunk_size: u32,
}

/// Messages on the sync control stream.
///
/// Field meanings are given in each variant's description.
#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncMessage {
    /// A new linked file, named `name` on the sending device.
    Link {
        id: LinkId,
        name: String,
        version: VersionVector,
        content: ContentHeader,
    },
    /// A new version made on the sending device.
    Changed {
        id: LinkId,
        version: VersionVector,
        content: ContentHeader,
    },
    /// The version the user chose in a conflict.
    Resolved {
        id: LinkId,
        version: VersionVector,
        content: ContentHeader,
    },
    /// Chunk hashes `start..start + hashes.len()` of the version just announced.
    Hashes {
        id: LinkId,
        start: u32,
        hashes: Vec<[u8; 32]>,
    },
    /// The versions of the files both devices share; `last` on the final batch.
    Summary {
        entries: Vec<(LinkId, VersionVector)>,
        last: bool,
    },
    /// The sending device left the link.
    Unlinked { id: LinkId },
    /// Answer to `Link`, `Changed` and `Resolved`: whether to send the content.
    Reply { id: LinkId, want: bool },
}

impl SyncMessage {
    /// The link a message is about, if it is about one.
    pub fn id(&self) -> Option<LinkId> {
        match self {
            SyncMessage::Link { id, .. }
            | SyncMessage::Changed { id, .. }
            | SyncMessage::Resolved { id, .. }
            | SyncMessage::Hashes { id, .. }
            | SyncMessage::Unlinked { id }
            | SyncMessage::Reply { id, .. } => Some(*id),
            SyncMessage::Summary { .. } => None,
        }
    }

    /// The content header of a message that announces a version.
    pub fn content(&self) -> Option<ContentHeader> {
        match self {
            SyncMessage::Link { content, .. }
            | SyncMessage::Changed { content, .. }
            | SyncMessage::Resolved { content, .. } => Some(*content),
            _ => None,
        }
    }

    /// Checks a message received from a peer and returns it.
    pub fn validate(self) -> Result<Self> {
        match &self {
            SyncMessage::Link {
                name,
                version,
                content,
                ..
            } => {
                check_name(name)?;
                check_version(version)?;
                check_content(content)?;
            }
            SyncMessage::Changed {
                version, content, ..
            }
            | SyncMessage::Resolved {
                version, content, ..
            } => {
                check_version(version)?;
                check_content(content)?;
            }
            SyncMessage::Hashes { hashes, .. } => {
                if hashes.len() > MAX_HASHES_PER_MSG {
                    return Err(Error::Refused("too many hashes in one message"));
                }
            }
            SyncMessage::Summary { entries, .. } => {
                if entries.len() > SUMMARY_BATCH {
                    return Err(Error::Refused("summary batch too large"));
                }
                for (_, version) in entries {
                    check_version(version)?;
                }
            }
            SyncMessage::Unlinked { .. } | SyncMessage::Reply { .. } => {}
        }
        Ok(self)
    }
}

/// A file name must be a single, safe path component, as received files are.
fn check_name(name: &str) -> Result<()> {
    if name.len() > MAX_NAME_BYTES || RelPath::new(vec![name.to_owned()]).is_err() {
        return Err(Error::Refused("unsafe file name"));
    }
    Ok(())
}

fn check_version(version: &VersionVector) -> Result<()> {
    if version.len() > MAX_VECTOR_DEVICES {
        return Err(Error::Refused("version vector too wide"));
    }
    Ok(())
}

fn check_content(content: &ContentHeader) -> Result<()> {
    if !(64 * 1024..=MAX_CHUNK_SIZE).contains(&content.chunk_size) {
        return Err(Error::Refused("chunk size out of range"));
    }
    if chunk_count(content) > ec_transfer::manifest::MAX_CHUNKS_PER_FILE {
        return Err(Error::Refused("file has too many chunks"));
    }
    Ok(())
}

fn chunk_count(content: &ContentHeader) -> u64 {
    FileEntry::chunk_count_for(content.size, content.chunk_size)
}

/// Sends a message that announces a version, followed by its chunk hashes.
pub async fn send_content(
    send: &mut SendStream,
    msg: &SyncMessage,
    hashes: &[[u8; 32]],
) -> Result<()> {
    let id = msg.id().ok_or(Error::Refused("message has no link"))?;
    write_sync(send, msg).await?;
    for (n, batch) in hashes.chunks(MAX_HASHES_PER_MSG).enumerate() {
        let part = SyncMessage::Hashes {
            id,
            start: (n * MAX_HASHES_PER_MSG) as u32,
            hashes: batch.to_vec(),
        };
        write_sync(send, &part).await?;
    }
    Ok(())
}

/// Reads the chunk hashes that follow `first` (a message that announces a version)
/// and returns it with them. Other messages are returned with no hashes.
pub async fn read_content(
    recv: &mut RecvStream,
    first: SyncMessage,
) -> Result<(SyncMessage, Vec<[u8; 32]>)> {
    let (Some(id), Some(content)) = (first.id(), first.content()) else {
        return Ok((first, Vec::new()));
    };
    // Validated above, so the count is bounded by MAX_CHUNKS_PER_FILE.
    let expected = chunk_count(&content) as usize;
    let mut hashes = Vec::with_capacity(expected);
    while hashes.len() < expected {
        match read_sync(recv).await? {
            SyncMessage::Hashes {
                id: of,
                start,
                hashes: part,
            } if of == id
                && start as usize == hashes.len()
                && hashes.len() + part.len() <= expected =>
            {
                hashes.extend(part);
            }
            _ => return Err(Error::Refused("unexpected message while reading hashes")),
        }
    }
    Ok((first, hashes))
}

/// Reads one message and validates it.
pub async fn read_sync(recv: &mut RecvStream) -> Result<SyncMessage> {
    read_msg::<SyncMessage>(recv).await?.validate()
}

/// Writes one message.
pub async fn write_sync(send: &mut SendStream, msg: &SyncMessage) -> Result<()> {
    Ok(write_msg(send, msg).await?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ec_core::DeviceId;

    const A: DeviceId = DeviceId::from_bytes([1; 32]);
    const ID: LinkId = [5; 16];
    const HEADER: ContentHeader = ContentHeader {
        size: 10,
        chunk_size: 64 * 1024,
    };

    fn link(name: &str) -> SyncMessage {
        SyncMessage::Link {
            id: ID,
            name: name.into(),
            version: [(A, 1)].into_iter().collect(),
            content: HEADER,
        }
    }

    #[test]
    fn names_are_checked() {
        for bad in ["../x", "a/b", "", "..", &"a".repeat(MAX_NAME_BYTES + 1)] {
            assert!(link(bad).validate().is_err(), "accepted {bad:?}");
        }
        assert_eq!(link("Rapor.pdf").validate().unwrap(), link("Rapor.pdf"));
    }

    #[test]
    fn bounds_are_checked() {
        let small_chunks = SyncMessage::Changed {
            id: ID,
            version: VersionVector::default(),
            content: ContentHeader {
                size: 10,
                chunk_size: 1024,
            },
        };
        assert!(small_chunks.validate().is_err());

        let too_many_chunks = SyncMessage::Changed {
            id: ID,
            version: VersionVector::default(),
            content: ContentHeader {
                size: u64::MAX,
                chunk_size: 64 * 1024,
            },
        };
        assert!(too_many_chunks.validate().is_err());

        let hashes = SyncMessage::Hashes {
            id: ID,
            start: 0,
            hashes: vec![[0; 32]; MAX_HASHES_PER_MSG + 1],
        };
        assert!(hashes.validate().is_err());

        let summary = SyncMessage::Summary {
            entries: vec![(ID, VersionVector::default()); SUMMARY_BATCH + 1],
            last: true,
        };
        assert!(summary.validate().is_err());

        let devices: VersionVector = (0..=MAX_VECTOR_DEVICES as u8)
            .map(|n| (DeviceId::from_bytes([n; 32]), 1))
            .collect();
        let wide = SyncMessage::Changed {
            id: ID,
            version: devices,
            content: HEADER,
        };
        assert!(wide.validate().is_err());
    }

    #[test]
    fn summary_batch_fits_in_a_frame() {
        let full: VersionVector = (0..MAX_VECTOR_DEVICES as u8)
            .map(|n| (DeviceId::from_bytes([n; 32]), u64::MAX))
            .collect();
        let summary = SyncMessage::Summary {
            entries: vec![(ID, full); SUMMARY_BATCH],
            last: false,
        };
        let len = postcard::to_stdvec(&summary).unwrap().len();
        assert!(len < ec_net::MAX_FRAME_LEN, "{len} bytes");
    }

    #[test]
    fn roundtrip() {
        let version: VersionVector = [(A, 3)].into_iter().collect();
        for msg in [
            link("Rapor.pdf"),
            SyncMessage::Changed {
                id: ID,
                version: version.clone(),
                content: HEADER,
            },
            SyncMessage::Resolved {
                id: ID,
                version: version.clone(),
                content: HEADER,
            },
            SyncMessage::Hashes {
                id: ID,
                start: 0,
                hashes: vec![[9; 32]],
            },
            SyncMessage::Summary {
                entries: vec![(ID, version)],
                last: true,
            },
            SyncMessage::Unlinked { id: ID },
            SyncMessage::Reply { id: ID, want: true },
        ] {
            let bytes = postcard::to_stdvec(&msg).unwrap();
            assert_eq!(ec_net::decode::<SyncMessage>(&bytes).unwrap(), msg);
        }
    }
}
