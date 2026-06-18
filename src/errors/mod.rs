use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppError {
    Configuration(String),
    Conflict(String),
    Forbidden(String),
    Internal(String),
    NotFound(String),
    Persistence(String),
    Unauthorized(String),
    Validation(String),
    /// A client-facing error carrying a stable machine `code` the frontend maps
    /// to user copy. `message` is the technical/fallback text (logs + clients
    /// that don't know the code).
    Coded {
        kind: CodedKind,
        code: &'static str,
        message: String,
    },
}

/// The HTTP semantics of a [`AppError::Coded`] error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodedKind {
    Conflict,
    Forbidden,
    NotFound,
    Validation,
}

impl AppError {
    pub fn conflict_code(code: &'static str, message: impl Into<String>) -> Self {
        Self::Coded {
            kind: CodedKind::Conflict,
            code,
            message: message.into(),
        }
    }

    pub fn forbidden_code(code: &'static str, message: impl Into<String>) -> Self {
        Self::Coded {
            kind: CodedKind::Forbidden,
            code,
            message: message.into(),
        }
    }

    pub fn not_found_code(code: &'static str, message: impl Into<String>) -> Self {
        Self::Coded {
            kind: CodedKind::NotFound,
            code,
            message: message.into(),
        }
    }

    pub fn validation_code(code: &'static str, message: impl Into<String>) -> Self {
        Self::Coded {
            kind: CodedKind::Validation,
            code,
            message: message.into(),
        }
    }

    /// The user-facing message without the technical type prefix, suitable for
    /// the HTTP response body. `Display` keeps the prefixed form for logs.
    pub fn client_message(&self) -> &str {
        match self {
            Self::Configuration(message)
            | Self::Conflict(message)
            | Self::Forbidden(message)
            | Self::Internal(message)
            | Self::NotFound(message)
            | Self::Persistence(message)
            | Self::Unauthorized(message)
            | Self::Validation(message)
            | Self::Coded { message, .. } => message,
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Configuration(message) => write!(f, "configuration error: {message}"),
            Self::Conflict(message) => write!(f, "conflict: {message}"),
            Self::Forbidden(message) => write!(f, "forbidden: {message}"),
            Self::Internal(message) => write!(f, "internal error: {message}"),
            Self::NotFound(message) => write!(f, "not found: {message}"),
            Self::Persistence(message) => write!(f, "persistence error: {message}"),
            Self::Unauthorized(message) => write!(f, "unauthorized: {message}"),
            Self::Validation(message) => write!(f, "validation error: {message}"),
            Self::Coded { code, message, .. } => write!(f, "{code}: {message}"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<sqlx::Error> for AppError {
    fn from(value: sqlx::Error) -> Self {
        Self::Persistence(value.to_string())
    }
}

impl From<crate::domain::DomainValidationError> for AppError {
    fn from(value: crate::domain::DomainValidationError) -> Self {
        Self::Validation(format!("{value:?}"))
    }
}
