//! What Ada says about an answer: hints, praise and word-problem stories,
//! and the pure rules that choose between them.
//!
//! The book (in `content/*.toml`) holds many hints and praise lines; each
//! answer needs exactly one of them. Every rule here is a plain function of
//! its arguments (the random choice takes the generator as an argument), so
//! each can be tested with a few lines and no server.

use primer_api::NoteDto;
use primer_core::{Look, MistakeKind, Stage};
use rand::{Rng, RngExt};

use super::schema::{LookTag, MistakeTag, StageTag, When};

/// An answer faster than this (in milliseconds) is called "fast".
pub const FAST_MS: u32 = 4_000;
/// This many right answers in a row is a "streak".
pub const STREAK: u32 = 3;

/// Does an entry written for `tag` belong in the child's `look`?
/// (`both` is for every look.)
pub fn in_look(tag: LookTag, look: Look) -> bool {
    matches!(
        (tag, look),
        (LookTag::Both, _)
            | (LookTag::Gentle, Look::Gentle)
            | (LookTag::Exceptional, Look::Exceptional)
    )
}

/// The file's word for a core stage.
pub fn stage_tag(stage: Stage) -> StageTag {
    match stage {
        Stage::Pictorial => StageTag::Pictorial,
        Stage::NumberLine => StageTag::NumberLine,
        Stage::Abstract => StageTag::Abstract,
    }
}

/// The file's word for a core mistake.
pub fn mistake_tag(kind: MistakeKind) -> MistakeTag {
    match kind {
        MistakeKind::OffByOne => MistakeTag::OffByOne,
        MistakeKind::WrongOperation => MistakeTag::WrongOperation,
        MistakeKind::DigitsSwapped => MistakeTag::DigitsSwapped,
        MistakeKind::CarryOrBorrow => MistakeTag::CarryOrBorrow,
        MistakeKind::Guess => MistakeTag::Guess,
        MistakeKind::Unknown => MistakeTag::Unknown,
    }
}

/// A hint for one kind of mistake at one stage, with what Ada says.
#[derive(Debug, Clone)]
pub struct Hint {
    pub look: LookTag,
    pub stage: StageTag,
    pub mistake: MistakeTag,
    pub note: NoteDto,
}

/// A short line for a right answer. `when` is the moment it fits; a line
/// with no `when` fits any right answer.
#[derive(Debug, Clone)]
pub struct Praise {
    pub look: LookTag,
    pub when: Option<When>,
    pub note: NoteDto,
}

/// A word problem: a story for the sum `a + b` at one stage.
#[derive(Debug, Clone)]
pub struct Story {
    pub look: LookTag,
    pub stage: StageTag,
    pub a: u32,
    pub b: u32,
    pub note: NoteDto,
}

/// The hint for this `mistake` at this `stage`: the one written for exactly
/// that pair, else the first hint written for the stage (any mistake), else
/// none. Hints for the other look are never chosen.
pub fn pick_hint(
    hints: &[Hint],
    look: Look,
    stage: StageTag,
    mistake: MistakeTag,
) -> Option<&Hint> {
    let mut here = hints
        .iter()
        .filter(|h| in_look(h.look, look) && h.stage == stage);
    // `find` runs on a copy of the iterator, so `here` is still whole for the
    // fallback.
    here.clone().find(|h| h.mistake == mistake).or(here.next())
}

/// What is true of a right answer, for choosing its praise.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Facts {
    pub ms_taken: u32,
    /// The number added, when the problem is an addition (`3 + 2` adds 2).
    pub adds: Option<u32>,
    /// This answer finished its set.
    pub set_end: bool,
    /// Right answers in a row, counting this one.
    pub streak: u32,
    /// The answer before this one was wrong.
    pub recovered: bool,
}

/// The moments that fit these facts, the most special first.
///
/// Finishing a set beats recovering, which beats a streak, which beats
/// "adds 1 or 2", which beats being fast.
pub fn moments(facts: Facts) -> Vec<When> {
    [
        (facts.set_end, When::SetEnd),
        (facts.recovered, When::Recovered),
        (facts.streak >= STREAK, When::Streak),
        (facts.adds == Some(2), When::Adds2),
        (facts.adds == Some(1), When::Adds1),
        (facts.ms_taken < FAST_MS, When::Fast),
    ]
    .into_iter()
    .filter_map(|(holds, moment)| holds.then_some(moment))
    .collect()
}

/// One line of praise for a right answer.
///
/// The first of `moments` that has any line written for it (in the child's
/// look) wins, and one of its lines is chosen with `rng`. If no moment has a
/// line, one of the lines with no `when` is chosen. `None` only if the book
/// has no praise at all for this look.
pub fn pick_praise<'a>(
    lines: &'a [Praise],
    look: Look,
    moments: &[When],
    rng: &mut impl Rng,
) -> Option<&'a Praise> {
    let for_look = || lines.iter().filter(|p| in_look(p.look, look));
    let fitting = moments
        .iter()
        .map(|moment| {
            for_look()
                .filter(|p| p.when == Some(*moment))
                .collect::<Vec<_>>()
        })
        .find(|found| !found.is_empty());
    let chosen = fitting.unwrap_or_else(|| for_look().filter(|p| p.when.is_none()).collect());
    if chosen.is_empty() {
        return None;
    }
    Some(chosen[rng.random_range(0..chosen.len())])
}

/// The story for the sum `a + b` at `stage`, if the book has one.
pub fn pick_story(
    stories: &[Story],
    look: Look,
    stage: StageTag,
    (a, b): (u32, u32),
) -> Option<&Story> {
    stories
        .iter()
        .find(|s| in_look(s.look, look) && s.stage == stage && (s.a, s.b) == (a, b))
}

/// A word problem's text with `{a}` and `{b}` replaced by its numbers.
pub fn fill_in(text: &str, a: u32, b: u32) -> String {
    text.replace("{a}", &a.to_string())
        .replace("{b}", &b.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn note(text: &str) -> NoteDto {
        NoteDto {
            text: text.into(),
            ..NoteDto::default()
        }
    }

    fn hint(look: LookTag, stage: StageTag, mistake: MistakeTag, text: &str) -> Hint {
        Hint {
            look,
            stage,
            mistake,
            note: note(text),
        }
    }

    fn praise(look: LookTag, when: Option<When>, text: &str) -> Praise {
        Praise {
            look,
            when,
            note: note(text),
        }
    }

    fn text(found: Option<&Hint>) -> Option<&str> {
        found.map(|h| h.note.text.as_str())
    }

    #[test]
    fn a_hint_matches_the_stage_and_the_mistake() {
        let hints = [
            hint(LookTag::Both, StageTag::Pictorial, MistakeTag::OffByOne, "tower"),
            hint(LookTag::Both, StageTag::Abstract, MistakeTag::OffByOne, "abstract"),
            hint(LookTag::Both, StageTag::Abstract, MistakeTag::Guess, "guess"),
        ];
        let pick = |stage, mistake| text(pick_hint(&hints, Look::Gentle, stage, mistake));
        assert_eq!(pick(StageTag::Abstract, MistakeTag::OffByOne), Some("abstract"));
        assert_eq!(pick(StageTag::Pictorial, MistakeTag::OffByOne), Some("tower"));
        assert_eq!(pick(StageTag::Abstract, MistakeTag::Guess), Some("guess"));
    }

    #[test]
    fn with_no_exact_hint_any_hint_for_the_stage_will_do() {
        let hints = [
            hint(LookTag::Both, StageTag::Pictorial, MistakeTag::OffByOne, "tower"),
            hint(LookTag::Both, StageTag::Abstract, MistakeTag::Guess, "guess"),
            hint(LookTag::Both, StageTag::Abstract, MistakeTag::OffByOne, "off"),
        ];
        // Nothing for `digits-swapped`: the first abstract hint is used.
        let found = pick_hint(&hints, Look::Gentle, StageTag::Abstract, MistakeTag::DigitsSwapped);
        assert_eq!(text(found), Some("guess"));
        // Nothing at all for a stage with no hints.
        let none = pick_hint(&hints, Look::Gentle, StageTag::NumberLine, MistakeTag::OffByOne);
        assert!(none.is_none());
    }

    #[test]
    fn a_hint_for_the_other_look_is_never_chosen() {
        let hints = [
            hint(LookTag::Exceptional, StageTag::Abstract, MistakeTag::Guess, "formal"),
            hint(LookTag::Gentle, StageTag::Abstract, MistakeTag::Guess, "gentle"),
        ];
        let found = pick_hint(&hints, Look::Gentle, StageTag::Abstract, MistakeTag::Guess);
        assert_eq!(text(found), Some("gentle"));
        let found = pick_hint(&hints, Look::Exceptional, StageTag::Abstract, MistakeTag::Guess);
        assert_eq!(text(found), Some("formal"));
    }

    #[test]
    fn moments_come_most_special_first() {
        let facts = Facts {
            ms_taken: 1_000,
            adds: Some(1),
            set_end: true,
            streak: 3,
            recovered: true,
        };
        assert_eq!(
            moments(facts),
            [When::SetEnd, When::Recovered, When::Streak, When::Adds1, When::Fast]
        );
        // A slow, plain answer fits no moment.
        let plain = Facts {
            ms_taken: 9_000,
            adds: Some(5),
            ..Facts::default()
        };
        assert!(moments(plain).is_empty());
        let adds_two = Facts {
            adds: Some(2),
            ms_taken: 9_000,
            ..Facts::default()
        };
        assert_eq!(moments(adds_two), [When::Adds2]);
    }

    /// The texts chosen over many seeds, sorted, without repeats.
    fn chosen(lines: &[Praise], look: Look, moments: &[When]) -> Vec<String> {
        let mut seen: Vec<String> = (0..40)
            .filter_map(|seed| {
                let mut rng = StdRng::seed_from_u64(seed);
                pick_praise(lines, look, moments, &mut rng).map(|p| p.note.text.clone())
            })
            .collect();
        seen.sort();
        seen.dedup();
        seen
    }

    #[test]
    fn praise_follows_the_first_moment_with_a_line() {
        let lines = [
            praise(LookTag::Both, Some(When::Fast), "fast-1"),
            praise(LookTag::Both, Some(When::Fast), "fast-2"),
            praise(LookTag::Both, Some(When::Streak), "streak"),
            praise(LookTag::Both, None, "plain"),
        ];
        // Streak comes before fast, so only the streak line is ever chosen.
        assert_eq!(chosen(&lines, Look::Gentle, &[When::Streak, When::Fast]), ["streak"]);
        // No line for set-end, so the next moment is used; two lines vary.
        assert_eq!(
            chosen(&lines, Look::Gentle, &[When::SetEnd, When::Fast]),
            ["fast-1", "fast-2"]
        );
    }

    #[test]
    fn praise_with_no_fitting_moment_is_any_untagged_line() {
        let lines = [
            praise(LookTag::Both, Some(When::Fast), "fast"),
            praise(LookTag::Both, None, "plain-1"),
            praise(LookTag::Both, None, "plain-2"),
        ];
        assert_eq!(chosen(&lines, Look::Gentle, &[]), ["plain-1", "plain-2"]);
        assert_eq!(chosen(&lines, Look::Gentle, &[When::Recovered]), ["plain-1", "plain-2"]);
    }

    #[test]
    fn praise_is_for_the_childs_look_and_may_be_missing() {
        let lines = [
            praise(LookTag::Exceptional, None, "formal"),
            praise(LookTag::Both, Some(When::Fast), "fast"),
        ];
        assert_eq!(chosen(&lines, Look::Gentle, &[]), Vec::<String>::new());
        assert_eq!(chosen(&lines, Look::Exceptional, &[]), ["formal"]);
    }

    #[test]
    fn the_same_seed_gives_the_same_praise() {
        let lines = [
            praise(LookTag::Both, None, "a"),
            praise(LookTag::Both, None, "b"),
            praise(LookTag::Both, None, "c"),
        ];
        let pick = |seed| {
            let mut rng = StdRng::seed_from_u64(seed);
            pick_praise(&lines, Look::Gentle, &[], &mut rng).map(|p| p.note.text.clone())
        };
        assert_eq!(pick(5), pick(5));
    }

    #[test]
    fn a_story_needs_the_same_sum_and_stage() {
        let story = |look, stage, a, b, text: &str| Story {
            look,
            stage,
            a,
            b,
            note: note(text),
        };
        let stories = [
            story(LookTag::Both, StageTag::Pictorial, 3, 1, "stones"),
            story(LookTag::Exceptional, StageTag::Pictorial, 4, 2, "formal only"),
        ];
        let pick = |look, stage, sum| {
            pick_story(&stories, look, stage, sum).map(|s| s.note.text.as_str())
        };
        assert_eq!(pick(Look::Gentle, StageTag::Pictorial, (3, 1)), Some("stones"));
        assert_eq!(pick(Look::Gentle, StageTag::Abstract, (3, 1)), None);
        assert_eq!(pick(Look::Gentle, StageTag::Pictorial, (1, 3)), None);
        assert_eq!(pick(Look::Gentle, StageTag::Pictorial, (4, 2)), None);
        assert_eq!(pick(Look::Exceptional, StageTag::Pictorial, (4, 2)), Some("formal only"));
    }

    #[test]
    fn the_numbers_are_filled_into_a_story() {
        assert_eq!(
            fill_in("{a} stones and {b} more, {a} again", 3, 4),
            "3 stones and 4 more, 3 again"
        );
    }
}
