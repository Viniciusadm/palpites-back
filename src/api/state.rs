use sqlx::MySqlPool;

use crate::infrastructure::storage::FileStorageSettings;

#[derive(Debug, Clone)]
pub struct AppState {
    pub db: Option<MySqlPool>,
    pub jwt_secret: String,
    pub file_storage: FileStorageSettings,
}

impl AppState {
    pub fn new(
        db: Option<MySqlPool>,
        jwt_secret: impl Into<String>,
        file_storage: FileStorageSettings,
    ) -> Self {
        Self {
            db,
            jwt_secret: jwt_secret.into(),
            file_storage,
        }
    }

    pub fn db(&self) -> Result<MySqlPool, crate::errors::AppError> {
        self.db.clone().ok_or_else(|| {
            crate::errors::AppError::Configuration("DATABASE_URL is not configured".to_owned())
        })
    }
}
