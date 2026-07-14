/// Deployment switch for allowing browser credentials to be copied into the
/// daemon's platform settings file.
pub const ENV_NAME: &str = "OPENPENCIL_PERSIST_WEB_CREDENTIALS_SERVER";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebCredentialPersistence {
    BrowserOnly,
    Server,
}

impl WebCredentialPersistence {
    pub fn server_persistence(self) -> bool {
        matches!(self, Self::Server)
    }
}

pub fn parse(raw: Option<&str>) -> WebCredentialPersistence {
    match raw.map(str::trim).filter(|value| !value.is_empty()) {
        Some(value) if value.eq_ignore_ascii_case("true") => WebCredentialPersistence::Server,
        _ => WebCredentialPersistence::BrowserOnly,
    }
}

pub fn from_env() -> WebCredentialPersistence {
    let raw = std::env::var(ENV_NAME).ok();
    let policy = parse(raw.as_deref());
    if raw.as_deref().is_some_and(|value| {
        let value = value.trim();
        !value.is_empty()
            && !["0", "1", "false", "true", "no", "yes", "off", "on"]
                .iter()
                .any(|known| value.eq_ignore_ascii_case(known))
    }) {
        eprintln!("openpencil --serve-web: invalid {ENV_NAME}; using browser-only credentials");
    }
    policy
}

#[cfg(test)]
mod tests {
    use super::{parse, WebCredentialPersistence};

    #[test]
    fn missing_or_invalid_policy_fails_closed() {
        assert_eq!(parse(None), WebCredentialPersistence::BrowserOnly);
        assert_eq!(parse(Some("")), WebCredentialPersistence::BrowserOnly);
        assert_eq!(parse(Some("maybe")), WebCredentialPersistence::BrowserOnly);
    }

    #[test]
    fn only_trimmed_case_insensitive_true_enables_server_persistence() {
        for raw in ["true", "TRUE", " TrUe ", "\ttrue\n"] {
            assert_eq!(parse(Some(raw)), WebCredentialPersistence::Server);
        }
    }

    #[test]
    fn truthy_aliases_do_not_enable_server_persistence() {
        for raw in ["1", "yes", "on", "enabled"] {
            assert_eq!(parse(Some(raw)), WebCredentialPersistence::BrowserOnly);
        }
    }
}
