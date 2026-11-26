//! Wire messages and pairing tickets for PC → phone sync.
//!
//! Framing and tickets follow reader-sync (`core/src/proto.rs`), with this app's own ALPNs,
//! ticket prefix and port, so the apps can never pair with each other.

use anyhow::{bail, Context, Result};
use iroh::EndpointAddr;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub const SYNC_ALPN: &[u8] = b"kbx/sync/1";
pub const PAIR_ALPN: &[u8] = b"kbx/pair/1";
/// Bumped whenever messages change incompatibly.
pub const PROTOCOL_VERSION: u32 = 1;
const MAX_FRAME: usize = 32 * 1024 * 1024;
const TICKET_PREFIX: &str = "KBX1";

/// What the phone asks for, one request per stream.
#[derive(Debug, Serialize, Deserialize)]
pub enum Request {
    /// What the PC has now. `schema` is the newest database schema the phone can read.
    Manifest { protocol: u32, schema: i64 },
    /// File bytes from `offset` on. Answered by `Have` and the raw bytes, or `Missing`.
    File { file: FileRef, offset: u64 },
    /// The phone has applied this snapshot (lets the PC show when the phone last synced).
    Applied { hash: [u8; 32] },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FileRef {
    Snapshot { hash: [u8; 32] },
    /// A path relative to the media folder, exactly as listed in the manifest.
    Media { path: String },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Response {
    Manifest(Manifest),
    Have { size: u64 },
    Missing,
    Refused { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Manifest {
    pub schema: i64,
    /// BLAKE3 of the snapshot database file.
    pub snapshot: [u8; 32],
    pub snapshot_size: u64,
    pub media: Vec<MediaEntry>,
    pub posts: u64,
    pub built_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct MediaEntry {
    pub path: String,
    pub size: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum PairMsg {
    Request { secret: [u8; 32], name: String, platform: String },
    Accepted { name: String, platform: String },
    Rejected { reason: String },
}

pub async fn write_msg<W: AsyncWrite + Unpin, T: Serialize>(w: &mut W, msg: &T) -> Result<()> {
    let bytes = postcard::to_stdvec(msg)?;
    w.write_all(&(bytes.len() as u32).to_le_bytes()).await?;
    w.write_all(&bytes).await?;
    w.flush().await?;
    Ok(())
}

/// Returns `None` on a clean end of stream between messages.
pub async fn read_msg<R: AsyncRead + Unpin, T: DeserializeOwned>(r: &mut R) -> Result<Option<T>> {
    let mut len = [0u8; 4];
    match r.read_exact(&mut len).await {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e.into()),
    }
    let len = u32::from_le_bytes(len) as usize;
    if len > MAX_FRAME {
        bail!("message of {len} bytes is too large");
    }
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf).await?;
    Ok(Some(postcard::from_bytes(&buf).context("malformed message")?))
}

/// What the PC's QR code or pasted pairing code carries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ticket {
    pub addr: EndpointAddr,
    pub secret: [u8; 32],
}

impl Ticket {
    /// Uppercase base32 so the QR code can use its compact alphanumeric mode.
    pub fn encode(&self) -> String {
        let bytes = postcard::to_stdvec(self).expect("ticket serializes");
        format!("{TICKET_PREFIX}{}", data_encoding::BASE32_NOPAD.encode(&bytes))
    }

    pub fn decode(text: &str) -> Result<Self> {
        let cleaned = text
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
            .to_uppercase();
        let body = cleaned
            .strip_prefix(TICKET_PREFIX)
            .context("This isn't an X Knowledge Base pairing code.")?;
        let bytes = data_encoding::BASE32_NOPAD
            .decode(body.as_bytes())
            .ok()
            .context("This pairing code is incomplete or mistyped.")?;
        postcard::from_bytes(&bytes)
            .ok()
            .context("This pairing code is incomplete or mistyped.")
    }
}

pub fn qr_svg(text: &str) -> Result<String> {
    use qrcode::{render::svg, EcLevel, QrCode};
    let code = QrCode::with_error_correction_level(text.as_bytes(), EcLevel::M)?;
    Ok(code
        .render::<svg::Color>()
        .min_dimensions(240, 240)
        .dark_color(svg::Color("#000"))
        .light_color(svg::Color("#fff"))
        .build())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticket_round_trip() {
        let key = iroh::SecretKey::generate();
        let addr = EndpointAddr::new(key.public()).with_ip_addr("192.168.1.20:47130".parse().unwrap());
        let ticket = Ticket { addr, secret: [7; 32] };
        let text = ticket.encode();
        assert!(text.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()));
        let back = Ticket::decode(&format!("  {}\n", text.to_lowercase())).unwrap();
        assert_eq!(back.addr, ticket.addr);
        assert_eq!(back.secret, ticket.secret);
        assert!(Ticket::decode("READ1ABC").is_err());
        assert!(qr_svg(&text).unwrap().starts_with("<?xml"));
    }
}
