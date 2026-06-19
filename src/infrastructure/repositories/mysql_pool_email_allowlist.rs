use async_trait::async_trait;
use sqlx::{MySqlPool, Row};

use crate::application::pools::{NewAllowedEmailRecord, PoolEmailAllowlistRepository};
use crate::domain::pools::PoolAllowedEmail;
use crate::errors::AppError;
use crate::infrastructure::repositories::mapper;

const ALLOWED_EMAIL_COLUMNS: &str =
    "SELECT id, pool_id, email, added_by_user_id, created_at FROM pool_email_allowlist";

#[derive(Clone)]
pub struct MySqlPoolEmailAllowlistRepository {
    pool: MySqlPool,
}

impl MySqlPoolEmailAllowlistRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PoolEmailAllowlistRepository for MySqlPoolEmailAllowlistRepository {
    async fn list_for_pool(&self, pool_id: &str) -> Result<Vec<PoolAllowedEmail>, AppError> {
        let sql = format!("{ALLOWED_EMAIL_COLUMNS} WHERE pool_id = ? ORDER BY created_at");
        let rows = sqlx::query(&sql)
            .bind(pool_id)
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter().map(map_allowed_email).collect()
    }

    async fn find_by_id(&self, id: &str) -> Result<Option<PoolAllowedEmail>, AppError> {
        let sql = format!("{ALLOWED_EMAIL_COLUMNS} WHERE id = ?");
        let row = sqlx::query(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        row.map(map_allowed_email).transpose()
    }

    async fn is_email_allowed(&self, pool_id: &str, email: &str) -> Result<bool, AppError> {
        let row = sqlx::query(
            "SELECT COUNT(*) AS matches FROM pool_email_allowlist \
             WHERE pool_id = ? AND email = ?",
        )
        .bind(pool_id)
        .bind(email)
        .fetch_one(&self.pool)
        .await?;
        let matches: i64 = row.try_get("matches")?;
        Ok(matches > 0)
    }

    async fn add(&self, record: NewAllowedEmailRecord) -> Result<PoolAllowedEmail, AppError> {
        sqlx::query(
            "INSERT INTO pool_email_allowlist (id, pool_id, email, added_by_user_id) \
             VALUES (?, ?, ?, ?)",
        )
        .bind(&record.id)
        .bind(&record.pool_id)
        .bind(&record.email)
        .bind(&record.added_by_user_id)
        .execute(&self.pool)
        .await
        .map_err(map_email_unique)?;

        self.find_by_id(&record.id)
            .await?
            .ok_or_else(|| AppError::Internal("allowed email was not persisted".to_owned()))
    }

    async fn remove(&self, pool_id: &str, id: &str) -> Result<(), AppError> {
        sqlx::query("DELETE FROM pool_email_allowlist WHERE pool_id = ? AND id = ?")
            .bind(pool_id)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

fn map_allowed_email(row: sqlx::mysql::MySqlRow) -> Result<PoolAllowedEmail, AppError> {
    Ok(PoolAllowedEmail {
        id: mapper::id(row.try_get("id")?)?,
        pool_id: mapper::id(row.try_get("pool_id")?)?,
        email: mapper::email(row.try_get("email")?)?,
        added_by_user_id: mapper::id(row.try_get("added_by_user_id")?)?,
        created_at: mapper::datetime(row.try_get("created_at")?)?,
    })
}

fn map_email_unique(error: sqlx::Error) -> AppError {
    if let sqlx::Error::Database(database_error) = &error {
        if database_error.is_unique_violation() {
            return AppError::conflict_code(
                "email_already_allowed",
                "this email is already on the allowlist",
            );
        }
    }
    AppError::from(error)
}
