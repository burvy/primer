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

/// A short line from the book that Ada says: a hint, a word of praise, a
/// question or notice at the end of a set, or the story of a word problem.
/// Like a beat, `text` and `formal` are HTML made by the server (the
/// Exceptional look uses `formal` when there is one), and `voice` says how
/// to read it aloud.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteDto {
    pub text: String,
    pub formal: Option<String>,
    pub voice: VoiceDto,
}

/// What Ada says when a set of problems is finished: first the `question`
/// (the child taps "I see it"), then the `notice`. Either may be missing.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetEndDto {
    pub question: Option<NoteDto>,
    pub notice: Option<NoteDto>,
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

/// The page that opens a chapter: shown before the chapter's first lesson.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenerDto {
    /// Which chapter of the book this is, counting from 1.
    pub number: u32,
    /// The chapter's title, as the writer gave it.
    pub title: String,
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
        /// Set when this is a chapter's first lesson: the page shown first.
        opener: Option<OpenerDto>,
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
        /// The story to tell instead of the bare sum, when the book has a
        /// word problem for this sum at this stage. (Boxed so a problem page
        /// is not much bigger than the other pages.)
        story: Option<Box<NoteDto>>,
        /// What Ada may ask after the answer, and when. `None` when the
        /// book has no `explain.toml` yet. (Boxed, like `story`.)
        ask: Option<Box<AskDto>>,
    },
    Story,
    /// The written chapters have run out: the end of the book so far.
    EndOfBook,
}

/// The table of contents: every written chapter, with the child's progress.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentsDto {
    pub chapters: Vec<ChapterDto>,
}

/// One chapter of the contents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChapterDto {
    /// Counting from 1, in ladder order (the same number as the opener).
    pub number: u32,
    pub skill: String,
    pub title: String,
    /// `"done"` (mastered), `"current"` (where the child is now) or `"later"`.
    pub status: String,
    /// Its lessons, sets and stories, in the order they are met.
    pub items: Vec<ItemDto>,
}

/// One lesson, set or story in a chapter, and how far the child is with it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemDto {
    /// `"lesson"`, `"set"` or `"story"`.
    pub kind: String,
    /// A lesson's id (to open it again); the id of a set or story otherwise.
    pub id: String,
    /// "Lesson 1", "Practice 2", "Story 1".
    pub label: String,
    /// `"new"`, `"started"` or `"done"`.
    pub state: String,
    /// Problems answered so far, and in all (a lesson counts 1 of 1 once seen).
    pub done: u32,
    pub total: u32,
}

/// `POST /api/child/{id}/practice`: ask for more practice on a skill, or
/// (`null`) go back to the main path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PracticeRequest {
    pub skill: Option<String>,
}

/// The questions Ada may ask after an answer on this page, with the rule's
/// inputs. The page decides with `primer_core::explain::asks` once it knows
/// whether the answer was right, so the answer and the taps go in one call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AskDto {
    /// No right answer yet in this set.
    pub first_right_in_set: bool,
    /// The answer before was slow.
    pub slow_before: bool,
    /// Slower than this (milliseconds) is slow.
    pub slow_ms: u32,
    /// "How did you work it out?"
    pub explain: QuestionDto,
    /// "How are you feeling?"
    pub feel: QuestionDto,
}

/// One question and the pictures to tap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestionDto {
    pub question: NoteDto,
    pub choices: Vec<ChoiceDto>,
}

/// One choice: its id (the only thing stored), a picture, and its words.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChoiceDto {
    pub id: String,
    /// A picture character, such as an emoji.
    pub picture: String,
    pub note: NoteDto,
}

/// `POST /api/child/{id}/answer`. An `answer` of `null` means "I don't know".
///
/// `explanation` and `feeling` are the ids of the choices the child tapped,
/// if Ada asked and they did not skip. Only ids are sent and stored; there
/// is no free text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnswerRequest {
    pub page_id: String,
    pub answer: Option<u32>,
    pub ms_taken: u32,
    #[serde(default)]
    pub explanation: Option<String>,
    /// `"happy"`, `"ok"`, `"stuck"` or `"tired"`.
    #[serde(default)]
    pub feeling: Option<String>,
}

/// The reply to an answer: was it right, what kind of slip if not, and the
/// next page, so the browser needs only one round trip per problem.
///
/// What Ada says about the answer comes with it: a `hint` after a wrong one,
/// `praise` after a right one, and `set_end` when that answer finished a set
/// (shown before `next`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnswerResponse {
    pub correct: bool,
    pub mistake: Option<String>,
    pub hint: Option<NoteDto>,
    pub praise: Option<NoteDto>,
    pub set_end: Option<SetEndDto>,
    /// "Let's rest the pencil": shown first when the child is frustrated
    /// (felt stuck or tired). The next page is then a stage easier.
    pub rest: Option<NoteDto>,
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

/// `GET /api/auth/config`: how the parent may sign in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthConfig {
    /// The Google client id; the sign-in page shows Google's button when set.
    pub google_client_id: Option<String>,
    /// A local sign-in with no Google, for running the prototype. Only on
    /// when there is no client id.
    pub dev_sign_in: bool,
}

/// `POST /api/auth/google`: what Google's button gave the page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoogleSignIn {
    pub credential: String,
}

/// The reply to a sign-in: the secret the page sends with every later call
/// as `Authorization: Bearer <token>`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionDto {
    pub token: String,
}

/// A child's profile. There is no real name anywhere: only the hero name the
/// child chose (none yet for a new profile).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChildDto {
    pub id: String,
    pub hero_name: Option<String>,
    /// `"5-6"` or `"7-8"`.
    pub age_band: String,
    /// `"gentle"` or `"exceptional"`.
    pub look: String,
}

/// `POST /api/children`: a new child profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewChild {
    pub age_band: String,
    pub look: String,
}

/// `POST /api/children/{id}/hero`: the hero name the child picked from the
/// list or typed. The server tells which it was.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeroRequest {
    pub name: String,
}
