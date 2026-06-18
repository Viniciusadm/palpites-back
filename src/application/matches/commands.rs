#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateMatch {
    pub stage_id: String,
    pub home_team_id: Option<String>,
    pub away_team_id: Option<String>,
    pub kickoff_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateMatch {
    pub stage_id: String,
    pub home_team_id: Option<String>,
    pub away_team_id: Option<String>,
    pub kickoff_at: String,
    pub status: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MatchFilters {
    pub stage_id: Option<String>,
    pub status: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
}
