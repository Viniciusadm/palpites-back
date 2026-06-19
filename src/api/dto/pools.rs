use serde::{Deserialize, Serialize};

use crate::application::pools::{
    ChangeMemberRole, CreatePool, JoinOutcome, JoinPool, PoolMemberWithName, ScoringRuleInput,
    UpdatePoolSettings, UpdateScoringRules,
};
use crate::domain::pools::{Pool, PoolAllowedEmail, PoolMember, PoolScoringRule};

#[derive(Debug, Deserialize)]
pub struct CreatePoolRequest {
    pub name: String,
    pub tournament_id: String,
    pub join_requires_allowlist: Option<bool>,
    pub prediction_lock_offset_minutes: Option<u16>,
}

impl From<CreatePoolRequest> for CreatePool {
    fn from(value: CreatePoolRequest) -> Self {
        Self {
            name: value.name,
            tournament_id: value.tournament_id,
            join_requires_allowlist: value.join_requires_allowlist,
            prediction_lock_offset_minutes: value.prediction_lock_offset_minutes,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdatePoolRequest {
    pub name: String,
    pub join_requires_allowlist: bool,
    pub prediction_lock_offset_minutes: u16,
    pub status: String,
}

impl From<UpdatePoolRequest> for UpdatePoolSettings {
    fn from(value: UpdatePoolRequest) -> Self {
        Self {
            name: value.name,
            join_requires_allowlist: value.join_requires_allowlist,
            prediction_lock_offset_minutes: value.prediction_lock_offset_minutes,
            status: value.status,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct AddAllowedEmailRequest {
    pub email: String,
}

#[derive(Debug, Serialize)]
pub struct AllowedEmailResponse {
    pub id: String,
    pub email: String,
    pub created_at: String,
}

impl AllowedEmailResponse {
    pub fn from_entry(entry: &PoolAllowedEmail) -> Self {
        Self {
            id: entry.id.as_str().to_owned(),
            email: entry.email.as_str().to_owned(),
            created_at: entry.created_at.as_str().to_owned(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AllowedEmailsListResponse {
    pub emails: Vec<AllowedEmailResponse>,
}

#[derive(Debug, Deserialize)]
pub struct DeletePoolRequest {
    pub confirm_name: String,
}

#[derive(Debug, Deserialize)]
pub struct JoinPoolRequest {
    pub invite_code: String,
}

impl From<JoinPoolRequest> for JoinPool {
    fn from(value: JoinPoolRequest) -> Self {
        Self {
            invite_code: value.invite_code,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ChangeMemberRoleRequest {
    pub role: String,
}

impl From<ChangeMemberRoleRequest> for ChangeMemberRole {
    fn from(value: ChangeMemberRoleRequest) -> Self {
        Self { role: value.role }
    }
}

#[derive(Debug, Deserialize)]
pub struct ScoringRuleInputRequest {
    pub rule_key: String,
    pub points: i16,
}

#[derive(Debug, Deserialize)]
pub struct UpdateScoringRulesRequest {
    pub rules: Vec<ScoringRuleInputRequest>,
}

impl From<UpdateScoringRulesRequest> for UpdateScoringRules {
    fn from(value: UpdateScoringRulesRequest) -> Self {
        Self {
            rules: value
                .rules
                .into_iter()
                .map(|rule| ScoringRuleInput {
                    rule_key: rule.rule_key,
                    points: rule.points,
                })
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PoolResponse {
    pub id: String,
    pub tournament_id: String,
    pub owner_user_id: String,
    pub name: String,
    pub invite_code: String,
    pub join_requires_allowlist: bool,
    pub prediction_lock_offset_minutes: u16,
    pub status: String,
}

impl PoolResponse {
    pub fn from_pool(pool: &Pool) -> Self {
        Self {
            id: pool.id.as_str().to_owned(),
            tournament_id: pool.tournament_id.as_str().to_owned(),
            owner_user_id: pool.owner_user_id.as_str().to_owned(),
            name: pool.name.as_str().to_owned(),
            invite_code: pool.invite_code.as_str().to_owned(),
            join_requires_allowlist: pool.join_requires_allowlist,
            prediction_lock_offset_minutes: pool.prediction_lock_offset_minutes,
            status: pool.status.as_str().to_owned(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PoolsListResponse {
    pub pools: Vec<PoolResponse>,
}

#[derive(Debug, Serialize)]
pub struct PoolMemberResponse {
    pub id: String,
    pub pool_id: String,
    pub user_id: String,
    pub display_name: String,
    pub role: String,
    pub status: String,
    pub joined_at: String,
    pub left_at: Option<String>,
}

impl PoolMemberResponse {
    pub fn from_member(member: &PoolMember) -> Self {
        Self::build(member, String::new())
    }

    pub fn from_member_with_name(value: &PoolMemberWithName) -> Self {
        Self::build(&value.member, value.display_name.clone())
    }

    fn build(member: &PoolMember, display_name: String) -> Self {
        Self {
            id: member.id.as_str().to_owned(),
            pool_id: member.pool_id.as_str().to_owned(),
            user_id: member.user_id.as_str().to_owned(),
            display_name,
            role: member.role.as_str().to_owned(),
            status: member.status.as_str().to_owned(),
            joined_at: member.joined_at.as_str().to_owned(),
            left_at: member.left_at.as_ref().map(|value| value.as_str().to_owned()),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PoolMembersListResponse {
    pub members: Vec<PoolMemberResponse>,
}

#[derive(Debug, Serialize)]
pub struct ScoringRuleResponse {
    pub id: String,
    pub pool_id: String,
    pub rule_key: String,
    pub points: i16,
}

impl ScoringRuleResponse {
    pub fn from_rule(rule: &PoolScoringRule) -> Self {
        Self {
            id: rule.id.as_str().to_owned(),
            pool_id: rule.pool_id.as_str().to_owned(),
            rule_key: rule.rule_key.as_str().to_owned(),
            points: rule.points,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ScoringRulesListResponse {
    pub rules: Vec<ScoringRuleResponse>,
}

#[derive(Debug, Serialize)]
pub struct JoinPoolResponse {
    pub pool: PoolResponse,
    pub member: PoolMemberResponse,
    pub already_member: bool,
}

impl JoinPoolResponse {
    pub fn from_outcome(outcome: &JoinOutcome) -> Self {
        Self {
            pool: PoolResponse::from_pool(&outcome.pool),
            member: PoolMemberResponse::from_member(&outcome.member),
            already_member: outcome.already_member,
        }
    }
}
