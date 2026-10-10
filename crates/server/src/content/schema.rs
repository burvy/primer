//! What a content file may contain: plain data types that `serde` fills in
//! from TOML.
//!
//! Every type has `#[serde(deny_unknown_fields)]`, so a typo such as `tex =`
//! for `text =` is an error instead of being silently ignored. Words that
//! must come from a fixed list (`show`, `stage`, `look`...) are `enum`s, so a
//! typo there fails too. `rename_all = "kebab-case"` makes the Rust name
//! `NumberLine` match the file's `number-line`.
//!
//! These types only describe the *file*. The checks live in `check.rs`, and
//! `mod.rs` turns the checked data into what the rest of the server uses.

// Some fields (stage and look of sets and hints, praise `when`, a word's
// glosses) are read by later slices. They are here now so that a file using
// them is accepted and typos next to them are still caught.
#![allow(dead_code)]

use serde::Deserialize;
use toml::Spanned;

/// One chapter file, like `content/02-add.toml`.
///
/// `#[serde(default)]` lets a list be left out of the file (it becomes empty).
///
/// A TOML file keeps `[[lesson]]` and `[[problem_set]]` in two separate
/// lists, so the order they were written in would be lost. `Spanned` wraps a
/// value together with the position in the file where it started; sorting by
/// that position (see `mod.rs`) puts lessons and sets back in the writer's
/// order.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChapterFile {
    /// The skill id this chapter is about, like `add-within-10`.
    pub skill: String,
    pub title: String,
    #[serde(default)]
    pub lesson: Vec<Spanned<Lesson>>,
    #[serde(default)]
    pub problem_set: Vec<Spanned<ProblemSet>>,
    #[serde(default)]
    pub word_problem: Vec<WordProblem>,
    #[serde(default)]
    pub hint: Vec<Hint>,
    #[serde(default)]
    pub praise: Vec<Praise>,
}

/// Which look an entry is for. `both` is the default.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LookTag {
    Gentle,
    Exceptional,
    #[default]
    Both,
}

/// The three stages, as the files spell them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StageTag {
    Pictorial,
    NumberLine,
    Abstract,
}

/// Ada's opening lesson: ordered steps, each with its own beats.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lesson {
    pub id: String,
    #[serde(default)]
    pub look: LookTag,
    pub step: Vec<Step>,
}

/// One stage of a lesson: Ada walks one worked `example` (like `"3 + 4"`)
/// through a few beats.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
    pub stage: StageTag,
    pub example: Option<String>,
    pub beat: Vec<Beat>,
}

/// The pictures a beat can show.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Show {
    Tower,
    Towers,
    Pouches,
    Machine,
    Machines,
    NumberLine,
    Equation,
    Counters,
}

impl Show {
    /// The word the web app looks for (the same word as in the file).
    pub fn word(self) -> &'static str {
        match self {
            Show::Tower => "tower",
            Show::Towers => "towers",
            Show::Pouches => "pouches",
            Show::Machine => "machine",
            Show::Machines => "machines",
            Show::NumberLine => "number-line",
            Show::Equation => "equation",
            Show::Counters => "counters",
        }
    }
}

/// What a beat may light up: a part of the machine, or of the equation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Highlight {
    Input,
    Output,
    Addends,
    Equals,
    Answer,
}

impl Highlight {
    /// The word the web app looks for. The machine's two drawn glows have
    /// longer names than the file uses.
    pub fn word(self) -> &'static str {
        match self {
            Highlight::Input => "input-glow",
            Highlight::Output => "output-glow",
            Highlight::Addends => "addends",
            Highlight::Equals => "equals",
            Highlight::Answer => "answer",
        }
    }
}

/// One tap of a lesson: a sentence (`text`, and `formal` for the Exceptional
/// look) and one picture (`show`) with its small settings. A picture ignores
/// the settings it does not use.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Beat {
    pub text: String,
    pub formal: Option<String>,
    pub show: Show,
    pub count: Option<u32>,
    pub adding: Option<u32>,
    pub at: Option<u32>,
    pub hop: Option<u32>,
    pub input: Option<u32>,
    pub output: Option<u32>,
    pub highlight: Option<Highlight>,
    pub label: Option<String>,
    pub open: Option<bool>,
    pub left: Option<u32>,
    pub right: Option<u32>,
    pub join: Option<bool>,
    pub swap: Option<bool>,
    pub total: Option<u32>,
    /// A hand-drawn picture to show, by file name under `assets/`.
    pub image: Option<String>,
}

/// An ordered run of problems, like `["3 + 4", "2 + 5"]`, and what to say
/// about the pattern at the end.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProblemSet {
    pub id: String,
    pub stage: StageTag,
    #[serde(default)]
    pub look: LookTag,
    /// A pattern id, like `successor` (see `Pattern::id` in the core).
    pub pattern: Option<String>,
    pub problems: Vec<String>,
    /// A question Ada asks before the notice.
    pub question: Option<String>,
    pub notice: Option<String>,
    pub formal_notice: Option<String>,
}

/// A story sum. `{a}` and `{b}` in `text` are filled in from `a` and `b`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WordProblem {
    pub id: String,
    pub stage: StageTag,
    #[serde(default)]
    pub look: LookTag,
    pub text: String,
    pub formal: Option<String>,
    pub a: u32,
    pub b: u32,
}

/// The kinds of slip a hint is for, spelled as in the files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MistakeTag {
    OffByOne,
    WrongOperation,
    DigitsSwapped,
    CarryOrBorrow,
    Guess,
    Unknown,
}

/// A pointer back to the idea, for one kind of mistake at one stage.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hint {
    pub id: String,
    pub stage: StageTag,
    #[serde(default)]
    pub look: LookTag,
    pub mistake: MistakeTag,
    pub text: String,
    pub formal: Option<String>,
}

/// When a praise line fits. A praise with no `when` fits any right answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum When {
    Fast,
    #[serde(rename = "adds-1")]
    Adds1,
    #[serde(rename = "adds-2")]
    Adds2,
    SetEnd,
    Streak,
    Recovered,
}

/// A short line for a right answer.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Praise {
    pub id: String,
    #[serde(default)]
    pub look: LookTag,
    pub text: String,
    pub formal: Option<String>,
    pub when: Option<When>,
}

/// `content/words.toml`: the vocabulary.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WordsFile {
    #[serde(default)]
    pub word: Vec<Word>,
}

/// One word: its id, the skill that teaches it, the plain `gentle` gloss, the
/// grown-up `formal` word, and `ada`'s one-line introduction.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Word {
    pub id: String,
    pub skill: String,
    pub gentle: String,
    pub formal: String,
    pub ada: String,
}
