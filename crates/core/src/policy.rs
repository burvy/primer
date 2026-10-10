//! The policy: given what we know about a child, what comes next?
//!
//! `next` is a pure function. The same learner, content, look and seed always
//! give the same page, so every rule below can be tested with a plain list of
//! events. The content (lessons, written sets and word problems) is passed in
//! as data; the server loads it from the content files.

use rand::SeedableRng;
use rand::rngs::StdRng;
use std::iter;

use crate::learner::{Learner, Strategy};
use crate::look::Look;
use crate::page::Page;
use crate::problem::{Op, Problem};
use crate::problem_set::{ProblemSet, generate_set};
use crate::skill::{Regroup, SKILLS, Skill, Stage};

/// How many generated sets to try before accepting one with a repeat.
const TRIES: usize = 30;

/// One page of a chapter's written content, in the order the child meets it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// A lesson, by its id in the content file.
    Lesson(String),
    Set(ProblemSet),
    /// A word problem: one sum told as a story, at its own stage.
    Story(Story),
}

/// A written word problem, as the policy sees it: which sum, at which stage.
/// The words of the story stay with the server, found by `id`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Story {
    pub id: String,
    pub problem: Problem,
    pub stage: Stage,
}

/// Story pages use `story:<id>` as their set id, so an answer to a story is
/// never mistaken for an answer in a set.
const STORY_PREFIX: &str = "story:";

/// The set id of a story's page.
pub fn story_set_id(story_id: &str) -> String {
    format!("{STORY_PREFIX}{story_id}")
}

/// The story id inside a story page's set id, or `None` for a set's page.
/// `strip_prefix` gives back the rest of the text only if it starts so.
pub fn story_id(set_id: &str) -> Option<&str> {
    set_id.strip_prefix(STORY_PREFIX)
}

/// A chapter's written content: lessons and sets in order. A chapter may
/// have several lessons, like chapter 1's short "two machines" lesson that
/// comes before its second set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chapter {
    pub skill: &'static str,
    pub steps: Vec<Step>,
}

/// Picks the next page for the child.
///
/// 1. The skill: the first one in the ladder not yet mastered whose
///    `requires` all are. None left means the story page.
/// 2. Two step-backs since the lesson: offer the chapter's first lesson again.
/// 3. The chapter's steps in order: the next unseen lesson, or the next
///    problem of the first unfinished written set.
/// 4. Written sets used up: a generated set (built from `seed`, so the same
///    set comes back on every call until it is finished).
///
/// Word problems wait for their stage. When the child is at a story's stage
/// and has not heard it yet, it comes before the next problem (3 or 4), even
/// in the middle of a set. Every chapter walks through each of its stages,
/// so every story in the child's look is told once.
///
/// `seed` is fixed per child (the server derives it from their id).
pub fn next(learner: &Learner, chapters: &[Chapter], look: Look, seed: u64) -> Page {
    let Some(skill) = SKILLS
        .iter()
        .find(|s| !learner.mastered(s.id) && s.requires.iter().all(|r| learner.mastered(r)))
    else {
        return Page::Story;
    };
    let steps: &[Step] = chapters
        .iter()
        .find(|c| c.skill == skill.id)
        .map_or(&[], |c| &c.steps);
    // `find_map` walks the steps and gives back the first lesson id it finds.
    let first_lesson = steps.iter().find_map(|step| match step {
        Step::Lesson(id) => Some(id),
        _ => None,
    });
    if learner.progress(skill.id).step_backs >= 2
        && let Some(first) = first_lesson
    {
        return lesson(skill, first, true);
    }
    let story = untold_story(learner, skill, steps);
    for step in steps {
        match step {
            Step::Lesson(id) if !learner.lessons_seen.contains(&(skill.id, id.clone())) => {
                return lesson(skill, id, false);
            }
            Step::Set(set) if !finished(learner, set) => {
                return story.map_or_else(
                    || practice(learner, skill, set),
                    |s| tell(learner, skill, s),
                );
            }
            _ => {}
        }
    }
    if let Some(story) = story {
        return tell(learner, skill, story);
    }
    let set = generated(learner, skill, steps, look, seed);
    practice(learner, skill, &set)
}

/// The first story written for the child's current stage that they have not
/// been told yet.
fn untold_story<'a>(learner: &Learner, skill: &Skill, steps: &'a [Step]) -> Option<&'a Story> {
    let stage = skill.stages.get(learner.progress(skill.id).stage)?;
    steps.iter().find_map(|step| match step {
        Step::Story(story)
            if story.stage == *stage && answered(learner, &story_set_id(&story.id)) == 0 =>
        {
            Some(story)
        }
        _ => None,
    })
}

/// A story's page: its one sum, at the stage it was written for.
fn tell(learner: &Learner, skill: &'static Skill, story: &Story) -> Page {
    Page::Practice {
        skill: skill.id,
        set_id: story_set_id(&story.id),
        problem: story.problem,
        stage: story.stage,
        hint: learner.progress(skill.id).hint,
        easier: false,
        nudge: nudge(learner.strategy, skill),
    }
}

fn lesson(skill: &Skill, id: &str, again: bool) -> Page {
    Page::Lesson {
        skill: skill.id,
        lesson: id.to_string(),
        again,
    }
}

/// The next problem of `set`, shown at the child's stage for this skill.
fn practice(learner: &Learner, skill: &'static Skill, set: &ProblemSet) -> Page {
    let progress = learner.progress(skill.id);
    let done = answered(learner, &set.id);
    // After a struggle, show this one page a stage easier.
    let index = match learner.struggling {
        true => progress.stage.saturating_sub(1),
        false => progress.stage,
    };
    let stage = skill.stages.get(index).copied().unwrap_or(Stage::Abstract);
    Page::Practice {
        skill: skill.id,
        set_id: set.id.clone(),
        problem: set.problems[done.min(set.problems.len() - 1)],
        stage,
        hint: progress.hint,
        easier: learner.struggling,
        nudge: nudge(learner.strategy, skill),
    }
}

/// A better strategy to suggest, if the child's explanation showed a slow one.
fn nudge(strategy: Option<Strategy>, skill: &Skill) -> Option<Strategy> {
    let crosses_ten = skill.regroup == Regroup::With;
    match strategy? {
        Strategy::CountAll if skill.op == Op::Add => Some(Strategy::CountOn),
        Strategy::CountAll | Strategy::CountOn if crosses_ten => Some(Strategy::MakeTen),
        _ => None,
    }
}

/// The current generated set, or a new one with no recent repeats.
///
/// Generated sets are numbered per skill. Set `n` is always built from the
/// same seed and the same `avoid` list, so asking again mid-set rebuilds the
/// very same set: nothing about it needs to be stored.
fn generated(
    learner: &Learner,
    skill: &'static Skill,
    steps: &[Step],
    look: Look,
    seed: u64,
) -> ProblemSet {
    let written = |id: &String| {
        steps
            .iter()
            .any(|s| matches!(s, Step::Set(set) if &set.id == id))
    };
    let n = learner
        .sets_started
        .get(skill.id)
        .map_or(0, |ids| ids.iter().filter(|id| !written(id)).count());
    if n > 0 {
        let open = build(skill, look, seed, n - 1, &learner.recent_at_set_start);
        if learner.current_set.as_ref() == Some(&open.id) && !finished(learner, &open) {
            return open;
        }
    }
    let recent: Vec<Problem> = learner.recent.iter().copied().collect();
    build(skill, look, seed, n, &recent)
}

/// Generated set number `n` for `skill`, avoiding the problems in `avoid`.
///
/// `iter::repeat_with` calls its closure again and again, making a fresh set
/// each time; `take` caps the tries and `find` keeps the first set with no
/// problem from `avoid`. If every try repeats something (a tiny chapter),
/// the first set is used anyway.
fn build(skill: &'static Skill, look: Look, seed: u64, n: usize, avoid: &[Problem]) -> ProblemSet {
    let position = Skill::position(skill.id).unwrap_or(0) as u64;
    let mut rng = StdRng::seed_from_u64(seed ^ (position << 32) ^ n as u64);
    let tries: Vec<ProblemSet> = iter::repeat_with(|| generate_set(skill, look, &mut rng))
        .take(TRIES)
        .collect();
    let fresh = |set: &&ProblemSet| set.problems.iter().all(|p| !avoid.contains(p));
    let mut set = tries.iter().find(fresh).unwrap_or(&tries[0]).clone();
    set.id = format!("{}#{n}", set.id);
    set
}

fn answered(learner: &Learner, set_id: &str) -> usize {
    learner.answered.get(set_id).copied().unwrap_or(0)
}

fn finished(learner: &Learner, set: &ProblemSet) -> bool {
    answered(learner, &set.id) >= set.problems.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::learner::{Attempt, Event, Outcome};
    use crate::mistake::MistakeKind;

    const CH1: &str = "count-on-within-5";
    const SEED: u64 = 99;

    fn written(id: &str, sums: &[(u32, u32)]) -> Step {
        let problems = sums
            .iter()
            .map(|&(a, b)| Problem::new(a, Op::Add, b).unwrap())
            .collect();
        Step::Set(ProblemSet {
            id: id.into(),
            skill: CH1,
            problems,
            pattern: None,
            notice: None,
        })
    }

    /// Chapter 1 as the human wrote it: a lesson, a set, the short "two
    /// machines" lesson, a second set.
    fn chapters() -> Vec<Chapter> {
        let steps = vec![
            Step::Lesson("intro".into()),
            written("ch1-a", &[(1, 1), (2, 1)]),
            Step::Lesson("two-machines".into()),
            written("ch1-b", &[(3, 1), (2, 2)]),
        ];
        vec![Chapter { skill: CH1, steps }]
    }

    /// The event a child makes on `page`: reading a lesson, or answering.
    fn respond(page: &Page, look: Look, outcome: Outcome) -> Event {
        match page {
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
            } => Event::Attempt(Attempt {
                skill,
                set_id: set_id.clone(),
                problem: *problem,
                stage: *stage,
                look,
                answer: Some(problem.answer()),
                outcome,
                ms_taken: 4_000,
                strategy: Strategy::CountOn,
                frustrated: false,
            }),
            Page::Story => panic!("no event on the story page"),
        }
    }

    /// Plays from a fresh start until the child has answered `answers`
    /// problems with `outcome`, reading every lesson on the way. Returns the
    /// pages shown and the events made.
    fn play(look: Look, answers: usize, outcome: Outcome) -> (Vec<Page>, Vec<Event>) {
        let mut events = Vec::new();
        let pages = more(&mut events, look, answers, outcome);
        (pages, events)
    }

    /// Like `play`, but carries on from `events`.
    fn more(events: &mut Vec<Event>, look: Look, answers: usize, outcome: Outcome) -> Vec<Page> {
        let mut pages = Vec::new();
        let mut left = answers;
        while left > 0 {
            let page = next(&Learner::from_events(events), &chapters(), look, SEED);
            if matches!(page, Page::Practice { .. }) {
                left -= 1;
            }
            events.push(respond(&page, look, outcome));
            pages.push(page);
        }
        pages
    }

    fn now(events: &[Event]) -> Page {
        next(
            &Learner::from_events(events),
            &chapters(),
            Look::Gentle,
            SEED,
        )
    }

    fn names(pages: &[Page]) -> Vec<String> {
        pages
            .iter()
            .map(|page| match page {
                Page::Lesson { lesson, .. } => format!("lesson {lesson}"),
                Page::Practice {
                    set_id, problem, ..
                } => format!("{set_id}: {problem}"),
                Page::Story => "story".into(),
            })
            .collect()
    }

    #[test]
    fn a_chapter_runs_lesson_set_lesson_set_in_order() {
        // "I don't know" every time makes no progress, so we see it all.
        let (pages, _) = play(Look::Gentle, 4, Outcome::DontKnow);
        let expected = [
            "lesson intro",
            "ch1-a: 1 + 1",
            "ch1-a: 2 + 1",
            "lesson two-machines",
            "ch1-b: 3 + 1",
            "ch1-b: 2 + 2",
        ];
        assert_eq!(names(&pages), expected);
    }

    #[test]
    fn a_new_skill_always_starts_with_its_lesson() {
        for look in [Look::Gentle, Look::Exceptional] {
            let page = next(&Learner::default(), &chapters(), look, SEED);
            assert!(matches!(page, Page::Lesson { lesson, again: false, .. } if lesson == "intro"));
        }
    }

    #[test]
    fn written_sets_come_before_generated_ones() {
        let (pages, _) = play(Look::Gentle, 12, Outcome::DontKnow);
        let ids: Vec<&String> = pages
            .iter()
            .filter_map(|page| match page {
                Page::Practice { set_id, .. } => Some(set_id),
                _ => None,
            })
            .collect();
        let first = ids
            .iter()
            .position(|id| id.contains('#'))
            .expect("a generated set");
        assert!(ids[..first].iter().all(|id| id.starts_with("ch1-")));
        assert!(ids[first..].iter().all(|id| id.contains('#')));
    }

    #[test]
    fn a_generated_set_stays_the_same_until_finished() {
        let (pages, _) = play(Look::Gentle, 12, Outcome::DontKnow);
        let generated: Vec<&String> = pages
            .iter()
            .filter_map(|page| match page {
                Page::Practice { set_id, .. } if set_id.contains('#') => Some(set_id),
                _ => None,
            })
            .collect();
        // The first generated set fills several pages in a row, one id.
        assert!(generated.len() >= 3 && generated[..3].iter().all(|id| *id == generated[0]));
    }

    #[test]
    fn no_new_set_repeats_a_problem_from_the_last_20() {
        // Master chapter 1, then answer "I don't know" in chapter 2 (which
        // has many patterned runs) so that generated sets keep coming.
        // Exceptional masters a skill in 1 + 3 + 3 right answers.
        let (_, mut events) = play(Look::Exceptional, 7, Outcome::Correct);
        assert!(matches!(
            now(&events),
            Page::Practice {
                skill: "add-within-10",
                ..
            }
        ));
        let mut new_sets = 0;
        for _ in 0..80 {
            let learner = Learner::from_events(&events);
            let page = next(&learner, &chapters(), Look::Gentle, SEED);
            if let Page::Practice {
                set_id, problem, ..
            } = &page
                && learner.current_set.as_ref() != Some(set_id)
            {
                new_sets += 1;
                assert!(
                    !learner.recent.contains(problem),
                    "{set_id} repeats {problem}"
                );
            }
            events.push(respond(&page, Look::Gentle, Outcome::DontKnow));
        }
        assert!(new_sets >= 5);
    }

    #[test]
    fn three_right_move_up_a_stage_but_exceptional_needs_one_on_pictures() {
        let stage_after = |look, rights: usize| {
            let (_, events) = play(look, rights, Outcome::Correct);
            Learner::from_events(&events).progress(CH1).stage
        };
        assert_eq!(stage_after(Look::Gentle, 2), 0);
        assert_eq!(stage_after(Look::Gentle, 3), 1);
        assert_eq!(stage_after(Look::Exceptional, 1), 1);
    }

    #[test]
    fn mastering_a_skill_unlocks_the_next_one() {
        // Gentle needs 3 right on each of the 3 stages.
        let (_, events) = play(Look::Gentle, 8, Outcome::Correct);
        assert!(!Learner::from_events(&events).mastered(CH1));
        let (_, events) = play(Look::Gentle, 9, Outcome::Correct);
        assert!(Learner::from_events(&events).mastered(CH1));
        // Chapter 2 has no written content here, so it starts with a set.
        assert!(matches!(
            now(&events),
            Page::Practice {
                skill: "add-within-10",
                ..
            }
        ));
    }

    #[test]
    fn two_wrong_step_back_with_a_hint_and_two_step_backs_bring_the_lesson() {
        let wrong = Outcome::Wrong(MistakeKind::OffByOne);
        // 3 right reaches the number line; then two wrong.
        let (_, mut events) = play(Look::Gentle, 3, Outcome::Correct);
        more(&mut events, Look::Gentle, 2, wrong);
        assert_eq!(Learner::from_events(&events).progress(CH1).stage, 0);
        assert!(matches!(
            now(&events),
            Page::Practice {
                hint: Some(MistakeKind::OffByOne),
                ..
            }
        ));
        // Two more wrong: the second step back offers the lesson again.
        more(&mut events, Look::Gentle, 2, wrong);
        let page = now(&events);
        assert!(matches!(&page, Page::Lesson { lesson, again: true, .. } if lesson == "intro"));
        // Once it is seen, practice resumes.
        more(&mut events, Look::Gentle, 1, wrong);
        assert!(matches!(now(&events), Page::Practice { .. }));
    }

    #[test]
    fn a_struggle_makes_the_next_page_easier() {
        let (_, mut events) = play(Look::Gentle, 3, Outcome::Correct);
        assert!(matches!(
            now(&events),
            Page::Practice {
                stage: Stage::NumberLine,
                easier: false,
                ..
            }
        ));
        more(&mut events, Look::Gentle, 1, Outcome::DontKnow);
        assert!(matches!(
            now(&events),
            Page::Practice {
                stage: Stage::Pictorial,
                easier: true,
                ..
            }
        ));
    }

    #[test]
    fn counting_all_gets_a_count_on_nudge() {
        let mut learner = Learner::from_events(&play(Look::Gentle, 1, Outcome::DontKnow).1);
        learner.strategy = Some(Strategy::CountAll);
        let page = next(&learner, &chapters(), Look::Gentle, SEED);
        assert!(matches!(
            page,
            Page::Practice {
                nudge: Some(Strategy::CountOn),
                ..
            }
        ));
    }

    #[test]
    fn folding_twice_gives_equal_learners_and_pages() {
        let (_, events) = play(Look::Exceptional, 50, Outcome::Correct);
        let (one, two) = (Learner::from_events(&events), Learner::from_events(&events));
        assert_eq!(one, two);
        let page = |learner| next(learner, &chapters(), Look::Gentle, SEED);
        assert_eq!(page(&one), page(&two));
    }

    /// Chapter 1 plus a word problem written for the number-line stage.
    fn with_story() -> Vec<Chapter> {
        let mut chapters = chapters();
        let story = Story {
            id: "tower".into(),
            problem: Problem::new(2, Op::Add, 1).unwrap(),
            stage: Stage::NumberLine,
        };
        chapters[0].steps.push(Step::Story(story));
        chapters
    }

    /// Plays with `chapters` until `answers` problems are answered right,
    /// returning every page shown.
    fn walk(chapters: &[Chapter], answers: usize) -> Vec<Page> {
        let (mut events, mut pages) = (Vec::new(), Vec::new());
        while pages
            .iter()
            .filter(|p| matches!(p, Page::Practice { .. }))
            .count()
            < answers
        {
            let page = next(&Learner::from_events(&events), chapters, Look::Gentle, SEED);
            events.push(respond(&page, Look::Gentle, Outcome::Correct));
            pages.push(page);
        }
        pages
    }

    #[test]
    fn a_story_waits_for_its_stage_then_comes_once_in_the_middle_of_a_set() {
        let pages = walk(&with_story(), 9);
        let told: Vec<usize> = (0..pages.len())
            .filter(|&i| matches!(&pages[i], Page::Practice { set_id, .. } if story_id(set_id) == Some("tower")))
            .collect();
        assert_eq!(told.len(), 1, "told exactly once");
        let Page::Practice { stage, problem, .. } = &pages[told[0]] else {
            unreachable!()
        };
        assert_eq!(
            (*stage, problem.to_string().as_str()),
            (Stage::NumberLine, "2 + 1")
        );
        // Three right answers at the pictures stage come first (Gentle).
        let before = pages[..told[0]]
            .iter()
            .filter(|p| matches!(p, Page::Practice { .. }))
            .count();
        assert_eq!(before, 3);
    }

    #[test]
    fn a_story_does_not_count_as_a_set() {
        let pages = walk(&with_story(), 9);
        let events: Vec<Event> = pages
            .iter()
            .map(|p| respond(p, Look::Gentle, Outcome::Correct))
            .collect();
        let learner = Learner::from_events(&events);
        assert!(
            !learner.sets_started[CH1]
                .iter()
                .any(|id| story_id(id).is_some())
        );
        assert_eq!(learner.answered.get(&story_set_id("tower")), Some(&1));
    }
}
