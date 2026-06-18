#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DomainId(String);

impl DomainId {
    pub fn new(value: impl Into<String>) -> Result<Self, DomainValidationError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(DomainValidationError::Empty("id"));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonEmptyString(String);

impl NonEmptyString {
    pub fn new(
        value: impl Into<String>,
        field: &'static str,
    ) -> Result<Self, DomainValidationError> {
        let value = value.into().trim().to_owned();
        if value.is_empty() {
            return Err(DomainValidationError::Empty(field));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Email(String);

impl Email {
    pub fn new(value: impl Into<String>) -> Result<Self, DomainValidationError> {
        let value = value.into().trim().to_ascii_lowercase();
        if !value.contains('@') || value.starts_with('@') || value.ends_with('@') {
            return Err(DomainValidationError::Invalid("email"));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UtcDateTime(String);

impl UtcDateTime {
    pub fn new_iso8601(value: impl Into<String>) -> Result<Self, DomainValidationError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(DomainValidationError::Empty("utc_datetime"));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamCode(String);

impl TeamCode {
    pub fn new(value: impl Into<String>) -> Result<Self, DomainValidationError> {
        let value = value.into().trim().to_ascii_lowercase();
        let valid_length = (2..=8).contains(&value.chars().count());
        let valid_chars = value.chars().all(|c| c.is_ascii_alphanumeric());
        if value.is_empty() || !valid_length || !valid_chars {
            return Err(DomainValidationError::Invalid("team_code"));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slug(String);

impl Slug {
    pub fn new(value: impl Into<String>) -> Result<Self, DomainValidationError> {
        let value = value.into().trim().to_owned();
        if value.is_empty() {
            return Err(DomainValidationError::Empty("slug"));
        }
        let segments: Vec<&str> = value.split('-').collect();
        let valid = segments.iter().all(|segment| {
            !segment.is_empty()
                && segment
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        });
        if !valid {
            return Err(DomainValidationError::Invalid("slug"));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CalendarDate(String);

impl CalendarDate {
    pub fn new(value: impl Into<String>) -> Result<Self, DomainValidationError> {
        let value = value.into().trim().to_owned();
        let segments: Vec<&str> = value.split('-').collect();
        if segments.len() != 3 {
            return Err(DomainValidationError::Invalid("calendar_date"));
        }
        let lengths_ok = segments[0].len() == 4 && segments[1].len() == 2 && segments[2].len() == 2;
        let digits_ok = segments
            .iter()
            .all(|segment| segment.chars().all(|c| c.is_ascii_digit()));
        if !lengths_ok || !digits_ok {
            return Err(DomainValidationError::Invalid("calendar_date"));
        }
        let month: u8 = segments[1].parse().unwrap_or(0);
        let day: u8 = segments[2].parse().unwrap_or(0);
        if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
            return Err(DomainValidationError::Invalid("calendar_date"));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Score(u8);

impl Score {
    pub fn new(value: u8) -> Result<Self, DomainValidationError> {
        if value > 99 {
            return Err(DomainValidationError::Invalid("score"));
        }
        Ok(Self(value))
    }

    pub fn value(&self) -> u8 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InviteCode(String);

impl InviteCode {
    pub fn new(value: impl Into<String>) -> Result<Self, DomainValidationError> {
        let value = value.into().trim().to_ascii_uppercase();
        let valid_length = (6..=20).contains(&value.chars().count());
        let valid_chars = value.chars().all(|c| c.is_ascii_alphanumeric());
        if value.is_empty() || !valid_length || !valid_chars {
            return Err(DomainValidationError::Invalid("invite_code"));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainValidationError {
    Empty(&'static str),
    Invalid(&'static str),
}
