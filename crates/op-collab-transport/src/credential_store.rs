use std::fmt;
use std::sync::{Arc, Mutex};

use zeroize::{Zeroize, Zeroizing};

use crate::{DeviceStaticKey, KeyStoreError, StaticKeyStore};

const PRIVATE_KEY_BYTES: usize = 32;

/// Host-provided secure credential storage for platforms whose credential
/// service is owned by the application shell (Android Keystore, for example).
///
/// Implementations must keep the value encrypted by a non-exportable
/// platform key and must implement `store_if_absent` atomically. A concurrent
/// writer winning creation is success: [`CredentialStoreKeyStore`] always
/// reloads the canonical value after the call.
pub trait CredentialStore: Send + Sync {
    fn load(&self) -> Result<Option<Zeroizing<Vec<u8>>>, KeyStoreError>;

    fn store_if_absent(&self, value: &[u8]) -> Result<(), KeyStoreError>;
}

/// Adapts an application-shell credential store into the collaboration
/// transport's X25519 static-key store.
#[derive(Clone)]
pub struct CredentialStoreKeyStore {
    store: Arc<dyn CredentialStore>,
    process_lock: Arc<Mutex<()>>,
}

impl CredentialStoreKeyStore {
    pub fn new(store: Arc<dyn CredentialStore>) -> Self {
        Self {
            store,
            process_lock: Arc::new(Mutex::new(())),
        }
    }
}

impl fmt::Debug for CredentialStoreKeyStore {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CredentialStoreKeyStore")
            .field("store", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

impl StaticKeyStore for CredentialStoreKeyStore {
    fn load_or_generate(&self) -> Result<DeviceStaticKey, KeyStoreError> {
        let _guard = self
            .process_lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        if let Some(value) = self.store.load()? {
            return decode_private_key(value);
        }

        let generated = DeviceStaticKey::generate()?;
        self.store.store_if_absent(generated.private_key())?;

        // The platform store is the source of truth. Another process may
        // have won first creation, so never return the pre-write key.
        let canonical = self
            .store
            .load()?
            .ok_or(KeyStoreError::PlatformStoreFailure)?;
        decode_private_key(canonical)
    }
}

fn decode_private_key(mut value: Zeroizing<Vec<u8>>) -> Result<DeviceStaticKey, KeyStoreError> {
    if value.len() != PRIVATE_KEY_BYTES {
        return Err(KeyStoreError::InvalidLength(value.len()));
    }
    let mut private = [0_u8; PRIVATE_KEY_BYTES];
    private.copy_from_slice(&value);
    value.zeroize();
    let result = DeviceStaticKey::from_private(private);
    private.zeroize();
    result
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use super::*;

    type LoadResult = Result<Option<Zeroizing<Vec<u8>>>, KeyStoreError>;

    #[derive(Default)]
    struct FakeStore {
        loads: Mutex<VecDeque<LoadResult>>,
        stores: Mutex<Vec<Zeroizing<Vec<u8>>>>,
    }

    impl FakeStore {
        fn with_loads(
            loads: impl IntoIterator<Item = Result<Option<Vec<u8>>, KeyStoreError>>,
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

    impl CredentialStore for FakeStore {
        fn load(&self) -> Result<Option<Zeroizing<Vec<u8>>>, KeyStoreError> {
            self.loads
                .lock()
                .unwrap()
                .pop_front()
                .expect("unexpected load")
        }

        fn store_if_absent(&self, value: &[u8]) -> Result<(), KeyStoreError> {
            self.stores
                .lock()
                .unwrap()
                .push(Zeroizing::new(value.to_vec()));
            Ok(())
        }
    }

    #[test]
    fn existing_key_is_loaded_without_a_write() {
        let store = Arc::new(FakeStore::with_loads([Ok(Some(vec![7; 32]))]));
        let key_store = CredentialStoreKeyStore::new(store.clone());

        let key = key_store.load_or_generate().unwrap();

        assert_eq!(
            key.public_key(),
            DeviceStaticKey::from_private([7; 32]).unwrap().public_key()
        );
        assert!(store.stores.lock().unwrap().is_empty());
    }

    #[test]
    fn first_creation_reloads_the_platform_canonical_value() {
        let store = Arc::new(FakeStore::with_loads([Ok(None), Ok(Some(vec![9; 32]))]));
        let key_store = CredentialStoreKeyStore::new(store.clone());

        let key = key_store.load_or_generate().unwrap();

        assert_eq!(
            key.public_key(),
            DeviceStaticKey::from_private([9; 32]).unwrap().public_key()
        );
        assert_eq!(store.stores.lock().unwrap().len(), 1);
    }

    #[test]
    fn malformed_or_missing_canonical_values_fail_closed() {
        for value in [Vec::new(), vec![1; 31], vec![1; 33], vec![0; 32]] {
            let store = Arc::new(FakeStore::with_loads([Ok(Some(value))]));
            let key_store = CredentialStoreKeyStore::new(store.clone());
            assert!(key_store.load_or_generate().is_err());
            assert!(store.stores.lock().unwrap().is_empty());
        }

        let store = Arc::new(FakeStore::with_loads([Ok(None), Ok(None)]));
        let key_store = CredentialStoreKeyStore::new(store.clone());
        assert!(matches!(
            key_store.load_or_generate(),
            Err(KeyStoreError::PlatformStoreFailure)
        ));
        assert_eq!(store.stores.lock().unwrap().len(), 1);
    }

    #[test]
    fn platform_errors_are_not_replaced_with_a_new_identity() {
        for error in [
            KeyStoreError::PlatformStoreAccessDenied,
            KeyStoreError::PlatformStoreFailure,
            KeyStoreError::UnsafePlatformEntry,
        ] {
            let store = Arc::new(FakeStore::with_loads([Err(error)]));
            let key_store = CredentialStoreKeyStore::new(store.clone());
            assert!(key_store.load_or_generate().is_err());
            assert!(store.stores.lock().unwrap().is_empty());
        }
    }

    #[test]
    fn debug_never_exposes_the_store_or_private_key() {
        let store = Arc::new(FakeStore::with_loads([Ok(Some(vec![0x5a; 32]))]));
        let key_store = CredentialStoreKeyStore::new(store);
        let debug = format!("{key_store:?}");
        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains("90"));
    }
}
