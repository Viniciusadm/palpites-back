use chrono::NaiveDateTime;
use uuid::Uuid;

use crate::application::matches::{
    CreateMatch, CreateMatchRecord, MatchFilters, MatchRepository, UpdateMatch, UpdateMatchRecord,
};
use crate::application::shared::Notifier;
use crate::domain::matches::{Match, MatchStatus};
use crate::domain::DomainId;
use crate::errors::AppError;

pub struct MatchUseCases<R, N> {
    matches: R,
    notifier: N,
}

struct ResolvedMatch {
    stage_id: String,
    home_team_id: Option<String>,
    away_team_id: Option<String>,
    kickoff_at: String,
}

impl<R, N> MatchUseCases<R, N>
where
    R: MatchRepository,
    N: Notifier,
{
    pub fn new(matches: R, notifier: N) -> Self {
        Self { matches, notifier }
    }

    pub async fn list(
        &self,
        tournament_id: &str,
        filters: MatchFilters,
    ) -> Result<Vec<Match>, AppError> {
        self.ensure_tournament(tournament_id).await?;
        if let Some(status) = &filters.status {
            MatchStatus::parse(status)?;
        }
        self.matches.list(tournament_id, filters).await
    }

    pub async fn get(&self, id: &str) -> Result<Match, AppError> {
        self.matches
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound("match was not found".to_owned()))
    }

    pub async fn create(
        &self,
        tournament_id: &str,
        command: CreateMatch,
    ) -> Result<Match, AppError> {
        self.ensure_tournament(tournament_id).await?;

        let resolved = self
            .validate_and_resolve(
                tournament_id,
                command.stage_id,
                command.home_team_id,
                command.away_team_id,
                command.kickoff_at,
            )
            .await?;

        let match_id = new_id();
        self.matches
            .create(CreateMatchRecord {
                match_id: match_id.clone(),
                tournament_id: tournament_id.to_owned(),
                stage_id: resolved.stage_id,
                home_team_id: resolved.home_team_id,
                away_team_id: resolved.away_team_id,
                kickoff_at: resolved.kickoff_at,
                status: MatchStatus::Scheduled.as_str().to_owned(),
            })
            .await?;

        let game = self.get(&match_id).await?;
        self.notifier.new_match(&game).await?;
        Ok(game)
    }

    pub async fn update(&self, id: &str, command: UpdateMatch) -> Result<Match, AppError> {
        let current = self.get(id).await?;
        let tournament_id = current.tournament_id.as_str().to_owned();

        let status = MatchStatus::parse(&command.status)?;

        let resolved = self
            .validate_and_resolve(
                &tournament_id,
                command.stage_id,
                command.home_team_id,
                command.away_team_id,
                command.kickoff_at,
            )
            .await?;

        self.matches
            .update(UpdateMatchRecord {
                match_id: id.to_owned(),
                stage_id: resolved.stage_id,
                home_team_id: resolved.home_team_id,
                away_team_id: resolved.away_team_id,
                kickoff_at: resolved.kickoff_at,
                status: status.as_str().to_owned(),
            })
            .await?;

        self.get(id).await
    }

    pub async fn delete(&self, id: &str) -> Result<(), AppError> {
        self.get(id).await?;
        self.matches.delete(id).await
    }

    async fn ensure_tournament(&self, tournament_id: &str) -> Result<(), AppError> {
        if !self.matches.tournament_exists(tournament_id).await? {
            return Err(AppError::NotFound("tournament was not found".to_owned()));
        }
        Ok(())
    }

    async fn validate_and_resolve(
        &self,
        tournament_id: &str,
        stage_id: String,
        home_team_id: Option<String>,
        away_team_id: Option<String>,
        kickoff_at: String,
    ) -> Result<ResolvedMatch, AppError> {
        let stage_id = validate_id(stage_id, "match.stage_id")?;
        let home_team_id = validate_opt_id(home_team_id, "match.home_team_id")?;
        let away_team_id = validate_opt_id(away_team_id, "match.away_team_id")?;
        let kickoff_at = validate_kickoff(kickoff_at)?;

        let home_domain = home_team_id
            .as_deref()
            .map(|value| DomainId::new(value.to_owned()))
            .transpose()?;
        let away_domain = away_team_id
            .as_deref()
            .map(|value| DomainId::new(value.to_owned()))
            .transpose()?;
        if Match::ensure_distinct_teams(home_domain.as_ref(), away_domain.as_ref()).is_err() {
            return Err(same_team());
        }

        match self.matches.find_stage_tournament_id(&stage_id).await? {
            None => return Err(AppError::NotFound("tournament stage was not found".to_owned())),
            Some(stage_tournament_id) if stage_tournament_id != tournament_id => {
                return Err(stage_tournament_mismatch());
            }
            Some(_) => {}
        }

        if let Some(team_id) = &home_team_id {
            if !self
                .matches
                .is_team_in_tournament(tournament_id, team_id)
                .await?
            {
                return Err(team_not_in_tournament());
            }
        }
        if let Some(team_id) = &away_team_id {
            if !self
                .matches
                .is_team_in_tournament(tournament_id, team_id)
                .await?
            {
                return Err(team_not_in_tournament());
            }
        }

        Ok(ResolvedMatch {
            stage_id,
            home_team_id,
            away_team_id,
            kickoff_at,
        })
    }
}

fn validate_id(value: String, field: &'static str) -> Result<String, AppError> {
    let trimmed = value.trim().to_owned();
    if trimmed.is_empty() {
        return Err(AppError::Validation(format!("{field} is required")));
    }
    Ok(trimmed)
}

fn validate_opt_id(
    value: Option<String>,
    field: &'static str,
) -> Result<Option<String>, AppError> {
    match value {
        None => Ok(None),
        Some(raw) => {
            let trimmed = raw.trim().to_owned();
            if trimmed.is_empty() {
                return Ok(None);
            }
            Ok(Some(validate_id(trimmed, field)?))
        }
    }
}

fn validate_kickoff(value: String) -> Result<String, AppError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::Validation("kickoff_at is required".to_owned()));
    }

    if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(trimmed) {
        return Ok(parsed.naive_utc().format("%Y-%m-%d %H:%M:%S").to_string());
    }

    let formats = [
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%dT%H:%M",
        "%Y-%m-%d %H:%M",
    ];
    for format in formats {
        if let Ok(parsed) = NaiveDateTime::parse_from_str(trimmed, format) {
            return Ok(parsed.format("%Y-%m-%d %H:%M:%S").to_string());
        }
    }

    Err(AppError::Validation(
        "kickoff_at must be a valid date-time".to_owned(),
    ))
}

fn same_team() -> AppError {
    AppError::conflict_code(
        "match_same_team",
        "home and away teams must be different",
    )
}

fn stage_tournament_mismatch() -> AppError {
    AppError::conflict_code(
        "stage_tournament_mismatch",
        "stage does not belong to this tournament",
    )
}

fn team_not_in_tournament() -> AppError {
    AppError::conflict_code(
        "team_not_in_tournament",
        "team does not participate in this tournament",
    )
}

fn new_id() -> String {
    Uuid::new_v4().to_string()
}
