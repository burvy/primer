//! The pages that make it a book: the cover, and each chapter's opener.

use leptos::prelude::*;
use primer_api::OpenerDto;

use crate::art;
use crate::audio;

/// Until children have names of their own (a later slice), the cover says
/// this where the hero's name goes.
const HERO: &str = "Reader";

/// The cover: shown once, before the first lesson.
///
/// If `assets/cover.svg` exists, it is the cover: the hero's name is written
/// into its empty `name-box`, and its own hand-lettered title is used. Without
/// it the cover is typeset. `on_open` turns the page.
#[component]
pub fn Cover(#[prop(into)] on_open: Callback<()>) -> impl IntoView {
    let art = art::drawing("cover.svg");
    let holder = NodeRef::<leptos::html::Div>::new();
    Effect::new(move |_| {
        if let (Some(_), Some(div)) = (art.get(), holder.get()) {
            art::write_in(&div, "name-box", HERO, 300.0, 780.0);
        }
    });
    view! {
        <section class="cover">
            {move || match art.get() {
                Some(svg) => view! {
                    <div class="cover-art" node_ref=holder role="img"
                        aria-label=format!("The Primer, for {HERO}") inner_html=svg></div>
                }
                .into_any(),
                None => view! {
                    <div class="cover-typeset">
                        <h1>"The Primer"</h1>
                        <p class="cover-for">"for"</p>
                        <p class="cover-name">{HERO}</p>
                    </div>
                }
                .into_any(),
            }}
            <button type="button" class="next" on:click=move |_| {
                audio::sound("page-turn");
                on_open.run(());
            }>"Open the book ▸"</button>
        </section>
    }
}

/// A chapter's opening page: its number, its title, and (if
/// `assets/chapter-<n>.svg` exists) its picture.
#[component]
pub fn ChapterOpener(opener: OpenerDto, #[prop(into)] on_begin: Callback<()>) -> impl IntoView {
    let art = art::drawing(&format!("chapter-{}.svg", opener.number));
    view! {
        <section class="opener">
            <p class="chapter-number">{format!("Chapter {}", opener.number)}</p>
            <h1>{opener.title}</h1>
            {move || art.get().map(|svg| view! { <div class="opener-art" inner_html=svg></div> })}
            <button type="button" class="next" on:click=move |_| {
                audio::sound("page-turn");
                on_begin.run(());
            }>"Begin ▸"</button>
        </section>
    }
}
