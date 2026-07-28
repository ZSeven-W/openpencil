use std::sync::Arc;

use op_collab::Epoch;
use op_collab_transport::{
    DeviceStaticKey, FileKeyStore, KeyStoreError, OsKeyStore, StaticKeyStore,
};

use super::{CollabRuntimeError, CollabRuntimeFailure};

struct ProductionKeyStore;

impl StaticKeyStore for ProductionKeyStore {
    fn load_or_generate(&self) -> Result<DeviceStaticKey, KeyStoreError> {
        match OsKeyStore::new().load_or_generate() {
            Err(error) if allows_file_fallback(&error) => {
                let root = op_config_store::openpencil_dir()?;
                FileKeyStore::new(root.join("collaboration")).load_or_generate()
            }
            result => result,
        }
    }
}

pub(super) fn production_key_store() -> Arc<dyn StaticKeyStore> {
    Arc::new(ProductionKeyStore)
}

fn allows_file_fallback(error: &KeyStoreError) -> bool {
    matches!(error, KeyStoreError::PlatformStoreUnavailable)
}

pub(super) fn random_epoch() -> Result<Epoch, CollabRuntimeError> {
    let mut bytes = [0_u8; 8];
    getrandom::fill(&mut bytes)
        .map_err(|_| CollabRuntimeError::new(CollabRuntimeFailure::SecureKeyUnavailable))?;
    Ok(Epoch(u64::from_le_bytes(bytes).max(1)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_fallback_is_limited_to_an_absent_platform_store() {
        assert!(allows_file_fallback(
            &KeyStoreError::PlatformStoreUnavailable
        ));
        assert!(!allows_file_fallback(
            &KeyStoreError::PlatformStoreAccessDenied
        ));
        assert!(!allows_file_fallback(&KeyStoreError::PlatformStoreFailure));
        assert!(!allows_file_fallback(&KeyStoreError::UnsafePlatformEntry));
    }
}
