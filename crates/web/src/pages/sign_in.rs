//! The grown-up signs in. Children have no login.
//!
//! With a Google client id on the server, Google's own button is shown. With
//! none, a clearly-labelled local sign-in stands in, so the prototype runs on
//! a laptop with no Google account set up.

use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::prelude::*;

use crate::api;

use super::root::{Frame, Route};

#[wasm_bindgen]
extern "C" {
    /// Draws Google's button inside `element` (see `crates/web/signin.js`);
    /// `on_credential` is called with Google's token when the parent signs in.
    #[wasm_bindgen(js_namespace = primerSignIn)]
    fn render(client_id: &str, element: &web_sys::Element, on_credential: &js_sys::Function);
}

/// Google's button. When the parent signs in, `on_credential` gets the token.
#[component]
fn GoogleButton(client_id: String, #[prop(into)] on_credential: Callback<String>) -> impl IntoView {
    let holder = NodeRef::<leptos::html::Div>::new();
    Effect::new(move |_| {
        if let Some(div) = holder.get() {
            // A closure is Rust code JavaScript can call. `forget` hands it
            // to JavaScript for good (it is made once per visit to this page).
            let callback = Closure::<dyn Fn(String)>::new(move |credential: String| {
                on_credential.run(credential);
            });
            render(&client_id, &div, callback.as_ref().unchecked_ref());
            callback.forget();
        }
    });
    view! { <div node_ref=holder></div> }
}

#[component]
pub fn SignIn(route: RwSignal<Route>) -> impl IntoView {
    let config = RwSignal::new(None);
    let error = RwSignal::new(String::new());
    spawn_local(async move {
        match api::auth_config().await {
            Ok(found) => config.set(Some(found)),
            Err(message) => error.set(message),
        }
    });

    // Both sign-ins end the same way: on to "who is reading?".
    let finish = move |result: Result<(), String>| match result {
        Ok(()) => route.set(Route::Profiles),
        Err(message) => error.set(message),
    };
    let on_google = move |credential: String| {
        spawn_local(async move { finish(api::google_sign_in(&credential).await) });
    };
    let on_dev = move |_| spawn_local(async move { finish(api::dev_sign_in().await) });

    view! {
        <Frame>
            <h1>"The Primer"</h1>
            <p>"A grown-up signs in here. Children do not need to."</p>
            {move || {
                config.get().map(|found| match found.google_client_id {
                    Some(client_id) => {
                        view! { <GoogleButton client_id on_credential=on_google /> }.into_any()
                    }
                    None => view! {
                        <p class="note">
                            "Local test sign-in: no Google account is set up on this server."
                        </p>
                        <button type="button" class="next" on:click=on_dev>
                            "Sign in (local test)"
                        </button>
                    }
                    .into_any(),
                })
            }}
            <p class="note error">{move || error.get()}</p>
            <p>
                <button type="button" class="link" on:click=move |_| route.set(Route::Privacy)>
                    "Privacy"
                </button>
            </p>
        </Frame>
    }
}
