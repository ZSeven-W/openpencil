//! Policy shared by the auth build script and its regression tests.

use std::fmt;

pub const MIN_DEVELOPMENT_ABI_VERSION: u32 = 2;
pub const MAX_DEVELOPMENT_ABI_VERSION: u32 = 4;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DevelopmentPrebuiltPolicyError {
    UnsupportedAbi,
    NonDebugProfile,
    HostedOptInRequired,
}

impl fmt::Display for DevelopmentPrebuiltPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedAbi => write!(
                formatter,
                "ABI version must be between {MIN_DEVELOPMENT_ABI_VERSION} and \
                 {MAX_DEVELOPMENT_ABI_VERSION}"
            ),
            Self::NonDebugProfile => formatter
                .write_str("unsigned auth archives are accepted only in Cargo's debug profile"),
            Self::HostedOptInRequired => {
                formatter.write_str("ABI-v4 requires the op-auth-bridge/dev-hosted feature")
            }
        }
    }
}

pub fn parse_development_abi_version(value: &str) -> Result<u32, DevelopmentPrebuiltPolicyError> {
    match value {
        "2" => Ok(2),
        "3" => Ok(3),
        "4" => Ok(4),
        _ => Err(DevelopmentPrebuiltPolicyError::UnsupportedAbi),
    }
}

pub fn require_hosted_opt_in(
    abi_version: u32,
    hosted_enabled: bool,
) -> Result<(), DevelopmentPrebuiltPolicyError> {
    if abi_version == 4 && !hosted_enabled {
        Err(DevelopmentPrebuiltPolicyError::HostedOptInRequired)
    } else {
        Ok(())
    }
}

pub fn require_debug_profile(profile: &str) -> Result<(), DevelopmentPrebuiltPolicyError> {
    if profile == "debug" {
        Ok(())
    } else {
        Err(DevelopmentPrebuiltPolicyError::NonDebugProfile)
    }
}
