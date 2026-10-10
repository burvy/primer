//! The Primer server: serves the built web app and the answer API.
//!
//! Settings come from environment variables, with local defaults:
//! `PRIMER_ADDR` (127.0.0.1:3000), `DATABASE_URL` (sqlite://primer.db) and
//! `PRIMER_DIST` (crates/web/dist, the folder `trunk build` writes).

mod content;
mod db;
mod judge;
mod names;
mod routes;

use std::path::PathBuf;
use std::sync::Arc;

use routes::AppState;

/// `#[tokio::main]` turns `async fn main` into a normal `main` that starts
/// tokio, the engine that runs async code, and waits for it to finish.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = std::env::var("PRIMER_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".into());
    let database = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://primer.db".into());
    let dist = std::env::var("PRIMER_DIST").unwrap_or_else(|_| "crates/web/dist".into());

    let pool = db::connect(&database).await?;

    // With `dev-local` there is no sign-in yet, so one built-in child exists.
    #[cfg(feature = "dev-local")]
    db::ensure_child(&pool, "dev", primer_core::Look::Gentle, 42).await?;

    let state = AppState {
        pool,
        judge: Arc::new(judge::FakeJudge),
        chapters: Arc::new(content::chapters()),
    };
    let app = routes::router(state, PathBuf::from(dist));

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("primer listening on http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}
