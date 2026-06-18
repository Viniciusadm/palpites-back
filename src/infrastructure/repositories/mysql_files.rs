use async_trait::async_trait;
use sqlx::{MySqlPool, Row};

use crate::application::files::{CreateFileRecord, FileLinkRepository, FileRepository};
use crate::domain::files::File;
use crate::errors::AppError;
use crate::infrastructure::repositories::mapper;

const SELECT_COLUMNS: &str = "SELECT id, owner_user_id, bucket, object_key, content_type, byte_size, original_name, checksum_sha256, created_at FROM files";

#[derive(Clone)]
pub struct MySqlFileRepository {
    pool: MySqlPool,
}

impl MySqlFileRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    fn map_write_error(error: sqlx::Error) -> AppError {
        if let sqlx::Error::Database(database_error) = &error {
            if database_error.is_unique_violation() {
                return AppError::conflict_code(
                    "file_object_key_taken",
                    "a file with this object key already exists",
                );
            }
        }
        AppError::from(error)
    }
}

#[async_trait]
impl FileRepository for MySqlFileRepository {
    async fn create(&self, record: CreateFileRecord) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO files (id, owner_user_id, bucket, object_key, content_type, byte_size, original_name, checksum_sha256) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&record.file_id)
        .bind(&record.owner_user_id)
        .bind(&record.bucket)
        .bind(&record.object_key)
        .bind(&record.content_type)
        .bind(record.byte_size)
        .bind(&record.original_name)
        .bind(&record.checksum_sha256)
        .execute(&self.pool)
        .await
        .map_err(Self::map_write_error)?;
        Ok(())
    }

    async fn find_by_id(&self, id: &str) -> Result<Option<File>, AppError> {
        let sql = format!("{SELECT_COLUMNS} WHERE id = ?");
        let row = sqlx::query(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        row.map(map_file).transpose()
    }
}

#[derive(Clone)]
pub struct MySqlFileLinkRepository {
    pool: MySqlPool,
}

impl MySqlFileLinkRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    async fn exists(&self, table: &str, id: &str) -> Result<bool, AppError> {
        let sql = format!("SELECT 1 FROM {table} WHERE id = ?");
        let row = sqlx::query(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.is_some())
    }
}

#[async_trait]
impl FileLinkRepository for MySqlFileLinkRepository {
    async fn set_user_avatar(
        &self,
        user_id: &str,
        file_id: Option<&str>,
    ) -> Result<bool, AppError> {
        if !self.exists("users", user_id).await? {
            return Ok(false);
        }
        sqlx::query("UPDATE users SET avatar_file_id = ? WHERE id = ?")
            .bind(file_id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(true)
    }

    async fn set_team_flag(&self, team_id: &str, file_id: Option<&str>) -> Result<bool, AppError> {
        if !self.exists("teams", team_id).await? {
            return Ok(false);
        }
        sqlx::query("UPDATE teams SET flag_file_id = ? WHERE id = ?")
            .bind(file_id)
            .bind(team_id)
            .execute(&self.pool)
            .await?;
        Ok(true)
    }
}

fn map_file(row: sqlx::mysql::MySqlRow) -> Result<File, AppError> {
    Ok(File {
        id: mapper::id(row.try_get("id")?)?,
        owner_user_id: mapper::opt_id(row.try_get("owner_user_id")?)?,
        bucket: mapper::non_empty(row.try_get("bucket")?, "file.bucket")?,
        object_key: mapper::non_empty(row.try_get("object_key")?, "file.object_key")?,
        content_type: mapper::non_empty(row.try_get("content_type")?, "file.content_type")?,
        byte_size: row.try_get::<u64, _>("byte_size")?,
        original_name: mapper::non_empty(row.try_get("original_name")?, "file.original_name")?,
        checksum_sha256: mapper::opt_non_empty(
            row.try_get("checksum_sha256")?,
            "file.checksum_sha256",
        )?,
        created_at: mapper::datetime(row.try_get("created_at")?)?,
    })
}
