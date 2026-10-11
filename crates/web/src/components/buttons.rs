//! Buttons that are not plain boxes.
//!
//! - [`Turn`]: a word with a hand-drawn underline, for "go on" (Next,
//!   Continue, Begin...). The underline is the `underline` layer of
//!   `assets/lessons/highlights.svg`; without that file the button keeps a
//!   plain CSS underline.
//! - [`Mark`]: the tick or cross beside a checked answer, from
//!   `assets/lessons/marks.svg`; without that file, a typed tick or cross.

use leptos::prelude::*;

use crate::art;

use super::equation::Stroke;

/// "Go on": an underlined word. `children` is the word.
#[component]
pub fn Turn(#[prop(into)] on_click: Callback<()>, children: Children) -> impl IntoView {
    view! {
        <button type="button" class="turn" on:click=move |_| on_click.run(())>
            {children()}
            <Stroke kind="equals" />
        </button>
    }
}

/// A tick (`right`) or a cross, drawn by hand when the file exists.
#[component]
pub fn Mark(right: bool) -> impl IntoView {
    let art = art::drawing("lessons/marks.svg");
    let (layer, word, glyph) = if right {
        ("tick", "Right", "\u{2713}")
    } else {
        ("cross", "Not right", "\u{2717}")
    };
    move || match art.get() {
        Some(svg) => view! {
            <svg class=format!("verdict-mark show-{layer}") viewBox="0 0 100 100"
                role="img" aria-label=word inner_html=art::inner(&svg)></svg>
        }
        .into_any(),
        None => view! {
            <span class=format!("verdict-mark glyph show-{layer}") role="img" aria-label=word>
                {glyph}
            </span>
        }
        .into_any(),
    }
}
