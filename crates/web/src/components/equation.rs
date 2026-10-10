//! The equation, with a box (□) where the child's answer goes.

use leptos::prelude::*;
use primer_api::ProblemDto;

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

/// Text with maths inside `$...$` (LaTeX). Real MathML rendering is slice 12;
/// until then the maths parts are shown as written, in the maths font.
/// `split('$')` cuts the text at each dollar sign, so every second piece is
/// maths.
#[component]
pub fn RichText(text: String) -> impl IntoView {
    text.split('$')
        .enumerate()
        .map(|(i, piece)| {
            if i % 2 == 1 {
                view! { <span class="maths">{piece.to_string()}</span> }.into_any()
            } else {
                view! { <span>{piece.to_string()}</span> }.into_any()
            }
        })
        .collect_view()
}
