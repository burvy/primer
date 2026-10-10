//! Request and response shapes shared by the server and the web app.
//!
//! These are plain data with `serde` (a library that turns Rust structs into
//! JSON and back). Enum-like values travel as lowercase words such as
//! `"pictorial"` or `"carry_or_borrow"`, so the browser needs no Rust types.

use serde::{Deserialize, Serialize};

/// One problem, ready to show: the numbers, where the box sits, and the
/// text the core's `Display` produced (like `3 + 4` or `3 + □ = 7`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProblemDto {
    pub a: u32,
    /// `"add"` or `"sub"`.
    pub op: String,
    pub b: u32,
    /// `"answer"`, `"second"` or `"first"`.
    pub blank: String,
    pub text: String,
}

/// One tap of a lesson: a short sentence and one picture.
///
/// `show` names the picture (`"machine"`, `"machines"`, `"tower"`,
/// `"number-line"`, `"equation"`, `"counters"` or `"pouches"`). The other fields are small
/// settings for that picture; a picture ignores the ones it does not use.
/// `formal` replaces `text` in the Exceptional look. `text` and `formal` are
/// HTML made by the server: the words are escaped and each `$...$` of LaTeX
/// is already MathML, so the browser only has to insert them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeatDto {
    pub text: String,
    pub formal: Option<String>,
    pub show: String,
    pub count: Option<u32>,
    pub adding: Option<u32>,
    pub at: Option<u32>,
    pub hop: Option<u32>,
    pub input: Option<u32>,
    pub output: Option<u32>,
    /// What to light up: a hand-drawn layer of a machine (`"input-glow"`,
    /// `"output-glow"`) or a part of an equation (`"addends"`, `"equals"`,
    /// `"answer"`).
    pub highlight: Option<String>,
    pub label: Option<String>,
    pub open: Option<bool>,
    /// `"pouches"` only: the number of coins in each pouch, and what to do with them.
    pub left: Option<u32>,
    pub right: Option<u32>,
    /// Pour both pouches out onto the table.
    pub join: Option<bool>,
    /// Trade the two pouches' places.
    pub swap: Option<bool>,
    /// Write this total above the pouches.
    pub total: Option<u32>,
    /// A hand-drawn picture to show, by file name under `assets/`.
    pub image: Option<String>,
    /// How Ada reads this beat aloud.
    pub voice: Option<VoiceDto>,
}

/// One line Ada can say: the stable `name` of its recording (see
/// `content/voice.rs` in the server) and the `speech`, the words to read
/// aloud (`3 plus 4 equals 7`, not LaTeX).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineDto {
    pub name: String,
    pub speech: String,
}

/// The line for each look. `exceptional` is only there when the beat has its
/// own `formal` wording; otherwise the Exceptional look reads `gentle`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoiceDto {
    pub gentle: LineDto,
    pub exceptional: Option<LineDto>,
}

/// Which sounds the child wants. Remembered per child on the server, like
/// the look, so the choice follows them to another device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioSettings {
    /// Ada reads every line aloud.
    pub narration: bool,
    /// Page turns, coins, hops and the chime.
    pub sounds: bool,
    /// A quiet tune in the background.
    pub music: bool,
}

impl Default for AudioSettings {
    /// Narration and sounds on, music off.
    fn default() -> Self {
        AudioSettings {
            narration: true,
            sounds: true,
            music: false,
        }
    }
}

/// What the child sees next. `#[serde(tag = "type")]` writes the variant name
/// into the JSON as `"type": "practice"`, so the browser can tell them apart.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PageDto {
    Lesson {
        skill: String,
        lesson: String,
        /// True when the child is being shown it again after struggling.
        again: bool,
        /// The lesson, one tap at a time.
        beats: Vec<BeatDto>,
    },
    Practice {
        /// Send this back with the answer, so a stale page is noticed.
        page_id: String,
        skill: String,
        set_id: String,
        problem: ProblemDto,
        /// `"pictorial"`, `"number_line"` or `"abstract"`.
        stage: String,
        /// The mistake the hint is about, if any (for example `"off_by_one"`).
        hint: Option<String>,
        easier: bool,
    },
    Story,
}

/// `POST /api/child/{id}/answer`. An `answer` of `null` means "I don't know".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnswerRequest {
    pub page_id: String,
    pub answer: Option<u32>,
    pub ms_taken: u32,
}

/// The reply to an answer: was it right, what kind of slip if not, and the
/// next page, so the browser needs only one round trip per problem.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnswerResponse {
    pub correct: bool,
    pub mistake: Option<String>,
    pub next: PageDto,
}

/// `POST /api/child/{id}/lesson-seen`: the child finished reading a lesson.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LessonSeenRequest {
    pub skill: String,
    pub lesson: String,
}

/// `POST /api/child/{id}/look`: the child switched looks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LookRequest {
    /// `"gentle"` or `"exceptional"`.
    pub look: String,
}
