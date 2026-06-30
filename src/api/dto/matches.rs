use serde::{Deserialize, Serialize};

use crate::application::matches::{CreateMatch, MatchFilters, UpdateMatch};
use crate::domain::matches::Match;

#[derive(Debug, Deserialize)]
pub struct CreateMatchRequest {
    pub stage_id: String,
    pub home_team_id: Option<String>,
    pub away_team_id: Option<String>,
    pub kickoff_at: String,
    #[serde(default)]
    pub can_go_to_penalties: bool,
}

impl From<CreateMatchRequest> for CreateMatch {
    fn from(value: CreateMatchRequest) -> Self {
        Self {
            stage_id: value.stage_id,
            home_team_id: value.home_team_id,
            away_team_id: value.away_team_id,
            kickoff_at: value.kickoff_at,
            can_go_to_penalties: value.can_go_to_penalties,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateMatchRequest {
    pub stage_id: String,
    pub home_team_id: Option<String>,
    pub away_team_id: Option<String>,
    pub kickoff_at: String,
    pub status: String,
    #[serde(default)]
    pub can_go_to_penalties: bool,
}

impl From<UpdateMatchRequest> for UpdateMatch {
    fn from(value: UpdateMatchRequest) -> Self {
        Self {
            stage_id: value.stage_id,
            home_team_id: value.home_team_id,
            away_team_id: value.away_team_id,
            kickoff_at: value.kickoff_at,
            status: value.status,
            can_go_to_penalties: value.can_go_to_penalties,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct MatchFilterQuery {
    pub stage: Option<String>,
    pub status: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
}

impl From<MatchFilterQuery> for MatchFilters {
    fn from(value: MatchFilterQuery) -> Self {
        Self {
            stage_id: value.stage,
            status: value.status,
            date_from: value.date_from,
            date_to: value.date_to,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct MatchResponse {
    pub id: String,
    pub tournament_id: String,
    pub stage_id: String,
    pub home_team_id: Option<String>,
    pub away_team_id: Option<String>,
    pub kickoff_at: String,
    pub status: String,
    pub home_score: Option<u8>,
    pub away_score: Option<u8>,
    pub can_go_to_penalties: bool,
    pub penalties_winner: Option<String>,
    pub finished_at: Option<String>,
}

impl MatchResponse {
    pub fn from_match(value: &Match) -> Self {
        Self {
            id: value.id.as_str().to_owned(),
            tournament_id: value.tournament_id.as_str().to_owned(),
            stage_id: value.stage_id.as_str().to_owned(),
            home_team_id: value
                .home_team_id
                .as_ref()
                .map(|team| team.as_str().to_owned()),
            away_team_id: value
                .away_team_id
                .as_ref()
                .map(|team| team.as_str().to_owned()),
            kickoff_at: value.kickoff_at.as_str().to_owned(),
            status: value.status.as_str().to_owned(),
            home_score: value.home_score.as_ref().map(|score| score.value()),
            away_score: value.away_score.as_ref().map(|score| score.value()),
            can_go_to_penalties: value.can_go_to_penalties,
            penalties_winner: value
                .penalties_winner
                .as_ref()
                .map(|side| side.as_str().to_owned()),
            finished_at: value
                .finished_at
                .as_ref()
                .map(|value| value.as_str().to_owned()),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct MatchesListResponse {
    pub matches: Vec<MatchResponse>,
}
