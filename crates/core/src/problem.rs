//! A single sum the child is asked to solve, such as `3 + 4`.

use std::fmt;

/// The kind of sum. An `enum` is a type that is exactly one of a fixed list
/// of variants, so a value can never be "some other operation" by accident.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Add,
    Sub,
}

/// Where the box (the unknown number the child must find) sits.
///
/// Call the whole `c`, so `a op b = c`. The three shapes are
/// `a op b = □`, `a op □ = c` and `□ op b = c`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Blank {
    /// `3 + 4 = □`: the child finds the result.
    Answer,
    /// `3 + □ = 7`: the child finds the second number.
    Second,
    /// `□ + 4 = 7`: the child finds the first number.
    First,
}

/// `a op b`, for example `3 + 4`, plus where the box sits.
///
/// We store only `a`, `op`, `b` and the box position. The whole (`c`, the 7
/// in `3 + □ = 7`) is never stored, because it can always be worked out. One
/// source of truth means the numbers can never disagree with each other.
///
/// The fields are private (no `pub`), so code outside this module cannot
/// build a `Problem` directly. The only way in is [`Problem::new`] or
/// [`Problem::with_blank`], which refuse bad input. That makes an invalid
/// problem (like `3 - 5`, which would be negative) *unrepresentable*: if you
/// hold a `Problem`, it is valid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Problem {
    a: u32,
    op: Op,
    b: u32,
    blank: Blank,
}

impl Problem {
    /// Builds `a op b = □`, or returns `None` if it is not allowed.
    ///
    /// `Option` is Rust's way to say "maybe there is no answer": it is either
    /// `Some(value)` or `None`. Subtraction may not go below zero.
    pub fn new(a: u32, op: Op, b: u32) -> Option<Problem> {
        Problem::with_blank(a, op, b, Blank::Answer)
    }

    /// Like [`Problem::new`], but you choose where the box sits.
    pub fn with_blank(a: u32, op: Op, b: u32, blank: Blank) -> Option<Problem> {
        match op {
            Op::Sub if b > a => None,
            _ => Some(Problem { a, op, b, blank }),
        }
    }

    /// Read-only access to the private fields ("getters").
    pub fn a(&self) -> u32 {
        self.a
    }

    pub fn op(&self) -> Op {
        self.op
    }

    pub fn b(&self) -> u32 {
        self.b
    }

    pub fn blank(&self) -> Blank {
        self.blank
    }

    /// The whole, `a op b` (the `c` in `3 + □ = 7`).
    ///
    /// `match` must cover every variant of `Op`; if we add a new one later,
    /// the compiler points at every `match` we forgot to update.
    pub fn whole(&self) -> u32 {
        match self.op {
            Op::Add => self.a + self.b,
            // Safe: `with_blank` guarantees `b <= a`, so this cannot underflow.
            Op::Sub => self.a - self.b,
        }
    }

    /// The number the child must type: whatever sits under the box.
    pub fn answer(&self) -> u32 {
        match self.blank {
            Blank::Answer => self.whole(),
            Blank::Second => self.b,
            Blank::First => self.a,
        }
    }
}

/// Lets us write `problem.to_string()` or `format!("{problem}")`.
///
/// `3 + 4` when the child finds the result (the page adds the `= □`), and
/// the full `3 + □ = 7` when the box is inside the sum.
impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let symbol = match self.op {
            Op::Add => '+',
            Op::Sub => '-',
        };
        let (a, b, c) = (self.a, self.b, self.whole());
        match self.blank {
            Blank::Answer => write!(f, "{a} {symbol} {b}"),
            Blank::Second => write!(f, "{a} {symbol} □ = {c}"),
            Blank::First => write!(f, "□ {symbol} {b} = {c}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds() {
        assert_eq!(Problem::new(3, Op::Add, 4).unwrap().answer(), 7);
    }

    #[test]
    fn subtracts() {
        assert_eq!(Problem::new(9, Op::Sub, 4).unwrap().answer(), 5);
    }

    #[test]
    fn subtracting_to_zero_is_allowed() {
        assert_eq!(Problem::new(4, Op::Sub, 4).unwrap().answer(), 0);
    }

    #[test]
    fn displays_as_a_sum() {
        assert_eq!(Problem::new(3, Op::Add, 4).unwrap().to_string(), "3 + 4");
        assert_eq!(Problem::new(9, Op::Sub, 4).unwrap().to_string(), "9 - 4");
    }

    #[test]
    fn rejects_negative_subtraction() {
        assert_eq!(Problem::new(3, Op::Sub, 5), None);
    }

    #[test]
    fn missing_number_answer_is_the_hidden_number() {
        let second = Problem::with_blank(3, Op::Add, 4, Blank::Second).unwrap();
        assert_eq!(second.answer(), 4);
        assert_eq!(second.to_string(), "3 + □ = 7");
        let first = Problem::with_blank(3, Op::Add, 4, Blank::First).unwrap();
        assert_eq!(first.answer(), 3);
        assert_eq!(first.to_string(), "□ + 4 = 7");
    }
}
