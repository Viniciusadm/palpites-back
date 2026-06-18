use crate::domain::{DomainId, DomainValidationError, NonEmptyString, UtcDateTime};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationType {
    NewMatch,
    MatchResult,
    PredictionReminder,
    RankingUpdate,
    MemberJoined,
    Invite,
}

impl NotificationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NewMatch => "new_match",
            Self::MatchResult => "match_result",
            Self::PredictionReminder => "prediction_reminder",
            Self::RankingUpdate => "ranking_update",
            Self::MemberJoined => "member_joined",
            Self::Invite => "invite",
        }
    }

    pub fn parse(value: &str) -> Result<Self, DomainValidationError> {
        match value {
            "new_match" => Ok(Self::NewMatch),
            "match_result" => Ok(Self::MatchResult),
            "prediction_reminder" => Ok(Self::PredictionReminder),
            "ranking_update" => Ok(Self::RankingUpdate),
            "member_joined" => Ok(Self::MemberJoined),
            "invite" => Ok(Self::Invite),
            _ => Err(DomainValidationError::Invalid("notification_type")),
        }
    }

    pub fn is_preferenceable(&self) -> bool {
        !matches!(self, Self::Invite)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    InApp,
    Email,
    Push,
}

impl Channel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::InApp => "in_app",
            Self::Email => "email",
            Self::Push => "push",
        }
    }

    pub fn parse(value: &str) -> Result<Self, DomainValidationError> {
        match value {
            "in_app" => Ok(Self::InApp),
            "email" => Ok(Self::Email),
            "push" => Ok(Self::Push),
            _ => Err(DomainValidationError::Invalid("channel")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notification {
    pub id: DomainId,
    pub user_id: DomainId,
    pub pool_id: Option<DomainId>,
    pub notification_type: NotificationType,
    pub title: NonEmptyString,
    pub body: NonEmptyString,
    pub related_match_id: Option<DomainId>,
    pub read_at: Option<UtcDateTime>,
    pub created_at: UtcDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationPreference {
    pub id: DomainId,
    pub user_id: DomainId,
    pub pool_id: Option<DomainId>,
    pub notification_type: NotificationType,
    pub channel: Channel,
    pub enabled: bool,
    pub created_at: UtcDateTime,
    pub updated_at: UtcDateTime,
}
