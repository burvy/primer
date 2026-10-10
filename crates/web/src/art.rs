//! Hand-drawn art: loading the SVG files and making them follow the look.
//!
//! The human draws the pictures in Inkscape (see `docs/ART_GUIDE.md`) and
//! saves them under `assets/`. This code never draws what a file already
//! draws; it loads the file, recolours it, and lets CSS show, hide and animate
//! its layers by id. When a file does not exist (yet), the caller keeps
//! drawing its own simple shape, so nothing breaks.
//!
//! Two pure string functions do the work on the text of a drawing:
//! [`recolour`] and [`inner`]. The rest loads files.

use std::cell::RefCell;
use std::collections::HashMap;

use leptos::prelude::*;
use wasm_bindgen::prelude::*;

use crate::api;

/// The Parchment palette (`docs/PALETTE.md`): each hex colour and the name of
/// the CSS variable that holds that colour in the current look.
const PARCHMENT: [(&str, &str); 7] = [
    ("#f4e9d0", "paper"),
    ("#e6d5b0", "paper-shade"),
    ("#3b2a1e", "ink"),
    ("#7a6450", "ink-soft"),
    ("#b07a4a", "warm"),
    ("#d9a441", "bright"),
    ("#a63d2f", "mark"),
];

/// Replaces every Parchment colour in a drawing with `var(--name)`, so the
/// same drawing is Parchment in the Gentle look and Byrne's colours in the
/// Exceptional look: the page's CSS says what each name means.
///
/// The match ignores upper or lower case (Inkscape writes `#f4e9d0`, a
/// palette file may say `#F4E9D0`). Colours that are not in the palette are
/// left alone.
pub fn recolour(svg: &str) -> String {
    let mut text = svg.to_string();
    for (hex, name) in PARCHMENT {
        text = replace_ignoring_case(&text, hex, &format!("var(--{name})"));
    }
    text
}

/// `text` with each `find` (compared ignoring ASCII case) replaced by `with`.
///
/// Lowercasing ASCII letters never changes a string's length, so a position
/// found in the lowercase copy is the same position in the original.
fn replace_ignoring_case(text: &str, find: &str, with: &str) -> String {
    let lower = text.to_ascii_lowercase();
    let mut out = String::with_capacity(text.len());
    let mut from = 0;
    while let Some(found) = lower[from..].find(find) {
        let at = from + found;
        out.push_str(&text[from..at]);
        out.push_str(with);
        from = at + find.len();
    }
    out.push_str(&text[from..]);
    out
}

/// What is inside a drawing's outer `<svg>...</svg>`: everything but the
/// wrapper. This lets the code place a drawing inside its own `<svg>` (at a
/// position and size it chooses). If there is no wrapper, the text is
/// returned as it is.
pub fn inner(svg: &str) -> String {
    let Some(open) = svg.find("<svg") else {
        return svg.to_string();
    };
    let Some(close_of_open) = svg[open..].find('>') else {
        return svg.to_string();
    };
    let start = open + close_of_open + 1;
    match svg.rfind("</svg>") {
        Some(end) if end >= start => svg[start..end].to_string(),
        _ => svg.to_string(),
    }
}

thread_local! {
    /// Drawings already asked for: `None` means the file does not exist.
    /// Every page asks again whenever it is built; this makes only the first
    /// ask a network request.
    static DRAWINGS: RefCell<HashMap<String, Option<String>>> = RefCell::new(HashMap::new());
}

/// A drawing from `assets/`, recoloured, as soon as it arrives.
///
/// `path` is under `/assets/`, like `lessons/coin.svg`. The signal holds
/// `None` until the file arrives, and stays `None` if there is no such file;
/// callers draw their own fallback while it is `None`.
pub fn drawing(path: &str) -> RwSignal<Option<String>> {
    let signal = RwSignal::new(None);
    let known = DRAWINGS.with(|all| all.borrow().get(path).cloned());
    if let Some(found) = known {
        signal.set(found);
        return signal;
    }
    let path = path.to_string();
    leptos::task::spawn_local(async move {
        let found = api::text(&format!("/assets/{path}")).await.ok();
        // Only an SVG counts: a server may answer a missing file with a page.
        let found = found
            .filter(|text| text.contains("<svg"))
            .map(|text| recolour(&text));
        DRAWINGS.with(|all| all.borrow_mut().insert(path, found.clone()));
        signal.try_set(found);
    });
    signal
}

#[wasm_bindgen]
extern "C" {
    /// Tidies a just-loaded machine drawing: drops the inline `display` and
    /// `opacity` the drawing program leaves on hidden layers, and wires up the
    /// block's path from funnel to chute (see `crates/web/art.js`).
    #[wasm_bindgen(js_namespace = primerArt)]
    pub fn prepare(container: &web_sys::Element);

    /// Lays the `.machine` children of `holder` out as a staircase, each
    /// funnel under the chute before it.
    #[wasm_bindgen(js_namespace = primerArt)]
    pub fn stairs(holder: &web_sys::Element);

    /// Writes `text` centred in the element `box_id` inside `container` (see
    /// `crates/web/art.js`). With no such element, writes it in the machine's
    /// `front-panel`, and failing that at (`x`, `y`).
    #[wasm_bindgen(js_namespace = primerArt, js_name = writeIn)]
    pub fn write_in(container: &web_sys::Element, box_id: &str, text: &str, x: f64, y: f64);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parchment_colours_become_variables_whatever_the_case() {
        let svg = r##"<path fill="#F4E9D0" style="stroke:#3b2a1e;fill:#B07A4A"/>"##;
        assert_eq!(
            recolour(svg),
            r#"<path fill="var(--paper)" style="stroke:var(--ink);fill:var(--warm)"/>"#
        );
    }

    #[test]
    fn every_palette_colour_has_its_name() {
        for (hex, name) in PARCHMENT {
            assert_eq!(recolour(hex), format!("var(--{name})"));
        }
    }

    #[test]
    fn other_colours_and_text_are_left_alone() {
        let svg = r##"<rect fill="#123456"/> <text>#f4e9d1</text>"##;
        assert_eq!(recolour(svg), svg);
        assert_eq!(recolour(""), "");
    }

    #[test]
    fn the_same_colour_twice_is_replaced_twice() {
        assert_eq!(
            recolour("#3B2A1E and #3b2a1e"),
            "var(--ink) and var(--ink)"
        );
    }

    #[test]
    fn inner_drops_the_outer_svg_tag() {
        let svg = "<?xml version=\"1.0\"?>\n<svg width=\"40\" viewBox=\"0 0 40 40\"><g id=\"coin\"/></svg>\n";
        assert_eq!(inner(svg), "<g id=\"coin\"/>");
    }

    #[test]
    fn inner_keeps_text_without_a_wrapper() {
        assert_eq!(inner("<g/>"), "<g/>");
        assert_eq!(inner("<svg"), "<svg");
    }
}
