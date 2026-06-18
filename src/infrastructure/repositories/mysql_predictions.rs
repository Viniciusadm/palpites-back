use async_trait::async_trait;
use sqlx::{MySqlPool, Row};

use crate::application::predictions::{PredictionRepository, UpsertPredictionRecord};
use crate::domain::predictions::Prediction;
use crate::errors::AppError;
use crate::infrastructure::repositories::mapper;

const PREDICTION_COLUMNS: &str = "SELECT id, pool_member_id, match_id, home_score, away_score, \
     points_awarded, scored_at, created_at, updated_at FROM predictions";

#[derive(Clone)]
pub struct MySqlPredictionRepository {
    pool: MySqlPool,
}

impl MySqlPredictionRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

fn map_write_error(error: sqlx::Error) -> AppError {
    if let sqlx::Error::Database(database_error) = &error {
        if database_error.is_foreign_key_violation() {
            return AppError::conflict_code(
                "prediction_reference_invalid",
                "prediction references a pool member or match that does not exist",
            );
        }
    }
    AppError::from(error)
}

#[async_trait]
impl PredictionRepository for MySqlPredictionRepository {
    async fn upsert(&self, record: UpsertPredictionRecord) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO predictions (id, pool_member_id, match_id, home_score, away_score) \
             VALUES (?, ?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE home_score = VALUES(home_score), away_score = VALUES(away_score)",
        )
        .bind(&record.prediction_id)
        .bind(&record.pool_member_id)
        .bind(&record.match_id)
        .bind(record.home_score)
        .bind(record.away_score)
        .execute(&self.pool)
        .await
        .map_err(map_write_error)?;
        Ok(())
    }

    async fn find_for_member_and_match(
        &self,
        pool_member_id: &str,
        match_id: &str,
    ) -> Result<Option<Prediction>, AppError> {
        let sql = format!("{PREDICTION_COLUMNS} WHERE pool_member_id = ? AND match_id = ?");
        let row = sqlx::query(&sql)
            .bind(pool_member_id)
            .bind(match_id)
            .fetch_optional(&self.pool)
            .await?;
        row.map(map_prediction).transpose()
    }

    async fn list_for_member(&self, pool_member_id: &str) -> Result<Vec<Prediction>, AppError> {
        let sql = format!("{PREDICTION_COLUMNS} WHERE pool_member_id = ? ORDER BY created_at");
        let rows = sqlx::query(&sql)
            .bind(pool_member_id)
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter().map(map_prediction).collect()
    }

    async fn list_for_match(&self, match_id: &str) -> Result<Vec<Prediction>, AppError> {
        let sql = format!("{PREDICTION_COLUMNS} WHERE match_id = ? ORDER BY created_at");
        let rows = sqlx::query(&sql)
            .bind(match_id)
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter().map(map_prediction).collect()
    }
}

fn map_prediction(row: sqlx::mysql::MySqlRow) -> Result<Prediction, AppError> {
    Ok(Prediction {
        id: mapper::id(row.try_get("id")?)?,
        pool_member_id: mapper::id(row.try_get("pool_member_id")?)?,
        match_id: mapper::id(row.try_get("match_id")?)?,
        home_score: mapper::score(row.try_get("home_score")?)?,
        away_score: mapper::score(row.try_get("away_score")?)?,
        points_awarded: row.try_get("points_awarded")?,
        scored_at: mapper::opt_datetime(row.try_get("scored_at")?)?,
        created_at: mapper::datetime(row.try_get("created_at")?)?,
        updated_at: mapper::datetime(row.try_get("updated_at")?)?,
    })
}
