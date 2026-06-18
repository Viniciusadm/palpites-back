use async_trait::async_trait;
use sqlx::{MySqlPool, Row};

use crate::application::tournaments::{
    CreateAssignmentRecord, CreateGroupRecord, CreateStageRecord, CreateTournamentRecord,
    TournamentRepository, UpdateGroupRecord, UpdateStageRecord, UpdateTournamentRecord,
};
use crate::domain::tournaments::{Tournament, TournamentGroup, TournamentStage, TournamentTeam};
use crate::errors::AppError;
use crate::infrastructure::repositories::mapper;

const TOURNAMENT_COLUMNS: &str =
    "SELECT id, name, slug, season_year, starts_on, ends_on, status, created_at, updated_at \
     FROM tournaments";
const STAGE_COLUMNS: &str =
    "SELECT id, tournament_id, name, kind, ordering, created_at, updated_at FROM tournament_stages";
const GROUP_COLUMNS: &str =
    "SELECT id, tournament_id, label, created_at, updated_at FROM tournament_groups";
const ASSIGNMENT_COLUMNS: &str =
    "SELECT id, tournament_id, team_id, group_id, created_at, updated_at FROM tournament_teams";

#[derive(Clone)]
pub struct MySqlTournamentRepository {
    pool: MySqlPool,
}

impl MySqlTournamentRepository {
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
impl TournamentRepository for MySqlTournamentRepository {
    async fn list(&self) -> Result<Vec<Tournament>, AppError> {
        let sql = format!("{TOURNAMENT_COLUMNS} ORDER BY name");
        let rows = sqlx::query(&sql).fetch_all(&self.pool).await?;
        rows.into_iter().map(map_tournament).collect()
    }

    async fn find_by_id(&self, id: &str) -> Result<Option<Tournament>, AppError> {
        let sql = format!("{TOURNAMENT_COLUMNS} WHERE id = ?");
        let row = sqlx::query(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        row.map(map_tournament).transpose()
    }

    async fn find_by_slug(&self, slug: &str) -> Result<Option<Tournament>, AppError> {
        let sql = format!("{TOURNAMENT_COLUMNS} WHERE slug = ?");
        let row = sqlx::query(&sql)
            .bind(slug)
            .fetch_optional(&self.pool)
            .await?;
        row.map(map_tournament).transpose()
    }

    async fn create(&self, record: CreateTournamentRecord) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO tournaments \
             (id, name, slug, season_year, starts_on, ends_on, status) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&record.tournament_id)
        .bind(&record.name)
        .bind(&record.slug)
        .bind(record.season_year)
        .bind(&record.starts_on)
        .bind(&record.ends_on)
        .bind(&record.status)
        .execute(&self.pool)
        .await
        .map_err(|error| map_unique(error, slug_taken()))?;
        Ok(())
    }

    async fn update(&self, record: UpdateTournamentRecord) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE tournaments \
             SET name = ?, slug = ?, season_year = ?, starts_on = ?, ends_on = ?, status = ? \
             WHERE id = ?",
        )
        .bind(&record.name)
        .bind(&record.slug)
        .bind(record.season_year)
        .bind(&record.starts_on)
        .bind(&record.ends_on)
        .bind(&record.status)
        .bind(&record.tournament_id)
        .execute(&self.pool)
        .await
        .map_err(|error| map_unique(error, slug_taken()))?;
        Ok(())
    }

    async fn delete(&self, id: &str) -> Result<(), AppError> {
        sqlx::query("DELETE FROM tournaments WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn is_referenced_by_pools(&self, _id: &str) -> Result<bool, AppError> {
        Ok(false)
    }

    async fn list_stages(&self, tournament_id: &str) -> Result<Vec<TournamentStage>, AppError> {
        let sql = format!("{STAGE_COLUMNS} WHERE tournament_id = ? ORDER BY ordering, name");
        let rows = sqlx::query(&sql)
            .bind(tournament_id)
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter().map(map_stage).collect()
    }

    async fn find_stage_by_id(
        &self,
        stage_id: &str,
    ) -> Result<Option<TournamentStage>, AppError> {
        let sql = format!("{STAGE_COLUMNS} WHERE id = ?");
        let row = sqlx::query(&sql)
            .bind(stage_id)
            .fetch_optional(&self.pool)
            .await?;
        row.map(map_stage).transpose()
    }

    async fn find_stage_by_name(
        &self,
        tournament_id: &str,
        name: &str,
    ) -> Result<Option<TournamentStage>, AppError> {
        let sql = format!("{STAGE_COLUMNS} WHERE tournament_id = ? AND name = ?");
        let row = sqlx::query(&sql)
            .bind(tournament_id)
            .bind(name)
            .fetch_optional(&self.pool)
            .await?;
        row.map(map_stage).transpose()
    }

    async fn create_stage(&self, record: CreateStageRecord) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO tournament_stages (id, tournament_id, name, kind, ordering) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&record.stage_id)
        .bind(&record.tournament_id)
        .bind(&record.name)
        .bind(&record.kind)
        .bind(record.ordering)
        .execute(&self.pool)
        .await
        .map_err(|error| map_unique(error, stage_name_taken()))?;
        Ok(())
    }

    async fn update_stage(&self, record: UpdateStageRecord) -> Result<(), AppError> {
        sqlx::query("UPDATE tournament_stages SET name = ?, kind = ?, ordering = ? WHERE id = ?")
            .bind(&record.name)
            .bind(&record.kind)
            .bind(record.ordering)
            .bind(&record.stage_id)
            .execute(&self.pool)
            .await
            .map_err(|error| map_unique(error, stage_name_taken()))?;
        Ok(())
    }

    async fn delete_stage(&self, stage_id: &str) -> Result<(), AppError> {
        sqlx::query("DELETE FROM tournament_stages WHERE id = ?")
            .bind(stage_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn list_groups(&self, tournament_id: &str) -> Result<Vec<TournamentGroup>, AppError> {
        let sql = format!("{GROUP_COLUMNS} WHERE tournament_id = ? ORDER BY label");
        let rows = sqlx::query(&sql)
            .bind(tournament_id)
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter().map(map_group).collect()
    }

    async fn find_group_by_id(
        &self,
        group_id: &str,
    ) -> Result<Option<TournamentGroup>, AppError> {
        let sql = format!("{GROUP_COLUMNS} WHERE id = ?");
        let row = sqlx::query(&sql)
            .bind(group_id)
            .fetch_optional(&self.pool)
            .await?;
        row.map(map_group).transpose()
    }

    async fn find_group_by_label(
        &self,
        tournament_id: &str,
        label: &str,
    ) -> Result<Option<TournamentGroup>, AppError> {
        let sql = format!("{GROUP_COLUMNS} WHERE tournament_id = ? AND label = ?");
        let row = sqlx::query(&sql)
            .bind(tournament_id)
            .bind(label)
            .fetch_optional(&self.pool)
            .await?;
        row.map(map_group).transpose()
    }

    async fn create_group(&self, record: CreateGroupRecord) -> Result<(), AppError> {
        sqlx::query("INSERT INTO tournament_groups (id, tournament_id, label) VALUES (?, ?, ?)")
            .bind(&record.group_id)
            .bind(&record.tournament_id)
            .bind(&record.label)
            .execute(&self.pool)
            .await
            .map_err(|error| map_unique(error, group_label_taken()))?;
        Ok(())
    }

    async fn update_group(&self, record: UpdateGroupRecord) -> Result<(), AppError> {
        sqlx::query("UPDATE tournament_groups SET label = ? WHERE id = ?")
            .bind(&record.label)
            .bind(&record.group_id)
            .execute(&self.pool)
            .await
            .map_err(|error| map_unique(error, group_label_taken()))?;
        Ok(())
    }

    async fn delete_group(&self, group_id: &str) -> Result<(), AppError> {
        sqlx::query("DELETE FROM tournament_groups WHERE id = ?")
            .bind(group_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn list_teams(&self, tournament_id: &str) -> Result<Vec<TournamentTeam>, AppError> {
        let sql = format!("{ASSIGNMENT_COLUMNS} WHERE tournament_id = ? ORDER BY created_at");
        let rows = sqlx::query(&sql)
            .bind(tournament_id)
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter().map(map_assignment).collect()
    }

    async fn find_assignment_by_id(
        &self,
        assignment_id: &str,
    ) -> Result<Option<TournamentTeam>, AppError> {
        let sql = format!("{ASSIGNMENT_COLUMNS} WHERE id = ?");
        let row = sqlx::query(&sql)
            .bind(assignment_id)
            .fetch_optional(&self.pool)
            .await?;
        row.map(map_assignment).transpose()
    }

    async fn find_assignment(
        &self,
        tournament_id: &str,
        team_id: &str,
    ) -> Result<Option<TournamentTeam>, AppError> {
        let sql = format!("{ASSIGNMENT_COLUMNS} WHERE tournament_id = ? AND team_id = ?");
        let row = sqlx::query(&sql)
            .bind(tournament_id)
            .bind(team_id)
            .fetch_optional(&self.pool)
            .await?;
        row.map(map_assignment).transpose()
    }

    async fn create_assignment(&self, record: CreateAssignmentRecord) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO tournament_teams (id, tournament_id, team_id, group_id) \
             VALUES (?, ?, ?, ?)",
        )
        .bind(&record.assignment_id)
        .bind(&record.tournament_id)
        .bind(&record.team_id)
        .bind(&record.group_id)
        .execute(&self.pool)
        .await
        .map_err(|error| map_unique(error, team_already_assigned()))?;
        Ok(())
    }

    async fn delete_assignment(&self, assignment_id: &str) -> Result<(), AppError> {
        sqlx::query("DELETE FROM tournament_teams WHERE id = ?")
            .bind(assignment_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn team_exists(&self, team_id: &str) -> Result<bool, AppError> {
        let row = sqlx::query("SELECT 1 FROM teams WHERE id = ?")
            .bind(team_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.is_some())
    }
}

fn map_tournament(row: sqlx::mysql::MySqlRow) -> Result<Tournament, AppError> {
    Ok(Tournament {
        id: mapper::id(row.try_get("id")?)?,
        name: mapper::non_empty(row.try_get("name")?, "tournament.name")?,
        slug: mapper::slug(row.try_get("slug")?)?,
        season_year: row.try_get("season_year")?,
        starts_on: mapper::opt_date(row.try_get("starts_on")?)?,
        ends_on: mapper::opt_date(row.try_get("ends_on")?)?,
        status: mapper::tournament_status(row.try_get("status")?)?,
        created_at: mapper::datetime(row.try_get("created_at")?)?,
        updated_at: mapper::datetime(row.try_get("updated_at")?)?,
    })
}

fn map_stage(row: sqlx::mysql::MySqlRow) -> Result<TournamentStage, AppError> {
    Ok(TournamentStage {
        id: mapper::id(row.try_get("id")?)?,
        tournament_id: mapper::id(row.try_get("tournament_id")?)?,
        name: mapper::non_empty(row.try_get("name")?, "stage.name")?,
        kind: mapper::stage_kind(row.try_get("kind")?)?,
        ordering: row.try_get("ordering")?,
        created_at: mapper::datetime(row.try_get("created_at")?)?,
        updated_at: mapper::datetime(row.try_get("updated_at")?)?,
    })
}

fn map_group(row: sqlx::mysql::MySqlRow) -> Result<TournamentGroup, AppError> {
    Ok(TournamentGroup {
        id: mapper::id(row.try_get("id")?)?,
        tournament_id: mapper::id(row.try_get("tournament_id")?)?,
        label: mapper::non_empty(row.try_get("label")?, "group.label")?,
        created_at: mapper::datetime(row.try_get("created_at")?)?,
        updated_at: mapper::datetime(row.try_get("updated_at")?)?,
    })
}

fn map_assignment(row: sqlx::mysql::MySqlRow) -> Result<TournamentTeam, AppError> {
    Ok(TournamentTeam {
        id: mapper::id(row.try_get("id")?)?,
        tournament_id: mapper::id(row.try_get("tournament_id")?)?,
        team_id: mapper::id(row.try_get("team_id")?)?,
        group_id: mapper::opt_id(row.try_get("group_id")?)?,
        created_at: mapper::datetime(row.try_get("created_at")?)?,
        updated_at: mapper::datetime(row.try_get("updated_at")?)?,
    })
}

fn slug_taken() -> AppError {
    AppError::conflict_code("tournament_slug_taken", "tournament slug is already in use")
}

fn stage_name_taken() -> AppError {
    AppError::conflict_code("stage_name_taken", "stage name is already in use")
}

fn group_label_taken() -> AppError {
    AppError::conflict_code("group_label_taken", "group label is already in use")
}

fn team_already_assigned() -> AppError {
    AppError::conflict_code(
        "team_already_assigned",
        "team is already assigned to this tournament",
    )
}
