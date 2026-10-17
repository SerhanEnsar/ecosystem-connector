//! Length-prefixed messages on QUIC streams.
//!
//! Each message is a big-endian `u32` length followed by a postcard-encoded body. The
//! length is checked against [`MAX_FRAME_LEN`] before any memory is allocated, so a
//! peer cannot make us allocate large buffers.

use quinn::{RecvStream, SendStream};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::{Error, Result};

/// Largest control message accepted, in bytes. File data does not use frames.
pub const MAX_FRAME_LEN: usize = 64 * 1024;

/// Encodes and writes one message.
pub(crate) async fn write_msg<T: Serialize>(send: &mut SendStream, msg: &T) -> Result<()> {
    let body = postcard::to_stdvec(msg)?;
    if body.len() > MAX_FRAME_LEN {
        return Err(Error::FrameTooLarge(body.len()));
    }
    let mut frame = Vec::with_capacity(4 + body.len());
    frame.extend_from_slice(&(body.len() as u32).to_be_bytes());
    frame.extend_from_slice(&body);
    send.write_all(&frame).await?;
    Ok(())
}

/// Reads and decodes one message.
pub(crate) async fn read_msg<T: DeserializeOwned>(recv: &mut RecvStream) -> Result<T> {
    let mut len = [0u8; 4];
    recv.read_exact(&mut len).await?;
    let len = u32::from_be_bytes(len) as usize;
    if len > MAX_FRAME_LEN {
        return Err(Error::FrameTooLarge(len));
    }
    let mut body = vec![0u8; len];
    recv.read_exact(&mut body).await?;
    decode(&body)
}

/// Decodes a message body. Trailing bytes are rejected.
pub(crate) fn decode<T: DeserializeOwned>(body: &[u8]) -> Result<T> {
    let (msg, rest) = postcard::take_from_bytes(body)?;
    if !rest.is_empty() {
        return Err(Error::Protocol("trailing bytes after message"));
    }
    Ok(msg)
}
