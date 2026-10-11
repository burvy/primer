//! The table of contents: the written chapters with the child's progress on
//! each lesson, set and story.
//!
//! One pure function. It looks only at the chapters, the titles and the
//! learner it is given, so it is tested with a few lines and no server.

use primer_api::{ChapterDto, ContentsDto, ItemDto};
use primer_core::policy::{Chapter, Step, story_set_id};
use primer_core::{Learner, SKILLS};

/// The contents for a child. `title` finds a chapter's title by skill id,
/// and `current` is the skill the child is on now (the one the policy would
/// show next), if any.
///
/// Chapters come in ladder order, each numbered by its place on the ladder,
/// the same number its opener page shows. A chapter with no written steps
/// is left out: there is nothing in it to open.
pub fn build(
    chapters: &[Chapter],
    title: impl Fn(&str) -> String,
    learner: &Learner,
    current: Option<&str>,
) -> ContentsDto {
    let listed = SKILLS
        .iter()
        .enumerate()
        .filter_map(|(place, skill)| {
            let chapter = chapters.iter().find(|c| c.skill == skill.id)?;
            (!chapter.steps.is_empty()).then_some((place, skill.id, chapter))
        })
        .map(|(place, skill, chapter)| {
            let status = if learner.mastered(skill) {
                "done"
            } else if current == Some(skill) {
                "current"
            } else {
                "later"
            };
            ChapterDto {
                number: place as u32 + 1,
                skill: skill.to_string(),
                title: title(skill),
                status: status.to_string(),
                items: items(skill, chapter, learner),
            }
        })
        .collect();
    ContentsDto { chapters: listed }
}

/// The lessons, sets and stories of one chapter, labelled by kind and by
/// how many of that kind came before ("Lesson 1", "Practice 2").
fn items(skill: &'static str, chapter: &Chapter, learner: &Learner) -> Vec<ItemDto> {
    let (mut lessons, mut sets, mut stories) = (0, 0, 0);
    chapter
        .steps
        .iter()
        .map(|step| match step {
            Step::Lesson(id) => {
                lessons += 1;
                let seen = learner.lessons_seen.contains(&(skill, id.clone()));
                item("lesson", id, format!("Lesson {lessons}"), seen as u32, 1)
            }
            Step::Set(set) => {
                sets += 1;
                let done = learner.answered.get(&set.id).copied().unwrap_or(0);
                let total = set.problems.len();
                item("set", &set.id, format!("Practice {sets}"), done.min(total) as u32, total as u32)
            }
            Step::Story(story) => {
                stories += 1;
                let told = learner.answered.get(&story_set_id(&story.id)).copied().unwrap_or(0);
                item("story", &story.id, format!("Story {stories}"), told.min(1) as u32, 1)
            }
        })
        .collect()
}

fn item(kind: &str, id: &str, label: String, done: u32, total: u32) -> ItemDto {
    let state = match done {
        0 => "new",
        d if d >= total => "done",
        _ => "started",
    };
    ItemDto {
        kind: kind.to_string(),
        id: id.to_string(),
        label,
        state: state.to_string(),
        done,
        total,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use primer_core::policy::Story;
    use primer_core::{Attempt, Event, Op, Outcome, Problem, ProblemSet, Stage, Strategy};

    const CH1: &str = "count-on-within-5";
    const CH2: &str = "add-within-10";

    fn problem(a: u32, b: u32) -> Problem {
        Problem::new(a, Op::Add, b).unwrap()
    }

    fn chapter1() -> Chapter {
        let set = ProblemSet {
            id: "walk".into(),
            skill: CH1,
            problems: vec![problem(0, 1), problem(1, 1), problem(2, 1)],
            pattern: None,
            notice: None,
        };
        let story = Story {
            id: "tower".into(),
            problem: problem(3, 1),
            stage: Stage::Pictorial,
        };
        Chapter {
            skill: CH1,
            steps: vec![
                Step::Lesson("intro".into()),
                Step::Set(set),
                Step::Lesson("two".into()),
                Step::Story(story),
            ],
        }
    }

    fn attempt(skill: &'static str, set_id: &str, p: Problem) -> Event {
        Event::Attempt(Attempt {
            skill,
            set_id: set_id.into(),
            problem: p,
            stage: Stage::Pictorial,
            look: primer_core::Look::Gentle,
            answer: Some(p.answer()),
            outcome: Outcome::Correct,
            ms_taken: 3_000,
            strategy: Strategy::Unknown,
            frustrated: false,
            reflection: Default::default(),
        })
    }

    fn title(skill: &str) -> String {
        format!("Title of {skill}")
    }

    #[test]
    fn items_are_labelled_in_the_order_they_are_met() {
        let toc = build(&[chapter1()], title, &Learner::default(), Some(CH1));
        assert_eq!(toc.chapters.len(), 1);
        let chapter = &toc.chapters[0];
        assert_eq!((chapter.number, chapter.status.as_str()), (1, "current"));
        assert_eq!(chapter.title, "Title of count-on-within-5");
        let labels: Vec<&str> = chapter.items.iter().map(|i| i.label.as_str()).collect();
        assert_eq!(labels, ["Lesson 1", "Practice 1", "Lesson 2", "Story 1"]);
        assert!(chapter.items.iter().all(|i| i.state == "new"));
    }

    #[test]
    fn progress_comes_from_the_learner() {
        let events = [
            Event::LessonSeen { skill: CH1, lesson: "intro".into() },
            attempt(CH1, "walk", problem(0, 1)),
            attempt(CH1, "walk", problem(1, 1)),
            attempt(CH1, "story:tower", problem(3, 1)),
        ];
        let learner = Learner::from_events(&events);
        let toc = build(&[chapter1()], title, &learner, Some(CH1));
        let states: Vec<(&str, u32, u32)> = toc.chapters[0]
            .items
            .iter()
            .map(|i| (i.state.as_str(), i.done, i.total))
            .collect();
        assert_eq!(
            states,
            [("done", 1, 1), ("started", 2, 3), ("new", 0, 1), ("done", 1, 1)]
        );
    }

    #[test]
    fn chapters_follow_the_ladder_and_unwritten_ones_are_left_out() {
        let mut second = chapter1();
        second.skill = CH2;
        // Handed over in the wrong order, and with an empty chapter between.
        let empty = Chapter { skill: "missing-addend-within-10", steps: vec![] };
        let toc = build(&[second, empty, chapter1()], title, &Learner::default(), None);
        let numbers: Vec<(u32, &str)> =
            toc.chapters.iter().map(|c| (c.number, c.skill.as_str())).collect();
        assert_eq!(numbers, [(1, CH1), (2, CH2)]);
        assert!(toc.chapters.iter().all(|c| c.status == "later"));
    }
}
