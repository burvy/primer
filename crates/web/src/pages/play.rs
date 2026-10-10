//! The play page: asks the server what comes next and shows it.
//!
//! State lives in *signals*: values the page watches. When a signal changes,
//! only the parts of the page that read it are redrawn. Here there are two
//! at the top: the child's `look` and the current `Screen`. Everything else
//! (what to teach, what is right) is decided by the server and the core; this
//! file only shows pages and sends taps.

use leptos::prelude::*;
use leptos::task::spawn_local;
use primer_api::{AnswerRequest, AudioSettings, OpenerDto, PageDto, ProblemDto};
use primer_core::{Input, Key, Look, press};

use crate::api;
use crate::audio;
use crate::components::audio_controls::AudioControls;
use crate::components::book::{ChapterOpener, Cover};
use crate::components::equation::Equation;
use crate::components::keypad::Keypad;
use crate::components::lesson::LessonPage;
use crate::components::look_switch::{LookSwitch, word};
use crate::components::pictures::{NumberLine, TenFrame, cells};

/// What the page is showing. An `enum` makes the impossible impossible: the
/// page is loading, or broken, or on a server page, or showing the verdict.
#[derive(Clone)]
enum Screen {
    Loading,
    Failed(String),
    Page(PageDto),
    /// The cover, before the very first lesson. `0` is the lesson page behind it.
    Cover(PageDto),
    /// A chapter's opening page, before its first lesson.
    Opener(PageDto),
    /// Shown after an answer; `next` is where "Continue" goes.
    Feedback {
        correct: bool,
        next: PageDto,
    },
}

/// Turns the result of a call into the next screen. A lesson that starts a
/// chapter shows the chapter's opening page first.
fn show(result: Result<PageDto, String>) -> Screen {
    match result {
        Ok(page @ PageDto::Lesson { opener: Some(_), .. }) => Screen::Opener(page),
        Ok(page) => Screen::Page(page),
        Err(message) => Screen::Failed(message),
    }
}

/// Like [`show`], but for the first page of a visit: the very first chapter
/// is preceded by the cover.
fn show_first(result: Result<PageDto, String>) -> Screen {
    match show(result) {
        Screen::Opener(page @ PageDto::Lesson { opener: Some(OpenerDto { number: 1, .. }), .. }) => {
            Screen::Cover(page)
        }
        other => other,
    }
}

/// The same lesson without its opener: the page after the opener is turned.
fn without_opener(page: PageDto) -> PageDto {
    match page {
        PageDto::Lesson { skill, lesson, again, beats, .. } => PageDto::Lesson {
            skill,
            lesson,
            again,
            beats,
            opener: None,
        },
        other => other,
    }
}

/// The book page frame. `children` is whatever is placed inside the tags.
#[component]
fn BookPage(children: Children) -> impl IntoView {
    view! { <main class="book">{children()}</main> }
}

#[component]
pub fn App() -> impl IntoView {
    let look = RwSignal::new(Look::Gentle);
    let screen = RwSignal::new(Screen::Loading);
    // What the child wants to hear. The server remembers it per child, so
    // the first thing the page does is ask.
    let sound = RwSignal::new(AudioSettings::default());
    spawn_local(async move {
        if let Ok(saved) = api::audio().await {
            sound.set(saved);
        }
    });
    // Whenever the switches change, tell the audio. An `Effect` is code that
    // re-runs when a signal it reads changes.
    Effect::new(move |_| {
        let s = sound.get();
        audio::configure(s.narration, s.sounds, s.music);
    });
    let on_sound = move |now: AudioSettings| {
        spawn_local(async move {
            let _ = api::set_audio(&now).await;
        });
    };

    // Start in Gentle, and tell the server so both sides agree. (Remembering
    // the look per child arrives with child profiles, slice 10.)
    spawn_local(async move { screen.set(show_first(api::set_look(word(Look::Gentle)).await)) });

    // Switching looks keeps the child on the page they are reading, so it
    // never goes back to an opener.
    let on_switch = move |to: Look| {
        look.set(to);
        spawn_local(async move {
            screen.set(match api::set_look(word(to)).await {
                Ok(page) => Screen::Page(page),
                Err(message) => Screen::Failed(message),
            })
        });
    };

    view! {
        <div class="shell" data-look=move || word(look.get())>
            <BookPage>
                <LookSwitch look=look on_switch=on_switch />
                <AudioControls settings=sound on_change=on_sound />
                // A fresh `leaf` for every screen turns the page. A lesson draws
                // its own leaf for each beat, so it is left out here.
                {move || {
                    let now = screen.get();
                    let turns = !matches!(&now, Screen::Page(PageDto::Lesson { .. }));
                    view! { <div class:leaf=turns>{screen_view(now, screen, look)}</div> }
                }}
            </BookPage>
        </div>
    }
}

/// What one screen looks like.
fn screen_view(now: Screen, screen: RwSignal<Screen>, look: RwSignal<Look>) -> AnyView {
    match now {
    Screen::Loading => view! { <p class="note">"Opening the book…"</p> }.into_any(),
    Screen::Failed(message) => {
        view! { <p class="note">{format!("Something went wrong: {message}")}</p> }
            .into_any()
    }
    Screen::Feedback { correct, next } => view! { <Feedback correct next screen /> }.into_any(),
    Screen::Cover(page) => {
        let on_open = move |()| screen.set(show(Ok(page.clone())));
        view! { <Cover on_open /> }.into_any()
    }
    Screen::Opener(page) => {
        match page.clone() {
            PageDto::Lesson { opener: Some(opener), .. } => {
                let on_begin =
                    move |()| screen.set(Screen::Page(without_opener(page.clone())));
                view! { <ChapterOpener opener on_begin /> }.into_any()
            }
            _ => ().into_any(),
        }
    }
    Screen::Page(PageDto::Lesson { skill, lesson, beats, .. }) => {
        let on_done = move |()| {
            let (skill, lesson) = (skill.clone(), lesson.clone());
            spawn_local(async move {
                screen.set(show(api::lesson_seen(&skill, &lesson).await));
            });
        };
        view! { <LessonPage beats look on_done /> }.into_any()
    }
    Screen::Page(PageDto::Practice { page_id, problem, stage, .. }) => {
        view! { <Practice page_id problem stage screen /> }.into_any()
    }
    Screen::Page(PageDto::Story) => {
        view! { <p>"The end of this part of the book."</p> }.into_any()
    }
    }
}

/// One problem: a picture that fits the stage, the equation, the keypad.
#[component]
fn Practice(
    page_id: String,
    problem: ProblemDto,
    stage: String,
    screen: RwSignal<Screen>,
) -> impl IntoView {
    let input = RwSignal::new(Input::Empty);
    let busy = RwSignal::new(false);
    let started = js_sys::Date::now();

    // Sends the answer once. `Callback` is a small copyable handle on a
    // function, so the keypad and the button can both use it.
    let submit = Callback::new(move |()| {
        if busy.get_untracked() || !input.get_untracked().is_ready() {
            return;
        }
        busy.set(true);
        let body = AnswerRequest {
            page_id: page_id.clone(),
            answer: input.get_untracked().answer(),
            ms_taken: (js_sys::Date::now() - started) as u32,
        };
        spawn_local(async move {
            screen.set(match api::answer(&body).await {
                Ok(reply) => Screen::Feedback {
                    correct: reply.correct,
                    next: reply.next,
                },
                Err(message) => Screen::Failed(message),
            });
        });
    });
    let on_key = move |key: Key| {
        input.update(|current| *current = press(*current, key));
        if key == Key::DontKnow {
            submit.run(());
        }
    };

    // Pictures help for every shape. When the answer is the missing part the
    // picture shows the sum; otherwise it shows the known numbers and leaves
    // the hidden one as an empty dashed circle or ring.
    let adding = problem.op == "add";
    let (a, b) = (problem.a, problem.b);
    let whole = if adding { a + b } else { a - b };
    let picture = match stage.as_str() {
        "pictorial" => {
            let shown = cells(&problem);
            if shown.is_empty() {
                ().into_any()
            } else {
                view! { <TenFrame cells=shown /> }.into_any()
            }
        }
        "number_line" => {
            let max = if a.max(whole).max(a + b) <= 10 {
                10
            } else {
                20
            };
            match problem.blank.as_str() {
                "answer" => {
                    view! { <NumberLine max from=Some(a) hop=b forward=adding /> }.into_any()
                }
                "second" => {
                    view! { <NumberLine max from=Some(a) hop=0 forward=adding target=whole /> }
                        .into_any()
                }
                _ => view! { <NumberLine max from=None hop=0 forward=adding target=whole /> }
                    .into_any(),
            }
        }
        _ => ().into_any(),
    };

    view! {
        {picture}
        <Equation problem=problem typed=Signal::derive(move || input.get().shown()) />
        <Keypad on_key=on_key />
        <button type="button" class="go"
            disabled=move || !input.get().is_ready() || busy.get()
            on:click=move |_| submit.run(())>
            "Check"
        </button>
    }
}

/// The verdict, then on to the next page.
#[component]
fn Feedback(correct: bool, next: PageDto, screen: RwSignal<Screen>) -> impl IntoView {
    // A right answer chimes.
    if correct {
        audio::sound("chime");
    }
    // STUB wording: the human writes the real lines.
    let message = if correct {
        "Yes, that's right."
    } else {
        "Not quite. Let's keep going."
    };
    view! {
        <p class="note">{message}</p>
        <button type="button" class="next" on:click=move |_| screen.set(Screen::Page(next.clone()))>
            "Continue ▸"
        </button>
    }
}
