//! "Who is reading?": the child taps their hero name to open the book.

use leptos::prelude::*;
use leptos::task::spawn_local;
use primer_api::ChildDto;

use crate::api;

use super::root::{Frame, Route};

#[component]
pub fn Profiles(route: RwSignal<Route>) -> impl IntoView {
    let children = RwSignal::new(None::<Vec<ChildDto>>);
    let error = RwSignal::new(String::new());
    spawn_local(async move {
        match api::children().await {
            Ok(all) => children.set(Some(all)),
            // A refused sign-in (it ran out) goes back to the sign-in page.
            Err(_) if api::load_token().is_none() => route.set(Route::SignIn),
            Err(message) => error.set(message),
        }
    });

    view! {
        <Frame>
            <h1>"Who is reading?"</h1>
            {move || {
                children.get().map(|all| {
                    if all.is_empty() {
                        view! {
                            <p>"There is no reader yet. A grown-up can add one."</p>
                        }
                        .into_any()
                    } else {
                        let buttons = all
                            .into_iter()
                            .map(|child| {
                                // A reader with no hero name yet picks one first.
                                let label = child
                                    .hero_name
                                    .clone()
                                    .unwrap_or_else(|| "New reader".to_string());
                                let go = child.clone();
                                view! {
                                    <button type="button" class="next choice"
                                        on:click=move |_| {
                                            route.set(match go.hero_name {
                                                Some(_) => Route::Play(go.clone()),
                                                None => Route::Hero(go.clone()),
                                            })
                                        }>
                                        {label}
                                    </button>
                                }
                            })
                            .collect_view();
                        view! { <div class="choices">{buttons}</div> }.into_any()
                    }
                })
            }}
            <p class="note error">{move || error.get()}</p>
            <p>
                <button type="button" class="link" on:click=move |_| route.set(Route::Gate)>
                    "Grown-ups"
                </button>
            </p>
        </Frame>
    }
}
