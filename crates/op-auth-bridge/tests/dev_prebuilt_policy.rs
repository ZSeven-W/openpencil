#[path = "../dev_prebuilt_policy.rs"]
mod dev_prebuilt_policy;

use dev_prebuilt_policy::{
    parse_development_abi_version, require_debug_profile, require_hosted_opt_in,
    DevelopmentPrebuiltPolicyError,
};

#[test]
fn accepts_supported_development_abis() {
    assert_eq!(parse_development_abi_version("2"), Ok(2));
    assert_eq!(parse_development_abi_version("3"), Ok(3));
    assert_eq!(parse_development_abi_version("4"), Ok(4));
}

#[test]
fn rejects_legacy_future_and_malformed_abis() {
    for value in ["", "1", "5", "03", "not-a-number"] {
        assert_eq!(
            parse_development_abi_version(value),
            Err(DevelopmentPrebuiltPolicyError::UnsupportedAbi),
            "{value:?} must fail closed"
        );
    }
}

#[test]
fn hosted_development_abi_requires_its_own_explicit_feature() {
    assert_eq!(require_hosted_opt_in(2, false), Ok(()));
    assert_eq!(require_hosted_opt_in(3, false), Ok(()));
    assert_eq!(require_hosted_opt_in(4, true), Ok(()));
    assert_eq!(
        require_hosted_opt_in(4, false),
        Err(DevelopmentPrebuiltPolicyError::HostedOptInRequired),
    );
}

#[test]
fn unsigned_archives_are_debug_profile_only() {
    assert_eq!(require_debug_profile("debug"), Ok(()));
    for profile in ["release", "private-ci", "dist", ""] {
        assert_eq!(
            require_debug_profile(profile),
            Err(DevelopmentPrebuiltPolicyError::NonDebugProfile),
            "{profile:?} must fail closed"
        );
    }
}
