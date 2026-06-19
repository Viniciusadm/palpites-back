mod support;

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use palpites_back::application::notifications::{
    NewNotificationRecord, NewOutboxRecord, NewPreferenceRecord, NotificationOutboxWriter,
    NotificationPreferenceRepository, NotificationRepository, NotificationUseCases, PreferenceInput,
    UpdatePreferences,
};
use palpites_back::application::pools::{
    CreatePoolSeed, NewMemberRecord, PoolMemberRepository, PoolMemberWithName, PoolRepository,
    UpdatePoolRecord,
};
use palpites_back::application::shared::Notifier;
use palpites_back::domain::matches::{Match, MatchStatus};
use palpites_back::domain::notifications::{
    Channel, Notification, NotificationPreference, NotificationType,
};
use palpites_back::domain::pools::{
    MemberStatus, Pool, PoolMember, PoolRole, PoolStatus,
};
use palpites_back::domain::{DomainId, InviteCode, NonEmptyString, UtcDateTime};
use palpites_back::errors::AppError;
use support::FixedClock;

const POOL_ID: &str = "pool-1";
const TOURNAMENT_ID: &str = "tournament-1";

type UseCases = NotificationUseCases<
    FakeNotifications,
    FakePreferences,
    FakePools,
    FakeMembers,
    FakeOutbox,
    FixedClock,
>;

fn build(
    notifications: FakeNotifications,
    preferences: FakePreferences,
    pools: FakePools,
    members: FakeMembers,
) -> UseCases {
    build_with_outbox(notifications, preferences, pools, members, FakeOutbox::default())
}

fn build_with_outbox(
    notifications: FakeNotifications,
    preferences: FakePreferences,
    pools: FakePools,
    members: FakeMembers,
    outbox: FakeOutbox,
) -> UseCases {
    NotificationUseCases::new(
        notifications,
        preferences,
        pools,
        members,
        outbox,
        FixedClock::new("2026-06-18 12:00:00"),
    )
}

#[tokio::test]
async fn member_joined_notifies_owner_and_admins_only() {
    let notifications = FakeNotifications::default();
    let members = FakeMembers::with(vec![
        member("m-owner", "owner-u", PoolRole::Owner, MemberStatus::Active),
        member("m-admin", "admin-u", PoolRole::Admin, MemberStatus::Active),
        member("m-member", "member-u", PoolRole::Member, MemberStatus::Active),
        member("m-joiner", "joiner-u", PoolRole::Member, MemberStatus::Active),
    ]);
    let use_cases = build(
        notifications.clone(),
        FakePreferences::default(),
        FakePools::default(),
        members,
    );

    use_cases.member_joined(POOL_ID, "joiner-u").await.unwrap();

    let rows = notifications.rows.lock().unwrap();
    let recipients: Vec<String> = rows.iter().map(|n| n.user_id.as_str().to_owned()).collect();
    assert_eq!(rows.len(), 2);
    assert!(recipients.contains(&"owner-u".to_owned()));
    assert!(recipients.contains(&"admin-u".to_owned()));
    assert!(!recipients.contains(&"member-u".to_owned()));
    assert!(!recipients.contains(&"joiner-u".to_owned()));
    assert!(rows
        .iter()
        .all(|n| n.notification_type == NotificationType::MemberJoined));
}

#[tokio::test]
async fn member_joined_enqueues_push_outbox_for_recipients() {
    let outbox = FakeOutbox::default();
    let members = FakeMembers::with(vec![
        member("m-owner", "owner-u", PoolRole::Owner, MemberStatus::Active),
        member("m-member", "member-u", PoolRole::Member, MemberStatus::Active),
    ]);
    let use_cases = build_with_outbox(
        FakeNotifications::default(),
        FakePreferences::default(),
        FakePools::default(),
        members,
        outbox.clone(),
    );

    use_cases.member_joined(POOL_ID, "joiner-u").await.unwrap();

    let enqueued = outbox.enqueued.lock().unwrap();
    // Only the owner can manage members, so a single push is enqueued, on the push channel.
    assert_eq!(enqueued.len(), 1);
    assert_eq!(enqueued[0].channel, "push");
    assert_eq!(enqueued[0].user_id, "owner-u");
}

#[tokio::test]
async fn disabled_push_channel_suppresses_outbox() {
    let outbox = FakeOutbox::default();
    let preferences = FakePreferences::with(vec![pref(
        "owner-u",
        Some(POOL_ID),
        NotificationType::MemberJoined,
        Channel::Push,
        false,
    )]);
    let members = FakeMembers::with(vec![member(
        "m-owner",
        "owner-u",
        PoolRole::Owner,
        MemberStatus::Active,
    )]);
    let use_cases = build_with_outbox(
        FakeNotifications::default(),
        preferences,
        FakePools::default(),
        members,
        outbox.clone(),
    );

    use_cases.member_joined(POOL_ID, "joiner-u").await.unwrap();

    assert!(outbox.enqueued.lock().unwrap().is_empty());
}

#[tokio::test]
async fn disabled_type_suppresses_creation() {
    let notifications = FakeNotifications::default();
    let preferences = FakePreferences::with(vec![pref(
        "owner-u",
        Some(POOL_ID),
        NotificationType::MemberJoined,
        Channel::InApp,
        false,
    )]);
    let members = FakeMembers::with(vec![member(
        "m-owner",
        "owner-u",
        PoolRole::Owner,
        MemberStatus::Active,
    )]);
    let use_cases = build(notifications.clone(), preferences, FakePools::default(), members);

    use_cases.member_joined(POOL_ID, "joiner-u").await.unwrap();

    assert!(notifications.rows.lock().unwrap().is_empty());
}

#[tokio::test]
async fn preference_resolution_specific_overrides_global_default() {
    let notifications = FakeNotifications::default();
    let preferences = FakePreferences::with(vec![
        pref("owner-u", None, NotificationType::MemberJoined, Channel::InApp, false),
        pref(
            "owner-u",
            Some(POOL_ID),
            NotificationType::MemberJoined,
            Channel::InApp,
            true,
        ),
    ]);
    let members = FakeMembers::with(vec![member(
        "m-owner",
        "owner-u",
        PoolRole::Owner,
        MemberStatus::Active,
    )]);
    let use_cases = build(notifications.clone(), preferences, FakePools::default(), members);

    use_cases.member_joined(POOL_ID, "joiner-u").await.unwrap();

    assert_eq!(notifications.rows.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn preference_resolution_global_default_applies_without_specific() {
    let notifications = FakeNotifications::default();
    let preferences = FakePreferences::with(vec![pref(
        "owner-u",
        None,
        NotificationType::MemberJoined,
        Channel::InApp,
        false,
    )]);
    let members = FakeMembers::with(vec![member(
        "m-owner",
        "owner-u",
        PoolRole::Owner,
        MemberStatus::Active,
    )]);
    let use_cases = build(notifications.clone(), preferences, FakePools::default(), members);

    use_cases.member_joined(POOL_ID, "joiner-u").await.unwrap();

    assert!(notifications.rows.lock().unwrap().is_empty());
}

#[tokio::test]
async fn preference_resolution_implicit_enabled_without_any_row() {
    let notifications = FakeNotifications::default();
    let members = FakeMembers::with(vec![member(
        "m-owner",
        "owner-u",
        PoolRole::Owner,
        MemberStatus::Active,
    )]);
    let use_cases = build(
        notifications.clone(),
        FakePreferences::default(),
        FakePools::default(),
        members,
    );

    use_cases.member_joined(POOL_ID, "joiner-u").await.unwrap();

    assert_eq!(notifications.rows.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn new_match_notifies_active_members_of_tournament_pools() {
    let notifications = FakeNotifications::default();
    let pools = FakePools::with(vec![pool(POOL_ID, TOURNAMENT_ID)]);
    let members = FakeMembers::with(vec![
        member("m-1", "user-1", PoolRole::Member, MemberStatus::Active),
        member("m-2", "user-2", PoolRole::Member, MemberStatus::Inactive),
    ]);
    let use_cases = build(notifications.clone(), FakePreferences::default(), pools, members);

    use_cases.new_match(&match_row("match-1", TOURNAMENT_ID)).await.unwrap();

    let rows = notifications.rows.lock().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].user_id.as_str(), "user-1");
    assert_eq!(rows[0].notification_type, NotificationType::NewMatch);
    assert_eq!(rows[0].related_match_id.as_ref().unwrap().as_str(), "match-1");
}

#[tokio::test]
async fn dispatch_deduplicates_per_user_across_pools() {
    let notifications = FakeNotifications::default();
    let outbox = FakeOutbox::default();
    let pools = FakePools::with(vec![
        pool("pool-a", TOURNAMENT_ID),
        pool("pool-b", TOURNAMENT_ID),
    ]);
    // The fake returns the same members for every pool, so this user belongs to both.
    let members = FakeMembers::with(vec![member(
        "m-1",
        "user-1",
        PoolRole::Member,
        MemberStatus::Active,
    )]);
    let use_cases = build_with_outbox(
        notifications.clone(),
        FakePreferences::default(),
        pools,
        members,
        outbox.clone(),
    );

    use_cases.new_match(&match_row("match-1", TOURNAMENT_ID)).await.unwrap();

    let rows = notifications.rows.lock().unwrap();
    assert_eq!(rows.len(), 1, "a user in two pools must get a single in-app notification");
    assert_eq!(rows[0].user_id.as_str(), "user-1");
    assert!(rows[0].pool_id.is_none(), "deduplicated notification is account-level");

    let enqueued = outbox.enqueued.lock().unwrap();
    assert_eq!(enqueued.len(), 1, "a user in two pools must get a single push");
    assert_eq!(enqueued[0].user_id, "user-1");
}

#[tokio::test]
async fn dispatch_enables_channel_if_any_pool_enables_it() {
    let outbox = FakeOutbox::default();
    let pools = FakePools::with(vec![
        pool("pool-a", TOURNAMENT_ID),
        pool("pool-b", TOURNAMENT_ID),
    ]);
    let members = FakeMembers::with(vec![member(
        "m-1",
        "user-1",
        PoolRole::Member,
        MemberStatus::Active,
    )]);
    // Push disabled in pool-a but enabled in pool-b -> the OR aggregate yields one push.
    let preferences = FakePreferences::with(vec![
        pref("user-1", Some("pool-a"), NotificationType::NewMatch, Channel::Push, false),
        pref("user-1", Some("pool-b"), NotificationType::NewMatch, Channel::Push, true),
    ]);
    let use_cases = build_with_outbox(
        FakeNotifications::default(),
        preferences,
        pools,
        members,
        outbox.clone(),
    );

    use_cases.new_match(&match_row("match-1", TOURNAMENT_ID)).await.unwrap();

    let enqueued = outbox.enqueued.lock().unwrap();
    assert_eq!(enqueued.len(), 1);
    assert_eq!(enqueued[0].user_id, "user-1");
}

#[tokio::test]
async fn mark_read_sets_read_at_for_owner() {
    let notifications = FakeNotifications::default();
    notifications.seed(notification("n-1", "user-1"));
    let use_cases = build(
        notifications.clone(),
        FakePreferences::default(),
        FakePools::default(),
        FakeMembers::default(),
    );

    let updated = use_cases.mark_read("user-1", "n-1").await.unwrap();

    assert!(updated.read_at.is_some());
}

#[tokio::test]
async fn mark_read_rejects_other_users_notification() {
    let notifications = FakeNotifications::default();
    notifications.seed(notification("n-1", "user-1"));
    let use_cases = build(
        notifications.clone(),
        FakePreferences::default(),
        FakePools::default(),
        FakeMembers::default(),
    );

    let result = use_cases.mark_read("user-2", "n-1").await;

    assert!(matches!(result, Err(AppError::NotFound(_))));
}

#[tokio::test]
async fn update_preferences_rejects_invite_type() {
    let preferences = FakePreferences::default();
    let use_cases = build(
        FakeNotifications::default(),
        preferences,
        FakePools::default(),
        FakeMembers::default(),
    );

    let result = use_cases
        .update_preferences(
            "user-1",
            POOL_ID,
            UpdatePreferences {
                items: vec![PreferenceInput {
                    notification_type: "invite".to_owned(),
                    channel: "in_app".to_owned(),
                    enabled: false,
                }],
            },
        )
        .await;

    assert!(matches!(
        result,
        Err(AppError::Coded {
            code: "notification_type_not_preferenceable",
            ..
        })
    ));
}

#[derive(Clone, Default)]
struct FakeOutbox {
    enqueued: Arc<Mutex<Vec<NewOutboxRecord>>>,
}

#[async_trait]
impl NotificationOutboxWriter for FakeOutbox {
    async fn enqueue_many(&self, records: Vec<NewOutboxRecord>) -> Result<(), AppError> {
        self.enqueued.lock().unwrap().extend(records);
        Ok(())
    }
}

#[derive(Clone, Default)]
struct FakeNotifications {
    rows: Arc<Mutex<Vec<Notification>>>,
}

impl FakeNotifications {
    fn seed(&self, notification: Notification) {
        self.rows.lock().unwrap().push(notification);
    }
}

#[async_trait]
impl NotificationRepository for FakeNotifications {
    async fn create_many(&self, records: Vec<NewNotificationRecord>) -> Result<(), AppError> {
        let mut rows = self.rows.lock().unwrap();
        for record in records {
            rows.push(notification_from_record(&record));
        }
        Ok(())
    }

    async fn list_for_user(
        &self,
        user_id: &str,
        only_unread: bool,
    ) -> Result<Vec<Notification>, AppError> {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .iter()
            .filter(|n| n.user_id.as_str() == user_id)
            .filter(|n| !only_unread || n.read_at.is_none())
            .cloned()
            .collect())
    }

    async fn find_by_id(&self, id: &str) -> Result<Option<Notification>, AppError> {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .iter()
            .find(|n| n.id.as_str() == id)
            .cloned())
    }

    async fn mark_read(&self, id: &str, read_at: &str) -> Result<(), AppError> {
        let mut rows = self.rows.lock().unwrap();
        if let Some(notification) = rows.iter_mut().find(|n| n.id.as_str() == id) {
            notification.read_at = Some(UtcDateTime::new_iso8601(read_at.to_owned()).unwrap());
        }
        Ok(())
    }

    async fn mark_all_read(&self, user_id: &str, read_at: &str) -> Result<(), AppError> {
        let mut rows = self.rows.lock().unwrap();
        for notification in rows.iter_mut().filter(|n| n.user_id.as_str() == user_id) {
            if notification.read_at.is_none() {
                notification.read_at = Some(UtcDateTime::new_iso8601(read_at.to_owned()).unwrap());
            }
        }
        Ok(())
    }
}

#[derive(Clone, Default)]
struct FakePreferences {
    rows: Arc<Mutex<Vec<NotificationPreference>>>,
    upserted: Arc<Mutex<Vec<NewPreferenceRecord>>>,
}

impl FakePreferences {
    fn with(rows: Vec<NotificationPreference>) -> Self {
        Self {
            rows: Arc::new(Mutex::new(rows)),
            upserted: Arc::default(),
        }
    }
}

#[async_trait]
impl NotificationPreferenceRepository for FakePreferences {
    async fn list_for_user(
        &self,
        user_id: &str,
    ) -> Result<Vec<NotificationPreference>, AppError> {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .iter()
            .filter(|pref| pref.user_id.as_str() == user_id)
            .cloned()
            .collect())
    }

    async fn list_for_scope(
        &self,
        user_id: &str,
        pool_id: &str,
    ) -> Result<Vec<NotificationPreference>, AppError> {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .iter()
            .filter(|pref| pref.user_id.as_str() == user_id)
            .filter(|pref| pref.pool_id.as_ref().map(|id| id.as_str()) == Some(pool_id))
            .cloned()
            .collect())
    }

    async fn list_for_global(
        &self,
        user_id: &str,
    ) -> Result<Vec<NotificationPreference>, AppError> {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .iter()
            .filter(|pref| pref.user_id.as_str() == user_id)
            .filter(|pref| pref.pool_id.is_none())
            .cloned()
            .collect())
    }

    async fn upsert_many(&self, records: Vec<NewPreferenceRecord>) -> Result<(), AppError> {
        self.upserted.lock().unwrap().extend(records);
        Ok(())
    }
}

#[derive(Clone, Default)]
struct FakePools {
    pools: Vec<Pool>,
}

impl FakePools {
    fn with(pools: Vec<Pool>) -> Self {
        Self { pools }
    }
}

#[async_trait]
impl PoolRepository for FakePools {
    async fn find_by_id(&self, id: &str) -> Result<Option<Pool>, AppError> {
        Ok(self.pools.iter().find(|pool| pool.id.as_str() == id).cloned())
    }

    async fn find_by_invite_code(&self, _invite_code: &str) -> Result<Option<Pool>, AppError> {
        Ok(None)
    }

    async fn list_for_user(&self, _user_id: &str) -> Result<Vec<Pool>, AppError> {
        Ok(Vec::new())
    }

    async fn list_for_tournament(&self, tournament_id: &str) -> Result<Vec<Pool>, AppError> {
        Ok(self
            .pools
            .iter()
            .filter(|pool| pool.tournament_id.as_str() == tournament_id)
            .cloned()
            .collect())
    }

    async fn find_tournament_status(&self, _tournament_id: &str) -> Result<Option<String>, AppError> {
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

#[derive(Clone, Default)]
struct FakeMembers {
    members: Vec<PoolMember>,
}

impl FakeMembers {
    fn with(members: Vec<PoolMember>) -> Self {
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

fn id(value: &str) -> DomainId {
    DomainId::new(value.to_owned()).unwrap()
}

fn now() -> UtcDateTime {
    UtcDateTime::new_iso8601("2026-06-18 12:00:00".to_owned()).unwrap()
}

fn member(member_id: &str, user_id: &str, role: PoolRole, status: MemberStatus) -> PoolMember {
    PoolMember {
        id: id(member_id),
        pool_id: id(POOL_ID),
        user_id: id(user_id),
        role,
        status,
        joined_at: now(),
        left_at: None,
        created_at: now(),
        updated_at: now(),
    }
}

fn pref(
    user_id: &str,
    pool_id: Option<&str>,
    notification_type: NotificationType,
    channel: Channel,
    enabled: bool,
) -> NotificationPreference {
    NotificationPreference {
        id: id("pref"),
        user_id: id(user_id),
        pool_id: pool_id.map(id),
        notification_type,
        channel,
        enabled,
        created_at: now(),
        updated_at: now(),
    }
}

fn pool(pool_id: &str, tournament_id: &str) -> Pool {
    Pool {
        id: id(pool_id),
        tournament_id: id(tournament_id),
        owner_user_id: id("owner-u"),
        name: NonEmptyString::new("Bolão".to_owned(), "pool.name").unwrap(),
        invite_code: InviteCode::new("ABCDEF".to_owned()).unwrap(),
        join_requires_allowlist: false,
        prediction_lock_offset_minutes: 0,
        status: PoolStatus::Active,
        created_at: now(),
        updated_at: now(),
    }
}

fn match_row(match_id: &str, tournament_id: &str) -> Match {
    Match {
        id: id(match_id),
        tournament_id: id(tournament_id),
        stage_id: id("stage-1"),
        home_team_id: None,
        away_team_id: None,
        kickoff_at: now(),
        status: MatchStatus::Scheduled,
        home_score: None,
        away_score: None,
        finished_at: None,
        created_at: now(),
        updated_at: now(),
    }
}

fn notification(id_value: &str, user_id: &str) -> Notification {
    Notification {
        id: id(id_value),
        user_id: id(user_id),
        pool_id: Some(id(POOL_ID)),
        notification_type: NotificationType::MemberJoined,
        title: NonEmptyString::new("New member joined".to_owned(), "notification.title").unwrap(),
        body: NonEmptyString::new("A new member has joined your pool.".to_owned(), "notification.body")
            .unwrap(),
        related_match_id: None,
        read_at: None,
        created_at: now(),
    }
}

fn notification_from_record(record: &NewNotificationRecord) -> Notification {
    Notification {
        id: id(&record.id),
        user_id: id(&record.user_id),
        pool_id: record.pool_id.as_deref().map(id),
        notification_type: NotificationType::parse(&record.notification_type).unwrap(),
        title: NonEmptyString::new(record.title.clone(), "notification.title").unwrap(),
        body: NonEmptyString::new(record.body.clone(), "notification.body").unwrap(),
        related_match_id: record.related_match_id.as_deref().map(id),
        read_at: None,
        created_at: now(),
    }
}
