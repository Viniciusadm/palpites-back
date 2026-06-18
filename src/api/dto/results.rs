use serde::{Deserialize, Serialize};

use crate::application::results::{
    EnterResult, HistoryEntry, HistorySummary, MemberPredictionView, Ranking, StandingWithName,
};

#[derive(Debug, Deserialize)]
pub struct EnterResultRequest {
    pub home_score: u8,
    pub away_score: u8,
}

impl From<EnterResultRequest> for EnterResult {
    fn from(value: EnterResultRequest) -> Self {
        Self {
            home_score: value.home_score,
            away_score: value.away_score,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct RankingEntryResponse {
    pub pool_member_id: String,
    pub display_name: String,
    pub total_points: i32,
    pub exact_count: i32,
    pub outcome_count: i32,
    pub hits_count: i32,
    pub position: i32,
}

impl RankingEntryResponse {
    pub fn from_entry(value: &StandingWithName) -> Self {
        Self {
            pool_member_id: value.standing.pool_member_id.as_str().to_owned(),
            display_name: value.display_name.clone(),
            total_points: value.standing.total_points,
            exact_count: value.standing.exact_count,
            outcome_count: value.standing.outcome_count,
            hits_count: value.standing.hits_count,
            position: value.standing.position,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct RankingResponse {
    pub standings: Vec<RankingEntryResponse>,
}

impl RankingResponse {
    pub fn from_ranking(value: &Ranking) -> Self {
        Self {
            standings: value
                .standings
                .iter()
                .map(RankingEntryResponse::from_entry)
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct HistoryEntryResponse {
    pub match_id: String,
    pub match_status: String,
    pub kickoff_at: String,
    pub prediction_home: u8,
    pub prediction_away: u8,
    pub result_home: Option<u8>,
    pub result_away: Option<u8>,
    pub points_awarded: Option<i16>,
    pub hit_kind: Option<String>,
}

impl HistoryEntryResponse {
    fn from_entry(value: &HistoryEntry) -> Self {
        Self {
            match_id: value.match_id.clone(),
            match_status: value.match_status.clone(),
            kickoff_at: value.kickoff_at.clone(),
            prediction_home: value.prediction_home,
            prediction_away: value.prediction_away,
            result_home: value.result_home,
            result_away: value.result_away,
            points_awarded: value.points_awarded,
            hit_kind: value.hit_kind.clone(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct HistoryResponse {
    pub pool_member_id: String,
    pub total_points: i32,
    pub exact_count: i32,
    pub outcome_count: i32,
    pub hits_count: i32,
    pub errors_count: i32,
    pub pending_count: i32,
    pub entries: Vec<HistoryEntryResponse>,
}

impl HistoryResponse {
    pub fn from_summary(value: &HistorySummary) -> Self {
        Self {
            pool_member_id: value.pool_member_id.clone(),
            total_points: value.total_points,
            exact_count: value.exact_count,
            outcome_count: value.outcome_count,
            hits_count: value.hits_count,
            errors_count: value.errors_count,
            pending_count: value.pending_count,
            entries: value
                .entries
                .iter()
                .map(HistoryEntryResponse::from_entry)
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct MemberPredictionResponse {
    pub match_id: String,
    pub match_status: String,
    pub kickoff_at: String,
    pub prediction_home: u8,
    pub prediction_away: u8,
    pub result_home: Option<u8>,
    pub result_away: Option<u8>,
    pub points_awarded: Option<i16>,
}

impl MemberPredictionResponse {
    fn from_view(value: &MemberPredictionView) -> Self {
        Self {
            match_id: value.match_id.clone(),
            match_status: value.match_status.clone(),
            kickoff_at: value.kickoff_at.clone(),
            prediction_home: value.prediction_home,
            prediction_away: value.prediction_away,
            result_home: value.result_home,
            result_away: value.result_away,
            points_awarded: value.points_awarded,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct MemberPredictionsResponse {
    pub predictions: Vec<MemberPredictionResponse>,
}

impl MemberPredictionsResponse {
    pub fn from_views(values: &[MemberPredictionView]) -> Self {
        Self {
            predictions: values.iter().map(MemberPredictionResponse::from_view).collect(),
        }
    }
}
