use serde::{Deserialize, Serialize};

use crate::application::files::{LinkFile, StoredFile};

#[derive(Debug, Serialize)]
pub struct UploadFileResponse {
    pub id: String,
    pub url: String,
}

impl UploadFileResponse {
    pub fn from_stored(stored: &StoredFile) -> Self {
        Self {
            id: stored.file.id.as_str().to_owned(),
            url: stored.url.clone(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FileResponse {
    pub id: String,
    pub owner_user_id: Option<String>,
    pub bucket: String,
    pub object_key: String,
    pub content_type: String,
    pub byte_size: u64,
    pub original_name: String,
    pub checksum_sha256: Option<String>,
    pub created_at: String,
    pub url: String,
}

impl FileResponse {
    pub fn from_stored(stored: &StoredFile) -> Self {
        let file = &stored.file;
        Self {
            id: file.id.as_str().to_owned(),
            owner_user_id: file.owner_user_id.as_ref().map(|id| id.as_str().to_owned()),
            bucket: file.bucket.as_str().to_owned(),
            object_key: file.object_key.as_str().to_owned(),
            content_type: file.content_type.as_str().to_owned(),
            byte_size: file.byte_size,
            original_name: file.original_name.as_str().to_owned(),
            checksum_sha256: file
                .checksum_sha256
                .as_ref()
                .map(|value| value.as_str().to_owned()),
            created_at: file.created_at.as_str().to_owned(),
            url: stored.url.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct LinkFileRequest {
    pub file_id: Option<String>,
}

impl From<LinkFileRequest> for LinkFile {
    fn from(value: LinkFileRequest) -> Self {
        Self {
            file_id: value.file_id,
        }
    }
}
