use axum::body::{to_bytes, Body};
use axum::extract::{Request, State};
use axum::http::{header, Method};
use axum::middleware::Next;
use axum::response::Response;
use tracing::error;

use crate::api::state::AppState;
use crate::application::auth::TokenVerifier;
use crate::infrastructure::auth::JwtTokenService;
use crate::infrastructure::repositories::mysql_audit_logs::{
    MySqlAuditLogRepository, NewAuditLog,
};

/// Limite defensivo para bufferizar o corpo de requisições admin (JSON pequeno).
const MAX_BODY_BYTES: usize = 1024 * 1024;

/// Middleware que registra em `admin_audit_logs` toda mutação bem-sucedida
/// (POST/PUT/PATCH/DELETE com resposta 2xx) feita pelas rotas administrativas.
///
/// Aplicado apenas ao grupo de rotas admin, então qualquer não-admin recebe 403
/// (status não-2xx) e não é registrado.
pub async fn audit_admin_changes(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let method = request.method().clone();
    let is_mutation = matches!(
        method,
        Method::POST | Method::PUT | Method::PATCH | Method::DELETE
    );

    // Leituras (GET, HEAD, etc.) seguem direto sem custo extra.
    if !is_mutation {
        return next.run(request).await;
    }

    let path = request.uri().path().to_owned();
    let query_string = request.uri().query().map(ToOwned::to_owned);
    let user_id = extract_user_id(&state, &request);
    let is_json = request
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.starts_with("application/json"))
        .unwrap_or(false);

    // Bufferiza o corpo para auditoria e reconstrói o request para o handler.
    let (request, request_body) = if is_json {
        let (parts, body) = request.into_parts();
        match to_bytes(body, MAX_BODY_BYTES).await {
            Ok(bytes) => {
                let body = sanitize_json_body(&bytes);
                (Request::from_parts(parts, Body::from(bytes)), body)
            }
            Err(_) => (Request::from_parts(parts, Body::empty()), None),
        }
    } else {
        (request, None)
    };

    let response = next.run(request).await;
    let status_code = response.status();

    if status_code.is_success() {
        let entry = NewAuditLog {
            user_id,
            method: method.to_string(),
            path,
            query_string,
            request_body,
            status_code: status_code.as_u16(),
        };

        match state.db() {
            Ok(db) => {
                if let Err(error) = MySqlAuditLogRepository::new(db).insert(entry).await {
                    error!("failed to write admin audit log: {error}");
                }
            }
            Err(error) => error!("cannot write admin audit log: {error}"),
        }
    }

    response
}

/// Reaproveita a mesma lógica do extractor `AuthenticatedUser` para descobrir
/// quem fez a mudança. Retorna `None` se o token estiver ausente/ inválido.
fn extract_user_id(state: &AppState, request: &Request) -> Option<String> {
    let token = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .filter(|value| !value.trim().is_empty())?;

    JwtTokenService::new(state.jwt_secret.clone())
        .verify_access_token(token)
        .ok()
}

/// Valida o corpo como JSON e o devolve em forma compacta. Corpo vazio ou
/// inválido vira `None`.
fn sanitize_json_body(bytes: &[u8]) -> Option<String> {
    if bytes.is_empty() {
        return None;
    }
    serde_json::from_slice::<serde_json::Value>(bytes)
        .ok()
        .and_then(|value| serde_json::to_string(&value).ok())
}
