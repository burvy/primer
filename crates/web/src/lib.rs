//! The browser app, as a library. Trunk compiles this crate to WebAssembly.
//!
//! Two ways in:
//! - [`run`]: the standalone app, mounted on the page body, talking to the
//!   server that served it (`main.rs` calls this).
//! - [`mount`]: for another site (burvy-dev) that hosts Primer inside one of
//!   its own pages. That site says where to put the app, and where the
//!   Primer server and the drawings and sounds are.
//!
//! Primer has no URL router (one signal holds the page; see `pages/root.rs`),
//! so there is no router base path to configure, and it cannot clash with the
//! host site's router.

mod api;
mod art;
mod audio;
mod components;
mod pages;

pub use api::Config;

/// The standalone app: on the page body, same-origin server, `/assets`.
pub fn run() {
    leptos::mount::mount_to_body(pages::root::Root);
}

/// Mounts the app inside `element` for a host site. The app lives until the
/// page is closed. `config` says where the server and the assets are.
pub fn mount(element: web_sys::HtmlElement, config: Config) {
    api::configure(config);
    leptos::mount::mount_to(element, pages::root::Root).forget();
}
