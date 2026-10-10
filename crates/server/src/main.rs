//! The Primer server: serves the built web app and the answer API.
//!
//! Settings come from environment variables, with local defaults:
//! `PRIMER_ADDR` (127.0.0.1:3000), `DATABASE_URL` (sqlite://primer.db) and
//! `PRIMER_DIST` (crates/web/dist, the folder `trunk build` writes) and
//! `PRIMER_CONTENT` (content, the folder of chapter files) and `PRIMER_ASSETS`
//! (assets, the folder of drawings, recordings and sounds).
//! `PRIMER_GOOGLE_CLIENT_ID` (none: local dev sign-in instead of Google's).
//! `PRIMER_JEV_KEY_FILE` (jev-key.env) names the Jev key file; with no such
//! file the server uses the fake judge.
//!
//! `primer-server voices` does not start the server: it makes placeholder
//! recordings and writes `assets/voice/RECORDINGS.md` (see `voices.rs`).

use std::path::PathBuf;
use std::sync::Arc;

use primer_server::routes::AppState;
use primer_server::{config, content, db, jev, judge, routes, voices};

/// `#[tokio::main]` turns `async fn main` into a normal `main` that starts
/// tokio, the engine that runs async code, and waits for it to finish.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = std::env::var("PRIMER_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".into());
    let database = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://primer.db".into());
    let dist = std::env::var("PRIMER_DIST").unwrap_or_else(|_| "crates/web/dist".into());
    let content_dir = std::env::var("PRIMER_CONTENT").unwrap_or_else(|_| "content".into());
    let assets = std::env::var("PRIMER_ASSETS").unwrap_or_else(|_| "assets".into());

    // The book is checked before anything starts. Broken content is a
    // refusal to start, listing every problem, not a surprise for a child.
    let content = content::Content::load(std::path::Path::new(&content_dir)).map_err(|errors| {
        let lines: Vec<String> = errors.iter().map(|e| format!("  {e}")).collect();
        format!("the content is broken:\n{}", lines.join("\n"))
    })?;
    // `voices` is a chore, not the server: do it and stop.
    if std::env::args().nth(1).as_deref() == Some("voices") {
        let script = std::path::Path::new("tools/voice/placeholders.ps1");
        return voices::run(&content, std::path::Path::new(&assets), script).map_err(Into::into);
    }
    for gap in &content.gaps {
        println!("not written yet: {gap}");
    }

    let pool = db::connect(&database).await?;

    // The real judge only when a key file exists; otherwise the fake one.
    let judge: judge::SharedJudge = match config::JevConfig::from_env() {
        Some(jev) => Arc::new(jev::JevJudge::new(jev)?),
        None => {
            println!("no Jev key file: using the fake judge");
            Arc::new(judge::FakeJudge)
        }
    };
    // Parents sign in with Google when a client id is set; without one the
    // page offers a local sign-in so the prototype still runs.
    let google_client_id = std::env::var("PRIMER_GOOGLE_CLIENT_ID")
        .ok()
        .filter(|id| !id.is_empty());
    if google_client_id.is_none() {
        println!("no PRIMER_GOOGLE_CLIENT_ID: parents sign in with the local dev sign-in");
    }
    let state = AppState {
        pool,
        judge,
        content: Arc::new(content),
        google_client_id,
        http: reqwest::Client::new(),
    };
    let app = routes::router(state, PathBuf::from(dist), PathBuf::from(assets));

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("primer listening on http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}
