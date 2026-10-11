//! The child explains: after some answers Ada asks "How did you work it
//! out?", and after a hard moment "How are you feeling?". The child answers
//! by tapping a picture, never by typing.
//!
//! This file decides *when* Ada asks. It is pure, and it runs in the browser
//! (the page asks before it sends the answer, so the answer and what the
//! child tapped arrive together) and in the tests.
//!
//! What is kept is only the id of the choice tapped, such as `"counted_on"`
//! or `"tired"`: never anything the child typed or said.

/// How the child said they feel. The four choices are fixed, because the
/// book reacts to two of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Feeling {
    Happy,
    Ok,
    Stuck,
    Tired,
}

impl Feeling {
    /// Every feeling, in the order the choices are shown.
    pub const ALL: [Feeling; 4] = [Feeling::Happy, Feeling::Ok, Feeling::Stuck, Feeling::Tired];

    /// "Stuck" and "tired" count as frustration straight away: the child
    /// has told us, so there is nothing to judge.
    pub fn frustrated(self) -> bool {
        matches!(self, Feeling::Stuck | Feeling::Tired)
    }
}

/// What the child tapped after one answer. Both are optional: every
/// question can be skipped.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Reflection {
    /// The id of the "How did you work it out?" choice, from `explain.toml`.
    pub explanation: Option<String>,
    pub feeling: Option<Feeling>,
}

/// What the learner's history says about asking on the next answer. The
/// server works it out (see [`Learner::ask_rules`](crate::Learner::ask_rules))
/// and sends it with the page.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AskRules {
    /// No right answer yet in this page's set: a right answer now is the
    /// set's first, and Ada asks how it was done.
    pub first_right_in_set: bool,
    /// The answer before this one was slow.
    pub slow_before: bool,
}

/// Which questions Ada asks after one answer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Asks {
    pub explain: bool,
    pub feel: bool,
}

/// The rule for when Ada asks, from the answer just given.
///
/// `right` is `Some(true)` for a right answer, `Some(false)` for a wrong
/// one, and `None` for "I don't know".
///
/// - "How did you work it out?" after a wrong answer, and after the first
///   right answer of each set. Not after "I don't know": there is nothing
///   to explain.
/// - "How are you feeling?" after a wrong answer or "I don't know", and
///   after the second slow answer in a row (slower than `slow_ms`).
///
/// Asking only sometimes keeps the questions from getting in the way.
pub fn asks(rules: AskRules, right: Option<bool>, ms_taken: u32, slow_ms: u32) -> Asks {
    let slow_streak = ms_taken > slow_ms && rules.slow_before;
    match right {
        Some(true) => Asks {
            explain: rules.first_right_in_set,
            feel: slow_streak,
        },
        Some(false) => Asks {
            explain: true,
            feel: true,
        },
        None => Asks {
            explain: false,
            feel: true,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SLOW: u32 = 30_000;

    fn rules(first_right_in_set: bool, slow_before: bool) -> AskRules {
        AskRules {
            first_right_in_set,
            slow_before,
        }
    }

    #[test]
    fn a_wrong_answer_asks_both() {
        let both = Asks {
            explain: true,
            feel: true,
        };
        assert_eq!(asks(rules(false, false), Some(false), 1_000, SLOW), both);
    }

    #[test]
    fn a_right_answer_asks_how_only_once_per_set() {
        let first = asks(rules(true, false), Some(true), 1_000, SLOW);
        assert_eq!(
            first,
            Asks {
                explain: true,
                feel: false
            }
        );
        let later = asks(rules(false, false), Some(true), 1_000, SLOW);
        assert_eq!(later, Asks::default());
    }

    #[test]
    fn two_slow_answers_in_a_row_ask_how_they_feel() {
        let once = asks(rules(false, false), Some(true), SLOW + 1, SLOW);
        assert!(!once.feel);
        let twice = asks(rules(false, true), Some(true), SLOW + 1, SLOW);
        assert!(twice.feel);
    }

    #[test]
    fn i_dont_know_asks_only_how_they_feel() {
        let asked = asks(rules(true, false), None, 1_000, SLOW);
        assert_eq!(
            asked,
            Asks {
                explain: false,
                feel: true
            }
        );
    }

    #[test]
    fn only_stuck_and_tired_are_frustration() {
        let frustrated: Vec<_> = Feeling::ALL
            .into_iter()
            .filter(|f| f.frustrated())
            .collect();
        assert_eq!(frustrated, [Feeling::Stuck, Feeling::Tired]);
    }
}
