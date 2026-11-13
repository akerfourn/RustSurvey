use rustsurvey_domain::participation::{InvitationLimits, ParticipationPolicy};

fn invitation(persistence: bool, allow_edit_after_submit: bool) -> ParticipationPolicy {
    ParticipationPolicy::Invitation {
        persistence,
        allow_edit_after_submit,
    }
}

#[test]
fn test_policy_should_require_a_response_link_when_editing_is_allowed() {
    assert_eq!(
        invitation(false, true).validate(),
        Err("Modifier une réponse exige de la relier à son invitation.")
    );
    for policy in [
        ParticipationPolicy::Open,
        ParticipationPolicy::BrowserOnce,
        invitation(false, false),
        invitation(true, false),
        invitation(true, true),
    ] {
        assert_eq!(policy.validate(), Ok(()));
    }
    assert!(invitation(true, true).allows_edit());
    assert!(!invitation(true, false).allows_edit());
}

#[test]
fn test_invitation_should_check_validity_boundaries_and_revocation() {
    let limits = InvitationLimits {
        max_uses: Some(1),
        valid_from: Some(10),
        valid_until: Some(20),
    };
    assert!(!limits.is_active(9, false));
    assert!(limits.is_active(10, false));
    assert!(limits.is_active(19, false));
    assert!(!limits.is_active(20, false));
    assert!(!limits.is_active(15, true));
    assert!(limits.has_uses_left(0));
    assert!(!limits.has_uses_left(1));
    assert!(!limits.has_uses_left(-1));
}

#[test]
fn test_limits_should_allow_bounded_or_unlimited_independent_responses() {
    let mut limits = InvitationLimits {
        max_uses: None,
        valid_from: None,
        valid_until: None,
    };
    assert_eq!(limits.validate(&invitation(false, false)), Ok(()));
    assert!(limits.has_uses_left(100_000));
    assert!(limits.validate(&invitation(true, false)).is_err());
    limits.max_uses = Some(1);
    assert_eq!(limits.validate(&invitation(true, true)), Ok(()));
    limits.valid_from = Some(20);
    limits.valid_until = Some(20);
    assert_eq!(
        limits.validate(&invitation(true, true)),
        Err("La fin de validité doit être postérieure au début.")
    );
}
