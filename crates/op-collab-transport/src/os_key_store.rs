#[cfg(all(
    feature = "native",
    any(target_os = "linux", target_os = "macos", target_os = "windows")
))]
use std::sync::Mutex;

#[cfg(any(
    test,
    all(
        feature = "native",
        any(target_os = "linux", target_os = "macos", target_os = "windows")
    )
))]
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
#[cfg(any(
    test,
    all(
        feature = "native",
        any(target_os = "linux", target_os = "macos", target_os = "windows")
    )
))]
use zeroize::{Zeroize, Zeroizing};

use crate::{DeviceStaticKey, KeyStoreError, StaticKeyStore};

#[cfg(all(feature = "native", any(target_os = "linux", target_os = "macos")))]
const KEYRING_SERVICE: &str = "app.openpencil.collaboration";
#[cfg(all(feature = "native", any(target_os = "linux", target_os = "macos")))]
const KEYRING_ACCOUNT: &str = "device-noise-x25519-v1";
#[cfg(any(
    test,
    all(
        feature = "native",
        any(target_os = "linux", target_os = "macos", target_os = "windows")
    )
))]
const STORED_KEY_PREFIX: &str = "op-noise-x25519-v1:";
#[cfg(any(
    test,
    all(
        feature = "native",
        any(target_os = "linux", target_os = "macos", target_os = "windows")
    )
))]
const PRIVATE_KEY_BYTES: usize = 32;

#[cfg(all(
    feature = "native",
    any(target_os = "linux", target_os = "macos", target_os = "windows")
))]
static PROCESS_CREDENTIAL_LOCK: Mutex<()> = Mutex::new(());

/// Stores the device Noise static key in the operating system's credential
/// store.
///
/// macOS uses Keychain, Windows uses Credential Manager, and Linux uses the
/// desktop Secret Service. A locked, inaccessible, ambiguous, or malformed
/// store fails closed. Hosts that need the hardened file fallback can select
/// [`crate::FileKeyStore`] explicitly when no platform credential service is
/// available.
#[derive(Debug, Clone, Copy, Default)]
pub struct OsKeyStore;

impl OsKeyStore {
    pub const fn new() -> Self {
        Self
    }
}

impl StaticKeyStore for OsKeyStore {
    fn load_or_generate(&self) -> Result<DeviceStaticKey, KeyStoreError> {
        #[cfg(not(all(
            feature = "native",
            any(target_os = "linux", target_os = "macos", target_os = "windows")
        )))]
        {
            Err(KeyStoreError::UnsupportedPlatform)
        }

        #[cfg(all(
            feature = "native",
            any(target_os = "linux", target_os = "macos", target_os = "windows")
        ))]
        {
            // Platform stores do not all serialize same-entry operations
            // reliably. Serialize callers in this process, then always re-read
            // after creation so the OS store remains the source of truth.
            let _guard = PROCESS_CREDENTIAL_LOCK
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let entry = PlatformEntry::new()?;
            load_or_generate_from(&entry)
        }
    }
}

#[cfg(any(
    test,
    all(
        feature = "native",
        any(target_os = "linux", target_os = "macos", target_os = "windows")
    )
))]
pub(crate) trait CredentialEntry {
    fn load(&self) -> Result<Option<Zeroizing<String>>, KeyStoreError>;
    fn store(&self, value: &str) -> Result<(), KeyStoreError>;
}

#[cfg(any(
    test,
    all(
        feature = "native",
        any(target_os = "linux", target_os = "macos", target_os = "windows")
    )
))]
fn load_or_generate_from(entry: &impl CredentialEntry) -> Result<DeviceStaticKey, KeyStoreError> {
    if let Some(encoded) = entry.load()? {
        return decode_private_key(&encoded);
    }

    let generated = DeviceStaticKey::generate()?;
    let encoded = encode_private_key(&generated);
    entry.store(&encoded)?;

    // Never return the pre-write value. Another process may have initialized
    // the credential concurrently, and the persisted entry is canonical.
    let canonical = entry.load()?.ok_or(KeyStoreError::PlatformStoreFailure)?;
    decode_private_key(&canonical)
}

#[cfg(any(
    test,
    all(
        feature = "native",
        any(target_os = "linux", target_os = "macos", target_os = "windows")
    )
))]
fn encode_private_key(key: &DeviceStaticKey) -> Zeroizing<String> {
    let mut encoded = Zeroizing::new(String::with_capacity(
        STORED_KEY_PREFIX.len() + PRIVATE_KEY_BYTES.div_ceil(3) * 4,
    ));
    encoded.push_str(STORED_KEY_PREFIX);
    URL_SAFE_NO_PAD.encode_string(key.private_key(), &mut encoded);
    encoded
}

#[cfg(any(
    test,
    all(
        feature = "native",
        any(target_os = "linux", target_os = "macos", target_os = "windows")
    )
))]
fn decode_private_key(encoded: &str) -> Result<DeviceStaticKey, KeyStoreError> {
    let payload = encoded
        .strip_prefix(STORED_KEY_PREFIX)
        .ok_or(KeyStoreError::UnsafePlatformEntry)?;
    let mut decoded = Zeroizing::new(
        URL_SAFE_NO_PAD
            .decode(payload)
            .map_err(|_| KeyStoreError::UnsafePlatformEntry)?,
    );
    if decoded.len() != PRIVATE_KEY_BYTES {
        return Err(KeyStoreError::InvalidLength(decoded.len()));
    }

    let mut private = [0_u8; PRIVATE_KEY_BYTES];
    private.copy_from_slice(&decoded);
    decoded.zeroize();
    let result = DeviceStaticKey::from_private(private);
    private.zeroize();
    result
}

#[cfg(all(feature = "native", any(target_os = "linux", target_os = "macos")))]
struct KeyringEntry(keyring::Entry);

#[cfg(all(feature = "native", any(target_os = "linux", target_os = "macos")))]
impl KeyringEntry {
    fn new() -> Result<Self, KeyStoreError> {
        keyring::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT)
            .map(Self)
            .map_err(map_keyring_error)
    }
}

#[cfg(all(feature = "native", any(target_os = "linux", target_os = "macos")))]
impl CredentialEntry for KeyringEntry {
    fn load(&self) -> Result<Option<Zeroizing<String>>, KeyStoreError> {
        match self.0.get_password() {
            Ok(value) => Ok(Some(Zeroizing::new(value))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(map_keyring_error(error)),
        }
    }

    fn store(&self, value: &str) -> Result<(), KeyStoreError> {
        self.0.set_password(value).map_err(map_keyring_error)
    }
}

#[cfg(all(feature = "native", any(target_os = "linux", target_os = "macos")))]
fn map_keyring_error(error: keyring::Error) -> KeyStoreError {
    match error {
        keyring::Error::NoStorageAccess(_) => KeyStoreError::PlatformStoreAccessDenied,
        keyring::Error::BadEncoding(_) | keyring::Error::Ambiguous(_) => {
            KeyStoreError::UnsafePlatformEntry
        }
        keyring::Error::PlatformFailure(error) => map_platform_failure(error.as_ref()),
        keyring::Error::NoEntry | keyring::Error::TooLong(_, _) | keyring::Error::Invalid(_, _) => {
            KeyStoreError::PlatformStoreFailure
        }
        _ => KeyStoreError::PlatformStoreFailure,
    }
}

#[cfg(all(feature = "native", target_os = "linux"))]
fn map_platform_failure(error: &(dyn std::error::Error + Send + Sync + 'static)) -> KeyStoreError {
    if matches!(
        error.downcast_ref::<secret_service::Error>(),
        Some(secret_service::Error::Unavailable)
    ) {
        KeyStoreError::PlatformStoreUnavailable
    } else {
        KeyStoreError::PlatformStoreFailure
    }
}

#[cfg(all(feature = "native", target_os = "macos"))]
fn map_platform_failure(_: &(dyn std::error::Error + Send + Sync + 'static)) -> KeyStoreError {
    KeyStoreError::PlatformStoreFailure
}

#[cfg(all(feature = "native", any(target_os = "linux", target_os = "macos")))]
type PlatformEntry = KeyringEntry;
#[cfg(all(feature = "native", target_os = "windows"))]
type PlatformEntry = crate::windows_key_store::WindowsCredentialEntry;

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::Mutex;

    use super::*;

    #[derive(Default)]
    struct FakeEntry {
        loads: Mutex<VecDeque<Result<Option<Zeroizing<String>>, KeyStoreError>>>,
        stores: Mutex<Vec<Zeroizing<String>>>,
    }

    impl FakeEntry {
        fn with_loads(
            loads: impl IntoIterator<Item = Result<Option<String>, KeyStoreError>>,
        ) -> Self {
            Self {
                loads: Mutex::new(
                    loads
                        .into_iter()
                        .map(|value| value.map(|value| value.map(Zeroizing::new)))
                        .collect(),
                ),
                stores: Mutex::new(Vec::new()),
            }
        }
    }

    impl CredentialEntry for FakeEntry {
        fn load(&self) -> Result<Option<Zeroizing<String>>, KeyStoreError> {
            self.loads
                .lock()
                .unwrap()
                .pop_front()
                .expect("unexpected load")
        }

        fn store(&self, value: &str) -> Result<(), KeyStoreError> {
            self.stores
                .lock()
                .unwrap()
                .push(Zeroizing::new(value.to_owned()));
            Ok(())
        }
    }

    fn encoded_key(byte: u8) -> String {
        let key = DeviceStaticKey::from_private([byte; PRIVATE_KEY_BYTES]).unwrap();
        encode_private_key(&key).to_string()
    }

    #[test]
    fn existing_key_round_trips_without_write() {
        let encoded = encoded_key(7);
        let entry = FakeEntry::with_loads([Ok(Some(encoded))]);
        let key = load_or_generate_from(&entry).unwrap();
        assert_eq!(
            key.public_key(),
            DeviceStaticKey::from_private([7; PRIVATE_KEY_BYTES])
                .unwrap()
                .public_key()
        );
        assert!(entry.stores.lock().unwrap().is_empty());
    }

    #[test]
    fn missing_key_is_written_then_reloaded_from_store() {
        let canonical = encoded_key(9);
        let entry = FakeEntry::with_loads([Ok(None), Ok(Some(canonical))]);
        let key = load_or_generate_from(&entry).unwrap();
        assert_eq!(
            key.public_key(),
            DeviceStaticKey::from_private([9; PRIVATE_KEY_BYTES])
                .unwrap()
                .public_key()
        );
        let stores = entry.stores.lock().unwrap();
        assert_eq!(stores.len(), 1);
        assert!(stores[0].starts_with(STORED_KEY_PREFIX));
        assert!(!stores[0].contains("[9, 9"));
    }

    #[test]
    fn malformed_or_all_zero_entries_fail_closed_without_overwrite() {
        for value in [
            "wrong-version:AAAA".to_owned(),
            format!("{STORED_KEY_PREFIX}not+base64"),
            format!("{STORED_KEY_PREFIX}{}", URL_SAFE_NO_PAD.encode([0_u8; 32])),
        ] {
            let entry = FakeEntry::with_loads([Ok(Some(value))]);
            assert!(load_or_generate_from(&entry).is_err());
            assert!(entry.stores.lock().unwrap().is_empty());
        }
    }

    #[test]
    fn store_errors_do_not_trigger_an_insecure_fallback() {
        let entry = FakeEntry::with_loads([Err(KeyStoreError::PlatformStoreAccessDenied)]);
        assert!(matches!(
            load_or_generate_from(&entry),
            Err(KeyStoreError::PlatformStoreAccessDenied)
        ));
        assert!(entry.stores.lock().unwrap().is_empty());
    }

    #[test]
    fn missing_post_write_value_fails_closed() {
        let entry = FakeEntry::with_loads([Ok(None), Ok(None)]);
        assert!(matches!(
            load_or_generate_from(&entry),
            Err(KeyStoreError::PlatformStoreFailure)
        ));
        assert_eq!(entry.stores.lock().unwrap().len(), 1);
    }

    #[cfg(all(feature = "native", any(target_os = "linux", target_os = "macos")))]
    #[test]
    fn native_entry_builder_is_configured_without_accessing_the_store() {
        KeyringEntry::new().unwrap();
    }

    #[cfg(all(feature = "native", target_os = "linux"))]
    #[test]
    fn linux_absent_secret_service_is_distinct_from_access_or_runtime_failures() {
        assert!(matches!(
            map_keyring_error(keyring::Error::PlatformFailure(Box::new(
                secret_service::Error::Unavailable
            ))),
            KeyStoreError::PlatformStoreUnavailable
        ));
        assert!(matches!(
            map_keyring_error(keyring::Error::NoStorageAccess(Box::new(
                secret_service::Error::Locked
            ))),
            KeyStoreError::PlatformStoreAccessDenied
        ));
        assert!(matches!(
            map_keyring_error(keyring::Error::PlatformFailure(Box::new(
                std::io::Error::other("credential service failed")
            ))),
            KeyStoreError::PlatformStoreFailure
        ));
    }
}
