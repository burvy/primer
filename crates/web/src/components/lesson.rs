//! A lesson: a list of beats, shown one tap at a time.

use leptos::prelude::*;
use primer_api::BeatDto;
use primer_core::Look;

use crate::art;
use crate::audio;

use super::equation::{LabelEquation, RichText};
use super::pictures::{Counters, Machine, Machines, NumberLine, Pouches, Tower};

/// The picture for one beat. `match` on the `show` word picks the drawing;
/// the beat's small settings (`count`, `at`, ...) are handed to it.
#[component]
fn Picture(beat: BeatDto) -> impl IntoView {
    match beat.show.as_str() {
        "counters" => {
            view! { <Counters count=beat.count.unwrap_or(0) adding=beat.adding.unwrap_or(0) /> }
                .into_any()
        }
        "number-line" => {
            let (at, hop) = (beat.at.unwrap_or(0), beat.hop.unwrap_or(0));
            view! { <NumberLine max=(at + hop).max(5) from=Some(at) hop=hop forward=true /> }
                .into_any()
        }
        "machine" => view! {
            <Machine
                input=beat.input
                output=beat.output
                highlight=beat.highlight.clone()
                open=beat.open.unwrap_or(false)
                label=beat.label.clone()
            />
        }
        .into_any(),
        "machines" => view! {
            <Machines
                input=beat.input
                output=beat.output
                highlight=beat.highlight.clone()
                open=beat.open.unwrap_or(false)
                label=beat.label.clone()
            />
        }
        .into_any(),
        "tower" => {
            view! { <Tower count=beat.count.unwrap_or(0) adding=beat.adding.unwrap_or(0) /> }
                .into_any()
        }
        "pouches" => view! {
            <Pouches
                left=beat.left.unwrap_or(0)
                right=beat.right.unwrap_or(0)
                join=beat.join.unwrap_or(false)
                swap=beat.swap.unwrap_or(false)
                total=beat.total
            />
        }
        .into_any(),
        "equation" => view! {
            <LabelEquation
                label=beat.label.clone().unwrap_or_default()
                highlight=beat.highlight.clone()
            />
        }
        .into_any(),
        other => view! { <p class="note">{format!("[picture \"{other}\" not drawn yet]")}</p> }
            .into_any(),
    }
}

/// Ada, the narrator. The hand-drawn `assets/lessons/ada.svg` if the file
/// exists, else the silhouette from the stylesheet. In the drawing, `play.css`
/// blinks her (`eyes-open` / `eyes-closed`) and opens her mouth (`mouth-open`)
/// while `<body>` has the class `speaking`. Tapping her reads the line again.
#[component]
fn Ada(#[prop(into)] on_tap: Callback<()>) -> impl IntoView {
    let art = art::drawing("lessons/ada.svg");
    view! {
        <div class="ada" class:has-art=move || art.get().is_some()
            role="button" tabindex="0"
            aria-label="Ada, the narrator. Tap to hear her again."
            inner_html=move || art.get().unwrap_or_default()
            on:click=move |_| on_tap.run(())></div>
    }
}

/// Shows beat number `step` of `beats`. Tapping the button moves on; after the
/// last beat it calls `on_done`. The Exceptional look shows each beat's
/// `formal` wording instead of `text` when it has one.
#[component]
pub fn LessonPage(
    beats: Vec<BeatDto>,
    look: RwSignal<Look>,
    #[prop(into)] on_done: Callback<()>,
) -> impl IntoView {
    let step = RwSignal::new(0usize);
    let last = beats.len().saturating_sub(1);
    let beats = StoredValue::new(beats);

    // Ada reads beat `at` aloud, in the line that fits the look: the
    // `exceptional` one when the beat has it, else the `gentle` one.
    let read_aloud = move |at: usize, look: Look| {
        let Some(voice) = beats.with_value(|all| all.get(at).and_then(|b| b.voice.clone())) else {
            return;
        };
        let line = match (look, voice.exceptional) {
            (Look::Exceptional, Some(formal)) => formal,
            _ => voice.gentle,
        };
        audio::say(&line.name, &line.speech);
    };
    // Each new beat is read, and gets its sound if its picture has one.
    // The Effect re-runs when the beat or the look changes.
    Effect::new(move |_| {
        let at = step.get();
        read_aloud(at, look.get());
        let beat = beats.with_value(|all| all.get(at).cloned()).unwrap_or_default();
        match beat.show.as_str() {
            "pouches" if beat.join == Some(true) => audio::sound("coin"),
            "number-line" if beat.hop.unwrap_or(0) > 0 => audio::sound("hop"),
            _ => {}
        }
    });
    // Leaving the lesson stops Ada mid-sentence.
    on_cleanup(audio::stop);

    view! {
        <div class="lesson">
            <Ada on_tap=move |()| read_aloud(step.get_untracked(), look.get_untracked()) />
            {move || {
                let beat = beats.with_value(|all| all.get(step.get()).cloned()).unwrap_or_default();
                let text = match (look.get(), &beat.formal) {
                    (Look::Exceptional, Some(formal)) => formal.clone(),
                    _ => beat.text.clone(),
                };
                // `leaf` plays the page-turn each time a new beat is drawn.
                view! {
                    <div class="leaf">
                        <Picture beat=beat />
                        <p><RichText text=text /></p>
                    </div>
                }
            }}
            <button type="button" class="next" on:click=move |_| {
                audio::sound("page-turn");
                if step.get() >= last {
                    on_done.run(());
                } else {
                    step.update(|s| *s += 1);
                }
            }>
                {move || if step.get() >= last { "Done" } else { "Next ▸" }}
            </button>
        </div>
    }
}
