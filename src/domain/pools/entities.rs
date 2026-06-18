use crate::domain::{DomainId, DomainValidationError, InviteCode, NonEmptyString, UtcDateTime};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolRole {
    Owner,
    Admin,
    Member,
}

impl PoolRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Admin => "admin",
            Self::Member => "member",
        }
    }

    pub fn parse(value: &str) -> Result<Self, DomainValidationError> {
        match value {
            "owner" => Ok(Self::Owner),
            "admin" => Ok(Self::Admin),
            "member" => Ok(Self::Member),
            _ => Err(DomainValidationError::Invalid("pool_role")),
        }
    }

    pub fn is_owner(&self) -> bool {
        matches!(self, Self::Owner)
    }

    pub fn can_manage_members(&self) -> bool {
        matches!(self, Self::Owner | Self::Admin)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Private,
}

impl Visibility {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Private => "private",
        }
    }

    pub fn parse(value: &str) -> Result<Self, DomainValidationError> {
        match value {
            "public" => Ok(Self::Public),
            "private" => Ok(Self::Private),
            _ => Err(DomainValidationError::Invalid("visibility")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolStatus {
    Active,
    Archived,
}

impl PoolStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Archived => "archived",
        }
    }

    pub fn parse(value: &str) -> Result<Self, DomainValidationError> {
        match value {
            "active" => Ok(Self::Active),
            "archived" => Ok(Self::Archived),
            _ => Err(DomainValidationError::Invalid("pool_status")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberStatus {
    Active,
    Inactive,
}

impl MemberStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Inactive => "inactive",
        }
    }

    pub fn parse(value: &str) -> Result<Self, DomainValidationError> {
        match value {
            "active" => Ok(Self::Active),
            "inactive" => Ok(Self::Inactive),
            _ => Err(DomainValidationError::Invalid("member_status")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScoringRuleKey {
    ExactScore,
    CorrectOutcome,
    CorrectGoalDifference,
}

impl ScoringRuleKey {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ExactScore => "exact_score",
            Self::CorrectOutcome => "correct_outcome",
            Self::CorrectGoalDifference => "correct_goal_difference",
        }
    }

    pub fn parse(value: &str) -> Result<Self, DomainValidationError> {
        match value {
            "exact_score" => Ok(Self::ExactScore),
            "correct_outcome" => Ok(Self::CorrectOutcome),
            "correct_goal_difference" => Ok(Self::CorrectGoalDifference),
            _ => Err(DomainValidationError::Invalid("scoring_rule_key")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pool {
    pub id: DomainId,
    pub tournament_id: DomainId,
    pub owner_user_id: DomainId,
    pub name: NonEmptyString,
    pub invite_code: InviteCode,
    pub visibility: Visibility,
    pub ranking_public: bool,
    pub prediction_lock_offset_minutes: u16,
    pub status: PoolStatus,
    pub created_at: UtcDateTime,
    pub updated_at: UtcDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolScoringRule {
    pub id: DomainId,
    pub pool_id: DomainId,
    pub rule_key: ScoringRuleKey,
    pub points: i16,
    pub created_at: UtcDateTime,
    pub updated_at: UtcDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolMember {
    pub id: DomainId,
    pub pool_id: DomainId,
    pub user_id: DomainId,
    pub role: PoolRole,
    pub status: MemberStatus,
    pub joined_at: UtcDateTime,
    pub left_at: Option<UtcDateTime>,
    pub created_at: UtcDateTime,
    pub updated_at: UtcDateTime,
}
