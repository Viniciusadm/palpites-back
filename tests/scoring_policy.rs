use palpites_back::domain::pools::ScoringRuleKey;
use palpites_back::domain::predictions::{
    penalty_bonus, point_reasons, score, HitKind, PenaltyHit, ScoringRules,
};
use palpites_back::domain::{PenaltySide, Score};

fn s(value: u8) -> Score {
    Score::new(value).unwrap()
}

fn default_rules() -> ScoringRules {
    ScoringRules::from_rules([
        (ScoringRuleKey::ExactScore, 10),
        (ScoringRuleKey::CorrectOutcome, 5),
    ])
}

fn penalty_rules() -> ScoringRules {
    ScoringRules::from_rules([
        (ScoringRuleKey::ExactScore, 10),
        (ScoringRuleKey::CorrectOutcome, 5),
        (ScoringRuleKey::PenaltiesWinner, 5),
        (ScoringRuleKey::PenaltiesWinnerNoDraw, 2),
    ])
}

#[test]
fn exact_score_awards_exact_points() {
    let (points, kind) = score((s(2), s(1)), (s(2), s(1)), &default_rules());
    assert_eq!(points, 10);
    assert_eq!(kind, HitKind::Exact);
}

#[test]
fn correct_outcome_awards_outcome_points() {
    let (points, kind) = score((s(3), s(1)), (s(2), s(0)), &default_rules());
    assert_eq!(points, 5);
    assert_eq!(kind, HitKind::Outcome);
}

#[test]
fn wrong_prediction_awards_zero() {
    let (points, kind) = score((s(0), s(2)), (s(2), s(0)), &default_rules());
    assert_eq!(points, 0);
    assert_eq!(kind, HitKind::None);
}

#[test]
fn exact_draw_awards_exact_points() {
    let (points, kind) = score((s(1), s(1)), (s(1), s(1)), &default_rules());
    assert_eq!(points, 10);
    assert_eq!(kind, HitKind::Exact);
}

#[test]
fn correct_draw_outcome_awards_outcome_points() {
    let (points, kind) = score((s(0), s(0)), (s(2), s(2)), &default_rules());
    assert_eq!(points, 5);
    assert_eq!(kind, HitKind::Outcome);
}

#[test]
fn predicted_draw_but_result_decided_is_wrong() {
    let (points, kind) = score((s(1), s(1)), (s(2), s(0)), &default_rules());
    assert_eq!(points, 0);
    assert_eq!(kind, HitKind::None);
}

#[test]
fn goal_difference_rule_beats_plain_outcome_when_difference_matches() {
    let rules = ScoringRules::from_rules([
        (ScoringRuleKey::ExactScore, 10),
        (ScoringRuleKey::CorrectOutcome, 5),
        (ScoringRuleKey::CorrectGoalDifference, 7),
    ]);

    let (points, kind) = score((s(3), s(1)), (s(2), s(0)), &rules);
    assert_eq!(points, 7);
    assert_eq!(kind, HitKind::Outcome);
}

#[test]
fn goal_difference_rule_does_not_apply_when_difference_differs() {
    let rules = ScoringRules::from_rules([
        (ScoringRuleKey::CorrectOutcome, 5),
        (ScoringRuleKey::CorrectGoalDifference, 7),
    ]);

    let (points, kind) = score((s(3), s(0)), (s(2), s(0)), &rules);
    assert_eq!(points, 5);
    assert_eq!(kind, HitKind::Outcome);
}

#[test]
fn rule_set_without_exact_key_recognizes_hit_with_zero_points() {
    let rules = ScoringRules::from_rules([(ScoringRuleKey::CorrectOutcome, 5)]);

    let (points, kind) = score((s(2), s(1)), (s(2), s(1)), &rules);
    assert_eq!(points, 0);
    assert_eq!(kind, HitKind::Exact);
}

#[test]
fn empty_rule_set_awards_zero_for_correct_outcome() {
    let rules = ScoringRules::default();

    let (points, kind) = score((s(3), s(1)), (s(2), s(0)), &rules);
    assert_eq!(points, 0);
    assert_eq!(kind, HitKind::Outcome);
}

// --- penalty bonus: WITH-DRAW category (predicted the draw) -----------------

#[test]
fn penalty_with_draw_awards_points_when_pick_is_right() {
    // Predicted 1x1 (draw), picked home, home won the shootout.
    let (points, hit) = penalty_bonus(1, 1, Some(PenaltySide::Home), Some(PenaltySide::Home), &penalty_rules());
    assert_eq!(points, 5);
    assert_eq!(hit, PenaltyHit::WithDraw);
}

#[test]
fn penalty_with_draw_awards_zero_when_pick_is_wrong() {
    let (points, hit) = penalty_bonus(1, 1, Some(PenaltySide::Home), Some(PenaltySide::Away), &penalty_rules());
    assert_eq!(points, 0);
    assert_eq!(hit, PenaltyHit::None);
}

#[test]
fn penalty_with_draw_awards_zero_when_member_did_not_pick() {
    let (points, hit) = penalty_bonus(0, 0, None, Some(PenaltySide::Home), &penalty_rules());
    assert_eq!(points, 0);
    assert_eq!(hit, PenaltyHit::None);
}

// --- penalty bonus: NO-DRAW category (predicted a decisive result) ----------

#[test]
fn penalty_no_draw_awards_points_when_predicted_winner_advances() {
    // Predicted 1x0 (home win); match went to penalties and home won the shootout.
    let (points, hit) = penalty_bonus(1, 0, None, Some(PenaltySide::Home), &penalty_rules());
    assert_eq!(points, 2);
    assert_eq!(hit, PenaltyHit::NoDraw);
}

#[test]
fn penalty_no_draw_implied_side_follows_the_predicted_score() {
    // Predicted 0x2 (away win); away won the shootout → implied pick correct.
    let (points, hit) = penalty_bonus(0, 2, None, Some(PenaltySide::Away), &penalty_rules());
    assert_eq!(points, 2);
    assert_eq!(hit, PenaltyHit::NoDraw);
}

#[test]
fn penalty_no_draw_awards_zero_when_predicted_loser() {
    // Predicted 2x1 (home win) but away won the shootout.
    let (points, hit) = penalty_bonus(2, 1, None, Some(PenaltySide::Away), &penalty_rules());
    assert_eq!(points, 0);
    assert_eq!(hit, PenaltyHit::None);
}

// --- penalty bonus: shared guards -------------------------------------------

#[test]
fn penalty_bonus_awards_zero_when_match_did_not_go_to_penalties() {
    // Draw prediction with a pick, but no shootout happened.
    let (points, hit) = penalty_bonus(1, 1, Some(PenaltySide::Home), None, &penalty_rules());
    assert_eq!(points, 0);
    assert_eq!(hit, PenaltyHit::None);
    // Same for a decisive prediction.
    let (points, hit) = penalty_bonus(2, 0, None, None, &penalty_rules());
    assert_eq!(points, 0);
    assert_eq!(hit, PenaltyHit::None);
}

#[test]
fn penalty_bonus_without_rule_recognizes_hit_with_zero_points() {
    // default_rules has no penalty rules, but the hit category is still reported.
    let (points, hit) = penalty_bonus(1, 1, Some(PenaltySide::Away), Some(PenaltySide::Away), &default_rules());
    assert_eq!(points, 0);
    assert_eq!(hit, PenaltyHit::WithDraw);

    let (points, hit) = penalty_bonus(1, 0, None, Some(PenaltySide::Home), &default_rules());
    assert_eq!(points, 0);
    assert_eq!(hit, PenaltyHit::NoDraw);
}

// --- point reasons (labels do modal) ----------------------------------------

#[test]
fn reasons_exact_score() {
    let r = point_reasons((s(2), s(1)), (s(2), s(1)), None, None, &penalty_rules());
    assert_eq!(r, vec!["exact"]);
}

#[test]
fn reasons_outcome_only() {
    let r = point_reasons((s(3), s(1)), (s(2), s(0)), None, None, &penalty_rules());
    assert_eq!(r, vec!["outcome"]);
}

#[test]
fn reasons_goal_difference_when_rule_is_configured() {
    let rules = ScoringRules::from_rules([
        (ScoringRuleKey::CorrectOutcome, 5),
        (ScoringRuleKey::CorrectGoalDifference, 7),
    ]);
    let r = point_reasons((s(3), s(1)), (s(2), s(0)), None, None, &rules);
    assert_eq!(r, vec!["goal_difference"]);
}

#[test]
fn reasons_exact_plus_penalties_with_draw() {
    let r = point_reasons(
        (s(1), s(1)),
        (s(1), s(1)),
        Some(PenaltySide::Home),
        Some(PenaltySide::Home),
        &penalty_rules(),
    );
    assert_eq!(r, vec!["exact", "penalties_winner"]);
}

#[test]
fn reasons_penalties_no_draw_only() {
    let r = point_reasons((s(1), s(0)), (s(1), s(1)), None, Some(PenaltySide::Home), &penalty_rules());
    assert_eq!(r, vec!["penalties_winner_no_draw"]);
}

#[test]
fn reasons_empty_when_nothing_scored() {
    let r = point_reasons((s(0), s(2)), (s(2), s(0)), None, None, &penalty_rules());
    assert!(r.is_empty());
}
