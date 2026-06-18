#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginCommand {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisterCommand {
    pub display_name: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedSession {
    pub access_token: String,
    pub user_id: String,
    pub display_name: String,
}
