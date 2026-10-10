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
    AnswerRequest, AnswerResponse, AudioSettings, ContentsDto, LessonSeenRequest, LookRequest,
    OpenerDto, PageDto, PracticeRequest, ProblemDto,
};
use primer_core::policy;
use primer_core::{
    Attempt, Event, Learner, MistakeKind, Op, Outcome, Page, Problem, Strategy, detect,
};
use rand::SeedableRng;
use rand::rngs::StdRng;
use sqlx::SqlitePool;
use tower_http::services::ServeDir;

use crate::auth::{self, Parent};
use crate::children;
use crate::content::Content;
use crate::content::feedback::Facts;
use crate::db::{self, ParentId};
use crate::judge::SharedJudge;
use crate::names::{BLANKS, LOOKS, MISTAKES, OPS, STAGES, name, parse};

/// Everything a handler may need. It is cloned for each request, so every
/// field is cheap to clone: a pool handle, or an `Arc` (a shared pointer).
#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub judge: SharedJudge,
    /// The checked book, loaded once at start.
    pub content: Arc<Content>,
    /// How parents sign in: Google, the local test sign-in, or neither
    /// (see [`SignIn::decide`](crate::config::SignIn::decide)).
    pub sign_in: crate::config::SignIn,
    /// For the call to Google that checks a sign-in.
    pub http: reqwest::Client,
}

/// The whole app: the API under `/api`, the `assets` folder under `/assets`,
/// and the built web app (`dist`) for every other address.
pub fn router(state: AppState, dist: PathBuf, assets: PathBuf) -> Router {
    Router::new()
        .route("/api/auth/config", get(auth::config))
        .route("/api/auth/google", post(auth::google))
        .route("/api/auth/dev", post(auth::dev))
        .route("/api/auth/sign-out", post(auth::sign_out))
        .route("/api/account", axum::routing::delete(children::delete_account))
        .route("/api/hero-names", get(children::hero_names))
        .route("/api/children", get(children::list).post(children::create))
        .route("/api/children/{id}", axum::routing::delete(children::delete))
        .route("/api/children/{id}/export", get(children::export))
        .route("/api/children/{id}/hero", post(children::set_hero))
        .route("/api/child/{id}/next", get(next_page))
        .route("/api/child/{id}/answer", post(answer))
        .route("/api/child/{id}/lesson-seen", post(lesson_seen))
        .route("/api/child/{id}/look", post(set_look))
        .route("/api/child/{id}/contents", get(contents))
        .route("/api/child/{id}/lesson/{skill}/{lesson}", get(lesson_again))
        .route("/api/child/{id}/practice", post(practice))
        .route("/api/child/{id}/audio", get(get_audio).post(set_audio))
        // Recordings and sounds are read from the real `assets/` folder, so a
        // file dropped in is heard at once, with no rebuild of the web app.
        .nest_service("/assets", ServeDir::new(assets))
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
    /// The skill the child asked more practice on, if they did.
    focus: Option<String>,
    /// The latest answer was not right (so a right one now is a recovery).
    previous_wrong: bool,
}

/// The child's id, if `parent` owns that child; otherwise `NotFound`, the same
/// answer as for a child who is not there.
async fn own(state: &AppState, parent: ParentId, id: &str) -> Result<(), ApiError> {
    db::owned_child(&state.pool, parent, id)
        .await?
        .map(|_| ())
        .ok_or(ApiError::NotFound)
}

/// Load the child, fold their events into a `Learner`, ask the policy.
async fn current(state: &AppState, parent: ParentId, id: &str) -> Result<Current, ApiError> {
    own(state, parent, id).await?;
    let child = db::child(&state.pool, id)
        .await?
        .ok_or(ApiError::NotFound)?;
    let events = db::events(&state.pool, id).await?;
    let learner = Learner::from_events(&events);
    let page = upcoming(&state.content, &learner, child.look, child.seed, child.practice_skill.as_deref());
    let previous_wrong = events
        .iter()
        .rev()
        .find_map(|event| match event {
            Event::Attempt(attempt) => Some(attempt.outcome != Outcome::Correct),
            Event::LessonSeen { .. } => None,
        })
        .unwrap_or(false);
    Ok(Current {
        look: child.look,
        seed: child.seed,
        learner,
        page,
        focus: child.practice_skill,
        previous_wrong,
    })
}

/// The page the child is to see next: more practice on the skill they asked
/// for, if they asked; otherwise whatever the policy says comes next.
fn upcoming(
    content: &Content,
    learner: &Learner,
    look: primer_core::Look,
    seed: u64,
    focus: Option<&str>,
) -> Page {
    let chapters = content.chapters(look);
    focus
        .and_then(|skill| policy::more_practice(learner, chapters, skill, look, seed))
        .unwrap_or_else(|| policy::next(learner, chapters, look, seed))
}

/// `GET /api/child/{id}/next`: what should the child see now?
async fn next_page(
    State(state): State<AppState>,
    parent: Parent,
    Path(id): Path<String>,
) -> Result<Json<PageDto>, ApiError> {
    let now = current(&state, parent.id, &id).await?;
    Ok(Json(page_dto(&now.page, &now.learner, &state.content, now.look, now.focus.is_some())))
}

/// `POST /api/child/{id}/answer`: check an answer, remember it, reply with
/// the verdict and the next page.
async fn answer(
    State(state): State<AppState>,
    parent: Parent,
    Path(id): Path<String>,
    Json(body): Json<AnswerRequest>,
) -> Result<Json<AnswerResponse>, ApiError> {
    let now = current(&state, parent.id, &id).await?;
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

    // What the praise needs from the learner as they were before this answer
    // (the next line consumes `now.learner`).
    let streak_before = now.learner.progress(skill).streak;
    let answers_before: usize = now.learner.answered.values().sum();

    // Fold the new event into the learner we already have; no second query.
    let learner = now.learner.apply(&event);
    let next = upcoming(&state.content, &learner, now.look, now.seed, now.focus.as_deref());

    // What Ada says about the answer. A hint after a wrong one ("I don't
    // know" counts as an unknown slip), praise after a right one, and the
    // set's question and notice when this answer was the last of its set
    // (the next page is then not another problem of the same set).
    let content = &state.content;
    let correct = outcome == Outcome::Correct;
    let set_ended = !matches!(&next, Page::Practice { set_id: next_set, .. } if next_set == set_id);
    let hint = match outcome {
        Outcome::Correct => None,
        Outcome::Wrong(kind) => content.hint(skill, now.look, *stage, kind),
        Outcome::DontKnow => content.hint(skill, now.look, *stage, MistakeKind::Unknown),
    };
    let praise = if correct {
        let facts = Facts {
            ms_taken: body.ms_taken,
            adds: (problem.op() == Op::Add).then(|| problem.b()),
            set_end: set_ended,
            streak: streak_before + 1,
            recovered: now.previous_wrong,
        };
        // One generator per answer, seeded from the child and how many
        // answers they have given, so the same answer gets the same praise.
        let mut rng = StdRng::seed_from_u64(now.seed ^ answers_before as u64);
        content.praise(skill, now.look, facts, &mut rng)
    } else {
        None
    };
    Ok(Json(AnswerResponse {
        correct,
        mistake: match outcome {
            Outcome::Wrong(kind) => Some(name(MISTAKES, kind).to_string()),
            _ => None,
        },
        hint,
        praise,
        set_end: set_ended.then(|| content.set_end(set_id)).flatten(),
        next: page_dto(&next, &learner, content, now.look, now.focus.is_some()),
    }))
}

/// `POST /api/child/{id}/lesson-seen`: the child read the lesson; reply with
/// the next page.
async fn lesson_seen(
    State(state): State<AppState>,
    parent: Parent,
    Path(id): Path<String>,
    Json(body): Json<LessonSeenRequest>,
) -> Result<Json<PageDto>, ApiError> {
    let now = current(&state, parent.id, &id).await?;
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
    let next = upcoming(&state.content, &learner, now.look, now.seed, now.focus.as_deref());
    Ok(Json(page_dto(&next, &learner, &state.content, now.look, now.focus.is_some())))
}

/// `POST /api/child/{id}/look`: switch looks, then reply with the next page
/// (the policy may choose differently for the other look).
async fn set_look(
    State(state): State<AppState>,
    parent: Parent,
    Path(id): Path<String>,
    Json(body): Json<LookRequest>,
) -> Result<Json<PageDto>, ApiError> {
    let look = parse(LOOKS, &body.look).ok_or(ApiError::Conflict("unknown look"))?;
    own(&state, parent.id, &id).await?;
    db::set_look(&state.pool, &id, look).await?;
    let now = current(&state, parent.id, &id).await?;
    Ok(Json(page_dto(&now.page, &now.learner, &state.content, now.look, now.focus.is_some())))
}

/// `GET /api/child/{id}/contents`: the table of contents with the child's
/// progress on every lesson, set and story.
async fn contents(
    State(state): State<AppState>,
    parent: Parent,
    Path(id): Path<String>,
) -> Result<Json<ContentsDto>, ApiError> {
    let now = current(&state, parent.id, &id).await?;
    // "Where the child is now" is the main path, not a side trip of more practice.
    let chapters = state.content.chapters(now.look);
    let on = match policy::next(&now.learner, chapters, now.look, now.seed) {
        Page::Lesson { skill, .. } | Page::Practice { skill, .. } => Some(skill),
        Page::Story => None,
    };
    Ok(Json(state.content.contents(now.look, &now.learner, on)))
}

/// `GET /api/child/{id}/lesson/{skill}/{lesson}`: a lesson to read again.
/// Nothing is recorded: reading it again changes no progress.
async fn lesson_again(
    State(state): State<AppState>,
    parent: Parent,
    Path((id, skill, lesson)): Path<(String, String, String)>,
) -> Result<Json<PageDto>, ApiError> {
    own(&state, parent.id, &id).await?;
    let beats = state.content.beats(&skill, &lesson);
    if beats.is_empty() {
        return Err(ApiError::Conflict("there is no such lesson"));
    }
    Ok(Json(PageDto::Lesson {
        skill,
        lesson,
        again: false,
        beats: beats.to_vec(),
        opener: None,
    }))
}

/// `POST /api/child/{id}/practice`: ask for more practice on a skill, or
/// (with `null`) go back to the main path. Replies with the page to show.
async fn practice(
    State(state): State<AppState>,
    parent: Parent,
    Path(id): Path<String>,
    Json(body): Json<PracticeRequest>,
) -> Result<Json<PageDto>, ApiError> {
    let now = current(&state, parent.id, &id).await?;
    if let Some(skill) = &body.skill
        && !state.content.is_written(skill, now.look)
    {
        return Err(ApiError::Conflict("there is no practice for that chapter"));
    }
    db::set_practice_skill(&state.pool, &id, body.skill.as_deref()).await?;
    let now = current(&state, parent.id, &id).await?;
    Ok(Json(page_dto(&now.page, &now.learner, &state.content, now.look, now.focus.is_some())))
}

/// `GET /api/child/{id}/audio`: what the child wants to hear.
async fn get_audio(
    State(state): State<AppState>,
    parent: Parent,
    Path(id): Path<String>,
) -> Result<Json<AudioSettings>, ApiError> {
    own(&state, parent.id, &id).await?;
    Ok(Json(db::audio(&state.pool, &id).await?))
}

/// `POST /api/child/{id}/audio`: remember the three switches, reply with them.
async fn set_audio(
    State(state): State<AppState>,
    parent: Parent,
    Path(id): Path<String>,
    Json(body): Json<AudioSettings>,
) -> Result<Json<AudioSettings>, ApiError> {
    own(&state, parent.id, &id).await?;
    db::set_audio(&state.pool, &id, body).await?;
    Ok(Json(body))
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

/// Core page to JSON page. A story page's words are found by its set id.
///
/// `more` is true when the child asked for more practice. Otherwise a
/// problem for a chapter with nothing written is not shown (the policy would
/// generate problems there for ever): the page is the end of the book so far.
fn page_dto(
    page: &Page,
    learner: &Learner,
    content: &Content,
    look: primer_core::Look,
    more: bool,
) -> PageDto {
    if let Page::Practice { skill, .. } = page
        && !more
        && !content.is_written(skill, look)
    {
        return PageDto::EndOfBook;
    }
    match page {
        Page::Lesson {
            skill,
            lesson,
            again,
        } => PageDto::Lesson {
            skill: skill.to_string(),
            lesson: lesson.clone(),
            again: *again,
            beats: content.beats(skill, lesson).to_vec(),
            // A lesson offered again after a struggle needs no new chapter page.
            opener: content
                .opener(skill, lesson)
                .filter(|_| !again)
                .map(|(number, title)| OpenerDto {
                    number,
                    title: title.to_string(),
                }),
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
            story: content.story(skill, set_id).map(Box::new),
        },
        Page::Story => PageDto::Story,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::judge::FakeJudge;
    use axum::body::Body;
    use axum::http::{Method, Request};
    use http_body_util::BodyExt;
    use primer_core::Look;
    use tower::ServiceExt;

    /// A tiny book for the tests: chapter 1 with one lesson and one set.
    fn book() -> Vec<(String, String)> {
        let chapter = r#"
skill = "count-on-within-5"
title = "The Next Number"

[[lesson]]
id = "intro"
[[lesson.step]]
stage = "pictorial"
example = "3 + 1"
[[lesson.step.beat]]
text = "Three blocks make a tower."
show = "tower"
count = 3

[[problem_set]]
id = "walk"
stage = "pictorial"
pattern = "successor"
problems = ["0 + 1", "1 + 1", "2 + 1", "3 + 1"]
question = "What do you notice?"
notice = "Did you see? Each answer is one more."
formal_notice = "Each answer is the successor."

[[word_problem]]
id = "stones"
stage = "pictorial"
text = "The builder has {a} stones and {b} more."
a = 0
b = 1

[[hint]]
id = "hint-off"
stage = "pictorial"
mistake = "off-by-one"
text = "Count the new step."

[[praise]]
id = "praise-any"
text = "Correct!"
"#;
        vec![
            ("01-count-on.toml".to_string(), chapter.to_string()),
            (
                "hero_names.toml".to_string(),
                "names = [\"Liu Hui\", \"Noether\"]".to_string(),
            ),
        ]
    }

    /// A fresh app on a database file that tests can reopen.
    async fn app(db_file: &std::path::Path) -> Router {
        app_with(db_file, Arc::new(FakeJudge)).await
    }

    /// The same, with the judge of your choice.
    async fn app_with(db_file: &std::path::Path, judge: SharedJudge) -> Router {
        app_signing_in(db_file, judge, crate::config::SignIn::Dev).await
    }

    /// The same, with the judge and sign-in of your choice.
    async fn app_signing_in(
        db_file: &std::path::Path,
        judge: SharedJudge,
        sign_in: crate::config::SignIn,
    ) -> Router {
        let pool = db::connect(&format!("sqlite://{}", db_file.display()))
            .await
            .unwrap();
        // A signed-in parent (their token is `TOKEN`) with one child, `kid`.
        let parent = db::parent_for_sub(&pool, "parent-one", 0).await.unwrap();
        // (A test may open the same file again after a "restart".)
        if db::parent_of_session(&pool, TOKEN, 0).await.unwrap().is_none() {
            db::create_session(&pool, TOKEN, parent, i64::MAX).await.unwrap();
            db::create_child(&pool, parent, "kid", Look::Gentle, "5-6", 7)
                .await
                .unwrap();
        }
        let state = AppState {
            pool,
            judge,
            content: Arc::new(Content::from_sources(&book()).unwrap()),
            sign_in,
            http: reqwest::Client::new(),
        };
        router(
            state,
            PathBuf::from("no-such-dist"),
            PathBuf::from("no-such-assets"),
        )
    }

    /// The signed-in test parent's token.
    const TOKEN: &str = "test-token";

    /// Sends one request as the test parent.
    async fn call<T: serde::de::DeserializeOwned>(
        app: &Router,
        method: Method,
        uri: &str,
        body: Option<serde_json::Value>,
    ) -> (StatusCode, Option<T>) {
        call_as(app, Some(TOKEN), method, uri, body).await
    }

    /// Sends one request straight into the router (no network), the way
    /// `oneshot` allows, with this token (or none), and reads the JSON reply.
    async fn call_as<T: serde::de::DeserializeOwned>(
        app: &Router,
        token: Option<&str>,
        method: Method,
        uri: &str,
        body: Option<serde_json::Value>,
    ) -> (StatusCode, Option<T>) {
        let mut request = Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json");
        if let Some(token) = token {
            request = request.header("authorization", format!("Bearer {token}"));
        }
        let request = request
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
    async fn audio_settings_start_at_the_defaults_and_are_remembered() {
        let file = temp_db("audio");
        let app = app(&file).await;
        let (_, first) =
            call::<AudioSettings>(&app, Method::GET, "/api/child/kid/audio", None).await;
        assert_eq!(first, Some(AudioSettings::default()));
        let wanted = AudioSettings {
            narration: false,
            sounds: true,
            music: true,
        };
        let body = serde_json::to_value(wanted).unwrap();
        call::<AudioSettings>(&app, Method::POST, "/api/child/kid/audio", Some(body)).await;

        // "Restart": the same file opened again still has the choice.
        drop(app);
        let app = self::app(&file).await;
        let (_, again) =
            call::<AudioSettings>(&app, Method::GET, "/api/child/kid/audio", None).await;
        assert_eq!(again, Some(wanted));
        let (status, _) =
            call::<AudioSettings>(&app, Method::GET, "/api/child/nobody/audio", None).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
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

    /// Reads the first lesson, then returns the first problem's page.
    async fn first_problem(app: &Router) -> PageDto {
        let PageDto::Lesson { skill, lesson, .. } = next(app).await else {
            panic!("expected the lesson first");
        };
        let body = serde_json::json!({ "skill": skill, "lesson": lesson });
        call(app, Method::POST, "/api/child/kid/lesson-seen", Some(body))
            .await
            .1
            .unwrap()
    }

    /// Sends `answer` for the problem on `page`, and returns the reply.
    async fn answer_with(app: &Router, page: &PageDto, answer: Option<u32>) -> AnswerResponse {
        let PageDto::Practice { page_id, .. } = page else {
            panic!("expected a problem, got {page:?}");
        };
        let body = serde_json::json!({ "page_id": page_id, "answer": answer, "ms_taken": 9000 });
        let (status, reply) =
            call(app, Method::POST, "/api/child/kid/answer", Some(body)).await;
        assert_eq!(status, StatusCode::OK);
        reply.unwrap()
    }

    /// The right answer to the problem on `page` (an addition, answer last).
    fn right_answer(page: &PageDto) -> u32 {
        let PageDto::Practice { problem, .. } = page else {
            panic!("expected a problem, got {page:?}");
        };
        problem.a + problem.b
    }

    #[tokio::test]
    async fn a_word_problem_is_told_once_at_its_stage() {
        let file = temp_db("story");
        let app = app(&file).await;
        let page = first_problem(&app).await;
        let PageDto::Practice { problem, story, .. } = &page else {
            panic!("expected a problem");
        };
        // The child starts at the pictures stage, so its story comes first.
        assert_eq!(problem.text, "0 + 1");
        let story = story.clone().expect("a story for 0 + 1");
        let story = *story;
        assert_eq!(story.text, "The builder has 0 stones and 1 more.");
        assert_eq!(story.voice.gentle.name, "stones.gentle");
        // Then the set: its own `0 + 1` is a plain sum, not the story again.
        let reply = answer_with(&app, &page, Some(1)).await;
        let PageDto::Practice { story, .. } = reply.next else {
            panic!("expected a problem");
        };
        assert_eq!(story, None);
        let _ = std::fs::remove_file(&file);
    }

    #[tokio::test]
    async fn a_wrong_answer_brings_its_hint_and_no_praise() {
        let file = temp_db("hint");
        let app = app(&file).await;
        let page = first_problem(&app).await;
        // `0 + 1 = 2` is one too many: the off-by-one hint.
        let reply = answer_with(&app, &page, Some(2)).await;
        assert!(!reply.correct);
        let hint = reply.hint.expect("a hint");
        assert_eq!(hint.text, "Count the new step.");
        assert_eq!(hint.voice.gentle.name, "hint-off.gentle");
        assert_eq!(reply.praise, None);
        assert_eq!(reply.set_end, None);
        let _ = std::fs::remove_file(&file);
    }

    #[tokio::test]
    async fn i_dont_know_gets_the_hint_for_the_stage() {
        let file = temp_db("dontknow");
        let app = app(&file).await;
        let page = first_problem(&app).await;
        let reply = answer_with(&app, &page, None).await;
        assert!(!reply.correct);
        // No hint is written for "unknown", so any hint of the stage is used.
        assert_eq!(reply.hint.map(|h| h.text), Some("Count the new step.".to_string()));
        let _ = std::fs::remove_file(&file);
    }

    #[tokio::test]
    async fn a_right_answer_brings_praise_and_a_finished_set_its_question_and_notice() {
        let file = temp_db("setend");
        let app = app(&file).await;
        let mut page = first_problem(&app).await;
        // The pictures-stage word problem is told first; then the set begins.
        page = answer_with(&app, &page, Some(right_answer(&page))).await.next;
        // Four problems in the set: the notice comes with the last answer only.
        for done in 1..=4 {
            let reply = answer_with(&app, &page, Some(right_answer(&page))).await;
            assert!(reply.correct);
            assert_eq!(reply.hint, None);
            assert_eq!(reply.praise.expect("praise").text, "Correct!");
            if done < 4 {
                assert_eq!(reply.set_end, None, "after answer {done}");
            } else {
                let end = reply.set_end.expect("the set is over");
                assert_eq!(end.question.unwrap().text, "What do you notice?");
                let notice = end.notice.unwrap();
                assert_eq!(notice.text, "Did you see? Each answer is one more.");
                assert_eq!(notice.formal.as_deref(), Some("Each answer is the successor."));
                assert_eq!(notice.voice.gentle.name, "walk.notice.gentle");
                assert_eq!(notice.voice.exceptional.unwrap().name, "walk.notice.exceptional");
            }
            page = reply.next;
        }
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

    /// Opens the test database again, for checks the API cannot make.
    async fn open(file: &std::path::Path) -> SqlitePool {
        db::connect(&format!("sqlite://{}", file.display()))
            .await
            .unwrap()
    }

    /// A second parent with their own child `other-kid` and a live token.
    async fn second_parent(file: &std::path::Path) -> &'static str {
        let pool = open(file).await;
        let parent = db::parent_for_sub(&pool, "parent-two", 0).await.unwrap();
        db::create_session(&pool, "second-token", parent, i64::MAX)
            .await
            .unwrap();
        db::create_child(&pool, parent, "other-kid", Look::Gentle, "7-8", 9)
            .await
            .unwrap();
        "second-token"
    }

    async fn status(app: &Router, token: Option<&str>, method: Method, uri: &str) -> StatusCode {
        call_as::<serde_json::Value>(app, token, method, uri, None)
            .await
            .0
    }

    #[tokio::test]
    async fn without_a_valid_token_everything_is_401() {
        let file = temp_db("401");
        let app = app(&file).await;
        let pool = open(&file).await;
        let parent = db::parent_for_sub(&pool, "parent-one", 0).await.unwrap();
        // A sign-in that ran out long ago.
        db::create_session(&pool, "old-token", parent, 5)
            .await
            .unwrap();
        for uri in ["/api/child/kid/next", "/api/children", "/api/hero-names"] {
            let none = status(&app, None, Method::GET, uri).await;
            assert_eq!(none, StatusCode::UNAUTHORIZED, "{uri}");
            let wrong = status(&app, Some("nonsense"), Method::GET, uri).await;
            assert_eq!(wrong, StatusCode::UNAUTHORIZED, "{uri}");
            let expired = status(&app, Some("old-token"), Method::GET, uri).await;
            assert_eq!(expired, StatusCode::UNAUTHORIZED, "{uri}");
            let valid = status(&app, Some(TOKEN), Method::GET, uri).await;
            assert_eq!(valid, StatusCode::OK, "{uri}");
        }
        let _ = std::fs::remove_file(&file);
    }

    #[tokio::test]
    async fn one_parent_cannot_reach_another_parents_child() {
        let file = temp_db("owner");
        let app = app(&file).await;
        let other = second_parent(&file).await;
        // Each reaches their own child ...
        let mine = status(&app, Some(TOKEN), Method::GET, "/api/child/kid/next").await;
        assert_eq!(mine, StatusCode::OK);
        let theirs = status(&app, Some(other), Method::GET, "/api/child/other-kid/next").await;
        assert_eq!(theirs, StatusCode::OK);
        // ... and gets 404 for the other one, whatever they ask for.
        for (method, uri) in [
            (Method::GET, "/api/child/other-kid/next"),
            (Method::GET, "/api/child/other-kid/audio"),
            (Method::GET, "/api/children/other-kid/export"),
            (Method::DELETE, "/api/children/other-kid"),
        ] {
            let got = status(&app, Some(TOKEN), method, uri).await;
            assert_eq!(got, StatusCode::NOT_FOUND, "{uri}");
        }
        // The list shows only the caller's children.
        let (_, list) =
            call::<Vec<primer_api::ChildDto>>(&app, Method::GET, "/api/children", None).await;
        let ids: Vec<String> = list.unwrap().into_iter().map(|c| c.id).collect();
        assert_eq!(ids, ["kid"]);
        // The other child is still there.
        let still = status(&app, Some(other), Method::GET, "/api/child/other-kid/next").await;
        assert_eq!(still, StatusCode::OK);
        let _ = std::fs::remove_file(&file);
    }

    /// How many rows the table has (written out per table: the SQL stays static).
    async fn count(pool: &SqlitePool, table: &str) -> i64 {
        let query = match table {
            "attempts" => sqlx::query_scalar("SELECT COUNT(*) FROM attempts"),
            _ => sqlx::query_scalar("SELECT COUNT(*) FROM children"),
        };
        query.fetch_one(pool).await.unwrap()
    }

    #[tokio::test]
    async fn deleting_a_child_or_the_account_leaves_no_attempts() {
        let file = temp_db("delete");
        let app = app(&file).await;
        play_right(&app, 3).await;
        let pool = open(&file).await;
        assert!(count(&pool, "attempts").await >= 3);

        // The export has what is stored, before deletion.
        let uri = "/api/children/kid/export";
        let (code, export) = call::<serde_json::Value>(&app, Method::GET, uri, None).await;
        assert_eq!(code, StatusCode::OK);
        assert!(export.unwrap()["attempts"].as_array().unwrap().len() >= 3);

        let gone = status(&app, Some(TOKEN), Method::DELETE, "/api/children/kid").await;
        assert_eq!(gone, StatusCode::NO_CONTENT);
        assert_eq!(count(&pool, "attempts").await, 0);
        let after = status(&app, Some(TOKEN), Method::GET, "/api/child/kid/next").await;
        assert_eq!(after, StatusCode::NOT_FOUND);

        // Deleting the account takes the children and the sign-in with it.
        let new_child = serde_json::json!({ "age_band": "5-6", "look": "gentle" });
        let (_, made) =
            call::<primer_api::ChildDto>(&app, Method::POST, "/api/children", Some(new_child))
                .await;
        let made_uri = format!("/api/child/{}/next", made.unwrap().id);
        assert_eq!(status(&app, Some(TOKEN), Method::GET, &made_uri).await, StatusCode::OK);
        let gone = status(&app, Some(TOKEN), Method::DELETE, "/api/account").await;
        assert_eq!(gone, StatusCode::NO_CONTENT);
        assert_eq!(count(&pool, "children").await, 0);
        let signed_out = status(&app, Some(TOKEN), Method::GET, "/api/children").await;
        assert_eq!(signed_out, StatusCode::UNAUTHORIZED);
        let _ = std::fs::remove_file(&file);
    }

    #[tokio::test]
    async fn the_dev_sign_in_gives_a_session_that_sign_out_ends() {
        let file = temp_db("dev");
        let app = app(&file).await;
        let (code, config) =
            call_as::<primer_api::AuthConfig>(&app, None, Method::GET, "/api/auth/config", None)
                .await;
        assert_eq!(code, StatusCode::OK);
        assert!(config.unwrap().dev_sign_in);
        let (code, session) =
            call_as::<primer_api::SessionDto>(&app, None, Method::POST, "/api/auth/dev", None)
                .await;
        assert_eq!(code, StatusCode::OK);
        let token = session.unwrap().token;
        let listed = status(&app, Some(&token), Method::GET, "/api/children").await;
        assert_eq!(listed, StatusCode::OK);
        let out = status(&app, Some(&token), Method::POST, "/api/auth/sign-out").await;
        assert_eq!(out, StatusCode::NO_CONTENT);
        let after = status(&app, Some(&token), Method::GET, "/api/children").await;
        assert_eq!(after, StatusCode::UNAUTHORIZED);
        // Google sign-in is not set up here.
        let google = serde_json::json!({ "credential": "x" });
        let uri = "/api/auth/google";
        let (code, _) =
            call_as::<serde_json::Value>(&app, None, Method::POST, uri, Some(google)).await;
        assert_eq!(code, StatusCode::NOT_FOUND);
        let _ = std::fs::remove_file(&file);
    }

    #[tokio::test]
    async fn without_the_flag_or_with_google_there_is_no_local_sign_in() {
        use crate::config::SignIn;
        for (name, sign_in) in [("off", SignIn::Off), ("google", SignIn::Google("id".into()))] {
            let file = temp_db(&format!("nodev-{name}"));
            let app = app_signing_in(&file, Arc::new(FakeJudge), sign_in).await;
            let uri = "/api/auth/config";
            let (_, config) =
                call_as::<primer_api::AuthConfig>(&app, None, Method::GET, uri, None).await;
            assert!(!config.unwrap().dev_sign_in, "{name}");
            let dev = status(&app, None, Method::POST, "/api/auth/dev").await;
            assert_eq!(dev, StatusCode::NOT_FOUND, "{name}");
            let _ = std::fs::remove_file(&file);
        }
    }

    /// A judge that calls every typed name identifying.
    struct StrictJudge;

    #[async_trait::async_trait]
    impl crate::judge::Judge for StrictJudge {
        async fn judge(
            &self,
            _: &Problem,
            _: Option<u32>,
            _: Option<&str>,
        ) -> crate::judge::Judgment {
            crate::judge::Judgment::UNKNOWN
        }
        async fn identifying(&self, _: &str) -> bool {
            true
        }
    }

    async fn pick(app: &Router, name: &str) -> (StatusCode, Option<primer_api::ChildDto>) {
        let body = serde_json::json!({ "name": name });
        call(app, Method::POST, "/api/children/kid/hero", Some(body)).await
    }

    #[tokio::test]
    async fn a_hero_name_from_the_list_or_a_typed_one_that_passes_is_saved() {
        let file = temp_db("hero");
        let app = app(&file).await;
        // From the list: accepted, even with a space.
        let (code, child) = pick(&app, "Liu Hui").await;
        assert_eq!(code, StatusCode::OK);
        assert_eq!(child.unwrap().hero_name.as_deref(), Some("Liu Hui"));
        // Typed, one word of letters, and the judge lets it through.
        let (code, child) = pick(&app, "Pip").await;
        assert_eq!(code, StatusCode::OK);
        assert_eq!(child.unwrap().hero_name.as_deref(), Some("Pip"));
        // Typed and refused by the code rule: nothing changes.
        for bad in ["Bob Smith", "Bob2", "Abcdefghijklm", ""] {
            let (code, _) = pick(&app, bad).await;
            assert_eq!(code, StatusCode::UNPROCESSABLE_ENTITY, "{bad}");
        }
        let (_, list) =
            call::<Vec<primer_api::ChildDto>>(&app, Method::GET, "/api/children", None).await;
        assert_eq!(list.unwrap()[0].hero_name.as_deref(), Some("Pip"));
        let _ = std::fs::remove_file(&file);
    }

    #[tokio::test]
    async fn a_typed_name_the_judge_calls_identifying_is_refused_but_the_list_still_works() {
        let file = temp_db("hero-strict");
        let app = app_with(&file, Arc::new(StrictJudge)).await;
        assert_eq!(pick(&app, "Pip").await.0, StatusCode::UNPROCESSABLE_ENTITY);
        // Names on the list never go to the judge.
        assert_eq!(pick(&app, "Noether").await.0, StatusCode::OK);
        let _ = std::fs::remove_file(&file);
    }

    #[tokio::test]
    async fn a_new_profile_starts_with_no_hero_name() {
        let file = temp_db("create");
        let app = app(&file).await;
        let body = serde_json::json!({ "age_band": "7-8", "look": "exceptional" });
        let (code, made) =
            call::<primer_api::ChildDto>(&app, Method::POST, "/api/children", Some(body)).await;
        assert_eq!(code, StatusCode::OK);
        let made = made.unwrap();
        assert_eq!(made.hero_name, None);
        assert_eq!((made.age_band.as_str(), made.look.as_str()), ("7-8", "exceptional"));
        let bad = serde_json::json!({ "age_band": "40-50", "look": "gentle" });
        let (code, _) =
            call::<serde_json::Value>(&app, Method::POST, "/api/children", Some(bad)).await;
        assert_eq!(code, StatusCode::UNPROCESSABLE_ENTITY);
        let _ = std::fs::remove_file(&file);
    }


    /// Plays the main path with right answers until the book runs out
    /// (chapter 2 has nothing written in the test book). Returns the
    /// final page.
    async fn play_to_the_end(app: &Router) -> PageDto {
        let mut page = next(app).await;
        for _ in 0..80 {
            page = match &page {
                PageDto::Lesson { skill, lesson, .. } => {
                    let body = serde_json::json!({ "skill": skill, "lesson": lesson });
                    call(app, Method::POST, "/api/child/kid/lesson-seen", Some(body))
                        .await
                        .1
                        .unwrap()
                }
                PageDto::Practice { .. } => {
                    answer_with(app, &page, Some(right_answer(&page))).await.next
                }
                _ => return page,
            };
        }
        panic!("the book never ended: {page:?}");
    }

    #[tokio::test]
    async fn when_the_written_chapters_run_out_the_page_is_the_end_of_the_book() {
        let file = temp_db("end");
        let app = app(&file).await;
        assert_eq!(play_to_the_end(&app).await, PageDto::EndOfBook);
        // It stays the end of the book; nothing is generated for ever.
        assert_eq!(next(&app).await, PageDto::EndOfBook);
        let _ = std::fs::remove_file(&file);
    }

    #[tokio::test]
    async fn the_contents_show_progress_and_lessons_open_again_without_recording() {
        let file = temp_db("contents");
        let app = app(&file).await;
        let uri = "/api/child/kid/contents";
        let (_, fresh) = call::<primer_api::ContentsDto>(&app, Method::GET, uri, None).await;
        let fresh = fresh.unwrap();
        assert_eq!(fresh.chapters.len(), 1);
        assert_eq!(fresh.chapters[0].title, "The Next Number");
        assert_eq!(fresh.chapters[0].status, "current");
        assert!(fresh.chapters[0].items.iter().all(|i| i.state == "new"));

        play_to_the_end(&app).await;
        let (_, done) = call::<primer_api::ContentsDto>(&app, Method::GET, uri, None).await;
        let chapter = &done.unwrap().chapters[0];
        assert_eq!(chapter.status, "done");
        assert!(chapter.items.iter().all(|i| i.state == "done"), "{:?}", chapter.items);

        // A lesson can be read again; the page is still the end of the book.
        let again = "/api/child/kid/lesson/count-on-within-5/intro";
        let (code, page) = call::<PageDto>(&app, Method::GET, again, None).await;
        assert_eq!(code, StatusCode::OK);
        let PageDto::Lesson { beats, again, opener, .. } = page.unwrap() else {
            panic!("expected the lesson");
        };
        assert!(!beats.is_empty() && !again && opener.is_none());
        let missing = status(&app, Some(TOKEN), Method::GET, "/api/child/kid/lesson/count-on-within-5/nope").await;
        assert_eq!(missing, StatusCode::CONFLICT);
        assert_eq!(next(&app).await, PageDto::EndOfBook);
        let _ = std::fs::remove_file(&file);
    }

    #[tokio::test]
    async fn more_practice_gives_generated_problems_until_the_child_goes_back() {
        let file = temp_db("more");
        let app = app(&file).await;
        play_to_the_end(&app).await;
        let ask = |skill: Option<&str>| {
            let body = serde_json::json!({ "skill": skill });
            call::<PageDto>(&app, Method::POST, "/api/child/kid/practice", Some(body))
        };
        // A chapter with nothing written has no practice to offer.
        assert_eq!(ask(Some("add-within-10")).await.0, StatusCode::CONFLICT);

        let (code, page) = ask(Some("count-on-within-5")).await;
        assert_eq!(code, StatusCode::OK);
        let page = page.unwrap();
        let PageDto::Practice { set_id, .. } = &page else {
            panic!("expected a problem, got {page:?}");
        };
        assert!(set_id.contains('#'), "{set_id}");
        // The problem can be answered, and the next one is more practice.
        let reply = answer_with(&app, &page, Some(right_answer(&page))).await;
        assert!(reply.correct);
        assert!(matches!(reply.next, PageDto::Practice { .. }));
        // It lasts across calls ...
        assert!(matches!(next(&app).await, PageDto::Practice { .. }));
        // ... until the child goes back to the main path.
        let (_, back) = ask(None).await;
        assert_eq!(back.unwrap(), PageDto::EndOfBook);
        assert_eq!(next(&app).await, PageDto::EndOfBook);
        let _ = std::fs::remove_file(&file);
    }

}
