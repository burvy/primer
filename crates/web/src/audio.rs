//! Ada's voice and the book's sounds, called from the page.
//!
//! The browser does the playing. `crates/web/audio.js` holds the few lines
//! that talk to the browser's audio, and this file only says which functions
//! of it Rust may call. `extern "C"` with `#[wasm_bindgen]` is how Rust names
//! functions that live in JavaScript: the compiler trusts the signatures and
//! writes the glue.
//!
//! The JavaScript decides what to play (a real recording, a placeholder, or
//! the browser's own voice) and waits for the child's first tap.

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    /// What the child switched on: narration, sounds, music.
    #[wasm_bindgen(js_namespace = primerAudio)]
    pub fn configure(narration: bool, sounds: bool, music: bool);

    /// Ada reads one line. `name` finds a recording, `speech` is the fallback.
    #[wasm_bindgen(js_namespace = primerAudio)]
    pub fn say(name: &str, speech: &str);

    /// Ada stops reading.
    #[wasm_bindgen(js_namespace = primerAudio)]
    pub fn stop();

    /// One short sound: `page-turn`, `coin`, `hop` or `chime`.
    #[wasm_bindgen(js_namespace = primerAudio)]
    pub fn sound(name: &str);
}
