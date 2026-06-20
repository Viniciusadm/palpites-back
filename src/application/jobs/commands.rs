#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReminderWindow {
    pub window_minutes: u32,
    /// Período mínimo entre lembretes para o mesmo usuário. Garante no máximo
    /// uma notificação de lembrete por usuário dentro dessa janela, mesmo que
    /// existam vários jogos próximos.
    pub throttle_minutes: u32,
}

impl ReminderWindow {
    pub fn new(window_minutes: u32, throttle_minutes: u32) -> Self {
        Self {
            window_minutes,
            throttle_minutes,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DispatchSummary {
    pub delivered: u32,
}
