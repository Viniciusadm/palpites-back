use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::Utc;
use palpites_back::application::matches::{
    CreateMatch, CreateMatchRecord, MatchFilters, MatchRepository, MatchUseCases, UpdateMatchRecord,
};
use palpites_back::domain::matches::{Match, MatchStatus};
use palpites_back::domain::{DomainId, UtcDateTime};
use palpites_back::errors::AppError;

mod support;

use support::NoopNotifier;

#[tokio::test]
async fn create_persists_match_with_default_scheduled_status() {
    let repo = FakeMatches::default()
        .with_tournament("t-1")
        .with_stage("s-1", "t-1")
        .with_team("t-1", "home")
        .with_team("t-1", "away");
    let use_cases = MatchUseCases::new(repo.clone(), NoopNotifier);

    let created = use_cases
        .create(
            "t-1",
            CreateMatch {
                stage_id: "s-1".to_owned(),
                home_team_id: Some("home".to_owned()),
                away_team_id: Some("away".to_owned()),
                kickoff_at: "2026-06-11T18:00:00".to_owned(),
            },
        )
        .await
        .unwrap();

    assert_eq!(created.status, MatchStatus::Scheduled);
    assert_eq!(created.stage_id.as_str(), "s-1");
    assert_eq!(created.kickoff_at.as_str(), "2026-06-11 18:00:00");
    assert!(!created.id.as_str().is_empty());
    assert_eq!(repo.matches.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn create_rejects_same_team() {
    let repo = FakeMatches::default()
        .with_tournament("t-1")
        .with_stage("s-1", "t-1")
        .with_team("t-1", "team-1");
    let use_cases = MatchUseCases::new(repo, NoopNotifier);

    let result = use_cases
        .create(
            "t-1",
            CreateMatch {
                stage_id: "s-1".to_owned(),
                home_team_id: Some("team-1".to_owned()),
                away_team_id: Some("team-1".to_owned()),
                kickoff_at: "2026-06-11T18:00:00".to_owned(),
            },
        )
        .await;

    assert!(matches!(
        result,
        Err(AppError::Coded {
            code: "match_same_team",
            ..
        })
    ));
}

#[tokio::test]
async fn create_rejects_team_not_in_tournament() {
    let repo = FakeMatches::default()
        .with_tournament("t-1")
        .with_stage("s-1", "t-1")
        .with_team("t-1", "home");
    let use_cases = MatchUseCases::new(repo, NoopNotifier);

    let result = use_cases
        .create(
            "t-1",
            CreateMatch {
                stage_id: "s-1".to_owned(),
                home_team_id: Some("home".to_owned()),
                away_team_id: Some("stranger".to_owned()),
                kickoff_at: "2026-06-11T18:00:00".to_owned(),
            },
        )
        .await;

    assert!(matches!(
        result,
        Err(AppError::Coded {
            code: "team_not_in_tournament",
            ..
        })
    ));
}

#[tokio::test]
async fn create_rejects_stage_from_other_tournament() {
    let repo = FakeMatches::default()
        .with_tournament("t-1")
        .with_tournament("t-2")
        .with_stage("s-2", "t-2");
    let use_cases = MatchUseCases::new(repo, NoopNotifier);

    let result = use_cases
        .create(
            "t-1",
            CreateMatch {
                stage_id: "s-2".to_owned(),
                home_team_id: None,
                away_team_id: None,
                kickoff_at: "2026-06-11T18:00:00".to_owned(),
            },
        )
        .await;

    assert!(matches!(
        result,
        Err(AppError::Coded {
            code: "stage_tournament_mismatch",
            ..
        })
    ));
}

#[tokio::test]
async fn create_rejects_unknown_stage() {
    let repo = FakeMatches::default().with_tournament("t-1");
    let use_cases = MatchUseCases::new(repo, NoopNotifier);

    let result = use_cases
        .create(
            "t-1",
            CreateMatch {
                stage_id: "ghost".to_owned(),
                home_team_id: None,
                away_team_id: None,
                kickoff_at: "2026-06-11T18:00:00".to_owned(),
            },
        )
        .await;

    assert!(matches!(result, Err(AppError::NotFound(_))));
}

#[tokio::test]
async fn create_rejects_unknown_tournament() {
    let use_cases = MatchUseCases::new(FakeMatches::default(), NoopNotifier);

    let result = use_cases
        .create(
            "ghost",
            CreateMatch {
                stage_id: "s-1".to_owned(),
                home_team_id: None,
                away_team_id: None,
                kickoff_at: "2026-06-11T18:00:00".to_owned(),
            },
        )
        .await;

    assert!(matches!(result, Err(AppError::NotFound(_))));
}

#[tokio::test]
async fn create_rejects_invalid_kickoff() {
    let repo = FakeMatches::default()
        .with_tournament("t-1")
        .with_stage("s-1", "t-1");
    let use_cases = MatchUseCases::new(repo, NoopNotifier);

    let result = use_cases
        .create(
            "t-1",
            CreateMatch {
                stage_id: "s-1".to_owned(),
                home_team_id: None,
                away_team_id: None,
                kickoff_at: "not-a-date".to_owned(),
            },
        )
        .await;

    assert!(matches!(result, Err(AppError::Validation(_))));
}

#[tokio::test]
async fn list_applies_status_filter() {
    let repo = FakeMatches::default().with_tournament("t-1");
    repo.matches
        .lock()
        .unwrap()
        .push(match_row("m-1", "t-1", "s-1", MatchStatus::Scheduled));
    repo.matches
        .lock()
        .unwrap()
        .push(match_row("m-2", "t-1", "s-1", MatchStatus::Live));
    let use_cases = MatchUseCases::new(repo, NoopNotifier);

    let result = use_cases
        .list(
            "t-1",
            MatchFilters {
                status: Some("live".to_owned()),
                ..MatchFilters::default()
            },
        )
        .await
        .unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].id.as_str(), "m-2");
}

#[tokio::test]
async fn list_rejects_invalid_status_filter() {
    let repo = FakeMatches::default().with_tournament("t-1");
    let use_cases = MatchUseCases::new(repo, NoopNotifier);

    let result = use_cases
        .list(
            "t-1",
            MatchFilters {
                status: Some("bogus".to_owned()),
                ..MatchFilters::default()
            },
        )
        .await;

    assert!(matches!(result, Err(AppError::Validation(_))));
}

#[derive(Clone, Default)]
struct FakeMatches {
    matches: Arc<Mutex<Vec<Match>>>,
    tournaments: Arc<Mutex<Vec<String>>>,
    stages: Arc<Mutex<Vec<(String, String)>>>,
    participations: Arc<Mutex<Vec<(String, String)>>>,
}

impl FakeMatches {
    fn with_tournament(self, tournament_id: &str) -> Self {
        self.tournaments.lock().unwrap().push(tournament_id.to_owned());
        self
    }

    fn with_stage(self, stage_id: &str, tournament_id: &str) -> Self {
        self.stages
            .lock()
            .unwrap()
            .push((stage_id.to_owned(), tournament_id.to_owned()));
        self
    }

    fn with_team(self, tournament_id: &str, team_id: &str) -> Self {
        self.participations
            .lock()
            .unwrap()
            .push((tournament_id.to_owned(), team_id.to_owned()));
        self
    }
}

#[async_trait]
impl MatchRepository for FakeMatches {
    async fn list(
        &self,
        tournament_id: &str,
        filters: MatchFilters,
    ) -> Result<Vec<Match>, AppError> {
        Ok(self
            .matches
            .lock()
            .unwrap()
            .iter()
            .filter(|item| item.tournament_id.as_str() == tournament_id)
            .filter(|item| {
                filters
                    .stage_id
                    .as_deref()
                    .map_or(true, |stage_id| item.stage_id.as_str() == stage_id)
            })
            .filter(|item| {
                filters
                    .status
                    .as_deref()
                    .map_or(true, |status| item.status.as_str() == status)
            })
            .cloned()
            .collect())
    }

    async fn find_by_id(&self, id: &str) -> Result<Option<Match>, AppError> {
        Ok(self
            .matches
            .lock()
            .unwrap()
            .iter()
            .find(|item| item.id.as_str() == id)
            .cloned())
    }

    async fn create(&self, record: CreateMatchRecord) -> Result<(), AppError> {
        self.matches.lock().unwrap().push(Match {
            id: DomainId::new(record.match_id).unwrap(),
            tournament_id: DomainId::new(record.tournament_id).unwrap(),
            stage_id: DomainId::new(record.stage_id).unwrap(),
            home_team_id: record.home_team_id.map(|value| DomainId::new(value).unwrap()),
            away_team_id: record.away_team_id.map(|value| DomainId::new(value).unwrap()),
            kickoff_at: UtcDateTime::new_iso8601(record.kickoff_at).unwrap(),
            status: MatchStatus::parse(&record.status).unwrap(),
            home_score: None,
            away_score: None,
            finished_at: None,
            created_at: now(),
            updated_at: now(),
        });
        Ok(())
    }

    async fn update(&self, record: UpdateMatchRecord) -> Result<(), AppError> {
        let mut matches = self.matches.lock().unwrap();
        if let Some(item) = matches.iter_mut().find(|item| item.id.as_str() == record.match_id) {
            item.stage_id = DomainId::new(record.stage_id).unwrap();
            item.home_team_id = record.home_team_id.map(|value| DomainId::new(value).unwrap());
            item.away_team_id = record.away_team_id.map(|value| DomainId::new(value).unwrap());
            item.kickoff_at = UtcDateTime::new_iso8601(record.kickoff_at).unwrap();
            item.status = MatchStatus::parse(&record.status).unwrap();
        }
        Ok(())
    }

    async fn delete(&self, id: &str) -> Result<(), AppError> {
        self.matches.lock().unwrap().retain(|item| item.id.as_str() != id);
        Ok(())
    }

    async fn tournament_exists(&self, tournament_id: &str) -> Result<bool, AppError> {
        Ok(self
            .tournaments
            .lock()
            .unwrap()
            .iter()
            .any(|id| id == tournament_id))
    }

    async fn find_stage_tournament_id(
        &self,
        stage_id: &str,
    ) -> Result<Option<String>, AppError> {
        Ok(self
            .stages
            .lock()
            .unwrap()
            .iter()
            .find(|(id, _)| id == stage_id)
            .map(|(_, tournament_id)| tournament_id.clone()))
    }

    async fn is_team_in_tournament(
        &self,
        tournament_id: &str,
        team_id: &str,
    ) -> Result<bool, AppError> {
        Ok(self
            .participations
            .lock()
            .unwrap()
            .iter()
            .any(|(t, team)| t == tournament_id && team == team_id))
    }
}

fn match_row(id: &str, tournament_id: &str, stage_id: &str, status: MatchStatus) -> Match {
    Match {
        id: DomainId::new(id.to_owned()).unwrap(),
        tournament_id: DomainId::new(tournament_id.to_owned()).unwrap(),
        stage_id: DomainId::new(stage_id.to_owned()).unwrap(),
        home_team_id: None,
        away_team_id: None,
        kickoff_at: now(),
        status,
        home_score: None,
        away_score: None,
        finished_at: None,
        created_at: now(),
        updated_at: now(),
    }
}

fn now() -> UtcDateTime {
    UtcDateTime::new_iso8601(Utc::now().naive_utc().to_string()).unwrap()
}
