//! Picks a practice problem for a skill.
//!
//! The random number generator (RNG) is *injected*: `generate` does not make
//! its own, the caller hands one in. So the function stays pure. Give it the
//! same seeded RNG and you get the same problem back, which makes tests exact.

use rand::{Rng, RngExt};

use crate::look::Look;
use crate::problem::{Blank, Problem};
use crate::skill::Skill;

/// Picks one problem that keeps `skill`'s promise (see [`Skill::allows`]).
///
/// `impl Rng` is a *trait bound*: "any type that can produce random numbers".
/// The core never says which one. The server passes a real RNG, tests pass a
/// seeded one. `&mut` is needed because drawing a number changes the RNG.
///
/// Method: list every allowed sum, then pick one uniformly. No retry loop, so
/// it always finishes, and every allowed sum is equally likely.
pub fn generate(skill: &Skill, look: Look, rng: &mut impl Rng) -> Problem {
    let candidates: Vec<(u32, u32)> = (skill.min_a..=skill.max_a)
        .flat_map(|a| (skill.min_b..=skill.max_b).map(move |b| (a, b)))
        .filter(|&(a, b)| skill.allows(a, b))
        .collect();
    let (a, b) = candidates[rng.random_range(0..candidates.len())];

    // `match` on a tuple looks at two facts at once. Only chapters with a
    // missing number hide anything but the result; Exceptional may also hide
    // the first number (`□ + 4 = 7`), flipping a coin between the two forms.
    let blank = match (skill.missing_addend, look.extra_missing_forms()) {
        (false, _) => Blank::Answer,
        (true, false) => Blank::Second,
        (true, true) if rng.random_bool(0.5) => Blank::First,
        (true, true) => Blank::Second,
    };

    // `allows` already rules out negative subtraction, so this cannot fail;
    // a test below draws thousands of problems to prove it.
    Problem::with_blank(a, skill.op, b, blank).expect("allowed sums are valid problems")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::problem::Op;
    use crate::skill::{Regroup, SKILLS};
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    #[test]
    fn same_seed_gives_same_problem() {
        let skill = &SKILLS[1];
        let first = generate(skill, Look::Gentle, &mut StdRng::seed_from_u64(7));
        let second = generate(skill, Look::Gentle, &mut StdRng::seed_from_u64(7));
        assert_eq!(first, second);
    }

    #[test]
    fn a_thousand_draws_per_skill_keep_the_chapters_promise() {
        let mut rng = StdRng::seed_from_u64(42);
        for skill in SKILLS {
            for look in [Look::Gentle, Look::Exceptional] {
                for _ in 0..1000 {
                    let p = generate(skill, look, &mut rng);
                    let (a, b) = (p.a(), p.b());
                    // Re-state the rules here, separately from `allows`, so a
                    // bug in `allows` cannot hide itself.
                    assert_eq!(p.op(), skill.op);
                    assert!(
                        (skill.min_a..=skill.max_a).contains(&a),
                        "{}: {p}",
                        skill.id
                    );
                    assert!(
                        (skill.min_b..=skill.max_b).contains(&b),
                        "{}: {p}",
                        skill.id
                    );
                    let (whole, crosses_ten) = match p.op() {
                        Op::Add => (a + b, a % 10 + b % 10 >= 10),
                        Op::Sub => (a, a % 10 < b % 10),
                    };
                    assert!(whole <= skill.max_whole, "{}: {p}", skill.id);
                    match skill.regroup {
                        Regroup::Either => {}
                        Regroup::Without => assert!(!crosses_ten, "{}: {p}", skill.id),
                        Regroup::With => assert!(crosses_ten, "{}: {p}", skill.id),
                    }
                }
            }
        }
    }

    #[test]
    fn only_exceptional_hides_the_first_number() {
        let skill = Skill::find("missing-addend-within-10").unwrap();
        let mut rng = StdRng::seed_from_u64(1);
        let gentle: Vec<Blank> = (0..200)
            .map(|_| generate(skill, Look::Gentle, &mut rng).blank())
            .collect();
        assert!(gentle.iter().all(|&blank| blank == Blank::Second));
        let exceptional: Vec<Blank> = (0..200)
            .map(|_| generate(skill, Look::Exceptional, &mut rng).blank())
            .collect();
        assert!(exceptional.contains(&Blank::First));
        assert!(exceptional.contains(&Blank::Second));
    }
}
