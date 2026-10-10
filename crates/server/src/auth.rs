//! Parent sign-in, and working out who sent a request.
//!
//! Children have no login. A parent signs in with Google (or, when no Google
//! client id is set, with a clearly-labelled local sign-in) and gets a random
//! session token. The browser sends it as `Authorization: Bearer <token>`.
//!
//! The [`Parent`] *extractor* is the access control: a handler that lists
//! `Parent` among its arguments is never run for a request without a valid
//! token, because axum builds the arguments first and answers 401 if
//! `Parent` cannot be built.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::Json;
use axum::extract::{FromRequestParts, State};
use axum::http::request::Parts;
use axum::http::{HeaderMap, StatusCode, header};
use primer_api::{AuthConfig, GoogleSignIn, SessionDto};
use serde::Deserialize;

use crate::db::{self, ParentId};
use crate::routes::AppState;

/// How long a sign-in lasts.
const SESSION_LENGTH: Duration = Duration::from_secs(60 * 60 * 24 * 30);

/// The `google_sub` of the local sign-in's parent.
const DEV_SUB: &str = "dev-local";

/// Seconds since 1970.
pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

/// A fresh secret: 128 random bits as hex, from the operating system's
/// random source (`uuid` v4), so it cannot be guessed.
pub fn new_token() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

/// The token in an `Authorization: Bearer <token>` header, if there is one.
pub fn bearer(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .filter(|token| !token.is_empty())
}

/// The signed-in parent. Any handler that takes a `Parent` requires a valid,
/// unexpired token.
pub struct Parent {
    pub id: ParentId,
    /// The token they came with (sign-out forgets it).
    pub token: String,
}

impl FromRequestParts<AppState> for Parent {
    /// What axum answers when a `Parent` cannot be built: 401.
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Parent, Self::Rejection> {
        const NOT_SIGNED_IN: (StatusCode, &str) = (StatusCode::UNAUTHORIZED, "not signed in");
        let token = bearer(&parts.headers).ok_or(NOT_SIGNED_IN)?;
        let found = db::parent_of_session(&state.pool, token, now())
            .await
            .map_err(failed)?;
        let id = found.ok_or(NOT_SIGNED_IN)?;
        Ok(Parent {
            id,
            token: token.to_string(),
        })
    }
}

/// What Google's `tokeninfo` endpoint says about an ID token. Google checks
/// the signature and the expiry; every field is a string.
#[derive(Debug, Deserialize)]
pub struct TokenInfo {
    /// "Audience": the client id the token was made for.
    pub aud: String,
    /// "Subject": the account's permanent id.
    pub sub: String,
    pub email_verified: String,
}

/// Is this token for us? It must name our client id and a verified email.
/// Returns the account's permanent id. A token made for another app, which
/// Google would also call valid, is refused here.
pub fn account_of(info: &TokenInfo, client_id: &str) -> Option<String> {
    (info.aud == client_id && info.email_verified == "true").then(|| info.sub.clone())
}

type Refusal = (StatusCode, &'static str);

/// A failure of ours (database, network): logged, and answered with a
/// plain 500 that tells the caller nothing.
pub fn failed(error: impl std::fmt::Display) -> Refusal {
    eprintln!("error: {error}");
    (StatusCode::INTERNAL_SERVER_ERROR, "something went wrong")
}

/// Makes the session for `google_sub` and returns its token.
async fn start_session(state: &AppState, google_sub: &str) -> Result<Json<SessionDto>, Refusal> {
    let parent = db::parent_for_sub(&state.pool, google_sub, now())
        .await
        .map_err(failed)?;
    let token = new_token();
    let expires = now() + SESSION_LENGTH.as_secs() as i64;
    db::create_session(&state.pool, &token, parent, expires)
        .await
        .map_err(failed)?;
    Ok(Json(SessionDto { token }))
}

/// `GET /api/auth/config`: which sign-in the page should offer.
pub async fn config(State(state): State<AppState>) -> Json<AuthConfig> {
    Json(AuthConfig {
        google_client_id: state.google_client_id.clone(),
        dev_sign_in: state.google_client_id.is_none(),
    })
}

/// `POST /api/auth/google`: Google's ID token in, a session out.
pub async fn google(
    State(state): State<AppState>,
    Json(body): Json<GoogleSignIn>,
) -> Result<Json<SessionDto>, Refusal> {
    let Some(client_id) = state.google_client_id.as_deref() else {
        return Err((StatusCode::NOT_FOUND, "Google sign-in is not set up"));
    };
    let refused = (StatusCode::UNAUTHORIZED, "invalid Google sign-in");
    let reply = state
        .http
        .get("https://oauth2.googleapis.com/tokeninfo")
        .query(&[("id_token", body.credential.as_str())])
        .send()
        .await
        .map_err(failed)?;
    if !reply.status().is_success() {
        return Err(refused);
    }
    let info: TokenInfo = reply.json().await.map_err(failed)?;
    let sub = account_of(&info, client_id).ok_or(refused)?;
    start_session(&state, &sub).await
}

/// `POST /api/auth/dev`: the local sign-in. It exists only while no Google
/// client id is set, so a real deployment cannot be signed into this way.
pub async fn dev(State(state): State<AppState>) -> Result<Json<SessionDto>, Refusal> {
    if state.google_client_id.is_some() {
        return Err((StatusCode::NOT_FOUND, "no local sign-in"));
    }
    start_session(&state, DEV_SUB).await
}

/// `POST /api/auth/sign-out`: forgets this token.
pub async fn sign_out(
    State(state): State<AppState>,
    parent: Parent,
) -> Result<StatusCode, Refusal> {
    db::delete_session(&state.pool, &parent.token)
        .await
        .map_err(failed)?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn headers(value: &str) -> HeaderMap {
        let mut map = HeaderMap::new();
        map.insert(header::AUTHORIZATION, HeaderValue::from_str(value).unwrap());
        map
    }

    #[test]
    fn the_token_is_read_from_a_bearer_header() {
        assert_eq!(bearer(&headers("Bearer abc123")), Some("abc123"));
        assert_eq!(bearer(&headers("Basic abc123")), None);
        assert_eq!(bearer(&headers("Bearer ")), None);
        assert_eq!(bearer(&HeaderMap::new()), None);
    }

    #[test]
    fn tokens_are_long_and_never_repeat() {
        let (a, b) = (new_token(), new_token());
        assert_eq!(a.len(), 32);
        assert_ne!(a, b);
    }

    fn info(aud: &str, verified: &str) -> TokenInfo {
        TokenInfo {
            aud: aud.into(),
            sub: "1234".into(),
            email_verified: verified.into(),
        }
    }

    #[test]
    fn only_a_token_made_for_us_with_a_verified_email_passes() {
        assert_eq!(account_of(&info("ours", "true"), "ours"), Some("1234".to_string()));
        // Valid to Google, but made for another app.
        assert_eq!(account_of(&info("theirs", "true"), "ours"), None);
        assert_eq!(account_of(&info("ours", "false"), "ours"), None);
    }
}
