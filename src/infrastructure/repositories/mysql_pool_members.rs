use async_trait::async_trait;
use sqlx::{MySqlPool, Row};

use crate::application::pools::{NewMemberRecord, PoolMemberRepository};
use crate::domain::pools::PoolMember;
use crate::errors::AppError;
use crate::infrastructure::repositories::mapper;

const MEMBER_COLUMNS: &str = "SELECT id, pool_id, user_id, role, status, joined_at, left_at, \
     created_at, updated_at FROM pool_members";

#[derive(Clone)]
pub struct MySqlPoolMemberRepository {
    pool: MySqlPool,
}

impl MySqlPoolMemberRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PoolMemberRepository for MySqlPoolMemberRepository {
    async fn find_membership(
        &self,
        pool_id: &str,
        user_id: &str,
    ) -> Result<Option<PoolMember>, AppError> {
        let sql = format!("{MEMBER_COLUMNS} WHERE pool_id = ? AND user_id = ?");
        let row = sqlx::query(&sql)
            .bind(pool_id)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?;
        row.map(map_member).transpose()
    }

    async fn find_by_id(&self, member_id: &str) -> Result<Option<PoolMember>, AppError> {
        let sql = format!("{MEMBER_COLUMNS} WHERE id = ?");
        let row = sqlx::query(&sql)
            .bind(member_id)
            .fetch_optional(&self.pool)
            .await?;
        row.map(map_member).transpose()
    }

    async fn list_for_pool(&self, pool_id: &str) -> Result<Vec<PoolMember>, AppError> {
        let sql = format!("{MEMBER_COLUMNS} WHERE pool_id = ? ORDER BY joined_at");
        let rows = sqlx::query(&sql)
            .bind(pool_id)
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter().map(map_member).collect()
    }

    async fn count_active_owners(&self, pool_id: &str) -> Result<u64, AppError> {
        let row = sqlx::query(
            "SELECT COUNT(*) AS owners FROM pool_members \
             WHERE pool_id = ? AND role = 'owner' AND status = 'active'",
        )
        .bind(pool_id)
        .fetch_one(&self.pool)
        .await?;
        let owners: i64 = row.try_get("owners")?;
        Ok(owners.max(0) as u64)
    }

    async fn create(&self, record: NewMemberRecord) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO pool_members (id, pool_id, user_id, role, status, joined_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&record.member_id)
        .bind(&record.pool_id)
        .bind(&record.user_id)
        .bind(&record.role)
        .bind(&record.status)
        .bind(&record.joined_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn reactivate(&self, member_id: &str, joined_at: &str) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE pool_members \
             SET status = 'active', left_at = NULL, role = 'member', joined_at = ? \
             WHERE id = ?",
        )
        .bind(joined_at)
        .bind(member_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn set_status(
        &self,
        member_id: &str,
        status: &str,
        left_at: Option<&str>,
    ) -> Result<(), AppError> {
        sqlx::query("UPDATE pool_members SET status = ?, left_at = ? WHERE id = ?")
            .bind(status)
            .bind(left_at)
            .bind(member_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn set_role(&self, member_id: &str, role: &str) -> Result<(), AppError> {
        sqlx::query("UPDATE pool_members SET role = ? WHERE id = ?")
            .bind(role)
            .bind(member_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

fn map_member(row: sqlx::mysql::MySqlRow) -> Result<PoolMember, AppError> {
    Ok(PoolMember {
        id: mapper::id(row.try_get("id")?)?,
        pool_id: mapper::id(row.try_get("pool_id")?)?,
        user_id: mapper::id(row.try_get("user_id")?)?,
        role: mapper::pool_role(row.try_get("role")?)?,
        status: mapper::member_status(row.try_get("status")?)?,
        joined_at: mapper::datetime(row.try_get("joined_at")?)?,
        left_at: mapper::opt_datetime(row.try_get("left_at")?)?,
        created_at: mapper::datetime(row.try_get("created_at")?)?,
        updated_at: mapper::datetime(row.try_get("updated_at")?)?,
    })
}
