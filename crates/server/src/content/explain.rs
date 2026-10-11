//! `content/explain.toml`: what Ada asks after some answers, and the
//! pictures the child taps to answer.
//!
//! - "How did you work it out?" with 3 or 4 choices, each standing for a
//!   strategy (`counted_on`, ...);
//! - "How are you feeling?" with exactly the four feelings the book knows
//!   (`happy`, `ok`, `stuck`, `tired`);
//! - the "rest the pencil" line shown when the child is stuck or tired.
//!
//! Every line has a Gentle wording (`gentle`) and an Exceptional one
//! (`formal`), and is read aloud like any other line of the book. Only the
//! id of a tapped choice is ever stored.

use std::collections::HashSet;

use primer_api::{AskDto, ChoiceDto, NoteDto, QuestionDto};
use primer_core::{AskRules, Feeling, Strategy};
use serde::Deserialize;

use super::check::ContentError;
use super::convert::note;
use super::voice::{self, Line};
use crate::names::{FEELINGS, STRATEGIES, name, parse};

/// The file's name in `content/`.
pub const FILE: &str = "explain.toml";

// ---- What the file may contain --------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExplainFile {
    pub explain: QuestionEntry,
    pub feel: QuestionEntry,
    pub rest: Wording,
}

/// A question and its choices.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuestionEntry {
    pub gentle: String,
    pub formal: String,
    #[serde(default)]
    pub choice: Vec<ChoiceEntry>,
}

/// One choice. `strategy` is only for "How did you work it out?".
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChoiceEntry {
    pub id: String,
    pub picture: String,
    pub gentle: String,
    pub formal: String,
    pub strategy: Option<String>,
}

/// A line with its two wordings.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Wording {
    pub gentle: String,
    pub formal: String,
}

// ---- What the server keeps --------------------------------------------------

/// The checked file, ready to send.
#[derive(Debug, Clone)]
pub struct Explain {
    pub explain: QuestionDto,
    pub feel: QuestionDto,
    /// "Let's rest the pencil."
    pub rest: NoteDto,
    /// Each explanation choice: its id and the strategy it stands for.
    strategies: Vec<(String, Strategy)>,
}

impl Explain {
    /// What the page needs to ask after an answer under `rules`.
    pub fn ask(&self, rules: AskRules, slow_ms: u32) -> AskDto {
        AskDto {
            first_right_in_set: rules.first_right_in_set,
            slow_before: rules.slow_before,
            slow_ms,
            explain: self.explain.clone(),
            feel: self.feel.clone(),
        }
    }

    /// The strategy explanation choice `id` stands for. `None` for an id
    /// the book does not have.
    pub fn explanation(&self, id: &str) -> Option<Strategy> {
        self.strategies
            .iter()
            .find(|(choice, _)| choice == id)
            .map(|(_, strategy)| *strategy)
    }

    /// Every line Ada reads aloud from this file, for the recordings list.
    pub fn lines(&self) -> Vec<Line> {
        let questions = [&self.explain, &self.feel];
        questions
            .iter()
            .flat_map(|q| {
                let choices = q.choices.iter().map(|c| &c.note);
                std::iter::once(&q.question).chain(choices)
            })
            .chain(std::iter::once(&self.rest))
            .flat_map(|n| voice::lines_of(&n.voice))
            .collect()
    }
}

/// The feeling with id `id`, if there is one.
pub fn feeling(id: &str) -> Option<Feeling> {
    parse(FEELINGS, id)
}

// ---- Checking and building ---------------------------------------------------

/// Checks the file and builds what the server keeps. Every problem is
/// reported, not only the first.
pub fn build(file: &ExplainFile) -> Result<Explain, Vec<ContentError>> {
    let mut errors = Vec::new();
    let mut fail = |id: &str, message: String| errors.push(ContentError::new(FILE, id, message));

    let explain_count = file.explain.choice.len();
    if !(3..=4).contains(&explain_count) {
        fail(
            "explain",
            format!("needs 3 or 4 choices, not {explain_count}"),
        );
    }
    let mut strategies = Vec::new();
    for choice in &file.explain.choice {
        match choice
            .strategy
            .as_deref()
            .map(|word| parse(STRATEGIES, word))
        {
            Some(Some(strategy)) => strategies.push((choice.id.clone(), strategy)),
            Some(None) => fail(
                &choice.id,
                format!(
                    "no strategy called `{}`",
                    choice.strategy.as_deref().unwrap_or("")
                ),
            ),
            None => fail(&choice.id, "needs a `strategy`".to_string()),
        }
    }
    let wanted: Vec<&str> = Feeling::ALL.iter().map(|f| name(FEELINGS, *f)).collect();
    let found: Vec<&str> = file.feel.choice.iter().map(|c| c.id.as_str()).collect();
    if found != wanted {
        fail(
            "feel",
            format!(
                "needs exactly these choices, in this order: {}",
                wanted.join(", ")
            ),
        );
    }
    for choice in &file.feel.choice {
        if choice.strategy.is_some() {
            fail(&choice.id, "a feeling has no `strategy`".to_string());
        }
    }
    let mut seen = HashSet::new();
    for choice in file.explain.choice.iter().chain(&file.feel.choice) {
        if !seen.insert(choice.id.as_str()) {
            fail(&choice.id, "this id is used twice".to_string());
        }
        if choice.picture.trim().is_empty() {
            fail(&choice.id, "needs a `picture`".to_string());
        }
    }

    // The lines themselves: each wording must read as text (and maths).
    let mut line = |id: String, gentle: &str, formal: &str| {
        note(&id, gentle, Some(formal)).unwrap_or_else(|problems| {
            for problem in problems {
                errors.push(ContentError::new(FILE, &id, problem));
            }
            NoteDto::default()
        })
    };
    let mut question = |part: &str, entry: &QuestionEntry| QuestionDto {
        question: line(format!("{part}.question"), &entry.gentle, &entry.formal),
        choices: entry
            .choice
            .iter()
            .map(|c| ChoiceDto {
                id: c.id.clone(),
                picture: c.picture.clone(),
                note: line(format!("{part}.{}", c.id), &c.gentle, &c.formal),
            })
            .collect(),
    };
    let explain = question("explain", &file.explain);
    let feel = question("feel", &file.feel);
    let rest = line("rest".to_string(), &file.rest.gentle, &file.rest.formal);

    if errors.is_empty() {
        Ok(Explain {
            explain,
            feel,
            rest,
            strategies,
        })
    } else {
        Err(errors)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A whole, valid file; tests change one part of it.
    pub const GOOD: &str = r#"
[explain]
gentle = "How did you work it out?"
formal = "Which way did you use?"

[[explain.choice]]
id = "counted_all"
picture = "A"
gentle = "I counted them all"
formal = "I counted all of them"
strategy = "counted_all"

[[explain.choice]]
id = "counted_on"
picture = "B"
gentle = "I counted on"
formal = "I counted on from the bigger one"
strategy = "counted_on"

[[explain.choice]]
id = "just_knew"
picture = "C"
gentle = "I just knew"
formal = "I remembered it"
strategy = "just_knew"

[feel]
gentle = "How are you feeling?"
formal = "How do you feel?"

[[feel.choice]]
id = "happy"
picture = "1"
gentle = "Happy"
formal = "Happy"

[[feel.choice]]
id = "ok"
picture = "2"
gentle = "OK"
formal = "OK"

[[feel.choice]]
id = "stuck"
picture = "3"
gentle = "Stuck"
formal = "Stuck"

[[feel.choice]]
id = "tired"
picture = "4"
gentle = "Tired"
formal = "Tired"

[rest]
gentle = "Let's rest the pencil."
formal = "Let's rest the pencil for a moment."
"#;

    fn built(text: &str) -> Result<Explain, Vec<String>> {
        let file: ExplainFile = toml::from_str(text).map_err(|e| vec![e.to_string()])?;
        build(&file).map_err(|errors| errors.iter().map(|e| e.to_string()).collect())
    }

    #[test]
    fn a_good_file_gives_questions_voices_and_strategies() {
        let explain = built(GOOD).unwrap();
        assert_eq!(explain.explain.choices.len(), 3);
        assert_eq!(explain.feel.choices[3].id, "tired");
        assert_eq!(
            explain.explain.choices[1].note.voice.gentle.name,
            "explain.counted_on.gentle"
        );
        assert_eq!(
            explain.rest.voice.exceptional.as_ref().unwrap().name,
            "rest.exceptional"
        );
        assert_eq!(explain.explanation("counted_on"), Some(Strategy::CountOn));
        assert_eq!(explain.explanation("typed words"), None);
        // Two questions with 3 + 4 choices and the rest line, in two looks.
        assert_eq!(explain.lines().len(), (2 + 3 + 4 + 1) * 2);
    }

    #[test]
    fn mistakes_in_the_file_are_all_reported() {
        let text = GOOD
            .replacen("strategy = \"counted_on\"", "strategy = \"counted_up\"", 1)
            .replacen("id = \"tired\"", "id = \"sleepy\"", 1)
            .replacen("picture = \"C\"", "picture = \" \"", 1);
        let errors = built(&text).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.contains("no strategy called `counted_up`")),
            "{errors:?}"
        );
        assert!(
            errors
                .iter()
                .any(|e| e.contains("needs exactly these choices")),
            "{errors:?}"
        );
        assert!(
            errors
                .iter()
                .any(|e| e.contains("just_knew: needs a `picture`")),
            "{errors:?}"
        );
    }

    #[test]
    fn three_or_four_ways_to_explain() {
        let two = GOOD.replacen(
            "[[explain.choice]]\nid = \"just_knew\"",
            "[[nothing]]\nid = \"x\"",
            1,
        );
        // Cutting a choice out leaves 2, which is too few.
        let file = two.split("[[nothing]]").next().unwrap().to_string()
            + &GOOD[GOOD.find("[feel]").unwrap()..];
        let errors = built(&file).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.contains("needs 3 or 4 choices, not 2")),
            "{errors:?}"
        );
    }

    #[test]
    fn feelings_are_read_by_id() {
        assert_eq!(feeling("tired"), Some(Feeling::Tired));
        assert_eq!(feeling("grumpy"), None);
    }
}
