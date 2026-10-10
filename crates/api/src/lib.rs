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
/// `"number-line"`, `"equation"` or `"counters"`). The other fields are small
/// settings for that picture; a picture ignores the ones it does not use.
/// `formal` replaces `text` in the Exceptional look. `$...$` in either marks
/// maths (LaTeX).
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
    /// A hand-drawn layer of the picture to light up, like `"input-glow"`.
    pub highlight: Option<String>,
    pub label: Option<String>,
    pub open: Option<bool>,
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
