//! Spotting *why* an answer is wrong, using only the numbers.
//!
//! Some wrong answers leave a fingerprint ("signature"): one too many, digits
//! swapped, the wrong operation. Code can recognise those for free. Anything
//! it cannot name is left to a later step (Jev, the AI helper).

use crate::problem::{Blank, Op, Problem};

/// The kinds of mistake we name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MistakeKind {
    /// One more or one less than the answer.
    OffByOne,
    /// Did the other operation (added instead of subtracting, or the reverse).
    WrongOperation,
    /// Right digits, wrong order: wrote 21 for 12.
    DigitsSwapped,
    /// Off by exactly ten on a sum that crosses a ten: forgot to carry or borrow.
    /// Also the classic "smaller from larger" bug: `13 - 5` written as 12,
    /// because 3 - 5 is stuck, so the child does 5 - 3 instead. It is the same
    /// root cause (not knowing how to borrow), so it shares this kind and the
    /// borrowing hint, and the set of hints to write does not grow.
    CarryOrBorrow,
    /// Never returned by `detect`; set elsewhere, for example from a very fast answer.
    Guess,
    /// Never returned by `detect`; stands for "no signature matched".
    Unknown,
}

/// Names the mistake in `given`, or `None` if it is correct or has no signature.
///
/// Each check below is a small `bool` function. `then_some(kind)` turns `true`
/// into `Some(kind)` (and `false` into `None`), and `or_else` tries the next
/// check only if the previous one gave `None`. That is `Option` chaining: the
/// first signature that matches wins, in the order written.
pub fn detect(problem: &Problem, given: u32) -> Option<MistakeKind> {
    let answer = problem.answer();
    if given == answer {
        return None;
    }
    (given.abs_diff(answer) == 1)
        .then_some(MistakeKind::OffByOne)
        .or_else(|| digits_swapped(answer, given).then_some(MistakeKind::DigitsSwapped))
        .or_else(|| forgot_to_regroup(problem, given).then_some(MistakeKind::CarryOrBorrow))
        .or_else(|| smaller_from_larger(problem, given).then_some(MistakeKind::CarryOrBorrow))
        .or_else(|| {
            (wrong_operation_answer(problem) == Some(given)).then_some(MistakeKind::WrongOperation)
        })
}

/// Two-digit answers only: 12 and 21. A palindrome like 11 can't be "swapped"
/// because `given` would equal the answer, which `detect` already handled.
fn digits_swapped(answer: u32, given: u32) -> bool {
    (10..100).contains(&answer) && given == (answer % 10) * 10 + answer / 10
}

/// Off by exactly ten, on a problem whose ones digits cross a ten.
fn forgot_to_regroup(problem: &Problem, given: u32) -> bool {
    let (a, b) = (problem.a() % 10, problem.b() % 10);
    let crosses_ten = match problem.op() {
        Op::Add => a + b >= 10,
        Op::Sub => a < b,
    };
    crosses_ten && given.abs_diff(problem.answer()) == 10
}

/// `13 - 5` written as 12: the ones digits are 3 and 5, so the child takes the
/// smaller from the larger (5 - 3 = 2) and keeps the tens (10), giving 12.
fn smaller_from_larger(problem: &Problem, given: u32) -> bool {
    let (a, b) = (problem.a(), problem.b());
    problem.op() == Op::Sub
        && problem.blank() == Blank::Answer
        && a % 10 < b % 10
        && given == a / 10 * 10 + (b % 10 - a % 10)
}

/// What the child would type had they used the wrong operation, if that is
/// meaningful. `match` on a tuple looks at two values at once.
fn wrong_operation_answer(problem: &Problem) -> Option<u32> {
    let (a, b) = (problem.a(), problem.b());
    match (problem.op(), problem.blank()) {
        (Op::Add, Blank::Answer) => a.checked_sub(b),
        (Op::Sub, Blank::Answer) => Some(a + b),
        // `3 + □ = 7` / `□ + 4 = 7`: the child adds the two numbers they can see.
        (Op::Add, Blank::Second) => Some(a + problem.whole()),
        (Op::Add, Blank::First) => Some(b + problem.whole()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn add(a: u32, b: u32) -> Problem {
        Problem::new(a, Op::Add, b).unwrap()
    }

    #[test]
    fn correct_answer_has_no_mistake() {
        assert_eq!(detect(&add(3, 4), 7), None);
    }

    #[test]
    fn off_by_one() {
        assert_eq!(detect(&add(3, 4), 6), Some(MistakeKind::OffByOne));
        assert_eq!(detect(&add(3, 4), 8), Some(MistakeKind::OffByOne));
    }

    #[test]
    fn wrong_operation() {
        assert_eq!(detect(&add(9, 4), 5), Some(MistakeKind::WrongOperation));
        let sub = Problem::new(9, Op::Sub, 4).unwrap();
        assert_eq!(detect(&sub, 13), Some(MistakeKind::WrongOperation));
    }

    #[test]
    fn wrong_operation_on_missing_number() {
        let p = Problem::with_blank(3, Op::Add, 4, Blank::Second).unwrap();
        assert_eq!(detect(&p, 10), Some(MistakeKind::WrongOperation));
    }

    #[test]
    fn digits_swapped() {
        // 8 + 4 = 12, written as 21.
        assert_eq!(detect(&add(8, 4), 21), Some(MistakeKind::DigitsSwapped));
    }

    #[test]
    fn forgot_to_carry() {
        // 8 + 7 = 15, written as 5.
        assert_eq!(detect(&add(8, 7), 5), Some(MistakeKind::CarryOrBorrow));
    }

    #[test]
    fn unrecognised_wrong_answer_has_no_signature() {
        assert_eq!(detect(&add(3, 4), 15), None);
    }
}
