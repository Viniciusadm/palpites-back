use async_trait::async_trait;
use sqlx::{MySqlPool, Row};

use crate::application::pools::{CreatePoolSeed, PoolRepository, UpdatePoolRecord};
use crate::database::transaction::DatabaseTransaction;
use crate::domain::pools::Pool;
use crate::errors::AppError;
use crate::infrastructure::repositories::mapper;

const POOL_COLUMNS: &str = "SELECT id, tournament_id, owner_user_id, name, invite_code, \
     join_requires_allowlist, prediction_lock_offset_minutes, status, created_at, updated_at \
     FROM pools";

#[derive(Clone)]
pub struct MySqlPoolRepository {
    pool: MySqlPool,
}

impl MySqlPoolRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

fn map_unique(error: sqlx::Error, on_unique: AppError) -> AppError {
    if let sqlx::Error::Database(database_error) = &error {
        if database_error.is_unique_violation() {
            return on_unique;
        }
    }
    AppError::from(error)
}

#[async_trait]
impl PoolRepository for MySqlPoolRepository {
    async fn find_by_id(&self, id: &str) -> Result<Option<Pool>, AppError> {
        let sql = format!("{POOL_COLUMNS} WHERE id = ?");
        let row = sqlx::query(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        row.map(map_pool).transpose()
    }

    async fn find_by_invite_code(&self, invite_code: &str) -> Result<Option<Pool>, AppError> {
        let sql = format!("{POOL_COLUMNS} WHERE invite_code = ?");
        let row = sqlx::query(&sql)
            .bind(invite_code)
            .fetch_optional(&self.pool)
            .await?;
        row.map(map_pool).transpose()
    }

    async fn list_for_user(&self, user_id: &str) -> Result<Vec<Pool>, AppError> {
        let sql = "SELECT pools.id, pools.tournament_id, pools.owner_user_id, pools.name, \
             pools.invite_code, pools.join_requires_allowlist, \
             pools.prediction_lock_offset_minutes, pools.status, pools.created_at, pools.updated_at \
             FROM pools \
             JOIN pool_members ON pool_members.pool_id = pools.id \
             WHERE pool_members.user_id = ? AND pool_members.status = 'active' \
             ORDER BY pools.name";
        let rows = sqlx::query(sql)
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter().map(map_pool).collect()
    }

    async fn list_for_tournament(&self, tournament_id: &str) -> Result<Vec<Pool>, AppError> {
        let sql = format!("{POOL_COLUMNS} WHERE tournament_id = ? ORDER BY name");
        let rows = sqlx::query(&sql)
            .bind(tournament_id)
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter().map(map_pool).collect()
    }

    async fn find_tournament_status(
        &self,
        tournament_id: &str,
    ) -> Result<Option<String>, AppError> {
        let row = sqlx::query("SELECT status FROM tournaments WHERE id = ?")
            .bind(tournament_id)
            .fetch_optional(&self.pool)
            .await?;
        row.map(|row| Ok(row.try_get::<String, _>("status")?)).transpose()
    }

    async fn create_with_seed(&self, seed: CreatePoolSeed) -> Result<(), AppError> {
        let mut tx: DatabaseTransaction = self.pool.begin().await?;

        sqlx::query(
            "INSERT INTO pools \
             (id, tournament_id, owner_user_id, name, invite_code, \
              join_requires_allowlist, prediction_lock_offset_minutes, status) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&seed.pool.pool_id)
        .bind(&seed.pool.tournament_id)
        .bind(&seed.pool.owner_user_id)
        .bind(&seed.pool.name)
        .bind(&seed.pool.invite_code)
        .bind(seed.pool.join_requires_allowlist)
        .bind(seed.pool.prediction_lock_offset_minutes)
        .bind(&seed.pool.status)
        .execute(&mut *tx)
        .await
        .map_err(|error| map_unique(error, invite_code_taken()))?;

        for rule in &seed.scoring_rules {
            sqlx::query(
                "INSERT INTO pool_scoring_rules (id, pool_id, rule_key, points) \
                 VALUES (?, ?, ?, ?)",
            )
            .bind(&rule.rule_id)
            .bind(&seed.pool.pool_id)
            .bind(&rule.rule_key)
            .bind(rule.points)
            .execute(&mut *tx)
            .await?;
        }

        sqlx::query(
            "INSERT INTO pool_members (id, pool_id, user_id, role, status, joined_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&seed.owner_member.member_id)
        .bind(&seed.owner_member.pool_id)
        .bind(&seed.owner_member.user_id)
        .bind(&seed.owner_member.role)
        .bind(&seed.owner_member.status)
        .bind(&seed.owner_member.joined_at)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn update_settings(&self, record: UpdatePoolRecord) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE pools \
             SET name = ?, join_requires_allowlist = ?, \
                 prediction_lock_offset_minutes = ?, status = ? \
             WHERE id = ?",
        )
        .bind(&record.name)
        .bind(record.join_requires_allowlist)
        .bind(record.prediction_lock_offset_minutes)
        .bind(&record.status)
        .bind(&record.pool_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete(&self, id: &str) -> Result<(), AppError> {
        sqlx::query("DELETE FROM pools WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

fn map_pool(row: sqlx::mysql::MySqlRow) -> Result<Pool, AppError> {
    Ok(Pool {
        id: mapper::id(row.try_get("id")?)?,
        tournament_id: mapper::id(row.try_get("tournament_id")?)?,
        owner_user_id: mapper::id(row.try_get("owner_user_id")?)?,
        name: mapper::non_empty(row.try_get("name")?, "pool.name")?,
        invite_code: mapper::invite_code(row.try_get("invite_code")?)?,
        join_requires_allowlist: row.try_get("join_requires_allowlist")?,
        prediction_lock_offset_minutes: row.try_get("prediction_lock_offset_minutes")?,
        status: mapper::pool_status(row.try_get("status")?)?,
        created_at: mapper::datetime(row.try_get("created_at")?)?,
        updated_at: mapper::datetime(row.try_get("updated_at")?)?,
    })
}

fn invite_code_taken() -> AppError {
    AppError::conflict_code("invite_code_taken", "invite code is already in use")
}
