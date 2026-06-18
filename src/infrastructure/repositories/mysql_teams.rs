use async_trait::async_trait;
use sqlx::{MySqlPool, Row};

use crate::application::teams::{CreateTeamRecord, TeamRepository, UpdateTeamRecord};
use crate::domain::teams::Team;
use crate::errors::AppError;
use crate::infrastructure::repositories::mapper;

const SELECT_COLUMNS: &str =
    "SELECT id, name, code, flag_emoji, flag_file_id, created_at, updated_at FROM teams";

#[derive(Clone)]
pub struct MySqlTeamRepository {
    pool: MySqlPool,
}

impl MySqlTeamRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    async fn find_by_column(&self, column: &str, value: &str) -> Result<Option<Team>, AppError> {
        let sql = format!("{SELECT_COLUMNS} WHERE {column} = ?");
        let row = sqlx::query(&sql)
            .bind(value)
            .fetch_optional(&self.pool)
            .await?;

        row.map(map_team).transpose()
    }

    fn map_write_error(error: sqlx::Error) -> AppError {
        if let sqlx::Error::Database(database_error) = &error {
            if database_error.is_unique_violation() {
                return AppError::conflict_code("team_code_taken", "team code is already in use");
            }
            if database_error.is_foreign_key_violation() {
                return AppError::conflict_code(
                    "team_in_use",
                    "team is referenced by a tournament or match",
                );
            }
        }
        AppError::from(error)
    }
}

#[async_trait]
impl TeamRepository for MySqlTeamRepository {
    async fn list(&self) -> Result<Vec<Team>, AppError> {
        let sql = format!("{SELECT_COLUMNS} ORDER BY name");
        let rows = sqlx::query(&sql).fetch_all(&self.pool).await?;
        rows.into_iter().map(map_team).collect()
    }

    async fn find_by_id(&self, id: &str) -> Result<Option<Team>, AppError> {
        self.find_by_column("id", id).await
    }

    async fn find_by_code(&self, code: &str) -> Result<Option<Team>, AppError> {
        self.find_by_column("code", code).await
    }

    async fn create(&self, record: CreateTeamRecord) -> Result<(), AppError> {
        sqlx::query("INSERT INTO teams (id, name, code, flag_emoji) VALUES (?, ?, ?, ?)")
            .bind(&record.team_id)
            .bind(&record.name)
            .bind(&record.code)
            .bind(&record.flag_emoji)
            .execute(&self.pool)
            .await
            .map_err(Self::map_write_error)?;
        Ok(())
    }

    async fn update(&self, record: UpdateTeamRecord) -> Result<(), AppError> {
        sqlx::query("UPDATE teams SET name = ?, code = ?, flag_emoji = ? WHERE id = ?")
            .bind(&record.name)
            .bind(&record.code)
            .bind(&record.flag_emoji)
            .bind(&record.team_id)
            .execute(&self.pool)
            .await
            .map_err(Self::map_write_error)?;
        Ok(())
    }

    async fn delete(&self, id: &str) -> Result<(), AppError> {
        sqlx::query("DELETE FROM teams WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(Self::map_write_error)?;
        Ok(())
    }

    async fn is_referenced(&self, _id: &str) -> Result<bool, AppError> {
        Ok(false)
    }
}

fn map_team(row: sqlx::mysql::MySqlRow) -> Result<Team, AppError> {
    Ok(Team {
        id: mapper::id(row.try_get("id")?)?,
        name: mapper::non_empty(row.try_get("name")?, "team.name")?,
        code: mapper::team_code(row.try_get("code")?)?,
        flag_emoji: mapper::opt_non_empty(row.try_get("flag_emoji")?, "team.flag_emoji")?,
        flag_file_id: mapper::opt_id(row.try_get("flag_file_id")?)?,
        created_at: mapper::datetime(row.try_get("created_at")?)?,
        updated_at: mapper::datetime(row.try_get("updated_at")?)?,
    })
}
