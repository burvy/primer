//! Child profiles: what a parent can do with their children.
//!
//! Every handler here takes a [`Parent`], so none runs without a valid
//! sign-in, and every handler that names a child first checks the parent owns
//! it (`owned`). Someone else's child answers 404, exactly like a child who is
//! not there, so ids cannot be probed.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use primer_api::{ChildDto, HeroRequest, NewChild};
use primer_core::{HeroName, HeroNameError};

use crate::auth::{Parent, failed};
use crate::db::{self, Profile};
use crate::names::{AGE_BANDS, LOOKS, name, parse};
use crate::routes::AppState;

type Refusal = (StatusCode, &'static str);

/// How a hero name came to be.
#[derive(Debug, PartialEq, Eq)]
pub enum Choice {
    /// Picked from the list: already known to be fine.
    FromList(String),
    /// Typed by the child: it passed the code rule, but a name of one word of
    /// letters could still be a real person's, so the judge looks next.
    Typed(HeroName),
}

/// Sorts a given name into [`Choice`], or says why a typed one is refused.
/// A name on the list is always taken as it stands, whatever it looks like;
/// anything else is a typed name and must pass [`HeroName::parse`].
pub fn choose(list: &[String], given: &str) -> Result<Choice, HeroNameError> {
    let given = given.trim();
    if list.iter().any(|name| name == given) {
        return Ok(Choice::FromList(given.to_string()));
    }
    HeroName::parse(given).map(Choice::Typed)
}

fn dto(profile: Profile) -> ChildDto {
    ChildDto {
        id: profile.id,
        hero_name: profile.hero_name,
        age_band: profile.age_band,
        look: name(LOOKS, profile.look).to_string(),
    }
}

/// The child, if this parent owns it; otherwise 404.
pub async fn owned(state: &AppState, parent: &Parent, id: &str) -> Result<Profile, Refusal> {
    db::owned_child(&state.pool, parent.id, id)
        .await
        .map_err(failed)?
        .ok_or((StatusCode::NOT_FOUND, "no such child"))
}

/// `GET /api/children`: the parent's children.
pub async fn list(
    State(state): State<AppState>,
    parent: Parent,
) -> Result<Json<Vec<ChildDto>>, Refusal> {
    let all = db::children_of(&state.pool, parent.id).await.map_err(failed)?;
    Ok(Json(all.into_iter().map(dto).collect()))
}

/// `POST /api/children`: a new profile, with no hero name yet.
pub async fn create(
    State(state): State<AppState>,
    parent: Parent,
    Json(body): Json<NewChild>,
) -> Result<Json<ChildDto>, Refusal> {
    let look = parse(LOOKS, &body.look).ok_or((StatusCode::UNPROCESSABLE_ENTITY, "unknown look"))?;
    let band = AGE_BANDS
        .iter()
        .find(|band| **band == body.age_band)
        .ok_or((StatusCode::UNPROCESSABLE_ENTITY, "unknown age band"))?;
    let random = uuid::Uuid::new_v4();
    let id = random.simple().to_string();
    // The seed makes this child's generated sets repeatable.
    let seed = random.as_u128() as u64;
    db::create_child(&state.pool, parent.id, &id, look, band, seed)
        .await
        .map_err(failed)?;
    let profile = owned(&state, &parent, &id).await?;
    Ok(Json(dto(profile)))
}

/// `DELETE /api/children/{id}`: the child and every attempt, gone.
pub async fn delete(
    State(state): State<AppState>,
    parent: Parent,
    Path(id): Path<String>,
) -> Result<StatusCode, Refusal> {
    owned(&state, &parent, &id).await?;
    db::delete_child(&state.pool, &id).await.map_err(failed)?;
    Ok(StatusCode::NO_CONTENT)
}

/// `GET /api/children/{id}/export`: the profile and every stored attempt, as
/// one JSON document.
pub async fn export(
    State(state): State<AppState>,
    parent: Parent,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, Refusal> {
    let profile = owned(&state, &parent, &id).await?;
    let attempts = db::export(&state.pool, &id).await.map_err(failed)?;
    Ok(Json(serde_json::json!({
        "child": dto(profile),
        "attempts": attempts,
    })))
}

/// `GET /api/hero-names`: the names to pick from.
pub async fn hero_names(State(state): State<AppState>, _parent: Parent) -> Json<Vec<String>> {
    Json(state.content.hero_names.clone())
}

/// `POST /api/children/{id}/hero`: the child's hero name.
///
/// A name from the list is taken. A typed one must pass the code rule, then
/// the judge (does it look like a real person or place?). If refused, nothing
/// is saved and the page goes back to the list.
pub async fn set_hero(
    State(state): State<AppState>,
    parent: Parent,
    Path(id): Path<String>,
    Json(body): Json<HeroRequest>,
) -> Result<Json<ChildDto>, Refusal> {
    owned(&state, &parent, &id).await?;
    let chosen = match choose(&state.content.hero_names, &body.name) {
        Ok(Choice::FromList(name)) => name,
        Ok(Choice::Typed(typed)) => {
            if state.judge.identifying(typed.as_str()).await {
                return Err((
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "that name could be a real one: please pick from the list",
                ));
            }
            typed.to_string()
        }
        Err(error) => return Err((StatusCode::UNPROCESSABLE_ENTITY, error.message())),
    };
    db::set_hero(&state.pool, &id, &chosen).await.map_err(failed)?;
    let profile = owned(&state, &parent, &id).await?;
    Ok(Json(dto(profile)))
}

/// `DELETE /api/account`: the parent, their sign-ins, children and attempts.
pub async fn delete_account(
    State(state): State<AppState>,
    parent: Parent,
) -> Result<StatusCode, Refusal> {
    db::delete_parent(&state.pool, parent.id).await.map_err(failed)?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list() -> Vec<String> {
        vec!["Lovelace".to_string(), "Noether".to_string(), "al-Khwarizmi".to_string(), "Liu Hui".to_string()]
    }

    #[test]
    fn a_name_on_the_list_is_taken_as_it_stands() {
        assert_eq!(choose(&list(), "Noether"), Ok(Choice::FromList("Noether".to_string())));
        assert_eq!(choose(&list(), " Noether "), Ok(Choice::FromList("Noether".to_string())));
        // The list is the writer's: hyphens and spaces are fine there.
        assert_eq!(choose(&list(), "Liu Hui"), Ok(Choice::FromList("Liu Hui".to_string())));
        assert_eq!(choose(&list(), "al-Khwarizmi"), Ok(Choice::FromList("al-Khwarizmi".to_string())));
        // The same shapes typed (not on the list) are refused.
        assert_eq!(choose(&list(), "Liu Chen"), Err(HeroNameError::NotOneWord));
        assert_eq!(choose(&list(), "al-Biruni"), Err(HeroNameError::NotLetters));
    }

    #[test]
    fn another_name_must_pass_the_code_rule() {
        let typed = choose(&list(), "Pip").unwrap();
        assert!(matches!(typed, Choice::Typed(name) if name.as_str() == "Pip"));
        assert_eq!(choose(&list(), "Bob Smith"), Err(HeroNameError::NotOneWord));
        assert_eq!(choose(&list(), "R2D2"), Err(HeroNameError::NotLetters));
        assert_eq!(choose(&list(), "Abcdefghijklm"), Err(HeroNameError::TooLong));
    }
}
