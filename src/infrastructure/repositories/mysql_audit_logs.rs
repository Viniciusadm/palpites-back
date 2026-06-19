use sqlx::MySqlPool;
use uuid::Uuid;

use crate::errors::AppError;

#[derive(Clone)]
pub struct MySqlAuditLogRepository {
    pool: MySqlPool,
}

#[derive(Debug, Clone)]
pub struct NewAuditLog {
    pub user_id: Option<String>,
    pub method: String,
    pub path: String,
    pub query_string: Option<String>,
    /// JSON compacto válido, ou `None` quando não há corpo (ou ele não é JSON).
    pub request_body: Option<String>,
    pub status_code: u16,
}

impl MySqlAuditLogRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, entry: NewAuditLog) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO admin_audit_logs \
             (id, user_id, method, path, query_string, request_body, status_code) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(entry.user_id)
        .bind(entry.method)
        .bind(entry.path)
        .bind(entry.query_string)
        .bind(entry.request_body)
        .bind(entry.status_code)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
