//! What the book knows about one child, worked out from what happened.
//!
//! The database never stores "progress". It stores *events*: "saw this
//! lesson", "answered this problem". The `Learner` is computed from them with
//! a *fold*: start from an empty learner, then apply each event in turn, each
//! step taking the old learner and giving back a new one:
//!
//! ```text
//! events.iter().fold(Learner::default(), Learner::apply)
//! ```
//!
//! Because the state is always re-derived, it can never drift away from the
//! history, and replaying the same events always gives the same learner.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::explain::{AskRules, Reflection};
use crate::look::Look;
use crate::mistake::MistakeKind;
use crate::policy::story_id;
use crate::problem::Problem;
use crate::skill::{Skill, Stage};

/// How many recent attempts we remember, to avoid repeating a problem.
pub const RECENT: usize = 20;
/// An answer slower than this (in milliseconds) counts as a struggle.
pub const SLOW_MS: u32 = 30_000;
/// Right answers in a row needed to leave a stage after pictorial.
pub const WINS_NEEDED: u32 = 3;

/// How the child said they worked it out (read from their explanation).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strategy {
    /// Counted every object from one.
    CountAll,
    /// Started at the bigger number and counted up.
    CountOn,
    /// Went to ten first, then the rest.
    MakeTen,
    /// Used a double they know, like 6 + 6.
    Doubles,
    /// Just knew it.
    Recall,
    /// Could not tell.
    Unknown,
}

/// What happened when the child answered. `Wrong` carries the kind of
/// mistake, so the policy can show the matching hint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Correct,
    Wrong(MistakeKind),
    /// The child tapped "I don't know".
    DontKnow,
}

/// One answered problem, exactly as it was shown and answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attempt {
    pub skill: &'static str,
    pub set_id: String,
    pub problem: Problem,
    pub stage: Stage,
    pub look: Look,
    pub answer: Option<u32>,
    pub outcome: Outcome,
    pub ms_taken: u32,
    pub strategy: Strategy,
    /// Frustration: a "stuck" or "tired" feeling, or the judge's reading of
    /// the child's explanation.
    pub frustrated: bool,
    /// What the child tapped after answering (choice ids only).
    pub reflection: Reflection,
}

/// Something that happened. The whole history is a list of these.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    LessonSeen { skill: &'static str, lesson: String },
    Attempt(Attempt),
}

/// Where the child is within one skill.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Progress {
    /// Index into the skill's `stages`: 0 is the first stage shown.
    pub stage: usize,
    pub streak: u32,
    pub wrong_in_row: u32,
    /// Times stepped back since the lesson was last seen.
    pub step_backs: u32,
    /// The mistake that caused the last step back, until the next right answer.
    pub hint: Option<MistakeKind>,
    pub mastered: bool,
}

/// Everything the policy needs to know about one child.
///
/// `BTreeMap` and `BTreeSet` keep their keys sorted, so two learners built
/// from the same events are equal, field for field.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Learner {
    pub lessons_seen: BTreeSet<(&'static str, String)>,
    pub skills: BTreeMap<&'static str, Progress>,
    /// For each skill, the ids of the sets started, in order.
    pub sets_started: BTreeMap<&'static str, Vec<String>>,
    /// How many problems of each set have been answered.
    pub answered: BTreeMap<String, usize>,
    /// How many of them were right.
    pub right: BTreeMap<String, usize>,
    /// The last `RECENT` problems answered, newest at the back.
    pub recent: VecDeque<Problem>,
    /// `recent` as it was when the current set started. A generated set is
    /// rebuilt from this, so it stays the same set while the child works.
    pub recent_at_set_start: Vec<Problem>,
    /// The set of the latest answer.
    pub current_set: Option<String>,
    /// The last answer was slow, "I don't know", or frustrated.
    pub struggling: bool,
    /// The last answer was slower than `SLOW_MS`.
    pub last_slow: bool,
    pub strategy: Option<Strategy>,
}

impl Learner {
    /// Replays a whole history: the fold described at the top of this file.
    pub fn from_events(events: &[Event]) -> Learner {
        events.iter().fold(Learner::default(), Learner::apply)
    }

    /// Progress in one skill; a skill never practised has default progress.
    pub fn progress(&self, skill: &str) -> Progress {
        self.skills.get(skill).cloned().unwrap_or_default()
    }

    pub fn mastered(&self, skill: &str) -> bool {
        self.progress(skill).mastered
    }

    /// What decides whether Ada asks after the next answer on a page of
    /// `set_id` (see [`crate::explain::asks`]).
    pub fn ask_rules(&self, set_id: &str) -> AskRules {
        AskRules {
            first_right_in_set: self.right.get(set_id).copied().unwrap_or(0) == 0,
            slow_before: self.last_slow,
        }
    }

    /// One step of the fold: the learner after `event`.
    ///
    /// `apply` takes `self` by value (not `&self`): it *owns* the old learner,
    /// changes it, and hands it back. No one else can see the half-changed
    /// value, so this is as safe as building a fresh copy, without the copy.
    /// The `match` must handle every kind of event; adding a new kind later
    /// makes the compiler point here.
    pub fn apply(mut self, event: &Event) -> Learner {
        match event {
            Event::LessonSeen { skill, lesson } => {
                self.lessons_seen.insert((*skill, lesson.clone()));
                // Seeing the lesson again is the fresh start after struggling.
                self.skills.entry(*skill).or_default().step_backs = 0;
            }
            Event::Attempt(attempt) => self = self.attempt(attempt),
        }
        self
    }

    fn attempt(mut self, at: &Attempt) -> Learner {
        // A story is told in the middle of whatever set is running, so it
        // must not count as starting a set: that would lose the place of a
        // generated set (see `policy::generated`).
        if story_id(&at.set_id).is_none() {
            let started = self.sets_started.entry(at.skill).or_default();
            if !started.contains(&at.set_id) {
                started.push(at.set_id.clone());
                self.recent_at_set_start = self.recent.iter().copied().collect();
            }
            self.current_set = Some(at.set_id.clone());
        }
        *self.answered.entry(at.set_id.clone()).or_default() += 1;
        if at.outcome == Outcome::Correct {
            *self.right.entry(at.set_id.clone()).or_default() += 1;
        }
        self.last_slow = at.ms_taken > SLOW_MS;
        self.recent.push_back(at.problem);
        if self.recent.len() > RECENT {
            self.recent.pop_front();
        }
        self.struggling = at.outcome == Outcome::DontKnow || at.ms_taken > SLOW_MS || at.frustrated;
        self.strategy = Some(at.strategy);
        let stages = Skill::find(at.skill).map_or(1, |s| s.stages.len());
        let progress = self.skills.entry(at.skill).or_default();
        *progress = step(progress.clone(), at, stages);
        self
    }
}

/// The DESIGN.md rules for one answer, as a function from old progress to new.
///
/// Right answers at the current stage build a streak; enough in a row moves
/// up a stage, or masters the skill from the last stage. Two wrong in a row
/// step back a stage and remember the mistake for a hint.
fn step(mut p: Progress, at: &Attempt, stages: usize) -> Progress {
    let at_current = stage_index(at) == Some(p.stage);
    match at.outcome {
        Outcome::Correct if at_current => {
            p.streak += 1;
            p.wrong_in_row = 0;
            p.hint = None;
            let needed = match at.stage {
                Stage::Pictorial => at.look.pictorial_wins_needed(),
                _ => WINS_NEEDED,
            };
            if p.streak >= needed {
                p.streak = 0;
                if p.stage + 1 < stages {
                    p.stage += 1;
                } else {
                    p.mastered = true;
                }
            }
        }
        // A right answer on an easier page is kind words, not progress.
        Outcome::Correct => p.wrong_in_row = 0,
        Outcome::Wrong(kind) => {
            p.streak = 0;
            p.wrong_in_row += 1;
            if p.wrong_in_row >= 2 {
                p.wrong_in_row = 0;
                p.stage = p.stage.saturating_sub(1);
                p.step_backs += 1;
                p.hint = Some(kind);
            }
        }
        Outcome::DontKnow => p.streak = 0,
    }
    p
}

/// Which position the attempt's stage has in its skill's list of stages.
fn stage_index(at: &Attempt) -> Option<usize> {
    Skill::find(at.skill)?
        .stages
        .iter()
        .position(|&s| s == at.stage)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::problem::Op;

    const CH1: &str = "count-on-within-5";
    const SET: &str = "ch1-a";

    /// One answered problem: `a + 1` in `set_id`, shown at `stage`.
    fn attempt(a: u32, set_id: &str, stage: Stage, outcome: Outcome) -> Event {
        Event::Attempt(Attempt {
            skill: CH1,
            set_id: set_id.into(),
            problem: Problem::new(a, Op::Add, 1).unwrap(),
            stage,
            look: Look::Gentle,
            answer: Some(a + 1),
            outcome,
            ms_taken: 4_000,
            strategy: Strategy::CountOn,
            frustrated: false,
            reflection: Reflection::default(),
        })
    }

    /// `n` right answers at `stage`, one after the other.
    fn rights(stage: Stage, n: u32) -> Vec<Event> {
        (0..n)
            .map(|i| attempt(2 + i, SET, stage, Outcome::Correct))
            .collect()
    }

    /// The attempt above, taking `SLOW_MS + 1` milliseconds.
    fn slow_attempt() -> Event {
        let mut event = attempt(2, SET, Stage::Pictorial, Outcome::Correct);
        let Event::Attempt(at) = &mut event else {
            unreachable!()
        };
        at.ms_taken = SLOW_MS + 1;
        event
    }

    #[test]
    fn an_empty_history_gives_the_default_learner() {
        let learner = Learner::from_events(&[]);
        assert_eq!(learner, Learner::default());
        // A skill never practised has default progress, and is not mastered.
        assert_eq!(learner.progress(CH1), Progress::default());
        assert_eq!(learner.progress("make-ten"), Progress::default());
        assert!(!learner.mastered(CH1));
        assert_eq!(learner.current_set, None);
        assert_eq!(learner.strategy, None);
        assert!(learner.recent.is_empty());
        assert!(!learner.struggling && !learner.last_slow);
    }
    #[test]
    fn one_right_answer_starts_a_streak_and_the_set() {
        let learner = Learner::from_events(&rights(Stage::Pictorial, 1));
        let progress = learner.progress(CH1);
        assert_eq!((progress.stage, progress.streak), (0, 1));
        assert_eq!(progress.wrong_in_row, 0);
        assert!(!progress.mastered);
        assert_eq!(learner.answered.get(SET), Some(&1));
        assert_eq!(learner.right.get(SET), Some(&1));
        assert_eq!(learner.current_set.as_deref(), Some(SET));
        assert_eq!(learner.sets_started[CH1], vec![SET.to_string()]);
        assert_eq!(learner.recent.len(), 1);
        assert!(learner.recent_at_set_start.is_empty());
        assert_eq!(learner.strategy, Some(Strategy::CountOn));
        assert!(!learner.struggling && !learner.last_slow);
    }

    #[test]
    fn a_wrong_answer_clears_the_streak_but_keeps_the_stage() {
        let learner = Learner::from_events(&[
            attempt(2, SET, Stage::Pictorial, Outcome::Correct),
            attempt(
                3,
                SET,
                Stage::Pictorial,
                Outcome::Wrong(MistakeKind::OffByOne),
            ),
        ]);
        let progress = learner.progress(CH1);
        assert_eq!(
            (progress.stage, progress.streak, progress.wrong_in_row),
            (0, 0, 1)
        );
        // One wrong in a row is not yet a step back, so there is no hint.
        assert_eq!(progress.hint, None);
        assert_eq!(progress.step_backs, 0);
        assert_eq!(learner.answered.get(SET), Some(&2));
        assert_eq!(learner.right.get(SET), Some(&1));
    }

    #[test]
    fn a_right_answer_after_a_wrong_one_is_progress_again() {
        let learner = Learner::from_events(&[
            attempt(
                2,
                SET,
                Stage::Pictorial,
                Outcome::Wrong(MistakeKind::OffByOne),
            ),
            attempt(3, SET, Stage::Pictorial, Outcome::Correct),
        ]);
        let progress = learner.progress(CH1);
        assert_eq!(
            (progress.stage, progress.streak, progress.wrong_in_row),
            (0, 1, 0)
        );
        assert_eq!(progress.hint, None);
        assert_eq!(progress.step_backs, 0);
    }

    #[test]
    fn two_wrong_answers_step_back_with_the_mistake_as_a_hint() {
        let mut events = rights(Stage::Pictorial, 3);
        assert_eq!(Learner::from_events(&events).progress(CH1).stage, 1);
        events.push(attempt(
            2,
            SET,
            Stage::NumberLine,
            Outcome::Wrong(MistakeKind::OffByOne),
        ));
        assert_eq!(Learner::from_events(&events).progress(CH1).stage, 1);
        events.push(attempt(
            3,
            SET,
            Stage::NumberLine,
            Outcome::Wrong(MistakeKind::DigitsSwapped),
        ));
        let progress = Learner::from_events(&events).progress(CH1);
        assert_eq!(progress.stage, 0);
        assert_eq!(progress.step_backs, 1);
        // The hint remembers the mistake that caused the step back.
        assert_eq!(progress.hint, Some(MistakeKind::DigitsSwapped));
        assert_eq!(progress.wrong_in_row, 0);
    }

    #[test]
    fn a_right_answer_clears_the_hint_left_by_a_step_back() {
        let mut events = rights(Stage::Pictorial, 3);
        events.push(attempt(
            2,
            SET,
            Stage::NumberLine,
            Outcome::Wrong(MistakeKind::Guess),
        ));
        events.push(attempt(
            3,
            SET,
            Stage::NumberLine,
            Outcome::Wrong(MistakeKind::Guess),
        ));
        events.push(attempt(4, SET, Stage::Pictorial, Outcome::Correct));
        let progress = Learner::from_events(&events).progress(CH1);
        assert_eq!(progress.hint, None);
        // The step back itself stays counted until the lesson is seen again.
        assert_eq!(progress.step_backs, 1);
        assert_eq!((progress.stage, progress.streak), (0, 1));
    }

    #[test]
    fn three_right_answers_move_up_a_stage() {
        let learner = Learner::from_events(&rights(Stage::Pictorial, 3));
        let progress = learner.progress(CH1);
        assert_eq!((progress.stage, progress.streak), (1, 0));
        assert!(!progress.mastered);
    }

    #[test]
    fn nine_right_answers_master_the_skill() {
        let mut events = rights(Stage::Pictorial, 3);
        events.extend(rights(Stage::NumberLine, 3));
        events.extend(rights(Stage::Abstract, 3));
        let learner = Learner::from_events(&events);
        assert!(learner.mastered(CH1));
        assert_eq!(learner.progress(CH1).stage, 2);
    }

    #[test]
    fn a_right_answer_on_an_easier_stage_is_not_progress() {
        let mut events = rights(Stage::Pictorial, 3);
        events.push(attempt(5, SET, Stage::Pictorial, Outcome::Correct));
        let progress = Learner::from_events(&events).progress(CH1);
        // Kind words: the stage and the streak both stay as they were.
        assert_eq!((progress.stage, progress.streak), (1, 0));
    }

    #[test]
    fn seeing_the_lesson_again_clears_the_step_backs() {
        let mut events = rights(Stage::Pictorial, 3);
        events.push(attempt(
            2,
            SET,
            Stage::NumberLine,
            Outcome::Wrong(MistakeKind::Guess),
        ));
        events.push(attempt(
            3,
            SET,
            Stage::NumberLine,
            Outcome::Wrong(MistakeKind::Guess),
        ));
        assert_eq!(Learner::from_events(&events).progress(CH1).step_backs, 1);
        events.push(Event::LessonSeen {
            skill: CH1,
            lesson: "intro".into(),
        });
        let learner = Learner::from_events(&events);
        assert_eq!(learner.progress(CH1).step_backs, 0);
        assert!(learner.lessons_seen.contains(&(CH1, "intro".to_string())));
        // Only the step backs reset: the child stays on the easier stage.
        assert_eq!(learner.progress(CH1).stage, 0);
    }

    #[test]
    fn the_recent_list_keeps_only_the_last_twenty() {
        let events: Vec<Event> = (0..RECENT as u32 + 5)
            .map(|a| attempt(a, SET, Stage::Pictorial, Outcome::Correct))
            .collect();
        let learner = Learner::from_events(&events);
        assert_eq!(learner.recent.len(), RECENT);
        // Newest at the back: the first five problems have fallen off.
        assert_eq!(learner.recent.front().unwrap().to_string(), "5 + 1");
        assert_eq!(learner.recent.back().unwrap().to_string(), "24 + 1");
    }

    #[test]
    fn a_story_answer_is_not_a_set_the_child_started() {
        let learner = Learner::from_events(&[
            attempt(2, SET, Stage::Pictorial, Outcome::Correct),
            attempt(3, "story:tower", Stage::Pictorial, Outcome::Correct),
        ]);
        // The story must not steal the place of the set being worked on.
        assert_eq!(learner.current_set.as_deref(), Some(SET));
        assert!(
            !learner.sets_started[CH1]
                .iter()
                .any(|id| story_id(id).is_some())
        );
        assert_eq!(learner.answered.get("story:tower"), Some(&1));
        assert_eq!(learner.right.get("story:tower"), Some(&1));
        // With no set started yet, a story alone leaves none current.
        let alone = Learner::from_events(&[attempt(
            2,
            "story:tower",
            Stage::Pictorial,
            Outcome::Correct,
        )]);
        assert_eq!(alone.current_set, None);
        assert!(alone.sets_started.get(CH1).is_none());
    }

    #[test]
    fn slow_unknown_or_frustrated_answers_count_as_a_struggle() {
        let slow = Learner::from_events(&[slow_attempt()]);
        assert!(slow.struggling && slow.last_slow);

        // "I don't know" costs the streak, but was answered quickly.
        let mut events = rights(Stage::Pictorial, 2);
        events.push(attempt(3, SET, Stage::Pictorial, Outcome::DontKnow));
        let dont_know = Learner::from_events(&events);
        assert!(dont_know.struggling && !dont_know.last_slow);
        assert_eq!(dont_know.progress(CH1).streak, 0);

        let mut tired = attempt(4, SET, Stage::Pictorial, Outcome::Correct);
        let Event::Attempt(at) = &mut tired else {
            unreachable!()
        };
        at.frustrated = true;
        let tired = Learner::from_events(&[tired]);
        assert!(tired.struggling && !tired.last_slow);
    }

    #[test]
    fn ask_rules_follow_the_first_right_answer_and_a_slow_one_before() {
        let fresh = Learner::from_events(&[]);
        assert_eq!(
            fresh.ask_rules(SET),
            AskRules {
                first_right_in_set: true,
                slow_before: false
            }
        );
        let answered = Learner::from_events(&rights(Stage::Pictorial, 1));
        assert_eq!(
            answered.ask_rules(SET),
            AskRules {
                first_right_in_set: false,
                slow_before: false
            }
        );
        let slow = Learner::from_events(&[slow_attempt()]);
        assert_eq!(
            slow.ask_rules(SET),
            AskRules {
                first_right_in_set: false,
                slow_before: true
            }
        );
    }
}
