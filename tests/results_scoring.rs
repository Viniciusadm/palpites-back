mod support;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::Utc;
use palpites_back::application::matches::{
    CreateMatchRecord, MatchFilters, MatchRepository, UpdateMatchRecord,
};
use palpites_back::application::pools::{
    CreatePoolSeed, NewMemberRecord, NewScoringRuleRecord, PoolMemberRepository, PoolRepository,
    ScoringRuleRepository, UpdatePoolRecord,
};
use palpites_back::application::results::{
    EnterResult, PoolRecompute, PredictionLine, ResultApplication, ResultUseCases, StandingRecord,
    StandingRepository,
};
use palpites_back::domain::matches::{Match, MatchStatus};
use palpites_back::domain::pools::{
    MemberStatus, Pool, PoolMember, PoolRole, PoolScoringRule, PoolStatus, ScoringRuleKey,
    Visibility,
};
use palpites_back::domain::predictions::{score, HitKind, ScoringRules};
use palpites_back::domain::standings::{ranking, Standing};
use palpites_back::domain::{DomainId, InviteCode, NonEmptyString, Score, UtcDateTime};
use palpites_back::errors::AppError;
use support::{FixedClock, NoopNotifier};

const MATCH_ID: &str = "match-1";

// --- pure ranking aggregation ----------------------------------------------

#[test]
fn ranking_orders_by_points_with_shared_positions_on_ties() {
    let rows = ranking::compute(
        ["a", "b", "c", "d"].into_iter().map(str::to_owned),
        [
            ("a".to_owned(), 30, HitKind::Exact),
            ("b".to_owned(), 20, HitKind::Outcome),
            ("c".to_owned(), 20, HitKind::Exact),
        ],
    );

    assert_eq!(rows[0].pool_member_id, "a");
    assert_eq!(rows[0].position, 1);
    assert_eq!(rows[0].total_points, 30);

    // b and c tie on 20 points and one hit each → tie-break by member id (asc).
    assert_eq!(rows[1].pool_member_id, "b");
    assert_eq!(rows[1].position, 2);
    assert_eq!(rows[2].pool_member_id, "c");
    assert_eq!(rows[2].position, 2);

    // d never scored but is still listed, last, with zero points.
    assert_eq!(rows[3].pool_member_id, "d");
    assert_eq!(rows[3].position, 4);
    assert_eq!(rows[3].total_points, 0);
}

// --- enter_result across pools with different rules ------------------------

#[tokio::test]
async fn enter_result_scores_predictions_across_pools_with_different_rules() {
    let standings = FakeStandings::default();
    standings.set_lines(
        "pool-a",
        vec![
            line("p-a1", "m1", MATCH_ID, 2, 1),
            line("p-a2", "m2", MATCH_ID, 1, 0),
        ],
    );
    standings.set_lines(
        "pool-b",
        vec![
            line("p-b1", "m3", MATCH_ID, 2, 1),
            line("p-b2", "m4", MATCH_ID, 3, 2),
        ],
    );

    let scoring = FakeScoring::default();
    scoring.set_rules(
        "pool-a",
        &[(ScoringRuleKey::ExactScore, 10), (ScoringRuleKey::CorrectOutcome, 5)],
    );
    scoring.set_rules(
        "pool-b",
        &[
            (ScoringRuleKey::ExactScore, 3),
            (ScoringRuleKey::CorrectOutcome, 5),
            (ScoringRuleKey::CorrectGoalDifference, 7),
        ],
    );

    let members = FakeMembers::default();
    members.add_active("pool-a", "m1", "u1");
    members.add_active("pool-a", "m2", "u2");
    members.add_active("pool-b", "m3", "u3");
    members.add_active("pool-b", "m4", "u4");

    let use_cases = ResultUseCases::new(
        standings.clone(),
        FakeMatches::with_status(MatchStatus::Scheduled),
        scoring,
        FakePools::default(),
        members,
        FixedClock::new("2026-06-18 18:00:00"),
        NoopNotifier,
    );

    let updated = use_cases
        .enter_result(MATCH_ID, EnterResult { home_score: 2, away_score: 1 })
        .await
        .unwrap();
    assert_eq!(updated.id.as_str(), MATCH_ID);

    let application = standings.last_application();
    assert_eq!(application.match_id, MATCH_ID);
    assert_eq!(application.home_score, 2);
    assert_eq!(application.away_score, 1);
    assert_eq!(application.finished_at, "2026-06-18 18:00:00");

    let pool_a = find_pool(&application, "pool-a");
    assert_eq!(points_for(pool_a, "p-a1"), 10); // exact
    assert_eq!(points_for(pool_a, "p-a2"), 5); // outcome
    assert_eq!(position_for(pool_a, "m1"), 1);
    assert_eq!(position_for(pool_a, "m2"), 2);
    assert_eq!(exact_for(pool_a, "m1"), 1);

    let pool_b = find_pool(&application, "pool-b");
    assert_eq!(points_for(pool_b, "p-b1"), 3); // exact (pool B exact=3)
    assert_eq!(points_for(pool_b, "p-b2"), 7); // goal-difference
    assert_eq!(position_for(pool_b, "m4"), 1);
    assert_eq!(position_for(pool_b, "m3"), 2);
}

// --- rescoring after a rule change -----------------------------------------

#[tokio::test]
async fn recompute_pool_rescoring_reflects_new_rules() {
    let standings = FakeStandings::default();
    standings.set_lines(
        "pool-a",
        vec![
            finished_line("p-a1", "m1", "match-x", 2, 1, 2, 1), // exact
            finished_line("p-a2", "m2", "match-x", 1, 0, 2, 1), // outcome (home win)
        ],
    );

    let scoring = FakeScoring::default();
    scoring.set_rules(
        "pool-a",
        &[(ScoringRuleKey::ExactScore, 10), (ScoringRuleKey::CorrectOutcome, 5)],
    );

    let members = FakeMembers::default();
    members.add_active("pool-a", "m1", "u1");
    members.add_active("pool-a", "m2", "u2");

    let use_cases = ResultUseCases::new(
        standings.clone(),
        FakeMatches::with_status(MatchStatus::Finished),
        scoring.clone(),
        FakePools::default(),
        members,
        FixedClock::new("2026-06-18 18:00:00"),
        NoopNotifier,
    );

    use_cases.recompute_pool("pool-a").await.unwrap();
    let first = standings.last_recompute();
    assert_eq!(points_for(&first, "p-a1"), 10);
    assert_eq!(points_for(&first, "p-a2"), 5);
    assert_eq!(position_for(&first, "m1"), 1);

    // change rules → re-run recompute → points reflect new rules.
    scoring.set_rules(
        "pool-a",
        &[(ScoringRuleKey::ExactScore, 20), (ScoringRuleKey::CorrectOutcome, 1)],
    );
    use_cases.recompute_pool("pool-a").await.unwrap();
    let second = standings.last_recompute();
    assert_eq!(points_for(&second, "p-a1"), 20);
    assert_eq!(points_for(&second, "p-a2"), 1);
}

// --- ranking_public gating -------------------------------------------------

#[tokio::test]
async fn ranking_is_forbidden_for_non_members_when_not_public() {
    let standings = FakeStandings::default();
    standings.seed_standings("pool-a", vec![standing("pool-a", "m1", 10, 1)]);

    let members = FakeMembers::default();
    members.add_active("pool-a", "m1", "u1");

    let use_cases = ResultUseCases::new(
        standings.clone(),
        FakeMatches::with_status(MatchStatus::Scheduled),
        FakeScoring::default(),
        FakePools::with_ranking_public(false),
        members,
        FixedClock::new("2026-06-18 18:00:00"),
        NoopNotifier,
    );

    let stranger = use_cases.ranking("pool-a", "stranger").await;
    assert!(matches!(stranger, Err(AppError::Coded { code, .. }) if code == "ranking_not_public"));

    let member = use_cases.ranking("pool-a", "u1").await.unwrap();
    assert_eq!(member.standings.len(), 1);
}

#[tokio::test]
async fn ranking_is_public_for_non_members_when_public() {
    let standings = FakeStandings::default();
    standings.seed_standings("pool-a", vec![standing("pool-a", "m1", 10, 1)]);

    let use_cases = ResultUseCases::new(
        standings,
        FakeMatches::with_status(MatchStatus::Scheduled),
        FakeScoring::default(),
        FakePools::with_ranking_public(true),
        FakeMembers::default(),
        FixedClock::new("2026-06-18 18:00:00"),
        NoopNotifier,
    );

    let result = use_cases.ranking("pool-a", "stranger").await.unwrap();
    assert_eq!(result.standings.len(), 1);
}

// --- history aggregates ----------------------------------------------------

#[tokio::test]
async fn history_aggregates_points_hits_errors_and_pending() {
    let standings = FakeStandings::default();
    standings.set_lines(
        "pool-a",
        vec![
            finished_line("p1", "m1", "match-1", 2, 1, 2, 1), // exact → 10
            finished_line("p2", "m1", "match-2", 1, 0, 0, 0), // wrong → error
            finished_line("p3", "m1", "match-3", 2, 0, 3, 1), // outcome (home win) → 5
            line("p4", "m1", "match-4", 1, 1),                // scheduled → pending
        ],
    );

    let scoring = FakeScoring::default();
    scoring.set_rules(
        "pool-a",
        &[(ScoringRuleKey::ExactScore, 10), (ScoringRuleKey::CorrectOutcome, 5)],
    );

    let members = FakeMembers::default();
    members.add_active("pool-a", "m1", "u1");

    let use_cases = ResultUseCases::new(
        standings,
        FakeMatches::with_status(MatchStatus::Scheduled),
        scoring,
        FakePools::default(),
        members,
        FixedClock::new("2026-06-18 18:00:00"),
        NoopNotifier,
    );

    let summary = use_cases.history("pool-a", "u1").await.unwrap();
    assert_eq!(summary.total_points, 15);
    assert_eq!(summary.exact_count, 1);
    assert_eq!(summary.outcome_count, 1);
    assert_eq!(summary.hits_count, 2);
    assert_eq!(summary.errors_count, 1);
    assert_eq!(summary.pending_count, 1);
    assert_eq!(summary.entries.len(), 4);
}

// --- member predictions lock filter ----------------------------------------

#[tokio::test]
async fn member_predictions_hides_open_matches() {
    let standings = FakeStandings::default();
    standings.set_lines(
        "pool-a",
        vec![
            scheduled_line("p-open", "m1", "match-open", 1, 0, "2026-06-18 12:30:00"),
            scheduled_line("p-locked", "m1", "match-locked", 2, 2, "2026-06-18 12:00:00"),
            finished_line("p-done", "m1", "match-done", 1, 0, 1, 0),
        ],
    );

    let members = FakeMembers::default();
    members.add_active("pool-a", "m1", "u1");

    let use_cases = ResultUseCases::new(
        standings,
        FakeMatches::with_status(MatchStatus::Scheduled),
        FakeScoring::default(),
        FakePools::with_ranking_public(true),
        members,
        FixedClock::new("2026-06-18 11:55:00"),
        NoopNotifier,
    );

    let views = use_cases
        .member_predictions("pool-a", "m1", "viewer")
        .await
        .unwrap();
    let match_ids: Vec<&str> = views.iter().map(|view| view.match_id.as_str()).collect();
    assert!(!match_ids.contains(&"match-open"));
    assert!(match_ids.contains(&"match-locked"));
    assert!(match_ids.contains(&"match-done"));
    assert_eq!(views.len(), 2);
}

// --- assertion helpers ------------------------------------------------------

fn find_pool<'a>(application: &'a ResultApplication, pool_id: &str) -> &'a PoolRecompute {
    application
        .pools
        .iter()
        .find(|pool| pool.pool_id == pool_id)
        .expect("pool recompute present")
}

fn points_for(pool: &PoolRecompute, prediction_id: &str) -> i16 {
    pool.scored_predictions
        .iter()
        .find(|scored| scored.prediction_id == prediction_id)
        .expect("scored prediction present")
        .points_awarded
}

fn position_for(pool: &PoolRecompute, member_id: &str) -> i32 {
    standing_of(pool, member_id).position
}

fn exact_for(pool: &PoolRecompute, member_id: &str) -> i32 {
    standing_of(pool, member_id).exact_count
}

fn standing_of<'a>(pool: &'a PoolRecompute, member_id: &str) -> &'a StandingRecord {
    pool.standings
        .iter()
        .find(|standing| standing.pool_member_id == member_id)
        .expect("standing present")
}

// --- line builders ----------------------------------------------------------

fn line(prediction_id: &str, member_id: &str, match_id: &str, home: u8, away: u8) -> PredictionLine {
    scheduled_line(prediction_id, member_id, match_id, home, away, "2026-06-20 18:00:00")
}

fn scheduled_line(
    prediction_id: &str,
    member_id: &str,
    match_id: &str,
    home: u8,
    away: u8,
    kickoff: &str,
) -> PredictionLine {
    PredictionLine {
        prediction_id: prediction_id.to_owned(),
        pool_member_id: member_id.to_owned(),
        match_id: match_id.to_owned(),
        prediction_home: home,
        prediction_away: away,
        match_status: MatchStatus::Scheduled.as_str().to_owned(),
        kickoff_at: kickoff.to_owned(),
        result_home: None,
        result_away: None,
    }
}

#[allow(clippy::too_many_arguments)]
fn finished_line(
    prediction_id: &str,
    member_id: &str,
    match_id: &str,
    home: u8,
    away: u8,
    result_home: u8,
    result_away: u8,
) -> PredictionLine {
    PredictionLine {
        prediction_id: prediction_id.to_owned(),
        pool_member_id: member_id.to_owned(),
        match_id: match_id.to_owned(),
        prediction_home: home,
        prediction_away: away,
        match_status: MatchStatus::Finished.as_str().to_owned(),
        kickoff_at: "2026-06-10 18:00:00".to_owned(),
        result_home: Some(result_home),
        result_away: Some(result_away),
    }
}

fn standing(pool_id: &str, member_id: &str, total_points: i32, position: i32) -> Standing {
    Standing {
        id: id(&format!("standing-{member_id}")),
        pool_id: id(pool_id),
        pool_member_id: id(member_id),
        total_points,
        exact_count: 0,
        outcome_count: 0,
        hits_count: 0,
        position,
        updated_at: now(),
    }
}

// --- fakes ------------------------------------------------------------------

#[derive(Clone, Default)]
struct FakeStandings {
    lines: Arc<Mutex<HashMap<String, Vec<PredictionLine>>>>,
    standings: Arc<Mutex<HashMap<String, Vec<Standing>>>>,
    applied: Arc<Mutex<Vec<ResultApplication>>>,
    rescored: Arc<Mutex<Vec<PoolRecompute>>>,
}

impl FakeStandings {
    fn set_lines(&self, pool_id: &str, lines: Vec<PredictionLine>) {
        self.lines.lock().unwrap().insert(pool_id.to_owned(), lines);
    }

    fn seed_standings(&self, pool_id: &str, rows: Vec<Standing>) {
        self.standings.lock().unwrap().insert(pool_id.to_owned(), rows);
    }

    fn last_application(&self) -> ResultApplication {
        self.applied.lock().unwrap().last().cloned().expect("apply_result called")
    }

    fn last_recompute(&self) -> PoolRecompute {
        self.rescored.lock().unwrap().last().cloned().expect("rescore_pool called")
    }
}

#[async_trait]
impl StandingRepository for FakeStandings {
    async fn list_for_pool(&self, pool_id: &str) -> Result<Vec<Standing>, AppError> {
        Ok(self.standings.lock().unwrap().get(pool_id).cloned().unwrap_or_default())
    }

    async fn prediction_lines_for_pool(
        &self,
        pool_id: &str,
    ) -> Result<Vec<PredictionLine>, AppError> {
        Ok(self.lines.lock().unwrap().get(pool_id).cloned().unwrap_or_default())
    }

    async fn affected_pools_for_match(&self, match_id: &str) -> Result<Vec<String>, AppError> {
        let mut pools: Vec<String> = self
            .lines
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, lines)| lines.iter().any(|line| line.match_id == match_id))
            .map(|(pool_id, _)| pool_id.clone())
            .collect();
        pools.sort();
        Ok(pools)
    }

    async fn apply_result(&self, application: ResultApplication) -> Result<(), AppError> {
        self.applied.lock().unwrap().push(application);
        Ok(())
    }

    async fn rescore_pool(&self, recompute: PoolRecompute) -> Result<(), AppError> {
        self.rescored.lock().unwrap().push(recompute);
        Ok(())
    }
}

#[derive(Clone, Default)]
struct FakeScoring {
    rules: Arc<Mutex<HashMap<String, Vec<PoolScoringRule>>>>,
}

impl FakeScoring {
    fn set_rules(&self, pool_id: &str, rules: &[(ScoringRuleKey, i16)]) {
        let rows = rules
            .iter()
            .enumerate()
            .map(|(index, (key, points))| PoolScoringRule {
                id: id(&format!("rule-{pool_id}-{index}")),
                pool_id: id(pool_id),
                rule_key: *key,
                points: *points,
                created_at: now(),
                updated_at: now(),
            })
            .collect();
        self.rules.lock().unwrap().insert(pool_id.to_owned(), rows);
    }
}

#[async_trait]
impl ScoringRuleRepository for FakeScoring {
    async fn list_for_pool(&self, pool_id: &str) -> Result<Vec<PoolScoringRule>, AppError> {
        Ok(self.rules.lock().unwrap().get(pool_id).cloned().unwrap_or_default())
    }

    async fn set_rules(
        &self,
        _pool_id: &str,
        _rules: Vec<NewScoringRuleRecord>,
    ) -> Result<(), AppError> {
        Ok(())
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
        Ok(Some(Match {
            id: id(match_id),
            tournament_id: id("tournament-1"),
            stage_id: id("stage-1"),
            home_team_id: Some(id("team-home")),
            away_team_id: Some(id("team-away")),
            kickoff_at: now(),
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
        Ok(Some("tournament-1".to_owned()))
    }

    async fn is_team_in_tournament(&self, _: &str, _: &str) -> Result<bool, AppError> {
        Ok(true)
    }
}

#[derive(Clone)]
struct FakePools {
    ranking_public: bool,
}

impl Default for FakePools {
    fn default() -> Self {
        Self { ranking_public: true }
    }
}

impl FakePools {
    fn with_ranking_public(ranking_public: bool) -> Self {
        Self { ranking_public }
    }
}

#[async_trait]
impl PoolRepository for FakePools {
    async fn find_by_id(&self, pool_id: &str) -> Result<Option<Pool>, AppError> {
        Ok(Some(Pool {
            id: id(pool_id),
            tournament_id: id("tournament-1"),
            owner_user_id: id("owner-1"),
            name: NonEmptyString::new("Bolão".to_owned(), "pool.name").unwrap(),
            invite_code: InviteCode::new("ABC123".to_owned()).unwrap(),
            visibility: Visibility::Private,
            ranking_public: self.ranking_public,
            prediction_lock_offset_minutes: 10,
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

#[derive(Clone, Default)]
struct FakeMembers {
    by_pool: Arc<Mutex<HashMap<String, Vec<PoolMember>>>>,
    by_membership: Arc<Mutex<HashMap<(String, String), PoolMember>>>,
    by_id: Arc<Mutex<HashMap<String, PoolMember>>>,
}

impl FakeMembers {
    fn add_active(&self, pool_id: &str, member_id: &str, user_id: &str) {
        let member = PoolMember {
            id: id(member_id),
            pool_id: id(pool_id),
            user_id: id(user_id),
            role: PoolRole::Member,
            status: MemberStatus::Active,
            joined_at: now(),
            left_at: None,
            created_at: now(),
            updated_at: now(),
        };
        self.by_pool
            .lock()
            .unwrap()
            .entry(pool_id.to_owned())
            .or_default()
            .push(member.clone());
        self.by_membership
            .lock()
            .unwrap()
            .insert((pool_id.to_owned(), user_id.to_owned()), member.clone());
        self.by_id.lock().unwrap().insert(member_id.to_owned(), member);
    }
}

#[async_trait]
impl PoolMemberRepository for FakeMembers {
    async fn find_membership(
        &self,
        pool_id: &str,
        user_id: &str,
    ) -> Result<Option<PoolMember>, AppError> {
        Ok(self
            .by_membership
            .lock()
            .unwrap()
            .get(&(pool_id.to_owned(), user_id.to_owned()))
            .cloned())
    }

    async fn find_by_id(&self, member_id: &str) -> Result<Option<PoolMember>, AppError> {
        Ok(self.by_id.lock().unwrap().get(member_id).cloned())
    }

    async fn list_for_pool(&self, pool_id: &str) -> Result<Vec<PoolMember>, AppError> {
        Ok(self.by_pool.lock().unwrap().get(pool_id).cloned().unwrap_or_default())
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

// --- primitives -------------------------------------------------------------

fn id(value: &str) -> DomainId {
    DomainId::new(value.to_owned()).unwrap()
}

fn now() -> UtcDateTime {
    UtcDateTime::new_iso8601(Utc::now().naive_utc().to_string()).unwrap()
}

// Keep the scoring policy import exercised so the suite fails fast if its
// signature drifts from what the use case relies on.
#[test]
fn scoring_policy_signature_is_stable() {
    let rules = ScoringRules::from_rules([(ScoringRuleKey::ExactScore, 10)]);
    let (points, kind) = score(
        (Score::new(2).unwrap(), Score::new(1).unwrap()),
        (Score::new(2).unwrap(), Score::new(1).unwrap()),
        &rules,
    );
    assert_eq!(points, 10);
    assert_eq!(kind, HitKind::Exact);
}
