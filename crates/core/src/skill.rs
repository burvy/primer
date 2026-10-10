//! The skill ladder as data: one chapter per skill, in one list.
//!
//! Each skill is a plain `Skill` value (a `struct`: named fields bundled
//! together). The list is a `const`: fixed when the program is built, shared
//! everywhere, never changed while running.

use super::look::Look;
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

/// One chapter of the book, for example "make ten" or "3 + box = 7".
///
/// `id` is a short machine name like `"add-within-10"`; `title` is the
/// chapter heading the child sees. `op` is which kind of sum this chapter
/// teaches. `min_a`, `max_a`, `min_b`, `max_b` bound the two operands: the
/// generator (a later slice) picks `a` and `b` inside those ranges.
/// `missing_addend` says the box sits in the middle (`3 + box = 7`) instead
/// of at the end (`3 + 4 = box`). `stages` lists how the chapter is shown, in
/// order. `requires` lists earlier skill ids to master first. `track` is
/// always `Track::Core` in version 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Skill {
    pub id: &'static str,
    pub title: &'static str,
    pub op: Op,
    pub min_a: u32,
    pub max_a: u32,
    pub min_b: u32,
    pub max_b: u32,
    pub missing_addend: bool,
    pub stages: &'static [Stage],
    pub requires: &'static [&'static str],
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

    /// The stages this look actually uses for this skill.
    ///
    /// `&[Stage]` is a "slice": a borrowed view of a list. Exceptional skips
    /// pictorial for every skill after the first, so those children meet the
    /// number line sooner. The first skill keeps all stages so every child
    /// starts with counters.
    pub fn stages_for(&self, look: Look) -> &'static [Stage] {
        if look == Look::Exceptional
            && self.id != "count-on-within-5"
            && self.stages.first() == Some(&Stage::Pictorial)
        {
            &self.stages[1..]
        } else {
            self.stages
        }
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
        min_a: 1,
        max_a: 4,
        min_b: 1,
        max_b: 2,
        missing_addend: false,
        stages: &[Stage::Pictorial, Stage::NumberLine, Stage::Abstract],
        requires: &[],
        track: Track::Core,
    },
    Skill {
        id: "add-within-10",
        title: "Add within 10",
        op: Op::Add,
        min_a: 2,
        max_a: 9,
        min_b: 1,
        max_b: 8,
        missing_addend: false,
        stages: &[Stage::Pictorial, Stage::NumberLine, Stage::Abstract],
        requires: &["count-on-within-5"],
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
        missing_addend: true,
        stages: &[Stage::Pictorial, Stage::NumberLine, Stage::Abstract],
        requires: &["add-within-10"],
        track: Track::Core,
    },
    Skill {
        id: "make-ten",
        title: "Make ten",
        op: Op::Add,
        min_a: 5,
        max_a: 9,
        min_b: 1,
        max_b: 9,
        missing_addend: false,
        stages: &[Stage::Pictorial, Stage::NumberLine, Stage::Abstract],
        requires: &["add-within-10"],
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
        missing_addend: false,
        stages: &[Stage::Pictorial, Stage::NumberLine, Stage::Abstract],
        requires: &["add-within-10"],
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
        missing_addend: false,
        stages: &[Stage::Pictorial, Stage::NumberLine, Stage::Abstract],
        requires: &["add-within-10", "make-ten"],
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
        missing_addend: false,
        stages: &[Stage::Pictorial, Stage::NumberLine, Stage::Abstract],
        requires: &["subtract-within-10"],
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
        missing_addend: false,
        stages: &[Stage::Pictorial, Stage::NumberLine, Stage::Abstract],
        requires: &["make-ten", "add-within-20-no-carry"],
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
        missing_addend: false,
        stages: &[Stage::Pictorial, Stage::NumberLine, Stage::Abstract],
        requires: &["subtract-within-20-no-borrow", "make-ten"],
        track: Track::Core,
    },
];

#[cfg(test)]
mod tests {
    use super::*;
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
    fn exceptional_skips_pictorial_after_first_skill() {
        let first = Skill::find("count-on-within-5").unwrap();
        assert_eq!(first.stages_for(Look::Exceptional).len(), 3);
        let later = Skill::find("add-within-10").unwrap();
        assert_eq!(
            later.stages_for(Look::Exceptional),
            &[Stage::NumberLine, Stage::Abstract]
        );
        assert_eq!(later.stages_for(Look::Gentle).len(), 3);
    }
}


