use uuid::Uuid;

use crate::application::teams::{
    CreateTeam, CreateTeamRecord, TeamRepository, UpdateTeam, UpdateTeamRecord,
};
use crate::domain::teams::Team;
use crate::domain::{NonEmptyString, TeamCode};
use crate::errors::AppError;

pub struct TeamUseCases<R> {
    teams: R,
}

impl<R> TeamUseCases<R>
where
    R: TeamRepository,
{
    pub fn new(teams: R) -> Self {
        Self { teams }
    }

    pub async fn list(&self) -> Result<Vec<Team>, AppError> {
        self.teams.list().await
    }

    pub async fn get(&self, id: &str) -> Result<Team, AppError> {
        self.teams
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound("team was not found".to_owned()))
    }

    pub async fn create(&self, command: CreateTeam) -> Result<Team, AppError> {
        let name = validate_name(command.name)?;
        let code = TeamCode::new(command.code)?.as_str().to_owned();
        let flag_emoji = validate_flag_emoji(command.flag_emoji)?;

        if self.teams.find_by_code(&code).await?.is_some() {
            return Err(AppError::conflict_code(
                "team_code_taken",
                "team code is already in use",
            ));
        }

        let team_id = new_id();
        self.teams
            .create(CreateTeamRecord {
                team_id: team_id.clone(),
                name,
                code,
                flag_emoji,
            })
            .await?;

        self.get(&team_id).await
    }

    pub async fn update(&self, id: &str, command: UpdateTeam) -> Result<Team, AppError> {
        self.get(id).await?;

        let name = validate_name(command.name)?;
        let code = TeamCode::new(command.code)?.as_str().to_owned();
        let flag_emoji = validate_flag_emoji(command.flag_emoji)?;

        if let Some(existing) = self.teams.find_by_code(&code).await? {
            if existing.id.as_str() != id {
                return Err(AppError::conflict_code(
                    "team_code_taken",
                    "team code is already in use",
                ));
            }
        }

        self.teams
            .update(UpdateTeamRecord {
                team_id: id.to_owned(),
                name,
                code,
                flag_emoji,
            })
            .await?;

        self.get(id).await
    }

    pub async fn delete(&self, id: &str) -> Result<(), AppError> {
        self.get(id).await?;

        if self.teams.is_referenced(id).await? {
            return Err(AppError::conflict_code(
                "team_in_use",
                "team is referenced by a tournament or match",
            ));
        }

        self.teams.delete(id).await
    }
}

fn validate_name(value: String) -> Result<String, AppError> {
    let name = NonEmptyString::new(value, "team.name")?.as_str().to_owned();
    if name.chars().count() > 120 {
        return Err(AppError::Validation(
            "team name must be at most 120 characters".to_owned(),
        ));
    }
    Ok(name)
}

fn validate_flag_emoji(value: Option<String>) -> Result<Option<String>, AppError> {
    match value {
        None => Ok(None),
        Some(raw) => {
            let flag_emoji = NonEmptyString::new(raw, "team.flag_emoji")?
                .as_str()
                .to_owned();
            if flag_emoji.chars().count() > 16 {
                return Err(AppError::Validation(
                    "team flag emoji must be at most 16 characters".to_owned(),
                ));
            }
            Ok(Some(flag_emoji))
        }
    }
}

fn new_id() -> String {
    Uuid::new_v4().to_string()
}
