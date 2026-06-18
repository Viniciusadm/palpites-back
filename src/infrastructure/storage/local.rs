use std::path::PathBuf;

use async_trait::async_trait;
use tokio::fs;

use crate::application::files::FileStorage;
use crate::errors::AppError;

#[derive(Debug, Clone)]
pub struct LocalFileStorage {
    root: PathBuf,
    bucket: String,
    public_base_url: String,
}

impl LocalFileStorage {
    pub fn new(
        root: impl Into<PathBuf>,
        bucket: impl Into<String>,
        public_base_url: impl Into<String>,
    ) -> Self {
        Self {
            root: root.into(),
            bucket: bucket.into(),
            public_base_url: public_base_url.into(),
        }
    }

    fn object_path(&self, object_key: &str) -> PathBuf {
        self.root.join(&self.bucket).join(object_key)
    }
}

#[async_trait]
impl FileStorage for LocalFileStorage {
    fn bucket(&self) -> String {
        self.bucket.clone()
    }

    async fn put(
        &self,
        object_key: &str,
        _content_type: &str,
        bytes: Vec<u8>,
    ) -> Result<(), AppError> {
        let path = self.object_path(object_key);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|error| AppError::Internal(format!("failed to create storage dir: {error}")))?;
        }
        fs::write(&path, bytes)
            .await
            .map_err(|error| AppError::Internal(format!("failed to write file: {error}")))?;
        Ok(())
    }

    async fn signed_url(&self, object_key: &str) -> Result<String, AppError> {
        let base = self.public_base_url.trim_end_matches('/');
        Ok(format!("{base}/{}/{object_key}", self.bucket))
    }

    async fn delete(&self, object_key: &str) -> Result<(), AppError> {
        let path = self.object_path(object_key);
        match fs::remove_file(&path).await {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(AppError::Internal(format!("failed to delete file: {error}"))),
        }
    }
}
