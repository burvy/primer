//! Placeholder browser app. The real UI arrives in a later slice; this only
//! proves Trunk can compile the crate to WebAssembly.

use wasm_bindgen::prelude::*;

/// Runs automatically when the browser loads the wasm file.
#[wasm_bindgen(start)]
fn start() {}

fn main() {}
