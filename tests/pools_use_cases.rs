mod support;

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use palpites_back::application::auth::{RegisterUserRecord, UserRepository};
use palpites_back::application::pools::{
    ChangeMemberRole, CreatePool, CreatePoolSeed, JoinPool, MembershipUseCases,
    NewAllowedEmailRecord, NewMemberRecord, NewScoringRuleRecord, PoolEmailAllowlistRepository,
    PoolMemberRepository, PoolMemberWithName, PoolRepository, PoolUseCases, ScoringRuleInput,
    ScoringRuleRepository, ScoringRuleUseCases, UpdatePoolRecord, UpdatePoolSettings,
    UpdateScoringRules,
};
use palpites_back::domain::pools::{
    MemberStatus, Pool, PoolAllowedEmail, PoolMember, PoolRole, PoolScoringRule, PoolStatus,
    ScoringRuleKey,
};
use palpites_back::domain::users::{User, UserRole};
use palpites_back::domain::{DomainId, Email, InviteCode, NonEmptyString, UtcDateTime};
use palpites_back::errors::AppError;
use support::{FixedClock, NoopNotifier};

#[tokio::test]
async fn create_seeds_rules_owner_and_invite_code() {
    let pools = FakePools::with_tournament("active");
    let use_cases = PoolUseCases::new(
        pools.clone(),
        FakeMembers::default(),
        clock(),
        NoopNotifier,
        FakeUsers::default(),
        FakeAllowlist::default(),
    );

    let pool = use_cases
        .create(
            "owner-1",
            CreatePool {
                name: "Bolão da Copa".to_owned(),
                tournament_id: "tournament-1".to_owned(),
                join_requires_allowlist: None,
                prediction_lock_offset_minutes: None,
            },
        )
        .await
        .unwrap();

    assert_eq!(pool.name.as_str(), "Bolão da Copa");
    assert_eq!(pool.owner_user_id.as_str(), "owner-1");
    assert!(!pool.invite_code.as_str().is_empty());

    let seeds = pools.seeds.lock().unwrap();
    assert_eq!(seeds.len(), 1);
    let seed = &seeds[0];

    assert_eq!(seed.scoring_rules.len(), 3);
    let exact = seed
        .scoring_rules
        .iter()
        .find(|rule| rule.rule_key == "exact_score")
        .unwrap();
    assert_eq!(exact.points, 10);
    let outcome = seed
        .scoring_rules
        .iter()
        .find(|rule| rule.rule_key == "correct_outcome")
        .unwrap();
    assert_eq!(outcome.points, 5);
    let penalties = seed
        .scoring_rules
        .iter()
        .find(|rule| rule.rule_key == "penalties_winner")
        .unwrap();
    assert_eq!(penalties.points, 5);

    assert_eq!(seed.owner_member.role, "owner");
    assert_eq!(seed.owner_member.user_id, "owner-1");
    assert!(!seed.pool.invite_code.is_empty());
}

#[tokio::test]
async fn create_rejects_archived_tournament() {
    let use_cases = PoolUseCases::new(
        FakePools::with_tournament("archived"),
        FakeMembers::default(),
        clock(),
        NoopNotifier,
        FakeUsers::default(),
        FakeAllowlist::default(),
    );

    let result = use_cases
        .create(
            "owner-1",
            CreatePool {
                name: "Bolão".to_owned(),
                tournament_id: "tournament-1".to_owned(),
                join_requires_allowlist: None,
                prediction_lock_offset_minutes: None,
            },
        )
        .await;

    assert!(matches!(
        result,
        Err(AppError::Coded {
            code: "tournament_archived",
            ..
        })
    ));
}

#[tokio::test]
async fn join_by_code_adds_active_member() {
    let pools = FakePools::with_existing_pool(pool_with_code("pool-1", "ABCDEF"));
    let members = FakeMembers::default();
    let use_cases = PoolUseCases::new(
        pools,
        members.clone(),
        clock(),
        NoopNotifier,
        FakeUsers::default(),
        FakeAllowlist::default(),
    );

    let outcome = use_cases
        .join_by_code(
            "user-9",
            JoinPool {
                invite_code: "abcdef".to_owned(),
            },
        )
        .await
        .unwrap();

    assert!(!outcome.already_member);
    assert_eq!(outcome.member.user_id.as_str(), "user-9");
    assert_eq!(outcome.member.role, PoolRole::Member);
    assert_eq!(outcome.member.status, MemberStatus::Active);
    assert_eq!(members.created.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn duplicate_join_is_idempotent() {
    let pools = FakePools::with_existing_pool(pool_with_code("pool-1", "ABCDEF"));
    let members = FakeMembers::default()
        .with_membership(member("m-1", "pool-1", "user-9", PoolRole::Member, MemberStatus::Active));
    let use_cases = PoolUseCases::new(
        pools,
        members.clone(),
        clock(),
        NoopNotifier,
        FakeUsers::default(),
        FakeAllowlist::default(),
    );

    let outcome = use_cases
        .join_by_code(
            "user-9",
            JoinPool {
                invite_code: "ABCDEF".to_owned(),
            },
        )
        .await
        .unwrap();

    assert!(outcome.already_member);
    assert_eq!(outcome.member.id.as_str(), "m-1");
    assert_eq!(members.created.lock().unwrap().len(), 0);
}

#[tokio::test]
async fn join_blocked_when_email_not_in_allowlist() {
    let pools = FakePools::with_existing_pool(pool_requiring_allowlist("pool-1", "ABCDEF"));
    let members = FakeMembers::default();
    let users = FakeUsers::default().with_user("user-9", "outsider@example.com");
    let allowlist = FakeAllowlist::default().with_email("pool-1", "allowed@example.com");
    let use_cases = PoolUseCases::new(pools, members.clone(), clock(), NoopNotifier, users, allowlist);

    let result = use_cases
        .join_by_code(
            "user-9",
            JoinPool {
                invite_code: "abcdef".to_owned(),
            },
        )
        .await;

    assert!(matches!(
        result,
        Err(AppError::Coded {
            code: "email_not_allowed",
            ..
        })
    ));
    assert_eq!(members.created.lock().unwrap().len(), 0);
}

#[tokio::test]
async fn join_allowed_when_email_in_allowlist() {
    let pools = FakePools::with_existing_pool(pool_requiring_allowlist("pool-1", "ABCDEF"));
    let members = FakeMembers::default();
    let users = FakeUsers::default().with_user("user-9", "Allowed@Example.com");
    let allowlist = FakeAllowlist::default().with_email("pool-1", "allowed@example.com");
    let use_cases = PoolUseCases::new(pools, members.clone(), clock(), NoopNotifier, users, allowlist);

    let outcome = use_cases
        .join_by_code(
            "user-9",
            JoinPool {
                invite_code: "abcdef".to_owned(),
            },
        )
        .await
        .unwrap();

    assert!(!outcome.already_member);
    assert_eq!(outcome.member.user_id.as_str(), "user-9");
    assert_eq!(members.created.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn member_role_cannot_manage_allowlist() {
    let use_cases = PoolUseCases::new(
        FakePools::default(),
        FakeMembers::default(),
        clock(),
        NoopNotifier,
        FakeUsers::default(),
        FakeAllowlist::default(),
    );

    let result = use_cases
        .add_allowed_email("pool-1", PoolRole::Member, "user-9", "x@example.com".to_owned())
        .await;

    assert!(matches!(result, Err(AppError::Forbidden(_))));
}

#[tokio::test]
async fn last_owner_cannot_leave() {
    let members = FakeMembers::default()
        .with_membership(member("m-1", "pool-1", "owner-1", PoolRole::Owner, MemberStatus::Active))
        .with_active_owners(1);
    let use_cases = PoolUseCases::new(
        FakePools::default(),
        members,
        clock(),
        NoopNotifier,
        FakeUsers::default(),
        FakeAllowlist::default(),
    );

    let result = use_cases.leave("pool-1", "owner-1").await;

    assert!(matches!(
        result,
        Err(AppError::Coded {
            code: "last_owner",
            ..
        })
    ));
}

#[tokio::test]
async fn owner_can_change_member_role() {
    let members = FakeMembers::default().with_member(member(
        "m-2",
        "pool-1",
        "user-2",
        PoolRole::Member,
        MemberStatus::Active,
    ));
    let use_cases = MembershipUseCases::new(members, clock());

    let updated = use_cases
        .change_role(
            "pool-1",
            "m-2",
            PoolRole::Owner,
            ChangeMemberRole {
                role: "admin".to_owned(),
            },
        )
        .await
        .unwrap();

    assert_eq!(updated.role, PoolRole::Admin);
}

#[tokio::test]
async fn cannot_demote_last_owner() {
    let members = FakeMembers::default()
        .with_member(member("m-1", "pool-1", "owner-1", PoolRole::Owner, MemberStatus::Active))
        .with_active_owners(1);
    let use_cases = MembershipUseCases::new(members, clock());

    let result = use_cases
        .change_role(
            "pool-1",
            "m-1",
            PoolRole::Owner,
            ChangeMemberRole {
                role: "member".to_owned(),
            },
        )
        .await;

    assert!(matches!(
        result,
        Err(AppError::Coded {
            code: "last_owner",
            ..
        })
    ));
}

#[tokio::test]
async fn member_role_cannot_update_settings() {
    let use_cases = PoolUseCases::new(
        FakePools::default(),
        FakeMembers::default(),
        clock(),
        NoopNotifier,
        FakeUsers::default(),
        FakeAllowlist::default(),
    );

    let result = use_cases
        .update_settings(
            "pool-1",
            PoolRole::Member,
            UpdatePoolSettings {
                name: "Renamed".to_owned(),
                join_requires_allowlist: false,
                prediction_lock_offset_minutes: 0,
                status: "active".to_owned(),
            },
        )
        .await;

    assert!(matches!(result, Err(AppError::Forbidden(_))));
}

#[tokio::test]
async fn non_owner_cannot_update_scoring_rules() {
    let use_cases = ScoringRuleUseCases::new(FakeScoring::default());

    let result = use_cases
        .update("pool-1", PoolRole::Admin, UpdateScoringRules { rules: vec![] })
        .await;

    assert!(matches!(result, Err(AppError::Forbidden(_))));
}

#[tokio::test]
async fn owner_updates_scoring_rules_and_rejects_duplicates() {
    let use_cases = ScoringRuleUseCases::new(FakeScoring::default());

    let rules = use_cases
        .update(
            "pool-1",
            PoolRole::Owner,
            UpdateScoringRules {
                rules: vec![
                    ScoringRuleInput {
                        rule_key: "exact_score".to_owned(),
                        points: 12,
                    },
                    ScoringRuleInput {
                        rule_key: "correct_outcome".to_owned(),
                        points: 4,
                    },
                ],
            },
        )
        .await
        .unwrap();
    assert_eq!(rules.len(), 2);

    let duplicate = use_cases
        .update(
            "pool-1",
            PoolRole::Owner,
            UpdateScoringRules {
                rules: vec![
                    ScoringRuleInput {
                        rule_key: "exact_score".to_owned(),
                        points: 1,
                    },
                    ScoringRuleInput {
                        rule_key: "exact_score".to_owned(),
                        points: 2,
                    },
                ],
            },
        )
        .await;
    assert!(matches!(
        duplicate,
        Err(AppError::Coded {
            code: "duplicate_scoring_rule",
            ..
        })
    ));
}

#[derive(Clone, Default)]
struct FakePools {
    tournament_status: Option<String>,
    pools: Arc<Mutex<Vec<Pool>>>,
    seeds: Arc<Mutex<Vec<CreatePoolSeed>>>,
}

impl FakePools {
    fn with_tournament(status: &str) -> Self {
        Self {
            tournament_status: Some(status.to_owned()),
            ..Self::default()
        }
    }

    fn with_existing_pool(pool: Pool) -> Self {
        let fake = Self::default();
        fake.pools.lock().unwrap().push(pool);
        fake
    }
}

#[async_trait]
impl PoolRepository for FakePools {
    async fn find_by_id(&self, id: &str) -> Result<Option<Pool>, AppError> {
        Ok(self
            .pools
            .lock()
            .unwrap()
            .iter()
            .find(|pool| pool.id.as_str() == id)
            .cloned())
    }

    async fn find_by_invite_code(&self, invite_code: &str) -> Result<Option<Pool>, AppError> {
        Ok(self
            .pools
            .lock()
            .unwrap()
            .iter()
            .find(|pool| pool.invite_code.as_str() == invite_code)
            .cloned())
    }

    async fn list_for_user(&self, _user_id: &str) -> Result<Vec<Pool>, AppError> {
        Ok(self.pools.lock().unwrap().clone())
    }

    async fn list_for_tournament(&self, _tournament_id: &str) -> Result<Vec<Pool>, AppError> {
        Ok(self.pools.lock().unwrap().clone())
    }

    async fn find_tournament_status(
        &self,
        _tournament_id: &str,
    ) -> Result<Option<String>, AppError> {
        Ok(self.tournament_status.clone())
    }

    async fn create_with_seed(&self, seed: CreatePoolSeed) -> Result<(), AppError> {
        self.pools.lock().unwrap().push(pool_from_seed(&seed));
        self.seeds.lock().unwrap().push(seed);
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
    membership: Option<PoolMember>,
    members: Arc<Mutex<Vec<PoolMember>>>,
    created: Arc<Mutex<Vec<NewMemberRecord>>>,
    active_owners: u64,
}

impl FakeMembers {
    fn with_membership(mut self, member: PoolMember) -> Self {
        self.membership = Some(member);
        self
    }

    fn with_member(self, member: PoolMember) -> Self {
        self.members.lock().unwrap().push(member);
        self
    }

    fn with_active_owners(mut self, owners: u64) -> Self {
        self.active_owners = owners;
        self
    }
}

#[async_trait]
impl PoolMemberRepository for FakeMembers {
    async fn find_membership(
        &self,
        _pool_id: &str,
        _user_id: &str,
    ) -> Result<Option<PoolMember>, AppError> {
        Ok(self.membership.clone())
    }

    async fn find_by_id(&self, member_id: &str) -> Result<Option<PoolMember>, AppError> {
        Ok(self
            .members
            .lock()
            .unwrap()
            .iter()
            .find(|member| member.id.as_str() == member_id)
            .cloned())
    }

    async fn list_for_pool(&self, _pool_id: &str) -> Result<Vec<PoolMember>, AppError> {
        Ok(self.members.lock().unwrap().clone())
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
        Ok(self.active_owners)
    }

    async fn create(&self, record: NewMemberRecord) -> Result<(), AppError> {
        self.members.lock().unwrap().push(member_from_record(&record));
        self.created.lock().unwrap().push(record);
        Ok(())
    }

    async fn reactivate(&self, member_id: &str, _joined_at: &str) -> Result<(), AppError> {
        let mut members = self.members.lock().unwrap();
        if let Some(existing) = members.iter_mut().find(|m| m.id.as_str() == member_id) {
            existing.status = MemberStatus::Active;
            existing.role = PoolRole::Member;
            existing.left_at = None;
        }
        Ok(())
    }

    async fn set_status(
        &self,
        member_id: &str,
        status: &str,
        _left_at: Option<&str>,
    ) -> Result<(), AppError> {
        let mut members = self.members.lock().unwrap();
        if let Some(existing) = members.iter_mut().find(|m| m.id.as_str() == member_id) {
            existing.status = MemberStatus::parse(status).unwrap();
        }
        Ok(())
    }

    async fn set_role(&self, member_id: &str, role: &str) -> Result<(), AppError> {
        let mut members = self.members.lock().unwrap();
        if let Some(existing) = members.iter_mut().find(|m| m.id.as_str() == member_id) {
            existing.role = PoolRole::parse(role).unwrap();
        }
        Ok(())
    }
}

#[derive(Clone, Default)]
struct FakeScoring {
    rules: Arc<Mutex<Vec<PoolScoringRule>>>,
}

#[async_trait]
impl ScoringRuleRepository for FakeScoring {
    async fn list_for_pool(&self, _pool_id: &str) -> Result<Vec<PoolScoringRule>, AppError> {
        Ok(self.rules.lock().unwrap().clone())
    }

    async fn set_rules(
        &self,
        pool_id: &str,
        rules: Vec<NewScoringRuleRecord>,
    ) -> Result<(), AppError> {
        let mapped = rules.iter().map(|rule| rule_from_record(pool_id, rule)).collect();
        *self.rules.lock().unwrap() = mapped;
        Ok(())
    }
}

#[derive(Clone, Default)]
struct FakeUsers {
    users: Arc<Mutex<Vec<User>>>,
}

impl FakeUsers {
    fn with_user(self, user_id: &str, email: &str) -> Self {
        self.users.lock().unwrap().push(user_record(user_id, email));
        self
    }
}

#[async_trait]
impl UserRepository for FakeUsers {
    async fn find_by_id(&self, user_id: &str) -> Result<Option<User>, AppError> {
        Ok(self
            .users
            .lock()
            .unwrap()
            .iter()
            .find(|user| user.id.as_str() == user_id)
            .cloned())
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError> {
        Ok(self
            .users
            .lock()
            .unwrap()
            .iter()
            .find(|user| user.email.as_str() == email)
            .cloned())
    }

    async fn register_user(&self, _record: RegisterUserRecord) -> Result<(), AppError> {
        Ok(())
    }

    async fn set_sync_predictions_across_pools(
        &self,
        _user_id: &str,
        _value: bool,
    ) -> Result<(), AppError> {
        Ok(())
    }
}

#[derive(Clone, Default)]
struct FakeAllowlist {
    entries: Arc<Mutex<Vec<PoolAllowedEmail>>>,
}

impl FakeAllowlist {
    fn with_email(self, pool_id: &str, email: &str) -> Self {
        let entry_id = format!("ae-{}", self.entries.lock().unwrap().len());
        self.entries
            .lock()
            .unwrap()
            .push(allowed_email(&entry_id, pool_id, email));
        self
    }
}

#[async_trait]
impl PoolEmailAllowlistRepository for FakeAllowlist {
    async fn list_for_pool(&self, pool_id: &str) -> Result<Vec<PoolAllowedEmail>, AppError> {
        Ok(self
            .entries
            .lock()
            .unwrap()
            .iter()
            .filter(|entry| entry.pool_id.as_str() == pool_id)
            .cloned()
            .collect())
    }

    async fn find_by_id(&self, entry_id: &str) -> Result<Option<PoolAllowedEmail>, AppError> {
        Ok(self
            .entries
            .lock()
            .unwrap()
            .iter()
            .find(|entry| entry.id.as_str() == entry_id)
            .cloned())
    }

    async fn is_email_allowed(&self, pool_id: &str, email: &str) -> Result<bool, AppError> {
        Ok(self
            .entries
            .lock()
            .unwrap()
            .iter()
            .any(|entry| entry.pool_id.as_str() == pool_id && entry.email.as_str() == email))
    }

    async fn add(&self, record: NewAllowedEmailRecord) -> Result<PoolAllowedEmail, AppError> {
        let entry = allowed_email(&record.id, &record.pool_id, &record.email);
        self.entries.lock().unwrap().push(entry.clone());
        Ok(entry)
    }

    async fn remove(&self, pool_id: &str, entry_id: &str) -> Result<(), AppError> {
        self.entries
            .lock()
            .unwrap()
            .retain(|entry| !(entry.pool_id.as_str() == pool_id && entry.id.as_str() == entry_id));
        Ok(())
    }
}

fn clock() -> FixedClock {
    FixedClock::new("2026-06-18 12:00:00")
}

fn id(value: &str) -> DomainId {
    DomainId::new(value.to_owned()).unwrap()
}

fn now() -> UtcDateTime {
    UtcDateTime::new_iso8601("2026-06-18 12:00:00".to_owned()).unwrap()
}

fn pool_with_code(pool_id: &str, code: &str) -> Pool {
    Pool {
        id: id(pool_id),
        tournament_id: id("tournament-1"),
        owner_user_id: id("owner-1"),
        name: NonEmptyString::new("Bolão".to_owned(), "pool.name").unwrap(),
        invite_code: InviteCode::new(code.to_owned()).unwrap(),
        join_requires_allowlist: false,
        prediction_lock_offset_minutes: 0,
        status: PoolStatus::Active,
        created_at: now(),
        updated_at: now(),
    }
}

fn pool_requiring_allowlist(pool_id: &str, code: &str) -> Pool {
    let mut pool = pool_with_code(pool_id, code);
    pool.join_requires_allowlist = true;
    pool
}

fn user_record(user_id: &str, email: &str) -> User {
    User {
        id: id(user_id),
        email: Email::new(email.to_owned()).unwrap(),
        password_hash: NonEmptyString::new("hash".to_owned(), "user.password_hash").unwrap(),
        display_name: NonEmptyString::new("Tester".to_owned(), "user.display_name").unwrap(),
        role: UserRole::Member,
        avatar_file_id: None,
        sync_predictions_across_pools: true,
        is_active: true,
        last_login_at: None,
        created_at: now(),
        updated_at: now(),
    }
}

fn allowed_email(entry_id: &str, pool_id: &str, email: &str) -> PoolAllowedEmail {
    PoolAllowedEmail {
        id: id(entry_id),
        pool_id: id(pool_id),
        email: Email::new(email.to_owned()).unwrap(),
        added_by_user_id: id("owner-1"),
        created_at: now(),
    }
}

fn pool_from_seed(seed: &CreatePoolSeed) -> Pool {
    Pool {
        id: id(&seed.pool.pool_id),
        tournament_id: id(&seed.pool.tournament_id),
        owner_user_id: id(&seed.pool.owner_user_id),
        name: NonEmptyString::new(seed.pool.name.clone(), "pool.name").unwrap(),
        invite_code: InviteCode::new(seed.pool.invite_code.clone()).unwrap(),
        join_requires_allowlist: seed.pool.join_requires_allowlist,
        prediction_lock_offset_minutes: seed.pool.prediction_lock_offset_minutes,
        status: PoolStatus::parse(&seed.pool.status).unwrap(),
        created_at: now(),
        updated_at: now(),
    }
}

fn member(
    member_id: &str,
    pool_id: &str,
    user_id: &str,
    role: PoolRole,
    status: MemberStatus,
) -> PoolMember {
    PoolMember {
        id: id(member_id),
        pool_id: id(pool_id),
        user_id: id(user_id),
        role,
        status,
        joined_at: now(),
        left_at: None,
        created_at: now(),
        updated_at: now(),
    }
}

fn member_from_record(record: &NewMemberRecord) -> PoolMember {
    PoolMember {
        id: id(&record.member_id),
        pool_id: id(&record.pool_id),
        user_id: id(&record.user_id),
        role: PoolRole::parse(&record.role).unwrap(),
        status: MemberStatus::parse(&record.status).unwrap(),
        joined_at: now(),
        left_at: None,
        created_at: now(),
        updated_at: now(),
    }
}

fn rule_from_record(pool_id: &str, record: &NewScoringRuleRecord) -> PoolScoringRule {
    PoolScoringRule {
        id: id(&record.rule_id),
        pool_id: id(pool_id),
        rule_key: ScoringRuleKey::parse(&record.rule_key).unwrap(),
        points: record.points,
        created_at: now(),
        updated_at: now(),
    }
}
