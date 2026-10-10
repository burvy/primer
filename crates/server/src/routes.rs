//! The HTTP edge: the routes a child's browser calls.
//!
//! Every handler is a *thin edge* around the pure core, in four steps:
//! parse the request, run the core, store what happened, reply. The maths and
//! the teaching decisions all live in `primer-core`; nothing here decides
//! what a child should see.
//!
//! A handler is an `async fn` whose arguments are *extractors*: axum looks at
//! each argument's type and builds it from the request. `State` is the shared
//! app data, `Path` is a piece of the URL, and `Json` is the request body
//! parsed as JSON. A handler returns `Result<Json<T>, ApiError>`: `Ok` becomes
//! a 200 reply, and `Err` becomes an error reply through [`IntoResponse`].

use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use primer_api::{
    AnswerRequest, AnswerResponse, LessonSeenRequest, LookRequest, PageDto, ProblemDto,
};
use primer_core::policy::{self, Chapter};
use primer_core::{Attempt, Event, Learner, Outcome, Page, Problem, Strategy, detect};
use sqlx::SqlitePool;
use tower_http::services::ServeDir;

use crate::content;
use crate::db;
use crate::judge::SharedJudge;
use crate::names::{BLANKS, LOOKS, MISTAKES, OPS, STAGES, name, parse};

/// Everything a handler may need. It is cloned for each request, so every
/// field is cheap to clone: a pool handle, or an `Arc` (a shared pointer).
#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub judge: SharedJudge,
    pub chapters: Arc<Vec<Chapter>>,
}

/// The whole app: the API under `/api`, and the built web app (`dist`)
/// for every other address.
pub fn router(state: AppState, dist: PathBuf) -> Router {
    Router::new()
        .route("/api/child/{id}/next", get(next_page))
        .route("/api/child/{id}/answer", post(answer))
        .route("/api/child/{id}/lesson-seen", post(lesson_seen))
        .route("/api/child/{id}/look", post(set_look))
        .fallback_service(ServeDir::new(dist))
        .with_state(state)
}

/// Everything that can go wrong in a handler. Each case knows its status code.
#[derive(Debug)]
enum ApiError {
    /// No such child: 404.
    NotFound,
    /// The request does not fit the child's current page: 409.
    Conflict(&'static str),
    /// The database failed: 500. The details are logged, not sent to the child.
    Database(sqlx::Error),
}

/// `?` on a `Result<_, sqlx::Error>` inside a handler turns the error into an
/// `ApiError` through this `From` conversion.
impl From<sqlx::Error> for ApiError {
    fn from(error: sqlx::Error) -> Self {
        ApiError::Database(error)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            ApiError::NotFound => (StatusCode::NOT_FOUND, "no such child").into_response(),
            ApiError::Conflict(why) => (StatusCode::CONFLICT, why).into_response(),
            ApiError::Database(error) => {
                eprintln!("database error: {error}");
                (StatusCode::INTERNAL_SERVER_ERROR, "something went wrong").into_response()
            }
        }
    }
}

/// What the core says about a child right now.
struct Current {
    look: primer_core::Look,
    seed: u64,
    learner: Learner,
    page: Page,
}

/// Load the child, fold their events into a `Learner`, ask the policy.
async fn current(state: &AppState, id: &str) -> Result<Current, ApiError> {
    let child = db::child(&state.pool, id)
        .await?
        .ok_or(ApiError::NotFound)?;
    let events = db::events(&state.pool, id).await?;
    let learner = Learner::from_events(&events);
    let page = policy::next(&learner, &state.chapters, child.look, child.seed);
    Ok(Current {
        look: child.look,
        seed: child.seed,
        learner,
        page,
    })
}

/// `GET /api/child/{id}/next`: what should the child see now?
async fn next_page(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<PageDto>, ApiError> {
    let now = current(&state, &id).await?;
    Ok(Json(page_dto(&now.page, &now.learner)))
}

/// `POST /api/child/{id}/answer`: check an answer, remember it, reply with
/// the verdict and the next page.
async fn answer(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<AnswerRequest>,
) -> Result<Json<AnswerResponse>, ApiError> {
    let now = current(&state, &id).await?;
    let Page::Practice {
        skill,
        set_id,
        problem,
        stage,
        ..
    } = &now.page
    else {
        return Err(ApiError::Conflict("there is no problem to answer"));
    };
    if body.page_id != page_id(&now.learner, set_id) {
        return Err(ApiError::Conflict("that page is out of date"));
    }

    // Pure check first. Only a wrong answer with no recognisable signature
    // is handed to the judge.
    let outcome = match body.answer {
        None => Outcome::DontKnow,
        Some(given) if given == problem.answer() => Outcome::Correct,
        Some(given) => Outcome::Wrong(match detect(problem, given) {
            Some(kind) => kind,
            None => state.judge.mistake(problem, given).await,
        }),
    };
    let event = Event::Attempt(Attempt {
        skill,
        set_id: set_id.clone(),
        problem: *problem,
        stage: *stage,
        look: now.look,
        answer: body.answer,
        outcome,
        ms_taken: body.ms_taken,
        strategy: Strategy::Unknown,
        frustrated: false,
    });
    db::append(&state.pool, &id, &event).await?;

    // Fold the new event into the learner we already have; no second query.
    let learner = now.learner.apply(&event);
    let next = policy::next(&learner, &state.chapters, now.look, now.seed);
    Ok(Json(AnswerResponse {
        correct: outcome == Outcome::Correct,
        mistake: match outcome {
            Outcome::Wrong(kind) => Some(name(MISTAKES, kind).to_string()),
            _ => None,
        },
        next: page_dto(&next, &learner),
    }))
}

/// `POST /api/child/{id}/lesson-seen`: the child read the lesson; reply with
/// the next page.
async fn lesson_seen(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<LessonSeenRequest>,
) -> Result<Json<PageDto>, ApiError> {
    let now = current(&state, &id).await?;
    let Page::Lesson { skill, lesson, .. } = &now.page else {
        return Err(ApiError::Conflict("there is no lesson to finish"));
    };
    if body.skill != *skill || body.lesson != *lesson {
        return Err(ApiError::Conflict("that lesson is out of date"));
    }
    let event = Event::LessonSeen {
        skill,
        lesson: lesson.clone(),
    };
    db::append(&state.pool, &id, &event).await?;
    let learner = now.learner.apply(&event);
    let next = policy::next(&learner, &state.chapters, now.look, now.seed);
    Ok(Json(page_dto(&next, &learner)))
}

/// `POST /api/child/{id}/look`: switch looks, then reply with the next page
/// (the policy may choose differently for the other look).
async fn set_look(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<LookRequest>,
) -> Result<Json<PageDto>, ApiError> {
    let look = parse(LOOKS, &body.look).ok_or(ApiError::Conflict("unknown look"))?;
    db::child(&state.pool, &id)
        .await?
        .ok_or(ApiError::NotFound)?;
    db::set_look(&state.pool, &id, look).await?;
    let now = current(&state, &id).await?;
    Ok(Json(page_dto(&now.page, &now.learner)))
}

/// Names one problem of one set: the set id plus how many are already
/// answered. It changes after every answer, so an old page cannot be
/// answered twice.
fn page_id(learner: &Learner, set_id: &str) -> String {
    let done = learner.answered.get(set_id).copied().unwrap_or(0);
    format!("{set_id}/{done}")
}

fn problem_dto(problem: &Problem) -> ProblemDto {
    ProblemDto {
        a: problem.a(),
        op: name(OPS, problem.op()).to_string(),
        b: problem.b(),
        blank: name(BLANKS, problem.blank()).to_string(),
        text: problem.to_string(),
    }
}

/// Core page to JSON page.
fn page_dto(page: &Page, learner: &Learner) -> PageDto {
    match page {
        Page::Lesson {
            skill,
            lesson,
            again,
        } => PageDto::Lesson {
            skill: skill.to_string(),
            lesson: lesson.clone(),
            again: *again,
            beats: content::beats(skill, lesson),
        },
        Page::Practice {
            skill,
            set_id,
            problem,
            stage,
            hint,
            easier,
            ..
        } => PageDto::Practice {
            page_id: page_id(learner, set_id),
            skill: skill.to_string(),
            set_id: set_id.clone(),
            problem: problem_dto(problem),
            stage: name(STAGES, *stage).to_string(),
            hint: hint.map(|kind| name(MISTAKES, kind).to_string()),
            easier: *easier,
        },
        Page::Story => PageDto::Story,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content;
    use crate::judge::FakeJudge;
    use axum::body::Body;
    use axum::http::{Method, Request};
    use http_body_util::BodyExt;
    use primer_core::Look;
    use tower::ServiceExt;

    /// A fresh app on a database file that tests can reopen.
    async fn app(db_file: &std::path::Path) -> Router {
        let pool = db::connect(&format!("sqlite://{}", db_file.display()))
            .await
            .unwrap();
        db::ensure_child(&pool, "kid", Look::Gentle, 7)
            .await
            .unwrap();
        let state = AppState {
            pool,
            judge: Arc::new(FakeJudge),
            chapters: Arc::new(content::chapters()),
        };
        router(state, PathBuf::from("no-such-dist"))
    }

    /// Sends one request straight into the router (no network), the way
    /// `oneshot` allows, and reads the JSON reply.
    async fn call<T: serde::de::DeserializeOwned>(
        app: &Router,
        method: Method,
        uri: &str,
        body: Option<serde_json::Value>,
    ) -> (StatusCode, Option<T>) {
        let request = Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(body.map(|b| b.to_string()).unwrap_or_default()))
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (status, serde_json::from_slice(&bytes).ok())
    }

    async fn next(app: &Router) -> PageDto {
        let (status, page) = call(app, Method::GET, "/api/child/kid/next", None).await;
        assert_eq!(status, StatusCode::OK);
        page.unwrap()
    }

    /// Reads the lesson (if that is what is showing), then answers `n`
    /// problems correctly. Returns the last reply.
    async fn play_right(app: &Router, n: usize) -> PageDto {
        let mut page = next(app).await;
        for _ in 0..n {
            if let PageDto::Lesson { skill, lesson, .. } = &page {
                let body = serde_json::json!({ "skill": skill, "lesson": lesson });
                page = call(app, Method::POST, "/api/child/kid/lesson-seen", Some(body))
                    .await
                    .1
                    .unwrap();
            }
            let PageDto::Practice {
                page_id, problem, ..
            } = &page
            else {
                panic!("expected a problem, got {page:?}");
            };
            let answer = match problem.blank.as_str() {
                "answer" if problem.op == "add" => problem.a + problem.b,
                "answer" => problem.a - problem.b,
                "second" => problem.b,
                _ => problem.a,
            };
            let body =
                serde_json::json!({ "page_id": page_id, "answer": answer, "ms_taken": 2000 });
            let (status, reply) =
                call::<AnswerResponse>(app, Method::POST, "/api/child/kid/answer", Some(body))
                    .await;
            assert_eq!(status, StatusCode::OK);
            let reply = reply.unwrap();
            assert!(reply.correct);
            page = reply.next;
        }
        page
    }

    fn temp_db(name: &str) -> PathBuf {
        let file =
            std::env::temp_dir().join(format!("primer-test-{name}-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&file);
        file
    }

    #[tokio::test]
    async fn three_answers_advance_the_learner_and_a_restart_keeps_them() {
        let file = temp_db("restart");
        let app = app(&file).await;

        // A new child starts with a lesson, then the first problem.
        assert!(matches!(next(&app).await, PageDto::Lesson { .. }));
        let after_three = play_right(&app, 3).await;
        // Three right on pictures moves the child up a stage.
        let PageDto::Practice { stage, .. } = &after_three else {
            panic!("expected a problem");
        };
        assert_eq!(stage, "number_line");

        // "Restart": drop the app, open the same file again.
        drop(app);
        let app = self::app(&file).await;
        assert_eq!(next(&app).await, after_three);
        let _ = std::fs::remove_file(&file);
    }

    #[tokio::test]
    async fn a_wrong_answer_is_named_and_a_stale_page_is_refused() {
        let file = temp_db("wrong");
        let app = app(&file).await;
        let lesson = next(&app).await;
        let PageDto::Lesson { skill, lesson, .. } = lesson else {
            panic!("expected the lesson first");
        };
        let body = serde_json::json!({ "skill": skill, "lesson": lesson });
        let page = call::<PageDto>(&app, Method::POST, "/api/child/kid/lesson-seen", Some(body))
            .await
            .1
            .unwrap();
        let PageDto::Practice {
            page_id, problem, ..
        } = page
        else {
            panic!("expected a problem");
        };

        let wrong = problem.a + problem.b + 1;
        let body = serde_json::json!({ "page_id": page_id, "answer": wrong, "ms_taken": 900 });
        let (status, reply) = call::<AnswerResponse>(
            &app,
            Method::POST,
            "/api/child/kid/answer",
            Some(body.clone()),
        )
        .await;
        let reply = reply.unwrap();
        assert_eq!(status, StatusCode::OK);
        assert!(!reply.correct);
        assert_eq!(reply.mistake.as_deref(), Some("off_by_one"));

        // The same page id again is now out of date.
        let (status, _) =
            call::<AnswerResponse>(&app, Method::POST, "/api/child/kid/answer", Some(body)).await;
        assert_eq!(status, StatusCode::CONFLICT);
        let _ = std::fs::remove_file(&file);
    }

    #[tokio::test]
    async fn an_unknown_child_is_not_found() {
        let file = temp_db("missing");
        let app = app(&file).await;
        let (status, _) = call::<PageDto>(&app, Method::GET, "/api/child/nobody/next", None).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        let _ = std::fs::remove_file(&file);
    }

    #[tokio::test]
    async fn switching_look_keeps_progress_and_rejects_unknown_looks() {
        let file = temp_db("look");
        let app = app(&file).await;
        let before = play_right(&app, 2).await;
        let (status, _) = call::<PageDto>(
            &app,
            Method::POST,
            "/api/child/kid/look",
            Some(serde_json::json!({ "look": "exceptional" })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        // Same child, same log: still on the same set.
        let (PageDto::Practice { set_id: a, .. }, PageDto::Practice { set_id: b, .. }) =
            (before, next(&app).await)
        else {
            panic!("expected problems");
        };
        assert_eq!(a, b);
        let (status, _) = call::<PageDto>(
            &app,
            Method::POST,
            "/api/child/kid/look",
            Some(serde_json::json!({ "look": "purple" })),
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        let _ = std::fs::remove_file(&file);
    }
}
