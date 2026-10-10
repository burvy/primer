//! The Jev checks for one line, and the cache that makes re-runs free.
//!
//! Each line gets one request with up to four questions (fan-out): how hard
//! it is to read, whether it could upset a child, whether it gives the
//! answer away, and (for story sums) which operation the story describes.
//!
//! **Caching by content hash.** Before asking, we hash exactly what we would
//! send. If that hash is in the cache, the stored verdict is used and Jev is
//! not called. Change one word and the hash changes, so only that line is
//! asked again. The cache stores Jev's raw verdicts, not our conclusions, so
//! changing a threshold below needs no new calls.

use std::collections::BTreeMap;

use primer_server::jev::{self, JevClient, JevError, NoulCriteria, Question, Request, Response};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::entries::{Audience, Entry, Kind};

/// Part of every hash. Bump it when the questions change, so old verdicts
/// (answers to old questions) are not reused.
pub const VERSION: &str = "lint-questions-1";

/// Reading levels, lowest first. Position 0 is "age 5", 4 is "too hard".
const READING_LEVELS: &[&str] = &[
    "A 5-year-old follows it easily when it is read aloud.",
    "Right for a 6-year-old.",
    "Right for a 7-year-old.",
    "Right for an 8-year-old.",
    "Too hard for most 8-year-olds.",
];
/// Gentle lines must suit ages 5 and 6: flag anything past "age 6".
pub const GENTLE_MAX_LEVEL: f64 = 1.5;
/// Formal lines may reach age 8: flag only "too hard".
pub const EXCEPTIONAL_MAX_LEVEL: f64 = 3.5;
/// A Noul at or above this raises a flag.
pub const FLAG_AT: f64 = 0.5;

/// What Jev said about one line, before any thresholds. `None` means the
/// question was not asked, or Jev was not confident.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Verdict {
    pub reading: Option<f64>,
    pub upsetting: Option<f64>,
    pub gives_away: Option<f64>,
    pub operation: Option<String>,
}

/// Saved verdicts, by hash. A `BTreeMap` keeps the file sorted and tidy.
pub type Cache = BTreeMap<String, Verdict>;

/// Anything that can answer a request. The real one is `JevClient`; tests
/// use a fake that counts its calls. `pub(crate)`: only this program sees it.
pub(crate) trait Ask {
    async fn ask(&self, request: &Request) -> Result<Response, JevError>;
}

impl Ask for JevClient {
    async fn ask(&self, request: &Request) -> Result<Response, JevError> {
        // Calls `JevClient`'s own `ask` method (written in the server's
        // `jev.rs`), not this trait method again.
        JevClient::ask(self, request).await
    }
}

/// The verdict for `entry`: from the cache if this exact line was checked
/// before, otherwise from one Jev call (then remembered).
///
/// `?` hands a failed call back to the caller, and nothing is cached, so the
/// next run tries that line again.
pub(crate) async fn review(
    asker: &impl Ask,
    entry: &Entry,
    cache: &mut Cache,
) -> Result<Verdict, JevError> {
    let key = hash_key(entry);
    if let Some(verdict) = cache.get(&key) {
        return Ok(verdict.clone());
    }
    let reply = asker
        .ask(&Request::new(state(entry), questions(entry)))
        .await?;
    let verdict = decode(&reply);
    cache.insert(key, verdict.clone());
    Ok(verdict)
}

/// The facts Jev reads. The place name is left out on purpose: renaming an
/// id or moving a line does not change what it says, so it should not cost
/// a new call.
fn state(entry: &Entry) -> Value {
    json!({ "text": entry.text, "audience": entry.audience, "line": entry.kind })
}

/// The questions for this kind of line. Every line gets the reading level
/// and the upsetting check; only lines beside a sum get the other two.
fn questions(entry: &Entry) -> BTreeMap<&'static str, Question> {
    let mut questions = BTreeMap::from([
        (
            "reading",
            Question::Score {
                instructions: "How old must a child be to follow this line when it is read aloud to them?",
                criteria: READING_LEVELS.to_vec(),
            },
        ),
        (
            "upsetting",
            Question::Noul {
                instructions: "Could this line scare, upset or shame a young child, or is it unkind or unsafe?",
                criteria: NoulCriteria {
                    yes: "It could upset or harm a young child.",
                    no: "Kind and safe for a young child.",
                },
            },
        ),
    ]);
    if matches!(entry.kind, Kind::Question { .. } | Kind::WordProblem { .. }) {
        questions.insert(
            "gives_away",
            Question::Noul {
                instructions: "The child must work out the answers given in the state. Does the line say or \
                               strongly hint at any of those answers before the child works them out?",
                criteria: NoulCriteria {
                    yes: "The line gives an answer away.",
                    no: "The line leaves the answer for the child.",
                },
            },
        );
    }
    if let Kind::WordProblem { .. } = entry.kind {
        questions.insert(
            "operation",
            Question::Choice {
                instructions: "Which operation does the story describe?",
                criteria: vec![
                    ("unclear", "The story does not make it clear."),
                    ("add", "Things are joined or added."),
                    ("subtract", "Things are taken away or compared."),
                ],
            },
        );
    }
    questions
}

/// Jev's answers, as plain numbers and words.
fn decode(reply: &Response) -> Verdict {
    Verdict {
        reading: jev::score(reply, "reading"),
        upsetting: jev::noul(reply, "upsetting"),
        gives_away: jev::noul(reply, "gives_away"),
        operation: jev::choice(reply, "operation").map(str::to_string),
    }
}

/// What to tell the writer about `entry`, given Jev's verdict.
///
/// `is_some_and` reads "there is a value and it passes this test", so a
/// question that was not asked (or not answered confidently) never flags.
pub fn findings(entry: &Entry, verdict: &Verdict) -> Vec<String> {
    let mut out = Vec::new();
    let (max, aim) = match entry.audience {
        Audience::Gentle => (GENTLE_MAX_LEVEL, "Gentle aims at ages 5-6"),
        Audience::Exceptional => (EXCEPTIONAL_MAX_LEVEL, "Exceptional aims at ages up to 8"),
    };
    if let Some(level) = verdict.reading.filter(|&level| level > max) {
        let age = 5 + level.round() as u32;
        let reads = if age >= 9 {
            "too hard even at 8".to_string()
        } else {
            format!("reads like age {age}")
        };
        out.push(format!("reading level too high ({reads}; {aim})"));
    }
    if verdict.upsetting.is_some_and(|p| p >= FLAG_AT) {
        out.push("may upset a young child".into());
    }
    if verdict.gives_away.is_some_and(|p| p >= FLAG_AT) {
        out.push("gives the answer away".into());
    }
    if let (Kind::WordProblem { op, .. }, Some(said)) = (&entry.kind, &verdict.operation)
        && said != "unclear"
        && said != op
    {
        out.push(format!(
            "the story sounds like \"{said}\", but the chapter asks to {op}"
        ));
    }
    out
}

/// The cache key: a hash of the question version and the exact state sent.
fn hash_key(entry: &Entry) -> String {
    format!(
        "{:016x}",
        fnv1a(format!("{VERSION}\n{}", state(entry)).as_bytes())
    )
}

/// FNV-1a, a tiny hash written out in full. Rust's built-in hasher may
/// change between Rust versions, which would empty the cache for nothing;
/// this one never changes. `fold` walks the bytes, carrying the hash along:
/// mix in each byte with `^` (xor), then spread it with a multiply.
/// `wrapping_mul` lets the multiply overflow on purpose.
fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, &byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::cell::Cell;

    /// A stand-in for Jev with simple rules: "photosynthesis" is too hard,
    /// and a line that contains one of its answers gives it away. It counts
    /// its calls, so a test can prove the cache works.
    #[derive(Default)]
    pub(crate) struct FakeJev {
        pub(crate) calls: Cell<usize>,
    }

    impl Ask for FakeJev {
        async fn ask(&self, request: &Request) -> Result<Response, JevError> {
            self.calls.set(self.calls.get() + 1);
            let body = serde_json::to_value(request).unwrap();
            let text = body["state"]["text"]
                .as_str()
                .unwrap_or_default()
                .to_string();
            let line = &body["state"]["line"];
            let answers: Vec<u64> = match line["kind"].as_str() {
                Some("question") => line["answers"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter_map(Value::as_u64)
                    .collect(),
                Some("word_problem") => line["answer"].as_u64().into_iter().collect(),
                _ => Vec::new(),
            };
            let says = |n: u64| {
                text.split(|c: char| !c.is_ascii_digit())
                    .any(|w| w == n.to_string())
            };
            let hard = if text.contains("photosynthesis") {
                4.0
            } else {
                0.0
            };
            let legend: Value = (0..5)
                .map(|i| (i.to_string(), json!("level")))
                .collect::<serde_json::Map<_, _>>()
                .into();
            let mut answers_json = serde_json::Map::new();
            for id in body["questions"].as_object().unwrap().keys() {
                let answer = match id.as_str() {
                    "reading" => {
                        json!({"type": "score", "score": hard, "legend": legend, "confidence": 0.9})
                    }
                    "gives_away" => {
                        json!({"type": "noul", "noul": if answers.iter().any(|&n| says(n)) { 0.9 } else { 0.1 }})
                    }
                    "operation" => json!({"type": "choice", "choice": "add", "confidence": 0.9}),
                    _ => json!({"type": "noul", "noul": 0.05}),
                };
                answers_json.insert(id.clone(), answer);
            }
            Ok(serde_json::from_value(json!({ "answers": answers_json })).unwrap())
        }
    }

    fn line(text: &str, kind: Kind) -> Entry {
        Entry {
            place: "test".into(),
            audience: Audience::Gentle,
            text: text.into(),
            kind,
        }
    }

    #[tokio::test]
    async fn a_too_hard_line_and_an_answer_giving_question_are_flagged() {
        let (fake, mut cache) = (FakeJev::default(), Cache::new());
        let hard = line("Leaves use photosynthesis to grow.", Kind::Lesson);
        let verdict = review(&fake, &hard, &mut cache).await.unwrap();
        assert_eq!(
            findings(&hard, &verdict),
            ["reading level too high (too hard even at 8; Gentle aims at ages 5-6)"]
        );

        let kind = Kind::Question {
            sums: vec!["3 + 4".into()],
            answers: vec![7],
        };
        let telling = line("Is it 7 again?", kind.clone());
        let verdict = review(&fake, &telling, &mut cache).await.unwrap();
        assert_eq!(findings(&telling, &verdict), ["gives the answer away"]);

        let fine = line("What did you notice?", kind);
        let verdict = review(&fake, &fine, &mut cache).await.unwrap();
        assert!(findings(&fine, &verdict).is_empty());
    }

    #[tokio::test]
    async fn a_story_with_the_wrong_operation_is_flagged() {
        let story = line(
            "3 birds sit; 1 flies off.",
            Kind::WordProblem {
                sum: "3 - 1".into(),
                answer: 2,
                op: "subtract",
            },
        );
        let verdict = review(&FakeJev::default(), &story, &mut Cache::new())
            .await
            .unwrap();
        assert_eq!(
            findings(&story, &verdict),
            ["the story sounds like \"add\", but the chapter asks to subtract"]
        );
    }

    #[tokio::test]
    async fn the_same_line_is_asked_once_and_a_changed_line_again() {
        let (fake, mut cache) = (FakeJev::default(), Cache::new());
        let first = line("Seven coins.", Kind::Lesson);
        review(&fake, &first, &mut cache).await.unwrap();
        review(&fake, &first, &mut cache).await.unwrap();
        // Same words in a new place: still free.
        review(
            &fake,
            &Entry {
                place: "moved".into(),
                ..first.clone()
            },
            &mut cache,
        )
        .await
        .unwrap();
        assert_eq!(fake.calls.get(), 1);
        review(&fake, &line("Eight coins.", Kind::Lesson), &mut cache)
            .await
            .unwrap();
        assert_eq!(fake.calls.get(), 2);
    }

    #[test]
    fn the_hash_never_changes() {
        // FNV-1a's published test value for "a".
        assert_eq!(fnv1a(b"a"), 0xaf63_dc4c_8601_ec8c);
    }
}
