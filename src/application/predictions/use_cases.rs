use chrono::Duration;
use uuid::Uuid;

use crate::application::matches::MatchRepository;
use crate::application::pools::{PoolMemberRepository, PoolRepository};
use crate::application::predictions::{
    PredictionRepository, UpsertPrediction, UpsertPredictionRecord,
};
use crate::application::shared::{parse_datetime, Clock};
use crate::domain::matches::MatchStatus;
use crate::domain::pools::MemberStatus;
use crate::domain::predictions::Prediction;
use crate::domain::Score;
use crate::errors::AppError;

pub struct PredictionUseCases<PredR, MatchR, PoolR, MemberR, C> {
    predictions: PredR,
    matches: MatchR,
    pools: PoolR,
    members: MemberR,
    clock: C,
}

impl<PredR, MatchR, PoolR, MemberR, C> PredictionUseCases<PredR, MatchR, PoolR, MemberR, C>
where
    PredR: PredictionRepository,
    MatchR: MatchRepository,
    PoolR: PoolRepository,
    MemberR: PoolMemberRepository,
    C: Clock,
{
    pub fn new(predictions: PredR, matches: MatchR, pools: PoolR, members: MemberR, clock: C) -> Self {
        Self {
            predictions,
            matches,
            pools,
            members,
            clock,
        }
    }

    pub async fn list_mine(
        &self,
        pool_id: &str,
        user_id: &str,
    ) -> Result<Vec<Prediction>, AppError> {
        let member_id = self.resolve_active_member(pool_id, user_id).await?;
        self.predictions.list_for_member(&member_id).await
    }

    pub async fn upsert(
        &self,
        pool_id: &str,
        user_id: &str,
        match_id: &str,
        command: UpsertPrediction,
    ) -> Result<Prediction, AppError> {
        let member_id = self.resolve_active_member(pool_id, user_id).await?;

        let pool = self
            .pools
            .find_by_id(pool_id)
            .await?
            .ok_or_else(|| AppError::NotFound("pool was not found".to_owned()))?;

        let game = self
            .matches
            .find_by_id(match_id)
            .await?
            .ok_or_else(|| AppError::NotFound("match was not found".to_owned()))?;

        if game.tournament_id.as_str() != pool.tournament_id.as_str() {
            return Err(AppError::conflict_code(
                "match_not_in_pool_tournament",
                "match does not belong to this pool's tournament",
            ));
        }

        if game.home_team_id.is_none() || game.away_team_id.is_none() {
            return Err(AppError::validation_code(
                "match_teams_undefined",
                "predictions are not open until both teams are defined",
            ));
        }

        let home_score = Score::new(command.home_score)?;
        let away_score = Score::new(command.away_score)?;

        self.ensure_open(&game.status, game.kickoff_at.as_str(), pool.prediction_lock_offset_minutes)?;

        self.predictions
            .upsert(UpsertPredictionRecord {
                prediction_id: Uuid::new_v4().to_string(),
                pool_member_id: member_id.clone(),
                match_id: match_id.to_owned(),
                home_score: home_score.value(),
                away_score: away_score.value(),
            })
            .await?;

        self.predictions
            .find_for_member_and_match(&member_id, match_id)
            .await?
            .ok_or_else(|| AppError::Internal("prediction was not persisted".to_owned()))
    }

    async fn resolve_active_member(
        &self,
        pool_id: &str,
        user_id: &str,
    ) -> Result<String, AppError> {
        let membership = self
            .members
            .find_membership(pool_id, user_id)
            .await?
            .filter(|member| member.status == MemberStatus::Active)
            .ok_or_else(|| {
                AppError::forbidden_code("not_pool_member", "you are not a member of this pool")
            })?;
        Ok(membership.id.as_str().to_owned())
    }

    fn ensure_open(
        &self,
        status: &MatchStatus,
        kickoff_at: &str,
        lock_offset_minutes: u16,
    ) -> Result<(), AppError> {
        if *status != MatchStatus::Scheduled {
            return Err(locked());
        }

        let kickoff = parse_datetime(kickoff_at).ok_or_else(|| {
            AppError::Internal("match kickoff_at is not a valid date-time".to_owned())
        })?;
        let now = parse_datetime(self.clock.now().as_str())
            .ok_or_else(|| AppError::Internal("clock produced an invalid date-time".to_owned()))?;

        let opens_at = kickoff - Duration::days(5);
        if now <= opens_at {
            return Err(AppError::validation_code(
                "prediction_not_open_yet",
                "predictions for this match are not open yet",
            ));
        }

        let lock_at = kickoff - Duration::minutes(i64::from(lock_offset_minutes));
        if now < lock_at {
            Ok(())
        } else {
            Err(locked())
        }
    }
}

fn locked() -> AppError {
    AppError::validation_code("prediction_locked", "predictions for this match are locked")
}
