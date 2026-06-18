use async_trait::async_trait;

use crate::domain::files::File;
use crate::errors::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateFileRecord {
    pub file_id: String,
    pub owner_user_id: Option<String>,
    pub bucket: String,
    pub object_key: String,
    pub content_type: String,
    pub byte_size: u64,
    pub original_name: String,
    pub checksum_sha256: Option<String>,
}

#[async_trait]
pub trait FileRepository: Send + Sync {
    async fn create(&self, record: CreateFileRecord) -> Result<(), AppError>;
    async fn find_by_id(&self, id: &str) -> Result<Option<File>, AppError>;
}

#[async_trait]
pub trait FileStorage: Send + Sync {
    fn bucket(&self) -> String;
    async fn put(
        &self,
        object_key: &str,
        content_type: &str,
        bytes: Vec<u8>,
    ) -> Result<(), AppError>;
    async fn signed_url(&self, object_key: &str) -> Result<String, AppError>;
    async fn delete(&self, object_key: &str) -> Result<(), AppError>;
}

#[async_trait]
pub trait FileLinkRepository: Send + Sync {
    async fn set_user_avatar(
        &self,
        user_id: &str,
        file_id: Option<&str>,
    ) -> Result<bool, AppError>;
    async fn set_team_flag(&self, team_id: &str, file_id: Option<&str>) -> Result<bool, AppError>;
}
