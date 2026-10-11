//! Every word problem the human wrote reaches the child.
//!
//! Walks the real chapters 1 and 2 in both looks with a child who gets every
//! answer right (the fastest way through, so the most likely to skip
//! something), and checks that each written word problem was told.

use std::collections::BTreeSet;
use std::path::Path;

use primer_core::policy::{self, story_id};
use primer_core::{Attempt, Event, Learner, Look, Outcome, Page, Strategy};
use primer_server::content::Content;
use primer_server::content::schema::{ChapterFile, LookTag};

const CHAPTERS: [&str; 2] = ["01-count-on.toml", "02-add.toml"];

fn content_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")
}

/// The word problem ids written for `look` in chapters 1 and 2.
fn written_stories(look: Look) -> BTreeSet<String> {
    CHAPTERS
        .iter()
        .flat_map(|name| {
            let text = std::fs::read_to_string(content_dir().join(name)).unwrap();
            toml::from_str::<ChapterFile>(&text).unwrap().word_problem
        })
        .filter(|story| match story.look {
            LookTag::Both => true,
            LookTag::Gentle => look == Look::Gentle,
            LookTag::Exceptional => look == Look::Exceptional,
        })
        .map(|story| story.id)
        .collect()
}

/// The story ids a child in `look` is told before leaving chapter 2.
fn stories_told(content: &Content, look: Look) -> BTreeSet<String> {
    let in_chapters_1_2 = ["count-on-within-5", "add-within-10"];
    let mut events = Vec::new();
    let mut told = BTreeSet::new();
    for _ in 0..500 {
        let page = policy::next(
            &Learner::from_events(&events),
            content.chapters(look),
            look,
            7,
        );
        let event = match &page {
            Page::Lesson { skill, .. } | Page::Practice { skill, .. }
                if !in_chapters_1_2.contains(skill) =>
            {
                break;
            }
            Page::Story => break,
            Page::Lesson { skill, lesson, .. } => Event::LessonSeen {
                skill,
                lesson: lesson.clone(),
            },
            Page::Practice {
                skill,
                set_id,
                problem,
                stage,
                ..
            } => {
                told.extend(story_id(set_id).map(str::to_string));
                Event::Attempt(Attempt {
                    skill,
                    set_id: set_id.clone(),
                    problem: *problem,
                    stage: *stage,
                    look,
                    answer: Some(problem.answer()),
                    outcome: Outcome::Correct,
                    ms_taken: 3_000,
                    strategy: Strategy::Unknown,
                    frustrated: false,
                    reflection: Default::default(),
                })
            }
        };
        events.push(event);
    }
    told
}

#[test]
fn every_word_problem_in_chapters_1_and_2_is_told() {
    let content = Content::load(&content_dir()).expect("the real content loads");
    for look in [Look::Gentle, Look::Exceptional] {
        let written = written_stories(look);
        assert!(written.len() >= 5, "{look:?}: {written:?}");
        assert_eq!(stories_told(&content, look), written, "{look:?}");
    }
}
