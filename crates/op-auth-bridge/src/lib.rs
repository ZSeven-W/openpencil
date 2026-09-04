//! Authentication bridge plus the open collaboration-ticket verifier.
//!
//! The real device-login implementation lives in the private
//! `ZSeven-W/op-platform` repository and ships as a prebuilt C-ABI static
//! library committed under `prebuilt/<target>/libop_auth.a`. When no
//! artifact exists for the current target — or its ABI version is unsupported
//! — this crate falls back to a stub whose [`available`] returns false, and
//! hosts keep the account UI hidden.
//!
//! Poll model: [`login_begin`] returns a handle the host polls each frame
//! via [`poll`]; handle [`SESSION_HANDLE`] reports the restored/signed-in
//! session. All strings cross the FFI as UTF-8 pointer + length and are
//! freed by the bridge immediately after copying.
//!
//! Collaboration ticket parsing, Ed25519 verification, strict claim
//! validation, and bounded JWKS caching are entirely open source. The private
//! library may only issue opaque tickets from its authenticated device session.

#[cfg(all(op_auth_development_prebuilt, not(debug_assertions)))]
compile_error!("the local op-auth archive override requires target debug assertions");

mod collab_claims;
mod collab_jwks;
mod collab_jwks_cache;
mod collab_relay_token;
mod collab_relay_token_verifier;
#[cfg(any(test, feature = "test-issuer"))]
mod collab_test_issuer;
mod collab_ticket;
mod collab_ticket_error;
mod collab_union_policy;
mod collab_verifier;
mod collab_verifier_error;
mod status;

#[cfg(op_auth_prebuilt)]
#[path = "real.rs"]
mod backend;

#[cfg(all(op_auth_prebuilt, op_auth_collab_ticket_prebuilt))]
mod real_collab_ticket;

#[cfg(not(op_auth_prebuilt))]
#[path = "stub.rs"]
mod backend;

use std::path::PathBuf;
use std::sync::Mutex;

pub use collab_claims::{
    CollabVerifierConfig, VerifiedCollabClaims, COLLAB_JWKS_PATH, COLLAB_JWS_ALGORITHM,
    COLLAB_JWS_TYPE, COLLAB_POLICY_PATH, COLLAB_TICKET_AUDIENCE, COLLAB_TICKET_CLOCK_SKEW_SECONDS,
    COLLAB_TICKET_MAX_LIFETIME_SECONDS, COLLAB_TICKET_SCOPE, COLLAB_TICKET_VERSION,
    MAX_COLLAB_PROFILE_AVATAR_URL_BYTES, MAX_COLLAB_PROFILE_DISPLAY_NAME_BYTES,
    MAX_COLLAB_PROFILE_DISPLAY_NAME_CHARS, PRODUCTION_COLLAB_ISSUER,
    PRODUCTION_COLLAB_JWKS_ENDPOINT, PRODUCTION_COLLAB_POLICY_ENDPOINT,
};
pub use collab_jwks::{
    CollabJwks, DEFAULT_MAX_COLLAB_JWKS_BYTES, DEFAULT_MAX_COLLAB_JWKS_KEYS,
    HARD_MAX_COLLAB_JWKS_BYTES, HARD_MAX_COLLAB_JWKS_KEYS,
};
pub use collab_jwks_cache::{
    CollabJwksCache, CollabJwksCacheLimits, CollabJwksFetchRequest, CollabJwksFetchResponse,
    CollabJwksFetcher, DEFAULT_FAILED_REFRESH_BACKOFF_SECONDS, DEFAULT_MAX_COLLAB_JWKS_AGE_SECONDS,
    DEFAULT_MAX_COLLAB_JWKS_ETAG_BYTES, DEFAULT_UNKNOWN_KID_REFRESH_SECONDS,
};
pub use collab_relay_token::{
    VerifiedRelayTokenClaims, MAX_COLLAB_RELAY_TOKEN_BYTES, RELAY_TOKEN_AUDIENCE,
    RELAY_TOKEN_JWS_TYPE, RELAY_TOKEN_SCOPE, RELAY_TOKEN_VERSION,
};
pub use collab_relay_token_verifier::{
    RelayBearerKind, RelayBearerVerifyError, VerifiedRelayBearer,
};
#[cfg(any(test, feature = "test-issuer"))]
pub use collab_test_issuer::{
    StaticTestJwksFetcher, TestCollabIssuer, TestCollabIssuerError, TestCollabSigningKey,
    TestCollabTicketSpec, TestRelayTokenSpec, TEST_AVATAR_URL, TEST_COLLAB_ISSUER,
    TEST_COLLAB_JWKS_ENDPOINT, TEST_DEVICE_ID, TEST_DISPLAY_NAME, TEST_SUBJECT, TEST_TICKET_ID,
};
pub use collab_ticket::{
    CollabTicketPoll, CollabTicketProvider, CollabTicketRequest, CollabTicketRequestId,
    OpaqueCollabRelayToken, OpaqueCollabTicket, UnavailableCollabTicketProvider,
    MAX_COLLAB_TICKET_BYTES,
};
pub use collab_ticket_error::{CollabTicketError, CollabTicketProviderErrorCode};
pub use collab_union_policy::{
    CollabUnionPolicy, COLLAB_UNION_POLICY_CURRENT_ROOT_X, COLLAB_UNION_POLICY_LEGACY_ROOT_X,
    COLLAB_UNION_POLICY_ROOT_X, COLLAB_UNION_POLICY_VERSION, MAX_COLLAB_UNION_POLICY_KEYS,
    MAX_COLLAB_UNION_POLICY_LIFETIME_SECONDS, MAX_COLLAB_UNION_POLICY_REGIONS,
};
pub use collab_verifier::{
    CollabTicketVerifier, MAX_COLLAB_JWS_CLAIMS_BYTES, MAX_COLLAB_JWS_HEADER_BYTES,
};
pub use collab_verifier_error::{
    CollabJwkErrorKind, CollabJwksError, CollabJwksFetchError, CollabUnionPolicyError,
    CollabVerifierConfigError, CollabVerifyError,
};
pub use status::AuthStatus;

/// Oldest supported base authentication ABI.
pub const REQUIRED_ABI_VERSION: u32 = 1;

/// Newest supported base authentication ABI.
pub const MAX_SUPPORTED_ABI_VERSION: u32 = 3;

/// Optional ABI revision that adds the collaboration-ticket capability.
///
/// Base ABI v1 remains usable for device login when this capability is absent;
/// build-time negotiation must not disable the existing account UI merely
/// because an older prebuilt cannot issue collaboration tickets.
pub const COLLAB_TICKET_ABI_VERSION: u32 = 2;

/// Optional ABI revision that adds the claim-minimized relay-token capability.
///
/// Appending `op_auth_collab_relay_token_begin` to the C ABI is a real ABI
/// change, so this is a deliberate bump rather than a reuse of
/// [`COLLAB_TICKET_ABI_VERSION`]. It is negotiated independently: an ABI-v2
/// prebuilt keeps issuing collaboration tickets and the host keeps using the
/// full ticket as its relay bearer, which the relay still dual-accepts.
/// The private `op-auth-ffi` `ABI_VERSION` must move in step.
pub const COLLAB_RELAY_TOKEN_ABI_VERSION: u32 = 3;

/// `poll` handle that reports the signed-in session instead of a flow.
pub const SESSION_HANDLE: u64 = 0;

/// Env var selecting the regional credential and ticket origin.
pub const ENV_SSO_URL: &str = "OPENPENCIL_SSO_URL";

/// Env var pinning the collaboration-ticket issuer independently of login.
pub const ENV_COLLAB_ISSUER: &str = "OPENPENCIL_COLLAB_ISSUER";

/// Env var pinning the regional offline-signed union-policy URL.
///
/// This override is accepted only when [`ENV_COLLAB_ISSUER`] is also set.
pub const ENV_COLLAB_POLICY_ENDPOINT: &str = "OPENPENCIL_COLLAB_POLICY_ENDPOINT";

/// Env var selecting the explicit legacy raw-JWKS compatibility path.
///
/// This override is accepted only when [`ENV_COLLAB_ISSUER`] is also set, so
/// operators cannot accidentally combine an explicit keyset with an implicit
/// issuer. It cannot be combined with [`ENV_COLLAB_POLICY_ENDPOINT`].
pub const ENV_COLLAB_JWKS_ENDPOINT: &str = "OPENPENCIL_COLLAB_JWKS_ENDPOINT";

/// Env var (`=1`) enabling the hosts' dev/demo fake-login fast path —
/// declared here so every host gates on the same name.
pub const ENV_DEV_FAKE_LOGIN: &str = "OPENPENCIL_DEV_FAKE_LOGIN";

/// Default production origin for device login and ticket requests.
///
/// This is deliberately separate from [`PRODUCTION_COLLAB_ISSUER`] even while
/// both values are identical, so moving the logical collaboration issuer does
/// not redirect credential-bearing requests.
pub const PRODUCTION_SSO_ORIGIN: &str = "https://sso.zseven.cn";

/// Overseas SSO origin. Same logical account space as
/// [`PRODUCTION_SSO_ORIGIN`]; the mobile shells pick one region per install
/// (IP-informed default with a user override) before the runtime starts.
pub const GLOBAL_SSO_ORIGIN: &str = "https://sso.zseven.tech";

/// Regional SSO deployment an embedded mobile shell signs in against.
///
/// Region is an *install-time* input: the proprietary runtime initializes
/// once per process and persists its device credential against a single
/// origin, so switching regions takes effect on the next launch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MobileSsoRegion {
    China,
    Global,
}

impl MobileSsoRegion {
    /// The credential-bearing SSO origin for this region.
    pub fn origin(self) -> &'static str {
        match self {
            MobileSsoRegion::China => PRODUCTION_SSO_ORIGIN,
            MobileSsoRegion::Global => GLOBAL_SSO_ORIGIN,
        }
    }
}

/// Everything the runtime needs at startup.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthInitConfig {
    /// Regional SSO origin, e.g. `https://sso.zseven.cn` (override via
    /// [`ENV_SSO_URL`]).
    pub base_url: String,
    /// Directory owning the persisted credential file.
    pub storage_dir: PathBuf,
    pub device_name: String,
    pub app_version: String,
}

/// Whether a working auth backend is linked into this build.
pub fn available() -> bool {
    backend::available()
}

static UNAVAILABLE_COLLAB_TICKET_PROVIDER: UnavailableCollabTicketProvider =
    UnavailableCollabTicketProvider;

#[cfg(any(op_auth_prebuilt, test))]
const fn supports_base_abi(version: u32) -> bool {
    version >= REQUIRED_ABI_VERSION && version <= MAX_SUPPORTED_ABI_VERSION
}

/// Whether the linked artifact exposes the optional collaboration-ticket ABI.
///
/// The current v1 artifacts intentionally fail closed. A future v2 backend
/// replaces the provider without changing base login availability.
pub fn collab_ticket_available() -> bool {
    collab_ticket_provider().available()
}

/// Provider for short-lived collaboration admission tickets.
pub fn collab_ticket_provider() -> &'static dyn CollabTicketProvider {
    backend::collab_ticket_provider()
}

/// Initialize the runtime once per process; returns false when the
/// backend is unavailable or already initialized.
pub fn init(config: &AuthInitConfig) -> bool {
    backend::init(config)
}

/// Initialize the process-global runtime for an embedded mobile shell.
///
/// Mobile shells can recreate their editor engine while the process remains
/// alive (for example after an Android activity recreation). The proprietary
/// runtime intentionally rejects a second initialization, so this wrapper
/// makes the *same* mobile configuration idempotent while still failing closed
/// if a later engine tries to change the credential directory or identity.
pub fn init_mobile(config: &AuthInitConfig) -> bool {
    static INITIALIZED_CONFIG: Mutex<Option<AuthInitConfig>> = Mutex::new(None);
    let mut initialized = INITIALIZED_CONFIG
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    if let Some(existing) = initialized.as_ref() {
        return existing == config;
    }
    if !backend::init(config) {
        return false;
    }
    *initialized = Some(config.clone());
    true
}

/// Load a persisted session, if any; the profile then arrives via
/// `poll(SESSION_HANDLE)`. Returns whether a credential was restored.
pub fn restore() -> bool {
    backend::restore()
}

/// Start a browser login and return the flow handle (0 when unavailable).
pub fn login_begin() -> u64 {
    backend::login_begin()
}

/// Poll a flow handle (or [`SESSION_HANDLE`]) for its current status.
pub fn poll(handle: u64) -> AuthStatus {
    backend::poll(handle)
}

/// Abort an in-flight login flow.
pub fn cancel(handle: u64) {
    backend::cancel(handle)
}

/// Drop the local session and revoke the device token server-side.
pub fn sign_out() {
    backend::sign_out()
}

/// The zseven-sso platform identifier for this build target.
pub fn platform_id() -> &'static str {
    if cfg!(target_os = "android") {
        "android"
    } else if cfg!(target_os = "ios") {
        "ios"
    } else if cfg!(target_os = "macos") {
        "desktop_macos"
    } else if cfg!(target_os = "windows") {
        "desktop_windows"
    } else {
        "desktop_linux"
    }
}

/// Pinned production configuration for an embedded iOS or Android shell.
/// Unlike desktop startup this deliberately ignores environment overrides:
/// an app process must not redirect persisted device credentials through an
/// ambient shell variable. The shell supplies the region it resolved
/// (persisted preference, falling back to an IP-informed default); both
/// values are first-party production origins, never caller-provided URLs.
pub fn mobile_init_config(
    storage_dir: PathBuf,
    device_name: impl Into<String>,
    app_version: impl Into<String>,
    region: MobileSsoRegion,
) -> AuthInitConfig {
    AuthInitConfig {
        base_url: region.origin().to_string(),
        storage_dir,
        device_name: device_name.into(),
        app_version: app_version.into(),
    }
}

/// Human-readable machine name shown on the SSO approval page and in the
/// account device list. The page itself labels the product and platform,
/// so this must be just the machine — never "OpenPencil …". Shared by the
/// desktop GUI and the serve-web daemon.
pub fn device_display_name() -> String {
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = std::process::Command::new("scutil")
            .args(["--get", "ComputerName"])
            .output()
        {
            if output.status.success() {
                let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !name.is_empty() {
                    return name;
                }
            }
        }
    }
    if let Ok(output) = std::process::Command::new("hostname").output() {
        if output.status.success() {
            let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !name.is_empty() {
                return name;
            }
        }
    }
    // Windows sets COMPUTERNAME; some unix shells export HOSTNAME.
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .ok()
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "Desktop".to_string())
}

/// Standard [`AuthInitConfig`] shared by the desktop GUI and the serve-web
/// daemon: sso base URL (override via `OPENPENCIL_SSO_URL`), credential
/// store under `<openpencil_dir>/auth`, machine device name.
pub fn desktop_init_config(openpencil_dir: &std::path::Path, app_version: &str) -> AuthInitConfig {
    // Preserve a present-but-non-UTF-8 override as an invalid empty origin so
    // backend URL validation fails instead of silently contacting production.
    let configured_sso_url =
        std::env::var_os(ENV_SSO_URL).map(|value| value.into_string().unwrap_or_default());
    AuthInitConfig {
        base_url: desktop_sso_origin(configured_sso_url.as_deref()).to_owned(),
        storage_dir: openpencil_dir.join("auth"),
        device_name: device_display_name(),
        app_version: app_version.to_string(),
    }
}

/// Pinned collaboration verifier trust root for the desktop process.
///
/// All inputs are trusted process-startup configuration. They are never
/// populated from tickets, the private provider, discovery, or peers:
///
/// - with no overrides, production uses the fixed signed-policy endpoint;
/// - an explicit legacy [`ENV_SSO_URL`] keeps its same-origin raw-JWKS path;
/// - [`ENV_COLLAB_ISSUER`] alone derives the fixed [`COLLAB_POLICY_PATH`];
/// - issuer plus [`ENV_COLLAB_POLICY_ENDPOINT`] pins a regional policy mirror;
/// - issuer plus [`ENV_COLLAB_JWKS_ENDPOINT`] explicitly selects legacy JWKS;
/// - conflicting or endpoint-only overrides fail closed.
///
/// This lets regional deployments keep separate local login origins while
/// pinning one shared collaboration issuer and signed public union.
pub fn desktop_collab_verifier_config() -> Result<CollabVerifierConfig, CollabVerifierConfigError> {
    let configured_sso_url =
        verifier_startup_env(ENV_SSO_URL, CollabVerifierConfigError::InvalidIssuer)?;
    let configured_collab_issuer =
        verifier_startup_env(ENV_COLLAB_ISSUER, CollabVerifierConfigError::InvalidIssuer)?;
    let configured_collab_policy_endpoint = verifier_startup_env(
        ENV_COLLAB_POLICY_ENDPOINT,
        CollabVerifierConfigError::InvalidPolicyEndpoint,
    )?;
    let configured_collab_jwks_endpoint = verifier_startup_env(
        ENV_COLLAB_JWKS_ENDPOINT,
        CollabVerifierConfigError::InvalidJwksEndpoint,
    )?;
    desktop_collab_verifier_config_from_startup(
        configured_sso_url.as_deref(),
        configured_collab_issuer.as_deref(),
        configured_collab_policy_endpoint.as_deref(),
        configured_collab_jwks_endpoint.as_deref(),
    )
}

fn verifier_startup_env(
    name: &str,
    invalid_error: CollabVerifierConfigError,
) -> Result<Option<String>, CollabVerifierConfigError> {
    verifier_startup_env_value(std::env::var(name), invalid_error)
}

fn verifier_startup_env_value(
    value: Result<String, std::env::VarError>,
    invalid_error: CollabVerifierConfigError,
) -> Result<Option<String>, CollabVerifierConfigError> {
    match value {
        Ok(value) => Ok(Some(value)),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(std::env::VarError::NotUnicode(_)) => Err(invalid_error),
    }
}

fn desktop_sso_origin(configured_sso_url: Option<&str>) -> &str {
    configured_sso_url.unwrap_or(PRODUCTION_SSO_ORIGIN)
}

fn desktop_collab_verifier_config_from_startup(
    configured_sso_url: Option<&str>,
    configured_collab_issuer: Option<&str>,
    configured_collab_policy_endpoint: Option<&str>,
    configured_collab_jwks_endpoint: Option<&str>,
) -> Result<CollabVerifierConfig, CollabVerifierConfigError> {
    match (
        configured_collab_issuer,
        configured_collab_policy_endpoint,
        configured_collab_jwks_endpoint,
    ) {
        (Some(_), Some(_), Some(_)) => Err(CollabVerifierConfigError::ConflictingKeyEndpoints),
        (Some(issuer), Some(policy_endpoint), None) => {
            CollabVerifierConfig::new_signed_policy(issuer, policy_endpoint)
        }
        (Some(issuer), None, Some(jwks_endpoint)) => {
            CollabVerifierConfig::new_pinned(issuer, jwks_endpoint)
        }
        (Some(issuer), None, None) => CollabVerifierConfig::for_policy_origin(issuer),
        (None, Some(_), _) | (None, _, Some(_)) => Err(CollabVerifierConfigError::InvalidIssuer),
        (None, None, None) => match configured_sso_url {
            Some(sso_origin) => CollabVerifierConfig::for_sso_origin(sso_origin),
            None => Ok(CollabVerifierConfig::production()),
        },
    }
}

#[cfg(test)]
mod abi_tests {
    use super::*;

    #[test]
    fn base_account_api_accepts_v1_through_v3_only() {
        assert!(!supports_base_abi(0));
        assert!(supports_base_abi(1));
        assert!(supports_base_abi(2));
        assert!(supports_base_abi(3));
        assert!(!supports_base_abi(4));
    }

    #[test]
    fn mobile_configuration_is_pinned_to_production() {
        let config = mobile_init_config(
            PathBuf::from("/private/app/auth"),
            "Test Phone",
            "1.0.0",
            MobileSsoRegion::China,
        );
        assert_eq!(config.base_url, PRODUCTION_SSO_ORIGIN);
        assert_eq!(config.storage_dir, PathBuf::from("/private/app/auth"));
        assert_eq!(config.device_name, "Test Phone");
        assert_eq!(config.app_version, "1.0.0");

        let global = mobile_init_config(
            PathBuf::from("/private/app/auth"),
            "Test Phone",
            "1.0.0",
            MobileSsoRegion::Global,
        );
        assert_eq!(global.base_url, GLOBAL_SSO_ORIGIN);
        assert_eq!(MobileSsoRegion::China.origin(), "https://sso.zseven.cn");
        assert_eq!(MobileSsoRegion::Global.origin(), "https://sso.zseven.tech");
    }

    #[test]
    fn relay_token_capability_is_negotiated_above_the_ticket_capability() {
        const { assert!(COLLAB_RELAY_TOKEN_ABI_VERSION > COLLAB_TICKET_ABI_VERSION) };
        assert_eq!(COLLAB_RELAY_TOKEN_ABI_VERSION, MAX_SUPPORTED_ABI_VERSION);
        assert!(supports_base_abi(COLLAB_RELAY_TOKEN_ABI_VERSION));
    }

    #[test]
    fn desktop_sso_selection_defaults_to_production_without_rewriting_overrides() {
        assert_eq!(desktop_sso_origin(None), PRODUCTION_SSO_ORIGIN);
        assert_eq!(
            desktop_sso_origin(Some("https://auth.self-hosted.example:9443")),
            "https://auth.self-hosted.example:9443"
        );
        assert_eq!(desktop_sso_origin(Some("")), "");
    }

    #[test]
    fn collaboration_trust_defaults_to_production() {
        assert_eq!(
            desktop_collab_verifier_config_from_startup(None, None, None, None).unwrap(),
            CollabVerifierConfig::production()
        );
        assert!(CollabVerifierConfig::production().uses_signed_policy());
    }

    #[test]
    fn legacy_sso_override_remains_the_collaboration_trust_root() {
        let config = desktop_collab_verifier_config_from_startup(
            Some("https://auth.self-hosted.example:9443"),
            None,
            None,
            None,
        )
        .unwrap();
        assert_eq!(config.issuer(), "https://auth.self-hosted.example:9443");
        assert_eq!(
            config.jwks_endpoint(),
            "https://auth.self-hosted.example:9443/api/v1/collab/jwks"
        );
    }

    #[test]
    fn issuer_override_derives_the_fixed_signed_policy_path() {
        let config = desktop_collab_verifier_config_from_startup(
            Some("https://login.cn.example"),
            Some("https://collab.example"),
            None,
            None,
        )
        .unwrap();
        assert_eq!(config.issuer(), "https://collab.example");
        assert_eq!(
            config.policy_endpoint(),
            Some("https://collab.example/api/v1/collab/policy")
        );
    }

    #[test]
    fn regional_login_origins_pin_one_issuer_and_region_local_union_mirrors() {
        let china = desktop_collab_verifier_config_from_startup(
            Some("https://login.cn.example"),
            Some("https://collab.example"),
            Some("https://login.cn.example/api/v1/collab/policy"),
            None,
        )
        .unwrap();
        let global = desktop_collab_verifier_config_from_startup(
            Some("https://login.global.example"),
            Some("https://collab.example"),
            Some("https://login.global.example/api/v1/collab/policy"),
            None,
        )
        .unwrap();

        assert_ne!(
            desktop_sso_origin(Some("https://login.cn.example")),
            desktop_sso_origin(Some("https://login.global.example"))
        );
        assert_eq!(china.issuer(), "https://collab.example");
        assert_eq!(global.issuer(), "https://collab.example");
        assert_eq!(
            china.policy_endpoint(),
            Some("https://login.cn.example/api/v1/collab/policy")
        );
        assert_eq!(
            global.policy_endpoint(),
            Some("https://login.global.example/api/v1/collab/policy")
        );
    }

    #[test]
    fn endpoint_override_without_explicit_issuer_fails_closed() {
        for sso_origin in [None, Some("https://login.cn.example")] {
            for (policy, jwks) in [
                (Some("https://keys.example/collab/policy"), None),
                (None, Some("https://keys.example/collab/jwks")),
            ] {
                assert_eq!(
                    desktop_collab_verifier_config_from_startup(sso_origin, None, policy, jwks,),
                    Err(CollabVerifierConfigError::InvalidIssuer)
                );
            }
        }
    }

    #[test]
    fn malformed_startup_trust_overrides_never_fall_back() {
        assert_eq!(
            desktop_collab_verifier_config_from_startup(
                Some(PRODUCTION_COLLAB_ISSUER),
                Some("http://collab.example"),
                None,
                None,
            ),
            Err(CollabVerifierConfigError::InvalidIssuer)
        );
        assert_eq!(
            desktop_collab_verifier_config_from_startup(
                Some(PRODUCTION_COLLAB_ISSUER),
                Some("https://collab.example"),
                Some("http://keys.example/openpencil/collab/policy"),
                None,
            ),
            Err(CollabVerifierConfigError::InvalidPolicyEndpoint)
        );
        assert_eq!(
            desktop_collab_verifier_config_from_startup(
                Some(PRODUCTION_COLLAB_ISSUER),
                Some("https://collab.example"),
                None,
                Some("http://keys.example/openpencil/collab.jwks"),
            ),
            Err(CollabVerifierConfigError::InvalidJwksEndpoint)
        );
        assert_eq!(
            desktop_collab_verifier_config_from_startup(
                Some(PRODUCTION_COLLAB_ISSUER),
                Some(""),
                None,
                None,
            ),
            Err(CollabVerifierConfigError::InvalidIssuer)
        );
        assert_eq!(
            desktop_collab_verifier_config_from_startup(
                Some(PRODUCTION_COLLAB_ISSUER),
                Some("https://collab.example"),
                None,
                Some(""),
            ),
            Err(CollabVerifierConfigError::InvalidJwksEndpoint)
        );
    }

    #[test]
    fn invalid_legacy_sso_override_still_fails_closed_without_collab_overrides() {
        assert_eq!(
            desktop_collab_verifier_config_from_startup(
                Some("http://login.example"),
                None,
                None,
                None
            ),
            Err(CollabVerifierConfigError::InvalidIssuer)
        );
    }

    #[test]
    fn policy_and_legacy_jwks_overrides_cannot_be_combined() {
        assert_eq!(
            desktop_collab_verifier_config_from_startup(
                None,
                Some("https://collab.example"),
                Some("https://cn.example/api/v1/collab/policy"),
                Some("https://cn.example/api/v1/collab/jwks"),
            ),
            Err(CollabVerifierConfigError::ConflictingKeyEndpoints)
        );
    }

    #[test]
    fn non_utf8_trust_root_environment_never_falls_back() {
        let not_unicode = std::env::VarError::NotUnicode(std::ffi::OsString::new());
        assert_eq!(
            verifier_startup_env_value(
                Err(not_unicode),
                CollabVerifierConfigError::InvalidJwksEndpoint,
            ),
            Err(CollabVerifierConfigError::InvalidJwksEndpoint)
        );
    }
}
