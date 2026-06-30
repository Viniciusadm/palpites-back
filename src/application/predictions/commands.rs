#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpsertPrediction {
    pub home_score: u8,
    pub away_score: u8,
    /// Raw "home"/"away" pick for the penalty shootout, only meaningful when the
    /// match can go to penalties and the predicted score is a draw.
    pub penalties_pick: Option<String>,
}
