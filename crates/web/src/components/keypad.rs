//! The on-screen keypad. It only draws buttons and reports which one was
//! tapped; what a tap *means* is decided by the pure `primer_core::press`.

use leptos::prelude::*;
use primer_core::Key;

#[component]
pub fn Keypad(#[prop(into)] on_key: Callback<Key>) -> impl IntoView {
    let digit = move |d: u8| {
        view! { <button type="button" on:click=move |_| on_key.run(Key::Digit(d))>{d}</button> }
    };
    view! {
        <div class="keypad" aria-label="Number keypad">
            {(1..=9).map(digit).collect_view()}
            <button type="button" aria-label="Backspace" on:click=move |_| on_key.run(Key::Back)>
                "⌫"
            </button>
            {digit(0)}
            // The last slot of the grid: not a digit, so it has its own look.
            <button type="button" class="dont-know" on:click=move |_| on_key.run(Key::DontKnow)>
                "I don't know"
            </button>
        </div>
    }
}
