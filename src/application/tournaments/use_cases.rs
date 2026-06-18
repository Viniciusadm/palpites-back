use uuid::Uuid;

use crate::application::tournaments::{
    AssignTeam, CreateAssignmentRecord, CreateGroup, CreateGroupRecord, CreateStage,
    CreateStageRecord, CreateTournament, CreateTournamentRecord, TournamentDetail,
    TournamentRepository, UpdateGroup, UpdateGroupRecord, UpdateStage, UpdateStageRecord,
    UpdateTournament, UpdateTournamentRecord,
};
use crate::domain::tournaments::{
    StageKind, Tournament, TournamentGroup, TournamentStage, TournamentStatus, TournamentTeam,
};
use crate::domain::{CalendarDate, NonEmptyString, Slug};
use crate::errors::AppError;

pub struct TournamentUseCases<R> {
    tournaments: R,
}

impl<R> TournamentUseCases<R>
where
    R: TournamentRepository,
{
    pub fn new(tournaments: R) -> Self {
        Self { tournaments }
    }

    pub async fn list(&self) -> Result<Vec<Tournament>, AppError> {
        self.tournaments.list().await
    }

    pub async fn get(&self, id: &str) -> Result<Tournament, AppError> {
        self.tournaments
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound("tournament was not found".to_owned()))
    }

    pub async fn get_detail(&self, id: &str) -> Result<TournamentDetail, AppError> {
        let tournament = self.get(id).await?;
        let stages = self.tournaments.list_stages(id).await?;
        let groups = self.tournaments.list_groups(id).await?;
        Ok(TournamentDetail {
            tournament,
            stages,
            groups,
        })
    }

    pub async fn create(&self, command: CreateTournament) -> Result<Tournament, AppError> {
        let name = validate_name(command.name)?;
        let slug = Slug::new(command.slug)?.as_str().to_owned();
        let starts_on = validate_date(command.starts_on)?;
        let ends_on = validate_date(command.ends_on)?;
        validate_date_range(starts_on.as_deref(), ends_on.as_deref())?;

        if self.tournaments.find_by_slug(&slug).await?.is_some() {
            return Err(slug_taken());
        }

        let tournament_id = new_id();
        self.tournaments
            .create(CreateTournamentRecord {
                tournament_id: tournament_id.clone(),
                name,
                slug,
                season_year: command.season_year,
                starts_on,
                ends_on,
                status: TournamentStatus::Draft.as_str().to_owned(),
            })
            .await?;

        self.get(&tournament_id).await
    }

    pub async fn update(
        &self,
        id: &str,
        command: UpdateTournament,
    ) -> Result<Tournament, AppError> {
        let current = self.get(id).await?;

        let name = validate_name(command.name)?;
        let slug = Slug::new(command.slug)?.as_str().to_owned();
        let starts_on = validate_date(command.starts_on)?;
        let ends_on = validate_date(command.ends_on)?;
        validate_date_range(starts_on.as_deref(), ends_on.as_deref())?;

        let target_status = TournamentStatus::parse(&command.status)?;
        if !current.status.can_transition_to(target_status) {
            return Err(AppError::conflict_code(
                "invalid_status_transition",
                "tournament status transition is not allowed",
            ));
        }

        if let Some(existing) = self.tournaments.find_by_slug(&slug).await? {
            if existing.id.as_str() != id {
                return Err(slug_taken());
            }
        }

        self.tournaments
            .update(UpdateTournamentRecord {
                tournament_id: id.to_owned(),
                name,
                slug,
                season_year: command.season_year,
                starts_on,
                ends_on,
                status: target_status.as_str().to_owned(),
            })
            .await?;

        self.get(id).await
    }

    pub async fn delete(&self, id: &str) -> Result<(), AppError> {
        self.get(id).await?;

        if self.tournaments.is_referenced_by_pools(id).await? {
            return Err(AppError::conflict_code(
                "tournament_in_use",
                "tournament is referenced by a pool",
            ));
        }

        self.tournaments.delete(id).await
    }

    pub async fn add_stage(
        &self,
        tournament_id: &str,
        command: CreateStage,
    ) -> Result<TournamentStage, AppError> {
        self.get(tournament_id).await?;

        let name = validate_stage_name(command.name)?;
        let kind = StageKind::parse(&command.kind)?.as_str().to_owned();

        if self
            .tournaments
            .find_stage_by_name(tournament_id, &name)
            .await?
            .is_some()
        {
            return Err(stage_name_taken());
        }

        let stage_id = new_id();
        self.tournaments
            .create_stage(CreateStageRecord {
                stage_id: stage_id.clone(),
                tournament_id: tournament_id.to_owned(),
                name,
                kind,
                ordering: command.ordering,
            })
            .await?;

        self.get_stage(&stage_id).await
    }

    pub async fn update_stage(
        &self,
        stage_id: &str,
        command: UpdateStage,
    ) -> Result<TournamentStage, AppError> {
        let current = self.get_stage(stage_id).await?;

        let name = validate_stage_name(command.name)?;
        let kind = StageKind::parse(&command.kind)?.as_str().to_owned();

        if let Some(existing) = self
            .tournaments
            .find_stage_by_name(current.tournament_id.as_str(), &name)
            .await?
        {
            if existing.id.as_str() != stage_id {
                return Err(stage_name_taken());
            }
        }

        self.tournaments
            .update_stage(UpdateStageRecord {
                stage_id: stage_id.to_owned(),
                name,
                kind,
                ordering: command.ordering,
            })
            .await?;

        self.get_stage(stage_id).await
    }

    pub async fn delete_stage(&self, stage_id: &str) -> Result<(), AppError> {
        self.get_stage(stage_id).await?;
        self.tournaments.delete_stage(stage_id).await
    }

    pub async fn add_group(
        &self,
        tournament_id: &str,
        command: CreateGroup,
    ) -> Result<TournamentGroup, AppError> {
        self.get(tournament_id).await?;

        let label = validate_group_label(command.label)?;

        if self
            .tournaments
            .find_group_by_label(tournament_id, &label)
            .await?
            .is_some()
        {
            return Err(group_label_taken());
        }

        let group_id = new_id();
        self.tournaments
            .create_group(CreateGroupRecord {
                group_id: group_id.clone(),
                tournament_id: tournament_id.to_owned(),
                label,
            })
            .await?;

        self.get_group(&group_id).await
    }

    pub async fn update_group(
        &self,
        group_id: &str,
        command: UpdateGroup,
    ) -> Result<TournamentGroup, AppError> {
        let current = self.get_group(group_id).await?;

        let label = validate_group_label(command.label)?;

        if let Some(existing) = self
            .tournaments
            .find_group_by_label(current.tournament_id.as_str(), &label)
            .await?
        {
            if existing.id.as_str() != group_id {
                return Err(group_label_taken());
            }
        }

        self.tournaments
            .update_group(UpdateGroupRecord {
                group_id: group_id.to_owned(),
                label,
            })
            .await?;

        self.get_group(group_id).await
    }

    pub async fn delete_group(&self, group_id: &str) -> Result<(), AppError> {
        self.get_group(group_id).await?;
        self.tournaments.delete_group(group_id).await
    }

    pub async fn list_teams(
        &self,
        tournament_id: &str,
    ) -> Result<Vec<TournamentTeam>, AppError> {
        self.get(tournament_id).await?;
        self.tournaments.list_teams(tournament_id).await
    }

    pub async fn assign_team(
        &self,
        tournament_id: &str,
        command: AssignTeam,
    ) -> Result<TournamentTeam, AppError> {
        self.get(tournament_id).await?;

        if !self.tournaments.team_exists(&command.team_id).await? {
            return Err(AppError::NotFound("team was not found".to_owned()));
        }

        if let Some(group_id) = &command.group_id {
            let group = self
                .tournaments
                .find_group_by_id(group_id)
                .await?
                .ok_or_else(|| AppError::NotFound("tournament group was not found".to_owned()))?;
            if group.tournament_id.as_str() != tournament_id {
                return Err(AppError::conflict_code(
                    "group_tournament_mismatch",
                    "group does not belong to this tournament",
                ));
            }
        }

        if self
            .tournaments
            .find_assignment(tournament_id, &command.team_id)
            .await?
            .is_some()
        {
            return Err(team_already_assigned());
        }

        let assignment_id = new_id();
        self.tournaments
            .create_assignment(CreateAssignmentRecord {
                assignment_id: assignment_id.clone(),
                tournament_id: tournament_id.to_owned(),
                team_id: command.team_id,
                group_id: command.group_id,
            })
            .await?;

        self.tournaments
            .find_assignment_by_id(&assignment_id)
            .await?
            .ok_or_else(|| AppError::NotFound("team assignment was not found".to_owned()))
    }

    pub async fn unassign_team(
        &self,
        tournament_id: &str,
        assignment_id: &str,
    ) -> Result<(), AppError> {
        let assignment = self
            .tournaments
            .find_assignment_by_id(assignment_id)
            .await?
            .ok_or_else(|| AppError::NotFound("team assignment was not found".to_owned()))?;

        if assignment.tournament_id.as_str() != tournament_id {
            return Err(AppError::NotFound("team assignment was not found".to_owned()));
        }

        self.tournaments.delete_assignment(assignment_id).await
    }

    async fn get_stage(&self, stage_id: &str) -> Result<TournamentStage, AppError> {
        self.tournaments
            .find_stage_by_id(stage_id)
            .await?
            .ok_or_else(|| AppError::NotFound("tournament stage was not found".to_owned()))
    }

    async fn get_group(&self, group_id: &str) -> Result<TournamentGroup, AppError> {
        self.tournaments
            .find_group_by_id(group_id)
            .await?
            .ok_or_else(|| AppError::NotFound("tournament group was not found".to_owned()))
    }
}

fn validate_name(value: String) -> Result<String, AppError> {
    let name = NonEmptyString::new(value, "tournament.name")?
        .as_str()
        .to_owned();
    if name.chars().count() > 140 {
        return Err(AppError::Validation(
            "tournament name must be at most 140 characters".to_owned(),
        ));
    }
    Ok(name)
}

fn validate_stage_name(value: String) -> Result<String, AppError> {
    let name = NonEmptyString::new(value, "stage.name")?.as_str().to_owned();
    if name.chars().count() > 60 {
        return Err(AppError::Validation(
            "stage name must be at most 60 characters".to_owned(),
        ));
    }
    Ok(name)
}

fn validate_group_label(value: String) -> Result<String, AppError> {
    let label = NonEmptyString::new(value, "group.label")?
        .as_str()
        .to_owned();
    if label.chars().count() > 8 {
        return Err(AppError::Validation(
            "group label must be at most 8 characters".to_owned(),
        ));
    }
    Ok(label)
}

fn validate_date(value: Option<String>) -> Result<Option<String>, AppError> {
    match value {
        None => Ok(None),
        Some(raw) => {
            let trimmed = raw.trim().to_owned();
            if trimmed.is_empty() {
                return Ok(None);
            }
            Ok(Some(CalendarDate::new(trimmed)?.as_str().to_owned()))
        }
    }
}

fn validate_date_range(starts_on: Option<&str>, ends_on: Option<&str>) -> Result<(), AppError> {
    if let (Some(starts_on), Some(ends_on)) = (starts_on, ends_on) {
        if ends_on < starts_on {
            return Err(AppError::Validation(
                "ends_on must be on or after starts_on".to_owned(),
            ));
        }
    }
    Ok(())
}

fn slug_taken() -> AppError {
    AppError::conflict_code("tournament_slug_taken", "tournament slug is already in use")
}

fn stage_name_taken() -> AppError {
    AppError::conflict_code("stage_name_taken", "stage name is already in use")
}

fn group_label_taken() -> AppError {
    AppError::conflict_code("group_label_taken", "group label is already in use")
}

fn team_already_assigned() -> AppError {
    AppError::conflict_code(
        "team_already_assigned",
        "team is already assigned to this tournament",
    )
}

fn new_id() -> String {
    Uuid::new_v4().to_string()
}
