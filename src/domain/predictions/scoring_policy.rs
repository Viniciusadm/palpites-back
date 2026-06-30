use crate::domain::pools::ScoringRuleKey;
use crate::domain::{PenaltySide, Score};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitKind {
    Exact,
    Outcome,
    None,
}

impl HitKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Outcome => "outcome",
            Self::None => "none",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ScoringRules {
    exact_score: Option<i16>,
    correct_outcome: Option<i16>,
    correct_goal_difference: Option<i16>,
    penalties_winner: Option<i16>,
}

impl ScoringRules {
    pub fn from_rules(rules: impl IntoIterator<Item = (ScoringRuleKey, i16)>) -> Self {
        let mut set = Self::default();
        for (key, points) in rules {
            match key {
                ScoringRuleKey::ExactScore => set.exact_score = Some(points),
                ScoringRuleKey::CorrectOutcome => set.correct_outcome = Some(points),
                ScoringRuleKey::CorrectGoalDifference => {
                    set.correct_goal_difference = Some(points)
                }
                ScoringRuleKey::PenaltiesWinner => set.penalties_winner = Some(points),
            }
        }
        set
    }
}

fn outcome(home: u8, away: u8) -> i8 {
    (home as i8 - away as i8).signum()
}

/// Pure scoring of a prediction against an official result.
///
/// Returns the awarded points and the [`HitKind`]. `correct_goal_difference` is
/// treated as a same-bucket alternative to `correct_outcome` (both are
/// [`HitKind::Outcome`]); when the outcome is right the higher of the applicable
/// rules is awarded. An absent rule contributes no points.
pub fn score(
    prediction: (Score, Score),
    result: (Score, Score),
    rules: &ScoringRules,
) -> (i16, HitKind) {
    let (ph, pa) = (prediction.0.value(), prediction.1.value());
    let (rh, ra) = (result.0.value(), result.1.value());

    if ph == rh && pa == ra {
        return (rules.exact_score.unwrap_or(0), HitKind::Exact);
    }

    if outcome(ph, pa) == outcome(rh, ra) {
        let mut points = rules.correct_outcome.unwrap_or(0);
        if (ph as i16 - pa as i16) == (rh as i16 - ra as i16) {
            if let Some(gd) = rules.correct_goal_difference {
                points = points.max(gd);
            }
        }
        return (points, HitKind::Outcome);
    }

    (0, HitKind::None)
}

/// Bonus points for correctly predicting the penalty-shootout winner.
///
/// This is an additive category, scored independently of [`score`] (the
/// 90-minute result). Points are only awarded when the member predicted a draw,
/// the match actually went to penalties (`result_winner` is `Some`), and the
/// member picked the side that won. Getting the draw right but the shootout
/// winner wrong awards nothing. Returns `(points, hit)` where `hit` flags a
/// correct penalty pick regardless of the rule's point value.
pub fn penalty_bonus(
    prediction_is_draw: bool,
    prediction_pick: Option<PenaltySide>,
    result_winner: Option<PenaltySide>,
    rules: &ScoringRules,
) -> (i16, bool) {
    match (prediction_is_draw, prediction_pick, result_winner) {
        (true, Some(pick), Some(winner)) if pick == winner => {
            (rules.penalties_winner.unwrap_or(0), true)
        }
        _ => (0, false),
    }
}
