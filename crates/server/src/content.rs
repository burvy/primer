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
/// STUB wording, for the human to rewrite. Chapters 1 and 2 have a real
/// sequence that uses every picture; every other lesson is one placeholder beat.
pub fn beats(skill: &str, _lesson: &str) -> Vec<BeatDto> {
    match skill {
        "count-on-within-5" => chapter_1(),
        "add-within-10" => chapter_2(),
        _ => vec![beat("A new chapter begins.", None, "equation")],
    }
}

/// Chapter 1: counting on, built on a tower, then a number line and a machine.
fn chapter_1() -> Vec<BeatDto> {
    vec![
        BeatDto {
            count: Some(3),
            ..beat("Three blocks make a tower.", None, "tower")
        },
        BeatDto {
            count: Some(3),
            adding: Some(1),
            ..beat("One more block lands on top.", None, "tower")
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
            highlight: Some("answer".into()),
            ..beat("Now you try the same kind of sum.", None, "equation")
        },
    ]
}

/// Chapter 2: adding, built on two coin pouches, two machines and a written sum.
fn chapter_2() -> Vec<BeatDto> {
    let pouches = |left: u32, right: u32, text: &str, formal: Option<&str>| BeatDto {
        left: Some(left),
        right: Some(right),
        ..beat(text, formal, "pouches")
    };
    let sum = |text: &str, highlight: &str| BeatDto {
        label: Some("3 + 4 = 7".into()),
        highlight: Some(highlight.into()),
        ..beat(text, None, "equation")
    };
    vec![
        pouches(
            3,
            4,
            "Here are two pouches of coins: 3 coins and 4 coins.",
            None,
        ),
        BeatDto {
            join: Some(true),
            total: Some(7),
            ..pouches(
                3,
                4,
                "Pour them out together. Now there are 7 coins.",
                Some("Joining $3$ and $4$ gives a total of $7$, the sum."),
            )
        },
        BeatDto {
            swap: Some(true),
            ..pouches(
                3,
                4,
                "Let the pouches trade places. Still 7 coins: the order does not matter.",
                Some("Addition is commutative: $3 + 4 = 4 + 3$."),
            )
        },
        BeatDto {
            total: Some(3),
            ..pouches(
                3,
                0,
                "This pouch is empty: zero coins. Adding it changes nothing.",
                Some("Zero is the identity: $3 + 0 = 3$."),
            )
        },
        BeatDto {
            input: Some(3),
            output: Some(5),
            highlight: Some("input-glow".into()),
            count: Some(2),
            ..beat(
                "Two machines in a row: the first adds one, then the second adds one more.",
                Some("Composing $x + 1$ with $x + 1$ gives $x + 2$."),
                "machines",
            )
        },
        sum("The numbers we add are called addends.", "addends"),
        sum(
            "The equals sign means both sides are the same amount.",
            "equals",
        ),
        BeatDto {
            label: Some("3 + 4 = □".into()),
            highlight: Some("answer".into()),
            ..beat("Now you try one.", None, "equation")
        },
    ]
}
