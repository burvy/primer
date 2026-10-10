//! Problem sets: an ordered run of problems the child works through together.
//!
//! Chapters open with human-written sets (from the content files). When a
//! child has used those up, [`generate_set`] builds a new one by walking one
//! of the chapter's patterns. This is the only place randomness enters: it
//! picks the pattern and the start. Everything else is the pure [`walk`].
//!
//! [`walk`]: crate::pattern::walk

use rand::{Rng, RngExt};

use crate::generate::generate;
use crate::look::Look;
use crate::pattern::{Pattern, groups};
use crate::problem::Problem;
use crate::skill::Skill;

/// The longest generated set.
pub const SET_LEN: usize = 8;
/// The shortest run that still shows its pattern.
pub const MIN_LEN: usize = 3;

/// An ordered run of problems for one skill.
///
/// `id` names the set, so the policy can avoid serving it twice. `skill` is
/// the skill id. `pattern` is `Some` for patterned sets and written sets that
/// follow one. `notice` is the human-written "Did you see?" line shown at the
/// end; the core never writes text for children, so generated sets leave it
/// `None` and the server looks it up from content by `pattern.id()`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProblemSet {
    pub id: String,
    pub skill: &'static str,
    pub problems: Vec<Problem>,
    pub pattern: Option<Pattern>,
    pub notice: Option<String>,
}

/// Builds a patterned set for `skill`, or a plain one if no pattern fits.
///
/// Method, like slice 3's `generate`: list every run that fits, then pick one
/// uniformly. The candidates are every pattern of the skill from every start
/// `(a, b)` up to `max_whole`. `look` is only used by the fallback.
pub fn generate_set(skill: &'static Skill, look: Look, rng: &mut impl Rng) -> ProblemSet {
    let starts = (0..=skill.max_whole).flat_map(|a| (0..=skill.max_whole).map(move |b| (a, b)));
    // `flat_map` pairs each pattern with each start; `filter` keeps runs that
    // are long enough and never repeat a problem.
    let mut runs: Vec<(Pattern, Vec<Problem>)> = skill
        .patterns
        .iter()
        .flat_map(|&pattern| starts.clone().map(move |start| (pattern, start)))
        .map(|(pattern, start)| (pattern, fitting_run(skill, pattern, start)))
        .filter(|(_, run)| run.len() >= MIN_LEN && all_distinct(run))
        .collect();
    // Some patterns ignore a number (doubles ignore `b`), so neighbouring
    // starts can give the same run. `dedup` drops repeats that sit side by
    // side, so every distinct run is equally likely.
    runs.dedup();

    if runs.is_empty() {
        return ProblemSet {
            id: format!("{}:mixed", skill.id),
            skill: skill.id,
            problems: (0..SET_LEN).map(|_| generate(skill, look, rng)).collect(),
            pattern: None,
            notice: None,
        };
    }
    let (pattern, problems) = runs.swap_remove(rng.random_range(0..runs.len()));
    ProblemSet {
        id: format!("{}:{}:{}", skill.id, pattern.id(), problems[0]),
        skill: skill.id,
        problems,
        pattern: Some(pattern),
        notice: None,
    }
}

/// The pattern's run from `start`, kept while it fits the chapter.
///
/// `take_while` stops at the first group that does not fit, so the run never
/// has gaps. `scan` carries a running count (its *state*) from group to
/// group and stops before the set would pass `SET_LEN`, so a pair like
/// `3 + 5, 5 + 3` is never cut in half.
fn fitting_run(skill: &Skill, pattern: Pattern, start: (u32, u32)) -> Vec<Problem> {
    groups(pattern, start)
        .take_while(|group| group_fits(skill, group))
        .scan(0, |count, group| {
            *count += group.len();
            (*count <= SET_LEN).then_some(group)
        })
        .flatten()
        .collect()
}

/// A group fits when every problem belongs to this chapter or to one it
/// requires, and at least one belongs to this chapter.
///
/// The earlier-chapter problems are the stepping stones: `8 + 2` before
/// `8 + 5` in a bridge, or `3 + 4` before `13 + 4` in a parallel run.
fn group_fits(skill: &Skill, group: &[Problem]) -> bool {
    let earlier: Vec<&Skill> = skill
        .requires
        .iter()
        .filter_map(|id| Skill::find(id))
        .collect();
    // Stepping stones may not use zero: `0 + 4` belongs to the chapters that
    // teach zero, not to every later chapter that builds on them.
    let known = |p: &Problem| {
        fits(skill, p) || (p.a() > 0 && p.b() > 0 && earlier.iter().any(|s| fits(s, p)))
    };
    group.iter().all(known) && group.iter().any(|p| fits(skill, p))
}

/// Does this problem keep `skill`'s promise, including its kind of sum?
fn fits(skill: &Skill, p: &Problem) -> bool {
    p.op() == skill.op && skill.allows(p.a(), p.b())
}

/// True when no problem appears twice (so `4 + 4, 4 + 4` is not a swap run).
fn all_distinct(run: &[Problem]) -> bool {
    run.iter().enumerate().all(|(i, p)| !run[..i].contains(p))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skill::SKILLS;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    #[test]
    fn same_seed_gives_same_set() {
        for skill in SKILLS {
            let first = generate_set(skill, Look::Gentle, &mut StdRng::seed_from_u64(7));
            let second = generate_set(skill, Look::Gentle, &mut StdRng::seed_from_u64(7));
            assert_eq!(first, second);
        }
    }

    #[test]
    fn every_chapter_gets_a_patterned_set_that_fits() {
        let mut rng = StdRng::seed_from_u64(11);
        for skill in SKILLS {
            for _ in 0..50 {
                let set = generate_set(skill, Look::Gentle, &mut rng);
                let pattern = set.pattern.expect("every chapter has a pattern that fits");
                assert!(skill.patterns.contains(&pattern), "{}", set.id);
                assert!(
                    (MIN_LEN..=SET_LEN).contains(&set.problems.len()),
                    "{}",
                    set.id
                );
                assert!(set.problems.iter().any(|p| fits(skill, p)), "{}", set.id);
                assert!(all_distinct(&set.problems), "{}", set.id);
            }
        }
    }

    #[test]
    fn chapter_8_bridges_through_ten() {
        let skill = Skill::find("add-within-20-carry").unwrap();
        let mut rng = StdRng::seed_from_u64(5);
        let set = (0..100)
            .map(|_| generate_set(skill, Look::Gentle, &mut rng))
            .find(|set| set.pattern == Some(Pattern::Bridge))
            .expect("bridge is one of chapter 8's patterns");
        assert_eq!(set.problems[0].answer(), 10, "the first sum is 10");
    }

    #[test]
    fn zero_appears_only_in_the_chapters_that_teach_it() {
        let mut rng = StdRng::seed_from_u64(3);
        for skill in SKILLS.iter().skip(2) {
            for _ in 0..200 {
                let set = generate_set(skill, Look::Gentle, &mut rng);
                for p in &set.problems {
                    assert!(p.a() > 0 && p.b() > 0, "{}: {p}", set.id);
                }
            }
        }
    }
}
