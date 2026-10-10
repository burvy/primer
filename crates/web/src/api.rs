//! Calls to the server, with `gloo-net` (a small wrapper over the browser's
//! `fetch`). Every function is `async`: it starts the request, pauses until
//! the reply arrives, and the page stays responsive meanwhile.
//!
//! Until sign-in exists (slice 9) there is one built-in child, `dev`.

use gloo_net::http::{Request, RequestBuilder};
use primer_api::{AnswerRequest, AnswerResponse, LessonSeenRequest, LookRequest, PageDto};
use serde::de::DeserializeOwned;

const CHILD: &str = "/api/child/dev";

/// Sends a finished request and reads the JSON reply as `T`. `Err` carries a
/// short message the page can show.
async fn read<T: DeserializeOwned>(request: Request) -> Result<T, String> {
    let response = request.send().await.map_err(|e| e.to_string())?;
    if !response.ok() {
        return Err(format!("the server said {}", response.status()));
    }
    response.json().await.map_err(|e| e.to_string())
}

/// Builds a POST with a JSON body.
fn post<B: serde::Serialize>(path: &str, body: &B) -> Result<Request, String> {
    RequestBuilder::new(&format!("{CHILD}/{path}"))
        .method(gloo_net::http::Method::POST)
        .json(body)
        .map_err(|e| e.to_string())
}

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

/// Fetches a text file such as an SVG drawing.
pub async fn text(path: &str) -> Result<String, String> {
    let response = Request::get(path).send().await.map_err(|e| e.to_string())?;
    response.text().await.map_err(|e| e.to_string())
}
