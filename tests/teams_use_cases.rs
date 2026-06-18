use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::Utc;
use palpites_back::application::teams::{
    CreateTeam, CreateTeamRecord, TeamRepository, TeamUseCases, UpdateTeam, UpdateTeamRecord,
};
use palpites_back::domain::teams::Team;
use palpites_back::domain::{DomainId, NonEmptyString, TeamCode, UtcDateTime};
use palpites_back::errors::AppError;

#[tokio::test]
async fn create_persists_team_with_generated_id_and_lowercased_code() {
    let teams = FakeTeams::default();
    let use_cases = TeamUseCases::new(teams.clone());

    let team = use_cases
        .create(CreateTeam {
            name: "Brazil".to_owned(),
            code: "BRA".to_owned(),
            flag_emoji: Some("🇧🇷".to_owned()),
        })
        .await
        .unwrap();

    assert_eq!(team.name.as_str(), "Brazil");
    assert_eq!(team.code.as_str(), "bra");
    assert!(!team.id.as_str().is_empty());

    let stored = teams.teams.lock().unwrap();
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].code.as_str(), "bra");
}

#[tokio::test]
async fn create_rejects_duplicate_code() {
    let teams = FakeTeams::seeded(vec![team("team-1", "bra")]);
    let use_cases = TeamUseCases::new(teams);

    let result = use_cases
        .create(CreateTeam {
            name: "Brasil".to_owned(),
            code: "BRA".to_owned(),
            flag_emoji: None,
        })
        .await;

    assert!(matches!(
        result,
        Err(AppError::Coded {
            code: "team_code_taken",
            ..
        })
    ));
}

#[tokio::test]
async fn update_rejects_code_owned_by_another_team() {
    let teams = FakeTeams::seeded(vec![team("team-1", "bra"), team("team-2", "arg")]);
    let use_cases = TeamUseCases::new(teams);

    let result = use_cases
        .update(
            "team-2",
            UpdateTeam {
                name: "Argentina".to_owned(),
                code: "bra".to_owned(),
                flag_emoji: None,
            },
        )
        .await;

    assert!(matches!(
        result,
        Err(AppError::Coded {
            code: "team_code_taken",
            ..
        })
    ));
}

#[tokio::test]
async fn update_keeps_own_code() {
    let teams = FakeTeams::seeded(vec![team("team-1", "arg")]);
    let use_cases = TeamUseCases::new(teams);

    let team = use_cases
        .update(
            "team-1",
            UpdateTeam {
                name: "Argentina".to_owned(),
                code: "ARG".to_owned(),
                flag_emoji: None,
            },
        )
        .await
        .unwrap();

    assert_eq!(team.name.as_str(), "Argentina");
    assert_eq!(team.code.as_str(), "arg");
}

#[tokio::test]
async fn delete_blocked_when_referenced() {
    let teams = FakeTeams::seeded(vec![team("team-1", "bra")]).with_references();
    let use_cases = TeamUseCases::new(teams);

    let result = use_cases.delete("team-1").await;

    assert!(matches!(
        result,
        Err(AppError::Coded {
            code: "team_in_use",
            ..
        })
    ));
}

#[tokio::test]
async fn delete_removes_unreferenced_team() {
    let teams = FakeTeams::seeded(vec![team("team-1", "bra")]);
    let use_cases = TeamUseCases::new(teams.clone());

    use_cases.delete("team-1").await.unwrap();

    assert!(teams.teams.lock().unwrap().is_empty());
}

#[tokio::test]
async fn get_update_delete_missing_team_is_not_found() {
    let use_cases = TeamUseCases::new(FakeTeams::default());

    assert!(matches!(
        use_cases.get("missing").await,
        Err(AppError::NotFound(_))
    ));
    assert!(matches!(
        use_cases
            .update(
                "missing",
                UpdateTeam {
                    name: "Ghost".to_owned(),
                    code: "gho".to_owned(),
                    flag_emoji: None,
                },
            )
            .await,
        Err(AppError::NotFound(_))
    ));
    assert!(matches!(
        use_cases.delete("missing").await,
        Err(AppError::NotFound(_))
    ));
}

#[derive(Clone, Default)]
struct FakeTeams {
    teams: Arc<Mutex<Vec<Team>>>,
    referenced: bool,
}

impl FakeTeams {
    fn seeded(teams: Vec<Team>) -> Self {
        Self {
            teams: Arc::new(Mutex::new(teams)),
            referenced: false,
        }
    }

    fn with_references(mut self) -> Self {
        self.referenced = true;
        self
    }
}

#[async_trait]
impl TeamRepository for FakeTeams {
    async fn list(&self) -> Result<Vec<Team>, AppError> {
        Ok(self.teams.lock().unwrap().clone())
    }

    async fn find_by_id(&self, id: &str) -> Result<Option<Team>, AppError> {
        Ok(self
            .teams
            .lock()
            .unwrap()
            .iter()
            .find(|team| team.id.as_str() == id)
            .cloned())
    }

    async fn find_by_code(&self, code: &str) -> Result<Option<Team>, AppError> {
        Ok(self
            .teams
            .lock()
            .unwrap()
            .iter()
            .find(|team| team.code.as_str() == code)
            .cloned())
    }

    async fn create(&self, record: CreateTeamRecord) -> Result<(), AppError> {
        self.teams.lock().unwrap().push(Team {
            id: DomainId::new(record.team_id).unwrap(),
            name: NonEmptyString::new(record.name, "team.name").unwrap(),
            code: TeamCode::new(record.code).unwrap(),
            flag_emoji: record
                .flag_emoji
                .map(|value| NonEmptyString::new(value, "team.flag_emoji").unwrap()),
            flag_file_id: None,
            created_at: now(),
            updated_at: now(),
        });
        Ok(())
    }

    async fn update(&self, record: UpdateTeamRecord) -> Result<(), AppError> {
        let mut teams = self.teams.lock().unwrap();
        if let Some(team) = teams
            .iter_mut()
            .find(|team| team.id.as_str() == record.team_id)
        {
            team.name = NonEmptyString::new(record.name, "team.name").unwrap();
            team.code = TeamCode::new(record.code).unwrap();
            team.flag_emoji = record
                .flag_emoji
                .map(|value| NonEmptyString::new(value, "team.flag_emoji").unwrap());
        }
        Ok(())
    }

    async fn delete(&self, id: &str) -> Result<(), AppError> {
        self.teams.lock().unwrap().retain(|team| team.id.as_str() != id);
        Ok(())
    }

    async fn is_referenced(&self, _id: &str) -> Result<bool, AppError> {
        Ok(self.referenced)
    }
}

fn team(id: &str, code: &str) -> Team {
    Team {
        id: DomainId::new(id.to_owned()).unwrap(),
        name: NonEmptyString::new(format!("Team {code}"), "team.name").unwrap(),
        code: TeamCode::new(code.to_owned()).unwrap(),
        flag_emoji: None,
        flag_file_id: None,
        created_at: now(),
        updated_at: now(),
    }
}

fn now() -> UtcDateTime {
    UtcDateTime::new_iso8601(Utc::now().naive_utc().to_string()).unwrap()
}
