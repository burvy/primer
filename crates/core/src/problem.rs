//! A single sum the child is asked to solve, such as `3 + 4`.

use std::fmt;

/// The kind of sum. An `enum` is a type that is exactly one of a fixed list
/// of variants, so a value can never be "some other operation" by accident.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Add,
    Sub,
}

/// `a op b`, for example `3 + 4`.
///
/// The fields are private (no `pub`), so code outside this module cannot
/// build a `Problem` directly. The only way in is [`Problem::new`], which
/// refuses bad input. That makes an invalid problem (like `3 - 5`, which
/// would be negative) *unrepresentable*: if you hold a `Problem`, it is valid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Problem {
    a: u32,
    op: Op,
    b: u32,
}

impl Problem {
    /// Builds a problem, or returns `None` if it is not allowed.
    ///
    /// `Option` is Rust's way to say "maybe there is no answer": it is either
    /// `Some(value)` or `None`. Subtraction may not go below zero.
    pub fn new(a: u32, op: Op, b: u32) -> Option<Problem> {
        match op {
            Op::Sub if b > a => None,
            _ => Some(Problem { a, op, b }),
        }
    }

    /// The correct answer.
    ///
    /// `match` must cover every variant of `Op`; if we add a new one later,
    /// the compiler points at every `match` we forgot to update.
    pub fn answer(&self) -> u32 {
        match self.op {
            Op::Add => self.a + self.b,
            // Safe: `new` guarantees `b <= a`, so this cannot underflow.
            Op::Sub => self.a - self.b,
        }
    }
}

/// Lets us write `problem.to_string()` or `format!("{problem}")`.
impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let symbol = match self.op {
            Op::Add => '+',
            Op::Sub => '-',
        };
        write!(f, "{} {} {}", self.a, symbol, self.b)
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
}
