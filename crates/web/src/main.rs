//! The browser app. Trunk compiles this crate to WebAssembly.

mod api;
mod audio;
mod components;
mod pages;

fn main() {
    leptos::mount::mount_to_body(pages::play::App);
}
