use crate::application::auth::{
    AuthenticatedSession, LoginCommand, PasswordHasher, RegisterCommand, RegisterUserRecord,
    TokenIssuer, UserRepository,
};
use crate::domain::{Email, NonEmptyString};
use crate::errors::AppError;
use uuid::Uuid;

pub struct AuthUseCases<R, H, T> {
    users: R,
    password_hasher: H,
    token_issuer: T,
}

impl<R, H, T> AuthUseCases<R, H, T>
where
    R: UserRepository,
    H: PasswordHasher,
    T: TokenIssuer,
{
    pub fn new(users: R, password_hasher: H, token_issuer: T) -> Self {
        Self {
            users,
            password_hasher,
            token_issuer,
        }
    }

    pub async fn login(&self, command: LoginCommand) -> Result<AuthenticatedSession, AppError> {
        let user = self
            .users
            .find_by_email(&command.email)
            .await?
            .ok_or_else(|| AppError::Unauthorized("invalid credentials".to_owned()))?;

        if !user.is_active {
            return Err(AppError::Unauthorized("inactive user".to_owned()));
        }

        let valid_password = self
            .password_hasher
            .verify_password(&command.password, user.password_hash.as_str())?;
        if !valid_password {
            return Err(AppError::Unauthorized("invalid credentials".to_owned()));
        }

        Ok(AuthenticatedSession {
            access_token: self.token_issuer.issue_access_token(user.id.as_str())?,
            user_id: user.id.as_str().to_owned(),
            display_name: user.display_name.as_str().to_owned(),
        })
    }

    pub async fn register(
        &self,
        command: RegisterCommand,
    ) -> Result<AuthenticatedSession, AppError> {
        let display_name = NonEmptyString::new(command.display_name, "display_name")?
            .as_str()
            .to_owned();
        let email = Email::new(command.email)?.as_str().to_owned();
        if command.password.len() < 8 || command.password.len() > 128 {
            return Err(AppError::Validation(
                "password must be between 8 and 128 characters".to_owned(),
            ));
        }
        let has_letter = command.password.chars().any(|c| c.is_alphabetic());
        let has_digit = command.password.chars().any(|c| c.is_ascii_digit());
        if !has_letter || !has_digit {
            return Err(AppError::Validation(
                "password must contain at least one letter and one number".to_owned(),
            ));
        }
        if self.users.find_by_email(&email).await?.is_some() {
            return Err(AppError::Conflict("email is already registered".to_owned()));
        }

        let user_id = new_id();
        self.users
            .register_user(RegisterUserRecord {
                user_id: user_id.clone(),
                email,
                password_hash: self.password_hasher.hash_password(&command.password)?,
                display_name: display_name.clone(),
            })
            .await?;

        Ok(AuthenticatedSession {
            access_token: self.token_issuer.issue_access_token(&user_id)?,
            user_id,
            display_name,
        })
    }
}

fn new_id() -> String {
    Uuid::new_v4().to_string()
}
