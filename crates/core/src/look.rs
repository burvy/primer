//! The child's chosen look: `Gentle` or `Exceptional`.
//!
//! A "look" is two things at once: a visual theme (colours, fonts) and a
//! difficulty tweak the teaching policy reads. Progress is shared, so a child
//! can switch looks at any time without losing work.

/// The two looks of the book. An `enum` is a type that is exactly one of a
/// fixed list of variants, so a value can never be "some other look".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Look {
    /// Storybook style: warm illustrated pages, normal pace.
    Gentle,
    /// Treatise style: cream paper, coloured shapes, harder track.
    Exceptional,
}

impl Look {
    /// How many right answers in a row move the child off the pictorial stage.
    ///
    /// `self` here means the look value itself (it is `Copy`, so using it does
    /// not move or borrow anything). Gentle takes 3; Exceptional takes 1, so
    /// Exceptional children skip pictorial work faster.
    pub fn pictorial_wins_needed(self) -> u32 {
        match self {
            Look::Gentle => 3,
            Look::Exceptional => 1,
        }
    }

    /// Whether this look sees extra missing-number forms.
    ///
    /// A "missing-number form" is where the box sits, for example `3 + □ = 7`
    /// versus `□ + 4 = 7`. Gentle sees only the first kind; Exceptional sees
    /// both. The generator (a later slice) reads this flag.
    pub fn extra_missing_forms(self) -> bool {
        match self {
            Look::Gentle => false,
            Look::Exceptional => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gentle_needs_three_pictorial_wins() {
        assert_eq!(Look::Gentle.pictorial_wins_needed(), 3);
    }

    #[test]
    fn exceptional_skips_pictorial_after_one() {
        assert_eq!(Look::Exceptional.pictorial_wins_needed(), 1);
    }

    #[test]
    fn only_exceptional_gets_extra_missing_forms() {
        assert!(!Look::Gentle.extra_missing_forms());
        assert!(Look::Exceptional.extra_missing_forms());
    }
}
