use serde::{Deserialize, Serialize};

use crate::application::tournaments::{
    AssignTeam, CreateGroup, CreateStage, CreateTournament, TournamentDetail, UpdateGroup,
    UpdateStage, UpdateTournament,
};
use crate::domain::tournaments::{Tournament, TournamentGroup, TournamentStage, TournamentTeam};

#[derive(Debug, Deserialize)]
pub struct CreateTournamentRequest {
    pub name: String,
    pub slug: String,
    pub season_year: Option<u16>,
    pub starts_on: Option<String>,
    pub ends_on: Option<String>,
}

impl From<CreateTournamentRequest> for CreateTournament {
    fn from(value: CreateTournamentRequest) -> Self {
        Self {
            name: value.name,
            slug: value.slug,
            season_year: value.season_year,
            starts_on: value.starts_on,
            ends_on: value.ends_on,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateTournamentRequest {
    pub name: String,
    pub slug: String,
    pub season_year: Option<u16>,
    pub starts_on: Option<String>,
    pub ends_on: Option<String>,
    pub status: String,
}

impl From<UpdateTournamentRequest> for UpdateTournament {
    fn from(value: UpdateTournamentRequest) -> Self {
        Self {
            name: value.name,
            slug: value.slug,
            season_year: value.season_year,
            starts_on: value.starts_on,
            ends_on: value.ends_on,
            status: value.status,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateStageRequest {
    pub name: String,
    pub kind: String,
    pub ordering: u16,
}

impl From<CreateStageRequest> for CreateStage {
    fn from(value: CreateStageRequest) -> Self {
        Self {
            name: value.name,
            kind: value.kind,
            ordering: value.ordering,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateStageRequest {
    pub name: String,
    pub kind: String,
    pub ordering: u16,
}

impl From<UpdateStageRequest> for UpdateStage {
    fn from(value: UpdateStageRequest) -> Self {
        Self {
            name: value.name,
            kind: value.kind,
            ordering: value.ordering,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateGroupRequest {
    pub label: String,
}

impl From<CreateGroupRequest> for CreateGroup {
    fn from(value: CreateGroupRequest) -> Self {
        Self { label: value.label }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateGroupRequest {
    pub label: String,
}

impl From<UpdateGroupRequest> for UpdateGroup {
    fn from(value: UpdateGroupRequest) -> Self {
        Self { label: value.label }
    }
}

#[derive(Debug, Deserialize)]
pub struct AssignTeamRequest {
    pub team_id: String,
    pub group_id: Option<String>,
}

impl From<AssignTeamRequest> for AssignTeam {
    fn from(value: AssignTeamRequest) -> Self {
        Self {
            team_id: value.team_id,
            group_id: value.group_id,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct TournamentResponse {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub season_year: Option<u16>,
    pub starts_on: Option<String>,
    pub ends_on: Option<String>,
    pub status: String,
}

impl TournamentResponse {
    pub fn from_tournament(tournament: &Tournament) -> Self {
        Self {
            id: tournament.id.as_str().to_owned(),
            name: tournament.name.as_str().to_owned(),
            slug: tournament.slug.as_str().to_owned(),
            season_year: tournament.season_year,
            starts_on: tournament
                .starts_on
                .as_ref()
                .map(|value| value.as_str().to_owned()),
            ends_on: tournament
                .ends_on
                .as_ref()
                .map(|value| value.as_str().to_owned()),
            status: tournament.status.as_str().to_owned(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct TournamentsListResponse {
    pub tournaments: Vec<TournamentResponse>,
}

#[derive(Debug, Serialize)]
pub struct StageResponse {
    pub id: String,
    pub tournament_id: String,
    pub name: String,
    pub kind: String,
    pub ordering: u16,
}

impl StageResponse {
    pub fn from_stage(stage: &TournamentStage) -> Self {
        Self {
            id: stage.id.as_str().to_owned(),
            tournament_id: stage.tournament_id.as_str().to_owned(),
            name: stage.name.as_str().to_owned(),
            kind: stage.kind.as_str().to_owned(),
            ordering: stage.ordering,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct GroupResponse {
    pub id: String,
    pub tournament_id: String,
    pub label: String,
}

impl GroupResponse {
    pub fn from_group(group: &TournamentGroup) -> Self {
        Self {
            id: group.id.as_str().to_owned(),
            tournament_id: group.tournament_id.as_str().to_owned(),
            label: group.label.as_str().to_owned(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct TournamentDetailResponse {
    #[serde(flatten)]
    pub tournament: TournamentResponse,
    pub stages: Vec<StageResponse>,
    pub groups: Vec<GroupResponse>,
}

impl TournamentDetailResponse {
    pub fn from_detail(detail: &TournamentDetail) -> Self {
        Self {
            tournament: TournamentResponse::from_tournament(&detail.tournament),
            stages: detail.stages.iter().map(StageResponse::from_stage).collect(),
            groups: detail.groups.iter().map(GroupResponse::from_group).collect(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct TournamentTeamResponse {
    pub id: String,
    pub tournament_id: String,
    pub team_id: String,
    pub group_id: Option<String>,
}

impl TournamentTeamResponse {
    pub fn from_assignment(assignment: &TournamentTeam) -> Self {
        Self {
            id: assignment.id.as_str().to_owned(),
            tournament_id: assignment.tournament_id.as_str().to_owned(),
            team_id: assignment.team_id.as_str().to_owned(),
            group_id: assignment
                .group_id
                .as_ref()
                .map(|value| value.as_str().to_owned()),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct TournamentTeamsListResponse {
    pub teams: Vec<TournamentTeamResponse>,
}
