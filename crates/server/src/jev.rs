//! `JevJudge`: the real judge, backed by TypeSafe's Jev.
//!
//! One answer is one HTTP request with up to three *questions* in it (the
//! "fan-out" pattern): what kind of mistake, which strategy the child
//! described, and whether they sound frustrated. Only the questions that are
//! needed are asked; a right answer with no explanation asks nothing.
//! A hero name is a separate, smaller request with one yes/no question.
//!
//! Code does the maths: the right answer and the child's answer are worked
//! out here and sent as facts, so Jev only judges.
//!
//! Errors never reach the child. A timeout, a refused key, a busy service or
//! a reply we cannot read all end the same way: `Judgment::UNKNOWN` (and, for
//! names, "refuse it"), so the lesson carries on.
//!
//! The child's explanation goes into the request and nowhere else: nothing
//! here derives `Debug` on a request, and errors carry only a status number.

use std::collections::{BTreeMap, HashMap};

use async_trait::async_trait;
use primer_core::{MistakeKind, Problem, Strategy, detect};
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize, Serializer};
use serde_json::{Value, json};

use crate::config::JevConfig;
use crate::judge::{Judge, Judgment};
use crate::names::{MISTAKES, parse};

/// Below this, a Choice answer is treated as "don't know".
pub const MIN_CONFIDENCE: f64 = 0.6;
/// A frustration Noul at or above this counts as frustrated.
pub const FRUSTRATED_AT: f64 = 0.7;
/// A name Noul at or above this is refused. Low on purpose: when unsure, the
/// child picks from the list instead, which is always safe.
pub const IDENTIFYING_AT: f64 = 0.3;

/// The mistake options, safest first: Jev leans towards the first option,
/// so a lean should land on "unknown", which changes nothing.
const MISTAKE_OPTIONS: &[(&str, &str)] = &[
    ("unknown", "None of the others clearly fits."),
    (
        "off_by_one",
        "The answer is one more or one less than correct.",
    ),
    (
        "wrong_operation",
        "The child added instead of subtracting, or the reverse.",
    ),
    ("digits_swapped", "The right digits in the wrong order."),
    (
        "carry_or_borrow",
        "A slip when crossing ten: a forgotten carry or borrow.",
    ),
    ("guess", "The answer has no sensible link to the problem."),
];

/// The strategy options, "unclear" first for the same reason. Each word maps
/// to the core's `Strategy`.
const STRATEGIES: &[(Strategy, &str, &str)] = &[
    (
        Strategy::Unknown,
        "unclear",
        "The explanation does not say how.",
    ),
    (
        Strategy::CountAll,
        "counted_all",
        "Counted every object from one.",
    ),
    (
        Strategy::CountOn,
        "counted_on",
        "Started at one number and counted up or down.",
    ),
    (
        Strategy::MakeTen,
        "made_ten",
        "Went to ten first, then the rest.",
    ),
    (
        Strategy::Doubles,
        "doubles",
        "Used a double they know, like 6 + 6.",
    ),
    (Strategy::Recall, "just_knew", "Remembered the fact."),
];

// ---- What we send ----------------------------------------------------------

/// The request body: `{ "model", "state", "questions" }`.
#[derive(Serialize)]
struct Request {
    model: &'static str,
    state: Value,
    questions: BTreeMap<&'static str, Question>,
}

/// One question. `#[serde(tag = "type")]` makes this a *tagged enum*: the
/// variant's name is written into the JSON as a `"type"` field, beside the
/// variant's own fields. So `Question::Noul { .. }` becomes
/// `{"type": "noul", "instructions": ..., "criteria": ...}`, exactly the
/// shape Jev's docs ask for. `rename_all = "lowercase"` turns `Noul` into
/// `"noul"`.
#[derive(Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum Question {
    Noul {
        instructions: &'static str,
        criteria: NoulCriteria,
    },
    Choice {
        instructions: &'static str,
        #[serde(serialize_with = "in_order")]
        criteria: Vec<(&'static str, &'static str)>,
    },
}

/// What "true" and "false" mean for a Noul (`true` is a Rust keyword, so the
/// fields are renamed on the way out).
#[derive(Serialize)]
struct NoulCriteria {
    #[serde(rename = "true")]
    yes: &'static str,
    #[serde(rename = "false")]
    no: &'static str,
}

/// Writes `[(option, description)]` as a JSON object *in this order*. A map
/// type would sort the keys and lose "safest first".
fn in_order<S: Serializer>(
    options: &[(&'static str, &'static str)],
    out: S,
) -> Result<S::Ok, S::Error> {
    out.collect_map(options.iter().copied())
}

// ---- What comes back -------------------------------------------------------

/// The reply: one answer per question id we sent. Fields we do not use
/// (`model`, `usage`, `probabilities`) are simply ignored by serde.
#[derive(Deserialize)]
struct Response {
    answers: HashMap<String, Answer>,
}

/// The tagged enum again, now for reading: serde looks at `"type"` first and
/// then expects that variant's fields. A reply with a missing field or an
/// unknown type is a decode error, not a silently wrong answer.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum Answer {
    Noul { noul: f64 },
    Choice { choice: String, confidence: f64 },
}

/// Everything that can go wrong talking to Jev. `thiserror` writes the
/// `Display` text from each `#[error]` line, and `#[from]` lets `?` turn a
/// `reqwest` or `serde_json` error into a `JevError` on its own.
#[derive(Debug, thiserror::Error)]
pub enum JevError {
    #[error("could not reach Jev: {0}")]
    Network(#[from] reqwest::Error),
    #[error("Jev answered with status {0}")]
    Status(u16),
    #[error("could not read Jev's reply: {0}")]
    Decode(#[from] serde_json::Error),
}

pub struct JevJudge {
    config: JevConfig,
    http: reqwest::Client,
}

impl JevJudge {
    /// One HTTP client for the whole server, with the 2 s timeout built in.
    pub fn new(config: JevConfig) -> Result<JevJudge, JevError> {
        let http = reqwest::Client::builder().timeout(config.timeout).build()?;
        Ok(JevJudge { config, http })
    }

    /// Sends one request and decodes the reply.
    ///
    /// The `?` after each step means "if this failed, return the error now;
    /// otherwise carry on with the value". So the happy path reads top to
    /// bottom, and every failure leaves through the same door.
    async fn ask(&self, request: &Request) -> Result<Response, JevError> {
        let reply = self
            .http
            .post(&self.config.url)
            .bearer_auth(self.config.key.expose_secret())
            .json(request)
            .send()
            .await?;
        if !reply.status().is_success() {
            return Err(JevError::Status(reply.status().as_u16()));
        }
        let body = reply.bytes().await?;
        Ok(serde_json::from_slice(&body)?)
    }
}

#[async_trait]
impl Judge for JevJudge {
    async fn judge(
        &self,
        problem: &Problem,
        given: Option<u32>,
        explanation: Option<&str>,
    ) -> Judgment {
        let Some(request) = answer_request(problem, given, explanation) else {
            return Judgment::UNKNOWN;
        };
        match self.ask(&request).await {
            Ok(reply) => decode(&reply),
            Err(err) => {
                // Only the error, never the request: it holds the explanation.
                eprintln!("jev: {err}; judging as unknown");
                Judgment::UNKNOWN
            }
        }
    }

    async fn identifying(&self, hero_name: &str) -> bool {
        match self.ask(&name_request(hero_name)).await {
            Ok(reply) => noul(&reply, "identifying").is_none_or(|p| p >= IDENTIFYING_AT),
            Err(err) => {
                eprintln!("jev: {err}; refusing the typed name");
                true
            }
        }
    }
}

/// The fan-out request for one answer, or `None` if there is nothing to ask.
///
/// The mistake question is asked only for a wrong answer that code could not
/// name; strategy and frustration only when the child explained.
fn answer_request(
    problem: &Problem,
    given: Option<u32>,
    explanation: Option<&str>,
) -> Option<Request> {
    let explanation = explanation.map(str::trim).filter(|text| !text.is_empty());
    let unnamed_slip = given.is_some_and(|g| g != problem.answer() && detect(problem, g).is_none());
    let mut questions = BTreeMap::new();
    if unnamed_slip {
        questions.insert(
            "mistake",
            Question::Choice {
                instructions: "A child aged 5 to 8 answered this sum wrongly. Using the facts in the state \
                               (the correct answer is already worked out), which slip best explains their answer?",
                criteria: MISTAKE_OPTIONS.to_vec(),
            },
        );
    }
    if explanation.is_some() {
        questions.insert(
            "strategy",
            Question::Choice {
                instructions: "The child explained how they worked out the sum. Which strategy did they describe?",
                criteria: STRATEGIES.iter().map(|&(_, word, about)| (word, about)).collect(),
            },
        );
        questions.insert(
            "frustration",
            Question::Noul {
                instructions: "Does the child's explanation sound frustrated, upset or like they want to stop?",
                criteria: NoulCriteria {
                    yes: "Frustrated, upset, or wants to give up.",
                    no: "Calm, neutral or pleased.",
                },
            },
        );
    }
    if questions.is_empty() {
        return None;
    }
    let state = json!({
        "sum": problem.to_string(),
        "correct_answer": problem.answer(),
        "child_answer": given,
        "explanation": explanation,
    });
    Some(Request {
        model: "jev-latest",
        state,
        questions,
    })
}

fn name_request(hero_name: &str) -> Request {
    let question = Question::Noul {
        instructions: "A child typed this as the name of the hero in their story book. Could it identify a \
                       real person: a real full name, a surname, a school, a place they live, a phone number \
                       or similar? Fairy-tale and made-up names are fine.",
        criteria: NoulCriteria {
            yes: "It could identify a real person.",
            no: "A made-up or story name; it identifies no one.",
        },
    };
    Request {
        model: "jev-latest",
        state: json!({ "hero_name": hero_name }),
        questions: BTreeMap::from([("identifying", question)]),
    }
}

/// Turns a reply into a `Judgment`. Anything missing, unsure or unexpected
/// becomes that field's "don't know".
fn decode(reply: &Response) -> Judgment {
    let strategy = choice(reply, "strategy").and_then(|word| {
        STRATEGIES
            .iter()
            .find(|(_, w, _)| *w == word)
            .map(|&(strategy, _, _)| strategy)
    });
    Judgment {
        mistake: choice(reply, "mistake")
            .and_then(|word| parse(MISTAKES, word))
            .unwrap_or(MistakeKind::Unknown),
        strategy: strategy.unwrap_or(Strategy::Unknown),
        frustrated: noul(reply, "frustration").is_some_and(|p| p >= FRUSTRATED_AT),
    }
}

/// The chosen option of Choice question `id`, if Jev was confident enough.
fn choice<'a>(reply: &'a Response, id: &str) -> Option<&'a str> {
    match reply.answers.get(id)? {
        Answer::Choice { choice, confidence } if *confidence >= MIN_CONFIDENCE => Some(choice),
        _ => None,
    }
}

/// The probability from Noul question `id`.
fn noul(reply: &Response, id: &str) -> Option<f64> {
    match reply.answers.get(id)? {
        Answer::Noul { noul } => Some(*noul),
        Answer::Choice { .. } => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use primer_core::Op;
    use secrecy::SecretString;

    fn fixture(json: &str) -> Response {
        serde_json::from_str(json).expect("fixture decodes")
    }

    fn sum(a: u32, op: Op, b: u32) -> Problem {
        Problem::new(a, op, b).unwrap()
    }

    #[test]
    fn a_confident_reply_decodes_into_a_judgment() {
        let reply = fixture(include_str!("../tests/fixtures/jev_judgment.json"));
        let expected = Judgment {
            mistake: MistakeKind::CarryOrBorrow,
            strategy: Strategy::CountOn,
            frustrated: true,
        };
        assert_eq!(decode(&reply), expected);
    }

    #[test]
    fn low_confidence_means_unknown() {
        let reply = fixture(include_str!("../tests/fixtures/jev_low_confidence.json"));
        assert_eq!(decode(&reply), Judgment::UNKNOWN);
    }

    #[test]
    fn a_likely_real_name_is_refused() {
        let reply = fixture(include_str!("../tests/fixtures/jev_hero_name.json"));
        assert!(noul(&reply, "identifying").is_some_and(|p| p >= IDENTIFYING_AT));
    }

    #[test]
    fn an_unexpected_reply_is_a_decode_error() {
        let wrong = r#"{"answers": {"mistake": {"type": "choice", "choice": "guess"}}}"#;
        assert!(
            serde_json::from_str::<Response>(wrong).is_err(),
            "missing confidence"
        );
        let unknown = r#"{"answers": {"x": {"type": "poem"}}}"#;
        assert!(
            serde_json::from_str::<Response>(unknown).is_err(),
            "unknown type"
        );
    }

    #[test]
    fn the_request_has_jevs_shape_with_the_safe_option_first() {
        // 13 - 5 answered 2: no code signature, and the child explained.
        let request = answer_request(&sum(13, Op::Sub, 5), Some(2), Some(" I took away ")).unwrap();
        let body = serde_json::to_value(&request).unwrap();
        assert_eq!(body["model"], "jev-latest");
        assert_eq!(body["state"]["correct_answer"], 8);
        assert_eq!(body["state"]["explanation"], "I took away");
        assert_eq!(body["questions"]["mistake"]["type"], "choice");
        assert_eq!(body["questions"]["frustration"]["type"], "noul");
        assert_eq!(
            body["questions"]["frustration"]["criteria"]["true"],
            "Frustrated, upset, or wants to give up."
        );
        let text = serde_json::to_string(&request).unwrap();
        assert!(text.find("\"unknown\"") < text.find("\"off_by_one\""));
        assert!(text.find("\"unclear\"") < text.find("\"counted_all\""));
    }

    #[test]
    fn only_needed_questions_are_asked() {
        let problem = sum(3, Op::Add, 4);
        // Right, no explanation: no request at all.
        assert!(answer_request(&problem, Some(7), None).is_none());
        // Off by one is named by code, so Jev is not asked about it.
        assert!(answer_request(&problem, Some(8), Some("")).is_none());
        // Right, with an explanation: strategy and frustration only.
        let request = answer_request(&problem, Some(7), Some("I counted on")).unwrap();
        let ids: Vec<&str> = request.questions.keys().copied().collect();
        assert_eq!(ids, ["frustration", "strategy"]);
    }

    /// Nothing listens on port 9 here, so the request fails at once: the
    /// "any error" path, with no network and a made-up key.
    #[tokio::test]
    async fn any_error_means_unknown_and_a_refused_name() {
        let config = JevConfig {
            key: SecretString::from("not-a-real-key"),
            url: "http://127.0.0.1:9/v1/systemone".into(),
            timeout: crate::config::JEV_TIMEOUT,
        };
        let judge = JevJudge::new(config).unwrap();
        let judgment = judge
            .judge(&sum(13, Op::Sub, 5), Some(2), Some("hmm"))
            .await;
        assert_eq!(judgment, Judgment::UNKNOWN);
        assert!(judge.identifying("Sparrow").await);
    }

    /// The one live test. Ignored by default; it calls the real Jev with the
    /// key file named by `PRIMER_JEV_KEY_FILE`.
    #[tokio::test]
    #[ignore = "calls the real Jev; needs PRIMER_JEV_KEY_FILE"]
    async fn live_jev() {
        let config = JevConfig::from_env().expect("set PRIMER_JEV_KEY_FILE to the key file");
        let judge = JevJudge::new(config).unwrap();
        let judgment = judge
            .judge(
                &sum(8, Op::Add, 5),
                Some(12),
                Some("I went to ten and then I counted on 3 more. This is too hard, I hate it"),
            )
            .await;
        println!("{judgment:?}");
        assert!(!judge.identifying("Sparrow").await, "a story name is fine");
        assert!(
            judge.identifying("Emily Johnson of 12 Oak Street").await,
            "a real name and address is refused"
        );
    }
}
