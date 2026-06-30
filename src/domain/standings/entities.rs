use crate::domain::{DomainId, UtcDateTime};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Standing {
    pub id: DomainId,
    pub pool_id: DomainId,
    pub pool_member_id: DomainId,
    pub total_points: i32,
    pub exact_count: i32,
    pub outcome_count: i32,
    pub hits_count: i32,
    pub penalties_count: i32,
    pub position: i32,
    pub updated_at: UtcDateTime,
}
