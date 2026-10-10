//! The database edge: SQLite through `sqlx`.
//!
//! Tables (see `migrations/`): `parents` and their `sessions`; `children`
//! (each owned by a parent); and `attempts`, an append-only log. We never
//! store a `Learner`; we store what happened and fold it back into a
//! `Learner` when needed, so progress cannot drift. Deleting a parent or a
//! child deletes everything below it (`ON DELETE CASCADE`).
//!
//! `async fn` means the function can pause while the database works, letting
//! the server handle other requests meanwhile. Callers write `.await` where
//! the pause may happen. The queries are checked when they run, not when the
//! program compiles (that would need a database at build time).

use primer_api::AudioSettings;
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

/// A parent's id. A *newtype*: a number that cannot be mixed up with any
/// other number (a child's seed, a count) by accident.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParentId(pub i64);

/// The parent who signed in with this Google account id, made on the first
/// sign-in. (`ON CONFLICT DO NOTHING` then `SELECT`: two sign-ins at once
/// cannot make two parents, because `google_sub` is unique.)
pub async fn parent_for_sub(
    pool: &SqlitePool,
    google_sub: &str,
    now: i64,
) -> Result<ParentId, sqlx::Error> {
    sqlx::query("INSERT INTO parents (google_sub, created_at) VALUES (?, ?) ON CONFLICT DO NOTHING")
        .bind(google_sub)
        .bind(now)
        .execute(pool)
        .await?;
    let id = sqlx::query_scalar("SELECT id FROM parents WHERE google_sub = ?")
        .bind(google_sub)
        .fetch_one(pool)
        .await?;
    Ok(ParentId(id))
}

/// Remembers a sign-in until `expires_at` (seconds since 1970).
pub async fn create_session(
    pool: &SqlitePool,
    token: &str,
    parent: ParentId,
    expires_at: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO sessions (token, parent_id, expires_at) VALUES (?, ?, ?)")
        .bind(token)
        .bind(parent.0)
        .bind(expires_at)
        .execute(pool)
        .await?;
    Ok(())
}

/// Who this token belongs to, if the sign-in has not run out at time `now`.
pub async fn parent_of_session(
    pool: &SqlitePool,
    token: &str,
    now: i64,
) -> Result<Option<ParentId>, sqlx::Error> {
    let id = sqlx::query_scalar("SELECT parent_id FROM sessions WHERE token = ? AND expires_at > ?")
        .bind(token)
        .bind(now)
        .fetch_optional(pool)
        .await?;
    Ok(id.map(ParentId))
}

/// Signs out: forgets this one token.
pub async fn delete_session(pool: &SqlitePool, token: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM sessions WHERE token = ?")
        .bind(token)
        .execute(pool)
        .await?;
    Ok(())
}

/// Deletes the parent and, by cascade, their sessions, children and every
/// attempt of every child. Nothing is kept.
pub async fn delete_parent(pool: &SqlitePool, parent: ParentId) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM parents WHERE id = ?")
        .bind(parent.0)
        .execute(pool)
        .await?;
    Ok(())
}

/// What the server knows about a child.
pub struct Child {
    pub look: Look,
    pub seed: u64,
    /// The skill the child asked more practice on, if they did.
    pub practice_skill: Option<String>,
}

/// A child's profile: no real name, only the hero name they chose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    pub id: String,
    pub hero_name: Option<String>,
    pub age_band: String,
    pub look: Look,
}

fn profile(row: &sqlx::sqlite::SqliteRow) -> Profile {
    Profile {
        id: row.get("id"),
        hero_name: row.get("hero_name"),
        age_band: row.get("age_band"),
        look: parse(LOOKS, row.get("look")).unwrap_or(Look::Gentle),
    }
}

/// Adds a child to `parent`.
pub async fn create_child(
    pool: &SqlitePool,
    parent: ParentId,
    id: &str,
    look: Look,
    age_band: &str,
    seed: u64,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO children (id, parent_id, look, age_band, seed) VALUES (?, ?, ?, ?, ?)")
        .bind(id)
        .bind(parent.0)
        .bind(name(LOOKS, look))
        .bind(age_band)
        .bind(seed as i64)
        .execute(pool)
        .await?;
    Ok(())
}

/// The parent's children, oldest first.
pub async fn children_of(pool: &SqlitePool, parent: ParentId) -> Result<Vec<Profile>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, hero_name, age_band, look FROM children WHERE parent_id = ? ORDER BY rowid",
    )
    .bind(parent.0)
    .fetch_all(pool)
    .await?;
    Ok(rows.iter().map(profile).collect())
}

/// One child's profile, but only if `parent` owns it. A parent asking for
/// someone else's child gets `None`, the same as for a child who is not there.
pub async fn owned_child(
    pool: &SqlitePool,
    parent: ParentId,
    id: &str,
) -> Result<Option<Profile>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT id, hero_name, age_band, look FROM children WHERE id = ? AND parent_id = ?",
    )
    .bind(id)
    .bind(parent.0)
    .fetch_optional(pool)
    .await?;
    Ok(row.as_ref().map(profile))
}

/// Remembers the hero name the child chose.
pub async fn set_hero(pool: &SqlitePool, id: &str, hero_name: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE children SET hero_name = ? WHERE id = ?")
        .bind(hero_name)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Deletes the child and, by cascade, every attempt of theirs.
pub async fn delete_child(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM children WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Everything stored about a child's attempts, as JSON, oldest first: for the
/// parent's export.
pub async fn export(pool: &SqlitePool, id: &str) -> Result<Vec<serde_json::Value>, sqlx::Error> {
    let rows = sqlx::query("SELECT data FROM attempts WHERE child_id = ? ORDER BY id")
        .bind(id)
        .fetch_all(pool)
        .await?;
    rows.iter()
        .map(|row| {
            serde_json::from_str(row.get("data")).map_err(|e| sqlx::Error::Decode(Box::new(e)))
        })
        .collect()
}

/// The child with this id, or `None`. `fetch_optional` gives `Option<Row>`.
pub async fn child(pool: &SqlitePool, id: &str) -> Result<Option<Child>, sqlx::Error> {
    let row = sqlx::query("SELECT look, seed, practice_skill FROM children WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|row| Child {
        look: parse(LOOKS, row.get("look")).unwrap_or(Look::Gentle),
        seed: row.get::<i64, _>("seed") as u64,
        practice_skill: row.get("practice_skill"),
    }))
}

/// Remembers (or, with `None`, forgets) the skill the child wants more
/// practice on.
pub async fn set_practice_skill(
    pool: &SqlitePool,
    id: &str,
    skill: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE children SET practice_skill = ? WHERE id = ?")
        .bind(skill)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
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

/// What the child wants to hear. A missing child gives the defaults.
pub async fn audio(pool: &SqlitePool, id: &str) -> Result<AudioSettings, sqlx::Error> {
    let row = sqlx::query("SELECT narration, sounds, music FROM children WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    // SQLite has no true/false, so 0 and 1 are stored; `!= 0` turns one back.
    Ok(row.map_or_else(AudioSettings::default, |row| AudioSettings {
        narration: row.get::<i64, _>("narration") != 0,
        sounds: row.get::<i64, _>("sounds") != 0,
        music: row.get::<i64, _>("music") != 0,
    }))
}

/// Remembers what the child wants to hear.
pub async fn set_audio(
    pool: &SqlitePool,
    id: &str,
    settings: AudioSettings,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE children SET narration = ?, sounds = ?, music = ? WHERE id = ?")
        .bind(settings.narration)
        .bind(settings.sounds)
        .bind(settings.music)
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
