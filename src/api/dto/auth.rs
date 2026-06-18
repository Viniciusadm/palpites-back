use serde::{Deserialize, Serialize};

use crate::application::auth::{LoginCommand, RegisterCommand};

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

impl From<LoginRequest> for LoginCommand {
    fn from(value: LoginRequest) -> Self {
        Self {
            email: value.email,
            password: value.password,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub display_name: String,
    pub email: String,
    pub password: String,
}

impl From<RegisterRequest> for RegisterCommand {
    fn from(value: RegisterRequest) -> Self {
        Self {
            display_name: value.display_name,
            email: value.email,
            password: value.password,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub token_type: String,
    pub user_id: String,
    pub display_name: String,
}

#[derive(Debug, Serialize)]
pub struct MeResponse {
    pub user: MeUser,
}

#[derive(Debug, Serialize)]
pub struct MeUser {
    pub id: String,
    pub display_name: String,
    pub email: String,
}
