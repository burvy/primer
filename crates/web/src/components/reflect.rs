//! Ada asks the child something after an answer: "How did you work it out?"
//! or "How are you feeling?". The child answers by tapping a picture.
//!
//! A tap reads the choice aloud and marks it, so a child who cannot read yet
//! can hear every choice before deciding; "Done" sends the marked one.
//! "Skip" goes on without an answer. Nothing is ever typed.

use leptos::prelude::*;
use primer_api::{ChoiceDto, QuestionDto};
use primer_core::Look;

use crate::audio;

use super::equation::{RichText, Stroke};
use super::notes::{AdaSays, wording};

/// One question and its choices. `on_done` gets the id of the choice
/// tapped, or `None` when the child skips.
#[component]
pub fn Reflect(
    question: QuestionDto,
    look: RwSignal<Look>,
    #[prop(into)] on_done: Callback<Option<String>>,
) -> impl IntoView {
    let picked: RwSignal<Option<String>> = RwSignal::new(None);
    let choices = question.choices.into_iter().map(|choice| {
        view! { <Choice choice picked look /> }
    });
    view! {
        <AdaSays note=question.question look />
        <div class="choices">{choices.collect_view()}</div>
        <div class="choice-actions">
            <button type="button" class="quiet" on:click=move |_| on_done.run(None)>
                "Skip"
            </button>
            <button type="button" class="turn"
                disabled=move || picked.get().is_none()
                on:click=move |_| on_done.run(picked.get_untracked())>
                "Done ▸"
                <Stroke kind="equals" />
            </button>
        </div>
    }
}

/// One big picture-and-word button. Tapping it reads its words aloud and
/// marks it as the child's answer.
#[component]
fn Choice(
    choice: ChoiceDto,
    picked: RwSignal<Option<String>>,
    look: RwSignal<Look>,
) -> impl IntoView {
    let ChoiceDto { id, picture, note } = choice;
    let note = StoredValue::new(note);
    // A `Memo` is a derived signal: "is this the marked choice?", worked out
    // once per change and read by both the class and the aria attribute.
    let mine = id.clone();
    let is_picked = Memo::new(move |_| picked.get().as_deref() == Some(mine.as_str()));
    let on_tap = move |_| {
        let (_, line) = note.with_value(|n| wording(n, look.get_untracked()));
        audio::say(&line.name, &line.speech);
        picked.set(Some(id.clone()));
    };
    view! {
        <button type="button" class="choice"
            class:picked=move || is_picked.get()
            aria-pressed=move || is_picked.get().to_string()
            on:click=on_tap>
            <span class="picture" aria-hidden="true">{picture}</span>
            <span class="words">
                {move || {
                    let (html, _) = note.with_value(|n| wording(n, look.get()));
                    view! { <RichText text=html /> }
                }}
            </span>
        </button>
    }
}
