use crate::domain::standings::Standing;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnterResult {
    pub home_score: u8,
    pub away_score: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistorySummary {
    pub pool_member_id: String,
    pub total_points: i32,
    pub exact_count: i32,
    pub outcome_count: i32,
    pub hits_count: i32,
    pub errors_count: i32,
    pub pending_count: i32,
    pub entries: Vec<HistoryEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberPredictionView {
    pub match_id: String,
    pub match_status: String,
    pub kickoff_at: String,
    pub prediction_home: u8,
    pub prediction_away: u8,
    pub result_home: Option<u8>,
    pub result_away: Option<u8>,
    pub points_awarded: Option<i16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ranking {
    pub standings: Vec<Standing>,
}
