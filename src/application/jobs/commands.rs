#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReminderWindow {
    pub window_minutes: u32,
}

impl ReminderWindow {
    pub fn new(window_minutes: u32) -> Self {
        Self { window_minutes }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DispatchSummary {
    pub delivered: u32,
}
