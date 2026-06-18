#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreferenceInput {
    pub notification_type: String,
    pub channel: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdatePreferences {
    pub items: Vec<PreferenceInput>,
}
