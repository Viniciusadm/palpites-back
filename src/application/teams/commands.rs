#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateTeam {
    pub name: String,
    pub code: String,
    pub flag_emoji: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateTeam {
    pub name: String,
    pub code: String,
    pub flag_emoji: Option<String>,
}
