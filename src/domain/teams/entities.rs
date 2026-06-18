use crate::domain::{DomainId, NonEmptyString, TeamCode, UtcDateTime};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Team {
    pub id: DomainId,
    pub name: NonEmptyString,
    pub code: TeamCode,
    pub flag_emoji: Option<NonEmptyString>,
    pub flag_file_id: Option<DomainId>,
    pub created_at: UtcDateTime,
    pub updated_at: UtcDateTime,
}
