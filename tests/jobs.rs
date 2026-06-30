mod support;

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::Utc;
use palpites_back::application::jobs::{
    DispatchNotifications, NotificationSender, OutboundNotification, OutboundNotificationRepository,
    ReminderQueryRepository, ReminderWindow, RemindedRecipient, SendPredictionReminders,
};
use palpites_back::application::pools::{
    CreatePoolSeed, NewMemberRecord, PoolMemberRepository, PoolMemberWithName, PoolRepository,
    UpdatePoolRecord,
};
use palpites_back::application::predictions::{PredictionRepository, UpsertPredictionRecord};
use palpites_back::application::shared::{Notifier, ReminderRecipient};
use palpites_back::domain::matches::{Match, MatchStatus};
use palpites_back::domain::notifications::Channel;
use palpites_back::domain::pools::{
    MemberStatus, Pool, PoolMember, PoolRole, PoolStatus,
};
use palpites_back::domain::predictions::Prediction;
use palpites_back::domain::{DomainId, InviteCode, NonEmptyString, Score, UtcDateTime};
use palpites_back::errors::AppError;
use support::FixedClock;

const TOURNAMENT_ID: &str = "tournament-1";
const MATCH_ID: &str = "match-1";
const POOL_ID: &str = "pool-1";
const NOW: &str = "2026-06-18 11:30:00";
const KICKOFF: &str = "2026-06-18 12:00:00";

#[tokio::test]
async fn reminds_only_members_lacking_a_prediction() {
    let notifier = RecordingNotifier::default();
    let runner = SendPredictionReminders::new(
        FakeReminderQuery::new(vec![scheduled_match()], vec![]),
        FakePools::new(vec![pool(10)]),
        FakeMembers::new(vec![member("member-a", "user-a"), member("member-b", "user-b")]),
        FakePredictions::new(vec![prediction("member-a")]),
        notifier.clone(),
        FixedClock::new(NOW),
    );

    let created = runner.run(ReminderWindow::new(60, 60)).await.unwrap();

    assert_eq!(created, 1);
    let calls = notifier.calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, MATCH_ID);
    assert_eq!(
        calls[0].1,
        vec![ReminderRecipient {
            user_id: "user-b".to_owned(),
            pool_id: POOL_ID.to_owned(),
        }]
    );
}

#[tokio::test]
async fn skips_recipients_already_reminded() {
    let notifier = RecordingNotifier::default();
    let runner = SendPredictionReminders::new(
        FakeReminderQuery::new(
            vec![scheduled_match()],
            vec![RemindedRecipient {
                user_id: "user-b".to_owned(),
                pool_id: POOL_ID.to_owned(),
            }],
        ),
        FakePools::new(vec![pool(10)]),
        FakeMembers::new(vec![member("member-b", "user-b"), member("member-c", "user-c")]),
        FakePredictions::new(vec![]),
        notifier.clone(),
        FixedClock::new(NOW),
    );

    let created = runner.run(ReminderWindow::new(60, 60)).await.unwrap();

    assert_eq!(created, 1);
    let calls = notifier.calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(
        calls[0].1,
        vec![ReminderRecipient {
            user_id: "user-c".to_owned(),
            pool_id: POOL_ID.to_owned(),
        }]
    );
}

#[tokio::test]
async fn does_not_remind_when_prediction_window_is_locked() {
    let notifier = RecordingNotifier::default();
    let runner = SendPredictionReminders::new(
        FakeReminderQuery::new(vec![scheduled_match()], vec![]),
        FakePools::new(vec![pool(40)]),
        FakeMembers::new(vec![member("member-b", "user-b")]),
        FakePredictions::new(vec![]),
        notifier.clone(),
        FixedClock::new(NOW),
    );

    let created = runner.run(ReminderWindow::new(60, 60)).await.unwrap();

    assert_eq!(created, 0);
    assert!(notifier.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn reminds_user_only_once_across_multiple_matches() {
    let notifier = RecordingNotifier::default();
    let runner = SendPredictionReminders::new(
        FakeReminderQuery::new(
            vec![scheduled_match(), scheduled_match_with_id("match-2")],
            vec![],
        ),
        FakePools::new(vec![pool(10)]),
        FakeMembers::new(vec![member("member-b", "user-b")]),
        FakePredictions::new(vec![]),
        notifier.clone(),
        FixedClock::new(NOW),
    );

    let created = runner.run(ReminderWindow::new(60, 60)).await.unwrap();

    // Mesmo com dois jogos, o usuário recebe um único lembrete.
    assert_eq!(created, 1);
    let calls = notifier.calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, MATCH_ID);
    assert_eq!(
        calls[0].1,
        vec![ReminderRecipient {
            user_id: "user-b".to_owned(),
            pool_id: POOL_ID.to_owned(),
        }]
    );
}

#[tokio::test]
async fn skips_users_reminded_within_throttle_window() {
    let notifier = RecordingNotifier::default();
    let runner = SendPredictionReminders::new(
        FakeReminderQuery::new(vec![scheduled_match()], vec![])
            .with_reminded_users(vec!["user-b".to_owned()]),
        FakePools::new(vec![pool(10)]),
        FakeMembers::new(vec![member("member-b", "user-b")]),
        FakePredictions::new(vec![]),
        notifier.clone(),
        FixedClock::new(NOW),
    );

    let created = runner.run(ReminderWindow::new(60, 60)).await.unwrap();

    assert_eq!(created, 0);
    assert!(notifier.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn dispatch_delivers_and_marks_pending_notifications() {
    let push = RecordingSender::new(Channel::Push);
    let outbound = FakeOutbound::new(vec![
        outbound("out-1", Channel::Push),
        outbound("out-2", Channel::Push),
    ]);
    let runner = DispatchNotifications::new(
        outbound.clone(),
        vec![Box::new(push.clone())],
        FixedClock::new(NOW),
    );

    let summary = runner.run(100).await.unwrap();

    assert_eq!(summary.delivered, 2);
    assert_eq!(
        *push.sent.lock().unwrap(),
        vec!["out-1".to_owned(), "out-2".to_owned()]
    );
    let delivered = outbound.delivered.lock().unwrap();
    assert_eq!(*delivered, vec!["out-1".to_owned(), "out-2".to_owned()]);
    assert!(outbound.failed.lock().unwrap().is_empty());
}

#[tokio::test]
async fn dispatch_marks_failed_when_sender_errors() {
    let outbound = FakeOutbound::new(vec![outbound("out-1", Channel::Push)]);
    let runner = DispatchNotifications::new(
        outbound.clone(),
        vec![Box::new(FailingSender::new(Channel::Push))],
        FixedClock::new(NOW),
    );

    let summary = runner.run(100).await.unwrap();

    assert_eq!(summary.delivered, 0);
    assert!(outbound.delivered.lock().unwrap().is_empty());
    assert_eq!(*outbound.failed.lock().unwrap(), vec!["out-1".to_owned()]);
}

// --- fakes -----------------------------------------------------------------

struct FakeReminderQuery {
    matches: Vec<Match>,
    reminded: Vec<RemindedRecipient>,
    reminded_users: Vec<String>,
}

impl FakeReminderQuery {
    fn new(matches: Vec<Match>, reminded: Vec<RemindedRecipient>) -> Self {
        Self {
            matches,
            reminded,
            reminded_users: Vec::new(),
        }
    }

    fn with_reminded_users(mut self, reminded_users: Vec<String>) -> Self {
        self.reminded_users = reminded_users;
        self
    }
}

#[async_trait]
impl ReminderQueryRepository for FakeReminderQuery {
    async fn upcoming_scheduled_matches(
        &self,
        _after: &str,
        _until: &str,
    ) -> Result<Vec<Match>, AppError> {
        Ok(self.matches.clone())
    }

    async fn reminded_recipients_for_match(
        &self,
        _match_id: &str,
    ) -> Result<Vec<RemindedRecipient>, AppError> {
        Ok(self.reminded.clone())
    }

    async fn users_reminded_since(&self, _since: &str) -> Result<Vec<String>, AppError> {
        Ok(self.reminded_users.clone())
    }
}

struct FakePools {
    pools: Vec<Pool>,
}

impl FakePools {
    fn new(pools: Vec<Pool>) -> Self {
        Self { pools }
    }
}

#[async_trait]
impl PoolRepository for FakePools {
    async fn find_by_id(&self, _id: &str) -> Result<Option<Pool>, AppError> {
        Ok(None)
    }
    async fn find_by_invite_code(&self, _invite_code: &str) -> Result<Option<Pool>, AppError> {
        Ok(None)
    }
    async fn list_for_user(&self, _user_id: &str) -> Result<Vec<Pool>, AppError> {
        Ok(Vec::new())
    }
    async fn list_for_tournament(&self, _tournament_id: &str) -> Result<Vec<Pool>, AppError> {
        Ok(self.pools.clone())
    }
    async fn find_tournament_status(
        &self,
        _tournament_id: &str,
    ) -> Result<Option<String>, AppError> {
        Ok(None)
    }
    async fn create_with_seed(&self, _seed: CreatePoolSeed) -> Result<(), AppError> {
        Ok(())
    }
    async fn update_settings(&self, _record: UpdatePoolRecord) -> Result<(), AppError> {
        Ok(())
    }
    async fn delete(&self, _id: &str) -> Result<(), AppError> {
        Ok(())
    }
}

struct FakeMembers {
    members: Vec<PoolMember>,
}

impl FakeMembers {
    fn new(members: Vec<PoolMember>) -> Self {
        Self { members }
    }
}

#[async_trait]
impl PoolMemberRepository for FakeMembers {
    async fn find_membership(
        &self,
        _pool_id: &str,
        _user_id: &str,
    ) -> Result<Option<PoolMember>, AppError> {
        Ok(None)
    }
    async fn find_by_id(&self, _member_id: &str) -> Result<Option<PoolMember>, AppError> {
        Ok(None)
    }
    async fn list_for_pool(&self, _pool_id: &str) -> Result<Vec<PoolMember>, AppError> {
        Ok(self.members.clone())
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
    async fn count_active_owners(&self, _pool_id: &str) -> Result<u64, AppError> {
        Ok(0)
    }
    async fn create(&self, _record: NewMemberRecord) -> Result<(), AppError> {
        Ok(())
    }
    async fn reactivate(&self, _member_id: &str, _joined_at: &str) -> Result<(), AppError> {
        Ok(())
    }
    async fn set_status(
        &self,
        _member_id: &str,
        _status: &str,
        _left_at: Option<&str>,
    ) -> Result<(), AppError> {
        Ok(())
    }
    async fn set_role(&self, _member_id: &str, _role: &str) -> Result<(), AppError> {
        Ok(())
    }
}

struct FakePredictions {
    predictions: Vec<Prediction>,
}

impl FakePredictions {
    fn new(predictions: Vec<Prediction>) -> Self {
        Self { predictions }
    }
}

#[async_trait]
impl PredictionRepository for FakePredictions {
    async fn upsert(&self, _record: UpsertPredictionRecord) -> Result<(), AppError> {
        Ok(())
    }
    async fn find_for_member_and_match(
        &self,
        _pool_member_id: &str,
        _match_id: &str,
    ) -> Result<Option<Prediction>, AppError> {
        Ok(None)
    }
    async fn list_for_member(&self, _pool_member_id: &str) -> Result<Vec<Prediction>, AppError> {
        Ok(Vec::new())
    }
    async fn list_for_match(&self, _match_id: &str) -> Result<Vec<Prediction>, AppError> {
        Ok(self.predictions.clone())
    }
}

#[derive(Clone, Default)]
struct RecordingNotifier {
    calls: Arc<Mutex<Vec<(String, Vec<ReminderRecipient>)>>>,
}

#[async_trait]
impl Notifier for RecordingNotifier {
    async fn new_match(&self, _game: &Match) -> Result<(), AppError> {
        Ok(())
    }
    async fn member_joined(&self, _pool_id: &str, _joined_user_id: &str) -> Result<(), AppError> {
        Ok(())
    }
    async fn match_result(&self, _game: &Match, _pool_ids: &[String]) -> Result<(), AppError> {
        Ok(())
    }
    async fn prediction_reminder(
        &self,
        match_id: &str,
        recipients: &[ReminderRecipient],
    ) -> Result<(), AppError> {
        self.calls
            .lock()
            .unwrap()
            .push((match_id.to_owned(), recipients.to_vec()));
        Ok(())
    }
}

#[derive(Clone)]
struct FakeOutbound {
    pending: Vec<OutboundNotification>,
    delivered: Arc<Mutex<Vec<String>>>,
    failed: Arc<Mutex<Vec<String>>>,
}

impl FakeOutbound {
    fn new(pending: Vec<OutboundNotification>) -> Self {
        Self {
            pending,
            delivered: Arc::default(),
            failed: Arc::default(),
        }
    }
}

#[async_trait]
impl OutboundNotificationRepository for FakeOutbound {
    async fn list_pending(&self, _limit: u32) -> Result<Vec<OutboundNotification>, AppError> {
        Ok(self.pending.clone())
    }
    async fn mark_delivered(&self, id: &str, _delivered_at: &str) -> Result<(), AppError> {
        self.delivered.lock().unwrap().push(id.to_owned());
        Ok(())
    }
    async fn mark_failed(&self, id: &str, _error: &str) -> Result<(), AppError> {
        self.failed.lock().unwrap().push(id.to_owned());
        Ok(())
    }
}

#[derive(Clone)]
struct FailingSender {
    channel: Channel,
}

impl FailingSender {
    fn new(channel: Channel) -> Self {
        Self { channel }
    }
}

#[async_trait]
impl NotificationSender for FailingSender {
    fn channel(&self) -> Channel {
        self.channel
    }
    async fn send(&self, _notification: &OutboundNotification) -> Result<(), AppError> {
        Err(AppError::Internal("boom".to_owned()))
    }
}

#[derive(Clone)]
struct RecordingSender {
    channel: Channel,
    sent: Arc<Mutex<Vec<String>>>,
}

impl RecordingSender {
    fn new(channel: Channel) -> Self {
        Self {
            channel,
            sent: Arc::default(),
        }
    }
}

#[async_trait]
impl NotificationSender for RecordingSender {
    fn channel(&self) -> Channel {
        self.channel
    }
    async fn send(&self, notification: &OutboundNotification) -> Result<(), AppError> {
        self.sent.lock().unwrap().push(notification.id.clone());
        Ok(())
    }
}

// --- builders --------------------------------------------------------------

fn scheduled_match() -> Match {
    Match {
        id: id(MATCH_ID),
        tournament_id: id(TOURNAMENT_ID),
        stage_id: id("stage-1"),
        home_team_id: Some(id("team-home")),
        away_team_id: Some(id("team-away")),
        kickoff_at: UtcDateTime::new_iso8601(KICKOFF.to_owned()).unwrap(),
        status: MatchStatus::Scheduled,
        home_score: None,
        away_score: None,
        can_go_to_penalties: false,
        penalties_winner: None,
        finished_at: None,
        created_at: now(),
        updated_at: now(),
    }
}

fn scheduled_match_with_id(match_id: &str) -> Match {
    Match {
        id: id(match_id),
        ..scheduled_match()
    }
}

fn pool(lock_offset_minutes: u16) -> Pool {
    Pool {
        id: id(POOL_ID),
        tournament_id: id(TOURNAMENT_ID),
        owner_user_id: id("owner-1"),
        name: NonEmptyString::new("Pool".to_owned(), "name").unwrap(),
        invite_code: InviteCode::new("CODE123".to_owned()).unwrap(),
        join_requires_allowlist: false,
        prediction_lock_offset_minutes: lock_offset_minutes,
        status: PoolStatus::Active,
        created_at: now(),
        updated_at: now(),
    }
}

fn member(member_id: &str, user_id: &str) -> PoolMember {
    PoolMember {
        id: id(member_id),
        pool_id: id(POOL_ID),
        user_id: id(user_id),
        role: PoolRole::Member,
        status: MemberStatus::Active,
        joined_at: now(),
        left_at: None,
        created_at: now(),
        updated_at: now(),
    }
}

fn prediction(member_id: &str) -> Prediction {
    Prediction {
        id: id("prediction-1"),
        pool_member_id: id(member_id),
        match_id: id(MATCH_ID),
        home_score: Score::new(1).unwrap(),
        away_score: Score::new(0).unwrap(),
        penalties_pick: None,
        points_awarded: None,
        scored_at: None,
        created_at: now(),
        updated_at: now(),
    }
}

fn outbound(id_value: &str, channel: Channel) -> OutboundNotification {
    OutboundNotification {
        id: id_value.to_owned(),
        channel,
        user_id: "user-a".to_owned(),
        title: "Title".to_owned(),
        body: "Body".to_owned(),
    }
}

fn id(value: &str) -> DomainId {
    DomainId::new(value.to_owned()).unwrap()
}

fn now() -> UtcDateTime {
    UtcDateTime::new_iso8601(Utc::now().naive_utc().to_string()).unwrap()
}
