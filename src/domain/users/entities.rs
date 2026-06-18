use crate::domain::{DomainId, DomainValidationError, Email, NonEmptyString, UtcDateTime};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserRole {
    Member,
    Admin,
}

impl UserRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Member => "member",
            Self::Admin => "admin",
        }
    }

    pub fn parse(value: &str) -> Result<Self, DomainValidationError> {
        match value {
            "member" => Ok(Self::Member),
            "admin" => Ok(Self::Admin),
            _ => Err(DomainValidationError::Invalid("user_role")),
        }
    }

    pub fn is_admin(&self) -> bool {
        matches!(self, Self::Admin)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub id: DomainId,
    pub email: Email,
    pub password_hash: NonEmptyString,
    pub display_name: NonEmptyString,
    pub role: UserRole,
    pub avatar_file_id: Option<DomainId>,
    pub is_active: bool,
    pub last_login_at: Option<UtcDateTime>,
    pub created_at: UtcDateTime,
    pub updated_at: UtcDateTime,
}
