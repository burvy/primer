//! The grown-up pages: behind a grown-up check, a parent adds and deletes
//! readers, exports a child's data, signs out, or deletes the whole account.

use leptos::prelude::*;
use leptos::task::spawn_local;
use primer_api::ChildDto;
use primer_core::grown_up;

use crate::api;

use super::root::{Frame, Route};

/// The grown-up check: a multiplication a young child cannot do. It stands
/// between child mode and the pages below. It is a gate, not security: the
/// parent has already signed in.
#[component]
pub fn Gate(route: RwSignal<Route>) -> impl IntoView {
    // The clock picks the question. A wrong answer picks another.
    let seed = RwSignal::new(js_sys::Date::now() as u32);
    let typed = RwSignal::new(String::new());
    let wrong = RwSignal::new(false);
    let try_it = move |_| {
        let right = typed.get_untracked().trim().parse().is_ok_and(|n| grown_up::is_right(seed.get_untracked(), n));
        if right {
            route.set(Route::Parent);
        } else {
            wrong.set(true);
            typed.set(String::new());
            seed.update(|s| *s = s.wrapping_add(1));
        }
    };
    view! {
        <Frame>
            <h1>"For grown-ups"</h1>
            <p>"To go on, answer this."</p>
            <p class="gate-question">
                {move || {
                    let (a, b) = grown_up::question(seed.get());
                    format!("What is {a} × {b}?")
                }}
            </p>
            <p>
                <input type="text" inputmode="numeric" aria-label="Your answer"
                    prop:value=move || typed.get()
                    on:input=move |event| typed.set(event_target_value(&event)) />
                <button type="button" class="next" on:click=try_it>"Go on"</button>
            </p>
            <p class="note error">{move || wrong.get().then_some("Not quite. Try this one.")}</p>
            <p>
                <button type="button" class="quiet" on:click=move |_| route.set(Route::Profiles)>
                    "Back to reading"
                </button>
            </p>
        </Frame>
    }
}

/// Reloads the list of readers.
fn refresh(children: RwSignal<Option<Vec<ChildDto>>>, error: RwSignal<String>) {
    spawn_local(async move {
        match api::children().await {
            Ok(all) => children.set(Some(all)),
            Err(message) => error.set(message),
        }
    });
}

/// What a button should say the first time and the second time it is
/// tapped, for things that cannot be undone.
fn twice(armed: bool, first: &'static str, second: &'static str) -> &'static str {
    if armed { second } else { first }
}

#[component]
pub fn Parent(route: RwSignal<Route>) -> impl IntoView {
    // `None` until the list has arrived, so "no readers yet" is not flashed.
    let children = RwSignal::new(None::<Vec<ChildDto>>);
    let error = RwSignal::new(String::new());
    // The reader whose "Delete" was tapped once (tap again to delete).
    let deleting = RwSignal::new(None::<String>);
    let deleting_account = RwSignal::new(false);
    // The export, once fetched: a link the parent taps to save the file.
    let export = RwSignal::new(None::<(String, String)>);
    let look = RwSignal::new("gentle".to_string());
    let band = RwSignal::new("5-6".to_string());
    refresh(children, error);

    let add = move |_| {
        let (look, band) = (look.get_untracked(), band.get_untracked());
        spawn_local(async move {
            match api::create_child(&look, &band).await {
                Ok(_) => refresh(children, error),
                Err(message) => error.set(message),
            }
        });
    };
    let sign_out = move |_| {
        spawn_local(async move {
            api::sign_out().await;
            route.set(Route::SignIn);
        });
    };
    let delete_account = move |_| {
        if !deleting_account.get_untracked() {
            deleting_account.set(true);
            return;
        }
        spawn_local(async move {
            match api::delete_account().await {
                Ok(()) => route.set(Route::SignIn),
                Err(message) => error.set(message),
            }
        });
    };

    view! {
        <Frame>
            <h1>"Grown-ups"</h1>
            <section class="group">
            <h2>"Readers"</h2>
            {move || {
                let all = children.get()?;
                if all.is_empty() {
                    return Some(
                        view! { <p class="note">"No readers yet. Add one below."</p> }.into_any(),
                    );
                }
                Some(all
                    .into_iter()
                    .map(|child| {
                        let name = child.hero_name.clone().unwrap_or_else(|| "(no hero name yet)".to_string());
                        let (for_export, for_delete) = (child.clone(), child.id.clone());
                        let delete_id = for_delete.clone();
                        let on_export = move |_| {
                            let child = for_export.clone();
                            spawn_local(async move {
                                match api::export_child(&child.id).await {
                                    Ok(text) => {
                                        let encoded = String::from(js_sys::encode_uri_component(&text));
                                        let href = format!("data:application/json;charset=utf-8,{encoded}");
                                        let file = format!("primer-{}.json", child.hero_name.unwrap_or(child.id));
                                        export.set(Some((file, href)));
                                    }
                                    Err(message) => error.set(message),
                                }
                            });
                        };
                        let on_delete = move |_| {
                            if deleting.get_untracked().as_deref() != Some(&delete_id) {
                                deleting.set(Some(delete_id.clone()));
                                return;
                            }
                            let id = delete_id.clone();
                            spawn_local(async move {
                                match api::delete_child(&id).await {
                                    Ok(()) => {
                                        deleting.set(None);
                                        refresh(children, error);
                                    }
                                    Err(message) => error.set(message),
                                }
                            });
                        };
                        let armed = move || deleting.get().as_deref() == Some(&for_delete);
                        view! {
                            <div class="reader">
                                <p class="reader-name">{name}</p>
                                <p class="small">
                                    {format!("age {} · {} look", child.age_band, child.look)}
                                </p>
                                <p class="reader-actions">
                                    <button type="button" class="quiet" on:click=on_export>
                                        "Export"
                                    </button>
                                    <button type="button" class="danger" on:click=on_delete>
                                        {move || twice(armed(), "Delete", "Really delete? Tap again")}
                                    </button>
                                </p>
                            </div>
                        }
                    })
                    .collect_view()
                    .into_any())
            }}
            {move || {
                export.get().map(|(file, href)| {
                    view! { <p><a href=href download=file>"Save the export file"</a></p> }
                })
            }}
            </section>
            <section class="group">
            <h2>"Add a reader"</h2>
            <p>
                <label>"Age "
                    <select on:change=move |event| band.set(event_target_value(&event))>
                        <option value="5-6">"5 to 6"</option>
                        <option value="7-8">"7 to 8"</option>
                    </select>
                </label>
                " "
                <label>"Look "
                    <select on:change=move |event| look.set(event_target_value(&event))>
                        <option value="gentle">"Gentle"</option>
                        <option value="exceptional">"Exceptional"</option>
                    </select>
                </label>
                " "
                <button type="button" class="next" on:click=add>"Add reader"</button>
            </p>
            </section>
            <section class="group">
            <h2>"Your account"</h2>
            <p>
                <button type="button" class="quiet" on:click=sign_out>"Sign out"</button>
            </p>
            <p>
                <button type="button" class="danger" on:click=delete_account>
                    {move || twice(
                        deleting_account.get(),
                        "Delete my account and all readers",
                        "Really delete everything? Tap again",
                    )}
                </button>
            </p>
            </section>
            <p class="note error">{move || error.get()}</p>
            <p>
                <button type="button" class="link" on:click=move |_| route.set(Route::Privacy)>
                    "Privacy"
                </button>
                " "
                <button type="button" class="quiet" on:click=move |_| route.set(Route::Profiles)>
                    "Back to reading"
                </button>
            </p>
        </Frame>
    }
}
