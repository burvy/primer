//! A new reader picks a hero name: from the list, or typed.
//!
//! Real names are never stored. A typed name goes through a plain rule (one
//! word of letters) and then a check for names that could identify a real
//! person. If either refuses it, the server says why and the child is back
//! at the list.

use leptos::prelude::*;
use leptos::task::spawn_local;
use primer_api::ChildDto;

use crate::api;

use super::root::{Frame, Route};

#[component]
pub fn Hero(child: ChildDto, route: RwSignal<Route>) -> impl IntoView {
    let names = RwSignal::new(Vec::<String>::new());
    let error = RwSignal::new(String::new());
    let typed = RwSignal::new(String::new());
    spawn_local(async move {
        match api::hero_names().await {
            Ok(list) => names.set(list),
            Err(message) => error.set(message),
        }
    });

    // Saves a name (picked or typed). On success the book opens; if the
    // name is refused the reason is shown and the list stays.
    let id = child.id.clone();
    let choose = Callback::new(move |name: String| {
        let id = id.clone();
        spawn_local(async move {
            match api::set_hero(&id, &name).await {
                Ok(updated) => route.set(Route::Play(updated)),
                Err(message) => {
                    typed.set(String::new());
                    error.set(message);
                }
            }
        });
    });

    view! {
        <Frame>
            <h1>"Pick your hero name"</h1>
            <div class="choices">
                {move || {
                    names
                        .get()
                        .into_iter()
                        .map(|name| {
                            let pick = name.clone();
                            view! {
                                <button type="button" class="next choice"
                                    on:click=move |_| choose.run(pick.clone())>
                                    {name}
                                </button>
                            }
                        })
                        .collect_view()
                }}
            </div>
            <p>"Or type a name you make up. Do not use a real name."</p>
            <p>
                <input type="text" maxlength="12" aria-label="A hero name"
                    prop:value=move || typed.get()
                    on:input=move |event| typed.set(event_target_value(&event)) />
                <button type="button" class="next"
                    disabled=move || typed.get().trim().is_empty()
                    on:click=move |_| choose.run(typed.get_untracked())>
                    "Use this name"
                </button>
            </p>
            <p class="note error">{move || error.get()}</p>
            <p>
                <button type="button" class="link" on:click=move |_| route.set(Route::Profiles)>
                    "Back"
                </button>
            </p>
        </Frame>
    }
}
