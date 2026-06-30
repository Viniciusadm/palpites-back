use palpites_back::domain::pools::ScoringRuleKey;
use palpites_back::domain::predictions::{penalty_bonus, score, HitKind, ScoringRules};
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

#[test]
fn penalty_bonus_awards_points_when_draw_and_pick_is_right() {
    let (points, hit) = penalty_bonus(
        true,
        Some(PenaltySide::Home),
        Some(PenaltySide::Home),
        &penalty_rules(),
    );
    assert_eq!(points, 5);
    assert!(hit);
}

#[test]
fn penalty_bonus_awards_zero_when_pick_is_wrong() {
    let (points, hit) = penalty_bonus(
        true,
        Some(PenaltySide::Home),
        Some(PenaltySide::Away),
        &penalty_rules(),
    );
    assert_eq!(points, 0);
    assert!(!hit);
}

#[test]
fn penalty_bonus_awards_zero_when_prediction_is_not_a_draw() {
    let (points, hit) = penalty_bonus(
        false,
        Some(PenaltySide::Home),
        Some(PenaltySide::Home),
        &penalty_rules(),
    );
    assert_eq!(points, 0);
    assert!(!hit);
}

#[test]
fn penalty_bonus_awards_zero_when_match_did_not_go_to_penalties() {
    let (points, hit) = penalty_bonus(true, Some(PenaltySide::Home), None, &penalty_rules());
    assert_eq!(points, 0);
    assert!(!hit);
}

#[test]
fn penalty_bonus_awards_zero_when_member_did_not_pick() {
    let (points, hit) = penalty_bonus(true, None, Some(PenaltySide::Home), &penalty_rules());
    assert_eq!(points, 0);
    assert!(!hit);
}

#[test]
fn penalty_bonus_without_rule_recognizes_hit_with_zero_points() {
    let (points, hit) = penalty_bonus(
        true,
        Some(PenaltySide::Away),
        Some(PenaltySide::Away),
        &default_rules(),
    );
    assert_eq!(points, 0);
    assert!(hit);
}
