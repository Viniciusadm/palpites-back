use crate::domain::{DomainId, DomainValidationError, PenaltySide, Score, UtcDateTime};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchStatus {
    Scheduled,
    Live,
    Finished,
    Canceled,
}

impl MatchStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Scheduled => "scheduled",
            Self::Live => "live",
            Self::Finished => "finished",
            Self::Canceled => "canceled",
        }
    }

    pub fn parse(value: &str) -> Result<Self, DomainValidationError> {
        match value {
            "scheduled" => Ok(Self::Scheduled),
            "live" => Ok(Self::Live),
            "finished" => Ok(Self::Finished),
            "canceled" => Ok(Self::Canceled),
            _ => Err(DomainValidationError::Invalid("match_status")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Match {
    pub id: DomainId,
    pub tournament_id: DomainId,
    pub stage_id: DomainId,
    pub home_team_id: Option<DomainId>,
    pub away_team_id: Option<DomainId>,
    pub kickoff_at: UtcDateTime,
    pub status: MatchStatus,
    pub home_score: Option<Score>,
    pub away_score: Option<Score>,
    pub can_go_to_penalties: bool,
    pub penalties_winner: Option<PenaltySide>,
    pub finished_at: Option<UtcDateTime>,
    pub created_at: UtcDateTime,
    pub updated_at: UtcDateTime,
}

impl Match {
    pub fn ensure_distinct_teams(
        home_team_id: Option<&DomainId>,
        away_team_id: Option<&DomainId>,
    ) -> Result<(), DomainValidationError> {
        if let (Some(home), Some(away)) = (home_team_id, away_team_id) {
            if home == away {
                return Err(DomainValidationError::Invalid("match_teams"));
            }
        }
        Ok(())
    }
}
