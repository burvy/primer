//! The database edge: SQLite through `sqlx`.
//!
//! Two tables (see `migrations/0001_init.sql`): `children`, and `attempts`, an
//! append-only log. We never store a `Learner`; we store what happened and
//! fold it back into a `Learner` when needed, so progress cannot drift.
//!
//! `async fn` means the function can pause while the database works, letting
//! the server handle other requests meanwhile. Callers write `.await` where
//! the pause may happen. The queries are checked when they run, not when the
//! program compiles (that would need a database at build time).

use primer_core::{Attempt, Event, Look, Outcome, Problem, Skill, Strategy};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};

use crate::names::{BLANKS, LOOKS, MISTAKES, OPS, STAGES, name, parse};

/// Opens (creating if needed) the database at `url` and brings its tables up
/// to date with the files in `migrations/`.
pub async fn connect(url: &str) -> Result<SqlitePool, sqlx::Error> {
    let options = url.parse::<SqliteConnectOptions>()?.create_if_missing(true);
    let pool = SqlitePoolOptions::new().connect_with(options).await?;
    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .map_err(|e| sqlx::Error::Migrate(Box::new(e)))?;
    Ok(pool)
}

/// What the server knows about a child.
pub struct Child {
    pub look: Look,
    pub seed: u64,
}

/// Adds a child unless one with this id already exists (`OR IGNORE`).
pub async fn ensure_child(
    pool: &SqlitePool,
    id: &str,
    look: Look,
    seed: u64,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT OR IGNORE INTO children (id, look, seed) VALUES (?, ?, ?)")
        .bind(id)
        .bind(name(LOOKS, look))
        .bind(seed as i64)
        .execute(pool)
        .await?;
    Ok(())
}

/// The child with this id, or `None`. `fetch_optional` gives `Option<Row>`.
pub async fn child(pool: &SqlitePool, id: &str) -> Result<Option<Child>, sqlx::Error> {
    let row = sqlx::query("SELECT look, seed FROM children WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|row| Child {
        look: parse(LOOKS, row.get("look")).unwrap_or(Look::Gentle),
        seed: row.get::<i64, _>("seed") as u64,
    }))
}

/// Changes the child's look. Their progress is untouched: it is the same log.
pub async fn set_look(pool: &SqlitePool, id: &str, look: Look) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE children SET look = ? WHERE id = ?")
        .bind(name(LOOKS, look))
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Every event for the child, oldest first.
pub async fn events(pool: &SqlitePool, child_id: &str) -> Result<Vec<Event>, sqlx::Error> {
    let rows = sqlx::query("SELECT data FROM attempts WHERE child_id = ? ORDER BY id")
        .bind(child_id)
        .fetch_all(pool)
        .await?;
    rows.iter()
        .map(|row| {
            let stored: Stored = serde_json::from_str(row.get("data"))
                .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
            decode(stored).ok_or_else(|| sqlx::Error::Decode("unknown name in stored event".into()))
        })
        .collect()
}

/// Adds one event to the end of the child's log.
pub async fn append(pool: &SqlitePool, child_id: &str, event: &Event) -> Result<(), sqlx::Error> {
    let stored = encode(event);
    let kind = match stored {
        Stored::Lesson { .. } => "lesson",
        Stored::Attempt { .. } => "attempt",
    };
    let data = serde_json::to_string(&stored).map_err(|e| sqlx::Error::Encode(Box::new(e)))?;
    sqlx::query("INSERT INTO attempts (child_id, kind, data) VALUES (?, ?, ?)")
        .bind(child_id)
        .bind(kind)
        .bind(data)
        .execute(pool)
        .await?;
    Ok(())
}

/// The JSON form of an event. The core's types hold `&'static str` skill ids
/// and plain enums, which JSON cannot hold directly, so we store words and
/// rebuild the real values on the way out.
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Stored {
    Lesson {
        skill: String,
        lesson: String,
    },
    Attempt {
        skill: String,
        set_id: String,
        a: u32,
        op: String,
        b: u32,
        blank: String,
        stage: String,
        look: String,
        answer: Option<u32>,
        /// `"correct"`, `"dont_know"` or `"wrong"` (then see `mistake`).
        outcome: String,
        mistake: Option<String>,
        ms_taken: u32,
    },
}

fn encode(event: &Event) -> Stored {
    match event {
        Event::LessonSeen { skill, lesson } => Stored::Lesson {
            skill: skill.to_string(),
            lesson: lesson.clone(),
        },
        Event::Attempt(at) => {
            let (outcome, mistake) = match at.outcome {
                Outcome::Correct => ("correct", None),
                Outcome::DontKnow => ("dont_know", None),
                Outcome::Wrong(kind) => ("wrong", Some(name(MISTAKES, kind).to_string())),
            };
            Stored::Attempt {
                skill: at.skill.to_string(),
                set_id: at.set_id.clone(),
                a: at.problem.a(),
                op: name(OPS, at.problem.op()).to_string(),
                b: at.problem.b(),
                blank: name(BLANKS, at.problem.blank()).to_string(),
                stage: name(STAGES, at.stage).to_string(),
                look: name(LOOKS, at.look).to_string(),
                answer: at.answer,
                outcome: outcome.to_string(),
                mistake,
                ms_taken: at.ms_taken,
            }
        }
    }
}

/// `None` if any word is not recognised. The `?` operator returns `None`
/// early from the function as soon as one step gives `None`.
fn decode(stored: Stored) -> Option<Event> {
    match stored {
        Stored::Lesson { skill, lesson } => Some(Event::LessonSeen {
            skill: skill_id(&skill)?,
            lesson,
        }),
        Stored::Attempt {
            skill,
            set_id,
            a,
            op,
            b,
            blank,
            stage,
            look,
            answer,
            outcome,
            mistake,
            ms_taken,
        } => {
            let outcome = match outcome.as_str() {
                "correct" => Outcome::Correct,
                "dont_know" => Outcome::DontKnow,
                _ => Outcome::Wrong(parse(MISTAKES, &mistake?)?),
            };
            Some(Event::Attempt(Attempt {
                skill: skill_id(&skill)?,
                set_id,
                problem: Problem::with_blank(a, parse(OPS, &op)?, b, parse(BLANKS, &blank)?)?,
                stage: parse(STAGES, &stage)?,
                look: parse(LOOKS, &look)?,
                answer,
                outcome,
                ms_taken,
                // Not collected yet (slice 11 adds strategy and frustration).
                strategy: Strategy::Unknown,
                frustrated: false,
            }))
        }
    }
}

/// The core keeps skill ids as `&'static str`; look the stored one up to get
/// the long-lived copy.
fn skill_id(id: &str) -> Option<&'static str> {
    Skill::find(id).map(|skill| skill.id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use primer_core::{Blank, MistakeKind, Op, SKILLS, Stage};

    #[test]
    fn events_survive_a_round_trip_through_json() {
        let attempt = Event::Attempt(Attempt {
            skill: SKILLS[2].id,
            set_id: "set".into(),
            problem: Problem::with_blank(3, Op::Add, 4, Blank::Second).unwrap(),
            stage: Stage::NumberLine,
            look: Look::Exceptional,
            answer: Some(5),
            outcome: Outcome::Wrong(MistakeKind::OffByOne),
            ms_taken: 1234,
            strategy: Strategy::Unknown,
            frustrated: false,
        });
        let lesson = Event::LessonSeen {
            skill: SKILLS[0].id,
            lesson: "intro".into(),
        };
        for event in [attempt, lesson] {
            let json = serde_json::to_string(&encode(&event)).unwrap();
            let back = decode(serde_json::from_str(&json).unwrap()).unwrap();
            assert_eq!(back, event);
        }
    }
}
