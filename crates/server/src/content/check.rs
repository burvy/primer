//! The checks on written content. Every function here is *pure*: it looks
//! only at what it is given and returns what it found. Nothing is read from
//! disk and nothing is printed, so each one is easy to test with a few lines
//! of text.
//!
//! Two kinds of finding:
//! - **errors** ([`ContentError`]): the content is broken, the server refuses
//!   to start. Each says which file and which entry, so the writer can go
//!   straight to it.
//! - **gaps** (plain `String` lines from [`coverage`]): nothing is broken, but
//!   something is not written yet. The server prints them at start-up.

use std::collections::HashSet;
use std::fmt;

use primer_core::{Blank, Op, Problem, SKILLS, Skill};

use super::schema::{
    ChapterFile, HeroNamesFile, MistakeTag, StageTag, Word, WordProblem, WordsFile,
};

/// One thing wrong in one file. `id` is the entry's id, or empty when the
/// whole file is the problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentError {
    pub file: String,
    pub id: String,
    pub message: String,
}

impl ContentError {
    pub fn new(file: &str, id: &str, message: impl Into<String>) -> ContentError {
        ContentError {
            file: file.to_string(),
            id: id.to_string(),
            message: message.into(),
        }
    }
}

/// Lets us print an error with `{}`: `02-add.toml: ch2-swap-set: ...`.
impl fmt::Display for ContentError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.id.is_empty() {
            write!(f, "{}: {}", self.file, self.message)
        } else {
            write!(f, "{}: {}: {}", self.file, self.id, self.message)
        }
    }
}

/// Reads one problem written as text.
///
/// Five shapes are allowed, with a space between every part:
/// `3 + 4`, `3 + 4 = □` (find the result), `3 + □ = 7` (find the second
/// number) and `□ + 4 = 7` (find the first). `-` works like `+`.
///
/// The text is cut at its spaces into *tokens*. A token is either a number
/// or the box `□`, and `match` on the whole list of tokens picks the shape.
pub fn parse_problem(text: &str) -> Result<Problem, String> {
    let bad = || format!("`{text}` is not a problem like `3 + 4` or `3 + □ = 7`");
    let tokens: Vec<&str> = text.split_whitespace().collect();
    let (first, symbol, second, whole) = match tokens.as_slice() {
        [a, op, b] => (*a, *op, *b, "□"),
        [a, op, b, "=", c] => (*a, *op, *b, *c),
        _ => return Err(bad()),
    };
    let op = match symbol {
        "+" => Op::Add,
        "-" | "−" => Op::Sub,
        _ => return Err(bad()),
    };
    // `None` stands for the box.
    let number = |token: &str| -> Result<Option<u32>, String> {
        match token {
            "□" => Ok(None),
            digits => digits.parse().map(Some).map_err(|_| bad()),
        }
    };
    let (a, b, c) = (number(first)?, number(second)?, number(whole)?);
    // Exactly one of the three is the box; the other two are known.
    let (a, b, blank) = match (a, b, c) {
        (Some(a), Some(b), None) => (a, b, Blank::Answer),
        (Some(a), None, Some(c)) => (a, missing_second(op, a, c).ok_or_else(bad)?, Blank::Second),
        (None, Some(b), Some(c)) => (missing_first(op, b, c).ok_or_else(bad)?, b, Blank::First),
        _ => return Err(bad()),
    };
    Problem::with_blank(a, op, b, blank).ok_or_else(|| format!("`{text}` goes below zero"))
}

/// The `b` in `a op b = c`. `checked_sub` is `None` instead of going below zero.
fn missing_second(op: Op, a: u32, c: u32) -> Option<u32> {
    match op {
        Op::Add => c.checked_sub(a),
        Op::Sub => a.checked_sub(c),
    }
}

/// The `a` in `a op b = c`.
fn missing_first(op: Op, b: u32, c: u32) -> Option<u32> {
    match op {
        Op::Add => c.checked_sub(b),
        Op::Sub => Some(c + b),
    }
}

/// Reads a problem and checks that it keeps the skill's promise: numbers in
/// range, the sum within the chapter's limit, and so on (`Skill::allows`).
pub fn problem_for(skill: &Skill, text: &str) -> Result<Problem, String> {
    let problem = parse_problem(text)?;
    if skill.allows(problem.a(), problem.b()) {
        Ok(problem)
    } else {
        Err(format!("`{text}` is outside what {} allows", skill.id))
    }
}

/// Finds the pattern the skill teaches under this id.
///
/// `Pattern` variants can carry data (`BondWalk { total }`), so the id alone
/// cannot build one. Instead we look the id up in the skill's own list.
pub fn pattern_for(skill: &Skill, id: &str) -> Result<primer_core::Pattern, String> {
    skill
        .patterns
        .iter()
        .find(|pattern| pattern.id() == id)
        .copied()
        .ok_or_else(|| format!("pattern `{id}` is not one of {}'s patterns", skill.id))
}

/// A word problem's slots: only `{a}` and `{b}` may appear in braces, and
/// the numbers must keep the skill's promise.
pub fn check_word_problem(skill: &Skill, problem: &WordProblem) -> Vec<String> {
    let mut found = Vec::new();
    for text in std::iter::once(&problem.text).chain(problem.formal.as_ref()) {
        // Remove the two valid slots; any brace left over is a typo.
        let rest = text.replace("{a}", "").replace("{b}", "");
        if rest.contains('{') || rest.contains('}') {
            found.push(format!("only {{a}} and {{b}} may appear in braces: {text}"));
        }
    }
    if !skill.allows(problem.a, problem.b) {
        found.push(format!(
            "{} + {} is outside what {} allows",
            problem.a, problem.b, skill.id
        ));
    }
    found
}

/// Ids must be unique across every file, so the same id can never mean two
/// things. `HashSet::insert` returns `false` when the value was already there.
pub fn duplicate_ids(ids: &[(String, String)]) -> Vec<ContentError> {
    let mut seen = HashSet::new();
    ids.iter()
        .filter(|(_, id)| !seen.insert(id))
        .map(|(file, id)| ContentError::new(file, id, "this id is used twice"))
        .collect()
}

/// The checks on `words.toml` alone: unique ids, and each word belongs to a
/// real skill.
pub fn check_words(file: &str, words: &WordsFile) -> Vec<ContentError> {
    let mut errors = Vec::new();
    let ids: Vec<(String, String)> = words
        .word
        .iter()
        .map(|w| (file.to_string(), w.id.clone()))
        .collect();
    errors.extend(duplicate_ids(&ids));
    for Word { id, skill, .. } in &words.word {
        if Skill::find(skill).is_none() {
            errors.push(ContentError::new(file, id, format!("no skill called `{skill}`")));
        }
    }
    errors
}

/// The checks on `hero_names.toml`: no name may be empty or repeat. The
/// writer's names are always accepted as they stand ("al-Khwarizmi" and
/// "Liu Hui" are fine); the one-word-of-letters rule is for typed names only.
pub fn check_hero_names(file: &str, hero_names: &HeroNamesFile) -> Vec<ContentError> {
    let mut errors = Vec::new();
    let mut seen = HashSet::new();
    for name in &hero_names.names {
        if name.trim().is_empty() {
            errors.push(ContentError::new(file, name, "a name cannot be empty"));
        }
        if !seen.insert(name) {
            errors.push(ContentError::new(file, name, "this name is listed twice"));
        }
    }
    errors
}

/// What is not written yet, one line per gap. Nothing here is an error.
///
/// For every skill on the ladder: is there a chapter file, a lesson, a
/// written set, a hint for each stage and likely mistake, and an entry in
/// `words.toml` for each word it teaches?
pub fn coverage(chapters: &[(String, ChapterFile)], words: Option<&WordsFile>) -> Vec<String> {
    let mut gaps = Vec::new();
    for skill in SKILLS {
        let Some((file, chapter)) = chapters.iter().find(|(_, c)| c.skill == skill.id) else {
            gaps.push(format!("{}: no chapter file yet", skill.id));
            continue;
        };
        if chapter.lesson.is_empty() {
            gaps.push(format!("{file}: no lesson"));
        }
        if chapter.problem_set.is_empty() {
            gaps.push(format!("{file}: no written problem set"));
        }
        // Off-by-one is the common slip, so it wants a hint at every stage.
        // The other kinds want one hint somewhere in the chapter. `any` asks
        // a list "does at least one of you pass this test?".
        let has = |stage: Option<StageTag>, kind: MistakeTag| {
            chapter
                .hint
                .iter()
                .any(|h| h.mistake == kind && stage.is_none_or(|s| h.stage == s))
        };
        let mut missing: Vec<String> = skill
            .stages
            .iter()
            .map(|&stage| stage_tag(stage))
            .filter(|&stage| !has(Some(stage), MistakeTag::OffByOne))
            .map(|stage| format!("{stage:?}/off-by-one"))
            .collect();
        for (kind, name) in [
            (MistakeTag::WrongOperation, "wrong-operation"),
            (MistakeTag::Guess, "guess"),
        ] {
            if !has(None, kind) {
                missing.push(name.to_string());
            }
        }
        if !missing.is_empty() {
            gaps.push(format!("{file}: no hint for {}", missing.join(", ")));
        }
    }
    if let Some(words) = words {
        let known: HashSet<&str> = words.word.iter().map(|w| w.id.as_str()).collect();
        for word in SKILLS.iter().flat_map(|skill| skill.words) {
            if !known.contains(word) {
                gaps.push(format!("words.toml: no entry for `{word}`"));
            }
        }
    }
    gaps
}

/// The core's stage as the file-side tag.
fn stage_tag(stage: primer_core::Stage) -> StageTag {
    match stage {
        primer_core::Stage::Pictorial => StageTag::Pictorial,
        primer_core::Stage::NumberLine => StageTag::NumberLine,
        primer_core::Stage::Abstract => StageTag::Abstract,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_every_shape_of_problem() {
        let shown = |text: &str| parse_problem(text).unwrap().to_string();
        assert_eq!(shown("3 + 4"), "3 + 4");
        assert_eq!(shown("3 + 4 = □"), "3 + 4");
        assert_eq!(shown("3 + □ = 10"), "3 + □ = 10");
        assert_eq!(shown("□ + 4 = 7"), "□ + 4 = 7");
        assert_eq!(shown("9 - □ = 2"), "9 - □ = 2");
        assert_eq!(shown("□ - 3 = 4"), "□ - 3 = 4");
        assert_eq!(parse_problem("3 + □ = 10").unwrap().answer(), 7);
    }

    #[test]
    fn refuses_what_is_not_a_problem() {
        for text in ["", "3 +", "3 * 4", "3 + x", "3 + 4 = 7", "□ + □ = 4", "5 + □ = 3", "2 - 5"] {
            assert!(parse_problem(text).is_err(), "{text:?} should not parse");
        }
    }

    #[test]
    fn sums_must_stay_within_the_skill() {
        let skill = Skill::find("count-on-within-5").unwrap();
        assert!(problem_for(skill, "3 + 2").is_ok());
        let error = problem_for(skill, "3 + 4").unwrap_err();
        assert!(error.contains("outside what count-on-within-5 allows"), "{error}");
    }

    #[test]
    fn a_pattern_must_belong_to_the_skill() {
        let skill = Skill::find("add-within-10").unwrap();
        assert!(pattern_for(skill, "swap").is_ok());
        assert!(pattern_for(skill, "bridge").is_err());
    }

    #[test]
    fn hero_names_may_be_anything_but_empty_or_repeated() {
        let file = |names: &[&str]| HeroNamesFile {
            names: names.iter().map(|n| n.to_string()).collect(),
        };
        let fine = file(&["Lovelace", "al-Khwarizmi", "Liu Hui", "Abcdefghijklmnop"]);
        assert!(check_hero_names("h.toml", &fine).is_empty());
        let errors = check_hero_names("h.toml", &file(&["Noether", "Noether", " "]));
        let said: Vec<String> = errors.iter().map(ToString::to_string).collect();
        assert_eq!(
            said,
            [
                "h.toml: Noether: this name is listed twice",
                "h.toml:  : a name cannot be empty",
            ]
        );
    }

    #[test]
    fn word_problem_slots_are_checked() {
        let skill = Skill::find("add-within-10").unwrap();
        let problem = |text: &str, a, b| WordProblem {
            id: "w".into(),
            stage: StageTag::Abstract,
            look: Default::default(),
            text: text.into(),
            formal: None,
            a,
            b,
        };
        assert!(check_word_problem(skill, &problem("{a} and {b} more", 3, 4)).is_empty());
        assert_eq!(check_word_problem(skill, &problem("{a} and {c}", 3, 4)).len(), 1);
        assert_eq!(check_word_problem(skill, &problem("{a} and {b}", 9, 8)).len(), 1);
    }

    #[test]
    fn repeated_ids_are_reported_with_their_file() {
        let ids = vec![
            ("a.toml".to_string(), "x".to_string()),
            ("b.toml".to_string(), "y".to_string()),
            ("b.toml".to_string(), "x".to_string()),
        ];
        let errors = duplicate_ids(&ids);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].to_string(), "b.toml: x: this id is used twice");
    }
}
