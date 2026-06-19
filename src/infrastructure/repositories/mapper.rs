use chrono::{NaiveDate, NaiveDateTime, SecondsFormat};

use crate::domain::matches::MatchStatus;
use crate::domain::notifications::{Channel, NotificationType};
use crate::domain::pools::{MemberStatus, PoolRole, PoolStatus, ScoringRuleKey};
use crate::domain::tournaments::{StageKind, TournamentStatus};
use crate::domain::users::UserRole;
use crate::domain::{
    CalendarDate, DomainId, Email, InviteCode, NonEmptyString, Score, Slug, TeamCode, UtcDateTime,
};
use crate::errors::AppError;

pub fn id(value: String) -> Result<DomainId, AppError> {
    Ok(DomainId::new(value)?)
}

pub fn opt_id(value: Option<String>) -> Result<Option<DomainId>, AppError> {
    value.map(id).transpose()
}

pub fn user_role(value: String) -> Result<UserRole, AppError> {
    Ok(UserRole::parse(&value)?)
}

pub fn non_empty(value: String, field: &'static str) -> Result<NonEmptyString, AppError> {
    Ok(NonEmptyString::new(value, field)?)
}

pub fn opt_non_empty(
    value: Option<String>,
    field: &'static str,
) -> Result<Option<NonEmptyString>, AppError> {
    value.map(|inner| non_empty(inner, field)).transpose()
}

pub fn team_code(value: String) -> Result<TeamCode, AppError> {
    Ok(TeamCode::new(value)?)
}

pub fn slug(value: String) -> Result<Slug, AppError> {
    Ok(Slug::new(value)?)
}

pub fn invite_code(value: String) -> Result<InviteCode, AppError> {
    Ok(InviteCode::new(value)?)
}

pub fn pool_role(value: String) -> Result<PoolRole, AppError> {
    Ok(PoolRole::parse(&value)?)
}

pub fn pool_status(value: String) -> Result<PoolStatus, AppError> {
    Ok(PoolStatus::parse(&value)?)
}

pub fn member_status(value: String) -> Result<MemberStatus, AppError> {
    Ok(MemberStatus::parse(&value)?)
}

pub fn scoring_rule_key(value: String) -> Result<ScoringRuleKey, AppError> {
    Ok(ScoringRuleKey::parse(&value)?)
}

pub fn tournament_status(value: String) -> Result<TournamentStatus, AppError> {
    Ok(TournamentStatus::parse(&value)?)
}

pub fn stage_kind(value: String) -> Result<StageKind, AppError> {
    Ok(StageKind::parse(&value)?)
}

pub fn match_status(value: String) -> Result<MatchStatus, AppError> {
    Ok(MatchStatus::parse(&value)?)
}

pub fn notification_type(value: String) -> Result<NotificationType, AppError> {
    Ok(NotificationType::parse(&value)?)
}

pub fn channel(value: String) -> Result<Channel, AppError> {
    Ok(Channel::parse(&value)?)
}

pub fn score(value: u8) -> Result<Score, AppError> {
    Ok(Score::new(value)?)
}

pub fn opt_score(value: Option<u8>) -> Result<Option<Score>, AppError> {
    value.map(|inner| Ok(Score::new(inner)?)).transpose()
}

pub fn opt_date(value: Option<NaiveDate>) -> Result<Option<CalendarDate>, AppError> {
    value
        .map(|date| Ok(CalendarDate::new(date.to_string())?))
        .transpose()
}

pub fn email(value: String) -> Result<Email, AppError> {
    Ok(Email::new(value)?)
}

pub fn datetime(value: NaiveDateTime) -> Result<UtcDateTime, AppError> {
    // O banco armazena UTC sem offset; emitimos RFC3339 com sufixo `Z` para que o
    // instante seja inequívoco no cliente (ex.: "2026-06-18T19:00:00.000Z").
    let iso = value.and_utc().to_rfc3339_opts(SecondsFormat::Millis, true);
    Ok(UtcDateTime::new_iso8601(iso)?)
}

pub fn opt_datetime(value: Option<NaiveDateTime>) -> Result<Option<UtcDateTime>, AppError> {
    value.map(datetime).transpose()
}
