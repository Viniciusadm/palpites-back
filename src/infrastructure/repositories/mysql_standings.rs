use async_trait::async_trait;
use sqlx::{MySqlPool, Row};

use crate::application::results::{
    PoolRecompute, PredictionLine, ResultApplication, StandingRepository, StandingWithName,
};
use crate::database::transaction::DatabaseTransaction;
use crate::domain::standings::Standing;
use crate::errors::AppError;
use crate::infrastructure::repositories::mapper;

const STANDING_COLUMNS: &str = "SELECT id, pool_id, pool_member_id, total_points, exact_count, \
     outcome_count, hits_count, penalties_count, position, updated_at FROM pool_standings";

#[derive(Clone)]
pub struct MySqlStandingRepository {
    pool: MySqlPool,
}

impl MySqlStandingRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl StandingRepository for MySqlStandingRepository {
    async fn list_for_pool(&self, pool_id: &str) -> Result<Vec<Standing>, AppError> {
        let sql = format!("{STANDING_COLUMNS} WHERE pool_id = ? ORDER BY position, total_points DESC");
        let rows = sqlx::query(&sql)
            .bind(pool_id)
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter().map(map_standing).collect()
    }

    async fn list_for_pool_with_names(
        &self,
        pool_id: &str,
    ) -> Result<Vec<StandingWithName>, AppError> {
        let sql = "SELECT s.id, s.pool_id, s.pool_member_id, s.total_points, s.exact_count, \
             s.outcome_count, s.hits_count, s.penalties_count, s.position, s.updated_at, u.display_name \
             FROM pool_standings s \
             JOIN pool_members pm ON pm.id = s.pool_member_id \
             JOIN users u ON u.id = pm.user_id \
             WHERE s.pool_id = ? ORDER BY s.position, s.total_points DESC";
        let rows = sqlx::query(sql)
            .bind(pool_id)
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter().map(map_standing_with_name).collect()
    }

    async fn prediction_lines_for_pool(
        &self,
        pool_id: &str,
    ) -> Result<Vec<PredictionLine>, AppError> {
        let sql = "SELECT p.id AS prediction_id, p.pool_member_id, p.match_id, \
             p.home_score AS prediction_home, p.away_score AS prediction_away, \
             p.penalties_pick AS prediction_penalties_pick, \
             m.status AS match_status, m.kickoff_at, \
             m.home_score AS result_home, m.away_score AS result_away, \
             m.penalties_winner AS result_penalties_winner \
             FROM predictions p \
             JOIN pool_members pm ON pm.id = p.pool_member_id \
             JOIN matches m ON m.id = p.match_id \
             WHERE pm.pool_id = ?";
        let rows = sqlx::query(sql)
            .bind(pool_id)
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter().map(map_prediction_line).collect()
    }

    async fn affected_pools_for_match(&self, match_id: &str) -> Result<Vec<String>, AppError> {
        let sql = "SELECT DISTINCT pm.pool_id \
             FROM predictions p \
             JOIN pool_members pm ON pm.id = p.pool_member_id \
             WHERE p.match_id = ?";
        let rows = sqlx::query(sql)
            .bind(match_id)
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter()
            .map(|row| Ok(row.try_get::<String, _>("pool_id")?))
            .collect()
    }

    async fn apply_result(&self, application: ResultApplication) -> Result<(), AppError> {
        let mut tx: DatabaseTransaction = self.pool.begin().await?;

        sqlx::query(
            "UPDATE matches \
             SET home_score = ?, away_score = ?, penalties_winner = ?, status = 'finished', finished_at = ? \
             WHERE id = ?",
        )
        .bind(application.home_score)
        .bind(application.away_score)
        .bind(application.penalties_winner.map(|side| side.as_str()))
        .bind(&application.finished_at)
        .bind(&application.match_id)
        .execute(&mut *tx)
        .await?;

        for pool in &application.pools {
            write_pool_recompute(&mut tx, pool).await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn rescore_pool(&self, recompute: PoolRecompute) -> Result<(), AppError> {
        let mut tx: DatabaseTransaction = self.pool.begin().await?;
        write_pool_recompute(&mut tx, &recompute).await?;
        tx.commit().await?;
        Ok(())
    }
}

async fn write_pool_recompute(
    tx: &mut DatabaseTransaction<'_>,
    pool: &PoolRecompute,
) -> Result<(), AppError> {
    for scored in &pool.scored_predictions {
        sqlx::query("UPDATE predictions SET points_awarded = ?, scored_at = ? WHERE id = ?")
            .bind(scored.points_awarded)
            .bind(&scored.scored_at)
            .bind(&scored.prediction_id)
            .execute(&mut **tx)
            .await?;
    }

    for standing in &pool.standings {
        sqlx::query(
            "INSERT INTO pool_standings \
             (id, pool_id, pool_member_id, total_points, exact_count, outcome_count, \
              hits_count, penalties_count, position) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE \
              total_points = VALUES(total_points), exact_count = VALUES(exact_count), \
              outcome_count = VALUES(outcome_count), hits_count = VALUES(hits_count), \
              penalties_count = VALUES(penalties_count), position = VALUES(position)",
        )
        .bind(&standing.standing_id)
        .bind(&standing.pool_id)
        .bind(&standing.pool_member_id)
        .bind(standing.total_points)
        .bind(standing.exact_count)
        .bind(standing.outcome_count)
        .bind(standing.hits_count)
        .bind(standing.penalties_count)
        .bind(standing.position)
        .execute(&mut **tx)
        .await?;
    }

    Ok(())
}

fn map_standing(row: sqlx::mysql::MySqlRow) -> Result<Standing, AppError> {
    Ok(Standing {
        id: mapper::id(row.try_get("id")?)?,
        pool_id: mapper::id(row.try_get("pool_id")?)?,
        pool_member_id: mapper::id(row.try_get("pool_member_id")?)?,
        total_points: row.try_get("total_points")?,
        exact_count: row.try_get("exact_count")?,
        outcome_count: row.try_get("outcome_count")?,
        hits_count: row.try_get("hits_count")?,
        penalties_count: row.try_get("penalties_count")?,
        position: row.try_get("position")?,
        updated_at: mapper::datetime(row.try_get("updated_at")?)?,
    })
}

fn map_standing_with_name(row: sqlx::mysql::MySqlRow) -> Result<StandingWithName, AppError> {
    let display_name: String = row.try_get("display_name")?;
    Ok(StandingWithName {
        standing: map_standing(row)?,
        display_name,
    })
}

fn map_prediction_line(row: sqlx::mysql::MySqlRow) -> Result<PredictionLine, AppError> {
    Ok(PredictionLine {
        prediction_id: row.try_get("prediction_id")?,
        pool_member_id: row.try_get("pool_member_id")?,
        match_id: row.try_get("match_id")?,
        prediction_home: row.try_get("prediction_home")?,
        prediction_away: row.try_get("prediction_away")?,
        prediction_penalties_pick: mapper::opt_penalty_side(
            row.try_get("prediction_penalties_pick")?,
        )?,
        match_status: row.try_get("match_status")?,
        kickoff_at: mapper::datetime(row.try_get("kickoff_at")?)?.as_str().to_owned(),
        result_home: row.try_get("result_home")?,
        result_away: row.try_get("result_away")?,
        result_penalties_winner: mapper::opt_penalty_side(
            row.try_get("result_penalties_winner")?,
        )?,
    })
}
