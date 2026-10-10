//! Every line a child will meet, pulled out of a chapter file, with a
//! readable place name, plus the code-only word-limit check.
//!
//! Pure: a parsed chapter in, a list of entries out. The parsing and the
//! server's own checks are reused from `primer_server::content`, not copied.

use primer_core::{Op, Skill};
use primer_server::content::check::problem_for;
use primer_server::content::schema::{ChapterFile, LookTag};
use serde::Serialize;

/// Lessons stay under this many words per line (writer's guide, rule 6).
pub const LESSON_WORDS: usize = 12;
/// Word problems may be a little longer.
pub const STORY_WORDS: usize = 20;

/// Who a line is written for, which sets the reading level to aim at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Audience {
    /// The plain wording: ages 5 and 6 must follow it.
    Gentle,
    /// The `formal` wording, for the Exceptional look: up to age 8.
    Exceptional,
}

/// What kind of line this is. Kinds that sit next to a sum carry it, so Jev
/// can be asked whether the line gives the answer away.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Kind {
    Lesson,
    /// Ada's question before a set's notice: it must not answer the set.
    Question {
        sums: Vec<String>,
        answers: Vec<u32>,
    },
    Notice,
    /// A story sum, already filled in (`{a}` and `{b}` replaced).
    /// `op` is `"add"` or `"subtract"`: what the chapter teaches.
    WordProblem {
        sum: String,
        answer: u32,
        op: &'static str,
    },
    Hint,
    Praise,
}

/// One line to check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Entry {
    /// Where it is, for the report: `ch2-joining step 1 beat 2 (formal)`.
    pub place: String,
    pub audience: Audience,
    pub text: String,
    #[serde(flatten)]
    pub kind: Kind,
}

/// All lines of one chapter, in file order within each kind.
pub fn entries(chapter: &ChapterFile) -> Vec<Entry> {
    let skill = Skill::find(&chapter.skill);
    let mut out = Vec::new();
    // Pushes the plain text and, if there is one, its formal twin.
    let mut push = |place: String, look: LookTag, text: &str, formal: Option<&str>, kind: Kind| {
        let plain = match look {
            LookTag::Exceptional => Audience::Exceptional,
            _ => Audience::Gentle,
        };
        out.push(Entry {
            place: place.clone(),
            audience: plain,
            text: text.to_string(),
            kind: kind.clone(),
        });
        if let Some(formal) = formal {
            let text = formal.to_string();
            out.push(Entry {
                place: format!("{place} (formal)"),
                audience: Audience::Exceptional,
                text,
                kind,
            });
        }
    };

    for lesson in &chapter.lesson {
        let lesson = lesson.get_ref();
        // `enumerate` counts from 0; people count steps and beats from 1.
        for (s, step) in lesson.step.iter().enumerate() {
            for (b, beat) in step.beat.iter().enumerate() {
                let place = format!("{} step {} beat {}", lesson.id, s + 1, b + 1);
                push(
                    place,
                    lesson.look,
                    &beat.text,
                    beat.formal.as_deref(),
                    Kind::Lesson,
                );
            }
        }
    }
    for set in &chapter.problem_set {
        let set = set.get_ref();
        if let Some(question) = &set.question {
            // Sums that do not parse are already reported by the code checks.
            let parsed: Vec<_> = skill
                .map(|skill| {
                    set.problems
                        .iter()
                        .filter_map(|p| problem_for(skill, p).ok())
                        .collect()
                })
                .unwrap_or_default();
            let kind = Kind::Question {
                sums: set.problems.clone(),
                answers: parsed.iter().map(|p| p.answer()).collect(),
            };
            push(
                format!("{} question", set.id),
                set.look,
                question,
                None,
                kind,
            );
        }
        if let Some(notice) = &set.notice {
            let formal = set.formal_notice.as_deref();
            push(
                format!("{} notice", set.id),
                set.look,
                notice,
                formal,
                Kind::Notice,
            );
        }
    }
    for story in &chapter.word_problem {
        let Some(skill) = skill else { continue };
        let fill = |text: &str| {
            text.replace("{a}", &story.a.to_string())
                .replace("{b}", &story.b.to_string())
        };
        let (symbol, op, answer) = match skill.op {
            Op::Add => ('+', "add", story.a + story.b),
            Op::Sub => ('-', "subtract", story.a.saturating_sub(story.b)),
        };
        let kind = Kind::WordProblem {
            sum: format!("{} {symbol} {}", story.a, story.b),
            answer,
            op,
        };
        let formal = story.formal.as_deref().map(fill);
        push(
            story.id.clone(),
            story.look,
            &fill(&story.text),
            formal.as_deref(),
            kind,
        );
    }
    for hint in &chapter.hint {
        push(
            hint.id.clone(),
            hint.look,
            &hint.text,
            hint.formal.as_deref(),
            Kind::Hint,
        );
    }
    for praise in &chapter.praise {
        push(
            praise.id.clone(),
            praise.look,
            &praise.text,
            praise.formal.as_deref(),
            Kind::Praise,
        );
    }
    out
}

/// The code-only check: too many words for its kind of line.
pub fn word_limit(entry: &Entry) -> Option<String> {
    let limit = match entry.kind {
        Kind::Lesson => LESSON_WORDS,
        Kind::WordProblem { .. } => STORY_WORDS,
        _ => return None,
    };
    let count = word_count(&entry.text);
    (count > limit).then(|| format!("{count} words; keep it under {limit}"))
}

/// Words in a line, counting each `$...$` maths part as one word, since
/// `$3 + 4 = 7$` is read as one idea, not five words.
///
/// `split('$')` cuts the line at every dollar sign: the even pieces are
/// words, the odd pieces are maths. A lone "." or "-" is not a word.
pub fn word_count(text: &str) -> usize {
    text.split('$')
        .enumerate()
        .map(|(i, piece)| match i % 2 {
            0 => piece
                .split_whitespace()
                .filter(|w| w.chars().any(char::is_alphanumeric))
                .count(),
            _ => usize::from(!piece.trim().is_empty()),
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maths_counts_as_one_word() {
        assert_eq!(word_count("We write it as $3 + 4 = 7$."), 5);
        assert_eq!(word_count("Seven coins."), 2);
    }

    #[test]
    fn a_long_lesson_line_is_flagged() {
        let entry = |text: &str| Entry {
            place: "x".into(),
            audience: Audience::Gentle,
            text: text.into(),
            kind: Kind::Lesson,
        };
        assert_eq!(word_limit(&entry("Three blocks make a tower.")), None);
        let long = "one two three four five six seven eight nine ten eleven twelve thirteen";
        assert_eq!(
            word_limit(&entry(long)),
            Some("13 words; keep it under 12".into())
        );
    }
}
