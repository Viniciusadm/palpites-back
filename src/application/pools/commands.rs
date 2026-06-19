use crate::domain::pools::{Pool, PoolMember, PoolScoringRule};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatePool {
    pub name: String,
    pub tournament_id: String,
    pub join_requires_allowlist: Option<bool>,
    pub prediction_lock_offset_minutes: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdatePoolSettings {
    pub name: String,
    pub join_requires_allowlist: bool,
    pub prediction_lock_offset_minutes: u16,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinPool {
    pub invite_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeMemberRole {
    pub role: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScoringRuleInput {
    pub rule_key: String,
    pub points: i16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateScoringRules {
    pub rules: Vec<ScoringRuleInput>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolDetail {
    pub pool: Pool,
    pub member: PoolMember,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinOutcome {
    pub pool: Pool,
    pub member: PoolMember,
    pub already_member: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScoringRules {
    pub rules: Vec<PoolScoringRule>,
}
