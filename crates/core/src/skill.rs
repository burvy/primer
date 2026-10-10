//! The skill ladder as data: one chapter per skill, in one list.
//!
//! Each skill is a plain `Skill` value (a `struct`: named fields bundled
//! together). The list is a `const`: fixed when the program is built, shared
//! everywhere, never changed while running.

use super::pattern::Pattern;
use super::problem::Op;

/// How a problem is *shown*: counters, a number line, or the bare equation.
///
/// Children move concrete to pictorial to abstract: first ten frames and
/// counters, then the number line, then just numbers. Real objects (the fully
/// concrete step) are the grown-up's part, so they are not a `Stage` here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// Ten frames and counters.
    Pictorial,
    /// A number line to hop along.
    NumberLine,
    /// The bare equation, like `3 + 4`.
    Abstract,
}

/// Which part of the book a skill belongs to. Version 1 ships only `Core`;
/// the field exists so later tracks need no code change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Track {
    Core,
}

/// Whether a chapter's sums cross a ten ("carry" when adding, "borrow" when
/// subtracting). Looking only at the ones digits: `7 + 5` carries because
/// 7 + 5 is 10 or more, and `13 - 5` borrows because 3 is less than 5.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Regroup {
    /// Both kinds are fine (used by the "within 10" chapters).
    Either,
    /// Never crosses a ten.
    Without,
    /// Always crosses a ten.
    With,
}

/// One chapter of the book, for example "make ten" or "3 + box = 7".
///
/// `id` is a short machine name like `"add-within-10"`; `title` is the
/// chapter heading the child sees. `op` is which kind of sum this chapter
/// teaches. `min_a`, `max_a`, `min_b`, `max_b` bound the two operands.
/// `max_whole` is the biggest number in the sum (the total when adding, the
/// starting number when subtracting) and `regroup` is the carry rule.
/// `total` is `Some` only for number-bond chapters: every sum must add up to
/// exactly that number. Together they are the chapter's promise:
/// [`Skill::allows`] checks it, and the generator only picks `a` and `b`
/// that pass.
/// `missing_addend` says the box sits in the middle (`3 + box = 7`) instead
/// of at the end (`3 + 4 = box`). `stages` lists how the chapter is shown, in
/// order. `requires` lists earlier skill ids to master first. `words` lists
/// the vocabulary ids this chapter teaches (see `content/words.toml`).
/// `patterns` lists the shapes generated sets may take (see `pattern.rs`).
/// `track` is always `Track::Core` in version 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Skill {
    pub id: &'static str,
    pub title: &'static str,
    pub op: Op,
    pub min_a: u32,
    pub max_a: u32,
    pub min_b: u32,
    pub max_b: u32,
    pub max_whole: u32,
    pub regroup: Regroup,
    pub total: Option<u32>,
    pub missing_addend: bool,
    pub stages: &'static [Stage],
    pub requires: &'static [&'static str],
    pub words: &'static [&'static str],
    pub patterns: &'static [Pattern],
    pub track: Track,
}

impl Skill {
    /// Looks up a skill by its `id`.
    ///
    /// `SKILLS.iter()` walks the list one item at a time; `find` stops at the
    /// first match and gives back `Some(skill)` or `None`. The result borrows
    /// from the list, so nothing is copied.
    pub fn find(id: &str) -> Option<&'static Skill> {
        SKILLS.iter().find(|skill| skill.id == id)
    }

    /// Where this skill sits in the ladder: `0` for the first chapter.
    ///
    /// `position` works like `find` but gives back the index. Useful for
    /// ordering checks and progress bars.
    pub fn position(id: &str) -> Option<usize> {
        SKILLS.iter().position(|skill| skill.id == id)
    }

    /// Does `a op b` keep this chapter's promise?
    ///
    /// True when both numbers are in range, the biggest number stays under
    /// `max_whole`, the carry rule holds, and (for bond chapters) the sum is
    /// exactly `total`. Subtracting more than you have is never allowed.
    ///
    /// `Option` here means "maybe a rule": `None` means no fixed total, so
    /// that check passes; `Some(10)` means only sums to 10 pass.
    pub fn allows(&self, a: u32, b: u32) -> bool {
        let in_range = (self.min_a..=self.max_a).contains(&a)
            && (self.min_b..=self.max_b).contains(&b)
            && (self.op == Op::Add || b <= a)
            // `0 + 0` teaches nothing, so no chapter asks it.
            && (a, b) != (0, 0);
        let (whole, crosses_ten) = match self.op {
            Op::Add => (a + b, a % 10 + b % 10 >= 10),
            Op::Sub => (a, a % 10 < b % 10),
        };
        let regroup_ok = match self.regroup {
            Regroup::Either => true,
            Regroup::Without => !crosses_ten,
            Regroup::With => crosses_ten,
        };
        // `is_none_or(...)` says: no rule (`None`) passes; otherwise the
        // closure must hold. `|t|` is the fixed total, checked against `whole`.
        let total_ok = self.total.is_none_or(|t| whole == t);
        in_range && whole <= self.max_whole && regroup_ok && total_ok
    }
}

/// Every chapter of version 1, in teaching order.
///
/// `&[Skill]` is a slice (a borrowed view) of the whole ladder.
pub const SKILLS: &[Skill] = &[
    Skill {
        id: "count-on-within-5",
        title: "Count on within 5",
        op: Op::Add,
        // 0 is allowed as the first number, so `0 + 1` (the successor of zero) works.
        min_a: 0,
        max_a: 4,
        min_b: 1,
        max_b: 2,
        max_whole: 5,
        regroup: Regroup::Either,
        total: None,
        missing_addend: false,
        stages: &[Stage::Pictorial, Stage::NumberLine, Stage::Abstract],
        requires: &[],
        words: &["successor", "function"],
        patterns: &[Pattern::Successor],
        track: Track::Core,
    },
    Skill {
        id: "add-within-10",
        title: "Add within 10",
        op: Op::Add,
        // 0 is allowed on either side (the identity), but `allows` rejects `0 + 0`.
        min_a: 0,
        max_a: 9,
        min_b: 0,
        max_b: 8,
        max_whole: 10,
        regroup: Regroup::Either,
        total: None,
        missing_addend: false,
        stages: &[Stage::Pictorial, Stage::NumberLine, Stage::Abstract],
        requires: &["count-on-within-5"],
        words: &[
            "sum",
            "addend",
            "equation",
            "equals",
            "commutative",
            "doubles",
            "identity",
        ],
        patterns: &[
            Pattern::Successor,
            Pattern::Swap,
            Pattern::Doubles,
            Pattern::Identity,
        ],
        track: Track::Core,
    },
    Skill {
        id: "missing-addend-within-10",
        title: "Missing addend within 10",
        op: Op::Add,
        min_a: 2,
        max_a: 6,
        min_b: 1,
        max_b: 5,
        max_whole: 10,
        regroup: Regroup::Either,
        total: None,
        missing_addend: true,
        stages: &[Stage::Pictorial, Stage::NumberLine, Stage::Abstract],
        requires: &["add-within-10"],
        words: &["unknown", "fact-family"],
        patterns: &[Pattern::Family],
        track: Track::Core,
    },
    Skill {
        id: "make-ten",
        title: "Make ten",
        op: Op::Add,
        min_a: 1,
        max_a: 9,
        min_b: 1,
        max_b: 9,
        max_whole: 10,
        regroup: Regroup::Either,
        total: Some(10),
        missing_addend: true,
        stages: &[Stage::Pictorial, Stage::NumberLine, Stage::Abstract],
        requires: &["add-within-10"],
        words: &["number-bond", "compensation"],
        patterns: &[Pattern::BondWalk { total: 10 }],
        track: Track::Core,
    },
    Skill {
        id: "subtract-within-10",
        title: "Subtract within 10",
        op: Op::Sub,
        min_a: 3,
        max_a: 10,
        min_b: 1,
        max_b: 9,
        max_whole: 10,
        regroup: Regroup::Either,
        total: None,
        missing_addend: false,
        stages: &[Stage::Pictorial, Stage::NumberLine, Stage::Abstract],
        requires: &["add-within-10"],
        words: &["difference", "predecessor", "inverse"],
        patterns: &[Pattern::Predecessor, Pattern::Undo],
        track: Track::Core,
    },
    Skill {
        id: "add-within-20-no-carry",
        title: "Add within 20 without carrying",
        op: Op::Add,
        min_a: 11,
        max_a: 19,
        min_b: 1,
        max_b: 8,
        max_whole: 20,
        regroup: Regroup::Without,
        total: None,
        missing_addend: false,
        stages: &[Stage::Pictorial, Stage::NumberLine, Stage::Abstract],
        requires: &["add-within-10", "make-ten"],
        words: &["ones", "tens", "place-value"],
        patterns: &[Pattern::Parallel { op: Op::Add }],
        track: Track::Core,
    },
    Skill {
        id: "subtract-within-20-no-borrow",
        title: "Subtract within 20 without borrowing",
        op: Op::Sub,
        min_a: 11,
        max_a: 19,
        min_b: 1,
        max_b: 9,
        max_whole: 20,
        regroup: Regroup::Without,
        total: None,
        missing_addend: false,
        stages: &[Stage::Pictorial, Stage::NumberLine, Stage::Abstract],
        requires: &["subtract-within-10"],
        words: &["pattern"],
        patterns: &[Pattern::Parallel { op: Op::Sub }],
        track: Track::Core,
    },
    Skill {
        id: "add-within-20-carry",
        title: "Add within 20 with carrying",
        op: Op::Add,
        min_a: 6,
        max_a: 15,
        min_b: 5,
        max_b: 14,
        max_whole: 20,
        regroup: Regroup::With,
        total: None,
        missing_addend: false,
        stages: &[Stage::Pictorial, Stage::NumberLine, Stage::Abstract],
        requires: &["make-ten", "add-within-20-no-carry"],
        words: &["bridge-through-ten", "near-doubles"],
        patterns: &[Pattern::Bridge, Pattern::NearDoubles],
        track: Track::Core,
    },
    Skill {
        id: "subtract-within-20-borrow",
        title: "Subtract within 20 with borrowing",
        op: Op::Sub,
        min_a: 11,
        max_a: 18,
        min_b: 3,
        max_b: 9,
        max_whole: 20,
        regroup: Regroup::With,
        total: None,
        missing_addend: false,
        stages: &[Stage::Pictorial, Stage::NumberLine, Stage::Abstract],
        requires: &["subtract-within-20-no-borrow", "make-ten"],
        words: &["bridge-back-through-ten"],
        patterns: &[Pattern::BridgeBack],
        track: Track::Core,
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::problem::Problem;
    use std::collections::HashSet;

    #[test]
    fn ids_are_unique() {
        // `map` turns each skill into its id; `collect` gathers them into a
        // set, which drops duplicates. A smaller set means some id repeats.
        let ids: HashSet<&str> = SKILLS.iter().map(|skill| skill.id).collect();
        assert_eq!(ids.len(), SKILLS.len(), "skill ids must be unique");
    }

    #[test]
    fn ranges_are_non_empty() {
        for skill in SKILLS {
            assert!(skill.min_a <= skill.max_a);
            assert!(skill.min_b <= skill.max_b);
        }
    }

    #[test]
    fn every_skill_has_at_least_one_allowed_sum() {
        for skill in SKILLS {
            let any = (skill.min_a..=skill.max_a).any(|a| {
                (skill.min_b..=skill.max_b)
                    .any(|b| skill.allows(a, b) && Problem::new(a, skill.op, b).is_some())
            });
            assert!(any, "{} has no allowed sums", skill.id);
        }
    }

    #[test]
    fn every_skill_ends_at_abstract() {
        for skill in SKILLS {
            assert_eq!(skill.stages.last(), Some(&Stage::Abstract));
        }
    }

    #[test]
    fn requires_ids_all_exist() {
        for skill in SKILLS {
            for required in skill.requires {
                assert!(Skill::find(required).is_some());
            }
        }
    }

    #[test]
    fn requires_graph_has_no_cycles() {
        // Walk each skill's requirements depth-first. `stack` holds the path
        // from the starting skill; meeting our own id again means a cycle.
        fn visit(id: &str, stack: &mut Vec<&'static str>) {
            assert!(!stack.contains(&id), "cycle in requires graph");
            let skill = Skill::find(id).expect("requires id must exist");
            stack.push(skill.id);
            for required in skill.requires {
                visit(required, stack);
            }
            stack.pop();
        }
        for skill in SKILLS {
            visit(skill.id, &mut Vec::new());
        }
    }

    #[test]
    fn find_and_position_agree() {
        for (index, skill) in SKILLS.iter().enumerate() {
            assert_eq!(Skill::find(skill.id), Some(skill));
            assert_eq!(Skill::position(skill.id), Some(index));
        }
        assert_eq!(Skill::find("no-such-skill"), None);
        assert_eq!(Skill::position("no-such-skill"), None);
    }

    #[test]
    fn make_ten_means_bonds_to_ten_with_box_second() {
        use crate::generate::generate;
        use crate::look::Look;
        use crate::problem::Blank;
        use rand::SeedableRng;
        use rand::rngs::StdRng;
        let skill = Skill::find("make-ten").unwrap();
        // The chapter's own rule says bonds: `Some(10)` with a missing addend.
        assert_eq!(skill.total, Some(10));
        assert!(skill.missing_addend);
        // Every allowed sum adds to exactly 10, and only chapters 8-9 cross
        // ten when adding (checked in the next test), so these are the pairs
        // 1+9, 2+8, ... 9+1.
        for a in 1..=9 {
            for b in 1..=9 {
                assert_eq!(skill.allows(a, b), a + b == 10);
            }
        }
        // Gentle always hides the second number (`a + box = 10`).
        let mut rng = StdRng::seed_from_u64(3);
        for _ in 0..200 {
            let problem = generate(skill, Look::Gentle, &mut rng);
            assert_eq!(problem.a() + problem.b(), 10);
            assert_eq!(problem.blank(), Blank::Second);
            assert_eq!(problem.to_string(), format!("{} + □ = 10", problem.a()));
        }
    }

    #[test]
    fn only_chapters_8_and_9_cross_ten() {
        // "Crossing ten" means the chapter's rule *requires* a carry (adding)
        // or a borrow (subtracting): `Regroup::With`. Only chapters 8 (add
        // with carrying) and 9 (subtract with borrowing) ask for that. Sums
        // that merely land on exactly 10 (chapters 2 and 4) are making ten,
        // not crossing it, so they do not count.
        for skill in SKILLS {
            let late = skill.id == "add-within-20-carry" || skill.id == "subtract-within-20-borrow";
            assert_eq!(
                skill.regroup == Regroup::With,
                late,
                "{}: only chapters 8-9 may require crossing ten",
                skill.id
            );
        }
        // Spot-check the real thing: 8 + 5 carries, 13 - 5 borrows.
        let carry = Skill::find("add-within-20-carry").unwrap();
        assert!(carry.allows(8, 5));
        let borrow = Skill::find("subtract-within-20-borrow").unwrap();
        assert!(borrow.allows(13, 5));
    }

    #[test]
    fn word_ids_are_unique_and_taught_once() {
        use std::collections::{HashMap, HashSet};
        // Every id appears in exactly one chapter, so each word is taught
        // once. `HashMap` maps each id to the chapter that teaches it.
        let mut taught_in: HashMap<&str, &str> = HashMap::new();
        for skill in SKILLS {
            let mut seen_here = HashSet::new();
            for word in skill.words {
                assert!(seen_here.insert(*word), "{} lists {word} twice", skill.id);
                if let Some(first) = taught_in.insert(*word, skill.id) {
                    panic!("{word} taught in both {first} and {}", skill.id);
                }
            }
        }
        // The ladder must teach every word in the design table: 22 ids, plus `identity`.
        assert_eq!(taught_in.len(), 23, "ladder must teach 23 words");
    }
}
