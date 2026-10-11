//! Words for the core's enums, used in the database and in JSON.
//!
//! Each table pairs every variant with its word. `name` and `parse` are
//! *generic*: `<T>` stands for "any type", so one pair of functions serves
//! every table. `T: Copy + PartialEq` is a trait bound: it says what `T` must
//! be able to do (be copied, and be compared with `==`) for the code to work.

use primer_core::{Blank, Feeling, Look, MistakeKind, Op, Stage, Strategy};

pub const OPS: &[(Op, &str)] = &[(Op::Add, "add"), (Op::Sub, "sub")];

pub const BLANKS: &[(Blank, &str)] = &[
    (Blank::Answer, "answer"),
    (Blank::Second, "second"),
    (Blank::First, "first"),
];

pub const STAGES: &[(Stage, &str)] = &[
    (Stage::Pictorial, "pictorial"),
    (Stage::NumberLine, "number_line"),
    (Stage::Abstract, "abstract"),
];

pub const LOOKS: &[(Look, &str)] = &[(Look::Gentle, "gentle"), (Look::Exceptional, "exceptional")];

pub const MISTAKES: &[(MistakeKind, &str)] = &[
    (MistakeKind::OffByOne, "off_by_one"),
    (MistakeKind::WrongOperation, "wrong_operation"),
    (MistakeKind::DigitsSwapped, "digits_swapped"),
    (MistakeKind::CarryOrBorrow, "carry_or_borrow"),
    (MistakeKind::Guess, "guess"),
    (MistakeKind::Unknown, "unknown"),
];

/// The same words Jev chooses from (see `jev.rs`), so a choice in
/// `explain.toml` can name the strategy it stands for.
pub const STRATEGIES: &[(Strategy, &str)] = &[
    (Strategy::Unknown, "unclear"),
    (Strategy::CountAll, "counted_all"),
    (Strategy::CountOn, "counted_on"),
    (Strategy::MakeTen, "made_ten"),
    (Strategy::Doubles, "doubles"),
    (Strategy::Recall, "just_knew"),
];

/// The four feelings a child can tap; `explain.toml` must have one choice
/// for each, with these ids.
pub const FEELINGS: &[(Feeling, &str)] = &[
    (Feeling::Happy, "happy"),
    (Feeling::Ok, "ok"),
    (Feeling::Stuck, "stuck"),
    (Feeling::Tired, "tired"),
];

/// The age bands a child profile may have.
pub const AGE_BANDS: [&str; 2] = ["5-6", "7-8"];

/// The word for `value`. Every variant is in its table, so the fallback is
/// never used; it only keeps the function total (no `unwrap`, no panic).
pub fn name<T: Copy + PartialEq>(table: &[(T, &'static str)], value: T) -> &'static str {
    table
        .iter()
        .find(|(v, _)| *v == value)
        .map_or("unknown", |(_, word)| word)
}

/// The variant for `word`, or `None` if the word is not in the table.
pub fn parse<T: Copy>(table: &[(T, &str)], word: &str) -> Option<T> {
    table.iter().find(|(_, w)| *w == word).map(|(v, _)| *v)
}
