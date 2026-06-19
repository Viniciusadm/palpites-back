use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::application::jobs::{NotificationSender, OutboundNotification};
use crate::application::notifications::DeviceTokenRepository;
use crate::domain::notifications::Channel;
use crate::errors::AppError;

const SCOPE: &str = "https://www.googleapis.com/auth/firebase.messaging";

/// Resolved FCM credentials, derived from the application configuration.
#[derive(Debug, Clone)]
pub struct FcmSettings {
    pub project_id: String,
    pub service_account_path: String,
}

#[derive(Debug, Deserialize)]
struct ServiceAccount {
    client_email: String,
    private_key: String,
    token_uri: String,
}

#[derive(Debug, Serialize)]
struct JwtClaims<'a> {
    iss: &'a str,
    scope: &'a str,
    aud: &'a str,
    iat: u64,
    exp: u64,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
}

struct CachedToken {
    token: String,
    expires_at: u64,
}

/// Delivers push notifications through the Firebase Cloud Messaging HTTP v1 API,
/// resolving each recipient's registered device tokens at send time.
pub struct FcmPushSender<D> {
    project_id: String,
    service_account: ServiceAccount,
    http: reqwest::Client,
    devices: D,
    cached: Mutex<Option<CachedToken>>,
}

impl<D> FcmPushSender<D>
where
    D: DeviceTokenRepository,
{
    pub fn new(settings: FcmSettings, devices: D) -> Result<Self, AppError> {
        let raw = std::fs::read_to_string(&settings.service_account_path).map_err(|err| {
            AppError::Internal(format!(
                "failed to read FCM service account at {}: {err}",
                settings.service_account_path
            ))
        })?;
        let service_account: ServiceAccount = serde_json::from_str(&raw).map_err(|err| {
            AppError::Internal(format!("invalid FCM service account json: {err}"))
        })?;
        Ok(Self {
            project_id: settings.project_id,
            service_account,
            http: reqwest::Client::new(),
            devices,
            cached: Mutex::new(None),
        })
    }

    fn endpoint(&self) -> String {
        format!(
            "https://fcm.googleapis.com/v1/projects/{}/messages:send",
            self.project_id
        )
    }

    async fn access_token(&self) -> Result<String, AppError> {
        let now = unix_now();
        {
            let guard = self.cached.lock().await;
            if let Some(cached) = guard.as_ref() {
                if cached.expires_at > now + 60 {
                    return Ok(cached.token.clone());
                }
            }
        }

        let assertion = self.build_assertion(now)?;
        let params = [
            ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
            ("assertion", assertion.as_str()),
        ];
        let response = self
            .http
            .post(&self.service_account.token_uri)
            .form(&params)
            .send()
            .await
            .map_err(|err| AppError::Internal(format!("fcm token request failed: {err}")))?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!(
                "fcm token request failed: {status} {body}"
            )));
        }
        let token: TokenResponse = response
            .json()
            .await
            .map_err(|err| AppError::Internal(format!("invalid fcm token response: {err}")))?;

        let mut guard = self.cached.lock().await;
        *guard = Some(CachedToken {
            token: token.access_token.clone(),
            expires_at: now + token.expires_in,
        });
        Ok(token.access_token)
    }

    fn build_assertion(&self, now: u64) -> Result<String, AppError> {
        let claims = JwtClaims {
            iss: &self.service_account.client_email,
            scope: SCOPE,
            aud: &self.service_account.token_uri,
            iat: now,
            exp: now + 3600,
        };
        let key = EncodingKey::from_rsa_pem(self.service_account.private_key.as_bytes())
            .map_err(|err| AppError::Internal(format!("invalid FCM private key: {err}")))?;
        encode(&Header::new(Algorithm::RS256), &claims, &key)
            .map_err(|err| AppError::Internal(format!("failed to sign FCM jwt: {err}")))
    }
}

#[async_trait]
impl<D> NotificationSender for FcmPushSender<D>
where
    D: DeviceTokenRepository,
{
    fn channel(&self) -> Channel {
        Channel::Push
    }

    async fn send(&self, notification: &OutboundNotification) -> Result<(), AppError> {
        let tokens = self.devices.list_for_user(&notification.user_id).await?;
        if tokens.is_empty() {
            return Ok(());
        }
        let access_token = self.access_token().await?;
        let endpoint = self.endpoint();
        for token in tokens {
            let payload = serde_json::json!({
                "message": {
                    "token": token,
                    "notification": {
                        "title": notification.title,
                        "body": notification.body,
                    }
                }
            });
            let response = self
                .http
                .post(&endpoint)
                .bearer_auth(&access_token)
                .json(&payload)
                .send()
                .await
                .map_err(|err| AppError::Internal(format!("fcm send failed: {err}")))?;
            if response.status().is_success() {
                continue;
            }
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            // 404 UNREGISTERED / 400 INVALID_ARGUMENT mean the token is dead; prune it.
            if status.as_u16() == 404 || status.as_u16() == 400 {
                let _ = self.devices.remove(&token).await;
                tracing::warn!(status = status.as_u16(), "pruned invalid FCM device token");
                continue;
            }
            return Err(AppError::Internal(format!(
                "fcm send failed: {status} {body}"
            )));
        }
        Ok(())
    }
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0)
}
