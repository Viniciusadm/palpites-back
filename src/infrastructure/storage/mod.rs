pub mod local;
pub mod s3;

use async_trait::async_trait;

use crate::application::files::FileStorage;
use crate::errors::AppError;
use crate::infrastructure::storage::local::LocalFileStorage;
use crate::infrastructure::storage::s3::S3FileStorage;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileStorageSettings {
    pub backend: String,
    pub bucket: String,
    pub local_dir: String,
    pub public_base_url: String,
    pub max_byte_size: u64,
    pub s3_region: Option<String>,
    pub s3_endpoint: Option<String>,
}

impl Default for FileStorageSettings {
    fn default() -> Self {
        Self {
            backend: "local".to_owned(),
            bucket: "palpites-local".to_owned(),
            local_dir: "./storage".to_owned(),
            public_base_url: "http://127.0.0.1:3001/files-local".to_owned(),
            max_byte_size: 5 * 1024 * 1024,
            s3_region: None,
            s3_endpoint: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum DynFileStorage {
    Local(LocalFileStorage),
    S3(S3FileStorage),
}

pub fn build(settings: &FileStorageSettings) -> DynFileStorage {
    match settings.backend.as_str() {
        "s3" => DynFileStorage::S3(S3FileStorage::new(
            settings.bucket.clone(),
            settings.s3_region.clone(),
            settings.s3_endpoint.clone(),
        )),
        _ => DynFileStorage::Local(LocalFileStorage::new(
            settings.local_dir.clone(),
            settings.bucket.clone(),
            settings.public_base_url.clone(),
        )),
    }
}

#[async_trait]
impl FileStorage for DynFileStorage {
    fn bucket(&self) -> String {
        match self {
            Self::Local(storage) => storage.bucket(),
            Self::S3(storage) => storage.bucket(),
        }
    }

    async fn put(
        &self,
        object_key: &str,
        content_type: &str,
        bytes: Vec<u8>,
    ) -> Result<(), AppError> {
        match self {
            Self::Local(storage) => storage.put(object_key, content_type, bytes).await,
            Self::S3(storage) => storage.put(object_key, content_type, bytes).await,
        }
    }

    async fn signed_url(&self, object_key: &str) -> Result<String, AppError> {
        match self {
            Self::Local(storage) => storage.signed_url(object_key).await,
            Self::S3(storage) => storage.signed_url(object_key).await,
        }
    }

    async fn delete(&self, object_key: &str) -> Result<(), AppError> {
        match self {
            Self::Local(storage) => storage.delete(object_key).await,
            Self::S3(storage) => storage.delete(object_key).await,
        }
    }
}
