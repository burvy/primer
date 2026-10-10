//! The hero's name: the child picks one from a list or types one.
//!
//! A child's real name must never be stored (docs/DESIGN.md, Privacy). This
//! file is the first guard, a plain rule that needs no network: one word,
//! letters only, at most [`MAX_LETTERS`] of them. "Bob Smith", "Bob2" and
//! "Supercalifragilistic" all fail. A single word of letters can still be a
//! real first name, so the server asks the judge next (`Judge::identifying`).
//!
//! `HeroName` is a *newtype*: a struct wrapping one `String`, with a private
//! field. The only way to make one is [`HeroName::parse`], so any `HeroName`
//! in the program has already passed the rule. ("Parse, don't validate": the
//! check happens once, and the type remembers it.)

use std::fmt;

/// The longest hero name, in letters.
pub const MAX_LETTERS: usize = 12;

/// A name that passed the rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeroName(String);

/// Why a typed name was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeroNameError {
    /// Nothing typed.
    Empty,
    /// More than one word, like "Bob Smith".
    NotOneWord,
    /// A digit, a symbol or punctuation.
    NotLetters,
    /// More than [`MAX_LETTERS`] letters.
    TooLong,
}

impl HeroName {
    /// The name, if it is one word of letters and not too long. Spaces around
    /// it are ignored.
    pub fn parse(typed: &str) -> Result<HeroName, HeroNameError> {
        let name = typed.trim();
        if name.is_empty() {
            return Err(HeroNameError::Empty);
        }
        if name.split_whitespace().count() > 1 {
            return Err(HeroNameError::NotOneWord);
        }
        if !name.chars().all(char::is_alphabetic) {
            return Err(HeroNameError::NotLetters);
        }
        if name.chars().count() > MAX_LETTERS {
            return Err(HeroNameError::TooLong);
        }
        Ok(HeroName(name.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for HeroName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl HeroNameError {
    /// A sentence for the grown-up, who may be helping with the typing.
    pub fn message(self) -> &'static str {
        match self {
            HeroNameError::Empty => "type a name",
            HeroNameError::NotOneWord => "a hero name is one word",
            HeroNameError::NotLetters => "a hero name has letters only",
            HeroNameError::TooLong => "a hero name has 12 letters at most",
        }
    }
}

impl fmt::Display for HeroNameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(typed: &str) -> Result<String, HeroNameError> {
        HeroName::parse(typed).map(|name| name.to_string())
    }

    #[test]
    fn one_word_of_letters_is_a_name() {
        assert_eq!(parsed("Lovelace"), Ok("Lovelace".to_string()));
        assert_eq!(parsed("  Noether "), Ok("Noether".to_string()));
        // Letters are letters in any alphabet.
        assert_eq!(parsed("Erdős"), Ok("Erdős".to_string()));
    }

    #[test]
    fn a_full_name_is_refused() {
        assert_eq!(parsed("Bob Smith"), Err(HeroNameError::NotOneWord));
        assert_eq!(parsed("Ada  Lovelace"), Err(HeroNameError::NotOneWord));
    }

    #[test]
    fn digits_and_symbols_are_refused() {
        assert_eq!(parsed("Bob2"), Err(HeroNameError::NotLetters));
        assert_eq!(parsed("1234"), Err(HeroNameError::NotLetters));
        assert_eq!(parsed("Mary-Jane"), Err(HeroNameError::NotLetters));
        assert_eq!(parsed("a@b.com"), Err(HeroNameError::NotLetters));
    }

    #[test]
    fn thirteen_letters_are_too_many_and_twelve_are_fine() {
        assert_eq!(parsed("Abcdefghijkl"), Ok("Abcdefghijkl".to_string()));
        assert_eq!(parsed("Abcdefghijklm"), Err(HeroNameError::TooLong));
    }

    #[test]
    fn nothing_is_refused() {
        assert_eq!(parsed(""), Err(HeroNameError::Empty));
        assert_eq!(parsed("   "), Err(HeroNameError::Empty));
    }
}
