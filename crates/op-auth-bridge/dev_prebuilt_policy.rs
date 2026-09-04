//! Policy shared by the auth build script and its regression tests.

use std::fmt;

pub const MIN_DEVELOPMENT_ABI_VERSION: u32 = 2;
pub const MAX_DEVELOPMENT_ABI_VERSION: u32 = 3;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DevelopmentPrebuiltPolicyError {
    UnsupportedAbi,
    NonDebugProfile,
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
        }
    }
}

pub fn parse_development_abi_version(value: &str) -> Result<u32, DevelopmentPrebuiltPolicyError> {
    match value {
        "2" => Ok(2),
        "3" => Ok(3),
        _ => Err(DevelopmentPrebuiltPolicyError::UnsupportedAbi),
    }
}

pub fn require_debug_profile(profile: &str) -> Result<(), DevelopmentPrebuiltPolicyError> {
    if profile == "debug" {
        Ok(())
    } else {
        Err(DevelopmentPrebuiltPolicyError::NonDebugProfile)
    }
}
