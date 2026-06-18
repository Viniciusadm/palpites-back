use async_trait::async_trait;

use crate::domain::tournaments::{Tournament, TournamentGroup, TournamentStage, TournamentTeam};
use crate::errors::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateTournamentRecord {
    pub tournament_id: String,
    pub name: String,
    pub slug: String,
    pub season_year: Option<u16>,
    pub starts_on: Option<String>,
    pub ends_on: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateTournamentRecord {
    pub tournament_id: String,
    pub name: String,
    pub slug: String,
    pub season_year: Option<u16>,
    pub starts_on: Option<String>,
    pub ends_on: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateStageRecord {
    pub stage_id: String,
    pub tournament_id: String,
    pub name: String,
    pub kind: String,
    pub ordering: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateStageRecord {
    pub stage_id: String,
    pub name: String,
    pub kind: String,
    pub ordering: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateGroupRecord {
    pub group_id: String,
    pub tournament_id: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateGroupRecord {
    pub group_id: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateAssignmentRecord {
    pub assignment_id: String,
    pub tournament_id: String,
    pub team_id: String,
    pub group_id: Option<String>,
}

#[async_trait]
pub trait TournamentRepository: Send + Sync {
    async fn list(&self) -> Result<Vec<Tournament>, AppError>;
    async fn find_by_id(&self, id: &str) -> Result<Option<Tournament>, AppError>;
    async fn find_by_slug(&self, slug: &str) -> Result<Option<Tournament>, AppError>;
    async fn create(&self, record: CreateTournamentRecord) -> Result<(), AppError>;
    async fn update(&self, record: UpdateTournamentRecord) -> Result<(), AppError>;
    async fn delete(&self, id: &str) -> Result<(), AppError>;
    async fn is_referenced_by_pools(&self, id: &str) -> Result<bool, AppError>;

    async fn list_stages(&self, tournament_id: &str) -> Result<Vec<TournamentStage>, AppError>;
    async fn find_stage_by_id(&self, stage_id: &str)
        -> Result<Option<TournamentStage>, AppError>;
    async fn find_stage_by_name(
        &self,
        tournament_id: &str,
        name: &str,
    ) -> Result<Option<TournamentStage>, AppError>;
    async fn create_stage(&self, record: CreateStageRecord) -> Result<(), AppError>;
    async fn update_stage(&self, record: UpdateStageRecord) -> Result<(), AppError>;
    async fn delete_stage(&self, stage_id: &str) -> Result<(), AppError>;

    async fn list_groups(&self, tournament_id: &str) -> Result<Vec<TournamentGroup>, AppError>;
    async fn find_group_by_id(&self, group_id: &str)
        -> Result<Option<TournamentGroup>, AppError>;
    async fn find_group_by_label(
        &self,
        tournament_id: &str,
        label: &str,
    ) -> Result<Option<TournamentGroup>, AppError>;
    async fn create_group(&self, record: CreateGroupRecord) -> Result<(), AppError>;
    async fn update_group(&self, record: UpdateGroupRecord) -> Result<(), AppError>;
    async fn delete_group(&self, group_id: &str) -> Result<(), AppError>;

    async fn list_teams(&self, tournament_id: &str) -> Result<Vec<TournamentTeam>, AppError>;
    async fn find_assignment_by_id(
        &self,
        assignment_id: &str,
    ) -> Result<Option<TournamentTeam>, AppError>;
    async fn find_assignment(
        &self,
        tournament_id: &str,
        team_id: &str,
    ) -> Result<Option<TournamentTeam>, AppError>;
    async fn create_assignment(&self, record: CreateAssignmentRecord) -> Result<(), AppError>;
    async fn delete_assignment(&self, assignment_id: &str) -> Result<(), AppError>;
    async fn team_exists(&self, team_id: &str) -> Result<bool, AppError>;
}
