//! The equation, with a box (□) where the child's answer goes.

use leptos::prelude::*;
use primer_api::ProblemDto;

use crate::art;

/// Shows `3 + 4 = □`, `3 + □ = 7` or `□ + 4 = 7`, depending on where the box
/// sits. `typed` is a *signal*: a value the page watches. Whenever it changes,
/// only the box is redrawn.
#[component]
pub fn Equation(problem: ProblemDto, #[prop(into)] typed: Signal<String>) -> impl IntoView {
    let (symbol, whole) = if problem.op == "add" {
        ('+', problem.a + problem.b)
    } else {
        ('−', problem.a - problem.b)
    };
    let the_box = move || {
        let text = typed.get();
        let text = if text.is_empty() {
            "\u{a0}".to_string()
        } else {
            text
        };
        view! { <span class="box">{text}</span> }
    };
    let (a, b) = (problem.a, problem.b);
    match problem.blank.as_str() {
        "second" => view! { <p class="equation">{a} " " {symbol} " " {the_box} " = " {whole}</p> }
            .into_any(),
        "first" => view! { <p class="equation">{the_box} " " {symbol} " " {b} " = " {whole}</p> }
            .into_any(),
        _ => view! { <p class="equation">{a} " " {symbol} " " {b} " = " {the_box}</p> }.into_any(),
    }
}

/// An equation written as text, like `3 + 4 = 7`, with one part lit up.
///
/// `highlight` is `"addends"` (the numbers being added), `"equals"` (the `=`
/// sign) or `"answer"` (what follows `=`). Each word of the label is wrapped
/// in a `<span>`; the lit ones get the class `hl`, which `play.css` styles.
/// If `assets/lessons/highlights.svg` exists, a hand-drawn stroke is drawn
/// over each lit word instead (see [`Stroke`]).
#[component]
pub fn LabelEquation(label: String, highlight: Option<String>) -> impl IntoView {
    let words: Vec<String> = label.split(' ').map(String::from).collect();
    let equals_at = words.iter().position(|w| w == "=");
    let parts = words
        .into_iter()
        .enumerate()
        .map(|(i, word)| {
            // Before `=` and a number means an addend.
            let kind = match equals_at {
                Some(eq) if i < eq && word.chars().all(|c| c.is_ascii_digit()) => "addends",
                Some(eq) if i == eq => "equals",
                Some(eq) if i > eq => "answer",
                _ => "",
            };
            let lit = highlight.as_deref() == Some(kind);
            let stroke = lit.then(|| view! { <Stroke kind=kind /> });
            view! { <span class:hl=lit>{word}{stroke}" "</span> }
        })
        .collect_view();
    view! { <p class="equation">{parts}</p> }
}

/// A hand-drawn stroke laid over the word it marks: a bracket under the
/// addends, an underline for the `=` sign, a circle round the answer.
///
/// The drawing `highlights.svg` holds four layers (`circle`, `underline`,
/// `arrow`, `bracket`). This stretches it over the word and `play.css` shows
/// just the layer that fits and draws it on, stroke by stroke. With no file
/// this draws nothing, and the word keeps its plain colour and underline.
#[component]
fn Stroke(kind: &'static str) -> impl IntoView {
    let art = art::drawing("lessons/highlights.svg");
    let layer = match kind {
        "addends" => "bracket",
        "equals" => "underline",
        _ => "circle",
    };
    move || {
        art.get().map(|svg| {
            view! {
                <svg class=format!("mark-stroke show-{layer}") viewBox="0 0 400 400"
                    preserveAspectRatio="none" inner_html=art::inner(&svg)></svg>
            }
        })
    }
}

/// A sentence from the server. The server has already escaped the words and
/// turned each `$...$` of LaTeX into MathML, so this only inserts that HTML
/// (`inner_html`). The browser draws the `<math>` parts in the maths font.
#[component]
pub fn RichText(text: String) -> impl IntoView {
    view! { <span inner_html=text></span> }
}
