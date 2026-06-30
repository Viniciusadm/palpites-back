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

/// Which penalty-shootout category a prediction landed (mutually exclusive with
/// each other and with no hit). `WithDraw` is the v1 category (predicted the
/// draw and the shootout winner); `NoDraw` is the v2 category (predicted a
/// decisive result but correctly called who won the shootout).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PenaltyHit {
    None,
    WithDraw,
    NoDraw,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ScoringRules {
    exact_score: Option<i16>,
    correct_outcome: Option<i16>,
    correct_goal_difference: Option<i16>,
    penalties_winner: Option<i16>,
    penalties_winner_no_draw: Option<i16>,
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
                ScoringRuleKey::PenaltiesWinnerNoDraw => {
                    set.penalties_winner_no_draw = Some(points)
                }
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

/// Bonus points for correctly calling who won a penalty shootout.
///
/// Additive category, scored independently of [`score`] (the 90-minute result).
/// Only relevant when the match actually went to penalties (`result_winner` is
/// `Some`). Two mutually exclusive sub-categories:
/// - **WithDraw** (`penalties_winner`): the member predicted a draw and their
///   explicit `prediction_pick` matched the shootout winner.
/// - **NoDraw** (`penalties_winner_no_draw`): the member predicted a decisive
///   result and the side their score points to won the shootout.
///
/// Returns `(points, hit)`; `hit` flags which category landed regardless of the
/// rule's point value.
pub fn penalty_bonus(
    prediction_home: u8,
    prediction_away: u8,
    prediction_pick: Option<PenaltySide>,
    result_winner: Option<PenaltySide>,
    rules: &ScoringRules,
) -> (i16, PenaltyHit) {
    let Some(winner) = result_winner else {
        return (0, PenaltyHit::None);
    };

    if prediction_home == prediction_away {
        // Predicted a draw → explicit pick (v1 category).
        match prediction_pick {
            Some(pick) if pick == winner => {
                (rules.penalties_winner.unwrap_or(0), PenaltyHit::WithDraw)
            }
            _ => (0, PenaltyHit::None),
        }
    } else {
        // Predicted a decisive result → the winning side is implied by the score.
        let implied = if prediction_home > prediction_away {
            PenaltySide::Home
        } else {
            PenaltySide::Away
        };
        if implied == winner {
            (rules.penalties_winner_no_draw.unwrap_or(0), PenaltyHit::NoDraw)
        } else {
            (0, PenaltyHit::None)
        }
    }
}

pub fn point_reasons(
    prediction: (Score, Score),
    result: (Score, Score),
    prediction_pick: Option<PenaltySide>,
    result_winner: Option<PenaltySide>,
    rules: &ScoringRules,
) -> Vec<&'static str> {
    let (ph, pa) = (prediction.0.value(), prediction.1.value());
    let (rh, ra) = (result.0.value(), result.1.value());

    let mut reasons = Vec::new();

    let (_, hit) = score(prediction, result, rules);
    match hit {
        HitKind::Exact => reasons.push("exact"),
        HitKind::Outcome => {
            let difference_matches = (ph as i16 - pa as i16) == (rh as i16 - ra as i16);
            if difference_matches && rules.correct_goal_difference.is_some() {
                reasons.push("goal_difference");
            } else {
                reasons.push("outcome");
            }
        }
        HitKind::None => {}
    }

    let (_, penalty_hit) = penalty_bonus(ph, pa, prediction_pick, result_winner, rules);
    match penalty_hit {
        PenaltyHit::WithDraw => reasons.push("penalties_winner"),
        PenaltyHit::NoDraw => reasons.push("penalties_winner_no_draw"),
        PenaltyHit::None => {}
    }

    reasons
}
