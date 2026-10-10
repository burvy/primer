//! Patterns: named shapes for a run of problems, like "the sum goes up by one".
//!
//! A child learns more from `3 + 4, 4 + 4, 5 + 4` than from three random sums,
//! because the run itself shows an idea. A `Pattern` names that idea, and
//! [`walk`] turns it into problems. Walking is pure: the same pattern and
//! start always give the same problems. Randomness lives elsewhere (in
//! `problem_set`), and only picks *which* pattern and *where* to start.
//!
//! Every pattern works the same way. It has a *cursor*, a pair `(a, b)` of
//! starting numbers. At each cursor it shows a small *group* of problems
//! (one sum, or a pair like `3 + 5, 5 + 3`), then moves the cursor one step.

use std::iter;

use crate::problem::{Blank, Op, Problem};

/// The most a bridge may add after making ten: `8 + 5` is `8 + 2`, then 3 more.
/// Five is as far as chapter 1's "count on" reaches.
const MAX_REST: u32 = 5;

/// The shapes a run can take. Each example shows the first few problems.
///
/// Most variants are plain names, but an `enum` variant can also *carry
/// data*: `BondWalk { total }` remembers which number the bonds add up to,
/// and `Parallel { op }` remembers whether it adds or subtracts. The data
/// travels with the variant, so a `match` gets it back out by name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pattern {
    /// `3 + 4, 4 + 4, 5 + 4`: the first number goes up by one, so the sum does too.
    Successor,
    /// `7 - 3, 6 - 3, 5 - 3`: the first number goes down by one, so the answer does too.
    Predecessor,
    /// `3 + 5, 5 + 3, 2 + 6, 6 + 2`: swapping the two numbers keeps the sum.
    Swap,
    /// `2 + 2, 3 + 3, 4 + 4`: a number added to itself.
    Doubles,
    /// `6 + 6, 6 + 7, 7 + 7, 7 + 8`: one more than a double is one more.
    NearDoubles,
    /// `3 + 4, 4 + 3, 3 + □ = 7, 4 + □ = 7`: one family, the same three numbers.
    Family,
    /// `3 + 4, 7 - 4`: taking away what you added gets you back.
    Undo,
    /// `1 + □ = 10, 2 + □ = 10, 3 + □ = 10`: one up, one down, same total.
    BondWalk { total: u32 },
    /// `3 + 4, 13 + 4`: same ones, one more ten, so the answer has one more ten.
    Parallel { op: Op },
    /// `3 + 0, 0 + 3, 7 + 0, 0 + 7`: adding zero changes nothing. Zero is the
    /// *identity* for addition: the one number that leaves the other alone.
    Identity,
    /// `8 + 2, 8 + 5`: make ten first, then add the rest. The second addend
    /// splits as `5 = 2 + 3`: the part that makes ten, then a small rest.
    Bridge,
    /// `13 - 3, 13 - 5`: go back to ten first, then take the rest.
    BridgeBack,
}

impl Pattern {
    /// A short machine name, used in set ids and to find the pattern's
    /// human-written `notice` line in the content files.
    pub fn id(self) -> &'static str {
        match self {
            Pattern::Successor => "successor",
            Pattern::Predecessor => "predecessor",
            Pattern::Swap => "swap",
            Pattern::Doubles => "doubles",
            Pattern::NearDoubles => "near-doubles",
            Pattern::Family => "family",
            Pattern::Undo => "undo",
            Pattern::BondWalk { .. } => "bond-walk",
            Pattern::Parallel { .. } => "parallel",
            Pattern::Identity => "identity",
            Pattern::Bridge => "bridge",
            Pattern::BridgeBack => "bridge-back",
        }
    }

    /// The group of problems shown at cursor `(a, b)`, or `None` if any of
    /// them is impossible (like `3 - 5`, which would go below zero).
    ///
    /// Each problem is built as an `Option<Problem>`. Collecting a list of
    /// `Option`s into `Option<Vec<_>>` gives `Some(list)` only if *every*
    /// item was `Some`; a single `None` makes the whole group `None`.
    /// `checked_sub` is subtraction that returns `None` instead of going
    /// below zero, so `and_then` only builds the problem when it can.
    fn group(self, (a, b): (u32, u32)) -> Option<Vec<Problem>> {
        let add = |x, y| Problem::new(x, Op::Add, y);
        let sub = |x, y| Problem::new(x, Op::Sub, y);
        let add_box = |x, y| Problem::with_blank(x, Op::Add, y, Blank::Second);
        let shown = match self {
            // Zero is left out of the patterns below on purpose: `3 + 0, 4 + 0`
            // would look like an identity run, and `0 + 0` is silly. Only
            // `Identity` shows zero as an addend. `Option::filter` keeps a
            // problem only when its test is true, otherwise it becomes `None`.
            Pattern::Successor => vec![add(a, b).filter(|_| b > 0)],
            Pattern::Predecessor => vec![sub(a, b)],
            Pattern::Swap => vec![
                add(a, b).filter(|_| a > 0 && b > 0),
                add(b, a).filter(|_| a > 0 && b > 0),
            ],
            Pattern::Doubles => vec![add(a, a).filter(|_| a > 0)],
            Pattern::NearDoubles => vec![add(a, a), add(a, a + 1)],
            Pattern::Family => vec![add(a, b), add(b, a), add_box(a, b), add_box(b, a)],
            Pattern::Undo => vec![add(a, b), sub(a + b, b)],
            Pattern::BondWalk { total } => vec![total.checked_sub(a).and_then(|b| add_box(a, b))],
            Pattern::Parallel { op } => vec![Problem::new(a, op, b), Problem::new(a + 10, op, b)],
            Pattern::Identity => vec![add(a, 0).filter(|_| a > 0), add(0, a).filter(|_| a > 0)],
            Pattern::Bridge => {
                // `split` is the part of `b` that makes ten, if `b` really
                // splits as "make ten, then a small rest" (1 to 5 more).
                // Otherwise it is `None` and the whole group is dropped.
                let split = 10u32
                    .checked_sub(a)
                    .filter(|&to_ten| to_ten > 0 && b > to_ten && b - to_ten <= MAX_REST);
                vec![
                    split.and_then(|to_ten| add(a, to_ten)),
                    split.and_then(|_| add(a, b)),
                ]
            }
            Pattern::BridgeBack => vec![a.checked_sub(10).and_then(|ones| sub(a, ones)), sub(a, b)],
        };
        shown.into_iter().collect()
    }

    /// Where the cursor goes next. Most patterns move the first number up by
    /// one; `Predecessor` moves it down, and `Swap` trades one from the first
    /// number to the second. `None` means "cannot go below zero, stop here".
    fn step(self, (a, b): (u32, u32)) -> Option<(u32, u32)> {
        match self {
            Pattern::Predecessor => a.checked_sub(1).map(|a| (a, b)),
            Pattern::Swap => a.checked_sub(1).map(|a| (a, b + 1)),
            _ => Some((a + 1, b)),
        }
    }
}

/// Every group of the pattern from `start`, one cursor step at a time.
///
/// `iter::successors` makes an iterator from a first value and a "next"
/// function: `start`, `step(start)`, `step(step(start))`, and so on until
/// `step` gives `None`. It is *lazy*: nothing is computed until asked for,
/// so the iterator may be endless and the caller decides when to stop.
/// `map_while` turns each cursor into its group and stops at the first
/// impossible one. `move` hands `pattern` (a small `Copy` value) to each
/// closure, so the iterator can outlive this function call.
pub fn groups(pattern: Pattern, start: (u32, u32)) -> impl Iterator<Item = Vec<Problem>> {
    iter::successors(Some(start), move |&at| pattern.step(at))
        .map_while(move |at| pattern.group(at))
}

/// The first `len` problems of the pattern from `start`, in order.
///
/// `flatten` joins the groups into one stream of problems and `take` stops
/// after `len` of them. The result may be shorter if the pattern runs out.
pub fn walk(pattern: Pattern, start: (u32, u32), len: usize) -> Vec<Problem> {
    groups(pattern, start).flatten().take(len).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Property tests by exhaustion: every start with both numbers up to 20.
    fn starts() -> impl Iterator<Item = (u32, u32)> {
        (0..=20).flat_map(|a| (0..=20).map(move |b| (a, b)))
    }

    /// The first five groups of the pattern from every start.
    fn all_groups(pattern: Pattern) -> Vec<Vec<Problem>> {
        starts()
            .flat_map(|start| groups(pattern, start).take(5))
            .collect()
    }

    /// Checks each neighbouring pair of answers along every walk.
    /// `windows(2)` slides a two-wide view along the list: `[p0, p1]`, `[p1, p2]`...
    fn each_neighbour(pattern: Pattern, check: impl Fn(&Problem, &Problem)) {
        for start in starts() {
            for pair in walk(pattern, start, 6).windows(2) {
                check(&pair[0], &pair[1]);
            }
        }
    }

    #[test]
    fn successor_answers_go_up_by_one() {
        each_neighbour(Pattern::Successor, |p, q| {
            assert_eq!(q.answer(), p.answer() + 1)
        });
    }

    #[test]
    fn predecessor_answers_go_down_by_one() {
        each_neighbour(Pattern::Predecessor, |p, q| {
            assert_eq!(q.answer() + 1, p.answer())
        });
        assert_eq!(
            walk(Pattern::Predecessor, (5, 3), 9).len(),
            3,
            "stops at 3 - 3"
        );
    }

    #[test]
    fn doubles_add_a_number_to_itself() {
        each_neighbour(Pattern::Doubles, |p, q| {
            assert_eq!((p.a(), q.a()), (p.b(), q.b()));
            assert_eq!(q.answer(), p.answer() + 2);
        });
    }

    #[test]
    fn swap_keeps_the_sum_with_the_numbers_swapped() {
        for g in all_groups(Pattern::Swap) {
            assert_eq!(g[0].answer(), g[1].answer());
            assert_eq!((g[0].a(), g[0].b()), (g[1].b(), g[1].a()));
        }
    }

    #[test]
    fn near_double_is_one_more_than_the_double() {
        for g in all_groups(Pattern::NearDoubles) {
            assert_eq!(g[0].a(), g[0].b());
            assert_eq!(g[1].answer(), g[0].answer() + 1);
        }
    }

    #[test]
    fn family_uses_the_same_three_numbers() {
        for g in all_groups(Pattern::Family) {
            let mut first = [g[0].a(), g[0].b(), g[0].whole()];
            first.sort();
            for p in &g {
                let mut these = [p.a(), p.b(), p.whole()];
                these.sort();
                assert_eq!(these, first);
            }
        }
    }

    #[test]
    fn undo_gets_back_to_the_first_number() {
        for g in all_groups(Pattern::Undo) {
            assert_eq!(g[1].op(), Op::Sub);
            assert_eq!(g[1].answer(), g[0].a());
        }
    }

    #[test]
    fn bond_walk_always_makes_the_total_with_a_box() {
        for g in all_groups(Pattern::BondWalk { total: 10 }) {
            assert_eq!(g[0].whole(), 10);
            assert_eq!(g[0].blank(), Blank::Second);
        }
        assert_eq!(
            walk(Pattern::BondWalk { total: 10 }, (1, 0), 3)[2].to_string(),
            "3 + □ = 10"
        );
    }

    #[test]
    fn parallel_adds_one_ten_to_the_answer() {
        for op in [Op::Add, Op::Sub] {
            for g in all_groups(Pattern::Parallel { op }) {
                assert_eq!(g[1].answer(), g[0].answer() + 10);
            }
        }
    }

    #[test]
    fn identity_adds_zero_and_keeps_the_other_number() {
        for g in all_groups(Pattern::Identity) {
            for p in &g {
                assert!(p.a() == 0 || p.b() == 0, "one addend is zero");
                assert_eq!(p.answer(), p.a() + p.b(), "the answer is the other addend");
            }
            assert_ne!(g[0].answer(), 0, "never 0 + 0");
        }
    }

    #[test]
    fn other_add_patterns_never_show_zero_addends() {
        for pattern in [Pattern::Successor, Pattern::Swap, Pattern::Doubles] {
            for g in all_groups(pattern) {
                for p in &g {
                    assert!(p.b() > 0, "{p}");
                    if pattern != Pattern::Successor {
                        assert!(p.a() > 0, "{p}");
                    }
                }
            }
        }
    }

    #[test]
    fn bridge_makes_ten_first_then_splits_the_second_addend() {
        for g in all_groups(Pattern::Bridge) {
            assert_eq!(g[0].answer(), 10);
            assert_eq!(g[0].a(), g[1].a(), "same first addend");
            // `8 + 2` then `8 + 5`: 5 = 2 + 3, so the rest (3) is 1 to 5.
            let rest = g[1].b() - g[0].b();
            assert!((1..=MAX_REST).contains(&rest), "{} then {}", g[0], g[1]);
        }
    }

    #[test]
    fn bridge_back_lands_on_ten_first() {
        for g in all_groups(Pattern::BridgeBack) {
            assert_eq!(g[0].answer(), 10);
            assert_eq!(g[0].a(), g[1].a());
        }
    }
}
