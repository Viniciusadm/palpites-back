use crate::domain::{DomainId, NonEmptyString, UtcDateTime};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct File {
    pub id: DomainId,
    pub owner_user_id: Option<DomainId>,
    pub bucket: NonEmptyString,
    pub object_key: NonEmptyString,
    pub content_type: NonEmptyString,
    pub byte_size: u64,
    pub original_name: NonEmptyString,
    pub checksum_sha256: Option<NonEmptyString>,
    pub created_at: UtcDateTime,
}
