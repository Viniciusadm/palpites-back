use async_trait::async_trait;
use sqlx::{MySqlPool, Row};

use crate::application::matches::{
    CreateMatchRecord, MatchFilters, MatchRepository, UpdateMatchRecord,
};
use crate::domain::matches::Match;
use crate::errors::AppError;
use crate::infrastructure::repositories::mapper;

pub(crate) const MATCH_COLUMNS: &str =
    "SELECT id, tournament_id, stage_id, home_team_id, away_team_id, \
     kickoff_at, status, home_score, away_score, can_go_to_penalties, penalties_winner, \
     finished_at, created_at, updated_at FROM matches";

#[derive(Clone)]
pub struct MySqlMatchRepository {
    pool: MySqlPool,
}

impl MySqlMatchRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

fn map_write_error(error: sqlx::Error) -> AppError {
    if let sqlx::Error::Database(database_error) = &error {
        if database_error.is_foreign_key_violation() {
            return AppError::conflict_code(
                "match_reference_invalid",
                "match references a tournament, stage or team that does not exist",
            );
        }
    }
    AppError::from(error)
}

#[async_trait]
impl MatchRepository for MySqlMatchRepository {
    async fn list(
        &self,
        tournament_id: &str,
        filters: MatchFilters,
    ) -> Result<Vec<Match>, AppError> {
        let mut sql = format!("{MATCH_COLUMNS} WHERE tournament_id = ?");
        if filters.stage_id.is_some() {
            sql.push_str(" AND stage_id = ?");
        }
        if filters.status.is_some() {
            sql.push_str(" AND status = ?");
        }
        if filters.date_from.is_some() {
            sql.push_str(" AND kickoff_at >= ?");
        }
        if filters.date_to.is_some() {
            sql.push_str(" AND kickoff_at <= ?");
        }
        sql.push_str(" ORDER BY kickoff_at");

        let mut query = sqlx::query(&sql).bind(tournament_id);
        if let Some(stage_id) = &filters.stage_id {
            query = query.bind(stage_id);
        }
        if let Some(status) = &filters.status {
            query = query.bind(status);
        }
        if let Some(date_from) = &filters.date_from {
            query = query.bind(date_from);
        }
        if let Some(date_to) = &filters.date_to {
            query = query.bind(date_to);
        }

        let rows = query.fetch_all(&self.pool).await?;
        rows.into_iter().map(map_match).collect()
    }

    async fn find_by_id(&self, id: &str) -> Result<Option<Match>, AppError> {
        let sql = format!("{MATCH_COLUMNS} WHERE id = ?");
        let row = sqlx::query(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        row.map(map_match).transpose()
    }

    async fn create(&self, record: CreateMatchRecord) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO matches \
             (id, tournament_id, stage_id, home_team_id, away_team_id, kickoff_at, status, can_go_to_penalties) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&record.match_id)
        .bind(&record.tournament_id)
        .bind(&record.stage_id)
        .bind(&record.home_team_id)
        .bind(&record.away_team_id)
        .bind(&record.kickoff_at)
        .bind(&record.status)
        .bind(record.can_go_to_penalties)
        .execute(&self.pool)
        .await
        .map_err(map_write_error)?;
        Ok(())
    }

    async fn update(&self, record: UpdateMatchRecord) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE matches \
             SET stage_id = ?, home_team_id = ?, away_team_id = ?, kickoff_at = ?, status = ?, \
             can_go_to_penalties = ? \
             WHERE id = ?",
        )
        .bind(&record.stage_id)
        .bind(&record.home_team_id)
        .bind(&record.away_team_id)
        .bind(&record.kickoff_at)
        .bind(&record.status)
        .bind(record.can_go_to_penalties)
        .bind(&record.match_id)
        .execute(&self.pool)
        .await
        .map_err(map_write_error)?;
        Ok(())
    }

    async fn delete(&self, id: &str) -> Result<(), AppError> {
        sqlx::query("DELETE FROM matches WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn tournament_exists(&self, tournament_id: &str) -> Result<bool, AppError> {
        let row = sqlx::query("SELECT 1 FROM tournaments WHERE id = ?")
            .bind(tournament_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.is_some())
    }

    async fn find_stage_tournament_id(
        &self,
        stage_id: &str,
    ) -> Result<Option<String>, AppError> {
        let row = sqlx::query("SELECT tournament_id FROM tournament_stages WHERE id = ?")
            .bind(stage_id)
            .fetch_optional(&self.pool)
            .await?;
        match row {
            Some(row) => Ok(Some(row.try_get("tournament_id")?)),
            None => Ok(None),
        }
    }

    async fn is_team_in_tournament(
        &self,
        tournament_id: &str,
        team_id: &str,
    ) -> Result<bool, AppError> {
        let row = sqlx::query("SELECT 1 FROM tournament_teams WHERE tournament_id = ? AND team_id = ?")
            .bind(tournament_id)
            .bind(team_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.is_some())
    }
}

pub(crate) fn map_match(row: sqlx::mysql::MySqlRow) -> Result<Match, AppError> {
    Ok(Match {
        id: mapper::id(row.try_get("id")?)?,
        tournament_id: mapper::id(row.try_get("tournament_id")?)?,
        stage_id: mapper::id(row.try_get("stage_id")?)?,
        home_team_id: mapper::opt_id(row.try_get("home_team_id")?)?,
        away_team_id: mapper::opt_id(row.try_get("away_team_id")?)?,
        kickoff_at: mapper::datetime(row.try_get("kickoff_at")?)?,
        status: mapper::match_status(row.try_get("status")?)?,
        home_score: mapper::opt_score(row.try_get("home_score")?)?,
        away_score: mapper::opt_score(row.try_get("away_score")?)?,
        can_go_to_penalties: row.try_get("can_go_to_penalties")?,
        penalties_winner: mapper::opt_penalty_side(row.try_get("penalties_winner")?)?,
        finished_at: mapper::opt_datetime(row.try_get("finished_at")?)?,
        created_at: mapper::datetime(row.try_get("created_at")?)?,
        updated_at: mapper::datetime(row.try_get("updated_at")?)?,
    })
}
