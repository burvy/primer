//! Ada saying one short line from the book: a hint, a word of praise, a
//! set's question or notice, or the story of a word problem.

use leptos::prelude::*;
use primer_api::{LineDto, NoteDto};
use primer_core::Look;

use crate::audio;

use super::equation::RichText;
use super::lesson::Ada;

/// The HTML and the line to read aloud for `look`: the Exceptional look uses
/// the `formal` wording when the note has one (and then it has its own line),
/// otherwise both looks read the plain one.
fn wording(note: &NoteDto, look: Look) -> (String, LineDto) {
    match (look, &note.formal, &note.voice.exceptional) {
        (Look::Exceptional, Some(formal), Some(line)) => (formal.clone(), line.clone()),
        _ => (note.text.clone(), note.voice.gentle.clone()),
    }
}

/// Ada, with the line beside her. The line is read aloud when it appears,
/// and again if the look changes or the child taps Ada. Leaving the page
/// stops her mid-sentence.
#[component]
pub fn AdaSays(note: NoteDto, look: RwSignal<Look>) -> impl IntoView {
    let note = StoredValue::new(note);
    let read = move |look: Look| {
        let (_, line) = note.with_value(|n| wording(n, look));
        audio::say(&line.name, &line.speech);
    };
    Effect::new(move |_| read(look.get()));
    on_cleanup(audio::stop);
    view! {
        <div class="says">
            <Ada on_tap=move |()| read(look.get_untracked()) />
            <p class="said">
                {move || {
                    let (html, _) = note.with_value(|n| wording(n, look.get()));
                    view! { <RichText text=html /> }
                }}
            </p>
        </div>
    }
}
