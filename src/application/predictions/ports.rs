use async_trait::async_trait;

use crate::domain::predictions::Prediction;
use crate::errors::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpsertPredictionRecord {
    pub prediction_id: String,
    pub pool_member_id: String,
    pub match_id: String,
    pub home_score: u8,
    pub away_score: u8,
}

#[async_trait]
pub trait PredictionRepository: Send + Sync {
    async fn upsert(&self, record: UpsertPredictionRecord) -> Result<(), AppError>;
    async fn find_for_member_and_match(
        &self,
        pool_member_id: &str,
        match_id: &str,
    ) -> Result<Option<Prediction>, AppError>;
    async fn list_for_member(&self, pool_member_id: &str) -> Result<Vec<Prediction>, AppError>;
    async fn list_for_match(&self, match_id: &str) -> Result<Vec<Prediction>, AppError>;
}
