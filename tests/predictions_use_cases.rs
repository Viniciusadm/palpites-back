mod support;

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::Utc;
use palpites_back::application::matches::{
    CreateMatchRecord, MatchFilters, MatchRepository, UpdateMatchRecord,
};
use palpites_back::application::pools::{
    CreatePoolSeed, NewMemberRecord, PoolMemberRepository, PoolMemberWithName, PoolRepository,
    UpdatePoolRecord,
};
use palpites_back::application::predictions::{
    PredictionRepository, PredictionUseCases, UpsertPrediction, UpsertPredictionRecord,
};
use palpites_back::domain::matches::{Match, MatchStatus};
use palpites_back::domain::pools::{
    MemberStatus, Pool, PoolMember, PoolRole, PoolStatus, Visibility,
};
use palpites_back::domain::predictions::Prediction;
use palpites_back::domain::{DomainId, InviteCode, NonEmptyString, Score, UtcDateTime};
use palpites_back::errors::AppError;
use support::FixedClock;

const POOL_ID: &str = "pool-1";
const TOURNAMENT_ID: &str = "tournament-1";
const MATCH_ID: &str = "match-1";
const USER_ID: &str = "user-1";
const MEMBER_ID: &str = "member-1";
const KICKOFF: &str = "2026-06-18 12:00:00";
const LOCK_OFFSET: u16 = 10;

#[tokio::test]
async fn upsert_before_lock_persists_prediction() {
    let predictions = FakePredictions::default();
    let use_cases = build(predictions.clone(), member(MemberStatus::Active), clock("2026-06-18 11:00:00"));

    let prediction = use_cases
        .upsert(POOL_ID, USER_ID, MATCH_ID, UpsertPrediction { home_score: 2, away_score: 1 })
        .await
        .unwrap();

    assert_eq!(prediction.home_score.value(), 2);
    assert_eq!(prediction.away_score.value(), 1);
    assert_eq!(predictions.rows.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn upsert_at_lock_is_rejected() {
    let use_cases = build(FakePredictions::default(), member(MemberStatus::Active), clock("2026-06-18 11:50:00"));

    let result = use_cases
        .upsert(POOL_ID, USER_ID, MATCH_ID, UpsertPrediction { home_score: 1, away_score: 0 })
        .await;

    assert_locked(result);
}

#[tokio::test]
async fn upsert_after_lock_is_rejected() {
    let use_cases = build(FakePredictions::default(), member(MemberStatus::Active), clock("2026-06-18 12:30:00"));

    let result = use_cases
        .upsert(POOL_ID, USER_ID, MATCH_ID, UpsertPrediction { home_score: 1, away_score: 0 })
        .await;

    assert_locked(result);
}

#[tokio::test]
async fn second_upsert_overwrites_the_first() {
    let predictions = FakePredictions::default();
    let use_cases = build(predictions.clone(), member(MemberStatus::Active), clock("2026-06-18 11:00:00"));

    use_cases
        .upsert(POOL_ID, USER_ID, MATCH_ID, UpsertPrediction { home_score: 2, away_score: 1 })
        .await
        .unwrap();
    let updated = use_cases
        .upsert(POOL_ID, USER_ID, MATCH_ID, UpsertPrediction { home_score: 0, away_score: 0 })
        .await
        .unwrap();

    assert_eq!(updated.home_score.value(), 0);
    assert_eq!(updated.away_score.value(), 0);
    assert_eq!(predictions.rows.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn non_member_is_rejected() {
    let use_cases = build(FakePredictions::default(), None, clock("2026-06-18 11:00:00"));

    let result = use_cases
        .upsert(POOL_ID, USER_ID, MATCH_ID, UpsertPrediction { home_score: 1, away_score: 0 })
        .await;

    assert!(matches!(result, Err(AppError::Coded { kind, code, .. })
        if kind == palpites_back::errors::CodedKind::Forbidden && code == "not_pool_member"));
}

#[tokio::test]
async fn inactive_member_is_rejected() {
    let use_cases = build(FakePredictions::default(), member(MemberStatus::Inactive), clock("2026-06-18 11:00:00"));

    let result = use_cases
        .upsert(POOL_ID, USER_ID, MATCH_ID, UpsertPrediction { home_score: 1, away_score: 0 })
        .await;

    assert!(matches!(result, Err(AppError::Coded { code, .. }) if code == "not_pool_member"));
}

#[tokio::test]
async fn finished_match_is_locked_regardless_of_time() {
    let use_cases = PredictionUseCases::new(
        FakePredictions::default(),
        FakeMatches::with_status(MatchStatus::Finished),
        FakePools::default(),
        FakeMembers::new(member(MemberStatus::Active)),
        clock("2026-06-18 11:00:00"),
    );

    let result = use_cases
        .upsert(POOL_ID, USER_ID, MATCH_ID, UpsertPrediction { home_score: 1, away_score: 0 })
        .await;

    assert_locked(result);
}

#[tokio::test]
async fn list_mine_returns_member_predictions() {
    let predictions = FakePredictions::default();
    let use_cases = build(predictions.clone(), member(MemberStatus::Active), clock("2026-06-18 11:00:00"));

    use_cases
        .upsert(POOL_ID, USER_ID, MATCH_ID, UpsertPrediction { home_score: 3, away_score: 2 })
        .await
        .unwrap();

    let mine = use_cases.list_mine(POOL_ID, USER_ID).await.unwrap();
    assert_eq!(mine.len(), 1);
    assert_eq!(mine[0].home_score.value(), 3);
}

// --- helpers & fakes -------------------------------------------------------

fn build(
    predictions: FakePredictions,
    membership: Option<PoolMember>,
    clock: FixedClock,
) -> PredictionUseCases<FakePredictions, FakeMatches, FakePools, FakeMembers, FixedClock> {
    PredictionUseCases::new(
        predictions,
        FakeMatches::with_status(MatchStatus::Scheduled),
        FakePools::default(),
        FakeMembers::new(membership),
        clock,
    )
}

fn clock(now: &str) -> FixedClock {
    FixedClock::new(now)
}

fn member(status: MemberStatus) -> Option<PoolMember> {
    Some(PoolMember {
        id: id(MEMBER_ID),
        pool_id: id(POOL_ID),
        user_id: id(USER_ID),
        role: PoolRole::Member,
        status,
        joined_at: now(),
        left_at: None,
        created_at: now(),
        updated_at: now(),
    })
}

fn assert_locked(result: Result<Prediction, AppError>) {
    assert!(matches!(result, Err(AppError::Coded { code, .. }) if code == "prediction_locked"));
}

fn id(value: &str) -> DomainId {
    DomainId::new(value.to_owned()).unwrap()
}

fn now() -> UtcDateTime {
    UtcDateTime::new_iso8601(Utc::now().naive_utc().to_string()).unwrap()
}

#[derive(Clone, Default)]
struct FakePredictions {
    rows: Arc<Mutex<Vec<UpsertPredictionRecord>>>,
}

#[async_trait]
impl PredictionRepository for FakePredictions {
    async fn upsert(&self, record: UpsertPredictionRecord) -> Result<(), AppError> {
        let mut rows = self.rows.lock().unwrap();
        rows.retain(|existing| {
            existing.pool_member_id != record.pool_member_id || existing.match_id != record.match_id
        });
        rows.push(record);
        Ok(())
    }

    async fn find_for_member_and_match(
        &self,
        pool_member_id: &str,
        match_id: &str,
    ) -> Result<Option<Prediction>, AppError> {
        let rows = self.rows.lock().unwrap();
        Ok(rows
            .iter()
            .find(|record| record.pool_member_id == pool_member_id && record.match_id == match_id)
            .map(to_prediction))
    }

    async fn list_for_member(&self, pool_member_id: &str) -> Result<Vec<Prediction>, AppError> {
        let rows = self.rows.lock().unwrap();
        Ok(rows
            .iter()
            .filter(|record| record.pool_member_id == pool_member_id)
            .map(to_prediction)
            .collect())
    }

    async fn list_for_match(&self, match_id: &str) -> Result<Vec<Prediction>, AppError> {
        let rows = self.rows.lock().unwrap();
        Ok(rows
            .iter()
            .filter(|record| record.match_id == match_id)
            .map(to_prediction)
            .collect())
    }
}

fn to_prediction(record: &UpsertPredictionRecord) -> Prediction {
    Prediction {
        id: id(&record.prediction_id),
        pool_member_id: id(&record.pool_member_id),
        match_id: id(&record.match_id),
        home_score: Score::new(record.home_score).unwrap(),
        away_score: Score::new(record.away_score).unwrap(),
        points_awarded: None,
        scored_at: None,
        created_at: now(),
        updated_at: now(),
    }
}

#[derive(Clone)]
struct FakeMatches {
    status: MatchStatus,
}

impl FakeMatches {
    fn with_status(status: MatchStatus) -> Self {
        Self { status }
    }
}

#[async_trait]
impl MatchRepository for FakeMatches {
    async fn list(&self, _: &str, _: MatchFilters) -> Result<Vec<Match>, AppError> {
        Ok(Vec::new())
    }

    async fn find_by_id(&self, match_id: &str) -> Result<Option<Match>, AppError> {
        if match_id != MATCH_ID {
            return Ok(None);
        }
        Ok(Some(Match {
            id: id(MATCH_ID),
            tournament_id: id(TOURNAMENT_ID),
            stage_id: id("stage-1"),
            home_team_id: Some(id("team-home")),
            away_team_id: Some(id("team-away")),
            kickoff_at: UtcDateTime::new_iso8601(KICKOFF.to_owned()).unwrap(),
            status: self.status,
            home_score: None,
            away_score: None,
            finished_at: None,
            created_at: now(),
            updated_at: now(),
        }))
    }

    async fn create(&self, _: CreateMatchRecord) -> Result<(), AppError> {
        Ok(())
    }

    async fn update(&self, _: UpdateMatchRecord) -> Result<(), AppError> {
        Ok(())
    }

    async fn delete(&self, _: &str) -> Result<(), AppError> {
        Ok(())
    }

    async fn tournament_exists(&self, _: &str) -> Result<bool, AppError> {
        Ok(true)
    }

    async fn find_stage_tournament_id(&self, _: &str) -> Result<Option<String>, AppError> {
        Ok(Some(TOURNAMENT_ID.to_owned()))
    }

    async fn is_team_in_tournament(&self, _: &str, _: &str) -> Result<bool, AppError> {
        Ok(true)
    }
}

#[derive(Clone, Default)]
struct FakePools;

#[async_trait]
impl PoolRepository for FakePools {
    async fn find_by_id(&self, pool_id: &str) -> Result<Option<Pool>, AppError> {
        if pool_id != POOL_ID {
            return Ok(None);
        }
        Ok(Some(Pool {
            id: id(POOL_ID),
            tournament_id: id(TOURNAMENT_ID),
            owner_user_id: id("owner-1"),
            name: NonEmptyString::new("Bolão".to_owned(), "pool.name").unwrap(),
            invite_code: InviteCode::new("ABC123".to_owned()).unwrap(),
            visibility: Visibility::Private,
            ranking_public: true,
            prediction_lock_offset_minutes: LOCK_OFFSET,
            status: PoolStatus::Active,
            created_at: now(),
            updated_at: now(),
        }))
    }

    async fn find_by_invite_code(&self, _: &str) -> Result<Option<Pool>, AppError> {
        Ok(None)
    }

    async fn list_for_user(&self, _: &str) -> Result<Vec<Pool>, AppError> {
        Ok(Vec::new())
    }

    async fn list_for_tournament(&self, _: &str) -> Result<Vec<Pool>, AppError> {
        Ok(Vec::new())
    }

    async fn find_tournament_status(&self, _: &str) -> Result<Option<String>, AppError> {
        Ok(Some("active".to_owned()))
    }

    async fn create_with_seed(&self, _: CreatePoolSeed) -> Result<(), AppError> {
        Ok(())
    }

    async fn update_settings(&self, _: UpdatePoolRecord) -> Result<(), AppError> {
        Ok(())
    }

    async fn delete(&self, _: &str) -> Result<(), AppError> {
        Ok(())
    }
}

#[derive(Clone)]
struct FakeMembers {
    membership: Option<PoolMember>,
}

impl FakeMembers {
    fn new(membership: Option<PoolMember>) -> Self {
        Self { membership }
    }
}

#[async_trait]
impl PoolMemberRepository for FakeMembers {
    async fn find_membership(
        &self,
        _: &str,
        _: &str,
    ) -> Result<Option<PoolMember>, AppError> {
        Ok(self.membership.clone())
    }

    async fn find_by_id(&self, _: &str) -> Result<Option<PoolMember>, AppError> {
        Ok(self.membership.clone())
    }

    async fn list_for_pool(&self, _: &str) -> Result<Vec<PoolMember>, AppError> {
        Ok(self.membership.clone().into_iter().collect())
    }
    async fn list_for_pool_with_names(
        &self,
        pool_id: &str,
    ) -> Result<Vec<PoolMemberWithName>, AppError> {
        Ok(self
            .list_for_pool(pool_id)
            .await?
            .into_iter()
            .map(|member| PoolMemberWithName {
                display_name: member.user_id.as_str().to_owned(),
                member,
            })
            .collect())
    }

    async fn count_active_owners(&self, _: &str) -> Result<u64, AppError> {
        Ok(0)
    }

    async fn create(&self, _: NewMemberRecord) -> Result<(), AppError> {
        Ok(())
    }

    async fn reactivate(&self, _: &str, _: &str) -> Result<(), AppError> {
        Ok(())
    }

    async fn set_status(&self, _: &str, _: &str, _: Option<&str>) -> Result<(), AppError> {
        Ok(())
    }

    async fn set_role(&self, _: &str, _: &str) -> Result<(), AppError> {
        Ok(())
    }
}
