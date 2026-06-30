use async_trait::async_trait;

use crate::application::matches::MatchFilters;
use crate::domain::matches::Match;
use crate::errors::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateMatchRecord {
    pub match_id: String,
    pub tournament_id: String,
    pub stage_id: String,
    pub home_team_id: Option<String>,
    pub away_team_id: Option<String>,
    pub kickoff_at: String,
    pub status: String,
    pub can_go_to_penalties: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateMatchRecord {
    pub match_id: String,
    pub stage_id: String,
    pub home_team_id: Option<String>,
    pub away_team_id: Option<String>,
    pub kickoff_at: String,
    pub status: String,
    pub can_go_to_penalties: bool,
}

#[async_trait]
pub trait MatchRepository: Send + Sync {
    async fn list(
        &self,
        tournament_id: &str,
        filters: MatchFilters,
    ) -> Result<Vec<Match>, AppError>;
    async fn find_by_id(&self, id: &str) -> Result<Option<Match>, AppError>;
    async fn create(&self, record: CreateMatchRecord) -> Result<(), AppError>;
    async fn update(&self, record: UpdateMatchRecord) -> Result<(), AppError>;
    async fn delete(&self, id: &str) -> Result<(), AppError>;

    async fn tournament_exists(&self, tournament_id: &str) -> Result<bool, AppError>;
    async fn find_stage_tournament_id(
        &self,
        stage_id: &str,
    ) -> Result<Option<String>, AppError>;
    async fn is_team_in_tournament(
        &self,
        tournament_id: &str,
        team_id: &str,
    ) -> Result<bool, AppError>;
}
