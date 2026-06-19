use uuid::Uuid;

use crate::application::auth::UserRepository;
use crate::application::pools::{
    ChangeMemberRole, CreatePool, CreatePoolSeed, JoinOutcome, JoinPool, NewAllowedEmailRecord,
    NewMemberRecord, NewPoolRecord, NewScoringRuleRecord, PoolEmailAllowlistRepository,
    PoolMemberRepository, PoolMemberWithName, PoolRepository, ScoringRuleInput,
    ScoringRuleRepository, UpdatePoolRecord, UpdatePoolSettings, UpdateScoringRules,
};
use crate::application::shared::{Clock, Notifier};
use crate::domain::pools::{
    MemberStatus, Pool, PoolAllowedEmail, PoolMember, PoolRole, PoolScoringRule, PoolStatus,
    ScoringRuleKey,
};
use crate::domain::{Email, InviteCode, NonEmptyString};
use crate::errors::AppError;

const MAX_LOCK_OFFSET_MINUTES: u16 = 10080;
const MAX_RULE_POINTS: i16 = 1000;
const INVITE_CODE_ATTEMPTS: usize = 8;

pub struct PoolUseCases<P, M, C, N, U, A> {
    pools: P,
    members: M,
    clock: C,
    notifier: N,
    users: U,
    allowlist: A,
}

impl<P, M, C, N, U, A> PoolUseCases<P, M, C, N, U, A>
where
    P: PoolRepository,
    M: PoolMemberRepository,
    C: Clock,
    N: Notifier,
    U: UserRepository,
    A: PoolEmailAllowlistRepository,
{
    pub fn new(pools: P, members: M, clock: C, notifier: N, users: U, allowlist: A) -> Self {
        Self {
            pools,
            members,
            clock,
            notifier,
            users,
            allowlist,
        }
    }

    pub async fn list_mine(&self, user_id: &str) -> Result<Vec<Pool>, AppError> {
        self.pools.list_for_user(user_id).await
    }

    pub async fn get(&self, pool_id: &str) -> Result<Pool, AppError> {
        self.pools
            .find_by_id(pool_id)
            .await?
            .ok_or_else(|| AppError::NotFound("pool was not found".to_owned()))
    }

    pub async fn create(
        &self,
        owner_user_id: &str,
        command: CreatePool,
    ) -> Result<Pool, AppError> {
        let name = validate_name(command.name)?;
        let join_requires_allowlist = command.join_requires_allowlist.unwrap_or(false);
        let lock_offset = validate_lock_offset(command.prediction_lock_offset_minutes.unwrap_or(0))?;

        let status = self
            .pools
            .find_tournament_status(&command.tournament_id)
            .await?
            .ok_or_else(|| AppError::NotFound("tournament was not found".to_owned()))?;
        if status == "archived" {
            return Err(AppError::conflict_code(
                "tournament_archived",
                "tournament is archived",
            ));
        }

        let invite_code = self.generate_invite_code().await?;
        let now = self.clock.now().as_str().to_owned();
        let pool_id = new_id();

        let seed = CreatePoolSeed {
            pool: NewPoolRecord {
                pool_id: pool_id.clone(),
                tournament_id: command.tournament_id,
                owner_user_id: owner_user_id.to_owned(),
                name,
                invite_code,
                join_requires_allowlist,
                prediction_lock_offset_minutes: lock_offset,
                status: PoolStatus::Active.as_str().to_owned(),
            },
            scoring_rules: vec![
                NewScoringRuleRecord {
                    rule_id: new_id(),
                    rule_key: ScoringRuleKey::ExactScore.as_str().to_owned(),
                    points: 10,
                },
                NewScoringRuleRecord {
                    rule_id: new_id(),
                    rule_key: ScoringRuleKey::CorrectOutcome.as_str().to_owned(),
                    points: 5,
                },
            ],
            owner_member: NewMemberRecord {
                member_id: new_id(),
                pool_id: pool_id.clone(),
                user_id: owner_user_id.to_owned(),
                role: PoolRole::Owner.as_str().to_owned(),
                status: MemberStatus::Active.as_str().to_owned(),
                joined_at: now,
            },
        };

        self.pools.create_with_seed(seed).await?;
        self.get(&pool_id).await
    }

    pub async fn update_settings(
        &self,
        pool_id: &str,
        actor: PoolRole,
        command: UpdatePoolSettings,
    ) -> Result<Pool, AppError> {
        if !actor.can_manage_members() {
            return Err(AppError::Forbidden(
                "only an owner or admin can update pool settings".to_owned(),
            ));
        }
        self.get(pool_id).await?;

        let name = validate_name(command.name)?;
        let status = PoolStatus::parse(&command.status)?;
        let lock_offset = validate_lock_offset(command.prediction_lock_offset_minutes)?;

        self.pools
            .update_settings(UpdatePoolRecord {
                pool_id: pool_id.to_owned(),
                name,
                join_requires_allowlist: command.join_requires_allowlist,
                prediction_lock_offset_minutes: lock_offset,
                status: status.as_str().to_owned(),
            })
            .await?;

        self.get(pool_id).await
    }

    pub async fn join_by_code(
        &self,
        user_id: &str,
        command: JoinPool,
    ) -> Result<JoinOutcome, AppError> {
        let invite_code = InviteCode::new(command.invite_code)?.as_str().to_owned();
        let pool = self
            .pools
            .find_by_invite_code(&invite_code)
            .await?
            .ok_or_else(|| AppError::NotFound("invite code was not found".to_owned()))?;
        let pool_id = pool.id.as_str().to_owned();

        if let Some(member) = self.members.find_membership(&pool_id, user_id).await? {
            if member.status == MemberStatus::Active {
                return Ok(JoinOutcome {
                    pool,
                    member,
                    already_member: true,
                });
            }

            self.ensure_email_allowed(&pool, user_id).await?;

            let now = self.clock.now().as_str().to_owned();
            self.members
                .reactivate(member.id.as_str(), &now)
                .await?;
            let member = self.reload_member(member.id.as_str()).await?;
            self.notifier.member_joined(&pool_id, user_id).await?;
            return Ok(JoinOutcome {
                pool,
                member,
                already_member: false,
            });
        }

        self.ensure_email_allowed(&pool, user_id).await?;

        let now = self.clock.now().as_str().to_owned();
        let member_id = new_id();
        self.members
            .create(NewMemberRecord {
                member_id: member_id.clone(),
                pool_id: pool_id.clone(),
                user_id: user_id.to_owned(),
                role: PoolRole::Member.as_str().to_owned(),
                status: MemberStatus::Active.as_str().to_owned(),
                joined_at: now,
            })
            .await?;
        let member = self.reload_member(&member_id).await?;
        self.notifier.member_joined(&pool_id, user_id).await?;
        Ok(JoinOutcome {
            pool,
            member,
            already_member: false,
        })
    }

    pub async fn leave(&self, pool_id: &str, user_id: &str) -> Result<(), AppError> {
        let member = self
            .members
            .find_membership(pool_id, user_id)
            .await?
            .filter(|member| member.status == MemberStatus::Active)
            .ok_or_else(|| AppError::Forbidden("you are not a member of this pool".to_owned()))?;

        if member.role == PoolRole::Owner && self.members.count_active_owners(pool_id).await? <= 1 {
            return Err(last_owner());
        }

        let now = self.clock.now().as_str().to_owned();
        self.members
            .set_status(member.id.as_str(), MemberStatus::Inactive.as_str(), Some(&now))
            .await
    }

    pub async fn delete(
        &self,
        pool_id: &str,
        actor: PoolRole,
        confirm_name: &str,
    ) -> Result<(), AppError> {
        if !actor.is_owner() {
            return Err(AppError::Forbidden(
                "only the owner can delete a pool".to_owned(),
            ));
        }
        let pool = self.get(pool_id).await?;
        if pool.name.as_str().trim() != confirm_name.trim() {
            return Err(AppError::Validation(
                "o nome informado não corresponde ao nome do bolão".to_owned(),
            ));
        }
        self.pools.delete(pool_id).await
    }

    pub async fn list_allowed_emails(
        &self,
        pool_id: &str,
        actor: PoolRole,
    ) -> Result<Vec<PoolAllowedEmail>, AppError> {
        if !actor.can_manage_members() {
            return Err(AppError::Forbidden(
                "only an owner or admin can manage the allowlist".to_owned(),
            ));
        }
        self.allowlist.list_for_pool(pool_id).await
    }

    pub async fn add_allowed_email(
        &self,
        pool_id: &str,
        actor: PoolRole,
        added_by_user_id: &str,
        email: String,
    ) -> Result<PoolAllowedEmail, AppError> {
        if !actor.can_manage_members() {
            return Err(AppError::Forbidden(
                "only an owner or admin can manage the allowlist".to_owned(),
            ));
        }
        let email = Email::new(email)?;
        self.allowlist
            .add(NewAllowedEmailRecord {
                id: new_id(),
                pool_id: pool_id.to_owned(),
                email: email.as_str().to_owned(),
                added_by_user_id: added_by_user_id.to_owned(),
            })
            .await
    }

    pub async fn remove_allowed_email(
        &self,
        pool_id: &str,
        actor: PoolRole,
        id: &str,
    ) -> Result<(), AppError> {
        if !actor.can_manage_members() {
            return Err(AppError::Forbidden(
                "only an owner or admin can manage the allowlist".to_owned(),
            ));
        }
        let entry = self
            .allowlist
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound("allowed email was not found".to_owned()))?;
        if entry.pool_id.as_str() != pool_id {
            return Err(AppError::NotFound("allowed email was not found".to_owned()));
        }
        self.allowlist.remove(pool_id, id).await
    }

    async fn ensure_email_allowed(&self, pool: &Pool, user_id: &str) -> Result<(), AppError> {
        if !pool.join_requires_allowlist {
            return Ok(());
        }
        let user = self
            .users
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("user was not found".to_owned()))?;
        if self
            .allowlist
            .is_email_allowed(pool.id.as_str(), user.email.as_str())
            .await?
        {
            Ok(())
        } else {
            Err(AppError::forbidden_code(
                "email_not_allowed",
                "seu e-mail não está autorizado a entrar neste bolão",
            ))
        }
    }

    async fn reload_member(&self, member_id: &str) -> Result<PoolMember, AppError> {
        self.members
            .find_by_id(member_id)
            .await?
            .ok_or_else(|| AppError::NotFound("pool member was not found".to_owned()))
    }

    async fn generate_invite_code(&self) -> Result<String, AppError> {
        for _ in 0..INVITE_CODE_ATTEMPTS {
            let candidate = InviteCode::new(&Uuid::new_v4().simple().to_string()[..10])?
                .as_str()
                .to_owned();
            if self.pools.find_by_invite_code(&candidate).await?.is_none() {
                return Ok(candidate);
            }
        }
        Err(AppError::Internal(
            "could not generate a unique invite code".to_owned(),
        ))
    }
}

pub struct MembershipUseCases<M, C> {
    members: M,
    clock: C,
}

impl<M, C> MembershipUseCases<M, C>
where
    M: PoolMemberRepository,
    C: Clock,
{
    pub fn new(members: M, clock: C) -> Self {
        Self { members, clock }
    }

    pub async fn list(&self, pool_id: &str) -> Result<Vec<PoolMemberWithName>, AppError> {
        self.members.list_for_pool_with_names(pool_id).await
    }

    pub async fn remove(
        &self,
        pool_id: &str,
        member_id: &str,
        actor: PoolRole,
    ) -> Result<(), AppError> {
        if !actor.can_manage_members() {
            return Err(AppError::Forbidden(
                "only an owner or admin can remove members".to_owned(),
            ));
        }
        let member = self.member_in_pool(pool_id, member_id).await?;
        if member.role == PoolRole::Owner {
            return Err(AppError::conflict_code(
                "cannot_remove_owner",
                "the pool owner cannot be removed",
            ));
        }

        let now = self.clock.now().as_str().to_owned();
        self.members
            .set_status(member.id.as_str(), MemberStatus::Inactive.as_str(), Some(&now))
            .await
    }

    pub async fn change_role(
        &self,
        pool_id: &str,
        member_id: &str,
        actor: PoolRole,
        command: ChangeMemberRole,
    ) -> Result<PoolMember, AppError> {
        if !actor.is_owner() {
            return Err(AppError::Forbidden(
                "only the owner can change member roles".to_owned(),
            ));
        }
        let role = PoolRole::parse(&command.role)?;
        let member = self.member_in_pool(pool_id, member_id).await?;

        if member.role == PoolRole::Owner
            && role != PoolRole::Owner
            && self.members.count_active_owners(pool_id).await? <= 1
        {
            return Err(last_owner());
        }

        self.members.set_role(member.id.as_str(), role.as_str()).await?;
        self.member_in_pool(pool_id, member_id).await
    }

    async fn member_in_pool(
        &self,
        pool_id: &str,
        member_id: &str,
    ) -> Result<PoolMember, AppError> {
        let member = self
            .members
            .find_by_id(member_id)
            .await?
            .ok_or_else(|| AppError::NotFound("pool member was not found".to_owned()))?;
        if member.pool_id.as_str() != pool_id {
            return Err(AppError::NotFound("pool member was not found".to_owned()));
        }
        Ok(member)
    }
}

pub struct ScoringRuleUseCases<S> {
    scoring_rules: S,
}

impl<S> ScoringRuleUseCases<S>
where
    S: ScoringRuleRepository,
{
    pub fn new(scoring_rules: S) -> Self {
        Self { scoring_rules }
    }

    pub async fn read(&self, pool_id: &str) -> Result<Vec<PoolScoringRule>, AppError> {
        self.scoring_rules.list_for_pool(pool_id).await
    }

    pub async fn update(
        &self,
        pool_id: &str,
        actor: PoolRole,
        command: UpdateScoringRules,
    ) -> Result<Vec<PoolScoringRule>, AppError> {
        if !actor.is_owner() {
            return Err(AppError::Forbidden(
                "only the owner can update scoring rules".to_owned(),
            ));
        }

        let mut seen: Vec<&'static str> = Vec::new();
        let mut records = Vec::with_capacity(command.rules.len());
        for ScoringRuleInput { rule_key, points } in command.rules {
            let key = ScoringRuleKey::parse(&rule_key)?;
            if seen.contains(&key.as_str()) {
                return Err(AppError::conflict_code(
                    "duplicate_scoring_rule",
                    "scoring rule keys must be unique per pool",
                ));
            }
            seen.push(key.as_str());
            if !(0..=MAX_RULE_POINTS).contains(&points) {
                return Err(AppError::Validation(
                    "scoring rule points must be between 0 and 1000".to_owned(),
                ));
            }
            records.push(NewScoringRuleRecord {
                rule_id: new_id(),
                rule_key: key.as_str().to_owned(),
                points,
            });
        }

        self.scoring_rules.set_rules(pool_id, records).await?;
        self.read(pool_id).await
    }
}

fn validate_name(value: String) -> Result<String, AppError> {
    let name = NonEmptyString::new(value, "pool.name")?.as_str().to_owned();
    if name.chars().count() > 140 {
        return Err(AppError::Validation(
            "pool name must be at most 140 characters".to_owned(),
        ));
    }
    Ok(name)
}

fn validate_lock_offset(value: u16) -> Result<u16, AppError> {
    if value > MAX_LOCK_OFFSET_MINUTES {
        return Err(AppError::Validation(
            "prediction lock offset must be between 0 and 10080 minutes".to_owned(),
        ));
    }
    Ok(value)
}

fn last_owner() -> AppError {
    AppError::conflict_code("last_owner", "the pool must keep at least one owner")
}

fn new_id() -> String {
    Uuid::new_v4().to_string()
}
