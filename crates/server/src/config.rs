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

/// How parents may sign in, worked out once at start from two settings:
/// `PRIMER_GOOGLE_CLIENT_ID` and `PRIMER_DEV_SIGNIN`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignIn {
    /// Google's sign-in, with this client id. The real deployment.
    Google(String),
    /// The local test sign-in: anyone who opens the page is let in as the
    /// same parent. Only on a laptop, and only when asked for by name.
    Dev,
    /// Neither is set up: nobody can sign in. Safe, if not useful.
    Off,
}

impl SignIn {
    /// Reads the two settings from the environment and decides with
    /// [`SignIn::decide`]. An empty variable counts as unset.
    pub fn from_env() -> Result<SignIn, String> {
        let read = |name| std::env::var(name).ok().filter(|v: &String| !v.is_empty());
        SignIn::decide(
            read("PRIMER_GOOGLE_CLIENT_ID"),
            read("PRIMER_DEV_SIGNIN").as_deref(),
        )
    }

    /// The rule, as a pure function so it can be tested:
    ///
    /// - The local sign-in is on only with `PRIMER_DEV_SIGNIN=1`. Nothing
    ///   turns it on by accident: no setting means no local sign-in.
    /// - It is refused when a Google client id is also set, because a
    ///   client id means a real deployment. The server will not start, so
    ///   the mistake is seen at once instead of leaving a way in.
    /// - Any other value of the flag is refused too, so a typo such as
    ///   `PRIMER_DEV_SIGNIN=true` is an error, not a silent "off".
    pub fn decide(
        google_client_id: Option<String>,
        dev_flag: Option<&str>,
    ) -> Result<SignIn, String> {
        match (google_client_id, dev_flag) {
            (Some(_), Some(_)) => Err(
                "PRIMER_DEV_SIGNIN is set together with PRIMER_GOOGLE_CLIENT_ID: \
                 the local sign-in is never allowed on a real deployment"
                    .into(),
            ),
            (None, Some("1")) => Ok(SignIn::Dev),
            (None, Some(other)) => Err(format!(
                "PRIMER_DEV_SIGNIN must be 1 or unset, not {other:?}"
            )),
            (Some(id), None) => Ok(SignIn::Google(id)),
            (None, None) => Ok(SignIn::Off),
        }
    }

    /// The Google client id, when Google's sign-in is on.
    pub fn google_client_id(&self) -> Option<&str> {
        match self {
            SignIn::Google(id) => Some(id),
            _ => None,
        }
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

    #[test]
    fn the_local_sign_in_is_on_only_when_asked_for_and_never_with_google() {
        let google = || Some("client-id".to_string());
        assert_eq!(SignIn::decide(None, Some("1")), Ok(SignIn::Dev));
        assert_eq!(SignIn::decide(None, None), Ok(SignIn::Off));
        assert_eq!(
            SignIn::decide(google(), None),
            Ok(SignIn::Google("client-id".into()))
        );
        // A real deployment with the flag left on refuses to start.
        assert!(SignIn::decide(google(), Some("1")).is_err());
        assert!(SignIn::decide(google(), Some("0")).is_err());
        // A typo is an error, not a quiet "off".
        assert!(SignIn::decide(None, Some("true")).is_err());
    }
}
