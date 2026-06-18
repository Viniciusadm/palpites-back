use async_trait::async_trait;

use crate::domain::standings::Standing;
use crate::errors::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PredictionLine {
    pub prediction_id: String,
    pub pool_member_id: String,
    pub match_id: String,
    pub prediction_home: u8,
    pub prediction_away: u8,
    pub match_status: String,
    pub kickoff_at: String,
    pub result_home: Option<u8>,
    pub result_away: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScoredPredictionRecord {
    pub prediction_id: String,
    pub points_awarded: i16,
    pub scored_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StandingRecord {
    pub standing_id: String,
    pub pool_id: String,
    pub pool_member_id: String,
    pub total_points: i32,
    pub exact_count: i32,
    pub outcome_count: i32,
    pub hits_count: i32,
    pub position: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolRecompute {
    pub pool_id: String,
    pub scored_predictions: Vec<ScoredPredictionRecord>,
    pub standings: Vec<StandingRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultApplication {
    pub match_id: String,
    pub home_score: u8,
    pub away_score: u8,
    pub finished_at: String,
    pub pools: Vec<PoolRecompute>,
}

#[async_trait]
pub trait StandingRepository: Send + Sync {
    async fn list_for_pool(&self, pool_id: &str) -> Result<Vec<Standing>, AppError>;
    async fn prediction_lines_for_pool(
        &self,
        pool_id: &str,
    ) -> Result<Vec<PredictionLine>, AppError>;
    async fn affected_pools_for_match(&self, match_id: &str) -> Result<Vec<String>, AppError>;
    async fn apply_result(&self, application: ResultApplication) -> Result<(), AppError>;
    async fn rescore_pool(&self, recompute: PoolRecompute) -> Result<(), AppError>;
}
