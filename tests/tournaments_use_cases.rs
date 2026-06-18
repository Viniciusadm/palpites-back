use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::Utc;
use palpites_back::application::tournaments::{
    AssignTeam, CreateAssignmentRecord, CreateGroup, CreateGroupRecord, CreateStage,
    CreateStageRecord, CreateTournament, CreateTournamentRecord, TournamentRepository,
    TournamentUseCases, UpdateGroupRecord, UpdateStageRecord, UpdateTournament,
    UpdateTournamentRecord,
};
use palpites_back::domain::tournaments::{
    StageKind, Tournament, TournamentGroup, TournamentStage, TournamentStatus, TournamentTeam,
};
use palpites_back::domain::{CalendarDate, DomainId, NonEmptyString, Slug, UtcDateTime};
use palpites_back::errors::AppError;

#[tokio::test]
async fn create_persists_tournament_with_default_draft_status() {
    let repo = FakeTournaments::default();
    let use_cases = TournamentUseCases::new(repo.clone());

    let tournament = use_cases
        .create(CreateTournament {
            name: "World Cup".to_owned(),
            slug: "world-cup-2026".to_owned(),
            season_year: Some(2026),
            starts_on: Some("2026-06-11".to_owned()),
            ends_on: Some("2026-07-19".to_owned()),
        })
        .await
        .unwrap();

    assert_eq!(tournament.slug.as_str(), "world-cup-2026");
    assert_eq!(tournament.status, TournamentStatus::Draft);
    assert!(!tournament.id.as_str().is_empty());
    assert_eq!(repo.tournaments.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn create_rejects_invalid_date_range() {
    let use_cases = TournamentUseCases::new(FakeTournaments::default());

    let result = use_cases
        .create(CreateTournament {
            name: "World Cup".to_owned(),
            slug: "world-cup".to_owned(),
            season_year: None,
            starts_on: Some("2026-07-19".to_owned()),
            ends_on: Some("2026-06-11".to_owned()),
        })
        .await;

    assert!(matches!(result, Err(AppError::Validation(_))));
}

#[tokio::test]
async fn create_rejects_duplicate_slug() {
    let repo = FakeTournaments::seeded(vec![tournament("t-1", "world-cup", TournamentStatus::Draft)]);
    let use_cases = TournamentUseCases::new(repo);

    let result = use_cases
        .create(CreateTournament {
            name: "World Cup".to_owned(),
            slug: "world-cup".to_owned(),
            season_year: None,
            starts_on: None,
            ends_on: None,
        })
        .await;

    assert!(matches!(
        result,
        Err(AppError::Coded {
            code: "tournament_slug_taken",
            ..
        })
    ));
}

#[tokio::test]
async fn update_rejects_invalid_status_transition() {
    let repo = FakeTournaments::seeded(vec![tournament("t-1", "world-cup", TournamentStatus::Draft)]);
    let use_cases = TournamentUseCases::new(repo);

    let result = use_cases
        .update(
            "t-1",
            UpdateTournament {
                name: "World Cup".to_owned(),
                slug: "world-cup".to_owned(),
                season_year: None,
                starts_on: None,
                ends_on: None,
                status: "finished".to_owned(),
            },
        )
        .await;

    assert!(matches!(
        result,
        Err(AppError::Coded {
            code: "invalid_status_transition",
            ..
        })
    ));
}

#[tokio::test]
async fn update_allows_valid_forward_transition() {
    let repo = FakeTournaments::seeded(vec![tournament("t-1", "world-cup", TournamentStatus::Draft)]);
    let use_cases = TournamentUseCases::new(repo);

    let tournament = use_cases
        .update(
            "t-1",
            UpdateTournament {
                name: "World Cup".to_owned(),
                slug: "world-cup".to_owned(),
                season_year: None,
                starts_on: None,
                ends_on: None,
                status: "active".to_owned(),
            },
        )
        .await
        .unwrap();

    assert_eq!(tournament.status, TournamentStatus::Active);
}

#[tokio::test]
async fn add_stage_rejects_duplicate_name() {
    let repo = FakeTournaments::seeded(vec![tournament("t-1", "world-cup", TournamentStatus::Draft)]);
    repo.stages.lock().unwrap().push(stage("s-1", "t-1", "Group Stage"));
    let use_cases = TournamentUseCases::new(repo);

    let result = use_cases
        .add_stage(
            "t-1",
            CreateStage {
                name: "Group Stage".to_owned(),
                kind: "group".to_owned(),
                ordering: 1,
            },
        )
        .await;

    assert!(matches!(
        result,
        Err(AppError::Coded {
            code: "stage_name_taken",
            ..
        })
    ));
}

#[tokio::test]
async fn add_group_rejects_duplicate_label() {
    let repo = FakeTournaments::seeded(vec![tournament("t-1", "world-cup", TournamentStatus::Draft)]);
    repo.groups.lock().unwrap().push(group("g-1", "t-1", "A"));
    let use_cases = TournamentUseCases::new(repo);

    let result = use_cases
        .add_group(
            "t-1",
            CreateGroup {
                label: "A".to_owned(),
            },
        )
        .await;

    assert!(matches!(
        result,
        Err(AppError::Coded {
            code: "group_label_taken",
            ..
        })
    ));
}

#[tokio::test]
async fn assign_team_rejects_group_from_other_tournament() {
    let repo = FakeTournaments::seeded(vec![
        tournament("t-1", "world-cup", TournamentStatus::Draft),
        tournament("t-2", "euro", TournamentStatus::Draft),
    ]);
    repo.groups.lock().unwrap().push(group("g-2", "t-2", "A"));
    repo.teams.lock().unwrap().push("team-1".to_owned());
    let use_cases = TournamentUseCases::new(repo);

    let result = use_cases
        .assign_team(
            "t-1",
            AssignTeam {
                team_id: "team-1".to_owned(),
                group_id: Some("g-2".to_owned()),
            },
        )
        .await;

    assert!(matches!(
        result,
        Err(AppError::Coded {
            code: "group_tournament_mismatch",
            ..
        })
    ));
}

#[tokio::test]
async fn assign_team_rejects_unknown_team() {
    let repo = FakeTournaments::seeded(vec![tournament("t-1", "world-cup", TournamentStatus::Draft)]);
    let use_cases = TournamentUseCases::new(repo);

    let result = use_cases
        .assign_team(
            "t-1",
            AssignTeam {
                team_id: "ghost".to_owned(),
                group_id: None,
            },
        )
        .await;

    assert!(matches!(result, Err(AppError::NotFound(_))));
}

#[tokio::test]
async fn assign_team_persists_assignment() {
    let repo = FakeTournaments::seeded(vec![tournament("t-1", "world-cup", TournamentStatus::Draft)]);
    repo.groups.lock().unwrap().push(group("g-1", "t-1", "A"));
    repo.teams.lock().unwrap().push("team-1".to_owned());
    let use_cases = TournamentUseCases::new(repo.clone());

    let assignment = use_cases
        .assign_team(
            "t-1",
            AssignTeam {
                team_id: "team-1".to_owned(),
                group_id: Some("g-1".to_owned()),
            },
        )
        .await
        .unwrap();

    assert_eq!(assignment.team_id.as_str(), "team-1");
    assert_eq!(assignment.group_id.unwrap().as_str(), "g-1");
    assert_eq!(repo.assignments.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn delete_blocked_when_referenced_by_pools() {
    let repo = FakeTournaments::seeded(vec![tournament("t-1", "world-cup", TournamentStatus::Draft)])
        .with_pool_reference();
    let use_cases = TournamentUseCases::new(repo);

    let result = use_cases.delete("t-1").await;

    assert!(matches!(
        result,
        Err(AppError::Coded {
            code: "tournament_in_use",
            ..
        })
    ));
}

#[derive(Clone, Default)]
struct FakeTournaments {
    tournaments: Arc<Mutex<Vec<Tournament>>>,
    stages: Arc<Mutex<Vec<TournamentStage>>>,
    groups: Arc<Mutex<Vec<TournamentGroup>>>,
    assignments: Arc<Mutex<Vec<TournamentTeam>>>,
    teams: Arc<Mutex<Vec<String>>>,
    referenced_by_pools: bool,
}

impl FakeTournaments {
    fn seeded(tournaments: Vec<Tournament>) -> Self {
        Self {
            tournaments: Arc::new(Mutex::new(tournaments)),
            ..Self::default()
        }
    }

    fn with_pool_reference(mut self) -> Self {
        self.referenced_by_pools = true;
        self
    }
}

#[async_trait]
impl TournamentRepository for FakeTournaments {
    async fn list(&self) -> Result<Vec<Tournament>, AppError> {
        Ok(self.tournaments.lock().unwrap().clone())
    }

    async fn find_by_id(&self, id: &str) -> Result<Option<Tournament>, AppError> {
        Ok(self
            .tournaments
            .lock()
            .unwrap()
            .iter()
            .find(|item| item.id.as_str() == id)
            .cloned())
    }

    async fn find_by_slug(&self, slug: &str) -> Result<Option<Tournament>, AppError> {
        Ok(self
            .tournaments
            .lock()
            .unwrap()
            .iter()
            .find(|item| item.slug.as_str() == slug)
            .cloned())
    }

    async fn create(&self, record: CreateTournamentRecord) -> Result<(), AppError> {
        self.tournaments.lock().unwrap().push(Tournament {
            id: DomainId::new(record.tournament_id).unwrap(),
            name: NonEmptyString::new(record.name, "tournament.name").unwrap(),
            slug: Slug::new(record.slug).unwrap(),
            season_year: record.season_year,
            starts_on: record.starts_on.map(|value| CalendarDate::new(value).unwrap()),
            ends_on: record.ends_on.map(|value| CalendarDate::new(value).unwrap()),
            status: TournamentStatus::parse(&record.status).unwrap(),
            created_at: now(),
            updated_at: now(),
        });
        Ok(())
    }

    async fn update(&self, record: UpdateTournamentRecord) -> Result<(), AppError> {
        let mut tournaments = self.tournaments.lock().unwrap();
        if let Some(item) = tournaments
            .iter_mut()
            .find(|item| item.id.as_str() == record.tournament_id)
        {
            item.name = NonEmptyString::new(record.name, "tournament.name").unwrap();
            item.slug = Slug::new(record.slug).unwrap();
            item.season_year = record.season_year;
            item.starts_on = record.starts_on.map(|value| CalendarDate::new(value).unwrap());
            item.ends_on = record.ends_on.map(|value| CalendarDate::new(value).unwrap());
            item.status = TournamentStatus::parse(&record.status).unwrap();
        }
        Ok(())
    }

    async fn delete(&self, id: &str) -> Result<(), AppError> {
        self.tournaments
            .lock()
            .unwrap()
            .retain(|item| item.id.as_str() != id);
        Ok(())
    }

    async fn is_referenced_by_pools(&self, _id: &str) -> Result<bool, AppError> {
        Ok(self.referenced_by_pools)
    }

    async fn list_stages(&self, tournament_id: &str) -> Result<Vec<TournamentStage>, AppError> {
        Ok(self
            .stages
            .lock()
            .unwrap()
            .iter()
            .filter(|item| item.tournament_id.as_str() == tournament_id)
            .cloned()
            .collect())
    }

    async fn find_stage_by_id(
        &self,
        stage_id: &str,
    ) -> Result<Option<TournamentStage>, AppError> {
        Ok(self
            .stages
            .lock()
            .unwrap()
            .iter()
            .find(|item| item.id.as_str() == stage_id)
            .cloned())
    }

    async fn find_stage_by_name(
        &self,
        tournament_id: &str,
        name: &str,
    ) -> Result<Option<TournamentStage>, AppError> {
        Ok(self
            .stages
            .lock()
            .unwrap()
            .iter()
            .find(|item| item.tournament_id.as_str() == tournament_id && item.name.as_str() == name)
            .cloned())
    }

    async fn create_stage(&self, record: CreateStageRecord) -> Result<(), AppError> {
        self.stages.lock().unwrap().push(TournamentStage {
            id: DomainId::new(record.stage_id).unwrap(),
            tournament_id: DomainId::new(record.tournament_id).unwrap(),
            name: NonEmptyString::new(record.name, "stage.name").unwrap(),
            kind: StageKind::parse(&record.kind).unwrap(),
            ordering: record.ordering,
            created_at: now(),
            updated_at: now(),
        });
        Ok(())
    }

    async fn update_stage(&self, record: UpdateStageRecord) -> Result<(), AppError> {
        let mut stages = self.stages.lock().unwrap();
        if let Some(item) = stages.iter_mut().find(|item| item.id.as_str() == record.stage_id) {
            item.name = NonEmptyString::new(record.name, "stage.name").unwrap();
            item.kind = StageKind::parse(&record.kind).unwrap();
            item.ordering = record.ordering;
        }
        Ok(())
    }

    async fn delete_stage(&self, stage_id: &str) -> Result<(), AppError> {
        self.stages
            .lock()
            .unwrap()
            .retain(|item| item.id.as_str() != stage_id);
        Ok(())
    }

    async fn list_groups(&self, tournament_id: &str) -> Result<Vec<TournamentGroup>, AppError> {
        Ok(self
            .groups
            .lock()
            .unwrap()
            .iter()
            .filter(|item| item.tournament_id.as_str() == tournament_id)
            .cloned()
            .collect())
    }

    async fn find_group_by_id(
        &self,
        group_id: &str,
    ) -> Result<Option<TournamentGroup>, AppError> {
        Ok(self
            .groups
            .lock()
            .unwrap()
            .iter()
            .find(|item| item.id.as_str() == group_id)
            .cloned())
    }

    async fn find_group_by_label(
        &self,
        tournament_id: &str,
        label: &str,
    ) -> Result<Option<TournamentGroup>, AppError> {
        Ok(self
            .groups
            .lock()
            .unwrap()
            .iter()
            .find(|item| {
                item.tournament_id.as_str() == tournament_id && item.label.as_str() == label
            })
            .cloned())
    }

    async fn create_group(&self, record: CreateGroupRecord) -> Result<(), AppError> {
        self.groups.lock().unwrap().push(TournamentGroup {
            id: DomainId::new(record.group_id).unwrap(),
            tournament_id: DomainId::new(record.tournament_id).unwrap(),
            label: NonEmptyString::new(record.label, "group.label").unwrap(),
            created_at: now(),
            updated_at: now(),
        });
        Ok(())
    }

    async fn update_group(&self, record: UpdateGroupRecord) -> Result<(), AppError> {
        let mut groups = self.groups.lock().unwrap();
        if let Some(item) = groups.iter_mut().find(|item| item.id.as_str() == record.group_id) {
            item.label = NonEmptyString::new(record.label, "group.label").unwrap();
        }
        Ok(())
    }

    async fn delete_group(&self, group_id: &str) -> Result<(), AppError> {
        self.groups
            .lock()
            .unwrap()
            .retain(|item| item.id.as_str() != group_id);
        Ok(())
    }

    async fn list_teams(&self, tournament_id: &str) -> Result<Vec<TournamentTeam>, AppError> {
        Ok(self
            .assignments
            .lock()
            .unwrap()
            .iter()
            .filter(|item| item.tournament_id.as_str() == tournament_id)
            .cloned()
            .collect())
    }

    async fn find_assignment_by_id(
        &self,
        assignment_id: &str,
    ) -> Result<Option<TournamentTeam>, AppError> {
        Ok(self
            .assignments
            .lock()
            .unwrap()
            .iter()
            .find(|item| item.id.as_str() == assignment_id)
            .cloned())
    }

    async fn find_assignment(
        &self,
        tournament_id: &str,
        team_id: &str,
    ) -> Result<Option<TournamentTeam>, AppError> {
        Ok(self
            .assignments
            .lock()
            .unwrap()
            .iter()
            .find(|item| {
                item.tournament_id.as_str() == tournament_id && item.team_id.as_str() == team_id
            })
            .cloned())
    }

    async fn create_assignment(&self, record: CreateAssignmentRecord) -> Result<(), AppError> {
        self.assignments.lock().unwrap().push(TournamentTeam {
            id: DomainId::new(record.assignment_id).unwrap(),
            tournament_id: DomainId::new(record.tournament_id).unwrap(),
            team_id: DomainId::new(record.team_id).unwrap(),
            group_id: record.group_id.map(|value| DomainId::new(value).unwrap()),
            created_at: now(),
            updated_at: now(),
        });
        Ok(())
    }

    async fn delete_assignment(&self, assignment_id: &str) -> Result<(), AppError> {
        self.assignments
            .lock()
            .unwrap()
            .retain(|item| item.id.as_str() != assignment_id);
        Ok(())
    }

    async fn team_exists(&self, team_id: &str) -> Result<bool, AppError> {
        Ok(self
            .teams
            .lock()
            .unwrap()
            .iter()
            .any(|id| id == team_id))
    }
}

fn tournament(id: &str, slug: &str, status: TournamentStatus) -> Tournament {
    Tournament {
        id: DomainId::new(id.to_owned()).unwrap(),
        name: NonEmptyString::new(format!("Tournament {slug}"), "tournament.name").unwrap(),
        slug: Slug::new(slug.to_owned()).unwrap(),
        season_year: None,
        starts_on: None,
        ends_on: None,
        status,
        created_at: now(),
        updated_at: now(),
    }
}

fn stage(id: &str, tournament_id: &str, name: &str) -> TournamentStage {
    TournamentStage {
        id: DomainId::new(id.to_owned()).unwrap(),
        tournament_id: DomainId::new(tournament_id.to_owned()).unwrap(),
        name: NonEmptyString::new(name.to_owned(), "stage.name").unwrap(),
        kind: StageKind::Group,
        ordering: 0,
        created_at: now(),
        updated_at: now(),
    }
}

fn group(id: &str, tournament_id: &str, label: &str) -> TournamentGroup {
    TournamentGroup {
        id: DomainId::new(id.to_owned()).unwrap(),
        tournament_id: DomainId::new(tournament_id.to_owned()).unwrap(),
        label: NonEmptyString::new(label.to_owned(), "group.label").unwrap(),
        created_at: now(),
        updated_at: now(),
    }
}

fn now() -> UtcDateTime {
    UtcDateTime::new_iso8601(Utc::now().naive_utc().to_string()).unwrap()
}
