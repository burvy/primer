//! The grown-up check: a question a child of five to eight cannot answer, to
//! stand between child mode and the parent's pages (profiles, delete, export).
//!
//! It is a gate, not security: the parent is already signed in. Like
//! everything in the core it is a pure function: the page picks the number
//! (from the clock), and the same number always gives the same question.

/// The question for `seed`: two factors from 6 to 9, so the answer is
/// something a grown-up knows and a young child does not.
pub fn question(seed: u32) -> (u32, u32) {
    (6 + seed % 4, 6 + (seed / 4) % 4)
}

/// Is `typed` the right answer to [`question`]`(seed)`?
pub fn is_right(seed: u32, typed: u32) -> bool {
    let (a, b) = question(seed);
    typed == a * b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_question_has_factors_from_six_to_nine() {
        for seed in 0..64 {
            let (a, b) = question(seed);
            assert!((6..=9).contains(&a) && (6..=9).contains(&b), "{seed}: {a} x {b}");
        }
    }

    #[test]
    fn only_the_product_passes() {
        let (a, b) = question(5);
        assert!(is_right(5, a * b));
        assert!(!is_right(5, a * b + 1));
        assert!(!is_right(5, a + b));
    }

    #[test]
    fn the_same_seed_gives_the_same_question() {
        assert_eq!(question(11), question(11));
    }
}
