//! Wire messages of the transfer protocol. See `docs/PROTOCOL.md`.

use serde::{Deserialize, Serialize};

use crate::manifest::ChunkHash;
use crate::path::RelPath;

/// Stream kind of a transfer's control stream (bidirectional).
pub const CONTROL_KIND: u8 = 1;
/// Stream kind of a chunk stream (unidirectional, one chunk per stream).
pub const CHUNK_KIND: u8 = 2;

/// Identifies one transfer.
pub type TransferId = [u8; 16];

/// Most chunk hashes in one [`Control::Hashes`] message (32 KiB of hashes).
pub const MAX_HASHES_PER_MSG: usize = 1024;
/// Most ranges in one [`Control::Need`] message.
pub const MAX_RANGES_PER_MSG: usize = 2048;

/// Messages on the control stream.
///
/// Field meanings are given in each variant's description.
#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Control {
    /// Sender → receiver: starts an offer of `files` files, `total_bytes` in all.
    Offer {
        id: TransferId,
        files: u32,
        total_bytes: u64,
    },
    /// Sender → receiver: one file of the offer, in order.
    File {
        index: u32,
        path: RelPath,
        size: u64,
        chunk_size: u32,
    },
    /// Sender → receiver: chunk hashes `start..start + hashes.len()` of `file`.
    Hashes {
        file: u32,
        start: u32,
        hashes: Vec<ChunkHash>,
    },
    /// Sender → receiver: the offer is complete.
    OfferEnd,
    /// Receiver → sender: the offer is declined.
    Rejected { reason: String },
    /// Receiver → sender: chunks of `file` it still needs, as half-open ranges.
    Need { file: u32, ranges: Vec<(u32, u32)> },
    /// Receiver → sender: all needs have been listed.
    NeedEnd,
    /// Receiver → sender: chunk received and verified (`ok`), or rejected.
    Ack { file: u32, chunk: u32, ok: bool },
    /// Receiver → sender: `file` was saved.
    FileDone { file: u32 },
    /// Receiver → sender: every file was saved.
    Done,
    /// Sender → receiver, optional, before `OfferEnd`: open the files once saved (a
    /// fling), at `page` if given.
    Hint { open: bool, page: Option<u32> },
}

/// Asks the receiver to open the files once they are saved.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OpenHint {
    /// The page to show, for documents that have pages.
    pub page: Option<u32>,
}

/// Header at the start of every chunk stream, followed by exactly the chunk's bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChunkHeader {
    /// The transfer.
    pub id: TransferId,
    /// File index in the offer.
    pub file: u32,
    /// Chunk index in the file.
    pub chunk: u32,
}

impl ChunkHeader {
    /// Encoded size in bytes.
    pub const LEN: usize = 24;

    /// Encodes the header (little-endian integers).
    pub fn encode(&self) -> [u8; Self::LEN] {
        let mut out = [0u8; Self::LEN];
        out[..16].copy_from_slice(&self.id);
        out[16..20].copy_from_slice(&self.file.to_le_bytes());
        out[20..].copy_from_slice(&self.chunk.to_le_bytes());
        out
    }

    /// Decodes a header. Every byte pattern is a syntactically valid header; the
    /// receiver checks the values against the transfer.
    pub fn decode(bytes: &[u8; Self::LEN]) -> Self {
        let mut id = [0u8; 16];
        id.copy_from_slice(&bytes[..16]);
        let mut word = [0u8; 4];
        word.copy_from_slice(&bytes[16..20]);
        let file = u32::from_le_bytes(word);
        word.copy_from_slice(&bytes[20..]);
        Self {
            id,
            file,
            chunk: u32::from_le_bytes(word),
        }
    }
}

/// Compresses sorted chunk indices into half-open ranges: `[0,1,2,5]` → `[(0,3),(5,6)]`.
pub fn to_ranges(indices: impl IntoIterator<Item = u32>) -> Vec<(u32, u32)> {
    let mut ranges: Vec<(u32, u32)> = Vec::new();
    for i in indices {
        match ranges.last_mut() {
            Some((_, end)) if *end == i => *end += 1,
            _ => ranges.push((i, i + 1)),
        }
    }
    ranges
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_round_trip() {
        let header = ChunkHeader {
            id: [7; 16],
            file: 3,
            chunk: 70_000,
        };
        assert_eq!(ChunkHeader::decode(&header.encode()), header);
    }

    #[test]
    fn ranges_compress_runs() {
        assert_eq!(to_ranges([0, 1, 2, 5, 7, 8]), vec![(0, 3), (5, 6), (7, 9)]);
        assert!(to_ranges([]).is_empty());
    }
}
