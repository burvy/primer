//! The on-screen keypad as a pure function.
//!
//! A *reducer* takes the current state and one event and returns the new
//! state: `(Input, Key) -> Input`. It touches no screen and keeps no hidden
//! memory, so every behaviour (typing, backspace, limits) is tested with plain
//! `assert_eq!`. The browser just calls it from a button click and shows the
//! result. This is the same idea as `Learner::apply`: state in, event in,
//! new state out.

/// The biggest answer the keypad accepts (two digits).
const MAX: u32 = 99;

/// What the child has typed so far.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Input {
    /// Nothing typed yet.
    #[default]
    Empty,
    /// A number, like `7` or `12`.
    Number(u32),
    /// The child tapped "I don't know".
    DontKnow,
}

/// One button on the keypad.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    /// A digit key, 0 to 9.
    Digit(u8),
    /// The backspace key, ⌫.
    Back,
    /// The "I don't know" key.
    DontKnow,
}

impl Input {
    /// The typed number, or `None` for "nothing yet" and "I don't know".
    pub fn answer(self) -> Option<u32> {
        match self {
            Input::Number(n) => Some(n),
            Input::Empty | Input::DontKnow => None,
        }
    }

    /// Whether there is something to send (a number or "I don't know").
    pub fn is_ready(self) -> bool {
        self != Input::Empty
    }

    /// What to show inside the box: the digits typed, or nothing.
    pub fn shown(self) -> String {
        match self {
            Input::Number(n) => n.to_string(),
            Input::Empty | Input::DontKnow => String::new(),
        }
    }
}

/// Applies one key press and gives back the new input.
///
/// `match` on a tuple looks at the old input and the key together.
/// A third digit is ignored (the box holds two), and a leading zero simply
/// disappears: typing 0 then 7 gives 7, because `0 * 10 + 7` is 7.
pub fn press(input: Input, key: Key) -> Input {
    match (input, key) {
        (_, Key::DontKnow) => Input::DontKnow,
        (Input::Number(n), Key::Digit(d)) if n * 10 + u32::from(d) <= MAX => {
            Input::Number(n * 10 + u32::from(d))
        }
        (Input::Number(n), Key::Digit(_)) => Input::Number(n),
        (Input::Empty | Input::DontKnow, Key::Digit(d)) => Input::Number(u32::from(d)),
        (Input::Number(n), Key::Back) if n >= 10 => Input::Number(n / 10),
        (_, Key::Back) => Input::Empty,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Presses a list of keys in order, starting from an empty box.
    fn type_keys(keys: &[Key]) -> Input {
        keys.iter()
            .fold(Input::Empty, |input, &key| press(input, key))
    }

    #[test]
    fn digits_build_a_number() {
        assert_eq!(
            type_keys(&[Key::Digit(1), Key::Digit(2)]),
            Input::Number(12)
        );
    }

    #[test]
    fn a_third_digit_is_ignored() {
        let keys = [Key::Digit(1), Key::Digit(2), Key::Digit(3)];
        assert_eq!(type_keys(&keys), Input::Number(12));
    }

    #[test]
    fn leading_zero_disappears() {
        assert_eq!(type_keys(&[Key::Digit(0), Key::Digit(7)]), Input::Number(7));
        assert_eq!(type_keys(&[Key::Digit(0)]), Input::Number(0));
    }

    #[test]
    fn back_removes_the_last_digit_then_empties() {
        let twelve = [Key::Digit(1), Key::Digit(2)];
        assert_eq!(
            type_keys(&[twelve[0], twelve[1], Key::Back]),
            Input::Number(1)
        );
        assert_eq!(type_keys(&[Key::Digit(1), Key::Back]), Input::Empty);
        assert_eq!(type_keys(&[Key::Back]), Input::Empty);
    }

    #[test]
    fn dont_know_replaces_anything_and_digits_start_again() {
        assert_eq!(type_keys(&[Key::Digit(4), Key::DontKnow]), Input::DontKnow);
        assert_eq!(type_keys(&[Key::DontKnow, Key::Digit(4)]), Input::Number(4));
        assert_eq!(type_keys(&[Key::DontKnow, Key::Back]), Input::Empty);
    }

    #[test]
    fn answer_and_ready() {
        assert_eq!(Input::Number(5).answer(), Some(5));
        assert_eq!(Input::DontKnow.answer(), None);
        assert!(!Input::Empty.is_ready());
        assert!(Input::DontKnow.is_ready());
        assert_eq!(Input::Number(5).shown(), "5");
    }
}
