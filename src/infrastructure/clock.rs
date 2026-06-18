use chrono::Utc;

use crate::application::shared::Clock;
use crate::domain::UtcDateTime;

#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> UtcDateTime {
        UtcDateTime::new_iso8601(Utc::now().naive_utc().to_string())
            .expect("system clock produced an invalid timestamp")
    }
}
