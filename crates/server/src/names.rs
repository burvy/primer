//! Words for the core's enums, used in the database and in JSON.
//!
//! Each table pairs every variant with its word. `name` and `parse` are
//! *generic*: `<T>` stands for "any type", so one pair of functions serves
//! every table. `T: Copy + PartialEq` is a trait bound: it says what `T` must
//! be able to do (be copied, and be compared with `==`) for the code to work.

use primer_core::{Blank, Look, MistakeKind, Op, Stage};

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
