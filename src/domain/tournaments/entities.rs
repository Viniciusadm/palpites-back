use crate::domain::{CalendarDate, DomainId, DomainValidationError, NonEmptyString, UtcDateTime};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TournamentStatus {
    Draft,
    Active,
    Finished,
    Archived,
}

impl TournamentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Active => "active",
            Self::Finished => "finished",
            Self::Archived => "archived",
        }
    }

    pub fn parse(value: &str) -> Result<Self, DomainValidationError> {
        match value {
            "draft" => Ok(Self::Draft),
            "active" => Ok(Self::Active),
            "finished" => Ok(Self::Finished),
            "archived" => Ok(Self::Archived),
            _ => Err(DomainValidationError::Invalid("tournament_status")),
        }
    }

    fn rank(&self) -> u8 {
        match self {
            Self::Draft => 0,
            Self::Active => 1,
            Self::Finished => 2,
            Self::Archived => 3,
        }
    }

    pub fn can_transition_to(&self, target: TournamentStatus) -> bool {
        let current = self.rank();
        let next = target.rank();
        next == current || next == current + 1
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageKind {
    Group,
    Knockout,
}

impl StageKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Group => "group",
            Self::Knockout => "knockout",
        }
    }

    pub fn parse(value: &str) -> Result<Self, DomainValidationError> {
        match value {
            "group" => Ok(Self::Group),
            "knockout" => Ok(Self::Knockout),
            _ => Err(DomainValidationError::Invalid("stage_kind")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tournament {
    pub id: DomainId,
    pub name: NonEmptyString,
    pub slug: crate::domain::Slug,
    pub season_year: Option<u16>,
    pub starts_on: Option<CalendarDate>,
    pub ends_on: Option<CalendarDate>,
    pub status: TournamentStatus,
    pub created_at: UtcDateTime,
    pub updated_at: UtcDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TournamentStage {
    pub id: DomainId,
    pub tournament_id: DomainId,
    pub name: NonEmptyString,
    pub kind: StageKind,
    pub ordering: u16,
    pub created_at: UtcDateTime,
    pub updated_at: UtcDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TournamentGroup {
    pub id: DomainId,
    pub tournament_id: DomainId,
    pub label: NonEmptyString,
    pub created_at: UtcDateTime,
    pub updated_at: UtcDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TournamentTeam {
    pub id: DomainId,
    pub tournament_id: DomainId,
    pub team_id: DomainId,
    pub group_id: Option<DomainId>,
    pub created_at: UtcDateTime,
    pub updated_at: UtcDateTime,
}
