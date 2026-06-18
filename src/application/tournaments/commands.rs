use crate::domain::tournaments::{Tournament, TournamentGroup, TournamentStage};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateTournament {
    pub name: String,
    pub slug: String,
    pub season_year: Option<u16>,
    pub starts_on: Option<String>,
    pub ends_on: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateTournament {
    pub name: String,
    pub slug: String,
    pub season_year: Option<u16>,
    pub starts_on: Option<String>,
    pub ends_on: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateStage {
    pub name: String,
    pub kind: String,
    pub ordering: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateStage {
    pub name: String,
    pub kind: String,
    pub ordering: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateGroup {
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateGroup {
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssignTeam {
    pub team_id: String,
    pub group_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TournamentDetail {
    pub tournament: Tournament,
    pub stages: Vec<TournamentStage>,
    pub groups: Vec<TournamentGroup>,
}
