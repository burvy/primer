//! Calls to the server, with `gloo-net` (a small wrapper over the browser's
//! `fetch`). Every function is `async`: it starts the request, pauses until
//! the reply arrives, and the page stays responsive meanwhile.
//!
//! Two things are remembered here for every call: the parent's sign-in
//! `token` (kept in the browser's `localStorage`, so a reload stays signed
//! in) and the `child` being read to. Both are sent as the server expects:
//! the token as `Authorization: Bearer <token>`, the child in the address.

use std::cell::RefCell;

use gloo_net::http::{Request, RequestBuilder};
use primer_api::{
    AnswerRequest, AnswerResponse, AudioSettings, AuthConfig, ChildDto, GoogleSignIn, HeroRequest,
    LessonSeenRequest, LookRequest, NewChild, PageDto, SessionDto,
};
use serde::de::DeserializeOwned;

/// Where the token lives in `localStorage`.
const TOKEN_KEY: &str = "primer-token";

// `thread_local!` is a variable that belongs to the one thread the page runs
// on; `RefCell` lets it be changed through a shared reference.
thread_local! {
    static TOKEN: RefCell<Option<String>> = const { RefCell::new(None) };
    static CHILD: RefCell<String> = const { RefCell::new(String::new()) };
}

fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

/// The saved token, if the parent signed in before (and has not signed out).
pub fn load_token() -> Option<String> {
    let saved = storage()?.get_item(TOKEN_KEY).ok().flatten()?;
    TOKEN.with(|token| *token.borrow_mut() = Some(saved.clone()));
    Some(saved)
}

/// Keeps `token` for every later call, and across reloads. `None` forgets it.
pub fn set_token(token: Option<String>) {
    if let Some(store) = storage() {
        let _ = match &token {
            Some(token) => store.set_item(TOKEN_KEY, token),
            None => store.remove_item(TOKEN_KEY),
        };
    }
    TOKEN.with(|saved| *saved.borrow_mut() = token);
}

/// Which child the page is reading to.
pub fn set_child(id: &str) {
    CHILD.with(|child| *child.borrow_mut() = id.to_string());
}

fn child_path() -> String {
    CHILD.with(|child| format!("/api/child/{}", child.borrow()))
}

/// The `Authorization` header value, if signed in.
fn bearer() -> Option<String> {
    TOKEN.with(|token| token.borrow().as_ref().map(|t| format!("Bearer {t}")))
}

/// A request builder with the token attached.
fn builder(method: gloo_net::http::Method, url: &str) -> RequestBuilder {
    let request = RequestBuilder::new(url).method(method);
    match bearer() {
        Some(value) => request.header("Authorization", &value),
        None => request,
    }
}

/// What the server said when it refused: the status, and its short message.
/// A 401 also forgets the token, so the page goes back to sign-in.
async fn refusal(response: gloo_net::http::Response) -> String {
    if response.status() == 401 {
        set_token(None);
    }
    match response.text().await {
        Ok(message) if !message.is_empty() => message,
        _ => format!("the server said {}", response.status()),
    }
}

/// Sends a finished request and reads the JSON reply as `T`. `Err` carries a
/// short message the page can show.
async fn read<T: DeserializeOwned>(request: Request) -> Result<T, String> {
    let response = request.send().await.map_err(|e| e.to_string())?;
    if !response.ok() {
        return Err(refusal(response).await);
    }
    response.json().await.map_err(|e| e.to_string())
}

/// Sends a request that has no reply body.
async fn done(request: Request) -> Result<(), String> {
    let response = request.send().await.map_err(|e| e.to_string())?;
    if !response.ok() {
        return Err(refusal(response).await);
    }
    Ok(())
}

fn get(url: &str) -> Result<Request, String> {
    builder(gloo_net::http::Method::GET, url)
        .build()
        .map_err(|e| e.to_string())
}

/// A POST with a JSON body.
fn post_to<B: serde::Serialize>(url: &str, body: &B) -> Result<Request, String> {
    builder(gloo_net::http::Method::POST, url)
        .json(body)
        .map_err(|e| e.to_string())
}

/// A POST with no body.
fn post_empty(url: &str) -> Result<Request, String> {
    builder(gloo_net::http::Method::POST, url)
        .build()
        .map_err(|e| e.to_string())
}

fn delete(url: &str) -> Result<Request, String> {
    builder(gloo_net::http::Method::DELETE, url)
        .build()
        .map_err(|e| e.to_string())
}

/// A POST to the child being read to.
fn post<B: serde::Serialize>(path: &str, body: &B) -> Result<Request, String> {
    post_to(&format!("{}/{path}", child_path()), body)
}

// ---- reading to a child

pub async fn answer(body: &AnswerRequest) -> Result<AnswerResponse, String> {
    read(post("answer", body)?).await
}

pub async fn lesson_seen(skill: &str, lesson: &str) -> Result<PageDto, String> {
    let body = LessonSeenRequest {
        skill: skill.into(),
        lesson: lesson.into(),
    };
    read(post("lesson-seen", &body)?).await
}

pub async fn set_look(look: &str) -> Result<PageDto, String> {
    read(post("look", &LookRequest { look: look.into() })?).await
}

/// What the child wants to hear.
pub async fn audio() -> Result<AudioSettings, String> {
    read(get(&format!("{}/audio", child_path()))?).await
}

/// Remembers what the child wants to hear.
pub async fn set_audio(settings: &AudioSettings) -> Result<AudioSettings, String> {
    read(post("audio", settings)?).await
}

// ---- the parent

/// How the parent may sign in.
pub async fn auth_config() -> Result<AuthConfig, String> {
    read(get("/api/auth/config")?).await
}

/// Signs in, keeping the new token.
async fn keep(session: Result<SessionDto, String>) -> Result<(), String> {
    set_token(Some(session?.token));
    Ok(())
}

/// The local sign-in (only offered when the server has no Google client id).
pub async fn dev_sign_in() -> Result<(), String> {
    keep(read(post_empty("/api/auth/dev")?).await).await
}

/// Signs in with what Google's button gave the page.
pub async fn google_sign_in(credential: &str) -> Result<(), String> {
    let body = GoogleSignIn {
        credential: credential.into(),
    };
    keep(read(post_to("/api/auth/google", &body)?).await).await
}

/// Signs out: the server forgets the token, and so does the page.
pub async fn sign_out() {
    if let Ok(request) = post_empty("/api/auth/sign-out") {
        let _ = done(request).await;
    }
    set_token(None);
}

pub async fn children() -> Result<Vec<ChildDto>, String> {
    read(get("/api/children")?).await
}

pub async fn create_child(look: &str, age_band: &str) -> Result<ChildDto, String> {
    let body = NewChild {
        age_band: age_band.into(),
        look: look.into(),
    };
    read(post_to("/api/children", &body)?).await
}

pub async fn delete_child(id: &str) -> Result<(), String> {
    done(delete(&format!("/api/children/{id}"))?).await
}

/// Everything stored about the child, as JSON text.
pub async fn export_child(id: &str) -> Result<String, String> {
    let response = get(&format!("/api/children/{id}/export"))?
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !response.ok() {
        return Err(refusal(response).await);
    }
    response.text().await.map_err(|e| e.to_string())
}

/// The hero names to pick from.
pub async fn hero_names() -> Result<Vec<String>, String> {
    read(get("/api/hero-names")?).await
}

/// Sets the hero name. `Err` carries the server's reason when a typed name
/// is refused ("a hero name is one word", ...).
pub async fn set_hero(id: &str, name: &str) -> Result<ChildDto, String> {
    let body = HeroRequest { name: name.into() };
    read(post_to(&format!("/api/children/{id}/hero"), &body)?).await
}

/// Deletes the parent's account and everything under it.
pub async fn delete_account() -> Result<(), String> {
    done(delete("/api/account")?).await?;
    set_token(None);
    Ok(())
}

/// Fetches a text file such as an SVG drawing. A missing file is an `Err`.
pub async fn text(path: &str) -> Result<String, String> {
    let response = Request::get(path).send().await.map_err(|e| e.to_string())?;
    if !response.ok() {
        return Err(format!("no file {path}"));
    }
    response.text().await.map_err(|e| e.to_string())
}
