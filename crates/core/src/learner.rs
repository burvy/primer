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
    /// The frustration signal read from what the child typed.
    pub frustrated: bool,
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
    /// The last `RECENT` problems answered, newest at the back.
    pub recent: VecDeque<Problem>,
    /// `recent` as it was when the current set started. A generated set is
    /// rebuilt from this, so it stays the same set while the child works.
    pub recent_at_set_start: Vec<Problem>,
    /// The set of the latest answer.
    pub current_set: Option<String>,
    /// The last answer was slow, "I don't know", or frustrated.
    pub struggling: bool,
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
