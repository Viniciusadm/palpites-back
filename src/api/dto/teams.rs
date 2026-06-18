use serde::{Deserialize, Serialize};

use crate::application::teams::{CreateTeam, UpdateTeam};
use crate::domain::teams::Team;

#[derive(Debug, Deserialize)]
pub struct CreateTeamRequest {
    pub name: String,
    pub code: String,
    pub flag_emoji: Option<String>,
}

impl From<CreateTeamRequest> for CreateTeam {
    fn from(value: CreateTeamRequest) -> Self {
        Self {
            name: value.name,
            code: value.code,
            flag_emoji: value.flag_emoji,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateTeamRequest {
    pub name: String,
    pub code: String,
    pub flag_emoji: Option<String>,
}

impl From<UpdateTeamRequest> for UpdateTeam {
    fn from(value: UpdateTeamRequest) -> Self {
        Self {
            name: value.name,
            code: value.code,
            flag_emoji: value.flag_emoji,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct TeamResponse {
    pub id: String,
    pub name: String,
    pub code: String,
    pub flag_emoji: Option<String>,
    pub flag_file_id: Option<String>,
}

impl TeamResponse {
    pub fn from_team(team: &Team) -> Self {
        Self {
            id: team.id.as_str().to_owned(),
            name: team.name.as_str().to_owned(),
            code: team.code.as_str().to_owned(),
            flag_emoji: team
                .flag_emoji
                .as_ref()
                .map(|value| value.as_str().to_owned()),
            flag_file_id: team
                .flag_file_id
                .as_ref()
                .map(|value| value.as_str().to_owned()),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct TeamsListResponse {
    pub teams: Vec<TeamResponse>,
}
