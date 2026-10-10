//! The privacy page. DRAFT wording: the human must check it, and check
//! TypeSafe's terms and the current FTC rule (this is not legal advice).

use leptos::prelude::*;

use crate::api;

use super::root::{Frame, Route};

#[component]
pub fn Privacy(route: RwSignal<Route>) -> impl IntoView {
    // Back to wherever the visitor came from: the sign-in page, or the
    // grown-up page if they are signed in.
    let back = move |_| {
        route.set(match api::load_token() {
            Some(_) => Route::Parent,
            None => Route::SignIn,
        })
    };
    view! {
        <Frame>
            <h1>"Privacy"</h1>
            <p class="note">"Draft wording, to be checked by the people who run this book."</p>
            <ul>
                <li>"A grown-up owns everything. Children have no login."</li>
                <li>"We keep a hero name, an age band (5 to 6, or 7 to 8), a look, and the answers given. No real names are kept, for anyone."</li>
                <li>"The parent's sign-in is Google's. We keep only the permanent number Google gives the account: no email address and no name."</li>
                <li>"When a child types how they worked something out, it goes to one service (TypeSafe) to be read, and is then dropped. Only a short label of the method is kept."</li>
                <li>"A grown-up can export a child's data, delete a child, or delete the whole account at any time. Deleting removes everything, answers included."</li>
            </ul>
            <p>
                <button type="button" class="next" on:click=back>"Back"</button>
            </p>
        </Frame>
    }
}
