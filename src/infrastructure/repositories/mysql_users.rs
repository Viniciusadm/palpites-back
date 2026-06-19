use async_trait::async_trait;
use sqlx::{MySqlPool, Row};

use crate::application::auth::{RegisterUserRecord, UserRepository};
use crate::domain::users::User;
use crate::errors::AppError;
use crate::infrastructure::repositories::mapper;

#[derive(Clone)]
pub struct MySqlUserRepository {
    pool: MySqlPool,
}

impl MySqlUserRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    async fn find_by_column(&self, column: &str, value: &str) -> Result<Option<User>, AppError> {
        let sql = format!(
            "SELECT id, email, password_hash, display_name, role, avatar_file_id, sync_predictions_across_pools, is_active, last_login_at, created_at, updated_at FROM users WHERE {column} = ?"
        );
        let row = sqlx::query(&sql)
            .bind(value)
            .fetch_optional(&self.pool)
            .await?;

        row.map(|row| {
            Ok(User {
                id: mapper::id(row.try_get("id")?)?,
                email: mapper::email(row.try_get("email")?)?,
                password_hash: mapper::non_empty(
                    row.try_get("password_hash")?,
                    "user.password_hash",
                )?,
                display_name: mapper::non_empty(row.try_get("display_name")?, "user.display_name")?,
                role: mapper::user_role(row.try_get("role")?)?,
                avatar_file_id: mapper::opt_id(row.try_get("avatar_file_id")?)?,
                sync_predictions_across_pools: row.try_get("sync_predictions_across_pools")?,
                is_active: row.try_get("is_active")?,
                last_login_at: mapper::opt_datetime(row.try_get("last_login_at")?)?,
                created_at: mapper::datetime(row.try_get("created_at")?)?,
                updated_at: mapper::datetime(row.try_get("updated_at")?)?,
            })
        })
        .transpose()
    }

    fn map_write_error(error: sqlx::Error) -> AppError {
        if let sqlx::Error::Database(database_error) = &error {
            if database_error.is_unique_violation() {
                return AppError::Conflict("user already exists".to_owned());
            }
        }
        AppError::from(error)
    }
}

#[async_trait]
impl UserRepository for MySqlUserRepository {
    async fn find_by_id(&self, id: &str) -> Result<Option<User>, AppError> {
        self.find_by_column("id", id).await
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError> {
        self.find_by_column("email", email).await
    }

    async fn register_user(&self, record: RegisterUserRecord) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO users (id, email, password_hash, display_name, is_active) VALUES (?, ?, ?, ?, TRUE)",
        )
        .bind(&record.user_id)
        .bind(&record.email)
        .bind(&record.password_hash)
        .bind(&record.display_name)
        .execute(&self.pool)
        .await
        .map_err(Self::map_write_error)?;
        Ok(())
    }

    async fn set_sync_predictions_across_pools(
        &self,
        user_id: &str,
        value: bool,
    ) -> Result<(), AppError> {
        sqlx::query("UPDATE users SET sync_predictions_across_pools = ? WHERE id = ?")
            .bind(value)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
