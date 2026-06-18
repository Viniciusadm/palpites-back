use crate::api::extractors::AuthenticatedUser;
use crate::application::auth::UserRepository;
use crate::errors::AppError;

pub async fn require_admin<R>(auth: &AuthenticatedUser, users: &R) -> Result<(), AppError>
where
    R: UserRepository,
{
    let user = users
        .find_by_id(&auth.user_id)
        .await?
        .ok_or_else(|| AppError::Unauthorized("authenticated user no longer exists".to_owned()))?;

    if !user.role.is_admin() {
        return Err(AppError::Forbidden(
            "admin privileges are required".to_owned(),
        ));
    }
    Ok(())
}
