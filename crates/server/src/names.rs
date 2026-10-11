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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Every pair works both ways: the word for each variant, and the
    /// variant for each word.
    fn round_trips<T: Copy + PartialEq + std::fmt::Debug>(table: &[(T, &'static str)]) {
        for (value, word) in table {
            assert_eq!(name(table, *value), *word);
            assert_eq!(parse(table, word), Some(*value));
        }
    }

    /// No word may stand for two variants: `parse` would only ever find
    /// the first of them.
    fn words_are_unique<T>(table: &[(T, &str)]) {
        let mut seen = HashSet::new();
        for (_, word) in table {
            assert!(seen.insert(*word), "the word `{word}` appears twice");
        }
    }

    #[test]
    fn every_word_and_variant_agree_in_every_table() {
        round_trips(OPS);
        round_trips(BLANKS);
        round_trips(STAGES);
        round_trips(LOOKS);
        round_trips(MISTAKES);
        round_trips(STRATEGIES);
        round_trips(FEELINGS);
    }

    #[test]
    fn no_word_stands_for_two_variants() {
        words_are_unique(OPS);
        words_are_unique(BLANKS);
        words_are_unique(STAGES);
        words_are_unique(LOOKS);
        words_are_unique(MISTAKES);
        words_are_unique(STRATEGIES);
        words_are_unique(FEELINGS);
    }

    #[test]
    fn every_variant_of_every_enum_has_a_word() {
        // The counts are the enums' own, so a new variant with no word in
        // its table fails here.
        assert_eq!(OPS.len(), 2); // Add, Sub
        assert_eq!(BLANKS.len(), 3); // Answer, Second, First
        assert_eq!(STAGES.len(), 3); // Pictorial, NumberLine, Abstract
        assert_eq!(LOOKS.len(), 2); // Gentle, Exceptional
        assert_eq!(MISTAKES.len(), 6);
        assert_eq!(STRATEGIES.len(), 6);
        assert_eq!(FEELINGS.len(), 4); // Happy, Ok, Stuck, Tired
    }

    #[test]
    fn an_unknown_word_is_not_parsed() {
        assert_eq!(parse(OPS, "multiply"), None);
        assert_eq!(parse(STAGES, "Pictures"), None);
        assert_eq!(parse(FEELINGS, "sleepy"), None);
    }

    #[test]
    fn a_variant_left_out_of_a_table_is_named_unknown() {
        // The fallback keeps `name` total: no panic, no empty word.
        let some_table = &[(Op::Add, "add")];
        assert_eq!(name(some_table, Op::Sub), "unknown");
        assert_eq!(parse(some_table, "add"), Some(Op::Add));
    }
}
