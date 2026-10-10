//! Settings for talking to Jev, and the one place the API key is loaded.
//!
//! The key lives in a file (`jev-key.env`, gitignored) that holds only the raw
//! key. It is read once, trimmed, and wrapped in [`SecretString`] straight
//! away. `SecretString` prints as `[REDACTED]` in `Debug`, so the key cannot
//! leak through `{config:?}` or a log line, and it is wiped from memory when
//! dropped. Only [`expose_secret`](ExposeSecret::expose_secret) reads it, and
//! only `jev.rs` calls that, when it builds the `Authorization` header.

use std::path::{Path, PathBuf};
use std::time::Duration;

use secrecy::SecretString;

/// Where Jev lives. Overridable so tests never reach the real service.
pub const JEV_URL: &str = "https://api.typesafe.ai/v1/systemone";
/// A child is waiting: past this, give up and answer `Unknown`.
pub const JEV_TIMEOUT: Duration = Duration::from_secs(2);

/// Everything the Jev client needs.
///
/// Deriving `Debug` is safe here *because* `key` is a `SecretString`: its
/// own `Debug` hides the value. A plain `String` would print the key.
#[derive(Debug, Clone)]
pub struct JevConfig {
    pub key: SecretString,
    pub url: String,
    pub timeout: Duration,
}

impl JevConfig {
    /// Reads the key file named by `PRIMER_JEV_KEY_FILE` (default
    /// `jev-key.env`, relative to where the server starts).
    ///
    /// `None` means "no key file": the server then uses the fake judge, so
    /// local runs and tests work with no key at all.
    pub fn from_env() -> Option<JevConfig> {
        let path = std::env::var("PRIMER_JEV_KEY_FILE").unwrap_or_else(|_| "jev-key.env".into());
        JevConfig::from_key_file(&PathBuf::from(path))
    }

    /// Reads the raw key from `path`. An empty or unreadable file is treated
    /// like a missing one. The read error is dropped on purpose: it could
    /// only say "file not found", and the caller already knows what to do.
    pub fn from_key_file(path: &Path) -> Option<JevConfig> {
        let raw = std::fs::read_to_string(path).ok()?;
        let key = raw.trim();
        if key.is_empty() {
            return None;
        }
        Some(JevConfig {
            key: SecretString::from(key),
            url: JEV_URL.to_string(),
            timeout: JEV_TIMEOUT,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::ExposeSecret;

    /// A made-up key in a temporary file; the real key file is never opened.
    #[test]
    fn debug_never_shows_the_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("key.env");
        std::fs::write(&path, "  not-a-real-key-123\r\n").unwrap();
        let config = JevConfig::from_key_file(&path).expect("a key");
        assert_eq!(config.key.expose_secret(), "not-a-real-key-123", "trimmed");
        let shown = format!("{config:?}");
        assert!(!shown.contains("not-a-real-key"), "{shown}");
        assert!(shown.contains("REDACTED"));
    }

    #[test]
    fn a_missing_or_empty_file_means_no_jev() {
        let dir = tempfile::tempdir().unwrap();
        assert!(JevConfig::from_key_file(&dir.path().join("absent.env")).is_none());
        let empty = dir.path().join("empty.env");
        std::fs::write(&empty, " \n").unwrap();
        assert!(JevConfig::from_key_file(&empty).is_none());
    }
}
