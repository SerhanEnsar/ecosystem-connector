//! Flinging small items to another device, which offers to open them.
//!
//! Files are flung with `ec-transfer` (with an "open" hint); this crate carries what is
//! not a file: web pages and text. A fling is one short bidirectional stream of
//! kind [`FLING_KIND`]: the sender writes one [`FlingItem`], the receiver validates it,
//! answers with [`FlingAck`] and hands it to the app.
//!
//! The apps show what arrives in a card and open it only when the user asks. Even so,
//! everything is validated here first: only `http` and `https` links are accepted, so a
//! peer can never get this device to launch an app through a custom URL scheme or open
//! a local file.

use std::sync::Arc;
use std::time::Duration;

use ec_core::DeviceId;
use ec_net::{PeerConnection, RecvStream, SendStream, Service, ServiceFuture, read_msg, write_msg};
use serde::{Deserialize, Serialize};
use tokio::time::timeout;

/// Stream kind of a fling.
pub const FLING_KIND: u8 = 3;
/// Longest accepted URL, in bytes.
pub const MAX_URL_LEN: usize = 4096;
/// Longest accepted title, in characters.
pub const MAX_TITLE_CHARS: usize = 200;
/// Longest accepted text, in characters.
pub const MAX_TEXT_CHARS: usize = 64 * 1024;

const STEP_TIMEOUT: Duration = Duration::from_secs(10);

/// Errors of this crate.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Network error.
    #[error(transparent)]
    Net(#[from] ec_net::Error),
    /// The item was refused.
    #[error("refused: {0}")]
    Refused(&'static str),
    /// The other device did not answer in time.
    #[error("timed out")]
    Timeout,
}

/// Result type of this crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Something flung to another device.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FlingItem {
    /// A web page to open in the browser.
    Url {
        /// The address; `http` or `https` only.
        url: String,
        /// The page title, for display.
        title: String,
    },
    /// Text, such as a message or a note, offered to copy.
    Text {
        /// The text; not empty, at most [`MAX_TEXT_CHARS`] characters.
        text: String,
    },
}

/// The receiver's answer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FlingAck {
    /// Accepted and handed to the app.
    Accepted,
    /// Refused, with the reason.
    Refused(String),
}

impl FlingItem {
    /// Checks the item and returns a cleaned copy (title trimmed and sanitized, text
    /// without control characters other than line breaks and tabs).
    pub fn validate(self) -> Result<Self> {
        match self {
            FlingItem::Url { url, title } => Ok(FlingItem::Url {
                url: validate_url(&url)?,
                title: clean_title(&title),
            }),
            FlingItem::Text { text } => Ok(FlingItem::Text {
                text: clean_text(&text)?,
            }),
        }
    }
}

/// Accepts only absolute `http`/`https` URLs with a host, no whitespace or control
/// characters, at most [`MAX_URL_LEN`] bytes. Returns the URL unchanged when valid.
pub fn validate_url(url: &str) -> Result<String> {
    if url.len() > MAX_URL_LEN {
        return Err(Error::Refused("URL too long"));
    }
    if url.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(Error::Refused("URL contains spaces or control characters"));
    }
    let lower = url.to_ascii_lowercase();
    let rest = lower
        .strip_prefix("https://")
        .or_else(|| lower.strip_prefix("http://"))
        .ok_or(Error::Refused("only http and https links are opened"))?;
    let host = rest.split(['/', '?', '#']).next().unwrap_or_default();
    // Strip credentials and port to check that a host is present.
    let host = host.rsplit('@').next().unwrap_or_default();
    let host = host.split(':').next().unwrap_or_default();
    if host.is_empty() {
        return Err(Error::Refused("URL has no host"));
    }
    Ok(url.to_owned())
}

fn clean_title(title: &str) -> String {
    let cleaned: String = title
        .chars()
        .map(|c| if c.is_whitespace() { ' ' } else { c })
        .filter(|c| !c.is_control())
        .collect();
    cleaned
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(MAX_TITLE_CHARS)
        .collect()
}

fn clean_text(text: &str) -> Result<String> {
    if text.chars().count() > MAX_TEXT_CHARS {
        return Err(Error::Refused("text too long"));
    }
    let cleaned: String = text
        .replace("\r\n", "\n")
        .chars()
        .filter(|&c| c == '\n' || c == '\t' || !c.is_control())
        .collect();
    let cleaned = cleaned.trim();
    if cleaned.is_empty() {
        return Err(Error::Refused("empty text"));
    }
    Ok(cleaned.to_owned())
}

/// Sends `item` to the device at the other end of `conn` and waits for its answer.
pub async fn fling(conn: &PeerConnection, item: FlingItem) -> Result<()> {
    let item = item.validate()?;
    let (mut send, mut recv) = conn.open_bi(FLING_KIND).await?;
    write_msg(&mut send, &item).await?;
    let ack: FlingAck = timeout(STEP_TIMEOUT, read_msg(&mut recv))
        .await
        .map_err(|_| Error::Timeout)??;
    let _ = send.finish();
    match ack {
        FlingAck::Accepted => Ok(()),
        FlingAck::Refused(_) => Err(Error::Refused("the other device refused the item")),
    }
}

/// Called with each accepted item and the device that sent it.
pub type OnFling = Arc<dyn Fn(DeviceId, FlingItem) + Send + Sync>;

/// Receives flings. Register it with an [`ec_net::Router`] for [`FLING_KIND`].
pub struct FlingService {
    on_fling: OnFling,
}

impl FlingService {
    /// Hands each accepted item to `on_fling`.
    pub fn new(on_fling: OnFling) -> Self {
        Self { on_fling }
    }

    async fn serve(
        &self,
        peer: DeviceId,
        mut send: SendStream,
        mut recv: RecvStream,
    ) -> Result<()> {
        let item: FlingItem = timeout(STEP_TIMEOUT, read_msg(&mut recv))
            .await
            .map_err(|_| Error::Timeout)??;
        match item.validate() {
            Ok(item) => {
                write_msg(&mut send, &FlingAck::Accepted).await?;
                let _ = send.finish();
                (self.on_fling)(peer, item);
                Ok(())
            }
            Err(e) => {
                write_msg(&mut send, &FlingAck::Refused(e.to_string())).await?;
                let _ = send.finish();
                Err(e)
            }
        }
    }
}

impl Service for FlingService {
    fn handle_bi(
        self: Arc<Self>,
        conn: PeerConnection,
        send: SendStream,
        recv: RecvStream,
    ) -> ServiceFuture {
        Box::pin(async move {
            if let Err(e) = self.serve(conn.peer(), send, recv).await {
                tracing::info!("fling from {} refused: {e}", conn.peer().short());
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_web_links_only() {
        for ok in [
            "https://example.com",
            "http://example.com/a?b=c#d",
            "HTTPS://Example.com:8443/path",
            "https://user@host.example/",
        ] {
            assert!(validate_url(ok).is_ok(), "{ok}");
        }
        for bad in [
            "",
            "file:///etc/passwd",
            "javascript:alert(1)",
            "intent://scan#Intent;end",
            "vnc://192.168.1.2",
            "https://",
            "https:///path",
            "https://exa mple.com",
            "https://example.com/\u{0}",
            "ftp://example.com",
        ] {
            assert!(validate_url(bad).is_err(), "accepted {bad:?}");
        }
        assert!(validate_url(&format!("https://e.com/{}", "a".repeat(MAX_URL_LEN))).is_err());
    }

    #[test]
    fn titles_are_cleaned() {
        let item = FlingItem::Url {
            url: "https://example.com".into(),
            title: "  Lecture\n\t3 \u{7}notes ".into(),
        };
        let FlingItem::Url { title, .. } = item.validate().unwrap() else {
            panic!("not a page");
        };
        assert_eq!(title, "Lecture 3 notes");
    }

    #[test]
    fn text_keeps_lines_and_drops_control_characters() {
        let item = FlingItem::Text {
            text: "  first line\r\n\tsecond \u{1b}[31mline\u{7}\n".into(),
        };
        assert_eq!(
            item.validate().unwrap(),
            FlingItem::Text {
                text: "first line\n\tsecond [31mline".into()
            }
        );
        for bad in [
            String::new(),
            " \n\t ".into(),
            "a".repeat(MAX_TEXT_CHARS + 1),
        ] {
            assert!(FlingItem::Text { text: bad }.validate().is_err());
        }
    }
}
