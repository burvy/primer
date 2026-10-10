//! Which page the browser shows: the front door of the app.
//!
//! The flow is: a grown-up signs in; the child picks who is reading (a hero
//! name, or makes one); then the book opens. To get to the grown-up pages
//! (add or delete a reader, export, sign out) there is a grown-up check.
//!
//! There is no URL router. One signal, `route`, holds the page being shown,
//! and each page changes it to move on.

use leptos::prelude::*;
use primer_api::ChildDto;

use crate::api;

use super::hero::Hero;
use super::parent::{Gate, Parent};
use super::play;
use super::privacy::Privacy;
use super::profiles::Profiles;
use super::sign_in::SignIn;

/// The pages. An `enum` holds exactly one at a time, with what that page
/// needs (the child, for the pages that are about one).
#[derive(Clone)]
pub enum Route {
    SignIn,
    /// "Who is reading?"
    Profiles,
    /// A child with no hero name yet picks one.
    Hero(ChildDto),
    /// The book, read to this child.
    Play(ChildDto),
    /// The grown-up check, before the grown-up pages.
    Gate,
    Parent,
    Privacy,
}

/// A page's frame: the same cream page as the book, in the gentle look.
#[component]
pub fn Frame(children: Children) -> impl IntoView {
    view! {
        <div class="shell" data-look="gentle">
            <main class="book panel">{children()}</main>
        </div>
    }
}

#[component]
pub fn Root() -> impl IntoView {
    // A saved sign-in skips the sign-in page. (If it has run out, the first
    // call is refused, the token is forgotten, and the page asks again.)
    let first = if api::load_token().is_some() {
        Route::Profiles
    } else {
        Route::SignIn
    };
    let route = RwSignal::new(first);
    move || match route.get() {
        Route::SignIn => view! { <SignIn route /> }.into_any(),
        Route::Profiles => view! { <Profiles route /> }.into_any(),
        Route::Hero(child) => view! { <Hero child route /> }.into_any(),
        Route::Play(child) => {
            let on_leave = move |()| route.set(Route::Gate);
            view! { <play::App child on_leave /> }.into_any()
        }
        Route::Gate => view! { <Gate route /> }.into_any(),
        Route::Parent => view! { <Parent route /> }.into_any(),
        Route::Privacy => view! { <Privacy route /> }.into_any(),
    }
}
