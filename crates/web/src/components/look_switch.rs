//! The button that flips between the two looks.

use leptos::prelude::*;
use primer_core::Look;

/// The word the server and the stylesheet use for a look.
pub fn word(look: Look) -> &'static str {
    match look {
        Look::Gentle => "gentle",
        Look::Exceptional => "exceptional",
    }
}

/// Shows the label for the *other* look. `on_switch` receives the look being
/// switched to; the page decides what to do (tell the server, redraw).
#[component]
pub fn LookSwitch(look: RwSignal<Look>, #[prop(into)] on_switch: Callback<Look>) -> impl IntoView {
    let other = move || match look.get() {
        Look::Gentle => Look::Exceptional,
        Look::Exceptional => Look::Gentle,
    };
    view! {
        <button type="button" class="look-switch" on:click=move |_| on_switch.run(other())>
            {move || format!("Show {} look", word(other()))}
        </button>
    }
}
