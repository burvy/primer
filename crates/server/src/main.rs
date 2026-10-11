//! The Primer server: serves the built web app and the answer API.
//!
//! Settings come from environment variables, with local defaults:
//! `PRIMER_ADDR` (127.0.0.1:3000), `DATABASE_URL` (sqlite://primer.db) and
//! `PRIMER_DIST` (crates/web/dist, the folder `trunk build` writes) and
//! `PRIMER_CONTENT` (content, the folder of chapter files) and `PRIMER_ASSETS`
//! (assets, the folder of drawings, recordings and sounds).
//! `PRIMER_GOOGLE_CLIENT_ID` turns on Google sign-in. `PRIMER_DEV_SIGNIN=1`
//! turns on the local test sign-in instead (never both: the server refuses
//! to start). With neither, nobody can sign in.
//! `PRIMER_TLS_CERT` and `PRIMER_TLS_KEY` (both, or neither) make it serve
//! https from those files; `PRIMER_ALLOWED_ORIGINS` (like
//! `https://burvy.dev`, comma-separated) lets those pages call it from a
//! browser. Both are for when another site hosts the app; unset, nothing
//! changes.
//! `PRIMER_JEV_KEY_FILE` (jev-key.env) names the Jev key file; with no such
//! file the server uses the fake judge.
//!
//! `primer-server voices` does not start the server: it makes placeholder
//! recordings and writes `assets/voice/RECORDINGS.md` (see `voices.rs`).

use std::path::PathBuf;
use std::sync::Arc;

use axum::http::{HeaderValue, Method, header};
use axum_server::tls_rustls::RustlsConfig;
use primer_server::routes::AppState;
use primer_server::{config, content, db, jev, judge, routes, voices};
use tower_http::cors::CorsLayer;

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

    // Google sign-in, the local test sign-in, or neither. A bad mix of
    // settings stops the server here, before it listens.
    let sign_in = config::SignIn::from_env()?;
    match &sign_in {
        config::SignIn::Google(_) => println!("sign-in: Google"),
        config::SignIn::Dev => println!("sign-in: LOCAL TEST (PRIMER_DEV_SIGNIN=1), never deploy"),
        config::SignIn::Off => println!("sign-in: none (no Google id, no PRIMER_DEV_SIGNIN)"),
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
    let state = AppState {
        pool,
        judge,
        content: Arc::new(content),
        sign_in,
        http: reqwest::Client::new(),
    };
    let mut app = routes::router(state, PathBuf::from(dist), PathBuf::from(assets));

    // Another site (burvy.dev) hosting the app is a different origin, so the
    // browser only lets it call us if we say so. (Same as plgroup-server.)
    let origins = config::allowed_origins(std::env::var("PRIMER_ALLOWED_ORIGINS").ok())?;
    if let Some(origins) = origins {
        let allowed = origins
            .iter()
            .map(|o| o.parse::<HeaderValue>())
            .collect::<Result<Vec<_>, _>>()?;
        println!("cors: {}", origins.join(", "));
        app = app.layer(
            CorsLayer::new()
                .allow_origin(allowed)
                .allow_methods([Method::GET, Method::POST, Method::DELETE])
                .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE]),
        );
    }

    match config::Tls::from_env()? {
        Some(tls) => {
            // The certificate is read once, now; restart to pick up a new one.
            let rustls = RustlsConfig::from_pem_file(&tls.cert, &tls.key).await?;
            println!("primer listening on https://{addr}");
            axum_server::bind_rustls(addr.parse::<std::net::SocketAddr>()?, rustls)
                .serve(app.into_make_service())
                .await?;
        }
        None => {
            let listener = tokio::net::TcpListener::bind(&addr).await?;
            println!("primer listening on http://{addr}");
            axum::serve(listener, app).await?;
        }
    }
    Ok(())
}
