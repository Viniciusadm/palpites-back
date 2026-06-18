use palpites_back::domain::{DomainValidationError, InviteCode, Score, Slug, TeamCode};

#[test]
fn score_accepts_zero_through_ninety_nine() {
    assert_eq!(Score::new(0).unwrap().value(), 0);
    assert_eq!(Score::new(99).unwrap().value(), 99);
}

#[test]
fn score_rejects_values_above_ninety_nine() {
    assert_eq!(Score::new(100), Err(DomainValidationError::Invalid("score")));
    assert_eq!(Score::new(255), Err(DomainValidationError::Invalid("score")));
}

#[test]
fn slug_accepts_kebab_case() {
    assert_eq!(Slug::new("world-cup-2026").unwrap().as_str(), "world-cup-2026");
    assert_eq!(Slug::new("teams").unwrap().as_str(), "teams");
}

#[test]
fn slug_rejects_invalid_shapes() {
    for invalid in ["World-Cup", "has space", "-leading", "trailing-", "double--hyphen", ""] {
        assert!(
            Slug::new(invalid).is_err(),
            "expected slug {invalid:?} to be rejected"
        );
    }
}

#[test]
fn team_code_lowercases_and_keeps_length_bounds() {
    assert_eq!(TeamCode::new("BRA").unwrap().as_str(), "bra");
    assert_eq!(TeamCode::new("  Ar  ").unwrap().as_str(), "ar");
}

#[test]
fn team_code_rejects_out_of_bounds_or_non_alphanumeric() {
    assert!(TeamCode::new("b").is_err());
    assert!(TeamCode::new("toolongcode").is_err());
    assert!(TeamCode::new("br-").is_err());
}

#[test]
fn invite_code_uppercases_and_validates_shape() {
    assert_eq!(InviteCode::new("abc123").unwrap().as_str(), "ABC123");
    assert_eq!(InviteCode::new("WORLDCUP2026").unwrap().as_str(), "WORLDCUP2026");
}

#[test]
fn invite_code_rejects_out_of_bounds_or_non_alphanumeric() {
    assert!(InviteCode::new("abc12").is_err());
    assert!(InviteCode::new("a".repeat(21)).is_err());
    assert!(InviteCode::new("abc-123").is_err());
}
