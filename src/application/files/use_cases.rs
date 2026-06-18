use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::application::files::{
    CreateFileRecord, FileLinkRepository, FileRepository, FileStorage, LinkFile, StoredFile,
    UploadFile,
};
use crate::domain::files::File;
use crate::domain::NonEmptyString;
use crate::errors::AppError;

const ALLOWED_CONTENT_TYPES: [&str; 5] = [
    "image/png",
    "image/jpeg",
    "image/webp",
    "image/gif",
    "image/svg+xml",
];

const MAX_ORIGINAL_NAME_LEN: usize = 255;

pub struct FileUseCases<R, S> {
    files: R,
    storage: S,
    max_byte_size: u64,
}

impl<R, S> FileUseCases<R, S>
where
    R: FileRepository,
    S: FileStorage,
{
    pub fn new(files: R, storage: S, max_byte_size: u64) -> Self {
        Self {
            files,
            storage,
            max_byte_size,
        }
    }

    pub async fn upload(&self, command: UploadFile) -> Result<StoredFile, AppError> {
        let content_type = NonEmptyString::new(command.content_type, "file.content_type")?
            .as_str()
            .to_owned();
        if !ALLOWED_CONTENT_TYPES.contains(&content_type.as_str()) {
            return Err(AppError::validation_code(
                "unsupported_content_type",
                "file content type is not an allowed image type",
            ));
        }

        let original_name = NonEmptyString::new(command.original_name, "file.original_name")?
            .as_str()
            .to_owned();
        if original_name.chars().count() > MAX_ORIGINAL_NAME_LEN {
            return Err(AppError::validation_code(
                "original_name_too_long",
                "file name must be at most 255 characters",
            ));
        }

        let byte_size = command.bytes.len() as u64;
        if byte_size == 0 {
            return Err(AppError::validation_code(
                "empty_file",
                "uploaded file is empty",
            ));
        }
        if byte_size > self.max_byte_size {
            return Err(AppError::validation_code(
                "file_too_large",
                "uploaded file exceeds the maximum allowed size",
            ));
        }

        let checksum = hex_sha256(&command.bytes);
        let file_id = Uuid::new_v4().to_string();
        let object_key = file_id.clone();
        let bucket = self.storage.bucket();

        self.storage
            .put(&object_key, &content_type, command.bytes)
            .await?;

        self.files
            .create(CreateFileRecord {
                file_id: file_id.clone(),
                owner_user_id: command.owner_user_id,
                bucket,
                object_key: object_key.clone(),
                content_type,
                byte_size,
                original_name,
                checksum_sha256: Some(checksum),
            })
            .await?;

        let file = self.find(&file_id).await?;
        let url = self.storage.signed_url(&object_key).await?;
        Ok(StoredFile { file, url })
    }

    pub async fn get(&self, id: &str) -> Result<StoredFile, AppError> {
        let file = self.find(id).await?;
        let url = self.storage.signed_url(file.object_key.as_str()).await?;
        Ok(StoredFile { file, url })
    }

    async fn find(&self, id: &str) -> Result<File, AppError> {
        self.files
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::not_found_code("file_not_found", "file was not found"))
    }
}

pub struct LinkUseCases<F, L> {
    files: F,
    links: L,
}

impl<F, L> LinkUseCases<F, L>
where
    F: FileRepository,
    L: FileLinkRepository,
{
    pub fn new(files: F, links: L) -> Self {
        Self { files, links }
    }

    pub async fn link_user_avatar(&self, user_id: &str, command: LinkFile) -> Result<(), AppError> {
        self.ensure_file_exists(command.file_id.as_deref()).await?;
        let found = self
            .links
            .set_user_avatar(user_id, command.file_id.as_deref())
            .await?;
        if !found {
            return Err(AppError::NotFound("user was not found".to_owned()));
        }
        Ok(())
    }

    pub async fn link_team_flag(&self, team_id: &str, command: LinkFile) -> Result<(), AppError> {
        self.ensure_file_exists(command.file_id.as_deref()).await?;
        let found = self
            .links
            .set_team_flag(team_id, command.file_id.as_deref())
            .await?;
        if !found {
            return Err(AppError::NotFound("team was not found".to_owned()));
        }
        Ok(())
    }

    async fn ensure_file_exists(&self, file_id: Option<&str>) -> Result<(), AppError> {
        if let Some(file_id) = file_id {
            if self.files.find_by_id(file_id).await?.is_none() {
                return Err(AppError::not_found_code("file_not_found", "file was not found"));
            }
        }
        Ok(())
    }
}

fn hex_sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}
