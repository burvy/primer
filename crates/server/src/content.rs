//! Chapter content for the server.
//!
//! STUB. Slice 12 replaces this with a loader that reads the human-written
//! content files. Until then every chapter gets one lesson and one set, so the
//! whole ladder can be played. The rest of the server only calls [`chapters`].

use primer_api::BeatDto;
use primer_core::policy::{Chapter, Step};
use primer_core::{Look, SKILLS, generate_set};
use rand::SeedableRng;
use rand::rngs::StdRng;

/// One chapter per skill: a lesson called `intro`, then a stand-in "written"
/// set. The set is built from a fixed seed, so it is the same every run.
pub fn chapters() -> Vec<Chapter> {
    SKILLS
        .iter()
        .map(|skill| {
            let mut set = generate_set(skill, Look::Gentle, &mut StdRng::seed_from_u64(1));
            set.id = format!("{}:stub", skill.id);
            Chapter {
                skill: skill.id,
                steps: vec![Step::Lesson("intro".into()), Step::Set(set)],
            }
        })
        .collect()
}

/// A beat with the usual three fields; the rest default to "not used".
/// `..BeatDto::default()` fills every other field with its default value.
fn beat(text: &str, formal: Option<&str>, show: &str) -> BeatDto {
    BeatDto {
        text: text.into(),
        formal: formal.map(Into::into),
        show: show.into(),
        ..BeatDto::default()
    }
}

/// The beats of a lesson: one short sentence and one picture per tap.
///
/// STUB wording, for the human to rewrite. Only chapter 1 has a real
/// sequence; every other lesson is a single placeholder beat.
pub fn beats(skill: &str, _lesson: &str) -> Vec<BeatDto> {
    if skill != "count-on-within-5" {
        return vec![beat("A new chapter begins.", None, "equation")];
    }
    vec![
        BeatDto {
            count: Some(3),
            ..beat("Three stars shine over the gate.", None, "counters")
        },
        BeatDto {
            count: Some(3),
            adding: Some(1),
            ..beat("One more star joins them.", None, "counters")
        },
        BeatDto {
            at: Some(3),
            hop: Some(1),
            ..beat(
                "After 3 comes 4. The next number is called the successor.",
                Some("The successor of $3$ is $4$."),
                "number-line",
            )
        },
        BeatDto {
            highlight: Some("input-glow".into()),
            ..beat(
                "Here is Ada's machine. Put a number in the top.",
                Some("The machine is a function: one number in, one number out."),
                "machine",
            )
        },
        BeatDto {
            input: Some(3),
            highlight: Some("input-glow".into()),
            ..beat("Put in 3.", None, "machine")
        },
        BeatDto {
            input: Some(3),
            output: Some(4),
            highlight: Some("output-glow".into()),
            open: Some(true),
            ..beat(
                "Out comes 4. The machine always adds one.",
                Some("$f(3) = 4$, because $f(x) = x + 1$."),
                "machine",
            )
        },
        BeatDto {
            label: Some("3 + 1 = □".into()),
            ..beat("Now you try the same kind of sum.", None, "equation")
        },
    ]
}
