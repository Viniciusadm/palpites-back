use async_trait::async_trait;

use crate::domain::teams::Team;
use crate::errors::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateTeamRecord {
    pub team_id: String,
    pub name: String,
    pub code: String,
    pub flag_emoji: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateTeamRecord {
    pub team_id: String,
    pub name: String,
    pub code: String,
    pub flag_emoji: Option<String>,
}

#[async_trait]
pub trait TeamRepository: Send + Sync {
    async fn list(&self) -> Result<Vec<Team>, AppError>;
    async fn find_by_id(&self, id: &str) -> Result<Option<Team>, AppError>;
    async fn find_by_code(&self, code: &str) -> Result<Option<Team>, AppError>;
    async fn create(&self, record: CreateTeamRecord) -> Result<(), AppError>;
    async fn update(&self, record: UpdateTeamRecord) -> Result<(), AppError>;
    async fn delete(&self, id: &str) -> Result<(), AppError>;
    async fn is_referenced(&self, id: &str) -> Result<bool, AppError>;
}
