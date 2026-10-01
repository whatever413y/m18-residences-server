//! File storage (R2 in production, memory in tests), file-type detection, and
//! the short-lived signed links the API hands out instead of R2 presigned URLs.
use std::{
    collections::BTreeMap,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::error::ApiError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredFile {
    pub bytes: Vec<u8>,
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileInfo {
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileError(pub String);

impl From<FileError> for ApiError {
    fn from(err: FileError) -> Self {
        ApiError::Upstream(err.0)
    }
}

#[async_trait::async_trait]
pub trait FileStore: Send + Sync {
    async fn put(&self, key: &str, bytes: Vec<u8>, content_type: &str) -> Result<(), FileError>;
    /// The file, or `None` if there is none under `key`.
    async fn get(&self, key: &str) -> Result<Option<StoredFile>, FileError>;
    /// The file's metadata, or `None` if there is none under `key`.
    async fn head(&self, key: &str) -> Result<Option<FileInfo>, FileError>;
    async fn delete(&self, key: &str) -> Result<(), FileError>;
}

/// In-memory [`FileStore`] for tests; [`MemoryFileStore::set_failing`] simulates an outage.
#[derive(Default)]
pub struct MemoryFileStore {
    files: Mutex<BTreeMap<String, StoredFile>>,
    failing: AtomicBool,
}

impl MemoryFileStore {
    pub fn set_failing(&self, failing: bool) {
        self.failing.store(failing, Ordering::SeqCst);
    }

    pub fn keys(&self) -> Vec<String> {
        self.files.lock().unwrap().keys().cloned().collect()
    }

    pub fn file(&self, key: &str) -> Option<StoredFile> {
        self.files.lock().unwrap().get(key).cloned()
    }

    /// Stores a file directly (e.g. the static payment images).
    pub fn insert(&self, key: &str, bytes: &[u8], content_type: &str) {
        self.files.lock().unwrap().insert(
            key.into(),
            StoredFile {
                bytes: bytes.to_vec(),
                content_type: Some(content_type.into()),
            },
        );
    }

    fn check(&self) -> Result<(), FileError> {
        if self.failing.load(Ordering::SeqCst) {
            Err(FileError("simulated storage outage".into()))
        } else {
            Ok(())
        }
    }
}

#[async_trait::async_trait]
impl FileStore for MemoryFileStore {
    async fn put(&self, key: &str, bytes: Vec<u8>, content_type: &str) -> Result<(), FileError> {
        self.check()?;
        self.files.lock().unwrap().insert(
            key.into(),
            StoredFile {
                bytes,
                content_type: Some(content_type.into()),
            },
        );
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Option<StoredFile>, FileError> {
        self.check()?;
        Ok(self.file(key))
    }

    async fn head(&self, key: &str) -> Result<Option<FileInfo>, FileError> {
        self.check()?;
        Ok(self.file(key).map(|f| FileInfo {
            content_type: f.content_type,
        }))
    }

    async fn delete(&self, key: &str) -> Result<(), FileError> {
        self.check()?;
        self.files.lock().unwrap().remove(key);
        Ok(())
    }
}

/// The content type a file really has, judged by its first bytes, for the
/// types the API recognizes (images, HEIC and PDF). `None` for anything else.
pub fn sniff(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some("image/jpeg");
    }
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Some("image/png");
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return Some("image/gif");
    }
    if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return Some("image/webp");
    }
    if bytes.starts_with(b"%PDF-") {
        return Some("application/pdf");
    }
    iso_media_type(bytes)
}

/// AVIF or HEIC, from the brands of an ISO base media `ftyp` box.
fn iso_media_type(bytes: &[u8]) -> Option<&'static str> {
    if bytes.len() < 16 || &bytes[4..8] != b"ftyp" {
        return None;
    }
    let size = u32::from_be_bytes(bytes[..4].try_into().ok()?) as usize;
    let end = size.clamp(16, bytes.len());
    // The major brand, then the compatible brands (skipping the minor version).
    let major: &[u8; 4] = bytes[8..12].try_into().ok()?;
    let (compatible, _) = bytes[16..end].as_chunks::<4>();
    let brands: Vec<&[u8; 4]> = std::iter::once(major).chain(compatible).collect();
    if brands.iter().any(|b| matches!(*b, b"avif" | b"avis")) {
        return Some("image/avif");
    }
    if brands.iter().any(|b| {
        matches!(
            *b,
            b"heic" | b"heix" | b"hevc" | b"hevx" | b"heim" | b"heis" | b"mif1" | b"msf1"
        )
    }) {
        return Some("image/heic");
    }
    None
}

/// How long a signed file link stays valid.
pub const LINK_TTL_SECONDS: i64 = 600;

type HmacSha256 = Hmac<Sha256>;

/// Signs links to stored files (`/api/files/<key>?expires=…&signature=…`) with a
/// key derived from the JWT secret, so links need no extra secret and die with
/// a rotated JWT secret.
pub struct FileSigner {
    key: Vec<u8>,
}

impl FileSigner {
    pub fn new(jwt_secret: &str) -> Self {
        let mut mac =
            HmacSha256::new_from_slice(jwt_secret.as_bytes()).expect("HMAC accepts any key length");
        mac.update(b"m18-residences/files/v1");
        Self {
            key: mac.finalize().into_bytes().to_vec(),
        }
    }

    fn mac(&self, key: &str, expires: i64) -> HmacSha256 {
        let mut mac = HmacSha256::new_from_slice(&self.key).expect("HMAC accepts any key length");
        mac.update(key.as_bytes());
        mac.update(b"\n");
        mac.update(expires.to_string().as_bytes());
        mac
    }

    pub fn signature(&self, key: &str, expires: i64) -> String {
        URL_SAFE_NO_PAD.encode(self.mac(key, expires).finalize().into_bytes())
    }

    /// Whether `signature` is valid for `key` and `expires`, and `expires` is not before `now` (unix seconds).
    pub fn verify(&self, key: &str, expires: i64, signature: &str, now: i64) -> bool {
        if expires < now {
            return false;
        }
        let Ok(signature) = URL_SAFE_NO_PAD.decode(signature) else {
            return false;
        };
        self.mac(key, expires).verify_slice(&signature).is_ok()
    }

    /// The path (with query) of a link to `key`, valid until `expires`.
    pub fn signed_path(&self, key: &str, expires: i64) -> String {
        format!(
            "/api/files/{}?expires={expires}&signature={}",
            encode_path(key),
            self.signature(key, expires)
        )
    }
}

/// Percent-encodes each segment of a slash-separated key for use in a URL path.
pub fn encode_path(key: &str) -> String {
    let mut out = String::with_capacity(key.len());
    for byte in key.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sniffs_the_supported_types() {
        assert_eq!(sniff(&[0xFF, 0xD8, 0xFF, 0xE0, 0, 0]), Some("image/jpeg"));
        assert_eq!(sniff(b"\x89PNG\r\n\x1a\n\0\0"), Some("image/png"));
        assert_eq!(sniff(b"GIF89a.."), Some("image/gif"));
        assert_eq!(sniff(b"RIFF\0\0\0\0WEBPVP8 "), Some("image/webp"));
        assert_eq!(sniff(b"%PDF-1.7\n"), Some("application/pdf"));
        assert_eq!(
            sniff(b"\0\0\0\x1cftypavif\0\0\0\0avifmif1miaf"),
            Some("image/avif")
        );
        assert_eq!(
            sniff(b"\0\0\0\x18ftypheic\0\0\0\0mif1heic"),
            Some("image/heic")
        );
        assert_eq!(
            sniff(b"\0\0\0\x18ftypmif1\0\0\0\0mif1avif"),
            Some("image/avif"),
            "AVIF announced as a compatible brand"
        );
        assert_eq!(sniff(b"MZ\x90\0"), None, "an executable");
        assert_eq!(sniff(b""), None);
    }

    #[test]
    fn signed_links_verify_and_expire() {
        let signer = FileSigner::new("jwt-secret");
        let sig = signer.signature("receipts/ANA/1-r2", 1_000);
        assert!(signer.verify("receipts/ANA/1-r2", 1_000, &sig, 999));
        assert!(signer.verify("receipts/ANA/1-r2", 1_000, &sig, 1_000));
        assert!(
            !signer.verify("receipts/ANA/1-r2", 1_000, &sig, 1_001),
            "expired"
        );
        assert!(
            !signer.verify("receipts/BEN/1-r2", 1_000, &sig, 999),
            "another key"
        );
        assert!(
            !signer.verify("receipts/ANA/1-r2", 2_000, &sig, 999),
            "another expiry"
        );
        assert!(!signer.verify("receipts/ANA/1-r2", 1_000, "garbage!", 999));
        assert!(
            !FileSigner::new("other").verify("receipts/ANA/1-r2", 1_000, &sig, 999),
            "another secret"
        );
    }

    #[test]
    fn encodes_keys_for_paths() {
        assert_eq!(
            encode_path("receipts/Juan Dela Cruz/1-r2"),
            "receipts/Juan%20Dela%20Cruz/1-r2"
        );
        assert_eq!(encode_path("payments/gcash.png"), "payments/gcash.png");
        assert_eq!(encode_path("a/ñ?#"), "a/%C3%B1%3F%23");
        let signer = FileSigner::new("s");
        assert!(
            signer
                .signed_path("receipts/A B/x", 5)
                .starts_with("/api/files/receipts/A%20B/x?expires=5&signature=")
        );
    }
}
