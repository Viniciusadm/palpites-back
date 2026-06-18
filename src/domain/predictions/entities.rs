use crate::domain::{DomainId, Score, UtcDateTime};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prediction {
    pub id: DomainId,
    pub pool_member_id: DomainId,
    pub match_id: DomainId,
    pub home_score: Score,
    pub away_score: Score,
    pub points_awarded: Option<i16>,
    pub scored_at: Option<UtcDateTime>,
    pub created_at: UtcDateTime,
    pub updated_at: UtcDateTime,
}
