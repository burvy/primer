//! Three small tick boxes, like a printed form: Voice, Sounds, Music.

use leptos::prelude::*;
use primer_api::AudioSettings;

/// One tick box. `read` says whether the box is ticked; `flip` turns the
/// switch. They are plain functions (`fn`), so each box needs no extra state.
#[component]
fn Tick(
    label: &'static str,
    settings: RwSignal<AudioSettings>,
    read: fn(&AudioSettings) -> bool,
    flip: fn(&mut AudioSettings),
    #[prop(into)] on_change: Callback<AudioSettings>,
) -> impl IntoView {
    view! {
        <button type="button" class="tickbox"
            aria-pressed=move || read(&settings.get()).to_string()
            on:click=move |_| {
                settings.update(flip);
                on_change.run(settings.get_untracked());
            }>
            <span class="box" aria-hidden="true"></span>
            {label}
        </button>
    }
}

/// The three switches. `on_change` receives the new settings after each tap;
/// the page tells the server and the audio.
#[component]
pub fn AudioControls(
    settings: RwSignal<AudioSettings>,
    #[prop(into)] on_change: Callback<AudioSettings>,
) -> impl IntoView {
    view! {
        <div class="audio-controls" role="group" aria-label="Sound">
            <Tick label="Voice" settings on_change
                read=|s| s.narration flip=|s| s.narration = !s.narration />
            <Tick label="Sounds" settings on_change
                read=|s| s.sounds flip=|s| s.sounds = !s.sounds />
            <Tick label="Music" settings on_change
                read=|s| s.music flip=|s| s.music = !s.music />
        </div>
    }
}
