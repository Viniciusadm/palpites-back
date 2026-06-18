use async_trait::async_trait;

use crate::domain::pools::{Pool, PoolMember, PoolScoringRule};
use crate::errors::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolMemberWithName {
    pub member: PoolMember,
    pub display_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewPoolRecord {
    pub pool_id: String,
    pub tournament_id: String,
    pub owner_user_id: String,
    pub name: String,
    pub invite_code: String,
    pub visibility: String,
    pub ranking_public: bool,
    pub prediction_lock_offset_minutes: u16,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdatePoolRecord {
    pub pool_id: String,
    pub name: String,
    pub visibility: String,
    pub ranking_public: bool,
    pub prediction_lock_offset_minutes: u16,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewMemberRecord {
    pub member_id: String,
    pub pool_id: String,
    pub user_id: String,
    pub role: String,
    pub status: String,
    pub joined_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewScoringRuleRecord {
    pub rule_id: String,
    pub rule_key: String,
    pub points: i16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatePoolSeed {
    pub pool: NewPoolRecord,
    pub scoring_rules: Vec<NewScoringRuleRecord>,
    pub owner_member: NewMemberRecord,
}

#[async_trait]
pub trait PoolRepository: Send + Sync {
    async fn find_by_id(&self, id: &str) -> Result<Option<Pool>, AppError>;
    async fn find_by_invite_code(&self, invite_code: &str) -> Result<Option<Pool>, AppError>;
    async fn list_for_user(&self, user_id: &str) -> Result<Vec<Pool>, AppError>;
    async fn list_for_tournament(&self, tournament_id: &str) -> Result<Vec<Pool>, AppError>;
    async fn find_tournament_status(
        &self,
        tournament_id: &str,
    ) -> Result<Option<String>, AppError>;
    async fn create_with_seed(&self, seed: CreatePoolSeed) -> Result<(), AppError>;
    async fn update_settings(&self, record: UpdatePoolRecord) -> Result<(), AppError>;
    async fn delete(&self, id: &str) -> Result<(), AppError>;
}

#[async_trait]
pub trait PoolMemberRepository: Send + Sync {
    async fn find_membership(
        &self,
        pool_id: &str,
        user_id: &str,
    ) -> Result<Option<PoolMember>, AppError>;
    async fn find_by_id(&self, member_id: &str) -> Result<Option<PoolMember>, AppError>;
    async fn list_for_pool(&self, pool_id: &str) -> Result<Vec<PoolMember>, AppError>;
    async fn list_for_pool_with_names(
        &self,
        pool_id: &str,
    ) -> Result<Vec<PoolMemberWithName>, AppError>;
    async fn count_active_owners(&self, pool_id: &str) -> Result<u64, AppError>;
    async fn create(&self, record: NewMemberRecord) -> Result<(), AppError>;
    async fn reactivate(&self, member_id: &str, joined_at: &str) -> Result<(), AppError>;
    async fn set_status(
        &self,
        member_id: &str,
        status: &str,
        left_at: Option<&str>,
    ) -> Result<(), AppError>;
    async fn set_role(&self, member_id: &str, role: &str) -> Result<(), AppError>;
}

#[async_trait]
pub trait ScoringRuleRepository: Send + Sync {
    async fn list_for_pool(&self, pool_id: &str) -> Result<Vec<PoolScoringRule>, AppError>;
    async fn set_rules(
        &self,
        pool_id: &str,
        rules: Vec<NewScoringRuleRecord>,
    ) -> Result<(), AppError>;
}
