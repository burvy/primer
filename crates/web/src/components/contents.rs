//! The table of contents: the written chapters, with the child's progress,
//! and a way into any lesson again.

use leptos::prelude::*;
use primer_api::{ChapterDto, ContentsDto};

/// The contents page.
///
/// - `resume` says whether there is somewhere to carry on reading
///   (`on_continue`); at the end of the book so far there is not.
/// - Tapping a lesson calls `on_lesson(skill, lesson)`; a set or story only
///   shows how far the child got.
/// - "More practice" calls `on_more(skill)`.
#[component]
pub fn Contents(
    toc: ContentsDto,
    resume: bool,
    #[prop(into)] on_continue: Callback<()>,
    #[prop(into)] on_lesson: Callback<(String, String)>,
    #[prop(into)] on_more: Callback<String>,
) -> impl IntoView {
    let chapters = toc
        .chapters
        .into_iter()
        .map(|chapter| view! { <Chapter chapter on_lesson on_more /> })
        .collect_view();
    view! {
        <section class="contents">
            <h1>"Contents"</h1>
            {if resume {
                view! {
                    <button type="button" class="next" on:click=move |_| on_continue.run(())>
                        "Carry on reading ▸"
                    </button>
                }
                .into_any()
            } else {
                view! { <p class="note">"You have read to the end of the book so far."</p> }
                    .into_any()
            }}
            {chapters}
        </section>
    }
}

/// What to show beside a chapter's title.
fn badge(status: &str) -> &'static str {
    match status {
        "done" => "✓ finished",
        "current" => "◂ you are here",
        _ => "",
    }
}

/// What to show beside an item: how far the child got.
fn progress(kind: &str, state: &str, done: u32, total: u32) -> String {
    match (kind, state) {
        (_, "done") => "✓".to_string(),
        ("set", "started") => format!("{done} of {total}"),
        _ => String::new(),
    }
}

#[component]
fn Chapter(
    chapter: ChapterDto,
    on_lesson: Callback<(String, String)>,
    on_more: Callback<String>,
) -> impl IntoView {
    let skill = chapter.skill.clone();
    let more = chapter.skill.clone();
    let items = chapter
        .items
        .into_iter()
        .map(|item| {
            let note = progress(&item.kind, &item.state, item.done, item.total);
            let label = item.label.clone();
            // A lesson opens again when tapped; the rest only show progress.
            let body = if item.kind == "lesson" {
                let (skill, lesson) = (skill.clone(), item.id.clone());
                view! {
                    <button type="button" class="link"
                        on:click=move |_| on_lesson.run((skill.clone(), lesson.clone()))>
                        {label}
                    </button>
                }
                .into_any()
            } else {
                view! { <span>{label}</span> }.into_any()
            };
            view! {
                <li class=format!("item-{}", item.state)>
                    {body}" "<span class="progress">{note}</span>
                </li>
            }
        })
        .collect_view();
    view! {
        <section class=format!("chapter chapter-{}", chapter.status)>
            <h2>
                {format!("Chapter {} · {}", chapter.number, chapter.title)}
                " "
                <span class="badge">{badge(&chapter.status)}</span>
            </h2>
            <ul>{items}</ul>
            <button type="button" class="link" on:click=move |_| on_more.run(more.clone())>
                "More practice"
            </button>
        </section>
    }
}
