use serde::{Deserialize, Serialize};

use crate::application::predictions::UpsertPrediction;
use crate::domain::predictions::Prediction;

#[derive(Debug, Deserialize)]
pub struct UpsertPredictionRequest {
    pub home_score: u8,
    pub away_score: u8,
    #[serde(default)]
    pub penalties_pick: Option<String>,
}

impl From<UpsertPredictionRequest> for UpsertPrediction {
    fn from(value: UpsertPredictionRequest) -> Self {
        Self {
            home_score: value.home_score,
            away_score: value.away_score,
            penalties_pick: value.penalties_pick,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PredictionResponse {
    pub id: String,
    pub match_id: String,
    pub home_score: u8,
    pub away_score: u8,
    pub penalties_pick: Option<String>,
    pub points_awarded: Option<i16>,
    pub scored_at: Option<String>,
}

impl PredictionResponse {
    pub fn from_prediction(value: &Prediction) -> Self {
        Self {
            id: value.id.as_str().to_owned(),
            match_id: value.match_id.as_str().to_owned(),
            home_score: value.home_score.value(),
            away_score: value.away_score.value(),
            penalties_pick: value
                .penalties_pick
                .as_ref()
                .map(|side| side.as_str().to_owned()),
            points_awarded: value.points_awarded,
            scored_at: value.scored_at.as_ref().map(|value| value.as_str().to_owned()),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PredictionsListResponse {
    pub predictions: Vec<PredictionResponse>,
}
