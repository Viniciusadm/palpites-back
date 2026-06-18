use async_trait::async_trait;
use sqlx::{MySqlPool, Row};

use crate::application::pools::{NewScoringRuleRecord, ScoringRuleRepository};
use crate::database::transaction::DatabaseTransaction;
use crate::domain::pools::PoolScoringRule;
use crate::errors::AppError;
use crate::infrastructure::repositories::mapper;

const RULE_COLUMNS: &str =
    "SELECT id, pool_id, rule_key, points, created_at, updated_at FROM pool_scoring_rules";

#[derive(Clone)]
pub struct MySqlScoringRuleRepository {
    pool: MySqlPool,
}

impl MySqlScoringRuleRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ScoringRuleRepository for MySqlScoringRuleRepository {
    async fn list_for_pool(&self, pool_id: &str) -> Result<Vec<PoolScoringRule>, AppError> {
        let sql = format!("{RULE_COLUMNS} WHERE pool_id = ? ORDER BY rule_key");
        let rows = sqlx::query(&sql)
            .bind(pool_id)
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter().map(map_rule).collect()
    }

    async fn set_rules(
        &self,
        pool_id: &str,
        rules: Vec<NewScoringRuleRecord>,
    ) -> Result<(), AppError> {
        let mut tx: DatabaseTransaction = self.pool.begin().await?;
        for rule in &rules {
            sqlx::query(
                "INSERT INTO pool_scoring_rules (id, pool_id, rule_key, points) \
                 VALUES (?, ?, ?, ?) \
                 ON DUPLICATE KEY UPDATE points = VALUES(points)",
            )
            .bind(&rule.rule_id)
            .bind(pool_id)
            .bind(&rule.rule_key)
            .bind(rule.points)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }
}

fn map_rule(row: sqlx::mysql::MySqlRow) -> Result<PoolScoringRule, AppError> {
    Ok(PoolScoringRule {
        id: mapper::id(row.try_get("id")?)?,
        pool_id: mapper::id(row.try_get("pool_id")?)?,
        rule_key: mapper::scoring_rule_key(row.try_get("rule_key")?)?,
        points: row.try_get("points")?,
        created_at: mapper::datetime(row.try_get("created_at")?)?,
        updated_at: mapper::datetime(row.try_get("updated_at")?)?,
    })
}
