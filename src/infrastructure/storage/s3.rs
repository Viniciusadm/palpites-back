use async_trait::async_trait;

use crate::application::files::FileStorage;
use crate::errors::AppError;

#[derive(Debug, Clone)]
pub struct S3FileStorage {
    bucket: String,
    region: Option<String>,
    endpoint: Option<String>,
}

impl S3FileStorage {
    pub fn new(
        bucket: impl Into<String>,
        region: Option<String>,
        endpoint: Option<String>,
    ) -> Self {
        Self {
            bucket: bucket.into(),
            region,
            endpoint,
        }
    }

    fn unavailable() -> AppError {
        AppError::Configuration(
            "S3 storage backend is not available in this build; use FILES_BACKEND=local".to_owned(),
        )
    }
}

#[async_trait]
impl FileStorage for S3FileStorage {
    fn bucket(&self) -> String {
        self.bucket.clone()
    }

    async fn put(
        &self,
        _object_key: &str,
        _content_type: &str,
        _bytes: Vec<u8>,
    ) -> Result<(), AppError> {
        let _ = (&self.region, &self.endpoint);
        Err(Self::unavailable())
    }

    async fn signed_url(&self, _object_key: &str) -> Result<String, AppError> {
        Err(Self::unavailable())
    }

    async fn delete(&self, _object_key: &str) -> Result<(), AppError> {
        Err(Self::unavailable())
    }
}
